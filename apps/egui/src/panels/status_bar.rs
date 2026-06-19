//! 编辑器底部状态栏（对齐 SwitchHosts `StatusBar`）。

use switch_hosts_core::manifest_edit::is_editor_read_only;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui::{Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::panels::widgets::format_bytes;
use crate::text_align;
use crate::theme::{self, layout};

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

/// 将面板底部分配给 status bar，上方区域交给 `draw_body`（对齐 `HostsEditor` / `HostsViewer`）。
pub fn pin_body_and_status_bar(
    ui: &mut Ui,
    draw_body: impl FnOnce(&mut Ui),
    draw_status: impl FnOnce(&mut Ui),
) {
    let outer = ui.max_rect();
    let status_h = layout::STATUS_BAR_HEIGHT;
    let body_rect = egui::Rect::from_min_max(
        outer.min,
        egui::pos2(outer.max.x, outer.max.y - status_h),
    );
    let status_rect = egui::Rect::from_min_max(
        egui::pos2(outer.min.x, outer.max.y - status_h),
        outer.max,
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), draw_body);
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(status_rect), draw_status);
    ui.expand_to_include_rect(outer);
}

/// 侧栏/列表面板底部占位（与编辑器 status bar 同高，背景对齐 `editor_bg`）。
pub fn draw_panel_status_spacer(ui: &mut Ui) {
    let t = theme::app(ui.ctx());
    let rect = ui.max_rect();
    let size = Vec2::new(rect.width().max(ui.available_width()), layout::STATUS_BAR_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 0.0, t.editor_bg);
}

pub fn draw_status_bar(ui: &mut Ui, status: &EditorStatus) {
    let t = theme::app(ui.ctx());
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, t.editor_bg);
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, t.separator));

    let cy = rect.center().y;
    let font = ui_font_id(10.0);
    let mut x = rect.left() + 10.0;
    let text_color = t.weak_text;

    let main = format!("{} 行  {}", status.line_count, format_bytes(status.bytes));
    let galley = text_align::layout_vcentered_galley(
        ui,
        main,
        font.clone(),
        text_color,
        STATUS_TEXT_LINE_HEIGHT,
    );
    let main_w = galley.size().x;
    text_align::paint_galley_row_centered(ui, x, cy, galley, text_color);
    x += main_w;

    if status.read_only && status.line_count > 0 {
        let ro = text_align::layout_vcentered_galley(
            ui,
            " · 只读".to_string(),
            font,
            text_color,
            STATUS_TEXT_LINE_HEIGHT,
        );
        text_align::paint_galley_row_centered(ui, x, cy, ro, text_color);
    }
}
