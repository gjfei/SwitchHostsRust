//! 右侧滑出面板：添加/编辑 hosts（对齐 `EditHostsInfo.tsx`）。

use switch_hosts_core::manifest_edit::{
    add_draft, ensure_folder_expanded, list_includable_nodes, remove_node_with_parent,
    update_node_in_root, HostsNodeDraft, HostsNodeKind, REFRESH_INTERVALS,
};
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::icons::{self, Icon};
use crate::theme::ACCENT;

const DRAWER_WIDTH: f32 = 420.0;
const TITLE_MAX_LEN: usize = 50;

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
}

impl EditHostsState {
    pub fn open_add(&mut self, parent_id: Option<String>) {
        self.open = true;
        self.mode = Some(EditHostsMode::Add);
        self.draft = HostsNodeDraft::for_add();
        self.title_error = false;
        self.parent_id = parent_id;
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
) -> EditHostsResult {
    if !state.open {
        return EditHostsResult::None;
    }

    let mut result = EditHostsResult::None;
    let is_add = state.is_add();

    let screen = ctx.input(|i| i.screen_rect());
    let backdrop_rect = egui::Rect::from_min_max(
        screen.min,
        egui::pos2(screen.right() - DRAWER_WIDTH, screen.bottom()),
    );
    egui::Area::new(egui::Id::new("edit_hosts_backdrop"))
        .order(egui::Order::Background)
        .fixed_pos(backdrop_rect.min)
        .show(ctx, |ui| {
            ui.set_width(backdrop_rect.width());
            ui.set_height(backdrop_rect.height());
            let resp = ui.allocate_rect(backdrop_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(backdrop_rect, 0.0, Color32::from_black_alpha(50));
            if resp.clicked() {
                state.open = false;
                result = EditHostsResult::Cancelled;
            }
        });

    egui::SidePanel::right("edit_hosts_drawer")
        .exact_width(DRAWER_WIDTH)
        .frame(
            egui::Frame::new()
                .fill(Color32::WHITE)
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ctx, |ui| {
            let title = if is_add { "添加 hosts" } else { "编辑 hosts" };
            ui.horizontal(|ui| {
                icons::icon(ui, Icon::Pencil, 20.0, Color32::from_rgb(80, 80, 90));
                ui.heading(RichText::new(title).size(18.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::icon_button(ui, Icon::X, 18.0, Color32::from_rgb(100, 100, 110))
                        .on_hover_text("关闭")
                        .clicked()
                    {
                        state.open = false;
                        result = EditHostsResult::Cancelled;
                    }
                });
            });

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new("Hosts 类型").size(14.0));
                    ui.add_space(8.0);
                    draw_kind_selector(ui, &mut state.draft.kind, is_add);

                    ui.add_space(16.0);
                    ui.label(RichText::new("Hosts 标题").size(14.0));
                    ui.add_space(8.0);
                    draw_title_field(ui, &mut state.draft.title, &mut state.title_error);

                    match state.draft.kind {
                        HostsNodeKind::Remote => draw_remote_fields(ui, &mut state.draft),
                        HostsNodeKind::Group => draw_group_fields(ui, &mut state.draft, manifest),
                        HostsNodeKind::Folder => draw_folder_fields(ui, &mut state.draft),
                        HostsNodeKind::Local => {}
                    }
                    ui.add_space(80.0);
                });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if !is_add {
                            if trash_button(ui).clicked() {
                                if let Some(EditHostsMode::Edit { id }) = state.mode.clone() {
                                    if let Some((node, parent_id)) =
                                        remove_node_with_parent(&mut manifest.root, &id)
                                    {
                                        let _ = manifest.save(paths);
                                        state.open = false;
                                        result = EditHostsResult::MovedToTrash { node, parent_id };
                                    }
                                }
                            }
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if accent_filled_button(ui, "确定").clicked() {
                            result = try_save(state, manifest, paths);
                        }
                        if accent_outline_button(ui, "取消").clicked() {
                            state.open = false;
                            result = EditHostsResult::Cancelled;
                        }
                    });
                });
            });
        });

    result
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

fn draw_title_field(ui: &mut Ui, title: &mut String, title_error: &mut bool) {
    if title.len() > TITLE_MAX_LEN {
        title.truncate(TITLE_MAX_LEN);
    }
    let stroke = if *title_error && title.trim().is_empty() {
        Stroke::new(1.5, ACCENT)
    } else {
        Stroke::new(1.0, Color32::from_rgb(200, 200, 210))
    };
    let edit = egui::TextEdit::singleline(title)
        .desired_width(f32::INFINITY)
        .margin(egui::Margin::symmetric(12, 10));
    let resp = egui::Frame::new()
        .stroke(stroke)
        .corner_radius(6.0)
        .show(ui, |ui| ui.add(edit))
        .inner;
    if resp.changed() {
        *title_error = false;
    }
}

fn draw_remote_fields(ui: &mut Ui, draft: &mut HostsNodeDraft) {
    ui.add_space(16.0);
    ui.label(RichText::new("URL").size(14.0));
    ui.add_space(8.0);
    ui.add(
        egui::TextEdit::singleline(&mut draft.url)
            .desired_width(f32::INFINITY)
            .hint_text("https://"),
    );

    ui.add_space(16.0);
    ui.label(RichText::new("自动刷新").size(14.0));
    ui.add_space(8.0);
    egui::ComboBox::from_id_salt("refresh_interval")
        .selected_text(refresh_label(draft.refresh_interval))
        .show_ui(ui, |ui| {
            for (secs, label) in REFRESH_INTERVALS {
                ui.selectable_value(&mut draft.refresh_interval, *secs, *label);
            }
        });
}

fn refresh_label(secs: u64) -> &'static str {
    REFRESH_INTERVALS
        .iter()
        .find(|(s, _)| *s == secs)
        .map(|(_, l)| *l)
        .unwrap_or("不刷新")
}

fn draw_group_fields(ui: &mut Ui, draft: &mut HostsNodeDraft, manifest: &Manifest) {
    ui.add_space(16.0);
    ui.label(RichText::new("内容").size(14.0));
    ui.add_space(8.0);
    let candidates = list_includable_nodes(&manifest.root);
    if candidates.is_empty() {
        ui.label(
            RichText::new("暂无 local/remote 方案可选")
                .color(Color32::from_rgb(140, 140, 150)),
        );
        return;
    }
    for (id, title, kind) in candidates {
        let mut checked = draft.include.iter().any(|i| i == &id);
        ui.horizontal(|ui| {
            if ui.checkbox(&mut checked, "").changed() {
                if checked {
                    if !draft.include.contains(&id) {
                        draft.include.push(id.clone());
                    }
                } else {
                    draft.include.retain(|i| i != &id);
                }
            }
            icons::icon(ui, icons::kind_icon(kind), Icon::DEFAULT_SIZE, Color32::from_rgb(80, 80, 90));
            ui.label(title);
        });
    }
}

fn draw_folder_fields(ui: &mut Ui, draft: &mut HostsNodeDraft) {
    ui.add_space(16.0);
    ui.label(RichText::new("切换模式").size(14.0));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        folder_mode_chip(ui, &mut draft.folder_mode, 0, "默认");
        folder_mode_chip(ui, &mut draft.folder_mode, 1, "单选");
        folder_mode_chip(ui, &mut draft.folder_mode, 2, "多选");
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new(folder_mode_hint(draft.folder_mode))
            .size(12.0)
            .color(Color32::from_rgb(140, 140, 150)),
    );
}

fn folder_mode_hint(mode: u8) -> &'static str {
    match mode {
        1 => "文件夹内仅允许一个子方案启用",
        2 => "文件夹内允许多个子方案同时启用",
        _ => "继承全局切换模式设置",
    }
}

fn folder_mode_chip(ui: &mut Ui, selected: &mut u8, value: u8, label: &str) {
    let active = *selected == value;
    let stroke = if active {
        Stroke::new(1.5, ACCENT)
    } else {
        Stroke::new(1.0, Color32::from_rgb(210, 210, 218))
    };
    if ui
        .add(
            egui::Button::new(label)
                .stroke(stroke)
                .corner_radius(6.0)
                .min_size(Vec2::new(72.0, 32.0)),
        )
        .clicked()
    {
        *selected = value;
    }
}

fn draw_kind_selector(ui: &mut Ui, kind: &mut HostsNodeKind, enabled: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        for candidate in [
            HostsNodeKind::Local,
            HostsNodeKind::Remote,
            HostsNodeKind::Group,
            HostsNodeKind::Folder,
        ] {
            if enabled {
                kind_chip(ui, kind, candidate);
            } else {
                ui.add_enabled_ui(false, |ui| {
                    ui.horizontal(|ui| {
                        icons::icon(
                            ui,
                            icons::kind_icon(candidate),
                            Icon::DEFAULT_SIZE,
                            Color32::from_rgb(160, 160, 170),
                        );
                        ui.label(candidate.label());
                    });
                });
            }
        }
    });
}

fn kind_chip(ui: &mut Ui, selected: &mut HostsNodeKind, kind: HostsNodeKind) {
    let active = *selected == kind;
    let stroke = if active {
        Stroke::new(1.5, ACCENT)
    } else {
        Stroke::new(1.0, Color32::from_rgb(210, 210, 218))
    };
    let response = egui::Frame::new()
        .stroke(stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(62.0, 24.0));
            ui.horizontal(|ui| {
                icons::icon(
                    ui,
                    icons::kind_icon(kind),
                    Icon::DEFAULT_SIZE,
                    Color32::from_rgb(60, 60, 70),
                );
                ui.label(kind.label());
            });
        })
        .response;
    if response.clicked() {
        *selected = kind;
    }
}

fn trash_button(ui: &mut Ui) -> egui::Response {
    ui.horizontal(|ui| {
        icons::icon(
            ui,
            Icon::Trash,
            Icon::DEFAULT_SIZE,
            Color32::from_rgb(100, 100, 110),
        );
        ui.button("移到回收站")
    })
    .inner
}

fn accent_outline_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(ACCENT))
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.5, ACCENT))
            .corner_radius(6.0)
            .min_size(Vec2::new(88.0, 36.0)),
    )
}

fn accent_filled_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE))
            .fill(ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(6.0)
            .min_size(Vec2::new(88.0, 36.0)),
    )
}
