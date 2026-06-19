use anyhow::Result;
use egui_app::app::SwitchHostsApp;
use egui_app::lifecycle;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let paths = AppPaths::default_user()?;
    paths.ensure_layout()?;

    let config = AppConfig::load(&paths.config_file);
    if let Ok(exe) = std::env::current_exe() {
        let _ = lifecycle::sync_launch_at_login(&config, &exe);
    }

    let target = if cfg!(debug_assertions) {
        HostsTarget::dev_default(&paths)
    } else {
        HostsTarget::system_default()
    };

    let visible = lifecycle::initial_window_visible(&config);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_visible(visible),
        ..Default::default()
    };

    eframe::run_native(
        "SwitchHostsRust",
        native_options,
        Box::new(|cc| Ok(Box::new(SwitchHostsApp::new(cc, paths, target)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
