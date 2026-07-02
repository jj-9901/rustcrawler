use clap::Parser;
use futures::future::join_all;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use url::Url;

/// RustCrawler - A web crawler written in Rust
#[derive(Parser, Debug)]
#[command(name = "rustcrawler")]
#[command(about = "Crawls a website and extracts links", long_about = None)]
struct Args {
    /// The starting URL to crawl
    #[arg(short, long)]
    url: String,

    /// Maximum depth to crawl
    #[arg(short, long, default_value_t = 2)]
    depth: u32,

    /// Maximum number of pages to visit
    #[arg(short, long, default_value_t = 50)]
    max_pages: u32,

    /// Number of concurrent workers
    #[arg(short, long, default_value_t = 5)]
    workers: usize,

    /// Output CSV file path
    #[arg(short, long, default_value = "crawl_results.csv")]
    output: String,

    /// Output graph DOT file path
    #[arg(short, long, default_value = "graph.dot")]
    graph: String,
}

#[derive(Debug, Serialize, Clone)]
struct PageRecord {
    url: String,
    status: String,
    depth: u32,
    links_found: usize,
    response_time_ms: u128,
}

// Stores an edge: page A links to page B
#[derive(Debug, Clone)]
struct LinkEdge {
    from: String,
    to: String,
}

async fn fetch_page(
    client: &reqwest::Client,
    url: &str,
) -> Result<(String, String, u128), reqwest::Error> {
    let start = Instant::now();
    let response = client.get(url).send().await?;
    let status = response.status().to_string();
    let body = response.text().await?;
    let elapsed = start.elapsed().as_millis();
    Ok((body, status, elapsed))
}

fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();

    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };

    let mut links = Vec::new();

    for element in document.select(&selector) {
        let href = match element.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        if href.starts_with('#')
            || href.starts_with("mailto:")
            || href.starts_with("javascript:")
        {
            continue;
        }

        let full_url = match base.join(href) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let full_url = &mut full_url.clone();
        full_url.set_fragment(None);

        if full_url.scheme() != "http" && full_url.scheme() != "https" {
            continue;
        }

        links.push(full_url.to_string())
    }

    links
}

fn build_graph(edges: &[LinkEdge]) -> DiGraph<String, ()> {
    let mut graph = DiGraph::new();
    let mut node_map: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();
    // Track seen (from, to) pairs to avoid duplicate edges
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();

    for edge in edges {
        let edge_key = (edge.from.clone(), edge.to.clone());
        if seen_edges.contains(&edge_key) {
            continue;  // skip duplicate
        }
        seen_edges.insert(edge_key);

        let from_idx = if let Some(&idx) = node_map.get(&edge.from) {
            idx
        } else {
            let idx = graph.add_node(edge.from.clone());
            node_map.insert(edge.from.clone(), idx);
            idx
        };

        let to_idx = if let Some(&idx) = node_map.get(&edge.to) {
            idx
        } else {
            let idx = graph.add_node(edge.to.clone());
            node_map.insert(edge.to.clone(), idx);
            idx
        };

        graph.add_edge(from_idx, to_idx, ());
    }

    graph
}

fn export_graph(graph: &DiGraph<String, ()>, path: &str) {
    // DOT format is a standard graph description language
    // readable by Graphviz and many online viewers
    let dot = format!("{:?}", Dot::with_config(graph, &[Config::EdgeNoLabel]));
    fs::write(path, dot).expect("Could not write graph file");
    println!("  Saved to: {}", path);
    println!("  View at : https://dreampuf.github.io/GraphvizOnline");
}

fn export_csv(records: &[PageRecord], path: &str) {
    let mut writer = csv::Writer::from_path(path).expect("Could not create CSV file");
    for record in records {
        writer.serialize(record).expect("Could not write record");
    }
    writer.flush().expect("Could not flush CSV");
    println!("  Saved to: {}", path);
}

fn print_summary(records: &[PageRecord], total_time_secs: f64) {
    let broken: Vec<_> = records.iter().filter(|r| r.status == "ERROR").collect();
    let successful: Vec<_> = records.iter().filter(|r| r.status != "ERROR").collect();

    let avg_response_ms = if successful.is_empty() {
        0
    } else {
        successful.iter().map(|r| r.response_time_ms).sum::<u128>() / successful.len() as u128
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

async fn crawl(
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

    // Collect all edges (from → to) across the entire crawl
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

        println!(
            "\n[Depth {}] Fetching {} URLs...",
            depth,
            current_frontier.len()
        );

        let next_frontier: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        for chunk in current_frontier.chunks(workers) {
            let tasks: Vec<_> = chunk
                .iter()
                .map(|url| {
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
                                });

                                if depth < max_depth {
                                    let mut visited = visited.lock().await;
                                    let mut next = next_frontier.lock().await;
                                    let mut edges = edges.lock().await;

                                    for link in links {
                                        // Record this edge regardless of visited status
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
                                });
                            }
                        }
                    })
                })
                .collect();

            join_all(tasks).await;
        }

        current_frontier = Arc::try_unwrap(next_frontier).unwrap().into_inner();
    }

    println!("\n{}", "-".repeat(60));

    let records = Arc::try_unwrap(records).unwrap().into_inner();
    let edges = Arc::try_unwrap(edges).unwrap().into_inner();
    (records, edges)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("RustCrawler starting...");
    println!("  URL       : {}", args.url);
    println!("  Depth     : {}", args.depth);
    println!("  Max pages : {}", args.max_pages);
    println!("  Workers   : {}", args.workers);
    println!("  Output    : {}", args.output);
    println!("  Graph     : {}", args.graph);
    println!();

    let start = Instant::now();
    let (records, edges) = crawl(&args.url, args.depth, args.max_pages, args.workers).await;
    let total_time = start.elapsed().as_secs_f64();

    print_summary(&records, total_time);

    println!("\nExporting CSV...");
    export_csv(&records, &args.output);

    println!("\nBuilding graph...");
    let graph = build_graph(&edges);
    println!("  Nodes : {}", graph.node_count());
    println!("  Edges : {}", graph.edge_count());
    export_graph(&graph, &args.graph);
}