use super::*;
use crate::events::{
    AgenticEvent, EnvironmentContext, EventMeta, GitContext, HardwareContext, OsContext,
    RepoContext,
};
use crate::provider::Provider;
use chrono::{TimeZone, Utc};
use darkmatter::markdown::compose::expression::{ParseMode, Parser, evaluate};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

fn parse_and_evaluate<L: EvaluationLookup>(
    expression: &str,
    mode: ParseMode,
    lookup: &L,
) -> Value {
    let mut parser = Parser::with_mode(expression, mode).expect("parser should construct");
    let parsed = parser.parse().expect("expression should parse");
    evaluate(&parsed, lookup).expect("expression should evaluate")
}

fn sample_meta() -> EventMeta {
    let mut extra: HashMap<String, Value> = HashMap::new();
    extra.insert("status".to_string(), json!("success"));
    extra.insert("attempt".to_string(), json!(3));
    extra.insert(
        "tags".to_string(),
        json!(["fast", "urgent", "infrastructure"]),
    );
    extra.insert("nested".to_string(), json!({"inner": {"depth": 7}}));

    EventMeta {
        provider: Provider::Claude,
        event: AgenticEvent::BeforeTool,
        timestamp: Utc.with_ymd_and_hms(2026, 4, 29, 10, 30, 0).unwrap(),
        session_id: Some("abc123".to_string()),
        cwd: Some("/tmp/project".to_string()),
        tool_name: Some("Bash".to_string()),
        tool_input: Some(json!({
            "command": "npm test",
            "options": {"cwd": "/repo", "timeout": 60}
        })),
        tool_response: Some(json!({"exit_code": 0})),
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        extra,
        env: EnvironmentContext {
            os: OsContext {
                os_type: "macos".to_string(),
                name: "macOS".to_string(),
                version: "15.3".to_string(),
                kernel: "Darwin 25.3.0".to_string(),
                hostname: "test-host".to_string(),
                linux_family: None,
                package_managers: vec!["brew".to_string()],
            },
            hardware: HardwareContext {
                arch: "aarch64".to_string(),
                cpu: "Apple M4 Max".to_string(),
                cores: 16,
                memory_bytes: 68_719_476_736,
                memory_available_bytes: 34_359_738_368,
            },
            git: Some(GitContext {
                repo_root: PathBuf::from("/tmp/project"),
                branch: Some("main".to_string()),
                is_dirty: true,
                staged_count: 2,
                unstaged_count: 1,
                untracked_count: 0,
                head_sha: Some("abc123def".to_string()),
                head_message: Some("feat: add feature".to_string()),
                user_name: Some("Test User".to_string()),
                user_email: None,
                remote_name: Some("origin".to_string()),
                remote_url: None,
                hosting_provider: Some("github".to_string()),
                repo_name: Some("rusty-biscuit".to_string()),
                repo_org: Some("anthropics".to_string()),
            }),
            repo: Some(RepoContext {
                is_monorepo: true,
                monorepo_standard: Some("cargo-workspace".to_string()),
                monorepo_orchestrators: vec!["nx".to_string()],
                monorepo_tool: Some("cargo-workspace".to_string()),
                root: PathBuf::from("/tmp/project"),
                packages: vec!["lib".to_string(), "cli".to_string()],
            }),
            primary_language: Some("Rust".to_string()),
            package_area: Some("claudine".to_string()),
            package: Some("claudine-cli".to_string()),
            claudine_pid: None,
        },
        agent_pid: None,
    }
}

#[test]
fn resolves_top_level_event_fields() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get("provider"), Some(json!("claude")));
    assert_eq!(lookup.get("event"), Some(json!("before_tool")));
    assert_eq!(lookup.get("session_id"), Some(json!("abc123")));
    assert_eq!(lookup.get("cwd"), Some(json!("/tmp/project")));
    assert_eq!(lookup.get("tool_name"), Some(json!("Bash")));
    assert_eq!(lookup.get("error"), None);
    assert_eq!(lookup.get_string("error"), "");
}

#[test]
fn resolves_grouped_environment_paths() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get("os.type"), Some(json!("macos")));
    assert_eq!(lookup.get("hardware.cores"), Some(json!(16)));
    assert_eq!(lookup.get("git.branch"), Some(json!("main")));
    assert_eq!(lookup.get("git.is_dirty"), Some(json!(true)));
    assert_eq!(lookup.get("git.repo_name"), Some(json!("rusty-biscuit")));
    assert_eq!(lookup.get("project.language"), Some(json!("Rust")));
    assert_eq!(lookup.get("project.is_monorepo"), Some(json!(true)));
    assert_eq!(
        lookup.get("project.monorepo_standard"),
        Some(json!("cargo-workspace"))
    );
    assert_eq!(
        lookup.get("project.monorepo_orchestrators"),
        Some(json!("nx"))
    );
    assert_eq!(
        lookup.get("project.monorepo_tool"),
        Some(json!("cargo-workspace"))
    );
}

#[test]
fn missing_keys_return_none() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get("missing"), None);
    assert_eq!(lookup.get("git.unknown"), None);
    assert_eq!(lookup.get("ctx.today"), None);
    assert_eq!(lookup.get_string("missing"), "");
}

#[test]
fn extra_paths_resolve_with_typed_values() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get("extra.status"), Some(json!("success")));
    assert_eq!(lookup.get("extra.attempt"), Some(json!(3)));
    assert_eq!(
        lookup.get("extra.nested.inner.depth"),
        Some(json!(7)),
        "nested extra paths should drill into JSON objects"
    );
    assert_eq!(
        lookup.get("extra.tags"),
        Some(json!(["fast", "urgent", "infrastructure"]))
    );
    assert_eq!(lookup.get("extra.missing"), None);
}

#[test]
fn nested_extra_walk_returns_leaf_without_cloning_intermediates() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    // Deep dotted path resolves to the scalar leaf.
    assert_eq!(lookup.get("extra.nested.inner.depth"), Some(json!(7)));
    // Partial path returns the subtree rooted at that node.
    let subtree = lookup.get("extra.nested.inner").expect("inner resolves");
    assert_eq!(subtree.pointer("/depth"), Some(&json!(7)));
    // Missing intermediate key propagates None.
    assert_eq!(lookup.get("extra.nested.missing.depth"), None);
}

#[test]
fn tool_input_paths_resolve_nested_json() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get("tool_input.command"), Some(json!("npm test")));
    assert_eq!(
        lookup.get("tool_input.options.cwd"),
        Some(json!("/repo")),
        "nested tool_input paths should drill in"
    );
    assert_eq!(lookup.get("tool_input.options.timeout"), Some(json!(60)));
    assert_eq!(lookup.get("tool_input.missing"), None);
}

#[test]
fn env_namespace_resolves_via_std_env() {
    let key = "CLAUDINE_EXPRESSION_TEST_PRESENT";
    // SAFETY: tests run sequentially within this module by default; the
    // env var is unique per test scope and is removed before exit.
    unsafe {
        std::env::set_var(key, "value-here");
    }
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);
    let resolved = lookup.get(&format!("env.{key}"));
    unsafe {
        std::env::remove_var(key);
    }
    assert_eq!(resolved, Some(json!("value-here")));
}

#[test]
fn env_namespace_returns_none_for_missing_var() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(
        lookup.get("env.CLAUDINE_EXPRESSION_TEST_DEFINITELY_MISSING"),
        None,
    );
}

#[test]
fn ctx_namespace_is_unresolved() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get("ctx.today"), None);
    assert_eq!(lookup.get("ctx"), None);
}

#[test]
fn evaluator_handles_fallback_for_missing_env_var() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    let value = parse_and_evaluate(
        "env.CLAUDINE_EXPRESSION_TEST_FALLBACK || \"local\"",
        ParseMode::Interpolation,
        &lookup,
    );

    assert_eq!(value, json!("local"));
}

#[test]
fn evaluator_handles_ternary_on_git_is_dirty() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    let value = parse_and_evaluate(
        "git.is_dirty ? \"dirty\" : \"clean\"",
        ParseMode::Interpolation,
        &lookup,
    );

    assert_eq!(value, json!("dirty"));
}

#[test]
fn evaluator_handles_numeric_comparison_on_hardware_cores() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    let value = parse_and_evaluate(
        "hardware.cores > 8 ? \"fast\" : \"slow\"",
        ParseMode::Interpolation,
        &lookup,
    );

    assert_eq!(value, json!("fast"));
}

#[test]
fn evaluator_supports_length_helper_on_extra_array() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    let value = parse_and_evaluate(
        "length(extra.tags) > 2 ? \"many\" : \"few\"",
        ParseMode::Interpolation,
        &lookup,
    );

    assert_eq!(value, json!("many"));
}

#[test]
fn evaluator_handles_string_equality_in_condition_mode() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    let value = parse_and_evaluate(
        "tool_name == 'Bash' && git.branch == 'main'",
        ParseMode::Condition,
        &lookup,
    );

    assert_eq!(value, json!(true));
}

#[test]
fn evaluator_treats_missing_optional_field_as_falsy() {
    let mut meta = sample_meta();
    meta.tool_name = None;
    let lookup = EventMetaExpressionLookup::new(&meta);

    // Interpolation fallback: tool_name is None -> Null -> falsy -> "default"
    let value = parse_and_evaluate(
        "tool_name || \"default\"",
        ParseMode::Interpolation,
        &lookup,
    );
    assert_eq!(value, json!("default"));
}

#[test]
fn evaluator_handles_unary_not_on_git_is_dirty() {
    let mut meta = sample_meta();
    if let Some(git) = meta.env.git.as_mut() {
        git.is_dirty = false;
    }
    let lookup = EventMetaExpressionLookup::new(&meta);

    let value = parse_and_evaluate(
        "provider == 'claude' && !git.is_dirty",
        ParseMode::Condition,
        &lookup,
    );

    assert_eq!(value, json!(true));
}

// ------------------------------------------------------------------
// EventMetaConditionLookup parity + ctx tests
// ------------------------------------------------------------------

#[test]
fn condition_lookup_resolves_top_level_event_fields() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("provider"), Some(json!("claude")));
    assert_eq!(lookup.get("event"), Some(json!("before_tool")));
    assert_eq!(lookup.get("session_id"), Some(json!("abc123")));
    assert_eq!(lookup.get("cwd"), Some(json!("/tmp/project")));
    assert_eq!(lookup.get("tool_name"), Some(json!("Bash")));
    assert_eq!(lookup.get("error"), None);
    assert_eq!(lookup.get_string("error"), "");
}

#[test]
fn condition_lookup_resolves_grouped_environment_paths() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("os.type"), Some(json!("macos")));
    assert_eq!(lookup.get("hardware.cores"), Some(json!(16)));
    assert_eq!(lookup.get("git.branch"), Some(json!("main")));
    assert_eq!(lookup.get("git.is_dirty"), Some(json!(true)));
    assert_eq!(lookup.get("git.repo_name"), Some(json!("rusty-biscuit")));
    assert_eq!(lookup.get("project.language"), Some(json!("Rust")));
    assert_eq!(lookup.get("project.is_monorepo"), Some(json!(true)));
    assert_eq!(
        lookup.get("project.monorepo_standard"),
        Some(json!("cargo-workspace"))
    );
    assert_eq!(
        lookup.get("project.monorepo_orchestrators"),
        Some(json!("nx"))
    );
    assert_eq!(
        lookup.get("project.monorepo_tool"),
        Some(json!("cargo-workspace"))
    );
}

#[test]
fn condition_lookup_missing_keys_return_none() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("missing"), None);
    assert_eq!(lookup.get("git.unknown"), None);
    assert_eq!(lookup.get_string("missing"), "");
}

#[test]
fn condition_lookup_extra_paths_resolve_with_typed_values() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("extra.status"), Some(json!("success")));
    assert_eq!(lookup.get("extra.attempt"), Some(json!(3)));
    assert_eq!(
        lookup.get("extra.nested.inner.depth"),
        Some(json!(7)),
        "nested extra paths should drill into JSON objects"
    );
    assert_eq!(
        lookup.get("extra.tags"),
        Some(json!(["fast", "urgent", "infrastructure"]))
    );
    assert_eq!(lookup.get("extra.missing"), None);
}

#[test]
fn condition_lookup_tool_input_paths_resolve_nested_json() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("tool_input.command"), Some(json!("npm test")));
    assert_eq!(
        lookup.get("tool_input.options.cwd"),
        Some(json!("/repo")),
        "nested tool_input paths should drill in"
    );
    assert_eq!(lookup.get("tool_input.options.timeout"), Some(json!(60)));
    assert_eq!(lookup.get("tool_input.missing"), None);
}

#[test]
fn condition_lookup_env_namespace_resolves_via_std_env() {
    let key = "CLAUDINE_CONDITION_TEST_PRESENT";
    // SAFETY: tests run sequentially within this module by default; the
    // env var is unique per test scope and is removed before exit.
    unsafe {
        std::env::set_var(key, "value-here");
    }
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));
    let resolved = lookup.get(&format!("env.{key}"));
    unsafe {
        std::env::remove_var(key);
    }
    assert_eq!(resolved, Some(json!("value-here")));
}

#[test]
fn condition_lookup_ctx_today_resolves() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    let value = lookup.get("ctx.today");
    assert!(
        value.is_some(),
        "ctx.today should resolve through the composite"
    );
    if let Some(Value::String(s)) = value {
        assert!(!s.is_empty(), "ctx.today should be non-empty");
    } else {
        panic!("ctx.today should return Value::String, got {:?}", value);
    }
}

#[test]
fn condition_lookup_ctx_year_resolves() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    let value = lookup.get("ctx.year");
    assert!(
        value.is_some(),
        "ctx.year should resolve through the composite"
    );
    if let Some(Value::String(s)) = value {
        assert!(!s.is_empty(), "ctx.year should be non-empty");
    } else {
        panic!("ctx.year should return Value::String, got {:?}", value);
    }
}

#[test]
fn condition_lookup_falls_through_to_inner() {
    let meta = sample_meta();
    let inner = EventMetaExpressionLookup::new(&meta);
    let composite = EventMetaConditionLookup::new(&meta, Path::new("."));

    // Every non-ctx path should be identical between inner and composite.
    let paths = [
        "provider",
        "event",
        "session_id",
        "cwd",
        "tool_name",
        "tool_input",
        "tool_response",
        "error",
        "extra.status",
        "extra.attempt",
        "extra.nested.inner.depth",
        "tool_input.command",
        "tool_input.options.cwd",
        "os.type",
        "hardware.cores",
        "git.branch",
        "git.is_dirty",
        "project.language",
        "project.is_monorepo",
    ];

    for path in paths {
        assert_eq!(
            composite.get(path),
            inner.get(path),
            "path '{}' should fall through to inner unchanged",
            path
        );
    }
}

#[test]
fn condition_lookup_ctx_short_circuit() {
    // Create a meta where the inner lookup would theoretically produce a
    // value for a fictional ctx.x path (it won't, because ctx.* is
    // hard-coded to None in EventMetaExpressionLookup). The important
    // invariant is that the composite returns None for unknown ctx keys
    // because CtxLookup does not recognise them, and never reaches the
    // inner adapter.
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("ctx.unknown_key"), None);
    assert_eq!(lookup.get("ctx"), None);
}

#[test]
fn condition_lookup_evaluator_handles_string_equality_in_condition_mode() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    let value = parse_and_evaluate(
        "tool_name == 'Bash' && git.branch == 'main'",
        ParseMode::Condition,
        &lookup,
    );

    assert_eq!(value, json!(true));
}

#[test]
fn condition_lookup_evaluator_handles_numeric_comparison_on_hardware_cores() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    let value = parse_and_evaluate(
        "hardware.cores > 8 ? \"fast\" : \"slow\"",
        ParseMode::Interpolation,
        &lookup,
    );

    assert_eq!(value, json!("fast"));
}

#[test]
fn doc_namespace_resolves_event_surface() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    // doc.<path> traverses the same object returned by bare doc.
    assert_eq!(lookup.get("doc.git.branch"), Some(json!("main")));
    assert_eq!(lookup.get("doc.extra.status"), Some(json!("success")));
    assert_eq!(lookup.get("doc.tool_name"), Some(json!("Bash")));

    // missing doc.* values do not fall back to another namespace.
    assert_eq!(lookup.get("doc.missing"), None);
    // env is process environment, not part of the doc object.
    assert_eq!(lookup.get("doc.env.PATH"), None);

    // bare doc returns the whole event object, including grouped env paths.
    let obj = lookup.get("doc").expect("bare doc resolves");
    assert!(obj.is_object());
    assert_eq!(obj.get("provider"), Some(&json!("claude")));
    assert_eq!(obj.get("tool_name"), Some(&json!("Bash")));
    assert_eq!(obj.pointer("/git/branch"), Some(&json!("main")));
    assert_eq!(obj.pointer("/os/type"), Some(&json!("macos")));
    assert_eq!(obj.pointer("/hardware/cores"), Some(&json!(16)));
    assert_eq!(obj.pointer("/project/language"), Some(&json!("Rust")));
}

#[test]
fn doc_dotted_path_matches_bare_doc_traversal() {
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    let doc = lookup.get("doc").expect("bare doc resolves");
    assert_eq!(lookup.get("doc.git.branch"), nested_pointer(&doc, "git.branch"));
    assert_eq!(lookup.get("doc.os.type"), nested_pointer(&doc, "os.type"));
}

#[test]
fn doc_env_does_not_resolve_even_when_env_does() {
    let key = "CLAUDINE_DOC_ENV_TEST_VAR";
    // SAFETY: tests run sequentially within this module by default; the
    // env var is unique per test scope and is removed before exit.
    unsafe {
        std::env::set_var(key, "value-here");
    }
    let meta = sample_meta();
    let lookup = EventMetaExpressionLookup::new(&meta);

    // The env.* namespace resolves, but doc.env.* must not leak into it.
    assert_eq!(lookup.get(&format!("env.{key}")), Some(json!("value-here")));
    assert_eq!(lookup.get(&format!("doc.env.{key}")), None);

    unsafe {
        std::env::remove_var(key);
    }
}

#[test]
fn condition_lookup_doc_namespace_resolves() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    assert_eq!(lookup.get("doc.tool_name"), Some(json!("Bash")));
    assert!(lookup.get("doc").is_some_and(|value| value.is_object()));
}

#[test]
fn condition_lookup_read_side_function_resolves_against_base_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("artifact"), "ready").unwrap();
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, dir.path());

    assert!(lookup.resolution_context().is_some());
    assert_eq!(
        parse_and_evaluate("file_exists('artifact')", ParseMode::Condition, &lookup),
        json!(true)
    );
    assert_eq!(
        parse_and_evaluate("file_exists('missing')", ParseMode::Condition, &lookup),
        json!(false)
    );
}

#[test]
fn condition_lookup_evaluator_supports_length_helper_on_extra_array() {
    let meta = sample_meta();
    let lookup = EventMetaConditionLookup::new(&meta, Path::new("."));

    let value = parse_and_evaluate(
        "length(extra.tags) > 2 ? \"many\" : \"few\"",
        ParseMode::Interpolation,
        &lookup,
    );

    assert_eq!(value, json!("many"));
}
