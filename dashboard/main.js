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
      <tr>
        <td class="url-cell"><a href="${r.url}" target="_blank">${r.url}</a></td>
        <td class="${r.status === 'ERROR' ? 'status-error' : 'status-ok'}">${r.status}</td>
        <td>${r.depth}</td>
        <td>${r.response_time_ms}ms</td>
        <td>${r.links_found}</td>
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
});