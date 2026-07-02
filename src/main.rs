use clap::Parser;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
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
}

fn fetch_page(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("RustCrawler/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(url).send()?;
    println!("  [{}] {}", response.status(), url);

    let body = response.text()?;
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

fn crawl(start_url: &str, max_depth: u32, max_pages: u32) {
    // Queue holds (url, depth) pairs
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();

    // Track visited URLs so we never fetch the same page twice
    let mut visited: HashSet<String> = HashSet::new();

    let mut pages_crawled = 0;
    let mut broken_links = 0;

    // Seed the queue with the starting URL at depth 0
    queue.push_back((start_url.to_string(), 0));
    visited.insert(start_url.to_string());

    println!("Starting crawl from: {}", start_url);
    println!("Max depth: {}  |  Max pages: {}", max_depth, max_pages);
    println!("{}", "-".repeat(60));

    while let Some((url, depth)) = queue.pop_front() {
        // Stop if we've hit the page limit
        if pages_crawled >= max_pages {
            println!("\nReached max pages limit ({}).", max_pages);
            break;
        }

        // Stop going deeper if we've hit the depth limit
        if depth > max_depth {
            continue;
        }

        // Fetch the page
        match fetch_page(&url) {
            Ok(body) => {
                pages_crawled += 1;

                // Only extract links if we haven't hit max depth yet
                if depth < max_depth {
                    let links = extract_links(&body, &url);

                    for link in links {
                        // Only add links we haven't seen yet
                        if !visited.contains(&link) {
                            visited.insert(link.clone());
                            queue.push_back((link, depth + 1));
                        }
                    }
                }
            }
            Err(e) => {
                broken_links += 1;
                println!("  [ERROR] {} — {}", url, e);
            }
        }
    }

    println!("{}", "-".repeat(60));
    println!("Crawl complete.");
    println!("  Pages crawled : {}", pages_crawled);
    println!("  Broken links  : {}", broken_links);
    println!("  URLs found    : {}", visited.len());
}

fn main() {
    let args = Args::parse();

    println!("RustCrawler starting...");
    println!("  URL       : {}", args.url);
    println!("  Depth     : {}", args.depth);
    println!("  Max pages : {}", args.max_pages);
    println!();

    crawl(&args.url, args.depth, args.max_pages);
}