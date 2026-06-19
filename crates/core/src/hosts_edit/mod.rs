#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ip,
    Hostname,
    Comment,
    Plain,
}

#[derive(Debug, Clone)]
pub struct ColoredSegment {
    pub kind: TokenKind,
    pub text: String,
}

pub fn parse_line_segments(line: &str) -> Vec<ColoredSegment> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return vec![ColoredSegment {
            kind: TokenKind::Plain,
            text: line.to_string(),
        }];
    }
    if trimmed.starts_with('#') {
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
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return line.to_string();
    }
    if trimmed.starts_with('#') {
        let uncommented = trimmed.trim_start_matches('#').trim_start();
        return uncommented.to_string();
    }
    format!("# {line}")
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
    fn toggle_comment_adds_hash() {
        assert_eq!(toggle_line_comment("127.0.0.1 x"), "# 127.0.0.1 x");
    }

    #[test]
    fn toggle_comment_removes_hash() {
        assert_eq!(toggle_line_comment("# 127.0.0.1 x"), "127.0.0.1 x");
    }
}
