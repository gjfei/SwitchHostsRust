use switch_hosts_core::storage::manifest::Manifest;

/// 托盘菜单项（纯数据结构，便于单测；原生托盘 API 在后续集成）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuEntry {
    pub id: String,
    pub label: String,
    pub checked: bool,
}

/// 从 manifest 构建托盘快捷切换菜单项。
pub fn build_tray_menu(manifest: &Manifest) -> Vec<TrayMenuEntry> {
    manifest
        .root
        .iter()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?.to_string();
            let title = node
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&id);
            let checked = node.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
            Some(TrayMenuEntry {
                id: id.clone(),
                label: format!("{} {title}", if checked { "✓" } else { "○" }),
                checked,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn menu_reflects_on_state() {
        let manifest = Manifest {
            root: json!([
                { "id": "1", "title": "Dev", "on": true },
                { "id": "2", "title": "Prod", "on": false }
            ])
            .as_array()
            .cloned()
            .unwrap(),
            ..Default::default()
        };
        let menu = build_tray_menu(&manifest);
        assert_eq!(menu.len(), 2);
        assert!(menu[0].checked);
        assert!(!menu[1].checked);
        assert!(menu[0].label.contains('✓'));
    }
}
