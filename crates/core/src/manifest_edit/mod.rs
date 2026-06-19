//! 向 manifest 添加/编辑 hosts 节点。

use serde_json::{json, Value};

use crate::storage::manifest::find_node;

use uuid::Uuid;

pub use crate::storage::tree_format::SYSTEM_NODE_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostsNodeKind {
    #[default]
    Local,
    Remote,
    Group,
    Folder,
}

impl HostsNodeKind {
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
            Self::Group => "group",
            Self::Folder => "folder",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "本地",
            Self::Remote => "远程",
            Self::Group => "组合",
            Self::Folder => "文件夹",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Local => "📄",
            Self::Remote => "🌐",
            Self::Group => "📚",
            Self::Folder => "📁",
        }
    }

    pub fn from_type_str(s: &str) -> Self {
        match s {
            "remote" => Self::Remote,
            "group" => Self::Group,
            "folder" => Self::Folder,
            _ => Self::Local,
        }
    }
}

/// 编辑抽屉中的节点草稿（对齐 SwitchHosts `EditHostsInfo` 表单字段）。
#[derive(Debug, Clone)]
pub struct HostsNodeDraft {
    pub id: Option<String>,
    pub kind: HostsNodeKind,
    pub title: String,
    pub url: String,
    pub refresh_interval: u64,
    pub include: Vec<String>,
    pub folder_mode: u8,
}

impl Default for HostsNodeDraft {
    fn default() -> Self {
        Self {
            id: None,
            kind: HostsNodeKind::Local,
            title: String::new(),
            url: String::new(),
            refresh_interval: 0,
            include: Vec::new(),
            folder_mode: 0,
        }
    }
}

impl HostsNodeDraft {
    pub fn for_add() -> Self {
        Self::default()
    }

    pub fn from_node(node: &Value) -> Self {
        Self {
            id: node.get("id").and_then(|v| v.as_str()).map(str::to_string),
            kind: HostsNodeKind::from_type_str(
                node.get("type").and_then(|v| v.as_str()).unwrap_or("local"),
            ),
            title: node
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            url: node.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            refresh_interval: node
                .get("refresh_interval")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            include: node
                .get("include")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            folder_mode: node
                .get("folder_mode")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u8,
        }
    }

    pub fn apply_to_new_node(&self) -> Value {
        let mut node = build_new_node(self.kind, self.title.trim());
        self.merge_into(&mut node);
        node
    }

    pub fn merge_into(&self, node: &mut Value) {
        if let Some(obj) = node.as_object_mut() {
            obj.insert("title".into(), json!(self.title.trim()));
            obj.insert("type".into(), json!(self.kind.type_name()));
            match self.kind {
                HostsNodeKind::Remote => {
                    let url = self.url.trim();
                    if self.id.is_some() {
                        obj.insert("url".into(), json!(url));
                        obj.insert("refresh_interval".into(), json!(self.refresh_interval));
                    } else {
                        if !url.is_empty() {
                            obj.insert("url".into(), json!(url));
                        }
                        if self.refresh_interval != 0 {
                            obj.insert("refresh_interval".into(), json!(self.refresh_interval));
                        }
                    }
                }
                HostsNodeKind::Group => {
                    obj.insert("include".into(), json!(self.include));
                }
                HostsNodeKind::Folder => {
                    if self.id.is_some() || self.folder_mode != 0 {
                        obj.insert("folder_mode".into(), json!(self.folder_mode));
                    }
                }
                HostsNodeKind::Local => {}
            }
        }
    }
}

/// 远程 hosts 自动刷新间隔选项（秒），对齐原版 Select 数据。
pub const REFRESH_INTERVALS: &[(u64, &str)] = &[
    (0, "不刷新"),
    (60, "1 分钟"),
    (300, "5 分钟"),
    (900, "15 分钟"),
    (3600, "1 小时"),
    (86400, "1 天"),
    (604_800, "1 周"),
];

/// 构建新的 legacy manifest 节点。
pub fn build_new_node(kind: HostsNodeKind, title: &str) -> Value {
    let id = Uuid::new_v4().to_string();
    let mut node = json!({
        "id": id,
        "type": kind.type_name(),
        "title": title,
        "on": false,
    });
    match kind {
        HostsNodeKind::Local => {}
        HostsNodeKind::Remote => {}
        HostsNodeKind::Group => {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("include".into(), json!([]));
            }
        }
        HostsNodeKind::Folder => {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("children".into(), json!([]));
            }
        }
    }
    node
}

/// 追加节点到 root，返回新节点 id。
pub fn add_node_to_root(root: &mut Vec<Value>, kind: HostsNodeKind, title: &str) -> String {
    let node = build_new_node(kind, title);
    let id = node
        .get("id")
        .and_then(|v| v.as_str())
        .expect("node has id")
        .to_string();
    root.push(node);
    id
}

/// 追加草稿节点；`parent_id` 为文件夹 id 时插入其 `children`，否则追加到 root。
pub fn add_draft(root: &mut Vec<Value>, draft: &HostsNodeDraft, parent_id: Option<&str>) -> String {
    let node = draft.apply_to_new_node();
    let id = node["id"].as_str().unwrap().to_string();
    insert_node(root, node, parent_id);
    id
}

/// 追加草稿节点到 root。
pub fn add_draft_to_root(root: &mut Vec<Value>, draft: &HostsNodeDraft) -> String {
    add_draft(root, draft, None)
}

/// 根据当前选中项推断新建节点的父文件夹 id。
///
/// - 选中文件夹 → 在该文件夹下创建
/// - 选中文件夹内的节点 → 在同一文件夹下创建
/// - 选中 root 节点 / 系统 Hosts / 无选中 → 在 root 创建
pub fn add_parent_for_selection(root: &[Value], selected_id: Option<&str>) -> Option<String> {
    let Some(id) = selected_id else {
        return None;
    };
    if id == SYSTEM_NODE_ID {
        return None;
    }
    let Some(node) = find_node(root, id) else {
        return None;
    };
    if node.get("type").and_then(Value::as_str) == Some("folder") {
        return Some(id.to_string());
    }
    parent_folder_id(root, id)
}

/// 展开目标文件夹，便于看到刚创建的子节点。
pub fn ensure_folder_expanded(root: &mut [Value], folder_id: &str) -> bool {
    for node in root.iter_mut() {
        if node.get("id").and_then(Value::as_str) == Some(folder_id) {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("is_collapsed".into(), json!(false));
            }
            return true;
        }
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if ensure_folder_expanded(children, folder_id) {
                return true;
            }
        }
    }
    false
}

/// 插入节点到 root 或指定文件夹。
pub fn insert_node(nodes: &mut Vec<Value>, node: Value, parent_id: Option<&str>) {
    if let Some(pid) = parent_id {
        if append_into_folder(nodes, &node, pid) {
            return;
        }
    }
    nodes.push(node);
}

fn append_into_folder(nodes: &mut Vec<Value>, node: &Value, parent_id: &str) -> bool {
    for current in nodes.iter_mut() {
        if current.get("id").and_then(Value::as_str) == Some(parent_id) {
            if let Some(children) = current
                .as_object_mut()
                .and_then(|o| o.get_mut("children"))
                .and_then(|c| c.as_array_mut())
            {
                children.push(node.clone());
                return true;
            }
            return false;
        }
        if let Some(children) = current
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if append_into_folder(children, node, parent_id) {
                return true;
            }
        }
    }
    false
}

fn parent_folder_id(nodes: &[Value], target_id: &str) -> Option<String> {
    for node in nodes {
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            if children
                .iter()
                .any(|c| c.get("id").and_then(Value::as_str) == Some(target_id))
            {
                if node.get("type").and_then(Value::as_str) == Some("folder") {
                    return node
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                return None;
            }
            if let Some(found) = parent_folder_id(children, target_id) {
                return Some(found);
            }
        }
    }
    None
}

/// 更新已有节点字段。
pub fn update_node_in_root(root: &mut [Value], draft: &HostsNodeDraft) -> bool {
    let Some(id) = draft.id.as_deref() else {
        return false;
    };
    update_node_recursive(root, id, draft)
}

fn update_node_recursive(nodes: &mut [Value], id: &str, draft: &HostsNodeDraft) -> bool {
    for node in nodes.iter_mut() {
        if node.get("id").and_then(|v| v.as_str()) == Some(id) {
            draft.merge_into(node);
            return true;
        }
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if update_node_recursive(children, id, draft) {
                return true;
            }
        }
    }
    false
}

/// 从树中移除节点并返回被移除的节点。
pub fn remove_node_by_id(root: &mut Vec<Value>, id: &str) -> Option<Value> {
    if let Some(pos) = root.iter().position(|n| n.get("id").and_then(|v| v.as_str()) == Some(id)) {
        return Some(root.remove(pos));
    }
    for node in root.iter_mut() {
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            if let Some(removed) = remove_node_by_id(children, id) {
                return Some(removed);
            }
        }
    }
    None
}

/// 组合类型可选的 local/remote 节点列表 `(id, title, kind)`。
pub fn list_includable_nodes(root: &[Value]) -> Vec<(String, String, HostsNodeKind)> {
    let mut flat = Vec::new();
    flatten_for_picker(root, &mut flat);
    flat
}

fn flatten_for_picker(nodes: &[Value], out: &mut Vec<(String, String, HostsNodeKind)>) {
    for node in nodes {
        if node.get("isSys").and_then(|v| v.as_bool()).unwrap_or(false)
            || node.get("is_sys").and_then(|v| v.as_bool()).unwrap_or(false)
        {
            continue;
        }
        let kind = HostsNodeKind::from_type_str(
            node.get("type").and_then(|v| v.as_str()).unwrap_or("local"),
        );
        if matches!(kind, HostsNodeKind::Local | HostsNodeKind::Remote) {
            if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
                let title = node
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id)
                    .to_string();
                out.push((id.to_string(), title, kind));
            }
        }
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            flatten_for_picker(children, out);
        }
    }
}

/// 判断编辑器是否只读（对齐 `useHostsData.isReadOnly`）。
pub fn is_editor_read_only(selected_id: Option<&str>, node: Option<&Value>) -> bool {
    let Some(id) = selected_id else {
        return true;
    };
    if id == SYSTEM_NODE_ID {
        return true;
    }
    let Some(node) = node else {
        return true;
    };
    match node.get("type").and_then(|v| v.as_str()) {
        Some("group") | Some("remote") | Some("folder") => true,
        _ => node.get("isSys").and_then(|v| v.as_bool()).unwrap_or(false)
            || node.get("is_sys").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_local_node() {
        let node = build_new_node(HostsNodeKind::Local, "basic");
        assert_eq!(node["type"], "local");
        assert_eq!(node["title"], "basic");
    }

    #[test]
    fn draft_round_trip_remote() {
        let mut draft = HostsNodeDraft::for_add();
        draft.kind = HostsNodeKind::Remote;
        draft.title = "shared".into();
        draft.url = "https://example.com/hosts".into();
        draft.refresh_interval = 3600;
        let node = draft.apply_to_new_node();
        assert_eq!(node["url"], "https://example.com/hosts");
        assert_eq!(node["refresh_interval"], 3600);
    }

    #[test]
    fn remove_and_update() {
        let mut root = vec![json!({"id":"a","type":"local","title":"A"})];
        let mut draft = HostsNodeDraft::from_node(&root[0]);
        draft.title = "B".into();
        assert!(update_node_in_root(&mut root, &draft));
        assert_eq!(root[0]["title"], "B");
        assert!(remove_node_by_id(&mut root, "a").is_some());
        assert!(root.is_empty());
    }

    #[test]
    fn read_only_types() {
        assert!(is_editor_read_only(Some(SYSTEM_NODE_ID), None));
        let folder = json!({"id":"f","type":"folder"});
        assert!(is_editor_read_only(Some("f"), Some(&folder)));
        let local = json!({"id":"l","type":"local"});
        assert!(!is_editor_read_only(Some("l"), Some(&local)));
    }

    #[test]
    fn add_draft_under_folder() {
        let mut root = vec![json!({
            "id": "f",
            "type": "folder",
            "title": "F",
            "children": [],
            "is_collapsed": true
        })];
        let mut draft = HostsNodeDraft::for_add();
        draft.title = "child".into();
        let id = add_draft(&mut root, &draft, Some("f"));
        let children = root[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["id"], id);
        assert_eq!(children[0]["title"], "child");
    }

    #[test]
    fn add_parent_for_selection_resolves_folder() {
        let root = vec![json!({
            "id": "f",
            "type": "folder",
            "children": [{"id": "c", "type": "local", "title": "C"}]
        })];
        assert_eq!(
            add_parent_for_selection(&root, Some("f")),
            Some("f".to_string())
        );
        assert_eq!(
            add_parent_for_selection(&root, Some("c")),
            Some("f".to_string())
        );
        assert_eq!(add_parent_for_selection(&root, Some(SYSTEM_NODE_ID)), None);
    }

    #[test]
    fn add_parent_for_selection_root_level() {
        let root = vec![json!({"id": "a", "type": "local", "title": "A"})];
        assert_eq!(add_parent_for_selection(&root, Some("a")), None);
    }

    #[test]
    fn ensure_folder_expanded_uncollapses() {
        let mut root = vec![json!({
            "id": "f",
            "type": "folder",
            "is_collapsed": true,
            "children": []
        })];
        assert!(ensure_folder_expanded(&mut root, "f"));
        assert_eq!(root[0]["is_collapsed"], false);
    }
}
