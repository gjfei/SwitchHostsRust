mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use switch_hosts_core::storage::paths::AppPaths;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "switch-hosts-rust", about = "SwitchHostsRust 命令行工具")]
struct Cli {
    /// 写入系统 hosts 文件（/etc/hosts）
    #[arg(long, global = true)]
    system: bool,

    /// 自定义 hosts 文件路径
    #[arg(long, global = true)]
    hosts_file: Option<std::path::PathBuf>,

    /// 覆盖数据目录根路径
    #[arg(long, global = true, env = "SWITCH_HOSTS_RUST_DATA_DIR")]
    data_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 列出所有 hosts 方案（扁平 JSON）
    List,
    /// 按 id 切换方案
    Toggle { id: String },
    /// 将选中方案写入 hosts 文件
    Apply,
    /// 导出 v5 JSON 备份到 stdout
    Export,
    /// 从文件导入 v5 JSON 备份
    Import { file: std::path::PathBuf },
    /// 从 live v5 数据目录导入
    ImportDir { source: std::path::PathBuf },
    /// 启动本地 HTTP API 服务
    Serve,
}

fn resolve_paths(data_dir: Option<std::path::PathBuf>) -> Result<AppPaths> {
    let paths = match data_dir {
        Some(p) => AppPaths::new(p),
        None => AppPaths::default_user()?,
    };
    paths.ensure_layout()?;
    Ok(paths)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let paths = resolve_paths(cli.data_dir)?;
    let target = switch_hosts_core::hosts_apply::HostsTarget::resolve(
        &paths,
        cli.system,
        cli.hosts_file,
    );

    match cli.command {
        Commands::List => commands::list(&paths)?,
        Commands::Toggle { id } => commands::toggle(&paths, &target, &id)?,
        Commands::Apply => commands::apply(&paths, &target)?,
        Commands::Export => commands::export_backup(&paths)?,
        Commands::Import { file } => commands::import_backup(&paths, &file)?,
        Commands::ImportDir { source } => commands::import_dir(&paths, &source)?,
        Commands::Serve => commands::serve(&paths, target).await?,
    }
    Ok(())
}
