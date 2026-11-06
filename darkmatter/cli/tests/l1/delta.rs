use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_delta_subcommand_output() {
    let fixture = CliProcessFixture::named("delta-test-delta-subcommand-output");
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    fixture
        .command()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified"));
}

#[test]
fn test_delta_subcommand_json_output() {
    let fixture = CliProcessFixture::named("delta-test-delta-subcommand-json-output");
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    fixture
        .command()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .args(["--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"classification\""));
}

// ── Level 1 golden tests ─────────────────────────────────────────────────
//
// `md delta` on a pipe is the report's plain form. Each scenario captures the
// full expected output, classification glyphs and visual diff included, and
// `delta_stdout` proves no escape byte reaches the pipe.

/// Writes `base` and `updated` to a tempdir and returns the full stdout
/// produced by `md delta` (or `md -v delta` when `verbose` is set).
fn delta_stdout(fixture: &CliProcessFixture, base: &str, updated: &str, verbose: bool) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let base_path = dir.path().join("base.md");
    let updated_path = dir.path().join("updated.md");
    std::fs::write(&base_path, base).expect("write base");
    std::fs::write(&updated_path, updated).expect("write updated");

    let mut cmd = fixture.command();
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
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(!stdout.contains('\x1b'), "no escape bytes on a pipe: {stdout:?}");
    stdout
}

#[test]
fn delta_golden_no_change() {
    let fixture = CliProcessFixture::named("delta-delta-golden-no-change");
    let out = delta_stdout(&fixture, "# Title\n\nHello\n", "# Title\n\nHello\n", false);
    assert_eq!(out, "\n\u{2713} No changes (0.0% changed)\n\n");
}

#[test]
fn delta_golden_frontmatter_change() {
    let fixture = CliProcessFixture::named("delta-delta-golden-frontmatter-change");
    let base = "---\ntitle: Old\n---\n\n# Title\n\nHello\n";
    let updated = "---\ntitle: New\n---\n\n# Title\n\nHello\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Frontmatter:\n  ~ title: Updated frontmatter property 'title'\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_frontmatter_scalar_type_change() {
    let fixture = CliProcessFixture::named("delta-delta-golden-frontmatter-scalar-type-change");
    // Same scalar key, different value type. The library parses `true` as
    // a boolean and `"true"` as a string, so the two frontmatters differ —
    // but the change is detected as a regular property update (not a
    // formatting-only change).
    let base = "---\nflag: true\n---\n\n# Doc\n";
    let updated = "---\nflag: \"true\"\n---\n\n# Doc\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Frontmatter:\n  ~ flag: Updated frontmatter property 'flag'\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_preamble_change() {
    let fixture = CliProcessFixture::named("delta-delta-golden-preamble-change");
    let base = "Old intro\n\n# Title\n";
    let updated = "New intro\n\n# Title\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Preamble: modified\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_section_added() {
    let fixture = CliProcessFixture::named("delta-delta-golden-section-added");
    let base = "# Title\n\nHello\n";
    let updated = "# Title\n\nHello\n\n## New\n\nNew content\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{25d0} Moderate changes (36.1% changed)\n\n\
        Added (1):\n  + Title > New\n\n\
        Whitespace only (1):\n  - Title: blank lines\n\n  \
        whitespace only changes have no visual effect when rendered\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_section_removed() {
    let fixture = CliProcessFixture::named("delta-delta-golden-section-removed");
    let base = "# Title\n\nHello\n\n## Old\n\nOld content\n";
    let updated = "# Title\n\nHello\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{25d0} Moderate changes (36.1% changed)\n\n\
        Removed (1):\n  - Title > Old\n\n\
        Whitespace only (1):\n  - Title: blank lines\n\n  \
        whitespace only changes have no visual effect when rendered\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_section_modified() {
    let fixture = CliProcessFixture::named("delta-delta-golden-section-modified");
    let base = "# Title\n\nHello world\n";
    let updated = "# Title\n\nHello there\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{25d0} Moderate changes (23.8% changed)\n\n\
        Modified (1):\n  - Title: text edited\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_moved_section() {
    let fixture = CliProcessFixture::named("delta-delta-golden-moved-section");
    // Renaming a heading keeps content identical but changes its path,
    // which the delta engine classifies as a structural move.
    let base = "# Hello\n\n## Section\n\nSee [link](#section)\n";
    let updated = "# Hello\n\n## Renamed\n\nSee [link](#section)\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{2295} Structural only (0.0% changed)\n\n\
        Moved (1):\n  \u{21b7} Hello > Section \u{2192} Hello > Renamed\n\n\
        \u{26a0} Broken links (1):\n  \u{2717} #section at line 6\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_whitespace_only() {
    let fixture = CliProcessFixture::named("delta-delta-golden-whitespace-only");
    let base = "# Title\n\nHello\n";
    let updated = "# Title\n\n  Hello  \n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n~ Whitespace changes only (0.0% changed)\n\n\
        Whitespace only (1):\n  - Title: trailing space, interior space\n\n  \
        whitespace only changes have no visual effect when rendered\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_code_block_modified() {
    let fixture = CliProcessFixture::named("delta-delta-golden-code-block-modified");
    let base = "# Title\n\n```rust\nlet x = 1;\n```\n";
    let updated = "# Title\n\n```rust\nlet x = 2;\n```\n";
    let out = delta_stdout(&fixture, base, updated, false);
    let expected = "\n\u{25b3} Minor changes (6.2% changed)\n\n\
        Modified (1):\n  - Title: text edited\n\n\
        Code blocks:\n  - rust code block in Title \
        was modified\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_code_block_language_changed() {
    let fixture = CliProcessFixture::named("delta-delta-golden-code-block-language-changed");
    // Switching the language token exercises the `Language: x → y` branch of
    // `format_code_block_change`. The leading token is the *updated* language
    // (the renderer receives `change.language`, which the engine populates
    // from the updated document), and the description spells out the
    // transition as `Language: rust → python`.
    let base = "# Title\n\n```rust\nlet x = 1;\n```\n";
    let updated = "# Title\n\n```python\nlet x = 1;\n```\n";
    let out = delta_stdout(&fixture, base, updated, false);
    assert!(
        out.contains("Code blocks:"),
        "expected code-block change section, got:\n{out}"
    );
    assert!(
        out.contains(
            "python code block in Title \
            changed its language setting: rust \u{2192} python"
        ),
        "expected language-change line, got:\n{out}"
    );
}

// ── Verbose mode (`md -v delta`) ─────────────────────────────────────────
//
// Verbose mode adds the statistics block and, when there are content or
// frontmatter changes, a visual diff.

#[test]
fn delta_golden_verbose_no_change() {
    let fixture = CliProcessFixture::named("delta-delta-golden-verbose-no-change");
    let out = delta_stdout(&fixture, "# Title\n\nHello\n", "# Title\n\nHello\n", true);
    let expected = "\n\u{2713} No changes (0.0% changed)\n\n\
        Statistics:\n  Bytes: 15 \u{2192} 15 (0 changed)\n  \
        Sections: 1 \u{2192} 1 (1 unchanged)\n\n";
    assert_eq!(out, expected);
}

#[test]
fn delta_golden_verbose_frontmatter_and_content() {
    let fixture = CliProcessFixture::named("delta-delta-golden-verbose-frontmatter-and-content");
    let base = "---\ntitle: Old\n---\n\n# Title\n\nHello\n";
    let updated = "---\ntitle: New\n---\n\n# Title\n\nGoodbye\n";
    let out = delta_stdout(&fixture, base, updated, true);

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
        out.contains("Frontmatter Visual Diff:\n"),
        "expected Frontmatter Visual Diff section, got:\n{out}"
    );
    assert!(
        out.contains(
            "\u{2500}\u{2500}\u{2500} original \u{2192} \
            updated \u{2500}\u{2500}\u{2500}\n"
        ),
        "expected visual diff header rule, got:\n{out}"
    );
    assert!(
        out.contains("-    1      \u{2502} title: Old\n"),
        "expected removed `Old` line, got:\n{out}"
    );
    assert!(
        out.contains("+         1 \u{2502} title: New\n"),
        "expected added `New` line, got:\n{out}"
    );

    // Body visual diff: one removed and one added line.
    assert!(
        out.contains("-    4      \u{2502} Hello\n"),
        "expected `Hello` body diff on original side, got:\n{out}"
    );
    assert!(
        out.contains("+         4 \u{2502} Goodbye\n"),
        "expected `Goodbye` body diff on updated side, got:\n{out}"
    );
}

#[test]
fn delta_golden_verbose_content_only_no_frontmatter_block() {
    let fixture =
        CliProcessFixture::named("delta-delta-golden-verbose-content-only-no-frontmatter-block");
    // Content change without frontmatter change: verbose mode should NOT
    // emit the `Frontmatter Visual Diff:` block, only the body visual diff.
    let base = "# Title\n\nHello\n";
    let updated = "# Title\n\nGoodbye\n";
    let out = delta_stdout(&fixture, base, updated, true);

    assert!(
        !out.contains("Frontmatter Visual Diff:"),
        "verbose should not emit frontmatter visual diff when frontmatter is unchanged:\n{out}"
    );
    assert!(
        out.contains("Statistics:\n  Bytes: 15 \u{2192} 17 (2 changed)\n"),
        "expected statistics block for content-only change, got:\n{out}"
    );
    // Body visual diff is emitted.
    assert!(
        out.contains("+         3 \u{2502} Goodbye\n"),
        "expected `Goodbye` body diff, got:\n{out}"
    );
}
