use std::path::Path;
use std::time::Duration;

use reqwest::Client;
use thiserror::Error;

pub const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub use_proxy: bool,
    pub proxy_protocol: String,
    pub proxy_host: String,
    pub proxy_port: u32,
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            use_proxy: false,
            proxy_protocol: String::new(),
            proxy_host: String::new(),
            proxy_port: 0,
            timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("unsupported url scheme")]
    UnsupportedScheme,
    #[error("response too large")]
    TooLarge,
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn fetch_url(url: &str, config: &ClientConfig) -> Result<String, FetchError> {
    let client = build_client(config)?;
    fetch_url_with_client(&client, url).await
}

pub async fn fetch_url_with_client(client: &Client, url: &str) -> Result<String, FetchError> {
    if let Some(path) = url.strip_prefix("file://") {
        let content = std::fs::read_to_string(path)?;
        return Ok(normalize_crlf(&content));
    }

    let resp = client.get(url).send().await?;
    let bytes = resp.bytes().await?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(FetchError::TooLarge);
    }
    Ok(normalize_crlf(&String::from_utf8_lossy(&bytes)))
}

pub fn build_client(config: &ClientConfig) -> Result<Client, FetchError> {
    let mut builder = Client::builder().timeout(config.timeout);
    if config.use_proxy && !config.proxy_host.is_empty() && config.proxy_port > 0 {
        let proxy_url = format!(
            "{}://{}:{}",
            config.proxy_protocol, config.proxy_host, config.proxy_port
        );
        let proxy = reqwest::Proxy::all(&proxy_url)?;
        builder = builder.proxy(proxy);
    }
    Ok(builder.build()?)
}

pub fn normalize_crlf(s: &str) -> String {
    s.replace("\r\n", "\n")
}

pub fn read_local_file(path: &Path) -> Result<String, FetchError> {
    let content = std::fs::read_to_string(path)?;
    Ok(normalize_crlf(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_crlf_works() {
        assert_eq!(normalize_crlf("a\r\nb"), "a\nb");
    }

    #[tokio::test]
    async fn file_url_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("h.txt");
        std::fs::write(&file, "127.0.0.1 file.test\n").unwrap();
        let url = format!("file://{}", file.display());
        let content = fetch_url(&url, &ClientConfig::default()).await.unwrap();
        assert!(content.contains("file.test"));
    }
}
