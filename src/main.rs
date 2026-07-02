use clap::Parser;
use scraper::{Html, Selector};
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
        .build()?;

    let response = client.get(url).send()?;

    println!("  Status : {}", response.status());

    let body = response.text()?;
    Ok(body)
}

fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);

    // CSS selector that finds all <a> tags
    let selector = Selector::parse("a[href]").unwrap();

    // Parse the base URL so we can resolve relative links
    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };

    let mut links = Vec::new();

    for element in document.select(&selector) {
        // Get the href attribute value
        let href = match element.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        // Skip anchors, mailto, javascript links
        if href.starts_with('#')
            || href.starts_with("mailto:")
            || href.starts_with("javascript:")
        {
            continue;
        }

        // Resolve relative URLs like /about → https://example.com/about
        let full_url = match base.join(href) {
            Ok(u) => u,
            Err(_) => continue,
        };

        // Only keep http and https links
        if full_url.scheme() != "http" && full_url.scheme() != "https" {
            continue;
        }

        links.push(full_url.to_string());
    }

    links
}

fn main() {
    let args = Args::parse();

    println!("RustCrawler starting...");
    println!("  URL       : {}", args.url);
    println!("  Depth     : {}", args.depth);
    println!("  Max pages : {}", args.max_pages);
    println!();

    println!("Fetching page...");
    match fetch_page(&args.url) {
        Ok(body) => {
            println!("  Got {} bytes", body.len());
            println!();

            println!("Extracting links...");
            let links = extract_links(&body, &args.url);
            println!("  Found {} links", links.len());
            println!();

            for link in &links {
                println!("  {}", link);
            }
        }
        Err(e) => {
            println!("  Error fetching page: {}", e);
        }
    }
}