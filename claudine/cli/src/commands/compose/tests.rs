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
fn positionals_two_files_errors() {
    let err = parse_composition_positionals(&s(&["a.md", "b.md"])).unwrap_err();
    assert!(err.to_string().contains("multiple"));
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
    let result = merge_set_overrides(None, serde_json::Map::new()).unwrap();
    assert!(result.is_none());
}

#[test]
fn merge_set_only() {
    let result = merge_set_overrides(Some(r#"{"a":"b"}"#), serde_json::Map::new()).unwrap();
    assert_eq!(result, Some(json!({"a": "b"})));
}

#[test]
fn merge_shorthand_only() {
    let mut short = serde_json::Map::new();
    short.insert("k".into(), Value::String("v".into()));
    let result = merge_set_overrides(None, short).unwrap();
    assert_eq!(result, Some(json!({"k": "v"})));
}

#[test]
fn merge_shorthand_wins() {
    let mut short = serde_json::Map::new();
    short.insert("k".into(), Value::String("new".into()));
    let result = merge_set_overrides(Some(r#"{"k":"old"}"#), short).unwrap();
    assert_eq!(result, Some(json!({"k": "new"})));
}

#[test]
fn merge_disjoint() {
    let mut short = serde_json::Map::new();
    short.insert("b".into(), json!(2));
    let result = merge_set_overrides(Some(r#"{"a":"1"}"#), short).unwrap();
    assert_eq!(result, Some(json!({"a": "1", "b": 2})));
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

    // Give the kernel a moment to deliver the signal.
    std::thread::sleep(std::time::Duration::from_millis(50));

    assert!(
        crate::output::user_interrupt_observed(),
        "SIGINT should set the user-interrupt flag"
    );

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
