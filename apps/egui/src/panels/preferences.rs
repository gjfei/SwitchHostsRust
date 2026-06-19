use switch_hosts_core::storage::config::AppConfig;
use eframe::egui;

/// 偏好设置窗口（写入模式、选择模式、HTTP API 等）。
pub fn draw_preferences(
    ctx: &egui::Context,
    open: &mut bool,
    config: &mut AppConfig,
) -> bool {
    let mut saved = false;
    if !*open {
        return false;
    }
    let mut window_open = true;
    egui::Window::new("偏好设置")
        .default_width(360.0)
        .open(&mut window_open)
        .show(ctx, |ui| {
            ui.heading("写入");
            egui::ComboBox::from_label("write_mode")
                .selected_text(&config.write_mode)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut config.write_mode, "append".into(), "append（追加）");
                    ui.selectable_value(&mut config.write_mode, "overwrite".into(), "overwrite（覆盖）");
                });

            ui.heading("切换");
            ui.horizontal(|ui| {
                ui.label("choice_mode");
                ui.radio_value(&mut config.choice_mode, 1, "单选 (1)");
                ui.radio_value(&mut config.choice_mode, 2, "多选 (2)");
            });
            ui.checkbox(
                &mut config.remove_duplicate_records,
                "聚合时去除重复记录",
            );

            ui.heading("界面");
            egui::ComboBox::from_label("theme")
                .selected_text(&config.theme)
                .show_ui(ui, |ui| {
                    for t in ["system", "light", "dark"] {
                        ui.selectable_value(&mut config.theme, t.to_string(), t);
                    }
                });
            ui.checkbox(&mut config.right_panel_show, "显示右侧详情面板");

            ui.heading("生命周期");
            ui.checkbox(&mut config.launch_at_login, "登录时启动");
            ui.checkbox(&mut config.hide_at_launch, "启动时隐藏主窗口");
            ui.checkbox(
                &mut config.tray_mini_window,
                "关闭窗口时最小化到托盘",
            );

            ui.heading("HTTP API");
            ui.checkbox(&mut config.http_api_on, "启用本地 HTTP API (:50761)");
            ui.checkbox(
                &mut config.http_api_only_local,
                "仅绑定 127.0.0.1",
            );

            ui.heading("代理");
            ui.checkbox(&mut config.use_proxy, "使用代理");
            ui.horizontal(|ui| {
                ui.label("协议");
                ui.text_edit_singleline(&mut config.proxy_protocol);
            });
            ui.horizontal(|ui| {
                ui.label("主机");
                ui.text_edit_singleline(&mut config.proxy_host);
            });
            ui.add(
                egui::DragValue::new(&mut config.proxy_port)
                    .prefix("端口 ")
                    .range(0..=65535),
            );

            ui.separator();
            if ui.button("保存并关闭").clicked() {
                saved = true;
            }
        });
    if !window_open && !saved {
        *open = false;
    }
    if saved {
        *open = false;
    }
    saved
}
