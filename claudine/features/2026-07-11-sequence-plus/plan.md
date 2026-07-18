---
total_phases: 13
created: 2026-07-12
phase: 1
yolo: "true"
source_files_during_phase_1:
  - claudine/lib/src/composition/sequence/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - claudine/features/2026-07-11-sequence-plus/phase-1-baseline.md
skills_files_updated_during_phase_1: []
packages:
  - claudine
---

# Sequence Plus Execution Plan

This plan converts [`spec.md`](spec.md) into a dependency-ordered implementation
program for `biscuit-file`, Darkmatter, `claudine`, and `claudine-cli`. Every
phase ends in an observable checkpoint; implementation must not advance through
a failed checkpoint.

## Scope and fixed decisions

- The ratified clean break is intentional: no aliases are retained for
  `previous_state`, `next_state`, `step`, or `total_steps`, and the external
  `list:` sequence shape is removed.
- Document-level `prompt: <prose>` retains inline-compose semantics; task-level
  `prompt: <file-ref>` identifies a referenced prompt document.
- Dynamic sources are snapshotted once during static preflight. Step/task
  composition is otherwise just in time and re-reads live source files at the
  boundaries defined in the specification.
- `outputs` is the only output accumulator. Darkmatter already implements
  `last(list)`, so this feature verifies and reuses it instead of adding another
  collection function.
- Group `loop` execution is blocked because iteration commit semantics remain
  unratified. This release must reject a group carrying `loop` with a typed,
  actionable error; non-looping serial and parallel groups remain in scope.
- Persisted sequence checkpoint/resume, nested sequences, nested groups,
  sequence-level parallelism, group-level fail-fast controls, and sibling
  cancellation are out of scope.
- All new path resolution goes through `biscuit_file::FileReference` and
  `resolve_from(authoring_document_directory)`. All terminal presentation goes
  through `TerminalRenderable` components.

## Dependency map

Phase 1 establishes regression and impact boundaries. Phase 2 supplies shared
library primitives. Phases 3–4 define and populate the normalized plan. Phase 5
builds the complete preflight graph. Phases 6–8 establish runtime state, atomic
tasks, and serial JIT execution. Phases 9–10 add groups. Phase 11 adds the
concurrent rendering/reporting contract. Phases 12–13 close cross-platform,
documentation, lint, and change-scope validation.

## Phase 1 — Baseline, blast radius, and characterization

**Goal:** Freeze retained behavior and establish safe seams before replacing the
HIGH-risk sequence resolver.

- [x] Re-run GitNexus upstream impact analysis for `resolve_sequence_plan`, `build_step_overlay`, `execute_sequence`, `run_phase_1c_with_schema`, `run_sequence_steps`, and `execute_composition_request_inner_with_guard`; record direct callers, affected flows/modules, and warn the implementation owner before editing any HIGH/CRITICAL symbol. *(GitNexus block is in a `CLAUDE.md` merge conflict / index possibly stale; used static call-graph analysis instead — callers + owner warning recorded in [`phase-1-baseline.md`](phase-1-baseline.md).)*
- [x] Capture the current sequence execution flow with GitNexus `context`/`query` and identify the exact replacement seams between library normalization, provider/model resolution, preparation, execution, reporting, and the shared composition executor. *(Flow + seams recorded in [`phase-1-baseline.md`](phase-1-baseline.md).)*
- [x] Add or tighten characterization tests for retained behavior: scalar/object steps, document-level inline-compose mode, fail-fast precedence, aggregate missing properties, dry-run target behavior, shell approval sharing, and Ctrl+C exit `130`. *(Existing coverage mapped in baseline doc; added `clean_break::characterize_current_overlay_keys` pinning the current 7-key overlay.)*
- [x] Add regression tests that describe the deliberate removals and initially fail: legacy overlay names and the external `list:` shape must be rejected rather than silently accepted. *(`clean_break::legacy_overlay_names_removed`, `clean_break::external_list_shape_rejected` — `#[ignore]`d to keep the phase harness green; both confirmed to fail today under `--run-ignored=all`.)*
- [x] Mark group `loop` as blocked in the implementation scope and add a failing typed-error test so no implementer invents commit semantics. *(`clean_break::group_loop_rejected`, `#[ignore]`d pending Phase 9 group parsing; fails today under `--run-ignored=all`.)*
- [x] Run the existing Claudine L1 suite with `just test` from `claudine/` and retain the baseline result for comparison. *(All five packages green; counts recorded in [`phase-1-baseline.md`](phase-1-baseline.md).)*
- [x] **Validation checkpoint:** the retained-behavior characterization tests pass on the pre-refactor implementation, the clean-break tests fail for the expected missing behavior, and every planned symbol edit has an impact report.

## Phase 2 — Shared library foundations

**Goal:** Land reusable list parsing and runtime-mutation vocabulary before
Claudine consumes either surface.

- [ ] **[Parallelizable: biscuit-file track]** Add a public `ListFormat` enum in `biscuit-file` for Markdown ordered, Markdown unordered, line-separated, TSV, CSV, space-separated, and scalar inputs, plus an `&str` classifier using the specified precedence.
- [ ] **[Parallelizable: biscuit-file track]** Add a delimiter-aware conversion API that turns a classified string into ordered entries, preserves quoted CSV/TSV delimiters and escaped quotes, normalizes CRLF safely, and drops whitespace-only entries without damaging meaningful Unicode or interior whitespace.
- [ ] **[Parallelizable: biscuit-file track]** Re-export the new API from `biscuit_file`, document its ownership alongside `DataFormat`/`FileType`, and add focused unit tests for every format, ambiguous precedence, quoted fields, CRLF, Unicode, scalar, and whitespace-only input.
- [ ] **[Parallelizable: Darkmatter track]** Add `set(key, value)` to Darkmatter's side-effect descriptor/catalog and implement a reusable top-level in-memory map mutation primitive that returns the prior value, preserves whole-value types, and performs no filesystem write.
- [ ] **[Parallelizable: Darkmatter track]** Extend side-effect errors/tests for invalid/non-top-level keys while leaving Claudine-specific reserved-key policy to Claudine; ensure positional and key/value lifecycle grammar derives the new signature from the catalog.
- [ ] **[Parallelizable: Darkmatter track]** Verify `last(list)` remains present in the expression catalog and evaluator, including empty-list `null` and typed array results; add only missing regression coverage.
- [ ] Run `just test` and `just lint` in both `biscuit-file/` and `darkmatter/`; run formatting in check-only mode if needed and do not run write-mode `cargo fmt`.
- [ ] **Validation checkpoint:** both shared packages compile independently, their focused tests pass, `claudine` can import the new public APIs, and no Claudine behavior has changed yet.

## Phase 3 — Sequence Plus domain model and reserved-state contract

**Goal:** Replace loosely shaped steps with typed states, tasks, groups, sources,
and runtime records without changing execution yet.

- [ ] Split the oversized sequence library module into focused model/normalization/source modules while preserving public re-exports needed by current callers during migration.
- [ ] Define typed representations for authored step state, generated `step_state`, executable task variants, external task references, group definitions/catalog references, source provenance, normalized plans, task outcomes, nested output entries, runtime mutations, and preflight nodes.
- [ ] Enforce exactly one executable field across `prompt`, `shell`, `side_effect`, `group`, and `task`; reject task fields that are meaningless for the chosen action.
- [ ] Centralize the complete reserved-key catalog, covering executable/task keys, generated state keys, and root overlay keys; use it for authored-state collisions, params/setters, and runtime `set` writes.
- [ ] Normalize every strict authored scalar into `{name: <string>}` and generate `id`, `sequence_id`, `is_first`, `is_last`, one-based `index`, and `count`; generate duplicate ids deterministically as `<dasherized-name>-<n>`.
- [ ] Generate one lowercase sequence invocation token with `biscuit_hash::xx_hash_bytes` from a canonical payload containing resolved source path, ordered state ids, high-resolution UTC timestamp, process id, and a process-local monotonic counter; copy it into every state and the root overlay.
- [ ] Build the new overlay with always-present `state`/`sequence_id`/`outputs` and absent-or-present `previous`/`next`; remove the retired root keys entirely.
- [ ] Implement sequence-state string coercion so `state`, `previous`, and `next` render as their `name` only in string context while whole-value expressions retain the typed object and absent neighbors remain `null`.
- [ ] Add typed error variants and terminal rendering for exclusive executable fields, reserved collisions/writes, invalid strict states, and sequence-id/state normalization failures.
- [ ] **Validation checkpoint:** library tests prove deterministic state normalization/id suffixing, overlay precedence, object-preserving whole-value expansion, name proxy coercion, absent neighbors, and rejection of every retired/reserved key.

## Phase 4 — Dynamic and file-backed source resolution

**Goal:** Resolve every supported sequence source into one normalized, immutable
preflight snapshot with provenance-aware strictness.

- [ ] Parse inline arrays, whole-value typed expression arrays, shell-expansion text, raw textual lists, and file references as distinct source variants so typed arrays bypass `ListFormat` classification.
- [ ] Implement the Claudine-only `<file-ref> [-> <offset.path>] [::<operator>(<args>)]` parser with span-aware handling of quotes and interpolation segments; reject trailing/ambiguous text and more than one operator with typed syntax errors.
- [ ] Resolve the untouched reference prefix through `FileReference::resolve_from` using the directory of the document that authored it, including plain, `@`, `!`, `~`, `vault:`, environment-interpolated, spaced, and `@`-containing paths.
- [ ] Load YAML, JSON, JSON5, JSONL, and NDJSON into a common value model; accept the identical top-level `sequence:` document shape for direct and referenced formal sequence files and reject the retired `list:` shape.
- [ ] Apply dot-path offsets only to YAML/JSON/JSON5 and return typed missing-path, non-list, and unsupported-JSONL/NDJSON errors with attempted path and observed type.
- [ ] Implement exactly one `map(from,to)`, `name(from)`, or `template(expr)` operator with item-indexed failures; evaluate templates through Darkmatter with item fields shadowing globals and require non-empty string names.
- [ ] Apply strict normalization to inline/formal sequence lists and lenient foreign-data normalization to expression, shell, arbitrary data-file, JSONL, and NDJSON sources; coerce number/boolean scalars, generate ordinal names for nameless objects, and reject `null` items.
- [ ] Apply formal sequence `template` values before generated fields and validate an optional `$schema` against the normalized state portion only, reporting step index/id and failing property path.
- [ ] Distinguish a static empty list (typed authoring error) from an empty dynamic snapshot (styled `TerminalRenderable` no-op notice and exit `0`).
- [ ] Add source tests covering every acceptance-criteria format, operator, reference family, quoted argument, newline form, strict/lenient boundary, and empty-list behavior.
- [ ] **Validation checkpoint:** all sources resolve exactly once to equivalent normalized plans, file references are document-relative, and malformed/unsupported input produces the specified typed error rather than a generic parse string.

## Phase 5 — Recursive task graph and static preflight

**Goal:** Discover and validate all potentially executable work before launching
any provider, command, or side effect.

- [ ] Build a recursive loader for inline tasks, `kind: task`, `kind: group`, `kind: group-catalog`, and prompt documents; retain each reference's authoring directory for descendant resolution.
- [ ] Parse catalog references as `{group-name}@{file-ref}`, including composed magic references such as `name@@path`, and require a unique named group.
- [ ] Maintain a canonical-path ancestry stack and return the complete typed cycle chain for task/group/prompt cycles while allowing independent branches to reuse immutable parsed documents.
- [ ] Reject nested sequence prompt documents, nested group tasks, group `loop`, and direct execution of group documents during preflight.
- [ ] Resolve group defaults and task overrides for `operation`/`flow`, retain CLI/document locks at their existing higher precedence, and reject fields that cannot affect the selected task type.
- [ ] Traverse conditional branches as potential work and collect every prompt dependency, schema requirement, `$(...)` expansion, lifecycle/setup/teardown shell action, and shell task before execution.
- [ ] Resolve shell bytes with an early-binding-only lookup, reject `outputs` and runtime-mutated/late-binding state, approve the exact resolved bytes once, and store those bytes as the only executable command representation.
- [ ] Aggregate schema/missing-property failures across steps and referenced prompts into the existing single interactive collection pass; ensure provider/model resolution still produces one stable per-task target vector.
- [ ] Detect parallel inline-compose write-back collisions by canonical target path, including collisions with the sequence source document.
- [ ] Treat every preflight failure as abort-all regardless of `fail_fast`, and make dry-run perform this identical preflight.
- [ ] **Validation checkpoint:** fixtures prove complete transitive loading, relative-origin correctness, full cycle chains, nested-work rejection, shell approved-byte parity, aggregate missing properties, and zero child launches on every preflight failure.

## Phase 6 — Runtime layers, `set`, and `outputs`

**Goal:** Establish one invocation-local state cell shared by standalone compose,
serial sequence work, lifecycle actions, and loop rematerialization.

- [ ] Introduce explicit runtime layers with precedence `live source < prompt/task params and sequence user setters < accumulated mutations < reserved overlay`, without mutating process environment or process CWD.
- [ ] Route Darkmatter's `set` action through Claudine's lifecycle executor into the runtime mutation layer; preserve typed values, return the prior value, allow top-level keys only, and reject all reserved keys with a typed error.
- [ ] Make runtime mutations visible to subsequent lifecycle actions, loop iterations, serial tasks, and later sequence steps while keeping `state`/`previous`/`next` as immutable authored views.
- [ ] Initialize `outputs` for every direct compose/inline-compose run as well as sequences; prevent authors, params, setters, and `set` from replacing it.
- [ ] Extend the shared composition execution outcome with captured undecorated final stdout and the existing invalid-byte policy; strip one trailing transport newline while preserving other whitespace.
- [ ] Append one output only after successful teardown: provider final assistant text for prompt tasks, concatenated command stdout for multi-command shell tasks, returned text-or-empty for side effects, and empty text for successful no-output tasks.
- [ ] Expose the correct temporal view to lifecycle hooks: prior entries before/during a run, appended current output on `success`, no new entry on `failure`, and accumulated entries on `finalize`.
- [ ] Add standalone compose/inline-compose tests proving `{{ last(outputs) }}` parity with sequence tasks and failure/finalize timing.
- [ ] **Validation checkpoint:** runtime-layer precedence and immutability tests pass, `set` is visible without disk writes, and captured outputs exclude status rendering, stderr, protocol records, ANSI decoration, and lifecycle messages.

## Phase 7 — Atomic task execution and lifecycle semantics

**Goal:** Execute each task variant through one outcome contract before adding
group or sequence scheduling.

- [ ] Implement a task executor with common setup/primary/teardown orchestration and structured success/failure/interruption, stdout, mutation delta, timing, and primary/secondary diagnostics.
- [ ] Run setup before the primary action; skip primary on setup failure; run teardown exactly once after setup starts, including after primary failure or interruption, with `err` in scope.
- [ ] Make teardown failure convert success to failure, retain a primary action error when both fail, attach teardown errors as secondary diagnostics, and preserve `no_error` as dispatch-only suppression.
- [ ] Implement prompt tasks by JIT-composing the referenced document, selecting inline-compose from that document's configuration, applying JIT-evaluated params with the ratified precedence, and using the preflight target/approval plan.
- [ ] Implement shell tasks with one or more pre-approved commands, a default `30s` timeout per command, typed duration overrides, rejection of `0s`, declaration-ordered stdout concatenation, and platform-neutral spawning.
- [ ] Implement side-effect tasks through the standard positional/key/value lifecycle grammar and capture their textual return value or an empty string.
- [ ] Implement external `task:` as exclusive, immutable expansion of the referenced `kind: task` file; reject referencing-site patches/overrides.
- [ ] Add unit/integration tests for every task variant, params precedence, setup/teardown ordering, timeout behavior, interruption, secondary errors, and output append timing.
- [ ] **Validation checkpoint:** the same task fixture produces the specified outcome standalone and when embedded, and no task executes an unapproved shell byte or resolves a reference from the wrong directory.

## Phase 8 — JIT serial sequence orchestration

**Goal:** Replace eager all-step preparation with static preflight followed by
live-disk, turn-by-turn composition and execution.

- [ ] Retire `phase1c`'s stored `PreparedComposition` vector and replace it with immutable preflight nodes plus the invocation runtime cell.
- [ ] At each step boundary, check interruption, re-read the live source, rebuild effective layers, validate, compose, execute the default body or explicit task, append output, and merge runtime mutations.
- [ ] Preserve default-body execution for steps without executable fields and explicit-task replacement semantics for steps with one; require an executable on every bodyless step.
- [ ] Re-read inline-compose/body/frontmatter changes between sequence steps but never re-resolve the snapshotted step list; demonstrate that live edits affect later composition only at allowed boundaries.
- [ ] Treat JIT composition/late-required failures as step failures governed by sequence `fail_fast`; retain interactive collection in serial TTY contexts and typed errors otherwise.
- [ ] Implement dry-run as full preflight followed by JIT composition of every task against initial state, empty outputs, and no runtime mutations, with no provider launch or write-back.
- [ ] Preserve provider/model selection, shared approval cache, timeouts/guards, lifecycle behavior, exit codes, summary counts, performance collection, and Ctrl+C checks from the existing orchestrator.
- [ ] Add end-to-end tests proving serial visibility of `set`, prior outputs, reserved-overlay precedence, live-disk chaining, continued execution with `fail_fast: false`, and immediate stop with `fail_fast: true`.
- [ ] **Validation checkpoint:** no step is fully composed before its turn, retained sequence CLI tests pass under the new architecture, and interruptions between or during steps return `130` with a partial summary.

## Phase 9 — Serial groups

**Goal:** Add reusable group/task bundles without introducing concurrency yet.

- [ ] Resolve and execute inline groups, `kind: group` files, and named group-catalog entries as sequence tasks with `tasks.len() >= 1`.
- [ ] Enter an invocation-local `group.*` variable scope for group variables and remove it when the group completes; prevent group variables from leaking to later sequence steps.
- [ ] Execute serial group tasks in declaration order against the shared live runtime layers so each task sees prior task mutations and `last(outputs)`.
- [ ] Stop a serial group at its first failed/interrupted task, mark the owning sequence step failed, leave remaining tasks unexecuted, and delegate continuation solely to sequence-level `fail_fast`.
- [ ] Aggregate group/task timings and names into sequence summaries without reintroducing the removed group `output` or task `passthrough` fields.
- [ ] Add group schema/error tests for invalid execution mode, empty tasks, illegal `max_parallel` on serial groups, nested groups, unsupported group `loop`, default/override precedence, and direct group execution.
- [ ] **Validation checkpoint:** inline/file/catalog serial groups are behaviorally equivalent, state/output chaining is declaration ordered, scope is cleaned up, and all blocked constructs fail during preflight.

## Phase 10 — Parallel groups and deterministic merge

**Goal:** Add bounded concurrency with snapshot isolation and deterministic
results independent of completion order.

- [ ] Implement `execution: parallel` scheduling with declaration-order admission and optional `max_parallel >= 1`; absent caps launch all tasks without mutating process-global environment or CWD.
- [ ] Snapshot effective state and prior `outputs` once at group start and give each task an independent runtime mutation/output buffer; do not re-read live files between sibling tasks.
- [ ] Wait for all siblings even after failures, preserve each task's partial stdout slot, and make the group fail if any task fails without canceling successful work.
- [ ] Commit one nested output entry in declaration order after all siblings finish, regardless of stream/completion order; retain a slot for every task.
- [ ] Merge mutation deltas in task declaration order, make later-declared values win on duplicate keys, and emit one warning naming each conflicting key and both tasks.
- [ ] Disable late interactive prompting inside parallel tasks so unresolved required properties become task failures; keep serial interaction unchanged.
- [ ] Fan Ctrl+C out through the existing cross-platform unified wait machinery to all running children, record interrupted outcomes, skip normal mutation/output commit as specified, and return sequence exit `130`.
- [ ] Add deterministic tests using inverted completion delays for scheduling caps, snapshot isolation, mutation merge/conflicts, nested output ordering, all-child completion after failure, no interactivity, and signal fan-out.
- [ ] **Validation checkpoint:** repeated parallel fixtures produce byte-identical summaries/state regardless of completion order, collision checks prevent racing write-backs, and concurrency never changes process-global env/CWD.

## Phase 11 — Concurrent terminal rendering, perf, and logging

**Goal:** Make serial and parallel execution attributable and readable without
breaking stdout/stderr contracts.

- [ ] Add a `TerminalRenderable` task-stream component built from `biscuit-terminal` primitives (prefer `BlockQuote` if its streaming/wrapping contract holds) with header, stable textual label, vertical bar, footer outcome, and duration.
- [ ] Use a fixed cycling palette for parallel tasks and an invisible bar with identical geometry for serial work so layout does not shift between modes.
- [ ] Route all sibling rendering through one synchronized sink that writes complete rendered lines/frames and cannot tear ANSI sequences; preserve arrival-order display without using display order for result ordering.
- [ ] Provide no-color and limited-glyph degradation where stable textual labels carry attribution; keep task/provider data on stdout and headers, footers, status, and warnings on stderr.
- [ ] Extend `SequencePerfAccumulator`, sequence summaries, and per-session logging with group/task hierarchy and timings while preserving per-provider JSONL session isolation.
- [ ] Add renderer tests for narrow widths, wrapping, Unicode, no color, palette cycling, invisible-bar alignment, stdout/stderr split, concurrent writes, and absence of torn escape sequences.
- [ ] Add L2 real-terminal coverage only for behavior a captured/virtual terminal cannot validate; gate OS-specific signal assertions while sharing outcome assertions.
- [ ] **Validation checkpoint:** high-contention rendering remains parseable and attributable, output capture remains undecorated, perf totals reconcile with child timings, and serial output geometry matches parallel output geometry.

## Phase 12 — Full regression, platform, and documentation closure

**Goal:** Prove the complete feature contract and publish the new clean-break
behavior.

- [ ] Expand CLI integration coverage for all acceptance-criteria errors: executable exclusivity, reserved writes, invalid formal states, empty static lists, suffix syntax, offsets/operators, cycles, nesting, write-back collisions, `max_parallel`, timeout, and group-loop rejection.
- [ ] Add cross-package fixtures covering YAML/JSON/JSON5/JSONL/NDJSON, all `ListFormat` forms, expression/shell dynamics, `FileReference` families, schemas/templates, external tasks/groups/catalogs, direct YAML invocation, and document-level versus task-level `prompt`.
- [ ] Audit process spawning, path syntax, CRLF/newline trimming, env/CWD handling, duration parsing, and interruption code for macOS, Windows, and Linux; use platform gates only around genuinely OS-specific assertions.
- [ ] Update `claudine/docs/topics/flow-control/sequences.md` as the primary user contract, prominently documenting the two meanings of `prompt`, reserved keys, source grammar, JIT/live-disk semantics, outputs, groups, concurrency, and the group-loop deferral.
- [ ] Update the Claudine CLI reference, composition docs/examples, README surfaces, `.claude/skills/claudine/architecture.md`, `timeline.md`, and `SKILL.md`; update package dependency docs only if dependencies changed.
- [ ] Review all touched rustdoc/module docs/inline comments for behavior drift, delete the stale eager/live-file comment identified in `current-state.md`, and avoid HOW-narration or literal rendering narration.
- [ ] Run `just test` in `biscuit-file/`, `darkmatter/`, and `claudine/`; run `just test-l2` in `claudine/` for terminal/concurrency behavior and `just lint` in every touched package area.
- [ ] Run read-only formatting checks; do not run `cargo fmt` or `rustfmt` in write mode.
- [ ] **Validation checkpoint:** every acceptance criterion maps to a passing test or updated document, all touched package-area suites/lints pass, and no test relies on Unix-only behavior without an explicit gate.

## Phase 13 — Change-scope audit and handoff

**Goal:** Demonstrate that the implementation changed only the intended
symbols, flows, packages, and public contracts.

- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})`; inspect every affected symbol/execution flow and resolve or document anything outside Sequence Plus, shared list parsing, Darkmatter `set`, output capture, rendering, and reporting.
- [ ] Re-run upstream impact analysis for the final forms of the HIGH-risk resolver/normalization symbols and verify their direct dependents are covered by tests.
- [ ] Review `git diff --check`, the full diff, and working-tree status for accidental formatting, unrelated edits, stale comments, generated artifacts, or missing documentation.
- [ ] Produce a validation matrix linking each specification acceptance criterion to its test file and command, including skipped L2/platform cases and the reason for each skip.
- [ ] Confirm the implementation did not add deprecation aliases, nested sequences/groups, group-loop semantics, checkpoint/resume, or unapproved process-global mutation.
- [ ] Move the feature directory to `_completed` only when the project workflow explicitly authorizes closure; do not commit or move lifecycle directories as an implicit part of implementation.
- [ ] **Final validation checkpoint:** all required commands pass, GitNexus reports only expected blast radius, the acceptance matrix is complete, and the feature is ready for a separate review/commit operation.
