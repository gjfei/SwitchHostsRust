use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui;

pub fn draw_details(ui: &mut egui::Ui, manifest: &Manifest, selected_id: Option<&str>) {
    ui.heading("详情");
    let Some(id) = selected_id else {
        ui.label("请选择方案节点");
        return;
    };
    let Some(node) = find_node(&manifest.root, id) else {
        ui.label("未找到节点");
        return;
    };
    ui.label(format!("ID: {id}"));
    if let Some(t) = node.get("type").and_then(|v| v.as_str()) {
        ui.label(format!("类型: {t}"));
    }
    if let Some(title) = node.get("title").and_then(|v| v.as_str()) {
        ui.label(format!("标题: {title}"));
    }
    if let Some(on) = node.get("on").and_then(|v| v.as_bool()) {
        ui.label(format!("已启用: {on}"));
    }
    if let Some(url) = node.get("url").and_then(|v| v.as_str()) {
        ui.label(format!("URL: {url}"));
    }
    if let Some(interval) = node.get("refresh_interval").and_then(|v| v.as_u64()) {
        ui.label(format!("刷新间隔: {interval} 秒"));
    }
}
