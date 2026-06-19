use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui;

pub fn draw_details(ui: &mut egui::Ui, manifest: &Manifest, selected_id: Option<&str>) {
    ui.heading("Details");
    let Some(id) = selected_id else {
        ui.label("Select a node");
        return;
    };
    let Some(node) = find_node(&manifest.root, id) else {
        ui.label("Node not found");
        return;
    };
    ui.label(format!("ID: {id}"));
    if let Some(t) = node.get("type").and_then(|v| v.as_str()) {
        ui.label(format!("Type: {t}"));
    }
    if let Some(on) = node.get("on").and_then(|v| v.as_bool()) {
        ui.label(format!("On: {on}"));
    }
    if let Some(url) = node.get("url").and_then(|v| v.as_str()) {
        ui.label(format!("URL: {url}"));
    }
    if let Some(interval) = node.get("refresh_interval").and_then(|v| v.as_u64()) {
        ui.label(format!("Refresh interval: {interval}s"));
    }
}
