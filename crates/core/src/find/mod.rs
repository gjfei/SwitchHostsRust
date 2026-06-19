use regex::Regex;

use crate::storage::entries;
use crate::storage::error::StorageError;
use crate::storage::manifest::{collect_content_ids, Manifest};
use crate::storage::paths::AppPaths;

#[derive(Debug, Clone)]
pub struct FindOptions {
    pub query: String,
    pub replace_with: Option<String>,
    pub is_regexp: bool,
    pub ignore_case: bool,
    pub do_replace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindMatch {
    pub entry_id: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
}

pub fn find_in_manifest(
    manifest: &Manifest,
    paths: &AppPaths,
    opts: &FindOptions,
) -> Result<Vec<FindMatch>, StorageError> {
    let mut ids = Vec::new();
    collect_content_ids(&manifest.root, &mut ids);
    let mut matches = Vec::new();
    for id in ids {
        let content = entries::read_entry(&paths.entries_dir, &id)?;
        for m in find_in_text(&id, &content, opts) {
            matches.push(m);
        }
    }
    Ok(matches)
}

pub fn replace_in_manifest(
    manifest: &Manifest,
    paths: &AppPaths,
    opts: &FindOptions,
) -> Result<(Manifest, usize), StorageError> {
    let m = manifest.clone();
    let mut ids = Vec::new();
    collect_content_ids(&m.root, &mut ids);
    let mut count = 0usize;
    for id in ids {
        let content = entries::read_entry(&paths.entries_dir, &id)?;
        let (new_content, n) = replace_in_text(&content, opts);
        if n > 0 {
            entries::write_entry(&paths.entries_dir, &id, &new_content)?;
            count += n;
        }
    }
    Ok((m, count))
}

fn find_in_text(entry_id: &str, content: &str, opts: &FindOptions) -> Vec<FindMatch> {
    let mut out = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        if line_contains(line, &opts.query, opts.ignore_case, opts.is_regexp) {
            let col = line.to_lowercase().find(&opts.query.to_lowercase()).unwrap_or(0);
            out.push(FindMatch {
                entry_id: entry_id.to_string(),
                line: line_no + 1,
                column: col + 1,
                text: line.to_string(),
            });
        }
    }
    out
}

fn replace_in_text(content: &str, opts: &FindOptions) -> (String, usize) {
    let Some(replace_with) = &opts.replace_with else {
        return (content.to_string(), 0);
    };
    if opts.is_regexp {
        let re = if opts.ignore_case {
            Regex::new(&format!("(?i){}", opts.query))
        } else {
            Regex::new(&opts.query)
        };
        if let Ok(re) = re {
            let count = re.find_iter(content).count();
            return (re.replace_all(content, replace_with.as_str()).to_string(), count);
        }
        return (content.to_string(), 0);
    }
    let mut count = 0;
    let mut result = content.to_string();
    loop {
        let idx = if opts.ignore_case {
            result.to_lowercase().find(&opts.query.to_lowercase())
        } else {
            result.find(&opts.query)
        };
        let Some(i) = idx else { break };
        result.replace_range(i..i + opts.query.len(), replace_with);
        count += 1;
    }
    (result, count)
}

fn line_contains(line: &str, query: &str, ignore_case: bool, is_regexp: bool) -> bool {
    if is_regexp {
        let pattern = if ignore_case {
            format!("(?i){query}")
        } else {
            query.to_string()
        };
        Regex::new(&pattern)
            .map(|re| re.is_match(line))
            .unwrap_or(false)
    } else if ignore_case {
        line.to_lowercase().contains(&query.to_lowercase())
    } else {
        line.contains(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_find_ignore_case() {
        let opts = FindOptions {
            query: "LOCALHOST".into(),
            replace_with: None,
            is_regexp: false,
            ignore_case: true,
            do_replace: false,
        };
        let m = find_in_text("1", "127.0.0.1 localhost\n", &opts);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn replace_literal() {
        let opts = FindOptions {
            query: "old".into(),
            replace_with: Some("new".into()),
            is_regexp: false,
            ignore_case: false,
            do_replace: true,
        };
        let (out, n) = replace_in_text("127.0.0.1 old.test\n", &opts);
        assert_eq!(n, 1);
        assert!(out.contains("new.test"));
    }
}
