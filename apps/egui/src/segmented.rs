//! Mantine `SegmentedControl` 风格分段选择（抽屉 / 表单通用）。

use eframe::egui::{self, Color32, Sense, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme;

/// 对齐 Mantine `SegmentedControl` size md（含 root padding）。
pub const HEIGHT: f32 = 36.0;
const INNER: f32 = 4.0;
pub const ICON_SIZE: f32 = 16.0;
pub const ICON_GAP: f32 = 4.0;
const SEGMENT_PAD_X: f32 = 12.0;
const MIN_SEGMENT_W: f32 = 36.0;

#[derive(Clone, Copy, Debug)]
pub struct SegmentedConfig {
    pub enabled: bool,
}

impl Default for SegmentedConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl SegmentedConfig {
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

/// 通用分段控件；点击后更新 `selected` 并返回是否变化。
pub fn segmented_control<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    selected: &mut T,
    options: &[T],
    config: SegmentedConfig,
    mut segment_width: impl FnMut(&Ui, &T) -> f32,
    mut render: impl FnMut(&Ui, &T, bool, Color32, egui::Rect),
) -> bool {
    let before = *selected;
    let selected_idx = options
        .iter()
        .position(|opt| *opt == *selected)
        .unwrap_or(0);
    let t = theme::app(ui.ctx());
    let label_tint = if config.enabled {
        t.text
    } else {
        t.weak_text
    };
    let widths: Vec<f32> = options
        .iter()
        .map(|opt| segment_width(ui, opt))
        .collect();

    if let Some(clicked) = segmented_bar(ui, id, &widths, selected_idx, config, |ui, i, active, seg_rect| {
        render(ui, &options[i], active, label_tint, seg_rect);
    }) {
        *selected = options[clicked];
    }
    before != *selected
}

/// 纯文本分段；值与标签一一对应，返回新选中的 value（若有变化）。
pub fn segmented_text_values<'a>(
    ui: &mut Ui,
    id: &str,
    current: &str,
    values: &[&'a str],
    labels: &[&'a str],
    config: SegmentedConfig,
) -> Option<&'a str> {
    debug_assert_eq!(values.len(), labels.len());
    if values.is_empty() {
        return None;
    }
    let selected_idx = values.iter().position(|v| *v == current).unwrap_or(0);
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| measure_text_segment_width(ui, label))
        .collect();
    let t = theme::app(ui.ctx());
    let clicked = segmented_bar(ui, id, &widths, selected_idx, config, |ui, i, active, seg_rect| {
        let color = if active {
            t.text
        } else {
            t.weak_text
        };
        paint_segment_text(ui, seg_rect, labels[i], color);
    })?;
    let next = values[clicked];
    if next == current {
        None
    } else {
        Some(next)
    }
}

/// 纯文本分段的建议宽度。
pub fn measure_text_segment_width(ui: &Ui, text: &str) -> f32 {
    let text_w = measure_text_width(ui, text);
    (text_w + SEGMENT_PAD_X * 2.0).max(MIN_SEGMENT_W)
}

/// 图标 + 文本分段的建议宽度。
pub fn measure_icon_text_segment_width(ui: &Ui, _icon: Icon, text: &str) -> f32 {
    let content_w = ICON_SIZE + ICON_GAP + measure_text_width(ui, text);
    (content_w + SEGMENT_PAD_X * 2.0).max(MIN_SEGMENT_W)
}

fn measure_text_width(ui: &Ui, text: &str) -> f32 {
    ui.fonts(|fonts| {
        fonts
            .layout_no_wrap(text.to_owned(), ui_font_id(14.0), Color32::WHITE)
            .size()
            .x
    })
}

fn segmented_bar(
    ui: &mut Ui,
    id: &str,
    widths: &[f32],
    selected_idx: usize,
    config: SegmentedConfig,
    mut paint_segment: impl FnMut(&Ui, usize, bool, egui::Rect),
) -> Option<usize> {
    let t = theme::app(ui.ctx());
    if widths.is_empty() {
        return None;
    }
    let mut clicked = None;
    let row_h = HEIGHT - INNER * 2.0;
    let total_w: f32 = widths.iter().sum();

    ui.push_id(id, |ui| {
        egui::Frame::new()
            .fill(t.segmented_bg)
            .corner_radius(t.corner_input())
            .inner_margin(INNER)
            .show(ui, |ui| {
                let (row_rect, _) =
                    ui.allocate_exact_size(Vec2::new(total_w, row_h), Sense::hover());

                let mut seg_x = row_rect.min.x;
                for (i, &seg_w) in widths.iter().enumerate() {
                    let seg_rect = egui::Rect::from_min_size(
                        egui::pos2(seg_x, row_rect.min.y),
                        Vec2::new(seg_w, row_h),
                    );
                    seg_x += seg_w;

                    let active = i == selected_idx;
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(seg_rect), |ui| {
                        if active {
                            ui.painter().rect_filled(
                                seg_rect.translate(egui::vec2(0.0, 1.0)),
                                t.corner_input(),
                                Color32::from_black_alpha(if t.dark { 40 } else { 12 }),
                            );
                            ui.painter()
                                .rect_filled(seg_rect, t.corner_input(), t.editor_bg);
                        }

                        paint_segment(ui, i, active, seg_rect);

                        let resp = ui.interact(
                            seg_rect,
                            ui.id().with(i),
                            if config.enabled {
                                Sense::click()
                            } else {
                                Sense::hover()
                            },
                        );
                        if config.enabled && resp.clicked() && !active {
                            clicked = Some(i);
                        }
                    });
                }
            });
    });
    clicked
}

/// 在分段内居中绘制纯文本标签。
pub fn paint_segment_text(ui: &Ui, seg_rect: egui::Rect, text: &str, tint: Color32) {
    let galley = text_align::layout_vcentered_galley(
        ui,
        text.to_owned(),
        ui_font_id(14.0),
        tint,
        ICON_SIZE,
    );
    let center = seg_rect.center();
    text_align::paint_galley_row_centered(
        ui,
        center.x - galley.size().x * 0.5,
        center.y,
        galley,
        tint,
    );
}

/// 在分段内居中绘制图标 + 文本。
pub fn paint_segment_icon_text(ui: &Ui, seg_rect: egui::Rect, icon: Icon, text: &str, tint: Color32) {
    let galley = text_align::layout_vcentered_galley(
        ui,
        text.to_owned(),
        ui_font_id(14.0),
        tint,
        ICON_SIZE,
    );
    let content_w = ICON_SIZE + ICON_GAP + galley.size().x;
    let center_y = seg_rect.center().y;
    let mut x = seg_rect.center().x - content_w / 2.0;

    icons::paint_icon(
        ui,
        icon,
        egui::pos2(x + ICON_SIZE / 2.0, center_y),
        ICON_SIZE,
        tint,
    );
    x += ICON_SIZE + ICON_GAP;
    text_align::paint_galley_row_centered(ui, x, center_y, galley, tint);
}

/// 偏好设置等场景：居中绘制 14px 文本（弱/强色由 active 决定）。
pub fn paint_segment_caption(ui: &Ui, seg_rect: egui::Rect, text: &str, active: bool) {
    let t = theme::app(ui.ctx());
    let color = if active {
        t.text
    } else {
        t.weak_text
    };
    let galley = text_align::layout_vcentered_galley(
        ui,
        text.to_owned(),
        ui_font_id(14.0),
        color,
        ICON_ROW_LINE_HEIGHT,
    );
    text_align::paint_galley_row_centered(
        ui,
        seg_rect.center().x - galley.size().x * 0.5,
        seg_rect.center().y,
        galley,
        color,
    );
}
