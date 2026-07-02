use crate::models::{LinkEdge, PageRecord};
use std::fs;

pub fn export_csv(records: &[PageRecord], path: &str) {
    let mut writer = csv::Writer::from_path(path).expect("Could not create CSV file");
    for record in records {
        writer.serialize(record).expect("Could not write record");
    }
    writer.flush().expect("Could not flush CSV");
    println!("  Saved to: {}", path);
}

pub fn export_json(records: &[PageRecord], edges: &[LinkEdge], path: &str) {
    let ok: Vec<_> = records.iter().filter(|r| r.status != "ERROR").collect();
    let avg_response_ms = if ok.is_empty() {
        0
    } else {
        ok.iter().map(|r| r.response_time_ms).sum::<u128>() / ok.len() as u128
    };

    let output = serde_json::json!({
        "total_pages": records.len(),
        "successful": ok.len(),
        "broken_links": records.iter().filter(|r| r.status == "ERROR").count(),
        "avg_response_ms": avg_response_ms,
        "pages": records,
        "edges": edges.iter().map(|e| serde_json::json!({
            "from": e.from,
            "to": e.to
        })).collect::<Vec<_>>()
    });

    fs::write(path, serde_json::to_string_pretty(&output).unwrap())
        .expect("Could not write JSON file");
    println!("  Saved to: {}", path);
}