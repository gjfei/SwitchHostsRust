use core::storage::manifest::Manifest;
use core::storage::paths::AppPaths;
use std::fs;
use tempfile::TempDir;

#[test]
fn loads_fixture_manifest_round_trip() {
    let fixture_root = core::storage::paths::fixtures_dir();
    let tmp = TempDir::new().unwrap();
    let paths = AppPaths::new(tmp.path().to_path_buf());
    paths.ensure_layout().unwrap();

    fs::copy(
        fixture_root.join("manifest.json"),
        &paths.manifest_file,
    )
    .unwrap();
    fs::create_dir_all(&paths.entries_dir).unwrap();
    fs::copy(
        fixture_root.join("entries/local-1.hosts"),
        paths.entry_file("local-1"),
    )
    .unwrap();

    let manifest = Manifest::load(&paths).unwrap();
    assert!(!manifest.root.is_empty());
    manifest.save(&paths).unwrap();
    let again = Manifest::load(&paths).unwrap();
    assert_eq!(again.root.len(), manifest.root.len());
}
