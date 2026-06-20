//! 本地 HTTP API 运行时（对齐 Pref Advanced `http_api_on`）。

use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;

use crate::shared_runtime;

pub struct HttpApiRuntime {
    handle: Option<service::api::ApiHandle>,
}

impl HttpApiRuntime {
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn sync(&mut self, config: &AppConfig, paths: &AppPaths, target: &HostsTarget) {
        self.shutdown();
        if !config.http_api_on {
            return;
        }
        match shared_runtime::block_on(service::api::start_api(
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
