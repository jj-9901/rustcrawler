# 🕷 RustCrawler — Website Analysis Platform

A high-performance, multi-threaded web crawler and website analysis platform built in Rust. RustCrawler crawls websites concurrently, analyzes their structure, and generates an interactive dashboard with graph visualization, analytics, and performance benchmarks.

---

## Features

- **Concurrent crawling** — async Tokio-based crawler with configurable worker count
- **Link extraction** — internal and external link detection with redirect tracking
- **PageRank** — iterative PageRank algorithm on the crawled graph
- **Graph visualization** — interactive D3.js force-directed graph with zoom, drag, depth coloring, neighbor highlighting, and node search
- **Website health score** — broken links, slow pages, orphan pages, duplicate detection
- **Analytics** — dead-end pages, graph density, connected components, response time distribution
- **Benchmarking** — compare crawling efficiency across worker counts with D3 charts
- **Exports** — CSV, JSON, DOT (Graphviz), and interactive HTML report
- **robots.txt compliance** — respects crawl rules per site

---

## Project Structure

```
src/
├── main.rs          # Entry point and orchestration
├── config.rs        # CLI argument definitions
├── models.rs        # PageRecord and LinkEdge structs
├── fetcher.rs       # Async HTTP fetching with redirect tracking
├── parser.rs        # HTML parsing and link extraction
├── crawler.rs       # BFS crawl engine with async workers
├── analytics.rs     # PageRank, dead-ends, components, statistics
├── graph.rs         # Petgraph integration and DOT export
├── exporter.rs      # CSV and JSON export
├── dashboard.rs     # HTML report generation
└── benchmark.rs     # Worker count benchmarking

dashboard/
├── index.html       # Report template
├── style.css        # Styles
├── graph.js         # D3.js graph rendering
└── main.js          # Dashboard logic and charts
```

---

## Installation

**Prerequisites:** Rust (1.75+), Cargo

```bash
git clone https://github.com/YOUR_USERNAME/rustcrawler
cd rustcrawler
cargo build --release
```

---

## Usage

### Basic crawl

```bash
cargo run -- --url https://example.com --depth 2 --max-pages 50 --workers 5
```

### View the dashboard

```bash
python3 -m http.server 8080
# Open http://localhost:8080/report.html
```

### Run benchmark

```bash
cargo run -- --url https://example.com --depth 1 --max-pages 20 --benchmark
# Open http://localhost:8080/benchmark.html
```

### All options

| Flag | Default | Description |
|------|---------|-------------|
| `--url` | required | Starting URL to crawl |
| `--depth` | 2 | Maximum crawl depth |
| `--max-pages` | 50 | Maximum pages to visit |
| `--workers` | 5 | Concurrent worker count |
| `--output` | crawl_results.csv | CSV output path |
| `--json` | crawl_results.json | JSON output path |
| `--graph` | graph.dot | DOT graph output path |
| `--report` | report.html | HTML report output path |
| `--benchmark` | false | Run benchmark mode |

---

## Dashboard

The interactive HTML report includes:

- **Stat cards** — pages crawled, broken links, avg response time, health score, dead ends, graph density, connected components
- **Link graph** — force-directed graph with zoom, pan, drag, depth coloring, neighbor highlighting, node search, and click-to-sidebar
- **Page sidebar** — detailed info for each page including title, status, depth, response time, size, PageRank, and redirect count
- **Analytics** — response time bars, external domain breakdown, link type distribution
- **PageRank table** — top pages ranked by importance
- **Pages table** — searchable table of all crawled pages with clickable rows

---

## Benchmarking

The benchmark mode runs the crawler 5 times with 1, 2, 4, 8, and 16 workers and measures:

- Total crawl time
- Pages per second
- Average response time
- Speedup vs single worker

Results are displayed as a bar chart (pages/sec) and line chart (time vs workers) in `benchmark.html`.

---

## Technologies

| Technology | Purpose |
|-----------|---------|
| Rust | Systems programming language |
| Tokio | Async runtime |
| Reqwest | HTTP client |
| Scraper | HTML parsing |
| Petgraph | Graph data structure |
| Serde | Serialization |
| Clap | CLI argument parsing |
| D3.js | Interactive graph visualization |

---

## Example Output

```
Starting crawl from: https://books.toscrape.com
Max depth: 2  |  Max pages: 50  |  Workers: 5

[Depth 0] Fetching 1 URLs...
  [200 OK] https://books.toscrape.com (1075ms, 94 links)

[Depth 1] Fetching 25 URLs...
  [200 OK] https://books.toscrape.com/catalogue/category/books/travel_2/index.html (838ms, 76 links)

[Depth 2] Fetching 24 URLs...
  [200 OK] https://books.toscrape.com/catalogue/a-light-in-the-attic_1000/index.html (912ms, 0 links)

========== CRAWL SUMMARY ==========
  Pages crawled     : 50
  Successful        : 50
  Broken links      : 0
  Avg response time : 980 ms
  Total time        : 12.4s
===================================

========== SITE ANALYTICS ==========
  Internal links    : 2847
  External links    : 0
  Dead-end pages    : 24
  Graph density     : 1.12
  Connected components: 1
  Duplicate pages   : 2
  Pages with redirects: 0
  Avg links per page: 56
=====================================
```

---

## Graph Visualization

The D3.js force-directed graph supports:

- **Scroll** to zoom in and out
- **Drag** to pan the canvas
- **Drag nodes** to reposition them
- **Hover** to highlight a node and its neighbors
- **Click** a node to open the page details sidebar
- **Color by Depth** button to visualize crawl depth levels
- **Search** bar to highlight nodes by URL

Node colors (default mode):
- 🔴 Red — start page
- 🟣 Purple — hub pages (3+ inbound links)
- 🔵 Blue — regular pages
- ⚫ Grey — linked but not crawled

---

## robots.txt Compliance

Before fetching any URL, RustCrawler checks the site's `robots.txt` and skips disallowed paths. The robots.txt is fetched once per domain and cached for the duration of the crawl.

```
[ROBOTS] Skipped: https://example.com/admin/
```

---

## Academic Context

Built as a semester project for a Systems Programming course demonstrating:

- Rust ownership, borrowing, and lifetimes
- Async programming with Tokio
- Safe concurrency with Arc and Mutex
- Graph algorithms (BFS traversal, PageRank)
- HTTP networking and redirect handling
- HTML parsing with CSS selectors
- Data visualization with D3.js
- Modular software architecture

---

## Team Members

Jahanvi

## Course

B.Tech Information Technology — Systems Programming