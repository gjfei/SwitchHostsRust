use switch_hosts_core::hosts_apply::elevation::SystemElevation;
use switch_hosts_core::hosts_apply::pipeline::ApplyPipeline;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::entries;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::storage::paths::AppPaths;
use switch_hosts_core::storage::trashcan::Trashcan;
use switch_hosts_core::toggle::toggle_item;
use eframe::egui;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

use crate::panels::{
    draw_activity_bar, draw_details, draw_editor, draw_find_replace, draw_preferences,
    draw_trash, draw_tree, FindReplaceState,
};
use crate::tray_native::{TrayAction, TrayController};

pub struct SwitchHostsApp {
    paths: AppPaths,
    target: HostsTarget,
    config: AppConfig,
    manifest: Manifest,
    trashcan: Trashcan,
    selected_id: Option<String>,
    editor_text: String,
    view_trash: bool,
    test_mode: bool,
    status_message: Option<String>,
    show_preferences: bool,
    show_find_replace: bool,
    find_replace: FindReplaceState,
    tray: Option<TrayController>,
}

impl SwitchHostsApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, paths: AppPaths, target: HostsTarget) -> Self {
        let config = AppConfig::load(&paths.config_file);
        let manifest = Manifest::load(&paths).unwrap_or_default();
        let trashcan = Trashcan::load(&paths.trashcan_file);
        let test_mode = matches!(target, HostsTarget::File(_)) && cfg!(debug_assertions);
        let tray = TrayController::try_new(&manifest);
        Self {
            paths,
            target,
            config,
            manifest,
            trashcan,
            selected_id: None,
            editor_text: String::new(),
            view_trash: false,
            test_mode,
            status_message: None,
            show_preferences: false,
            show_find_replace: false,
            find_replace: FindReplaceState::default(),
            tray,
        }
    }

    fn reload_editor(&mut self) {
        if let Some(id) = &self.selected_id {
            self.editor_text = entries::read_entry(&self.paths.entries_dir, id).unwrap_or_default();
        } else {
            self.editor_text.clear();
        }
    }

    fn save_editor(&mut self) {
        if let Some(id) = &self.selected_id.clone() {
            let _ = entries::write_entry(&self.paths.entries_dir, id, &self.editor_text);
        }
    }

    fn apply_hosts(&mut self) {
        let elevation = SystemElevation;
        let pipeline = ApplyPipeline {
            paths: &self.paths,
            config: &self.config,
            elevation: &elevation,
        };
        let _ = pipeline.apply(&self.manifest, &self.target);
    }

    fn show_main_window(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn handle_tray_action(&mut self, ctx: &egui::Context, action: TrayAction) {
        match action {
            TrayAction::ShowWindow => self.show_main_window(ctx),
            TrayAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            TrayAction::ToggleScheme(id) => {
                toggle_item(&mut self.manifest.root, &id, self.config.choice_mode);
                let _ = self.manifest.save(&self.paths);
                self.apply_hosts();
                if let Some(tray) = &mut self.tray {
                    tray.refresh(&self.manifest);
                }
                self.status_message = Some(format!("托盘已切换方案 {id}"));
            }
        }
    }

    fn poll_tray_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(tray) = &self.tray {
                if let Some(action) = tray.map_menu_event(&event) {
                    self.handle_tray_action(ctx, action);
                }
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let Some(action) = TrayController::map_tray_event(&event) {
                self.handle_tray_action(ctx, action);
            }
        }
    }
}

impl eframe::App for SwitchHostsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_tray_events(ctx);

        if self.config.tray_mini_window && self.tray.is_some() {
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        if self.test_mode {
            egui::TopBottomPanel::top("test_banner").show(ctx, |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(200, 120, 0),
                    "测试模式 — 写入 dev test.hosts，非系统 /etc/hosts",
                );
            });
        }

        draw_activity_bar(ctx, &mut self.view_trash);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("偏好设置").clicked() {
                    self.show_preferences = true;
                }
                if ui.button("查找 / 替换").clicked() {
                    self.show_find_replace = true;
                }
            });
        });

        if draw_preferences(ctx, &mut self.show_preferences, &mut self.config) {
            let _ = self.config.save(&self.paths.config_file);
            if let Ok(exe) = std::env::current_exe() {
                let _ = crate::lifecycle::sync_launch_at_login(&self.config, &exe);
            }
            self.status_message = Some("偏好设置已保存".into());
        }

        if draw_find_replace(
            ctx,
            &mut self.show_find_replace,
            &mut self.find_replace,
            &mut self.config,
            &self.manifest,
            &self.paths,
        ) {
            self.reload_editor();
            self.status_message = Some(format!(
                "已替换 {} 处",
                self.find_replace.last_count
            ));
        }

        egui::SidePanel::left("tree_panel")
            .default_width(self.config.left_panel_width as f32)
            .show(ctx, |ui| {
                if self.view_trash {
                    draw_trash(ui, &self.trashcan);
                } else if draw_tree(ui, &mut self.manifest, &mut self.selected_id, &self.config) {
                    self.reload_editor();
                    let _ = self.manifest.save(&self.paths);
                    if let Some(tray) = &mut self.tray {
                        tray.refresh(&self.manifest);
                    }
                }
            });

        if self.config.right_panel_show {
            egui::SidePanel::right("details_panel")
                .default_width(self.config.right_panel_width as f32)
                .show(ctx, |ui| {
                    draw_details(ui, &self.manifest, self.selected_id.as_deref());
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("应用 (Apply)").clicked() {
                    self.save_editor();
                    let _ = self.manifest.save(&self.paths);
                    self.apply_hosts();
                    self.status_message = Some(format!(
                        "已写入: {}",
                        self.target.path().display()
                    ));
                }
                if ui.button("保存条目").clicked() {
                    self.save_editor();
                    if self.manifest.save(&self.paths).is_ok() {
                        self.status_message = Some("条目与 manifest 已保存".into());
                    }
                }
            });
            if let Some(msg) = &self.status_message {
                ui.small(msg);
            }
            draw_editor(ui, &mut self.editor_text, self.selected_id.as_deref());
        });
    }
}

/// 从 manifest 构建托盘菜单标签（单元测试用，无需原生托盘 API）。
pub fn tray_menu_labels(manifest: &Manifest) -> Vec<String> {
    crate::tray::build_tray_menu(manifest)
        .into_iter()
        .map(|e| e.label)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tray_labels_show_checkmarks() {
        let manifest = Manifest {
            root: json!([
                { "id": "a", "title": "A", "on": true },
                { "id": "b", "title": "B", "on": false }
            ])
            .as_array()
            .cloned()
            .unwrap(),
            ..Default::default()
        };
        let labels = tray_menu_labels(&manifest);
        assert!(labels[0].starts_with('✓'));
        assert!(labels[1].contains('○') || labels[1].starts_with(' '));
    }
}
