use core::storage::config::AppConfig;
use core::toggle::toggle_item;
use serde_json::json;

#[test]
fn toggle_then_aggregate_respects_on_flags() {
    let mut root = json!([
        { "id": "a", "type": "local", "on": false },
        { "id": "b", "type": "local", "on": false }
    ])
    .as_array()
    .cloned()
    .unwrap();
    let config = AppConfig::default();
    toggle_item(&mut root, "a", config.choice_mode);
    assert_eq!(root[0]["on"], true);
    assert_eq!(root[1]["on"], false);
}
