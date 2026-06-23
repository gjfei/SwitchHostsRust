use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use egui_app::app::SwitchHostsApp;
use egui_app::lifecycle;
use egui_app::viewport;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::{resolve_hosts_file_from_env, AppPaths};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "switch-hosts-rust-gui", about = "SwitchHostsRust 桌面 GUI")]
struct Args {
    /// 写入系统 hosts 文件（/etc/hosts）
    #[arg(long)]
    system: bool,

    /// 自定义 hosts 文件路径（Debug 默认写入 dev test.hosts）
    #[arg(long)]
    hosts_file: Option<PathBuf>,
}

fn resolve_target(paths: &AppPaths, args: &Args) -> HostsTarget {
    let explicit = args.system
        || args.hosts_file.is_some()
        || resolve_hosts_file_from_env().is_some();

    if explicit {
        return HostsTarget::resolve(paths, args.system, args.hosts_file.clone());
    }

    if cfg!(debug_assertions) {
        HostsTarget::dev_default(paths)
    } else {
        HostsTarget::system_default()
    }
}

fn init_tracing() {
    if cfg!(not(debug_assertions)) && std::env::var_os("RUST_LOG").is_none() {
        return;
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();

    let paths = AppPaths::default_user()?;
    paths.ensure_layout()?;

    let config = AppConfig::load(&paths.config_file);
    if let Ok(exe) = std::env::current_exe() {
        let _ = lifecycle::sync_launch_at_login(&config, &exe);
    }

    let target = resolve_target(&paths, &args);

    let visible = lifecycle::initial_window_visible(&config);
    let native_options = eframe::NativeOptions {
        viewport: viewport::root_viewport_builder(&config).with_visible(visible),
        ..Default::default()
    };

    eframe::run_native(
        "SwitchHostsRust",
        native_options,
        Box::new(|cc| Ok(Box::new(SwitchHostsApp::new(cc, paths, target)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
