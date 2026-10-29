---
area: claudine
canonical_docs:
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/topics/frontmatter-properties.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - biscuit-tui/docs/components/input_table.md
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 4
human_review: false
pipeline_step: 16
depends-on:
    - 2026-10-21-claudine-schemas
    - 2026-10-25-loops-and-flow-control
related:
    - 2026-10-24-sequence-containers
    - 2026-10-14-union-type-constraints
    - 2026-10-13-named-type-presence-and-local-names
implemented: true
implemented_by: "claude/opus"
completed: true
---

# Task inputs and providers

Three things a sequence author touches on every task are rough today. Values
for the document a task runs are passed with `params:`, while the same idea
is spelled `with:` on `proxy`, `retry`, and `prep`, and the two resolve a
relative file path from different places. A sequence document with no `agent`
refuses to run even when every task names its own provider, the review
screen discards choices for tasks that name one, and status lines name the
wrong provider. And sequence documents get no editor validation, because
there is no sequence schema.

This spec renames `params:` to `with:` with one file-provenance rule for every
`with`, predicts each task's provider at preflight so a sequence of
self-describing tasks just runs, and adds the `sequence.yaml` schema so DMLS
validates sequence documents in the grammar the preceding specs built.

It absorbs the 2026-09-27 amendments file that lived beside the old sequence
spec (provider probe correction, provider selection per task, its acceptance
criteria, the `params` → `with` rename, and the topic-page note); that file is
removed in this change.

## Decided

Decided 2026-10-10 by the owner; the provider-selection design was agreed
2026-09-27 and is merged as written.

### 1. Task `params:` becomes `with:`

A task's `with:` carries frontmatter values for the document the task is
about to run. It is the same word, and the same mechanism, as the `with:` on
`proxy`, `retry`, and `prep`, so Claudine has one word for "frontmatter
values for the document I am about to run".

```yaml
sequence:
    - name: review
      prompt: "@prompts/_reviews/feature-review.md"
      with:
          spec: "{{ spec }}"
          notes: ./notes/review-hints.md    # resolves from this sequence file
```

| | `proxy` / `retry` / `prep` `with` | task `with` |
|---|---|---|
| Target | the target document's top-level frontmatter | the task document's top-level frontmatter |
| Precedence, lowest → highest | target frontmatter → `with` → caller setters (runtime layering retained) | task document frontmatter → `with` → caller setters → runtime mutations → generated bindings |
| Evaluation | once, in the source, when the directive fires; the target receives resolved data | at the task's turn, in the sequence's live state; the child receives resolved data |

- **Hard rename, and `params` is freed.** `with` is the task key. `params` is
  no longer a reserved task key: on a step it is ordinary step data, read as
  `seq.params`. The migration sweep converts task-input `params:` in maintained code,
  prompts, fixtures, docs, and skills; unrelated data named `params` and
  historical specification snapshots are preserved.
- **Where it is accepted.** `with` is accepted on `prompt:` tasks. It is not an
  overlay for `shell:`, `side_effect:`, `group:`, `task:`, or nested `sequence:`
  entries, and is rejected there by the task-option error naming `with`.
  External task references remain immutable: put `with` inside the referenced
  task file, not beside `task:`. Body steps retain their existing option rules.
- **Caller setters outrank `with`.** `claudine sequence doc.md agent=codex`
  overrides each task's authored `agent`, including `with.agent`. This does
  not pin the provider against later runtime `set` mutations: the existing
  task override order is `with` → caller setters → runtime mutations →
  generated container bindings. An explicit provider flag pins every launch.
  This spec preserves runtime layering rather than changing it during a rename.

### 2. One provenance rule for every `with`

A relative file path in any `with:` — a task's, or a `proxy`, `retry`, or
`prep` directive's — resolves from the **file that authored it**: the
sequence document for an inline task, the `kind: task` or `kind: group` file
for a task written there, and the document whose lifecycle event holds the
directive for `proxy`/`retry`/`prep`. Each `with` value carries that origin
the way a caller setter carries its launch origin, and is materialized against
the target's schema the same way (an eager `file` resolves from the origin; a
lazy `file` binds to its first candidate from the origin). The value is still
not caller input: caller setters keep their own launch origin and their place
above `with`.

Only schema-selected file values are anchored; ordinary strings that happen
to look like paths remain strings. Reuse `biscuit-file`'s `FileReference`
resolver and the captured source context, including repository roots, home,
and environment; do not join a directory to a string or recapture process
CWD. For schema unions, use the existing selected-arm or unanimous-file-type
rule; ambiguous file/string arms remain raw for validation. Lazy remote values
remain URLs, recursive lazy references keep their existing typed refusal,
and body presentation remains separate from native path identity.

Preserve each winning value's raw data and origin through nested containers,
retry, and proxy handoffs. Re-evaluate task `with` at each task turn, using a
concurrent group's shared start snapshot; a fired directive evaluates its
mapping once and retains the resolved data for its target's fresh reads.
Do not re-interpret resolved strings as templates. If an expression copies an
already anchored file value, keep its identity; a newly produced relative file
string uses the `with` entry's authoring origin. Mapping evaluation is atomic:
a failing entry installs no partial overlay and launches no target. Retry
retention follows the lifecycle grammar contract; this feature changes the
origin, not which attempts retain an overlay.

> **Reader note — intentional behavior change.** This keeps the current task
> `params` behavior and changes `proxy.with`, whose
> values were previously resolved as the target's own frontmatter. Existing
> lifecycle overlays that relied on target-relative paths must be rewritten
> relative to their authoring file. The migration covers those usages and the
> documentation's old rule; values retain their source ownership and are never
> relabeled as caller input.

```yaml
# prompts/router.md
success:
    - proxy:
          target: "@prompts/next.md"
          with:
              plan: ./plans/today.md     # prompts/plans/today.md, wherever next.md lives
```

### 3. `with:` follows `proxy`'s shape rules

| Mistake | Error |
|---|---|
| `with:` is not a mapping | `LifecycleWithNotMapping` |
| a key contains `{{ … }}` | `LifecycleWithDynamicKey` |
| the whole mapping is one span, `with: "{{ payload }}"` | `LifecycleWithWholeMapping` |
| a value fails to evaluate | `LifecycleWithEvaluationFailed` |

These are the `LifecycleProxyWith*` errors renamed, because `with` is no
longer proxy-specific; task `with` raises the same errors with the task's
source location. (If 2026-10-20-lifecycle-grammar already renamed the family,
its names are kept and task `with` uses them.) The whole-mapping error takes
priority over the generic non-mapping error. `with: {}` equals no `with`.
Values follow the existing typing: a value that is exactly one span keeps the
expression's type; text with spans becomes a string; YAML scalars, arrays, and
objects keep their authored types.

### 4. Reserved task keys

`with` and `when` are reserved task keys: a step whose data uses either is
read as the task option, never as step data. `params` leaves the list. The
"Keys you cannot author as state" list in `sequences.md` is updated; a
sequence that used `with` as step data now gets task-option behavior (this is
stated in the docs).

### 5. Provider selection per task

#### What happens today

A provider is chosen twice:

1. **Before the run**, one target is planned from the **sequence document's**
   `agent`/`model` and the caller's flags and setters
   in Claudine CLI's [sequence entry point](../../../cli/src/commands/wrap/sequence/mod.rs). If that
   does not resolve, a terminal gets the review screen and anything else gets
   `AgentResolutionFailed`.
2. **At every launch**, the provider is rebuilt from the composed frontmatter
   of the document about to run (Claudine CLI's
   [select_rebuilt_provider](../../../cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs),
   which selects the provider for a refreshed target):
   explicit flag, then that document's `agent` (its frontmatter, then `with`,
   then caller setters), then the planned target. This rebuild is deliberate:
   it lets a `retry` or `proxy` target change `agent:`.

So each task already runs on its own `agent`; the planned target is used only
for tasks whose document names none. What is wrong is everything built on the
planned target:

- **The gate.** A sequence document with no `agent` blocks the run, or forces
  the review screen, even when every provider-launching task names one.
  `prompts/review-loop.md` hits this.
- **The review screen.** Every row starts on the planned default, and a choice
  made for a task whose document names an `agent` is silently discarded at
  launch.
- **Reporting.** The step status line and the `env.AGENT`/`env.MODEL` values the
  step's prompt is composed with (Claudine CLI's
  [step_env_overrides](../../../cli/src/commands/wrap/sequence/jit.rs),
  which supplies a step's composition environment) come from the planned target,
  not the provider that launched. The launched process's own
  `AGENT`/`MODEL`/`YOLO` are already correct (set by Claudine CLI's
  [apply_target_env_overrides](../../../cli/src/commands/wrap/harness_orch/loop_control.rs)).
- **Late failure.** A task naming a provider that is not installed is refused
  only at its turn, after earlier tasks have run.

#### Requirements

- **Predict each task's provider during preflight.** For every
  provider-launching task, preflight evaluates the task's `agent`/`model` in the
  launch order, in the same early-binding context shell approval uses. The
  launch-time rebuild stays authoritative; the prediction exists to gate,
  report, and fail early.
  - If a task's `agent` cannot be evaluated early (it reads `seq.outputs`,
    `loop.*`, `group.*`, or a runtime-mutated value), the task is reported as
    **decided at launch**. It does not gate the run, and the launch-time rule
    applies.
- **Gate per task.** The review screen opens, or the run fails without a
  terminal, only when some provider-launching task has no predictable provider
  of its own **and** no enclosing sequence document's `agent` resolves. A task
  whose predicted provider is invalid or not installed fails preflight with a
  typed error naming the task, before any task launches.
- **An honest review screen.** Tasks with a predicted provider of their own are
  read-only rows showing it. Only the remaining tasks are editable, and a choice
  made there is what launches.
- **Report what launched.** Preflight and dry-run rows show a prediction or
  **decided at launch**. Completion lines and sequence reports use the actual
  launch identity. Refresh selection at the existing preparation boundary so
  composition-time `env.AGENT`/`env.MODEL` and the child environment agree.
  Retries and proxy targets may change providers; record each attempt's actual
  identity and label any single completion summary with the final attempt's
  provider. A task that launched no provider must not be reported as having
  used its prediction.

Provider precedence for a prompt task, highest first (model selection is
separate, as described below):

| Source | Scope |
|---|---|
| `--<provider>` / `--provider` | every task |
| the task document's effective `agent`: frontmatter, then `with`, caller setters, and runtime mutations | that task |
| a review-screen choice for a task with no own hint, otherwise the nearest enclosing sequence document's `agent` (nested file first, then outward to the root) | that task |
| favorite / picker | that task |

Preflight classifies tasks as **predicted**, **decided at launch**, or
**missing a provider**. Only the last state needs an editable review row or
fails without a terminal. Dynamic hints are not missing hints: retain them
for launch-time evaluation rather than replacing them with a picker result.
A graph containing only shell and side-effect tasks needs no provider.

Prediction is a read-only inspection after the root's shell-free initialization.
It does not execute descendant lifecycle events, `prep`, shell substitutions,
or side-effecting expressions, compose prompt bodies, or contact providers to
refresh model catalogs. It follows nested sequences, groups, and external
task files using their source contexts. Inspect only selection inputs and the
`with` entries they depend on. Values that need a later output, mutable state,
`current.*`, or the provider environment being selected are
**decided at launch**; ordinary expression errors in otherwise static inputs
remain typed failures. A launch-time hint that depends on its own `env.AGENT`
or `env.MODEL` has a circular dependency: reject it with a source-located
selection error instead of composing with another task's stale identity. A statically known higher-precedence override makes a
lower dynamic hint irrelevant. Preflight still checks tasks with a false
`when`, following the existing approval contract.

Use the canonical provider resolver for scalar and list-valued hints, including
installed-provider filtering, favorites, ambiguity, and non-terminal errors;
do not introduce a second picker or select an unavailable list member. A
predictable hint that the canonical non-terminal rule cannot resolve gets the
same typed failure, annotated with the task's source and enclosing container
names. Dynamic selection gets that check at preparation, before spawning the
task; prior tasks may already have run. A changed document or live-state hint
may change the prediction, so it is never an authorization to skip launch-time
validation. Store choices by task identity and container ancestry, so repeated
names in nested containers do not share a review-screen selection.

Model resolution keeps the established order: explicit `--model`, captured
provider-specific model environment, captured generic `MODEL`, effective task
`model`, compatible planned fallback, then provider defaults. Use the shared
model resolver, preserving scalar/list hint behavior and warnings. An
inherited or review-selected model is eligible only for the provider it was
resolved against; switching provider cannot carry that provider's model into
the new launch. Clear stale `MODEL` overlays when no model resolves. A review
choice is a fallback for its task, not an authored `agent` setter that could
prevent a later proxy target from selecting its own provider.

A body step composes its sequence document, so its own hints are that
document's. A group has no `agent`: each prompt task inside it resolves on its
own. `shell:` and `side_effect:` tasks launch no provider.

### 6. The `sequence.yaml` schema

Two files in `claudine/schemas/`, discovered by the normal schema-root walk:

- `sequence.yaml`, `kind: trigger-schema`, matching
  `sequence: [string(required), number, boolean, object, "unknown[]"]`
  (every non-null YAML type, so malformed values also activate validation;
  `required` is attached to a concrete type because it has no effect on
  `any`/`unknown`);
  its payload types a sequence document's root;
- `sequence-types.yaml`, `kind: schema`, the type library the payload imports.
  Types defined in it reference each other by **bare local name**
  (2026-10-13-named-type-presence-and-local-names); types from another file use
  `@`. Unions are written as **YAML lists**, and a list of a union (`step[]`) is
  legal (2026-10-14-union-type-constraints).

It types the grammar of 2026-10-24-sequence-containers and
2026-10-25-loops-and-flow-control: `when:` and `with:` on a task, inline and
file-reference nested sequences, concurrent groups with `fail_fast`, `loop:` on
a group, at the root, and on an inline nested sequence's task entry (with
`gate:` typed as a stack), and the lifecycle events — `cancelled` included — on
the root, on groups, and on inline nested sequences, each typed with the event
type 2026-10-21-claudine-schemas exports for it. Removed keys such as group `execution` and root `fail_fast` are absent,
so DMLS reports them as unknown. Step objects deliberately accept authored
data: `params` is valid data, while removed step options `setup`/`teardown`
remain parser errors even if the catch-all accepts their shape. The parser
also enforces which executables can carry `with`.

The sketches below describe the intended coverage, not a loadable recursive
type library. `steps → task → sequence → steps` and `task → group → tasks →
task` are cycles; the schema engine at this rollout point rejects them, so
the interim schema uses bounded expansion, matching
2026-10-21-claudine-schemas' interim approach: distinct task, group, and
sequence types at three typed container levels, then an untyped container
boundary. A valid document nested deeper than the typed levels stays valid;
the schema checks less there, and the parser still accepts unlimited
nesting. Documentation explains the boundary.
2026-10-30-claudine-recursive-schemas removes it.
Use the actual exported `initialize-stack`, `stack`, and `loop` types from
the lifecycle library, extending it for `cancelled` using the container
contract's restricted actions. Reuse that shared loop type instead of creating
a second version that drifts from `claudine.yaml`.

```yaml
# claudine/schemas/sequence.yaml
kind: trigger-schema
match:
    sequence:
        - string(required)
        - number
        - boolean
        - object
        - "unknown[]"
$schema:
    sequence: steps@./sequence-types.yaml
    template: object -> values merged into every step (formal sequence documents)
    loop: loop@./lifecycle.yaml
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml
    blocked: stack@./lifecycle.yaml
    success: stack@./lifecycle.yaml
    failure: stack@./lifecycle.yaml
    cancelled: cancelled-stack@./lifecycle.yaml
    finalize: stack@./lifecycle.yaml
```

```yaml
# claudine/schemas/sequence-types.yaml
kind: schema
types:
    steps:
    - step[] -> an inline list of steps
    - string -> a file reference (with optional `-> offset` and `::operator`), a `{{ expression }}`, or a `$( command )`
    step:
    - string -> shorthand for `{ name: <string> }`
    - task
    task:
        name: string -> the step or leg name; required on a group leg
        when: expression -> run this task only when true
        with: object -> frontmatter values for the document this task runs
        prompt: file -> a prompt document to compose and run
        shell:
        - shell
        - shell[]
        side_effect: object -> one lifecycle action
        group:
        - group
        - string -> a `kind: group` file, or `name@catalog-file`
        task: file -> an externalized `kind: task` file
        sequence: steps -> a nested sequence
        timeout: string -> per-command shell timeout, such as `45s`
        operation: string
        flow: string
        loop: loop@./lifecycle.yaml -> only beside an inline `sequence:` list
        initialize: initialize-stack@./lifecycle.yaml
        start: stack@./lifecycle.yaml
        blocked: stack@./lifecycle.yaml
        success: stack@./lifecycle.yaml
        failure: stack@./lifecycle.yaml
        cancelled: cancelled-stack@./lifecycle.yaml
        finalize: stack@./lifecycle.yaml
        <string>: any -> authored step data, read as `seq.<field>`
    group:
        name: string(required)
        description: string
        variables: object -> read as `group.<variable>`
        tasks: task[](required; min(1))
        max_parallel: number(integer; min(1))
        fail_fast: boolean -> true (default) = the legs are atomic
        operation: string
        flow: string
        loop: loop@./lifecycle.yaml
        initialize: initialize-stack@./lifecycle.yaml
        start: stack@./lifecycle.yaml
        blocked: stack@./lifecycle.yaml
        success: stack@./lifecycle.yaml
        failure: stack@./lifecycle.yaml
        cancelled: cancelled-stack@./lifecycle.yaml
        finalize: stack@./lifecycle.yaml
```

The shared lifecycle `loop` type includes the loose verb keys allowed instead
of `gate`, updated by the loop specification. No object-merge syntax is assumed.
The schema cannot express the rules the parser enforces (exactly one
of `while`/`until`; `gate:` never beside loose verbs; event keys only on an
inline-sequence task; reserved step keys; unique leg names), and does not try.
When a document matches both `sequence.yaml` and `claudine.yaml`, shared
keys have identical declarations. Upgrade `claudine.yaml`'s previously loose
`sequence` declaration to the same imported sequence type so a later payload
cannot weaken task validation. The general Claudine payload retains `agent`,
`model`, and the other common frontmatter fields; neither trigger requires a
document-level provider. The sequence root accepts `template` and the
container's `operation`/`flow` according to their existing types. Add both new
files to the embedded launch-validation bundle as well as disk discovery, and
verify every relative import without a repository checkout. Installation
instructions copy the whole schema bundle, rather than individual files.

## Evidence

The 2026-09-27 rehearsal note said a sequence honors each task's own `agent:`
when no provider flag is given. Corrected:

> A follow-up probe with stub providers showed that each prompt task launches
> on its own document's `agent:`, including a value passed through
> `params: { agent: … }`, and that a command-line `--<provider>` overrides every
> task. This holds only when the sequence document's own `agent` resolves. A
> sequence document with no `agent` stops before any task runs: it opens the
> review screen on a terminal, or fails with `AgentResolutionFailed` without
> one, even when every task names a provider. Step status lines report the
> planned target rather than the provider that launched, so a mixed-provider
> sequence prints the wrong name.

Verified 2026-09-27 with a debug build: one sequence with no document `agent`
and one with `agent: claude`, each with a step passing `params: { agent: codex }`
and a step whose prompt declares `agent: opencode`. The first failed before any
launch. In the second, step 1 launched `codex` but reported
`succeeded (via Claude)`, and step 2 selected `opencode`.

## Requirements

1. **Rename.** Task `with:` replaces `params:` in the task parser, the
   sequence runner (Claudine library's
   [layered_overrides](../../../lib/src/composition/sequence/task/mod.rs), which
   combines task input layers, and the Claudine CLI's
   [task runner](../../../cli/src/commands/wrap/sequence/task_run.rs)),
   the reserved-key catalog, and every shipped prompt, fixture, test, doc page,
   and skill reference, including `prompts/review-loop.md`,
   `docs/topics/flow-control/sequences.md`, and `docs/cli/sequence.md`.
2. **Provenance.** `proxy`, `retry`, and `prep` `with` values carry their
   authoring file's origin and are materialized like caller file values;
   `composition.md` ("Caller File Provenance and Materialization") and
   `flow-control-reference.md` ("Passing values with `with:`") state the one
   rule.
3. **Errors.** The `with` error family of section 3, raised for task `with`
   with task source locations.
4. **Provider prediction**, gate, review screen, and reporting per section 5.
5. **Schemas.** `sequence.yaml` and `sequence-types.yaml` per section 6, with
   fixtures.
6. **Docs.** `sequences.md` "Phase 1 — Static preflight" states that preflight
   predicts each task's provider from its own document's `agent`, the same way
   the launch chooses it; that a sequence whose tasks all name providers runs
   without a document-level `agent` or a review screen; that a task naming a
   provider that is not installed fails before any task launches; and that
   status lines name the provider that ran. The schema topic in
   `claudine/docs/` lists `sequence.yaml`. The `claudine` skill is updated.

## Out of scope

- Container semantics, `seq`/`group` roots: 2026-10-24-sequence-containers.
- Loops, `break`, `prep`, `when:` semantics: 2026-10-25-loops-and-flow-control.
- The lifecycle and action type libraries the schema imports:
  2026-10-21-claudine-schemas; switching them to recursive types:
  2026-10-30-claudine-recursive-schemas.
- Moving lifecycle ownership to Darkmatter:
  2026-10-27-darkmatter-lifecycle-ownership.

## Acceptance criteria

All L1 unless noted. Tests use the monorepo Test Toolkit and
`CliProcessFixture` with a private CWD and
home, stub providers (with stderr redirected), and `PLAYA_DRY_RUN` with a
private spool; none launches a real provider or focuses a window; all are
portable to macOS, Linux, native Windows, and WSL2. `just test` and `just lint`
are green in `claudine/` and, for the schema fixtures, `darkmatter/` (DMLS
included). Run the review-screen L2 coverage with `just test-l2` in
`claudine/`, using the terminal test harness without focusing a window.
No new CI cell.

1. **`with` on tasks.** A task `with:` reaches the task document with the
   section 1 precedence; `params:` on a step is plain step data readable as
   `seq.params`; `with:` on a `shell:` task is the existing typed error.
2. **Provenance.** Table-driven: a relative file value in a task `with`, a
   `proxy.with`, a `retry.with`, and a `prep.with` each resolves from its
   authoring file, including a task in a `kind: task` file in another
   directory and a `proxy` target in another directory; a caller setter still
   resolves from the launch directory and outranks `with`. Include lazy
   missing local files, eager missing-file errors, remote lazy values,
   ambiguous file/string schema arms, an expression forwarding an already
   anchored value, and a chained handoff whose target schema changes. Assert
   an evaluation error installs no partial overlay.
3. **Shape errors.** Each row of section 3 raises its error, for task `with`
   and for `proxy.with`, `retry.with`, and `prep.with`, pointing at the source.
   Whole-mapping interpolation gets its specific error rather than the generic
   non-mapping error; reserved generated bindings remain protected.
4. **Tasks name their providers.** A sequence document with no `agent` runs
   against stub providers: one task passes `agent: codex` through `with`, one
   runs a prompt declaring `agent: opencode`, inside a looping sequence. The run
   starts with no picker or error, each stub receives only its own task's
   prompts on every pass, and each status line names the stub that ran.
5. **Precedence.** A table-driven test covers each row of the precedence table
   once, including a caller setter `agent=gemini` overriding a task's
   `with.agent` and a nested file-reference sequence's own `agent` applying to
   its tasks. Include runtime `set` overriding a caller setter, scalar/list
   provider hints under canonical installed-provider rules, and model
   environment precedence. A provider switch drops an incompatible inherited
   model and clears stale `MODEL` when necessary.
6. **Early failure.** A task naming a provider that is not installed fails
   preflight, naming the task and its source, before any task launches, even
   when its `when` is false. Shell-only and side-effect-only graphs run without
   a provider, picker, or provider-resolution error.
7. **Review screen** (L2). With a terminal, and some tasks without a hint of
   their own, tasks that have their own provider are read-only rows, and a
   choice on an editable row is the provider that launches.
8. **Decided at launch.** A task whose `agent` reads `seq.outputs` is reported
   as decided at launch, does not gate the run, and launches on the value
   composed at its turn. Repeat with a runtime-mutated value and a hint in a
   concurrent leg using the group's start snapshot. Preflight executes no
   child lifecycle action, shell substitution, or model-network refresh.
   Retries and proxy provider switches report actual attempt identities;
   skipped and blocked tasks claim no launched provider. An unresolved or
   unavailable dynamic hint fails before that task spawns.
9. **Schema.** `md schema triggers` picks up `sequence.yaml` for a sequence
   document with no `$schema:` pointer; `md schema validate` and DMLS accept
   every sequence example in 2026-10-24-sequence-containers and
   2026-10-25-loops-and-flow-control (saved as fixtures), and report
   `execution` and a typo'd key such as `max_paralel:` on a group object, and
   `fail_fast` at a sequence document's root, by name. A non-null malformed
   `sequence: 42` or mapping activates the sequence trigger and fails payload
   validation; missing/null `sequence` does not activate it. With both Claudine
   triggers active, malformed task `with` still fails. Installed launch
   validation resolves the embedded bundle's imports without checkout files.
   Add valid and invalid fixtures at each of the three typed container levels
   and a valid deeper fixture demonstrating the documented untyped boundary. Steps accept
   authored data, so an unknown step key is step data, not a schema error.
10. **Sweep.** No maintained task-input use of `params:` remains; ordinary
    authored data named `params`, historical spec snapshots, and timeline
    evidence remain intact. Shipped-prompt fixtures and tests use the new
    grammar; tests of prompt snapshots remain opt-in under repository rules.

## Open questions

None.
