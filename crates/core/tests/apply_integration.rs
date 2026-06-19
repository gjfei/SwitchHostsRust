use core::hosts_apply::pipeline::ApplyPipeline;
use core::hosts_apply::target::HostsTarget;
use core::hosts_apply::elevation::MockElevation;
use core::storage::entries;
use core::storage::manifest::Manifest;
use core::storage::paths::AppPaths;
use serde_json::json;
use tempfile::TempDir;

#[test]
fn apply_writes_dev_hosts_file() {
    let tmp = TempDir::new().unwrap();
    let paths = AppPaths::new(tmp.path().to_path_buf());
    paths.ensure_layout().unwrap();
    entries::write_entry(&paths.entries_dir, "local-1", "127.0.0.1 apply.test\n").unwrap();

    let manifest = Manifest {
        root: json!([{ "id": "local-1", "type": "local", "on": true }])
            .as_array()
            .cloned()
            .unwrap(),
        ..Default::default()
    };
    let config = core::storage::config::AppConfig::default();
    let target = HostsTarget::File(tmp.path().join("out.hosts"));
    let pipeline = ApplyPipeline {
        paths: &paths,
        config: &config,
        elevation: &MockElevation,
    };
    let result = pipeline.apply(&manifest, &target).unwrap();
    assert!(result.written);
}
