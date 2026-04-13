# Tech Design: Better Interview for Install

Feature spec: `sniff/features/2026-04-12-better-interview-for-install/spec.md`

## Overview

This feature replaces the current split install UX with a shared library-owned interview flow.

The `sniff` library will own:

- install attempt sequencing
- preflight copy generation
- command execution with stdout/stderr capture
- retry/fallback selection state
- remote-script consent gating

The `sniff` CLI will own:

- constructing the actual `Terminal`
- mapping semantic interview events to `Prose`, `BlockQuote`, and `Status`
- interactive prompts via `inquire`
- `--plain` output stripping at emit time

This intentionally avoids a direct `biscuit-terminal` dependency from the `sniff` library. The library will return semantic events plus the already-authored strings that belong inside `Prose` and `Status`, while callers decide which concrete `Renderable` component to wrap around those strings.

## Goals

1. The install UX for both `sniff <category> install <name>` and the category picker flow uses the same interview logic.
2. Before execution, the user sees a rich explanation of what will be installed, from where, and with which command.
3. After execution, successful installs show captured stdout and failed installs show captured stderr.
4. If the chosen method fails and other runnable methods exist, the user can retry with a fallback method.
5. The reusable logic lives in `sniff/lib` as far as possible without introducing a circular dependency on `biscuit-terminal`.
6. CLI rendering uses a single real `Terminal` instance so wrapping, colors, and OSC8 links reflect the actual terminal.

## Non-Goals

- Changing install-plan selection rules or bucket priority. This feature sits on top of the existing `InstallPlan`.
- Replacing `inquire` with a new prompt framework.
- Adding `biscuit-terminal` as a dependency of `sniff/lib`.
- Redesigning `install-plan --json`.
- Making blocked methods retryable. Retry prompts only surface actionable alternatives.

## Current State

### Existing split

- `sniff/cli/src/install_plan_cmd.rs` handles the named install flow.
- `sniff/cli/src/install.rs` handles the multi-select category picker flow.
- `sniff/lib/src/programs/install_plan.rs` chooses one method, but does not drive an interview.
- `sniff/lib/src/programs/installer.rs` executes commands and captures output only on success.

### Current problems

1. The multi-select flow calls `detector.install()` directly, so it:
   - does not show the command before execution
   - does not show captured stdout/stderr in a structured way
   - does not offer fallback methods
2. The named install flow renders only a preflight summary and then executes.
3. `execute_install()` returns `Err(SniffInstallationError::PackageManagerFailed { .. })` on non-zero exit, which means the interview layer loses structured stdout/stderr for failed commands.
4. Rendering is currently built around locally-constructed terminal values instead of threading the actual terminal instance through the flow, which is the bug called out in the spec.

## Design Decisions

### 1. Library emits semantic interview events, not renderables

The library must not construct `Prose`, `BlockQuote`, or `Status` directly. Instead it emits semantic events with the strings to be rendered.

Example event categories:

- preflight explanation text for `Prose`
- status text for `Status`
- raw command output text for `BlockQuote`
- retry choice copy for `Prose`

This keeps the interview reusable by non-CLI callers and avoids a circular dependency.

### 2. The library owns copy, the CLI owns component choice

The wording required by the spec should live in the library, not be duplicated across callers. The CLI is only a presentation adapter:

- `Announcement` event -> `Prose`
- `CapturedOutput(stdout)` -> neutral `BlockQuote`
- `CapturedOutput(stderr)` -> error `BlockQuote`
- `Status(success)` / `Status(error)` -> `Status` with circular style

### 3. Retry prompts only show runnable alternatives

This is an implementation inference from the spec. Prompting the user to “try installing using apt instead” when `apt` is unavailable on the host is useless. Retry choices will therefore be limited to methods that were already runnable but were not initially chosen because of priority.

Operationally, that means alternatives come from `InstallPlanOption`s whose `reason_type` is `LowerPriorityAlternative`.

### 4. Failure output must remain structured

The current execution API throws away structured failure output by returning an error before the caller receives an `InstallResult`. This feature introduces a captured execution path that preserves stdout, stderr, exit code, and command string for both success and failure.

### 5. One `Terminal` per CLI command

The CLI will construct one `Terminal::new()` for the install command and pass `&Terminal` through all render helpers. No install renderer should create its own `Terminal::default()` or `Terminal::new()` internally.

## Architecture

```txt
sniff/lib/src/programs/
├── install_interview.rs      ← NEW: shared interview engine + event types
├── installer.rs              ← add captured execution API + copy helpers
├── install_plan.rs           ← add helpers to enumerate retryable alternatives
├── mod.rs                    ← export new interview types
└── types.rs                  ← existing ProgramDetector API delegates to shared path where useful

sniff/cli/src/
├── install.rs                ← picker flow now resolves programs and calls shared interview runner
├── install_plan_cmd.rs       ← named install flow calls same runner
├── install_ui.rs             ← NEW: CLI renderer/prompt adapter over interview events
└── output/mod.rs             ← unchanged emit_text/emit_stderr behavior
```

## Part 1: Captured Execution in the Library

### Problem

`execute_install()` and `execute_versioned_install()` only return `InstallResult` on success. On failure they return `SniffInstallationError`, usually with only stderr text folded into `msg`.

That is not sufficient for the new UX because the interview needs:

- command string
- exit code
- stdout
- stderr
- a reliable distinction between “command ran and failed” vs “command could not be started”

### New types

Add a non-rendering execution type in `installer.rs`:

```rust
pub struct InstallCapturedResult {
    pub command: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub enum InstallCapturedOutcome {
    Completed(InstallCapturedResult),
    SetupError(SniffInstallationError),
}
```

`Completed` covers:

- dry run
- successful execution
- non-zero exit after the command actually ran

`SetupError` is reserved for failures before meaningful command output exists, such as:

- invalid package name
- invalid remote-bash URL
- unsupported versioned install shape

### New execution helpers

Add:

```rust
pub fn execute_install_captured(
    method: &InstallationMethod,
    opts: &InstallOptions,
) -> InstallCapturedOutcome

pub fn execute_versioned_install_captured(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
) -> InstallCapturedOutcome
```

Behavior:

- build and validate command exactly as today
- on dry run: return `Completed` with `executed = false` and `success = true`
- on spawn success + exit code `0`: return `Completed(success = true)`
- on spawn success + non-zero exit: return `Completed(success = false)`
- on spawn failure: return `Completed(success = false)` with the spawn error string placed into `stderr`

This last rule is important. It keeps the interview output model uniform and lets the failure branch still render a red `BlockQuote` even when the program failed to start.

### Backward compatibility

Keep the current APIs:

- `execute_install`
- `execute_versioned_install`

They become thin wrappers over the captured API:

- `success = true` -> return existing `InstallResult`
- `success = false` -> map back to `SniffInstallationError::PackageManagerFailed`

This preserves existing callers while enabling the richer interview path.

## Part 2: Shared Interview Engine

### New file: `sniff/lib/src/programs/install_interview.rs`

This module owns the control flow for one install session.

### Core input

The named install flow already mutates `InstallPlan` via `--via`, `--no-sudo`, and `--force`, so the interview runner should accept an already-built plan rather than rebuild one internally.

```rust
pub struct InstallInterviewInput {
    pub program: String,
    pub website: &'static str,
    pub plan: InstallPlan,
}
```

Optional convenience helpers can build this from `ProgramMetadata`, but the core runner should consume the prepared input.

### Event model

The library emits semantic events with caller-renderable strings:

```rust
pub enum InstallInterviewEvent {
    Announcement {
        prose: String,
    },
    ConsentWarning {
        prose: String,
    },
    CapturedOutput {
        stream: InstallOutputStream,
        body: String,
    },
    Status {
        kind: InstallStatusKind,
        text: String,
    },
}

pub enum InstallOutputStream {
    Stdout,
    Stderr,
}

pub enum InstallStatusKind {
    Success,
    Error,
}
```

Notes:

- `Announcement.prose` and `Status.text` are strings suitable for `Prose::new(...)` or as the text payload inside `Status`.
- `CapturedOutput.body` is raw text, not prose markup. The CLI should pass it straight into `BlockQuote`.

### Prompt/decision interface

The library cannot prompt directly, so it asks a caller-supplied delegate for decisions:

```rust
pub trait InstallInterviewDelegate {
    fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError>;

    fn confirm_remote_script(
        &mut self,
        prose: &str,
    ) -> Result<bool, SniffInstallationError>;

    fn choose_retry(
        &mut self,
        prompt: &RetryPrompt,
    ) -> Result<RetryChoice, SniffInstallationError>;
}
```

Retry types:

```rust
pub struct RetryPrompt {
    pub heading_prose: String,
    pub choices: Vec<RetryPromptChoice>,
}

pub struct RetryPromptChoice {
    pub label: String,
    pub prose: String,
    pub method: InstallationMethod,
}

pub enum RetryChoice {
    RetryWith(InstallationMethod),
    Quit,
}
```

`label` exists because `inquire::Select` needs a plain string. `prose` exists because the caller may also want to render the richer explanatory line before prompting.

### Interview options

```rust
pub struct InstallInterviewOptions {
    pub install: InstallOptions,
    pub prompt_on_failure: bool,
}
```

Defaults:

- `prompt_on_failure = true` for CLI interactive flows
- silent callers can set it to `false`

### Runner API

```rust
pub fn run_install_interview<D: InstallInterviewDelegate>(
    input: &InstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
) -> Result<InstallInterviewOutcome, SniffInstallationError>
```

### Outcome

```rust
pub enum InstallInterviewOutcome {
    Installed { method: InstallationMethod },
    DryRun { method: InstallationMethod },
    AbortedByUser,
    Failed { attempted: Vec<InstallationMethod> },
    NotInstallable,
}
```

## Part 3: Copy Generation

### New helpers in `installer.rs`

The library should centralize all end-user strings required by the spec.

Suggested helpers:

```rust
pub fn build_install_announcement(
    program: &str,
    website: &str,
    method: &InstallationMethod,
    command: &str,
) -> String

pub fn build_install_success_status(
    program: &str,
    website: &str,
) -> String

pub fn build_install_failure_status(
    program: &str,
    website: &str,
) -> String

pub fn build_retry_choice_prose(
    method: &InstallationMethod,
) -> String
```

### Announcement templates

Package manager methods:

```text
The <b><blue><a href="{website}">{program}</a></blue></b> will be installed through the **{manager}** package manager using the command: <dim><green>{command}</green></dim>
```

`RemoteBash`:

```text
The <b><blue><a href="{website}">{program}</a></blue></b> will be installed using the remote installer script at <a href="{url}">{url}</a> using the command: <dim><green>{command}</green></dim>
```

`UvWithInstall`:

```text
The <b><blue><a href="{website}">{program}</a></blue></b> will be installed by bootstrapping <b>uv</b> from <a href="{astral_url}">{astral_url}</a> if needed, then running: <dim><green>{command}</green></dim>
```

### Status templates

Success:

```text
<b><blue><a href="{website}">{program}</a></blue></b> has been installed successfully
```

Error:

```text
failed to install <b><blue><a href="{website}">{program}</a></blue></b>.
```

### Retry templates

Alternative:

```text
Try installing using **{alternative}** instead
```

Quit:

```text
Quit (_and try manually if desired_)
```

## Part 4: Interview Control Flow

### Initial preflight

1. Validate `input.plan`.
2. If `plan.successful == false`:
   - emit the existing “no viable method” prose/fallback copy
   - return `InstallInterviewOutcome::NotInstallable`
3. Determine the chosen method.
4. Build the command string via `get_install_command()`.
5. Emit `Announcement`.

### Remote-script consent

If the chosen method is:

- `RemoteBash(_)`
- `UvWithInstall(_)`

then the runner emits `ConsentWarning` prose and calls `delegate.confirm_remote_script(...)` before execution unless:

- `options.install.dry_run == true`, or
- `options.install.approve_remote_bash == true`

If consent is denied, return `AbortedByUser`.

### Execution

The runner calls `execute_install_captured(...)`.

#### Success branch

1. Prefer `stdout` for the quoted output block.
2. If `stdout` is empty, do not render an empty quote block.
3. Emit `Status { kind: Success, ... }`.
4. Return `Installed` or `DryRun`.

#### Failure branch

1. Prefer `stderr` for the quoted output block.
2. If `stderr` is empty, fall back to `stdout`.
3. If both are empty, skip the quote block.
4. Emit `Status { kind: Error, ... }`.
5. Gather retryable alternatives.
6. If none exist, return `Failed`.
7. If alternatives exist and `prompt_on_failure == true`, ask the delegate whether to retry or quit.
8. If the user selects a fallback method, repeat the loop with that method marked as attempted.

### Retry ordering

Retry choices preserve the existing plan order. The runner does not recompute priority. It simply offers the remaining methods that were already deemed runnable.

This keeps fallback behavior aligned with `InstallPlan` and avoids introducing a second policy engine.

## Part 5: `InstallPlan` Helpers

### Add helper methods in `install_plan.rs`

No JSON shape change is required.

Add:

```rust
impl InstallPlan {
    pub fn retryable_alternatives(
        &self,
        attempted: &[InstallationMethod],
    ) -> Vec<&InstallPlanOption>;
}
```

Selection rule:

- `reason_type == InstallPlanReason::LowerPriorityAlternative`
- `kind` not in `attempted`

This avoids adding a new serialized `runnable` field to `InstallPlanOption`.

## Part 6: CLI Adapter

### New file: `sniff/cli/src/install_ui.rs`

This module implements the presentation adapter for the shared interview engine.

```rust
pub struct CliInstallUi<'a> {
    pub terminal: &'a Terminal,
    pub plain: bool,
}
```

Responsibilities:

- render `Announcement` and retry prose with `Prose`
- render `CapturedOutput` with `BlockQuote`
- render `Status` with circular style
- emit one blank line after a quote block
- prompt for consent and retry choices via `inquire`

### Rendering rules

| Event | Component |
|---|---|
| `Announcement` | `Prose` |
| `ConsentWarning` | `Prose` |
| `CapturedOutput(Stdout)` | `BlockQuote` with neutral/grey bar |
| `CapturedOutput(Stderr)` | `BlockQuote` with error/red bar |
| `Status(Success)` | `Status` circular/success |
| `Status(Error)` | `Status` circular/error |

### Terminal handling

The CLI constructs one `Terminal::new()` and passes `&Terminal` into `CliInstallUi`.

That same instance is reused for:

- preflight prose
- output block quotes
- status rendering
- retry prompt preamble

This is the fix for the rendering bug described in the spec.

## Part 7: Entry Point Integration

### Named install flow

Update `sniff/cli/src/install_plan_cmd.rs`:

- keep `build_plan_for_args(...)`
- keep `apply_via(...)`
- replace direct execution logic in `execute_install_flow(...)` with:
  - build `InstallInterviewInput`
  - construct `Terminal::new()`
  - construct `CliInstallUi`
  - call `run_install_interview(...)`

`install-plan` remains a non-executing command, but it may reuse the same copy builders for consistency.

### Category picker flow

Update `sniff/cli/src/install.rs`:

- keep the `MultiSelect` picker itself
- after selection, resolve each chosen item to a concrete program
- build an `InstallPlan` for each selected program
- run the same shared interview runner used by the named install flow

This is the key behavioral change that fixes the screenshot case from the spec.

### `ProgramDetector::install()`

Keep the existing API for compatibility. Internally it may delegate to the captured execution path without emitting interview events.

That keeps library behavior stable for callers that only want fire-and-forget installation.

## Part 8: Error Handling

### Setup errors

If the library cannot even prepare the command:

- emit an error `Status`
- optionally emit the error string as a stderr-style `CapturedOutput`
- return `Failed`

### Interrupted prompts

The CLI adapter should preserve current behavior:

- user cancel -> `AbortedByUser`
- ctrl-c / interrupted prompt -> exit code `130`

### Plain mode

Plain mode remains a CLI concern. The interview engine still emits the rich prose strings; `emit_text(..., plain)` strips terminal escapes after rendering.

## Part 9: Testing

### Library unit tests

Add tests for:

1. package-manager announcement copy
2. remote-bash announcement copy
3. `UvWithInstall` announcement copy
4. captured execution preserves stderr on non-zero exit
5. captured execution preserves spawn error text
6. retryable alternatives exclude blocked methods
7. interview loops to the next runnable alternative after failure
8. denied remote consent returns `AbortedByUser`
9. dry run emits announcement and success status without execution

### CLI tests

Add tests for:

1. event -> component mapping in `CliInstallUi`
2. stdout quote uses neutral styling
3. stderr quote uses error styling
4. one blank line is emitted after quote blocks
5. named install path calls shared interview runner
6. category picker path calls shared interview runner

### Interactive regression tests

Use `expectrl` for one end-to-end prompt flow:

- select a program
- trigger a failed first method
- verify retry prompt appears
- choose fallback
- verify second attempt executes

## Rollout Notes

1. Implement the captured execution API first.
2. Add the interview engine and unit tests in `sniff/lib`.
3. Add the CLI adapter and move the named install flow to it.
4. Move the multi-select flow to the same runner.
5. Snapshot the rendered output for a few representative programs:
   - brew success
   - remote bash consent
   - first method failure with cargo fallback

## Summary

The central design choice is:

- `sniff/lib` owns interview sequencing and the exact copy strings
- `sniff/cli` owns `biscuit-terminal` renderables and interactive prompts

That gives the project a single install interview implementation, fixes the current picker-path UX gap, preserves structured stdout/stderr on failure, and avoids introducing a `sniff` -> `biscuit-terminal` dependency edge.
