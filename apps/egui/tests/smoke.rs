#[test]
fn core_hosts_edit_compiles_in_gui_crate() {
    let segs = switch_hosts_core::hosts_edit::parse_line_segments("127.0.0.1 localhost");
    assert!(!segs.is_empty());
}

#[test]
fn tray_menu_label_helper() {
    use switch_hosts_core::storage::manifest::Manifest;
    use serde_json::json;
    let m = Manifest {
        root: json!([{ "id": "x", "title": "X", "on": true }])
            .as_array()
            .cloned()
            .unwrap(),
        ..Default::default()
    };
    let labels = egui_app::app::tray_menu_labels(&m);
    assert_eq!(labels.len(), 1);
}
