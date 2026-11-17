use std::collections::HashSet;

use super::*;

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

fn hoisted(tokens: &[&str]) -> Vec<String> {
    hoist_wrapper_options(argv(tokens))
        .unwrap()
        .into_iter()
        .map(|token| token.into_string().unwrap())
        .collect()
}

/// Every value-taking spelling a wrapper owns. A new `WrapperArgs` option
/// fails [`the_wrapper_surface_is_the_clap_surface`] until it is listed here,
/// and is then covered by the position tests below.
const VALUE_SPELLINGS: &[&str] = &[
    "--debug",
    "--include",
    "--model",
    "-m",
    "--output",
    "-o",
    "--append-system-prompt",
    "--asp",
    "--replace-system-prompt",
    "--rsp",
    "--timeout",
    "-t",
    "--step-timeout",
    "--stall-timeout",
    "--operation",
    "--op",
    "--use",
];

/// Every boolean spelling a wrapper owns.
const BOOL_SPELLINGS: &[&str] = &[
    "--verbose", "-v", "--plain", "--help", "-h", "--yolo", "-y", "--interactive", "-i", "--edit", "--dry-run",
    "--quiet", "-q", "--silent", "--sandbox", "--repo", "--mcp", "--strict", "--perf",
];

#[test]
fn the_wrapper_surface_is_the_clap_surface() {
    let owned = OwnedFlags::for_wrapper();
    let set = |spellings: &[&str]| spellings.iter().map(|s| s.to_string()).collect::<HashSet<_>>();
    assert_eq!(owned.value_flags_for_tests(), &set(VALUE_SPELLINGS), "value-taking wrapper options");
    assert_eq!(owned.bool_flags_for_tests(), &set(BOOL_SPELLINGS), "boolean wrapper options");
}

#[test]
fn every_value_option_after_the_prompt_moves_ahead_of_it_in_each_spelling() {
    for spelling in VALUE_SPELLINGS {
        assert_eq!(
            hoisted(&["claudine", "claude", "hi", spelling, "v", "--native"]),
            ["claudine", "claude", spelling, "v", "hi", "--native"],
            "{spelling} v"
        );
        if spelling.starts_with("--") {
            let attached = format!("{spelling}=v");
            assert_eq!(
                hoisted(&["claudine", "claude", "hi", &attached]),
                ["claudine", "claude", attached.as_str(), "hi"],
                "{attached}"
            );
        } else {
            // The lenient wrapper parse would drop `-mv` into the
            // passthrough, so an attached short value is split.
            let attached = format!("{spelling}v");
            assert_eq!(
                hoisted(&["claudine", "claude", "hi", &attached]),
                ["claudine", "claude", spelling, "v", "hi"],
                "{attached}"
            );
            assert_eq!(
                hoisted(&["claudine", "claude", &attached, "hi"]),
                ["claudine", "claude", spelling, "v", "hi"],
                "{attached} before the prompt"
            );
        }
    }
}

#[test]
fn every_boolean_option_after_the_prompt_moves_ahead_of_it() {
    for spelling in BOOL_SPELLINGS {
        assert_eq!(
            hoisted(&["claudine", "goose", "--native", "hi", spelling]),
            ["claudine", "goose", spelling, "--native", "hi"],
            "{spelling}"
        );
    }
}

#[test]
fn a_short_cluster_of_owned_options_is_split_into_its_options() {
    assert_eq!(hoisted(&["claudine", "codex", "hi", "-yq"]), ["claudine", "codex", "-y", "-q", "hi"]);
    assert_eq!(hoisted(&["claudine", "codex", "-yojson", "hi"]), ["claudine", "codex", "-y", "-o", "json", "hi"]);
    assert_eq!(hoisted(&["claudine", "codex", "hi", "-yo", "json"]), ["claudine", "codex", "-y", "-o", "json", "hi"]);
    assert_eq!(hoisted(&["claudine", "codex", "hi", "-yz"]), ["claudine", "codex", "hi", "-yz"], "-z is not Claudine's");
}

#[test]
fn options_keep_their_written_order_and_provider_tokens_keep_theirs() {
    assert_eq!(
        hoisted(&["claudine", "claude", "--foo", "hi", "-o", "json", "--bar", "x", "--dry-run"]),
        ["claudine", "claude", "-o", "json", "--dry-run", "--foo", "hi", "--bar", "x"]
    );
}

#[test]
fn nothing_at_or_after_the_first_separator_is_read() {
    assert_eq!(
        hoisted(&["claudine", "claude", "hi", "-m", "x", "--", "--output", "json", "--help", "--", "-y"]),
        ["claudine", "claude", "-m", "x", "hi", "--", "--output", "json", "--help", "--", "-y"]
    );
    assert_eq!(
        hoisted(&["claudine", "claude", "--", "hi", "--output", "json"]),
        ["claudine", "claude", "--", "hi", "--output", "json"]
    );
}

#[test]
fn a_value_option_without_a_value_before_the_boundary_is_refused() {
    for (tokens, option) in [
        (&["claudine", "claude", "hi", "-m"][..], "-m"),
        (&["claudine", "claude", "hi", "--operation", "--", "x"], "--operation"),
        (&["claudine", "claude", "--op"], "--op"),
    ] {
        let error = hoist_wrapper_options(argv(tokens)).unwrap_err();
        assert_eq!(error.option, option, "{tokens:?}");
        assert!(error.to_string().contains("missing value"), "{error}");
    }
}

#[test]
fn root_options_that_are_not_global_stay_with_the_provider() {
    assert_eq!(hoisted(&["claudine", "claude", "hi", "--version"]), ["claudine", "claude", "hi", "--version"]);
}

#[test]
fn non_wrapper_argv_is_unchanged() {
    assert_eq!(hoisted(&["claudine", "hooks", "list", "-v"]), ["claudine", "hooks", "list", "-v"]);
}
