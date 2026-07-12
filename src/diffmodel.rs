#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub kind: LineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<Hunk>,
}

/// Counts added/removed lines across all of a file's hunks.
pub fn file_stats(file: &FileDiff) -> (usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    for hunk in &file.hunks {
        for line in &hunk.lines {
            match line.kind {
                LineKind::Added => added += 1,
                LineKind::Removed => removed += 1,
                LineKind::Context => {}
            }
        }
    }
    (added, removed)
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

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures below are real `git diff` output (captured via a scratch repo), not
    // hand-written -- so they reflect git's actual formatting quirks rather than a
    // guess at them.

    const MIXED: &str = "\
diff --git a/added.txt b/added.txt
new file mode 100644
index 0000000..b698677
--- /dev/null
+++ b/added.txt
@@ -0,0 +1 @@
+brand new file
diff --git a/deleted.txt b/deleted.txt
deleted file mode 100644
index 4202011..0000000
--- a/deleted.txt
+++ /dev/null
@@ -1 +0,0 @@
-to be deleted
diff --git a/modified.txt b/modified.txt
index 83db48f..adc4dce 100644
--- a/modified.txt
+++ b/modified.txt
@@ -1,3 +1,4 @@
-line1
+line1 CHANGED
 line2
 line3
+line4 NEW
diff --git a/no_trailing_newline.txt b/no_trailing_newline.txt
index 2802503..a27014a 100644
--- a/no_trailing_newline.txt
+++ b/no_trailing_newline.txt
@@ -1 +1 @@
-no newline at end
\\ No newline at end of file
+no newline at end CHANGED
\\ No newline at end of file
diff --git a/renamed_new.txt b/renamed_new.txt
new file mode 100644
index 0000000..cc719ed
--- /dev/null
+++ b/renamed_new.txt
@@ -0,0 +1,2 @@
+new content
+extra line
diff --git a/renamed_old.txt b/renamed_old.txt
deleted file mode 100644
index 33194a0..0000000
--- a/renamed_old.txt
+++ /dev/null
@@ -1 +0,0 @@
-old content
";

    fn find<'a>(files: &'a [FileDiff], path: &str) -> &'a FileDiff {
        files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("no file with path {path} found in {files:?}"))
    }

    #[test]
    fn parses_new_file() {
        let files = parse(MIXED);
        let f = find(&files, "added.txt");
        assert_eq!(file_stats(f), (1, 0));
        assert_eq!(f.hunks[0].lines[0].kind, LineKind::Added);
        assert_eq!(f.hunks[0].lines[0].content, "brand new file");
    }

    #[test]
    fn parses_deleted_file_keeping_the_old_path() {
        let files = parse(MIXED);
        // The `b/` side is /dev/null for a deletion; the path must come from `a/`.
        let f = find(&files, "deleted.txt");
        assert_eq!(file_stats(f), (0, 1));
    }

    #[test]
    fn parses_modification_with_mixed_add_remove_context() {
        let files = parse(MIXED);
        let f = find(&files, "modified.txt");
        assert_eq!(file_stats(f), (2, 1));
        let kinds: Vec<LineKind> = f.hunks[0].lines.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                LineKind::Removed,
                LineKind::Added,
                LineKind::Context,
                LineKind::Context,
                LineKind::Added,
            ]
        );
    }

    #[test]
    fn no_newline_at_end_of_file_marker_does_not_produce_a_line() {
        let files = parse(MIXED);
        let f = find(&files, "no_trailing_newline.txt");
        // Exactly one removed + one added line; the "\ No newline..." marker lines
        // must not be parsed as content lines.
        assert_eq!(f.hunks[0].lines.len(), 2);
        assert_eq!(file_stats(f), (1, 1));
    }

    #[test]
    fn every_file_in_a_multi_file_diff_is_captured() {
        let files = parse(MIXED);
        let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "added.txt",
                "deleted.txt",
                "modified.txt",
                "no_trailing_newline.txt",
                "renamed_new.txt",
                "renamed_old.txt",
            ]
        );
    }

    #[test]
    fn parses_pure_rename_with_no_content_change_as_zero_hunks() {
        // A rename git considers 100% similar has no `---`/`+++`/`@@` lines at all --
        // just `rename from`/`rename to`. We still record the file (using the `b/`
        // path from the `diff --git` header), just with no hunks to render.
        let diff = "\
diff --git a/renamed_old.txt b/renamed_new2.txt
similarity index 100%
rename from renamed_old.txt
rename to renamed_new2.txt
";
        let files = parse(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "renamed_new2.txt");
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn parses_rename_with_content_change() {
        let diff = "\
diff --git a/big.txt b/big_renamed.txt
similarity index 57%
rename from big.txt
rename to big_renamed.txt
index 6f195b4..5a317f8 100644
--- a/big.txt
+++ b/big_renamed.txt
@@ -1,5 +1,5 @@
 aaa
-bbb
+bbb CHANGED
 ccc
 ddd
 eee
";
        let files = parse(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "big_renamed.txt");
        assert_eq!(file_stats(&files[0]), (1, 1));
    }

    #[test]
    fn binary_file_diff_degrades_to_zero_hunks_without_crashing() {
        // No `---`/`+++`/`@@` lines for true binary content -- just a "Binary files
        // ... differ" line. We can't render a binary diff, but parsing it must not
        // panic or drop the file entirely.
        let diff = "\
diff --git a/image.bin b/image.bin
new file mode 100644
index 0000000..f5d691f
Binary files /dev/null and b/image.bin differ
";
        let files = parse(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "image.bin");
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn empty_diff_produces_no_files() {
        assert!(parse("").is_empty());
    }
}
