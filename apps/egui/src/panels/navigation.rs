//! 最左侧图标导航栏（对齐 `LeftSidebar`）。

use eframe::egui::{self, Color32, FontId, Sense, Ui, Vec2};

use crate::icons::{self, Icon};
use crate::theme::{self, layout};

/// 当前主导航视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavView {
    #[default]
    Hosts,
    Trash,
}

/// 导航栏触发的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavAction {
    pub open_search: bool,
    pub open_history: bool,
    pub open_settings: bool,
    /// 侧栏显隐变更（`Some(true)` 展开 / `Some(false)` 收起）。
    pub left_panel_visible: Option<bool>,
}

pub fn draw_navigation(
    ctx: &egui::Context,
    view: &mut NavView,
    hosts_list_visible: bool,
    trash_count: usize,
) -> NavAction {
    let t = theme::app(ctx);
    let mut action = NavAction::default();

    egui::SidePanel::left("nav_rail")
        .exact_width(layout::NAV_WIDTH)
        .frame(egui::Frame::new().fill(t.window_bg))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                if nav_icon(ui, Icon::List, *view == NavView::Hosts).clicked() {
                    action.left_panel_visible =
                        panel_nav_click(NavView::Hosts, view, hosts_list_visible);
                }
                ui.add_space(layout::NAV_ICON_GAP);
                if nav_trash_icon(ui, *view == NavView::Trash, trash_count).clicked() {
                    action.left_panel_visible =
                        panel_nav_click(NavView::Trash, view, hosts_list_visible);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    if nav_icon(ui, Icon::Settings, false).clicked() {
                        action.open_settings = true;
                    }
                    ui.add_space(layout::NAV_ICON_GAP);
                    if nav_icon(ui, Icon::History, false).clicked() {
                        action.open_history = true;
                    }
                    ui.add_space(layout::NAV_ICON_GAP);
                    if nav_icon(ui, Icon::Search, false).clicked() {
                        action.open_search = true;
                    }
                    ui.add_space(layout::NAV_ICON_PAD_BOTTOM);
                });
            });
        });

    action
}

/// 对齐 LeftSidebar `handleClick`：隐藏时展开并切换视图；同视图再点收起；否则仅切换视图。
fn panel_nav_click(
    target: NavView,
    view: &mut NavView,
    panel_visible: bool,
) -> Option<bool> {
    if !panel_visible {
        *view = target;
        Some(true)
    } else if *view == target {
        Some(false)
    } else {
        *view = target;
        None
    }
}

fn nav_trash_icon(ui: &mut Ui, active: bool, count: usize) -> egui::Response {
    let response = nav_icon(ui, Icon::Trash, active);
    if count > 0 && ui.is_rect_visible(response.rect) {
        paint_trash_count_badge(ui, response.rect, count);
    }
    response
}

/// Mantine `Indicator` on trash ActionIcon（count 为 0 时不显示）。
fn paint_trash_count_badge(ui: &Ui, icon_rect: egui::Rect, count: usize) {
    let t = theme::app(ui.ctx());
    let label = count.to_string();
    let font_id = FontId::proportional(layout::NAV_BADGE_FONT_SIZE);
    let galley = ui
        .painter()
        .layout_no_wrap(label, font_id, t.nav_badge_text);
    let pad_x = 4.0;
    let badge_w = galley.size().x.max(6.0) + pad_x * 2.0;
    let badge_h = layout::NAV_BADGE_SIZE;
    let anchor = icon_rect.right_top() + Vec2::new(layout::NAV_BADGE_OFFSET, -layout::NAV_BADGE_OFFSET);
    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(anchor.x - badge_w, anchor.y),
        Vec2::new(badge_w, badge_h),
    );
    let radius = badge_h * 0.5;
    ui.painter().rect_filled(badge_rect, radius, t.nav_badge_bg);
    ui.painter().galley(
        badge_rect.center() - galley.size() * 0.5,
        galley,
        t.nav_badge_text,
    );
}

/// Mantine ActionIcon：`light`（选中）/ `subtle`（默认 + hover 灰底）。
fn nav_icon(ui: &mut Ui, icon: Icon, active: bool) -> egui::Response {
    let t = theme::app(ui.ctx());
    let hit = Vec2::splat(layout::NAV_ICON_HIT);
    let (rect, response) = ui.allocate_exact_size(hit, Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if active {
            t.nav_icon_active_bg
        } else if response.hovered() {
            t.nav_icon_hover_bg
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, layout::NAV_ICON_RADIUS, bg);
        }
        let tint = if active {
            t.accent
        } else {
            t.nav_icon_inactive_tint
        };
        icons::paint_icon(ui, icon, rect.center(), layout::NAV_ICON_SIZE, tint);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_nav_click_matches_left_sidebar() {
        let mut view = NavView::Hosts;

        assert_eq!(
            panel_nav_click(NavView::Trash, &mut view, false),
            Some(true)
        );
        assert_eq!(view, NavView::Trash);

        view = NavView::Hosts;
        assert_eq!(panel_nav_click(NavView::Hosts, &mut view, true), Some(false));
        assert_eq!(view, NavView::Hosts);

        view = NavView::Hosts;
        assert_eq!(panel_nav_click(NavView::Trash, &mut view, true), None);
        assert_eq!(view, NavView::Trash);
    }
}
