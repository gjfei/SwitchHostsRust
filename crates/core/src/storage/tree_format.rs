//! Legacy（渲染层）与 v5 磁盘格式的双向转换。

use std::collections::HashSet;

use serde_json::{json, Map, Value};

const KEY_ID: &str = "id";
const KEY_TYPE: &str = "type";
const KEY_TITLE: &str = "title";
const KEY_ON: &str = "on";
const KEY_CHILDREN: &str = "children";
const KEY_EXTRAS: &str = "extras";

const KEY_LEGACY_IS_SYS: &str = "is_sys";
const KEY_LEGACY_URL: &str = "url";
const KEY_LEGACY_LAST_REFRESH: &str = "last_refresh";
const KEY_LEGACY_LAST_REFRESH_MS: &str = "last_refresh_ms";
const KEY_LEGACY_REFRESH_INTERVAL: &str = "refresh_interval";
const KEY_LEGACY_INCLUDE: &str = "include";
const KEY_LEGACY_FOLDER_MODE: &str = "folder_mode";
const KEY_LEGACY_FOLDER_OPEN: &str = "folder_open";
const KEY_LEGACY_IS_COLLAPSED: &str = "is_collapsed";

const KEY_V5_IS_SYS: &str = "isSys";
const KEY_V5_CONTENT_FILE: &str = "contentFile";
const KEY_V5_SOURCE: &str = "source";
const KEY_V5_SOURCE_URL: &str = "url";
const KEY_V5_SOURCE_LAST_REFRESH: &str = "lastRefresh";
const KEY_V5_SOURCE_LAST_REFRESH_MS: &str = "lastRefreshMs";
const KEY_V5_SOURCE_REFRESH_INTERVAL_SEC: &str = "refreshIntervalSec";
const KEY_V5_GROUP: &str = "group";
const KEY_V5_GROUP_INCLUDE: &str = "include";
const KEY_V5_FOLDER: &str = "folder";
const KEY_V5_FOLDER_MODE: &str = "mode";

pub const SYSTEM_NODE_ID: &str = "0";

pub fn legacy_root_to_v5(legacy: &[Value]) -> (Vec<Value>, Vec<String>) {
    let mut collapsed = Vec::new();
    let out = legacy
        .iter()
        .map(|n| legacy_node_to_v5(n, &mut collapsed))
        .collect();
    (out, collapsed)
}

fn legacy_node_to_v5(node: &Value, collapsed: &mut Vec<String>) -> Value {
    let Some(obj) = node.as_object() else {
        return node.clone();
    };

    let kind = obj
        .get(KEY_TYPE)
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_string();
    let id = obj.get(KEY_ID).and_then(Value::as_str).map(String::from);
    let is_sys = obj
        .get(KEY_LEGACY_IS_SYS)
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if kind == "folder" {
        if let Some(node_id) = id.as_deref() {
            let is_collapsed = obj.get(KEY_LEGACY_IS_COLLAPSED).and_then(Value::as_bool);
            let folder_open = obj.get(KEY_LEGACY_FOLDER_OPEN).and_then(Value::as_bool);
            let collapsed_now = match (is_collapsed, folder_open) {
                (Some(c), _) => c,
                (None, Some(open)) => !open,
                (None, None) => false,
            };
            if collapsed_now {
                collapsed.push(node_id.to_string());
            }
        }
    }

    let mut out = Map::new();
    if let Some(id) = id.clone() {
        out.insert(KEY_ID.into(), Value::String(id));
    }
    out.insert(KEY_TYPE.into(), Value::String(kind.clone()));
    if let Some(title) = obj.get(KEY_TITLE).cloned() {
        out.insert(KEY_TITLE.into(), title);
    }
    if let Some(on) = obj.get(KEY_ON).cloned() {
        out.insert(KEY_ON.into(), on);
    }
    if is_sys {
        out.insert(KEY_V5_IS_SYS.into(), json!(true));
    }

    match kind.as_str() {
        "local" | "remote" => {
            if let Some(node_id) = id.as_deref() {
                if !is_sys && node_id != SYSTEM_NODE_ID {
                    out.insert(
                        KEY_V5_CONTENT_FILE.into(),
                        json!(format!("entries/{node_id}.hosts")),
                    );
                }
            }
            if kind == "remote" {
                let mut source = Map::new();
                if let Some(url) = obj.get(KEY_LEGACY_URL).cloned() {
                    source.insert(KEY_V5_SOURCE_URL.into(), url);
                }
                if let Some(v) = obj.get(KEY_LEGACY_LAST_REFRESH).cloned() {
                    source.insert(KEY_V5_SOURCE_LAST_REFRESH.into(), v);
                }
                if let Some(v) = obj.get(KEY_LEGACY_LAST_REFRESH_MS).cloned() {
                    source.insert(KEY_V5_SOURCE_LAST_REFRESH_MS.into(), v);
                }
                if let Some(v) = obj.get(KEY_LEGACY_REFRESH_INTERVAL).cloned() {
                    source.insert(KEY_V5_SOURCE_REFRESH_INTERVAL_SEC.into(), v);
                }
                if !source.is_empty() {
                    out.insert(KEY_V5_SOURCE.into(), Value::Object(source));
                }
            }
        }
        "group" => {
            let include = obj
                .get(KEY_LEGACY_INCLUDE)
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));
            out.insert(
                KEY_V5_GROUP.into(),
                json!({ KEY_V5_GROUP_INCLUDE: include }),
            );
        }
        "folder" => {
            let mode = obj
                .get(KEY_LEGACY_FOLDER_MODE)
                .cloned()
                .unwrap_or(Value::Number(serde_json::Number::from(0)));
            out.insert(KEY_V5_FOLDER.into(), json!({ KEY_V5_FOLDER_MODE: mode }));
            if let Some(children) = obj.get(KEY_CHILDREN).and_then(Value::as_array) {
                let new_children: Vec<Value> = children
                    .iter()
                    .map(|c| legacy_node_to_v5(c, collapsed))
                    .collect();
                out.insert(KEY_CHILDREN.into(), Value::Array(new_children));
            }
        }
        _ => {}
    }

    let mut extras = Map::new();
    for (k, v) in obj {
        if is_modeled_legacy_key(k) {
            continue;
        }
        extras.insert(k.clone(), v.clone());
    }
    if !extras.is_empty() {
        out.insert(KEY_EXTRAS.into(), Value::Object(extras));
    }

    Value::Object(out)
}

fn is_modeled_legacy_key(key: &str) -> bool {
    matches!(
        key,
        KEY_ID
            | KEY_TYPE
            | KEY_TITLE
            | KEY_ON
            | KEY_CHILDREN
            | KEY_LEGACY_IS_SYS
            | KEY_LEGACY_URL
            | KEY_LEGACY_LAST_REFRESH
            | KEY_LEGACY_LAST_REFRESH_MS
            | KEY_LEGACY_REFRESH_INTERVAL
            | KEY_LEGACY_INCLUDE
            | KEY_LEGACY_FOLDER_MODE
            | KEY_LEGACY_FOLDER_OPEN
            | KEY_LEGACY_IS_COLLAPSED
            | KEY_EXTRAS
    )
}

pub fn v5_root_to_legacy(v5: &[Value], collapsed_ids: &[String]) -> Vec<Value> {
    let collapsed_set: HashSet<&str> = collapsed_ids.iter().map(String::as_str).collect();
    v5.iter()
        .map(|n| v5_node_to_legacy(n, &collapsed_set))
        .collect()
}

fn v5_node_to_legacy(node: &Value, collapsed_set: &HashSet<&str>) -> Value {
    let Some(obj) = node.as_object() else {
        return node.clone();
    };

    let mut out = Map::new();
    let id = obj.get(KEY_ID).and_then(Value::as_str).map(String::from);
    let kind = obj
        .get(KEY_TYPE)
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_string();

    if let Some(id) = id.clone() {
        out.insert(KEY_ID.into(), Value::String(id));
    }
    if let Some(title) = obj.get(KEY_TITLE).cloned() {
        out.insert(KEY_TITLE.into(), title);
    }
    if let Some(on) = obj.get(KEY_ON).cloned() {
        out.insert(KEY_ON.into(), on);
    }
    out.insert(KEY_TYPE.into(), Value::String(kind.clone()));

    let is_sys = obj
        .get(KEY_V5_IS_SYS)
        .and_then(Value::as_bool)
        .or_else(|| obj.get(KEY_LEGACY_IS_SYS).and_then(Value::as_bool))
        .unwrap_or(false);
    if is_sys {
        out.insert(KEY_LEGACY_IS_SYS.into(), json!(true));
    }

    match kind.as_str() {
        "remote" => {
            if let Some(source) = obj.get(KEY_V5_SOURCE).and_then(Value::as_object) {
                if let Some(v) = source.get(KEY_V5_SOURCE_URL) {
                    out.insert(KEY_LEGACY_URL.into(), v.clone());
                }
                if let Some(v) = source.get(KEY_V5_SOURCE_LAST_REFRESH) {
                    out.insert(KEY_LEGACY_LAST_REFRESH.into(), v.clone());
                }
                if let Some(v) = source.get(KEY_V5_SOURCE_LAST_REFRESH_MS) {
                    out.insert(KEY_LEGACY_LAST_REFRESH_MS.into(), v.clone());
                }
                if let Some(v) = source.get(KEY_V5_SOURCE_REFRESH_INTERVAL_SEC) {
                    out.insert(KEY_LEGACY_REFRESH_INTERVAL.into(), v.clone());
                }
            } else {
                copy_if_present(obj, &mut out, KEY_LEGACY_URL);
                copy_if_present(obj, &mut out, KEY_LEGACY_LAST_REFRESH);
                copy_if_present(obj, &mut out, KEY_LEGACY_LAST_REFRESH_MS);
                copy_if_present(obj, &mut out, KEY_LEGACY_REFRESH_INTERVAL);
            }
        }
        "group" => {
            if let Some(group) = obj.get(KEY_V5_GROUP).and_then(Value::as_object) {
                if let Some(include) = group.get(KEY_V5_GROUP_INCLUDE) {
                    out.insert(KEY_LEGACY_INCLUDE.into(), include.clone());
                }
            } else {
                copy_if_present(obj, &mut out, KEY_LEGACY_INCLUDE);
            }
        }
        "folder" => {
            if let Some(folder) = obj.get(KEY_V5_FOLDER).and_then(Value::as_object) {
                if let Some(mode) = folder.get(KEY_V5_FOLDER_MODE) {
                    out.insert(KEY_LEGACY_FOLDER_MODE.into(), mode.clone());
                }
            } else {
                copy_if_present(obj, &mut out, KEY_LEGACY_FOLDER_MODE);
            }
            if let Some(children) = obj.get(KEY_CHILDREN).and_then(Value::as_array) {
                let new_children: Vec<Value> = children
                    .iter()
                    .map(|c| v5_node_to_legacy(c, collapsed_set))
                    .collect();
                out.insert(KEY_CHILDREN.into(), Value::Array(new_children));
            }
            if let Some(node_id) = id.as_deref() {
                if collapsed_set.contains(node_id) {
                    out.insert(KEY_LEGACY_IS_COLLAPSED.into(), json!(true));
                }
            }
        }
        _ => {}
    }

    if let Some(extras) = obj.get(KEY_EXTRAS).and_then(Value::as_object) {
        for (k, v) in extras {
            if !out.contains_key(k) {
                out.insert(k.clone(), v.clone());
            }
        }
    }

    Value::Object(out)
}

fn copy_if_present(src: &Map<String, Value>, dst: &mut Map<String, Value>, key: &str) {
    if let Some(v) = src.get(key) {
        dst.insert(key.into(), v.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_fixture() -> Vec<Value> {
        json!([
            { "id": "local-1", "type": "local", "title": "A", "on": true },
            {
                "id": "folder-a",
                "type": "folder",
                "folder_mode": 0,
                "is_collapsed": true,
                "children": [
                    { "id": "local-2", "type": "local", "on": false }
                ]
            }
        ])
        .as_array()
        .cloned()
        .unwrap()
    }

    #[test]
    fn round_trip_preserves_ids() {
        let legacy = tree_fixture();
        let (v5, collapsed) = legacy_root_to_v5(&legacy);
        let back = v5_root_to_legacy(&v5, &collapsed);
        let ids: Vec<_> = back
            .iter()
            .flat_map(collect_ids)
            .collect();
        assert!(ids.contains(&"local-1".to_string()));
        assert!(ids.contains(&"local-2".to_string()));
    }

    fn collect_ids(node: &Value) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
            out.push(id.to_string());
        }
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for c in children {
                out.extend(collect_ids(c));
            }
        }
        out
    }
}
