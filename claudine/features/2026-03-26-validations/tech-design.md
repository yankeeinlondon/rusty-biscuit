# Validations, Timeouts, and Handlers Tech Design

This document defines the implementation-ready technical design for the validations/timeout/handler feature described in:

- `claudine/features/2026-03-26-validations/spec.md`
- the current wrapper execution flow in `claudine/cli/src/commands/wrap/mod.rs`
- the current composition APIs in `claudine/lib/src/composition/`
- the current structured stream summary in `claudine/lib/src/stream/summary.rs`
- the existing Darkmatter shell approval system in `darkmatter/lib/src/markdown/compose/shell_expansion/`

The design goal is to let Markdown-backed non-interactive prompts behave like a small, typed job harness without breaking the current wrapper architecture.

## Summary

Claudine already knows how to:

1. resolve Markdown prompt sources
2. compose them through Darkmatter
3. run providers in non-interactive mode
4. capture structured summaries including `session_id` and final assistant text
5. perform a narrow amount of post-run validation for `--frontmatter-prompt`

This feature generalizes that into a reusable harness layer with four capabilities:

1. typed pre-run and post-run validations declared in frontmatter
2. per-page timeout configuration
3. typed recovery handlers (`retry`, `resume`, `redirect`, `deviate`)
4. an execution loop that can react to pre-check failures, agent failures, post-check failures, and timeouts

The recommended shape is:

- add a new `claudine::harness` library module for parsing, normalization, validation, timeout parsing, and handler resolution
- keep provider process spawning in `claudine/cli/src/commands/wrap/exec.rs`
- add a wrapper-side orchestration loop that uses the harness plan to drive repeated attempts
- reuse Darkmatter's shell policy and tokenizer for any runtime commands (`shell_command`, `deviate`, and `handle`)

## Goals

1. Support frontmatter-defined `pre_checks`, `post_checks`, `timeout`, `handle`, and `handle_{event}` on Markdown-backed prompt sources.
2. Reuse the existing prompt-file, inline frontmatter-prompt, and chained compose flows instead of inventing a second prompt loader.
3. Keep error reporting plain-English and immediately actionable.
4. Preserve deterministic non-interactive behavior.
5. Reuse existing structured stream summaries for response-based validations and session resume.
6. Reuse Darkmatter's shell approval model instead of building a second approval system.

## Non-Goals

1. This feature does not add harness behavior to raw positional prompt strings in v1.
2. This feature does not replace the existing `claudine handle` hook-dispatch system.
3. This feature does not invent provider resume behavior where the provider does not support resume today.
4. This feature does not turn validation failures into silent best-effort warnings; failures remain job failures unless a handler resolves them.
5. This feature does not add general-purpose shell semantics. Runtime commands are direct process execution with shell-like tokenization only.

## Scope Boundary

The harness is only activated for Markdown-backed prompt workflows:

1. `claudine <provider> --prompt-file <file>`
2. `claudine <provider> --frontmatter-prompt <file>`
3. `claudine <provider> --compose <file>`

The initial implementation should wire these three wrapper flows first because they already have:

1. resolved source-file paths
2. composed prompt text
3. repo/package context
4. structured session summaries
5. provider-specific prompt delivery logic

`claudine compose` should be able to reuse the same library module later, but it should not be the first integration point.

## Spec Clarifications

The spec leaves a few details ambiguous. This design resolves them explicitly.

### 1. `pre_checks` / `post_checks` input shape

The spec text says these properties are "a list of dictionaries", but the example uses a mapping. Supporting only one shape would create needless friction, so v1 should accept both:

```yaml
pre_checks:
  - file_exists: "@foo.md"
  - has_write_permission: "@foo.md"
```

and:

```yaml
pre_checks:
  file_exists: "@foo.md"
  has_write_permission: "@foo.md"
```

Internally both normalize to:

```rust
Vec<ValidationRule>
```

Recommendation for docs:

1. describe list form as canonical because it preserves order and allows repeated checks of the same kind
2. keep mapping form as ergonomic shorthand

### 2. Handler precedence

When multiple handler mechanisms are present, resolution order should be:

1. subject-specific YAML handler for the failing event
2. generic YAML handler for the failing event
3. programmatic `handle`
4. unhandled failure

This keeps statically declared handlers deterministic and parse-time validated while still allowing a programmatic fallback.

### 3. Relative path semantics inside validations

Wrapper CLI prompt-file resolution currently uses special `@` and `./` behavior. That is correct for CLI arguments, but it is the wrong default for paths embedded inside a Markdown document.

For harness validations and redirect targets:

1. absolute paths stay absolute
2. `@foo/bar.md` resolves relative to repo root
3. all other relative paths resolve relative to the source document's directory

This is the most intuitive authoring model for document-local checks.

### 4. Retry ceiling

Default retry ceiling is `3`.

If a handler explicitly sets `retries`, that value becomes the ceiling for that handler action. Claudine does not impose a second hidden lower limit.

### 5. `say`

`msg` is the terminal-rendered message.

`say` is a best-effort spoken mirror routed through the existing TTS/audio path. Failure to speak never changes handler or validation outcomes.

## Current Baseline

Relevant existing behavior:

1. `claudine/cli/src/commands/wrap/prompt_file.rs` already resolves prompt files, composes them with Darkmatter, and derives env vars from residual frontmatter.
2. `claudine/lib/src/composition/prepare.rs` already prepares inline and chained prompts.
3. `claudine/cli/src/commands/wrap/mod.rs` already captures source/body hashes for `--frontmatter-prompt`.
4. `claudine/lib/src/stream/summary.rs` already captures:
   - `session_id`
   - `assistant_text`
   - `stderr_text`
   - `exit_code`
   - `is_error`
   - `error_kind`
   - `error_message`
5. `claudine/lib/src/agents/*.rs` already records whether each provider supports non-interactive resume.
6. `darkmatter/lib/src/markdown/compose/shell_expansion/` already provides:
   - argv tokenization
   - allow/deny policy files
   - approval callbacks
   - timeouts

Current gaps:

1. no typed harness plan
2. no typed validation engine
3. no timeout parsing from frontmatter
4. no way to distinguish provider timeout vs generic non-zero exit in wrapper control flow
5. no handler loop
6. no programmatic handler protocol

## Recommended Module Layout

Add a new library module:

```txt
claudine/lib/src/
├── harness/
│   ├── mod.rs
│   ├── error.rs
│   ├── model.rs
│   ├── parse.rs
│   ├── timeout.rs
│   ├── resolve.rs
│   ├── validate.rs
│   ├── handlers.rs
│   └── runtime.rs
```

Responsibilities:

1. `model.rs`
   - typed frontmatter model
   - normalized validation rules
   - normalized handler actions
   - run/failure/output snapshot structs
2. `parse.rs`
   - parse composed frontmatter into `HarnessPlan`
   - support shorthand and expanded YAML forms
3. `timeout.rs`
   - parse `{#}{unit}` timeout strings
4. `resolve.rs`
   - source-relative path resolution for validation subjects and redirect files
5. `validate.rs`
   - execute pre and post validations
   - capture pre/post state snapshots
   - render success/failure messages
6. `handlers.rs`
   - resolve YAML and programmatic handlers
   - validate handler config
7. `runtime.rs`
   - shared structs for attempts, outcomes, failure context, and resume metadata

Wrapper-side integration:

```txt
claudine/cli/src/commands/wrap/
├── mod.rs        # orchestration loop
├── exec.rs       # richer process result for timeout detection
└── prompt_file.rs
```

## Data Model

### Harness plan

```rust
pub struct HarnessPlan {
    pub source_path: PathBuf,
    pub timeout: Option<std::time::Duration>,
    pub pre_checks: Vec<ValidationRule>,
    pub post_checks: Vec<ValidationRule>,
    pub handlers: HandlerTable,
    pub programmatic_handler: Option<ApprovedRuntimeCommand>,
}
```

### Validation rule

```rust
pub struct ValidationRule {
    pub id: ValidationRuleId,
    pub event: ValidationEvent,
    pub phase: ValidationPhase,
    pub kind: ValidationKind,
    pub message_template: Option<String>,
    pub subject_key: Option<String>,
}
```

Notes:

1. `id` is stable per parsed document and preserves author order.
2. `event` is the handler lookup key.
3. `subject_key` is a normalized discriminator for subject-specific handlers, for example a resolved path or property name.

### Validation kinds

```rust
pub enum ValidationKind {
    FileExists { file: PathBuf },
    DirExists { dir: PathBuf },
    JsonFileExists { file: PathBuf, shape: Option<StructuredShape> },
    YamlFileExists { file: PathBuf, shape: Option<StructuredShape> },
    TomlFileExists { file: PathBuf },
    HasWritePermission { file: PathBuf },
    ShellCommand { command: ApprovedRuntimeCommand, show_stdout: bool, show_stderr: bool },
    NoDirtySourceCode { root: PathBuf },
    HasDirtySourceCode { root: PathBuf },

    FileChanged { file: PathBuf },
    FileUnchanged { file: PathBuf },
    FrontmatterPropChanged { prop: String },
    FrontmatterPropUnchanged { prop: String },
    FrontmatterPropEquals { expected: indexmap::IndexMap<String, serde_json::Value> },
    ResponseLengthAtLeast { length: usize },
    ResponseLengthAtMost { length: usize },
    ResponseIncludes { needle: String },
    ResponseMissing { needle: String },
}
```

`StructuredShape`:

```rust
pub enum StructuredShape {
    Scalar,
    Array,
    Object,
}
```

### Runtime command

```rust
pub struct ApprovedRuntimeCommand {
    pub raw: String,
    pub executable: String,
    pub args: Vec<String>,
}
```

This is the normalized command form produced after tokenization, path resolution, and approval/policy validation.

### Handler table

```rust
pub struct HandlerTable {
    pub exact: Vec<HandlerRule>,
    pub generic: Vec<HandlerRule>,
}

pub struct HandlerRule {
    pub event: FailureEvent,
    pub subject_key: Option<String>,
    pub action: HandlerAction,
}
```

### Handler action

```rust
pub enum HandlerAction {
    Retry {
        prompt_suffix: Option<String>,
        set: Option<indexmap::IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
        retries: Option<u32>,
    },
    Resume {
        prompt: String,
        set: Option<indexmap::IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
        retries: Option<u32>,
    },
    Redirect {
        file: String,
        set: Option<indexmap::IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
        resume: bool,
    },
    Deviate {
        command: ApprovedRuntimeCommand,
        set: Option<indexmap::IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
    },
}
```

### Runtime snapshots

```rust
pub struct PreRunSnapshot {
    pub source_markdown: Option<darkmatter::markdown::Markdown>,
    pub tracked_files: indexmap::IndexMap<PathBuf, FileFingerprint>,
    pub tracked_frontmatter: indexmap::IndexMap<String, serde_json::Value>,
}

pub struct FileFingerprint {
    pub exists: bool,
    pub is_dir: bool,
    pub blake3: Option<String>,
}

pub struct AttemptOutcome {
    pub attempt: u32,
    pub session_id: Option<String>,
    pub final_response: String,
    pub exit_code: i32,
    pub termination: ProcessTermination,
    pub stderr_text: Option<String>,
}

pub enum ProcessTermination {
    Completed,
    TimedOut,
    Interrupted,
    LaunchFailed,
}
```

## Frontmatter Parsing and Normalization

### Accepted validation syntax

Every validation should support:

1. shorthand scalar form
2. expanded object form

Examples:

```yaml
pre_checks:
  - file_exists: "@docs/plan.md"
  - json_file_exists:
      file: "./data/result.json"
      shape: object
      msg: "{{status}} result JSON exists and is an object"
```

`frontmatter_prop_equals` should accept one or more pairs:

```yaml
post_checks:
  - frontmatter_prop_equals:
      status: complete
      approved: true
```

### Accepted handler syntax

Generic event handler:

```yaml
handle_timeout:
  resume:
    prompt: "Continue from where you stopped and finish the document."
    retries: 2
```

Subject-specific handler:

```yaml
handle_file_exists:
  "@docs/output.md":
    retry:
      prompt: "Create the missing file."
      retries: 1
```

Programmatic handler:

```yaml
handle:
  command: ["./scripts/repair-handler", "--json"]
```

### Parse-time validation

Invalid frontmatter should fail before the provider is launched.

That includes:

1. unknown validation names
2. post-only validations placed in `pre_checks`
3. missing required handler fields
4. `resume` without `prompt`
5. `redirect` without `file`
6. `deviate` or `handle` commands that fail policy validation
7. invalid `timeout` strings
8. malformed `shape` values

All parse-time failures should render the exact source file path and the exact frontmatter property name.

## Path Resolution

Validation subjects and redirect targets need source-aware resolution.

Recommended resolver contract:

```rust
pub struct HarnessResolutionContext<'a> {
    pub source_path: &'a Path,
    pub repo_root: Option<&'a Path>,
}
```

Resolution rules:

1. absolute path: use as-is
2. `@foo/bar`: repo-root-relative, error if repo root unknown
3. other relative path: resolve against `source_path.parent()`

This resolver should be used by:

1. file validations
2. directory validations
3. dirty-source-code roots
4. redirect target files

## Timeout Design

### Parsing

Support:

1. `30s`, `30sec`, `30seconds`
2. `5m`, `5min`, `5minutes`
3. `2h`, `2hr`, `2hours`

Whitespace between the number and the unit should also be accepted.

### Precedence

Effective timeout per attempt:

1. wrapper CLI `--timeout` if present
2. current document frontmatter `timeout` after any handler `set` overrides are applied
3. no timeout

### Process result changes

The existing `exec.rs` API returns plain exit codes or summaries. That is not enough because handler resolution must distinguish:

1. timeout
2. user interrupt
3. ordinary non-zero exit

Recommended change:

```rust
pub struct ProcessResult<T> {
    pub data: T,
    pub termination: ProcessTermination,
}
```

Use this for:

1. `run_child`
2. `run_child_capture`
3. `run_child_stream`

On timeout, the process result should carry `ProcessTermination::TimedOut` even if the OS-level exit code is `143` or similar.

## Validation Engine

### Phases

Validation runs in three moments:

1. pre-run
2. post-run
3. post-handler-attempt, if a handler performs another action

The core loop is:

1. parse harness plan
2. evaluate pre-checks
3. capture pre-run snapshot for post checks
4. run provider or handler-deviated command
5. evaluate post-checks
6. if any failure occurs, resolve a handler and either continue or fail

### Pre-check execution

Pre-checks run before each attempt, not only before the first attempt.

This matters because handlers can:

1. redirect to a different document
2. change effective frontmatter through `set`
3. create or remove files between attempts

### Post-check snapshot strategy

For post validations that compare before vs after, the engine captures pre-run state only for the subjects that are actually referenced by post checks.

That keeps the snapshot small and deterministic.

Recommended capture behavior:

1. `file_changed` / `file_unchanged`
   - capture `exists`, `is_dir`, and BLAKE3 hash of file bytes
2. `frontmatter_prop_*`
   - capture source document frontmatter values from the on-disk source markdown
3. response validations
   - use `AttemptOutcome.final_response`

### Message rendering

Do not reuse the hook-event template engine directly.

Reason:

1. the existing template engine is tied to `EventMeta`
2. validation messages need a much smaller, validation-specific variable set

Add a small harness renderer that supports Handlebars-style placeholders over a flat string map.

Common keys:

1. `status`
2. `file`
3. `dir`
4. `prop`
5. `expected`
6. `actual`
7. `length`
8. `response_length`
9. `command`

Default status tokens:

1. success: `<b><green-500>✓</green-500></b>`
2. failure: `<b><red-500>⤫</red-500></b>`

Rendered text should be passed through the existing `Prose` output path.

## Dirty Source Code Checks

`no_dirty_source_code` and `has_dirty_source_code` should not guess based on all files in git status. The check must be source-code aware.

Recommended implementation:

1. detect repo root from the current wrapper environment
2. run a non-interactive git status query scoped to the requested root
3. filter paths by known source-code extensions and common source filenames

Initial file-class filter:

1. Rust: `.rs`
2. JS/TS: `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`
3. Python: `.py`
4. Go: `.go`
5. JVM: `.java`, `.kt`
6. Web: `.css`, `.scss`, `.html`
7. Shell: `.sh`, `.bash`, `.zsh`
8. Config/code-adjacent files explicitly named by this repo such as `justfile`, `Cargo.toml`, `package.json`

The implementation should live in the harness layer, not inline shell snippets inside the wrapper.

## Write-Permission Validation

`has_write_permission` is not an OS writability check. The spec explicitly says it checks whether the agent is allowed to write.

v1 should define it as:

1. filesystem path must be writable by the current OS user
2. provider runtime policy must not obviously forbid writes to that path

Provider policy evaluation should be adapter-based:

```rust
pub trait HarnessPermissionProbe {
    fn can_write(&self, path: &Path, launch: &AttemptLaunchContext) -> PermissionAssessment;
}
```

`PermissionAssessment`:

```rust
pub enum PermissionAssessment {
    Allowed,
    Denied { reason: String },
    Unknown { reason: String },
}
```

For v1, `Unknown` should fail the validation with a clear message instead of silently passing.

## Handler Design

### Failure events

The engine should normalize failures into:

```rust
pub enum FailureEvent {
    AgentFailure,
    Timeout,
    Validation(ValidationEvent),
}
```

`ValidationEvent` names match frontmatter names exactly, for example:

1. `file_exists`
2. `response_missing`
3. `frontmatter_prop_equals`

### Programmatic handler protocol

`handle` should use the same runtime-command type and shell approval path as other runtime commands.

Invocation contract:

1. command receives a JSON payload on stdin
2. Claudine sets environment variables describing the attempt
3. command stdout is parsed as:
   - empty / `null` / `false`: unhandled
   - `true`: `retry` with defaults
   - JSON object: explicit handler response

Recommended stdin payload:

```json
{
  "provider": "codex",
  "source_file": "/abs/path/file.md",
  "attempt": 1,
  "session_id": "sess_123",
  "termination": "timeout",
  "failure_event": "timeout",
  "failure_phase": "agent",
  "message": "Timed out after 5m",
  "check": {
    "name": "response_missing",
    "subject_key": "failed"
  },
  "response": {
    "text": "partial response here"
  }
}
```

Programmatic handlers are not allowed to return `deviate`. That is enforced after JSON parsing.

### Retry

`retry` always starts a fresh non-interactive session.

Prompt behavior:

1. rerun the same effective prompt body
2. append handler prompt text if provided
3. if no handler prompt is provided, append a short default instruction explaining the failure and asking the agent to correct it

### Resume

`resume` requires:

1. provider resume support
2. a captured `session_id`
3. a non-empty `prompt`

If any requirement is missing, the handler itself fails with an actionable error.

Resume behavior:

1. start a new process using the provider's non-interactive resume entrypoint
2. resume the prior session by `session_id`
3. append the new handler prompt

### Redirect

`redirect` reparses and recomposes a different Markdown source.

Behavior:

1. resolve redirect file with harness path resolution
2. load its harness plan
3. apply `set` overrides to the redirected document
4. if `resume: true`, continue the existing session if the provider supports resume and a session exists
5. otherwise start a fresh session

The redirected document becomes the current harness document for subsequent attempts.

### Deviate

`deviate` executes an approved host command instead of launching the provider.

Behavior:

1. run the command directly, not through a shell
2. expose the same attempt context env vars available to programmatic handlers
3. after successful command completion, run post-checks for the current document
4. if post-checks still fail, continue normal handler resolution

## Resume Support Matrix

Based on the current agent capability catalog, v1 should treat provider resume support as:

| Provider | Resume support | Current non-interactive entrypoints |
|----------|----------------|-------------------------------------|
| Claude | Yes | `claude --print`, `claude -c --print`, `claude -r <session> --print` |
| Codex | Yes | `codex exec`, `codex review`, `codex exec review`, `codex exec resume` |
| Gemini | No | prompt/stdin only |
| Goose | No | no supported non-interactive resume flow |
| Kimi Code | Yes | `kimi --print`, `kimi --quiet`, structured modes |
| OpenCode | No | `opencode run` only |
| Qwen Code | Yes | `qwen --continue`, `qwen --resume <id>` |
| Roo Code | No | no supported wrapped resume flow |

The harness should not maintain a second hand-written support list. It should derive from `claudine::agents`.

Recommended addition:

```rust
pub struct ResumeLaunchSpec {
    pub supported: bool,
    pub description: Vec<&'static str>,
}
```

exposed from the wrapper profile layer so the harness can ask the provider profile to build resume args instead of hardcoding them.

## Shell Policy Reuse

The spec requires runtime-detectable commands to be approved against the whitelist immediately. Claudine should not build its own approval format.

Reuse Darkmatter shell policy for:

1. `shell_command`
2. `deviate`
3. `handle`

Recommended approach:

1. add a thin adapter in `claudine::harness` that uses:
   - Darkmatter tokenizer
   - Darkmatter whitelist/blacklist store
   - Darkmatter approval callback contract
2. store policy files alongside Claudine config, for example:
   - `~/.claudine/.shell-whitelist`
   - `~/.claudine/.shell-blacklist`

Prefer a shared adapter, not a copy of Darkmatter internals.

If exact file-name reuse with Darkmatter is acceptable, that is even better because it avoids separate approval surfaces.

## Wrapper Orchestration Loop

The current wrapper is mostly single-shot. This feature requires one orchestration layer above the existing execution call.

Recommended flow:

1. resolve prompt source and build effective prompt as today
2. parse composed frontmatter into `HarnessPlan`
3. if no harness properties are present, continue with current one-shot behavior
4. otherwise enter `run_harnessed_prompt(...)`

Pseudo-flow:

```rust
loop {
    evaluate_pre_checks()?;
    let snapshot = capture_pre_run_snapshot()?;
    let outcome = launch_attempt()?;

    match evaluate_post_checks(&snapshot, &outcome) {
        Ok(()) => return success,
        Err(failure) => match resolve_handler(&failure, &state)? {
            Some(next) => apply_handler(next)?,
            None => return failure,
        }
    }
}
```

State carried across attempts:

1. current document source
2. current effective frontmatter overrides
3. current prompt body
4. attempt counter per handler action
5. most recent `session_id`
6. last structured summary / final response text

## Frontmatter Override Semantics

Handler `set` is an in-memory overlay, not an immediate file mutation.

Rules:

1. merge shallowly at the top-level frontmatter map
2. `null` removes a property
3. overrides affect:
   - prompt recomposition
   - env var derivation from residual frontmatter
   - timeout resolution
   - validation parsing for the next attempt
4. overrides do not directly change the on-disk file unless the agent or a deviated command edits it

## Output Model

Every check should emit a single success/failure line in declaration order.

Recommended behavior:

1. pre-check success lines print before the attempt starts
2. agent failure / timeout line prints once per attempt
3. post-check lines print after the attempt result
4. handler `msg` prints immediately before the next attempt begins
5. `say` is best-effort and never blocks the loop

Existing helpers in `claudine/cli/src/output.rs` should be generalized from frontmatter-prompt-specific naming (`fm_check_ok`, `fm_check_fail`) to harness-wide check rendering.

## Error Strategy

Use plain-English errors with enough context to fix the problem immediately.

Recommended error families:

1. parse/configuration errors
2. runtime validation failures
3. handler resolution failures
4. provider resume unsupported failures
5. shell approval failures

Do not collapse them all into generic `eyre!` strings deep in the validation engine. Add a dedicated `HarnessError` enum in the library module.

## Testing Strategy

### Unit tests

Add focused unit tests for:

1. timeout parsing
2. validation parsing from mapping and list forms
3. handler parsing in generic and subject-specific forms
4. relative path resolution
5. response validations
6. file hash snapshot/compare logic
7. programmatic handler stdout parsing
8. shell approval reuse for runtime commands

### Integration tests

Add wrapper integration tests using a fake provider binary/script to simulate:

1. successful first pass
2. pre-check failure with no handler
3. pre-check failure with `retry`
4. timeout with `resume`
5. post-check `file_changed` failure with `redirect`
6. `response_missing` failure driven by structured summary text
7. `deviate` command success and failure
8. programmatic `handle` returning `true`
9. programmatic `handle` returning a JSON handler response

### Provider-aware tests

Add smaller tests at the profile layer for:

1. building resume args from `session_id`
2. rejecting resume for unsupported providers
3. timeout termination being reported as `TimedOut`, not generic exit code `143`

## Documentation Updates

This feature changes public behavior and authoring conventions. The implementation must update:

1. `claudine/cli/README.md`
2. `claudine/docs/prompt-file-design.md`
3. `claudine/lib/README.md`
4. `.claude/skills/claudine/SKILL.md`

If policy files are shared with Darkmatter, document that explicitly so users do not end up approving the same command twice in two different places.

## Implementation Order

Recommended delivery order:

1. add `claudine::harness` parse/types/timeout modules
2. add validation engine without handlers
3. refactor `exec.rs` to report timeout explicitly
4. wire pre/post checks into wrapper composition flows
5. add YAML handlers (`retry`, `redirect`)
6. add resume support via wrapper profile resume builders
7. add runtime command support (`shell_command`, `deviate`, `handle`) using shared shell policy

That order keeps the first milestone useful and testable before the control-flow complexity of retries and resume is introduced.
