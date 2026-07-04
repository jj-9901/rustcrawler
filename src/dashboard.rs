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
            r#"{{"url":"{}","status":"{}","depth":{},"response_time_ms":{},"links_found":{},"pagerank":{:.4},"title":"{}","size_bytes":{}}}"#,
            escape_json(&r.url), r.status, r.depth, r.response_time_ms,
            r.links_found, pr, escape_json(&r.title), r.size_bytes
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
    "connected_components": {}
    }}"#,
        records.len(), successful, broken, avg_ms,
        edges.len(), node_set.len(), total_time_secs,
        internal, external, health_score,
        health_issues_json, ext_domains_json,
        dead_ends.len(), density, components
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