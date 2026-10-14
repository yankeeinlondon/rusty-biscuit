---
total_phases: 5
created: 2026-10-12
phase: 1
agent: claude/sonnet
yolo: true
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-dmls
---

# Task Inputs and Providers Implementation Plan

## Summary and definition of success

Three changes ship together:

1. **`with:` everywhere.** A task's `params:` becomes `with:`, joining the
   `with:` on `proxy`, `retry`, and `prep`. Every `with` shares the shape
   errors (section 3) and one provenance rule (section 2): a relative file
   path resolves from the file that authored it, not from the target or the
   process CWD. `params` is freed and becomes ordinary step data (`seq.params`).
2. **Per-task provider prediction.** Preflight predicts each prompt task's
   provider from its own `agent`/`model`; the gate, review screen, status
   lines, and `env.AGENT`/`env.MODEL` use the provider that launches.
   A sequence whose tasks all name providers runs with no document `agent`.
   The launch-time rebuild (`select_rebuilt_provider`) stays authoritative.
3. **`sequence.yaml` / `sequence-types.yaml`.** DMLS and `md schema validate`
   validate sequence documents, with bounded expansion at three typed
   container levels.

Success means all 10 acceptance criteria have an observable test, the sweep
leaves no maintained task-input `params:`, docs and the `claudine` skill state
the new rules, and `just test` and `just lint` are green in `claudine/` and (for
schema fixtures) `darkmatter/`, with `just test-l2` green for the review screen.
No new CI cell. The terminal state is *implementation complete, ready for
review*; the author closes the feature. Never run `cargo fmt`, commit, or move
the feature to a lifecycle directory.

Code landmarks (verify in Wave 1; they are starting points, not guarantees):

| Concern | Location |
|---|---|
| Task parser / layering | `claudine/lib/src/composition/sequence/task/mod.rs` (`layered_overrides`), `sequence/reserved.rs`, `sequence/preflight/` |
| Task runner | `claudine/cli/src/commands/wrap/sequence/task_run.rs` |
| Sequence entry, review, reporting | `claudine/cli/src/commands/wrap/sequence/{mod,review,report,jit,phase1c}.rs` |
| Launch-time provider rebuild | `claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs`, `loop_control.rs` |
| Lifecycle `with` (proxy/retry/prep) | `claudine/lib/src/composition/lifecycle/{executor,parse}.rs`, `composition/error/mod.rs` |
| Schemas | `claudine/schemas/{claudine,lifecycle}.yaml`, `claudine/docs/schemas/`, DMLS `darkmatter/dmls/src/{overlay,providers/frontmatter}` |
| Docs | `claudine/docs/topics/{flow-control/sequences.md,flow-control/flow-control-reference.md,composition.md,schemas.md}`, `claudine/docs/cli/sequence.md`, `.claude/skills/claudine/` |

## Phase 1 — Establish Contracts and Reduce Risk

### Necessary Rules

These are rulings the specification leaves implicit or contradicts. Items
marked **ASK** need the author's answer before the dependent wave starts; the
plan states a default so work is not blocked, and Wave 1 records the outcome.

1. **Dependency gate (ASK).** The spec depends on `2026-10-21-claudine-schemas`
   and `2026-10-25-loops-and-flow-control` and relates to
   `2026-10-24-sequence-containers`, `2026-10-14-union-type-constraints`, and
   `2026-10-13-named-type-presence-and-local-names`. At planning time the first
   three are `finalized-spec` with no implementation log. Section 6 imports
   `initialize-stack`/`stack`/`loop` from `lifecycle.yaml` (currently the only
   partial) and needs `cancelled`, `loop:`, `gate:`, and `fail_fast` to exist in
   the parser. *Default:* Phases 2 and 3 may start against the current parser
   (they only need the `with`/`agent` seams); **Phase 4 does not start until
   the schema and loop specs are implemented**. Wave 1 confirms which landed.
2. **Error-family names (ASK).** Section 3 renames `LifecycleProxyWith*` to
   `LifecycleWith*`, unless `2026-10-20-lifecycle-grammar` already did. *Default:*
   grep first; keep whichever family exists; otherwise rename in this change
   across `composition/error/{mod,tests,render}`, executor tests, and CLI
   `error_routing`. Rename touches no wire/diagnostic *codes* unless the
   existing codes embed the old name; if they do, record that in the log.
3. **`with` accepted only on `prompt:` tasks.** Rejected on `shell:`,
   `side_effect:`, `group:`, `task:`, and nested `sequence:` by the existing
   task-option error naming `with`. A `kind: task` file carries its own `with`;
   `task: file` plus a sibling `with` is rejected (external tasks are immutable).
4. **Precedence is runtime layering preserved.** Task chain, lowest→highest:
   document frontmatter → `with` → caller setters → runtime `set` → generated
   bindings. Caller setters do **not** pin the provider against later runtime
   `set`; only an explicit `--<provider>` pins. Do not "fix" this.
5. **Anchoring is schema-selected only.** Only values the target schema marks
   `file` are anchored; strings that look like paths stay strings. Reuse
   `biscuit-file` `FileReference` and the captured source context (repo roots,
   home, env). Never join a directory to a string and never re-read process CWD.
   Ambiguous file/string union arms stay raw for validation.
6. **Origin travels with the value.** Each `with` value stores raw data +
   origin through containers, retry, and proxy handoffs. Resolved strings are
   not re-templated. An expression that copies an already-anchored value keeps
   its identity; a newly produced relative file string takes the `with`
   entry's authoring origin. Mapping evaluation is atomic: any failing entry
   installs no overlay and launches nothing.
7. **Intentional behavior change.** `proxy.with` relative paths move from
   target-relative to authoring-file-relative. The migration sweep must find
   and rewrite shipped usages (`prompts/`, fixtures, docs). Retry retention is
   unchanged — only origin changes.
8. **Prediction is read-only and purity-checked.** It runs after the root's
   shell-free initialization; it runs no descendant lifecycle event, `prep`,
   shell substitution, or side-effecting expression, composes no bodies, and
   contacts no network. Classification is exactly **predicted / decided at
   launch / missing a provider**. A false `when` is still checked (existing
   approval contract). Dynamic hints (reading `seq.outputs`, `loop.*`,
   `group.*`, `current.*`, runtime-mutated values, or the selected
   `env.AGENT`/`env.MODEL`) are *decided at launch*, never replaced by a
   picker result. A hint depending on its own `env.AGENT`/`env.MODEL` is a
   source-located selection error (circular).
9. **Gate rule.** The review screen opens (terminal) or the run fails
   `AgentResolutionFailed` (non-terminal) only when some provider-launching task
   is *missing a provider* **and** no enclosing sequence document's `agent`
   resolves (nested file first, then outward to root). Shell/side-effect-only
   graphs need no provider. A group has no `agent`; a body step uses its own
   document's hints.
10. **Review-screen state keyed by identity + ancestry.** Choices are stored by
    task identity and container ancestry (so repeated nested names do not
    collide), are fallbacks (not authored `agent` setters, so a later proxy
    target can still choose its own), and own-provider tasks are read-only rows.
11. **One resolver.** Use the canonical provider resolver (installed filtering,
    favorites, ambiguity, scalar *and* list hints, non-terminal errors) and the
    shared model resolver. No second picker; never select an unavailable list
    member. Model order: `--model` → provider-specific model env → captured
    generic `MODEL` → task `model` → compatible planned fallback → provider
    default. An inherited/review-selected model is eligible only for the
    provider it was resolved against; clear stale `MODEL` overlays when none
    resolves.
12. **Reporting uses actual identity.** Record each attempt's actual provider
    identity; a single completion summary is labeled with the final attempt's
    provider; skipped/blocked tasks and tasks that launched no provider claim
    none. Composition-time `env.AGENT`/`env.MODEL` are refreshed at the existing
    preparation boundary so they match the child environment.
13. **Sequence schema limits (per spec).** Bounded expansion: distinct
    task/group/sequence types at three typed container levels, then an untyped
    boundary (valid deeper documents stay valid). Parser-only rules are *not*
    encoded (`while`/`until` exclusivity, `gate` beside loose verbs, event keys
    only on inline-sequence tasks, reserved step keys, unique leg names).
    Reuse the lifecycle `loop`/`stack`/`initialize-stack` types; add
    `cancelled-stack` there; do not fork a second `loop`. Upgrade
    `claudine.yaml`'s loose `sequence` declaration to the same imported type.
14. **Schema file ownership (ASK).** The spec names `claudine/schemas/`, but
    copies of `claudine.yaml` exist at `claudine/docs/schemas/` and
    `darkmatter/docs/schemas/` (the latter modified in the working tree by
    another feature). *Default:* `claudine/schemas/` is canonical for the new
    files; Wave 1 determines whether the docs copies are generated/mirrored and
    whether the embedded launch-validation bundle (and its import-resolution
    path) is listed in code or in a build/`just` step. Coordinate with the
    in-flight `2026-10-16-core-schema-types` edits rather than overwriting them.
15. **Tests.** All L1 (plus the review-screen L2), portable to macOS, Linux,
    native Windows, WSL2; Test Toolkit and `CliProcessFixture` with private
    CWD/home; stub providers with stderr redirected; `PLAYA_DRY_RUN` with a
    private spool; no real provider launches and no window focus. Register new
    modules in consolidated L1 roots. Remember the `os` skill for any path
    comparison and `#[cfg(windows)]` code.
16. **Out of scope** (do not drift): container semantics, loop/`break`/`prep`/
    `when:` semantics, the lifecycle/action type libraries, recursive schemas,
    Darkmatter lifecycle ownership.

### Spike (runs once, before Wave 4)

- [ ] **Bounded-expansion schema spike** — one host, in-memory/temp-dir.
  Write a throwaway three-level `task → group → sequence` expansion with
  `steps` unions (`step[]`, list-form unions) and `loop@./lifecycle.yaml`
  imports, and load it through the current schema engine and the DMLS overlay.
  Confirm: no cycle rejection, bare-local-name references resolve, `step[]` list
  of union is accepted, load time of one document is reasonable (a quick
  sample, not a benchmark), and the untyped boundary accepts deeper nesting.
  Record the working construction or the concrete blocker in the log. No OS
  fleet and no re-check in later phases. The spec's own figures already settle
  everything else (no performance spike for prediction; it is a read-only
  inspection whose cost the spec does not call out).

*Candidates considered and dropped:* a provider-prediction performance spike
(no cost is named in the spec) and a cross-OS path-anchoring spike (covered by
the L1 provenance tests on CI's per-event OS matrix and the `os` skill).
*Wider measurement for the author to decide:* none requested.

### Input Robustness Matrix

Applies because the work changes readers of YAML frontmatter configuration:
task `with`, lifecycle `with` (shape errors), task `agent`/`model` hints, and
the `sequence` trigger key. It does not apply to the schema *writers* or to
Markdown prose. One data-driven driver per format (YAML frontmatter) walks the
table from a real fixture (a copied shipped sequence + the two lifecycle
fixtures), one edit per cell, each column with a positive control (unedited
fixture → success). Assertions go through public results (`claudine sequence`
exit/stderr via `CliProcessFixture`, `md schema validate`), not parser returns.

| Shape | `with` (task/proxy/retry/prep) | `agent` / `model` hint | `sequence` trigger key |
|---|---|---|---|
| absent | no overlay; document frontmatter wins | task is *missing a provider* → enclosing doc `agent` → gate | trigger does not activate |
| explicit null (`with:` / `with: null`) | `LifecycleWithNotMapping` (not conflated with absent) | typed hint error; not "no hint" | does not activate (per spec) |
| wrong type, whole field (`with: 42`, `agent: 7`) | `LifecycleWithNotMapping` / typed hint error | typed hint error naming task + source | `42`/mapping activates; payload validation fails |
| wrong type, one element (`agent: [codex, 7]`) | n/a | typed error; member not filtered silently | `[…]` handled by `unknown[]` arm |
| wrong type, every element (`agent: [7]`) | n/a | typed error; not read as empty list | n/a |
| empty (`with: {}`, `agent: []`) | equals no `with` | empty list → typed non-terminal rule, not "absent" | `sequence: []` activates and validates |
| duplicate key (`with:` twice, key twice inside `with`) | rejected at load, before a mapping loses it | rejected at load | rejected at load |
| trailing / invalid content | rejected, source-located | rejected, source-located | rejected, source-located |
| special rows | `with: "{{ payload }}"` → `LifecycleWithWholeMapping` (priority over NotMapping); key with `{{ }}` → `LifecycleWithDynamicKey`; value failing evaluation → `LifecycleWithEvaluationFailed`, no partial overlay, no launch; reserved generated bindings stay protected | dynamic hint → *decided at launch*, not a gate; hint reading own `env.AGENT` → circular-selection error | missing/null `sequence` never activates |

Smell grep before declaring done: `#[serde(default)]` or `Option<T>`/`unwrap_or_default()`/`filter_map(.. as_str())`/`.ok()` on the changed `with`, `agent`, `model` parses; any hit must be justified in the log.

### Wave 1 — Inventory and Dependency Check

- [ ] **Dependency check** — Read the status/implementation logs of
  `2026-10-20-lifecycle-grammar`, `2026-10-21-claudine-schemas`,
  `2026-10-24-sequence-containers`, `2026-10-25-loops-and-flow-control`,
  `2026-10-23-shell-template-approval`. Record which landed and the resulting
  error-family names (Rule 2). Ask the author on Rules 1, 2, 14 if unresolved.
- [ ] **Contract inventory** — Map acceptance criteria 1–10 to existing tests
  and needed additions. Record in `implementation-log.md` (beside this plan):
  every `params` reader/writer in the task parser, reserved-key lists
  (`reserved.rs` has `params` in two lists), runner, and preflight; every
  `with` evaluator for proxy/retry/prep and how each resolves file values
  today; the planned-target consumers (gate, review, status line,
  `step_env_overrides`, `apply_target_env_overrides`).
- [ ] **Migration inventory** — *Parallel with Contract inventory.* Search
  `prompts/` (incl. `review-loop.md`), `claudine/prompts/`, fixtures, tests,
  `docs/`, `.claude/skills/` for task-input `params:`, `proxy|retry|prep`
  `with:` relying on target-relative paths, and the old doc rule. Classify each
  hit: convert, preserve (unrelated data / historical spec snapshots / timeline
  evidence), or rewrite-relative. Exclude `claudine/docs/research/**`, protocol
  fixtures (`stream/**`), and `features/`/`fixes/_completed` snapshots by
  inspection.
- [ ] **Test baseline** — Run `just test` and `just lint` in `claudine/` and the
  darkmatter schema/DMLS suites; record failures separately from intended
  changes. Note unrelated working-tree changes without touching them.
- [ ] **Skill load** — Read `claudine`, `darkmatter`, `rust-testing`, `just`,
  `os`, `biscuit-file` skills before delegating.

Checkpoint: log lists dependency state, rulings answered, exact files per
phase, and baseline failures.

### Wave 2 — Spike

- [ ] **Run the bounded-expansion spike** (above). Prerequisite: Wave 1 and
  dependency-gate answer for Rule 1. If dependencies have not landed, run it
  against the current engine with stand-in lifecycle types and note that.

Checkpoint: construction recorded, or a blocker raised to the author before
Phase 4 begins.

## Phase 2 — `with:` Rename, Provenance, and Shape Errors

Parallel with Phase 4's schema authoring only after Wave 2; file ownership is
disjoint (Phases 2/3 own `claudine/lib` + `claudine/cli`; Phase 4 owns schemas,
fixtures, and DMLS).

### Wave 3 — Rename and error family

- [ ] **Task rename** — In the task parser, `layered_overrides`, the task
  runner, and the reserved-key catalog, replace `params` with `with`. Make
  `with`/`when` reserved, remove `params` from both lists in `reserved.rs`,
  and keep `params` readable as `seq.params` step data. Update doc comments in
  the same change.
- [ ] **Acceptance of `with`** — Accept only on `prompt:` tasks; reject on
  `shell`, `side_effect`, `group`, `task`, nested `sequence` with the existing
  task-option error naming `with`; reject `task:` + sibling `with`.
- [ ] **Error family** — Apply Rule 2. Raise `NotMapping`, `DynamicKey`,
  `WholeMapping`, `EvaluationFailed` for task `with` with task source
  locations, whole-mapping taking priority over non-mapping, `with: {}` equal
  to no `with`; keep typing rules (single span keeps type; text with spans →
  string; YAML types preserved). Share one evaluator between task and lifecycle
  `with` instead of duplicating it.
- [ ] **Tests** — Acceptance 1 and 3 (task side) plus the matrix `with` column.

Checkpoint: `just test` in `claudine/` green; `params:` on a step is plain data.

### Wave 4 — Provenance

- [ ] **Origin carrier** — Attach an authoring-file origin to each `with` value
  (inline task → sequence doc; task/group file → that file; proxy/retry/prep →
  the document holding the lifecycle event), parallel to how caller setters
  carry launch origin. Materialize against the target schema via
  `FileReference` (eager file resolves from origin; lazy binds first candidate;
  lazy remote stays URL; recursive lazy keeps its typed refusal).
- [ ] **Preservation** — Keep raw data + origin through nested containers,
  retry, and proxy chains, including when a chained target's schema changes;
  keep identity for copied anchored values; atomic evaluation; re-evaluate task
  `with` at each task turn using a concurrent group's start snapshot; evaluate a
  lifecycle `with` once when the directive fires.
- [ ] **Tests (table-driven)** — Acceptance 2: task `with`, `proxy.with`,
  `retry.with`, `prep.with` each from its authoring file; a `kind: task` file
  and a proxy target in another directory; caller setter from launch dir and
  above `with`; lazy-missing, eager-missing, remote lazy, ambiguous file/string
  arms, forwarded anchored value, chained handoff with schema change, and no
  partial overlay on evaluation error. Add lifecycle shape-error tests for
  proxy/retry/prep (acceptance 3, second half).

Checkpoint: provenance table green on this host; the `os` skill consulted for
any path comparison. Windows/WSL2 are proven by CI per the event schedule.

## Phase 3 — Per-Task Provider Prediction

Depends on Phase 2 (`with.agent` must reach the task document).

### Wave 5 — Prediction core

- [ ] **Prediction pass** — Add a read-only preflight inspection (lib side,
  `sequence/preflight/`) that, for each provider-launching prompt task, evaluates
  `agent`/`model` in launch order in the shell-approval early-binding context,
  following nested sequences, groups, and external task files with their source
  contexts. Output per task: `Predicted{provider, model?}`, `DecidedAtLaunch`,
  or `MissingProvider`, keyed by task identity + container ancestry.
- [ ] **Purity guard** — Ensure no lifecycle action, `prep`, shell
  substitution, body composition, or model-catalog network refresh runs; add a
  stub that fails the test if invoked (acceptance 8).
- [ ] **Early failure** — Typed error naming the task, source, and enclosing
  container names for an invalid/uninstalled predicted provider, before any
  launch, including false-`when` tasks (acceptance 6). Static higher-precedence
  override makes a lower dynamic hint irrelevant.
- [ ] **Resolver reuse** — Route scalar/list hints through the canonical
  provider resolver and the shared model resolver (Rule 11).

Checkpoint: unit tests for each classification state and for a shell-only graph.

### Wave 6 — Gate, review, launch, reporting

*Wave 6 tasks share `sequence/mod.rs` and `review.rs`; one owner integrates.*

- [ ] **Gate** — Replace the single planned-target gate in the sequence entry
  point with the per-task rule (Rule 9); enclosing-document `agent` lookup nested
  first, outward to root.
- [ ] **Review screen** — Read-only rows for predicted tasks, editable rows only
  for *missing* tasks, selections stored by identity + ancestry as fallbacks;
  decided-at-launch tasks are shown and never overwritten by a picker value.
  L2 test (acceptance 7) with the terminal harness, no window focus.
- [ ] **Launch** — Ensure the launch-time rebuild uses a review fallback only
  below the task's own hint, drops an incompatible inherited model on a
  provider switch, clears stale `MODEL`, and fails a dynamic unresolved/
  unavailable hint at preparation *before* spawning; detect self-referential
  `env.AGENT`/`env.MODEL` hints.
- [ ] **Reporting** — Preflight/dry-run rows show prediction or "decided at
  launch"; status lines, completion lines, and reports use actual launch
  identity; per-attempt records for retry/proxy; refresh `env.AGENT`/`env.MODEL`
  at the preparation boundary so composition and the child agree.
- [ ] **Tests** — Acceptance 4 (looping sequence, stub providers, no document
  `agent`, each stub sees only its own prompts every pass, status lines name
  the stub), 5 (table over every precedence row incl. caller `agent=gemini`
  over `with.agent`, nested-file sequence `agent`, runtime `set` over caller
  setter, scalar/list hints, model env precedence, provider-switch model drop),
  6, 8 (seq.outputs, runtime-mutated, concurrent-leg snapshot, retry/proxy
  identities, skipped/blocked tasks claim none, unavailable dynamic hint fails
  pre-spawn), plus the `agent`/`model` matrix column.

Checkpoint: `just test` in `claudine/`; `just test-l2` for the review screen;
`just lint`.

## Phase 4 — `sequence.yaml` Schemas

Prerequisite: Rule 1 (dependency specs implemented) and the Wave 2 spike.
Concurrent with Phases 2–3 *after* Wave 2 where file ownership permits.

### Wave 7 — Schema authoring

- [ ] **Lifecycle additions** — Extend `lifecycle.yaml` with `cancelled-stack`
  (container contract's restricted actions); reuse `loop`, `stack`,
  `initialize-stack`, including the loose verb keys from the loop spec.
- [ ] **Type library** — Create `claudine/schemas/sequence-types.yaml`
  (`kind: schema`): `steps`, `step`, `task`, `group`, and the bounded-expansion
  variants for three typed levels plus the untyped boundary, using bare local
  names and YAML-list unions. Omit removed keys (`group.execution`, root
  `fail_fast`); `<string>: any` keeps authored step data (so `params` is valid
  data while `setup`/`teardown` remain parser errors).
- [ ] **Trigger** — Create `claudine/schemas/sequence.yaml`
  (`kind: trigger-schema`) matching `sequence: [string(required), number,
  boolean, object, "unknown[]"]`; payload types the root including `template`,
  `loop`, and the lifecycle events.
- [ ] **claudine.yaml upgrade** — Replace the loose `sequence` declaration with
  the imported type so both triggers agree; keep `agent`, `model`, and common
  fields, with no document-level provider requirement.
- [ ] **Embedding and install** — Add both files to the embedded
  launch-validation bundle and disk discovery; verify every relative import
  resolves with no checkout; update install instructions to copy the whole
  schema bundle. Sync any mirrored copies determined in Wave 1.

### Wave 8 — Fixtures and tests

*May split across subagents: Claudine fixtures vs DMLS/darkmatter tests; each
owns disjoint files.*

- [ ] **Example fixtures** — Save every sequence example from
  `2026-10-24-sequence-containers` and `2026-10-25-loops-and-flow-control` as
  valid fixtures; add invalid fixtures for `execution`, `max_paralel:` on a
  group object, and `fail_fast` at the root; valid + invalid fixture at each of
  the three typed levels; a valid deeper fixture for the untyped boundary;
  `sequence: 42` and mapping → activates and fails payload validation; missing/
  null `sequence` → no activation; both triggers active with malformed task
  `with` still fails; unknown step key is accepted step data.
- [ ] **Tests** — Acceptance 9 through `md schema triggers`, `md schema validate`
  (`CliProcessFixture`), and DMLS L1; installed-launch validation test without
  checkout files; trigger-column matrix cells.
- [ ] **Docs link** — Add `sequence.yaml` to the schema topic in
  `claudine/docs/` (boundary explained: typed levels, then untyped; parser still
  allows unlimited nesting).

Checkpoint: `just test` and `just lint` green in `claudine/` and `darkmatter/`
(DMLS included).

## Phase 5 — Sweep, Docs, and Closeout

### Wave 9 — Sweep and documentation

*Sweep and docs can run in parallel (disjoint files).*

- [ ] **Sweep** — Apply the Wave 1 migration inventory: convert task-input
  `params:` to `with:` in maintained prompts (`prompts/review-loop.md`),
  fixtures, tests, docs, and skills; rewrite relative `proxy|retry|prep` `with`
  paths to be authoring-file-relative; preserve unrelated `params` data,
  historical snapshots, and timeline evidence. Shipped-prompt tests use the new
  grammar (prompt tests stay opt-in; frozen copies stay frozen).
- [ ] **Docs** — Update `sequences.md` (reserved keys list, "Keys you cannot
  author as state", "Phase 1 — Static preflight": per-task prediction, all-named
  tasks run without document `agent`/review screen, uninstalled provider fails
  before any launch, status lines name the provider that ran),
  `flow-control-reference.md` ("Passing values with `with:`"), `composition.md`
  ("Caller File Provenance and Materialization" — the one rule), and
  `docs/cli/sequence.md`. Each page leads with what the reader can do, gives a
  compact example per rule, and uses a Mermaid diagram for the provider
  decision flow. Docs state behavior, never link to or name this feature.
  Describe the intentional `proxy.with` change.
- [ ] **Skills and amendments** — Update the `claudine` skill; remove the
  2026-09-27 amendments file if it still exists beside the old sequence spec
  (verify it exists first); update `claudine/docs/dependencies.md` only if
  crates changed.
- [ ] **Comment drift pass** — Every behavior-changing symbol gets its `///`/
  `//` comments reviewed; report drift found and how it was resolved.

### Wave 10 — Final validation

- [ ] `just test`, `just test-l2` (review screen), `just lint` in `claudine/`;
  darkmatter schema/DMLS suites; confirm no new CI cell
  (`just ci-local --plan`).
- [ ] Walk acceptance criteria 1–10 against test names in the log; run the
  robustness smell greps; confirm `grep` finds no maintained task-input
  `params:`.
- [ ] Set the spec/plan status per the repo's convention (*implementation
  complete, ready for review*); do not run `just complete`, move directories, or
  commit.

Checkpoint: log records outcomes, departures from the spec (docs corrected, spec
left as-is), and any ASK items still open.
