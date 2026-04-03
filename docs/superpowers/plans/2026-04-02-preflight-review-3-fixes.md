# Preflight Review-3 Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address the three coverage gaps and one medium-risk code issue identified in `claudine/features/2026-04-01-preflight/review-3.md`: freeze shell approvals after initial preflight so redirect/retry can't prompt, lock in the false `::block` composition regression fix with a test, and add CLI integration tests for compose preflight with provenance.

**Architecture:** Four independent tasks: (1) code fix to freeze the approval handler after the first successful audit for composition flows, (2) library test proving the freeze denies new commands on redirect, (3) regression test for the false `::block` composition fix, (4) CLI integration test for compose preflight provenance. All changes are in `claudine/` only.

**Tech Stack:** Rust, claudine (harness, preflight, composition), darkmatter (shell expansion), tempfile (tests), assert_cmd (CLI tests)

---

## File Map

### Modified Files

| File | Responsibility | Review Item |
|------|---------------|-------------|
| `claudine/cli/src/commands/wrap/mod.rs:276-315` | Add `freeze_shell_approvals()` to `CachedHarnessLoopContext`; call it after first audit passes for composition flows | Medium risk: redirect/retry |
| `claudine/lib/src/harness/audit.rs` (test module) | Regression test for false `::block` source scan | Coverage gap #2 |
| `claudine/lib/src/harness/shell.rs` (test module) | Test that frozen approval options deny new commands | Medium risk: redirect/retry |
| `claudine/cli/tests/wrap_commands.rs` | CLI integration test for compose preflight provenance | Coverage gap #1 |

---

## Task 1: Freeze approval handler after first audit for composition flows

The harness loop re-runs `audit_shell_commands` on every iteration. For composition flows (Compose/Inline), all shell approvals are resolved during preflight before the loop begins. But if a handler resolution redirects to a different file, the next loop iteration could surface a new command not in the cache, triggering a live approval prompt mid-workflow. The spec says "all shell approvals are resolved before the provider workflow begins."

The fix: after the first successful audit for composition flows, strip the `approval_handler` from the shell options so subsequent iterations operate in deny-only mode. Cached and whitelisted commands still pass; new uncached commands get denied without prompting.

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs:276-315` (CachedHarnessLoopContext)
- Modify: `claudine/cli/src/commands/wrap/mod.rs:2330-2332` (harness loop, between audit block and pre-checks)

- [ ] **Step 1: Add `freeze_shell_approvals` method to `CachedHarnessLoopContext`**

In `claudine/cli/src/commands/wrap/mod.rs`, add a new method to the `impl CachedHarnessLoopContext` block (after line 314, before the closing `}`):

```rust
    /// Strip the interactive approval handler so subsequent harness-loop
    /// iterations operate in deny-only mode.  Cached and whitelisted
    /// commands still pass; new uncached commands are denied without
    /// prompting.  This enforces the spec contract: "all shell approvals
    /// are resolved before the provider workflow begins."
    fn freeze_shell_approvals(&mut self) {
        self.shell_options.approval_handler = None;
    }
```

- [ ] **Step 2: Call `freeze_shell_approvals` after first successful audit for composition flows**

In `claudine/cli/src/commands/wrap/mod.rs`, insert the following between line 2330 (closing `}` of the `if !audit_report.all_passed()` block) and line 2332 (`let pre_report = ...`):

```rust
        // Composition flows resolved all shell approvals during preflight.
        // Freeze the approval set so redirect/retry iterations cannot
        // trigger new interactive prompts — only cached/whitelisted
        // commands pass; new uncached commands are denied.  Passthrough
        // mode has no prior preflight so its handler stays active.
        if attempt == 1 && !matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            harness_context.freeze_shell_approvals();
        }
```

- [ ] **Step 3: Run tests to verify nothing is broken**

Run: `cargo test -p claudine harness && cargo test -p claudine composition::preflight`

Expected: All existing tests PASS. The new code path only activates in the live harness loop, which is not exercised by existing unit tests.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs
git commit -m "fix(claudine): freeze shell approvals after first audit for composition flows

After the initial preflight resolves all shell approvals, strip the
interactive approval handler so redirect/retry loop iterations operate
in deny-only mode.  Cached and whitelisted commands still pass; new
uncached commands are denied without prompting.  Passthrough mode
(no prior preflight) is unaffected."
```

---

## Task 2: Library test proving frozen approvals deny new commands

This test validates the mechanism Task 1 relies on: once `approval_handler` is `None`, a command that is not cached and not whitelisted is denied — even though the same `ShellApprovalOptions` previously had a handler.

**Files:**
- Modify: `claudine/lib/src/harness/shell.rs` (test module, after the `provenance_is_passed_through_to_approval_request` test)

- [ ] **Step 1: Write the test**

Add to the `#[cfg(test)] mod tests` block in `claudine/lib/src/harness/shell.rs`, after the last existing test:

```rust
    #[test]
    fn frozen_options_deny_new_commands_after_cache_populated() {
        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CountingApprovalHandler {
            approvals: AtomicUsize::new(0),
        });
        let mut options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        // Approve a command while the handler is active — populates the cache.
        let first = validate_and_approve_command("echo hello", &options);
        assert!(first.is_ok(), "should approve with active handler");
        assert_eq!(handler.approvals(), 1);

        // Freeze: strip the handler (simulates CachedHarnessLoopContext::freeze_shell_approvals).
        options.approval_handler = None;

        // Previously-approved command still passes via cache.
        let cached = validate_and_approve_command("echo hello", &options);
        assert!(cached.is_ok(), "cached command should still pass after freeze");

        // A NEW command that was never approved is denied — no handler to prompt.
        let new_cmd = validate_and_approve_command("curl https://example.com", &options);
        assert!(
            new_cmd.is_err(),
            "new command must be denied after freeze — no handler available"
        );
        assert!(matches!(
            new_cmd.unwrap_err(),
            HarnessError::ShellCommandDenied { .. }
        ));

        // Handler was only called once (for the original "echo hello"), never for "curl".
        assert_eq!(handler.approvals(), 1, "handler must not be called after freeze");
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p claudine frozen_options_deny_new_commands -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/harness/shell.rs
git commit -m "test(claudine): prove frozen approval options deny new commands

Validates the mechanism used by freeze_shell_approvals: after stripping
the handler, cached commands still pass via the approval cache, but any
new command not previously approved is denied without prompting."
```

---

## Task 3: Regression test for false `::block` composition fix

The recent commit `ef6e3cf2` fixed a bug where composition flows re-parsed raw source text and picked up `::shell` directives hidden by `::block false` sections. The fix passes `None` for `source_text` in Compose/Inline modes. This test locks that behavior in to prevent regression.

**Files:**
- Modify: `claudine/lib/src/harness/audit.rs` (test module, after the `collect_auditable_commands_excludes_source_directives_when_none` test)

- [ ] **Step 1: Write the test**

Add to the `#[cfg(test)] mod tests` block in `claudine/lib/src/harness/audit.rs`, after the last existing test:

```rust
    #[test]
    fn source_scan_finds_shell_hidden_by_false_block() {
        let plan = empty_plan();
        let source = "# Title\n::block false\n::shell curl https://example.com\n::block\nRegular text\n";

        // Raw source scan (Passthrough mode) picks up ::shell despite ::block false
        // because parse_directives does line-level scanning without block context.
        let with_source = collect_auditable_commands(&plan, Some(source)).unwrap();
        let source_count = with_source
            .iter()
            .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
            .count();
        assert_eq!(
            source_count, 1,
            "raw source scan should pick up ::shell inside ::block false (line-level scan)"
        );

        // Composition mode passes None — no source-page re-audit.
        // This is the fix from ef6e3cf2: composition flows must not re-parse raw
        // source because the false ::block hides the directive at the Darkmatter
        // level but not at the raw-text level.
        let without_source = collect_auditable_commands(&plan, None).unwrap();
        let source_count = without_source
            .iter()
            .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
            .count();
        assert_eq!(
            source_count, 0,
            "composition mode (None source_text) must not find source-page directives"
        );
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p claudine source_scan_finds_shell_hidden -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/harness/audit.rs
git commit -m "test(claudine): lock in false ::block composition regression fix

Proves that raw source scanning picks up ::shell inside ::block false
(because parse_directives does line-level scanning) while composition
mode correctly avoids re-scanning by passing None source_text.
Prevents regression of the fix from ef6e3cf2."
```

---

## Task 4: CLI integration test for compose preflight provenance

This test validates that the CLI correctly reports the source file name in preflight error messages when a `::shell` directive is not whitelisted. It exercises the full CLI → composition → preflight → error path without needing a PTY.

**Files:**
- Modify: `claudine/cli/tests/wrap_commands.rs` (after the `compose_uses_wrapper_grade_execution` test)

- [ ] **Step 1: Write the test**

Add to `claudine/cli/tests/wrap_commands.rs`, after the `compose_uses_wrapper_grade_execution` test (around line 1487):

```rust
#[cfg(unix)]
#[test]
fn compose_preflight_error_includes_source_provenance() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // Markdown with a ::shell directive that is NOT whitelisted.
    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: provenance test\n---\n::shell curl https://example.com\n",
    )
    .unwrap();

    // Provider binary (should never be reached — preflight should abort first).
    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\necho 'ERROR: provider should not run' >&2\nexit 99\n",
    );

    // Run without --interactive so preflight has no approval handler →
    // the non-whitelisted command triggers a clear error with provenance.
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    // Error message should mention the source file name (provenance).
    assert!(
        plain.contains("template.md"),
        "preflight error should include the source file name for provenance; stderr was:\n{plain}"
    );
    // Error message should mention the denied command.
    assert!(
        plain.contains("curl"),
        "preflight error should identify the denied command; stderr was:\n{plain}"
    );
    // Provider should NOT have run.
    assert!(
        !plain.contains("ERROR: provider should not run"),
        "provider binary should not execute when preflight fails; stderr was:\n{plain}"
    );
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p claudine-cli --test wrap_commands compose_preflight_error_includes_source_provenance -- --nocapture`

Expected: PASS. The error message from `resolve_shell_approvals` includes the source file path via the `PreFlightFailed` error variant.

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/tests/wrap_commands.rs
git commit -m "test(claudine): CLI integration test for compose preflight provenance

Validates that compose reports the source file name in preflight errors
when a ::shell directive is not whitelisted. Exercises the full CLI →
composition → preflight → error path without PTY."
```

---

## Task 5: Final verification

- [ ] **Step 1: Run all claudine tests**

Run: `cargo test -p claudine`

Expected: All tests PASS.

- [ ] **Step 2: Run all claudine-cli tests**

Run: `cargo test -p claudine-cli`

Expected: All tests PASS.

- [ ] **Step 3: Run darkmatter discovery tests**

Run: `cargo test -p darkmatter shell_expansion::discovery`

Expected: All tests PASS.

- [ ] **Step 4: Run lint**

Run: `cargo clippy -p claudine -p claudine-cli -- -D warnings`

Expected: No warnings.
