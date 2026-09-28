# Sequence: Current Implementation

This document describes the sequence implementation that exists before the Sequence Plus refactor. It is an implementation inventory, not a design proposal. Statements below are based on the current library, CLI orchestration, error rendering, completion code, and tests.

## Executive summary

`claudine sequence` runs one Markdown composition document repeatedly against an ordered list of states. Each state becomes a reserved per-step overlay, and each prepared prompt is executed serially through the same wrapper-grade composition executor used by `compose` and `inline-compose`.

The current implementation is already more than a simple loop:

- It accepts Markdown sequence documents and top-level YAML sequence documents.
- A Markdown document can define its list inline or reference a separate YAML list.
- Steps can be strings or objects with a required string `name`.
- An external list can apply a small string-template layer to object steps.
- Provider/model resolution happens before composition or provider launch.
- Schema validation and shell approval are performed for every step before any provider step launches.
- Missing schema properties are aggregated across steps and can be collected once in an interactive terminal.
- Provider sessions execute serially with shared shell approvals, per-step timeouts, lifecycle handling, dry-run support, and an aggregated performance report.
- Execution failures obey effective fail-fast policy. A sequence still exits nonzero if any executed step fails when fail-fast is disabled.
- A `prompt` property changes the entire sequence to inline-compose semantics, including body write-back.

The main architectural limitation is that a sequence is still one document plus a static list of overlays. Steps cannot select different prompt documents, declare dependencies, consume structured results from earlier steps, or vary provider/model/lifecycle configuration independently. The current `previous_state` and `next_state` values refer to neighboring authored list items, not execution results.

## Entry point and command surface

The clap command is `claudine sequence <ARG>...`, routed through `commands::sequence::run_sequence`. The positional parser expects exactly one file reference plus zero or more shorthand `key=value` setters. Composition argv normalization also applies, so provider booleans, flags placed after the file, help hoisting, and the setter separator behave consistently with `compose` and `inline-compose`.

Sequence adds one command-specific option:

```text
--fail-fast <BOOL>
```

Accepted values are `true`, `false`, `1`, `0`, `yes`, and `no`, case-insensitively. The CLI value overrides document `fail_fast`; without either, fail-fast defaults to `true`.

The flattened composition flags include provider selection, model, `--set`, shorthand setters, YOLO, include/exclude, system-prompt files, wall-clock timeout, step-silence timeout, OpenCode stall timeout, operation mode, sandbox/repo mode, MCP selection, strict mode, interactive overrides, quiet/silent, dry-run, and performance reporting.

Validation performed before orchestration includes:

- `--timeout` and `--step-timeout` are rejected with `--interactive`.
- Timeout and stall-timeout strings are parsed before doing sequence work.
- A setter-only invocation is rejected because it has no file reference.
- Missing operation files can enter the sequence-scoped operation-file autocomplete path.
- A source without a `sequence` property is rejected.
- `interactive: true` in authored frontmatter is rejected. `interactive: false`, `null`, or absence is accepted. A one-run `--interactive` flag remains available.

`run_sequence` exits the process with the integer returned by the orchestrator. Typed composition errors instead bubble through the top-level error walker and use the standard styled `BlockError` rendering path.

## Accepted source documents

### Markdown source

Markdown uses the normal composition source resolver, including `biscuit_file::FileReference` behavior and package-area magic paths. The document frontmatter contains the sequence definition and all shared composition configuration; the Markdown body is the prompt in normal sequence mode.

An inline list can contain scalar strings:

```yaml
---
sequence:
  - alpha
  - beta
---
Work on {{ state }}.
```

Each string is both the display name and the value of `state`.

An inline list can instead contain objects:

```yaml
---
sequence:
  - name: alpha
    topic: parsing
  - name: beta
    topic: rendering
---
Work on {{ state.topic }}.
```

Every object must contain a string `name`. The complete object becomes `state`; arbitrary additional fields are retained.

The list must be nonempty. Numbers, booleans, nulls, arrays nested as step values, and objects without a string `name` are rejected with typed sequence errors.

### Top-level YAML source

The CLI also accepts a `.yaml` or `.yml` file directly. It loads the top-level mapping and converts it to synthetic Markdown frontmatter with an empty body. This allows a YAML file containing a top-level `sequence` key and, optionally, `prompt`, schema, selection, lifecycle, or other composition properties.

In practice, a useful top-level YAML sequence normally needs `prompt`, because its synthetic Markdown body is empty. Presence of a string `prompt` selects inline mode.

Despite the loader's general wording, the direct YAML entry point currently still requires a top-level `sequence` property. The external-only `kind: sequence` plus `list:` form is not recognized when that YAML file is passed directly to the command, because `resolve_sequence_plan` first looks only for `sequence`. That richer form works only when referenced by a Markdown document's `sequence: <path>` property.

### Referenced external YAML list

A Markdown document can set `sequence` to a string path/reference. Resolution supports:

- Plain relative paths, resolved relative to the source document directory.
- Absolute paths.
- `~` expansion against the home directory.
- `@` magic references.
- `!` package references.
- `vault:` and `%` references.
- File references containing `{{...}}` environment expansion.

Special references are resolved from the source document directory, not the process CWD. This is important when the command is launched from elsewhere or when magic lookup needs the source document's repository/package context.

External YAML accepts two shapes. The simple shape is:

```yaml
sequence:
  - alpha
  - beta
```

The richer shape is:

```yaml
kind: sequence
list:
  - name: alpha
    topic: parsing
  - name: beta
    topic: rendering
template:
  output: "docs/{{name}}.md"
  label: "{{topic || general}}"
```

`kind` is optional when `list` is present, but if supplied it must be the string `sequence`. `template` is optional, must be an object, and every template value must be a string. Templates require every list item to be an object.

The template renderer is intentionally small and separate from Darkmatter composition. It supports `{{key}}` and `{{key || default}}` placeholders over top-level item fields. String values are inserted directly, null/missing values use the default or an empty string, and other JSON values use their JSON display form. Single quotes surrounding the default are trimmed. It does not implement the full Darkmatter expression language, nested property lookup, functions, escaping, or recursive evaluation.

Template values are added only when a step does not already define that key, so item fields override template defaults. Template keys may not collide with the seven reserved per-step overlay keys. The simple external `sequence:` shape rejects a sibling `template`; authors must use the `list` form.

The parent composition document supplies `fail_fast`. A `fail_fast` property inside the referenced external YAML is not read.

## Normalized plan and per-step overlay

The library normalizes input into:

- `SequencePlan`: source location category, ordered steps, and document fail-fast value.
- `SequenceStep`: zero-based index, display name, and raw JSON state.
- `SequenceSource`: either inline or an external resolved path.

For each step, `build_step_overlay` creates these reserved values:

| Property | Meaning |
|---|---|
| `state` | Current authored step value: string or object. |
| `previous_state` | Previous authored step value, or `null` for the first step. |
| `next_state` | Next authored step value, or `null` for the last step. |
| `is_first` | Whether this is the first step. |
| `is_last` | Whether this is the last step. |
| `step` | One-based step number. |
| `total_steps` | Total number of steps. |

User setters are merged first and the overlay is applied afterward. Therefore the reserved values always win over `--set` and shorthand setters. Among user inputs, shorthand positional setters win over overlapping `--set` values.

These overlays are static views of the normalized plan. They do not carry previous provider output, prior exit status, accumulated artifacts, or mutations made during execution.

Each step also receives process-context overrides used during composition and execution:

- `CLAUDINE_FAIL_FAST` is the effective boolean policy.
- `AGENT` is the resolved provider slug when a target exists.
- `MODEL` is the resolved model when one exists.
- `YOLO` is the CLI YOLO value.

In an unresolved dry-run agent state, `AGENT` is intentionally absent so composition matches direct compose dry-run behavior.

## Mode selection and document mutation

Mode is chosen once for the entire sequence from the source document:

- No `prompt` key: chained-document mode. The composed Markdown body is sent to the provider and the source file is not rewritten by sequence itself.
- String `prompt`: inline mode. The composed `prompt` value is sent to the provider and each successful inline closure can replace the source body while preserving frontmatter. The final executed write-back is what remains on disk.
- Non-string `prompt`: rejected before any step launches with the same typed error used by `inline-compose`.

All steps are prepared during Phase 1c before Phase 2 executes any provider. Consequently, later step prompts are composed from the original resolved source and their own static overlays; they are not recomposed after an earlier inline step rewrites the body. There is a stale comment in `cli/src/commands/wrap/sequence/mod.rs` claiming each step reads the live file and sees the prior rewritten body. The current control flow does not do that: Phase 1c stores every `PreparedComposition` first, and Phase 2 clones those prepared values. The code behavior is authoritative.

Dry-run does not launch a provider and therefore does not perform inline write-back.

## Orchestration pipeline

### 1. Source and document-level preparation

The command resolves the source, rejects authored interactive mode, builds the normalized plan, merges setters, and performs a schema-aware document-level scrub of invalid optional values. Invalid optional schema values are dropped with warnings. Required-value validation is deliberately deferred so failures can be aggregated across steps.

Launch-area CWD is captured before wrapper setup can change process CWD. File-typed schema values and read-side file references can then resolve document-first with launch-area fallback rather than accidentally depending on a later process CWD.

### 2. Effective fail-fast and shared invocation state

Effective fail-fast is CLI override, then document value, then `true`. A shell approval cache is created once and shared by all preparations and executions. A sequence-wide interrupt flag is also registered.

### 3. Provider and model resolution (Phase 1a/1b)

The document has one shared raw `agent`/`model` hint surface; steps cannot author independent targets in their state objects. Top-level user setters for `agent` and `model` update the parsed selection hints. Explicit provider and CLI model flags remain higher-precedence locked choices.

Installed-provider discovery, selection configuration, launch context, repository root, and environment context are captured once in `CompositionPrepContext` and reused by every step. Model catalog refresh is provider-scoped and only occurs when a frontmatter model hint actually needs catalog validation.

Live resolution behavior mirrors direct compose:

- Explicit provider flags bypass agent-state prompting.
- A valid single agent or a list with exactly one installed provider auto-selects.
- Missing, invalid, not-installed, ambiguous, or zero-installed-list states require review in a stderr TTY.
- The review screen proposes a provider/model for every step and returns a per-step target vector.
- In a non-TTY, a prompting state aborts before any provider launch with `AgentResolutionFailed`.
- The TTY gate uses stderr, so redirecting stdout does not disable review.
- Canceling the review is treated as exit 130.

Although the review data is per-step, the initial provider/model hints are document-level and identical across steps. The review UI is the current mechanism by which targets may differ between steps in one interactive run.

Dry-run never opens the picker. Explicit and auto-selectable states produce concrete targets; all other states remain unresolved so the normal dry-run renderer can show the same agent-resolution state per step without launching or silently choosing a provider. The model catalog is not consulted in this dry-run resolution path.

### 4. All-step schema, shell, and composition preparation (Phase 1c)

Every step is prepared before any provider runs:

1. Build and merge the reserved overlay.
2. Build the `AGENT`, `MODEL`, `YOLO`, and `CLAUDINE_FAIL_FAST` context.
3. Pre-validate the effective frontmatter against `$schema`.
4. Collect missing required properties instead of immediately failing that step.
5. Resolve template shell directives and collect/approve shell commands.
6. Prepare the final direct or inline composition, including the normal lifecycle and harness surfaces.
7. Store `PreparedComposition` plus the step environment overrides.

Shell approvals accumulate across steps and the cache prevents the same approved command from prompting repeatedly. Template shell preflight defers lifecycle keys, because lifecycle commands have their own audit path and lifecycle interpolation is event-time behavior. The preflight compose and final prepare share source-file and launch-area resolution semantics.

Preparation errors other than missing required schema properties abort immediately, regardless of execution fail-fast. This includes invalid required schema values, malformed composition, shell/preflight failures, and other typed preparation errors. Thus `fail_fast: false` governs serial execution failures; it does not turn the prelaunch preparation phase into a best-effort partial planner.

If one or more steps are missing required schema properties:

- Non-interactive execution returns one `SequenceMissingProperties` error containing every failing step and its missing properties.
- If interactive schema collection is allowed, missing properties are deduplicated by name, type label, and description and prompted once via `biscuit-tui`.
- A per-step status report reflects effective overlay and setter values, so already-satisfied fields are not shown as missing.
- Collected values are merged into user overrides and all steps are prepared again once.
- Unsupported interactive shapes produce the standard typed `UnsupportedInteractiveSchema` error in an interactive session. In a non-TTY they remain part of the actionable aggregate report.
- Canceling or failing collection falls back to the aggregate missing-properties error.

The preparation phase checks the interrupt flag between steps. If interrupted, it returns no prepared contexts and the command exits 130 without provider execution.

### 5. Serial execution (Phase 2)

Prepared steps run in order through `execute_composition_request_inner`, the wrapper-grade executor. Each request carries the shared launch/prep snapshots, resolved target, composition mode, lifecycle configuration, provider flags, timeouts, system prompt, sandbox/repo/MCP options, strictness, interactivity, and shared approval cache. The request is marked as a sequence run.

A step succeeds only when the executor returns exit code 0. Nonzero provider outcomes and executor errors are recorded as failures. With effective fail-fast enabled, the loop stops after the first execution failure. With it disabled, later prepared steps continue. The summary records only steps that actually executed; skipped tail steps after fail-fast are not counted as failures.

Ctrl+C is checked between steps and recognized from exit code 130 or the sequence interrupt flag during a step. The interrupted step is recorded as failed, remaining steps do not run, the performance report is marked partial, and the sequence returns 130. Unix has an additional sequence-wide signal-hook registration; non-Unix builds rely on the shared wrapper signal machinery and returned outcome rather than that Unix registration.

All provider launches remain serial. There is no parallel execution, DAG scheduling, per-step retry policy at the sequence layer, checkpoint/resume cursor, or persisted sequence run state. Provider lifecycle stacks and loop behavior can still act inside each ordinary composition execution.

## Failure and exit semantics

The normal returned codes are:

| Code | Meaning |
|---|---|
| `0` | Every executed/planned step completed successfully. |
| `1` | At least one step failed and the run was not interrupted. |
| `130` | Ctrl+C/review cancellation interrupted the sequence. |

Clap usage errors use clap's normal exit code 2. Errors raised before the orchestrator returns an integer flow through the application error walker and produce a nonzero process exit.

Fail-fast precedence is:

1. `--fail-fast` CLI option.
2. Markdown document `fail_fast` boolean.
3. Default `true`.

Wrongly typed document `fail_fast` is rejected. Referenced external sequence files do not override it. Effective fail-fast is exposed to composition/provider execution through `CLAUDINE_FAIL_FAST`, allowing prompts and child behavior to observe the policy.

The library defines typed errors for invalid sequence shapes, empty lists, external reference/YAML load causes, external structure, missing or wrongly typed step names, invalid templates, reserved-key collisions, aggregated missing properties, and rejected sequence interactivity. External load errors retain typed `FileReferenceError` or `YamlError` sources. Frontmatter-rooted errors participate in the standard TTY-gated, syntax-highlighted frontmatter appendix.

`SequenceSelectionFailed` and its supporting record type still exist in the library error model, but the current live orchestration path uses the shared `AgentResolutionFailed` behavior for non-TTY prompting states and the review UI for TTY states. It is not the principal error emitted by the current target-resolution path.

## Output and reporting

Normal stderr status output includes:

- A sequence header with step count and effective fail-fast value.
- Provider/model review when required.
- Schema collection/status information when required.
- A preflight-start message and all-steps-approved message.
- Per-step starting and success/failure statuses.
- A final succeeded/failed count.

Status rendering uses `biscuit-terminal` components (`Status`, `Prose`, and `HorizontalRule`) through the shared logging/terminal context. The final report contains counts, while `SequenceRunSummary` internally also retains per-executed-step name, success flag, optional error text, and duration.

`--silent` suppresses routine status and final summary output, but it does not suppress errors or an explicitly requested performance report. `--quiet` is forwarded to the per-step executor. Dry-run explicitly ignores quiet/silent for its rendered artifacts.

With `--perf`, one sequence-level report aggregates startup/pre-dispatch timing and all prepared/executed step timings. Fail-fast and interrupt runs mark it partial. The report is emitted to stderr even under quiet/silent.

## Dry-run behavior

Sequence dry-run performs source resolution, selection classification, schema validation, real shell expansion/side effects, shell audit/approval, and composition, but stops before provider launch. Inline sources are not mutated.

For each step:

- The composed body goes to stdout.
- Highlighted effective frontmatter and metadata go to stderr.
- A full-width dashed `HorizontalRule` separates documents on stderr before steps 2 through N.
- Unresolved agent states are rendered rather than auto-selected or prompted.

Because each dry-run step returns a successful no-launch outcome when composition succeeds, normal execution fail-fast does not change successful dry-run iteration. A composition/preparation failure happens in the eager all-step phase and aborts before Phase 2; the practical behavior is fail-fast preparation even if document fail-fast is false.

## Completion support

Sequence participates in the shared composition completion pipeline and has a sequence-specific completion mode. It completes operation files, file references, provider flags, schema-derived setters, and setter values using the same frontmatter filtering approach as the other composition commands. Integration coverage includes sequence-specific magic references and completion contracts.

## Test coverage currently present

The implementation has substantial L1/L2 coverage distributed by responsibility:

- Library unit tests in `lib/src/composition/sequence.rs` cover no-sequence detection, scalar/object normalization, invalid and empty definitions, fail-fast parsing, both external forms, template behavior and failures, reference variants (`relative`, absolute, magic, package, vault, environment, tilde), typed load failures, overlays, and placeholder fallback behavior.
- Type tests in `lib/src/composition/types.rs` cover first/last/middle overlay values and reserved-key precedence over user setters.
- CLI command unit tests cover sequence interactivity parsing/rejection and enriched source-load errors.
- Orchestrator unit tests cover agent-state auto-selection classification, unsupported interactive schema detection, and launch-area-independent shell preflight.
- `sequence_cli.rs` covers command validation, malformed fences, fail-fast true/false and CLI precedence, environment propagation, setter precedence, state interpolation, shell whitelist reuse, model requirements, and final summary output.
- `sequence_magic_reference.rs` covers relative and magic external lists, source-location versus CWD behavior, and missing external files.
- `sequence_schema.rs` covers cross-step missing-property aggregation, setter satisfaction, unsupported interactive shapes, per-step timeout, shell-expanded schema violations, and malformed step documents.
- `level2_sequence_overlay_pty.rs` covers interactive missing-property deduplication, overlay-satisfied required properties, status reports with setter-satisfied values, provider review messaging, stderr-TTY/stdout-redirection behavior, and review bypass for auto-selectable states.
- `wrap_sequence_composition.rs` covers provider-wide dry-run composition, multi-document output, quiet/silent behavior, dry-run composition failure, unresolved agent-state rendering, live non-TTY aborts, auto-selection, and silent error visibility.
- `sequence_prompt_property.rs` covers inline prompt selection, final body write-back, rejected `interactive: true`, and non-string prompts.
- `sequence_perf.rs` covers the single aggregated report, partial fail-fast reports, and startup timing propagation.
- Additional integration files cover command routing, argv normalization, completion, prompt reporting, schema capture, interrupt behavior inherited through the wrapper executor, and inline-compose's detection of documents containing both `prompt` and `sequence`.

Many provider-launch integration tests use Unix shell stubs and are `#[cfg(unix)]`. Pure parsing, normalization, typed errors, and core cross-platform compilation are not Unix-specific. The implementation is designed to compile on macOS, Linux, and Windows, but the strongest real-process sequence coverage is presently Unix-oriented.

## Current boundaries relevant to Sequence Plus

The following are implementation boundaries, not proposed solutions:

- One source document supplies the prompt/configuration for every step.
- A step is data (`state`), not an independently addressable operation or prompt document.
- Neighbor variables expose authored state only; there is no result/output dataflow.
- All steps are eagerly prepared before execution, so execution-time file/body changes do not recompose later steps.
- Preparation is global and aborting; `fail_fast: false` applies only after serial execution begins.
- Provider/model hints are document-level. Per-step variation is possible only through the interactive review's returned target vector, not authored step configuration.
- External templates are a bespoke shallow string substitution feature, not Darkmatter expressions.
- Referenced YAML can use `kind/list/template`, while a directly invoked YAML file effectively requires `sequence`; the two YAML entry modes are asymmetric.
- External YAML does not own fail-fast or general document settings; those remain on the parent composition document.
- There is no named step ID distinct from the display `name`, dependency graph, conditional step selection, parallelism, persisted checkpoint, resume, structured result contract, or sequence-level retry/backoff.
- The final terminal summary is count-based even though richer per-step result records exist internally.
- The sequence-wide extra SIGINT registration is Unix-only; Windows relies on the shared child-execution signal path.
- Some current comments describe intended live-file chaining that the eager preparation architecture does not implement.

These boundaries define the compatibility surface the Sequence Plus refactor must either preserve deliberately or replace explicitly.
