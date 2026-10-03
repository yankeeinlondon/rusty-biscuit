//! Class guard: text spliced into `biscuit-terminal` Prose markup is escaped by
//! `Prose::escape_text`, not by a hand-rolled escaper in `claudine-cli` or
//! `claudine`.
//!
//! The hand-rolled escapers this replaced each covered a subset of the Prose
//! grammar (`<`, `>`, `{`, `\`, sometimes `"`), so an author's `_pr_open_.md`
//! or `_loop_count` lost its underscores to emphasis. The scan recognizes the
//! shapes they took: a `.replace('<', …)` call, or a `'<'` pattern whose arm
//! pushes a backslash or an entity. Every file's count must match its entry in
//! [`ALLOWED`] or [`UNFIXED`], zero for a file in neither.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Hand-rolled escapers that are not escaping text, with the reason.
const ALLOWED: &[(&str, usize, &str)] = &[
    (
        "cli/src/commands/help.rs",
        1,
        "escapes fixed option labels such as `--debug <LEVEL>`, never author text",
    ),
    (
        "lib/src/stream/path_link.rs",
        1,
        "`escape_href` escapes an `href` attribute value; the visible text goes through `escape_text`",
    ),
];

/// Text escapers that still skip `_`, `*`, `[`, and `(`, so a name such as
/// `_draft_.md` renders as italic `draft.md`. When one is fixed, this test
/// fails until its entry is removed.
const UNFIXED: &[(&str, usize, &str)] = &[];

/// Lines of `source` that belong to a hand-rolled Prose escaper, 1-based.
/// An inline `#[cfg(test)] mod … {` block and everything after it is skipped.
fn escaper_lines(source: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let end = lines
        .windows(2)
        .position(|pair| {
            let next = pair[1].trim();
            pair[0].trim() == "#[cfg(test)]" && next.starts_with("mod ") && next.ends_with('{')
        })
        .unwrap_or(lines.len());
    (0..end)
        .filter(|&index| {
            let line = lines[index];
            let arm = lines[index..(index + 3).min(end)].join("\n");
            line.contains("replace('<'")
                || (line.contains("'<'")
                    && (arm.contains(r"push('\\')") || arm.contains(r#""\\<""#) || arm.contains(r#""&lt;""#)))
        })
        .map(|index| (index + 1, lines[index].trim().to_string()))
        .collect()
}

fn collect_rs_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn prose_text_is_escaped_only_by_prose_escape_text() {
    let area = biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("claudine-cli sits in the claudine area")
        .to_path_buf();
    let mut files = Vec::new();
    collect_rs_files(&biscuit_test_harness::manifest_dir!().join("src"), &mut files);
    collect_rs_files(
        &biscuit_test_harness::manifest_dir!()
            .parent()
            .expect("claudine-cli sits in the claudine area")
            .join("lib/src"),
        &mut files,
    );
    assert!(files.len() > 100, "the scan found the sources ({} files)", files.len());

    let mut found: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for file in &files {
        let relative = file
            .strip_prefix(&area)
            .expect("scanned under the area")
            .to_string_lossy()
            .replace('\\', "/");
        if relative.contains("/tests/") || relative.ends_with("/tests.rs") {
            continue;
        }
        let source = fs::read_to_string(file).unwrap_or_else(|error| panic!("read {relative}: {error}"));
        let lines = escaper_lines(&source);
        if !lines.is_empty() {
            found.insert(relative, lines);
        }
    }

    let expected: BTreeMap<&str, usize> =
        ALLOWED.iter().chain(UNFIXED).map(|(path, count, _)| (*path, *count)).collect();
    let mut problems = Vec::new();
    for (path, lines) in &found {
        if expected.get(path.as_str()) != Some(&lines.len()) {
            let listing: Vec<String> = lines.iter().map(|(line, text)| format!("    {path}:{line}: {text}")).collect();
            problems.push(format!(
                "{path}: {} hand-rolled escaper line(s), {} expected; splice text with `Prose::escape_text` (attributes with `Prose::quoted_attr`):\n{}",
                lines.len(),
                expected.get(path.as_str()).copied().unwrap_or(0),
                listing.join("\n")
            ));
        }
    }
    for (path, count) in &expected {
        if !found.contains_key(*path) {
            problems.push(format!("{path}: listed with {count} line(s) but has none; remove its entry"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}

/// The scan recognizes every shape the replaced escapers took, and not the
/// delegating form that replaced them.
#[test]
fn the_scan_recognizes_each_replaced_escaper_shape() {
    let char_match = r"
fn escape_prose_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' | '\x22' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}
";
    let push_str_arm = r#"
        match ch {
            '<' => out.push_str("\\<"),
            '>' => out.push_str("\\>"),
            _ => out.push(ch),
        }
"#;
    let replace_backslash = r#"    text.replace('<', "\\<").replace('>', "\\>")"#;
    let replace_entity = r#"    input.replace('<', "&lt;").replace('>', "&gt;")"#;
    for (shape, source) in [
        ("char match", char_match),
        ("push_str arm", push_str_arm),
        ("backslash replace", replace_backslash),
        ("entity replace", replace_entity),
    ] {
        assert_eq!(escaper_lines(source).len(), 1, "{shape}:\n{source}");
    }

    let delegating = "fn escape_prose(input: &str) -> String {\n    Prose::escape_text(input)\n}\n";
    assert!(escaper_lines(delegating).is_empty());
    let parser = "match ch {\n    '<' => depth += 1,\n    '>' => depth -= 1,\n    _ => {}\n}\n";
    assert!(escaper_lines(parser).is_empty(), "a `'<'` arm that does not escape is not an escaper");
    let in_tests = format!("fn a() {{}}\n\n#[cfg(test)]\nmod tests {{\n{replace_backslash}\n}}\n");
    assert!(escaper_lines(&in_tests).is_empty(), "an inline test module is not scanned");
}
