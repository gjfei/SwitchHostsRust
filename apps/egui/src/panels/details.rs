//! 右侧详情面板（对齐 `SwitchHosts/src/renderer/components/RightPanel/`）。

use switch_hosts_core::manifest_edit::{HostsNodeKind, REFRESH_INTERVALS, SYSTEM_NODE_ID};
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use switch_hosts_core::storage::trashcan::Trashcan;
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::panels::NavView;
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme;

const HEADER_PAD: f32 = 12.0;
const HEADER_H: f32 = 40.0;
const SECTION_PAD: f32 = 12.0;
const ROW_GAP: f32 = 10.0;
const TITLE_FONT: f32 = 14.0;
const LABEL_FONT: f32 = 12.0;
const VALUE_FONT: f32 = 13.0;
const MONO_FONT: f32 = 12.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DetailsAction {
    pub edit: bool,
    pub refresh: bool,
    pub restore: bool,
    pub delete: bool,
    pub open_history: bool,
}

pub fn draw_details(
    ui: &mut Ui,
    manifest: &Manifest,
    trashcan: &Trashcan,
    nav_view: NavView,
    selected_id: Option<&str>,
    editor_text: &str,
    system_hosts_path: &str,
) -> DetailsAction {
    paint_panel_border(ui);
    let mut action = DetailsAction::default();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.max_rect().width());

            if nav_view == NavView::Trash {
                if let Some(id) = selected_id {
                    if let Some(item) = trashcan.items.iter().find(|i| i.id == id) {
                        draw_node_panel(ui, &item.node, manifest, editor_text, true, &mut action);
                        return;
                    }
                }
            }

            let Some(id) = selected_id else {
                draw_system_panel(ui, system_hosts_path, editor_text, &mut action);
                return;
            };

            if id == SYSTEM_NODE_ID {
                draw_system_panel(ui, system_hosts_path, editor_text, &mut action);
                return;
            }

            if let Some(node) = find_node(&manifest.root, id) {
                draw_node_panel(ui, &node, manifest, editor_text, false, &mut action);
            } else {
                ui.add_space(SECTION_PAD);
                let t = theme::app(ui.ctx());
                ui.label(RichText::new("未找到节点").size(VALUE_FONT).color(t.weak_text));
            }
        });

    action
}

fn paint_panel_border(ui: &Ui) {
    let t = theme::app(ui.ctx());
    let rect = ui.max_rect();
    let stroke = Stroke::new(1.0, t.separator);
    let p = ui.painter();
    let r = t.corner_panel();
    p.rect_filled(
        rect,
        CornerRadius {
            nw: 0,
            ne: r.nw,
            se: r.se,
            sw: 0,
        },
        t.sidebar_bg,
    );
    p.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
        stroke,
    );
    p.line_segment(
        [egui::pos2(rect.right(), rect.top()), egui::pos2(rect.right(), rect.bottom())],
        stroke,
    );
    p.line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        stroke,
    );
}

fn draw_system_panel(ui: &mut Ui, path: &str, content: &str, action: &mut DetailsAction) {
    draw_header(ui, "系统 Hosts", Icon::DeviceDesktop, false, action);
    ui.add_space(SECTION_PAD);
    section(ui, |ui| {
        info_row(ui, "Hosts 类型", "系统");
        info_row_mono(ui, "Hosts 文件位于", if path.is_empty() { "—" } else { path });
        info_row(ui, "规则", &count_rules(content).to_string());
    });
    ui.add_space(SECTION_PAD);
    section(ui, |ui| {
        if compact_btn(ui, Icon::History, "查看历史").clicked() {
            action.open_history = true;
        }
    });
}

fn draw_node_panel(
    ui: &mut Ui,
    node: &serde_json::Value,
    manifest: &Manifest,
    editor_text: &str,
    in_trashcan: bool,
    action: &mut DetailsAction,
) {
    let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = node
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名");
    let kind = node.get("type").and_then(|v| v.as_str()).unwrap_or("local");
    let icon = icons::node_icon(node, false);
    let has_content = matches!(kind, "local" | "remote");

    draw_header(ui, title, icon, !in_trashcan, action);

    ui.add_space(SECTION_PAD);
    section(ui, |ui| {
        info_row(ui, "Hosts 类型", type_label(kind));
        if has_content {
            info_row(ui, "规则", &count_rules(editor_text).to_string());
        }
    });

    if kind == "remote" {
        ui.add_space(SECTION_PAD);
        section(ui, |ui| {
            let url = node.get("url").and_then(|v| v.as_str()).unwrap_or("");
            info_row_mono(ui, "URL", if url.is_empty() { "—" } else { url });
            if !in_trashcan {
                let interval = node
                    .get("refresh_interval")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                info_row(ui, "自动刷新", refresh_label(interval));
            }
            let last = node
                .get("last_refresh")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A");
            info_row(ui, "上次刷新", last);
            if !in_trashcan {
                ui.add_space(14.0);
                if compact_btn(ui, Icon::World, "刷新").clicked() {
                    action.refresh = true;
                }
            }
        });
    }

    if kind == "folder" {
        ui.add_space(SECTION_PAD);
        section(ui, |ui| {
            let mode = node
                .get("folder_mode")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u8;
            info_row(ui, "选择模式", folder_mode_label(mode));
        });
    }

    if kind == "group" {
        let include: Vec<&str> = node
            .get("include")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        ui.add_space(SECTION_PAD);
        section(ui, |ui| {
            ui.label(
                RichText::new(format!("内容 ({})", include.len()))
                    .size(LABEL_FONT)
                    .color(theme::app(ui.ctx()).weak_text)
                    .strong(),
            );
            ui.add_space(8.0);
            if include.is_empty() {
                ui.label(
                    RichText::new("—")
                        .size(VALUE_FONT)
                        .color(theme::app(ui.ctx()).weak_text),
                );
            } else {
                for inc_id in include {
                    draw_include_row(ui, manifest, inc_id);
                    ui.add_space(6.0);
                }
            }
        });
    }

    if in_trashcan {
        ui.add_space(SECTION_PAD);
        draw_trash_footer(ui, action);
    }

    let _ = id;
}

fn draw_header(
    ui: &mut Ui,
    title: &str,
    icon: Icon,
    show_edit: bool,
    action: &mut DetailsAction,
) {
    let t = theme::app(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, HEADER_H), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, t.separator));

    let cy = rect.center().y;
    icons::paint_icon(
        ui,
        icon,
        egui::pos2(rect.left() + HEADER_PAD + 8.0, cy),
        16.0,
        Color32::from_rgb(100, 100, 110),
    );

    let title_x = rect.left() + HEADER_PAD + 16.0 + 8.0;
    let title_max_w = if show_edit {
        rect.width() - HEADER_PAD * 2.0 - 16.0 - 8.0 - 56.0
    } else {
        rect.width() - title_x + rect.left() - HEADER_PAD
    };
    let galley = text_align::layout_vcentered_galley(
        ui,
        title.to_owned(),
        ui_font_id(TITLE_FONT),
        t.text,
        ICON_ROW_LINE_HEIGHT,
    );
    let clip = egui::Rect::from_min_size(
        egui::pos2(title_x, rect.top()),
        egui::vec2(title_max_w.max(0.0), rect.height()),
    );
    text_align::paint_galley_row_centered_clipped(
        ui,
        clip,
        title_x,
        cy,
        galley,
        t.text,
    );

    if show_edit {
        let btn = egui::Rect::from_min_size(
            egui::pos2(rect.right() - HEADER_PAD - 48.0, rect.center().y - 14.0),
            egui::vec2(48.0, 28.0),
        );
        let resp = ui.interact(btn, ui.id().with("details_edit"), Sense::click());
        if ui.is_rect_visible(btn) {
            if resp.hovered() {
                ui.painter()
                    .rect_filled(btn, t.corner_icon(), t.icon_hover_bg);
            }
            text_align::paint_icon_text_row(
                ui,
                btn.center().y,
                btn.left() + 6.0,
                Icon::Edit,
                14.0,
                4.0,
                "编辑",
                ui_font_id(12.0),
                t.accent,
                ICON_ROW_LINE_HEIGHT,
            );
        }
        if resp.clicked() {
            action.edit = true;
        }
    }
}

fn draw_include_row(ui: &mut Ui, manifest: &Manifest, id: &str) {
    let t = theme::app(ui.ctx());
    let (icon, label, missing) = if let Some(node) = find_node(&manifest.root, id) {
        (
            icons::node_icon(&node, false),
            node.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("未命名")
                .to_string(),
            false,
        )
    } else {
        (Icon::FileText, id.to_string(), true)
    };

    let left = ui.cursor().min.x;
    let cy = ui.cursor().min.y + VALUE_FONT / 2.0 + 2.0;
    icons::paint_icon(
        ui,
        icon,
        egui::pos2(left + 8.0, cy),
        16.0,
        if missing {
            t.weak_text
        } else {
            t.nav_icon_inactive_tint
        },
    );
    let galley = text_align::layout_vcentered_galley(
        ui,
        label,
        ui_font_id(VALUE_FONT),
        if missing { t.weak_text } else { t.text },
        ICON_ROW_LINE_HEIGHT,
    );
    text_align::paint_galley_row_centered(
        ui,
        left + 16.0 + 6.0,
        cy,
        galley,
        if missing { t.weak_text } else { t.text },
    );
    ui.add_space(VALUE_FONT + 4.0);
}

fn draw_trash_footer(ui: &mut Ui, action: &mut DetailsAction) {
    let t = theme::app(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 52.0), Sense::hover());
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + SECTION_PAD, rect.top()),
            egui::pos2(rect.right() - SECTION_PAD, rect.top()),
        ],
        Stroke::new(1.0, t.separator),
    );

    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(SECTION_PAD, 8.0))),
        |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if compact_btn(ui, Icon::ArrowLeft, "恢复").clicked() {
                    action.restore = true;
                }
                ui.add_space(8.0);
                if compact_btn(ui, Icon::Trash, "删除").clicked() {
                    action.delete = true;
                }
            });
        },
    );
}

fn section(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(SECTION_PAD as i8, 0))
        .show(ui, |ui| body(ui));
}

fn info_row(ui: &mut Ui, label: &str, value: &str) {
    let t = theme::app(ui.ctx());
    ui.label(
        RichText::new(label)
            .size(LABEL_FONT)
            .color(t.weak_text),
    );
    ui.add_space(2.0);
    ui.label(
        RichText::new(value)
            .size(VALUE_FONT)
            .color(t.text),
    );
    ui.add_space(ROW_GAP);
}

fn info_row_mono(ui: &mut Ui, label: &str, value: &str) {
    let t = theme::app(ui.ctx());
    ui.label(
        RichText::new(label)
            .size(LABEL_FONT)
            .color(t.weak_text),
    );
    ui.add_space(2.0);
    ui.label(
        RichText::new(value)
            .font(FontId::monospace(MONO_FONT))
            .size(MONO_FONT)
            .color(t.accent),
    );
    ui.add_space(ROW_GAP);
}

fn compact_btn(ui: &mut Ui, icon: Icon, label: &str) -> egui::Response {
    let t = theme::app(ui.ctx());
    let galley = ui.fonts(|f| {
        f.layout_no_wrap(
            label.to_owned(),
            ui_font_id(12.0),
            t.text,
        )
    });
    let w = 14.0 + 4.0 + galley.size().x + 16.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if response.hovered() {
            t.icon_hover_bg
        } else {
            t.segmented_bg
        };
        ui.painter().rect_filled(rect, t.corner_icon(), fill);
        text_align::paint_icon_text_row(
            ui,
            rect.center().y,
            rect.left() + 8.0,
            icon,
            14.0,
            4.0,
            label,
            ui_font_id(12.0),
            t.text,
            ICON_ROW_LINE_HEIGHT,
        );
    }
    response
}

fn count_rules(content: &str) -> usize {
    content
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .count()
}

fn type_label(kind: &str) -> &'static str {
    HostsNodeKind::from_type_str(kind).label()
}

fn folder_mode_label(mode: u8) -> &'static str {
    match mode {
        1 => "单选",
        2 => "多选",
        _ => "默认",
    }
}

fn refresh_label(secs: u64) -> &'static str {
    REFRESH_INTERVALS
        .iter()
        .find(|(s, _)| *s == secs)
        .map(|(_, l)| *l)
        .unwrap_or("从不")
}

