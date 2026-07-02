use std::time::Instant;

pub async fn fetch_page(
    client: &reqwest::Client,
    url: &str,
) -> Result<(String, String, u128), reqwest::Error> {
    let start = Instant::now();
    let response = client.get(url).send().await?;
    let status = response.status().to_string();
    let body = response.text().await?;
    let elapsed = start.elapsed().as_millis();
    Ok((body, status, elapsed))
}