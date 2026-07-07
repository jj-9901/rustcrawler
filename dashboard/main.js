function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

// Render stat cards
function renderCards(stats) {
  document.getElementById('stat-total').textContent    = stats.total_pages;
  document.getElementById('stat-success').textContent  = stats.successful;
  document.getElementById('stat-broken').textContent   = stats.broken_links;
  document.getElementById('stat-avgms').textContent    = stats.avg_response_ms + 'ms';
  document.getElementById('stat-edges').textContent    = stats.total_edges;
  document.getElementById('stat-nodes').textContent    = stats.unique_nodes;
  document.getElementById('stat-time').textContent     = stats.crawl_time_secs.toFixed(2) + 's';
  document.getElementById('stat-health').textContent   = stats.health_score + '%';
  document.getElementById('stat-deadends').textContent   = stats.dead_ends;
  document.getElementById('stat-density').textContent    = stats.graph_density.toFixed(4);
  document.getElementById('stat-components').textContent = stats.connected_components;
  const dupEl = document.getElementById('stat-duplicates');
  const redEl = document.getElementById('stat-redirects');
  if (dupEl) dupEl.textContent = stats.duplicate_pages || 0;
  if (redEl) redEl.textContent = stats.redirect_pages || 0;
  const healthCard = document.getElementById('card-health');
  if (stats.health_score >= 80) healthCard.className = 'card green';
  else if (stats.health_score >= 50) healthCard.className = 'card yellow';
  else healthCard.className = 'card red';
}

// Render pages table with search
function renderTable(pages) {
  const tbody = document.getElementById('pages-tbody');

  function draw(filtered) {
    tbody.innerHTML = filtered.map(r => `
      <tr onclick="openSidebarByUrl('${r.url}')" style="cursor:pointer">
        <td style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;
            color:#abb2bf" title="${r.title}">${r.title}</td>
        <td class="${r.status === 'ERROR' ? 'status-error' : 'status-ok'}">${r.status}</td>
        <td>${r.depth}</td>
        <td>${r.response_time_ms}ms</td>
        <td>${formatBytes(r.size_bytes)}</td>
        <td>${r.links_found}</td>
        <td>${r.redirects > 0 ?
          `<span style="color:#e5c07b">${r.redirects} hop${r.redirects > 1 ? 's' : ''}</span>`
          : '—'}</td>
        <td>${r.is_duplicate ?
          '<span style="color:#e06c75">Yes</span>' : '—'}</td>
        <td>${(r.pagerank || 0).toFixed(4)}</td>
      </tr>
    `).join('');
  }

  draw(pages);

  document.getElementById('table-search').addEventListener('input', e => {
    const q = e.target.value.toLowerCase();
    draw(q ? pages.filter(p => p.url.toLowerCase().includes(q)) : pages);
  });
}

// Render response time bars
function renderResponseBars(pages) {
  const sorted = [...pages]
    .filter(p => p.status !== 'ERROR')
    .sort((a, b) => b.response_time_ms - a.response_time_ms)
    .slice(0, 8);

  const max = sorted[0]?.response_time_ms || 1;
  const container = document.getElementById('response-bars');

  container.innerHTML = sorted.map(p => {
    const pct = (p.response_time_ms / max * 100).toFixed(0);
    const short = p.url.replace(/https?:\/\//, '').substring(0, 35);
    const color = p.response_time_ms > 1000 ? 'red' : p.response_time_ms > 500 ? '' : 'green';
    return `
      <div class="bar-row">
        <div class="bar-label" title="${p.url}">${short}</div>
        <div class="bar-track">
          <div class="bar-fill ${color}" style="width:${pct}%"></div>
        </div>
        <div class="bar-count">${p.response_time_ms}ms</div>
      </div>`;
  }).join('');
}

// Render link type stats
function renderLinkStats(stats) {
  document.getElementById('stat-internal').textContent = stats.internal_links;
  document.getElementById('stat-external').textContent = stats.external_links;
  document.getElementById('stat-total-edges').textContent = stats.total_edges;
}

// Render PageRank top pages
function renderPageRank(pages) {
  const sorted = [...pages]
    .filter(p => p.pagerank)
    .sort((a, b) => b.pagerank - a.pagerank)
    .slice(0, 8);

  const container = document.getElementById('pagerank-list');
  container.innerHTML = sorted.map((p, i) => {
    const short = p.url.replace(/https?:\/\//, '').substring(0, 45);
    return `
      <div class="pagerank-row">
        <div class="pagerank-rank">${i + 1}</div>
        <div class="pagerank-url" title="${p.url}">${short}</div>
        <div class="pagerank-score">${p.pagerank.toFixed(4)}</div>
      </div>`;
  }).join('');
}

// Render health issues
function renderHealth(stats) {
  const score = stats.health_score;
  const circle = document.getElementById('health-circle');
  circle.textContent = score + '%';
  circle.className = 'score-circle' +
    (score >= 80 ? '' : score >= 50 ? ' yellow' : ' red');

  const items = document.getElementById('health-items');
  items.innerHTML = stats.health_issues.map(issue => `
    <div class="health-item">
      <div class="dot" style="background:${issue.ok ? '#98c379' : '#e06c75'}"></div>
      ${issue.message}
    </div>
  `).join('');
}

// Render external domains
function renderExternalDomains(domains) {
  if (!domains || domains.length === 0) return;
  const max = domains[0].count;
  const container = document.getElementById('external-domains');
  container.innerHTML = domains.slice(0, 8).map(d => `
    <div class="bar-row">
      <div class="bar-label">${d.domain}</div>
      <div class="bar-track">
        <div class="bar-fill purple" style="width:${(d.count / max * 100).toFixed(0)}%"></div>
      </div>
      <div class="bar-count">${d.count}</div>
    </div>
  `).join('');
}

function openSidebar(page) {
  const sidebar = document.getElementById('sidebar');
  const overlay = document.getElementById('sidebar-overlay');
  const content = document.getElementById('sidebar-content');

  const statusColor = page.status === 'ERROR' ? '#e06c75' : '#98c379';
  const depthColors = ['#e06c75','#98c379','#61afef','#e5c07b','#c678dd','#56b6c2'];
  const depthColor  = depthColors[page.depth % depthColors.length] || '#888';

  content.innerHTML = `
    <div style="margin-bottom:20px">
      <div style="font-size:11px;color:#888;text-transform:uppercase;margin-bottom:6px">URL</div>
      <a href="${page.url}" target="_blank" style="
        color:#61afef;font-size:12px;word-break:break-all;line-height:1.5
      ">${page.url}</a>
    </div>

    <div style="margin-bottom:20px">
      <div style="font-size:11px;color:#888;text-transform:uppercase;margin-bottom:6px">Title</div>
      <div style="color:#e0e0e0;font-size:14px">${page.title || 'No title'}</div>
    </div>

    <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-bottom:20px">
      <div style="background:#0f1117;border-radius:8px;padding:12px">
        <div style="font-size:11px;color:#888;margin-bottom:4px">STATUS</div>
        <div style="font-size:16px;font-weight:700;color:${statusColor}">${page.status}</div>
      </div>
      <div style="background:#0f1117;border-radius:8px;padding:12px">
        <div style="font-size:11px;color:#888;margin-bottom:4px">DEPTH</div>
        <div style="font-size:16px;font-weight:700;color:${depthColor}">${page.depth}</div>
      </div>
      <div style="background:#0f1117;border-radius:8px;padding:12px">
        <div style="font-size:11px;color:#888;margin-bottom:4px">RESPONSE</div>
        <div style="font-size:16px;font-weight:700;color:#e5c07b">
          ${page.response_time_ms === '—' ? '—' : page.response_time_ms + 'ms'}
        </div>      
      </div>
      <div style="background:#0f1117;border-radius:8px;padding:12px">
        <div style="font-size:11px;color:#888;margin-bottom:4px">SIZE</div>
        <div style="font-size:16px;font-weight:700;color:#61afef">${formatBytes(page.size_bytes)}</div>
      </div>
      <div style="background:#0f1117;border-radius:8px;padding:12px">
        <div style="font-size:11px;color:#888;margin-bottom:4px">LINKS</div>
        <div style="font-size:16px;font-weight:700;color:#98c379">${page.links_found}</div>
      </div>
      <div style="background:#0f1117;border-radius:8px;padding:12px">
        <div style="font-size:11px;color:#888;margin-bottom:4px">PAGERANK</div>
        <div style="font-size:16px;font-weight:700;color:#c678dd">${(page.pagerank||0).toFixed(4)}</div>
      </div>
    </div>

    ${page.redirects > 0 ? `
    <div style="margin-bottom:20px;background:#0f1117;border-radius:8px;padding:12px">
      <div style="font-size:11px;color:#888;margin-bottom:4px">REDIRECTS</div>
      <div style="color:#e5c07b">${page.redirects} hop(s)</div>
    </div>` : ''}

    ${page.is_duplicate ? `
    <div style="margin-bottom:20px;background:#0f1117;border-radius:8px;padding:12px;
        border:1px solid #e06c75">
      <div style="color:#e06c75;font-size:13px">⚠ Possible duplicate page</div>
    </div>` : ''}

    <div style="margin-bottom:20px">
      <div style="font-size:11px;color:#888;text-transform:uppercase;margin-bottom:10px">
        PageRank Score
      </div>
      <div style="background:#2a2d3e;border-radius:4px;height:8px">
        <div style="
          height:8px;border-radius:4px;background:#c678dd;
          width:${Math.min((page.pagerank||0) * 5000, 100)}%
        "></div>
      </div>
    </div>
  `;

  sidebar.style.right = '0';
  overlay.style.display = 'block';
}

function closeSidebar() {
  document.getElementById('sidebar').style.right = '-380px';
  document.getElementById('sidebar-overlay').style.display = 'none';
}

function openSidebarByUrl(url) {
  const page = window.__CRAWL_PAGES__.find(p => p.url === url);
  if (page) openSidebar(page);
}

// Boot
document.addEventListener('DOMContentLoaded', () => {
  const stats = window.__CRAWL_STATS__;
  const pages = window.__CRAWL_PAGES__;
  const nodes = window.__GRAPH_NODES__;
  const edges = window.__GRAPH_EDGES__;

  renderCards(stats);
  renderTable(pages);
  renderResponseBars(pages);
  renderLinkStats(stats);
  renderPageRank(pages);
  renderHealth(stats);
  renderExternalDomains(stats.external_domains);
  renderGraph(nodes, edges);
  setTimeout(() => {
    d3.selectAll('circle').on('click', null);
    d3.selectAll('circle').on('click.sidebar', (event, d) => {
      event.preventDefault();
      event.stopPropagation();
      const page = window.__CRAWL_PAGES__.find(p => p.url === d.url);
      if (page) {
        openSidebar(page);
      } else {
        // Node exists in graph but wasn't crawled — show basic info
        openSidebar({
          url: d.url,
          title: d.label || 'Not crawled',
          status: 'Not crawled',
          depth: d.depth === 99 ? '—' : d.depth,
          response_time_ms: '—',
          size_bytes: 0,
          links_found: '—',
          pagerank: d.pagerank || 0,
          redirects: 0,
          is_duplicate: false,
        });
      }
    });
  }, 1000);
  document.getElementById('sidebar-close').onclick = closeSidebar;
  document.getElementById('sidebar-overlay').onclick = closeSidebar;

  // Depth color toggle — must be set AFTER renderGraph runs
  const depthColorMap = ['#e06c75', '#98c379', '#61afef', '#e5c07b', '#c678dd', '#56b6c2'];
  let colorByDepth = false;

  document.getElementById('btn-color-depth').onclick = () => {
    colorByDepth = !colorByDepth;

    document.getElementById('btn-color-depth').style.background =
      colorByDepth ? '#61afef' : '#21253a';

    // Build dynamic legend
    const legendDepth = document.getElementById('legend-depth');
    if (colorByDepth) {
      legendDepth.style.display = 'flex';
      legendDepth.style.gap = '12px';
      legendDepth.innerHTML = depthColorMap.map((color, i) => `
        <div class="legend-item">
          <div class="legend-dot" style="background:${color}"></div>
          Depth ${i}
        </div>
      `).join('');
    } else {
      legendDepth.style.display = 'none';
      legendDepth.innerHTML = '';
    }

    d3.selectAll('circle').attr('fill', (d, i) => {
      if (colorByDepth) {
        if (d.depth === undefined || d.depth === 99) return '#888';
        return depthColorMap[d.depth % depthColorMap.length];
      }
      if (i === 0) return '#e06c75';
      return '#61afef';
    });
  };
  // Graph search
  document.getElementById('graph-search').addEventListener('input', e => {
    const q = e.target.value.toLowerCase().trim();

    if (!q) {
      d3.selectAll('circle').attr('opacity', 1).attr('stroke', '#0f1117').attr('stroke-width', 2);
      return;
    }

    d3.selectAll('circle').each(function(d) {
      const matches = d.url.toLowerCase().includes(q);
      d3.select(this)
        .attr('opacity', matches ? 1 : 0.15)
        .attr('stroke', matches ? '#e5c07b' : '#0f1117')
        .attr('stroke-width', matches ? 4 : 2);
    });
  });
});