use clap::Parser;

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
            println!("--- First 500 characters ---");
            println!("{}", &body[..body.len().min(500)]);
        }
        Err(e) => {
            println!("  Error fetching page: {}", e);
        }
    }
}