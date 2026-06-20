//! GUI 共享 async runtime 与 HTTP client 缓存（避免每次网络操作新建 tokio / reqwest）。

use std::sync::{Mutex, OnceLock};

use reqwest::Client;
use service::{build_client, fetch_url_with_client, ClientConfig, FetchError};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("switch-hosts-async")
            .build()
            .expect("async runtime")
    })
}

#[derive(Clone, PartialEq, Eq)]
struct ClientCacheKey {
    use_proxy: bool,
    proxy_protocol: String,
    proxy_host: String,
    proxy_port: u32,
}

impl From<&ClientConfig> for ClientCacheKey {
    fn from(config: &ClientConfig) -> Self {
        Self {
            use_proxy: config.use_proxy,
            proxy_protocol: config.proxy_protocol.clone(),
            proxy_host: config.proxy_host.clone(),
            proxy_port: config.proxy_port,
        }
    }
}

static HTTP_CLIENT: Mutex<Option<(ClientCacheKey, Client)>> = Mutex::new(None);

pub fn invalidate_http_client() {
    if let Ok(mut guard) = HTTP_CLIENT.lock() {
        *guard = None;
    }
}

fn http_client(config: &ClientConfig) -> Result<Client, FetchError> {
    let key = ClientCacheKey::from(config);
    let mut guard = HTTP_CLIENT
        .lock()
        .map_err(|_| FetchError::Io(std::io::Error::other("http client cache poisoned")))?;
    if let Some((cached_key, client)) = guard.as_ref() {
        if cached_key == &key {
            return Ok(client.clone());
        }
    }
    let client = build_client(config)?;
    *guard = Some((key, client.clone()));
    Ok(client)
}

pub fn fetch(config: &ClientConfig, url: &str) -> Result<String, FetchError> {
    let client = http_client(config)?;
    block_on(fetch_url_with_client(&client, url))
}
