//! Structural analyzer for Claudine's inline-test placement convention.
//!
//! The repository assertion is a Level 1 hard gate for Claudine's package area.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const PRODUCTION_LINE_LIMIT: usize = 800;
const INLINE_TEST_LINE_LIMIT: usize = 300;

const SOURCE_ROOTS: &[&str] = &[
    "lib/src",
    "cli/src",
    "contract/src",
    "catalog-types/src",
    "gen/src",
    "rendezvous/core/src",
    "rendezvous/client/src",
    "rendezvous/daemon/src",
];

struct Exception {
    path: &'static str,
    rationale: &'static str,
}

// Exceptions are deliberately file-specific and must explain why co-location
// is materially better. The repository test rejects entries after they become
// stale, so this table cannot become a permanent grandfather list.
const EXCEPTIONS: &[Exception] = &[];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ExceededThreshold {
    Production,
    InlineTests,
}

#[derive(Debug, PartialEq, Eq)]
struct Violation {
    path: String,
    production_lines: usize,
    test_lines: usize,
    exceeded: BTreeSet<ExceededThreshold>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineCounts {
    production: usize,
    tests: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InlineTestModule {
    start: usize,
    close: usize,
}

fn is_ident(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

/// Blanks comments and literals while preserving bytes and newline offsets.
fn sanitize(source: &str) -> Vec<u8> {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let blank = |output: &mut [u8], from: usize, to: usize| {
        for byte in output.iter_mut().take(to).skip(from) {
            if !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
    };

    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            let start = index;
            while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                index += 1;
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let start = index;
            let mut depth = 1usize;
            index += 2;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            blank(&mut output, start, index);
        } else if let Some((content_start, hashes)) = raw_string_open(bytes, index) {
            let start = index;
            index = content_start;
            while index < bytes.len() {
                if bytes[index] == b'"'
                    && bytes[index + 1..].len() >= hashes
                    && bytes[index + 1..].iter().take(hashes).all(|&byte| byte == b'#')
                {
                    index += 1 + hashes;
                    break;
                }
                index += 1;
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'"' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == b'"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            blank(&mut output, start, index);
        } else if bytes[index] == b'\'' {
            if let Some(end) = char_literal_end(bytes, index) {
                blank(&mut output, index, end);
                index = end;
            } else {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    output
}

fn raw_string_open(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if index > 0 && is_ident(bytes[index - 1]) {
        return None;
    }
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;
    let mut hashes = 0;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1, hashes))
}

fn char_literal_end(bytes: &[u8], index: usize) -> Option<usize> {
    let mut cursor = index + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        if bytes.get(cursor) == Some(&b'u') && bytes.get(cursor + 1) == Some(&b'{') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
        }
        cursor += 1;
    } else {
        let width = std::str::from_utf8(&bytes[cursor..])
            .ok()?
            .chars()
            .next()?
            .len_utf8();
        cursor += width;
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn skip_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn matching_delimiter(bytes: &[u8], open: usize, opening: u8) -> Option<usize> {
    let closing = match opening {
        b'[' => b']',
        b'{' => b'}',
        b'(' => b')',
        _ => return None,
    };
    let mut depth = 0usize;
    for (index, &byte) in bytes.iter().enumerate().skip(open) {
        if byte == opening {
            depth += 1;
        } else if byte == closing {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

struct CfgParser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl CfgParser<'_> {
    fn skip_whitespace(&mut self) {
        while self.bytes.get(self.index).is_some_and(u8::is_ascii_whitespace) {
            self.index += 1;
        }
    }

    fn identifier(&mut self) -> &[u8] {
        self.skip_whitespace();
        let start = self.index;
        while self.bytes.get(self.index).is_some_and(|&byte| is_ident(byte)) {
            self.index += 1;
        }
        &self.bytes[start..self.index]
    }

    /// Whether this cfg expression can only be true when `cfg(test)` is true.
    fn requires_test(&mut self) -> bool {
        let name = self.identifier().to_vec();
        if name == b"test" {
            return true;
        }
        self.skip_whitespace();
        if self.bytes.get(self.index) != Some(&b'(') {
            return false;
        }
        self.index += 1;
        let mut children = Vec::new();
        loop {
            self.skip_whitespace();
            if self.bytes.get(self.index) == Some(&b')') {
                self.index += 1;
                break;
            }
            if self.index >= self.bytes.len() {
                break;
            }
            children.push(self.requires_test());
            self.skip_to_cfg_separator();
            if self.bytes.get(self.index) == Some(&b',') {
                self.index += 1;
            }
        }
        match name.as_slice() {
            b"all" => children.into_iter().any(|requires_test| requires_test),
            b"any" => {
                !children.is_empty() && children.into_iter().all(|requires_test| requires_test)
            }
            // `not(test)` excludes tests, and an unknown predicate is not safe
            // to classify as test-only.
            _ => false,
        }
    }

    fn skip_to_cfg_separator(&mut self) {
        let mut depth = 0usize;
        while let Some(&byte) = self.bytes.get(self.index) {
            match byte {
                b'(' => depth += 1,
                b')' if depth == 0 => break,
                b')' => depth -= 1,
                b',' if depth == 0 => break,
                _ => {}
            }
            self.index += 1;
        }
    }
}

fn attribute_requires_test(attribute: &[u8]) -> bool {
    let text = std::str::from_utf8(attribute).unwrap_or_default();
    let attribute = text.trim_start_matches("#[").trim_start();
    let Some(predicate) = attribute.strip_prefix("cfg") else {
        return false;
    };
    let predicate = predicate.trim_start();
    let Some(predicate) = predicate.strip_prefix('(') else {
        return false;
    };
    CfgParser {
        bytes: predicate.as_bytes(),
        index: 0,
    }
    .requires_test()
}

fn skip_visibility(bytes: &[u8], cursor: usize) -> usize {
    if bytes.get(cursor..cursor + 3) != Some(b"pub")
        || bytes.get(cursor + 3).is_some_and(|&byte| is_ident(byte))
    {
        return cursor;
    }
    let mut cursor = skip_whitespace(bytes, cursor + 3);
    if bytes.get(cursor) == Some(&b'(')
        && let Some(close) = matching_delimiter(bytes, cursor, b'(')
    {
        cursor = skip_whitespace(bytes, close + 1);
    }
    cursor
}

fn inline_test_modules(source: &str) -> Vec<InlineTestModule> {
    let bytes = sanitize(source);
    let mut modules = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'#' || bytes.get(index + 1) != Some(&b'[') {
            index += 1;
            continue;
        }

        let attribute_start = index;
        let mut cursor = index;
        let mut gated_by_test = false;
        while let Some(close) = matching_delimiter(&bytes, cursor + 1, b'[') {
            gated_by_test |= attribute_requires_test(&bytes[cursor..=close]);
            cursor = skip_whitespace(&bytes, close + 1);
            if bytes.get(cursor) != Some(&b'#') || bytes.get(cursor + 1) != Some(&b'[') {
                break;
            }
        }

        cursor = skip_visibility(&bytes, cursor);
        if gated_by_test
            && bytes.get(cursor..cursor + 3) == Some(b"mod")
            && bytes.get(cursor + 3).is_some_and(|&byte| !is_ident(byte))
        {
            cursor = skip_whitespace(&bytes, cursor + 3);
            while bytes.get(cursor).is_some_and(|&byte| is_ident(byte)) {
                cursor += 1;
            }
            cursor = skip_whitespace(&bytes, cursor);
            if bytes.get(cursor) == Some(&b'{')
                && let Some(close) = matching_delimiter(&bytes, cursor, b'{')
            {
                modules.push(InlineTestModule {
                    start: attribute_start,
                    close,
                });
                index = close + 1;
                continue;
            }
        }
        index = cursor.max(index + 1);
    }
    modules
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
    );
    if source.ends_with('\n') {
        starts.pop();
    }
    starts
}

fn line_index(starts: &[usize], offset: usize) -> usize {
    starts.partition_point(|&start| start <= offset).saturating_sub(1)
}

fn count_lines(source: &str) -> Option<LineCounts> {
    let modules = inline_test_modules(source);
    if modules.is_empty() {
        return None;
    }
    let starts = line_starts(source);
    let mut test_lines = BTreeSet::new();
    for module in modules {
        let first = line_index(&starts, module.start);
        let last = line_index(&starts, module.close);
        test_lines.extend(first..=last);
    }
    Some(LineCounts {
        production: starts.len().saturating_sub(test_lines.len()),
        tests: test_lines.len(),
    })
}

fn normalized(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn generated_path(path: &str) -> bool {
    path == "lib/src/signals/generated.rs"
        || path == "lib/src/model_catalog/families_generated.rs"
        || path == "lib/src/stream/providers/vocabulary.rs"
        || path.starts_with("lib/src/provider/") && path.ends_with("/data.rs")
}

fn generated_header(source: &str) -> bool {
    source
        .lines()
        .take(12)
        .any(|line| line.starts_with("// GENERATED by claudine-gen") && line.contains("DO NOT EDIT"))
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn violation(path: String, counts: LineCounts) -> Option<Violation> {
    let mut exceeded = BTreeSet::new();
    if counts.production > PRODUCTION_LINE_LIMIT {
        exceeded.insert(ExceededThreshold::Production);
    }
    if counts.tests > INLINE_TEST_LINE_LIMIT {
        exceeded.insert(ExceededThreshold::InlineTests);
    }
    (!exceeded.is_empty()).then_some(Violation {
        path,
        production_lines: counts.production,
        test_lines: counts.tests,
        exceeded,
    })
}

fn apply_exceptions(violations: &mut Vec<Violation>, exceptions: &[Exception]) -> Vec<String> {
    let live_paths: BTreeSet<_> = violations.iter().map(|item| item.path.as_str()).collect();
    let mut errors = Vec::new();
    for exception in exceptions {
        if exception.rationale.trim().is_empty() {
            errors.push(format!("exception {} has no rationale", exception.path));
        } else if !live_paths.contains(exception.path) {
            errors.push(format!("exception {} is stale", exception.path));
        }
    }
    violations.retain(|item| !exceptions.iter().any(|entry| entry.path == item.path));
    errors
}

fn scan_repository(area_root: &Path) -> Result<(Vec<Violation>, Vec<String>), String> {
    let mut files = Vec::new();
    for root in SOURCE_ROOTS {
        collect_rust_files(&area_root.join(root), &mut files)
            .map_err(|error| format!("failed to scan {root}: {error}"))?;
    }
    files.sort();

    let mut all_violations = Vec::new();
    for file in files {
        let relative = normalized(file.strip_prefix(area_root).map_err(|error| error.to_string())?);
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read {relative}: {error}"))?;
        if generated_path(&relative) || generated_header(&source) {
            continue;
        }
        if let Some(counts) = count_lines(&source)
            && let Some(found) = violation(relative, counts)
        {
            all_violations.push(found);
        }
    }

    let exception_errors = apply_exceptions(&mut all_violations, EXCEPTIONS);
    Ok((all_violations, exception_errors))
}

fn report(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return String::new();
    }
    let mut both = 0;
    let mut production_only = 0;
    let mut tests_only = 0;
    for item in violations {
        match (
            item.exceeded.contains(&ExceededThreshold::Production),
            item.exceeded.contains(&ExceededThreshold::InlineTests),
        ) {
            (true, true) => both += 1,
            (true, false) => production_only += 1,
            (false, true) => tests_only += 1,
            (false, false) => unreachable!("a violation must exceed a threshold"),
        }
    }
    let details = violations
        .iter()
        .map(|item| {
            let exceeded = item
                .exceeded
                .iter()
                .map(|threshold| match threshold {
                    ExceededThreshold::Production => format!("production>{PRODUCTION_LINE_LIMIT}"),
                    ExceededThreshold::InlineTests => format!("inline-tests>{INLINE_TEST_LINE_LIMIT}"),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}: production={}, inline-tests={} ({exceeded})",
                item.path, item.production_lines, item.test_lines
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "violations={} (both={both}, production-only={production_only}, \
         inline-tests-only={tests_only})\n{details}",
        violations.len()
    )
}

#[test]
fn repository_test_placement() {
    let area_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CLI crate should be inside the Claudine package area");
    let (violations, exception_errors) = scan_repository(area_root).expect("repository scan");
    assert!(exception_errors.is_empty(), "{}", exception_errors.join("\n"));

    let diagnostics = report(&violations);
    assert!(violations.is_empty(), "test-placement violations:\n{diagnostics}");
}

#[test]
fn analyzer_handles_rust_syntax_and_nested_braces() {
    let source = r####"
fn production() {
    let fake = "#[cfg(test)] mod tests { }";
    let raw = r###"mod tests { { } }"###;
    /* #[cfg(test)] mod ignored { } /* nested { comment } */ */
}

#[allow(clippy::bool_assert_comparison)]
#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn braces_in_literals_do_not_close_the_module() {
        assert_eq!('}', '}');
        assert_eq!(b'}', b'}');
        assert_eq!("}", "}");
    }
}
"####;
    assert_eq!(
        count_lines(source),
        Some(LineCounts {
            production: 7,
            tests: 10,
        })
    );
}

#[test]
fn analyzer_is_newline_portable_and_ignores_external_test_modules() {
    let unix = "fn live() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn case() {}\n}\n";
    let windows = unix.replace('\n', "\r\n");
    assert_eq!(count_lines(unix), count_lines(&windows));
    assert_eq!(count_lines("#[cfg(test)]\nmod tests;\n"), None);
    assert_eq!(
        count_lines("#[cfg(any(test, unix))]\nmod mixed { fn live() {} }\n"),
        None
    );
    assert!(count_lines("#[cfg(all(test, unix))]\nmod tests { fn case() {} }\n").is_some());
}

#[test]
fn analyzer_recognizes_visibility_qualified_inline_test_modules() {
    for visibility in ["", "pub ", "pub(crate) ", "pub(super) "] {
        let source = format!(
            "fn live() {{}}\n#[cfg(test)]\n{visibility}mod tests {{\n    fn case() {{}}\n}}\n"
        );
        assert_eq!(
            count_lines(&source),
            Some(LineCounts {
                production: 1,
                tests: 4,
            }),
            "visibility {visibility:?} was not recognized"
        );
    }
}

#[test]
fn thresholds_and_diagnostics_are_centralized() {
    let both = violation(
        "lib/src/example.rs".into(),
        LineCounts {
            production: PRODUCTION_LINE_LIMIT + 1,
            tests: INLINE_TEST_LINE_LIMIT + 1,
        },
    )
    .expect("both limits should be exceeded");
    assert_eq!(
        both.exceeded,
        BTreeSet::from([
            ExceededThreshold::Production,
            ExceededThreshold::InlineTests,
        ])
    );
    assert!(report(&[both]).contains("production=801, inline-tests=301"));
}

#[test]
fn generated_files_require_an_explicit_path_or_header() {
    assert!(generated_path("lib/src/provider/claude/data.rs"));
    assert!(generated_path("lib/src/signals/generated.rs"));
    assert!(generated_header(
        "// GENERATED by claudine-gen — DO NOT EDIT BY HAND.\n"
    ));
    assert!(!generated_path("gen/src/registry.rs"));
    assert!(!generated_header("// This helper processes generated data.\n"));
}

#[test]
fn exceptions_require_a_live_violation_and_durable_rationale() {
    let mut violations = vec![violation(
        "lib/src/live.rs".into(),
        LineCounts {
            production: PRODUCTION_LINE_LIMIT + 1,
            tests: 1,
        },
    )
    .expect("fixture violation")];
    let errors = apply_exceptions(
        &mut violations,
        &[
            Exception {
                path: "lib/src/live.rs",
                rationale: "The test depends on private parser state.",
            },
            Exception {
                path: "lib/src/stale.rs",
                rationale: "No longer applicable.",
            },
            Exception {
                path: "lib/src/missing-reason.rs",
                rationale: "",
            },
        ],
    );
    assert!(violations.is_empty());
    assert_eq!(
        errors,
        [
            "exception lib/src/stale.rs is stale",
            "exception lib/src/missing-reason.rs has no rationale",
        ]
    );
}
