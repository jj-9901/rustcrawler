use clap::Parser;
use futures::future::join_all;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::sync::Arc;
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
}

// async fn: this function can be suspended while waiting for network
async fn fetch_page(client: &reqwest::Client, url: &str) -> Result<String, reqwest::Error> {
    let response = client.get(url).send().await?;  // .await = "pause here, let others run"
    println!("  [{}] {}", response.status(), url);
    let body = response.text().await?;
    Ok(body)
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

        if full_url.scheme() != "http" && full_url.scheme() != "https" {
            continue;
        }

        links.push(full_url.to_string());
    }

    links
}

async fn crawl(start_url: &str, max_depth: u32, max_pages: u32, workers: usize) {
    let client = reqwest::Client::builder()
        .user_agent("RustCrawler/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    // Arc = shared ownership, Mutex = only one task modifies at a time
    let visited: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let pages_crawled: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    let broken_links: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    // Current frontier = URLs to fetch at the current depth level
    let mut current_frontier = vec![start_url.to_string()];
    visited.lock().await.insert(start_url.to_string());

    println!("Starting crawl from: {}", start_url);
    println!("Max depth: {}  |  Max pages: {}  |  Workers: {}", max_depth, max_pages, workers);
    println!("{}", "-".repeat(60));

    for depth in 0..=max_depth {
        if current_frontier.is_empty() {
            break;
        }

        // Check if we've already hit the page limit
        if *pages_crawled.lock().await >= max_pages {
            break;
        }

        println!("\n[Depth {}] Fetching {} URLs concurrently...", depth, current_frontier.len());

        // Next depth's URLs collected here
        let next_frontier: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Process current frontier in chunks of `workers` size
        for chunk in current_frontier.chunks(workers) {
            // Build a list of async tasks, one per URL in this chunk
            let tasks: Vec<_> = chunk.iter().map(|url| {
                let client = client.clone();
                let url = url.clone();
                let visited = Arc::clone(&visited);
                let next_frontier = Arc::clone(&next_frontier);
                let pages_crawled = Arc::clone(&pages_crawled);
                let broken_links = Arc::clone(&broken_links);

                // Spawn each URL fetch as an independent async task
                tokio::spawn(async move {
                    // Stop if page limit hit
                    if *pages_crawled.lock().await >= max_pages {
                        return;
                    }

                    match fetch_page(&client, &url).await {
                        Ok(body) => {
                            *pages_crawled.lock().await += 1;

                            if depth < max_depth {
                                let links = extract_links(&body, &url);
                                let mut visited = visited.lock().await;
                                let mut next = next_frontier.lock().await;

                                for link in links {
                                    if !visited.contains(&link) {
                                        visited.insert(link.clone());
                                        next.push(link);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            *broken_links.lock().await += 1;
                            println!("  [ERROR] {} — {}", url, e);
                        }
                    }
                })
            }).collect();

            // Wait for all tasks in this chunk to finish before next chunk
            join_all(tasks).await;
        }

        // Move to next depth
        current_frontier = Arc::try_unwrap(next_frontier)
            .unwrap()
            .into_inner();
    }

    println!("\n{}", "-".repeat(60));
    println!("Crawl complete.");
    println!("  Pages crawled : {}", *pages_crawled.lock().await);
    println!("  Broken links  : {}", *broken_links.lock().await);
    println!("  URLs found    : {}", visited.lock().await.len());
}

// #[tokio::main] turns main() into an async entry point
#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("RustCrawler starting...");
    println!("  URL       : {}", args.url);
    println!("  Depth     : {}", args.depth);
    println!("  Max pages : {}", args.max_pages);
    println!("  Workers   : {}", args.workers);
    println!();

    crawl(&args.url, args.depth, args.max_pages, args.workers).await;
}