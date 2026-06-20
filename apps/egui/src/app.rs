use std::sync::atomic::{AtomicBool, Ordering};

use switch_hosts_core::hosts_apply::elevation::SystemElevation;
use switch_hosts_core::hosts_apply::pipeline::ApplyPipeline;
use switch_hosts_core::hosts_apply::target::{read_target_hosts_content, HostsTarget};
use switch_hosts_core::manifest_edit::{add_parent_for_selection, insert_node, remove_node_with_parent, is_editor_read_only, SYSTEM_NODE_ID};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::entries::{self, delete_entry};
use switch_hosts_core::storage::manifest::{collect_content_ids, find_node, Manifest};
use switch_hosts_core::storage::paths::AppPaths;
use switch_hosts_core::storage::trashcan::{TrashItem, Trashcan};
use switch_hosts_core::toggle::toggle_item;
use eframe::egui;
use raw_window_handle::HasWindowHandle;

use crate::config_effects::apply_config_side_effects;
use crate::data_transfer::{
    import_error_message, import_from_url as import_from_url_data, run_export_dialog,
    run_import_dialog, ExportResult, ImportResult,
};
use crate::http_api_runtime::HttpApiRuntime;
use crate::panels::{
    draw_details, draw_edit_hosts_drawer, draw_editor_with_status_bar, draw_find_replace_drawer,
    draw_history_drawer,
    draw_hosts_sidebar,
    paint_list_panel_border,
    draw_navigation, draw_panel_status_spacer, draw_preferences_drawer, draw_top_bar,
    pin_body_and_status_bar,
    draw_trash_clear_confirm,
    draw_trash_delete_confirm, draw_trash_panel, draw_import_from_url_modal, draw_import_error_modal,
    draw_apply_error_modal,
    DetailsAction, EditHostsResult,
    EditHostsState, FindReplaceAction, FindReplaceState, HistoryResult, HistoryState, NavView,
    ConfigMenuAction, ImportFromUrlResult, ImportFromUrlState, PreferencesAction, PreferencesState, TrashDeleteConfirmResult, TrashEvent, TreeEvent,
};
use crate::remote_refresh::{refresh_all_remote_hosts, refresh_remote_node};
use crate::theme::{self, layout};
use crate::tray_native::{try_init_tray, try_recv_tray_action, TrayAction, TrayController};

const FEEDBACK_URL: &str = "";
const HOMEPAGE_URL: &str = "";

fn open_url(url: &str) {
    if url.is_empty() {
        return;
    }
    if let Err(err) = webbrowser::open(url) {
        tracing::warn!("open url failed ({url}): {err}");
    }
}

pub struct SwitchHostsApp {
    paths: AppPaths,
    target: HostsTarget,
    config: AppConfig,
    manifest: Manifest,
    trashcan: Trashcan,
    selected_id: Option<String>,
    editor_text: String,
    nav_view: NavView,
    hosts_list_visible: bool,
    test_mode: bool,
    preferences: PreferencesState,
    edit_hosts: EditHostsState,
    history: HistoryState,
    find_replace: FindReplaceState,
    editor_pending_selection: Option<(usize, usize)>,
    http_api: HttpApiRuntime,
    system_dark: bool,
    startup_refresh_scheduled: bool,
    startup_refresh_done: bool,
    startup_refresh_rx: Option<std::sync::mpsc::Receiver<Manifest>>,
    /// 每次 `reload_editor` 递增，用于重置 egui TextEdit 内部缓存。
    editor_revision: u64,
    tray: Option<TrayController>,
    editor_dirty: bool,
    pending_trash_delete: Option<String>,
    pending_trash_clear: bool,
    import_from_url: ImportFromUrlState,
    import_error: Option<String>,
    apply_error: Option<String>,
    will_quit: AtomicBool,
    #[cfg(target_os = "macos")]
    traffic_lights_positioned: bool,
    #[cfg(target_os = "macos")]
    dock_icon_installed: bool,
}

impl SwitchHostsApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: AppPaths, target: HostsTarget) -> Self {
        crate::fonts::setup_fonts(&cc.egui_ctx);

        let config = AppConfig::load(&paths.config_file);
        let manifest = Manifest::load(&paths).unwrap_or_default();
        let trashcan = Trashcan::load(&paths.trashcan_file);
        let test_mode = matches!(target, HostsTarget::File(_)) && cfg!(debug_assertions);
        let tray = None;
        let hosts_list_visible = config.left_panel_show;

        let system_dark = cc.egui_ctx.global_style().visuals.dark_mode;
        theme::apply_theme(&cc.egui_ctx, &config.theme, system_dark);

        let mut http_api = HttpApiRuntime::new();
        http_api.sync(&config, &paths, &target);
        let startup_refresh_scheduled = config.refresh_remote_hosts_on_startup;

        let mut app = Self {
            paths,
            target,
            config,
            manifest,
            trashcan,
            selected_id: Some(SYSTEM_NODE_ID.to_string()),
            editor_text: String::new(),
            nav_view: NavView::Hosts,
            hosts_list_visible,
            test_mode,
            preferences: PreferencesState::default(),
            edit_hosts: EditHostsState::default(),
            history: HistoryState::default(),
            find_replace: FindReplaceState::default(),
            editor_pending_selection: None,
            http_api,
            system_dark,
            startup_refresh_scheduled,
            startup_refresh_done: false,
            startup_refresh_rx: None,
            editor_revision: 0,
            tray,
            editor_dirty: false,
            pending_trash_delete: None,
            pending_trash_clear: false,
            import_from_url: ImportFromUrlState::default(),
            import_error: None,
            apply_error: None,
            will_quit: AtomicBool::new(false),
            #[cfg(target_os = "macos")]
            traffic_lights_positioned: false,
            #[cfg(target_os = "macos")]
            dock_icon_installed: false,
        };
        app.reload_editor();
        app.apply_config_effects(&cc.egui_ctx);
        app.sync_macos_traffic_lights(cc);
        app.sync_macos_dock_icon();
        app
    }

    fn apply_config_effects(&mut self, ctx: &egui::Context) {
        apply_config_side_effects(
            ctx,
            &self.config,
            &self.paths,
            &self.target,
            self.system_dark,
            &mut self.http_api,
        );
        self.hosts_list_visible = self.config.left_panel_show;
    }

    fn tick_startup_remote_refresh(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.startup_refresh_rx {
            if let Ok(manifest) = rx.try_recv() {
                self.manifest = manifest;
                self.startup_refresh_rx = None;
                if self
                    .selected_id
                    .as_ref()
                    .is_some_and(|id| {
                        find_node(&self.manifest.root, id)
                            .is_some_and(|n| n.get("type").and_then(|v| v.as_str()) == Some("remote"))
                    })
                {
                    self.reload_editor();
                }
                ctx.request_repaint();
            }
            return;
        }

        if self.startup_refresh_done || !self.startup_refresh_scheduled {
            return;
        }
        let start = ctx.input(|i| i.time);
        if start < 5.0 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(5.0 - start));
            return;
        }
        self.startup_refresh_done = true;

        let paths = self.paths.clone();
        let config = self.config.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.startup_refresh_rx = Some(rx);
        std::thread::Builder::new()
            .name("startup-remote-refresh".into())
            .spawn(move || {
                let mut manifest = Manifest::load(&paths).unwrap_or_default();
                let _ = refresh_all_remote_hosts(&paths, &mut manifest, &config);
                let _ = tx.send(manifest);
            })
            .ok();
    }

    #[cfg(target_os = "macos")]
    fn sync_macos_traffic_lights(&mut self, handle: &impl HasWindowHandle) {
        if self.config.use_system_window_frame || self.traffic_lights_positioned {
            return;
        }
        if crate::macos::position_traffic_lights(handle) {
            self.traffic_lights_positioned = true;
        }
    }

    #[cfg(target_os = "macos")]
    fn sync_macos_dock_icon(&mut self) {
        if self.dock_icon_installed {
            return;
        }
        if crate::macos::configure_macos_app() {
            self.dock_icon_installed = true;
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn sync_macos_dock_icon(&mut self) {}

    #[cfg(not(target_os = "macos"))]
    fn sync_macos_traffic_lights(&mut self, _handle: &impl HasWindowHandle) {}

    fn reload_editor(&mut self) {
        self.editor_revision = self.editor_revision.wrapping_add(1);
        self.editor_text = if self.selected_id.as_deref() == Some(SYSTEM_NODE_ID) {
            read_target_hosts_content(&self.target)
        } else if let Some(id) = &self.selected_id {
            entries::read_entry(&self.paths.entries_dir, id).unwrap_or_default()
        } else {
            String::new()
        };
        self.editor_dirty = false;
    }

    fn save_editor(&mut self) -> bool {
        let Some(id) = self.selected_id.clone() else {
            self.editor_dirty = false;
            return false;
        };
        if id == SYSTEM_NODE_ID {
            self.editor_dirty = false;
            return false;
        }
        let _ = entries::write_entry(&self.paths.entries_dir, &id, &self.editor_text);
        self.editor_dirty = false;
        true
    }

    fn apply_hosts(&mut self) -> bool {
        let elevation = SystemElevation;
        let pipeline = ApplyPipeline {
            paths: &self.paths,
            config: &self.config,
            elevation: &elevation,
        };
        match pipeline.apply(&self.manifest, &self.target) {
            Ok(result) => {
                if result.written && self.selected_id.as_deref() == Some(SYSTEM_NODE_ID) {
                    self.reload_editor();
                }
                true
            }
            Err(err) => {
                tracing::warn!("apply hosts failed: {err}");
                self.apply_error = Some(err.user_message());
                false
            }
        }
    }

    /// 切换方案并写入 hosts；写入失败时恢复切换前的 manifest。
    fn toggle_and_apply_hosts(&mut self, id: &str) {
        let before = self.manifest.clone();
        if !toggle_item(&mut self.manifest.root, id, self.config.choice_mode) {
            return;
        }
        self.persist_manifest();
        if !self.apply_hosts() {
            self.manifest = before;
            self.persist_manifest();
        }
    }

    fn persist_manifest(&mut self) {
        let _ = self.manifest.save(&self.paths);
        if let Some(tray) = &mut self.tray {
            tray.refresh(&self.manifest);
        }
    }

    fn on_tree_event(&mut self, event: TreeEvent) {
        match event {
            TreeEvent::None => {}
            TreeEvent::SelectionChanged => self.reload_editor(),
            TreeEvent::EditRequested(id) => {
                if let Some(node) = find_node(&self.manifest.root, &id) {
                    self.edit_hosts.open_edit(&node);
                }
            }
            TreeEvent::AddRequested => {
                let parent_id = add_parent_for_selection(
                    &self.manifest.root,
                    self.selected_id.as_deref(),
                );
                self.edit_hosts.open_add(parent_id);
            }
            TreeEvent::MoveToTrashRequested(ids) => self.move_nodes_to_trash(&ids),
            TreeEvent::RefreshRequested(id) => self.refresh_remote_hosts(&id),
            TreeEvent::ToggleRequested(id) => self.toggle_and_apply_hosts(&id),
            TreeEvent::CollapsedChanged => self.persist_manifest(),
        }
    }

    fn move_nodes_to_trash(&mut self, ids: &[String]) {
        let mut moved = false;
        for id in ids {
            if id == SYSTEM_NODE_ID {
                continue;
            }
            if let Some((node, parent_id)) = remove_node_with_parent(&mut self.manifest.root, id) {
                self.trashcan.push(TrashItem {
                    id: id.clone(),
                    node,
                    parent_id,
                    deleted_at: None,
                });
                if self.selected_id.as_deref() == Some(id.as_str()) {
                    self.selected_id = Some(SYSTEM_NODE_ID.to_string());
                    self.reload_editor();
                }
                moved = true;
            }
        }
        if moved {
            let _ = self.trashcan.save(&self.paths.trashcan_file);
            self.persist_manifest();
        }
    }

    fn refresh_remote_hosts(&mut self, id: &str) {
        match refresh_remote_node(&self.paths, &mut self.manifest, &self.config, id) {
            Ok(content_changed) => {
                self.persist_manifest();
                if content_changed && self.selected_id.as_deref() == Some(id) {
                    self.reload_editor();
                    self.apply_hosts();
                }
            }
            Err(message) => {
                tracing::warn!("refresh remote hosts failed: {message}");
            }
        }
    }

    fn on_details_action(&mut self, action: DetailsAction) {
        if action.edit {
            if self.nav_view == NavView::Trash {
                if let Some(id) = self.selected_id.as_deref() {
                    if let Some(item) = self.trashcan.items.iter().find(|i| i.id == id) {
                        self.edit_hosts.open_edit(&item.node);
                    }
                }
            } else if let Some(id) = self.selected_id.as_deref() {
                if id != SYSTEM_NODE_ID {
                    if let Some(node) = find_node(&self.manifest.root, id) {
                        self.edit_hosts.open_edit(&node);
                    }
                }
            }
        }
        if action.refresh {
            if let Some(id) = self.selected_id.clone() {
                self.refresh_remote_hosts(&id);
            }
        }
        if action.restore {
            if let Some(id) = self.selected_id.clone() {
                self.restore_from_trash(&id);
            }
        }
        if action.delete {
            if let Some(id) = self.selected_id.clone() {
                self.pending_trash_delete = Some(id);
            }
        }
        if action.open_history {
            self.history.open_drawer();
        }
    }

    fn on_trash_event(&mut self, event: TrashEvent) {
        match event {
            TrashEvent::None => {}
            TrashEvent::SelectionChanged => self.reload_editor(),
            TrashEvent::RestoreRequested(id) => self.restore_from_trash(&id),
            TrashEvent::DeleteRequested(id) => {
                self.pending_trash_delete = Some(id);
            }
            TrashEvent::ClearRequested => {
                self.pending_trash_clear = true;
            }
        }
    }

    fn restore_from_trash(&mut self, id: &str) {
        let Some(item) = self.trashcan.remove(id) else {
            return;
        };
        insert_node(
            &mut self.manifest.root,
            item.node,
            item.parent_id.as_deref(),
        );
        let _ = self.trashcan.save(&self.paths.trashcan_file);
        self.persist_manifest();
    }

    fn permanently_delete_from_trash(&mut self, id: &str) {
        let Some(item) = self.trashcan.remove(id) else {
            return;
        };
        let mut content_ids = Vec::new();
        collect_content_ids(std::slice::from_ref(&item.node), &mut content_ids);
        for cid in content_ids {
            let _ = delete_entry(&self.paths.entries_dir, &cid);
        }
        let _ = self.trashcan.save(&self.paths.trashcan_file);
        if self.selected_id.as_deref() == Some(id) {
            self.selected_id = Some(SYSTEM_NODE_ID.to_string());
            self.reload_editor();
        }
    }

    fn clear_trashcan(&mut self) {
        let mut content_ids = Vec::new();
        for item in &self.trashcan.items {
            collect_content_ids(std::slice::from_ref(&item.node), &mut content_ids);
        }
        for cid in content_ids {
            let _ = delete_entry(&self.paths.entries_dir, &cid);
        }
        self.trashcan.items.clear();
        let _ = self.trashcan.save(&self.paths.trashcan_file);
        self.selected_id = Some(SYSTEM_NODE_ID.to_string());
        self.reload_editor();
    }

    fn trash_delete_title(&self, id: &str) -> String {
        self.trashcan
            .items
            .iter()
            .find(|i| i.id == id)
            .and_then(|i| i.node.get("title").and_then(|v| v.as_str()))
            .unwrap_or(id)
            .to_string()
    }

    fn show_main_window(&self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        crate::macos::show_main_window();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn prepare_for_quit(&mut self) {
        if self.will_quit.swap(true, Ordering::SeqCst) {
            return;
        }
        #[cfg(target_os = "macos")]
        crate::macos_delegate::mark_quit_requested();
        self.tray.take();
        self.http_api.shutdown();
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        self.prepare_for_quit();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        #[cfg(target_os = "macos")]
        crate::macos::quit_app();
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if self.will_quit.load(Ordering::SeqCst) {
            return;
        }
        #[cfg(target_os = "macos")]
        if crate::macos_delegate::quit_was_requested() {
            return;
        }

        if self.config.tray_mini_window && self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            // 关闭已在进行中：只做清理，让窗口自然关闭；macOS 通过
            // applicationShouldTerminateAfterLastWindowClosed 正常退出，避免重复 terminate 崩溃。
            self.prepare_for_quit();
        }
    }

    fn handle_tray_action(&mut self, ctx: &egui::Context, action: TrayAction) {
        match action {
            TrayAction::ShowWindow => self.show_main_window(ctx),
            TrayAction::Quit => self.request_quit(ctx),
            TrayAction::ToggleScheme(id) => self.toggle_and_apply_hosts(&id),
        }
    }

    fn poll_tray_events(&mut self, ctx: &egui::Context) {
        try_init_tray(&mut self.tray, &self.manifest);
        crate::tray_native::poll_tray_events_on_runloop();
        while let Some(action) = try_recv_tray_action() {
            self.handle_tray_action(ctx, action);
        }
    }

    fn reload_after_import(&mut self, ctx: &egui::Context) {
        self.manifest = Manifest::load(&self.paths).unwrap_or_default();
        self.trashcan = Trashcan::load(&self.paths.trashcan_file);
        self.nav_view = NavView::Hosts;
        self.selected_id = Some(SYSTEM_NODE_ID.to_string());
        self.reload_editor();
        if let Some(tray) = &mut self.tray {
            tray.refresh(&self.manifest);
        }
        self.apply_hosts();
        ctx.request_repaint();
    }

    fn handle_import_result(&mut self, ctx: &egui::Context, result: ImportResult) {
        match result {
            ImportResult::Cancelled => {}
            ImportResult::Success => self.reload_after_import(ctx),
            ImportResult::SoftError(code) => {
                tracing::warn!("import failed: {code}");
                self.import_error = Some(import_error_message(&code));
            }
            ImportResult::HardError(err) => {
                tracing::warn!("import failed: {err}");
                self.import_error = Some(format!("导入失败：{err}"));
            }
        }
    }

    fn handle_config_menu_action(&mut self, ctx: &egui::Context, action: ConfigMenuAction) {
        match action {
            ConfigMenuAction::None => {}
            ConfigMenuAction::OpenPreferences => self.preferences.open_drawer(),
            ConfigMenuAction::Quit => self.request_quit(ctx),
            ConfigMenuAction::OpenFeedback => open_url(FEEDBACK_URL),
            ConfigMenuAction::OpenHomepage => open_url(HOMEPAGE_URL),
            ConfigMenuAction::Export => match run_export_dialog(&self.paths) {
                ExportResult::Cancelled => {}
                ExportResult::Failed => tracing::warn!("export failed"),
                ExportResult::Success(path) => {
                    tracing::info!("exported to {}", path.display());
                }
            },
            ConfigMenuAction::Import => {
                self.handle_import_result(ctx, run_import_dialog(&self.paths));
            }
            ConfigMenuAction::ImportFromUrl => self.import_from_url.open_modal(),
            other => {
                tracing::info!("config menu action not implemented yet: {other:?}");
            }
        }
    }
}

impl eframe::App for SwitchHostsApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(visuals.window_fill()).to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.sync_macos_dock_icon();
        self.sync_macos_traffic_lights(frame);
        self.poll_tray_events(ctx);

        self.tick_startup_remote_refresh(ctx);

        let t = theme::app(ctx);

        if ctx.input(|i| {
            i.key_pressed(egui::Key::F) && (i.modifiers.command || i.modifiers.ctrl)
        }) {
            self.find_replace.open_drawer();
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            self.handle_close_request(ctx);
        }

        #[cfg(target_os = "macos")]
        if crate::macos_delegate::take_dock_show_request() {
            self.show_main_window(ctx);
        }

        // 顶栏必须最先绘制（标题栏 overlay 区域），再绘制其下方的测试横幅。
        // 顶栏对齐原版 `background: transparent`，避免 egui 不透明填充盖住 macOS 交通灯。
        egui::Panel::top("top_bar")
            .exact_size(layout::TOP_BAR_HEIGHT)
            .show_separator_line(false)
            .frame(top_bar_frame(&self.config, &t))
            .show_inside(ui, |ui| {
                let action = draw_top_bar(
                    ui,
                    &mut self.manifest,
                    &self.selected_id,
                    self.hosts_list_visible,
                    self.config.right_panel_show,
                    self.config.use_system_window_frame,
                );
                if action.toggle_left_panel {
                    self.hosts_list_visible = !self.hosts_list_visible;
                }
                if action.add_new {
                    let parent_id = add_parent_for_selection(
                        &self.manifest.root,
                        self.selected_id.as_deref(),
                    );
                    self.edit_hosts.open_add(parent_id);
                }
                if action.toggle_right_panel {
                    self.config.right_panel_show = !self.config.right_panel_show;
                }
                if let Some(id) = action.toggle_current_id {
                    self.toggle_and_apply_hosts(&id);
                }
            });

        if self.test_mode {
            egui::Panel::top("test_banner")
                .exact_size(layout::TEST_BANNER_HEIGHT)
                .show_separator_line(false)
                .frame(
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(255, 248, 230))
                        .inner_margin(egui::Margin::symmetric(8, 0)),
                )
                .show_inside(ui, |ui| {
                    ui.set_min_height(layout::TEST_BANNER_HEIGHT);
                    ui.set_max_height(layout::TEST_BANNER_HEIGHT);
                    ui.centered_and_justified(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(180, 100, 0),
                            "测试模式 — 写入 dev test.hosts",
                        );
                    });
                });
        }

        if draw_preferences_drawer(
            ctx,
            &mut self.preferences,
            &mut self.config,
            &self.paths,
        ) == PreferencesAction::ConfigChanged
        {
            self.apply_config_effects(ctx);
            self.startup_refresh_scheduled = self.config.refresh_remote_hosts_on_startup;
        }

        if let Some(action) = draw_find_replace_drawer(
            ctx,
            &mut self.find_replace,
            &mut self.config,
            &self.manifest,
            &self.paths,
        ) {
            match action {
                FindReplaceAction::None => {}
                FindReplaceAction::ContentChanged(ids) => {
                    if self
                        .selected_id
                        .as_ref()
                        .is_some_and(|id| ids.iter().any(|changed| changed == id))
                    {
                        self.reload_editor();
                    }
                }
                FindReplaceAction::JumpToMatch {
                    entry_id,
                    start_char,
                    end_char,
                } => {
                    self.nav_view = NavView::Hosts;
                    if self.selected_id.as_deref() != Some(entry_id.as_str()) {
                        self.selected_id = Some(entry_id);
                        self.reload_editor();
                    }
                    self.editor_pending_selection = Some((start_char, end_char));
                }
            }
        }

        let nav_action = draw_navigation(
            ui,
            &mut self.nav_view,
            self.hosts_list_visible,
            self.trashcan.items.len(),
        );
        if nav_action.open_history {
            self.history.open_drawer();
        }
        self.handle_config_menu_action(ctx, nav_action.config_menu);
        if nav_action.open_search {
            self.find_replace.open_drawer();
        }
        if let Some(visible) = nav_action.left_panel_visible {
            self.hosts_list_visible = visible;
        }

        if self.hosts_list_visible {
            egui::Panel::left("hosts_sidebar")
                .default_size(self.config.left_panel_width as f32)
                .frame(egui::Frame::new().fill(t.sidebar_bg))
                .show_inside(ui, |ui| {
                    paint_list_panel_border(ui);
                    pin_body_and_status_bar(
                        ui,
                        |ui| match self.nav_view {
                            NavView::Hosts => {
                                let event = draw_hosts_sidebar(
                                    ui,
                                    &mut self.manifest,
                                    &mut self.selected_id,
                                    &self.config,
                                );
                                self.on_tree_event(event);
                            }
                            NavView::Trash => {
                                let event = draw_trash_panel(
                                    ui,
                                    &self.trashcan,
                                    &mut self.selected_id,
                                );
                                self.on_trash_event(event);
                            }
                        },
                        draw_panel_status_spacer,
                    );
                });
        }

        if self.config.right_panel_show {
            egui::Panel::right("details_panel")
                .default_size(self.config.right_panel_width as f32)
                .frame(egui::Frame::new().fill(t.window_bg).inner_margin(0.0))
                .show_inside(ui, |ui| {
                    let action = pin_body_and_status_bar(
                        ui,
                        |ui| {
                            draw_details(
                                ui,
                                &self.manifest,
                                &self.trashcan,
                                self.nav_view,
                                self.selected_id.as_deref(),
                                &self.editor_text,
                                &self.target.path().display().to_string(),
                            )
                        },
                        draw_panel_status_spacer,
                    );
                    self.on_details_action(action);
                });
        }

        let editor_text_before = self.editor_text.clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(t.editor_bg).inner_margin(0.0))
            .show_inside(ui, |ui| {
                draw_editor_with_status_bar(
                    ui,
                    &mut self.editor_text,
                    &self.manifest,
                    self.selected_id.as_deref(),
                    self.editor_revision,
                    &mut self.editor_pending_selection,
                );
            });

        if self.editor_text != editor_text_before {
            let node = self
                .selected_id
                .as_deref()
                .and_then(|id| find_node(&self.manifest.root, id));
            if !is_editor_read_only(self.selected_id.as_deref(), node.as_ref()) {
                self.editor_dirty = true;
            }
        }
        if self.editor_dirty {
            if self.save_editor() {
                self.apply_hosts();
            }
        }

        match draw_edit_hosts_drawer(
            ctx,
            &mut self.edit_hosts,
            &mut self.manifest,
            &self.paths,
            &self.config,
        ) {
            EditHostsResult::Saved { id } => {
                self.selected_id = Some(id);
                self.reload_editor();
                self.persist_manifest();
            }
            EditHostsResult::MovedToTrash { node, parent_id } => {
                let id = node
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.trashcan.push(TrashItem {
                    id: id.clone(),
                    node,
                    parent_id,
                    deleted_at: None,
                });
                let _ = self.trashcan.save(&self.paths.trashcan_file);
                self.selected_id = Some(SYSTEM_NODE_ID.to_string());
                self.reload_editor();
                self.persist_manifest();
            }
            EditHostsResult::Cancelled => {}
            EditHostsResult::None => {}
        }

        match draw_history_drawer(ctx, &mut self.history, &self.paths, &mut self.config) {
            HistoryResult::ConfigChanged => {
                let _ = self.config.save(&self.paths.config_file);
            }
            HistoryResult::Closed | HistoryResult::None => {}
        }

        match draw_import_from_url_modal(ctx, &mut self.import_from_url) {
            ImportFromUrlResult::None | ImportFromUrlResult::Cancelled => {}
            ImportFromUrlResult::Confirmed(url) => {
                self.handle_import_result(
                    ctx,
                    import_from_url_data(&url, &self.paths, &self.config),
                );
            }
        }

        if let Some(message) = self.import_error.clone() {
            if draw_import_error_modal(ctx, &message) {
                self.import_error = None;
            }
        }

        if let Some(message) = self.apply_error.clone() {
            if draw_apply_error_modal(ctx, &message) {
                self.apply_error = None;
            }
        }

        if self.pending_trash_clear {
            match draw_trash_clear_confirm(ctx) {
                TrashDeleteConfirmResult::Confirmed(_) => {
                    self.clear_trashcan();
                    self.pending_trash_clear = false;
                }
                TrashDeleteConfirmResult::Cancelled => {
                    self.pending_trash_clear = false;
                }
                TrashDeleteConfirmResult::None => {}
            }
        }

        if let Some(id) = self.pending_trash_delete.clone() {
            let title = self.trash_delete_title(&id);
            match draw_trash_delete_confirm(ctx, &id, &title) {
                TrashDeleteConfirmResult::Confirmed(deleted_id) => {
                    self.permanently_delete_from_trash(&deleted_id);
                    self.pending_trash_delete = None;
                }
                TrashDeleteConfirmResult::Cancelled => {
                    self.pending_trash_delete = None;
                }
                TrashDeleteConfirmResult::None => {}
            }
        }
    }
}

fn top_bar_frame(config: &AppConfig, t: &theme::AppTheme) -> egui::Frame {
    let fill = if cfg!(target_os = "macos") && !config.use_system_window_frame {
        egui::Color32::TRANSPARENT
    } else {
        t.top_bar_bg
    };
    egui::Frame::new().fill(fill).inner_margin(0.0)
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
