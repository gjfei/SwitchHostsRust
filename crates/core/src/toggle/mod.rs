use serde_json::{json, Value};

/// 按 choice_mode 与文件夹语义传播节点 toggle 状态。
pub fn toggle_item(root: &mut [Value], id: &str, choice_mode: u8) -> bool {
    if choice_mode == 1 {
        // 单选：先关闭同级节点，再切换目标
        if let Some(on) = find_and_toggle_single(root, id) {
            return on;
        }
        return false;
    }
    toggle_recursive(root, id)
}

fn toggle_recursive(nodes: &mut [Value], id: &str) -> bool {
    for node in nodes.iter_mut() {
        if node.get("id").and_then(Value::as_str) == Some(id) {
            let on = !node.get("on").and_then(Value::as_bool).unwrap_or(false);
            if let Some(obj) = node.as_object_mut() {
                obj.insert("on".into(), json!(on));
            }
            if node.get("type").and_then(Value::as_str) == Some("folder") {
                set_children_on(node, on);
            }
            return true;
        }
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if toggle_recursive(children, id) {
                return true;
            }
        }
    }
    false
}

fn find_and_toggle_single(nodes: &mut [Value], id: &str) -> Option<bool> {
    if let Some(idx) = nodes
        .iter()
        .position(|node| node.get("id").and_then(Value::as_str) == Some(id))
    {
        let new_on = !nodes[idx]
            .get("on")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        for (i, node) in nodes.iter_mut().enumerate() {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("on".into(), json!(i == idx && new_on));
            }
        }
        return Some(new_on);
    }
    for node in nodes.iter_mut() {
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if let Some(v) = find_and_toggle_single(children, id) {
                return Some(v);
            }
        }
    }
    None
}

fn set_children_on(node: &mut Value, on: bool) {
    if let Some(children) = node
        .as_object_mut()
        .and_then(|o| o.get_mut("children"))
        .and_then(|c| c.as_array_mut())
    {
        for child in children.iter_mut() {
            if let Some(obj) = child.as_object_mut() {
                obj.insert("on".into(), json!(on));
            }
            if child.get("type").and_then(Value::as_str) == Some("folder") {
                set_children_on(child, on);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn multi_choice_toggles_one_node() {
        let mut root = json!([
            { "id": "a", "type": "local", "on": true },
            { "id": "b", "type": "local", "on": false }
        ])
        .as_array()
        .cloned()
        .unwrap();
        assert!(toggle_item(&mut root, "b", 2));
        assert_eq!(root[1]["on"], true);
    }

    #[test]
    fn single_choice_turns_off_others() {
        let mut root = json!([
            { "id": "a", "type": "local", "on": true },
            { "id": "b", "type": "local", "on": false }
        ])
        .as_array()
        .cloned()
        .unwrap();
        toggle_item(&mut root, "b", 1);
        assert_eq!(root[0]["on"], false);
        assert_eq!(root[1]["on"], true);
    }

    #[test]
    fn folder_toggle_propagates_to_children() {
        let mut root = json!([{
            "id": "f",
            "type": "folder",
            "on": false,
            "children": [{ "id": "c", "type": "local", "on": false }]
        }])
        .as_array()
        .cloned()
        .unwrap();
        toggle_item(&mut root, "f", 2);
        assert_eq!(root[0]["children"][0]["on"], true);
    }
}
