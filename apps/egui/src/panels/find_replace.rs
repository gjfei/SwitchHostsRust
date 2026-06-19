use switch_hosts_core::find::{find_in_manifest, replace_in_manifest, FindMatch, FindOptions};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui;

pub struct FindReplaceState {
    pub query: String,
    pub replace_with: String,
    pub matches: Vec<FindMatch>,
    pub last_count: usize,
}

impl Default for FindReplaceState {
    fn default() -> Self {
        Self {
            query: String::new(),
            replace_with: String::new(),
            matches: Vec::new(),
            last_count: 0,
        }
    }
}

/// 查找/替换对话框。
pub fn draw_find_replace(
    ctx: &egui::Context,
    open: &mut bool,
    state: &mut FindReplaceState,
    config: &mut AppConfig,
    manifest: &Manifest,
    paths: &AppPaths,
) -> bool {
    let mut replaced = false;
    if !*open {
        return false;
    }
    let mut window_open = true;
    egui::Window::new("查找 / 替换")
        .default_width(480.0)
        .open(&mut window_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("查找");
                ui.text_edit_singleline(&mut state.query);
            });
            ui.horizontal(|ui| {
                ui.label("替换为");
                ui.text_edit_singleline(&mut state.replace_with);
            });
            let mut is_regexp = config.find_is_regexp;
            let mut ignore_case = config.find_is_ignore_case;
            if ui.checkbox(&mut is_regexp, "正则表达式").changed() {
                config.find_is_regexp = is_regexp;
            }
            if ui.checkbox(&mut ignore_case, "忽略大小写").changed() {
                config.find_is_ignore_case = ignore_case;
            }

            ui.horizontal(|ui| {
                if ui.button("查找").clicked() {
                    let opts = FindOptions {
                        query: state.query.clone(),
                        replace_with: None,
                        is_regexp,
                        ignore_case,
                        do_replace: false,
                    };
                    state.matches = find_in_manifest(manifest, paths, &opts).unwrap_or_default();
                    state.last_count = state.matches.len();
                }
                if ui.button("全部替换").clicked() {
                    let opts = FindOptions {
                        query: state.query.clone(),
                        replace_with: Some(state.replace_with.clone()),
                        is_regexp,
                        ignore_case,
                        do_replace: true,
                    };
                    if let Ok((_, n)) = replace_in_manifest(manifest, paths, &opts) {
                        state.last_count = n;
                        replaced = true;
                    }
                }
            });

            ui.label(format!("匹配数: {}", state.last_count));
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for m in &state.matches {
                        ui.label(format!(
                            "{}:{} — {}",
                            m.entry_id, m.line, m.text
                        ));
                    }
                });
        });
    if !window_open {
        *open = false;
    }
    replaced
}
