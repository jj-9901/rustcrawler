use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct PageRecord {
    pub url: String,
    pub status: String,
    pub depth: u32,
    pub links_found: usize,
    pub response_time_ms: u128,
    pub title: String,
    pub size_bytes: usize,
    #[serde(skip)]
    pub redirect_chain: Vec<String>,
    pub redirect_count: usize,
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkEdge {
    pub from: String,
    pub to: String,
}