use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../..").canonicalize().unwrap()
}

#[test]
fn context_default_exits_zero_and_produces_stdout() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "default context should produce stdout");
    assert!(
        stdout.contains("ctx.today"),
        "should display ctx.today; got: {stdout}"
    );
    assert!(
        stdout.contains("Property"),
        "should have Property column; got: {stdout}"
    );
    assert!(
        stdout.contains("Description"),
        "should have Description column; got: {stdout}"
    );
    assert!(
        stdout.contains("Date and Time"),
        "should have category grouping; got: {stdout}"
    );
}

#[test]
fn context_default_works_outside_repo() {
    let temp_dir = std::env::temp_dir();
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(&temp_dir)
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("ctx.today"),
        "should show ctx.today outside repo; got: {stdout}"
    );
    assert!(
        stdout.contains("Property"),
        "should have Property column outside repo; got: {stdout}"
    );
}

#[test]
fn context_values_exits_zero_and_produces_stdout() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "--values should produce stdout");
    assert!(
        stdout.contains("ctx.today"),
        "should display ctx.today; got: {stdout}"
    );
    assert!(
        stdout.contains("Value"),
        "should have Value column; got: {stdout}"
    );
    assert!(
        !stdout.contains("Description"),
        "should NOT have Description column; got: {stdout}"
    );
}

/// Verify that the canonical keys behind the date aliases render with real
/// values. The aliases (`ctx.utc`/`ctx.dow`/`ctx.dow_abbr`) are themselves
/// first-class descriptor rows in the `Date and Time → Aliases` subsection, but
/// their canonical counterparts must also resolve to non-null values.
#[test]
fn context_values_renders_canonical_keys_non_null() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    // Check canonical keys that correspond to the old aliases.
    for key in ["ctx.now_utc", "ctx.day", "ctx.day_abbr"] {
        let row = stdout
            .lines()
            .find(|line| line.contains(key))
            .unwrap_or_else(|| panic!("expected a row for {key}; got stdout:\n{stdout}"));
        assert!(
            !row.contains("null"),
            "{key} row must contain a real value, not `null`; row: {row}",
        );
    }
}

/// Regression: `--values` must also surface canonical values, not just
/// aliases. `ctx.today` always has a value, so use it as a sentinel.
#[test]
fn context_values_renders_non_null_for_canonical_keys() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let today_row = stdout
        .lines()
        .find(|line| line.contains("ctx.today"))
        .unwrap_or_else(|| panic!("expected ctx.today row; got stdout:\n{stdout}"));
    assert!(
        !today_row.contains("null"),
        "ctx.today must have a real value; row: {today_row}",
    );
}

#[test]
fn context_expressions_exits_zero_and_produces_stdout() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "--expressions should produce stdout");
    assert!(
        stdout.contains("Operator Precedence"),
        "should show precedence; got: {stdout}"
    );
    assert!(
        stdout.contains("Truthiness"),
        "should show truthiness; got: {stdout}"
    );
    assert!(
        stdout.contains("Unary Operators"),
        "should show unary operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Comparison Operators"),
        "should show comparison operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Arithmetic Operators"),
        "should show arithmetic operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Variable Access"),
        "should show variable access; got: {stdout}"
    );
    assert!(
        stdout.contains("Null Propagation"),
        "should show null propagation; got: {stdout}"
    );
    assert!(
        stdout.contains("Functions"),
        "should show functions; got: {stdout}"
    );
}

#[test]
fn context_side_effects_exits_zero_and_produces_stdout() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stdout.contains("Darkmatter Side-Effect Capabilities"),
        "--side-effects should show capability catalog on stdout; got: {stdout}"
    );
    assert!(
        stdout.contains("FilesystemWrite") || stdout.contains("Network") || stdout.contains("MarkdownMutation"),
        "--side-effects should show safety classifications; got: {stdout}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr must carry the --expressions hint; got: {stderr}"
    );
}

/// Regression: review-2 finding "tables are parsed with the first data row
/// as the header" — verify the rendered output now contains the real
/// `Value` / `Falsy` headers from the Truthiness table rather than the
/// first data row's contents being promoted to a header.
#[test]
fn context_expressions_truthiness_table_uses_real_headers() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    // Find the Truthiness section, then look for `Value` and `Falsy` in the
    // bytes that follow it. These are the documented column headers; if the
    // parser regressed they would be replaced by the first data row's
    // contents (`null` / `yes`).
    let truthiness_idx = stdout
        .find("Truthiness")
        .unwrap_or_else(|| panic!("expected Truthiness heading; got stdout:\n{stdout}"));
    let after = &stdout[truthiness_idx..];

    assert!(
        after.contains("Value"),
        "Truthiness table must carry the documented `Value` header; \
         output after heading:\n{after}",
    );
    assert!(
        after.contains("Falsy"),
        "Truthiness table must carry the documented `Falsy` header; \
         output after heading:\n{after}",
    );
}

/// F5: inline-code spans must render through one Prose-aware path. In plain
/// (`NO_COLOR`) output that means visible backticks and — crucially — never the
/// raw `<inverse>` Prose markup that leaks when `render_inline_code` output is
/// dropped into a `TableCellContent::Text` without rendering. The `||` mode
/// header and the operator cells both carry inline code.
#[test]
fn context_expressions_inline_code_renders_without_leaking_markup() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    // The modes-table header keeps its visible backticks in plain output.
    assert!(
        stdout.contains("`||`"),
        "plain expressions report must keep visible backticks on the `||` header; got:\n{stdout}"
    );
    // No raw Prose markup may leak into any header or cell.
    assert!(
        !stdout.contains("<inverse>") && !stdout.contains("</inverse>"),
        "plain output must not contain raw <inverse> markup; got:\n{stdout}"
    );
}

#[test]
fn context_default_writes_footer_to_stderr() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--values"),
        "stderr should mention --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_values_writes_footer_to_stderr() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("--values"),
        "stderr should NOT mention --values when already using --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_expressions_writes_footer_to_stderr() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--values"),
        "stderr should mention --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_side_effects_writes_footer_to_stderr() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--values"),
        "stderr should mention --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

// =====================================================================
// Phase 4 — Catalog contract tests
// =====================================================================

/// Strips ANSI SGR escape sequences so a captured table cell compares as plain
/// text. Even under `NO_COLOR` the centered `Type` column can carry a trailing
/// reset; the first cell (the canonical identifier / signature column these
/// tests parse) is otherwise clean.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            // Consume a CSI sequence: ESC '[' … final byte in `@`..=`~`.
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// The trimmed, de-styled first table-column cell of every box-drawing data row.
/// Header rows, wrapped continuation rows (blank first cell), and non-table
/// lines fall out. This is the column carrying each report's canonical
/// identifier (`ctx.NAME`) or signature, so duplicate, missing, or stray rows
/// are visible here even though presence-only `contains` checks would miss them.
fn first_column_cells(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut segments = line.split('│');
            segments.next()?; // discard the left-margin segment before the first border
            let cell = strip_ansi(segments.next()?).trim().to_string();
            (!cell.is_empty()).then_some(cell)
        })
        .collect()
}

/// Asserts `actual` is a permutation of `expected` — exactly one rendered row
/// per descriptor, with no missing, extra, or duplicate identifiers. Reports
/// each failure class separately so a regression is immediately diagnosable.
fn assert_one_row_each(kind: &str, expected: &[String], actual: &[String]) {
    use std::collections::BTreeMap;

    let mut want: BTreeMap<&str, usize> = BTreeMap::new();
    for id in expected {
        *want.entry(id.as_str()).or_default() += 1;
    }
    let mut got: BTreeMap<&str, usize> = BTreeMap::new();
    for id in actual {
        *got.entry(id.as_str()).or_default() += 1;
    }

    let duplicates: Vec<&str> = got
        .iter()
        .filter_map(|(&id, &count)| (count > 1).then_some(id))
        .collect();
    let missing: Vec<&str> = want.keys().filter(|id| !got.contains_key(*id)).copied().collect();
    let extra: Vec<&str> = got.keys().filter(|id| !want.contains_key(*id)).copied().collect();

    assert!(
        duplicates.is_empty() && missing.is_empty() && extra.is_empty(),
        "{kind} must render exactly one row per descriptor.\n  \
         duplicate: {duplicates:?}\n  missing: {missing:?}\n  extra: {extra:?}"
    );
}

/// Runs `claudine context <args>` at a wide terminal (200 cells) so no canonical
/// identifier wraps, then returns the rendered first-column cells.
fn context_first_column_cells(args: &[&str]) -> Vec<String> {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("COLUMNS", "200")
        .current_dir(repo_root())
        .args(args)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    first_column_cells(&stdout)
}

/// Default report must emit exactly one row per context descriptor — no missing,
/// extra, or duplicate. A wide terminal keeps every `ctx.` identifier on one
/// line so the first column can be parsed intact.
#[test]
fn context_default_includes_every_descriptor() {
    let descriptors = darkmatter::markdown::compose::context::context_variable_descriptors();
    let expected: Vec<String> = descriptors.iter().map(|d| format!("ctx.{}", d.name)).collect();

    let actual: Vec<String> = context_first_column_cells(&["context"])
        .into_iter()
        .filter(|cell| cell.starts_with("ctx."))
        .collect();

    assert_one_row_each("default report", &expected, &actual);
}

/// Values report must emit exactly one row per context descriptor — no missing,
/// extra, or duplicate.
#[test]
fn context_values_includes_every_descriptor() {
    let descriptors = darkmatter::markdown::compose::context::context_variable_descriptors();
    let expected: Vec<String> = descriptors.iter().map(|d| format!("ctx.{}", d.name)).collect();

    let actual: Vec<String> = context_first_column_cells(&["context", "--values"])
        .into_iter()
        .filter(|cell| cell.starts_with("ctx."))
        .collect();

    assert_one_row_each("values report", &expected, &actual);
}

/// Expression report must render exactly one row per expression-function
/// descriptor. The report interleaves several tables (operators, modes,
/// truthiness, functions), so the first-column cells are filtered to the
/// signature set before asserting one row per function.
#[test]
fn context_expressions_includes_every_function() {
    let functions = darkmatter::markdown::compose::expression::expression_function_descriptors();
    let expected: Vec<String> = functions.iter().map(|f| f.signature.to_string()).collect();
    let signatures: std::collections::HashSet<&str> =
        functions.iter().map(|f| f.signature).collect();

    let actual: Vec<String> = context_first_column_cells(&["context", "--expressions"])
        .into_iter()
        .filter(|cell| signatures.contains(cell.as_str()))
        .collect();

    assert_one_row_each("expression report", &expected, &actual);
}

/// Side-effects report must render exactly one row per side-effect descriptor
/// (overloaded arities are folded into one signature per the catalog).
#[test]
fn context_side_effects_includes_every_capability() {
    let effects = darkmatter::effects::effect_descriptors();
    let expected: Vec<String> = effects.iter().map(|e| e.signature.to_string()).collect();
    let signatures: std::collections::HashSet<&str> =
        effects.iter().map(|e| e.signature).collect();

    let actual: Vec<String> = context_first_column_cells(&["context", "--side-effects"])
        .into_iter()
        .filter(|cell| signatures.contains(cell.as_str()))
        .collect();

    assert_one_row_each("side-effects report", &expected, &actual);
}

/// Output ordering must be deterministic across invocations.
#[test]
fn context_deterministic_output() {
    let run = || {
        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .current_dir(repo_root())
            .args(["context"])
            .assert()
            .success();
        String::from_utf8_lossy(&assert.get_output().stdout).to_string()
    };

    let first = run();
    let second = run();
    assert_eq!(
        first, second,
        "default report output must be deterministic across invocations"
    );
}

/// No report may depend on Markdown parsing — guard against Markdown artifacts.
#[test]
fn context_no_markdown_parsing_artifacts() {
    for args in [vec!["context"], vec!["context", "--values"], vec!["context", "--expressions"], vec!["context", "--side-effects"]] {
        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .current_dir(repo_root())
            .args(&args)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        // Markdown heading syntax should not appear in parsed output
        assert!(
            !stdout.contains("## "),
            "report for {:?} must not contain Markdown heading syntax `## `; got:\n{stdout}",
            args
        );
        // Raw Markdown table row separators should not appear
        assert!(
            !stdout.contains("|---|---"),
            "report for {:?} must not contain Markdown table separators; got:\n{stdout}",
            args
        );
    }
}

/// clap must reject combined report-selection flags.
#[test]
fn context_clap_rejects_combined_flags() {
    for pair in [
        vec!["context", "--values", "--expressions"],
        vec!["context", "--values", "--side-effects"],
        vec!["context", "--expressions", "--side-effects"],
        vec!["context", "--values", "--expressions", "--side-effects"],
    ] {
        assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .current_dir(repo_root())
            .args(&pair)
            .assert()
            .failure();
    }
}

// =====================================================================
// Phase 4 — Command tests
// =====================================================================

/// `--values` must include every context row, even when null/unavailable.
#[test]
fn context_values_preserves_null_rows() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Some variables are always present (e.g., ctx.today), others may be null.
    // The key assertion is that null rows are present, not dropped.
    assert!(
        stdout.contains("null"),
        "values report must contain at least one `null` row (unavailable values are shown, not dropped); got:\n{stdout}"
    );
}

/// F6/Spec 13: `--side-effects` must perform no side effect. The report reads
/// only static metadata, and the `EffectEngine`'s default mutation root is the
/// current directory — so any accidental filesystem effect would surface as a
/// new/changed entry under the work dir. We run with a separate sandbox `HOME`
/// (so Claudine's own logging cannot create false positives in the work dir)
/// and assert the work dir is byte-for-byte unchanged.
#[test]
fn context_side_effects_makes_no_filesystem_changes() {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(bytes) = fs::read(&path) {
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                    out.insert(rel, bytes);
                }
            }
        }
        out
    }

    let home = tempfile::tempdir().expect("home sandbox");
    let work = tempfile::tempdir().expect("work sandbox");
    // A sentinel the report must not touch.
    fs::write(work.path().join("sentinel.txt"), b"untouched").unwrap();

    let before = snapshot(work.path());

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", home.path())
        .current_dir(work.path())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let after = snapshot(work.path());
    assert_eq!(
        before, after,
        "`context --side-effects` must not create, delete, or modify any file in the work dir"
    );
    // Engine construction and network attempts are not observable from a
    // filesystem snapshot; that gap is closed in-process by the unit test
    // `metadata_reports_construct_no_engine_and_attempt_no_network`, which
    // asserts Darkmatter's engine-build and network-attempt counters stay flat
    // across the render.
}

/// Side-effects report must use "capabilities" language and avoid availability claims.
#[test]
fn context_side_effects_uses_capability_language() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("capabilities"),
        "side-effects report must use 'capabilities' language; got:\n{stdout}"
    );
    assert!(
        !stdout.to_lowercase().contains("available"),
        "side-effects report must not claim capabilities are 'available'; got:\n{stdout}"
    );
    assert!(
        !stdout.to_lowercase().contains("enabled"),
        "side-effects report must not claim capabilities are 'enabled'; got:\n{stdout}"
    );
}

/// Footer hints must not claim side effects are enabled, allowlist-exempt, or config-free.
#[test]
fn context_footer_no_availability_claims() {
    for args in [vec!["context"], vec!["context", "--values"], vec!["context", "--expressions"], vec!["context", "--side-effects"]] {
        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .current_dir(repo_root())
            .args(&args)
            .assert()
            .success();

        let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
        let stderr_lower = stderr.to_lowercase();
        assert!(
            !stderr_lower.contains("enabled"),
            "footer for {:?} must not claim side effects are enabled; got stderr:\n{stderr}",
            args
        );
        assert!(
            !stderr_lower.contains("allowlist-exempt") && !stderr_lower.contains("exempt"),
            "footer for {:?} must not claim side effects are allowlist-exempt; got stderr:\n{stderr}",
            args
        );
        assert!(
            !stderr_lower.contains("configuration-free") && !stderr_lower.contains("config-free"),
            "footer for {:?} must not claim side effects are configuration-free; got stderr:\n{stderr}",
            args
        );
    }
}

// =====================================================================
// Narrow-terminal regression (review-3 finding 1)
// =====================================================================

/// At and above the documented minimum supported width (53 cells) every report
/// must render its catalog by wrapping with **all required columns preserved** —
/// never dropping a column and never degrading into the table planner's
/// "Table could not be rendered …" diagnostic. The spec forbids a
/// Claudine-specific narrow layout, so this asserts every required header and
/// representative content survive (not merely the absence of the diagnostic),
/// checking the whole report (full stdout). 53 is the binding floor for the
/// default and values reports; the expression and side-effect reports preserve
/// all columns below it, but are exercised here at the shared floor too.
#[test]
fn context_reports_preserve_all_columns_at_minimum_supported_width() {
    // (args, required headers, representative content sentinel)
    let cases: &[(&[&str], &[&str], &str)] = &[
        (&["context"], &["Property", "Type", "Description"], "ctx.today"),
        (&["context", "--values"], &["Property", "Type", "Value"], "ctx.today"),
        (&["context", "--expressions"], &["Function", "Description"], "min(a, b)"),
        (
            &["context", "--side-effects"],
            &["Capability", "Description", "Safety"],
            "ensure_file",
        ),
    ];

    // 53 is the minimum supported width; 60 and 80 confirm columns persist above it.
    for width in ["53", "60", "80"] {
        for (args, headers, sentinel) in cases {
            let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
                .env("NO_COLOR", "1")
                .env("COLUMNS", width)
                .current_dir(repo_root())
                .args(*args)
                .assert()
                .success();

            let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
            assert!(
                !stdout.contains("could not be rendered"),
                "`claudine {args:?}` at COLUMNS={width} must wrap, not emit the \
                 planner width-error; got stdout:\n{stdout}"
            );
            for header in *headers {
                assert!(
                    stdout.contains(header),
                    "`claudine {args:?}` at COLUMNS={width} must keep the `{header}` \
                     column; got stdout:\n{stdout}"
                );
            }
            assert!(
                stdout.contains(sentinel),
                "`claudine {args:?}` at COLUMNS={width} must still render catalog \
                 content ({sentinel}); got stdout:\n{stdout}"
            );
        }
    }
}

/// The removed narrow layout silently dropped the `Type` column at some narrow
/// widths (the iteration-4 regression at `COLUMNS=35`). Dropping is now gone:
/// at any width the default report either renders a full three-column table or
/// falls back to the shared `Table` component's diagnostic — it must never emit
/// a two-column table that keeps `Property`/`Description` but omits `Type`.
///
/// This sweeps the narrow band straddling the minimum supported width and
/// asserts that *every* table header row (a line carrying `Property` and
/// `Description`) also carries `Type`. Widths below the floor produce no header
/// rows (the diagnostic, not a degraded table); widths at and above it produce
/// header rows that must keep all three columns.
#[test]
fn context_default_report_never_drops_type_column_at_any_width() {
    let mut saw_header_row = false;

    for width in ["35", "40", "45", "50", "53", "56", "60"] {
        let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("COLUMNS", width)
            .current_dir(repo_root())
            .args(["context"])
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        for row in stdout
            .lines()
            .filter(|line| line.contains("Property") && line.contains("Description"))
        {
            saw_header_row = true;
            assert!(
                row.contains("Type"),
                "default report at COLUMNS={width} dropped the `Type` column; \
                 header row was:\n  {row}\n\nfull stdout:\n{stdout}"
            );
        }
    }

    assert!(
        saw_header_row,
        "expected at least one three-column header row across the swept widths; \
         the sweep proved nothing if no table ever rendered"
    );
}

/// The "Interpolation vs. Condition Mode" introduction uses corrected wording.
#[test]
fn context_expressions_corrected_interpolation_wording() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let intro = "The parser supports two modes with different operator behavior:";
    assert!(
        stdout.contains(intro),
        "expression report must use corrected interpolation intro:\n  {intro}\ngot:\n{stdout}"
    );

    // Ensure there are no consecutive colon-terminated intro lines
    let lines: Vec<&str> = stdout.lines().collect();
    for window in lines.windows(2) {
        let a = window[0].trim();
        let b = window[1].trim();
        if a.ends_with(':') && b.ends_with(':') {
            panic!(
                "found consecutive colon-terminated lines:\n  {a}\n  {b}\n\nfull output:\n{stdout}"
            );
        }
    }
}
