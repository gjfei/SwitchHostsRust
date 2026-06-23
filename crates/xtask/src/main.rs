//! Workspace 开发/打包任务（`cargo package-macos`、`cargo dev-gui` 等别名入口）。

mod dev_gui;
mod dmg;
mod package;
mod packager_config;
mod release;
mod sync_fonts;
mod sync_icons;
mod util;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "SwitchHostsRust workspace tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// macOS Release：构建 .dmg → dist/（.app 为中间产物，不写入 dist）
    PackageMacos {
        /// 仅打包指定 app（可多次指定；默认全部 enabled app）
        #[arg(long = "app", short = 'a')]
        apps: Vec<String>,
        /// 仅构建 .app，不生成 .dmg
        #[arg(long)]
        app_only: bool,
    },
    /// 同 package-macos（保留别名）
    PackageDmg {
        #[arg(long = "app", short = 'a')]
        apps: Vec<String>,
    },
    /// Windows Release：构建 NSIS 安装包 → dist/
    PackageWindows {
        #[arg(long = "app", short = 'a')]
        apps: Vec<String>,
    },
    /// macOS Debug：构建 egui-app .app 并用 open 启动（Mission Control / Dock 图标）
    RunGuiMacos,
    /// 单次运行 GUI（等同 cargo run -p egui-app）
    DevGui {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// 监听源码变更并自动 cargo run -p egui-app
    DevGuiWatch {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// 同步阿里巴巴普惠体到 crates/ui-assets/assets/fonts/
    SyncFonts,
    /// 从本地 SwitchHosts 同步应用图标
    SyncIcons,
    /// 解析 git tag、更新 workspace / Packager 版本（CI release 用）
    ReleasePrepare {
        /// 例如 v0.1.0 或 egui-app-v0.1.0
        #[arg(long)]
        tag: String,
        /// 只解析 tag，不写版本文件
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::PackageMacos { apps, app_only } => package::package_macos(package::PackageOptions {
            release: true,
            app_only,
            apps,
            out_dir: None,
        }),
        Command::PackageDmg { apps } => package::package_macos(package::PackageOptions {
            release: true,
            app_only: false,
            apps,
            out_dir: None,
        }),
        Command::PackageWindows { apps } => package::package_windows(package::PackageOptions {
            release: true,
            app_only: false,
            apps,
            out_dir: None,
        }),
        Command::RunGuiMacos => package::open_app_bundle(false),
        Command::DevGui { args } => {
            let status = dev_gui::dev_gui(&args)?;
            if status.success() {
                Ok(())
            } else {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::DevGuiWatch { args } => dev_gui::dev_gui_watch(&args),
        Command::SyncFonts => sync_fonts::sync_fonts(),
        Command::SyncIcons => sync_icons::sync_icons(),
        Command::ReleasePrepare { tag, dry_run } => release::release_prepare(&tag, dry_run),
    }
}
