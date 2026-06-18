mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_delta_subcommand_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified"));
}

#[test]
fn test_delta_subcommand_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"classification\""));
}

// ── Level 1 golden tests ─────────────────────────────────────────────────
//
// Phase 1 of the CLI Atheist feature (Leak 6b) replaced the CLI's
// hand-rolled `print_delta` ANSI writer with
// `darkmatter::markdown::delta::DeltaReport`, a `TerminalRenderable`
// that emits raw ANSI/Unicode bytes. The migration is required to be
// byte-for-byte equal to the previous CLI shape, so each scenario
// below captures the full expected output — ANSI escapes, Unicode
// classification glyphs, and visual-diff bytes — and compares the
// CLI output against it.
//
// The renderer writes fixed `format!` strings that depend only on the
// input bytes (verified deterministic across `NO_COLOR`, `FORCE_COLOR`,
// `COLUMNS`, and `CI`), so byte-level comparison is the right level.

/// Writes `base` and `updated` to a tempdir and returns the full stdout
/// produced by `md delta` (or `md -v delta` when `verbose` is set).
fn delta_stdout(base: &str, updated: &str, verbose: bool) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let base_path = dir.path().join("base.md");
    let updated_path = dir.path().join("updated.md");
    std::fs::write(&base_path, base).expect("write base");
    std::fs::write(&updated_path, updated).expect("write updated");

    let mut cmd = md_cmd();
    if verbose {
        cmd.arg("-v");
    }
    cmd.arg("delta").arg(&base_path).arg(&updated_path);

    let output = cmd.output().expect("md spawn");
    assert!(
        output.status.success(),
        "md delta failed with status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn delta_golden_no_change() {
    let out = delta_stdout("# Title\n\nHello\n", "# Title\n\nHello\n", false);
    assert_eq!(out, "\n\u{2713} No changes (0.0% changed)\n\n");
}

#[test]
fn delta_golden_frontmatter_change() {
    let base = "---\ntitle: Old\n---\n\n# Title\n\nHello\n";
    let updated = "---\ntitle: New\n---\n\n# Title\n\nHello\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Frontmatter:\n  ~ title: Updated frontmatter property 'title'\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_frontmatter_scalar_type_change() {
    // Same scalar key, different value type. The library parses `true` as
    // a boolean and `"true"` as a string, so the two frontmatters differ —
    // but the change is detected as a regular property update (not a
    // formatting-only change).
    let base = "---\nflag: true\n---\n\n# Doc\n";
    let updated = "---\nflag: \"true\"\n---\n\n# Doc\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Frontmatter:\n  ~ flag: Updated frontmatter property 'flag'\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_preamble_change() {
    let base = "Old intro\n\n# Title\n";
    let updated = "New intro\n\n# Title\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Preamble: modified\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_section_added() {
    let base = "# Title\n\nHello\n";
    let updated = "# Title\n\nHello\n\n## New\n\nNew content\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{25d0} Moderate changes (36.1% changed)\n\n\
        Added (1):\n  + Title > New\n\n\
        Whitespace only (1):\n  - Title: \x1b[3mblank lines\x1b[0m\n\n  \
        \x1b[2m\x1b[3mwhitespace only changes have no visual effect when rendered\x1b[0m\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_section_removed() {
    let base = "# Title\n\nHello\n\n## Old\n\nOld content\n";
    let updated = "# Title\n\nHello\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{25d0} Moderate changes (36.1% changed)\n\n\
        Removed (1):\n  - Title > Old\n\n\
        Whitespace only (1):\n  - Title: \x1b[3mblank lines\x1b[0m\n\n  \
        \x1b[2m\x1b[3mwhitespace only changes have no visual effect when rendered\x1b[0m\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_section_modified() {
    let base = "# Title\n\nHello world\n";
    let updated = "# Title\n\nHello there\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{25d0} Moderate changes (23.8% changed)\n\n\
        Modified (1):\n  - Title: text edited\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_moved_section() {
    // Renaming a heading keeps content identical but changes its path,
    // which the delta engine classifies as a structural move.
    let base = "# Hello\n\n## Section\n\nSee [link](#section)\n";
    let updated = "# Hello\n\n## Renamed\n\nSee [link](#section)\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{2295} Structural only (0.0% changed)\n\n\
        Moved (1):\n  \u{21b7} Hello > Section \u{2192} Hello > Renamed\n\n\
        \u{26a0} Broken links (1):\n  \u{2717} #section at line 6\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_whitespace_only() {
    let base = "# Title\n\nHello\n";
    let updated = "# Title\n\n  Hello  \n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n~ Whitespace changes only (0.0% changed)\n\n\
        Whitespace only (1):\n  - Title: \x1b[3mtrailing space, interior space\x1b[0m\n\n  \
        \x1b[2m\x1b[3mwhitespace only changes have no visual effect when rendered\x1b[0m\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_code_block_modified() {
    let base = "# Title\n\n```rust\nlet x = 1;\n```\n";
    let updated = "# Title\n\n```rust\nlet x = 2;\n```\n";
    let out = delta_stdout(base, updated, false);
    let expected = "\n\u{25b3} Minor changes (6.2% changed)\n\n\
        Modified (1):\n  - Title: text edited\n\n\
        Code blocks:\n  - \x1b[7mrust\x1b[0m code block in \x1b[1mTitle\x1b[0m \
        was \x1b[1mmodified\x1b[0m\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_code_block_language_changed() {
    // Switching the language token exercises the `Language: x → y` branch of
    // `format_code_block_change`. The styled token is the *updated* language
    // (the renderer receives `change.language`, which the engine populates
    // from the updated document), and the description spells out the
    // transition as `Language: rust → python`.
    let base = "# Title\n\n```rust\nlet x = 1;\n```\n";
    let updated = "# Title\n\n```python\nlet x = 1;\n```\n";
    let out = delta_stdout(base, updated, false);
    assert!(
        out.contains("Code blocks:"),
        "expected code-block change section, got:\n{out}"
    );
    assert!(
        out.contains("\x1b[7mpython\x1b[0m code block in \x1b[1mTitle\x1b[0m \
            changed its \x1b[1mlanguage\x1b[0m setting: rust \u{2192} python"),
        "expected styled language-change line, got:\n{out}"
    );
}

// ── Verbose mode (`md -v delta`) ─────────────────────────────────────────
//
// Verbose mode adds the statistics block and, when there are content or
// frontmatter changes, a visual diff. The visual diff bytes are also
// deterministic across `NO_COLOR`, `FORCE_COLOR`, and `COLUMNS`.

#[test]
fn delta_golden_verbose_no_change() {
    let out = delta_stdout("# Title\n\nHello\n", "# Title\n\nHello\n", true);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Statistics:\n  Bytes: 15 \u{2192} 15 (0 changed)\n  \
        Sections: 1 \u{2192} 1 (1 unchanged)\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_verbose_frontmatter_and_content() {
    let base = "---\ntitle: Old\n---\n\n# Title\n\nHello\n";
    let updated = "---\ntitle: New\n---\n\n# Title\n\nGoodbye\n";
    let out = delta_stdout(base, updated, true);

    // Header + statistics block. The percentages and byte counts are
    // deterministic given the input bytes above.
    let header = "\n\u{25c9} Major changes (41.2% changed)\n\n\
        Frontmatter:\n  ~ title: Updated frontmatter property 'title'\n\n\
        Modified (1):\n  - Title: +2 chars\n\n\
        Statistics:\n  Bytes: 15 \u{2192} 17 (2 changed)\n  \
        Sections: 1 \u{2192} 1 (0 unchanged)\n\n";
    assert!(
        out.starts_with(header),
        "verbose header did not match, got:\n{out}"
    );

    // Visual diff: frontmatter block first.
    assert!(
        out.contains("\x1b[1mFrontmatter Visual Diff:\x1b[0m\n"),
        "expected Frontmatter Visual Diff section, got:\n{out}"
    );
    assert!(
        out.contains("\x1b[2m\u{2500}\u{2500}\u{2500} \x1b[0moriginal\x1b[2m \u{2192} \
            \x1b[0mupdated\x1b[2m \u{2500}\u{2500}\u{2500}\x1b[0m\n"),
        "expected visual diff header rule, got:\n{out}"
    );
    // `Old` highlighted in red (256-color bg 88), `New` in green (bg 28).
    assert!(
        out.contains("title: \x1b[48;5;88m\x1b[1m\x1b[4mOld\x1b[0m"),
        "expected red `Old` highlight, got:\n{out}"
    );
    assert!(
        out.contains("title: \x1b[48;5;28m\x1b[1m\x1b[4mNew\x1b[0m"),
        "expected green `New` highlight, got:\n{out}"
    );

    // Body visual diff: `Hello` is partially highlighted (only the `H` and
    // `llo` runs survive on the original side; `Goodby` is highlighted on
    // the updated side and the trailing `e` is common).
    assert!(
        out.contains("\x1b[48;5;88m\x1b[1m\x1b[4mH\x1b[0me\x1b[48;5;88m\x1b[1m\x1b[4mllo\x1b[0m"),
        "expected `Hello` body diff on original side, got:\n{out}"
    );
    assert!(
        out.contains("\x1b[48;5;28m\x1b[1m\x1b[4mGoodby\x1b[0me"),
        "expected `Goodbye` body diff on updated side, got:\n{out}"
    );
}

#[test]
fn delta_golden_verbose_content_only_no_frontmatter_block() {
    // Content change without frontmatter change: verbose mode should NOT
    // emit the `Frontmatter Visual Diff:` block, only the body visual diff.
    let base = "# Title\n\nHello\n";
    let updated = "# Title\n\nGoodbye\n";
    let out = delta_stdout(base, updated, true);

    assert!(
        !out.contains("Frontmatter Visual Diff:"),
        "verbose should not emit frontmatter visual diff when frontmatter is unchanged:\n{out}"
    );
    assert!(
        out.contains("Statistics:\n  Bytes: 15 \u{2192} 17 (2 changed)\n"),
        "expected statistics block for content-only change, got:\n{out}"
    );
    // Body visual diff is emitted (Goodbye highlighted).
    assert!(
        out.contains("\x1b[48;5;28m\x1b[1m\x1b[4mGoodby\x1b[0me"),
        "expected `Goodbye` body diff, got:\n{out}"
    );
}
