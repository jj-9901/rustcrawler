use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rustcrawler")]
#[command(about = "Crawls a website and extracts links", long_about = None)]
pub struct Args {
    /// The starting URL to crawl
    #[arg(short, long)]
    pub url: String,

    /// Maximum depth to crawl
    #[arg(short, long, default_value_t = 2)]
    pub depth: u32,

    /// Maximum number of pages to visit
    #[arg(short, long, default_value_t = 50)]
    pub max_pages: u32,

    /// Number of concurrent workers
    #[arg(short, long, default_value_t = 5)]
    pub workers: usize,

    /// Output CSV file path
    #[arg(short, long, default_value = "crawl_results.csv")]
    pub output: String,

    /// Output JSON file path
    #[arg(short, long, default_value = "crawl_results.json")]
    pub json: String,

    /// Output graph DOT file path
    #[arg(short, long, default_value = "graph.dot")]
    pub graph: String,

    /// Output HTML report path
    #[arg(short, long, default_value = "report.html")]
    pub report: String,

    /// Run benchmark mode comparing different worker counts
    #[arg(short, long, default_value_t = false)]
    pub benchmark: bool,
}