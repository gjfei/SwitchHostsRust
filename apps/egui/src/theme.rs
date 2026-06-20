//! SwitchHosts 主题：颜色与间距统一写入 egui `Style` + 应用语义 token。

use std::collections::BTreeMap;
use std::sync::Arc;

use eframe::egui::{
    self, Color32, Context, CornerRadius, FontFamily, FontId, Id, Margin, Shadow, Stroke,
    Style, TextStyle, Vec2,
};

/// 存储在 egui 上下文中的应用主题 token。
pub const APP_THEME_ID: Id = Id::NULL;

/// 与明暗无关的布局尺寸（顶栏高度、抽屉宽度等）。
pub mod layout {
    pub const TOP_BAR_HEIGHT: f32 = 40.0;
    pub const TEST_BANNER_HEIGHT: f32 = 24.0;
    pub const STATUS_BAR_HEIGHT: f32 = 22.0;
    pub const TOP_BAR_PAD_X: f32 = 10.0;
    pub const TOP_BAR_MAC_PAD_LEFT: f32 = 88.0;
    pub const TOP_BAR_TRAFFIC_LIGHT_X: f32 = 12.0;
    pub const TOP_BAR_TRAFFIC_LIGHT_Y: f32 = 18.0;
    pub const TOP_BAR_CLUSTER_WIDTH: f32 = 240.0;
    pub const TOP_BAR_ICON_HIT: f32 = 28.0;
    pub const TOP_BAR_ICON_RADIUS: f32 = 4.0;

    pub const NAV_WIDTH: f32 = 40.0;
    pub const NAV_ICON_HIT: f32 = 28.0;
    pub const NAV_ICON_SIZE: f32 = 18.0;
    pub const NAV_ICON_RADIUS: f32 = 4.0;
    pub const NAV_ICON_GAP: f32 = 20.0;
    pub const NAV_ICON_PAD_BOTTOM: f32 = 24.0;
    pub const NAV_BADGE_SIZE: f32 = 14.0;
    pub const NAV_BADGE_OFFSET: f32 = 4.0;
    pub const NAV_BADGE_FONT_SIZE: f32 = 10.0;

    pub const TRASH_HEADER_HEIGHT: f32 = 28.0;
    pub const TRASH_HEADER_PAD_X: f32 = 10.0;
    pub const TRASH_HEADER_FONT_SIZE: f32 = 12.0;
    pub const TRASH_BODY_PAD_X: f32 = 10.0;
    pub const TRASH_BODY_PAD_Y: f32 = 5.0;
    pub const TRASH_CLEAR_ICON: f32 = 16.0;
    pub const TRASH_CLEAR_HIT: f32 = 24.0;

    pub const DRAWER_WIDTH: f32 = 440.0;
    /// Mantine Drawer `size="lg"`（Pref / EditHosts / History）
    pub const DRAWER_WIDTH_LG: f32 = 620.0;
    pub const DRAWER_OFFSET: f32 = 8.0;
    pub const DRAWER_RADIUS: f32 = 8.0;
    pub const DRAWER_PAD: f32 = 16.0;
    pub const DRAWER_HEADER_HEIGHT: f32 = 56.0;
    pub const DRAWER_FOOTER_HEIGHT: f32 = 68.0;
    pub const DRAWER_SECTION_GAP: f32 = 20.0;
    pub const DRAWER_LABEL_GAP: f32 = 8.0;
    pub const DRAWER_INPUT_RADIUS: f32 = 4.0;
    pub const DRAWER_INPUT_HEIGHT: f32 = 36.0;
    pub const DRAWER_INPUT_H_PAD: f32 = 12.0;
    pub const DRAWER_BTN_H: f32 = 36.0;
    pub const DRAWER_BTN_MIN_W: f32 = 88.0;

    pub const RIGHT_PANEL_RADIUS: f32 = 4.0;

    pub const TREE_FONT_SIZE: f32 = 14.0;
    pub const TREE_ROW_HEIGHT: f32 = TREE_FONT_SIZE * 2.0;
    pub const TREE_ROW_GAP: f32 = 4.0;
    pub const TREE_ROW_RADIUS: f32 = 4.0;
    pub const TREE_INDENT: f32 = 20.0;
    pub const TREE_INDENT_PAD: f32 = 4.0;
    pub const SWITCH_WIDTH: f32 = TREE_FONT_SIZE * 1.6;
    pub const SWITCH_HEIGHT: f32 = TREE_FONT_SIZE * 0.9;
    pub const TREE_STATUS_RIGHT: f32 = 6.0;
    pub const TREE_STATUS_GAP: f32 = 5.0;

    pub const EDITOR_FONT_SIZE: f32 = 14.0;
    pub const EDITOR_LINE_HEIGHT: f32 = 25.2;

    /// Mantine `Checkbox` size md（`--checkbox-size` 默认 20px）
    pub const CHECKBOX_SIZE: f32 = 20.0;
    pub const CHECKBOX_LABEL_GAP: f32 = 8.0;
    pub const CHECKBOX_RADIUS: f32 = 4.0;
    /// 带 description 的嵌套项缩进（box + gap）
    pub const CHECKBOX_NESTED_INDENT: f32 = CHECKBOX_SIZE + CHECKBOX_LABEL_GAP;
}

/// 应用语义主题（颜色 + 与 egui `Spacing` 对齐的间距 token）。
#[derive(Clone, Debug)]
pub struct AppTheme {
    pub dark: bool,

    // --- Surfaces ---
    pub window_bg: Color32,
    pub sidebar_bg: Color32,
    pub editor_bg: Color32,
    pub editor_readonly_bg: Color32,
    pub top_bar_bg: Color32,

    // --- Brand ---
    pub accent: Color32,

    // --- Text & borders ---
    pub text: Color32,
    pub text_selected: Color32,
    pub weak_text: Color32,
    pub separator: Color32,
    pub border: Color32,
    pub input_border: Color32,

    // --- Interactive ---
    pub hover_bg: Color32,
    pub icon_hover_bg: Color32,
    pub segmented_bg: Color32,
    pub tree_hover: Color32,
    pub nav_icon_active_bg: Color32,
    pub nav_icon_inactive_tint: Color32,
    pub nav_icon_hover_bg: Color32,
    pub nav_badge_bg: Color32,
    pub nav_badge_text: Color32,
    pub trash_header_text: Color32,

    // --- Editor syntax ---
    pub editor_text: Color32,
    pub editor_comment: Color32,
    pub editor_ip: Color32,
    pub editor_error: Color32,
    pub editor_line_number: Color32,
    pub editor_selection_bg: Color32,

    // --- Switch ---
    pub switch_off_track: Color32,
    pub switch_off_knob: Color32,
    pub switch_on_track: Color32,
    pub switch_on_knob: Color32,

    // --- Find / highlight ---
    pub find_highlight_bg: Color32,
    pub find_error: Color32,

    // --- Spacing (mirrors egui `Style.spacing`) ---
    pub item_spacing: Vec2,
    pub button_padding: Vec2,
    pub interact_size: Vec2,
    pub window_margin: Margin,
}

impl AppTheme {
    pub fn light() -> Self {
        Self {
            dark: false,
            window_bg: Color32::from_rgb(248, 249, 250),
            sidebar_bg: Color32::WHITE,
            editor_bg: Color32::WHITE,
            editor_readonly_bg: Color32::from_rgb(245, 245, 245),
            top_bar_bg: Color32::from_rgb(248, 249, 250),
            accent: Color32::from_rgb(207, 57, 73),
            text: Color32::from_rgb(30, 30, 35),
            text_selected: Color32::WHITE,
            weak_text: Color32::from_rgb(153, 153, 153),
            separator: Color32::from_rgb(233, 233, 236),
            border: Color32::from_rgb(233, 233, 236),
            input_border: Color32::from_rgb(222, 226, 230),
            hover_bg: Color32::from_rgb(255, 235, 238),
            icon_hover_bg: Color32::from_rgb(241, 243, 245),
            segmented_bg: Color32::from_rgb(241, 243, 245),
            tree_hover: Color32::from_rgb(255, 235, 238),
            nav_icon_active_bg: Color32::from_rgb(255, 235, 238),
            nav_icon_inactive_tint: Color32::from_rgb(100, 100, 110),
            nav_icon_hover_bg: Color32::from_rgb(241, 243, 245),
            nav_badge_bg: Color32::from_rgb(134, 142, 150),
            nav_badge_text: Color32::WHITE,
            trash_header_text: Color32::from_rgb(153, 153, 153),
            editor_text: Color32::BLACK,
            editor_comment: Color32::from_rgb(0, 153, 0),
            editor_ip: Color32::from_rgb(9, 109, 217),
            editor_error: Color32::from_rgb(204, 51, 102),
            editor_line_number: Color32::from_rgb(153, 153, 153),
            editor_selection_bg: Color32::from_rgb(215, 212, 240),
            switch_off_track: Color32::from_rgb(204, 204, 204),
            switch_off_knob: Color32::from_rgb(153, 153, 153),
            switch_on_track: Color32::WHITE,
            switch_on_knob: Color32::from_rgb(145, 217, 130),
            find_highlight_bg: Color32::from_rgb(238, 238, 0),
            find_error: Color32::from_rgb(250, 82, 82),
            item_spacing: Vec2::new(layout::DRAWER_LABEL_GAP, layout::DRAWER_LABEL_GAP),
            button_padding: Vec2::new(12.0, 8.0),
            interact_size: Vec2::new(layout::DRAWER_BTN_MIN_W, layout::DRAWER_BTN_H),
            window_margin: Margin::symmetric(layout::DRAWER_PAD as i8, layout::DRAWER_PAD as i8),
        }
    }

    pub fn dark() -> Self {
        Self {
            dark: true,
            window_bg: Color32::from_rgb(26, 27, 30),
            sidebar_bg: Color32::from_rgb(37, 38, 43),
            editor_bg: Color32::from_rgb(37, 38, 43),
            editor_readonly_bg: Color32::from_rgb(32, 33, 36),
            top_bar_bg: Color32::from_rgb(26, 27, 30),
            accent: Color32::from_rgb(207, 57, 73),
            text: Color32::from_rgb(220, 221, 225),
            text_selected: Color32::WHITE,
            weak_text: Color32::from_rgb(134, 142, 150),
            separator: Color32::from_rgb(55, 58, 64),
            border: Color32::from_rgb(55, 58, 64),
            input_border: Color32::from_rgb(68, 71, 78),
            hover_bg: Color32::from_rgb(55, 58, 64),
            icon_hover_bg: Color32::from_rgb(55, 58, 64),
            segmented_bg: Color32::from_rgb(44, 46, 51),
            tree_hover: Color32::from_rgb(55, 58, 64),
            nav_icon_active_bg: Color32::from_rgb(55, 58, 64),
            nav_icon_inactive_tint: Color32::from_rgb(134, 142, 150),
            nav_icon_hover_bg: Color32::from_rgb(55, 58, 64),
            nav_badge_bg: Color32::from_rgb(134, 142, 150),
            nav_badge_text: Color32::WHITE,
            trash_header_text: Color32::from_rgb(134, 142, 150),
            editor_text: Color32::from_rgb(220, 221, 225),
            editor_comment: Color32::from_rgb(106, 153, 85),
            editor_ip: Color32::from_rgb(86, 156, 214),
            editor_error: Color32::from_rgb(244, 135, 113),
            editor_line_number: Color32::from_rgb(134, 142, 150),
            editor_selection_bg: Color32::from_rgb(56, 74, 110),
            switch_off_track: Color32::from_rgb(68, 71, 78),
            switch_off_knob: Color32::from_rgb(134, 142, 150),
            switch_on_track: Color32::from_rgb(44, 46, 51),
            switch_on_knob: Color32::from_rgb(145, 217, 130),
            find_highlight_bg: Color32::from_rgb(120, 120, 0),
            find_error: Color32::from_rgb(250, 82, 82),
            item_spacing: Vec2::new(layout::DRAWER_LABEL_GAP, layout::DRAWER_LABEL_GAP),
            button_padding: Vec2::new(12.0, 8.0),
            interact_size: Vec2::new(layout::DRAWER_BTN_MIN_W, layout::DRAWER_BTN_H),
            window_margin: Margin::symmetric(layout::DRAWER_PAD as i8, layout::DRAWER_PAD as i8),
        }
    }

    pub fn corner_input(&self) -> CornerRadius {
        CornerRadius::same(layout::DRAWER_INPUT_RADIUS as u8)
    }

    pub fn corner_drawer(&self) -> CornerRadius {
        CornerRadius::same(layout::DRAWER_RADIUS as u8)
    }

    pub fn corner_panel(&self) -> CornerRadius {
        CornerRadius::same(layout::RIGHT_PANEL_RADIUS as u8)
    }

    pub fn corner_icon(&self) -> CornerRadius {
        CornerRadius::same(layout::TOP_BAR_ICON_RADIUS as u8)
    }

    pub fn drawer_shadow(&self) -> Shadow {
        Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: if self.dark {
                Color32::from_black_alpha(80)
            } else {
                Color32::from_black_alpha(30)
            },
        }
    }
}

/// 从 egui 上下文读取当前应用主题；未初始化时回退浅色。
pub fn app(ctx: &Context) -> AppTheme {
    ctx.data(|d| d.get_temp(APP_THEME_ID))
        .unwrap_or_else(AppTheme::light)
}

pub fn apply_theme(ctx: &Context, theme: &str, system_dark: bool) {
    let dark = match theme {
        "dark" => true,
        "light" => false,
        _ => system_dark,
    };
    let tokens = if dark {
        AppTheme::dark()
    } else {
        AppTheme::light()
    };
    apply_style(ctx, &tokens);
    ctx.data_mut(|d| d.insert_temp(APP_THEME_ID, tokens));
}

fn apply_style(ctx: &Context, t: &AppTheme) {
    let mut spacing = egui::Spacing {
        item_spacing: t.item_spacing,
        window_margin: t.window_margin,
        button_padding: t.button_padding,
        menu_margin: Margin::same(8),
        indent: layout::TREE_INDENT,
        interact_size: t.interact_size,
        text_edit_width: layout::DRAWER_WIDTH - layout::DRAWER_PAD * 2.0,
        icon_width: layout::CHECKBOX_SIZE,
        icon_width_inner: 12.0,
        icon_spacing: layout::CHECKBOX_LABEL_GAP,
        ..Default::default()
    };
    spacing.scroll.dormant_background_opacity = 0.0;
    spacing.scroll.dormant_handle_opacity = 0.0;

    let mut style = Style {
        spacing,
        text_styles: default_text_styles(),
        visuals: build_visuals(t),
        ..Default::default()
    };
    style.visuals.dark_mode = t.dark;
    ctx.set_global_style(Arc::new(style));
}

fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    BTreeMap::from([
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(layout::TREE_FONT_SIZE, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(layout::TREE_FONT_SIZE, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(layout::EDITOR_FONT_SIZE, FontFamily::Monospace)),
    ])
}

fn build_visuals(t: &AppTheme) -> egui::Visuals {
    let mut visuals = if t.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = t.window_bg;
    visuals.window_fill = t.window_bg;
    visuals.extreme_bg_color = t.window_bg;
    visuals.faint_bg_color = t.window_bg;
    visuals.code_bg_color = t.editor_readonly_bg;

    let corner_sm = CornerRadius::same(layout::CHECKBOX_RADIUS as u8);

    visuals.widgets.noninteractive.bg_fill = t.sidebar_bg;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.separator);
    visuals.widgets.noninteractive.weak_bg_fill = t.icon_hover_bg;
    visuals.widgets.noninteractive.corner_radius = corner_sm;

    visuals.widgets.inactive.bg_fill = t.editor_bg;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, t.input_border);
    visuals.widgets.inactive.weak_bg_fill = t.segmented_bg;
    visuals.widgets.inactive.corner_radius = corner_sm;

    visuals.widgets.hovered.bg_fill = t.hover_bg;
    // SidePanel / 窗口拖拽分隔线在 hover 时使用 fg_stroke（对齐 SwitchHosts 红色 resize 提示）
    visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, t.accent);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.input_border);
    visuals.widgets.hovered.weak_bg_fill = t.icon_hover_bg;
    visuals.widgets.hovered.corner_radius = corner_sm;

    visuals.widgets.active.bg_fill = t.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, t.text_selected);
    visuals.widgets.active.weak_bg_fill = t.accent;

    visuals.widgets.open.bg_fill = t.segmented_bg;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, t.text);
    visuals.widgets.open.weak_bg_fill = t.segmented_bg;

    visuals.selection.bg_fill = t.editor_selection_bg;
    visuals.selection.stroke = if t.dark {
        Stroke::new(1.0, Color32::from_rgb(120, 170, 220))
    } else {
        Stroke::new(1.0, Color32::from_rgb(0, 83, 125))
    };

    visuals.hyperlink_color = t.accent;
    visuals.warn_fg_color = t.find_error;
    visuals.error_fg_color = t.find_error;

    visuals.indent_has_left_vline = false;

    visuals.window_corner_radius = t.corner_input();
    visuals.menu_corner_radius = t.corner_input();
    visuals.window_stroke = Stroke::new(1.0, t.separator);
    visuals.window_shadow = t.drawer_shadow();
    visuals.popup_shadow = t.drawer_shadow();

    visuals
}
