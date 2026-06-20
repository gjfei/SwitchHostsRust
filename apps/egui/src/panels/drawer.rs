//! 右侧 SideDrawer 共享壳层（编辑 / 历史等抽屉复用）。

use eframe::egui::{self, Color32, RichText, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme::{self, layout};

pub use layout::{
    DRAWER_BTN_H, DRAWER_BTN_MIN_W, DRAWER_INPUT_H_PAD, DRAWER_INPUT_HEIGHT,
};

/// 抽屉几何（对齐 `edit_hosts` / `SideDrawer` 右侧 inset）。
pub struct SideDrawerGeometry {
    pub backdrop_rect: egui::Rect,
    pub drawer_rect: egui::Rect,
    pub area_rect: egui::Rect,
    pub shadow_margin: egui::Margin,
}

pub fn side_drawer_geometry(ctx: &egui::Context, width: f32) -> SideDrawerGeometry {
    let t = theme::app(ctx);
    let shadow = t.drawer_shadow();
    let screen = ctx.input(|i| i.content_rect());
    let backdrop_rect = screen;
    let drawer_rect = {
        let inset = screen.shrink2(Vec2::splat(layout::DRAWER_OFFSET));
        egui::Rect::from_min_max(
            egui::pos2(inset.right() - width, inset.top()),
            egui::pos2(inset.right(), inset.bottom()),
        )
    };
    let shadow_margin = {
        let sm = shadow.margin();
        egui::Margin {
            left: sm.left.ceil() as i8,
            right: sm.right.ceil() as i8,
            top: sm.top.ceil() as i8,
            bottom: sm.bottom.ceil() as i8,
        }
    };
    let area_rect = egui::Rect::from_min_max(
        egui::pos2(
            drawer_rect.min.x - shadow_margin.left as f32,
            drawer_rect.min.y - shadow_margin.top as f32,
        ),
        egui::pos2(
            drawer_rect.max.x + shadow_margin.right as f32,
            drawer_rect.max.y + shadow_margin.bottom as f32,
        ),
    );
    SideDrawerGeometry {
        backdrop_rect,
        drawer_rect,
        area_rect,
        shadow_margin,
    }
}

pub fn paint_side_drawer_backdrop(ctx: &egui::Context, backdrop_id: &str, backdrop_rect: egui::Rect) {
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new(backdrop_id),
    ))
    .rect_filled(backdrop_rect, 0.0, Color32::from_black_alpha(100));
}

pub fn backdrop_dismiss_clicked(
    ctx: &egui::Context,
    backdrop_rect: egui::Rect,
    drawer_rect: egui::Rect,
    allow: bool,
) -> bool {
    allow
        && ctx.input(|i| {
            i.pointer.primary_clicked()
                && i.pointer.interact_pos().is_some_and(|pos| {
                    backdrop_rect.contains(pos) && !drawer_rect.contains(pos)
                })
        })
}

/// 标题栏 + 关闭按钮（hover 圆角底）。
pub fn draw_drawer_header(ui: &mut Ui, icon: Icon, title: &str, close_id: &str) -> bool {
    let t = theme::app(ui.ctx());
    let mut close = false;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, layout::DRAWER_HEADER_HEIGHT), Sense::hover());
    let cy = rect.center().y;
    text_align::paint_icon_text_row(
        ui,
        cy,
        rect.left() + layout::DRAWER_PAD,
        icon,
        18.0,
        10.0,
        title,
        ui_font_id(16.0),
        t.text,
        18.0,
    );
    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - layout::DRAWER_PAD - 14.0, cy),
        Vec2::splat(28.0),
    );
    let close_resp = ui.interact(close_rect, ui.id().with(close_id), Sense::click());
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_rect, t.corner_icon(), t.icon_hover_bg);
    }
    icons::paint_icon(
        ui,
        Icon::X,
        close_rect.center(),
        18.0,
        t.nav_icon_inactive_tint,
    );
    if close_resp.clicked() {
        close = true;
    }
    close
}

pub fn drawer_text_button(
    ui: &mut Ui,
    label: &str,
    fill: Color32,
    stroke: Stroke,
    text_color: Color32,
    enabled: bool,
) -> egui::Response {
    let t = theme::app(ui.ctx());
    let (rect, mut response) = ui.allocate_at_least(
        Vec2::new(layout::DRAWER_BTN_MIN_W, layout::DRAWER_BTN_H),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if ui.is_rect_visible(rect) {
        let (fill, stroke, text_color) = if enabled {
            (fill, stroke, text_color)
        } else {
            (
                t.window_bg,
                Stroke::new(1.0, t.input_border),
                t.weak_text,
            )
        };
        ui.painter().rect(
            rect,
            t.corner_input(),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        let galley = text_align::layout_vcentered_galley(
            ui,
            label.to_owned(),
            ui_font_id(14.0),
            text_color,
            ICON_ROW_LINE_HEIGHT,
        );
        text_align::paint_galley_row_centered(
            ui,
            rect.center().x - galley.size().x / 2.0,
            rect.center().y,
            galley,
            text_color,
        );
    }
    if !enabled {
        response = ui.interact(rect, response.id, Sense::hover());
    }
    response
}

pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    let t = theme::app(ui.ctx());
    drawer_text_button(ui, label, t.accent, Stroke::NONE, t.text_selected, true)
}

pub fn outline_button(ui: &mut Ui, label: &str) -> egui::Response {
    outline_button_enabled(ui, label, true)
}

pub fn outline_button_enabled(ui: &mut Ui, label: &str, enabled: bool) -> egui::Response {
    let t = theme::app(ui.ctx());
    drawer_text_button(
        ui,
        label,
        t.editor_bg,
        Stroke::new(1.0, t.accent),
        t.accent,
        enabled,
    )
}

pub fn outline_button_with_icon(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    danger: bool,
    enabled: bool,
) -> egui::Response {
    let t = theme::app(ui.ctx());
    let stroke = if danger {
        Stroke::new(1.0, t.accent)
    } else {
        Stroke::new(1.0, t.input_border)
    };
    let text_color = if enabled {
        if danger {
            t.accent
        } else {
            t.text
        }
    } else {
        t.weak_text
    };
    let galley = text_align::layout_vcentered_galley(
        ui,
        label.to_owned(),
        ui_font_id(14.0),
        text_color,
        ICON_ROW_LINE_HEIGHT,
    );
    let content_w = icons::DEFAULT_ICON_SIZE + 8.0 + galley.size().x;
    let btn_w = (content_w + layout::DRAWER_INPUT_H_PAD * 2.0).max(layout::DRAWER_BTN_MIN_W);
    let (rect, mut response) = ui.allocate_at_least(Vec2::new(btn_w, layout::DRAWER_BTN_H), if enabled {
        Sense::click()
    } else {
        Sense::hover()
    });
    if ui.is_rect_visible(rect) {
        let fill = if enabled {
            t.editor_bg
        } else {
            t.window_bg
        };
        ui.painter().rect(
            rect,
            t.corner_input(),
            fill,
            if enabled {
                stroke
            } else {
                Stroke::new(1.0, t.input_border)
            },
            egui::StrokeKind::Inside,
        );
        text_align::paint_icon_text_row(
            ui,
            rect.center().y,
            rect.left() + layout::DRAWER_INPUT_H_PAD,
            icon,
            icons::DEFAULT_ICON_SIZE,
            8.0,
            label,
            ui_font_id(14.0),
            text_color,
            ICON_ROW_LINE_HEIGHT,
        );
    }
    if !enabled {
        response = ui.interact(rect, response.id, Sense::hover());
    }
    response
}

fn with_flat_combo_style<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let t = theme::app(ui.ctx());
    let style = ui.style_mut();
    let saved_inactive = style.visuals.widgets.inactive;
    let saved_hovered = style.visuals.widgets.hovered;
    let saved_open = style.visuals.widgets.open;
    let saved_btn_pad = style.spacing.button_padding;

    let text_stroke = Stroke::new(1.0, t.text);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.open,
    ] {
        widget.weak_bg_fill = Color32::TRANSPARENT;
        widget.bg_fill = Color32::TRANSPARENT;
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke = text_stroke;
    }
    style.spacing.button_padding = egui::vec2(0.0, 0.0);

    let result = add(ui);

    let style = ui.style_mut();
    style.visuals.widgets.inactive = saved_inactive;
    style.visuals.widgets.hovered = saved_hovered;
    style.visuals.widgets.open = saved_open;
    style.spacing.button_padding = saved_btn_pad;

    result
}

/// Mantine `Select`：与 TextInput 同高（36px）。
pub fn drawer_select(
    ui: &mut Ui,
    id_salt: &'static str,
    width: f32,
    selected: &str,
    menu: impl FnOnce(&mut Ui),
) {
    let t = theme::app(ui.ctx());
    let combo_id = egui::Id::new(id_salt);
    let is_open = egui::ComboBox::is_open(ui.ctx(), combo_id);
    let inner_w = (width - layout::DRAWER_INPUT_H_PAD * 2.0).max(0.0);

    let (row_rect, _) = ui.allocate_exact_size(
        Vec2::new(width, layout::DRAWER_INPUT_HEIGHT),
        Sense::hover(),
    );

    ui.painter().rect(
        row_rect,
        t.corner_input(),
        t.editor_bg,
        Stroke::new(
            1.0,
            if is_open {
                t.accent
            } else {
                t.input_border
            },
        ),
        egui::StrokeKind::Inside,
    );

    let inner_rect = row_rect.shrink2(egui::vec2(layout::DRAWER_INPUT_H_PAD, 0.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_height(inner_rect.height());
            with_flat_combo_style(ui, |ui| {
                egui::ComboBox::from_id_salt(id_salt)
                    .width(inner_w)
                    .selected_text(
                        RichText::new(selected)
                            .size(14.0)
                            .color(t.text)
                            .font(ui_font_id(14.0)),
                    )
                    .show_ui(ui, menu);
            });
        });
    });
}

/// Mantine Select 下拉项：选中项浅红底 + 主色文字。
pub fn drawer_select_option(ui: &mut Ui, current: &mut u64, value: u64, label: &str) {
    let t = theme::app(ui.ctx());
    let selected = *current == value;
    let row_h = 28.0;
    let text_color = if selected { t.accent } else { t.text };
    let (rect, mut response) =
        ui.allocate_at_least(Vec2::new(ui.available_width(), row_h), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if selected {
            t.tree_hover
        } else if response.hovered() {
            t.icon_hover_bg
        } else {
            t.editor_bg
        };
        ui.painter()
            .rect_filled(rect, t.corner_input(), fill);
        let galley = text_align::layout_vcentered_galley(
            ui,
            label.to_owned(),
            ui_font_id(14.0),
            text_color,
            row_h,
        );
        text_align::paint_galley_row_centered(ui, rect.left() + 8.0, rect.center().y, galley, text_color);
    }
    if response.clicked() && *current != value {
        *current = value;
        response.mark_changed();
    }
}

/// 抽屉白底 + 圆角 + 边框 + 阴影 Frame。
pub fn drawer_frame(ctx: &egui::Context) -> egui::Frame {
    let t = theme::app(ctx);
    egui::Frame::new()
        .fill(t.editor_bg)
        .corner_radius(t.corner_drawer())
        .stroke(Stroke::new(1.0, t.border))
}

/// 带阴影的抽屉 Frame（对齐 SideDrawer）。
pub fn drawer_panel_frame(ctx: &egui::Context) -> egui::Frame {
    let t = theme::app(ctx);
    drawer_frame(ctx).shadow(t.drawer_shadow())
}

/// 确认弹窗最大宽度（内容自适应，不超过此值）。
const CONFIRM_MODAL_MAX_WIDTH: f32 = 400.0;
const CONFIRM_MODAL_MIN_WIDTH: f32 = 280.0;
/// 对齐 `ConfirmModal` `<Text mb="lg">`（`--mantine-spacing-lg`）。
const CONFIRM_MODAL_MESSAGE_GAP: f32 = 16.0;
const CONFIRM_MODAL_BTN_GAP: f32 = 12.0;

const MODAL_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 8],
    blur: 24,
    spread: 0,
    color: Color32::from_black_alpha(40),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmModalResult {
    #[default]
    None,
    Cancelled,
    Confirmed,
}

/// 居中确认弹窗（对齐 `ConfirmModal.tsx`：标题、正文、`outline` 取消、危险确认）。
pub fn draw_confirm_modal(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    message: &str,
    confirm_label: &str,
    _danger: bool,
) -> ConfirmModalResult {
    let t = theme::app(ctx);
    let mut result = ConfirmModalResult::None;
    let screen = ctx.input(|i| i.content_rect());

    let backdrop_id = egui::Id::new(format!("{id}_backdrop"));
    egui::Area::new(backdrop_id)
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(100));
            if ui
                .interact(screen, backdrop_id, Sense::click())
                .clicked()
            {
                result = ConfirmModalResult::Cancelled;
            }
        });

    let modal_width = confirm_modal_width(ctx, title, message, confirm_label);

    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(modal_width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    egui::Frame {
                        fill: t.editor_bg,
                        corner_radius: t.corner_drawer(),
                        shadow: MODAL_SHADOW,
                        inner_margin: egui::Margin::same(layout::DRAWER_PAD as i8),
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(title)
                                .size(16.0)
                                .strong()
                                .color(t.text),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(message)
                                .size(14.0)
                                .color(t.text),
                        );
                        ui.add_space(CONFIRM_MODAL_MESSAGE_GAP);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if drawer_text_button(
                                    ui,
                                    confirm_label,
                                    t.accent,
                                    Stroke::NONE,
                                    t.text_selected,
                                    true,
                                )
                                .clicked()
                                {
                                    result = ConfirmModalResult::Confirmed;
                                }
                                ui.add_space(CONFIRM_MODAL_BTN_GAP);
                                if drawer_text_button(
                                    ui,
                                    "取消",
                                    t.editor_bg,
                                    Stroke::new(1.0, t.input_border),
                                    t.text,
                                    true,
                                )
                                .clicked()
                                {
                                    result = ConfirmModalResult::Cancelled;
                                }
                            },
                        );
                    });
                },
            );
        });

    result
}

fn measure_text_width(ctx: &egui::Context, text: &str, font_id: egui::FontId) -> f32 {
    ctx.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(
                text.to_owned(),
                font_id,
                Color32::PLACEHOLDER,
            )
            .size()
            .x
    })
}

fn confirm_modal_width(ctx: &egui::Context, title: &str, message: &str, confirm_label: &str) -> f32 {
    let inner_max = CONFIRM_MODAL_MAX_WIDTH - layout::DRAWER_PAD * 2.0;
    let title_w = measure_text_width(ctx, title, ui_font_id(16.0));
    let message_w = measure_text_width(ctx, message, ui_font_id(14.0));
    let confirm_w = measure_text_width(ctx, confirm_label, ui_font_id(14.0));
    let buttons_row_w =
        (confirm_w + layout::DRAWER_INPUT_H_PAD * 2.0).max(layout::DRAWER_BTN_MIN_W) * 2.0
            + CONFIRM_MODAL_BTN_GAP;
    let inner_w = title_w
        .max(message_w.min(inner_max))
        .max(buttons_row_w)
        .clamp(CONFIRM_MODAL_MIN_WIDTH, inner_max);
    inner_w + layout::DRAWER_PAD * 2.0
}
