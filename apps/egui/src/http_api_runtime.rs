//! 本地 HTTP API 运行时（对齐 Pref Advanced `http_api_on`）。

use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;

pub struct HttpApiRuntime {
    rt: tokio::runtime::Runtime,
    handle: Option<service::api::ApiHandle>,
}

impl HttpApiRuntime {
    pub fn new() -> Self {
        Self {
            rt: tokio::runtime::Runtime::new().expect("tokio runtime"),
            handle: None,
        }
    }

    pub fn sync(&mut self, config: &AppConfig, paths: &AppPaths, target: &HostsTarget) {
        self.shutdown();
        if !config.http_api_on {
            return;
        }
        match self.rt.block_on(service::api::start_api(
            paths.clone(),
            config.clone(),
            target.clone(),
            config.http_api_only_local,
        )) {
            Ok(handle) => self.handle = Some(handle),
            Err(err) => tracing::warn!("HTTP API 启动失败: {err}"),
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join.abort();
        }
    }
}

impl Drop for HttpApiRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}
