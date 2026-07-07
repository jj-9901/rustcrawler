use crate::crawler::crawl;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct BenchmarkResult {
    pub workers: usize,
    pub pages_crawled: usize,
    pub total_time_secs: f64,
    pub pages_per_sec: f64,
    pub avg_response_ms: u128,
}

pub async fn run_benchmark(url: &str, depth: u32, pages: u32) -> Vec<BenchmarkResult> {
    let worker_counts = vec![1, 2, 4, 8, 16];
    let mut results = vec![];

    println!();
    println!("========== BENCHMARKING ==========");
    println!("  URL    : {}", url);
    println!("  Depth  : {}", depth);
    println!("  Pages  : {}", pages);
    println!("  Runs   : {:?}", worker_counts);
    println!("==================================");

    for workers in worker_counts {
        println!("\n[Benchmark] Running with {} worker(s)...", workers);

        let start = std::time::Instant::now();
        let (records, _edges) = crawl(url, depth, pages, workers).await;
        let elapsed = start.elapsed().as_secs_f64();

        let pages_crawled = records.len();
        let pages_per_sec = if elapsed > 0.0 {
            pages_crawled as f64 / elapsed
        } else {
            0.0
        };

        let avg_response_ms = if pages_crawled == 0 {
            0
        } else {
            records.iter().map(|r| r.response_time_ms).sum::<u128>()
                / pages_crawled as u128
        };

        println!(
            "  ✓ {} workers | {} pages | {:.2}s | {:.2} pages/sec | {}ms avg",
            workers, pages_crawled, elapsed, pages_per_sec, avg_response_ms
        );

        results.push(BenchmarkResult {
            workers,
            pages_crawled,
            total_time_secs: elapsed,
            pages_per_sec,
            avg_response_ms,
        });
    }

    println!();
    println!("========== BENCHMARK RESULTS ==========");
    println!(
        "{:<10} {:<15} {:<12} {:<15} {:<12}",
        "Workers", "Pages", "Time (s)", "Pages/sec", "Avg ms"
    );
    println!("{}", "-".repeat(64));
    for r in &results {
        println!(
            "{:<10} {:<15} {:<12.2} {:<15.2} {:<12}",
            r.workers, r.pages_crawled, r.total_time_secs,
            r.pages_per_sec, r.avg_response_ms
        );
    }
    println!("=======================================");

    results
}