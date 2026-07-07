mod analytics;
mod benchmark;
mod config;
mod crawler;
mod dashboard;
mod exporter;
mod fetcher;
mod graph;
mod models;
mod parser;

use clap::Parser;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let args = config::Args::parse();

    // ── Benchmark mode ────────────────────────────────────────
    if args.benchmark {
        let results = benchmark::run_benchmark(
            &args.url,
            args.depth,
            args.max_pages,
        ).await;

        let json = serde_json::to_string_pretty(&results).unwrap();
        std::fs::write("benchmark_results.json", json)
            .expect("Could not write benchmark results");
        println!("\nBenchmark saved to: benchmark_results.json");

        dashboard::generate_benchmark_report(&results, &args.url, "benchmark.html");
        println!("Report saved to: benchmark.html");
        println!("Run: python3 -m http.server 8080");
        println!("Open: http://localhost:8080/benchmark.html");
        return;
    }

    // ── Normal crawl mode ─────────────────────────────────────
    println!("RustCrawler starting...");
    println!("  URL       : {}", args.url);
    println!("  Depth     : {}", args.depth);
    println!("  Max pages : {}", args.max_pages);
    println!("  Workers   : {}", args.workers);
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
    analytics::print_analytics(&records, &edges, &args.url);

    println!("\nExporting CSV...");
    exporter::export_csv(&records, &args.output);

    println!("\nExporting JSON...");
    exporter::export_json(&records, &edges, &args.json);

    println!("\nBuilding graph...");
    let g = graph::build_graph(&edges);
    println!("  Nodes : {}", g.node_count());
    println!("  Edges : {}", g.edge_count());
    graph::export_graph(&g, &args.graph);

    println!("\nGenerating HTML report...");
    dashboard::generate_report(
        &records,
        &edges,
        &args.url,
        total_time,
        &args.report,
    );
}