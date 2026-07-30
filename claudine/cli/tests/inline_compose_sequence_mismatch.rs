//! Integration tests for `claudine inline-compose` rejecting documents that
//! author **both** a non-null `prompt` and a non-null `sequence` (an inline
//! sequence). Such a document must run with `claudine sequence`, not
//! `inline-compose`.
//!
//! Covers the externally observable contract for the inline-compose /
//! sequence mismatch (spec criteria 1-10, 12, 15). These are **L1** process
//! tests: `assert_cmd` pipes stderr (never a TTY), so every run exercises the
//! non-TTY (YAML-withheld) branch. The real-terminal TTY full-diagnostic path
//! (criteria 11, 13, 14 — verbatim YAML, SGR styling, OSC 8 link) is covered
//! by the L1 PTY tests in `level1_inline_compose_mismatch_pty.rs`; the plain
//! readability contract (criterion 16) is covered both here (raw stderr has no
//! escape byte) and by the L1 render tests in
//! `claudine/lib/src/composition/error.rs`.

use std::fs;
use tempfile::tempdir;
mod common;
use common::strip_ansi;
#[cfg(unix)]
use common::{augmented_path, write_executable};

/// The unique marker the mismatch diagnostic always emits, regardless of TTY
/// state. Negative-path tests assert its **absence** to prove the document
/// took the ordinary code path instead.
const MISMATCH_MARKER: &str = "configured as a sequence";

/// Collapse every run of whitespace (including the diagnostic's word-wrap
/// newlines and the bordered-block `┃ ` prefixes) to a single space so
/// substring assertions are insensitive to wrap width.
fn normalize_ws(input: &str) -> String {
    strip_ansi(input)
        .replace('┃', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run `claudine inline-compose <doc>` (no provider, piped stderr) and return
/// the **raw** stderr bytes plus whether the process failed. Piped stderr is
/// never a TTY, so this always exercises the non-TTY (YAML-withheld) branch.
fn run_inline_compose_raw(md_file: &std::path::Path) -> (Vec<u8>, bool) {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["inline-compose", md_file.to_str().unwrap()])
        .assert();
    let output = assert.get_output();
    let failed = !output.status.success();
    (output.stderr.clone(), failed)
}

/// Run `claudine inline-compose <doc>` and return the whitespace-normalized
/// stderr plus whether the process failed.
fn run_inline_compose(md_file: &std::path::Path) -> (String, bool) {
    let (raw, failed) = run_inline_compose_raw(md_file);
    (normalize_ws(&String::from_utf8_lossy(&raw)), failed)
}

/// Assert that `plain` is the mismatch diagnostic: it names `prompt`,
/// `sequence`, and the `claudine sequence` directive.
fn assert_is_mismatch_diagnostic(plain: &str) {
    assert!(
        plain.contains(MISMATCH_MARKER),
        "expected the mismatch diagnostic; stderr:\n{plain}"
    );
    assert!(
        plain.contains("prompt") && plain.contains("sequence"),
        "diagnostic should name both `prompt` and `sequence`; stderr:\n{plain}"
    );
    assert!(
        plain.contains("claudine sequence"),
        "diagnostic should direct the user to `claudine sequence`; stderr:\n{plain}"
    );
}

// ============================================================================
// Rejection cases (criteria 1, 2, 3)
// ============================================================================

#[test]
fn rejects_string_prompt_with_nonempty_list_sequence() {
    // Criterion 1: valid string prompt + nonempty list sequence.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Do something\nsequence:\n  - name: Hello\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "mismatch must exit nonzero; stderr:\n{plain}");
    assert_is_mismatch_diagnostic(&plain);
}

#[test]
fn rejects_empty_list_sequence() {
    // Criterion 2: `sequence: []` is non-null.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Do something\nsequence: []\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "mismatch must exit nonzero; stderr:\n{plain}");
    assert_is_mismatch_diagnostic(&plain);
}

#[test]
fn rejects_scalar_sequence() {
    // Criterion 3: prompt + scalar (wrong-type, non-null) sequence.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Do something\nsequence: nope\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "mismatch must exit nonzero; stderr:\n{plain}");
    assert_is_mismatch_diagnostic(&plain);
}

#[test]
fn rejects_mapping_sequence() {
    // Criterion 3: prompt + mapping (wrong-type, non-null) sequence.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Do something\nsequence:\n  key: value\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "mismatch must exit nonzero; stderr:\n{plain}");
    assert_is_mismatch_diagnostic(&plain);
}

// ============================================================================
// Wrong-type but non-null `prompt` + non-null sequence (criterion 7)
//
// The mismatch tests authored non-null values, NOT type validity, so an empty,
// numeric, collection, or mapping `prompt` must still produce the mismatch
// diagnostic — observably, before any prompt-shape validation runs.
// ============================================================================

/// Assert `claudine inline-compose <frontmatter>` rejects the document with the
/// mismatch diagnostic (criterion 7). `frontmatter` is the YAML body between
/// the `---` delimiters.
fn assert_prompt_variant_rejected(frontmatter: &str) {
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, format!("---\n{frontmatter}\n---\nbody\n")).unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "mismatch must exit nonzero; stderr:\n{plain}");
    assert_is_mismatch_diagnostic(&plain);
}

#[test]
fn rejects_empty_string_prompt_with_sequence() {
    assert_prompt_variant_rejected("prompt: \"\"\nsequence:\n  - name: Hello");
}

#[test]
fn rejects_numeric_prompt_with_sequence() {
    assert_prompt_variant_rejected("prompt: 42\nsequence:\n  - name: Hello");
}

#[test]
fn rejects_collection_prompt_with_sequence() {
    assert_prompt_variant_rejected("prompt:\n  - a\n  - b\nsequence: []");
}

#[test]
fn rejects_mapping_prompt_with_sequence() {
    assert_prompt_variant_rejected("prompt:\n  key: value\nsequence: []");
}

// ============================================================================
// Non-TTY output: YAML withheld (criterion 12)
// ============================================================================

#[test]
fn non_tty_withholds_yaml_but_keeps_guidance() {
    // Criterion 12: under piped stderr the diagnostic retains the mismatch +
    // `sections` guidance and omits the authored YAML block entirely (the
    // frontmatter excerpt is TTY-gated). The unique frontmatter token must not
    // leak.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\n# secret-frontmatter-token\nprompt: Do something\nsequence: []\n---\nbody\n",
    )
    .unwrap();

    let (raw, failed) = run_inline_compose_raw(&md_file);
    assert!(failed, "mismatch must exit nonzero");

    // Criterion 16: redirected (non-TTY) output must contain no terminal
    // escape byte at all — neither SGR styling nor an OSC 8 hyperlink may leak
    // into a log or pipe. Asserted on the RAW bytes, before any normalization.
    assert!(
        !raw.contains(&0x1b),
        "non-TTY stderr must contain no escape byte; raw stderr:\n{}",
        String::from_utf8_lossy(&raw)
    );

    let plain = normalize_ws(&String::from_utf8_lossy(&raw));
    assert_is_mismatch_diagnostic(&plain);
    assert!(
        plain.contains("sections"),
        "non-TTY output should keep the `sections` guidance; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("secret-frontmatter-token"),
        "the authored YAML must not leak under non-TTY; stderr:\n{plain}"
    );
}

// ============================================================================
// Negative paths: existing behavior preserved (criteria 6, 8, 9)
// ============================================================================

#[test]
fn sequence_without_prompt_yields_missing_prompt_not_mismatch() {
    // Criterion 6: non-null sequence + no prompt → existing missing-prompt
    // behavior, NOT the mismatch.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nsequence:\n  - name: Hello\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "missing prompt must exit nonzero; stderr:\n{plain}");
    assert!(
        !plain.contains(MISMATCH_MARKER),
        "must NOT be the mismatch diagnostic; stderr:\n{plain}"
    );
    assert!(
        plain.contains("prompt"),
        "expected a prompt-related error; stderr:\n{plain}"
    );
}

#[test]
fn null_prompt_with_sequence_yields_existing_prompt_error_not_mismatch() {
    // Criterion 8: `prompt: null` + non-null sequence → existing null-prompt
    // behavior, NOT the mismatch.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: null\nsequence:\n  - name: Hello\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(failed, "null prompt must exit nonzero; stderr:\n{plain}");
    assert!(
        !plain.contains(MISMATCH_MARKER),
        "must NOT be the mismatch diagnostic; stderr:\n{plain}"
    );
    assert!(
        plain.contains("prompt"),
        "expected a prompt-related error; stderr:\n{plain}"
    );
}

#[test]
fn malformed_frontmatter_keeps_parse_precedence_not_mismatch() {
    // Criterion 9: a document whose frontmatter cannot parse must surface the
    // frontmatter-parse diagnostic; the mismatch check never runs.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    // `prompt: [unclosed` is an unterminated YAML flow sequence.
    fs::write(
        &md_file,
        "---\nprompt: [unclosed\nsequence: []\n---\nbody\n",
    )
    .unwrap();

    let (plain, failed) = run_inline_compose(&md_file);
    assert!(
        failed,
        "malformed frontmatter must exit nonzero; stderr:\n{plain}"
    );
    assert!(
        !plain.contains(MISMATCH_MARKER),
        "FrontmatterParse must keep precedence over the mismatch; stderr:\n{plain}"
    );
    // Positive identity: the existing frontmatter-parse diagnostic must be the
    // one surfaced (criterion 9), not merely the mismatch's absence.
    assert!(
        plain.contains("frontmatter parse failed"),
        "expected the frontmatter-parse diagnostic to be retained; stderr:\n{plain}"
    );
}

// ============================================================================
// Negative paths requiring a provider stub (criteria 4, 5)
// ============================================================================

#[cfg(unix)]
fn write_goose_stub(bin_dir: &std::path::Path) {
    // Minimal Goose stub: ignore the delivered prompt, emit a replacement
    // body on stdout, and succeed. inline-compose rewrites the doc body with
    // this output.
    write_executable(
        &bin_dir.join("goose"),
        "#!/bin/sh\ncat > /dev/null 2>&1\necho 'composed replacement body'\nexit 0\n",
    );
}

#[cfg(unix)]
#[test]
fn prompt_with_null_sequence_proceeds_to_ordinary_behavior() {
    // Criterion 4: `prompt` + `sequence: null` is not a mismatch — it proceeds
    // to ordinary inline-compose behavior and runs the provider.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_goose_stub(&bin_dir);

    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Do something\nsequence: null\n---\nbody\n",
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let plain = normalize_ws(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        !plain.contains(MISMATCH_MARKER),
        "`sequence: null` must NOT trigger the mismatch; stderr:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn prompt_without_sequence_retains_inline_behavior() {
    // Criterion 5: `prompt` + no sequence key → ordinary inline-compose.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_goose_stub(&bin_dir);

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, "---\nprompt: Do something\n---\nbody\n").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let plain = normalize_ws(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        !plain.contains(MISMATCH_MARKER),
        "a prompt-only doc must NOT trigger the mismatch; stderr:\n{plain}"
    );
}

// ============================================================================
// Overrides neither create nor suppress detection (criterion 10)
// ============================================================================

#[test]
fn override_cannot_suppress_authored_mismatch() {
    // Criterion 10: detection reads authored frontmatter only — a `--set`
    // override that nulls `sequence` must NOT suppress the mismatch.
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nprompt: Do something\nsequence: []\n---\nbody\n",
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args([
            "inline-compose",
            "--set",
            r#"{"sequence":null}"#,
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let plain = normalize_ws(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert_is_mismatch_diagnostic(&plain);
}

#[cfg(unix)]
#[test]
fn override_cannot_create_mismatch() {
    // Criterion 10: a `--set` override that adds `sequence` to a prompt-only
    // document must NOT create a mismatch — the run proceeds normally.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_goose_stub(&bin_dir);

    let md_file = workspace.path().join("doc.md");
    fs::write(&md_file, "---\nprompt: Do something\n---\nbody\n").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args([
            "inline-compose",
            "--goose",
            "--set",
            r#"{"sequence":["a"]}"#,
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let plain = normalize_ws(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        !plain.contains(MISMATCH_MARKER),
        "an override-added `sequence` must NOT create a mismatch; stderr:\n{plain}"
    );
}

// ============================================================================
// Side-effect freedom (criterion 15)
// ============================================================================

#[cfg(unix)]
#[test]
fn rejection_has_no_side_effects() {
    // Criterion 15: the rejection precedes every side-effect surface. No shell
    // directive runs, no provider launches, and the source file is unchanged.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let shell_sentinel = workspace.path().join("shell-ran.flag");
    let provider_sentinel = workspace.path().join("provider-ran.flag");

    // Provider stub records any launch so we can prove it was never invoked.
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho launched > {marker}\necho body\nexit 0\n",
            marker = provider_sentinel.display()
        ),
    );

    // Mismatch doc whose body also embeds a shell directive that would touch a
    // sentinel if composition (which never runs) reached it.
    let md_file = workspace.path().join("doc.md");
    let original = format!(
        "---\nprompt: Do something\nsequence: []\n---\n::shell touch {sentinel}\n",
        sentinel = shell_sentinel.display()
    );
    fs::write(&md_file, &original).unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args(["inline-compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let plain = normalize_ws(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert_is_mismatch_diagnostic(&plain);

    assert!(
        !shell_sentinel.exists(),
        "no shell directive should have run before rejection"
    );
    assert!(
        !provider_sentinel.exists(),
        "no provider should have launched before rejection"
    );

    let after = fs::read_to_string(&md_file).unwrap();
    assert_eq!(
        after, original,
        "the source document must be byte-for-byte unchanged (no body rewrite, no `last_updated` bump)"
    );
}
