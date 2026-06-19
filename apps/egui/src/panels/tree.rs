use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::toggle::toggle_item;
use eframe::egui;
use serde_json::Value;

pub fn draw_tree(
    ui: &mut egui::Ui,
    manifest: &mut Manifest,
    selected_id: &mut Option<String>,
    config: &AppConfig,
) -> bool {
    let mut changed_selection = false;
    let mut pending_toggle = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for node in manifest.root.iter_mut() {
            render_node(
                ui,
                node,
                selected_id,
                &mut changed_selection,
                &mut pending_toggle,
                0,
            );
        }
    });
    if let Some(id) = pending_toggle {
        toggle_item(&mut manifest.root, &id, config.choice_mode);
    }
    changed_selection
}

fn render_node(
    ui: &mut egui::Ui,
    node: &mut Value,
    selected_id: &mut Option<String>,
    changed: &mut bool,
    pending_toggle: &mut Option<String>,
    depth: usize,
) {
    let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let title = node
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| id.clone());
    let mut on = node.get("on").and_then(|v| v.as_bool()).unwrap_or(false);

    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 12.0);
        if ui.checkbox(&mut on, "").changed() {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("on".into(), serde_json::json!(on));
            }
        }
        let label = format!("{title} ({id})");
        if ui.selectable_label(selected_id.as_deref() == Some(&id), label).clicked() {
            *selected_id = Some(id.clone());
            *changed = true;
        }
        if ui.small_button("↕").clicked() {
            *pending_toggle = Some(id);
        }
    });

    if let Some(children) = node
        .as_object_mut()
        .and_then(|o| o.get_mut("children"))
        .and_then(|c| c.as_array_mut())
    {
        for child in children.iter_mut() {
            render_node(ui, child, selected_id, changed, pending_toggle, depth + 1);
        }
    }
}
