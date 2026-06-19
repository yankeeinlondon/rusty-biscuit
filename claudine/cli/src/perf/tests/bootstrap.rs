use super::*;
use std::ffi::OsString;

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

#[test]
fn bootstrap_enabled_for_wrapper_with_perf() {
    let raw = argv(&["claudine", "codex", "prompt", "--perf"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(bootstrap.enabled);
    assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Wrapper));
}

#[test]
fn bootstrap_enabled_for_compose_with_perf() {
    let raw = argv(&["claudine", "compose", "--perf", "file.md"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(bootstrap.enabled);
    assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Compose));
}

#[test]
fn bootstrap_enabled_for_inline_compose_with_perf() {
    let raw = argv(&["claudine", "inline-compose", "--perf", "file.md"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(bootstrap.enabled);
    assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::InlineCompose));
}

#[test]
fn bootstrap_enabled_for_sequence_with_perf() {
    let raw = argv(&["claudine", "sequence", "--perf", "file.md"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(bootstrap.enabled);
    assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Sequence));
}

#[test]
fn bootstrap_disabled_without_perf() {
    let raw = argv(&["claudine", "codex", "prompt"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(!bootstrap.enabled);
    assert!(bootstrap.command_kind.is_none());
}

#[test]
fn bootstrap_disabled_for_hooks_with_perf() {
    let raw = argv(&["claudine", "hooks", "--perf"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(!bootstrap.enabled);
    assert!(bootstrap.command_kind.is_none());
}

#[test]
fn bootstrap_disabled_for_logs_with_perf() {
    let raw = argv(&["claudine", "logs", "--perf"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(!bootstrap.enabled);
}

#[test]
fn bootstrap_ignores_perf_after_dash_dash() {
    let raw = argv(&["claudine", "codex", "--", "--perf"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(!bootstrap.enabled);
}

#[test]
fn bootstrap_disabled_for_empty_argv() {
    let raw = argv(&["claudine"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert!(!bootstrap.enabled);
}

#[test]
fn bootstrap_uses_first_matching_kind_for_wrapper() {
    let raw = argv(&["claudine", "claude", "--perf"]);
    let bootstrap = scan_perf_bootstrap(&raw);
    assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Wrapper));
}
