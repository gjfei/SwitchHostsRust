//! SwitchHosts 风格浅色主题（对齐 `styles/themes/light.scss`）。

use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Mantine `--mantine-color-gray-0`（`--swh-window-bg`）
pub const WINDOW_BG: Color32 = Color32::from_rgb(248, 249, 250);

/// Mantine primary filled ≈ `#cf3949`
pub const ACCENT: Color32 = Color32::from_rgb(207, 57, 73);

/// `--swh-tree-hover-bg` primary light tint
pub const TREE_HOVER: Color32 = Color32::from_rgb(255, 235, 238);

/// 方案列表侧栏背景（white）
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(255, 255, 255);

/// 编辑器区域背景
pub const EDITOR_BG: Color32 = Color32::from_rgb(255, 255, 255);

/// 只读编辑器背景
pub const EDITOR_READONLY_BG: Color32 = Color32::from_rgb(245, 245, 245);

/// 编辑器语法色（对齐 `light.scss` `--swh-editor-*`）
pub const EDITOR_TEXT: Color32 = Color32::from_rgb(0, 0, 0);
pub const EDITOR_COMMENT: Color32 = Color32::from_rgb(0, 153, 0);
pub const EDITOR_IP: Color32 = Color32::from_rgb(9, 109, 217);
pub const EDITOR_ERROR: Color32 = Color32::from_rgb(204, 51, 102);
pub const EDITOR_LINE_NUMBER: Color32 = Color32::from_rgb(153, 153, 153);

/// CodeMirror light 主题聚焦选区 `.cm-selectionBackground`（`@codemirror/view` base theme）
pub const TEXT_SELECTION_BG: Color32 = Color32::from_rgb(215, 212, 240);

pub const EDITOR_FONT_SIZE: f32 = 14.0;
pub const EDITOR_LINE_HEIGHT: f32 = 25.2; // 14px * 1.8

pub const STATUS_BAR_HEIGHT: f32 = 22.0;

pub const SEPARATOR: Color32 = Color32::from_rgb(233, 233, 236);

/// RightPanel `border-radius: 0 md md 0`
pub const RIGHT_PANEL_RADIUS: f32 = 4.0;

pub const TOP_BAR_HEIGHT: f32 = 40.0;
/// debug 测试模式横幅高度（位于 TopBar 下方，不占标题栏区域）
pub const TEST_BANNER_HEIGHT: f32 = 24.0;
/// 顶栏背景（对齐 TopBar `background: transparent`，透出 `--swh-window-bg`）
pub const TOP_BAR_BG: Color32 = WINDOW_BG;
/// 对齐 `TopBar/index.module.scss` `$p`
pub const TOP_BAR_PAD_X: f32 = 10.0;
/// 对齐 `.platform-darwin .root { padding-left: 88px }`（交通灯区域）
pub const TOP_BAR_MAC_PAD_LEFT: f32 = 88.0;
/// 对齐 Tauri `traffic_light_position(12, 18)`
pub const TOP_BAR_TRAFFIC_LIGHT_X: f32 = 12.0;
pub const TOP_BAR_TRAFFIC_LIGHT_Y: f32 = 18.0;
/// 左右操作区宽度，用于标题居中计算（对齐 `$w: 240px`）
pub const TOP_BAR_CLUSTER_WIDTH: f32 = 240.0;
/// Mantine ActionIcon `variant="subtle"` hover（`gray-1`）
pub const TOP_BAR_ICON_HOVER: Color32 = Color32::from_rgb(241, 243, 245);
pub const TOP_BAR_ICON_HIT: f32 = 28.0;
pub const TOP_BAR_ICON_RADIUS: f32 = 4.0;

pub const NAV_WIDTH: f32 = 40.0;
/// LeftSidebar `ActionIcon size={28}` / icon 18px
pub const NAV_ICON_HIT: f32 = 28.0;
pub const NAV_ICON_SIZE: f32 = 18.0;
pub const NAV_ICON_RADIUS: f32 = 4.0;
/// LeftSidebar `Stack gap={20}`
pub const NAV_ICON_GAP: f32 = 20.0;
pub const NAV_ICON_PAD_BOTTOM: f32 = 24.0;
/// ActionIcon `variant="light"`（`--mantine-primary-color-light`）
pub const NAV_ICON_ACTIVE_BG: Color32 = TREE_HOVER;
/// ActionIcon `variant="subtle" color="gray"`
pub const NAV_ICON_INACTIVE_TINT: Color32 = Color32::from_rgb(100, 100, 110);
pub const NAV_ICON_HOVER_BG: Color32 = TOP_BAR_ICON_HOVER;
/// Indicator `color="gray" size={14} offset={4}`
pub const NAV_BADGE_SIZE: f32 = 14.0;
pub const NAV_BADGE_OFFSET: f32 = 4.0;
/// Mantine `gray` indicator
pub const NAV_BADGE_BG: Color32 = Color32::from_rgb(134, 142, 150);
pub const NAV_BADGE_TEXT: Color32 = Color32::WHITE;
pub const NAV_BADGE_FONT_SIZE: f32 = 10.0;

/// Trashcan header（`Trashcan.module.scss` `.header_title`）
pub const TRASH_HEADER_HEIGHT: f32 = 28.0;
pub const TRASH_HEADER_PAD_X: f32 = 10.0;
pub const TRASH_HEADER_FONT_SIZE: f32 = 12.0;
pub const TRASH_HEADER_TEXT: Color32 = Color32::from_rgb(153, 153, 153);
pub const TRASH_BODY_PAD_X: f32 = 10.0;
pub const TRASH_BODY_PAD_Y: f32 = 5.0;
pub const TRASH_CLEAR_ICON: f32 = 16.0;
pub const TRASH_CLEAR_HIT: f32 = 24.0;

/// SideDrawer `size="lg"` + Mantine spacing
pub const DRAWER_WIDTH: f32 = 440.0;
/// Mantine Drawer `offset: 8`
pub const DRAWER_OFFSET: f32 = 8.0;
/// Mantine Drawer `radius: 'md'`
pub const DRAWER_RADIUS: f32 = 8.0;
pub const DRAWER_PAD: f32 = 16.0;
pub const DRAWER_HEADER_HEIGHT: f32 = 56.0;
pub const DRAWER_FOOTER_HEIGHT: f32 = 68.0;
pub const DRAWER_SECTION_GAP: f32 = 20.0;
pub const DRAWER_LABEL_GAP: f32 = 8.0;
pub const DRAWER_INPUT_RADIUS: f32 = 4.0;
pub const DRAWER_WEAK_TEXT: Color32 = Color32::from_rgb(153, 153, 153);
pub const DRAWER_BORDER: Color32 = SEPARATOR;
pub const DRAWER_INPUT_BORDER: Color32 = Color32::from_rgb(222, 226, 230);
pub const DRAWER_SEGMENTED_BG: Color32 = TOP_BAR_ICON_HOVER;

/// 列表字体 body `0.875rem` @ 16px root
pub const TREE_FONT_SIZE: f32 = 14.0;
/// `--swh-tree-row-height: 2em`
pub const TREE_ROW_HEIGHT: f32 = TREE_FONT_SIZE * 2.0;
/// Tree `.content { margin: 2px 0 }` → 行间 4px
pub const TREE_ROW_GAP: f32 = 4.0;
pub const TREE_ROW_RADIUS: f32 = 4.0;
pub const TREE_INDENT: f32 = 20.0;
pub const TREE_INDENT_PAD: f32 = 4.0;
/// SwitchButton `1.6em × 0.9em`
pub const SWITCH_WIDTH: f32 = TREE_FONT_SIZE * 1.6;
pub const SWITCH_HEIGHT: f32 = TREE_FONT_SIZE * 0.9;
pub const TREE_STATUS_RIGHT: f32 = 6.0;
pub const TREE_STATUS_GAP: f32 = 5.0;

/// 列表默认文字/图标色
pub const TREE_TEXT: Color32 = Color32::from_rgb(30, 30, 35);
/// 选中行文字/图标色（`--swh-font-color-reverse`）
pub const TREE_TEXT_SELECTED: Color32 = Color32::WHITE;

pub fn setup_light_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = WINDOW_BG;
    visuals.window_fill = WINDOW_BG;
    visuals.extreme_bg_color = WINDOW_BG;
    visuals.faint_bg_color = WINDOW_BG;
    visuals.widgets.noninteractive.bg_fill = SIDEBAR_BG;
    visuals.widgets.inactive.bg_fill = Color32::WHITE;
    visuals.widgets.hovered.bg_fill = TREE_HOVER;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = TEXT_SELECTION_BG;
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(0, 83, 125));
    visuals.window_corner_radius = CornerRadius::same(4);
    ctx.set_visuals(visuals);
}
