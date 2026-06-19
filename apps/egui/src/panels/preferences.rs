//! 偏好设置抽屉（对齐 `SwitchHosts/src/renderer/components/Pref/`）。

use switch_hosts_core::hosts_apply::{
    clear_cmd_history, cmd_history_path, delete_cmd_history_item, load_cmd_history,
    CommandRunResult,
};
use switch_hosts_core::hosts_apply::target::system_hosts_path;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui::{self, Color32, Id, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::config_effects::reveal_path_in_file_manager;
use crate::fonts::ui_font_id;
use crate::icons::Icon;
use crate::segmented::{SegmentedConfig, segmented_text_values};
use crate::panels::drawer::{
    backdrop_dismiss_clicked, drawer_panel_frame, drawer_select, draw_drawer_header, outline_button,
    paint_side_drawer_backdrop, primary_button, side_drawer_geometry, DRAWER_BTN_H,
    DRAWER_INPUT_HEIGHT,
};
use crate::theme::{self, layout};

const TAB_BAR_H: f32 = 40.0;
const TAB_CONTENT_PAD_Y: f32 = 20.0;
const HTTP_API_PORT: u16 = 50761;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PrefTab {
    #[default]
    General,
    Commands,
    Proxy,
    Advanced,
}

impl PrefTab {
    fn label(self) -> &'static str {
        match self {
            Self::General => "通用",
            Self::Commands => "命令",
            Self::Proxy => "代理",
            Self::Advanced => "高级",
        }
    }

    fn needs_footer(self) -> bool {
        matches!(self, Self::Commands | Self::Proxy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DraftSaveStatus {
    #[default]
    Idle,
    Saved,
}

#[derive(Debug, Default)]
pub struct PreferencesState {
    pub open: bool,
    open_last_frame: bool,
    active_tab: PrefTab,
    draft_save_status: DraftSaveStatus,
    draft_saved_at: Option<f64>,
    draft_cmd: String,
    draft_use_proxy: bool,
    draft_proxy_protocol: String,
    draft_proxy_host: String,
    draft_proxy_port: u32,
    show_cmd_history: bool,
    cmd_history: Vec<CommandRunResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreferencesAction {
    #[default]
    None,
    ConfigChanged,
}

impl PreferencesState {
    pub fn open_drawer(&mut self) {
        self.open = true;
    }

    fn sync_drafts_from_config(&mut self, config: &AppConfig) {
        self.draft_cmd = config.cmd_after_hosts_apply.clone();
        self.draft_use_proxy = config.use_proxy;
        self.draft_proxy_protocol = config.proxy_protocol.clone();
        self.draft_proxy_host = config.proxy_host.clone();
        self.draft_proxy_port = config.proxy_port;
    }

    fn reload_cmd_history(&mut self, paths: &AppPaths) {
        self.cmd_history = load_cmd_history(&cmd_history_path(&paths.histories_dir))
            .unwrap_or_default();
        self.cmd_history.reverse();
    }
}

pub fn draw_preferences_drawer(
    ctx: &egui::Context,
    state: &mut PreferencesState,
    config: &mut AppConfig,
    paths: &AppPaths,
) -> PreferencesAction {
    if !state.open {
        state.open_last_frame = false;
        return PreferencesAction::None;
    }

    if !state.open_last_frame {
        state.sync_drafts_from_config(config);
        state.draft_save_status = DraftSaveStatus::Idle;
    }

    if state.draft_save_status == DraftSaveStatus::Saved {
        if let Some(at) = state.draft_saved_at {
            if ctx.input(|i| i.time) - at > 1.8 {
                state.draft_save_status = DraftSaveStatus::Idle;
                state.draft_saved_at = None;
            }
        }
    }

    let mut action = PreferencesAction::None;
    let allow_backdrop_dismiss = state.open_last_frame;
    let geom = side_drawer_geometry(ctx, layout::DRAWER_WIDTH_LG);

    paint_side_drawer_backdrop(ctx, "pref_backdrop", geom.backdrop_rect);
    if allow_backdrop_dismiss
        && backdrop_dismiss_clicked(ctx, geom.backdrop_rect, geom.drawer_rect, true)
    {
        state.open = false;
        state.open_last_frame = false;
        return PreferencesAction::None;
    }

    egui::Area::new(Id::new("pref_drawer"))
        .order(egui::Order::Foreground)
        .fixed_pos(geom.area_rect.min)
        .show(ctx, |ui| {
            ui.set_min_size(geom.area_rect.size());
            ui.set_max_size(geom.area_rect.size());

            drawer_panel_frame(ctx)
                .outer_margin(geom.shadow_margin)
                .show(ui, |ui| {
                    ui.set_width(geom.drawer_rect.width());
                    ui.set_height(geom.drawer_rect.height());

                    ui.vertical(|ui| {
                        if draw_drawer_header(ui, Icon::Settings, "偏好设置", "pref_close") {
                            state.open = false;
                        }

                        let footer_h = if state.active_tab.needs_footer() {
                            layout::DRAWER_FOOTER_HEIGHT
                        } else {
                            0.0
                        };
                        let body_h = ui.available_height() - footer_h;
                        let body_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(geom.drawer_rect.width(), body_h.max(0.0)),
                        );
                        ui.painter().rect_filled(body_rect, 0.0, theme::app(ctx).editor_bg);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            draw_tab_bar(ui, &mut state.active_tab);
                            ScrollArea::vertical()
                                .id_salt("pref_drawer_body")
                                .auto_shrink([false; 2])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                )
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add_space(layout::DRAWER_PAD);
                                        ui.vertical(|ui| {
                                            ui.set_width(
                                                layout::DRAWER_WIDTH_LG - layout::DRAWER_PAD * 2.0,
                                            );
                                            ui.add_space(TAB_CONTENT_PAD_Y);
                                            match state.active_tab {
                                                PrefTab::General => draw_general_tab(
                                                    ui,
                                                    config,
                                                    paths,
                                                    &mut action,
                                                ),
                                                PrefTab::Commands => draw_commands_tab(
                                                    ui,
                                                    state,
                                                    paths,
                                                ),
                                                PrefTab::Proxy => draw_proxy_tab(ui, state),
                                                PrefTab::Advanced => draw_advanced_tab(
                                                    ui,
                                                    config,
                                                    paths,
                                                    &mut action,
                                                ),
                                            }
                                            ui.add_space(60.0);
                                        });
                                    });
                                });
                        });

                        if state.active_tab.needs_footer() {
                            let footer_rect = egui::Rect::from_min_size(
                                ui.cursor().min,
                                Vec2::new(geom.drawer_rect.width(), layout::DRAWER_FOOTER_HEIGHT),
                            );
                            ui.allocate_new_ui(
                                egui::UiBuilder::new().max_rect(footer_rect),
                                |ui| {
                                    if draw_draft_footer(
                                        ui,
                                        state,
                                        config,
                                        paths,
                                        &mut action,
                                    ) {
                                        state.draft_save_status = DraftSaveStatus::Saved;
                                        state.draft_saved_at =
                                            Some(ctx.input(|i| i.time));
                                    }
                                },
                            );
                        }
                    });
                });
        });

    state.open_last_frame = state.open;
    action
}

fn draw_tab_bar(ui: &mut Ui, active: &mut PrefTab) {
    let t = theme::app(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, TAB_BAR_H), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, t.separator));

    let tabs = [
        PrefTab::General,
        PrefTab::Commands,
        PrefTab::Proxy,
        PrefTab::Advanced,
    ];
    let tab_w = rect.width() / tabs.len() as f32;
    for (i, tab) in tabs.iter().enumerate() {
        let tab_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + tab_w * i as f32, rect.top()),
            Vec2::new(tab_w, TAB_BAR_H),
        );
        let resp = ui.interact(tab_rect, ui.id().with(i), Sense::click());
        if resp.clicked() {
            *active = *tab;
        }
        let color = if *active == *tab {
            t.accent
        } else if resp.hovered() {
            t.text
        } else {
            t.weak_text
        };
        ui.painter().text(
            tab_rect.center(),
            egui::Align2::CENTER_CENTER,
            tab.label(),
            ui_font_id(14.0),
            color,
        );
        if *active == *tab {
            ui.painter().hline(
                egui::Rangef::new(tab_rect.left() + 8.0, tab_rect.right() - 8.0),
                tab_rect.bottom() - 1.0,
                Stroke::new(2.0, t.accent),
            );
        }
    }
}

fn draw_general_tab(
    ui: &mut Ui,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) {
    pref_grid_row(ui, "语言", |ui| {
        let selected_key = config.locale.as_deref().unwrap_or("system").to_string();
        let locale_display = locale_label(&selected_key);
        drawer_select(ui, "pref_locale", 200.0, &locale_display, |ui| {
            for (value, label) in LOCALE_OPTIONS {
                if ui
                    .selectable_label(selected_key == *value, *label)
                    .clicked()
                {
                    config.locale = if *value == "system" {
                        None
                    } else {
                        Some(value.to_string())
                    };
                    save_if_changed(true, config, paths, action);
                }
            }
        });
    });
    ui.add_space(layout::DRAWER_SECTION_GAP);

    pref_grid_row(ui, "主题", |ui| {
        theme_segmented(ui, config, paths, action);
    });
    ui.add_space(layout::DRAWER_SECTION_GAP);

    pref_grid_row(ui, "写入模式", |ui| {
        ui.vertical(|ui| {
            write_mode_segmented(ui, config, paths, action);
            pref_description(ui, if config.write_mode == "overwrite" {
                "覆盖：每次写入时替换整个系统 hosts 文件内容。"
            } else {
                "追加：在 SWITCHHOSTS 标记区域内更新内容，保留文件其他部分。"
            });
        });
    });
    ui.add_space(layout::DRAWER_SECTION_GAP);

    pref_grid_row(ui, "选择模式", |ui| {
        ui.vertical(|ui| {
            choice_mode_segmented(ui, config, paths, action);
            pref_description(ui, "控制顶层 hosts 方案的单选或多选行为。");
        });
    });
    ui.add_space(layout::DRAWER_SECTION_GAP);

    save_if_changed(
        pref_checkbox(ui, &mut config.launch_at_login, "登录时启动", None),
        config,
        paths,
        action,
    );

    save_if_changed(
        pref_checkbox(ui, &mut config.hide_at_launch, "启动时隐藏主窗口", None),
        config,
        paths,
        action,
    );

    #[cfg(target_os = "macos")]
    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.hide_dock_icon,
            "隐藏 Dock 图标",
            Some("需要重启应用后生效。"),
        ),
        config,
        paths,
        action,
    );

    #[cfg(target_os = "linux")]
    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.use_system_window_frame,
            "使用系统窗口边框",
            Some("需要重启应用后生效。"),
        ),
        config,
        paths,
        action,
    );
}

fn draw_commands_tab(ui: &mut Ui, state: &mut PreferencesState, paths: &AppPaths) {
    let t = theme::app(ui.ctx());
    ui.label(
        RichText::new("应用 hosts 后执行的命令")
            .size(14.0)
            .color(t.text),
    );
    pref_description(ui, "每次成功写入系统 hosts 后，将执行以下 shell 命令（超时 30 秒）。");
    ui.add_space(8.0);

    egui::Frame::new()
        .stroke(Stroke::new(1.0, t.input_border))
        .corner_radius(t.corner_input())
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.draft_cmd)
                    .desired_rows(6)
                    .font(ui_font_id(14.0))
                    .hint_text("# echo \"ok!\""),
            );
        });

    ui.add_space(12.0);
    let label = if state.show_cmd_history {
        "隐藏历史"
    } else {
        "显示历史"
    };
    if outline_button(ui, label).clicked() {
        state.show_cmd_history = !state.show_cmd_history;
        if state.show_cmd_history {
            state.reload_cmd_history(paths);
        }
    }

    if state.show_cmd_history {
        ui.add_space(12.0);
        draw_cmd_history(ui, state, paths);
    }
}

fn draw_proxy_tab(ui: &mut Ui, state: &mut PreferencesState) {
    let t = theme::app(ui.ctx());
    pref_checkbox_draft(
        ui,
        &mut state.draft_use_proxy,
        "使用代理",
        Some("远程 hosts 刷新与 URL 导入将通过代理访问网络。"),
    );

    ui.add_space(12.0);
    let enabled = state.draft_use_proxy;
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("协议").size(14.0).color(t.weak_text));
            ui.add_space(8.0);
            drawer_select(
                ui,
                "pref_proxy_protocol",
                200.0,
                &state.draft_proxy_protocol.to_uppercase(),
                |ui| {
                    for p in ["http", "https", "socks5"] {
                        if ui
                            .selectable_label(state.draft_proxy_protocol == p, p.to_uppercase())
                            .clicked()
                        {
                            state.draft_proxy_protocol = p.to_string();
                        }
                    }
                },
            );
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("主机").size(14.0).color(t.weak_text));
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.draft_proxy_host)
                    .desired_width(200.0)
                    .font(ui_font_id(14.0)),
            );
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("端口").size(14.0).color(t.weak_text));
            ui.add_space(8.0);
            ui.add(
                egui::DragValue::new(&mut state.draft_proxy_port)
                    .range(0..=65535)
                    .speed(1.0),
            );
        });
    });
}

fn draw_advanced_tab(
    ui: &mut Ui,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) {
    #[cfg(target_os = "macos")]
    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.show_title_on_tray,
            "在托盘显示当前方案标题",
            None,
        ),
        config,
        paths,
        action,
    );

    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.remove_duplicate_records,
            "去除重复记录",
            Some("聚合 hosts 时，相同域名只保留最后一条，其余转为注释。"),
        ),
        config,
        paths,
        action,
    );

    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.refresh_remote_hosts_on_startup,
            "启动时刷新远程 hosts",
            Some("应用启动约 5 秒后，自动刷新所有远程方案。"),
        ),
        config,
        paths,
        action,
    );

    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.multi_chose_folder_switch_all,
            "切换文件夹时同步子项目",
            Some("多选模式下，切换文件夹开关时同步其直接子项。"),
        ),
        config,
        paths,
        action,
    );

    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.tray_mini_window,
            "关闭窗口时最小化到托盘",
            None,
        ),
        config,
        paths,
        action,
    );

    ui.add_space(8.0);
    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.http_api_on,
            "启用本地 HTTP API",
            Some(&format!("在端口 {HTTP_API_PORT} 提供 HTTP 接口。")),
        ),
        config,
        paths,
        action,
    );

    ui.horizontal(|ui| {
        ui.add_space(layout::CHECKBOX_NESTED_INDENT);
        ui.add_enabled_ui(config.http_api_on, |ui| {
            save_if_changed(
                pref_checkbox(
                    ui,
                    &mut config.http_api_only_local,
                    "仅绑定 127.0.0.1",
                    None,
                ),
                config,
                paths,
                action,
            );
        });
    });

    let t = theme::app(ui.ctx());
    ui.add_space(layout::DRAWER_SECTION_GAP);
    ui.label(
        RichText::new("帮助改进 SwitchHosts")
            .size(14.0)
            .color(t.text),
    );
    pref_description(ui, "可选发送匿名使用数据以帮助改进产品（当前版本尚未接入上报）。");
    save_if_changed(
        pref_checkbox(
            ui,
            &mut config.send_usage_data,
            "我同意发送使用数据",
            None,
        ),
        config,
        paths,
        action,
    );

    ui.add_space(layout::DRAWER_SECTION_GAP);
    ui.label(
        RichText::new("我的 hosts 文件在哪？")
            .size(14.0)
            .color(t.text),
    );
    pref_description(ui, "系统 hosts 文件路径：");
    let hosts_path = system_hosts_path();
    draw_path_link(ui, &hosts_path);

    ui.add_space(layout::DRAWER_SECTION_GAP);
    ui.label(
        RichText::new("我的数据在哪？")
            .size(14.0)
            .color(t.text),
    );
    pref_description(ui, "SwitchHosts 数据目录：");
    draw_path_link(ui, &paths.root);
}

fn draw_cmd_history(ui: &mut Ui, state: &mut PreferencesState, paths: &AppPaths) {
    let t = theme::app(ui.ctx());
    if state.cmd_history.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("暂无记录").size(13.0).color(t.weak_text));
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if outline_button(ui, "清空全部").clicked() {
                let path = cmd_history_path(&paths.histories_dir);
                let _ = clear_cmd_history(&path);
                state.cmd_history.clear();
            }
        });
    });
    ui.add_space(8.0);

    for item in state.cmd_history.clone() {
        let color = if item.success {
            Color32::from_rgb(235, 251, 238)
        } else {
            Color32::from_rgb(255, 240, 240)
        };
        egui::Frame::new()
            .fill(color)
            .corner_radius(t.corner_input())
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("#{}", item.id))
                            .size(12.0)
                            .color(t.text),
                    );
                    ui.label(
                        RichText::new(format_cmd_time(item.add_time_ms))
                            .size(12.0)
                            .color(t.weak_text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if outline_button(ui, "删除").clicked() {
                            let path = cmd_history_path(&paths.histories_dir);
                            let _ = delete_cmd_history_item(&path, &item.id);
                            state.reload_cmd_history(paths);
                        }
                    });
                });
                if !item.stdout.is_empty() {
                    ui.label(RichText::new("stdout:").size(12.0).strong());
                    ui.label(
                        RichText::new(&item.stdout)
                            .size(12.0)
                            .color(t.text),
                    );
                }
                if !item.stderr.is_empty() {
                    ui.label(RichText::new("stderr:").size(12.0).strong());
                    ui.label(
                        RichText::new(&item.stderr)
                            .size(12.0)
                            .color(t.text),
                    );
                }
            });
        ui.add_space(8.0);
    }
}

fn draw_draft_footer(
    ui: &mut Ui,
    state: &mut PreferencesState,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) -> bool {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, layout::DRAWER_FOOTER_HEIGHT), Sense::hover());

    let row_top = rect.top() + (layout::DRAWER_FOOTER_HEIGHT - DRAWER_BTN_H) * 0.5;
    let row_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left(), row_top),
        egui::pos2(rect.right(), row_top + DRAWER_BTN_H),
    );

    let mut saved = false;
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(row_rect), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(layout::DRAWER_PAD);
            let label = match state.draft_save_status {
                DraftSaveStatus::Saved => "已保存",
                DraftSaveStatus::Idle => "保存",
            };
            if primary_button(ui, label).clicked() {
                match state.active_tab {
                    PrefTab::Commands => {
                        config.cmd_after_hosts_apply = state.draft_cmd.clone();
                    }
                    PrefTab::Proxy => {
                        config.use_proxy = state.draft_use_proxy;
                        config.proxy_protocol = state.draft_proxy_protocol.clone();
                        config.proxy_host = state.draft_proxy_host.clone();
                        config.proxy_port = state.draft_proxy_port;
                    }
                    _ => {}
                }
                if config.save(&paths.config_file).is_ok() {
                    *action = PreferencesAction::ConfigChanged;
                    saved = true;
                }
            }
        });
    });
    saved
}

fn pref_checkbox(ui: &mut Ui, value: &mut bool, label: &str, desc: Option<&str>) -> bool {
    let changed = ui.checkbox(value, label).changed();
    if let Some(d) = desc {
        ui.horizontal(|ui| {
            ui.add_space(layout::CHECKBOX_NESTED_INDENT);
            pref_description(ui, d);
        });
    }
    ui.add_space(12.0);
    changed
}

fn save_if_changed(
    changed: bool,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) {
    if changed && config.save(&paths.config_file).is_ok() {
        *action = PreferencesAction::ConfigChanged;
    }
}

fn pref_checkbox_draft(ui: &mut Ui, value: &mut bool, label: &str, desc: Option<&str>) {
    ui.checkbox(value, label);
    if let Some(d) = desc {
        ui.horizontal(|ui| {
            ui.add_space(layout::CHECKBOX_NESTED_INDENT);
            pref_description(ui, d);
        });
    }
    ui.add_space(12.0);
}

fn pref_grid_row(ui: &mut Ui, label: &str, body: impl FnOnce(&mut Ui)) {
    let t = theme::app(ui.ctx());
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(88.0, DRAWER_INPUT_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(14.0).color(t.text));
            },
        );
        body(ui);
    });
}

fn pref_description(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(12.0)
            .color(theme::app(ui.ctx()).weak_text),
    );
}

fn draw_path_link(ui: &mut Ui, path: &std::path::Path) {
    let text = path.display().to_string();
    let resp = ui.link(RichText::new(text).size(13.0));
    if resp.clicked() {
        reveal_path_in_file_manager(path);
    }
}

fn theme_segmented(
    ui: &mut Ui,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) {
    if let Some(v) = segmented_text_values(
        ui,
        "pref_theme",
        &config.theme,
        &["light", "dark", "system"],
        &["浅色", "深色", "跟随系统"],
        SegmentedConfig::default(),
    ) {
        config.theme = v.to_string();
        save_if_changed(true, config, paths, action);
    }
}

fn write_mode_segmented(
    ui: &mut Ui,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) {
    if let Some(v) = segmented_text_values(
        ui,
        "pref_write_mode",
        &config.write_mode,
        &["append", "overwrite"],
        &["追加", "覆盖"],
        SegmentedConfig::default(),
    ) {
        config.write_mode = v.to_string();
        save_if_changed(true, config, paths, action);
    }
}

fn choice_mode_segmented(
    ui: &mut Ui,
    config: &mut AppConfig,
    paths: &AppPaths,
    action: &mut PreferencesAction,
) {
    let current = if config.choice_mode == 1 { "1" } else { "2" };
    if let Some(v) = segmented_text_values(
        ui,
        "pref_choice_mode",
        current,
        &["1", "2"],
        &["单选", "多选"],
        SegmentedConfig::default(),
    ) {
        config.choice_mode = if v == "1" { 1 } else { 2 };
        save_if_changed(true, config, paths, action);
    }
}

fn format_cmd_time(ms: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}

fn locale_label(value: &str) -> String {
    LOCALE_OPTIONS
        .iter()
        .find(|(v, _)| *v == value)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| value.to_string())
}

const LOCALE_OPTIONS: &[(&str, &str)] = &[
    ("system", "跟随系统"),
    ("zh", "简体中文"),
    ("zh_hant", "繁體中文"),
    ("en", "English"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("ja", "日本語"),
    ("tr", "Türkçe"),
    ("ko", "한국어"),
    ("pl", "Polski"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pref_tab_footer_only_on_draft_tabs() {
        assert!(!PrefTab::General.needs_footer());
        assert!(PrefTab::Commands.needs_footer());
        assert!(PrefTab::Proxy.needs_footer());
        assert!(!PrefTab::Advanced.needs_footer());
    }
}
