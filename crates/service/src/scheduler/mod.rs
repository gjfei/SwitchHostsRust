use std::sync::Arc;
use std::time::Duration;

use switch_hosts_core::storage::entries;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::storage::paths::AppPaths;
use tokio::task::JoinHandle;
use tokio::time;

use crate::client::{fetch_url, ClientConfig};

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub scan_interval: Duration,
    pub startup_refresh: bool,
    pub startup_delay: Duration,
    pub client: ClientConfig,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            scan_interval: Duration::from_secs(60),
            startup_refresh: false,
            startup_delay: Duration::from_secs(5),
            client: ClientConfig::default(),
        }
    }
}

pub struct RefreshScheduler {
    join: JoinHandle<()>,
}

impl RefreshScheduler {
    pub fn start(
        paths: AppPaths,
        config: SchedulerConfig,
    ) -> Self {
        let paths = Arc::new(paths);
        let join = tokio::spawn(async move {
            if config.startup_refresh {
                time::sleep(config.startup_delay).await;
                let _ = refresh_all(&paths, &config.client).await;
            }
            let mut interval = time::interval(config.scan_interval);
            loop {
                interval.tick().await;
                let _ = refresh_all(&paths, &config.client).await;
            }
        });
        Self { join }
    }

    pub fn abort(self) {
        self.join.abort();
    }
}

async fn refresh_all(paths: &AppPaths, client_cfg: &ClientConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest = Manifest::load(paths)?;
    let nodes = collect_remote_nodes(&manifest.root);
    for (id, url, interval) in nodes {
        if !should_refresh(interval) {
            continue;
        }
        let content = fetch_url(&url, client_cfg).await?;
        entries::write_entry(&paths.entries_dir, &id, &content)?;
    }
    Ok(())
}

fn should_refresh(interval_sec: Option<u64>) -> bool {
    interval_sec.is_some()
}

fn collect_remote_nodes(nodes: &[serde_json::Value]) -> Vec<(String, String, Option<u64>)> {
    let mut out = Vec::new();
    walk_remote(nodes, &mut out);
    out
}

fn walk_remote(nodes: &[serde_json::Value], out: &mut Vec<(String, String, Option<u64>)>) {
    for node in nodes {
        if node.get("type").and_then(|v| v.as_str()) == Some("remote") {
            if let (Some(id), Some(url)) = (
                node.get("id").and_then(|v| v.as_str()),
                node.get("url").and_then(|v| v.as_str()),
            ) {
                let interval = node
                    .get("refresh_interval")
                    .and_then(|v| v.as_u64());
                out.push((id.to_string(), url.to_string(), interval));
            }
        }
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            walk_remote(children, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_remote_nodes() {
        let nodes = serde_json::json!([{
            "id": "r1",
            "type": "remote",
            "url": "http://example.com/hosts",
            "refresh_interval": 3600
        }]);
        let list = collect_remote_nodes(nodes.as_array().unwrap());
        assert_eq!(list.len(), 1);
    }
}
