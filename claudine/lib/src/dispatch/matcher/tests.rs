use super::*;
use crate::events::*;
use crate::provider::Provider;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing_test::traced_test;

fn tool_meta(event: AgenticEvent, tool_name: Option<&str>) -> EventMeta {
    EventMeta {
        provider: Provider::Claude,
        event,
        timestamp: Utc::now(),
        session_id: None,
        cwd: None,
        tool_name: tool_name.map(String::from),
        tool_input: None,
        tool_response: None,
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        agent_pid: None,
        extra: HashMap::new(),
        env: EnvironmentContext::default(),
    }
}

fn notification_meta(ntype: Option<&str>) -> EventMeta {
    EventMeta {
        provider: Provider::Claude,
        event: AgenticEvent::Notification,
        timestamp: Utc::now(),
        session_id: None,
        cwd: None,
        tool_name: None,
        tool_input: None,
        tool_response: None,
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: ntype.map(String::from),
        notification_message: None,
        agent_pid: None,
        extra: HashMap::new(),
        env: EnvironmentContext::default(),
    }
}

fn tool_meta_with_branch(
    event: AgenticEvent,
    tool_name: Option<&str>,
    branch: Option<&str>,
    is_dirty: bool,
) -> EventMeta {
    let mut meta = tool_meta(event, tool_name);
    meta.env.git = Some(GitContext {
        repo_root: PathBuf::from("/tmp/repo"),
        branch: branch.map(String::from),
        is_dirty,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        head_sha: None,
        head_message: None,
        user_name: None,
        user_email: None,
        remote_name: None,
        remote_url: None,
        hosting_provider: None,
        repo_name: None,
        repo_org: None,
    });
    meta
}

#[test]
fn no_matcher_returns_true() {
    let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
    assert!(matches_with_pattern(None, &meta));
}

#[test]
fn regex_matches_tool_name() {
    let meta_bash = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
    let meta_edit = tool_meta(AgenticEvent::AfterTool, Some("Edit"));
    let meta_read = tool_meta(AgenticEvent::BeforeTool, Some("Read"));

    assert!(matches_with_pattern(Some("Bash|Edit"), &meta_bash));
    assert!(matches_with_pattern(Some("Bash|Edit"), &meta_edit));
    assert!(!matches_with_pattern(Some("Bash|Edit"), &meta_read));
}

#[test]
fn regex_matches_tool_error() {
    let meta = tool_meta(AgenticEvent::ToolError, Some("Bash"));
    assert!(matches_with_pattern(Some("Bash"), &meta));
}

#[test]
fn regex_matches_notification_type() {
    let meta_match = notification_meta(Some("permission_prompt"));
    let meta_nomatch = notification_meta(Some("info"));

    assert!(matches_with_pattern(
        Some("permission_prompt|ToolPermission"),
        &meta_match
    ));
    assert!(!matches_with_pattern(
        Some("permission_prompt|ToolPermission"),
        &meta_nomatch
    ));
}

#[test]
fn tool_event_with_no_tool_name_returns_false() {
    let meta = tool_meta(AgenticEvent::BeforeTool, None);
    assert!(!matches_with_pattern(Some("Bash"), &meta));
}

#[test]
fn notification_with_no_type_returns_false() {
    let meta = notification_meta(None);
    assert!(!matches_with_pattern(Some("info"), &meta));
}

#[test]
fn invalid_matcher_returns_false() {
    let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
    // A pattern that is neither a valid expression nor a valid regex
    // compiles to None, which causes matches_with_pattern to return
    // false (binding skipped).
    assert!(!matches_with_pattern(Some("[invalid(regex"), &meta));
}

#[test]
fn non_tool_event_with_regex_matcher_returns_true() {
    let meta = tool_meta(AgenticEvent::SessionStart, None);
    assert!(matches_with_pattern(Some("anything"), &meta));
}

#[test]
fn matches_with_pattern_function() {
    let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
    assert!(matches_with_pattern(Some("Bash"), &meta));
    assert!(!matches_with_pattern(Some("Read"), &meta));
    assert!(matches_with_pattern(None, &meta));
}

// -----------------------------------------------------------------
// Expression-mode matcher tests
// -----------------------------------------------------------------

#[test]
fn expression_matches_tool_and_branch() {
    let meta =
        tool_meta_with_branch(AgenticEvent::BeforeTool, Some("Bash"), Some("main"), false);

    assert!(matches_with_pattern(
        Some("tool_name == 'Bash' && git.branch == 'main'"),
        &meta,
    ));
}

#[test]
fn expression_provider_and_not_dirty() {
    let meta =
        tool_meta_with_branch(AgenticEvent::BeforeTool, Some("Bash"), Some("main"), false);

    assert!(matches_with_pattern(
        Some("provider == 'claude' && !git.is_dirty"),
        &meta,
    ));
}

#[test]
fn expression_fails_when_branch_does_not_match() {
    let meta = tool_meta_with_branch(
        AgenticEvent::BeforeTool,
        Some("Bash"),
        Some("feature/foo"),
        false,
    );

    assert!(!matches_with_pattern(
        Some("tool_name == 'Bash' && git.branch == 'main'"),
        &meta,
    ));
}

#[test]
fn expression_returns_false_for_missing_field() {
    // tool_name is missing but the expression references it.
    let meta = tool_meta(AgenticEvent::SessionStart, None);

    assert!(!matches_with_pattern(Some("tool_name == 'Bash'"), &meta,));
}

#[test]
fn expression_compiles_with_helper_function() {
    let mut meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
    meta.tool_input = Some(serde_json::json!({"command": "echo hi"}));

    let matcher =
        RuntimeMatcher::compile("tool_name == 'Bash' && length(tool_input.command) > 0")
            .expect("compile should succeed");
    assert!(matches!(matcher, RuntimeMatcher::Expression { .. }));
    assert!(matches(Some(&matcher), &meta));
}

#[test]
fn compile_prefers_regex_for_bare_word() {
    // `Bash` parses as a bare variable; we prefer regex semantics so
    // legacy `tool_name`-style matchers keep working.
    let matcher =
        RuntimeMatcher::compile("Bash|Edit").expect("compile should succeed for regex");
    assert!(matches!(matcher, RuntimeMatcher::Regex(_)));
}

#[test]
fn compile_returns_none_for_invalid_input() {
    // Neither valid expression nor valid regex.
    assert!(RuntimeMatcher::compile("[invalid(regex").is_none());
}

#[test]
fn compile_returns_none_for_empty_string() {
    assert!(RuntimeMatcher::compile("").is_none());
    assert!(RuntimeMatcher::compile("   ").is_none());
}

#[traced_test]
#[test]
fn compile_many_aggregates_one_warning_for_invalid_matchers() {
    let bindings = &[
        (AgenticEvent::BeforeTool, "[invalid(regex"),
        (AgenticEvent::AfterTool, "(also[bad"),
    ];
    let results = compile_many(bindings);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|(_, m)| m.is_none()));

    logs_assert(|logs| {
        let matching: Vec<_> = logs
            .iter()
            .filter(|l| l.contains("listed bindings will fire unconditionally"))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "expected exactly one aggregated warning, got: {:?}",
            logs
        );
        let warning = matching[0];
        assert!(warning.contains("before_tool"));
        assert!(warning.contains("after_tool"));
        assert!(warning.contains("[invalid(regex"));
        assert!(warning.contains("(also[bad"));
        Ok(())
    });
}

#[traced_test]
#[test]
fn compile_many_emits_no_warning_for_valid_matchers() {
    let bindings = &[
        (AgenticEvent::BeforeTool, "Bash|Edit"),
        (AgenticEvent::AfterTool, "tool_name == 'Bash'"),
    ];
    let results = compile_many(bindings);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|(_, m)| m.is_some()));

    logs_assert(|logs| {
        let matching: Vec<_> = logs
            .iter()
            .filter(|l| l.contains("listed bindings will fire unconditionally"))
            .collect();
        assert!(
            matching.is_empty(),
            "expected no warnings, got: {:?}",
            logs
        );
        Ok(())
    });
}

#[traced_test]
#[test]
fn compile_many_skips_empty_matchers_in_warning() {
    let bindings = &[
        (AgenticEvent::BeforeTool, ""),
        (AgenticEvent::AfterTool, "[invalid(regex"),
    ];
    compile_many(bindings);

    logs_assert(|logs| {
        let matching: Vec<_> = logs
            .iter()
            .filter(|l| l.contains("listed bindings will fire unconditionally"))
            .collect();
        assert_eq!(matching.len(), 1, "expected one warning, got: {:?}", logs);
        let warning = matching[0];
        assert!(!warning.contains("before_tool"));
        assert!(warning.contains("after_tool"));
        Ok(())
    });
}
