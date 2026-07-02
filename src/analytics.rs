use crate::models::{LinkEdge, PageRecord};
use std::collections::HashMap;

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

    // --- Internal vs External links ---
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

    // --- Most linked-to pages (inbound links) ---
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

    // --- Slowest pages ---
    let mut sorted_by_time: Vec<_> = records
        .iter()
        .filter(|r| r.status != "ERROR")
        .collect();
    sorted_by_time.sort_by(|a, b| b.response_time_ms.cmp(&a.response_time_ms));

    println!();
    println!("  Top 5 slowest pages:");
    for record in sorted_by_time.iter().take(5) {
        println!("    {}ms  {}", record.response_time_ms, shorten(&record.url, 55));
    }

    // --- Orphan pages (crawled but nobody links to them) ---
    let linked_urls: std::collections::HashSet<_> = edges.iter().map(|e| &e.to).collect();
    let orphans: Vec<_> = records
        .iter()
        .filter(|r| !linked_urls.contains(&r.url) && r.url != start_url)
        .collect();

    println!();
    println!("  Orphan pages (not linked by anyone): {}", orphans.len());
    for o in orphans.iter().take(5) {
        println!("    {}", shorten(&o.url, 70));
    }
    if orphans.len() > 5 {
        println!("    ... and {} more", orphans.len() - 5);
    }

    // --- Average links per page ---
    let total_links: usize = records.iter().map(|r| r.links_found).sum();
    let avg_links = if records.is_empty() {
        0
    } else {
        total_links / records.len()
    };
    println!();
    println!("  Avg links per page: {}", avg_links);

    // --- External domains ---
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

// Pull just the hostname out of a URL
fn extract_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
}

// Truncate long URLs for display
fn shorten(url: &str, max: usize) -> String {
    if url.len() <= max {
        url.to_string()
    } else {
        format!("{}...", &url[..max])
    }
}