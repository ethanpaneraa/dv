#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub kind: LineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<Hunk>,
}

/// Parses `git diff`-style unified diff text into a list of per-file hunks.
pub fn parse(diff_text: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut old_line = 0u32;
    let mut new_line = 0u32;

    for raw_line in diff_text.lines() {
        if let Some(rest) = raw_line.strip_prefix("diff --git ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(f) = current.as_mut() {
                    f.hunks.push(hunk);
                }
            }
            if let Some(f) = current.take() {
                files.push(f);
            }
            let path = extract_path(rest);
            current = Some(FileDiff {
                path,
                hunks: Vec::new(),
            });
            continue;
        }

        // Prefer the b/ path from +++ if we couldn't cleanly parse the diff --git line
        // (e.g. paths containing spaces).
        if let Some(rest) = raw_line.strip_prefix("+++ ") {
            if let Some(f) = current.as_mut() {
                if let Some(p) = clean_ab_path(rest) {
                    f.path = p;
                }
            }
            continue;
        }

        if raw_line.starts_with("--- ") || raw_line.starts_with("index ") {
            continue;
        }

        if let Some(rest) = raw_line.strip_prefix("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(f) = current.as_mut() {
                    f.hunks.push(hunk);
                }
            }
            let (o, n) = parse_hunk_header(rest);
            old_line = o;
            new_line = n;
            current_hunk = Some(Hunk {
                header: raw_line.to_string(),
                lines: Vec::new(),
            });
            continue;
        }

        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };

        if let Some(content) = raw_line.strip_prefix('+') {
            hunk.lines.push(Line {
                kind: LineKind::Added,
                old_lineno: None,
                new_lineno: Some(new_line),
                content: content.to_string(),
            });
            new_line += 1;
        } else if let Some(content) = raw_line.strip_prefix('-') {
            hunk.lines.push(Line {
                kind: LineKind::Removed,
                old_lineno: Some(old_line),
                new_lineno: None,
                content: content.to_string(),
            });
            old_line += 1;
        } else if let Some(content) = raw_line.strip_prefix(' ') {
            hunk.lines.push(Line {
                kind: LineKind::Context,
                old_lineno: Some(old_line),
                new_lineno: Some(new_line),
                content: content.to_string(),
            });
            old_line += 1;
            new_line += 1;
        }
        // Lines like "\ No newline at end of file" are silently skipped.
    }

    if let Some(hunk) = current_hunk.take() {
        if let Some(f) = current.as_mut() {
            f.hunks.push(hunk);
        }
    }
    if let Some(f) = current.take() {
        files.push(f);
    }

    files
}

fn extract_path(rest: &str) -> String {
    // rest looks like: `a/some/path b/some/path`
    if let Some(idx) = rest.find(" b/") {
        return rest[idx + 3..].to_string();
    }
    rest.to_string()
}

fn clean_ab_path(rest: &str) -> Option<String> {
    let rest = rest.trim();
    if rest == "/dev/null" {
        return None;
    }
    let rest = rest
        .strip_prefix("b/")
        .or_else(|| rest.strip_prefix("a/"))
        .unwrap_or(rest);
    Some(rest.to_string())
}

fn parse_hunk_header(rest: &str) -> (u32, u32) {
    // rest looks like: `-12,7 +12,8 @@ optional context`
    let mut old_start = 1u32;
    let mut new_start = 1u32;

    let parts: Vec<&str> = rest.split(' ').collect();
    for part in parts {
        if let Some(spec) = part.strip_prefix('-') {
            old_start = spec.split(',').next().unwrap_or("1").parse().unwrap_or(1);
        } else if let Some(spec) = part.strip_prefix('+') {
            new_start = spec.split(',').next().unwrap_or("1").parse().unwrap_or(1);
        } else if part == "@@" {
            break;
        }
    }

    (old_start, new_start)
}
