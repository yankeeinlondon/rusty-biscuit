//! Integration tests for the `claudine hooks` command family.
//!
//! These tests guard the migration of hook-related output from atomic
//! Prose tokens (`{{bold}}`, `{{reset}}`, …) to bracketed tags. The
//! plain-text suite runs the real binary with `NO_COLOR=1` and asserts on
//! rendered content; the Level-1 ANSI suite runs it with `FORCE_COLOR=1`
//! and asserts the migrated tags emit real SGR styling with no raw tag
//! markup left behind.

use std::fs;


mod common;
use common::TestWorkspace;

/// Atomic style tokens that must no longer appear in any hooks output.
const ATOMIC_STYLE_TOKENS: &[&str] = &[
    "{{bold}}",
    "{{dim}}",
    "{{italic}}",
    "{{reset}}",
    "{{red}}",
    "{{green}}",
    "{{yellow}}",
    "{{blue}}",
    "{{cyan}}",
    "{{magenta}}",
    "{{strikethrough}}",
    "{{normal-font-weight}}",
    "{{not-italic}}",
];

/// Run `claudine` with `NO_COLOR` set and an isolated `HOME`, returning
/// stdout as a string. Asserts the command exited successfully.
fn run_hooks(home: &std::path::Path, args: &[&str]) -> String {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .args(args)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf-8")
}

/// Assert that no atomic style token leaked into rendered output.
fn assert_no_atomic_tokens(output: &str) {
    for token in ATOMIC_STYLE_TOKENS {
        assert!(
            !output.contains(token),
            "atomic style token `{token}` leaked into hooks output:\n{output}"
        );
    }
}

/// The support matrix uses a single glyph vocabulary; every glyph a cell
/// can show must also appear in the legend, and the retired glyphs
/// (⛔️ non-hook, ❌ none) must not resurface.
#[test]
fn hooks_support_legend_documents_glyph_vocabulary() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_hooks(&home, &["hooks", "--support"]);

    for glyph in ["✅", "🔶", "🅐", "–"] {
        assert!(
            output.contains(glyph),
            "support view should render the {glyph} glyph:\n{output}"
        );
    }
    for retired in ["⛔", "❌"] {
        assert!(
            !output.contains(retired),
            "support view resurrected retired glyph {retired}:\n{output}"
        );
    }
    assert!(
        !output.contains("Table could not be rendered"),
        "support view refused to render instead of chunking:\n{output}"
    );
    assert!(
        output.contains("Not mappable — configure natively"),
        "support view is missing the unmapped native events note:\n{output}"
    );
    assert!(
        output.contains("BeforeToolSelection"),
        "unmapped note is missing Gemini BeforeToolSelection:\n{output}"
    );
    assert_no_atomic_tokens(&output);
}

/// `--mapping` closes with the same unmapped-events list: these phases
/// exist natively but have no canonical row in the tables above.
#[test]
fn hooks_mapping_lists_unmapped_native_events() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_hooks(&home, &["hooks", "--mapping"]);

    assert!(
        output.contains("Not mappable — configure natively"),
        "mapping view is missing the unmapped native events note:\n{output}"
    );
    assert!(
        output.contains("tool.definition"),
        "unmapped note is missing OpenCode tool.definition:\n{output}"
    );
    assert_no_atomic_tokens(&output);
}

/// The template-variables view must preserve literal `{{...}}` placeholders
/// (template syntax) as display text while still applying styling.
#[test]
fn hooks_variables_preserves_literal_template_placeholders() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_hooks(&home, &["hooks", "--variables"]);

    for placeholder in ["{{tool_name}}", "{{git.branch}}", "{{error}}"] {
        assert!(
            output.contains(placeholder),
            "variables output dropped literal placeholder `{placeholder}`:\n{output}"
        );
    }
    assert_no_atomic_tokens(&output);
}

/// Command routing remains testable without redirecting the native user home.
#[test]
fn hooks_command_help_routes_without_user_config() {
    assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .args(["hooks", "--help"])
        .assert()
        .success();
}

/// None of the static `hooks` views may emit atomic style tokens.
#[test]
fn hooks_views_emit_no_atomic_style_tokens() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    for view in [
        "--support",
        "--mapping",
        "--describe",
        "--variables",
        "--capture-method",
    ] {
        let output = run_hooks(&home, &["hooks", view]);
        assert_no_atomic_tokens(&output);
    }
}

// ── Level-1 ANSI styling verification ────────────────────────────────
//
// The tests above run with `NO_COLOR=1` and assert on plain text. These
// run the binary with `FORCE_COLOR=1` so the migrated bracketed tags
// actually emit ANSI SGR sequences, verifying that the tag grammar both
// (a) produces real styling and (b) leaves no raw tag markup in output.

/// Run `claudine` with color forced on, returning stdout.
fn run_hooks_colored(home: &std::path::Path, args: &[&str]) -> String {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", home)
        .env("FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .args(args)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf-8")
}

/// Raw bracketed style tags that must never survive into rendered output.
const RAW_STYLE_TAGS: &[&str] = &[
    "<dim>",
    "</dim>",
    "<bold>",
    "</bold>",
    "<blue>",
    "</blue>",
    "<cyan>",
    "</cyan>",
    "<red>",
    "</red>",
    "<green>",
    "</green>",
    "<yellow>",
    "</yellow>",
];

/// Assert rendered output emits ANSI styling and leaks no raw tag markup.
fn assert_styled(output: &str) {
    assert!(
        output.contains('\x1b'),
        "expected ANSI SGR escapes in colored output:\n{output:?}"
    );
    assert!(
        output.contains("\x1b[0m"),
        "expected at least one SGR reset in colored output:\n{output:?}"
    );
    for tag in RAW_STYLE_TAGS {
        assert!(
            !output.contains(tag),
            "raw bracketed tag `{tag}` leaked into rendered output:\n{output:?}"
        );
    }
    assert_no_atomic_tokens(output);
}

#[test]
fn hooks_support_view_emits_ansi_styling() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_hooks_colored(&home, &["hooks", "--support"]);
    assert_styled(&output);
    assert!(
        output.contains('✅'),
        "support legend missing glyph:\n{output:?}"
    );
}

/// `--capture-method` is a hidden alias of `--support`; it must keep
/// rendering the styled support matrix so existing invocations don't break.
#[test]
fn hooks_capture_method_view_emits_ansi_styling() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_hooks_colored(&home, &["hooks", "--capture-method"]);
    assert_styled(&output);
    assert!(
        output.contains("✅"),
        "alias output should be the support matrix:\n{output:?}"
    );
}

#[test]
fn hooks_variables_view_styles_and_keeps_template_placeholders() {
    let workspace = TestWorkspace::named("claudine-hooks-it");
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_hooks_colored(&home, &["hooks", "--variables"]);
    assert_styled(&output);
    // Literal template placeholders must survive styled rendering.
    for placeholder in ["{{tool_name}}", "{{git.branch}}", "{{error}}"] {
        assert!(
            output.contains(placeholder),
            "styled variables output dropped placeholder `{placeholder}`:\n{output:?}"
        );
    }
}
