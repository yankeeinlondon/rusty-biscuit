# Sequence Tech Design

This document turns the sequence spec into an implementation-ready design for a new top-level Claudine command built on the existing composition pipeline.

Primary inputs:

- `claudine/features/2026-04-04-sequence/spec.md`
- current composition pipeline in `claudine/lib/src/composition/`
- current top-level composition commands in `claudine/cli/src/commands/compose.rs`
- current wrapper-grade executor in `claudine/cli/src/commands/wrap/composition.rs`
- current composition behavior documented in `claudine/docs/topics/composition.md`

## Summary

Sequence is a new top-level command, `claudine sequence <file>`, layered on top of Claudine's existing composition functionality.

When a user runs `claudine sequence <file>` against a source document that defines `sequence`, Claudine will:

1. resolve the source document once
2. normalize the sequence into a typed step list
3. compute the effective sequence `fail_fast` policy
4. run the existing composition pipeline once per step with step-specific state injected through Darkmatter `set_overrides`
5. stop on the first failed step when `fail_fast = true`, or continue and report a final mixed-result summary when `fail_fast = false`

The design does not introduce a second execution engine. Sequence is a dedicated CLI entrypoint and outer orchestration layer around the same wrapper-grade composition executor used by Claudine's existing composition flows.

## Goals

1. Support inline sequence definitions and external YAML-backed sequence definitions.
2. Support scalar steps (`- one`) and object steps (`- name: one`).
3. Inject `state`, `previous_state`, `next_state`, `is_first`, `is_last`, `step`, and `total_steps` into each compose run.
4. Add a new public `claudine sequence <file>` command.
5. Add document-default and CLI-override fail-fast behavior for step failures.
6. Keep harness, lifecycle notifications, provider selection, MCP, and streaming behavior unchanged inside each individual step.

## Non-Goals

1. No new generic workflow engine beyond serial step orchestration.
2. No parallel step execution.
3. No persistent sequence resume/checkpoint file in v1.
4. No sequence-specific branching, optional steps, retries across steps, or shell-only steps from the older March draft.
5. No automatic activation inside `claudine compose` or `claudine inline-compose` in v1.
6. No inline-document rewrite mode for `claudine sequence` in v1.

## Command Boundary

The important distinction for implementation is:

1. `claudine sequence` is a CLI command
2. `claudine compose` is a different CLI command
3. both commands use the same underlying composition functionality

Decision for this design:

1. Sequence is exposed as a new top-level command: `claudine sequence <file>`.
2. `claudine compose` and `claudine inline-compose` keep their current one-shot behavior.
3. `claudine sequence` reuses the same underlying wrapper-grade composition machinery as the existing composition flows.

This preserves the intended public command while keeping one shared composition implementation underneath it.

## Spec Clarifications

### 1. Object-step variable access

The spec clearly defines `state` as the current step value, but it is less explicit about whether object fields should also be promoted as top-level variables.

Decision for this design:

1. `state` always contains the full current step value.
2. For object steps, authors access fields as `{{state.name}}`, `{{state.color}}`, etc.
3. Sequence metadata stays in reserved top-level keys:
   - `state`
   - `previous_state`
   - `next_state`
   - `is_first`
   - `is_last`
   - `step`
   - `total_steps`

This avoids silently overwriting unrelated frontmatter keys like `agent`, `timeout`, or `start`.

### 2. `fail_fast` naming overlap with Darkmatter

Darkmatter already has a compose-level `fail_fast` option. Claudine sequence also wants a frontmatter property with the same name.

Decision for this design:

1. Claudine sequence will read frontmatter `fail_fast` as sequence-control metadata only.
2. Claudine will not map that frontmatter key into Darkmatter's `ComposeOptions.fail_fast`.
3. The naming overlap should be documented as reserved behavior inside Claudine composition docs.

This avoids a runtime collision today, but the overlap is real and should not be ignored in later Darkmatter-facing design work.

## User-Facing Contract

### Inline sequence definition

Supported inside source document frontmatter:

```yaml
sequence:
  - one
  - two
  - three
fail_fast: false
```

or:

```yaml
sequence:
  - name: one
    color: red
  - name: two
    color: blue
```

Rules:

1. `sequence` must be a non-empty YAML list, or a string file reference to external YAML.
2. Scalar list items must be strings.
3. Object list items must contain `name`, and `name` must be a string.
4. `fail_fast` is optional and defaults to `true`.

### External YAML sequence definition

When frontmatter `sequence` is a string, it resolves to a YAML file relative to the source document using the same document-centric reference behavior Claudine already uses for composed files.

Supported external shapes:

```yaml
sequence:
  - name: one
    color: red
```

or:

```yaml
kind: sequence
template:
  desc: "{{name}} (_site: {{site}}, repo: {{repo || 'n/a'}}_)"
list:
  - name: Codex CLI
    site: https://developers.openai.com/codex/cli
    repo: https://github.com/openai/codex
```

Rules:

1. `kind: sequence` is optional; when present it must equal `sequence`.
2. `list` must be a non-empty list.
3. `template` is only supported in the `kind/list/template` external-file form.
4. `template` values must be strings in v1.
5. `template` is applied only to object items.

## Target Architecture

Sequence is a dedicated command that wraps the existing single-run composition flow.

```txt
Sequence Command
  -> Resolve Source Once
  -> Detect + Normalize Sequence
  -> For each step:
       Build step overlay
       -> Step-specific pre-flight shell approval
       -> Prepare composition
       -> Execute wrapper-grade composition run
       -> Record success/failure
  -> Emit final sequence summary
```

Each step is a normal one-shot composition run:

```txt
Resolve -> Pre-Flight -> Prepare -> Select Provider -> Launch
```

## Recommended Module Layout

### Library

Add sequence-specific logic under `claudine/lib/src/composition/`:

```txt
claudine/lib/src/composition/
├── mod.rs
├── error.rs
├── prepare.rs
├── resolve.rs
├── select.rs
├── sequence.rs      # new
├── closure.rs
└── types.rs
```

Responsibilities:

- `sequence.rs`
  - detect whether a source has a sequence
  - resolve external YAML references
  - validate and normalize sequence definitions
  - apply external templates
  - build per-step override payloads
- `prepare.rs`
  - accept richer prepare options so sequence can inject env overrides in addition to `set_overrides`
- `types.rs`
  - add typed sequence plan/result structs
- `error.rs`
  - add sequence parse/validation/runtime errors

### CLI / Wrapper

Split outer orchestration from single-step execution:

```txt
claudine/cli/src/commands/
├── compose.rs
├── sequence.rs
└── wrap/
   ├── mod.rs
   ├── composition.rs
   └── sequence.rs     # new
```

Responsibilities:

- `sequence.rs`
  - parse `--fail-fast`
  - build common runtime options
  - resolve source and sequence plan
  - dispatch to sequence orchestration
- `wrap/composition.rs`
  - keep one internal function that executes exactly one prepared composition step
- `wrap/sequence.rs`
  - own the outer serial step loop
  - emit sequence progress and summary

## Data Model

Add the following types to `composition/types.rs`:

```rust
pub struct SequencePlan {
    pub source: SequenceSource,
    pub steps: Vec<SequenceStep>,
    pub document_fail_fast: bool,
}

pub enum SequenceSource {
    Inline,
    External { path: PathBuf },
}

pub struct SequenceStep {
    pub index: usize,
    pub name: String,
    pub raw_state: serde_json::Value, // string or object
}

pub struct SequenceStepOverlay {
    pub state: serde_json::Value,
    pub previous_state: serde_json::Value,
    pub next_state: serde_json::Value,
    pub is_first: bool,
    pub is_last: bool,
    pub step: usize,
    pub total_steps: usize,
}

pub struct SequenceExecutionOptions {
    pub fail_fast_override: Option<bool>,
}

pub struct SequenceRunSummary {
    pub total_steps: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub steps: Vec<SequenceStepResult>,
}

pub struct SequenceStepResult {
    pub step: usize,
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration: std::time::Duration,
}
```

Reserved overlay keys must always win over user-provided `--set` keys.

Merge order:

1. user `--set`
2. sequence overlay

That means a caller cannot override `state`, `step`, etc. with `--set`, which is the only safe behavior.

## Parsing And Normalization

`composition::sequence` should expose:

```rust
pub fn resolve_sequence_plan(
    source: &ResolvedCompositionSource,
) -> Result<Option<SequencePlan>, CompositionError>
```

Behavior:

1. If frontmatter has no `sequence` key, return `Ok(None)`.
2. If `sequence` is a list, normalize inline.
3. If `sequence` is a string, resolve and parse external YAML.
4. Validate list shape and item types.
5. Read optional document `fail_fast`; default to `true`.

### External template application

For external `kind/list/template` YAML:

1. each list item must already be an object with `name`
2. build a small template-evaluation document per item
3. render each template string against the item's own fields
4. merge rendered template fields into that item
5. reject template keys that collide with reserved sequence keys

This keeps template logic local to sequence normalization instead of spreading it into the wrapper runtime.

## Prepare And Composition Context Changes

Today `prepare_direct()` and `prepare_inline()` only accept:

1. `set_overrides`
2. `pre_approved_commands`

Sequence needs one more capability:

1. inject `FAIL_FAST=true|false` into the composition-time environment so `{{env.FAIL_FAST}}` and `::shell` directives see the same state as the child provider process

Recommended refactor:

```rust
pub struct PrepareOptions {
    pub set_overrides: Option<serde_json::Value>,
    pub pre_approved_commands: Option<HashSet<String>>,
    pub env_overrides: BTreeMap<String, String>,
}
```

Then:

```rust
pub fn prepare_direct(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError>
```

### Small Darkmatter dependency

Claudine likely needs a small Darkmatter-facing extension here. The current `ComposeContext` captures env at construction time and exposes production reads but not production writes.

Recommended addition in Darkmatter:

1. `ComposeContext::with_env_overrides(...)`
2. or `ComposeOptions::with_env_overrides(...)`

Without that, Claudine would need to mutate process-global env during composition, which is not acceptable.

## CLI Design

Add dedicated sequence args:

```rust
pub struct SequenceArgs {
    #[arg(value_name = "FILE")]
    pub file: String,

    #[arg(long = "fail-fast", value_name = "BOOL")]
    pub fail_fast: Option<bool>,
}
```

alongside the existing provider/session flags reused from composition:

```rust
#[arg(long = "fail-fast", value_name = "BOOL")]
pub fail_fast: Option<bool>
```

Behavior:

1. absent: use the document default
2. present: override the document default for this invocation only

Recommended parsing:

- `BoolishValueParser`
- accepted values: `true`, `false`, `1`, `0`, `yes`, `no`

## Execution Flow

### Dispatch

Add a new command entrypoint:

1. `run_sequence_inner()`
2. resolve source once
3. require a valid sequence plan
4. call `execute_sequence_request(...)`

### Sequence step loop

For each step:

1. build `SequenceStepOverlay`
2. merge user `--set` with reserved overlay keys
3. run pre-flight shell discovery for that step's effective compose inputs
4. prepare that step's `PreparedComposition`
5. build a normal `CompositionExecutionRequest`
6. execute one step through the existing wrapper-grade pipeline
7. record outcome and decide whether to continue

### Step-specific pre-flight

Pre-flight must run per step, not once for the whole document.

Reason:

1. `::shell` directives can be gated by state-dependent interpolation or conditionals
2. harness shell commands can differ between steps because effective frontmatter changes per step

Recommended optimization:

1. keep an in-memory union of previously approved normalized commands for the current sequence run
2. merge that union into later steps' approval set so "allow once" remains "allow once for this sequence run"

## Single-Step Executor Refactor

Today `execute_composition_request()` is both:

1. the public composition executor
2. the only place that knows how to launch one run

Sequence needs the second role without duplicating the first.

Recommended refactor:

```rust
pub(crate) fn execute_composition_request(
    request: CompositionExecutionRequest,
    verbose: u8,
) -> Result<i32>

fn execute_single_prepared_composition(
    request: CompositionExecutionRequest,
    verbose: u8,
) -> Result<SingleCompositionOutcome>
```

where:

```rust
pub struct SingleCompositionOutcome {
    pub exit_code: i32,
    pub provider: Provider,
    pub selection_reason: SelectionReason,
}
```

The existing public function can map `exit_code` back to today's behavior, and sequence orchestration can consume the richer structured result.

## Failure Semantics

Sequence step failure means any of the following:

1. sequence step pre-flight fails
2. step preparation fails
3. provider execution returns non-zero
4. harness resolution or execution fails
Sequence-level behavior:

1. when effective `fail_fast = true`, stop immediately and return exit code `1`
2. when effective `fail_fast = false`, record the step failure and continue
3. after the last step:
   - return `0` when all steps succeeded
   - return `1` when one or more steps failed

Sequence does not introduce cross-step recovery. Harness `retry`, `resume`, `redirect`, and `deviate` remain step-local behaviors inside the single-step executor.

## Provider Selection And Harness Semantics

Provider selection remains step-local and uses the step's effective frontmatter after sequence overrides are applied.

That means:

1. different steps may resolve to different providers if the document authors that behavior intentionally
2. harness plans are parsed separately per step
3. lifecycle notifications fire separately per step
4. MCP tag resolution and prompt cleaning remain unchanged per step

This is the least surprising behavior because each step is still just a normal composed run with extra state injected.

## Terminal Reporting

Claudine should add explicit sequence progress output around the normal step output:

1. start-of-step line:
   - `[2/5] Codex CLI`
2. end-of-step result:
   - `step 2/5 succeeded`
   - `step 2/5 failed: <reason>`
3. final summary:
   - `Sequence finished: 4 succeeded, 1 failed`

When `--silent` is set, sequence progress output should be suppressed in the same way as other composition preamble output.

## Error Surface

Add sequence-specific `CompositionError` variants:

```rust
SequenceInvalid(String)
SequenceEmpty
SequenceExternalLoad(String)
SequenceExternalWrongType(String)
SequenceStepNameMissing { index: usize }
SequenceStepNameWrongType { index: usize, found: String }
SequenceTemplateWrongType { key: String, found: String }
SequenceTemplateRequiresObjectItems
SequenceReservedTemplateKey(String)
```

These should fail before any provider step launches.

## Testing Strategy

### Library tests

Add focused tests in `claudine/lib/src/composition/sequence.rs`:

1. inline scalar list normalizes correctly
2. inline object list requires `name`
3. external `sequence:` YAML loads correctly
4. external `kind/list/template` YAML applies rendered template fields
5. empty lists fail
6. invalid scalar/object mixtures fail with clear errors
7. overlay generation sets correct first/last/step counters
8. user `--set` values are preserved but reserved keys are overwritten by sequence overlay

### Prepare/context tests

Add tests in `prepare.rs`:

1. `FAIL_FAST` env is visible during composition
2. step overlays are visible in effective frontmatter interpolation

### CLI / wrapper integration tests

Add integration coverage for:

1. `sequence` with `fail_fast=true` stops on first failed step
2. `sequence` with `fail_fast=false` continues and returns exit code `1` when any step failed
3. per-step harness parsing changes with step state
4. per-step provider selection changes with step state
5. per-step shell pre-flight honors allow-once approvals within one sequence run
6. `sequence` errors clearly when the source file has no `sequence` property

## Implementation Order

1. add sequence types and parser/normalizer in `composition::sequence`
2. refactor prepare helpers to accept richer options
3. add composition-time env override support for `FAIL_FAST`
4. refactor wrapper executor into reusable single-step machinery
5. add `wrap/sequence.rs` orchestration
6. add `commands/sequence.rs` plus `claudine sequence <file>` CLI wiring
7. add `--fail-fast` CLI flag and command dispatch
8. add tests
9. update `claudine/docs/topics/composition.md`

## Final Recommendation

The clean implementation path is:

1. make `claudine sequence <file>` a dedicated top-level command
2. model it as a typed outer loop over the existing wrapper-grade composition executor
3. run pre-flight and preparation per step
4. keep step semantics identical to today's one-shot composition pipeline

That gives Claudine a useful serial composition feature without turning composition into a second workflow system.
