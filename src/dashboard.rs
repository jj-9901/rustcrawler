use crate::models::{LinkEdge, PageRecord};
use std::collections::HashMap;
use std::fs;

pub fn generate_report(
    records: &[PageRecord],
    edges: &[LinkEdge],
    start_url: &str,
    total_time_secs: f64,
    path: &str,
) {
    let successful = records.iter().filter(|r| r.status != "ERROR").count();
    let broken = records.iter().filter(|r| r.status == "ERROR").count();
    let avg_ms = if successful == 0 {
        0
    } else {
        records
            .iter()
            .filter(|r| r.status != "ERROR")
            .map(|r| r.response_time_ms)
            .sum::<u128>()
            / successful as u128
    };

    // Build nodes and edges JSON for the graph
    let mut node_set: HashMap<String, usize> = HashMap::new();
    let mut node_id = 0usize;

    // First, add all URLs from records
    for record in records {
        if !node_set.contains_key(&record.url) {
            node_set.insert(record.url.clone(), node_id);
            node_id += 1;
        }
    }
    
    // Then add from edges
    for edge in edges {
        if !node_set.contains_key(&edge.from) {
            node_set.insert(edge.from.clone(), node_id);
            node_id += 1;
        }
        if !node_set.contains_key(&edge.to) {
            node_set.insert(edge.to.clone(), node_id);
            node_id += 1;
        }
    }

    // Deduplicate edges
    let mut seen = std::collections::HashSet::new();
    let mut unique_edges: Vec<&LinkEdge> = Vec::new();
    for edge in edges {
        let key = format!("{}->{}", edge.from, edge.to);
        if !seen.contains(&key) {
            seen.insert(key);
            unique_edges.push(edge);
        }
    }

    // Build nodes array properly as JSON
    let mut nodes_vec = Vec::new();
    for (url, id) in &node_set {
        let short = shorten_url(url, 40);
        nodes_vec.push(format!(
            r#"{{"id":{},"label":"{}","url":"{}"}}"#,
            id, short, url
        ));
    }
    let nodes_json = format!("[{}]", nodes_vec.join(","));

    // Build edges array properly as JSON
    let mut edges_vec = Vec::new();
    for edge in &unique_edges {
        if let (Some(from_id), Some(to_id)) = (node_set.get(&edge.from), node_set.get(&edge.to)) {
            edges_vec.push(format!(
            r#"{{"source":{},"target":{}}}"#,
            from_id,
            to_id
        ));
        }
    }
    let edges_json = format!("[{}]", edges_vec.join(","));

    // Debug output
    println!("🔍 Graph Data:");
    println!("  Total Records: {}", records.len());
    println!("  Total Edges: {}", edges.len());
    println!("  Unique Nodes: {}", node_set.len());
    println!("  Unique Edges: {}", unique_edges.len());
    println!("  Nodes JSON length: {}", nodes_json.len());
    println!("  Edges JSON length: {}", edges_json.len());

    // Build pages table rows
    let rows: String = records
        .iter()
        .map(|r| {
            let status_class = if r.status == "ERROR" {
                "error"
            } else {
                "ok"
            };
            format!(
                r#"<tr>
                    <td class="url-cell">{}</td>
                    <td class="{}">{}</td>
                    <td>{}</td>
                    <td>{}ms</td>
                    <td>{}</td>
                </tr>"#,
                r.url, status_class, r.status, r.depth, r.response_time_ms, r.links_found
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Internal vs external
    let start_host = extract_host(start_url);
    let internal = edges
        .iter()
        .filter(|e| extract_host(&e.to) == start_host)
        .count();
    let external = edges.len() - internal;

    // Build response bars
    let response_bars = build_response_bars(records);

    // Write the HTML file
    let html_content = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>RustCrawler Report</title>
<script src="https://cdnjs.cloudflare.com/ajax/libs/d3/7.8.5/d3.min.js"></script>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0f1117; color: #e0e0e0; }}
  header {{ background: #1a1d2e; padding: 24px 40px; border-bottom: 1px solid #2a2d3e; }}
  header h1 {{ font-size: 24px; color: #e06c75; letter-spacing: 1px; }}
  header p {{ color: #888; margin-top: 4px; font-size: 14px; }}
  .container {{ max-width: 1400px; margin: 0 auto; padding: 32px 40px; }}
  .cards {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; margin-bottom: 40px; }}
  .card {{ background: #1a1d2e; border-radius: 12px; padding: 24px; border: 1px solid #2a2d3e; }}
  .card .value {{ font-size: 36px; font-weight: 700; color: #61afef; }}
  .card .label {{ font-size: 13px; color: #888; margin-top: 6px; text-transform: uppercase; letter-spacing: 0.5px; }}
  .card.red .value {{ color: #e06c75; }}
  .card.green .value {{ color: #98c379; }}
  .card.yellow .value {{ color: #e5c07b; }}
  .section {{ margin-bottom: 48px; }}
  .section h2 {{ font-size: 18px; color: #abb2bf; margin-bottom: 16px; padding-bottom: 8px; border-bottom: 1px solid #2a2d3e; }}
  #graph-container {{ background: #1a1d2e; border-radius: 12px; border: 1px solid #2a2d3e; height: 500px; overflow: hidden; position: relative; }}
  #graph-container svg {{ width: 100%; height: 100%; }}
  .graph-hint {{ position: absolute; bottom: 12px; right: 16px; font-size: 12px; color: #555; }}
  table {{ width: 100%; border-collapse: collapse; background: #1a1d2e; border-radius: 12px; overflow: hidden; border: 1px solid #2a2d3e; }}
  th {{ background: #21253a; padding: 12px 16px; text-align: left; font-size: 13px; color: #888; text-transform: uppercase; letter-spacing: 0.5px; }}
  td {{ padding: 10px 16px; font-size: 13px; border-top: 1px solid #2a2d3e; }}
  .url-cell {{ max-width: 500px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #61afef; }}
  .ok {{ color: #98c379; }}
  .error {{ color: #e06c75; }}
  .analytics-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }}
  .analytics-card {{ background: #1a1d2e; border-radius: 12px; padding: 24px; border: 1px solid #2a2d3e; }}
  .analytics-card h3 {{ font-size: 14px; color: #888; text-transform: uppercase; margin-bottom: 16px; }}
  .bar-row {{ display: flex; align-items: center; gap: 12px; margin-bottom: 10px; }}
  .bar-label {{ font-size: 12px; color: #abb2bf; width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex-shrink: 0; }}
  .bar-track {{ flex: 1; background: #2a2d3e; border-radius: 4px; height: 8px; }}
  .bar-fill {{ height: 8px; border-radius: 4px; background: #61afef; }}
  .bar-count {{ font-size: 12px; color: #888; width: 30px; text-align: right; }}
  .link-stat {{ display: flex; justify-content: space-between; align-items: center; padding: 12px 0; border-bottom: 1px solid #2a2d3e; }}
  .link-stat:last-child {{ border-bottom: none; }}
  .link-stat .name {{ font-size: 14px; color: #abb2bf; }}
  .link-stat .num {{ font-size: 20px; font-weight: 700; color: #61afef; }}
  #tooltip {{ position: fixed; background: #21253a; border: 1px solid #2a2d3e; border-radius: 8px; padding: 10px 14px; font-size: 12px; pointer-events: none; opacity: 0; transition: opacity 0.2s; max-width: 300px; word-break: break-all; z-index: 100; }}
  .no-data {{ display: flex; align-items: center; justify-content: center; height: 100%; flex-direction: column; color: #888; }}
  .debug-info {{ background: #1a1d2e; padding: 10px; margin: 10px 0; border-radius: 4px; font-family: monospace; font-size: 12px; color: #61afef; }}
</style>
</head>
<body>
<div id="tooltip"></div>
<header>
  <h1>🕷 RustCrawler Report</h1>
  <p>Crawled: <strong>{start_url}</strong> &nbsp;|&nbsp; Generated in {total_time:.2}s</p>
</header>
<div class="container">

  <div class="cards">
    <div class="card">
      <div class="value">{total}</div>
      <div class="label">Pages Crawled</div>
    </div>
    <div class="card green">
      <div class="value">{successful}</div>
      <div class="label">Successful</div>
    </div>
    <div class="card red">
      <div class="value">{broken}</div>
      <div class="label">Broken Links</div>
    </div>
    <div class="card yellow">
      <div class="value">{avg_ms}ms</div>
      <div class="label">Avg Response</div>
    </div>
    <div class="card">
      <div class="value">{total_edges}</div>
      <div class="label">Total Links</div>
    </div>
    <div class="card">
      <div class="value">{nodes}</div>
      <div class="label">Unique URLs</div>
    </div>
  </div>

  <div class="section">
    <h2>Link Graph</h2>
    <div id="graph-container">
      <svg id="graph-svg"></svg>
      <div class="graph-hint">scroll to zoom · drag to pan · hover nodes</div>
    </div>
  </div>

  <div class="section">
    <h2>Analytics</h2>
    <div class="analytics-grid">
      <div class="analytics-card">
        <h3>Link Types</h3>
        <div class="link-stat"><span class="name">Internal Links</span><span class="num" style="color:#98c379">{internal}</span></div>
        <div class="link-stat"><span class="name">External Links</span><span class="num" style="color:#e06c75">{external}</span></div>
        <div class="link-stat"><span class="name">Total Edges</span><span class="num">{total_edges}</span></div>
      </div>
      <div class="analytics-card">
        <h3>Response Times</h3>
        {response_bars}
      </div>
    </div>
  </div>

  <div class="section">
    <h2>Pages</h2>
    <table>
      <thead>
        <tr>
          <th>URL</th><th>Status</th><th>Depth</th><th>Response Time</th><th>Links Found</th>
        </tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>
  </div>

  <div class="debug-info" id="debug-info">Loading...</div>
</div>

<script>
// The data is injected as proper JSON arrays
var nodesData = {nodes_json};
var edgesData = {edges_json};

console.log('Data loaded:');
console.log('Nodes:', nodesData.length);
console.log('Edges:', edgesData.length);

// Show debug info
document.getElementById('debug-info').innerHTML = 
  'Nodes: ' + nodesData.length + ' | Edges: ' + edgesData.length;

if (nodesData.length === 0) {{
  document.getElementById('graph-container').innerHTML = 
    '<div class="no-data"><span style="font-size:48px;">📊</span><p>No nodes to display</p></div>';
}} else {{
  // Render the graph
  var container = document.getElementById('graph-container');
  var width = container.clientWidth || 800;
  var height = 500;

  var svg = d3.select('#graph-svg')
    .attr('viewBox', [0, 0, width, height])
    .style('width', '100%')
    .style('height', '100%');

  var g = svg.append('g');

  // Zoom behavior
  svg.call(d3.zoom()
    .scaleExtent([0.1, 8])
    .on('zoom', function(event) {{
      g.attr('transform', event.transform);
    }}));

  // Arrow marker
  svg.append('defs').append('marker')
    .attr('id', 'arrow')
    .attr('viewBox', '0 -5 10 10')
    .attr('refX', 20)
    .attr('refY', 0)
    .attr('markerWidth', 6)
    .attr('markerHeight', 6)
    .attr('orient', 'auto')
    .append('path')
    .attr('fill', '#444')
    .attr('d', 'M0,-5L10,0L0,5');

  // Force simulation
  console.log(nodesData);
  console.log(edgesData); 
  var simulation = d3.forceSimulation(nodesData)
    .force('link', d3.forceLink(edgesData).id(function(d) {{ return d.id; }}).distance(80))
    .force('charge', d3.forceManyBody().strength(-300))
    .force('center', d3.forceCenter(width / 2, height / 2))
    .force('collision', d3.forceCollide(30));

  // Draw edges
  var link = g.append('g')
    .selectAll('line')
    .data(edgesData)
    .join('line')
    .attr('stroke', '#2a2d3e')
    .attr('stroke-width', 1.5)
    .attr('marker-end', 'url(#arrow)');

  // Draw nodes
  var node = g.append('g')
    .selectAll('circle')
    .data(nodesData)
    .join('circle')
    .attr('r', 10)
    .attr('fill', function(d, i) {{ return i === 0 ? '#e06c75' : '#61afef'; }})
    .attr('stroke', '#0f1117')
    .attr('stroke-width', 2)
    .style('cursor', 'pointer')
    .call(d3.drag()
      .on('start', function(event, d) {{
        if (!event.active) simulation.alphaTarget(0.3).restart();
        d.fx = d.x; d.fy = d.y;
      }})
      .on('drag', function(event, d) {{ d.fx = event.x; d.fy = event.y; }})
      .on('end', function(event, d) {{
        if (!event.active) simulation.alphaTarget(0);
        d.fx = null; d.fy = null;
      }}));

  // Tooltip
  var tooltip = document.getElementById('tooltip');

  node.on('mouseover', function(event, d) {{
    tooltip.style.opacity = '1';
    tooltip.innerHTML = '<strong>' + (d.label || d.url) + '</strong><br><small>' + d.url + '</small>';
  }})
  .on('mousemove', function(event) {{
    tooltip.style.left = (event.clientX + 12) + 'px';
    tooltip.style.top = (event.clientY + 12) + 'px';
  }})
  .on('mouseout', function() {{
    tooltip.style.opacity = '0';
  }})
  .on('click', function(event, d) {{
    window.open(d.url, '_blank');
  }});

  // Update positions
  simulation.on('tick', function() {{
    link
      .attr('x1', function(d) {{ return d.source.x; }})
      .attr('y1', function(d) {{ return d.source.y; }})
      .attr('x2', function(d) {{ return d.target.x; }})
      .attr('y2', function(d) {{ return d.target.y; }});
    node
      .attr('cx', function(d) {{ return d.x; }})
      .attr('cy', function(d) {{ return d.y; }});
  }});

  console.log('Graph rendered successfully!');
}}
</script>
</body>
</html>"#,
        start_url = start_url,
        total_time = total_time_secs,
        total = records.len(),
        successful = successful,
        broken = broken,
        avg_ms = avg_ms,
        total_edges = edges.len(),
        nodes = node_set.len(),
        internal = internal,
        external = external,
        rows = rows,
        nodes_json = nodes_json,
        edges_json = edges_json,
        response_bars = response_bars,
    );

    fs::write(path, html_content).expect("Could not write report file");
    println!("  ✅ Report saved to: {}", path);
    println!("  📊 Open with: python3 -m http.server 8080");
    println!("  📊 Then visit: http://localhost:8080/report.html");
}

fn build_response_bars(records: &[PageRecord]) -> String {
    let mut sorted: Vec<_> = records
        .iter()
        .filter(|r| r.status != "ERROR")
        .collect();
    sorted.sort_by(|a, b| b.response_time_ms.cmp(&a.response_time_ms));

    let max_ms = sorted.first().map(|r| r.response_time_ms).unwrap_or(1);

    sorted
        .iter()
        .take(8)
        .map(|r| {
            let pct = (r.response_time_ms as f64 / max_ms as f64 * 100.0) as u32;
            let short = shorten_url(&r.url, 30);
            format!(
                r#"<div class="bar-row">
                  <div class="bar-label" title="{}">{}</div>
                  <div class="bar-track"><div class="bar-fill" style="width:{}%"></div></div>
                  <div class="bar-count">{}ms</div>
                </div>"#,
                r.url, short, pct, r.response_time_ms
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn shorten_url(url: &str, max: usize) -> String {
    let trimmed = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    if trimmed.len() <= max {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..max])
    }
}

fn extract_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
}