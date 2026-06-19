use anyhow::Result;
use egui_app::app::SwitchHostsApp;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::paths::AppPaths;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let paths = AppPaths::default_user()?;
    paths.ensure_layout()?;

    let target = if cfg!(debug_assertions) {
        HostsTarget::dev_default(&paths)
    } else {
        HostsTarget::system_default()
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SwitchHostsRust",
        native_options,
        Box::new(|cc| Ok(Box::new(SwitchHostsApp::new(cc, paths, target)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
