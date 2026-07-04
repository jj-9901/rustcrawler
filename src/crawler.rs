use crate::fetcher::fetch_page;
use crate::models::{LinkEdge, PageRecord};
use crate::parser::{extract_links, extract_title};
use futures::future::join_all;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn crawl(
    start_url: &str,
    max_depth: u32,
    max_pages: u32,
    workers: usize,
) -> (Vec<PageRecord>, Vec<LinkEdge>) {
    let client = reqwest::Client::builder()
        .user_agent("RustCrawler/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    let visited: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let pages_crawled: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    let records: Arc<Mutex<Vec<PageRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let edges: Arc<Mutex<Vec<LinkEdge>>> = Arc::new(Mutex::new(Vec::new()));

    let mut current_frontier = vec![start_url.to_string()];
    visited.lock().await.insert(start_url.to_string());

    println!("Starting crawl from: {}", start_url);
    println!(
        "Max depth: {}  |  Max pages: {}  |  Workers: {}",
        max_depth, max_pages, workers
    );
    println!("{}", "-".repeat(60));

    for depth in 0..=max_depth {
        if current_frontier.is_empty() {
            break;
        }
        if *pages_crawled.lock().await >= max_pages {
            break;
        }

        println!("\n[Depth {}] Fetching {} URLs...", depth, current_frontier.len());

        let next_frontier: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        for chunk in current_frontier.chunks(workers) {
            let tasks: Vec<_> = chunk.iter().map(|url| {
                let client = client.clone();
                let url = url.clone();
                let visited = Arc::clone(&visited);
                let next_frontier = Arc::clone(&next_frontier);
                let pages_crawled = Arc::clone(&pages_crawled);
                let records = Arc::clone(&records);
                let edges = Arc::clone(&edges);

                tokio::spawn(async move {
                    if *pages_crawled.lock().await >= max_pages {
                        return;
                    }

                    match fetch_page(&client, &url).await {
                        Ok((body, status, elapsed_ms)) => {
                            *pages_crawled.lock().await += 1;

                            let links = if depth < max_depth {
                                extract_links(&body, &url)
                            } else {
                                vec![]
                            };

                            let links_found = links.len();
                            let title = extract_title(&body);
                            let size_bytes = body.len();

                            println!(
                                "  [{}] {} ({}ms, {} links)",
                                status, url, elapsed_ms, links_found
                            );

                            records.lock().await.push(PageRecord {
                                url: url.clone(),
                                status,
                                depth,
                                links_found,
                                response_time_ms: elapsed_ms,
                                title,
                                size_bytes, 
                            });

                            if depth < max_depth {
                                let mut visited = visited.lock().await;
                                let mut next = next_frontier.lock().await;
                                let mut edges = edges.lock().await;

                                for link in links {
                                    edges.push(LinkEdge {
                                        from: url.clone(),
                                        to: link.clone(),
                                    });
                                    if !visited.contains(&link) {
                                        visited.insert(link.clone());
                                        next.push(link);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("  [ERROR] {} — {}", url, e);
                            records.lock().await.push(PageRecord {
                                url: url.clone(),
                                status: "ERROR".to_string(),
                                depth,
                                links_found: 0,
                                response_time_ms: 0,
                                title: "Error".to_string(),
                                size_bytes: 0, 
                            });
                        }
                    }
                })
            }).collect();

            join_all(tasks).await;
        }

        current_frontier = Arc::try_unwrap(next_frontier).unwrap().into_inner();
    }

    println!("\n{}", "-".repeat(60));

    let records = Arc::try_unwrap(records).unwrap().into_inner();
    let edges = Arc::try_unwrap(edges).unwrap().into_inner();
    (records, edges)
}