//! 回收站列表（中间栏视图）。

use switch_hosts_core::storage::trashcan::Trashcan;
use eframe::egui::{self, Color32, RichText};

use crate::icons::{self, Icon};
use crate::theme::SIDEBAR_BG;

pub fn draw_trash_panel(ui: &mut egui::Ui, trashcan: &Trashcan) {
    ui.painter().rect_filled(ui.max_rect(), 0.0, SIDEBAR_BG);

    ui.add_space(8.0);
    ui.heading("回收站");
    ui.separator();

    if trashcan.items.is_empty() {
        ui.label(
            RichText::new("回收站为空")
                .color(Color32::from_rgb(140, 140, 150)),
        );
        return;
    }

    let tint = Color32::from_rgb(100, 100, 110);
    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in &trashcan.items {
            let title = item
                .node
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&item.id);
            ui.horizontal(|ui| {
                icons::icon(ui, Icon::Trash, Icon::DEFAULT_SIZE, tint);
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).strong());
                    ui.small(format!("ID: {}", item.id));
                    if let Some(at) = &item.deleted_at {
                        ui.small(format!("删除于: {at}"));
                    }
                });
            });
            ui.separator();
        }
    });
}
