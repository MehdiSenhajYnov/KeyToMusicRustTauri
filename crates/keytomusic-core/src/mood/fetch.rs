//! Downloading the pages of a chapter from their URLs.
//!
//! Readers rarely let a browser extension read their images: the image host
//! sends no CORS headers (the canvas is "tainted") and refuses requests
//! without the reader's Referer (hotlink protection). The app has neither
//! limit: it downloads the pages itself, with the reader page the user has
//! open as Referer, and only asks the extension for the pages it could not
//! get.

use std::time::Duration;

use super::image::prepare_bytes;

/// The app says who it is; image hosts that refuse anonymous tools
/// (`curl/…`) accept it.
pub const USER_AGENT: &str = concat!(
    "KeyToMusic/",
    env!("CARGO_PKG_VERSION"),
    " (Mood AI; +https://github.com/MehdiSenhajYnov/KeyToMusicRustTauri)"
);
const TIMEOUT: Duration = Duration::from_secs(30);
/// Bigger than any manga page; protects against a wrong URL.
const MAX_BYTES: usize = 25 * 1024 * 1024;

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(TIMEOUT)
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

/// Can the app download this URL itself? (`blob:` and `data:` live in the page.)
pub fn downloadable(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Download one page and prepare it for the model (base64 JPEG tiles).
pub async fn fetch_page(
    client: &reqwest::Client,
    url: &str,
    referer: Option<&str>,
) -> Result<Vec<String>, String> {
    if !downloadable(url) {
        return Err("adresse locale à la page".into());
    }
    let mut request = client.get(url).header(
        reqwest::header::ACCEPT,
        "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
    );
    if let Some(referer) = referer.filter(|r| downloadable(r)) {
        request = request.header(reqwest::header::REFERER, referer);
    }
    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            "délai dépassé".to_string()
        } else {
            format!("requête impossible ({e})")
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|n| n as usize > MAX_BYTES)
    {
        return Err("image trop lourde".into());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("téléchargement interrompu ({e})"))?;
    if bytes.len() > MAX_BYTES {
        return Err("image trop lourde".into());
    }
    tokio::task::spawn_blocking(move || prepare_bytes(&bytes))
        .await
        .map_err(|e| e.to_string())?
}

/// Page URLs a "ts_reader" page declares (`ts_reader.run({... "sources":
/// [{"images": [...]}]})`), the reader of most WordPress manga sites
/// (sushiscan and the like). Used by tools; the extension reads the same.
pub fn ts_reader_images(html: &str) -> Option<Vec<String>> {
    let start = html.find("ts_reader.run(")? + "ts_reader.run(".len();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut end = None;
    for (i, c) in html[start..].char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    end = Some(start + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let config: serde_json::Value = serde_json::from_str(&html[start..end?]).ok()?;
    let images: Vec<String> = config
        .get("sources")?
        .as_array()?
        .iter()
        .find_map(|source| source.get("images")?.as_array().filter(|a| !a.is_empty()))?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    (!images.is_empty()).then_some(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_ts_reader_page_list() {
        let html = r#"<script>ts_reader.run({"post_id":1,"noimagehtml":"<center>{x}<\/center>","sources":[{"source":"Server 1","images":["https:\/\/c.example.net\/a-01.webp","https:\/\/c.example.net\/a-02.webp"]}],"lazyload":true});</script>"#;
        assert_eq!(
            ts_reader_images(html).unwrap(),
            [
                "https://c.example.net/a-01.webp",
                "https://c.example.net/a-02.webp"
            ]
        );
        assert!(ts_reader_images("<html>no reader</html>").is_none());
    }

    #[test]
    fn only_web_urls_are_downloaded() {
        assert!(downloadable("https://x/y.jpg"));
        assert!(!downloadable("blob:https://x/123"));
        assert!(!downloadable("data:image/png;base64,AAAA"));
    }
}
