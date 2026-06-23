//! 编辑器底部状态栏（对齐 SwitchHosts `StatusBar`）。

use switch_hosts_core::manifest_edit::is_editor_read_only;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Align, CornerRadius, Sense, Stroke, Ui};

use crate::fonts::ui_font_id;
use crate::panels::widgets::format_bytes;
use crate::theme::{self, layout};

const STATUS_FONT_SIZE: f32 = 10.0;
const STATUS_SEPARATOR_H: f32 = 1.0;

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
pub fn pin_body_and_status_bar<R>(
    ui: &mut Ui,
    draw_body: impl FnOnce(&mut Ui) -> R,
    draw_status: impl FnOnce(&mut Ui),
) -> R {
    let area = ui.max_rect().intersect(ui.clip_rect());
    let status_h = layout::STATUS_BAR_HEIGHT;
    let status_top = (area.max.y - status_h).max(area.min.y);
    let body_rect = egui::Rect::from_min_max(area.min, egui::pos2(area.max.x, status_top));
    let status_rect = egui::Rect::from_min_max(egui::pos2(area.min.x, status_top), area.max);

    let body = ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .id_salt("pinned_editor_body"),
        draw_body,
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(status_rect)
            .id_salt("pinned_editor_status"),
        draw_status,
    );
    body.inner
}

/// 侧栏/列表面板底部占位（与编辑器 status bar 同高，背景对齐 `editor_bg`）。
pub fn draw_panel_status_spacer(ui: &mut Ui) {
    draw_panel_status_spacer_with_corners(ui, CornerRadius::ZERO);
}

pub fn draw_panel_status_spacer_with_corners(ui: &mut Ui, corner_radius: CornerRadius) {
    let t = theme::app(ui.ctx());
    let rect = ui.max_rect();
    let _ = ui.allocate_rect(rect, Sense::hover());
    ui.painter()
        .rect_filled(rect, corner_radius, t.editor_bg);
}

pub fn draw_status_bar(ui: &mut Ui, status: &EditorStatus) {
    draw_status_bar_with_corners(ui, status, CornerRadius::ZERO);
}

pub fn draw_status_bar_with_corners(
    ui: &mut Ui,
    status: &EditorStatus,
    corner_radius: CornerRadius,
) {
    let t = theme::app(ui.ctx());
    let rect = ui.max_rect();
    let _ = ui.allocate_rect(rect, Sense::hover());

    ui.painter()
        .rect_filled(rect, corner_radius, t.editor_bg);
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, t.separator));

    let mut label = format!("{} 行  {}", status.line_count, format_bytes(status.bytes));
    if status.read_only && status.line_count > 0 {
        label.push_str(" · 只读");
    }

    let text_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + layout::STATUS_BAR_PAD_X, rect.top() + STATUS_SEPARATOR_H),
        egui::pos2(rect.right() - layout::STATUS_BAR_PAD_X, rect.bottom()),
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .id_salt("status_text"),
        |ui| {
            let mut job = LayoutJob::single_section(
                label,
                TextFormat {
                    font_id: ui_font_id(STATUS_FONT_SIZE),
                    color: t.weak_text,
                    valign: Align::Center,
                    ..Default::default()
                },
            );
            job.first_row_min_height = text_rect.height();
            job.halign = Align::LEFT;

            ui.with_layout(egui::Layout::top_down(Align::LEFT), |ui| {
                ui.label(job);
            });
        },
    );
}
