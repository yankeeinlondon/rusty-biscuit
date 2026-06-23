//! Pre-flight shell command approval for provider sessions.
//!
//! Scans all sources of shell commands (template directives and lifecycle
//! stacks), checks them against the whitelist, prompts the user for any that
//! need approval, and returns the full pre-approved set.

use std::collections::HashSet;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;

use crate::composition::error::CompositionError;
use crate::composition::lifecycle::{LifecycleConfig, collect_lifecycle_shell_commands};
use crate::harness::shell::{ShellApprovalOptions, tokenize_words_strict};

/// Result of pre-flight shell command approval.
#[derive(Debug)]
pub struct PreFlightResult {
    /// Normalized command strings approved for execution.
    pub approved_commands: HashSet<String>,
    /// Total commands discovered across all sources.
    pub total_discovered: usize,
    /// Commands that were already whitelisted.
    pub already_whitelisted: usize,
    /// Commands the user approved interactively.
    pub user_approved: usize,
}

/// Scans all sources of shell commands, checks whitelist, prompts user,
/// returns the full pre-approved set.
///
/// ## Sources
///
/// 1. Template `::shell` directives (via Darkmatter's document graph walker)
/// 2. Lifecycle stack shell commands (from every reachable `action: shell`
///    across all seven lifecycle events)
///
/// ## Errors
///
/// - `ShellCommandDenied` if the user denies any command
/// - `PreFlightDiscoveryFailed` if Darkmatter's document graph walk fails
/// - `PreFlightFailed` for blacklisted commands or missing approval handler
///
/// ## Arguments
///
/// * `markdown` — composed Markdown for template `::shell` discovery.
/// * `compose_options` — Darkmatter compose options for the template walker.
/// * `approval_options` — shell approval policy, handler, and cache.
/// * `lifecycle` — parsed lifecycle configuration; when present, every
///   reachable `action: shell` command (and `on_error` command) across all
///   seven lifecycle events is collected and audited alongside the other
///   sources. `lifecycle_source_path` names the composition source file for
///   diagnostics.
/// * `lifecycle_source_path` — source path for lifecycle-stack shell
///   commands; ignored when `lifecycle` is `None`.
pub fn resolve_shell_approvals(
    markdown: Option<&Markdown>,
    compose_options: Option<&ComposeOptions>,
    approval_options: &ShellApprovalOptions,
    lifecycle: Option<&LifecycleConfig>,
    lifecycle_source_path: Option<&std::path::Path>,
) -> Result<PreFlightResult, CompositionError> {
    let mut all_commands: Vec<(String, std::path::PathBuf, usize)> = Vec::new();

    // -- Source 1: Template ::shell directives ---------------------------------
    // Darkmatter discovers condition-blind: every command that could run under
    // any document state (including dead branches). Claudine authorizes the
    // union of these plus lifecycle stack commands below.
    if let (Some(md), Some(opts)) = (markdown, compose_options) {
        let preflight = md
            .compose_preflight(opts)
            .map_err(CompositionError::PreFlightDiscoveryFailed)?;
        for entry in &preflight.entries {
            all_commands.push((
                entry.normalized.clone(),
                entry.source_file.clone(),
                entry.origin.line_number(),
            ));
        }
    }

    // -- Source 2: Lifecycle stack shell commands ------------------------------
    // Condition-blind, matching the template posture: every reachable
    // `action: shell` (and its `on_error`) is gathered regardless of the
    // `when:` guard, because the guard may evaluate differently under
    // document states the audit cannot predict.
    if let Some(lifecycle_config) = lifecycle {
        let source_file = lifecycle_source_path
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("<lifecycle-stack>"));
        for (command, _property_path) in collect_lifecycle_shell_commands(lifecycle_config) {
            all_commands.push((command, source_file.clone(), 0));
        }
    }

    // -- Deduplicate -----------------------------------------------------------
    let unique: Vec<(String, std::path::PathBuf, usize)> = {
        let mut seen = HashSet::new();
        all_commands
            .into_iter()
            .filter(|(normalized, _, _)| seen.insert(normalized.clone()))
            .collect()
    };

    let total_discovered = unique.len();
    let mut approved = HashSet::new();

    // Snapshot the approval cache size before we start so we can distinguish
    // whitelisted commands from interactively-approved ones.
    let cache_size_before = approval_options
        .approval_cache
        .lock()
        .map(|c| c.len())
        .unwrap_or(0);

    // -- Check each command against policy -------------------------------------
    for (normalized, source_file, line) in &unique {
        // Split normalized command back into parts for the existing validator.
        let parts: Vec<String> =
            tokenize_words_strict(normalized).unwrap_or_else(|_| vec![normalized.clone()]);

        match crate::harness::shell::validate_and_approve_command_parts(
            &parts,
            approval_options,
            Some(source_file.as_path()),
            Some(*line),
        ) {
            Ok(_) => {
                approved.insert(normalized.clone());
            }
            Err(crate::harness::error::HarnessError::ShellCommandDenied { command }) => {
                if approval_options.approval_handler.is_some() {
                    // Handler was available but user denied
                    return Err(CompositionError::ShellCommandDenied {
                        command,
                        source_file: source_file.clone(),
                        line: *line,
                    });
                }
                // No handler -- cannot get approval. Under `--dry-run` the
                // CI/non-TTY gate names the offending command and points at
                // the two ways to proceed (spec: Non-TTY Behavior).
                if approval_options.dry_run {
                    return Err(CompositionError::PreFlightFailed(format!(
                        "Cannot dry-run: shell command '{command}' requires interactive approval. \
                         Run with --yolo to auto-approve, or pre-approve the command in your \
                         configuration."
                    )));
                }
                let location = if *line > 0 {
                    format!("{}:{}", source_file.display(), line)
                } else {
                    source_file.display().to_string()
                };
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' at {location} requires approval but no approval handler \
                     is available. Add to whitelist or run interactively."
                )));
            }
            Err(crate::harness::error::HarnessError::ShellCommandBlacklisted {
                command,
                reason,
            }) => {
                let location = if *line > 0 {
                    format!("{}:{}", source_file.display(), line)
                } else {
                    source_file.display().to_string()
                };
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' at {location} is blacklisted: {reason}"
                )));
            }
            Err(e) => {
                return Err(CompositionError::PreFlightFailed(e.to_string()));
            }
        }
    }

    // Count how many were newly approved by the handler vs already whitelisted.
    let cache_size_after = approval_options
        .approval_cache
        .lock()
        .map(|c| c.len())
        .unwrap_or(0);
    let user_approved = cache_size_after.saturating_sub(cache_size_before);
    let already_whitelisted = total_discovered.saturating_sub(user_approved);

    Ok(PreFlightResult {
        approved_commands: approved,
        total_discovered,
        already_whitelisted,
        user_approved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            matches!(err, CompositionError::PreFlightFailed(ref msg) if msg.contains("blacklisted")),
            "expected PreFlightFailed with blacklisted, got: {err}"
        );
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
}
