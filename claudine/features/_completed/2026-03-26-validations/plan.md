# Validations, Timeouts, and Handlers: Implementation Plan

This plan implements the harness feature described in `spec.md` and `tech-design.md`. It is organized into seven phases following the tech design's recommended delivery order, where each phase produces a testable, self-contained increment.

---

## Phase 1: Harness Module Scaffold, Types, and Timeout Parsing

**Goal:** Establish the `claudine::harness` library module with all core types, frontmatter parsing, and timeout string parsing. No runtime behavior yet -- just the data model and parse layer.

### 1.1 Create module scaffold

**Files to create:**

- `claudine/lib/src/harness/mod.rs` -- public re-exports
- `claudine/lib/src/harness/error.rs` -- `HarnessError` enum
- `claudine/lib/src/harness/model.rs` -- all data model structs/enums
- `claudine/lib/src/harness/parse.rs` -- frontmatter-to-plan parser
- `claudine/lib/src/harness/timeout.rs` -- timeout string parser
- `claudine/lib/src/harness/resolve.rs` -- source-relative path resolution

**Files to modify:**

- `claudine/lib/src/lib.rs` -- add `pub mod harness;`

### 1.2 Define `HarnessError`

Create a dedicated error enum in `error.rs` covering:

```rust
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    // --- Parse / configuration ---
    #[error("...")]
    InvalidFrontmatter { source_path: PathBuf, property: String, detail: String },
    #[error("...")]
    UnknownValidation { source_path: PathBuf, name: String },
    #[error("...")]
    PostOnlyInPreChecks { source_path: PathBuf, name: String },
    #[error("...")]
    InvalidTimeout { source_path: PathBuf, raw: String },
    #[error("...")]
    InvalidShape { source_path: PathBuf, raw: String },
    #[error("...")]
    MissingHandlerField { source_path: PathBuf, handler: String, field: String },

    // --- Runtime validation failures ---
    #[error("...")]
    PreCheckFailed { failures: Vec<ValidationFailure> },
    #[error("...")]
    PostCheckFailed { failures: Vec<ValidationFailure> },

    // --- Handler resolution failures ---
    #[error("...")]
    ResumeUnsupported { provider: String },
    #[error("...")]
    ResumeNoSession,
    #[error("...")]
    HandlerFailed { action: String, detail: String },

    // --- Shell approval failures ---
    #[error("...")]
    ShellCommandDenied { command: String },
    #[error("...")]
    ShellCommandBlacklisted { command: String, reason: String },

    // --- Path resolution ---
    #[error("...")]
    RepoRootRequired { path: String },
    #[error("...")]
    PathResolutionFailed { raw: String, detail: String },
}
```

### 1.3 Define core data model

Create all structs/enums in `model.rs` as specified in the tech design:

- `HarnessPlan` -- top-level plan struct
- `ValidationRule` with `ValidationRuleId`, `ValidationEvent`, `ValidationPhase`, `ValidationKind`
- `ValidationKind` enum (19 variants covering all spec operations)
- `StructuredShape` enum (`Scalar`, `Array`, `Object`)
- `ApprovedRuntimeCommand` struct
- `HandlerTable` with `HandlerRule`
- `HandlerAction` enum (`Retry`, `Resume`, `Redirect`, `Deviate`)
- `FailureEvent` enum (`AgentFailure`, `Timeout`, `Validation(ValidationEvent)`)
- `PreRunSnapshot`, `FileFingerprint`
- `AttemptOutcome`, `ProcessTermination`
- `ValidationFailure` -- captures a single failed check with rule, message, and context
- `HarnessResolutionContext` -- path resolver input

`ValidationPhase` should enforce the pre/post distinction:

```rust
pub enum ValidationPhase {
    PreOnly,
    PostOnly,
    Both,
}
```

Map each `ValidationKind` variant to its allowed phase so the parser can reject post-only validations in `pre_checks`.

### 1.4 Implement timeout parser

In `timeout.rs`:

- `parse_timeout(input: &str) -> Result<Duration, HarnessError>`
- Accept `{number}{unit}` with optional whitespace between number and unit.
- Units: `s`, `sec`, `second`, `seconds`, `m`, `min`, `minute`, `minutes`, `h`, `hr`, `hour`, `hours`.
- Reject zero, negative, and non-numeric values.
- Reject unknown units with an actionable error naming the valid options.

**Unit tests (in `timeout.rs` or `claudine/lib/src/harness/tests/`):**

| Input | Expected |
|-------|----------|
| `"30s"` | `Duration::from_secs(30)` |
| `"30 seconds"` | `Duration::from_secs(30)` |
| `"5m"` | `Duration::from_secs(300)` |
| `"5 min"` | `Duration::from_secs(300)` |
| `"2h"` | `Duration::from_secs(7200)` |
| `"2 hours"` | `Duration::from_secs(7200)` |
| `"0s"` | Error |
| `"abc"` | Error |
| `"5x"` | Error (unknown unit) |
| `"5"` | Error (missing unit) |

### 1.5 Implement path resolver

In `resolve.rs`:

- `resolve_harness_path(raw: &str, ctx: &HarnessResolutionContext) -> Result<PathBuf, HarnessError>`
- Rules:
  1. Absolute path: return as-is.
  2. `@foo/bar`: strip `@`, join with `ctx.repo_root`. Error if `repo_root` is `None`.
  3. All other relative paths: join with `ctx.source_path.parent()`.

**Unit tests:**

| Raw | Context | Expected |
|-----|---------|----------|
| `"/abs/path.md"` | any | `/abs/path.md` |
| `"@docs/plan.md"` | repo_root = `/repo` | `/repo/docs/plan.md` |
| `"@docs/plan.md"` | repo_root = None | Error: repo root required |
| `"./local.md"` | source = `/repo/prompts/run.md` | `/repo/prompts/local.md` |
| `"sub/file.md"` | source = `/repo/prompts/run.md` | `/repo/prompts/sub/file.md` |

### 1.6 Implement frontmatter parser

In `parse.rs`:

- `parse_harness_plan(frontmatter: &serde_json::Value, source_path: &Path, ctx: &HarnessResolutionContext) -> Result<HarnessPlan, HarnessError>`
- Accept `pre_checks` and `post_checks` in both forms:
  - **List form** (canonical): `[{ "file_exists": "@foo.md" }, ...]`
  - **Map form** (shorthand): `{ "file_exists": "@foo.md", ... }`
  - Internally normalize both to `Vec<ValidationRule>`.
- Parse each validation key to its `ValidationKind`, resolving file paths through the path resolver.
- Enforce phase constraints: reject `file_changed`, `file_unchanged`, `frontmatter_prop_*`, and `response_*` in `pre_checks`.
- Parse `timeout` through the timeout parser.
- Parse `handle` (programmatic handler) -- store raw command, defer approval to Phase 7.
- Parse `handle_{event}` entries into `HandlerTable`:
  - Detect subject-specific vs generic handlers from the YAML structure.
  - Validate required fields per handler type (`resume` needs `prompt`, `redirect` needs `file`).
- Assign stable `ValidationRuleId` values preserving author declaration order.

**Validation shorthand and expanded form parsing:**

Each validation key should accept:

1. **Scalar shorthand:** `file_exists: "@docs/plan.md"` -> single-argument validation.
2. **Object expanded form:** `json_file_exists: { file: "./data.json", shape: "object", msg: "..." }` -> multi-argument validation.
3. **Dictionary form (frontmatter_prop_equals):** key/value pairs become the expected map.

**Parse-time failures that must error before provider launch:**

1. Unknown validation name
2. Post-only validation in `pre_checks`
3. Missing required handler field (`prompt` for `resume`, `file` for `redirect`)
4. Invalid timeout string
5. Invalid `shape` value (not `scalar`, `array`, `object`)
6. Structurally malformed `pre_checks` / `post_checks` (not list or map)

**Unit tests:**

- Parse list-form `pre_checks` with multiple validations.
- Parse map-form `pre_checks` shorthand.
- Parse mixed shorthand and expanded forms.
- Parse `frontmatter_prop_equals` with multiple pairs.
- Reject unknown validation name.
- Reject `file_changed` in `pre_checks` (post-only).
- Reject `response_includes` in `pre_checks` (post-only).
- Parse valid timeout string from frontmatter.
- Reject invalid timeout string with error naming the source file.
- Parse generic `handle_timeout` handler.
- Parse subject-specific `handle_file_exists` handler.
- Reject `resume` handler missing `prompt`.
- Reject `redirect` handler missing `file`.
- `has_harness_properties()` returns `false` for plain frontmatter.

### 1.7 Add `has_harness_properties` helper

A small utility that checks whether composed frontmatter contains any harness-relevant keys (`pre_checks`, `post_checks`, `timeout`, `handle`, or any `handle_*`). The wrapper uses this to decide whether to enter the harness loop or continue with one-shot behavior.

---

## Phase 2: Validation Engine (No Handlers)

**Goal:** Implement the validation execution engine capable of running pre-checks and post-checks, capturing snapshots, and rendering success/failure messages. No handler loop yet -- failures are terminal.

### 2.1 Create validation executor

**Files to create:**

- `claudine/lib/src/harness/validate.rs`

**Responsibilities:**

- `evaluate_pre_checks(plan: &HarnessPlan) -> Result<(), HarnessError>`
  - Run each `pre_checks` rule in declaration order.
  - Collect all failures (do not short-circuit on first failure).
  - Render one success/failure line per check.
  - Return `HarnessError::PreCheckFailed` with all `ValidationFailure` entries if any fail.

- `capture_pre_run_snapshot(plan: &HarnessPlan) -> Result<PreRunSnapshot, HarnessError>`
  - Only capture state for subjects referenced by `post_checks`.
  - For `file_changed` / `file_unchanged`: capture BLAKE3 hash of file bytes, `exists`, and `is_dir`.
  - For `frontmatter_prop_*`: parse source document frontmatter and capture referenced property values.
  - Skip capture for response-based validations (no pre-state needed).

- `evaluate_post_checks(plan: &HarnessPlan, snapshot: &PreRunSnapshot, outcome: &AttemptOutcome) -> Result<(), HarnessError>`
  - Run each `post_checks` rule using pre-state from snapshot and post-state from disk/outcome.
  - Same collect-all-failures pattern.

### 2.2 Implement individual validation checks

Each `ValidationKind` variant needs a check function. Group by dependency:

**Filesystem checks (pre or post):**

- `file_exists` -- `Path::exists()` and `!is_dir()`
- `dir_exists` -- `Path::exists()` and `is_dir()`
- `json_file_exists` -- exists + `serde_json::from_str` + optional root shape check
- `yaml_file_exists` -- exists + `serde_yaml_ng::from_str` + optional root shape check
- `toml_file_exists` -- exists + `toml::from_str`
- `has_write_permission` -- OS writability check (`OpenOptions::new().write(true).open()`) + provider policy probe (Phase 6)

**Git checks (pre or post):**

- `no_dirty_source_code` -- detect repo root, run scoped `git status --porcelain`, filter by source extensions
- `has_dirty_source_code` -- inverse of above

**Post-only: file comparison:**

- `file_changed` -- compare pre/post BLAKE3 hashes; fail if unchanged
- `file_unchanged` -- compare pre/post BLAKE3 hashes; fail if changed

**Post-only: frontmatter comparison:**

- `frontmatter_prop_changed` -- compare pre/post property values; fail if unchanged
- `frontmatter_prop_unchanged` -- compare pre/post property values; fail if changed
- `frontmatter_prop_equals` -- compare post property values against expected map

**Post-only: response checks:**

- `response_length_at_least` -- `outcome.final_response.len() >= length`
- `response_length_at_most` -- `outcome.final_response.len() <= length`
- `response_includes` -- `outcome.final_response.contains(needle)`
- `response_missing` -- `!outcome.final_response.contains(needle)`

**Shell check (pre or post, deferred to Phase 7):**

- `shell_command` -- stub that returns `HarnessError::ShellCommandDenied` with a message explaining shell validations are not yet wired.

### 2.3 Implement dirty-source-code detection

In `validate.rs` or a helper function:

1. Detect repo root from the wrapper environment or walk up from the validation root.
2. Run `git status --porcelain -- <root>` (scoped to the specified path).
3. Filter results by known source extensions:
   - Rust: `.rs`
   - JS/TS: `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`
   - Python: `.py`
   - Go: `.go`
   - JVM: `.java`, `.kt`
   - Web: `.css`, `.scss`, `.html`
   - Shell: `.sh`, `.bash`, `.zsh`
   - Config: `justfile`, `Cargo.toml`, `package.json`
4. Return the list of dirty source files (or empty).

### 2.4 Implement message renderer

A small harness-specific renderer (not the hook-event template engine):

- Replace `{{status}}`, `{{file}}`, `{{dir}}`, `{{prop}}`, `{{expected}}`, `{{actual}}`, `{{length}}`, `{{response_length}}`, `{{command}}` in user-provided `msg` templates.
- When `msg` is `None`, use a sensible default per `ValidationKind`.
- Prepend status token:
  - Success: `<b><green-500>✓</green-500></b>`
  - Failure: `<b><red-500>⤫</red-500></b>`
- Pass rendered text through the existing `Prose` struct for terminal formatting.

**Default messages per validation (examples):**

| Validation | Default message |
|------------|----------------|
| `file_exists` | `{{status}} the file {{file}} exists` |
| `dir_exists` | `{{status}} the directory {{dir}} exists` |
| `json_file_exists` | `{{status}} {{file}} is a valid JSON file` |
| `file_changed` | `{{status}} the file {{file}} was modified` |
| `file_unchanged` | `{{status}} the file {{file}} was not modified` |
| `response_length_at_least` | `{{status}} response is at least {{length}} characters (actual: {{response_length}})` |
| `response_includes` | `{{status}} response includes "{{expected}}"` |
| `no_dirty_source_code` | `{{status}} no dirty source code in {{dir}}` |
| `has_dirty_source_code` | `{{status}} dirty source code found in {{dir}}` |

### 2.5 Unit tests

**Validation execution tests:**

- Pre-check `file_exists` passes for existing file (use `tempfile`).
- Pre-check `file_exists` fails for missing file.
- Pre-check `dir_exists` passes for existing directory.
- Pre-check `json_file_exists` passes for valid JSON file with correct shape.
- Pre-check `json_file_exists` fails for invalid JSON.
- Pre-check `yaml_file_exists` passes for valid YAML with `shape: array`.
- Pre-check `toml_file_exists` fails for non-TOML file.
- Post-check `file_changed` passes when BLAKE3 hashes differ.
- Post-check `file_changed` fails when hashes match.
- Post-check `file_unchanged` passes when hashes match.
- Post-check `frontmatter_prop_equals` passes when values match.
- Post-check `frontmatter_prop_changed` fails when value is unchanged.
- Post-check `response_length_at_least` passes at boundary.
- Post-check `response_length_at_most` fails above boundary.
- Post-check `response_includes` passes with substring present.
- Post-check `response_missing` fails with substring present.
- All failures collected (not short-circuited) when multiple checks fail.

**Snapshot tests:**

- `capture_pre_run_snapshot` captures BLAKE3 hash for `file_changed` subject.
- `capture_pre_run_snapshot` captures frontmatter value for `frontmatter_prop_changed`.
- Snapshot only captures subjects referenced by post-checks (not all files).

**Message renderer tests:**

- Template substitution replaces all known placeholders.
- Default message used when `msg` is `None`.
- Status token prepended correctly for success and failure.

---

## Phase 3: Process Result Enrichment

**Goal:** Refactor `claudine/cli/src/commands/wrap/exec.rs` so the wrapper can distinguish timeout from ordinary exit from interrupt.

### 3.1 Add `ProcessTermination` to process results

**Files to modify:**

- `claudine/cli/src/commands/wrap/exec.rs`

The existing `wait_with_timeout()` function already handles timeout detection and signal escalation. The change is to propagate that information in the return type.

Add:

```rust
pub struct ProcessResult<T> {
    pub data: T,
    pub termination: ProcessTermination,
}
```

Where `ProcessTermination` is the enum defined in `model.rs` (`Completed`, `TimedOut`, `Interrupted`, `LaunchFailed`).

Modify:

- `run_child()` -- return `ProcessResult<i32>` instead of bare `i32`
- `run_child_capture()` -- return `ProcessResult<CapturedChildOutput>` instead of bare `CapturedChildOutput`
- `run_child_stream()` -- return `ProcessResult<StreamExecutionSummary>` instead of bare exit code / summary

Detection logic:

- If `wait_with_timeout()` hits the deadline: `TimedOut`
- If the process is killed by SIGINT/SIGTERM during signal escalation: `Interrupted`
- If `Command::spawn()` fails: `LaunchFailed`
- Otherwise: `Completed`

### 3.2 Update call sites

**Files to modify:**

- `claudine/cli/src/commands/wrap/mod.rs` -- all places that call `run_child`, `run_child_capture`, `run_child_stream`

This is a signature change so every call site must destructure `ProcessResult`. For the existing non-harness paths, the behavior is unchanged -- they simply read `.data` and ignore `.termination`.

### 3.3 Tests

- Unit test: simulate timeout (short deadline, long-running command) -> `ProcessTermination::TimedOut`.
- Unit test: normal completion -> `ProcessTermination::Completed`.
- Integration: existing wrapper tests continue to pass (no behavioral change for non-harness flows).

---

## Phase 4: Wire Pre/Post Checks into Wrapper Flows

**Goal:** Connect the harness parse and validation layers into the three wrapper composition pipelines so that documents with `pre_checks` / `post_checks` / `timeout` activate the harness. No handler loop yet -- failures exit immediately.

### 4.1 Add harness entry point

**Files to modify:**

- `claudine/cli/src/commands/wrap/mod.rs`

After prompt composition completes (for `--prompt-file`, `--frontmatter-prompt`, and `--compose`), check `has_harness_properties()` on the composed frontmatter. If present:

1. Call `parse_harness_plan()` to build the `HarnessPlan`.
2. If parse fails, render a plain-English error and exit immediately (before launching provider).
3. Run `evaluate_pre_checks()`. Print each check result. Exit on failure.
4. Call `capture_pre_run_snapshot()`.
5. Launch provider as before, but use `HarnessPlan.timeout` (if set and no CLI `--timeout`) as the effective timeout.
6. Build `AttemptOutcome` from the `ProcessResult` and `StreamExecutionSummary`.
7. Run `evaluate_post_checks()`. Print each check result. Exit on failure.

If no harness properties are present, the existing one-shot behavior is unchanged.

### 4.2 Build `AttemptOutcome` from process results

Add a helper in `claudine/lib/src/harness/runtime.rs`:

```rust
pub fn build_attempt_outcome(
    attempt: u32,
    process_result: &ProcessResult<StreamExecutionSummary>,
) -> AttemptOutcome
```

Map fields from `StreamExecutionSummary` (`session_id`, `assistant_text`, `exit_code`, `stderr_text`) and `ProcessResult.termination` into the `AttemptOutcome` struct.

### 4.3 Generalize output helpers

**Files to modify:**

- `claudine/cli/src/output.rs`

Rename/alias `fm_check_ok` -> `check_ok` and `fm_check_fail` -> `check_fail` (keep the old names as thin wrappers for backwards compatibility). The harness validation renderer should call these generalized helpers.

### 4.4 Handle existing inline composition post-checks

The existing `--frontmatter-prompt` flow already performs body-hash and frontmatter-hash checks (lines ~1247-1346 in `wrap/mod.rs`). When the harness is active, these existing checks should be _skipped_ because the harness's `post_checks` subsume them. Add a guard:

```rust
if harness_plan.is_some() {
    // harness post-checks already ran above
} else {
    // existing inline validation logic
}
```

### 4.5 Integration tests

Use a fake provider script that:

1. Exits successfully, writing a known file -> test `file_changed` post-check passes.
2. Exits successfully but does NOT write the file -> test `file_changed` post-check fails.
3. Exits successfully with controlled stdout -> test `response_includes` / `response_missing`.
4. Pre-check `file_exists` fails before provider ever launches (verify provider was never spawned).
5. Document with no harness properties -> existing behavior unchanged.
6. Invalid frontmatter (`pre_checks` is a string) -> parse error before provider launch.

---

## Phase 5: YAML Handlers (`retry`, `resume`, `redirect`)

**Goal:** Add the handler resolution layer and the orchestration loop that can retry, resume, or redirect on failure.

### 5.1 Create handler resolver

**Files to create:**

- `claudine/lib/src/harness/handlers.rs`

Implement:

- `resolve_handler(failure: &FailureContext, table: &HandlerTable, programmatic: Option<&ApprovedRuntimeCommand>) -> Result<Option<HandlerAction>, HarnessError>`
  - Resolution order per tech design:
    1. Subject-specific YAML handler for the failing event
    2. Generic YAML handler for the failing event
    3. Programmatic `handle` (deferred to Phase 7, returns `None` here)
    4. `None` (unhandled)
  - Map `AttemptOutcome` and validation failures to `FailureEvent`.

- `FailureContext` struct:
  ```rust
  pub struct FailureContext {
      pub event: FailureEvent,
      pub subject_key: Option<String>,
      pub message: String,
      pub attempt: u32,
      pub session_id: Option<String>,
      pub outcome: Option<AttemptOutcome>,
  }
  ```

### 5.2 Add orchestration loop

**Files to modify:**

- `claudine/cli/src/commands/wrap/mod.rs`

Replace the single-shot harness path from Phase 4 with:

```rust
// Pseudo-code for the harness loop
let mut state = HarnessLoopState::new(plan, ...);

loop {
    evaluate_pre_checks(&state)?;
    let snapshot = capture_pre_run_snapshot(&state)?;
    let outcome = launch_attempt(&state)?;

    match evaluate_post_checks(&snapshot, &outcome) {
        Ok(()) => return success,
        Err(failures) => {
            let failure_ctx = build_failure_context(&failures, &outcome, &state);
            match resolve_handler(&failure_ctx, &state.plan.handlers, ...)? {
                Some(action) => apply_handler(&mut state, action)?,
                None => return failure,
            }
        }
    }
}
```

**`HarnessLoopState`** carries across attempts:

- Current document source path
- Current effective frontmatter overrides (`set` overlay)
- Current prompt body
- Attempt counter per handler action type
- Most recent `session_id`
- Last `AttemptOutcome`

### 5.3 Implement `retry` handler

When the resolved handler is `Retry`:

1. Check attempt count against ceiling (default 3 or handler's `retries`).
2. If ceiling reached, return `HarnessError::HandlerFailed`.
3. Render handler `msg` if present (through `Prose`).
4. Fire `say` through the existing TTS path (best-effort, non-blocking).
5. Apply `set` overrides to effective frontmatter.
6. If handler has `prompt_suffix`: append to the composed prompt body.
7. If handler has no `prompt_suffix`: append a default message like: `"The previous attempt failed: {failure.message}. Please correct the issue and try again."`
8. Recompose the prompt with updated frontmatter/overrides.
9. Increment attempt counter and continue loop.

### 5.4 Implement `redirect` handler

When the resolved handler is `Redirect`:

1. Resolve redirect `file` through harness path resolution.
2. Load and parse the redirected Markdown source.
3. Parse its harness plan (the redirected document becomes the new current document).
4. Apply `set` overrides.
5. If `resume: true` and provider supports resume and a `session_id` exists:
   - Use resume launch for next attempt (Phase 6).
6. Otherwise: start fresh session.
7. Continue loop with the redirected document as the current source.

### 5.5 Implement frontmatter override semantics

Handler `set` is an in-memory overlay, not a disk mutation:

- Merge shallowly at top-level frontmatter map.
- `null` removes a property.
- Overrides affect: prompt recomposition, env var derivation, timeout resolution, validation parsing for next attempt.
- The on-disk file is never changed by `set`.

### 5.6 Detect agent failure and timeout in the loop

Map `ProcessTermination` and `StreamExecutionSummary` to `FailureEvent`:

- `TimedOut` -> `FailureEvent::Timeout`
- `is_error == true` or non-zero exit with `Completed` -> `FailureEvent::AgentFailure`
- `Interrupted` -> immediate exit (user canceled), no handler resolution

### 5.7 Tests

**Handler resolver tests:**

- Subject-specific handler matched by event + subject_key.
- Generic handler matched when no subject-specific handler exists.
- No handler -> returns `None`.
- Subject-specific takes precedence over generic for same event.

**Retry tests (integration with fake provider):**

- Provider fails, `handle_agent_failure: retry`, second attempt succeeds -> overall success.
- Provider fails 4 times with retry ceiling 3 -> final failure after 3 retries.
- Retry with `set` overrides -> env vars reflect overrides on retry.
- Retry with `prompt` suffix -> composed prompt includes suffix.
- Retry without `prompt` -> default failure message appended.

**Redirect tests:**

- Post-check failure, `handle_file_changed: redirect: { file: "fallback.md" }` -> fallback document loaded and executed.
- Redirect with `resume: true` -> resume launch used.
- Redirect target has its own `pre_checks` -> those are evaluated.

**Frontmatter override tests:**

- `set: { timeout: "10m" }` changes effective timeout on retry.
- `set: { some_prop: null }` removes property from effective frontmatter.
- Overrides do not mutate on-disk file.

---

## Phase 6: Resume Support via Provider Profiles

**Goal:** Enable the `resume` handler by wiring provider-specific resume launch args through the existing agent/profile layer.

### 6.1 Add `ResumeLaunchSpec` to agent capabilities

**Files to modify:**

- `claudine/lib/src/agents/model.rs`

Add:

```rust
pub struct ResumeLaunchSpec {
    pub supported: bool,
    pub description: Vec<&'static str>,
}
```

This can live alongside or be derived from the existing `NonInteractiveCapabilities.resume_supported` field. The key addition is making the resume arg-building available to the harness.

### 6.2 Add resume arg builders to provider profiles

**Files to modify:**

- `claudine/cli/src/commands/wrap/profile.rs` (or each provider adapter)

Add a method/function per provider that builds resume args from a `session_id`:

| Provider | Resume args |
|----------|-------------|
| Claude | `["claude", "-r", "{session_id}", "--print"]` |
| Codex | `["codex", "exec", "resume", "{session_id}"]` |
| Kimi Code | `["kimi", "--resume", "{session_id}", "--print"]` |
| Qwen CLI | `["qwen", "--resume", "{session_id}"]` |

For providers without resume support (Gemini, Goose, OpenCode, Roo): return `HarnessError::ResumeUnsupported`.

The harness should call into the provider profile to get resume args rather than hardcoding them.

### 6.3 Implement `resume` handler

When the resolved handler is `Resume`:

1. Check `provider.resume_supported`. Error if not supported with an actionable message naming the provider.
2. Check `session_id` exists from last attempt. Error if missing.
3. Check attempt count against ceiling.
4. Render handler `msg` and fire `say`.
5. Apply `set` overrides.
6. Build resume args from provider profile.
7. Append the handler's `prompt` as a follow-up message.
8. Launch the resume process.
9. Build new `AttemptOutcome` and continue loop.

### 6.4 Tests

**Resume tests:**

- `resume` with Claude provider + valid session_id -> resume args built correctly.
- `resume` with Gemini (unsupported) -> `HarnessError::ResumeUnsupported` with provider name.
- `resume` without `session_id` -> `HarnessError::ResumeNoSession`.
- `resume` with `prompt` -> prompt appended to resume.
- Integration: timeout with `handle_timeout: resume` -> resumes from where it stopped.

---

## Phase 7: Runtime Commands (`shell_command`, `deviate`, `handle`)

**Goal:** Wire runtime shell commands into the harness using Darkmatter's shell policy system. This phase completes the feature.

### 7.1 Create shell policy adapter

**Files to create:**

- `claudine/lib/src/harness/shell.rs`

A thin adapter that reuses Darkmatter's shell expansion infrastructure:

- `validate_and_approve_command(raw: &str, options: &ShellApprovalOptions) -> Result<ApprovedRuntimeCommand, HarnessError>`
  - Tokenize using `darkmatter::markdown::compose::shell_expansion::tokenize()`
  - Check against Darkmatter's built-in blacklist
  - Check against user blacklist
  - Check against whitelist
  - If not whitelisted, invoke approval callback
  - Return `ApprovedRuntimeCommand` with resolved executable and args

- `ShellApprovalOptions`:
  ```rust
  pub struct ShellApprovalOptions {
      pub policy_root: Option<PathBuf>,
      pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
  }
  ```

Policy file locations: use the same `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist` files. Do not create a separate approval surface.

### 7.2 Wire `shell_command` validation

Replace the stub from Phase 2 with a real implementation:

1. Tokenize and approve the command through the shell policy adapter.
2. Execute the command with timeout protection.
3. Check exit code: zero = pass, non-zero = fail.
4. Optionally capture and display stdout/stderr based on `show_stdout` / `show_stderr` flags.

### 7.3 Implement `deviate` handler

When the resolved handler is `Deviate`:

1. The command was already approved at parse time (Phase 1 stores it as `ApprovedRuntimeCommand`).
2. Set attempt context environment variables:
   - `CLAUDINE_ATTEMPT` -- current attempt number
   - `CLAUDINE_SESSION_ID` -- last session ID
   - `CLAUDINE_FAILURE_EVENT` -- the event name
   - `CLAUDINE_SOURCE_FILE` -- absolute path to source document
3. Execute the command directly (not through a shell).
4. After successful completion, run post-checks for the current document.
5. If post-checks still fail, continue normal handler resolution (the deviate result feeds back into the loop).

### 7.4 Implement programmatic `handle`

When the harness falls through to the programmatic handler:

1. The command was approved at parse time.
2. Build the JSON payload per the tech design (provider, source_file, attempt, session_id, termination, failure_event, failure_phase, message, check details, response text).
3. Write JSON to the command's stdin.
4. Set the same attempt context environment variables.
5. Capture stdout.
6. Parse stdout:
   - Empty / `null` / `false` -> unhandled, return `None`.
   - `true` -> `HandlerAction::Retry` with defaults.
   - JSON object -> deserialize to `HandlerAction` (validate all fields).
7. **Reject `deviate`** in the returned action. The tech design explicitly forbids programmatic handlers from returning `deviate` because the command is only known at runtime and cannot be pre-screened.
8. Return the resolved `HandlerAction` for the loop to apply.

### 7.5 Approve all detectable commands at parse time

During `parse_harness_plan()`, any command that can be detected statically should be validated immediately:

- `shell_command` validations
- `deviate` handler commands
- `handle` programmatic handler command

If any fail approval, emit an error before provider launch.

If any are not whitelisted, invoke the approval callback immediately (the user is prompted once at parse time, not mid-execution).

### 7.6 Tests

**Shell policy adapter tests:**

- Whitelisted command -> `ApprovedRuntimeCommand` returned.
- Blacklisted command -> `HarnessError::ShellCommandBlacklisted`.
- Unknown command -> approval callback invoked.
- Tokenization failure (shell metacharacters) -> error.

**shell_command validation tests:**

- Command exits 0 -> validation passes.
- Command exits 1 -> validation fails.
- Stdout captured when `show_stdout: true`.
- Stderr suppressed when `show_stderr: false`.

**Deviate handler tests:**

- Deviate command succeeds, post-checks pass -> overall success.
- Deviate command succeeds, post-checks fail -> continues handler resolution.
- Attempt context env vars set correctly.

**Programmatic handler tests:**

- Handler returns `true` -> `retry` with defaults.
- Handler returns `false` -> unhandled.
- Handler returns `null` -> unhandled.
- Handler returns valid JSON object -> parsed `HandlerAction`.
- Handler returns `deviate` -> rejected with error.
- JSON payload on stdin includes all expected fields.

---

## Phase 8: Output Generalization and Documentation

**Goal:** Clean up output, update documentation, and ensure the feature is fully integrated.

### 8.1 Output section ordering

The harness output should follow the existing section ordering conventions:

1. Pre-check results print before the execution header.
2. Execution header prints as normal.
3. Agent output streams as normal.
4. Post-check results print after agent output.
5. Handler messages print between attempts.

### 8.2 Documentation updates

**Files to update:**

- `claudine/cli/README.md` -- document `pre_checks`, `post_checks`, `timeout`, and handler frontmatter properties.
- `claudine/docs/topics/composition.md` -- add a "Validations and Handlers" section explaining how harness features compose with inline and chained workflows.
- `claudine/lib/README.md` -- document the `harness` module.
- `.claude/skills/claudine/SKILL.md` -- update with harness capability information.

### 8.3 Shell policy documentation

If policy files are shared with Darkmatter (recommended), document this explicitly so users don't approve the same command in two places.

---

## Cross-Cutting Concerns

### Error Messages

All errors must be plain English with enough context to fix immediately:

- **Parse errors:** Include the source file path and the exact frontmatter property name.
  - Example: `ERROR: /repo/prompts/run.md has an invalid pre_checks frontmatter property: "flie_exists" is not a recognized validation. Did you mean "file_exists"?`
- **Validation failures:** Include the check name, expected state, and actual state.
- **Handler failures:** Include the handler type, the event being handled, and what went wrong.
- **Resume failures:** Name the provider and explain it doesn't support resume.

### Dependency Additions

The harness module will need:

- `blake3` -- file content hashing for `file_changed` / `file_unchanged` (already available in `biscuit-hash`)
- `indexmap` -- ordered maps for `frontmatter_prop_equals` and handler `set` (likely already a transitive dependency)
- `serde_yaml_ng` -- YAML validation in `yaml_file_exists`
- `toml` -- TOML validation in `toml_file_exists`

Check which of these are already workspace dependencies before adding new ones.

### Testing Infrastructure

- **Fake provider binary:** Create a small shell script or Rust binary in `claudine/cli/tests/fixtures/` that simulates provider behavior (controlled exit codes, stdout content, file mutations) based on environment variables.
- **`tempfile` crate:** For filesystem validation tests.
- **`serial_test`:** For tests that manipulate environment variables or shared state.

---

## Delivery Summary

| Phase | Scope | Key Deliverable | Depends On |
|-------|-------|-----------------|------------|
| 1 | Types, parsing, timeout | `HarnessPlan` from frontmatter | -- |
| 2 | Validation engine | Pre/post check execution | Phase 1 |
| 3 | Process result enrichment | `ProcessTermination` in exec | -- |
| 4 | Wrapper integration | Harness-aware composition flows | Phases 1, 2, 3 |
| 5 | YAML handlers | Retry, redirect, orchestration loop | Phase 4 |
| 6 | Resume support | Provider resume arg builders | Phase 5 |
| 7 | Runtime commands | shell_command, deviate, handle | Phases 5, 6 |
| 8 | Output and docs | Documentation, output polish | Phase 7 |

**Parallelization:** Phases 1 and 3 have no dependency on each other and can be implemented concurrently. Phase 2 depends only on Phase 1. Phase 4 is the first integration point requiring all three prior phases.
