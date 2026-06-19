use serde_json::Value;

use crate::storage::entries;
use crate::storage::error::StorageError;
use crate::storage::paths::AppPaths;

pub fn aggregate_selected_content(
    list: &[Value],
    paths: &AppPaths,
    remove_duplicate_records: bool,
) -> Result<String, StorageError> {
    let mut chunks: Vec<String> = Vec::new();
    collect_selected(list, paths, &mut chunks)?;
    let mut content = chunks.join("\n\n");
    if remove_duplicate_records {
        content = remove_duplicate_records_pass(&content);
    }
    Ok(content)
}

fn collect_selected(
    nodes: &[Value],
    paths: &AppPaths,
    out: &mut Vec<String>,
) -> Result<(), StorageError> {
    for node in nodes {
        if is_on(node) {
            collect_on_node_content(node, paths, out)?;
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            collect_selected(children, paths, out)?;
        }
    }
    Ok(())
}

/// 已开启节点的 hosts 内容：local/remote 读自身 entry；group 展开 `include` 列表。
fn collect_on_node_content(
    node: &Value,
    paths: &AppPaths,
    out: &mut Vec<String>,
) -> Result<(), StorageError> {
    match node.get("type").and_then(Value::as_str) {
        Some("group") => {
            if let Some(include) = node.get("include").and_then(Value::as_array) {
                for id_val in include {
                    if let Some(id) = id_val.as_str() {
                        push_entry_if_non_empty(paths, id, out)?;
                    }
                }
            }
        }
        Some("folder") => {}
        _ => {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                push_entry_if_non_empty(paths, id, out)?;
            }
        }
    }
    Ok(())
}

fn push_entry_if_non_empty(
    paths: &AppPaths,
    id: &str,
    out: &mut Vec<String>,
) -> Result<(), StorageError> {
    let content = entries::read_entry(&paths.entries_dir, id)?;
    if !content.is_empty() {
        out.push(content);
    }
    Ok(())
}

fn is_on(node: &Value) -> bool {
    node.get("on").and_then(Value::as_bool).unwrap_or(false)
}

struct ParsedLine {
    ip: String,
    domains: Vec<String>,
    comment: String,
}

fn parse_line(line: &str) -> ParsedLine {
    let (cnt, comment) = match line.split_once('#') {
        Some((before, after)) => (before, after.trim().to_string()),
        None => (line, String::new()),
    };
    let normalized: String = cnt.trim().split_whitespace().collect::<Vec<_>>().join(" ");
    let mut parts = normalized.split(' ').filter(|s| !s.is_empty());
    let ip = parts.next().unwrap_or("").to_string();
    let domains: Vec<String> = parts.map(|s| s.to_string()).collect();
    ParsedLine {
        ip,
        domains,
        comment,
    }
}

fn format_line(ip: &str, domains: &[String], comment: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(ip.to_string());
    parts.extend(domains.iter().cloned());
    if !comment.is_empty() {
        parts.push(format!("# {comment}"));
    }
    parts.join(" ").trim().to_string()
}

pub fn remove_duplicate_records_pass(content: &str) -> String {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut new_lines: Vec<String> = Vec::new();

    for line in content.split('\n') {
        let parsed = parse_line(line);
        if parsed.ip.is_empty() || parsed.domains.is_empty() {
            new_lines.push(line.to_string());
            continue;
        }
        let ipv = if parsed.ip.contains(':') { 6 } else { 4 };

        let mut new_domains: Vec<String> = Vec::new();
        let mut duplicate_domains: Vec<String> = Vec::new();
        for domain in &parsed.domains {
            let key = format!("{domain}_{ipv}");
            if seen.contains(&key) {
                duplicate_domains.push(domain.clone());
            } else {
                seen.insert(key);
                new_domains.push(domain.clone());
            }
        }

        if !new_domains.is_empty() {
            new_lines.push(format_line(&parsed.ip, &new_domains, &parsed.comment));
        }
        if !duplicate_domains.is_empty() {
            let inner = format_line(&parsed.ip, &duplicate_domains, "");
            new_lines.push(format!("# invalid hosts (repeated): {inner}"));
        }
    }

    new_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::paths::AppPaths;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn aggregates_on_nodes_only() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "a", "127.0.0.1 a.test\n").unwrap();
        entries::write_entry(&paths.entries_dir, "b", "127.0.0.1 b.test\n").unwrap();

        let list = json!([
            { "id": "a", "type": "local", "on": true },
            { "id": "b", "type": "local", "on": false }
        ])
        .as_array()
        .cloned()
        .unwrap();

        let content = aggregate_selected_content(&list, &paths, false).unwrap();
        assert!(content.contains("a.test"));
        assert!(!content.contains("b.test"));
    }

    #[test]
    fn aggregates_group_include_when_on() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "a", "127.0.0.1 a.test\n").unwrap();
        entries::write_entry(&paths.entries_dir, "b", "127.0.0.1 b.test\n").unwrap();

        let list = json!([
            { "id": "a", "type": "local", "on": false },
            { "id": "b", "type": "local", "on": false },
            {
                "id": "g1",
                "type": "group",
                "on": true,
                "include": ["a", "b"]
            }
        ])
        .as_array()
        .cloned()
        .unwrap();

        let content = aggregate_selected_content(&list, &paths, false).unwrap();
        assert!(content.contains("a.test"));
        assert!(content.contains("b.test"));
    }

    #[test]
    fn group_off_does_not_include_members() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "a", "127.0.0.1 a.test\n").unwrap();

        let list = json!([{
            "id": "g1",
            "type": "group",
            "on": false,
            "include": ["a"]
        }])
        .as_array()
        .cloned()
        .unwrap();

        let content = aggregate_selected_content(&list, &paths, false).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn dedupe_marks_repeated_domains() {
        let input = "127.0.0.1 foo bar\n127.0.0.1 foo\n";
        let out = remove_duplicate_records_pass(input);
        assert!(out.contains("invalid hosts (repeated)"));
    }
}
