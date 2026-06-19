//! Global find / replace across manifest entries (aligned with SwitchHosts `find.rs`).

use regex::{Regex, RegexBuilder};
use serde_json::Value;

use crate::storage::entries;
use crate::storage::error::StorageError;
use crate::storage::manifest::{find_node, Manifest};
use crate::storage::paths::AppPaths;

#[derive(Debug, Clone, Default)]
pub struct FindSearchOptions {
    pub is_regexp: bool,
    pub ignore_case: bool,
}

#[derive(Debug, Clone)]
pub struct FindPosition {
    pub start_byte: usize,
    pub end_byte: usize,
    pub line: usize,
    pub line_pos: usize,
    pub before: String,
    pub match_text: String,
    pub after: String,
}

#[derive(Debug, Clone)]
pub struct FindItem {
    pub item_id: String,
    pub item_title: String,
    pub item_type: String,
    pub positions: Vec<FindPosition>,
}

/// Flattened row for the find window result list.
#[derive(Debug, Clone)]
pub struct FindMatchRow {
    pub item_id: String,
    pub item_title: String,
    pub item_type: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub line: usize,
    pub before: String,
    pub match_text: String,
    pub after: String,
    pub is_readonly: bool,
    pub is_disabled: bool,
    pub replace_to: Option<String>,
}

pub fn flatten_find_items(items: &[FindItem]) -> Vec<FindMatchRow> {
    let mut rows = Vec::new();
    for item in items {
        let is_readonly = item.item_type != "local";
        for pos in &item.positions {
            rows.push(FindMatchRow {
                item_id: item.item_id.clone(),
                item_title: item.item_title.clone(),
                item_type: item.item_type.clone(),
                start_byte: pos.start_byte,
                end_byte: pos.end_byte,
                line: pos.line,
                before: pos.before.clone(),
                match_text: pos.match_text.clone(),
                after: pos.after.clone(),
                is_readonly,
                is_disabled: false,
                replace_to: None,
            });
        }
    }
    rows
}

/// Shift byte range after earlier replacements in the same file were applied.
pub fn adjusted_replace_range(rows: &[FindMatchRow], index: usize) -> Option<(usize, usize)> {
    let row = rows.get(index)?;
    let delta = rows.iter().enumerate().fold(0isize, |sum, (i, item)| {
        if i == index
            || item.item_id != row.item_id
            || !item.is_disabled
            || item.start_byte >= row.start_byte
        {
            return sum;
        }
        let replaced_len = item.replace_to.as_ref().map(|s| s.len()).unwrap_or(0);
        sum + replaced_len as isize - item.match_text.len() as isize
    });
    let start = (row.start_byte as isize + delta).max(0) as usize;
    let end = (row.end_byte as isize + delta).max(start as isize) as usize;
    Some((start, end))
}

pub fn find_in_manifest(
    manifest: &Manifest,
    paths: &AppPaths,
    keyword: &str,
    options: &FindSearchOptions,
) -> Result<Vec<FindItem>, FindError> {
    if keyword.is_empty() {
        return Ok(Vec::new());
    }
    let regex = build_regex(keyword, options)?;
    let mut out = Vec::new();
    walk_searchable(&manifest.root, &mut |id, title, kind| {
        let content = match entries::read_entry(&paths.entries_dir, id) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("find read {id}: {e}");
                return;
            }
        };
        let positions = find_positions_in_content(&content, &regex);
        if positions.is_empty() {
            return;
        }
        out.push(FindItem {
            item_id: id.to_string(),
            item_title: title.to_string(),
            item_type: kind.to_string(),
            positions,
        });
    });
    Ok(out)
}

pub struct ReplaceOneArgs {
    pub item_id: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub expected: String,
    pub replace_to: String,
}

pub fn replace_one(
    manifest: &Manifest,
    paths: &AppPaths,
    args: &ReplaceOneArgs,
) -> Result<bool, StorageError> {
    if args.start_byte > args.end_byte {
        return Ok(false);
    }
    let Some(node) = find_node(&manifest.root, &args.item_id) else {
        return Ok(false);
    };
    let kind = node.get("type").and_then(Value::as_str).unwrap_or("local");
    if kind != "local" {
        return Ok(false);
    }

    let mut content = entries::read_entry(&paths.entries_dir, &args.item_id)?;
    if content.get(args.start_byte..args.end_byte) != Some(args.expected.as_str()) {
        return Ok(false);
    }
    content.replace_range(args.start_byte..args.end_byte, &args.replace_to);
    entries::write_entry(&paths.entries_dir, &args.item_id, &content)?;
    Ok(true)
}

pub struct ReplaceAllOutcome {
    pub item_ids: Vec<String>,
    pub replaced_count: usize,
}

pub fn replace_all_in_manifest(
    manifest: &Manifest,
    paths: &AppPaths,
    keyword: &str,
    options: &FindSearchOptions,
    replace_to: &str,
) -> Result<ReplaceAllOutcome, FindError> {
    if keyword.is_empty() {
        return Ok(ReplaceAllOutcome {
            item_ids: Vec::new(),
            replaced_count: 0,
        });
    }
    let regex = build_regex(keyword, options)?;
    let mut pending_writes: Vec<(String, String, usize)> = Vec::new();
    let mut read_error: Option<StorageError> = None;

    walk_searchable(&manifest.root, &mut |id, _title, kind| {
        if read_error.is_some() || kind != "local" {
            return;
        }
        let content = match entries::read_entry(&paths.entries_dir, id) {
            Ok(c) => c,
            Err(e) => {
                read_error = Some(e);
                return;
            }
        };
        let (next, count) = replace_all_in_content(&content, &regex, replace_to);
        if count > 0 {
            pending_writes.push((id.to_string(), next, count));
        }
    });

    if let Some(err) = read_error {
        return Err(FindError::Storage(err));
    }

    let mut item_ids = Vec::new();
    let mut replaced_count = 0usize;
    for (id, next, count) in pending_writes {
        entries::write_entry(&paths.entries_dir, &id, &next)?;
        item_ids.push(id);
        replaced_count += count;
    }

    Ok(ReplaceAllOutcome {
        item_ids,
        replaced_count,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum FindError {
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

pub fn byte_to_char_index(content: &str, byte: usize) -> usize {
    content.char_indices().take_while(|(i, _)| *i < byte).count()
}

fn build_regex(keyword: &str, options: &FindSearchOptions) -> Result<Regex, FindError> {
    let pattern = if options.is_regexp {
        keyword.to_string()
    } else {
        regex::escape(keyword)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(options.ignore_case)
        .build()
        .map_err(|e| FindError::InvalidPattern(e.to_string()))
}

fn find_positions_in_content(content: &str, regex: &Regex) -> Vec<FindPosition> {
    let line_index = LineIndex::new(content);
    let mut positions = Vec::new();
    let mut line_idx = 0;

    for mat in regex.find_iter(content) {
        let start = mat.start();
        let end = mat.end();
        line_idx = line_index.line_idx_for_start(line_idx, start);
        let line_info = &line_index.lines[line_idx];
        let line = line_idx + 1;
        let line_pos = content[line_info.start_byte..start].chars().count();

        let match_text = mat.as_str();
        let end_line_end_byte = line_index
            .lines
            .get(line_idx)
            .map(|l| l.end_byte)
            .unwrap_or(content.len());

        positions.push(FindPosition {
            start_byte: start,
            end_byte: end,
            line,
            line_pos,
            before: content[line_info.start_byte..start].to_string(),
            match_text: match_text.to_string(),
            after: content[end..end_line_end_byte].to_string(),
        });
    }
    positions
}

fn replace_all_in_content(content: &str, regex: &Regex, replace_to: &str) -> (String, usize) {
    let mut out = String::with_capacity(content.len());
    let mut last_end = 0;
    let mut count = 0;

    for mat in regex.find_iter(content) {
        out.push_str(&content[last_end..mat.start()]);
        out.push_str(replace_to);
        last_end = mat.end();
        count += 1;
    }

    if count == 0 {
        return (content.to_string(), 0);
    }
    out.push_str(&content[last_end..]);
    (out, count)
}

#[derive(Debug, Clone, Copy)]
struct LineInfo {
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug)]
struct LineIndex {
    lines: Vec<LineInfo>,
}

impl LineIndex {
    fn new(content: &str) -> Self {
        let mut lines = vec![LineInfo {
            start_byte: 0,
            end_byte: content.len(),
        }];
        for (byte_idx, ch) in content.char_indices() {
            if ch == '\n' {
                if let Some(last) = lines.last_mut() {
                    last.end_byte = byte_idx;
                }
                lines.push(LineInfo {
                    start_byte: byte_idx + ch.len_utf8(),
                    end_byte: content.len(),
                });
            }
        }
        Self { lines }
    }

    fn line_idx_for_start(&self, mut current_idx: usize, byte_idx: usize) -> usize {
        while current_idx + 1 < self.lines.len()
            && self.lines[current_idx + 1].start_byte <= byte_idx
        {
            current_idx += 1;
        }
        current_idx
    }
}

fn walk_searchable<F: FnMut(&str, &str, &str)>(nodes: &[Value], visit: &mut F) {
    for node in nodes {
        let kind = node.get("type").and_then(Value::as_str).unwrap_or("local");
        if kind != "group" && kind != "folder" {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                let title = node.get("title").and_then(Value::as_str).unwrap_or("");
                visit(id, title, kind);
            }
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            walk_searchable(children, visit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_find_multiple_occurrences() {
        let regex = build_regex(
            "old",
            &FindSearchOptions {
                is_regexp: false,
                ignore_case: false,
            },
        )
        .unwrap();
        let positions = find_positions_in_content("old one old two\n", &regex);
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn literal_escape_metacharacters() {
        let regex = build_regex(
            "127.0.0.1",
            &FindSearchOptions {
                is_regexp: false,
                ignore_case: false,
            },
        )
        .unwrap();
        assert!(regex.is_match("127.0.0.1 localhost"));
    }

    #[test]
    fn replace_all_literal() {
        let regex = build_regex(
            "old",
            &FindSearchOptions {
                is_regexp: false,
                ignore_case: false,
            },
        )
        .unwrap();
        let (out, n) = replace_all_in_content("old one old two", &regex, "new");
        assert_eq!(n, 2);
        assert_eq!(out, "new one new two");
    }

    #[test]
    fn adjusted_range_after_replace() {
        let mut rows = vec![
            FindMatchRow {
                item_id: "a".into(),
                item_title: String::new(),
                item_type: "local".into(),
                start_byte: 0,
                end_byte: 3,
                line: 1,
                before: String::new(),
                match_text: "old".into(),
                after: String::new(),
                is_readonly: false,
                is_disabled: true,
                replace_to: Some("x".into()),
            },
            FindMatchRow {
                item_id: "a".into(),
                item_title: String::new(),
                item_type: "local".into(),
                start_byte: 8,
                end_byte: 11,
                line: 1,
                before: String::new(),
                match_text: "old".into(),
                after: String::new(),
                is_readonly: false,
                is_disabled: false,
                replace_to: None,
            },
        ];
        let (start, end) = adjusted_replace_range(&rows, 1).unwrap();
        assert_eq!(start, 6);
        assert_eq!(end, 9);
        rows[1].is_disabled = true;
        let _ = rows;
    }
}
