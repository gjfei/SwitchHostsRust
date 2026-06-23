//! 远程方案刷新（对齐 SwitchHosts `refresh_remote_hosts`）。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Local, TimeZone};
use serde_json::json;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::entries::{read_entry, write_entry};
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use switch_hosts_core::storage::paths::AppPaths;

use crate::shared_runtime;

pub fn client_config_from_app(config: &AppConfig) -> service::ClientConfig {
    service::ClientConfig {
        use_proxy: config.use_proxy,
        proxy_protocol: config.proxy_protocol.clone(),
        proxy_host: config.proxy_host.clone(),
        proxy_port: config.proxy_port,
        timeout: Duration::from_secs(30),
    }
}

pub fn refresh_all_remote_hosts(
    paths: &AppPaths,
    manifest: &mut Manifest,
    config: &AppConfig,
) -> usize {
    let ids = remote_node_ids(&manifest.root);
    let mut changed = 0usize;
    for id in ids {
        if refresh_remote_node(paths, manifest, config, &id).unwrap_or(false) {
            changed += 1;
        }
    }
    if changed > 0 {
        let _ = manifest.save(paths);
    }
    changed
}

fn remote_node_ids(nodes: &[serde_json::Value]) -> Vec<String> {
    let mut out = Vec::new();
    collect_remote_ids(nodes, &mut out);
    out
}

fn collect_remote_ids(nodes: &[serde_json::Value], out: &mut Vec<String>) {
    for node in nodes {
        if node.get("type").and_then(|v| v.as_str()) == Some("remote") {
            if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
        }
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            collect_remote_ids(children, out);
        }
    }
}

pub fn refresh_remote_node(
    paths: &AppPaths,
    manifest: &mut Manifest,
    config: &AppConfig,
    id: &str,
) -> Result<bool, String> {
    let snapshot = find_node(&manifest.root, id).ok_or_else(|| "节点不存在".to_string())?;
    if snapshot.get("type").and_then(|v| v.as_str()) != Some("remote") {
        return Err("不是远程方案".to_string());
    }
    let url = snapshot
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|u| !u.is_empty())
        .ok_or_else(|| "未设置 URL".to_string())?;

    let client_cfg = client_config_from_app(config);

    let new_content = shared_runtime::fetch(&client_cfg, url).map_err(|e| e.to_string())?;

    let old_content = read_entry(&paths.entries_dir, id).unwrap_or_default();
    let content_changed = new_content != old_content;
    if content_changed {
        write_entry(&paths.entries_dir, id, &new_content).map_err(|e| e.to_string())?;
    }

    touch_last_refresh(manifest, id);
    Ok(content_changed)
}

fn touch_last_refresh(manifest: &mut Manifest, id: &str) {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let stamp = Local
        .timestamp_millis_opt(ms as i64)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    touch_last_refresh_recursive(&mut manifest.root, id, ms, &stamp);
}

fn touch_last_refresh_recursive(
    nodes: &mut [serde_json::Value],
    id: &str,
    ms: u64,
    stamp: &str,
) -> bool {
    for node in nodes.iter_mut() {
        if node.get("id").and_then(|v| v.as_str()) == Some(id) {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("last_refresh_ms".into(), json!(ms));
                obj.insert("last_refresh".into(), json!(stamp));
            }
            return true;
        }
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if touch_last_refresh_recursive(children, id, ms, stamp) {
                return true;
            }
        }
    }
    false
}
