//! Compose CLI parsing and helper unit tests.
//!
//! Split into thematically grouped sections so the file remains navigable
//! even as new coverage is added:
//!   * shorthand / setter / positional parsing
//!   * override merging
//!   * session interactivity resolution
//!   * SIGINT during prep (Unix-only)

use super::*;
#[cfg(unix)]
use super::interrupt::{format_user_interrupt_message, install_user_interrupt_guard};
use super::setters::{parse_compose_setter, parse_shorthand_value};
use serde_json::{Value, json};

fn s(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

// ── parse_shorthand_value ────────────────────────────────────────

#[test]
fn shorthand_value_empty_string() {
    assert_eq!(parse_shorthand_value(""), Value::String(String::new()));
}

#[test]
fn shorthand_value_number() {
    assert_eq!(parse_shorthand_value("3"), json!(3));
}

#[test]
fn shorthand_value_boolean() {
    assert_eq!(parse_shorthand_value("true"), json!(true));
}

#[test]
fn shorthand_value_array() {
    assert_eq!(parse_shorthand_value(r#"["a","b"]"#), json!(["a", "b"]));
}

#[test]
fn shorthand_value_object_json5() {
    assert_eq!(
        parse_shorthand_value(r#"{mode:"fast"}"#),
        json!({"mode": "fast"})
    );
}

#[test]
fn shorthand_value_plain_string_fallback() {
    assert_eq!(
        parse_shorthand_value("review.md"),
        Value::String("review.md".into())
    );
}

#[test]
fn shorthand_value_url_fallback() {
    assert_eq!(
        parse_shorthand_value("https://x/?a=b"),
        Value::String("https://x/?a=b".into())
    );
}

// ── parse_compose_setter ─────────────────────────────────────────

#[test]
fn setter_valid_string() {
    let res = parse_compose_setter("review=review.md").unwrap().unwrap();
    assert_eq!(res, ("review".into(), Value::String("review.md".into())));
}

#[test]
fn setter_underscore_key() {
    let res = parse_compose_setter("_private=true").unwrap().unwrap();
    assert_eq!(res, ("_private".into(), json!(true)));
}

#[test]
fn setter_hyphen_key() {
    let res = parse_compose_setter("my-key=value").unwrap().unwrap();
    assert_eq!(res, ("my-key".into(), Value::String("value".into())));
}

#[test]
fn setter_empty_value() {
    let res = parse_compose_setter("key=").unwrap().unwrap();
    assert_eq!(res, ("key".into(), Value::String("".into())));
}

#[test]
fn setter_first_eq_split() {
    let res = parse_compose_setter("url=https://x/?a=b").unwrap().unwrap();
    assert_eq!(res, ("url".into(), Value::String("https://x/?a=b".into())));
}

#[test]
fn setter_empty_key_errors() {
    let res = parse_compose_setter("=foo").unwrap();
    assert!(res.is_err());
}

#[test]
fn setter_digit_start_rejected() {
    assert!(parse_compose_setter("9key=value").is_none());
}

#[test]
fn setter_dot_path_rejected() {
    assert!(parse_compose_setter("foo.bar=baz").is_none());
}

#[test]
fn setter_slash_key_rejected() {
    assert!(parse_compose_setter("/path=val").is_none());
}

#[test]
fn setter_no_equals_rejected() {
    assert!(parse_compose_setter("file.md").is_none());
}

/// Tokens on both sides of the setter grammar's edges.
const SETTER_GRAMMAR_CORPUS: &[&str] = &[
    "phase=2", "_k=v", "a-b=", "k9_-=x=y", "K=", "a=b=c", "foo.bar=baz", "9key=v", "-k=v", "k k=v",
    "é=v", "kä=v", "/p=v", "=v", "=", "phase", "", "--config=x",
];

/// The CLI's setter parser reads the shared grammar
/// (`claudine::composition::setter_key`): it accepts exactly the tokens the
/// grammar does, splits at the same `=`, and differs only by rejecting an
/// empty key (`=v`), which the grammar reads as a bare word.
#[test]
fn the_setter_parser_and_ownership_share_one_key_grammar() {
    for token in SETTER_GRAMMAR_CORPUS {
        let parsed = parse_compose_setter(token);
        match claudine::composition::setter_key(token) {
            Some(key) => {
                let (parsed_key, _) = parsed.unwrap_or_else(|| panic!("`{token}`")).unwrap();
                assert_eq!(parsed_key, key, "`{token}`");
            }
            None if token.starts_with('=') => assert!(matches!(parsed, Some(Err(_))), "`{token}`"),
            None => assert!(parsed.is_none(), "`{token}`"),
        }
        assert_eq!(crate::argv::looks_like_setter(token), claudine::composition::setter_key(token).is_some(), "`{token}`");
    }
}

// ── parse_composition_positionals ────────────────────────────────

#[test]
fn positionals_file_only() {
    let parsed = parse_composition_positionals(&s(&["file.md"])).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
    assert!(parsed.shorthand_setters.is_empty());
}

#[test]
fn positionals_setter_only() {
    let parsed = parse_composition_positionals(&s(&["key=val"])).unwrap();
    assert!(parsed.file_ref.is_none());
    assert_eq!(
        parsed.shorthand_setters.get("key"),
        Some(&Value::String("val".into()))
    );
}

#[test]
fn positionals_file_then_setter() {
    let parsed = parse_composition_positionals(&s(&["file.md", "key=val"])).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
    assert_eq!(
        parsed.shorthand_setters.get("key"),
        Some(&Value::String("val".into()))
    );
}

#[test]
fn positionals_setter_then_file() {
    let parsed = parse_composition_positionals(&s(&["key=val", "file.md"])).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
    assert_eq!(
        parsed.shorthand_setters.get("key"),
        Some(&Value::String("val".into()))
    );
}

#[test]
fn positionals_multiple_setters_around_file() {
    let parsed = parse_composition_positionals(&s(&["a=1", "file.md", "b=2"])).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
    assert_eq!(parsed.shorthand_setters.get("a"), Some(&json!(1)));
    assert_eq!(parsed.shorthand_setters.get("b"), Some(&json!(2)));
}

#[test]
fn positionals_duplicate_setter_last_wins() {
    let parsed = parse_composition_positionals(&s(&["k=old", "k=new"])).unwrap();
    assert_eq!(
        parsed.shorthand_setters.get("k"),
        Some(&Value::String("new".into()))
    );
}

#[test]
fn positionals_after_the_file_are_collected_in_order() {
    let parsed = parse_composition_positionals(&s(&["a.md", "b.md", "k=v", "c", "b.md"])).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("a.md"));
    assert_eq!(parsed.positionals, s(&["b.md", "c", "b.md"]));
    assert_eq!(parsed.shorthand_setters.get("k"), Some(&Value::String("v".into())));
}

#[test]
fn positionals_reject_an_argv_setter() {
    for tokens in [&["file.md", "argv=a"][..], &["argv=[1]", "file.md"], &["file.md", "argv=null"]] {
        let err = parse_composition_positionals(&s(tokens)).unwrap_err().to_string();
        assert!(err.contains("reserved for positional arguments"), "{tokens:?}: {err}");
        assert!(err.contains("bare words"), "{err}");
    }
}

#[test]
fn positionals_empty_key_errors() {
    let err = parse_composition_positionals(&s(&["=foo"])).unwrap_err();
    assert!(err.to_string().contains("must not be empty"));
}

#[test]
fn positionals_dot_path_is_file_candidate() {
    let parsed = parse_composition_positionals(&s(&["foo.bar=baz"])).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("foo.bar=baz"));
    assert!(parsed.shorthand_setters.is_empty());
}

// ── merge_set_overrides ──────────────────────────────────────────

#[test]
fn merge_both_empty() {
    let result = merge_set_overrides(None, serde_json::Map::new(), Vec::new()).unwrap();
    assert!(result.is_none());
}

#[test]
fn merge_set_only() {
    let result = merge_set_overrides(Some(r#"{"a":"b"}"#), serde_json::Map::new(), Vec::new()).unwrap();
    assert_eq!(result, Some(json!({"a": "b"})));
}

#[test]
fn merge_shorthand_only() {
    let mut short = serde_json::Map::new();
    short.insert("k".into(), Value::String("v".into()));
    let result = merge_set_overrides(None, short, Vec::new()).unwrap();
    assert_eq!(result, Some(json!({"k": "v"})));
}

#[test]
fn merge_shorthand_wins() {
    let mut short = serde_json::Map::new();
    short.insert("k".into(), Value::String("new".into()));
    let result = merge_set_overrides(Some(r#"{"k":"old"}"#), short, Vec::new()).unwrap();
    assert_eq!(result, Some(json!({"k": "new"})));
}

#[test]
fn merge_disjoint() {
    let mut short = serde_json::Map::new();
    short.insert("b".into(), json!(2));
    let result = merge_set_overrides(Some(r#"{"a":"1"}"#), short, Vec::new()).unwrap();
    assert_eq!(result, Some(json!({"a": "1", "b": 2})));
}

#[test]
fn merge_sets_argv_from_positionals_only_when_there_are_some() {
    let result = merge_set_overrides(None, serde_json::Map::new(), s(&["alpha", "1", ""])).unwrap();
    // Strings, not JSON5-parsed: `1` stays a string; an empty word stays.
    assert_eq!(result, Some(json!({"argv": ["alpha", "1", ""]})));
    let result = merge_set_overrides(Some(r#"{"a":1}"#), serde_json::Map::new(), Vec::new()).unwrap();
    assert_eq!(result, Some(json!({"a": 1})), "no positionals leaves argv unset");
}

#[test]
fn merge_rejects_argv_in_set_json() {
    for raw in [r#"{"argv":["a"]}"#, r#"{"argv":null}"#] {
        let err = merge_set_overrides(Some(raw), serde_json::Map::new(), Vec::new()).unwrap_err().to_string();
        assert!(err.contains("reserved for positional arguments"), "{raw}: {err}");
    }
    // Rejected whether or not positionals are given.
    let err = merge_set_overrides(Some(r#"{"argv":"a"}"#), serde_json::Map::new(), s(&["b"])).unwrap_err();
    assert!(err.to_string().contains("reserved"), "{err}");
}

// ── Setters reclaimed after a provider switch ───────────────────
//
// A setter Claudine owns after a provider switch reaches the same override
// path as one before the file: partition, ownership, then
// `parse_composition_positionals` over the clap positionals followed by the
// owned tokens, and `merge_set_overrides`, as `run_composition_inner` does.

use claudine::composition::SchemaParameters;

/// The caller overrides and forwarded tokens of
/// `claudine compose {before} plan.md {after}` for Codex. `before` holds
/// setters only; a `--set` anywhere after the file is taken from the
/// partition's Claudine argv.
fn reclaimed(before: &[&str], after: &[&str], schema: &SchemaParameters) -> (Option<Value>, Vec<String>) {
    let mut line = vec!["claudine", "compose"];
    line.extend_from_slice(before);
    line.push("plan.md");
    line.extend_from_slice(after);
    let (claudine_argv, after_file) =
        crate::argv::partition_composition_tail(line.iter().map(std::ffi::OsString::from).collect()).unwrap();
    let claudine_argv: Vec<String> = claudine_argv
        .into_iter()
        .map(|token| token.into_string().unwrap())
        .collect();
    let set = claudine_argv
        .iter()
        .position(|token| token == "--set")
        .map(|at| claudine_argv[at + 1].clone());

    let candidates = [claudine::composition::OwnershipCandidate {
        provider: Provider::Codex,
        command_path: s(&["exec"]),
    }];
    let owned = claudine::composition::own_arguments(&after_file, schema, &candidates).unwrap();
    let clap_positionals: Vec<String> = before.iter().chain(&["plan.md"]).map(|token| token.to_string()).collect();
    let parsed = parse_composition_positionals(&[clap_positionals, owned.claudine].concat()).unwrap();
    assert_eq!(parsed.file_ref.as_deref(), Some("plan.md"));
    let overrides = merge_set_overrides(set.as_deref(), parsed.shorthand_setters, parsed.positionals).unwrap();
    (overrides, owned.tail.launch_args().to_vec())
}

#[test]
fn reclaimed_setters_keep_their_types() {
    let (overrides, forwarded) = reclaimed(
        &[],
        &["-c", "x=y", "count=3", "enabled=true", "phase=", "label=a=b"],
        &SchemaParameters::NoSchema,
    );
    assert_eq!(overrides, Some(json!({"count": 3, "enabled": true, "phase": "", "label": "a=b"})));
    assert_eq!(forwarded, s(&["-c", "x=y"]));
}

#[test]
fn a_switch_value_is_forwarded_as_an_unchanged_string_and_never_a_setter() {
    let (overrides, forwarded) = reclaimed(&[], &["-c", "count=3"], &SchemaParameters::NoSchema);
    assert_eq!(overrides, None);
    assert_eq!(forwarded, s(&["-c", "count=3"]));
}

#[test]
fn the_last_shorthand_setter_wins_across_a_provider_switch() {
    let (overrides, forwarded) = reclaimed(&["k=1"], &["k=2", "-c", "x=y", "k=3"], &SchemaParameters::NoSchema);
    assert_eq!(overrides, Some(json!({"k": 3})));
    assert_eq!(forwarded, s(&["-c", "x=y"]));
    // A value forwarded with the switch is not an occurrence of the key.
    let (overrides, forwarded) = reclaimed(&["k=1"], &["-c", "k=2"], &SchemaParameters::NoSchema);
    assert_eq!(overrides, Some(json!({"k": 1})));
    assert_eq!(forwarded, s(&["-c", "k=2"]));
}

#[test]
fn a_reclaimed_shorthand_setter_beats_set_wherever_set_is_placed() {
    let set = r#"{"phase":1,"other":"kept"}"#;
    for after in [
        &["--set", set, "-c", "x=y", "phase=2"][..],
        &["-c", "x=y", "phase=2", "--set", set],
        &["-c", "x=y", "--set", set, "phase=2"],
    ] {
        let (overrides, forwarded) = reclaimed(&[], after, &SchemaParameters::NoSchema);
        assert_eq!(overrides, Some(json!({"phase": 2, "other": "kept"})), "{after:?}");
        assert_eq!(forwarded, s(&["-c", "x=y"]), "{after:?}");
    }
}

/// A dotted key is not a setter: after a value it is a positional, and
/// directly after a string switch it is that switch's value.
#[test]
fn a_dotted_key_is_never_a_reclaimed_setter() {
    let (overrides, forwarded) = reclaimed(&[], &["-c", "x=y", "foo.bar=baz"], &SchemaParameters::NoSchema);
    assert_eq!(overrides, Some(json!({"argv": ["foo.bar=baz"]})));
    assert_eq!(forwarded, s(&["-c", "x=y"]));
    let (overrides, forwarded) = reclaimed(&[], &["-c", "foo.bar=baz"], &SchemaParameters::NoSchema);
    assert_eq!(overrides, None);
    assert_eq!(forwarded, s(&["-c", "foo.bar=baz"]));
}

/// `argv=` stays reserved after a provider switch: no exception is added
/// for a reclaimed setter.
#[test]
fn argv_stays_reserved_after_a_provider_switch() {
    let mut line = vec!["claudine", "compose", "plan.md", "-c", "x=y", "argv=a"];
    let (_, after_file) =
        crate::argv::partition_composition_tail(line.drain(..).map(std::ffi::OsString::from).collect()).unwrap();
    let candidates = [claudine::composition::OwnershipCandidate {
        provider: Provider::Codex,
        command_path: s(&["exec"]),
    }];
    let err = claudine::composition::own_arguments(&after_file, &SchemaParameters::NoSchema, &candidates).unwrap_err();
    assert!(err.to_string().contains("bare words"), "{err}");
}

// ── resolve_session_interactivity ────────────────────────────────

#[test]
fn no_interactive_wins_over_frontmatter_true() {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let shared = Probe::try_parse_from(["probe", "--no-interactive"])
        .expect("--no-interactive must parse")
        .shared;
    let resolved = shared.resolve_session_interactivity(Some(true));
    assert!(!resolved.value);
    assert_eq!(
        resolved.source,
        claudine::composition::SessionInteractivitySource::NoInteractiveFlag
    );
}

#[test]
fn interactive_flag_wins_over_frontmatter_true() {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let shared = Probe::try_parse_from(["probe", "-i"])
        .expect("-i must parse")
        .shared;
    let resolved = shared.resolve_session_interactivity(Some(true));
    assert!(resolved.value);
    assert_eq!(
        resolved.source,
        claudine::composition::SessionInteractivitySource::InteractiveFlag
    );
}

#[test]
fn frontmatter_true_beats_default_false() {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let shared = Probe::try_parse_from(["probe"])
        .expect("baseline probe must parse")
        .shared;
    let resolved = shared.resolve_session_interactivity(Some(true));
    assert!(resolved.value);
    assert_eq!(
        resolved.source,
        claudine::composition::SessionInteractivitySource::Frontmatter
    );
}

#[test]
fn absent_frontmatter_uses_default_non_interactive() {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let shared = Probe::try_parse_from(["probe"])
        .expect("baseline probe must parse")
        .shared;
    let resolved = shared.resolve_session_interactivity(None);
    assert!(!resolved.value);
    assert_eq!(
        resolved.source,
        claudine::composition::SessionInteractivitySource::Default
    );
}

#[test]
fn interactive_and_no_interactive_are_mutually_exclusive() {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let result = Probe::try_parse_from(["probe", "--interactive", "--no-interactive"]);
    assert!(
        result.is_err(),
        "--interactive + --no-interactive must be rejected by clap"
    );
}

// ── stall_timeout_secs validation gate ───────────────────────────

fn shared_from(extra: &[&str]) -> SharedComposeArgs {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: SharedComposeArgs,
    }

    let mut argv = vec!["probe"];
    argv.extend_from_slice(extra);
    Probe::try_parse_from(argv)
        .expect("probe must parse")
        .shared
}

#[test]
fn stall_timeout_secs_none_when_flag_absent() {
    assert_eq!(shared_from(&[]).stall_timeout_secs().unwrap(), None);
}

#[test]
fn stall_timeout_secs_zero_literal_is_disable_sentinel() {
    assert_eq!(
        shared_from(&["--stall-timeout", "0s"])
            .stall_timeout_secs()
            .unwrap(),
        Some(0)
    );
}

#[test]
fn stall_timeout_secs_accepts_fractional() {
    // 0.5s is a valid 500ms budget; integer-seconds truncation yields 0, but
    // the value is accepted (not rejected) by the validation gate.
    assert!(
        shared_from(&["--stall-timeout", "0.5s"])
            .stall_timeout_secs()
            .is_ok()
    );
}

#[test]
fn stall_timeout_secs_accepts_positive() {
    assert_eq!(
        shared_from(&["--stall-timeout", "10m"])
            .stall_timeout_secs()
            .unwrap(),
        Some(600)
    );
}

#[test]
fn stall_timeout_secs_rejects_invalid() {
    let err = shared_from(&["--stall-timeout", "nope"])
        .stall_timeout_secs()
        .unwrap_err();
    assert!(
        err.to_string().contains("invalid --stall-timeout value"),
        "got: {err}"
    );
}

// ── SIGINT / Ctrl+C during prep (Phase 5) ────────────────────────

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn sigint_during_prep_sets_interrupt_flag_and_renders_notice() {
    // Ensure the global flag starts clean and will be restored on exit.
    crate::output::clear_user_interrupt_for_tests();
    assert!(!crate::output::user_interrupt_observed());

    let prompt = "prompts/test.md";
    let _guard = install_user_interrupt_guard(prompt);

    // Deliver SIGINT to ourselves. The handler must be async-signal-safe
    // and may run on this thread or a signal-delivery thread.
    unsafe {
        libc::kill(libc::getpid(), libc::SIGINT);
    }

    // Poll for the flag rather than sleeping a fixed interval: under
    // full-suite contention the thread the kernel delivers the signal to
    // may not be scheduled within a small fixed window, which flaked the
    // previous 50ms sleep. The deadline is generous; a working handler
    // sets the flag in well under a millisecond.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !crate::output::user_interrupt_observed() {
        assert!(
            std::time::Instant::now() < deadline,
            "SIGINT should set the user-interrupt flag"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // The notice formatting should produce a non-empty string containing
    // the prompt argument.
    let notice = format_user_interrupt_message(prompt);
    assert!(
        notice.contains("User interrupted compose operation"),
        "notice should contain the interrupt message"
    );
    assert!(
        notice.contains(prompt),
        "notice should reference the prompt file"
    );

    // Clean up so later tests in the same process see a clean flag.
    crate::output::clear_user_interrupt_for_tests();
}
