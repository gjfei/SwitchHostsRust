//! Workspace 开发/打包任务（`cargo dev egui`、`cargo package-macos` 等别名入口）。

mod apps;
mod dev_app;
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
    /// macOS Debug：构建 .app 并用 open 启动（需 Packager.toml 已配置）
    RunAppMacos {
        /// app/ 下的目录名，或完整 crate 名（如 egui-app）
        app: String,
    },
    /// 单次运行 app（等同 cargo run -p <package>）
    Dev {
        /// app/ 下的目录名，或完整 crate 名（如 egui-app）
        app: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// 监听源码变更并自动 cargo run
    DevWatch {
        /// app/ 下的目录名，或完整 crate 名（如 egui-app）
        app: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// 列出 app/ 下可运行的 app
    ListApps,
    /// 输出 app 对应的 Cargo package 名（供脚本使用）
    ResolvePackage {
        /// app/ 下的目录名或完整 crate 名
        app: String,
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
        Command::RunAppMacos { app } => {
            let package = apps::resolve(&app)?.package;
            package::open_app_bundle(false, &package)
        }
        Command::Dev { app, args } => {
            let status = dev_app::dev(&app, &args)?;
            if status.success() {
                Ok(())
            } else {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::DevWatch { app, args } => dev_app::dev_watch(&app, &args),
        Command::ListApps => {
            for entry in apps::discover_apps()? {
                println!("{} → {}", entry.id, entry.package);
            }
            Ok(())
        }
        Command::ResolvePackage { app } => {
            println!("{}", apps::resolve(&app)?.package);
            Ok(())
        }
        Command::SyncFonts => sync_fonts::sync_fonts(),
        Command::SyncIcons => sync_icons::sync_icons(),
        Command::ReleasePrepare { tag, dry_run } => release::release_prepare(&tag, dry_run),
    }
}
