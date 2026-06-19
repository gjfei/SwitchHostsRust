//! 全局底部状态栏（对齐 SwitchHosts `StatusBar`，横跨整窗宽度）。

use switch_hosts_core::manifest_edit::is_editor_read_only;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui::{RichText, Stroke, Ui};

use crate::panels::widgets::format_bytes;
use crate::theme::{EDITOR_LINE_NUMBER, SEPARATOR, STATUS_BAR_HEIGHT};

pub struct EditorStatus {
    pub line_count: usize,
    pub bytes: usize,
    pub read_only: bool,
}

pub fn editor_status(
    manifest: &Manifest,
    selected_id: Option<&str>,
    text: &str,
) -> EditorStatus {
    let node = selected_id.and_then(|id| find_node(&manifest.root, id));
    EditorStatus {
        line_count: if selected_id.is_some() {
            text.lines().count().max(1)
        } else {
            0
        },
        bytes: if selected_id.is_some() { text.len() } else { 0 },
        read_only: is_editor_read_only(selected_id, node.as_ref()),
    }
}

pub fn draw_status_bar(ui: &mut Ui, status: &EditorStatus) {
    ui.set_min_height(STATUS_BAR_HEIGHT);
    ui.set_max_height(STATUS_BAR_HEIGHT);
    ui.set_width(ui.available_width());

    let rect = ui.max_rect();
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, SEPARATOR));

    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!(
                "{} 行  {}",
                status.line_count,
                format_bytes(status.bytes)
            ))
            .size(10.0)
            .color(EDITOR_LINE_NUMBER),
        );
        if status.read_only && status.line_count > 0 {
            ui.label(
                RichText::new(" · 只读")
                    .size(10.0)
                    .color(EDITOR_LINE_NUMBER),
            );
        }
    });
}
