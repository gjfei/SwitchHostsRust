//! Mantine 风格弹出菜单（`ConfigMenu` / `PopupMenu` 共用）。

use eframe::egui::{
    self, Align, Align2, Area, Color32, CornerRadius, Frame, Id, Key, Layout, Order, PointerButton,
    Pos2, Rect, Response, Sense, Stroke, Ui, UiKind, Vec2,
};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::text_align;
use crate::theme::{self, layout};

fn context_menu_popup() -> Id {
    Id::new("app_context_menu")
}

fn context_menu_owner_key() -> Id {
    Id::new("app_context_menu_owner")
}

/// 打开菜单时的 `input.time`，同一帧内不触发外部关闭。
fn context_menu_open_time_key() -> Id {
    Id::new("app_context_menu_open_time")
}

const MENU_ITEM_H: f32 = 36.0;
const MENU_ITEM_PAD_X: f32 = 12.0;
const MENU_ICON_SIZE: f32 = 16.0;
const MENU_ICON_GAP: f32 = 8.0;
const MENU_FONT_SIZE: f32 = 14.0;
const MENU_INNER_PAD: f32 = 4.0;
const MENU_ANCHOR_GAP: f32 = 4.0;

fn popup_pos_id(popup_id: Id) -> Id {
    popup_id.with("app_menu_pos")
}

fn menu_width_key(popup_id: Id) -> Id {
    popup_id.with("app_menu_width")
}

pub fn is_menu_open(ui: &Ui, popup_id: Id) -> bool {
    ui.memory(|mem| mem.is_popup_open(popup_id))
}

pub fn close_menu(ui: &Ui, popup_id: Id) {
    ui.ctx().data_mut(|d| {
        d.remove_temp::<Pos2>(popup_pos_id(popup_id));
        d.remove_temp::<f32>(menu_width_key(popup_id));
        if popup_id == context_menu_popup() {
            d.remove::<Id>(context_menu_owner_key());
            d.remove::<f64>(context_menu_open_time_key());
        }
    });
    ui.memory_mut(|mem| mem.close_popup());
}

fn invalidate_menu_width(ui: &Ui, popup_id: Id) {
    ui.ctx()
        .data_mut(|d| d.remove_temp::<f32>(menu_width_key(popup_id)));
}

/// 左键点击锚点：切换菜单（对齐 ConfigMenu）。
pub fn toggle_click_menu(ui: &Ui, popup_id: Id, anchor: &Response) {
    if anchor.clicked() {
        invalidate_menu_width(ui, popup_id);
        ui.ctx()
            .data_mut(|d| d.remove_temp::<Pos2>(popup_pos_id(popup_id)));
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }
}

/// 右键打开菜单于指针位置（对齐 PopupMenu）。
pub fn open_context_menu(_ui: &Ui, response: &Response) {
    if !response.secondary_clicked() {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let ctx = &response.ctx;
    let global_pos = layer_to_global(ctx, response.layer_id, pos);
    let open_time = ctx.input(|i| i.time);
    ctx.data_mut(|d| {
        d.remove_temp::<f32>(menu_width_key(context_menu_popup()));
        d.insert_temp(popup_pos_id(context_menu_popup()), global_pos);
        d.insert_temp(context_menu_owner_key(), response.id);
        d.insert_temp(context_menu_open_time_key(), open_time);
    });
    ctx.memory_mut(|mem| mem.open_popup(context_menu_popup()));
}

/// 绘制已打开的右键菜单；仅 `owner` 与打开时的控件一致时才绘制。
pub fn show_context_menu_if_open<R>(
    ui: &Ui,
    owner: &Response,
    mut add_contents: impl FnMut(&mut dyn MenuContents) -> R,
) -> Option<R> {
    if !is_menu_open(ui, context_menu_popup()) {
        return None;
    }
    let is_owner = ui.ctx().data(|d| d.get_temp::<Id>(context_menu_owner_key()))
        == Some(owner.id);
    if !is_owner {
        return None;
    }
    show_popup_menu(
        ui,
        context_menu_popup(),
        None,
        Some(owner),
        &mut add_contents,
    )
}

fn layer_to_global(ctx: &egui::Context, layer_id: egui::LayerId, pos: Pos2) -> Pos2 {
    ctx.layer_transform_to_global(layer_id)
        .map(|t| t * pos)
        .unwrap_or(pos)
}

fn measure_item_width(ctx: &egui::Context, icon: Option<Icon>, label: &str) -> f32 {
    let font_id = ui_font_id(MENU_FONT_SIZE);
    let text_w = ctx
        .fonts(|fonts| fonts.layout_no_wrap(label.to_owned(), font_id, Color32::PLACEHOLDER))
        .size()
        .x;
    let icon_part = if icon.is_some() {
        MENU_ICON_SIZE + MENU_ICON_GAP
    } else {
        0.0
    };
    (MENU_ITEM_PAD_X * 2.0 + icon_part + text_w).ceil()
}

/// 菜单内容构建（测量 / 绘制共用接口）。
pub trait MenuContents {
    fn item(&mut self, label: &str) -> bool;
    fn item_if(&mut self, show: bool, label: &str) -> bool;
    fn item_icon(&mut self, icon: Icon, label: &str) -> bool;
    fn divider(&mut self);
}

struct MenuMeasurer<'a> {
    ctx: &'a egui::Context,
    max_width: f32,
}

impl MenuContents for MenuMeasurer<'_> {
    fn item(&mut self, label: &str) -> bool {
        self.max_width = self
            .max_width
            .max(measure_item_width(self.ctx, None, label));
        false
    }

    fn item_if(&mut self, show: bool, label: &str) -> bool {
        if show {
            self.item(label)
        } else {
            false
        }
    }

    fn item_icon(&mut self, icon: Icon, label: &str) -> bool {
        self.max_width = self
            .max_width
            .max(measure_item_width(self.ctx, Some(icon), label));
        false
    }

    fn divider(&mut self) {}
}

/// 菜单已打开时绘制；`anchor` 为左键锚点（设置菜单）。
pub fn show_menu_if_open<R>(
    ui: &Ui,
    popup_id: Id,
    anchor: Option<&Response>,
    mut add_contents: impl FnMut(&mut dyn MenuContents) -> R,
) -> Option<R> {
    if !is_menu_open(ui, popup_id) {
        return None;
    }
    show_popup_menu(ui, popup_id, anchor, None, &mut add_contents)
}

fn show_popup_menu<R>(
    ui: &Ui,
    popup_id: Id,
    anchor: Option<&Response>,
    context_owner: Option<&Response>,
    add_contents: &mut impl FnMut(&mut dyn MenuContents) -> R,
) -> Option<R> {
    let (pos, pivot) = if let Some(pointer) =
        ui.ctx().data(|d| d.get_temp::<Pos2>(popup_pos_id(popup_id)))
    {
        (pointer, Align2::LEFT_TOP)
    } else if let Some(anchor) = anchor {
        let mut pos = anchor.rect.right_bottom() + Vec2::new(MENU_ANCHOR_GAP, 0.0);
        pos = layer_to_global(ui.ctx(), anchor.layer_id, pos);
        (pos, Align2::LEFT_BOTTOM)
    } else {
        close_menu(ui, popup_id);
        return None;
    };

    let content_width = if let Some(w) = ui.ctx().data(|d| d.get_temp::<f32>(menu_width_key(popup_id)))
    {
        w
    } else {
        let mut measure = MenuMeasurer {
            ctx: ui.ctx(),
            max_width: 0.0,
        };
        let _ = add_contents(&mut measure);
        let w = measure.max_width;
        ui.ctx()
            .data_mut(|d| d.insert_temp(menu_width_key(popup_id), w));
        w
    };

    let area_response = Area::new(popup_id)
        .kind(UiKind::Popup)
        .order(Order::Foreground)
        .fixed_pos(pos)
        .pivot(pivot)
        .constrain(true)
        .show(ui.ctx(), |inner| {
            menu_frame(inner.ctx())
                .show(inner, |inner| {
                    inner.set_width(content_width + MENU_INNER_PAD * 2.0);
                    inner.spacing_mut().item_spacing = Vec2::ZERO;
                    inner.with_layout(Layout::top_down(Align::LEFT), |inner| {
                        inner.set_width(content_width);
                        let mut menu = AppMenuUi {
                            ui: inner,
                            host: ui,
                            popup_id,
                            content_width,
                        };
                        add_contents(&mut menu)
                    })
                    .inner
                })
                .inner
        });

    if should_close_menu(
        ui,
        popup_id,
        anchor,
        context_owner,
        area_response.response.rect,
    ) {
        close_menu(ui, popup_id);
    }

    Some(area_response.inner)
}

fn should_close_menu(
    ui: &Ui,
    popup_id: Id,
    anchor: Option<&Response>,
    context_owner: Option<&Response>,
    menu_rect: Rect,
) -> bool {
    if ui.input(|i| i.key_pressed(Key::Escape)) {
        return true;
    }

    // 布局 pass 中 Area 尺寸可能为 0，此时不做外部点击判定。
    if menu_rect.width() <= 0.0 || menu_rect.height() <= 0.0 {
        return false;
    }

    if context_owner.is_some() || popup_id == context_menu_popup() {
        let open_time = ui
            .ctx()
            .data(|d| d.get_temp::<f64>(context_menu_open_time_key()));
        if open_time.is_some_and(|t| ui.input(|i| i.time) == t) {
            return false;
        }
        return ui.input(|i| {
            i.pointer.button_clicked(PointerButton::Primary)
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pos| !menu_rect.contains(pos))
        });
    }

    if let Some(a) = anchor {
        return a.clicked_elsewhere() && pointer_primary_clicked_outside(ui, menu_rect);
    }

    false
}

fn pointer_primary_clicked_outside(ui: &Ui, rect: Rect) -> bool {
    ui.input(|i| {
        i.pointer.button_clicked(PointerButton::Primary)
            && i.pointer
                .interact_pos()
                .is_some_and(|pos| !rect.contains(pos))
    })
}

fn menu_frame(ctx: &egui::Context) -> Frame {
    let t = theme::app(ctx);
    Frame::new()
        .fill(t.window_bg)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(t.corner_input())
        .inner_margin(egui::Margin::same(MENU_INNER_PAD as i8))
        .shadow(t.drawer_shadow())
}

/// 弹出菜单内容构建器。
pub struct AppMenuUi<'a> {
    ui: &'a mut Ui,
    host: &'a Ui,
    popup_id: Id,
    content_width: f32,
}

impl MenuContents for AppMenuUi<'_> {
    fn item(&mut self, label: &str) -> bool {
        self.item_impl(None, label, true)
    }

    fn item_if(&mut self, show: bool, label: &str) -> bool {
        if show {
            self.item(label)
        } else {
            false
        }
    }

    fn item_icon(&mut self, icon: Icon, label: &str) -> bool {
        self.item_impl(Some(icon), label, true)
    }

    fn divider(&mut self) {
        draw_menu_divider(self.ui, self.content_width);
    }
}

impl AppMenuUi<'_> {
    fn item_impl(&mut self, icon: Option<Icon>, label: &str, enabled: bool) -> bool {
        let clicked =
            draw_menu_item(self.ui, icon, label, enabled, self.content_width).clicked();
        if clicked {
            close_menu(self.host, self.popup_id);
        }
        clicked
    }
}

fn draw_menu_item(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    enabled: bool,
    width: f32,
) -> Response {
    let t = theme::app(ui.ctx());
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, MENU_ITEM_H), sense);
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let text_color = if enabled { t.text } else { t.weak_text };
    if enabled && response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(layout::CHECKBOX_RADIUS as u8),
            t.icon_hover_bg,
        );
    }

    let text_x = if let Some(icon) = icon {
        let icon_center = egui::pos2(
            rect.left() + MENU_ITEM_PAD_X + MENU_ICON_SIZE * 0.5,
            rect.center().y,
        );
        icons::paint_icon(ui, icon, icon_center, MENU_ICON_SIZE, text_color);
        rect.left() + MENU_ITEM_PAD_X + MENU_ICON_SIZE + MENU_ICON_GAP
    } else {
        rect.left() + MENU_ITEM_PAD_X
    };

    let galley = text_align::layout_vcentered_galley(
        ui,
        label.to_owned(),
        ui_font_id(MENU_FONT_SIZE),
        text_color,
        MENU_ITEM_H,
    );
    text_align::paint_galley_row_centered(ui, text_x, rect.center().y, galley, text_color);

    response
}

fn draw_menu_divider(ui: &mut Ui, width: f32) {
    let t = theme::app(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 9.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, t.separator));
}
