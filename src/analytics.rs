use crate::models::PageRecord;

pub fn print_summary(records: &[PageRecord], total_time_secs: f64) {
    let broken: Vec<_> = records.iter().filter(|r| r.status == "ERROR").collect();
    let successful: Vec<_> = records.iter().filter(|r| r.status != "ERROR").collect();

    let avg_response_ms = if successful.is_empty() {
        0
    } else {
        successful.iter().map(|r| r.response_time_ms).sum::<u128>()
            / successful.len() as u128
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