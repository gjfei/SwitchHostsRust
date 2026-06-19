use anyhow::Result;
use switch_hosts_core::hosts_apply::elevation::SystemElevation;
use switch_hosts_core::hosts_apply::pipeline::ApplyPipeline;
use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::{flatten_nodes, Manifest};
use switch_hosts_core::storage::paths::AppPaths;
use switch_hosts_core::toggle::toggle_item;

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
