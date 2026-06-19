use anyhow::{Context, Result};
use switch_hosts_core::hosts_apply::elevation::SystemElevation;
use switch_hosts_core::hosts_apply::pipeline::ApplyPipeline;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::import_export::{export_v5_backup, import_from_directory, import_v5_backup};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::{flatten_nodes, Manifest};
use switch_hosts_core::storage::paths::AppPaths;
use switch_hosts_core::toggle::toggle_item;
use service::start_api;
use std::fs;
use std::io::Write;

pub fn list(paths: &AppPaths) -> Result<()> {
    let manifest = Manifest::load(paths)?;
    let mut flat = Vec::new();
    flatten_nodes(&manifest.root, &mut flat);
    println!("{}", serde_json::to_string_pretty(&flat)?);
    Ok(())
}

pub fn toggle(paths: &AppPaths, target: &HostsTarget, id: &str) -> Result<()> {
    let mut manifest = Manifest::load(paths)?;
    let config = AppConfig::load(&paths.config_file);
    if !toggle_item(&mut manifest.root, id, config.choice_mode) {
        anyhow::bail!("node not found: {id}");
    }
    manifest.save(paths)?;
    apply(paths, target)?;
    Ok(())
}

pub fn apply(paths: &AppPaths, target: &HostsTarget) -> Result<()> {
    let manifest = Manifest::load(paths)?;
    let config = AppConfig::load(&paths.config_file);
    let elevation = SystemElevation;
    let pipeline = ApplyPipeline {
        paths,
        config: &config,
        elevation: &elevation,
    };
    let result = pipeline.apply(&manifest, target)?;
    eprintln!("Wrote hosts to: {}", result.target_path.display());
    if !result.written {
        eprintln!("Skipped write (content unchanged)");
    }
    Ok(())
}

pub fn export_backup(paths: &AppPaths) -> Result<()> {
    let manifest = Manifest::load(paths)?;
    let backup = export_v5_backup(&manifest, paths)?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", serde_json::to_string_pretty(&backup)?)?;
    Ok(())
}

pub fn import_backup(paths: &AppPaths, file: &std::path::Path) -> Result<()> {
    let bytes = fs::read(file).with_context(|| format!("read {}", file.display()))?;
    let backup: serde_json::Value = serde_json::from_slice(&bytes)?;
    let manifest = import_v5_backup(paths, &backup)?;
    manifest.save(paths)?;
    Ok(())
}

pub fn import_dir(paths: &AppPaths, source: &std::path::Path) -> Result<()> {
    import_from_directory(paths, source)?;
    Ok(())
}

pub async fn serve(paths: &AppPaths, target: HostsTarget) -> Result<()> {
    let config = AppConfig::load(&paths.config_file);
    let only_local = config.http_api_only_local;
    let handle = start_api(paths.clone(), config, target, only_local).await?;
    eprintln!("HTTP API listening on port {}", service::HTTP_API_PORT);
    handle.join.await?;
    Ok(())
}
