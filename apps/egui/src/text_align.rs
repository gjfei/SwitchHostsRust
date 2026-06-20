//! 行内文字垂直居中：图标 + 中英文混排时对齐 epaint mesh 视觉中心。

use std::sync::Arc;

use eframe::egui::{text::LayoutJob, Align, Color32, FontId, Galley, Pos2, Rect, Ui};

use crate::icons::{self, Icon};

/// 与 16px 图标同行的标准行高（列表、TopBar、Transfer 等）。
pub const ICON_ROW_LINE_HEIGHT: f32 = 16.0;

/// 固定行高 + `valign: Center`，便于与图标中心对齐。
pub fn layout_vcentered_galley(
    ui: &Ui,
    text: String,
    font_id: FontId,
    color: Color32,
    line_height: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::simple(text, font_id, color, f32::INFINITY);
    if let Some(section) = job.sections.first_mut() {
        section.format.valign = Align::Center;
        section.format.line_height = Some(line_height);
    }
    ui.fonts(|fonts| fonts.layout_job(job))
}

#[inline]
pub fn galley_top_y_at_center(galley: &Galley, center_y: f32) -> f32 {
    center_y - galley.mesh_bounds.center().y
}

pub fn paint_galley_row_centered(
    ui: &Ui,
    left_x: f32,
    row_center_y: f32,
    galley: Arc<Galley>,
    color: Color32,
) {
    ui.painter().galley(
        Pos2::new(left_x, galley_top_y_at_center(&galley, row_center_y)),
        galley,
        color,
    );
}

pub fn paint_galley_row_centered_clipped(
    ui: &Ui,
    clip: Rect,
    left_x: f32,
    row_center_y: f32,
    galley: Arc<Galley>,
    color: Color32,
) {
    ui.painter()
        .with_clip_rect(clip)
        .galley(
            Pos2::new(left_x, galley_top_y_at_center(&galley, row_center_y)),
            galley,
            color,
        );
}

/// 图标 + 文字同一垂直中心线。返回内容右边界 x。
pub fn paint_icon_text_row(
    ui: &Ui,
    row_center_y: f32,
    left_x: f32,
    icon: Icon,
    icon_size: f32,
    gap: f32,
    text: &str,
    font_id: FontId,
    color: Color32,
    line_height: f32,
) -> f32 {
    icons::paint_icon(
        ui,
        icon,
        Pos2::new(left_x + icon_size / 2.0, row_center_y),
        icon_size,
        color,
    );
    let text_x = left_x + icon_size + gap;
    let galley = layout_vcentered_galley(ui, text.to_owned(), font_id, color, line_height);
    let w = galley.size().x;
    paint_galley_row_centered(ui, text_x, row_center_y, galley, color);
    text_x + w
}
