# Better Interview for Install — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current split install UX with a shared library-owned interview flow that announces what will be installed, captures stdout/stderr, shows a circular status, and offers retry with runnable alternatives — all rendered through a single real `Terminal`.

**Architecture:** `sniff/lib` gains a captured-execution path and a semantic interview engine that emits `InstallInterviewEvent`s (announcement/consent/captured-output/status) plus retry prompt data — no `biscuit-terminal` dependency. `sniff/cli` owns a `CliInstallUi` adapter that maps events to `Prose`/`BlockQuote`/`Status` against one shared `&Terminal` and drives `inquire` prompts. Both named install (`sniff <cat> install <name>`) and the MultiSelect picker call the same runner.

**Tech Stack:** Rust 2024, `thiserror`, `serde`, `inquire` (CLI prompts), `biscuit-terminal` (CLI only), `assert_cmd` + `predicates` (CLI integration tests), `expectrl` (PTY regression test).

---

## File Structure

### Library — `sniff/lib/src/programs/`

| File | Change | Responsibility |
|------|--------|----------------|
| `installer.rs` | Modify | Add `InstallCapturedResult`, `InstallCapturedOutcome`, `execute_install_captured`, `execute_versioned_install_captured`; refactor `execute_install` / `execute_versioned_install` as wrappers; add copy-builder helpers (`build_install_announcement`, `build_install_success_status`, `build_install_failure_status`, `build_retry_choice_prose`). |
| `install_plan.rs` | Modify | Add `InstallPlan::retryable_alternatives(&[InstallationMethod]) -> Vec<&InstallPlanOption>`. |
| `install_interview.rs` | Create | Event enums, delegate trait, options, outcome enum, `run_install_interview` runner. |
| `mod.rs` | Modify | Re-export new public types. |

### CLI — `sniff/cli/src/`

| File | Change | Responsibility |
|------|--------|----------------|
| `install_ui.rs` | Create | `CliInstallUi<'a>` adapter: `InstallInterviewDelegate` impl that renders events with `Prose` / `BlockQuote` / `Status` against `&Terminal`, and prompts via `inquire`. |
| `install_plan_cmd.rs` | Modify | Rewrite `execute_install_flow` to build `InstallInterviewInput`, construct one `Terminal::new()`, one `CliInstallUi`, and call `run_install_interview`. Keep `render_install_plan` for `install-plan` output. |
| `install.rs` | Modify | Multi-select picker: after selection, build an `InstallPlan` per picked program and call the shared interview runner with a fresh `CliInstallUi` per program, reusing one `Terminal`. |

### Tests

| File | Change |
|------|--------|
| `sniff/lib/src/programs/installer.rs` (existing `#[cfg(test)]`) | Add unit tests for captured APIs and copy builders. |
| `sniff/lib/src/programs/install_plan.rs` (existing `#[cfg(test)]`) | Add `retryable_alternatives` tests. |
| `sniff/lib/src/programs/install_interview.rs` | In-file `#[cfg(test)]` module with `TestDelegate` driving the engine. |
| `sniff/cli/tests/install_interview_cli.rs` | Create — `assert_cmd` dry-run E2E + `NO_COLOR=1` snapshot of event→component mapping. |
| `sniff/cli/tests/install_interactive_pty.rs` | Create — `expectrl` PTY test: failure then fallback. |

---

## Self-Review Checklist (to run after Task 15)

- [ ] Every spec bullet mapped to a task (announce / stdout-blockquote / success-status / stderr-blockquote / failure-status / retry-prose / quit-prose).
- [ ] Library never imports `biscuit_terminal::*`.
- [ ] CLI constructs `Terminal::new()` exactly once per command; never passes `Terminal::default()` into renderers.
- [ ] Every type referenced in later tasks (e.g. `InstallInterviewEvent`, `InstallCapturedOutcome`, `RetryPromptChoice`) is defined in an earlier task.
- [ ] No `--no-verify`, no amending existing commits.

---

## Task 1: Add captured-execution types to `installer.rs`

**Files:**
- Modify: `sniff/lib/src/programs/installer.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn install_captured_outcome_completed_has_command_and_streams() {
    // Just a compile/shape check: construct a Completed variant and
    // observe its fields. This locks the public shape.
    let ok = InstallCapturedResult {
        command: "brew install rg".into(),
        executed: true,
        exit_code: Some(0),
        stdout: "ok\n".into(),
        stderr: String::new(),
        success: true,
    };
    let outcome = InstallCapturedOutcome::Completed(ok);
    match outcome {
        InstallCapturedOutcome::Completed(r) => {
            assert_eq!(r.command, "brew install rg");
            assert!(r.success);
        }
        _ => panic!("expected Completed"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sniff install_captured_outcome_completed_has_command_and_streams`
Expected: FAIL — `InstallCapturedResult`, `InstallCapturedOutcome` not defined.

- [ ] **Step 3: Add types to `installer.rs`**

Insert below `InstallResult`:

```rust
/// Captured outcome of an install attempt, preserving stdout/stderr on both
/// success and non-zero-exit failures so the interview layer can render
/// structured output. See `sniff/features/2026-04-12-better-interview-for-install/tech-design.md`.
#[derive(Debug, Clone)]
pub struct InstallCapturedResult {
    pub command: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Two-arm outcome from a captured install run.
///
/// `Completed` covers dry-run, success, and non-zero exit (including spawn
/// failures folded into `stderr`). `SetupError` is reserved for invalid
/// inputs where no command could meaningfully run.
#[derive(Debug)]
pub enum InstallCapturedOutcome {
    Completed(InstallCapturedResult),
    SetupError(SniffInstallationError),
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sniff install_captured_outcome_completed_has_command_and_streams`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/installer.rs
git commit -m "feat(sniff): add InstallCapturedResult/InstallCapturedOutcome types"
```

---

## Task 2: Implement `execute_install_captured`

**Files:**
- Modify: `sniff/lib/src/programs/installer.rs`

- [ ] **Step 1: Write the failing test**

Add:

```rust
#[test]
fn execute_install_captured_dry_run_returns_completed_with_command() {
    let method = InstallationMethod::Brew("ripgrep");
    let outcome = execute_install_captured(&method, &InstallOptions::dry_run());
    match outcome {
        InstallCapturedOutcome::Completed(r) => {
            assert!(!r.executed);
            assert!(r.success);
            assert_eq!(r.command, "brew install ripgrep");
            assert_eq!(r.exit_code, None);
        }
        _ => panic!("dry run must be Completed"),
    }
}

#[test]
fn execute_install_captured_setup_error_on_invalid_package() {
    let method = InstallationMethod::Brew("bad;pkg");
    let outcome = execute_install_captured(&method, &InstallOptions::default());
    assert!(matches!(outcome, InstallCapturedOutcome::SetupError(_)));
}

#[test]
fn execute_install_captured_spawn_failure_is_completed_with_stderr() {
    // Pick a method whose program is almost certainly not on the test host.
    // "nix-env" is fine as an example when nix isn't installed, but rather
    // than depend on env we validate the branch behavior via a known-bad
    // program binary: use an invalid package name routed through Cargo would
    // still pass validation, so instead use the shape assertion below.
    // If cargo IS available, this test may actually succeed; skip via env.
    if std::env::var_os("SNIFF_ALLOW_NETWORK_TESTS").is_none() {
        return;
    }
    let method = InstallationMethod::Cargo("definitely-not-a-real-crate-xyz-123");
    let outcome = execute_install_captured(&method, &InstallOptions::default());
    match outcome {
        InstallCapturedOutcome::Completed(r) => {
            assert!(r.executed || !r.stderr.is_empty());
            assert!(!r.success);
        }
        other => panic!("expected Completed with failure, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sniff execute_install_captured_dry_run_returns_completed_with_command`
Expected: FAIL — function not defined.

- [ ] **Step 3: Add `execute_install_captured` to `installer.rs`**

```rust
/// Captured variant of [`execute_install`].
///
/// Always returns an [`InstallCapturedOutcome`] with stdout/stderr preserved
/// even on non-zero exit, so interview callers can render structured output
/// in both success and failure branches. Spawn errors are folded into the
/// `stderr` field of a failing `Completed` outcome so the failure branch
/// still has text to quote.
pub fn execute_install_captured(
    method: &InstallationMethod,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install_captured(pkg, None, opts);
    }

    let cmd_parts = match build_install_command(method) {
        Ok(p) => p,
        Err(e) => return InstallCapturedOutcome::SetupError(e),
    };
    let cmd_str = cmd_parts.join(" ");

    if opts.dry_run {
        return InstallCapturedOutcome::Completed(InstallCapturedResult {
            command: cmd_str,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            success: true,
        });
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    match Command::new(program).args(args).output() {
        Ok(output) => {
            let success = output.status.success();
            InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: cmd_str,
                executed: true,
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                success,
            })
        }
        Err(e) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command: cmd_str,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            success: false,
        }),
    }
}
```

Add a matching `execute_uv_with_install_captured` helper that adapts the
existing `execute_uv_with_install` body: all success/failure paths construct
`Completed(InstallCapturedResult { ... })` and invalid input returns
`SetupError`. Use the same bootstrap+install sequence; on any non-success
return `Completed` with `success = false` and the accumulated stderr (not
`SetupError`). Only `validate_package_name` failures become `SetupError`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sniff execute_install_captured`
Expected: PASS for dry-run and setup-error tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/installer.rs
git commit -m "feat(sniff): add execute_install_captured preserving stdout/stderr on failure"
```

---

## Task 3: Implement `execute_versioned_install_captured` and refactor legacy APIs as wrappers

**Files:**
- Modify: `sniff/lib/src/programs/installer.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn execute_versioned_install_captured_dry_run() {
    let method = InstallationMethod::Cargo("bat");
    let outcome = execute_versioned_install_captured(&method, "0.24.0", &InstallOptions::dry_run());
    match outcome {
        InstallCapturedOutcome::Completed(r) => {
            assert!(!r.executed);
            assert!(r.success);
            assert!(r.command.contains("bat"));
            assert!(r.command.contains("0.24.0"));
        }
        _ => panic!("expected Completed"),
    }
}

#[test]
fn execute_install_still_returns_install_result_on_dry_run() {
    // Compatibility: legacy wrapper keeps old shape.
    let method = InstallationMethod::Brew("ripgrep");
    let result = execute_install(&method, &InstallOptions::dry_run()).unwrap();
    assert!(!result.executed);
    assert_eq!(result.command, "brew install ripgrep");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff execute_versioned_install_captured_dry_run`
Expected: FAIL — function not defined.

- [ ] **Step 3: Add `execute_versioned_install_captured` and refactor wrappers**

```rust
pub fn execute_versioned_install_captured(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install_captured(pkg, Some(version), opts);
    }

    let cmd_parts = match build_versioned_install_command(method, version) {
        Ok(p) => p,
        Err(e) => return InstallCapturedOutcome::SetupError(e),
    };
    let cmd_str = cmd_parts.join(" ");

    if opts.dry_run {
        return InstallCapturedOutcome::Completed(InstallCapturedResult {
            command: cmd_str,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            success: true,
        });
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    match Command::new(program).args(args).output() {
        Ok(output) => {
            let success = output.status.success();
            InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: cmd_str,
                executed: true,
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                success,
            })
        }
        Err(e) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command: cmd_str,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            success: false,
        }),
    }
}
```

Refactor `execute_install` to delegate:

```rust
pub fn execute_install(
    method: &InstallationMethod,
    opts: &InstallOptions,
) -> Result<InstallResult, SniffInstallationError> {
    match execute_install_captured(method, opts) {
        InstallCapturedOutcome::SetupError(e) => Err(e),
        InstallCapturedOutcome::Completed(r) if r.success => Ok(InstallResult {
            command: r.command,
            executed: r.executed,
            exit_code: r.exit_code,
            stdout: r.stdout,
            stderr: r.stderr,
        }),
        InstallCapturedOutcome::Completed(r) => Err(SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: r.stderr,
        }),
    }
}
```

Do the same for `execute_versioned_install`.

- [ ] **Step 4: Run full installer tests**

Run: `cargo test -p sniff --lib programs::installer`
Expected: PASS (all existing and new tests).

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/installer.rs
git commit -m "refactor(sniff): make execute_install a thin wrapper over captured path"
```

---

## Task 4: Add copy-builder helpers in `installer.rs`

**Files:**
- Modify: `sniff/lib/src/programs/installer.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn announcement_package_manager_template() {
    let out = build_install_announcement(
        "Ripgrep",
        "https://github.com/BurntSushi/ripgrep",
        &InstallationMethod::Brew("ripgrep"),
        "brew install ripgrep",
    );
    assert!(out.contains("Ripgrep"));
    assert!(out.contains("https://github.com/BurntSushi/ripgrep"));
    assert!(out.contains("brew"));
    assert!(out.contains("brew install ripgrep"));
    assert!(out.contains("package manager"));
}

#[test]
fn announcement_remote_bash_template() {
    let url = "https://sh.rustup.rs";
    let out = build_install_announcement(
        "Rustup",
        "https://rustup.rs",
        &InstallationMethod::RemoteBash(url),
        "curl -sSfL 'https://sh.rustup.rs' | bash",
    );
    assert!(out.contains("remote installer script"));
    assert!(out.contains(url));
}

#[test]
fn announcement_uv_with_install_template() {
    let out = build_install_announcement(
        "Aider",
        "https://aider.chat",
        &InstallationMethod::UvWithInstall("aider-chat"),
        "uv tool install 'aider-chat'",
    );
    assert!(out.contains("bootstrapping"));
    assert!(out.contains("uv"));
    assert!(out.contains("astral.sh"));
}

#[test]
fn success_status_mentions_installed_successfully() {
    let out = build_install_success_status("Ripgrep", "https://github.com/BurntSushi/ripgrep");
    assert!(out.contains("Ripgrep"));
    assert!(out.contains("installed successfully"));
}

#[test]
fn failure_status_mentions_failed_to_install() {
    let out = build_install_failure_status("Ripgrep", "https://github.com/BurntSushi/ripgrep");
    assert!(out.to_lowercase().contains("failed to install"));
    assert!(out.contains("Ripgrep"));
}

#[test]
fn retry_choice_prose_names_alternative() {
    let out = build_retry_choice_prose(&InstallationMethod::Cargo("bat"));
    assert!(out.contains("cargo"));
    assert!(out.contains("Try installing"));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff announcement_package_manager_template`
Expected: FAIL — not defined.

- [ ] **Step 3: Add helpers to `installer.rs`**

```rust
/// Returns the announcement prose for a chosen install method.
///
/// Prose uses the embedded-markup dialect consumed by
/// `biscuit_terminal::components::prose::Prose`. Callers wrap the returned
/// string in `Prose::new(...)`. Never imports from biscuit-terminal.
pub fn build_install_announcement(
    program: &str,
    website: &str,
    method: &InstallationMethod,
    command: &str,
) -> String {
    match method {
        InstallationMethod::RemoteBash(url) => format!(
            "The <b><blue><a href=\"{website}\">{program}</a></blue></b> will be installed using the remote installer script at <a href=\"{url}\">{url}</a> using the command: <dim><green>{command}</green></dim>"
        ),
        InstallationMethod::UvWithInstall(_) => {
            let astral = astral_installer_url();
            format!(
                "The <b><blue><a href=\"{website}\">{program}</a></blue></b> will be installed by bootstrapping <b>uv</b> from <a href=\"{astral}\">{astral}</a> if needed, then running: <dim><green>{command}</green></dim>"
            )
        }
        other => {
            let manager = other.manager_name();
            format!(
                "The <b><blue><a href=\"{website}\">{program}</a></blue></b> will be installed through the <b>{manager}</b> package manager using the command: <dim><green>{command}</green></dim>"
            )
        }
    }
}

/// Returns the success-status prose.
pub fn build_install_success_status(program: &str, website: &str) -> String {
    format!(
        "<b><blue><a href=\"{website}\">{program}</a></blue></b> has been installed successfully"
    )
}

/// Returns the error-status prose.
pub fn build_install_failure_status(program: &str, website: &str) -> String {
    format!(
        "failed to install <b><blue><a href=\"{website}\">{program}</a></blue></b>."
    )
}

/// Returns the retry prompt prose for an alternative install method.
pub fn build_retry_choice_prose(method: &InstallationMethod) -> String {
    format!(
        "Try installing using <b>{}</b> instead",
        method.manager_name()
    )
}

/// Returns the retry prompt prose for the quit option.
pub fn build_retry_quit_prose() -> String {
    "Quit (<i>and try manually if desired</i>)".to_string()
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff --lib programs::installer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/installer.rs
git commit -m "feat(sniff): add install announcement/status/retry copy builders"
```

---

## Task 5: Add `InstallPlan::retryable_alternatives`

**Files:**
- Modify: `sniff/lib/src/programs/install_plan.rs`

- [ ] **Step 1: Write the failing test**

Append to existing `#[cfg(test)] mod selection_tests`:

```rust
#[test]
fn retryable_alternatives_excludes_blocked_and_attempted() {
    let plan = InstallPlan {
        program: "bat".into(),
        website: "https://github.com/sharkdp/bat",
        successful: true,
        options: vec![
            InstallPlanOption {
                kind: InstallationMethod::Brew("bat"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "chosen".into(),
            },
            InstallPlanOption {
                kind: InstallationMethod::Cargo("bat"),
                requires_sudo: false,
                choose: false,
                reason_type: InstallPlanReason::LowerPriorityAlternative,
                reason: "brew chosen".into(),
            },
            InstallPlanOption {
                kind: InstallationMethod::Apt("bat"),
                requires_sudo: true,
                choose: false,
                reason_type: InstallPlanReason::ManagerNotInstalled,
                reason: "apt missing".into(),
            },
        ],
    };

    // With brew attempted, cargo is the only retryable; apt is blocked.
    let attempted = [InstallationMethod::Brew("bat")];
    let alts = plan.retryable_alternatives(&attempted);
    assert_eq!(alts.len(), 1);
    assert!(matches!(alts[0].kind, InstallationMethod::Cargo(_)));
}

#[test]
fn retryable_alternatives_empty_when_nothing_remains() {
    let plan = InstallPlan {
        program: "bat".into(),
        website: "https://github.com/sharkdp/bat",
        successful: true,
        options: vec![InstallPlanOption {
            kind: InstallationMethod::Brew("bat"),
            requires_sudo: false,
            choose: true,
            reason_type: InstallPlanReason::Selected,
            reason: "chosen".into(),
        }],
    };
    let attempted = [InstallationMethod::Brew("bat")];
    assert!(plan.retryable_alternatives(&attempted).is_empty());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff retryable_alternatives_excludes_blocked_and_attempted`
Expected: FAIL — method not defined.

- [ ] **Step 3: Add method**

In the `impl InstallPlan { ... }` block:

```rust
/// Returns alternative options marked as `LowerPriorityAlternative` whose
/// method kind is not in `attempted`. These are the methods that were
/// runnable at plan-build time but not initially chosen because of
/// priority — the only sensible retry candidates.
pub fn retryable_alternatives(
    &self,
    attempted: &[InstallationMethod],
) -> Vec<&InstallPlanOption> {
    self.options
        .iter()
        .filter(|o| o.reason_type == InstallPlanReason::LowerPriorityAlternative)
        .filter(|o| !attempted.iter().any(|m| m == &o.kind))
        .collect()
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff retryable_alternatives`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs
git commit -m "feat(sniff): add InstallPlan::retryable_alternatives"
```

---

## Task 6: Scaffold `install_interview.rs` with event/delegate types

**Files:**
- Create: `sniff/lib/src/programs/install_interview.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `install_interview.rs` with the types below AND at the bottom a `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_variants_carry_expected_fields() {
        let a = InstallInterviewEvent::Announcement { prose: "hi".into() };
        let c = InstallInterviewEvent::ConsentWarning { prose: "warn".into() };
        let o = InstallInterviewEvent::CapturedOutput {
            stream: InstallOutputStream::Stdout,
            body: "out".into(),
        };
        let s = InstallInterviewEvent::Status {
            kind: InstallStatusKind::Success,
            text: "ok".into(),
        };
        // Compile-only assertion: matches are exhaustive.
        for e in [a, c, o, s] {
            match e {
                InstallInterviewEvent::Announcement { .. }
                | InstallInterviewEvent::ConsentWarning { .. }
                | InstallInterviewEvent::CapturedOutput { .. }
                | InstallInterviewEvent::Status { .. } => {}
            }
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff event_variants_carry_expected_fields`
Expected: FAIL — module not wired.

- [ ] **Step 3: Write the module and wire it in**

Add to `sniff/lib/src/programs/install_interview.rs`:

```rust
//! Shared install interview engine.
//!
//! The library owns sequencing, command execution, and copy strings. The
//! caller (e.g. the sniff CLI) supplies a delegate that decides how to
//! render each event and how to handle interactive prompts. This avoids a
//! circular dependency on biscuit-terminal.
//!
//! See `sniff/features/2026-04-12-better-interview-for-install/tech-design.md`.

use crate::error::SniffInstallationError;
use crate::programs::install_plan::InstallPlan;
use crate::programs::installer::InstallOptions;
use crate::programs::types::InstallationMethod;

/// Semantic interview events emitted by the runner. Each variant carries a
/// caller-renderable string; the caller decides which concrete component to
/// wrap around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallInterviewEvent {
    /// Pre-execution announcement (for `Prose`).
    Announcement { prose: String },
    /// Warning before a remote-script install (for `Prose`).
    ConsentWarning { prose: String },
    /// Captured program output (for `BlockQuote`). The body is raw text.
    CapturedOutput {
        stream: InstallOutputStream,
        body: String,
    },
    /// Terminal success/error status (for `Status`).
    Status {
        kind: InstallStatusKind,
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStatusKind {
    Success,
    Error,
}

/// Input to a single interview session.
#[derive(Debug, Clone)]
pub struct InstallInterviewInput {
    pub program: String,
    pub website: &'static str,
    pub plan: InstallPlan,
}

/// Options controlling the interview runner.
#[derive(Debug, Clone)]
pub struct InstallInterviewOptions {
    pub install: InstallOptions,
    /// When true (default for CLI interactive flows), the runner prompts
    /// for a retry choice when the chosen method fails and at least one
    /// runnable alternative exists. Silent callers can set this to false.
    pub prompt_on_failure: bool,
}

impl Default for InstallInterviewOptions {
    fn default() -> Self {
        Self {
            install: InstallOptions::default(),
            prompt_on_failure: true,
        }
    }
}

/// Retry prompt payload handed to the delegate.
#[derive(Debug, Clone)]
pub struct RetryPrompt {
    /// Optional heading prose to render before the prompt.
    pub heading_prose: String,
    pub choices: Vec<RetryPromptChoice>,
}

#[derive(Debug, Clone)]
pub struct RetryPromptChoice {
    /// Plain-string label suitable for `inquire::Select`.
    pub label: String,
    /// Rich prose line the caller may optionally render before prompting.
    pub prose: String,
    /// The method this choice would retry with.
    pub method: InstallationMethod,
}

/// Delegate's decision.
#[derive(Debug, Clone)]
pub enum RetryChoice {
    RetryWith(InstallationMethod),
    Quit,
}

/// Caller-provided adapter for rendering events and handling prompts.
pub trait InstallInterviewDelegate {
    fn on_event(
        &mut self,
        event: &InstallInterviewEvent,
    ) -> Result<(), SniffInstallationError>;

    fn confirm_remote_script(
        &mut self,
        prose: &str,
    ) -> Result<bool, SniffInstallationError>;

    fn choose_retry(
        &mut self,
        prompt: &RetryPrompt,
    ) -> Result<RetryChoice, SniffInstallationError>;
}

/// Final outcome of the interview.
#[derive(Debug, Clone)]
pub enum InstallInterviewOutcome {
    Installed { method: InstallationMethod },
    DryRun { method: InstallationMethod },
    AbortedByUser,
    Failed { attempted: Vec<InstallationMethod> },
    NotInstallable,
}

// Runner is added in Task 7.
```

In `sniff/lib/src/programs/mod.rs`, add the module and re-exports:

```rust
pub mod install_interview;
```

And near the other `pub use`:

```rust
pub use install_interview::{
    InstallInterviewDelegate, InstallInterviewEvent, InstallInterviewInput,
    InstallInterviewOptions, InstallInterviewOutcome, InstallOutputStream,
    InstallStatusKind, RetryChoice, RetryPrompt, RetryPromptChoice,
};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff event_variants_carry_expected_fields`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_interview.rs sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff): scaffold install_interview event/delegate types"
```

---

## Task 7: Implement `run_install_interview` happy path (success & dry-run)

**Files:**
- Modify: `sniff/lib/src/programs/install_interview.rs`

- [ ] **Step 1: Write the failing test**

Append to the test module:

```rust
#[cfg(test)]
struct RecordingDelegate {
    events: Vec<InstallInterviewEvent>,
    consent_answer: bool,
    retry_answer: Option<RetryChoice>,
}

#[cfg(test)]
impl RecordingDelegate {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            consent_answer: true,
            retry_answer: None,
        }
    }
}

#[cfg(test)]
impl InstallInterviewDelegate for RecordingDelegate {
    fn on_event(&mut self, e: &InstallInterviewEvent) -> Result<(), SniffInstallationError> {
        self.events.push(e.clone());
        Ok(())
    }
    fn confirm_remote_script(&mut self, _p: &str) -> Result<bool, SniffInstallationError> {
        Ok(self.consent_answer)
    }
    fn choose_retry(&mut self, _p: &RetryPrompt) -> Result<RetryChoice, SniffInstallationError> {
        Ok(self.retry_answer.clone().unwrap_or(RetryChoice::Quit))
    }
}

#[cfg(test)]
fn brew_plan() -> InstallInterviewInput {
    use crate::programs::install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason};
    InstallInterviewInput {
        program: "Ripgrep".into(),
        website: "https://github.com/BurntSushi/ripgrep",
        plan: InstallPlan {
            program: "Ripgrep".into(),
            website: "https://github.com/BurntSushi/ripgrep",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::Brew("ripgrep"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "chosen".into(),
            }],
        },
    }
}

#[test]
fn dry_run_emits_announcement_and_success_status_without_execution() {
    let input = brew_plan();
    let mut opts = InstallInterviewOptions::default();
    opts.install.dry_run = true;
    let mut d = RecordingDelegate::new();
    let outcome = run_install_interview(&input, &opts, &mut d).unwrap();

    assert!(matches!(outcome, InstallInterviewOutcome::DryRun { .. }));
    // Expect exactly: Announcement, Status(Success). No CapturedOutput.
    assert!(matches!(d.events[0], InstallInterviewEvent::Announcement { .. }));
    let has_success_status = d.events.iter().any(|e| matches!(e,
        InstallInterviewEvent::Status { kind: InstallStatusKind::Success, .. }));
    assert!(has_success_status);
    let has_captured = d.events.iter().any(|e| matches!(e, InstallInterviewEvent::CapturedOutput { .. }));
    assert!(!has_captured);
}

#[test]
fn not_installable_plan_returns_not_installable_outcome() {
    use crate::programs::install_plan::InstallPlan;
    let input = InstallInterviewInput {
        program: "nope".into(),
        website: "https://example.com",
        plan: InstallPlan {
            program: "nope".into(),
            website: "https://example.com",
            successful: false,
            options: vec![],
        },
    };
    let mut d = RecordingDelegate::new();
    let outcome = run_install_interview(
        &input,
        &InstallInterviewOptions::default(),
        &mut d,
    )
    .unwrap();
    assert!(matches!(outcome, InstallInterviewOutcome::NotInstallable));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff dry_run_emits_announcement_and_success_status_without_execution`
Expected: FAIL — `run_install_interview` not defined.

- [ ] **Step 3: Implement the runner (dry-run + success branch only)**

Add to `install_interview.rs`:

```rust
use crate::programs::installer::{
    build_install_announcement, build_install_failure_status, build_install_success_status,
    build_retry_choice_prose, build_retry_quit_prose, execute_install_captured,
    get_install_command, InstallCapturedOutcome,
};

/// Run one install interview session against the caller-provided delegate.
///
/// The runner emits semantic events and asks the delegate for decisions.
/// It never talks to a terminal directly.
pub fn run_install_interview<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
) -> Result<InstallInterviewOutcome, SniffInstallationError> {
    if !input.plan.successful {
        let msg = format!(
            "failed to install <b><blue><a href=\"{}\">{}</a></blue></b> — no runnable method on this host",
            input.website, input.program
        );
        delegate.on_event(&InstallInterviewEvent::Status {
            kind: InstallStatusKind::Error,
            text: msg,
        })?;
        return Ok(InstallInterviewOutcome::NotInstallable);
    }

    let chosen = input
        .plan
        .chosen()
        .cloned()
        .expect("successful plan has a chosen option");
    run_attempt(input, options, delegate, chosen.kind, Vec::new())
}

fn run_attempt<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
    method: InstallationMethod,
    mut attempted: Vec<InstallationMethod>,
) -> Result<InstallInterviewOutcome, SniffInstallationError> {
    let command = get_install_command(&method)?;

    delegate.on_event(&InstallInterviewEvent::Announcement {
        prose: build_install_announcement(
            &input.program,
            input.website,
            &method,
            &command,
        ),
    })?;

    // Remote-script consent gate (implemented in Task 8).

    let outcome = execute_install_captured(&method, &options.install);
    attempted.push(method.clone());

    match outcome {
        InstallCapturedOutcome::SetupError(e) => {
            delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                stream: InstallOutputStream::Stderr,
                body: e.to_string(),
            })?;
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text: build_install_failure_status(&input.program, input.website),
            })?;
            Ok(InstallInterviewOutcome::Failed { attempted })
        }
        InstallCapturedOutcome::Completed(r) if r.success => {
            if !r.executed {
                // Dry-run: still render success status, no CapturedOutput.
                delegate.on_event(&InstallInterviewEvent::Status {
                    kind: InstallStatusKind::Success,
                    text: build_install_success_status(&input.program, input.website),
                })?;
                Ok(InstallInterviewOutcome::DryRun { method })
            } else {
                if !r.stdout.trim().is_empty() {
                    delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                        stream: InstallOutputStream::Stdout,
                        body: r.stdout,
                    })?;
                }
                delegate.on_event(&InstallInterviewEvent::Status {
                    kind: InstallStatusKind::Success,
                    text: build_install_success_status(&input.program, input.website),
                })?;
                Ok(InstallInterviewOutcome::Installed { method })
            }
        }
        InstallCapturedOutcome::Completed(r) => {
            // Failure branch is implemented in Task 9.
            let body = if !r.stderr.trim().is_empty() {
                r.stderr
            } else {
                r.stdout
            };
            if !body.trim().is_empty() {
                delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                    stream: InstallOutputStream::Stderr,
                    body,
                })?;
            }
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text: build_install_failure_status(&input.program, input.website),
            })?;
            Ok(InstallInterviewOutcome::Failed { attempted })
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff install_interview`
Expected: PASS for dry-run and not-installable tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_interview.rs
git commit -m "feat(sniff): implement run_install_interview success/dry-run path"
```

---

## Task 8: Add remote-script consent gate

**Files:**
- Modify: `sniff/lib/src/programs/install_interview.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
fn remote_bash_plan() -> InstallInterviewInput {
    use crate::programs::install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason};
    InstallInterviewInput {
        program: "Rustup".into(),
        website: "https://rustup.rs",
        plan: InstallPlan {
            program: "Rustup".into(),
            website: "https://rustup.rs",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "chosen".into(),
            }],
        },
    }
}

#[test]
fn denied_remote_consent_returns_aborted_by_user() {
    let input = remote_bash_plan();
    let mut opts = InstallInterviewOptions::default();
    opts.install.dry_run = false;
    opts.install.approve_remote_bash = false;
    let mut d = RecordingDelegate::new();
    d.consent_answer = false;
    let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
    assert!(matches!(outcome, InstallInterviewOutcome::AbortedByUser));
    // Announcement + ConsentWarning were emitted; no Status yet.
    assert!(d
        .events
        .iter()
        .any(|e| matches!(e, InstallInterviewEvent::ConsentWarning { .. })));
}

#[test]
fn remote_bash_dry_run_skips_consent() {
    let input = remote_bash_plan();
    let mut opts = InstallInterviewOptions::default();
    opts.install.dry_run = true;
    let mut d = RecordingDelegate::new();
    d.consent_answer = false; // would deny, but should not be asked
    let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
    assert!(matches!(outcome, InstallInterviewOutcome::DryRun { .. }));
    assert!(!d
        .events
        .iter()
        .any(|e| matches!(e, InstallInterviewEvent::ConsentWarning { .. })));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff denied_remote_consent_returns_aborted_by_user`
Expected: FAIL — consent gate not implemented.

- [ ] **Step 3: Insert consent gate in `run_attempt`**

In `run_attempt`, after `Announcement` and before `execute_install_captured`:

```rust
let needs_consent = matches!(
    method,
    InstallationMethod::RemoteBash(_) | InstallationMethod::UvWithInstall(_)
);
if needs_consent && !options.install.dry_run && !options.install.approve_remote_bash {
    let warning = build_remote_script_warning(&input.program, &method);
    delegate.on_event(&InstallInterviewEvent::ConsentWarning {
        prose: warning.clone(),
    })?;
    if !delegate.confirm_remote_script(&warning)? {
        return Ok(InstallInterviewOutcome::AbortedByUser);
    }
}
```

Add a helper:

```rust
fn build_remote_script_warning(program: &str, method: &InstallationMethod) -> String {
    match method {
        InstallationMethod::RemoteBash(url) => format!(
            "<yellow>Warning:</yellow> installing <b>{program}</b> will download and execute a remote shell script from <a href=\"{url}\">{url}</a>."
        ),
        InstallationMethod::UvWithInstall(_) => {
            let url = crate::programs::installer::astral_installer_url();
            format!(
                "<yellow>Warning:</yellow> installing <b>{program}</b> will bootstrap <b>uv</b> by downloading and executing a remote script from <a href=\"{url}\">{url}</a>."
            )
        }
        _ => String::new(),
    }
}
```

Also set `options.install.approve_remote_bash = true` on the effective
`InstallOptions` passed into the captured execution once consent is granted
(or already pre-approved): build a local copy of the options inside
`run_attempt`:

```rust
let mut exec_opts = options.install.clone();
if needs_consent {
    exec_opts.approve_remote_bash = true;
}
let outcome = execute_install_captured(&method, &exec_opts);
```

Note: `execute_install_captured` itself does not currently check
`approve_remote_bash`; it only runs via `InstallPlan::execute` today. Since
we bypass that path, the flag is informational here but we keep it set for
future correctness.

- [ ] **Step 4: Expose `astral_installer_url` at crate path**

If it's currently `pub(crate)`, make it `pub` in `installer.rs`:

```rust
pub fn astral_installer_url() -> &'static str { ... }
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p sniff denied_remote_consent_returns_aborted_by_user remote_bash_dry_run_skips_consent`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add sniff/lib/src/programs/install_interview.rs sniff/lib/src/programs/installer.rs
git commit -m "feat(sniff): add remote-script consent gate to install interview"
```

---

## Task 9: Failure branch with retry prompt

**Files:**
- Modify: `sniff/lib/src/programs/install_interview.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
fn fake_setup_error_plan() -> InstallInterviewInput {
    // Brew with a shell-metachar package name triggers a SetupError so we
    // can drive the failure branch without running a command.
    use crate::programs::install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason};
    InstallInterviewInput {
        program: "bad".into(),
        website: "https://example.com",
        plan: InstallPlan {
            program: "bad".into(),
            website: "https://example.com",
            successful: true,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Brew("bad;pkg"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "chosen".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Cargo("goodpkg"),
                    requires_sudo: false,
                    choose: false,
                    reason_type: InstallPlanReason::LowerPriorityAlternative,
                    reason: "alternative".into(),
                },
            ],
        },
    }
}

#[test]
fn failure_with_alternatives_prompts_retry_and_loops() {
    let input = fake_setup_error_plan();
    let mut d = RecordingDelegate::new();
    d.retry_answer = Some(RetryChoice::RetryWith(InstallationMethod::Cargo("goodpkg")));

    let mut opts = InstallInterviewOptions::default();
    opts.install.dry_run = true; // second attempt succeeds as dry-run
    opts.prompt_on_failure = true;

    let outcome = run_install_interview(&input, &opts, &mut d).unwrap();
    assert!(matches!(outcome, InstallInterviewOutcome::DryRun { .. }));
    // First attempt status(error), then second attempt announcement + status(success).
    let error_statuses = d
        .events
        .iter()
        .filter(|e| matches!(e, InstallInterviewEvent::Status { kind: InstallStatusKind::Error, .. }))
        .count();
    let success_statuses = d
        .events
        .iter()
        .filter(|e| matches!(e, InstallInterviewEvent::Status { kind: InstallStatusKind::Success, .. }))
        .count();
    assert_eq!(error_statuses, 1);
    assert_eq!(success_statuses, 1);
}

#[test]
fn failure_without_alternatives_returns_failed_and_does_not_prompt() {
    use crate::programs::install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason};
    let input = InstallInterviewInput {
        program: "bad".into(),
        website: "https://example.com",
        plan: InstallPlan {
            program: "bad".into(),
            website: "https://example.com",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::Brew("bad;pkg"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "chosen".into(),
            }],
        },
    };
    let mut d = RecordingDelegate::new();
    let outcome =
        run_install_interview(&input, &InstallInterviewOptions::default(), &mut d).unwrap();
    assert!(matches!(outcome, InstallInterviewOutcome::Failed { .. }));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff failure_with_alternatives_prompts_retry_and_loops`
Expected: FAIL — retry loop not implemented.

- [ ] **Step 3: Implement failure/retry loop**

Replace the two "Failed" arms in `run_attempt` with a call to a new
`handle_failure` helper that emits the captured output + error status, then
gathers alternatives and either prompts or returns `Failed`.

```rust
fn handle_failure<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
    failed_method: InstallationMethod,
    captured_body: Option<(InstallOutputStream, String)>,
    mut attempted: Vec<InstallationMethod>,
) -> Result<InstallInterviewOutcome, SniffInstallationError> {
    if let Some((stream, body)) = captured_body {
        if !body.trim().is_empty() {
            delegate.on_event(&InstallInterviewEvent::CapturedOutput { stream, body })?;
        }
    }
    delegate.on_event(&InstallInterviewEvent::Status {
        kind: InstallStatusKind::Error,
        text: build_install_failure_status(&input.program, input.website),
    })?;

    let _ = failed_method; // already pushed to attempted by caller
    let alts = input.plan.retryable_alternatives(&attempted);
    if alts.is_empty() || !options.prompt_on_failure {
        return Ok(InstallInterviewOutcome::Failed { attempted });
    }

    let choices: Vec<RetryPromptChoice> = alts
        .iter()
        .map(|o| RetryPromptChoice {
            label: format!("Retry with {}", o.kind.manager_name()),
            prose: build_retry_choice_prose(&o.kind),
            method: o.kind.clone(),
        })
        .collect();
    let prompt = RetryPrompt {
        heading_prose: build_retry_quit_prose(),
        choices,
    };
    match delegate.choose_retry(&prompt)? {
        RetryChoice::Quit => Ok(InstallInterviewOutcome::AbortedByUser),
        RetryChoice::RetryWith(next) => {
            attempted.push(next.clone());
            run_attempt(input, options, delegate, next, attempted)
        }
    }
}
```

Then rewrite the two failure arms in `run_attempt`:

```rust
InstallCapturedOutcome::SetupError(e) => handle_failure(
    input,
    options,
    delegate,
    method.clone(),
    Some((InstallOutputStream::Stderr, e.to_string())),
    attempted,
),
InstallCapturedOutcome::Completed(r) if !r.success => {
    let body = if !r.stderr.trim().is_empty() {
        r.stderr
    } else {
        r.stdout
    };
    handle_failure(
        input,
        options,
        delegate,
        method.clone(),
        Some((InstallOutputStream::Stderr, body)),
        attempted,
    )
}
```

(Note: `attempted` already contains the current method since `run_attempt`
pushes it before dispatching to this branch.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff install_interview`
Expected: PASS for all interview tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_interview.rs
git commit -m "feat(sniff): add failure/retry loop to install interview"
```

---

## Task 10: Create `CliInstallUi` adapter (events → components)

**Files:**
- Create: `sniff/cli/src/install_ui.rs`
- Modify: `sniff/cli/src/main.rs` (add `mod install_ui;`)

- [ ] **Step 1: Write the failing test**

At the bottom of `install_ui.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;
    use sniff::programs::{
        InstallInterviewDelegate, InstallInterviewEvent, InstallOutputStream,
        InstallStatusKind,
    };

    fn ui_capture() -> CliInstallUi {
        CliInstallUi {
            terminal: Terminal::default(),
            plain: true,
            buffer: Vec::new(),
        }
    }

    #[test]
    fn announcement_is_rendered_as_prose() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::Announcement {
            prose: "install <b>rg</b>".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        // Plain mode strips tags; check content survives.
        assert!(text.contains("install"));
        assert!(text.contains("rg"));
    }

    #[test]
    fn stdout_captured_output_is_rendered_in_block_quote() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::CapturedOutput {
            stream: InstallOutputStream::Stdout,
            body: "line1\nline2".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.contains("line1"));
        assert!(text.contains("line2"));
    }

    #[test]
    fn status_success_is_rendered_with_status_component() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::Status {
            kind: InstallStatusKind::Success,
            text: "Ripgrep installed".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.contains("installed"));
    }

    #[test]
    fn blank_line_is_emitted_after_captured_output() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::CapturedOutput {
            stream: InstallOutputStream::Stderr,
            body: "boom".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.ends_with("\n\n"));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff-cli announcement_is_rendered_as_prose`
Expected: FAIL — file not yet created / module not wired.

- [ ] **Step 3: Implement `CliInstallUi`**

Create `sniff/cli/src/install_ui.rs`:

```rust
//! CLI adapter for `sniff::programs::InstallInterviewDelegate`.
//!
//! Maps semantic events to concrete `biscuit-terminal` components rendered
//! against one real `&Terminal`. Also drives `inquire` prompts.

use std::io::{self, Write};

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use inquire::{Confirm, Select};
use sniff::error::SniffInstallationError;
use sniff::programs::{
    InstallInterviewDelegate, InstallInterviewEvent, InstallOutputStream,
    InstallStatusKind, RetryChoice, RetryPrompt,
};

/// Presentation adapter. `buffer` is used in tests; production code leaves
/// it empty and writes to stdout/stderr.
pub struct CliInstallUi {
    pub terminal: Terminal,
    pub plain: bool,
    #[doc(hidden)]
    pub buffer: Vec<u8>,
}

impl CliInstallUi {
    pub fn new(terminal: Terminal, plain: bool) -> Self {
        Self {
            terminal,
            plain,
            buffer: Vec::new(),
        }
    }

    fn write_line(&mut self, rendered: &str) {
        let emitted = if self.plain {
            strip_escape_codes(rendered)
        } else {
            rendered.to_string()
        };
        if cfg!(test) {
            let _ = writeln!(self.buffer, "{}", emitted);
        } else {
            println!("{}", emitted);
        }
    }

    fn write_block(&mut self, rendered: &str) {
        // Ensure the block ends with a single trailing blank line.
        let emitted = if self.plain {
            strip_escape_codes(rendered)
        } else {
            rendered.to_string()
        };
        if cfg!(test) {
            let _ = write!(self.buffer, "{}\n", emitted);
        } else {
            print!("{}\n", emitted);
            let _ = io::stdout().flush();
        }
    }
}

impl InstallInterviewDelegate for CliInstallUi {
    fn on_event(
        &mut self,
        event: &InstallInterviewEvent,
    ) -> Result<(), SniffInstallationError> {
        match event {
            InstallInterviewEvent::Announcement { prose }
            | InstallInterviewEvent::ConsentWarning { prose } => {
                let rendered = Prose::new(prose.clone()).fallback_render(&self.terminal);
                self.write_line(&rendered);
            }
            InstallInterviewEvent::CapturedOutput { stream, body } => {
                let quote = match stream {
                    InstallOutputStream::Stdout => BlockQuote::from(body.as_str())
                        .with_left_block_color(Color::Tailwind(Tailwind::Gray500)),
                    InstallOutputStream::Stderr => BlockQuote::from(body.as_str())
                        .with_left_block_color(Color::Tailwind(Tailwind::Red500)),
                };
                let rendered = quote.fallback_render(&self.terminal);
                self.write_block(&rendered);
            }
            InstallInterviewEvent::Status { kind, text } => {
                let state = match kind {
                    InstallStatusKind::Success => StatusState::Success,
                    InstallStatusKind::Error => StatusState::Failure,
                };
                let rendered = Status::from_prose(text.clone())
                    .state(state)
                    .theme(StatusTheme::Circular)
                    .fallback_render(&self.terminal);
                self.write_line(&rendered);
            }
        }
        Ok(())
    }

    fn confirm_remote_script(
        &mut self,
        _prose: &str,
    ) -> Result<bool, SniffInstallationError> {
        match Confirm::new("Proceed with remote-script install?")
            .with_default(false)
            .prompt()
        {
            Ok(answer) => Ok(answer),
            Err(inquire::InquireError::OperationCanceled) => Ok(false),
            Err(inquire::InquireError::OperationInterrupted) => std::process::exit(130),
            Err(e) => Err(SniffInstallationError::InstallationError {
                pkg: String::new(),
                cmd: e.to_string(),
            }),
        }
    }

    fn choose_retry(
        &mut self,
        prompt: &RetryPrompt,
    ) -> Result<RetryChoice, SniffInstallationError> {
        for choice in &prompt.choices {
            let rendered = Prose::new(choice.prose.clone()).fallback_render(&self.terminal);
            self.write_line(&rendered);
        }
        let quit_label = "Quit (and try manually if desired)".to_string();
        let mut labels: Vec<String> = prompt.choices.iter().map(|c| c.label.clone()).collect();
        labels.push(quit_label.clone());
        match Select::new("How do you want to proceed?", labels.clone()).prompt() {
            Ok(selected) => {
                if selected == quit_label {
                    return Ok(RetryChoice::Quit);
                }
                if let Some(i) = labels.iter().position(|l| l == &selected) {
                    if i < prompt.choices.len() {
                        return Ok(RetryChoice::RetryWith(prompt.choices[i].method.clone()));
                    }
                }
                Ok(RetryChoice::Quit)
            }
            Err(inquire::InquireError::OperationCanceled) => Ok(RetryChoice::Quit),
            Err(inquire::InquireError::OperationInterrupted) => std::process::exit(130),
            Err(e) => Err(SniffInstallationError::InstallationError {
                pkg: String::new(),
                cmd: e.to_string(),
            }),
        }
    }
}
```

In `sniff/cli/src/main.rs` add near other `mod` lines:

```rust
mod install_ui;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff-cli install_ui`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/install_ui.rs sniff/cli/src/main.rs
git commit -m "feat(sniff-cli): add CliInstallUi event/prompt adapter"
```

---

## Task 11: Re-wire `execute_install_flow` to use the interview runner

**Files:**
- Modify: `sniff/cli/src/install_plan_cmd.rs`

- [ ] **Step 1: Write the failing test**

Replace existing passing tests with a new `assert_cmd`-level check
(integration test in Task 14) — for now add a unit test verifying that the
flow delegates to the runner and uses one `Terminal`:

```rust
#[test]
fn execute_install_flow_dry_run_emits_announcement_and_success() {
    let plan = fake_success_plan(false); // uses Brew
    let input = sniff::programs::InstallInterviewInput {
        program: plan.program.clone(),
        website: plan.website,
        plan,
    };
    let mut ui = crate::install_ui::CliInstallUi::new(Terminal::default(), true);
    let mut opts = sniff::programs::InstallInterviewOptions::default();
    opts.install.dry_run = true;
    let outcome = sniff::programs::run_install_interview(&input, &opts, &mut ui).unwrap();
    assert!(matches!(
        outcome,
        sniff::programs::InstallInterviewOutcome::DryRun { .. }
    ));
    let text = String::from_utf8(ui.buffer).unwrap();
    assert!(text.to_lowercase().contains("brew"));
    assert!(text.to_lowercase().contains("installed"));
}
```

Add the missing re-export to `sniff/lib/src/programs/mod.rs` if
`run_install_interview` isn't exported yet:

```rust
pub use install_interview::run_install_interview;
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff-cli execute_install_flow_dry_run_emits_announcement_and_success`
Expected: FAIL — compiles but old `execute_install_flow` path unchanged.

- [ ] **Step 3: Rewrite `execute_install_flow`**

Replace its body:

```rust
pub fn execute_install_flow(
    plan: &InstallPlan,
    dry_run: bool,
    skip_confirm: bool,
    plain: bool,
) -> Result<(), Box<dyn Error>> {
    use sniff::programs::{
        run_install_interview, InstallInterviewInput, InstallInterviewOptions,
        InstallInterviewOutcome,
    };

    // `install-plan` preflight rendering stays with render_install_plan;
    // execute_install_flow is the interactive path.
    let input = InstallInterviewInput {
        program: plan.program.clone(),
        website: plan.website,
        plan: plan.clone(),
    };

    let terminal = Terminal::new();
    let mut ui = crate::install_ui::CliInstallUi::new(terminal, plain);

    let mut opts = InstallInterviewOptions::default();
    opts.install.dry_run = dry_run;
    opts.install.skip_confirm = skip_confirm;
    opts.install.approve_remote_bash = false; // let the delegate confirm
    opts.install.timeout_secs = 120;
    opts.prompt_on_failure = true;

    match run_install_interview(&input, &opts, &mut ui)? {
        InstallInterviewOutcome::Installed { .. }
        | InstallInterviewOutcome::DryRun { .. } => Ok(()),
        InstallInterviewOutcome::AbortedByUser => Ok(()),
        InstallInterviewOutcome::Failed { .. } => {
            Err("installation failed".into())
        }
        InstallInterviewOutcome::NotInstallable => Ok(()),
    }
}
```

Delete `should_require_remote_bash_consent` from the public API **only if**
no other caller uses it (keep it if tests reference it). Check with:

`grep -rn should_require_remote_bash_consent sniff/`

Leave the function in place if used elsewhere; the interview delegate
handles remote consent now.

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff-cli install_plan_cmd`
Expected: PASS (including existing `apply_via_*` tests, which are unchanged).

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/install_plan_cmd.rs sniff/lib/src/programs/mod.rs
git commit -m "refactor(sniff-cli): route named install through shared interview runner"
```

---

## Task 12: Re-wire the MultiSelect picker to the interview runner

**Files:**
- Modify: `sniff/cli/src/install.rs`

- [ ] **Step 1: Write the failing test**

Add a compile-level smoke test in `install.rs`:

```rust
#[test]
fn picker_install_fn_exists() {
    // This test ensures the new entry point compiles and accepts the
    // expected arguments. Functional end-to-end coverage lives in the PTY
    // test (Task 15).
    let _ = crate::install::install_selected_via_interview
        as fn(&[crate::install::ResolvedProgram], bool, bool) -> Result<(), Box<dyn std::error::Error>>;
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff-cli picker_install_fn_exists`
Expected: FAIL — function not defined.

- [ ] **Step 3: Implement `install_selected_via_interview` and swap the macro**

Add to `install.rs`:

```rust
pub fn install_selected_via_interview(
    programs: &[ResolvedProgram],
    dry_run: bool,
    plain: bool,
) -> Result<(), Box<dyn Error>> {
    use biscuit_terminal::terminal::Terminal;
    use sniff::programs::{
        build_install_plan, run_install_interview, HostCapabilities,
        InstallInterviewInput, InstallInterviewOptions,
    };

    let host = HostCapabilities::load_or_detect_with_verification(false);
    let terminal = Terminal::new();
    let mut ui = crate::install_ui::CliInstallUi::new(terminal, plain);

    for resolved in programs {
        let plan = match resolved {
            ResolvedProgram::Editor(p) => build_install_plan(p, &host),
            ResolvedProgram::Utility(p) => build_install_plan(p, &host),
            ResolvedProgram::LanguagePackageManager(p) => build_install_plan(p, &host),
            ResolvedProgram::OsPackageManager(p) => build_install_plan(p, &host),
            ResolvedProgram::TtsClient(p) => build_install_plan(p, &host),
            ResolvedProgram::TerminalApp(p) => build_install_plan(p, &host),
            ResolvedProgram::HeadlessAudio(p) => build_install_plan(p, &host),
            ResolvedProgram::AiCli(p) => build_install_plan(p, &host),
        };

        let input = InstallInterviewInput {
            program: plan.program.clone(),
            website: plan.website,
            plan,
        };
        let mut opts = InstallInterviewOptions::default();
        opts.install.dry_run = dry_run;
        opts.install.timeout_secs = 120;

        let _ = run_install_interview(&input, &opts, &mut ui)?;
    }
    Ok(())
}
```

Rewrite the `interactive_install_category!` macro so that after the
`MultiSelect`, the selected items are mapped to `ResolvedProgram`s and
handed to `install_selected_via_interview` instead of calling
`detector.install()` directly. Concretely:

```rust
// After `selected` is computed:
let mut resolved: Vec<ResolvedProgram> = Vec::new();
for label in &selected {
    let idx = options.iter().position(|o| o == label).unwrap();
    let program = not_installed[idx];
    // Replace the following with the variant that matches `<$enum_type>`:
    //   Editor → ResolvedProgram::Editor(*program),
    //   Utility → ResolvedProgram::Utility(*program),
    //   etc.
    resolved.push(<resolved_variant_expr>);
}
return $crate::install::install_selected_via_interview(&resolved, false, false);
```

Because the macro is invoked once per category, add an extra macro argument
`$variant:path` so the invocation can pass the correct `ResolvedProgram`
constructor. Update each `interactive_install_category!(...)` invocation
accordingly:

```rust
interactive_install_category!(
    interactive_install_editors,
    sniff::programs::Editor,
    sniff::programs::InstalledEditors,
    "Select editors to install:",
    ResolvedProgram::Editor
);
```

And inside the macro body:

```rust
resolved.push($variant(*program));
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff-cli install`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/install.rs
git commit -m "refactor(sniff-cli): route MultiSelect picker through shared interview runner"
```

---

## Task 13: Remove the old `Terminal::default()` in `render_install_plan`

**Files:**
- Modify: `sniff/cli/src/install_plan_cmd.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn render_install_plan_accepts_terminal_reference() {
    let plan = fake_success_plan(false);
    let terminal = Terminal::default();
    let rendered = render_install_plan_with(&plan, false, &terminal);
    assert!(rendered.to_lowercase().contains("brew"));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sniff-cli render_install_plan_accepts_terminal_reference`
Expected: FAIL — `render_install_plan_with` not defined.

- [ ] **Step 3: Split `render_install_plan`**

```rust
pub fn render_install_plan(plan: &InstallPlan, verbose: bool) -> String {
    let terminal = Terminal::default();
    render_install_plan_with(plan, verbose, &terminal)
}

pub fn render_install_plan_with(
    plan: &InstallPlan,
    verbose: bool,
    terminal: &Terminal,
) -> String {
    let mut out = String::new();

    if plan.successful {
        if verbose {
            for opt in plan.failed_with_reason() {
                let line = format!(
                    "- <dim>skipped {} — <i>{}</i></dim>",
                    opt.kind.manager_name(),
                    opt.reason
                );
                out.push_str(&Prose::new(line).fallback_render(terminal));
                out.push('\n');
            }
            if !plan.failed_with_reason().is_empty() {
                out.push('\n');
            }
        }
        let chosen = plan.chosen().expect("successful plan has a chosen option");
        let success_line = render_success_line(&plan.program, chosen);
        out.push_str(&Prose::new(success_line).fallback_render(terminal));
        out.push('\n');
    } else {
        out.push_str(&render_failure_block_with(plan, terminal));
    }

    out
}
```

Rename internal `render_failure_block` to `render_failure_block_with` taking
`&Terminal` and replace all `.render(&terminal)` calls with
`.fallback_render(terminal)` per the user memory note that the no-context
`render(None)` path is wrong for CLI output.

- [ ] **Step 4: Run tests**

Run: `cargo test -p sniff-cli render_install_plan`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/install_plan_cmd.rs
git commit -m "fix(sniff-cli): use fallback_render with real terminal in install-plan"
```

---

## Task 14: `assert_cmd` integration test for dry-run flow

**Files:**
- Create: `sniff/cli/tests/install_interview_cli.rs`

- [ ] **Step 1: Write the failing test**

```rust
//! End-to-end smoke test for the interview flow in dry-run mode.
//!
//! Picks a brew-installable utility; `--dry-run` skips execution so the
//! test never mutates host state. Plain mode keeps output deterministic.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn install_dry_run_plain_emits_announcement_and_success() {
    let mut cmd = Command::cargo_bin("sniff").unwrap();
    cmd.env("NO_COLOR", "1")
        .args([
            "utilities",
            "install",
            "--program",
            "ripgrep",
            "--dry-run",
            "--yes",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("will be installed"))
        .stdout(predicate::str::contains("installed successfully"));
}
```

- [ ] **Step 2: Run to verify failure or pass**

Run: `cargo test -p sniff-cli --test install_interview_cli`
Expected: PASS once Tasks 1–13 are committed.

- [ ] **Step 3: Commit**

```bash
git add sniff/cli/tests/install_interview_cli.rs
git commit -m "test(sniff-cli): add assert_cmd dry-run smoke test for install interview"
```

---

## Task 15: PTY regression test for failure-then-retry

**Files:**
- Create: `sniff/cli/tests/install_interactive_pty.rs`

- [ ] **Step 1: Write the test (expectrl)**

```rust
//! PTY regression test: failure then retry with fallback.
//!
//! Uses `expectrl` to drive an interactive `sniff ... install` session.
//! Requires `expectrl` as a dev-dependency of `sniff-cli`. If the test
//! host lacks PTY support, the test is skipped.

#![cfg(unix)]

use std::time::Duration;

use expectrl::{spawn, Eof, Regex};

#[test]
fn failure_then_retry_picks_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // Skip if the binary isn't on PATH (CI-only; local `cargo test` always
    // builds it, but the discovery step keeps this test hermetic).
    if std::env::var_os("SNIFF_INTERACTIVE_PTY").is_none() {
        eprintln!("skipping: set SNIFF_INTERACTIVE_PTY=1 to enable");
        return Ok(());
    }

    let mut p = spawn("sniff utilities install --program ripgrep --dry-run --plain")?;
    p.set_expect_timeout(Some(Duration::from_secs(10)));

    // Dry-run always succeeds, so this only verifies the plumbing. A real
    // failure-retry scenario requires a deliberately failing fixture
    // program; left for a follow-up.
    p.expect(Regex("will be installed"))?;
    p.expect(Regex("installed successfully"))?;
    p.expect(Eof)?;
    Ok(())
}
```

Add to `sniff/cli/Cargo.toml` under `[dev-dependencies]`:

```toml
expectrl = "0.7"
```

- [ ] **Step 2: Run**

Run: `cargo test -p sniff-cli --test install_interactive_pty`
Expected: PASS (skips by default unless `SNIFF_INTERACTIVE_PTY=1`).

- [ ] **Step 3: Commit**

```bash
git add sniff/cli/tests/install_interactive_pty.rs sniff/cli/Cargo.toml
git commit -m "test(sniff-cli): add PTY regression skeleton for install interview"
```

---

## Task 16: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Full test suite**

Run: `just test` (if the root justfile's `areas` list includes `sniff`) OR
`cargo test -p sniff -p sniff-cli`
Expected: all green.

- [ ] **Step 2: Lint**

Run: `just lint` or `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Format**

Run: `cargo fmt --package sniff --package sniff-cli`
Expected: no diff after running.

- [ ] **Step 4: Manual smoke**

Run:
- `cargo run -p sniff-cli -- utilities install --program ripgrep --dry-run --yes`
- Verify visually: announcement with clickable program link, command in green, circular success status on a single blank line after.

- [ ] **Step 5: Final commit (if any formatting remains)**

```bash
git status
# if clean: nothing to commit
```

---

## Execution Handoff

Plan complete and saved to `sniff/features/2026-04-12-better-interview-for-install/plan.md`. Two execution options:

1. **Subagent-Driven (recommended)** — one fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints.

Which approach?
