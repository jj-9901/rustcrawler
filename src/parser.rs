use scraper::{Html, Selector};
use url::Url;

pub fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();

    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };

    let mut links = Vec::new();

    for element in document.select(&selector) {
        let href = match element.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        if href.starts_with('#')
            || href.starts_with("mailto:")
            || href.starts_with("javascript:")
        {
            continue;
        }

        let full_url = match base.join(href) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let full_url = &mut full_url.clone();
        full_url.set_fragment(None);

        if full_url.scheme() != "http" && full_url.scheme() != "https" {
            continue;
        }

        links.push(full_url.to_string());
    }

    links
}