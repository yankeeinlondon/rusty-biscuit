use super::*;
use crate::diagnostics::Diagnostic;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use std::sync::{Arc, Mutex};

struct CapturingHandler {
    captured: Arc<Mutex<Vec<ShellApprovalRequest>>>,
}

impl CapturingHandler {
    fn new() -> Self {
        Self {
            captured: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn captured_requests(&self) -> Vec<ShellApprovalRequest> {
        self.captured.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for CapturingHandler {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.captured.lock().unwrap().push(request);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

struct MockApprovalHandler {
    decision: ShellApprovalDecision,
    call_count: Arc<Mutex<usize>>,
}

impl MockApprovalHandler {
    fn new(decision: ShellApprovalDecision) -> Self {
        Self {
            decision,
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    fn calls(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

impl ShellApprovalHandler for MockApprovalHandler {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        *self.call_count.lock().unwrap() += 1;
        Ok(self.decision.clone())
    }
}

/// Creates a temp dir with a whitelist file that allows commands prefixed
/// with the given executables.
fn approval_options_with_whitelist(
    prefixes: &[&str],
) -> (tempfile::TempDir, ShellApprovalOptions) {
    let dir = tempfile::TempDir::new().unwrap();
    let whitelist_content: String = prefixes.iter().map(|p| format!("prefix {p}\n")).collect();
    std::fs::write(
        dir.path().join(".darkmatter-shell-whitelist"),
        whitelist_content,
    )
    .unwrap();
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };
    (dir, options)
}

#[test]
fn empty_sources_returns_empty_approved_set() {
    let options = ShellApprovalOptions::default();
    let result = resolve_shell_approvals(None, None, &options, None, None).unwrap();

    assert!(result.approved_commands.is_empty());
    assert_eq!(result.total_discovered, 0);
    assert_eq!(result.already_whitelisted, 0);
    assert_eq!(result.user_approved, 0);
}

/// Regression: the template `::shell` preflight runs a full compose
/// pipeline. A `success` (lifecycle) frontmatter key that reads a file the
/// run is about to create (`frontmatter(<missing>, …)`) must NOT be
/// resolved at compose time — its `{{ … }}` spans are late-bound and fire
/// only when the event does. The CLI defers the lifecycle keys by passing
/// `with_exclude_keys(LIFECYCLE_EVENT_KEYS)` into the preflight options;
/// without it, the now-fatal file-reference check aborts preflight before
/// the event that would make the file exist ever runs.
#[test]
fn lifecycle_key_with_missing_file_ref_is_deferred_in_preflight() {
    let doc = "---\n\
status: \"{{ frontmatter('does-not-exist.md', 'ready') }}\"\n\
---\nbody\n";
    let md: Markdown = doc.into();

    // Without the exclusion, the file-ref read on a missing file is fatal.
    let no_exclude = ComposeOptions::new();
    let unguarded = resolve_shell_approvals(
        Some(&md),
        Some(&no_exclude),
        &ShellApprovalOptions::default(),
        None,
        None,
    );
    assert!(
        unguarded.is_err(),
        "a compose-time read of a missing file must be fatal when not deferred"
    );

    // With the lifecycle key excluded (DM1), the key survives raw and
    // preflight succeeds — mirroring the CLI's preflight option builders.
    let excluded = ComposeOptions::new().with_exclude_keys(["status"]);
    let guarded = resolve_shell_approvals(
        Some(&md),
        Some(&excluded),
        &ShellApprovalOptions::default(),
        None,
        None,
    )
    .expect("excluding the lifecycle key must defer its file-ref read past preflight");
    assert!(guarded.approved_commands.is_empty());
}

#[test]
fn dry_run_no_handler_emits_cannot_dry_run_message() {
    // Non-TTY dry-run gate: an unapproved command with no approval
    // handler surfaces the spec's `Cannot dry-run: …` message naming
    // the offending command and pointing at `--yolo` / pre-approval.
    let md: Markdown = "# Test\n::shell curl https://example.com\n".into();
    let compose_options = ComposeOptions::new();
    let dir = tempfile::TempDir::new().unwrap();
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: None,
        dry_run: true,
        ..Default::default()
    };

    let err = resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None)
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Cannot dry-run: shell command 'curl https://example.com' requires \
                      interactive approval."),
        "expected dry-run gate message naming the command; got: {msg}"
    );
    assert!(msg.contains("--yolo"), "message should mention --yolo; got: {msg}");
}

#[test]
fn non_dry_run_no_handler_keeps_generic_message() {
    // Without `--dry-run` the no-handler path keeps the generic
    // provenance-bearing message (unchanged behavior).
    let md: Markdown = "# Test\n::shell curl https://example.com\n".into();
    let compose_options = ComposeOptions::new();
    let (_dir, options) = approval_options_with_whitelist(&[]);

    let err = resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None)
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("no approval handler"),
        "expected generic no-handler message; got: {msg}"
    );
    assert!(
        !msg.contains("Cannot dry-run"),
        "non-dry-run path must not use the dry-run framing; got: {msg}"
    );
}

#[test]
fn discovers_commands_from_template() {
    let md: Markdown = "# Test\n::shell echo hello\n".into();
    let compose_options = ComposeOptions::new();
    let (_dir, approval_options) = approval_options_with_whitelist(&["echo"]);

    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_options), &approval_options, None, None)
            .unwrap();

    assert_eq!(result.total_discovered, 1);
    assert!(result.approved_commands.contains("echo hello"));
}

#[test]
fn blacklisted_command_returns_error() {
    let md: Markdown = "# Test\n::shell rm -rf /\n".into();
    let compose_options = ComposeOptions::new();
    let (_dir, approval_options) = approval_options_with_whitelist(&["rm"]);

    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_options), &approval_options, None, None);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::ShellApprovalUnavailable {
                failure: ShellApprovalFailure::Blacklisted(_),
                ..
            }
        ),
        "expected ShellApprovalUnavailable/Blacklisted, got: {err}"
    );
    // The prose the variant replaced is a user-visible surface; typing the
    // error must not reword it.
    assert!(
        err.to_string().contains("is blacklisted:"),
        "blacklist message must survive the typed variant; got: {err}"
    );
}

/// Every approval failure claims `composition.shell_approval` and projects the
/// authored command, its source, and a matchable `reason` — the distinction a
/// `when:` clause lost when these three all collapsed into
/// `PreFlightFailed(String)`'s `composition.failed` catch-all (Phase 7 finding,
/// ruled in decisions.md §D-14).
#[test]
fn approval_failures_project_a_matchable_reason() {
    let cases = [
        (
            ShellApprovalFailure::Blacklisted("destructive".to_string()),
            "blacklisted",
        ),
        (ShellApprovalFailure::NoHandler, "no_handler"),
        (ShellApprovalFailure::DryRun, "dry_run"),
    ];

    for (failure, expected_reason) in cases {
        let err = CompositionError::ShellApprovalUnavailable {
            command: "rm -rf /".to_string(),
            source_file: std::path::PathBuf::from("/repo/run.md"),
            line: 12,
            failure,
        };
        assert_eq!(err.code(), "composition.shell_approval");
        let detail = err.detail();
        assert_eq!(detail["reason"], serde_json::json!(expected_reason));
        assert_eq!(detail["command"], serde_json::json!("rm -rf /"));
        assert_eq!(detail["source_path"], serde_json::json!("/repo/run.md"));
        assert_eq!(detail["line"], serde_json::json!(12));
    }
}

/// A user declining shares the family's code, so one `when:` clause catches
/// every approval failure, and `reason` separates a denial from a blacklist hit.
#[test]
fn a_user_denial_shares_the_approval_code_with_reason_denied() {
    let err = CompositionError::ShellCommandDenied {
        command: "curl example.com".to_string(),
        source_file: std::path::PathBuf::from("/repo/run.md"),
        line: 3,
    };
    assert_eq!(err.code(), "composition.shell_approval");
    assert_eq!(err.detail()["reason"], serde_json::json!("denied"));
}

/// The lifecycle-stack source carries no line number, and `0` is its sentinel.
/// Projecting `0` would assert a line that does not exist, so it must be `null`
/// — the same absent-optional rule the rest of `err.detail.*` follows.
#[test]
fn a_line_less_source_projects_a_null_line_not_zero() {
    let err = CompositionError::ShellApprovalUnavailable {
        command: "rm -rf /".to_string(),
        source_file: std::path::PathBuf::from("<lifecycle-stack>"),
        line: 0,
        failure: ShellApprovalFailure::NoHandler,
    };
    assert!(err.detail()["line"].is_null(), "line 0 must project null");
}

/// `PreFlightFailed` keeps the `composition.failed` catch-all deliberately: it
/// is prose covering unrelated failures (the early-binding state builder, a
/// shell-audit error outside the family), so claiming the approval code would
/// mean parsing its `Display` to find a reason — the exact defect this feature
/// exists to remove (decisions.md §D-14).
#[test]
fn preflight_failed_does_not_claim_the_approval_code() {
    let err = CompositionError::PreFlightFailed("building early-binding state failed".to_string());
    assert_eq!(err.code(), "composition.failed");
}

#[test]
fn full_flow_template_with_whitelisted_commands() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");
    std::fs::write(&file_path, "# Test\n::shell echo hello\n").unwrap();

    // Create whitelist with echo prefix
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

    let md = Markdown::try_from(file_path.as_path()).unwrap();
    let compose_opts = ComposeOptions::new().with_source_file(&file_path);

    let options = ShellApprovalOptions {
        policy_root: Some(temp_dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };

    // Pre-flight should discover and approve "echo hello"
    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_opts), &options, None, None).unwrap();
    assert!(result.approved_commands.contains("echo hello"));

    // Now compose with the pre-approved set — should succeed
    let compose_with_approval = ComposeOptions::new()
        .with_source_file(&file_path)
        .with_pre_approved_commands(result.approved_commands);
    let (composed, _) = md.compose_with(compose_with_approval).unwrap();
    assert!(composed.content().contains("hello"));
}

/// End-to-end regression for the `review-feature.md` "command not
/// pre-approved" bug. The prompt has:
///
/// - a shell-pending `dir` whose template (`{{ spec || design }}`) is
///   transitively blocked (`design` references `dir`), so it resolves only
///   in the interpolation fallback pass; and
/// - a sibling `iteration` key calling `file_exists()`, a filesystem
///   function that errors during the context-free pre-flight pass.
///
/// Pre-flight must collect the RESOLVED `dirname <spec>` command so the
/// pre-approved set covers exactly what `compose_with` later executes —
/// otherwise execution fails with `NotPreApproved`.
#[test]
fn full_flow_shell_pending_dir_with_context_requiring_sibling_key() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("review.md");
    std::fs::write(
        &file_path,
        "---\n\
dir: \"$(dirname '{{ spec || design }}')\"\n\
design: \"{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}\"\n\
iteration: \"{{ file_exists('design.md') ? 2 : 1 }}\"\n\
---\nReview {{dir}} iteration {{iteration}}\n",
    )
    .unwrap();

    // Whitelist `dirname` so pre-flight approves without an interactive handler.
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    let md = Markdown::try_from(file_path.as_path()).unwrap();
    let overrides = serde_json::json!({ "spec": "fixes/x/spec.md" });
    let compose_opts = ComposeOptions::new()
        .with_source_file(&file_path)
        .with_set_overrides(overrides.clone());

    let options = ShellApprovalOptions {
        policy_root: Some(temp_dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };

    // Pre-flight must discover the RESOLVED command, not the raw template.
    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_opts), &options, None, None).unwrap();
    assert!(
        result.approved_commands.contains("dirname fixes/x/spec.md"),
        "pre-approved set must contain the resolved command; got: {:?}",
        result.approved_commands
    );

    // Execution against the pre-approved set must succeed — no NotPreApproved.
    let compose_with_approval = ComposeOptions::new()
        .with_source_file(&file_path)
        .with_set_overrides(overrides)
        .with_pre_approved_commands(result.approved_commands);
    let (composed, _) = md.compose_with(compose_with_approval).unwrap();
    assert!(
        composed.content().contains("fixes/x"),
        "composed body should contain the resolved dir; got: {:?}",
        composed.content()
    );
}

#[test]
fn full_flow_blacklisted_command_aborts_preflight() {
    let content = "# Test\n::shell rm -rf /\n";
    let md: Markdown = content.into();
    let compose_opts = ComposeOptions::new();

    let temp_dir = tempfile::TempDir::new().unwrap();
    let options = ShellApprovalOptions {
        policy_root: Some(temp_dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };

    let err = resolve_shell_approvals(Some(&md), Some(&compose_opts), &options, None, None);
    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(msg.contains("blacklisted"), "got: {msg}");
    // Compose was never called — session would never start
}

#[test]
fn full_flow_unapproved_command_rejected_at_compose_time() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");
    std::fs::write(
        &file_path,
        "# Test\n::shell echo hello\n::shell echo sneaky\n",
    )
    .unwrap();

    // Only pre-approve "echo hello", not "echo sneaky"
    let mut approved = std::collections::HashSet::new();
    approved.insert("echo hello".to_string());

    let options = ComposeOptions::new()
        .with_source_file(&file_path)
        .with_pre_approved_commands(approved);

    let md = Markdown::try_from(file_path.as_path()).unwrap();
    let err = md.compose_with(options).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not pre-approved"), "got: {msg}");
}

#[test]
fn allow_once_populates_cache_without_persisting() {
    let md: Markdown = "# Test\n::shell echo test-once\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None).unwrap();

    assert_eq!(result.total_discovered, 1);
    assert_eq!(result.user_approved, 1);
    assert!(result.approved_commands.contains("echo test-once"));
}

#[test]
fn deny_returns_shell_command_denied_error() {
    let md: Markdown = "# Test\n::shell echo test-deny\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::Deny));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let result = resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None);

    assert!(result.is_err());
    assert!(
        matches!(
            result.unwrap_err(),
            CompositionError::ShellCommandDenied { .. }
        ),
        "expected ShellCommandDenied"
    );
}

#[test]
fn warm_cache_prevents_second_handler_invocation() {
    let md: Markdown = "# Test\n::shell echo cached\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    // First call: handler invoked
    let result1 =
        resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None).unwrap();
    assert_eq!(result1.user_approved, 1);
    assert_eq!(handler.calls(), 1);

    // Second call: cache hit, handler NOT invoked
    let result2 =
        resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None).unwrap();
    assert_eq!(result2.total_discovered, 1);
    assert_eq!(
        handler.calls(),
        1,
        "handler should not be called again — cache hit"
    );
}

#[test]
fn interactive_handler_is_invoked_for_non_whitelisted_command() {
    let md: Markdown = "# Test\n::shell curl https://example.com\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let result =
        resolve_shell_approvals(Some(&md), Some(&compose_options), &options, None, None).unwrap();

    assert_eq!(
        handler.calls(),
        1,
        "handler must be invoked for non-whitelisted command"
    );
    assert!(
        result
            .approved_commands
            .contains("curl https://example.com")
    );
    assert_eq!(result.user_approved, 1);
    assert_eq!(result.already_whitelisted, 0);
}

// --- Shared approval cache across sequence steps -----------------------
//
// The sequence orchestrator builds a fresh `ShellApprovalOptions` per
// step (via `build_harness_shell_options_with_cache`) and wires the
// SAME `Arc<Mutex<HashMap>>` approval cache into each one. These
// tests exercise that exact pattern: two independent options structs
// sharing only their approval cache via `Arc::clone`. Each covers a
// different command source so cross-step reuse is proven for every
// path the sequence runner threads through pre-flight.

fn shared_cache_options(
    dir: &tempfile::TempDir,
    handler: Arc<dyn ShellApprovalHandler>,
    cache: Arc<
        Mutex<std::collections::HashMap<String, crate::harness::shell::CachedApprovalDecision>>,
    >,
) -> ShellApprovalOptions {
    ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler),
        approval_cache: cache,
        ..Default::default()
    }
}

#[test]
fn shared_cache_across_distinct_options_prevents_reprompt() {
    // Mirrors the sequence runner: each "step" builds a fresh
    // ShellApprovalOptions but reuses the same approval cache via
    // Arc::clone. A non-whitelisted template `::shell` command
    // approved on step 1 must NOT re-prompt on step 2.
    let md: Markdown = "# Test\n::shell curl https://example.com\n".into();
    let compose_options = ComposeOptions::new();

    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
    let shared_cache = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // Step 1: fresh options, cache wired in.
    let step1 = shared_cache_options(&dir, handler.clone(), Arc::clone(&shared_cache));
    let r1 = resolve_shell_approvals(Some(&md), Some(&compose_options), &step1, None, None).unwrap();
    assert_eq!(r1.user_approved, 1);
    assert_eq!(handler.calls(), 1, "step 1 must prompt once");

    // Step 2: BRAND NEW options, same Arc-cloned cache. Cache hit.
    let step2 = shared_cache_options(&dir, handler.clone(), Arc::clone(&shared_cache));
    let r2 = resolve_shell_approvals(Some(&md), Some(&compose_options), &step2, None, None).unwrap();
    assert_eq!(r2.total_discovered, 1);
    assert_eq!(
        handler.calls(),
        1,
        "step 2 must NOT prompt again — shared cache should be hit"
    );
}

#[test]
fn approval_request_carries_real_source_provenance() {
    use darkmatter::markdown::compose::ComposeSource;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("template.md");
    std::fs::write(&file_path, "# Test\n::shell curl https://example.com\n").unwrap();

    let md = Markdown::try_from(file_path.as_path()).unwrap();
    let compose_opts = ComposeOptions::new().with_source_file(&file_path);

    let handler = Arc::new(CapturingHandler::new());
    let options = ShellApprovalOptions {
        policy_root: Some(temp_dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let _result =
        resolve_shell_approvals(Some(&md), Some(&compose_opts), &options, None, None).unwrap();

    let requests = handler.captured_requests();
    assert_eq!(requests.len(), 1, "handler should be called once");

    let req = &requests[0];
    // The source should be the real file, not a dummy path
    match &req.source {
        ComposeSource::File(path) => {
            assert_eq!(
                path, &file_path,
                "source should be the template file, not a dummy"
            );
        }
        other => panic!("expected File source, got: {other:?}"),
    }
    assert!(
        req.origin.line_number() > 0,
        "line should be the real line number, not 0"
    );
}

// --- Lifecycle shell read-side resolution: launch-area fallback ---------
//
// A lifecycle `shell` command whose `{{ }}` span calls a read-side
// filesystem function (`file_exists`) against a launch-area-relative path
// must resolve via the threaded `file_ref_fallback_dir`, not the prompt
// directory alone. The regression below keeps the prompt dir, the
// launch-area fallback, and the ambient CWD all distinct, with `spec.md`
// present ONLY under the fallback, and proves the resolved (stamped)
// command depends on the fallback being supplied — independently of the
// post-launch process CWD.

use darkmatter::markdown::compose::ComposeContext;

/// RAII guard that switches the process CWD and restores it on drop
/// (including on panic). Tests using it are `#[serial_test::serial]` to
/// avoid racing on process-global CWD with other CWD-mutating tests.
struct CwdGuard {
    prior: std::path::PathBuf,
}

impl CwdGuard {
    fn enter(dir: &std::path::Path) -> Self {
        let prior = std::env::current_dir().expect("read CWD");
        std::env::set_current_dir(dir).expect("set CWD");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// Builds a `start` lifecycle config with a single positional `shell`
/// action whose command interpolates `file_exists(spec)`, plus the
/// effective frontmatter that supplies `spec`.
fn lifecycle_with_file_exists_shell()
-> (crate::composition::lifecycle::LifecycleConfig, serde_json::Value) {
    let frontmatter = serde_json::json!({ "spec": "spec.md" });
    let fm_with_event = serde_json::json!({
        "spec": "spec.md",
        "start": {
            "stack": [{"action": {"shell": "echo {{ file_exists(spec) }}"}}]
        }
    });
    let config = crate::composition::lifecycle::parse_lifecycle_config(
        &fm_with_event,
        Path::new("<test>"),
    )
    .expect("lifecycle config parses");
    (config, frontmatter)
}

/// Reads the resolved (stamped) command string from the first `start`
/// stack action.
fn resolved_start_shell_command(
    config: &crate::composition::lifecycle::LifecycleConfig,
) -> String {
    let stack = config
        .stack(LifecycleSignal::Start)
        .expect("start stack present");
    match &stack[0].actions[0].kind {
        LifecycleActionKind::Shell(shell) => match &shell.command {
            Expr::StringLiteral(s) => s.clone(),
            other => panic!("expected string-literal command, got: {other:?}"),
        },
        other => panic!("expected shell action, got: {other:?}"),
    }
}

#[test]
#[serial_test::serial(preflight_cwd)]
fn lifecycle_shell_read_side_resolves_via_launch_area_fallback() {
    let doc_dir = tempfile::TempDir::new().unwrap();
    let launch_dir = tempfile::TempDir::new().unwrap();
    let unrelated = tempfile::TempDir::new().unwrap();
    // spec.md exists ONLY under the launch-area fallback — not the prompt
    // (document) directory, not the ambient CWD.
    std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source_path = doc_dir.path().join("prompt.md");
    let (mut config, frontmatter) = lifecycle_with_file_exists_shell();

    // Switch the ambient CWD elsewhere to prove resolution is independent
    // of any post-launch chdir.
    let _cwd = CwdGuard::enter(unrelated.path());

    resolve_lifecycle_shell_commands(
        &mut config,
        &frontmatter,
        &ComposeContext::capture(),
        &source_path,
        Some(launch_dir.path()),
    )
    .expect("resolution with the launch-area fallback must succeed");

    assert_eq!(
        resolved_start_shell_command(&config),
        "echo true",
        "file_exists(spec) must see spec.md via the launch-area fallback",
    );
}

/// Same setup WITHOUT the fallback: `file_exists(spec)` cannot find the
/// launch-only file via the prompt dir or the unrelated CWD, so it resolves
/// to `false`. This confirms the test above passes because of the fallback,
/// not because the file is reachable some other way. Before the fix this
/// was the only code path, so the launch-relative reference resolved wrong.
#[test]
#[serial_test::serial(preflight_cwd)]
fn lifecycle_shell_read_side_without_fallback_misses_launch_area_file() {
    let doc_dir = tempfile::TempDir::new().unwrap();
    let launch_dir = tempfile::TempDir::new().unwrap();
    let unrelated = tempfile::TempDir::new().unwrap();
    std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source_path = doc_dir.path().join("prompt.md");
    let (mut config, frontmatter) = lifecycle_with_file_exists_shell();

    let _cwd = CwdGuard::enter(unrelated.path());

    resolve_lifecycle_shell_commands(
        &mut config,
        &frontmatter,
        &ComposeContext::capture(),
        &source_path,
        None,
    )
    .expect("resolution still succeeds; file just resolves to absent");

    assert_eq!(
        resolved_start_shell_command(&config),
        "echo false",
        "without the fallback the launch-only spec.md is unreachable",
    );
}
