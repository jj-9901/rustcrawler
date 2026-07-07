use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type RobotsCache = Arc<Mutex<HashMap<String, Vec<String>>>>;

pub fn make_robots_cache() -> RobotsCache {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn is_allowed_by_robots(
    client: &reqwest::Client,
    url: &str,
    cache: &RobotsCache,
) -> bool {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return true,
    };

    let host = match parsed.host_str() {
        Some(h) => h.to_string(),
        None => return true,
    };

    let scheme = parsed.scheme();
    let robots_url = format!("{}://{}/robots.txt", scheme, host);

    // Check cache first
    {
        let cache = cache.lock().await;
        if let Some(disallowed) = cache.get(&host) {
            let path = parsed.path();
            return !disallowed.iter().any(|d| path.starts_with(d.as_str()));
        }
    }

    // Fetch robots.txt
    let disallowed = match client.get(&robots_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let text = resp.text().await.unwrap_or_default();
            parse_robots_txt(&text)
        }
        _ => vec![],
    };

    let path = parsed.path();
    let allowed = !disallowed.iter().any(|d| path.starts_with(d.as_str()));

    cache.lock().await.insert(host, disallowed);
    allowed
}

fn parse_robots_txt(text: &str) -> Vec<String> {
    let mut disallowed = vec![];
    let mut applies = false;

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("User-agent:") {
            let agent = line["User-agent:".len()..].trim();
            applies = agent == "*" || agent == "RustCrawler";
        } else if applies && line.starts_with("Disallow:") {
            let path = line["Disallow:".len()..].trim().to_string();
            if !path.is_empty() {
                disallowed.push(path);
            }
        }
    }

    disallowed
}

use std::time::Instant;

pub struct FetchResult {
    pub body: String,
    pub status: String,
    pub elapsed_ms: u128,
    pub final_url: String,
    pub redirect_chain: Vec<String>,
}

pub async fn fetch_page(
    _client: &reqwest::Client,
    url: &str,
) -> Result<FetchResult, reqwest::Error> {
    let start = Instant::now();

    // Build a client that does NOT auto-follow redirects
    // so we can track the chain manually
    let manual_client = reqwest::Client::builder()
        .user_agent("RustCrawler/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let mut current_url = url.to_string();
    let mut redirect_chain: Vec<String> = vec![];
    let final_status;
    let final_body;
    let mut hops = 0;

    loop {
        let response = manual_client.get(&current_url).send().await?;
        let status = response.status();
        let status_str = status.to_string();

        if status.is_redirection() && hops < 10 {
            // Get the Location header
            if let Some(location) = response.headers().get("location") {
                let next = location.to_str().unwrap_or("").to_string();
                // Resolve relative redirects
                let next_url = if next.starts_with("http") {
                    next
                } else {
                    let base = url::Url::parse(&current_url).unwrap();
                    base.join(&next).map(|u| u.to_string()).unwrap_or(next)
                };
                redirect_chain.push(current_url.clone());
                current_url = next_url;
                hops += 1;
                continue;
            }
        }

        final_status = status_str;
        final_body = response.text().await?;
        break;
    }

    let elapsed = start.elapsed().as_millis();

    Ok(FetchResult {
        body: final_body,
        status: final_status,
        elapsed_ms: elapsed,
        final_url: current_url,
        redirect_chain,
    })
}