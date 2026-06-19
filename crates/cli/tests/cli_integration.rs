use assert_cmd::Command;
use switch_hosts_core::storage::entries;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::storage::paths::AppPaths;
use predicates::prelude::*;
use serde_json::json;
use tempfile::TempDir;

fn cli_cmd(data_dir: &std::path::Path, hosts_file: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("switch-hosts-rust").unwrap();
    cmd.arg("--data-dir").arg(data_dir);
    cmd.arg("--hosts-file").arg(hosts_file);
    cmd
}

#[test]
fn list_toggle_apply_flow() {
    let tmp = TempDir::new().unwrap();
    let paths = AppPaths::new(tmp.path().to_path_buf());
    paths.ensure_layout().unwrap();
    entries::write_entry(&paths.entries_dir, "1", "127.0.0.1 cli.test\n").unwrap();
    let manifest = Manifest {
        root: json!([{ "id": "1", "type": "local", "title": "T", "on": false }])
            .as_array()
            .cloned()
            .unwrap(),
        ..Default::default()
    };
    manifest.save(&paths).unwrap();

    let hosts_out = tmp.path().join("out.hosts");

    cli_cmd(tmp.path(), &hosts_out)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": \"1\""));

    cli_cmd(tmp.path(), &hosts_out)
        .arg("toggle")
        .arg("1")
        .assert()
        .success();

    assert!(hosts_out.exists());
    let content = std::fs::read_to_string(&hosts_out).unwrap();
    assert!(content.contains("cli.test"));
}
