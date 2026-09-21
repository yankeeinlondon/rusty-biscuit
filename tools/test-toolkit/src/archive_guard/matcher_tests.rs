//! Fixture corpus for the token-aware matcher.
//!
//! Kept out of `archive_guard.rs` because every fixture below necessarily
//! *spells* a forbidden form. This file is one of [`GUARD_OWN_SOURCES`], so the
//! guard never scans it.

use super::{
    BIN_EXE_REMEDY, HOSTED_ROOT_REMEDY, MANIFEST_REMEDY, raw_forms, violations_in,
};

/// Every violation as `(line, remedy)`, ordered by line.
fn found(source: &str) -> Vec<(usize, &'static str)> {
    violations_in(source)
}

fn lines(source: &str) -> Vec<usize> {
    found(source).into_iter().map(|(line, _)| line).collect()
}

// ---------------------------------------------------------------------------
// Runtime-first manifest fallbacks
// ---------------------------------------------------------------------------

#[test]
fn a_runtime_first_manifest_fallback_is_exempt() {
    let source = r#"
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}
"#;

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn the_map_or_else_spelling_of_the_same_fallback_is_exempt() {
    let source = r#"
fn fixture_root() -> PathBuf {
    env::var_os("CARGO_MANIFEST_DIR")
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}
"#;

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn a_fallback_that_does_not_reject_empty_values_is_a_violation() {
    // An exported-but-empty variable is how a shell spells "unset"; accepting
    // it disagrees with `biscuit_test_harness::manifest_dir!`.
    let source = r#"
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}
"#;

    assert_eq!(found(source), vec![(5, MANIFEST_REMEDY)]);
}

#[test]
fn a_runtime_read_elsewhere_in_the_file_grants_no_exemption() {
    let source = r#"
fn observed() -> Option<OsString> {
    std::env::var_os("CARGO_MANIFEST_DIR").filter(|value| !value.is_empty())
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
"#;

    assert_eq!(found(source), vec![(7, MANIFEST_REMEDY)]);
}

#[test]
fn a_runtime_read_of_a_different_variable_grants_no_exemption() {
    let source = r#"
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}
"#;

    assert_eq!(found(source), vec![(6, MANIFEST_REMEDY)]);
}

#[test]
fn an_unused_runtime_read_grants_no_exemption() {
    // The read happens, its value is dropped, and the compile-time path is what
    // the expression actually returns.
    let source = r#"
fn fixture_root() -> PathBuf {
    let _ = std::env::var_os("CARGO_MANIFEST_DIR").filter(|value| !value.is_empty());
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
"#;

    assert_eq!(found(source), vec![(4, MANIFEST_REMEDY)]);
}

#[test]
fn a_second_unguarded_occurrence_in_an_otherwise_exempt_file_is_reported_once() {
    let source = r#"
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn sibling_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
"#;

    assert_eq!(found(source), vec![(10, MANIFEST_REMEDY)]);
}

#[test]
fn an_unsupported_custom_helper_is_reported_with_the_shared_macro_as_the_remedy() {
    let source = r#"
fn fixture_root() -> PathBuf {
    my_helper::pick_checkout("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"))
}
"#;

    assert_eq!(found(source), vec![(3, MANIFEST_REMEDY)]);
}

// ---------------------------------------------------------------------------
// Runtime-first binary fallbacks
// ---------------------------------------------------------------------------

#[test]
fn a_runtime_first_binary_fallback_in_the_shared_order_is_exempt() {
    let source = r#"
fn tool() -> PathBuf {
    std::env::var_os("NEXTEST_BIN_EXE_so_you_say")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("CARGO_BIN_EXE_so-you-say").filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_so-you-say")))
}
"#;

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn a_binary_fallback_naming_a_different_binary_than_the_runtime_read_is_a_violation() {
    let source = r#"
fn tool() -> PathBuf {
    std::env::var_os("NEXTEST_BIN_EXE_other_bin")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("CARGO_BIN_EXE_other-bin").filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_so-you-say")))
}
"#;

    assert_eq!(found(source), vec![(7, BIN_EXE_REMEDY)]);
}

#[test]
fn a_binary_fallback_checking_cargos_variable_before_nextests_is_a_violation() {
    // Nextest republishes the extracted location under `NEXTEST_BIN_EXE_*`;
    // reading Cargo's verbatim variable first can win with the producer's path.
    let source = r#"
fn tool() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_so-you-say")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("NEXTEST_BIN_EXE_so_you_say").filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_so-you-say")))
}
"#;

    assert_eq!(found(source), vec![(7, BIN_EXE_REMEDY)]);
}

#[test]
fn an_unrelated_nextest_variable_does_not_exempt_another_executable() {
    let source = r#"
fn tool() -> PathBuf {
    let _ = std::env::var_os("NEXTEST_BIN_EXE_so_you_say");
    PathBuf::from(env!("CARGO_BIN_EXE_md"))
}
"#;

    assert_eq!(found(source), vec![(4, BIN_EXE_REMEDY)]);
}

#[test]
fn a_binary_fallback_missing_the_empty_value_rejection_is_a_violation() {
    let source = r#"
fn tool() -> PathBuf {
    std::env::var_os("NEXTEST_BIN_EXE_md")
        .or_else(|| std::env::var_os("CARGO_BIN_EXE_md"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_md")))
}
"#;

    assert_eq!(found(source), vec![(6, BIN_EXE_REMEDY)]);
}

// ---------------------------------------------------------------------------
// Token awareness
// ---------------------------------------------------------------------------

#[test]
fn a_multiline_invocation_is_matched() {
    let source = "fn root() -> &'static str {\n    env!(\n        \"CARGO_MANIFEST_DIR\"\n    )\n}\n";

    assert_eq!(found(source), vec![(2, MANIFEST_REMEDY)]);
}

#[test]
fn whitespace_between_the_macro_name_and_its_arguments_is_matched() {
    let source = "fn root() -> &'static str {\n    env !  ( \"CARGO_MANIFEST_DIR\" )\n}\n";

    assert_eq!(found(source), vec![(2, MANIFEST_REMEDY)]);
}

#[test]
fn the_form_inside_a_line_comment_is_not_a_violation() {
    let source = "// never write env!(\"CARGO_MANIFEST_DIR\") in a test\n/// nor env!(\"CARGO_BIN_EXE_md\")\nfn main() {}\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn the_form_inside_a_block_comment_is_not_a_violation() {
    let source = "/* env!(\"CARGO_MANIFEST_DIR\") is the trap */\nfn main() {}\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn the_form_inside_a_nested_block_comment_is_not_a_violation() {
    let source =
        "/* outer /* env!(\"CARGO_MANIFEST_DIR\") */ still commented */\nfn main() {}\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn a_nested_block_comment_does_not_swallow_the_code_after_it() {
    // The trap the nesting rule exists for: a single-level scanner would end
    // the comment at the inner `*/` and then miss (or mis-place) what follows.
    let source =
        "/* outer /* inner */ still commented */\nfn root() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }\n";

    assert_eq!(found(source), vec![(2, MANIFEST_REMEDY)]);
}

#[test]
fn the_form_inside_a_string_literal_is_not_a_violation() {
    let source = "fn doc() -> &'static str {\n    \"env!(\\\"CARGO_MANIFEST_DIR\\\")\"\n}\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn the_form_inside_a_raw_string_is_not_a_violation() {
    let source = "fn fixture() -> &'static str {\n    r#\"env!(\"CARGO_MANIFEST_DIR\")\"#\n}\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn a_raw_string_with_many_hashes_is_not_a_violation_and_does_not_swallow_later_code() {
    let source = "fn fixture() -> &'static str {\n    r###\"env!(\"CARGO_BIN_EXE_md\")\"###\n}\nfn root() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }\n";

    assert_eq!(found(source), vec![(4, MANIFEST_REMEDY)]);
}

#[test]
fn a_char_literal_holding_a_quote_does_not_desynchronize_the_lexer() {
    let source = "fn quote() -> char { '\"' }\nfn root() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }\n";

    assert_eq!(found(source), vec![(2, MANIFEST_REMEDY)]);
}

#[test]
fn a_lifetime_is_not_mistaken_for_a_char_literal() {
    let source = "struct Holder<'a> { name: &'a str }\nfn root() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }\n";

    assert_eq!(found(source), vec![(2, MANIFEST_REMEDY)]);
}

#[test]
fn an_escaped_backslash_ending_a_string_does_not_desynchronize_the_lexer() {
    let source = "const SEP: &str = \"\\\\\";\nfn root() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }\n";

    assert_eq!(found(source), vec![(2, MANIFEST_REMEDY)]);
}

#[test]
fn an_env_lookup_of_an_unrelated_variable_is_not_a_violation() {
    let source = "fn version() -> &'static str { env!(\"CARGO_PKG_VERSION\") }\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn every_reported_line_is_the_line_the_invocation_starts_on() {
    let source = "fn a() -> &'static str { env!(\"CARGO_MANIFEST_DIR\") }\n\n\n\nfn b() -> &'static str { env!(\"CARGO_BIN_EXE_md\") }\n";

    assert_eq!(lines(source), vec![1, 5]);
}

// ---------------------------------------------------------------------------
// The hosted-root check stays independent
// ---------------------------------------------------------------------------

#[test]
fn a_hosted_root_literal_is_reported_even_beside_a_valid_runtime_fallback() {
    let source = r#"
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

const PRODUCER: &str = "/home/runner/work/rusty-biscuit/rusty-biscuit";
"#;

    assert_eq!(found(source), vec![(9, HOSTED_ROOT_REMEDY)]);
}

#[test]
fn a_hosted_root_literal_in_a_comment_is_not_a_violation() {
    let source = "// the producer checks out under \"/home/runner/work/rusty-biscuit\"\nfn main() {}\n";

    assert!(found(source).is_empty(), "{:?}", found(source));
}

#[test]
fn a_hosted_root_literal_in_a_raw_string_is_still_a_violation() {
    // A baked path is a string literal by definition, so unlike the macro
    // forms, this check must fire inside one.
    let source = "const PRODUCER: &str = r\"/home/runner/work/rusty-biscuit\";\n";

    assert_eq!(found(source), vec![(1, HOSTED_ROOT_REMEDY)]);
}

// ---------------------------------------------------------------------------
// Exemption maintenance reads raw forms, not violations
// ---------------------------------------------------------------------------

#[test]
fn a_safe_fallback_still_counts_as_a_raw_live_form() {
    // This mirrors `biscuit-test-harness/src/bin_exe.rs`: validating `ALLOWED`
    // against violations would declare its entry stale.
    let source = r#"
fn fixture_root() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}
"#;

    assert!(found(source).is_empty());
    assert_eq!(raw_forms(source).len(), 1);
}

#[test]
fn the_shared_harness_macro_body_is_a_raw_live_form() {
    let source = r#"
macro_rules! manifest_dir {
    () => {
        $crate::bin_exe::manifest_dir(env!("CARGO_MANIFEST_DIR"))
    };
}
"#;

    assert_eq!(raw_forms(source).len(), 1);
}
