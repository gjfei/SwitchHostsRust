//! 右侧滑出面板：添加/编辑 hosts（对齐 `EditHostsInfo.tsx` + `SideDrawer`）。

use switch_hosts_core::manifest_edit::{
    add_draft, ensure_folder_expanded, list_includable_nodes, remove_node_with_parent,
    update_node_in_root, HostsNodeDraft, HostsNodeKind, REFRESH_INTERVALS,
};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::remote_refresh::refresh_remote_node;
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme::{
    ACCENT, DRAWER_BORDER, DRAWER_FOOTER_HEIGHT, DRAWER_HEADER_HEIGHT, DRAWER_INPUT_BORDER,
    DRAWER_INPUT_RADIUS, DRAWER_LABEL_GAP, DRAWER_OFFSET, DRAWER_PAD, DRAWER_RADIUS,
    DRAWER_SECTION_GAP, DRAWER_SEGMENTED_BG, DRAWER_WEAK_TEXT, DRAWER_WIDTH,
    TOP_BAR_ICON_HOVER, TOP_BAR_ICON_RADIUS,
};

const TITLE_MAX_LEN: usize = 50;
const TRANSFER_LIST_H: f32 = 200.0;
/// 对齐原版 `gridTemplateColumns: minmax(0, 1fr) 40px minmax(0, 1fr); gap: 4`
const TRANSFER_ARROWS_W: f32 = 40.0;
const TRANSFER_COL_GAP: f32 = 4.0;
const REFRESH_SELECT_W: f32 = 160.0;
const DRAWER_INPUT_HEIGHT: f32 = 36.0;
const DRAWER_INPUT_TEXT: Color32 = Color32::from_rgb(30, 30, 35);
const DRAWER_INPUT_H_PAD: f32 = 12.0;
/// Mantine `SegmentedControl` size md（含 root padding）
const SEGMENTED_HEIGHT: f32 = 36.0;
const SEGMENTED_INNER: f32 = 4.0;
const SEGMENTED_ICON: f32 = 16.0;
const SEGMENTED_GAP: f32 = 4.0;
const DRAWER_BTN_H: f32 = 36.0;
const DRAWER_BTN_MIN_W: f32 = 88.0;
const DRAWER_CORNER_RADIUS: CornerRadius = CornerRadius::same(DRAWER_RADIUS as u8);
const DRAWER_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 4],
    blur: 16,
    spread: 0,
    color: Color32::from_black_alpha(30),
};

/// 抽屉模式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditHostsMode {
    Add,
    Edit { id: String },
}

/// 抽屉状态。
#[derive(Debug, Clone, Default)]
pub struct EditHostsState {
    pub open: bool,
    pub mode: Option<EditHostsMode>,
    pub draft: HostsNodeDraft,
    pub title_error: bool,
    /// 添加模式下的目标父文件夹 id（`None` 表示 root）。
    pub parent_id: Option<String>,
    transfer_left_selected: Vec<String>,
    transfer_right_selected: Vec<String>,
    /// 打开后首帧聚焦标题输入（对齐 `data-autofocus`）。
    focus_title: bool,
    /// 上一帧抽屉是否已打开（用于忽略「打开抽屉」同帧的遮罩点击）。
    open_last_frame: bool,
}

impl EditHostsState {
    pub fn open_add(&mut self, parent_id: Option<String>) {
        self.open = true;
        self.mode = Some(EditHostsMode::Add);
        self.draft = HostsNodeDraft::for_add();
        self.title_error = false;
        self.parent_id = parent_id;
        self.transfer_left_selected.clear();
        self.transfer_right_selected.clear();
        self.focus_title = true;
    }

    pub fn open_edit(&mut self, node: &serde_json::Value) {
        self.open = true;
        self.mode = Some(EditHostsMode::Edit {
            id: node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
        self.draft = HostsNodeDraft::from_node(node);
        self.title_error = false;
        self.transfer_left_selected.clear();
        self.transfer_right_selected.clear();
        self.focus_title = true;
    }

    pub fn is_add(&self) -> bool {
        matches!(self.mode, Some(EditHostsMode::Add))
    }
}

/// 抽屉提交结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditHostsResult {
    None,
    Cancelled,
    Saved { id: String },
    MovedToTrash {
        node: serde_json::Value,
        parent_id: Option<String>,
    },
}

pub fn draw_edit_hosts_drawer(
    ctx: &egui::Context,
    state: &mut EditHostsState,
    manifest: &mut Manifest,
    paths: &AppPaths,
    config: &AppConfig,
) -> EditHostsResult {
    if !state.open {
        state.open_last_frame = false;
        return EditHostsResult::None;
    }

    let mut result = EditHostsResult::None;
    let is_add = state.is_add();
    let allow_backdrop_dismiss = state.open_last_frame;

    let screen = ctx.input(|i| i.screen_rect());
    let backdrop_rect = screen;

    let drawer_rect = {
        let inset = screen.shrink2(egui::Vec2::splat(DRAWER_OFFSET));
        egui::Rect::from_min_max(
            egui::pos2(inset.right() - DRAWER_WIDTH, inset.top()),
            egui::pos2(inset.right(), inset.bottom()),
        )
    };

    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("edit_hosts_backdrop"),
    ))
    .rect_filled(backdrop_rect, 0.0, Color32::from_black_alpha(100));

    let backdrop_clicked = ctx.input(|i| {
        i.pointer.primary_clicked()
            && i.pointer
                .interact_pos()
                .is_some_and(|pos| backdrop_rect.contains(pos) && !drawer_rect.contains(pos))
    });
    if allow_backdrop_dismiss && backdrop_clicked {
        state.open = false;
        result = EditHostsResult::Cancelled;
    }

    let shadow_margin = {
        let sm = DRAWER_SHADOW.margin();
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

    egui::Area::new(egui::Id::new("edit_hosts_drawer"))
        .order(egui::Order::Foreground)
        .fixed_pos(area_rect.min)
        .show(ctx, |ui| {
            ui.set_min_size(area_rect.size());
            ui.set_max_size(area_rect.size());

            egui::Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(DRAWER_CORNER_RADIUS)
                .stroke(Stroke::new(1.0, DRAWER_BORDER))
                .outer_margin(shadow_margin)
                .shadow(DRAWER_SHADOW)
                .show(ui, |ui| {
                    ui.set_width(drawer_rect.width());
                    ui.set_height(drawer_rect.height());

                    let title = if is_add { "添加 hosts" } else { "编辑 hosts" };
                    ui.vertical(|ui| {
                        if draw_drawer_header(ui, title) {
                            state.open = false;
                            result = EditHostsResult::Cancelled;
                        }

                        let body_h = ui.available_height() - DRAWER_FOOTER_HEIGHT;
                        let body_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(drawer_rect.width(), body_h.max(0.0)),
                        );
                        ui.painter()
                            .rect_filled(body_rect, 0.0, Color32::WHITE);
                        ScrollArea::vertical()
                            .id_salt("edit_hosts_drawer_body")
                            .auto_shrink([false; 2])
                            .max_height(body_h.max(0.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(DRAWER_PAD);
                                    ui.vertical(|ui| {
                                        ui.set_width(DRAWER_WIDTH - DRAWER_PAD * 2.0);
                                        ui.add_space(DRAWER_PAD);
                                        form_section(ui, "Hosts 类型", |ui| {
                                            draw_kind_segmented(ui, &mut state.draft.kind, is_add);
                                        });
                                        form_section(ui, "Hosts 标题", |ui| {
                                            draw_title_field(
                                                ui,
                                                &mut state.draft.title,
                                                &mut state.title_error,
                                                &mut state.focus_title,
                                            );
                                        });
                                        match state.draft.kind {
                                            HostsNodeKind::Remote => {
                                                draw_remote_fields(
                                                    ui,
                                                    state,
                                                    manifest,
                                                    paths,
                                                    config,
                                                    is_add,
                                                );
                                            }
                                            HostsNodeKind::Group => {
                                                draw_group_transfer(ui, state, manifest);
                                            }
                                            HostsNodeKind::Folder => {
                                                draw_folder_fields(ui, &mut state.draft.folder_mode);
                                            }
                                            HostsNodeKind::Local => {}
                                        }
                                        ui.add_space(24.0);
                                    });
                                });
                            });

                        draw_drawer_footer(ui, state, manifest, paths, is_add, &mut result);
                    });
                });
        });

    state.open_last_frame = state.open;
    result
}

fn draw_drawer_header(ui: &mut Ui, title: &str) -> bool {
    let mut close = false;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, DRAWER_HEADER_HEIGHT), Sense::hover());
    let cy = rect.center().y;
    text_align::paint_icon_text_row(
        ui,
        cy,
        rect.left() + DRAWER_PAD,
        Icon::Edit,
        18.0,
        10.0,
        title,
        ui_font_id(16.0),
        Color32::BLACK,
        18.0,
    );
    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - DRAWER_PAD - 14.0, cy),
        Vec2::splat(28.0),
    );
    let close_resp = ui.interact(close_rect, ui.id().with("drawer_close"), Sense::click());
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_rect, TOP_BAR_ICON_RADIUS, TOP_BAR_ICON_HOVER);
    }
    icons::paint_icon(
        ui,
        Icon::X,
        close_rect.center(),
        18.0,
        Color32::from_rgb(100, 100, 110),
    );
    if close_resp.clicked() {
        close = true;
    }
    close
}

fn draw_drawer_footer(
    ui: &mut Ui,
    state: &mut EditHostsState,
    manifest: &mut Manifest,
    paths: &AppPaths,
    is_add: bool,
    result: &mut EditHostsResult,
) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, DRAWER_FOOTER_HEIGHT), Sense::hover());
    let half = (rect.width() - DRAWER_PAD * 2.0) * 0.5;
    let left = egui::Rect::from_min_size(
        egui::pos2(rect.left() + DRAWER_PAD, rect.top() + 16.0),
        Vec2::new(half, 36.0),
    );
    let right = egui::Rect::from_min_size(
        egui::pos2(rect.left() + DRAWER_PAD + half, rect.top() + 16.0),
        Vec2::new(half, 36.0),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left), |ui| {
        if !is_add {
            if outline_button_with_icon(ui, Icon::Trash, "移到回收站", false).clicked() {
                if let Some(EditHostsMode::Edit { id }) = state.mode.clone() {
                    if let Some((node, parent_id)) =
                        remove_node_with_parent(&mut manifest.root, &id)
                    {
                        let _ = manifest.save(paths);
                        state.open = false;
                        *result = EditHostsResult::MovedToTrash { node, parent_id };
                    }
                }
            }
        }
    });

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, "确定").clicked() {
                *result = try_save(state, manifest, paths);
            }
            ui.add_space(12.0);
            if outline_button(ui, "取消").clicked() {
                state.open = false;
                *result = EditHostsResult::Cancelled;
            }
        });
    });
}

fn form_section(ui: &mut Ui, label: &str, body: impl FnOnce(&mut Ui)) {
    form_label(ui, label);
    body(ui);
    ui.add_space(DRAWER_SECTION_GAP);
}

fn form_label(ui: &mut Ui, label: &str) {
    let size = 14.0;
    let line_h = ICON_ROW_LINE_HEIGHT;
    let row_h = line_h + 4.0;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, row_h), Sense::hover());
    let galley = text_align::layout_vcentered_galley(
        ui,
        label.to_owned(),
        ui_font_id(size),
        DRAWER_INPUT_TEXT,
        line_h,
    );
    text_align::paint_galley_row_centered(ui, rect.left(), rect.center().y, galley, DRAWER_INPUT_TEXT);
    ui.add_space(DRAWER_LABEL_GAP);
}

fn with_flat_combo_style<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let style = ui.style_mut();
    let saved_inactive = style.visuals.widgets.inactive;
    let saved_hovered = style.visuals.widgets.hovered;
    let saved_open = style.visuals.widgets.open;
    let saved_btn_pad = style.spacing.button_padding;

    let text_stroke = Stroke::new(1.0, DRAWER_INPUT_TEXT);
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

/// Mantine `Select`：`maw={160}` + 与 TextInput 同高（36px）。
fn drawer_select(
    ui: &mut Ui,
    id_salt: &'static str,
    width: f32,
    selected: &str,
    menu: impl FnOnce(&mut Ui),
) {
    let combo_id = egui::Id::new(id_salt);
    let is_open = egui::ComboBox::is_open(ui.ctx(), combo_id);
    let inner_w = (width - DRAWER_INPUT_H_PAD * 2.0).max(0.0);

    let (row_rect, _) = ui.allocate_exact_size(
        Vec2::new(width, DRAWER_INPUT_HEIGHT),
        Sense::hover(),
    );

    ui.painter().rect(
        row_rect,
        DRAWER_INPUT_RADIUS,
        Color32::WHITE,
        Stroke::new(
            1.0,
            if is_open {
                ACCENT
            } else {
                DRAWER_INPUT_BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );

    let inner_rect = row_rect.shrink2(egui::vec2(DRAWER_INPUT_H_PAD, 0.0));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_height(inner_rect.height());
            with_flat_combo_style(ui, |ui| {
                egui::ComboBox::from_id_salt(id_salt)
                    .width(inner_w)
                    .selected_text(
                        RichText::new(selected)
                            .size(14.0)
                            .color(DRAWER_INPUT_TEXT)
                            .font(ui_font_id(14.0)),
                    )
                    .show_ui(ui, menu);
            });
        });
    });
}

/// Mantine Select 下拉项：选中项浅红底 + 主色文字。
fn drawer_select_option(ui: &mut Ui, current: &mut u64, value: u64, label: &str) {
    let selected = *current == value;
    let row_h = 28.0;
    let text_color = if selected { ACCENT } else { DRAWER_INPUT_TEXT };
    let (rect, mut response) = ui.allocate_at_least(Vec2::new(ui.available_width(), row_h), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if selected {
            crate::theme::TREE_HOVER
        } else if response.hovered() {
            TOP_BAR_ICON_HOVER
        } else {
            Color32::WHITE
        };
        ui.painter()
            .rect_filled(rect, DRAWER_INPUT_RADIUS, fill);
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

/// Mantine `Button variant="subtle" size="sm"`。
fn drawer_subtle_button(ui: &mut Ui, label: &str, enabled: bool) -> egui::Response {
    let text_color = if enabled {
        Color32::from_rgb(60, 60, 70)
    } else {
        DRAWER_WEAK_TEXT
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).size(14.0).color(text_color))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .corner_radius(DRAWER_INPUT_RADIUS)
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

fn draw_title_field(
    ui: &mut Ui,
    title: &mut String,
    title_error: &mut bool,
    focus_title: &mut bool,
) {
    if title.len() > TITLE_MAX_LEN {
        title.truncate(TITLE_MAX_LEN);
    }

    let id = ui.id().with("hosts_title");
    let is_error = *title_error && title.trim().is_empty();
    let will_focus = *focus_title || ui.memory(|m| m.has_focus(id));
    let edit = egui::TextEdit::singleline(title)
        .id(id)
        .desired_width(f32::INFINITY)
        .margin(egui::Margin::symmetric(12, 10))
        .frame(false);

    let framed = egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(
            if is_error { 1.5 } else { 1.0 },
            if is_error || will_focus {
                ACCENT
            } else {
                DRAWER_INPUT_BORDER
            },
        ))
        .corner_radius(DRAWER_INPUT_RADIUS)
        .show(ui, |ui| ui.add(edit));

    if *focus_title {
        framed.inner.request_focus();
        *focus_title = false;
    }

    if framed.inner.changed() {
        *title_error = false;
    }
}

fn draw_kind_segmented(ui: &mut Ui, kind: &mut HostsNodeKind, enabled: bool) {
    let options = [
        HostsNodeKind::Local,
        HostsNodeKind::Remote,
        HostsNodeKind::Group,
        HostsNodeKind::Folder,
    ];
    segmented_control(ui, "hosts_kind", kind, &options, enabled, |ui, k, tint, seg_rect| {
        paint_segment_icon_text(
            ui,
            seg_rect,
            icons::kind_icon(*k),
            k.label(),
            tint,
        );
    });
}

fn paint_segment_icon_text(ui: &Ui, seg_rect: egui::Rect, icon: Icon, text: &str, tint: Color32) {
    let galley = text_align::layout_vcentered_galley(
        ui,
        text.to_owned(),
        ui_font_id(14.0),
        tint,
        SEGMENTED_ICON,
    );
    let content_w = SEGMENTED_ICON + SEGMENTED_GAP + galley.size().x;
    let center_y = seg_rect.center().y;
    let mut x = seg_rect.center().x - content_w / 2.0;

    icons::paint_icon(
        ui,
        icon,
        egui::pos2(x + SEGMENTED_ICON / 2.0, center_y),
        SEGMENTED_ICON,
        tint,
    );
    x += SEGMENTED_ICON + SEGMENTED_GAP;
    text_align::paint_galley_row_centered(ui, x, center_y, galley, tint);
}

fn paint_segment_text(ui: &Ui, seg_rect: egui::Rect, text: &str, tint: Color32) {
    let galley = text_align::layout_vcentered_galley(
        ui,
        text.to_owned(),
        ui_font_id(14.0),
        tint,
        SEGMENTED_ICON,
    );
    let center = seg_rect.center();
    text_align::paint_galley_row_centered(
        ui,
        center.x - galley.size().x / 2.0,
        center.y,
        galley,
        tint,
    );
}

fn draw_remote_fields(
    ui: &mut Ui,
    state: &mut EditHostsState,
    manifest: &mut Manifest,
    paths: &AppPaths,
    config: &AppConfig,
    is_add: bool,
) {
    form_section(ui, "URL", |ui| {
        let edit = egui::TextEdit::singleline(&mut state.draft.url)
            .desired_width(f32::INFINITY)
            .hint_text("http:// 或 https:// 或 file://")
            .margin(egui::Margin::symmetric(12, 10));
        egui::Frame::new()
            .stroke(Stroke::new(1.0, DRAWER_INPUT_BORDER))
            .corner_radius(DRAWER_INPUT_RADIUS)
            .show(ui, |ui| {
                ui.add(edit);
            });
    });

    form_section(ui, "自动刷新", |ui| {
        drawer_select(
            ui,
            "refresh_interval",
            REFRESH_SELECT_W,
            refresh_label(state.draft.refresh_interval),
            |ui| {
                for (secs, label) in REFRESH_INTERVALS {
                    drawer_select_option(ui, &mut state.draft.refresh_interval, *secs, label);
                }
            },
        );

        if !is_add {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let last = state
                    .draft
                    .last_refresh
                    .as_deref()
                    .unwrap_or("N/A");
                ui.label(
                    RichText::new(format!("最后刷新：{last}"))
                        .size(14.0)
                        .color(DRAWER_WEAK_TEXT),
                );
                let refresh_enabled = state.draft.id.is_some();
                if drawer_subtle_button(ui, "刷新", refresh_enabled).clicked() {
                    if let Some(id) = state.draft.id.clone() {
                        if refresh_remote_node(paths, manifest, config, &id).is_ok() {
                            if let Some(node) = find_node(&manifest.root, &id) {
                                state.draft.last_refresh = node
                                    .get("last_refresh")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_string);
                                state.draft.last_refresh_ms =
                                    node.get("last_refresh_ms").and_then(|v| v.as_u64());
                            }
                        }
                    }
                }
            });
        }
    });
}

fn draw_group_transfer(ui: &mut Ui, state: &mut EditHostsState, manifest: &Manifest) {
    form_section(ui, "内容", |ui| {
        let candidates = list_includable_nodes(&manifest.root);
        if candidates.is_empty() {
            ui.label(
                RichText::new("暂无 local/remote 方案可选")
                    .color(DRAWER_WEAK_TEXT),
            );
            return;
        }

        let left_ids: Vec<String> = candidates
            .iter()
            .map(|(id, _, _)| id.clone())
            .filter(|id| !state.draft.include.contains(id))
            .collect();
        let right_ids = state.draft.include.clone();

        let row_w = ui.available_width();
        let col_w = (row_w - TRANSFER_ARROWS_W - TRANSFER_COL_GAP * 2.0) * 0.5;
        let col_h = TRANSFER_LIST_H + 28.0;
        let (row_rect, _) = ui.allocate_exact_size(Vec2::new(row_w, col_h), Sense::hover());

        let left_rect = egui::Rect::from_min_size(row_rect.min, Vec2::new(col_w, col_h));
        let mid_rect = egui::Rect::from_min_size(
            egui::pos2(row_rect.left() + col_w + TRANSFER_COL_GAP, row_rect.top()),
            Vec2::new(TRANSFER_ARROWS_W, col_h),
        );
        let right_rect = egui::Rect::from_min_size(
            egui::pos2(mid_rect.right() + TRANSFER_COL_GAP, row_rect.top()),
            Vec2::new(col_w, col_h),
        );

        let mut move_to_selected = false;
        let mut move_to_all = false;

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            transfer_column(
                ui,
                ui.id().with("xfer_left"),
                col_w,
                "全部",
                &left_ids,
                &candidates,
                &mut state.transfer_left_selected,
            );
        });
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(mid_rect), |ui| {
            transfer_arrows(
                ui,
                !state.transfer_left_selected.is_empty(),
                !state.transfer_right_selected.is_empty(),
                &mut move_to_selected,
                &mut move_to_all,
            );
        });
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            transfer_column(
                ui,
                ui.id().with("xfer_right"),
                col_w,
                "已选",
                &right_ids,
                &candidates,
                &mut state.transfer_right_selected,
            );
        });
        if move_to_selected {
            for id in state.transfer_left_selected.clone() {
                if !state.draft.include.contains(&id) {
                    state.draft.include.push(id);
                }
            }
            state.transfer_left_selected.clear();
        }
        if move_to_all {
            let remove = state.transfer_right_selected.clone();
            state
                .draft
                .include
                .retain(|id| !remove.contains(id));
            state.transfer_right_selected.clear();
        }
    });
}

fn transfer_column(
    ui: &mut Ui,
    id: egui::Id,
    width: f32,
    title: &str,
    ids: &[String],
    candidates: &[(String, String, HostsNodeKind)],
    selected: &mut Vec<String>,
) {
    let col_h = TRANSFER_LIST_H + 28.0;
    ui.allocate_ui_with_layout(
        Vec2::new(width, col_h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.set_width(width);
            ui.push_id(id, |ui| {
                egui::Frame::new()
                    .stroke(Stroke::new(1.0, DRAWER_BORDER))
                    .corner_radius(DRAWER_INPUT_RADIUS)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        let header_h = 28.0;
                        let (header_rect, _) = ui.allocate_exact_size(
                            Vec2::new(width, header_h),
                            Sense::hover(),
                        );
                        ui.painter().line_segment(
                            [
                                egui::pos2(header_rect.left(), header_rect.bottom()),
                                egui::pos2(header_rect.right(), header_rect.bottom()),
                            ],
                            Stroke::new(1.0, DRAWER_BORDER),
                        );
                        let count_label = if selected.is_empty() {
                            ids.len().to_string()
                        } else {
                            format!("{}/{}", selected.len(), ids.len())
                        };
                        let header_label = format!("{title} ({count_label})");
                        let header_galley = text_align::layout_vcentered_galley(
                            ui,
                            header_label,
                            ui_font_id(14.0),
                            Color32::BLACK,
                            ICON_ROW_LINE_HEIGHT,
                        );
                        text_align::paint_galley_row_centered(
                            ui,
                            header_rect.left() + 12.0,
                            header_rect.center().y,
                            header_galley,
                            Color32::BLACK,
                        );

                        ScrollArea::vertical()
                            .id_salt(id.with("list"))
                            .max_height(TRANSFER_LIST_H)
                            .min_scrolled_height(TRANSFER_LIST_H)
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .show(ui, |ui| {
                                ui.set_width(width);
                                ui.vertical(|ui| {
                                    for item_id in ids {
                                        let Some((_, title, kind)) = candidates
                                            .iter()
                                            .find(|(cid, _, _)| cid == item_id)
                                        else {
                                            continue;
                                        };
                                        let is_sel = selected.contains(item_id);
                                        let row_h = 28.0;
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), row_h),
                                            Sense::click(),
                                        );
                                        if is_sel {
                                            ui.painter().rect_filled(rect, 4.0, ACCENT);
                                        }
                                        let cy = rect.center().y;
                                        let row_color = if is_sel {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_rgb(60, 60, 70)
                                        };
                                        text_align::paint_icon_text_row(
                                            ui,
                                            cy,
                                            rect.left() + 8.0,
                                            icons::kind_icon(*kind),
                                            Icon::DEFAULT_SIZE,
                                            8.0,
                                            title,
                                            ui_font_id(14.0),
                                            row_color,
                                            ICON_ROW_LINE_HEIGHT,
                                        );
                                        if resp.clicked() {
                                            if is_sel {
                                                selected.retain(|s| s != item_id);
                                            } else {
                                                selected.push(item_id.clone());
                                            }
                                        }
                                    }
                                });
                            });
                    });
            });
        },
    );
}

fn transfer_arrows(
    ui: &mut Ui,
    can_right: bool,
    can_left: bool,
    move_to_selected: &mut bool,
    move_to_all: &mut bool,
) {
    let col_h = TRANSFER_LIST_H + 28.0;
    ui.allocate_ui_with_layout(
        Vec2::new(TRANSFER_ARROWS_W, col_h),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(TRANSFER_LIST_H * 0.35);
            if transfer_arrow_btn(ui, Icon::ArrowRight, can_right).clicked() {
                *move_to_selected = true;
            }
            ui.add_space(8.0);
            if transfer_arrow_btn(ui, Icon::ArrowLeft, can_left).clicked() {
                *move_to_all = true;
            }
        },
    );
}

fn transfer_arrow_btn(ui: &mut Ui, icon: Icon, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(28.0),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    if ui.is_rect_visible(rect) {
        if resp.hovered() && enabled {
            ui.painter()
                .rect_filled(rect, TOP_BAR_ICON_RADIUS, TOP_BAR_ICON_HOVER);
        }
        let tint = if enabled {
            Color32::from_rgb(60, 60, 70)
        } else {
            Color32::from_rgb(200, 200, 205)
        };
        icons::paint_icon(ui, icon, rect.center(), 16.0, tint);
    }
    resp
}

fn draw_folder_fields(ui: &mut Ui, folder_mode: &mut u8) {
    form_section(ui, "选择模式", |ui| {
        let options: [u8; 3] = [0, 1, 2];
        segmented_control(ui, "folder_mode", folder_mode, &options, true, |ui, mode, tint, seg_rect| {
            paint_segment_text(ui, seg_rect, folder_mode_label(*mode), tint);
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(folder_mode_hint(*folder_mode))
                .size(12.0)
                .color(DRAWER_WEAK_TEXT),
        );
    });
}

fn folder_mode_label(mode: u8) -> &'static str {
    match mode {
        1 => "单选",
        2 => "多选",
        _ => "默认",
    }
}

fn folder_mode_hint(mode: u8) -> &'static str {
    match mode {
        1 => "此文件夹内的直接子项目一次只能开启一个。",
        2 => "此文件夹内的直接子项目可以同时开启多个。",
        _ => "继承偏好设置中的全局选择模式。",
    }
}

fn segmented_control<T: Copy + PartialEq>(
    ui: &mut Ui,
    id_salt: &str,
    selected: &mut T,
    options: &[T],
    enabled: bool,
    mut render_label: impl FnMut(&Ui, &T, Color32, egui::Rect),
) {
    let n = options.len().max(1);
    let row_h = SEGMENTED_HEIGHT - SEGMENTED_INNER * 2.0;
    let label_tint = if enabled {
        Color32::from_rgb(60, 60, 70)
    } else {
        Color32::from_rgb(180, 180, 185)
    };
    ui.push_id(id_salt, |ui| {
        egui::Frame::new()
            .fill(DRAWER_SEGMENTED_BG)
            .corner_radius(DRAWER_INPUT_RADIUS)
            .inner_margin(SEGMENTED_INNER)
            .show(ui, |ui| {
                let inner_w = ui.available_width();
                let (row_rect, _) =
                    ui.allocate_exact_size(Vec2::new(inner_w, row_h), Sense::hover());

                let seg_w_base = (row_rect.width() / n as f32).floor();
                let mut seg_x = row_rect.min.x;

                for (i, opt) in options.iter().enumerate() {
                    let seg_w = if i + 1 == n {
                        row_rect.max.x - seg_x
                    } else {
                        seg_w_base
                    };
                    let seg_rect = egui::Rect::from_min_size(
                        egui::pos2(seg_x, row_rect.min.y),
                        Vec2::new(seg_w, row_h),
                    );
                    seg_x += seg_w;

                    let active = *selected == *opt;
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(seg_rect), |ui| {
                        if active {
                            ui.painter().rect_filled(
                                seg_rect.translate(egui::vec2(0.0, 1.0)),
                                DRAWER_INPUT_RADIUS,
                                Color32::from_black_alpha(12),
                            );
                            ui.painter().rect_filled(seg_rect, DRAWER_INPUT_RADIUS, Color32::WHITE);
                        }

                        render_label(ui, opt, label_tint, seg_rect);

                        let resp = ui.interact(
                            seg_rect,
                            ui.id().with(i),
                            if enabled {
                                Sense::click()
                            } else {
                                Sense::hover()
                            },
                        );
                        if enabled && resp.clicked() {
                            *selected = *opt;
                        }
                    });
                }
            });
    });
}

fn refresh_label(secs: u64) -> &'static str {
    REFRESH_INTERVALS
        .iter()
        .find(|(s, _)| *s == secs)
        .map(|(_, l)| *l)
        .unwrap_or("从不")
}

fn try_save(
    state: &mut EditHostsState,
    manifest: &mut Manifest,
    paths: &AppPaths,
) -> EditHostsResult {
    let title = state.draft.title.trim();
    if title.is_empty() {
        state.title_error = true;
        return EditHostsResult::None;
    }
    if state.is_add() {
        let parent_id = state.parent_id.clone();
        let id = add_draft(
            &mut manifest.root,
            &state.draft,
            parent_id.as_deref(),
        );
        if let Some(pid) = parent_id.as_deref() {
            ensure_folder_expanded(&mut manifest.root, pid);
        }
        let _ = manifest.save(paths);
        state.open = false;
        state.parent_id = None;
        state.draft = HostsNodeDraft::for_add();
        EditHostsResult::Saved { id }
    } else if let Some(EditHostsMode::Edit { id }) = state.mode.clone() {
        state.draft.id = Some(id.clone());
        update_node_in_root(&mut manifest.root, &state.draft);
        let _ = manifest.save(paths);
        state.open = false;
        EditHostsResult::Saved { id }
    } else {
        EditHostsResult::None
    }
}

fn drawer_text_button(
    ui: &mut Ui,
    label: &str,
    fill: Color32,
    stroke: Stroke,
    text_color: Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_at_least(
        Vec2::new(DRAWER_BTN_MIN_W, DRAWER_BTN_H),
        Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        ui.painter().rect(
            rect,
            DRAWER_INPUT_RADIUS,
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
    response
}

fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    drawer_text_button(ui, label, ACCENT, Stroke::NONE, Color32::WHITE)
}

fn outline_button(ui: &mut Ui, label: &str) -> egui::Response {
    drawer_text_button(
        ui,
        label,
        Color32::WHITE,
        Stroke::new(1.0, ACCENT),
        ACCENT,
    )
}

fn outline_button_with_icon(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    danger: bool,
) -> egui::Response {
    let stroke = if danger {
        Stroke::new(1.0, ACCENT)
    } else {
        Stroke::new(1.0, DRAWER_INPUT_BORDER)
    };
    ui.horizontal(|ui| {
        icons::icon(
            ui,
            icon,
            Icon::DEFAULT_SIZE,
            if danger {
                ACCENT
            } else {
                Color32::from_rgb(60, 60, 70)
            },
        );
        ui.add(
            egui::Button::new(label)
                .fill(Color32::WHITE)
                .stroke(stroke)
                .corner_radius(DRAWER_INPUT_RADIUS)
                .min_size(Vec2::new(120.0, 36.0)),
        )
    })
    .inner
}
