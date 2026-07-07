use crate::analytics::compute_pagerank;
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
    // ── PageRank ──────────────────────────────────────────────
    let pagerank = compute_pagerank(records, edges, 50, 0.85);

    // ── Basic stats ───────────────────────────────────────────
    let successful = records.iter().filter(|r| r.status != "ERROR").count();
    let broken     = records.iter().filter(|r| r.status == "ERROR").count();
    let avg_ms = if successful == 0 { 0 } else {
        records.iter().filter(|r| r.status != "ERROR")
            .map(|r| r.response_time_ms).sum::<u128>() / successful as u128
    };

    // ── Internal / external ───────────────────────────────────
    let start_host = extract_host(start_url);
    let internal = edges.iter().filter(|e| extract_host(&e.to) == start_host).count();
    let external = edges.len().saturating_sub(internal);

    // ── External domains ──────────────────────────────────────
    let mut ext_map: HashMap<String, usize> = HashMap::new();
    for edge in edges {
        let host = extract_host(&edge.to);
        if host != start_host && !host.is_empty() {
            *ext_map.entry(host).or_insert(0) += 1;
        }
    }
    let mut ext_domains: Vec<_> = ext_map.iter().collect();
    ext_domains.sort_by(|a, b| b.1.cmp(a.1));
    let ext_domains_json: String = ext_domains.iter().take(8).map(|(d, c)| {
        format!(r#"{{"domain":"{}","count":{}}}"#, d, c)
    }).collect::<Vec<_>>().join(",");

    // ── Health score ──────────────────────────────────────────
    let (health_score, health_issues) = compute_health(records, edges, &start_host);

    let health_issues_json: String = health_issues.iter().map(|(ok, msg)| {
        format!(r#"{{"ok":{},"message":"{}"}}"#, ok, msg)
    }).collect::<Vec<_>>().join(",");

    // ── Graph nodes ───────────────────────────────────────────
    let mut node_set: HashMap<String, usize> = HashMap::new();
    let mut node_id = 0usize;

    for record in records {
        if !node_set.contains_key(&record.url) {
            node_set.insert(record.url.clone(), node_id);
            node_id += 1;
        }
    }
    for edge in edges {
        for url in [&edge.from, &edge.to] {
            if !node_set.contains_key(url) {
                node_set.insert(url.clone(), node_id);
                node_id += 1;
            }
        }
    }

    let nodes_json: String = {
        let mut v: Vec<_> = node_set.iter().collect();
        v.sort_by_key(|(_, id)| *id);

        // Build a depth map from records
        let depth_map: HashMap<&str, u32> = records
            .iter()
            .map(|r| (r.url.as_str(), r.depth))
            .collect();

        v.iter().map(|(url, id)| {
            let short = shorten_url(url, 40);
            let pr = pagerank.get(*url).copied().unwrap_or(0.0);
            let depth = depth_map.get(url.as_str()).copied().unwrap_or(99);
            format!(
                r#"{{"id":{},"label":"{}","url":"{}","pagerank":{:.4},"depth":{}}}"#,
                id, escape_json(&short), escape_json(url), pr, depth
            )
        }).collect::<Vec<_>>().join(",")
    };

    // ── Graph edges ───────────────────────────────────────────
    let mut seen_edges = std::collections::HashSet::new();
    let edges_json: String = edges.iter().filter_map(|e| {
        let key = format!("{}->{}", e.from, e.to);
        if seen_edges.contains(&key) { return None; }
        seen_edges.insert(key);
        let fi = node_set.get(&e.from)?;
        let ti = node_set.get(&e.to)?;
        Some(format!(r#"{{"source":{},"target":{}}}"#, fi, ti))
    }).collect::<Vec<_>>().join(",");

    // ── Pages JSON (with pagerank) ────────────────────────────
    let pages_json: String = records.iter().map(|r| {
        let pr = pagerank.get(&r.url).copied().unwrap_or(0.0);
        format!(
            r#"{{"url":"{}","status":"{}","depth":{},"response_time_ms":{},"links_found":{},"pagerank":{:.4},"title":"{}","size_bytes":{},"is_duplicate":{},"redirects":{}}}"#,
            escape_json(&r.url), r.status, r.depth, r.response_time_ms,
            r.links_found, pr, escape_json(&r.title), r.size_bytes,
            r.is_duplicate, r.redirect_count
        )
    }).collect::<Vec<_>>().join(",");

    // ── Graph analytics ───────────────────────────────────────
    let dead_ends = crate::analytics::find_dead_ends(records, edges);
    let density   = crate::analytics::compute_density(records.len(), edges.len());
    let components = crate::analytics::count_connected_components(records, edges);

    let stats_json = format!(
        r#"{{
    "total_pages": {},
    "successful": {},
    "broken_links": {},
    "avg_response_ms": {},
    "total_edges": {},
    "unique_nodes": {},
    "crawl_time_secs": {:.2},
    "internal_links": {},
    "external_links": {},
    "health_score": {},
    "health_issues": [{}],
    "external_domains": [{}],
    "dead_ends": {},
    "graph_density": {:.4},
    "connected_components": {},
    "duplicate_pages": {},
    "redirect_pages": {}
    }}"#,
        records.len(), successful, broken, avg_ms,
        edges.len(), node_set.len(), total_time_secs,
        internal, external, health_score,
        health_issues_json, ext_domains_json,
        dead_ends.len(), density, components,
        records.iter().filter(|r| r.is_duplicate).count(),
        records.iter().filter(|r| !r.redirect_chain.is_empty()).count()
    );

    // ── Data script block ─────────────────────────────────────
    let data_script = format!(
        "<script>\nwindow.__CRAWL_STATS__ = {};\nwindow.__CRAWL_PAGES__ = [{}];\nwindow.__GRAPH_NODES__ = [{}];\nwindow.__GRAPH_EDGES__ = [{}];\n</script>",
        stats_json, pages_json, nodes_json, edges_json
    );

    // ── Load template ─────────────────────────────────────────
    let template = fs::read_to_string("dashboard/index.html")
        .expect("Could not read dashboard/index.html — make sure the dashboard/ folder exists");

    let html = template
        .replace("__CRAWL_URL__", start_url)
        .replace("__CRAWL_TIME__", &format!("Generated in {:.2}s", total_time_secs))
        .replace("__CRAWL_DATA__", &data_script)
        .replace(r#"href="style.css""#, r#"href="dashboard/style.css""#)
        .replace(r#"src="graph.js""#, r#"src="dashboard/graph.js""#)
        .replace(r#"src="main.js""#, r#"src="dashboard/main.js""#);

    fs::write(path, html).expect("Could not write report file");
    println!("  Saved to : {}", path);
    println!("  Run      : python3 -m http.server 8080");
    println!("  Open     : http://localhost:8080/report.html");
}

// ── Health score ──────────────────────────────────────────────
fn compute_health(
    records: &[PageRecord],
    edges: &[LinkEdge],
    start_host: &str,
) -> (u32, Vec<(bool, String)>) {
    let mut score = 100i32;
    let mut issues = vec![];

    let broken = records.iter().filter(|r| r.status == "ERROR").count();
    if broken == 0 {
        issues.push((true, "No broken links found".to_string()));
    } else {
        score -= (broken as i32) * 5;
        issues.push((false, format!("{} broken links detected", broken)));
    }

    let slow = records.iter()
        .filter(|r| r.status != "ERROR" && r.response_time_ms > 1000)
        .count();
    if slow == 0 {
        issues.push((true, "All pages load under 1 second".to_string()));
    } else {
        score -= (slow as i32) * 3;
        issues.push((false, format!("{} pages load over 1 second", slow)));
    }

    let linked: std::collections::HashSet<_> = edges.iter().map(|e| &e.to).collect();
    let orphans = records.iter()
        .filter(|r| !linked.contains(&r.url) && extract_host(&r.url) == start_host)
        .count();
    if orphans == 0 {
        issues.push((true, "No orphan pages detected".to_string()));
    } else {
        score -= (orphans as i32) * 2;
        issues.push((false, format!("{} orphan pages (no inbound links)", orphans)));
    }

    let avg_ms = if records.is_empty() { 0u128 } else {
        records.iter().map(|r| r.response_time_ms).sum::<u128>() / records.len() as u128
    };
    if avg_ms < 500 {
        issues.push((true, format!("Good avg response time: {}ms", avg_ms)));
    } else {
        score -= 10;
        issues.push((false, format!("Slow avg response time: {}ms", avg_ms)));
    }

    (score.max(0) as u32, issues)
}

fn extract_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
}

fn shorten_url(url: &str, max: usize) -> String {
    let t = url.trim_start_matches("https://").trim_start_matches("http://");
    if t.len() <= max { t.to_string() } else { format!("{}...", &t[..max]) }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
}

pub fn generate_benchmark_report(
    results: &[crate::benchmark::BenchmarkResult],
    url: &str,
    path: &str,
) {
    let results_json = serde_json::to_string(results).unwrap();

    let max_pages_per_sec = results
        .iter()
        .map(|r| r.pages_per_sec)
        .fold(0.0f64, f64::max);

    let speedup = if results.first().map(|r| r.pages_per_sec).unwrap_or(0.0) > 0.0 {
        let baseline = results[0].pages_per_sec;
        results.last().map(|r| r.pages_per_sec / baseline).unwrap_or(1.0)
    } else {
        1.0
    };

    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>RustCrawler Benchmark</title>
<script src="https://cdnjs.cloudflare.com/ajax/libs/d3/7.8.5/d3.min.js"></script>
<link rel="stylesheet" href="dashboard/style.css">
</head>
<body>
<header>
  <div>
    <h1>⚡ RustCrawler Benchmark</h1>
    <p>Concurrency performance analysis for: <strong>{url}</strong></p>
  </div>
</header>
<div class="container">

  <div class="cards">
    <div class="card green">
      <div class="value">{speedup:.1}x</div>
      <div class="label">Max Speedup</div>
    </div>
    <div class="card">
      <div class="value">{max_pages_per_sec:.1}</div>
      <div class="label">Peak Pages/sec</div>
    </div>
    <div class="card">
      <div class="value">{results_len}</div>
      <div class="label">Worker Configs Tested</div>
    </div>
  </div>

  <div class="section">
    <h2>Pages per Second by Worker Count</h2>
    <div class="analytics-card">
      <div id="chart-bars"></div>
    </div>
  </div>

  <div class="section">
    <h2>Time vs Workers</h2>
    <div class="analytics-card">
      <div id="chart-time"></div>
    </div>
  </div>

  <div class="section">
    <h2>Raw Results</h2>
    <div class="table-wrapper">
      <table>
        <thead>
          <tr>
            <th>Workers</th>
            <th>Pages Crawled</th>
            <th>Total Time</th>
            <th>Pages/sec</th>
            <th>Avg Response</th>
            <th>Speedup vs 1 Worker</th>
          </tr>
        </thead>
        <tbody id="results-tbody"></tbody>
      </table>
    </div>
  </div>

</div>
<script>
const results = {results_json};
const baseline = results[0].pages_per_sec;

// Table
const tbody = document.getElementById('results-tbody');
tbody.innerHTML = results.map(r => `
  <tr>
    <td>${{r.workers}}</td>
    <td>${{r.pages_crawled}}</td>
    <td>${{r.total_time_secs.toFixed(2)}}s</td>
    <td style="color:#98c379">${{r.pages_per_sec.toFixed(2)}}</td>
    <td>${{r.avg_response_ms}}ms</td>
    <td style="color:#61afef">${{(r.pages_per_sec / baseline).toFixed(2)}}x</td>
  </tr>
`).join('');

// Pages/sec bar chart
const margin = {{top: 20, right: 30, bottom: 40, left: 60}};
const width  = document.getElementById('chart-bars').clientWidth - margin.left - margin.right || 700;
const height = 300 - margin.top - margin.bottom;

const svg1 = d3.select('#chart-bars')
  .append('svg')
  .attr('width', width + margin.left + margin.right)
  .attr('height', height + margin.top + margin.bottom)
  .append('g')
  .attr('transform', `translate(${{margin.left}},${{margin.top}})`);

const x1 = d3.scaleBand()
  .domain(results.map(r => r.workers + ' workers'))
  .range([0, width]).padding(0.3);

const y1 = d3.scaleLinear()
  .domain([0, d3.max(results, r => r.pages_per_sec) * 1.1])
  .range([height, 0]);

svg1.append('g').attr('transform', `translate(0,${{height}})`)
  .call(d3.axisBottom(x1))
  .selectAll('text').style('fill', '#888');

svg1.append('g').call(d3.axisLeft(y1))
  .selectAll('text').style('fill', '#888');

svg1.selectAll('.domain,.tick line').attr('stroke', '#2a2d3e');

svg1.selectAll('rect')
  .data(results)
  .join('rect')
  .attr('x', r => x1(r.workers + ' workers'))
  .attr('y', r => y1(r.pages_per_sec))
  .attr('width', x1.bandwidth())
  .attr('height', r => height - y1(r.pages_per_sec))
  .attr('fill', '#61afef')
  .attr('rx', 4);

svg1.selectAll('.label')
  .data(results)
  .join('text')
  .attr('class', 'label')
  .attr('x', r => x1(r.workers + ' workers') + x1.bandwidth() / 2)
  .attr('y', r => y1(r.pages_per_sec) - 6)
  .attr('text-anchor', 'middle')
  .attr('fill', '#e0e0e0')
  .attr('font-size', '12px')
  .text(r => r.pages_per_sec.toFixed(1));

// Time line chart
const svg2 = d3.select('#chart-time')
  .append('svg')
  .attr('width', width + margin.left + margin.right)
  .attr('height', height + margin.top + margin.bottom)
  .append('g')
  .attr('transform', `translate(${{margin.left}},${{margin.top}})`);

const x2 = d3.scaleLinear()
  .domain([0, d3.max(results, r => r.workers)])
  .range([0, width]);

const y2 = d3.scaleLinear()
  .domain([0, d3.max(results, r => r.total_time_secs) * 1.1])
  .range([height, 0]);

svg2.append('g').attr('transform', `translate(0,${{height}})`)
  .call(d3.axisBottom(x2).ticks(5))
  .selectAll('text').style('fill', '#888');

svg2.append('g').call(d3.axisLeft(y2))
  .selectAll('text').style('fill', '#888');

svg2.selectAll('.domain,.tick line').attr('stroke', '#2a2d3e');

const line = d3.line()
  .x(r => x2(r.workers))
  .y(r => y2(r.total_time_secs))
  .curve(d3.curveMonotoneX);

svg2.append('path')
  .datum(results)
  .attr('fill', 'none')
  .attr('stroke', '#e06c75')
  .attr('stroke-width', 2.5)
  .attr('d', line);

svg2.selectAll('circle')
  .data(results)
  .join('circle')
  .attr('cx', r => x2(r.workers))
  .attr('cy', r => y2(r.total_time_secs))
  .attr('r', 5)
  .attr('fill', '#e06c75');

svg2.selectAll('.time-label')
  .data(results)
  .join('text')
  .attr('class', 'time-label')
  .attr('x', r => x2(r.workers) + 8)
  .attr('y', r => y2(r.total_time_secs) - 8)
  .attr('fill', '#e0e0e0')
  .attr('font-size', '11px')
  .text(r => r.total_time_secs.toFixed(1) + 's');
</script>
</body>
</html>"#,
        url = url,
        speedup = speedup,
        max_pages_per_sec = max_pages_per_sec,
        results_len = results.len(),
        results_json = results_json,
    );

    fs::write(path, html).expect("Could not write benchmark report");
}