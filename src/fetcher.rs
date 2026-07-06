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