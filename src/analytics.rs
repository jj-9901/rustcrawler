use crate::models::{LinkEdge, PageRecord};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn print_summary(records: &[PageRecord], total_time_secs: f64) {
    let broken: Vec<_> = records.iter().filter(|r| r.status == "ERROR").collect();
    let successful: Vec<_> = records.iter().filter(|r| r.status != "ERROR").collect();

    let avg_response_ms = if successful.is_empty() {
        0
    } else {
        successful.iter().map(|r| r.response_time_ms).sum::<u128>()
            / successful.len() as u128
    };

    println!();
    println!("========== CRAWL SUMMARY ==========");
    println!("  Pages crawled     : {}", records.len());
    println!("  Successful        : {}", successful.len());
    println!("  Broken links      : {}", broken.len());
    println!("  Avg response time : {} ms", avg_response_ms);
    println!("  Total time        : {:.2}s", total_time_secs);
    println!("===================================");
}

pub fn print_analytics(records: &[PageRecord], edges: &[LinkEdge], start_url: &str) {
    println!();
    println!("========== SITE ANALYTICS ==========");

    let start_host = extract_host(start_url);
    let mut internal = 0;
    let mut external = 0;
    let mut external_domains: HashMap<String, usize> = HashMap::new();

    for edge in edges {
        let host = extract_host(&edge.to);
        if host == start_host {
            internal += 1;
        } else {
            external += 1;
            *external_domains.entry(host).or_insert(0) += 1;
        }
    }

    println!("  Internal links    : {}", internal);
    println!("  External links    : {}", external);

    // Dead-end pages
    let dead_ends = find_dead_ends(records, edges);
    println!("  Dead-end pages    : {}", dead_ends.len());

    // Graph density
    let density = compute_density(records.len(), edges.len());
    println!("  Graph density     : {:.4}", density);

    // Connected components
    let components = count_connected_components(records, edges);
    println!("  Connected components: {}", components);

    // Most linked pages
    let mut inbound: HashMap<String, usize> = HashMap::new();
    for edge in edges {
        *inbound.entry(edge.to.clone()).or_insert(0) += 1;
    }
    let mut inbound_sorted: Vec<_> = inbound.iter().collect();
    inbound_sorted.sort_by(|a, b| b.1.cmp(a.1));

    println!();
    println!("  Top 5 most linked pages:");
    for (url, count) in inbound_sorted.iter().take(5) {
        println!("    {} ← {} links", shorten(url, 60), count);
    }

    // Slowest pages
    let mut sorted_by_time: Vec<_> = records
        .iter()
        .filter(|r| r.status != "ERROR")
        .collect();
    sorted_by_time.sort_by(|a, b| b.response_time_ms.cmp(&a.response_time_ms));

    println!();
    println!("  Top 5 slowest pages:");
    for record in sorted_by_time.iter().take(5) {
        println!(
            "    {}ms  {}",
            record.response_time_ms,
            shorten(&record.url, 55)
        );
    }

    // Orphans
    let linked_urls: HashSet<_> = edges.iter().map(|e| &e.to).collect();
    let orphans: Vec<_> = records
        .iter()
        .filter(|r| !linked_urls.contains(&r.url) && r.url != start_url)
        .collect();

    println!();
    println!(
        "  Orphan pages (not linked by anyone): {}",
        orphans.len()
    );
    for o in orphans.iter().take(5) {
        println!("    {}", shorten(&o.url, 70));
    }
    if orphans.len() > 5 {
        println!("    ... and {} more", orphans.len() - 5);
    }

    // Duplicates
    let duplicates = records.iter().filter(|r| r.is_duplicate).count();
    println!();
    println!("  Duplicate pages   : {}", duplicates);

    // Redirect chains
    let redirected: Vec<_> = records.iter().filter(|r| r.redirect_count > 0).collect();
    println!("  Pages with redirects: {}", redirected.len());
    for r in redirected.iter().take(3) {
        println!(
            "    {} → {} hops",
            shorten(&r.url, 50),
            r.redirect_chain.len()
        );
    }

    // Avg links per page
    let total_links: usize = records.iter().map(|r| r.links_found).sum();
    let avg_links = if records.is_empty() {
        0
    } else {
        total_links / records.len()
    };
    println!();
    println!("  Avg links per page: {}", avg_links);

    // External domains
    if !external_domains.is_empty() {
        let mut ext_sorted: Vec<_> = external_domains.iter().collect();
        ext_sorted.sort_by(|a, b| b.1.cmp(a.1));
        println!();
        println!("  Top external domains:");
        for (domain, count) in ext_sorted.iter().take(5) {
            println!("    {} ({} links)", domain, count);
        }
    }

    println!("=====================================");
}

// Dead-end pages: pages with no outbound links to other crawled pages
pub fn find_dead_ends(records: &[PageRecord], edges: &[LinkEdge]) -> Vec<String> {
    let crawled: HashSet<&str> = records.iter().map(|r| r.url.as_str()).collect();
    let has_outbound: HashSet<&str> = edges
        .iter()
        .filter(|e| crawled.contains(e.to.as_str()))
        .map(|e| e.from.as_str())
        .collect();

    records
        .iter()
        .filter(|r| !has_outbound.contains(r.url.as_str()))
        .map(|r| r.url.clone())
        .collect()
}

// Graph density: ratio of actual edges to possible edges
pub fn compute_density(node_count: usize, edge_count: usize) -> f64 {
    if node_count <= 1 {
        return 0.0;
    }
    let max_edges = node_count * (node_count - 1);
    edge_count as f64 / max_edges as f64
}

// Count weakly connected components using BFS
pub fn count_connected_components(records: &[PageRecord], edges: &[LinkEdge]) -> usize {
    if records.is_empty() {
        return 0;
    }

    // Build adjacency list (undirected for weak connectivity)
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for record in records {
        adj.entry(record.url.as_str()).or_default();
    }
    for edge in edges {
        adj.entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
        adj.entry(edge.to.as_str())
            .or_default()
            .push(edge.from.as_str());
    }

    let mut visited: HashSet<&str> = HashSet::new();
    let mut components = 0;

    for record in records {
        let url = record.url.as_str();
        if visited.contains(url) {
            continue;
        }

        // BFS from this node
        components += 1;
        let mut queue = VecDeque::new();
        queue.push_back(url);
        visited.insert(url);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = adj.get(current) {
                for &neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
    }

    components
}

// PageRank
pub fn compute_pagerank(
    records: &[PageRecord],
    edges: &[LinkEdge],
    iterations: u32,
    damping: f64,
) -> HashMap<String, f64> {
    let n = records.len();
    if n == 0 {
        return HashMap::new();
    }

    let urls: Vec<String> = records.iter().map(|r| r.url.clone()).collect();
    let url_index: HashMap<String, usize> = urls
        .iter()
        .enumerate()
        .map(|(i, u)| (u.clone(), i))
        .collect();

    let mut out_degree = vec![0usize; n];
    let mut inbound: Vec<Vec<usize>> = vec![vec![]; n];

    for edge in edges {
        if let (Some(&fi), Some(&ti)) =
            (url_index.get(&edge.from), url_index.get(&edge.to))
        {
            if fi != ti {
                out_degree[fi] += 1;
                inbound[ti].push(fi);
            }
        }
    }

    let mut rank = vec![1.0 / n as f64; n];

    for _ in 0..iterations {
        let mut new_rank = vec![(1.0 - damping) / n as f64; n];
        for i in 0..n {
            for &from in &inbound[i] {
                if out_degree[from] > 0 {
                    new_rank[i] += damping * rank[from] / out_degree[from] as f64;
                }
            }
        }
        rank = new_rank;
    }

    urls.into_iter()
        .enumerate()
        .map(|(i, url)| (url, rank[i]))
        .collect()
}

fn extract_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
}

fn shorten(url: &str, max: usize) -> String {
    if url.len() <= max {
        url.to_string()
    } else {
        format!("{}...", &url[..max])
    }
}