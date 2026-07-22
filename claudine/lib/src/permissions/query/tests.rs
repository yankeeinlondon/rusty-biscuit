use super::*;
use crate::permissions::{
    CanonicalPolicy, CanonicalRuleProvenance, PathAccessRule, PolicyContext, PolicyMode,
    PolicyWarning,
};

#[test]
fn command_query_tokenizes_quotes_and_escapes() {
    let query = CommandQuery::from_raw(r#"git commit -m "hello world" path\ with\ spaces"#);

    assert_eq!(query.executable.as_deref(), Some("git"));
    assert_eq!(
        query.argv,
        vec!["git", "commit", "-m", "hello world", "path with spaces"],
    );
}

#[test]
fn command_query_skips_env_assignments_for_executable() {
    let query = CommandQuery::from_raw("FOO=bar BAZ=qux /usr/bin/python script.py");

    assert_eq!(query.executable.as_deref(), Some("python"));
    assert_eq!(query.argv[0], "FOO=bar");
    assert_eq!(query.argv[2], "/usr/bin/python");
}

#[test]
fn path_queries_are_normalized_relative_to_cwd() {
    let cwd = PathBuf::from("/workspace/project");
    let ctx = PolicyContext::new(cwd.clone()).with_repo_root(cwd.clone());
    let mut canonical = CanonicalPolicy::empty(Provider::Claude, PolicyMode::Configured);
    canonical.axes.filesystem.write_rules.push(PathAccessRule {
        pattern: "/workspace/project/src".to_owned(),
        effect: PolicyEffect::Allow,
        provenance: CanonicalRuleProvenance::exact("repo", "sandbox.filesystem.allowWrite"),
    });

    let snapshot = ConfiguredPolicySnapshot::from_parts(
        Provider::Claude,
        NativeEffectivePolicy::new(Provider::Claude, Vec::new(), ()),
        canonical,
        &ctx,
    );
    let result = snapshot.can_write("src/../src/main.rs");

    assert!(result.is_allowed());
    assert_eq!(result.stability, QueryStability::MayChangeWithCli);
    assert!(result.explanation.summary.contains("workspace"));
    assert!(
        result
            .explanation
            .summary
            .contains("/workspace/project/src/main.rs")
    );
}

// --- MCP server→tool inheritance tests ---
//
// These verify the precedence: tool rule beats server rule; otherwise the
// server rule becomes the fallback tool answer.

fn mcp_snapshot(
    server_rules: Vec<super::super::canonical::McpServerRule>,
    tool_rules: Vec<super::super::canonical::McpToolRule>,
) -> ConfiguredPolicySnapshot {
    let cwd = PathBuf::from("/workspace/project");
    let ctx = PolicyContext::new(cwd.clone()).with_repo_root(cwd);
    let mut canonical = CanonicalPolicy::empty(Provider::Claude, PolicyMode::Configured);
    canonical.axes.mcp.server_rules = server_rules;
    canonical.axes.mcp.tool_rules = tool_rules;
    ConfiguredPolicySnapshot::from_parts(
        Provider::Claude,
        NativeEffectivePolicy::new(Provider::Claude, Vec::new(), ()),
        canonical,
        &ctx,
    )
}

fn server_rule(server: &str, effect: PolicyEffect) -> super::super::canonical::McpServerRule {
    super::super::canonical::McpServerRule {
        server_id: server.to_owned(),
        effect,
        provenance: CanonicalRuleProvenance::exact("test", "mcp.server"),
    }
}

fn tool_rule(
    server: &str,
    tool: &str,
    effect: PolicyEffect,
) -> super::super::canonical::McpToolRule {
    super::super::canonical::McpToolRule {
        server_id: server.to_owned(),
        tool_name: tool.to_owned(),
        effect,
        provenance: CanonicalRuleProvenance::exact("test", "mcp.tool"),
    }
}

#[test]
fn mcp_server_deny_overrides_tool_allow() {
    let snapshot = mcp_snapshot(
        vec![server_rule("github", PolicyEffect::Deny)],
        vec![tool_rule("github", "create_issue", PolicyEffect::Allow)],
    );
    assert!(
        snapshot
            .can_use_mcp_tool("github", "create_issue")
            .is_denied()
    );
}

#[test]
fn mcp_server_allow_inherits_to_tool_when_no_tool_rule() {
    let snapshot = mcp_snapshot(vec![server_rule("filesystem", PolicyEffect::Allow)], vec![]);
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "read_file")
            .is_allowed()
    );
}

#[test]
fn mcp_server_ask_inherits_to_tool_when_no_tool_rule() {
    let snapshot = mcp_snapshot(vec![server_rule("filesystem", PolicyEffect::Ask)], vec![]);
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "read_file")
            .is_ask()
    );
}

#[test]
fn mcp_tool_deny_overrides_server_allow() {
    let snapshot = mcp_snapshot(
        vec![server_rule("filesystem", PolicyEffect::Allow)],
        vec![tool_rule("filesystem", "delete_file", PolicyEffect::Deny)],
    );
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "delete_file")
            .is_denied()
    );
    // Other tools on the same server still inherit the allow.
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "read_file")
            .is_allowed()
    );
}

#[test]
fn trust_unknown_warnings_make_query_results_unknown() {
    let cwd = PathBuf::from("/workspace/project");
    let ctx = PolicyContext::new(cwd.clone()).with_repo_root(cwd);
    let mut canonical = CanonicalPolicy::empty(Provider::Codex, PolicyMode::Configured);
    canonical.axes.filesystem.write_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: PolicyEffect::Deny,
        provenance: CanonicalRuleProvenance::exact("codex-user", "sandbox_mode"),
    });
    canonical.warnings.push(PolicyWarning {
        code: "codex.trust_unknown".to_owned(),
        message: "Repo-scoped Codex config is trust-gated and trust was not supplied."
            .to_owned(),
        source_id: None,
    });

    let snapshot = ConfiguredPolicySnapshot::from_parts(
        Provider::Codex,
        NativeEffectivePolicy::new(Provider::Codex, Vec::new(), ()),
        canonical,
        &ctx,
    );
    let result = snapshot.can_write("src/main.rs");

    assert!(result.is_unknown());
    assert_eq!(result.certainty, PolicyCertainty::Unknown);
    assert_eq!(result.stability, QueryStability::Unknown);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.explanation.summary.contains("Trust is unknown"));
}
