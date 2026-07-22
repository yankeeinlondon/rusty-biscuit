use super::*;
use crate::events::*;
use crate::provider::Provider;
use chrono::Utc;
use std::collections::HashMap;

fn sample_meta() -> EventMeta {
    EventMeta {
        provider: Provider::Claude,
        event: AgenticEvent::BeforeTool,
        timestamp: Utc::now(),
        session_id: Some("abc123".to_string()),
        cwd: Some("/tmp/project".to_string()),
        tool_name: Some("Bash".to_string()),
        tool_input: None,
        tool_response: None,
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        agent_pid: None,
        extra: HashMap::new(),
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
                memory_bytes: 68719476736,
                memory_available_bytes: 34359738368,
            },
            git: Some(GitContext {
                repo_root: std::path::PathBuf::from("/tmp/project"),
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
                root: std::path::PathBuf::from("/tmp/project"),
                packages: vec!["lib".to_string(), "cli".to_string()],
            }),
            primary_language: Some("Rust".to_string()),
            package_area: Some("claudine".to_string()),
            package: Some("claudine-cli".to_string()),
            claudine_pid: None,
        },
    }
}

#[test]
fn provider_placeholder() {
    let meta = sample_meta();
    assert_eq!(interpolate("Hello {{provider}}", &meta), "Hello claude");
}

#[test]
fn tool_name_and_git_branch() {
    let meta = sample_meta();
    assert_eq!(
        interpolate("{{tool_name}} on {{git.branch}}", &meta),
        "Bash on main"
    );
}

#[test]
fn unknown_placeholder_left_as_is() {
    let meta = sample_meta();
    assert_eq!(interpolate("{{unknown_field}}", &meta), "{{unknown_field}}");
}

#[test]
fn none_optional_renders_empty() {
    let mut meta = sample_meta();
    meta.tool_name = None;
    assert_eq!(interpolate("Tool: {{tool_name}}", &meta), "Tool: ");
}

#[test]
fn no_placeholders_returns_original() {
    let meta = sample_meta();
    assert_eq!(
        interpolate("no placeholders here", &meta),
        "no placeholders here"
    );
}

#[test]
fn event_display() {
    let meta = sample_meta();
    assert_eq!(interpolate("{{event}}", &meta), "before_tool");
}

#[test]
fn hardware_cores() {
    let meta = sample_meta();
    assert_eq!(interpolate("{{hardware.cores}}", &meta), "16");
}

#[test]
fn multiple_context_placeholders() {
    let meta = sample_meta();
    assert_eq!(
        interpolate("{{os.type}} {{hardware.arch}}", &meta),
        "macos aarch64"
    );
}

#[test]
fn git_repo_name() {
    let meta = sample_meta();
    assert_eq!(interpolate("{{git.repo_name}}", &meta), "rusty-biscuit");
}

#[test]
fn git_repo_org() {
    let meta = sample_meta();
    assert_eq!(interpolate("{{git.repo_org}}", &meta), "anthropics");
}

#[test]
fn git_repo_name_and_org_combined() {
    let meta = sample_meta();
    assert_eq!(
        interpolate("{{git.repo_org}}/{{git.repo_name}}", &meta),
        "anthropics/rusty-biscuit"
    );
}

#[test]
fn project_monorepo_standard() {
    let meta = sample_meta();
    assert_eq!(
        interpolate("{{project.monorepo_standard}}", &meta),
        "cargo-workspace"
    );
}

#[test]
fn project_monorepo_orchestrators() {
    let meta = sample_meta();
    assert_eq!(
        interpolate("{{project.monorepo_orchestrators}}", &meta),
        "nx"
    );
}

#[test]
fn project_monorepo_tool_alias() {
    let meta = sample_meta();
    // `project.monorepo_tool` is now a deprecated alias for
    // `project.monorepo_standard` and returns the kebab-case authority id.
    assert_eq!(
        interpolate("{{project.monorepo_tool}}", &meta),
        "cargo-workspace"
    );
}

#[test]
fn env_variable_resolution() {
    let meta = sample_meta();
    let expected = std::env::var("HOME").unwrap_or_default();
    let rendered = interpolate("{{env.HOME}}", &meta);
    assert_eq!(rendered, expected);
}

#[test]
fn env_variable_default_is_used_when_not_present() {
    let meta = sample_meta();
    let rendered = interpolate(
        "{{env.CLAUDINE_TEMPLATE_TEST_MISSING || \"fallback\"}}",
        &meta,
    );
    assert_eq!(rendered, "fallback");
}

#[test]
fn ternary_on_git_is_dirty() {
    let meta = sample_meta();
    let rendered = interpolate("Status: {{git.is_dirty ? \"dirty\" : \"clean\"}}", &meta);
    assert_eq!(rendered, "Status: dirty");
}

#[test]
fn numeric_comparison_on_hardware_cores() {
    let meta = sample_meta();
    let rendered = interpolate("Speed: {{hardware.cores > 8 ? \"fast\" : \"slow\"}}", &meta);
    assert_eq!(rendered, "Speed: fast");
}

#[test]
fn fallback_on_missing_env_variable() {
    let meta = sample_meta();
    // SAFETY: tests run sequentially in a single process; we remove the
    // var on the way out.
    unsafe {
        std::env::remove_var("CLAUDINE_TEMPLATE_TEST_FALLBACK_X");
    }
    let rendered = interpolate(
        "Mode: {{env.CLAUDINE_TEMPLATE_TEST_FALLBACK_X || \"local\"}}",
        &meta,
    );
    assert_eq!(rendered, "Mode: local");
}

#[test]
fn length_helper_on_branch_name() {
    let meta = sample_meta();
    let rendered = interpolate(
        "{{length(git.branch) > 30 ? \"long-branch\" : git.branch}}",
        &meta,
    );
    // sample_meta branch is "main" (4 chars), so we get the branch name.
    assert_eq!(rendered, "main");
}

#[test]
fn boolean_value_renders_as_text() {
    let meta = sample_meta();
    assert_eq!(interpolate("{{git.is_dirty}}", &meta), "true");
}

#[test]
fn malformed_expression_is_preserved() {
    let meta = sample_meta();
    // `+` is not a supported operator; parser fails -> token preserved.
    assert_eq!(interpolate("{{a + b}}", &meta), "{{a + b}}");
}

#[test]
fn legacy_single_pipe_fallback_is_preserved_verbatim() {
    // Single-pipe `|` is no longer recognised as a fallback operator;
    // Darkmatter's interpolation lexer only accepts `||`. The token
    // must therefore round-trip unchanged so operators can spot stale
    // configs in their output.
    let meta = sample_meta();
    let raw = "{{env.CLAUDINE_TEMPLATE_TEST_LEGACY_PIPE | \"fallback\"}}";
    // SAFETY: tests run sequentially in this module.
    unsafe {
        std::env::remove_var("CLAUDINE_TEMPLATE_TEST_LEGACY_PIPE");
    }
    assert_eq!(interpolate(raw, &meta), raw);
}

#[test]
fn legacy_single_brace_template_is_rewritten() {
    let meta = sample_meta();
    assert_eq!(interpolate("Tool {tool_name}", &meta), "Tool Bash");
}

// Tests for TemplateVariable enum
#[test]
fn all_variables_have_unique_keys() {
    let keys: Vec<_> = TemplateVariable::all().iter().map(|v| v.key()).collect();
    let mut unique_keys = keys.clone();
    unique_keys.sort();
    unique_keys.dedup();
    assert_eq!(
        keys.len(),
        unique_keys.len(),
        "Duplicate keys found in TemplateVariable::all()"
    );
}

#[test]
fn from_key_finds_all_variables() {
    for var in TemplateVariable::all() {
        let found = TemplateVariable::from_key(var.key());
        assert_eq!(
            found,
            Some(*var),
            "from_key({}) should find {:?}",
            var.key(),
            var
        );
    }
}

#[test]
fn from_key_returns_none_for_unknown() {
    assert_eq!(TemplateVariable::from_key("unknown"), None);
    assert_eq!(TemplateVariable::from_key("git.unknown"), None);
}

#[test]
fn placeholder_format() {
    assert_eq!(TemplateVariable::Provider.placeholder(), "{{provider}}");
    assert_eq!(TemplateVariable::EnvBranch.placeholder(), "{{git.branch}}");
}

#[test]
fn event_variables_filter() {
    let event_vars: Vec<_> = TemplateVariable::event_variables().collect();
    assert!(event_vars.contains(&&TemplateVariable::Provider));
    assert!(event_vars.contains(&&TemplateVariable::ToolName));
    assert!(!event_vars.contains(&&TemplateVariable::EnvBranch));
}

#[test]
fn context_variables_filter() {
    let ctx_vars: Vec<_> = TemplateVariable::context_variables().collect();
    assert!(ctx_vars.contains(&&TemplateVariable::EnvBranch));
    assert!(ctx_vars.contains(&&TemplateVariable::EnvRepoName));
    assert!(!ctx_vars.contains(&&TemplateVariable::Provider));
}

#[test]
fn all_variables_have_descriptions() {
    for var in TemplateVariable::all() {
        assert!(
            !var.description().is_empty(),
            "{:?} should have a description",
            var
        );
    }
}

#[test]
fn all_variables_have_availability() {
    for var in TemplateVariable::all() {
        assert!(
            !var.availability().is_empty(),
            "{:?} should have availability",
            var
        );
    }
}
