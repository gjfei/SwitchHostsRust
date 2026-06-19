//! Hosts 文件语法分类与注释切换（对齐 SwitchHosts `hosts_highlight.ts`）。

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ip,
    Hostname,
    Comment,
    Plain,
    Error,
}

#[derive(Debug, Clone)]
pub struct ColoredSegment {
    pub kind: TokenKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentToggleResult {
    pub content: String,
    pub selection_start: usize,
    pub selection_end: usize,
    pub changed: bool,
}

static HOSTS_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*([\d.]+|[\da-f:.%lo]+)\s+\w").expect("valid regex"));

static COMMENT_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)#\s*").expect("valid regex"));

#[derive(Clone)]
struct LineInfo {
    start: usize,
    text: String,
}

#[derive(Clone, Copy)]
enum Transform {
    Insert { at: usize, len: usize },
    Remove { start: usize, end: usize },
}

pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn is_hosts_comment_line(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

pub fn is_valid_hosts_line(line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }
    if is_hosts_comment_line(line) {
        return true;
    }
    HOSTS_LINE_RE.is_match(line)
}

pub fn parse_line_segments(line: &str) -> Vec<ColoredSegment> {
    if !line.trim().is_empty() && !is_hosts_comment_line(line) && !is_valid_hosts_line(line) {
        return vec![ColoredSegment {
            kind: TokenKind::Error,
            text: line.to_string(),
        }];
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
        return vec![ColoredSegment {
            kind: TokenKind::Plain,
            text: line.to_string(),
        }];
    }
    if is_hosts_comment_line(line) {
        return vec![ColoredSegment {
            kind: TokenKind::Comment,
            text: line.to_string(),
        }];
    }

    let (before, comment) = match line.split_once('#') {
        Some((b, c)) => (b, Some(c)),
        None => (line, None),
    };
    let parts: Vec<&str> = before.split_whitespace().collect();
    if parts.is_empty() {
        return vec![ColoredSegment {
            kind: TokenKind::Plain,
            text: line.to_string(),
        }];
    }

    let mut segments = Vec::new();
    segments.push(ColoredSegment {
        kind: if looks_like_ip(parts[0]) {
            TokenKind::Ip
        } else {
            TokenKind::Plain
        },
        text: parts[0].to_string(),
    });
    for host in parts.iter().skip(1) {
        segments.push(ColoredSegment {
            kind: TokenKind::Hostname,
            text: host.to_string(),
        });
    }
    if let Some(c) = comment {
        segments.push(ColoredSegment {
            kind: TokenKind::Comment,
            text: format!("#{c}"),
        });
    }
    segments
}

pub fn toggle_line_comment(line: &str) -> String {
    toggle_comment_by_line(line, 0, 0, 0).content
}

pub fn toggle_comment_by_selection(
    code: &str,
    selection_start: usize,
    selection_end: usize,
    move_to_next_line: bool,
) -> CommentToggleResult {
    let normalized = normalize_line_endings(code);
    let lines = get_lines(&normalized);
    let (start, end) = selection_range(selection_start, selection_end);
    let start_line = line_index_at_offset(&lines, start);
    let end_line = if start == end {
        start_line
    } else {
        line_index_at_offset(&lines, end.saturating_sub(1).max(start))
    };
    toggle_comment_lines(
        &normalized,
        selection_start,
        selection_end,
        start_line,
        end_line,
        move_to_next_line,
    )
}

pub fn toggle_comment_by_line(
    code: &str,
    line_index: usize,
    selection_start: usize,
    selection_end: usize,
) -> CommentToggleResult {
    let normalized = normalize_line_endings(code);
    let lines = get_lines(&normalized);
    if line_index >= lines.len() {
        return CommentToggleResult {
            content: normalized,
            selection_start,
            selection_end,
            changed: false,
        };
    }
    toggle_comment_lines(
        &normalized,
        selection_start,
        selection_end,
        line_index,
        line_index,
        false,
    )
}

fn toggle_comment_lines(
    code: &str,
    selection_start: usize,
    selection_end: usize,
    start_line_index: usize,
    end_line_index: usize,
    move_to_next_line: bool,
) -> CommentToggleResult {
    let lines = get_lines(code);
    let mut next_lines: Vec<String> = lines.iter().map(|l| l.text.clone()).collect();
    let mut transforms = Vec::new();
    let mut changed = false;

    for i in start_line_index..=end_line_index {
        let line = &lines[i];
        let result = toggle_line(&line.text, line.start);
        next_lines[i] = result.text;
        changed |= result.changed;
        if let Some(transform) = result.transform {
            transforms.push(transform);
        }
    }

    if !changed {
        return CommentToggleResult {
            content: code.to_string(),
            selection_start,
            selection_end,
            changed: false,
        };
    }

    let next_content = next_lines.join("\n");

    if move_to_next_line && selection_start == selection_end {
        let next_starts = line_start_offsets(&next_lines);
        let next_line_index = start_line_index + 1;
        let next_offset = next_starts
            .get(next_line_index)
            .copied()
            .unwrap_or(next_content.chars().count());
        return CommentToggleResult {
            content: next_content,
            selection_start: next_offset,
            selection_end: next_offset,
            changed: true,
        };
    }

    CommentToggleResult {
        content: next_content,
        selection_start: map_offset(selection_start, &transforms),
        selection_end: map_offset(selection_end, &transforms),
        changed: true,
    }
}

struct ToggleLineResult {
    text: String,
    changed: bool,
    transform: Option<Transform>,
}

fn toggle_line(line: &str, line_start: usize) -> ToggleLineResult {
    if line.trim().is_empty() {
        return ToggleLineResult {
            text: line.to_string(),
            changed: false,
            transform: None,
        };
    }

    if let Some(comment_match) = COMMENT_LINE_RE.captures(line) {
        let indent = comment_match.get(1).map(|m| m.as_str()).unwrap_or("");
        let full = comment_match.get(0).map(|m| m.as_str()).unwrap_or("");
        let indent_len = indent.chars().count();
        let full_len = full.chars().count();
        return ToggleLineResult {
            text: COMMENT_LINE_RE.replace(line, "$1").to_string(),
            changed: true,
            transform: Some(Transform::Remove {
                start: line_start + indent_len,
                end: line_start + full_len,
            }),
        };
    }

    ToggleLineResult {
        text: format!("# {line}"),
        changed: true,
        transform: Some(Transform::Insert { at: line_start, len: 2 }),
    }
}

fn get_lines(code: &str) -> Vec<LineInfo> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    for part in code.split('\n') {
        lines.push(LineInfo {
            start,
            text: part.to_string(),
        });
        start += part.chars().count() + 1;
    }
    if code.is_empty() {
        lines.push(LineInfo {
            start: 0,
            text: String::new(),
        });
    }
    lines
}

fn line_index_at_offset(lines: &[LineInfo], offset: usize) -> usize {
    for (i, line) in lines.iter().enumerate().rev() {
        if offset >= line.start {
            return i;
        }
    }
    0
}

fn line_start_offsets(lines: &[String]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(lines.len());
    let mut start = 0usize;
    for line in lines {
        starts.push(start);
        start += line.chars().count() + 1;
    }
    starts
}

fn selection_range(selection_start: usize, selection_end: usize) -> (usize, usize) {
    (selection_start.min(selection_end), selection_start.max(selection_end))
}

fn map_offset(offset: usize, transforms: &[Transform]) -> usize {
    let mut mapped = offset;
    for transform in transforms {
        match *transform {
            Transform::Insert { at, len } => {
                if offset >= at {
                    mapped += len;
                }
            }
            Transform::Remove { start, end } => {
                if offset <= start {
                    continue;
                }
                if offset < end {
                    mapped -= offset - start;
                    continue;
                }
                mapped -= end - start;
            }
        }
    }
    mapped
}

fn looks_like_ip(s: &str) -> bool {
    s.parse::<std::net::IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ip_and_hostname() {
        let segs = parse_line_segments("127.0.0.1 localhost # comment");
        assert_eq!(segs[0].kind, TokenKind::Ip);
        assert_eq!(segs[1].kind, TokenKind::Hostname);
    }

    #[test]
    fn marks_invalid_line_as_error() {
        let segs = parse_line_segments("not-a-valid-hosts-line");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].kind, TokenKind::Error);
    }

    #[test]
    fn validates_hosts_line() {
        assert!(is_valid_hosts_line("127.0.0.1 localhost"));
        assert!(is_valid_hosts_line("  # comment"));
        assert!(is_valid_hosts_line(""));
        assert!(!is_valid_hosts_line("invalid"));
    }

    #[test]
    fn toggle_comment_adds_hash() {
        assert_eq!(toggle_line_comment("127.0.0.1 x"), "# 127.0.0.1 x");
    }

    #[test]
    fn toggle_comment_removes_hash() {
        assert_eq!(toggle_line_comment("# 127.0.0.1 x"), "127.0.0.1 x");
    }

    #[test]
    fn toggle_selection_moves_cursor_to_next_line() {
        let code = "127.0.0.1 localhost\nfoo";
        let result = toggle_comment_by_selection(code, 0, 0, true);
        assert_eq!(result.content, "# 127.0.0.1 localhost\nfoo");
        assert_eq!(result.selection_start, "# 127.0.0.1 localhost\n".chars().count());
        assert_eq!(result.selection_end, result.selection_start);
    }

    #[test]
    fn toggle_selection_comments_all_touched_lines() {
        let code = "127.0.0.1 localhost\nfoo";
        let result = toggle_comment_by_selection(code, 0, code.chars().count(), false);
        assert_eq!(result.content, "# 127.0.0.1 localhost\n# foo");
        assert_eq!(result.selection_start, 2);
        assert_eq!(result.selection_end, code.chars().count() + 4);
    }

    #[test]
    fn blank_line_toggle_is_noop() {
        let code = "foo\n\nbar";
        let result = toggle_comment_by_selection(code, 4, 4, true);
        assert!(!result.changed);
        assert_eq!(result.content, code);
    }

    #[test]
    fn uncomment_preserves_indent_and_selection() {
        let code = "  # foo\nbar";
        let result = toggle_comment_by_selection(code, 4, 7, false);
        assert_eq!(result.content, "  foo\nbar");
        assert_eq!(result.selection_start, 2);
        assert_eq!(result.selection_end, 5);
    }

    #[test]
    fn toggle_by_gutter_index() {
        let code = "foo\nbar";
        let result = toggle_comment_by_line(code, 1, 0, 0);
        assert_eq!(result.content, "foo\n# bar");
        assert_eq!(result.selection_start, 0);
        assert_eq!(result.selection_end, 0);
    }

    #[test]
    fn normalizes_crlf_before_toggle() {
        let result = toggle_comment_by_selection("foo\r\nbar", 0, 0, true);
        assert_eq!(result.content, "# foo\nbar");
        assert_eq!(
            result.selection_start,
            "# foo\n".chars().count()
        );
    }
}
