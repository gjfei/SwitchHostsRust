use switch_hosts_core::storage::trashcan::Trashcan;
use eframe::egui;

/// 绘制回收站条目列表。
pub fn draw_trash(ui: &mut egui::Ui, trashcan: &Trashcan) {
    ui.heading("回收站");
    if trashcan.items.is_empty() {
        ui.label("回收站为空");
        return;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in &trashcan.items {
            let title = item
                .node
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&item.id);
            ui.group(|ui| {
                ui.label(format!("ID: {}", item.id));
                ui.label(format!("标题: {title}"));
                if let Some(at) = &item.deleted_at {
                    ui.small(format!("删除于: {at}"));
                }
            });
        }
    });
}
