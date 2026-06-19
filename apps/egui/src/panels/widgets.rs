//! 通用 UI 控件。

use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::theme::{self, layout};

/// 滑块与轨道边缘的间距
const SWITCH_KNOB_INSET: f32 = 1.5;

pub fn toggle_switch(ui: &mut egui::Ui, on: bool) -> egui::Response {
    let size = Vec2::new(layout::SWITCH_WIDTH, layout::SWITCH_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    paint_toggle_switch(ui, rect, on);
    response
}

/// 在固定矩形内绘制并检测开关点击（用于树行等自定义 layout）。
pub fn toggle_switch_at(
    ui: &mut Ui,
    rect: egui::Rect,
    id: egui::Id,
    on: bool,
) -> egui::Response {
    let response = ui.interact(rect, id, Sense::click());
    paint_toggle_switch(ui, rect, on);
    response
}

fn paint_toggle_switch(ui: &Ui, rect: egui::Rect, on: bool) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let t = theme::app(ui.ctx());
    let radius = CornerRadius::same((layout::SWITCH_HEIGHT * 0.5).round() as u8);
    let (track, knob, ring) = if on {
        (t.switch_on_track, t.switch_on_knob, t.switch_on_knob)
    } else {
        (t.switch_off_track, t.switch_off_knob, t.switch_off_knob)
    };
    ui.painter().rect_filled(rect, radius, track);
    ui.painter()
        .rect_stroke(rect, radius, Stroke::new(1.0, ring), StrokeKind::Inside);
    let knob_r = (layout::SWITCH_HEIGHT - SWITCH_KNOB_INSET * 2.0) * 0.5;
    let cy = rect.center().y;
    let cx = if on {
        rect.right() - SWITCH_KNOB_INSET - knob_r
    } else {
        rect.left() + SWITCH_KNOB_INSET + knob_r
    };
    ui.painter()
        .circle_filled(Pos2::new(cx, cy), knob_r, knob);
}

pub fn format_bytes(n: usize) -> String {
    if n < 1024 {
        format!("{n} B")
    } else {
        format!("{:.1} KB", n as f64 / 1024.0)
    }
}

/// 按像素宽度截断并追加省略号（对齐 `.label { text-overflow: ellipsis }`）。
pub fn ellipsize_text(ui: &Ui, text: &str, font_id: FontId, max_width: f32) -> String {
    if max_width <= 0.0 {
        return String::new();
    }
    let measure = |s: &str| {
        ui.fonts(|fonts| {
            fonts
                .layout_no_wrap(s.to_owned(), font_id.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    if measure(text) <= max_width {
        return text.to_owned();
    }
    const ELLIPSIS: &str = "…";
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        let candidate: String = chars.iter().collect::<String>() + ELLIPSIS;
        if measure(&candidate) <= max_width {
            return candidate;
        }
        chars.pop();
    }
    ELLIPSIS.to_owned()
}
