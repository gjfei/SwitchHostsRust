//! 全局底部状态栏（对齐 SwitchHosts `StatusBar`，横跨整窗宽度）。

use switch_hosts_core::manifest_edit::is_editor_read_only;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui::{Stroke, Ui};

use crate::fonts::ui_font_id;
use crate::panels::widgets::format_bytes;
use crate::text_align;
use crate::theme::{EDITOR_LINE_NUMBER, SEPARATOR, STATUS_BAR_HEIGHT};

const STATUS_TEXT_LINE_HEIGHT: f32 = 12.0;

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

    let cy = rect.center().y;
    let font = ui_font_id(10.0);
    let mut x = rect.left() + 10.0;

    let main = format!("{} 行  {}", status.line_count, format_bytes(status.bytes));
    let galley = text_align::layout_vcentered_galley(
        ui,
        main,
        font.clone(),
        EDITOR_LINE_NUMBER,
        STATUS_TEXT_LINE_HEIGHT,
    );
    let main_w = galley.size().x;
    text_align::paint_galley_row_centered(ui, x, cy, galley, EDITOR_LINE_NUMBER);
    x += main_w;

    if status.read_only && status.line_count > 0 {
        let ro = text_align::layout_vcentered_galley(
            ui,
            " · 只读".to_string(),
            font,
            EDITOR_LINE_NUMBER,
            STATUS_TEXT_LINE_HEIGHT,
        );
        text_align::paint_galley_row_centered(ui, x, cy, ro, EDITOR_LINE_NUMBER);
    }
}
