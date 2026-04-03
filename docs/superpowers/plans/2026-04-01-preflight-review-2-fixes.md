# Preflight Review-2 Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three high-severity regressions identified in `claudine/features/2026-04-01-preflight/review-2.md`: interactive compose sessions failing on unapproved commands, synthetic provenance in approval prompts, and false-positive audit failures from raw source-page re-audit after composition.

**Architecture:** Each fix is independent and targets a different layer: CLI entrypoint wiring (compose.rs), library approval plumbing (shell.rs + preflight.rs), and harness loop logic (wrap/mod.rs). All three can be implemented in any order. TDD: write failing test first, then fix.

**Tech Stack:** Rust, claudine (harness/preflight/composition), darkmatter (shell expansion types), tempfile (tests)

---

## File Map

### Modified Files

| File | Responsibility | Finding |
|------|---------------|---------|
| `claudine/cli/src/commands/compose.rs` | Thread `args.interactive` into `build_harness_shell_options` for both entrypoints | #1 |
| `claudine/lib/src/harness/shell.rs` | Accept optional provenance params in `validate_and_approve_command_parts`; use them in `ShellApprovalRequest` | #2 |
| `claudine/lib/src/composition/preflight.rs` | Pass real `(source_file, line)` provenance through to `validate_and_approve_command_parts` | #2 |
| `claudine/lib/src/harness/audit.rs` | Pass provenance from `AuditedCommand` through to `validate_and_approve_command_parts` | #2 |
| `claudine/cli/src/commands/wrap/mod.rs` | Skip source-page `::shell` re-audit for composition-driven flows | #3 |

### Test Files (modified, existing `#[cfg(test)]` modules)

| File | What is tested |
|------|---------------|
| `claudine/lib/src/composition/preflight.rs` | Approval handler receives real provenance; interactive handler invoked when present |
| `claudine/lib/src/harness/shell.rs` | `validate_and_approve_command_parts` passes provenance through to `ShellApprovalRequest` |
| `claudine/lib/src/harness/audit.rs` | `audit_shell_commands` passes provenance through |

---

## Task 1: Thread `args.interactive` into compose preflight (Finding #1)

The `run_compose_inner` and `run_inline_compose_inner` functions both hardcode `false` for the `interactive` parameter when calling `build_harness_shell_options`. This means even when the user passes `--interactive`, no approval handler is created, and unapproved commands fail with "no approval handler is available" instead of prompting.

**Files:**
- Modify: `claudine/lib/src/composition/preflight.rs` (test module, ~line 489)
- Modify: `claudine/cli/src/commands/compose.rs:281` and `:386`

- [ ] **Step 1: Write the failing test**

Add a test to `claudine/lib/src/composition/preflight.rs` in the existing `#[cfg(test)] mod tests` block that proves the approval handler is invoked and the command is approved when a handler is present for a non-whitelisted command.

This test already partially exists (`allow_once_populates_cache_without_persisting`), but we need one that captures the `ShellApprovalRequest` to verify the handler **was called** and could have prompted. The key difference from existing tests: this one uses a handler that records whether it was called, paired with a command that is NOT whitelisted, confirming interactive approval works end-to-end.

```rust
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
        resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();

    assert_eq!(handler.calls(), 1, "handler must be invoked for non-whitelisted command");
    assert!(result.approved_commands.contains("curl https://example.com"));
    assert_eq!(result.user_approved, 1);
    assert_eq!(result.already_whitelisted, 0);
}
```

- [ ] **Step 2: Run test to verify it passes (baseline)**

Run: `cargo test -p claudine interactive_handler_is_invoked -- --nocapture`

Expected: PASS (this test validates the library layer, which already works correctly).

This test establishes the baseline: the library correctly invokes handlers. The actual bug is at the CLI layer where `interactive=false` prevents the handler from being created.

- [ ] **Step 3: Fix `run_compose_inner` to thread `args.interactive`**

In `claudine/cli/src/commands/compose.rs`, change line 281 from:

```rust
    let approval_options =
        super::wrap::build_harness_shell_options(&source.resolved_path, None, false);
```

to:

```rust
    let approval_options =
        super::wrap::build_harness_shell_options(&source.resolved_path, None, args.interactive);
```

- [ ] **Step 4: Fix `run_inline_compose_inner` to thread `args.interactive`**

In `claudine/cli/src/commands/compose.rs`, change line 386 from:

```rust
    let approval_options =
        super::wrap::build_harness_shell_options(&source.resolved_path, None, false);
```

to:

```rust
    let approval_options =
        super::wrap::build_harness_shell_options(&source.resolved_path, None, args.interactive);
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p claudine composition::preflight`

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add claudine/cli/src/commands/compose.rs claudine/lib/src/composition/preflight.rs
git commit -m "fix(claudine): thread args.interactive into compose preflight approval

Both run_compose_inner and run_inline_compose_inner were hardcoding
interactive=false when building shell approval options. This meant
--interactive compose sessions could never prompt for unapproved
commands, failing with 'no approval handler is available' instead."
```

---

## Task 2: Push real provenance into `ShellApprovalRequest` (Finding #2)

`validate_and_approve_command_parts()` constructs a `ShellApprovalRequest` with `ComposeSource::File(root.join("dummy"))` and `line: 0` (shell.rs:98-101, 170-172). The real `(source_file, line)` provenance is available in callers (`resolve_shell_approvals` and `audit_shell_commands`) but never passed through.

The fix adds two optional provenance parameters to `validate_and_approve_command_parts` and threads real values from both callers.

**Files:**
- Modify: `claudine/lib/src/harness/shell.rs:74-180` (function signature + body)
- Modify: `claudine/lib/src/composition/preflight.rs:106` (caller)
- Modify: `claudine/lib/src/harness/audit.rs` (caller in `audit_shell_commands`)
- Test: `claudine/lib/src/harness/shell.rs` (test module)
- Test: `claudine/lib/src/composition/preflight.rs` (test module)

- [ ] **Step 1: Write the failing test for shell.rs**

Add a test in `claudine/lib/src/harness/shell.rs`'s `#[cfg(test)] mod tests` block that captures the `ShellApprovalRequest` passed to the handler and asserts `source` and `line` match the provided provenance.

First, create a capturing mock handler. Add to the test module:

```rust
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};

struct CapturingHandler {
    captured: Arc<Mutex<Option<ShellApprovalRequest>>>,
}

impl CapturingHandler {
    fn new() -> Self {
        Self {
            captured: Arc::new(Mutex::new(None)),
        }
    }

    fn captured_request(&self) -> ShellApprovalRequest {
        self.captured.lock().unwrap().clone().expect("handler was never called")
    }
}

impl ShellApprovalHandler for CapturingHandler {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        *self.captured.lock().unwrap() = Some(request);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}
```

Then write the test:

```rust
#[test]
fn provenance_is_passed_through_to_approval_request() {
    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(CapturingHandler::new());
    let options = ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let source_file = PathBuf::from("/home/user/project/template.md");
    let source_line = 42usize;

    let _ = validate_and_approve_command_parts(
        &["echo".to_string(), "hello".to_string()],
        &options,
        Some(&source_file),
        Some(source_line),
    );

    let captured = handler.captured_request();
    assert_eq!(
        captured.source,
        ComposeSource::File(source_file),
        "request should carry the real source file, not a dummy path"
    );
    assert_eq!(captured.line, 42, "request should carry the real line number");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine provenance_is_passed_through -- --nocapture`

Expected: FAIL — compilation error because `validate_and_approve_command_parts` does not accept the new parameters yet.

- [ ] **Step 3: Add provenance parameters to `validate_and_approve_command_parts`**

In `claudine/lib/src/harness/shell.rs`, change the function signature at line 74 from:

```rust
pub fn validate_and_approve_command_parts(
    parts: &[String],
    options: &ShellApprovalOptions,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
```

to:

```rust
pub fn validate_and_approve_command_parts(
    parts: &[String],
    options: &ShellApprovalOptions,
    source_file: Option<&Path>,
    source_line: Option<usize>,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
```

Add `use std::path::Path;` to the imports if not already present.

Then change the `source` construction at lines 98-101. Keep the original for policy resolution but create a separate display source for the approval request. Replace:

```rust
    // Resolve policy paths
    let source = match &options.policy_root {
        Some(root) => ComposeSource::File(root.join("dummy")),
        None => ComposeSource::Unknown,
    };
```

with:

```rust
    // Resolve policy paths — needs a file under the policy root for path resolution.
    let policy_source = match &options.policy_root {
        Some(root) => ComposeSource::File(root.join("dummy")),
        None => ComposeSource::Unknown,
    };

    // Display source for approval prompts — use real provenance when available.
    let display_source = match source_file {
        Some(path) => ComposeSource::File(path.to_path_buf()),
        None => policy_source.clone(),
    };
    let display_line = source_line.unwrap_or(0);
```

Update the `resolve_policy_paths` call at line 109 to use `policy_source`:

```rust
    let policy_paths = resolve_policy_paths(&shell_opts, &policy_source).map_err(|_| {
```

Update the `ShellApprovalRequest` construction at lines 170-180 to use `display_source` and `display_line`:

```rust
        let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
            source: display_source,
            line: display_line,
            raw_command: raw.clone(),
            executable: executable.to_string(),
            args: args.clone(),
            normalized_exact: normalized.clone(),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            alias_name: None,
        };
```

- [ ] **Step 4: Update caller in `preflight.rs`**

In `claudine/lib/src/composition/preflight.rs`, change line 106 from:

```rust
        match crate::harness::shell::validate_and_approve_command_parts(&parts, approval_options) {
```

to:

```rust
        match crate::harness::shell::validate_and_approve_command_parts(
            &parts,
            approval_options,
            Some(source_file.as_path()),
            Some(*line),
        ) {
```

- [ ] **Step 5: Update caller in `audit.rs`**

In `claudine/lib/src/harness/audit.rs`, inside `audit_shell_commands`, find the call to `validate_and_approve_command_parts` and add the provenance parameters. The `AuditedCommand` has a `source` field but no direct file path. For harness audit commands, provenance is less critical (the harness plan source_path is the relevant file). Pass `None` for both to preserve existing behavior:

Find the call that looks like:

```rust
        match crate::harness::shell::validate_and_approve_command_parts(&parts, options) {
```

and change to:

```rust
        match crate::harness::shell::validate_and_approve_command_parts(&parts, options, None, None) {
```

- [ ] **Step 6: Update existing tests in `shell.rs`**

All existing tests in `shell.rs` that call `validate_and_approve_command_parts` need the two new parameters. Add `None, None` to each call site. Search the test module for `validate_and_approve_command_parts` and append `None, None` after `&options`. Example — if a test has:

```rust
let result = validate_and_approve_command_parts(&parts, &options);
```

change to:

```rust
let result = validate_and_approve_command_parts(&parts, &options, None, None);
```

- [ ] **Step 7: Run all tests to verify they pass**

Run: `cargo test -p claudine harness::shell && cargo test -p claudine composition::preflight`

Expected: All tests PASS, including the new `provenance_is_passed_through_to_approval_request`.

- [ ] **Step 8: Write the provenance assertion test for preflight**

Add a test in `claudine/lib/src/composition/preflight.rs` that uses a `CapturingHandler` (same pattern as shell.rs) to verify that `resolve_shell_approvals` passes the real source file and line to the approval handler.

First, add the `CapturingHandler` to preflight's test module:

```rust
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
```

Then write the test:

```rust
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
        resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options).unwrap();

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
    assert!(req.line > 0, "line should be the real line number, not 0");
}
```

- [ ] **Step 9: Run test to verify it passes**

Run: `cargo test -p claudine approval_request_carries_real_source_provenance -- --nocapture`

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add claudine/lib/src/harness/shell.rs claudine/lib/src/composition/preflight.rs claudine/lib/src/harness/audit.rs
git commit -m "fix(claudine): pass real provenance through to ShellApprovalRequest

validate_and_approve_command_parts was constructing a ShellApprovalRequest
with ComposeSource::File(root.join(\"dummy\")) and line: 0. Now accepts
optional source_file and source_line params, and resolve_shell_approvals
passes the real (source_file, line) tuples from discovery through to the
approval handler. The user-facing prompt now shows the correct file and
line number."
```

---

## Task 3: Skip source-page re-audit for composition flows (Finding #3)

The harness loop reads raw source text and passes it to `collect_auditable_commands()`, which re-parses `::shell` directives directly. For composition-driven flows (Compose/Inline modes), this reintroduces directives hidden by false `::block`s that preflight correctly excluded. The fix: only pass source text for Passthrough mode.

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs:2251-2254`
- Test: `claudine/lib/src/harness/audit.rs` (test module)

- [ ] **Step 1: Write the failing test**

Add a test in `claudine/lib/src/harness/audit.rs` that proves `collect_auditable_commands` with `source_text: None` does NOT include source-page directives, while `source_text: Some(...)` does. This validates the behavior we will rely on.

```rust
#[test]
fn collect_auditable_commands_excludes_source_directives_when_none() {
    let plan = empty_plan_with_pre_check("echo", &["preflight"]);

    // With source text: includes both harness + source-page commands
    let with_source =
        collect_auditable_commands(&plan, Some("# Test\n::shell echo hidden\n")).unwrap();
    let source_page_count = with_source
        .iter()
        .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
        .count();
    assert_eq!(source_page_count, 1, "source text should produce source-page commands");

    // Without source text: only harness commands
    let without_source = collect_auditable_commands(&plan, None).unwrap();
    let source_page_count = without_source
        .iter()
        .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
        .count();
    assert_eq!(
        source_page_count, 0,
        "None source_text must produce zero source-page commands"
    );

    // Harness commands still present in both
    assert!(
        without_source.iter().any(|c| matches!(c.source, AuditedCommandSource::PreCheck(_))),
        "harness pre-check commands should still be collected"
    );
}
```

This test needs a helper. Add `empty_plan_with_pre_check` to the audit.rs test module:

```rust
fn empty_plan_with_pre_check(executable: &str, args: &[&str]) -> HarnessPlan {
    let mut plan = HarnessPlan {
        source_path: PathBuf::from("/tmp/test.md"),
        timeout: None,
        pre_checks: Vec::new(),
        post_checks: Vec::new(),
        handlers: HandlerTable::default(),
        programmatic_handler: None,
    };
    plan.pre_checks.push(ValidationRule {
        id: ValidationRuleId(0),
        event: ValidationEvent::ShellCommand,
        phase: ValidationPhase::Both,
        kind: ValidationKind::ShellCommand {
            command: ApprovedRuntimeCommand {
                raw: format!("{} {}", executable, args.join(" ")),
                executable: executable.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
            },
            show_stdout: false,
            show_stderr: false,
        },
        message_template: None,
        subject_key: None,
    });
    plan
}
```

- [ ] **Step 2: Run test to verify it passes (baseline)**

Run: `cargo test -p claudine collect_auditable_commands_excludes_source_directives -- --nocapture`

Expected: PASS. This validates the existing `collect_auditable_commands` API already supports `None` correctly — it's the harness loop that needs to use it.

- [ ] **Step 3: Fix the harness loop to skip source-page audit for composition flows**

In `claudine/cli/src/commands/wrap/mod.rs`, change lines 2251-2254 from:

```rust
        // Shell audit preflight
        let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();
        let auditable =
            claudine::harness::collect_auditable_commands(&plan, source_text.as_deref())?;
```

to:

```rust
        // Shell audit preflight.
        // Composition flows (Compose/Inline) already preflight ::shell directives
        // during composition — re-parsing raw source would reintroduce commands
        // hidden by false ::block directives.  Only passthrough mode needs raw
        // source-page audit.
        let source_text = match prompt_state.mode {
            HarnessPromptMode::Passthrough => {
                std::fs::read_to_string(&prompt_state.source_path).ok()
            }
            _ => None,
        };
        let auditable =
            claudine::harness::collect_auditable_commands(&plan, source_text.as_deref())?;
```

- [ ] **Step 4: Run all harness and preflight tests**

Run: `cargo test -p claudine harness && cargo test -p claudine composition::preflight`

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs claudine/lib/src/harness/audit.rs
git commit -m "fix(claudine): skip source-page re-audit for composition flows

The harness loop was unconditionally reading raw source text and passing
it to collect_auditable_commands(), which re-parsed ::shell directives
directly. For composition flows, this reintroduced commands hidden by
false ::block directives that preflight correctly excluded. Now only
Passthrough mode reads raw source text; Compose and Inline modes pass
None since their ::shell directives were already preflighted."
```

---

## Task 4: Final verification

- [ ] **Step 1: Run all claudine tests**

Run: `cargo test -p claudine`

Expected: All tests PASS.

- [ ] **Step 2: Run darkmatter discovery tests**

Run: `cargo test -p darkmatter shell_expansion::discovery`

Expected: All tests PASS.

- [ ] **Step 3: Run lint**

Run: `cargo clippy -p claudine -p claudine-cli -- -D warnings`

Expected: No warnings.
