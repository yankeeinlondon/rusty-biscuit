//! Class guard: text spliced into `biscuit-terminal` Prose markup is escaped by
//! `Prose::escape_text`, not by a hand-rolled escaper in `darkmatter-cli`.
//!
//! The replaced escapers (`approval.rs`, `clean/frontmatter_repair.rs`,
//! `schema/{validate,detect}.rs`) turned `<` into `&lt;`, which the terminal
//! printed literally, or escaped only `<` and `>`. The scan recognizes the
//! shapes they took: a `.replace('<', …)` call, or a `'<'` pattern whose arm
//! pushes a backslash or an entity. Every file's count must match its entry
//! in [`ALLOWED`], zero for a file not listed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Hand-rolled escapers that stay, with the reason.
const ALLOWED: &[(&str, usize, &str)] = &[(
    "src/commands/schema/about.rs",
    2,
    "`escape_markdown` targets Markdown output; `escape_prose` escapes the same set as \
     `Prose::escape_text` except `{`, which Prose no longer reads as markup",
)];

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
    let crate_root = biscuit_test_harness::manifest_dir!();
    let mut files = Vec::new();
    collect_rs_files(&biscuit_test_harness::manifest_dir!().join("src"), &mut files);
    assert!(files.len() > 20, "the scan found the sources ({} files)", files.len());

    let mut found: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for file in &files {
        let relative = file
            .strip_prefix(&crate_root)
            .expect("scanned under the crate")
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

    let expected: BTreeMap<&str, usize> = ALLOWED.iter().map(|(path, count, _)| (*path, *count)).collect();
    let mut problems = Vec::new();
    for (path, lines) in &found {
        if expected.get(path.as_str()) != Some(&lines.len()) {
            let listing: Vec<String> = lines.iter().map(|(line, text)| format!("    {path}:{line}: {text}")).collect();
            problems.push(format!(
                "{path}: {} hand-rolled escaper line(s), {} expected; splice text with `Prose::escape_text`:\n{}",
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

/// The scan recognizes the shapes the replaced escapers took, and not the
/// delegating form that replaced them.
#[test]
fn the_scan_recognizes_each_replaced_escaper_shape() {
    let replace_backslash = r#"    text.replace('<', "\\<").replace('>', "\\>")"#;
    let replace_entity = r#"    input.replace('<', "&lt;").replace('>', "&gt;")"#;
    for source in [replace_backslash, replace_entity] {
        assert_eq!(escaper_lines(source).len(), 1, "{source}");
    }
    let delegating = "fn escape_prose(input: &str) -> String {\n    Prose::escape_text(input)\n}\n";
    assert!(escaper_lines(delegating).is_empty());
}
