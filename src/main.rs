mod analytics;
mod crawler;
mod exporter;
mod fetcher;
mod graph;
mod models;
mod parser;

use clap::Parser;
use std::time::Instant;

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

    /// Output JSON file path
    #[arg(short, long, default_value = "crawl_results.json")]
    json: String,

    /// Output graph DOT file path
    #[arg(short, long, default_value = "graph.dot")]
    graph: String,
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
    println!("  JSON      : {}", args.json);
    println!("  Graph     : {}", args.graph);
    println!();

    let start = Instant::now();
    let (records, edges) = crawler::crawl(
        &args.url,
        args.depth,
        args.max_pages,
        args.workers,
    ).await;
    let total_time = start.elapsed().as_secs_f64();

    analytics::print_summary(&records, total_time);

    println!("\nExporting CSV...");
    exporter::export_csv(&records, &args.output);

    println!("\nExporting JSON...");
    exporter::export_json(&records, &edges, &args.json);

    println!("\nBuilding graph...");
    let g = graph::build_graph(&edges);
    println!("  Nodes : {}", g.node_count());
    println!("  Edges : {}", g.edge_count());
    graph::export_graph(&g, &args.graph);
}