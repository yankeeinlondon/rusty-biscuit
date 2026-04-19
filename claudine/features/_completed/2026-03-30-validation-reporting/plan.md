# Validation Reporting Implementation Plan

This plan implements the validation-reporting feature described in `spec.md` and `tech-design.md`. Each phase is designed to be independently testable and incrementally buildable.

## Inputs

- `claudine/features/2026-03-28-validation-reporting.md/spec.md`
- `claudine/features/2026-03-28-validation-reporting.md/tech-design.md`

## Phase 1: Data Model Additions

**Goal:** Add the structured outcome and shell audit types that all subsequent phases depend on.

### 1.1 Add `ValidationCheckOutcome` and `ValidationPhaseReport` to `claudine/lib/src/harness/model.rs`

```rust
/// Structured outcome of evaluating a single validation rule.
pub struct ValidationCheckOutcome {
    pub rule_id: ValidationRuleId,
    pub event: ValidationEvent,
    pub subject_key: Option<String>,
    pub passed: bool,
    /// Prose-ready markup body (not ANSI-rendered). Rendering belongs in `report.rs`.
    pub markup: String,
    /// Human-readable failure reason when `passed` is false.
    pub failure_message: Option<String>,
}

/// All outcomes for one validation phase (pre or post).
pub struct ValidationPhaseReport {
    pub phase: FailurePhase,
    pub outcomes: Vec<ValidationCheckOutcome>,
}
```

Add derived traits: `Debug`, `Clone`.

`ValidationPhaseReport` should expose convenience methods:

```rust
impl ValidationPhaseReport {
    /// True when every outcome passed.
    pub fn all_passed(&self) -> bool { ... }

    /// Collect failed outcomes into `Vec<ValidationFailure>` for existing error propagation.
    pub fn failures(&self) -> Vec<ValidationFailure> { ... }

    /// Number of checks in this phase.
    pub fn count(&self) -> usize { ... }
}
```

### 1.2 Add shell audit types to `claudine/lib/src/harness/model.rs`

```rust
/// Where an audited command originates.
#[derive(Debug, Clone)]
pub enum AuditedCommandSource {
    PreCheck(ValidationRuleId),
    PostCheck(ValidationRuleId),
    ProgrammaticHandle,
    DeclarativeHandler {
        event: FailureEvent,
        subject_key: Option<String>,
    },
    ComposeSourceLine {
        line: usize,
    },
}

/// A command discovered during shell audit.
#[derive(Debug, Clone)]
pub struct AuditedCommand {
    pub source: AuditedCommandSource,
    pub raw: String,
    pub executable: String,
    pub args: Vec<String>,
}

/// Result of auditing a single command.
#[derive(Debug, Clone)]
pub struct ShellAuditOutcome {
    pub command: AuditedCommand,
    pub passed: bool,
    /// Prose-ready human-readable message.
    pub message: String,
}

/// Complete audit report.
#[derive(Debug, Clone)]
pub struct ShellAuditReport {
    pub outcomes: Vec<ShellAuditOutcome>,
}
```

`ShellAuditReport` convenience:

```rust
impl ShellAuditReport {
    pub fn all_passed(&self) -> bool { ... }
    pub fn failures(&self) -> Vec<&ShellAuditOutcome> { ... }
}
```

### 1.3 Export new types from `claudine/lib/src/harness/mod.rs`

Add the new types to the existing `pub use model::*` re-export. Since `model.rs` already uses a glob re-export, no new `pub use` lines are needed unless the glob is removed later.

### 1.4 Tests

- Unit test `ValidationPhaseReport::all_passed` returns true when empty, true when all pass, false when any fail.
- Unit test `ValidationPhaseReport::failures` collects correct subset.
- Unit test `ShellAuditReport::all_passed` and `failures`.

**Files touched:** `claudine/lib/src/harness/model.rs`

---

## Phase 2: Refactor `validate.rs` to Return Structured Outcomes

**Goal:** Stop printing from the validation engine. Return `ValidationPhaseReport` so callers control rendering.

### 2.1 Change `run_checks` return type

Current signature (private):

```rust
fn run_checks(
    rules: &[ValidationRule],
    snapshot: Option<&PreRunSnapshot>,
    outcome: Option<&AttemptOutcome>,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
    failure_phase: FailurePhase,
    term: &Terminal,
) -> Vec<ValidationFailure>
```

New signature — remove `term` parameter, return `ValidationPhaseReport`:

```rust
fn run_checks(
    rules: &[ValidationRule],
    snapshot: Option<&PreRunSnapshot>,
    outcome: Option<&AttemptOutcome>,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
    failure_phase: FailurePhase,
) -> ValidationPhaseReport
```

Inside the loop, instead of calling `render_check_result` and `eprintln!`, build a `ValidationCheckOutcome` for each rule:

```rust
for rule in rules {
    let result = evaluate_single(rule, snapshot, outcome, source_path, permission_probe, post_run_markdown.as_ref());
    let passed = result.is_ok();
    let markup = build_check_markup(rule, &result);  // extracted from render_check_result
    let failure_message = result.err();
    outcomes.push(ValidationCheckOutcome {
        rule_id: rule.id,
        event: rule.event.clone(),
        subject_key: rule.subject_key.clone(),
        passed,
        markup,
        failure_message,
    });
}
```

### 2.2 Extract `build_check_markup` from `render_check_result`

Current `render_check_result` (line ~786) does two things:
1. Builds the prose markup string (template interpolation, status token insertion)
2. Renders it through `Prose::new(...).render(term)`

Split into:
- `build_check_markup(rule, result) -> String` — returns the prose-ready markup (no ANSI, no terminal). This is the template-interpolated string with `{{status}}` replaced by the appropriate Prose markup token (`<b><green-500>✓</green-500></b>` or `<b><red-500>⊘</red-500></b>`).
- Keep `render_check_result` as a thin wrapper that calls `build_check_markup` + `Prose::new(...).render(term)` — this stays in `validate.rs` only for existing tests that assert rendered output.

### 2.3 Change public function signatures

**`evaluate_pre_checks`:**

```rust
pub fn evaluate_pre_checks(
    plan: &HarnessPlan,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> ValidationPhaseReport
```

Remove `term` parameter. Return `ValidationPhaseReport` directly (never `Err`). The caller decides whether failures are fatal.

**`evaluate_post_checks`:**

```rust
pub fn evaluate_post_checks(
    plan: &HarnessPlan,
    snapshot: &PreRunSnapshot,
    outcome: &AttemptOutcome,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> ValidationPhaseReport
```

Same pattern — remove `term`, return report.

### 2.4 Provide a bridge for existing error flow

Add a helper to convert from report to the existing error variant:

```rust
impl ValidationPhaseReport {
    /// Convert to the legacy error if any checks failed.
    pub fn into_result(self) -> Result<Self, HarnessError> {
        if self.all_passed() {
            Ok(self)
        } else {
            let failures = self.failures();
            match self.phase {
                FailurePhase::PreCheck => Err(HarnessError::PreCheckFailed { failures }),
                FailurePhase::PostCheck => Err(HarnessError::PostCheckFailed { failures }),
                _ => Err(HarnessError::PreCheckFailed { failures }),
            }
        }
    }
}
```

### 2.5 Update `mod.rs` exports

The public API changes from returning `Result<(), HarnessError>` to returning `ValidationPhaseReport`. Update the re-export in `claudine/lib/src/harness/mod.rs` (already covered by glob).

### 2.6 Tests

- Existing `render_check_result_success_token` and `render_check_result_failure_token` tests should still pass (the thin wrapper is preserved).
- New test: `evaluate_pre_checks` returns a `ValidationPhaseReport` with correct `passed` flags and `markup` content.
- New test: `evaluate_post_checks` returns report with correct `failure_message` values.
- New test: `build_check_markup` produces expected markup strings for each `ValidationKind` variant.
- New test: `ValidationPhaseReport::into_result` returns `Ok` when all pass, `Err(PreCheckFailed)` when any fail.

**Files touched:** `claudine/lib/src/harness/validate.rs`, `claudine/lib/src/harness/model.rs`, `claudine/lib/src/harness/mod.rs`

---

## Phase 3: Add `harness/report.rs`

**Goal:** Own all validation-related presentation logic through `Status::from_prose` with `StatusTheme::Circular`.

### 3.1 Create `claudine/lib/src/harness/report.rs`

This module renders all validation-related status output to stderr. Every function takes `&Terminal` and writes to stderr.

**Core rendering helper:**

```rust
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::Terminal;

/// Render a single status line to stderr.
fn emit_status(markup: &str, state: StatusState, term: &Terminal) {
    let rendered = Status::from_prose(markup)
        .state(state)
        .theme(StatusTheme::Circular)
        .render(term);
    eprintln!("{rendered}");
}
```

### 3.2 Source-file existence reporting

```rust
/// Emit the source-file existence status.
///
/// Uses the spec's exact messages with prose-escaped user fragments.
pub fn report_source_file(
    original_ref: &str,
    resolved_path: &Path,
    term: &Terminal,
) {
    let ref_escaped = prose_escape(original_ref);
    let path_display = resolved_path.display().to_string();
    let path_escaped = prose_escape(&path_display);

    if resolved_path.exists() {
        let filepath = resolved_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_display.clone());
        let filepath_escaped = prose_escape(&filepath);
        let abs_escaped = prose_escape(&path_display);
        emit_status(
            &format!(
                "the file reference <blue-500>{ref_escaped}</blue-500> to the \
                 <blue-500><a href=\"{abs_escaped}\">{filepath_escaped}</a></blue-500> \
                 file on this host"
            ),
            StatusState::Success,
            term,
        );
    } else {
        emit_status(
            &format!(
                "the file reference <blue-500>{ref_escaped}</blue-500> \
                 found no match on host computer!"
            ),
            StatusState::Failure,
            term,
        );
    }
}
```

### 3.3 Prose escaping helper

```rust
/// Escape user-controlled strings for safe Prose interpolation.
///
/// Escapes `<`, `>`, `{`, `}`, and `\` to prevent unintended markup.
fn prose_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('{', "\\{")
        .replace('}', "\\}")
}
```

### 3.4 Phase discovery headers

```rust
/// Emit the discovery header for a validation phase.
///
/// Only emits when count > 0. Uses correct singular/plural grammar.
pub fn report_phase_discovery(phase: FailurePhase, count: usize, term: &Terminal) {
    if count == 0 {
        return;
    }
    let phase_label = match phase {
        FailurePhase::PreCheck => "pre",
        FailurePhase::PostCheck => "post",
        _ => return,
    };
    let (check_word, verb) = if count == 1 {
        ("check", "was")
    } else {
        ("checks", "were")
    };
    emit_status(
        &format!("<b>{count}</b> validation <i>{phase_label} {check_word}</i> {verb} found:"),
        StatusState::Info,
        term,
    );
}
```

### 3.5 Validation result items

```rust
/// Emit individual check outcomes from a phase report.
pub fn report_check_outcomes(report: &ValidationPhaseReport, term: &Terminal) {
    for outcome in &report.outcomes {
        let state = if outcome.passed {
            StatusState::Success
        } else {
            StatusState::Failure
        };
        emit_status(&outcome.markup, state, term);
    }
}
```

### 3.6 Shell audit reporting

```rust
/// Emit the shell audit header.
pub fn report_shell_audit_header(count: usize, term: &Terminal) {
    if count == 0 {
        return;
    }
    let (cmd_word, verb) = if count == 1 {
        ("command", "was")
    } else {
        ("commands", "were")
    };
    emit_status(
        &format!("<b>{count}</b> shell {cmd_word} {verb} audited:"),
        StatusState::Info,
        term,
    );
}

/// Emit individual shell audit outcomes.
pub fn report_shell_audit_outcomes(report: &ShellAuditReport, term: &Terminal) {
    for outcome in &report.outcomes {
        let state = if outcome.passed {
            StatusState::Success
        } else {
            StatusState::Failure
        };
        emit_status(&outcome.message, state, term);
    }
}
```

### 3.7 Handler engagement and terminal failure banners

```rust
/// Emit the handler-engagement banner once per failure episode.
///
/// Uses the spec's exact message.
pub fn report_handler_engagement(source_display: &str, term: &Terminal) {
    let escaped = prose_escape(source_display);
    emit_status(
        &format!(
            "an <red>error</red> was encountered while processing \
             <blue>{escaped}</blue>, engaging registered handlers."
        ),
        StatusState::Warning,
        term,
    );
}

/// Emit a terminal unhandled failure banner.
pub fn report_unhandled_failure(message: &str, term: &Terminal) {
    emit_status(message, StatusState::Failure, term);
}
```

### 3.8 Register the module

Add `pub mod report;` to `claudine/lib/src/harness/mod.rs` and export the public functions.

### 3.9 Tests

- `report_source_file` with existing path emits `Success` state (test by capturing stderr or by testing the underlying `emit_status` logic with a mock).
- `report_source_file` with missing path emits `Failure` state.
- `report_phase_discovery` emits nothing when count is 0.
- `report_phase_discovery` with count=1 uses singular grammar.
- `report_phase_discovery` with count=3 uses plural grammar.
- `prose_escape` escapes `<`, `>`, `{`, `}`, `\`.
- `report_handler_engagement` produces the spec-mandated message.

**Files touched:** `claudine/lib/src/harness/report.rs` (new), `claudine/lib/src/harness/mod.rs`

---

## Phase 4: Add `harness/audit.rs`

**Goal:** Shell-command discovery and policy preflight without execution.

### 4.1 Create `claudine/lib/src/harness/audit.rs`

### 4.2 Command collection

Collect all auditable commands from a `HarnessPlan` and optional source page text:

```rust
/// Collect all shell commands that must pass audit before the run proceeds.
pub fn collect_auditable_commands(
    plan: &HarnessPlan,
    source_text: Option<&str>,
) -> Result<Vec<AuditedCommand>, HarnessError>
```

**Sources to inspect (in order):**

1. **`pre_checks`** — iterate `plan.pre_checks`, extract commands from `ValidationKind::ShellCommand { command, .. }`:
   ```rust
   AuditedCommand {
       source: AuditedCommandSource::PreCheck(rule.id),
       raw: command.raw.clone(),
       executable: command.executable.clone(),
       args: command.args.clone(),
   }
   ```

2. **`post_checks`** — same pattern with `AuditedCommandSource::PostCheck(rule.id)`.

3. **Programmatic `handle`** — if `plan.programmatic_handler` is `Some(cmd)`:
   ```rust
   AuditedCommand {
       source: AuditedCommandSource::ProgrammaticHandle,
       raw: cmd.raw.clone(),
       executable: cmd.executable.clone(),
       args: cmd.args.clone(),
   }
   ```

4. **Declarative `deviate` handlers** — iterate `plan.handlers.exact` and `plan.handlers.generic`, extract from `HandlerAction::Deviate { command, .. }`:
   ```rust
   AuditedCommand {
       source: AuditedCommandSource::DeclarativeHandler {
           event: rule.event.clone(),
           subject_key: rule.subject_key.clone(),
       },
       raw: command.raw.clone(),
       executable: command.executable.clone(),
       args: command.args.clone(),
   }
   ```

5. **Source page `::shell` directives** — when `source_text` is provided, use Darkmatter's parser:
   ```rust
   use darkmatter::markdown::compose::shell_expansion::parser::parse_directives;

   if let Some(text) = source_text {
       for directive in parse_directives(text)? {
           commands.push(AuditedCommand {
               source: AuditedCommandSource::ComposeSourceLine { line: directive.line },
               raw: directive.raw_command.clone(),
               executable: directive.executable.clone(),
               args: directive.args.clone(),
           });
       }
   }
   ```

   Map `ShellExpansionError` to a suitable `HarnessError` variant. Add a new variant if needed:
   ```rust
   ShellAuditParseError { detail: String }
   ```

### 4.3 Audit execution

Run each collected command through the existing shell policy without executing it:

```rust
/// Audit all collected commands against shell policy.
///
/// Reuses `validate_and_approve_command_parts` for harness commands.
/// Reuses Darkmatter's policy primitives for source-page directives.
pub fn audit_shell_commands(
    commands: &[AuditedCommand],
    options: &ShellApprovalOptions,
) -> ShellAuditReport
```

For each command:

```rust
let result = validate_and_approve_command_parts(
    &std::iter::once(cmd.executable.clone())
        .chain(cmd.args.iter().cloned())
        .collect::<Vec<_>>(),
    options,
);
match result {
    Ok(_) => ShellAuditOutcome {
        command: cmd.clone(),
        passed: true,
        message: format!("<green-500>{}</green-500> approved", prose_escape(&cmd.raw)),
    },
    Err(HarnessError::ShellCommandDenied { .. }) => ShellAuditOutcome {
        command: cmd.clone(),
        passed: false,
        message: format!("<red-500>{}</red-500> denied by policy", prose_escape(&cmd.raw)),
    },
    Err(HarnessError::ShellCommandBlacklisted { reason, .. }) => ShellAuditOutcome {
        command: cmd.clone(),
        passed: false,
        message: format!(
            "<red-500>{}</red-500> blacklisted: {}",
            prose_escape(&cmd.raw),
            prose_escape(&reason),
        ),
    },
    Err(e) => ShellAuditOutcome {
        command: cmd.clone(),
        passed: false,
        message: format!(
            "<red-500>{}</red-500> audit error: {}",
            prose_escape(&cmd.raw),
            prose_escape(&e.to_string()),
        ),
    },
}
```

### 4.4 Register the module

Add `pub mod audit;` to `claudine/lib/src/harness/mod.rs` and export public functions:

```rust
pub use audit::{audit_shell_commands, collect_auditable_commands};
```

### 4.5 Tests

- `collect_auditable_commands` with empty plan returns empty vec.
- `collect_auditable_commands` with `ShellCommand` pre-check returns `PreCheck` source.
- `collect_auditable_commands` with `ShellCommand` post-check returns `PostCheck` source.
- `collect_auditable_commands` with programmatic handler returns `ProgrammaticHandle` source.
- `collect_auditable_commands` with `deviate` handler returns `DeclarativeHandler` source.
- `collect_auditable_commands` with source text containing `::shell echo hello` returns `ComposeSourceLine` source.
- `collect_auditable_commands` ignores `retry`/`resume`/`redirect` handlers.
- `audit_shell_commands` with all-whitelisted commands returns `all_passed() == true`.
- `audit_shell_commands` with a blacklisted command returns failure with reason.
- `audit_shell_commands` with a built-in blacklisted command (e.g., `rm -rf /`) returns failure.

**Files touched:** `claudine/lib/src/harness/audit.rs` (new), `claudine/lib/src/harness/mod.rs`, possibly `claudine/lib/src/harness/error.rs`

---

## Phase 5: Thread `original_ref` Through Wrapper Prompt State

**Goal:** Make the original file reference string available in the harness loop for reporting.

### 5.1 Add `original_ref` to `HarnessPromptState`

In `claudine/cli/src/commands/wrap/mod.rs`, add the field:

```rust
pub(crate) struct HarnessPromptState {
    pub(crate) mode: HarnessPromptMode,
    pub(crate) source_path: PathBuf,
    pub(crate) original_ref: String,  // NEW
    pub(crate) base_prompt: Option<String>,
    pub(crate) overlay: indexmap::IndexMap<String, serde_json::Value>,
    pub(crate) prompt_tail: Vec<String>,
    pub(crate) next_prompt_override: Option<String>,
    pub(crate) next_resume_session_id: Option<String>,
}
```

### 5.2 Populate `original_ref` at construction sites

Find all places where `HarnessPromptState` is constructed (search for `HarnessPromptState {`). At each site:

- If coming from `CompositionExecutionRequest` / `PreparedComposition` / `ResolvedCompositionSource`, use `source.original_ref.clone()`.
- If coming from the passthrough wrapper path (line ~1522 where `original_ref: state.source_path.display().to_string()` is already used for `ResolvedCompositionSource`), use `state.source_path.display().to_string()`.

### 5.3 Tests

- No unit tests needed for this field — it is a data-threading change. Integration tests in Phase 6 will validate it.

**Files touched:** `claudine/cli/src/commands/wrap/mod.rs`

---

## Phase 6: Integrate Reporting into `run_harness_loop`

**Goal:** Wire up source-file reporting, shell audit, phase discovery headers, check-outcome rendering, handler-engagement banners, and terminal failure banners into the harness loop.

### 6.1 Source-file status (top of each iteration)

At the top of the loop body in `run_harness_loop`, before harness plan parsing:

```rust
if show_checks {
    claudine::harness::report::report_source_file(
        &prompt_state.original_ref,
        &prompt_state.source_path,
        term,
    );
}
```

If the source file does not exist, return an error immediately after emitting the status — there is no plan to parse.

### 6.2 Shell audit (after plan parse, before pre-checks)

After `parse_harness_plan_with_shell` and the inline writability pre-check insertion:

```rust
// Read source text for shell directive audit
let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();
let auditable = claudine::harness::collect_auditable_commands(
    &plan,
    source_text.as_deref(),
)?;

let audit_report = claudine::harness::audit_shell_commands(
    &auditable,
    harness_context.shell_options(),
);

if show_checks {
    claudine::harness::report::report_shell_audit_header(audit_report.outcomes.len(), term);
    claudine::harness::report::report_shell_audit_outcomes(&audit_report, term);
}

if !audit_report.all_passed() {
    let harness_failures: Vec<_> = audit_report.failures().iter().filter(|o| {
        !matches!(o.command.source, AuditedCommandSource::ComposeSourceLine { .. })
    }).collect();
    let source_failures: Vec<_> = audit_report.failures().iter().filter(|o| {
        matches!(o.command.source, AuditedCommandSource::ComposeSourceLine { .. })
    }).collect();

    // Source-page ::shell failures are terminal in v1
    if !source_failures.is_empty() {
        if show_checks {
            claudine::harness::report::report_unhandled_failure(
                "shell audit failed for source-page directives — cannot proceed",
                term,
            );
        }
        return Err(eyre!("shell audit failed: {} denied directive(s) in source page",
            source_failures.len()));
    }

    // Harness-declared command failures can flow through handler resolution
    if !harness_failures.is_empty() {
        // Convert to failure contexts and attempt handler resolution
        // (similar pattern to existing pre-check failure handling)
        // If no handler resolves, emit terminal failure and return error
        if show_checks {
            claudine::harness::report::report_unhandled_failure(
                &format!("shell audit failed: {} denied command(s)", harness_failures.len()),
                term,
            );
        }
        return Err(eyre!("shell audit failed"));
    }
}
```

### 6.3 Pre-check discovery and execution

Replace the current `evaluate_pre_checks` call with the new structured flow:

```rust
let pre_report = claudine::harness::evaluate_pre_checks(&plan, Some(&permission_probe));

if show_checks {
    claudine::harness::report::report_phase_discovery(
        FailurePhase::PreCheck,
        pre_report.count(),
        term,
    );
    claudine::harness::report::report_check_outcomes(&pre_report, term);
}

if !pre_report.all_passed() {
    let failures = pre_report.failures();
    // ... existing handler resolution logic using `failures` ...
    // On entering recovery:
    if show_checks {
        let source_display = prompt_state.source_path.display().to_string();
        claudine::harness::report::report_handler_engagement(&source_display, term);
    }
    // ... apply_next_attempt_plan and continue ...
    // On terminal failure:
    if show_checks {
        claudine::harness::report::report_unhandled_failure(
            &format!("pre-check validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 { "failure" } else { "failures" }),
            term,
        );
    }
    return Err(eyre!("pre-check validation failed"));
}
```

### 6.4 Post-check discovery and execution

Same pattern for `evaluate_post_checks`:

```rust
let post_report = claudine::harness::evaluate_post_checks(
    &plan, &snapshot, &outcome, Some(&permission_probe),
);

if show_checks {
    claudine::harness::report::report_phase_discovery(
        FailurePhase::PostCheck,
        post_report.count(),
        term,
    );
    claudine::harness::report::report_check_outcomes(&post_report, term);
}

if post_report.all_passed() {
    return Ok(outcome.exit_code);
}

let failures = post_report.failures();
// ... existing handler resolution ...
```

### 6.5 Handler engagement banner

Before applying any recovery action (in all four handler-resolution sites: pre-check, agent failure, inline closure, post-check), emit the handler-engagement banner **once**:

```rust
if show_checks {
    claudine::harness::report::report_handler_engagement(
        &prompt_state.source_path.display().to_string(),
        term,
    );
}
```

This replaces any existing ad-hoc warning output at these sites. Emit once per failure episode, not once per failed check.

### 6.6 Terminal failure banner

Before each `return Err(...)` in the harness loop where validation or agent failure is unhandled:

```rust
if show_checks {
    claudine::harness::report::report_unhandled_failure(&message, term);
}
```

### 6.7 `show_checks` / silent suppression

All reporting calls are already gated behind `if show_checks`. This preserves the existing `--silent` suppression behavior.

### 6.8 Tests

Integration-level tests in `claudine/cli/`:

1. **Missing source file** — reports `Failure` status, returns error before provider launch.
2. **Existing source file** — reports `Success` status with correct ref/path.
3. **Populated `pre_checks`** — emits discovery header with correct count and grammar, then itemized outcomes.
4. **Populated `post_checks`** — emits discovery header and itemized outcomes.
5. **Denied shell command in `pre_checks`** — fails during shell audit, not during check execution.
6. **Denied `::shell` directive** — fails before provider launch with terminal status.
7. **Handled pre-check failure** — emits handler-engagement banner exactly once (not per-check).
8. **Redirect** — next iteration re-emits source-file status and shell audit for the new file.
9. **`--silent`** — no status output emitted, exit behavior preserved.

**Files touched:** `claudine/cli/src/commands/wrap/mod.rs`

---

## Phase 7: Integrate into Composition Path

**Goal:** Ensure `claudine compose` and `claudine inline-compose` also get the new reporting.

### 7.1 Update `claudine/cli/src/commands/wrap/composition.rs`

The composition path (`run_inline_harness_attempt` and related functions) currently uses `fm_check_ok`/`fm_check_fail` for validation output. These should be migrated to use the same `report.rs` functions.

Specifically:
- Replace `log::message(&crate::output::fm_check_ok(...))` calls with `claudine::harness::report::emit_status(...)` using `StatusState::Success`.
- Replace `log::message(&crate::output::fm_check_fail(...))` calls with `claudine::harness::report::report_unhandled_failure(...)` or appropriate status calls.

Note: Only migrate validation-related output. Non-validation output (provider completion status, interruption reports) stays as-is per the tech design's non-goal #5.

### 7.2 Tests

- Covered by Phase 6 integration tests (the composition path flows through `run_harness_loop`).

**Files touched:** `claudine/cli/src/commands/wrap/composition.rs`

---

## Phase 8: Documentation

**Goal:** Update docs to describe the new reporting contract and shell-audit preflight.

### 8.1 Update `claudine/docs/topics/validations-and-handlers.md`

Add sections covering:
- Status-based reporting (all validation output uses `Status` with circular theme)
- Shell audit preflight (when it runs, what it inspects, failure behavior)
- Handler engagement banner (when it appears, what it means)
- The target UX sequence from the tech design

### 8.2 Update `claudine/lib/README.md`

Add mention of the new `report.rs` and `audit.rs` modules in the harness module description.

**Files touched:** `claudine/docs/topics/validations-and-handlers.md`, `claudine/lib/README.md`

---

## Implementation Sequence Summary

| Phase | Description | Depends On | New Files |
|-------|-------------|------------|-----------|
| 1 | Data model additions | — | — |
| 2 | Refactor `validate.rs` to return structured outcomes | Phase 1 | — |
| 3 | Add `report.rs` | Phase 1 | `harness/report.rs` |
| 4 | Add `audit.rs` | Phase 1 | `harness/audit.rs` |
| 5 | Thread `original_ref` through wrapper | — | — |
| 6 | Integrate into `run_harness_loop` | Phases 2, 3, 4, 5 | — |
| 7 | Integrate into composition path | Phase 3 | — |
| 8 | Documentation | Phase 6 | — |

Phases 2, 3, 4, and 5 can proceed in parallel after Phase 1. Phase 6 is the integration point. Phase 7 can follow Phase 3. Phase 8 follows Phase 6.

## StatusState Mapping Reference

| Situation | StatusState |
|-----------|-------------|
| Source file found | `Success` |
| Source file missing | `Failure` |
| Phase discovery header | `Info` |
| Shell audit header | `Info` |
| Shell audit item approved | `Success` |
| Shell audit item denied/blacklisted | `Failure` |
| Individual validation pass | `Success` |
| Individual validation fail | `Failure` |
| Handler engagement banner | `Warning` |
| Terminal unhandled failure | `Failure` |
