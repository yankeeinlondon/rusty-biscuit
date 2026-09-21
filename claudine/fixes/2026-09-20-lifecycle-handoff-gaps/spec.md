---
created: 2026-09-20
status: draft-spec
clarified: false
reviewed: true
reviewed_by: codex/gpt-6-astra
reviewed_on: 2026-09-20
review_iterations: 0
needs_rulings: false
implemented: false
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
# Finding ids (F1–F8, D1–D2) whose fix has landed, wherever it landed. `prompts/_prompt.md`
# reads this list and stops warning agents about a defect once its id is here (R10).
# Keep the key present even when empty: the guide's gates do not tolerate a missing list.
fixed: []
area: claudine
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - dmls
related:
    - 2026-07-13-proxy-with
    - 2026-09-15-initialize-after-proxy
    - 2026-09-17-remove-strict-mode
---

# Lifecycle Handoff Gaps Found by the PR Flow

## Outcome

A multi-document flow built from `proxy` handoffs behaves the way the lifecycle
and composition references say it does. Eight things currently fall short of that:

1. `ctx` is meant to be evaluated once, eagerly, for **each composition run**. It
   is instead answered from a cache that lives for the whole invocation, so a
   proxy target, a later sequence step, a later loop iteration, and a retried
   attempt all reuse the first composition's git working state.
2. A `proxy` fired from a terminal event of a looping document is recorded but
   never performed.
3. A shell command in a transcluded partial is not pre-approved when it
   interpolates a value delivered by `proxy … with:`.
4. An `error` action in a `success` stack fires `failure` and `finalize` but the
   process still exits `0` and renders no error.

5. A `ctx` property mentioned only in a transcluded file renders empty.
6. `looping.md` documents a pre-checked loop. The loop is post-checked by design,
   and the other two references say so. It documented a second, pre-checked
   engine that no command uses and that is to be deleted (R7).
7. A `proxy` inside a sequence `prompt:` task is refused, although the reference
   says a sequence contains a proxy within its step.
8. The `loop:` block's own notification fields cannot read the `_loop_*` values,
   although `lifecycle.md` shows that as its example.

The first four were found while rebuilding `prompts/pr.md` into a chain of stages
(`pr.md` → `_pr/dirty.md` → `commit.md` → `_pr/push.md` → `_pr/open.md`, with
`_pr/diagnose.md` → `_pr/triage.md` → `_pr/fix.md` → `commit.md` on the failure
path). That flow is the motivating consumer, and it carries a workaround for
each defect. Removing those workarounds is part of this fix (R6). The fifth was
found while writing `prompts/_prompt.md`, a partial meant to be transcluded. The
sixth and seventh were found while evaluating a loop and a sequence as
alternatives to the proxy chain. Two defects in Darkmatter and in terminal rendering (D1, D2) were found the
same way and are in scope. R9 is the one requirement that adds authoring surface
rather than repairing behavior, and is included at the author's direction.

They share one spec because each concerns a document that is reached through,
or hands off through, the active-document coordinator, and each is measured
against the equivalence contract in `docs/topics/composition.md` ("a document
reached through a proxy behaves like the same document invoked directly").

## Review Boundaries and Existing Contracts

This remains a draft specification, not an implementation report. Preserve the
author's decisions about per-composition context, post-checked loops, and result
suffixes. The four interactions the review raised have since been ruled on by
the author; see **Open Questions** for the index.

The main contracts checked during this review are:

- Claudine's [composition contract](../../docs/topics/composition.md), including
  launch-anchored context, target identity, sequence ownership, and dry runs.
- Claudine's [lifecycle contract](../../docs/topics/lifecycle.md), including
  shell-free initialization, early approval, and atomic mapping assignments.
- Darkmatter's [frontmatter shell expansion contract](../../../darkmatter/docs/inline/fm-shell-expansion.md),
  including whole-value parsing, conditional expressions, and cache behavior.

Source paths beginning with `lib/`, `cli/`, or `docs/` below are relative to
the Claudine package area; other unlinked paths are repository-relative.

## Reported Failures

The draft author reports that every reproduction below was run against a stub
provider (an executable named `claude` that prints one line and exits `0`), so no model was involved and the
results do not depend on model behavior. This specification review checked the
contracts and source; it did not independently rerun those probes. Turn each
reproduction into an isolated fixture rather than running it in this checkout.

### F1. `ctx` is not re-evaluated for each composition run

**The principle** (author's ruling, 2026-09-20): `ctx` is evaluated once,
eagerly, for **each composition run**, and only for the properties found on the
page. `current` is its lazy counterpart and is available only to lifecycle
hooks. A composition run is one document being composed and executed, however
it was reached:

| Vehicle | One composition run is… |
|---|---|
| direct `compose` / `inline-compose` | the document |
| `sequence` | each step; inside a group, each task of a **serial** group, and a **parallel** group as a whole |
| lifecycle flow control (`proxy`, later `prep`) | each document adopted |
| `loop` | each iteration, which runs a full composition cycle |
| `retry` / `resume` | each attempt, per `2026-07-13-proxy-with` R8 ("a new coherent prepared context … not … a stale context snapshot") |

Transclusion is not on this list. `::file`, `::code`, and the other directives
build one document out of a tree of files, and that tree is one composition run
with one `ctx`.

**Observed.** Every probe has the same shape. The working tree starts with one
modified file and nothing staged. **Run A** is a composition run that reads
`length(ctx.staged_files)` before anything is staged. Then the file is staged.
**Run B** is a *separate, later* composition run that reads the same expression.
Under the principle, run B should report `1`.

| Vehicle | Run A (before staging) | What stages the file | Run B (a later composition run) | Run B `ctx`: observed | expected |
|---|---|---|---|---|---|
| `proxy` | `router.md`, in its `start` stack: `0` | `router.md`'s own `start` stack, before it proxies | `target.md`, composed after the handoff | `0` | `1` |
| `sequence` | step 1 (`prompt:`): `0` | step 2 (`shell: git add .`) | step 3 (`prompt:`) | `0` | `1` |
| `loop` | iteration 1: `0` | iteration 1's `success` stack | iteration 2 | `0` | `1` |
| `retry` | attempt 1: `0` | attempt 1's `success` stack | attempt 2 | `0` | `1` |

In every run B, `length(current.ctx.staged_files)` read from a lifecycle hook is
`1`, so the evidence is obtainable; only `ctx` is stale.

Within run A itself `ctx` stays `0` after the staging, in every vehicle. That is
correct: one composition run, one eager evaluation.

There is one more observation, and it is what makes the cause unambiguous. In
the `proxy` case, delete the source's only mention of `ctx.staged_files`, so the
target is the first composition to ask for it. The target then reports `1`. The
value is not "the state at launch" and not "the state when this run began". It
is "the state when any run first asked".

**Cause.** `lib/src/invocation_context.rs` holds git working state in
`SourceEvidence.file_changes`, a `OnceLock` owned by the invocation-scoped
`RepositoryEntry`. Each composition run does build a fresh `ComposeContext`
(`capture_launch_context` / `extend_launch_context`), but every group is
answered from those invocation-lifetime cells: "the retained caches answer every
group". The cell is filled by the first composition that requests the group and
is never observed again. Volatile working-tree state shares one cache lifetime
with evidence that really is stable for an invocation: repository identity,
package topology, and host facts.

The reference is inconsistent with itself here, which is how this went
unnoticed. `docs/topics/composition.md` "Launch-Anchored Prepared Context"
already says "one snapshot per document epoch" and that a proxy, a retry, and a
resume each "start a new epoch". Its "Binding time" table says `ctx` is "captured
once at the start of the run, shared by the whole run". The same document says
loop iterations reuse "the already-audited structural plan".

**Consequence in the PR flow.** `prompts/commit.md` lists `ctx.staged_files` in
its body. Reached after an earlier stage staged files, it shows the agent the
earlier list, so `_pr/dirty.md` and `_pr/fix.md` pass the real list through
`message`. `_pr/push.md` reads git state through `::shell-block`s for the same
reason. `commit.md`'s `initialize` *gate* reads `current.ctx.staged_files`; that
one is correct as written and stays, because a gate asks about now.

### F2. A proxy from inside a looping document is not performed

`loop.md`:

```yaml
---
n: 0
loop:
    while: "n < 2"
    action: "increment(n)"
success:
    stack:
        - action:
              - info: "success fired, iteration {{_loop_count}}"
              - proxy: ./target.md
---
loop body {{_loop_count}}
```

Observed:

```text
success fired, iteration 1
success fired, iteration 2
CompositionError: iteration failed
Iteration 2 exited with code 1.
lifecycle `proxy` hand-off to `./target.md` forms a cycle or exceeds the proxy
hop limit (16); active chain: …/loop.md -> …/target.md
```

Iteration 1's `proxy` does not hand off: the loop proceeds to iteration 2. The
target was nevertheless appended to the run ledger's chain, so iteration 2's
identical request is rejected as a cycle against an entry for a handoff that
never happened. `target.md` is never adopted.

`2026-07-13-proxy-with` R7 already rules the intended behavior: "A proxy emitted
by the loop lifecycle ends the source document and returns a handoff to the
active-document coordinator. It does not make the target an extra iteration of
the source loop." `docs/topics/lifecycle.md` says the same ("a `proxy` from
`success`/`failure` skips that attempt's ordinary `finalize`"). Existing loop
coverage exercises a proxy from `initialize`
(`loop_initialize_proxy_hands_off_without_iterating`) and a proxy *into* a
looping target, but not a proxy from a terminal event of a looping source.

**Cause (traced, not yet fixed).** The engine already implements the ruling:
`execute_loop_with_lifecycle` in `lib/src/composition/looping/engine.rs` checks
`output.handoff` after every iteration and returns the handoff without running
the gate ("A proxy surfaced by the iteration's own lifecycle ends the source
document *now*"). Nothing ever sets that field. The only production builder of a
`LoopIterationOutput` is `build_loop_iteration_output` in
`cli/src/commands/compose/loop_run.rs`, and neither of its branches calls
`with_handoff`; the two `with_handoff` call sites outside tests are both inside
the engine. The iteration's proxy request reaches the run ledger, which is why
the chain gains an entry, but never reaches the loop.

### F3. A transcluded shell command that interpolates an overlay value is not pre-approved

`router.md` proxies with `with: { base: main }`. `target.md` declares
`base: string(required)` and contains only `::file ./part.md`. `part.md`:

```md
::shell-block when_error="(unavailable)"
git rev-parse --short {{base}}
::end-block
```

Observed:

```text
warning: [transclusion] Shell block error: … Command 'git rev-parse --short main'
… was not pre-approved (in …/part.md). This is a bug in the pre-flight scanner --
please report it.
```

The matrix, all under identical approval conditions:

| Command location | How `base` arrives | Result |
|---|---|---|
| inline in the target | `proxy … with:` | approved |
| transcluded partial | caller `base=main` | approved |
| transcluded partial, no interpolation | n/a | approved |
| transcluded partial | `proxy … with:` | **not pre-approved** |
| transcluded partial, target invoked directly | caller `base=main` | approved |

Only the combination fails. The approved command and the executed command
differ, which points at discovery of the transclusion graph interpolating
against state that does not include the immediate proxy overlay, while
execution does include it. That is a lead, not an established root cause.

### F4. `error` in a `success` stack does not fail the process

```yaml
---
success:
    stack:
        - action:
              - warn: "success fired"
              - error: "gave up"
failure:
    stack:
        - action:
              - warn: "failure fired: {{ err.msg }}"
finalize:
    stack:
        - action:
              - warn: "finalize fired; err={{ err.msg || 'none' }}"
---
body
```

Observed: all three `warn` lines print, with `err.msg` equal to `gave up` in
`failure` and `finalize`; then the process **exits `0`** and renders no
`Error:` line.

Move the same `error` action into `start` and the process prints
`Error: gave up` and exits `1`. An `error` raised from `finalize` also behaves
correctly (`Error: gave up in finalize`, exit `1`), so the loss is specific to a
downgrade that starts in `success`.

So the downgrade is performed inside the lifecycle and is then lost before it
reaches the rendered outcome and the exit code. `docs/topics/lifecycle.md`
says `error` "at `success`/`finalize` … converts success to failure", and its
completion-verdict section says the process exits non-zero when no recovery
takes the failure.

`level2_lifecycle_success_stack_error_downgrades_keeps_success_comm` pins the
event order (`success`, `failure`, `finalize`) for exactly this document shape
and asserts nothing about the exit code or the rendered error. The defect lives
in that gap. The same loss occurs when the `error` fires on a retried attempt.

Consequence: every shipped prompt that verifies an artifact in `success` and
raises `error` when it is missing reports success to its caller, to a
`sequence` step's `fail_fast`, and to CI.

### F5. A `ctx` property mentioned only in a transcluded file renders empty

`kid.md`:

```md
child reads ctx.repo=[{{ ctx.repo }}] ctx.os=[{{ ctx.os }}]
```

`parent.md` contains one line of prose and `::file ./kid.md`.

| Composed by | Parent mentions | Child renders |
|---|---|---|
| `claudine compose` | no `ctx` property | `ctx.repo=[] ctx.os=[]` |
| `claudine compose` | `ctx.repo` only | `ctx.repo=[rusty-biscuit] ctx.os=[]` |
| `md compose` (Darkmatter alone) | no `ctx` property | `ctx.repo=[rusty-biscuit] ctx.os=[macOS]` |

Confirmed in a real run against a stub provider as well as under `--dry-run`:
the agent receives the empty values, with no warning.

Claudine evaluates only the `ctx` properties found on the page, for
performance. The page it scans is the root document; the transclusion tree is
not walked, so a property that appears only in a `::file`d partial is never
requested and resolves to nothing. Darkmatter, given no requirement list,
evaluates what the composed tree needs and gets it right.

Shipped partials that read `ctx` and are transcluded by other prompts include
`prompts/_os.md` (four `_implement/` prompts), `prompts/_repo-context.md` and
`prompts/_agent-skills.md` (`lineage.md`), and `prompts/_writing-clearly.md`.
Each renders correctly only where the including prompt happens to mention the
same properties.

### F6. The references disagree about when a loop checks its condition

**The design** (author, 2026-09-20): the check is at the **end** of an
iteration, so a loop always runs at least once. The behavior matches that, with
one detail worth stating because it sets the count: the gate reads the state the
*finished* iteration ran with, and the actions are applied only after the gate
decides to continue. `docs/topics/composition.md` and `docs/topics/lifecycle.md`
describe exactly this ("the loop condition is evaluated after lifecycle concerns
and before per-iteration mutations are applied").

With `n: 0` and `action: "increment(n)"`:

| Condition | Iterations | Each body sees |
|---|---|---|
| `while: "n < 0"` | 1 | `n=0` |
| `while: "n < 1"` | 2 | `n=0`, `n=1` |
| `while: "n < 2"` | 3 | `n=0`, `n=1`, `n=2` |
| `until: "n == 2"` | 3 | `n=0`, `n=1`, `n=2` |

Read as "continue while the iteration that just ran satisfied the condition",
every row is right: `until: "n == 2"` stops after the iteration that ran with
`n=2`.

**Where the other description came from.** `looping/engine.rs` holds two
engines. `execute_loop_with_lifecycle` is post-checked and is what
`claudine compose` and `claudine inline-compose` call. `execute_loop` and
`execute_loop_with_config` are an older, **pre-checked** engine that can run zero
times; they are still exported from `claudine::composition`, and only library
tests call them. `looping.md` documented that one.

`docs/topics/flow-control/looping.md` said something else, in detail. Its
"Iteration semantics" lists a strict **pre-check order**: evaluate the condition,
exit if it says stop, *then* run the prompt, *then* apply actions. It says the
loop "exits with the current `_loop_count - 1` iterations recorded", and its
`_loop_is_last` definition is built on the same order. An author who reads only
that page predicts 0, 1, 2, and 2 for the table above, writes `while: "n < 3"`
for three runs, and gets four. This spec's first draft made that mistake. The
page's own running example, `until: "_loop_count > 3"` above a body reading
"Iteration N of 3", runs four times.

`looping.md` was rewritten to the post-checked design alongside this draft, and
`composition.md` and `lifecycle.md` were given the same rule. What remains
is deleting the unused engine and adding the characterization test described
in “One loop engine, one description of its timing” below.

### F7. A `proxy` inside a sequence `prompt:` task is refused

A sequence step `prompt: ./fix.md`, where `fix.md` proxies from its `start`
stack, fails the step:

```text
step 2/5 failed: task `./fix.md` could not run prompt …/fix.md: lifecycle `proxy`
hand-off to `./triage.md` has no owni…
```

That is `LifecycleProxyWithoutOwningCoordinator`, the refusal documented for the
direct provider wrappers, which own no coordinator. `docs/topics/composition.md`
("Command-level ownership") says the opposite for sequences: "`sequence`
contains a proxy within its current step: no step advance, no restart, and the
step keeps its scoped inputs and timing identity."

Not a defect, and recorded because it was suspected: a step whose *document*
declares `interactive: true` does launch interactively. Only `interactive: true`
on the sequence document itself is rejected.

### F8. The `loop:` block cannot read the loop's own ambient values

```yaml
loop:
    until: "_loop_count >= 2"
    action: increment(counter)
    info: "gate after iteration {{_loop_count}}"
```

The run fails at the first gate with a lifecycle evaluation error, "unknown
root". `{{counter}}` in the same field works, and `{{_loop_count}}` and
`{{_loop_is_last}}` work in `start`, `success`, `failure`, and `finalize`. So the
`_loop_*` values are supplied to the iteration's events and to the condition,
but not to the lifecycle concerns the gate runs. `docs/topics/lifecycle.md`
("Loop lifecycle concerns") gives `info: "iteration {{_loop_count}}"` inside a
`loop:` stack as its example.

The diagnostic names the root as `loop_count`, without its underscore. That is
D2 acting on error text, and it sends the reader looking for a name that is
spelled correctly in their document.

`2026-09-17-remove-strict-mode` will turn this error into an empty rendering.
That hides the failure without supplying the value, so it does not close this
finding.

## Findings in Darkmatter and the Terminal Renderer

These reproduce without Claudine's lifecycle. They are in scope: this monorepo
draws no border between package areas for a spec, and both were found by, and
get in the way of, the same prompt work. Each has a requirement (R12, R13), and
`fixed:` tracks them like any other finding.

### D1. Interpolation literals convert only in a file that has a real span (Darkmatter)

A file containing `only a literal: {{{ expr }}}` composes with its triple braces
intact. Add any real span to the same file and the literal converts to double
braces as documented. The decision is per file, so a transcluded partial made
only of literals is affected even when its parent has real spans. Reproduces
under `md compose`. The pass that converts literals appears to be skipped when a
file has nothing to interpolate. `prompts/_prompt.md` carries one real span for
this reason.

### D2. A composition's `description` is mangled in the run header (terminal rendering)

```yaml
description: |-
    1. **_pr/open.md** opens it.
    2. **_pr/triage.md** triages it.

    See `_pr/_report.md`.
```

The header table renders:

```text
1. pr/open.md opens it.
2. </i>pr/triage.md triages it.

See `pr/report.md`.
```

Leading underscores are consumed as emphasis markers across lines, a literal
`</i>` reaches the terminal, and underscores are consumed even inside an inline
code span, where Markdown treats them as text. File names are shown wrong, which
matters in a header whose job is to say what is about to run. The same mangling
reaches error text: F8's diagnostic reports the unknown root `_loop_count` as
`loop_count`.

## Required Behavior

### R1. `ctx` is evaluated once for each composition run

Every composition run in the F1 table observes its volatile `ctx` properties at
the start of that run, once, and holds them stable for that run's body,
frontmatter, schema evaluation, shell preflight, and lifecycle events.

- **Volatile** means evidence that can change between runs, including changes
  made by another process: git working state
  (`file_changes`, and whatever backs `ctx.branch` and `ctx.worktree`, since a
  stage can create a branch). These move from invocation lifetime to
  composition-run lifetime.
- **Stable** evidence keeps its invocation lifetime, and the launch-anchored
  contract is untouched: launch CWD, repository identity, package topology, area
  resolution, host facts, and the environment capture. Moving a document must
  still not move these, and a new run must not rediscover them.
- "Only the properties found on the page" is unchanged. A run that mentions no
  git property still pays for none.
- Which run first mentions a property must have no effect on the value a later
  run observes. This is the testable statement of the fix.
- **Sequence groups** (author's ruling, 2026-09-20): a serial group evaluates
  `ctx` once per task, like any other step. A **parallel** group evaluates it
  once for the group, so concurrent siblings share one view of the working tree.
- **A parallel group shares at launch and refreshes on re-entry** (author's
  ruling, 2026-09-20). Before any sibling starts, collect the `ctx` properties
  mentioned by every task the group launches and capture that combined set once;
  all siblings begin from it. A sibling that later retries, resumes, proxies, or
  begins another loop iteration is a new composition run and captures its own
  fresh evidence, exactly as it would outside a group. Siblings therefore stop
  sharing one snapshot once one of them re-enters, which is intended: a recovery
  path must see the working tree as it is, including what its siblings changed.
- **Target identity stays task-specific.** Sharing repository evidence must not
  share `ctx.agent`, `ctx.model`, `env.AGENT`, or `env.MODEL` across siblings
  that choose different providers. Each task layers its resolved identity over
  the shared evidence, as the existing composition contract requires.
- **Requirements are read from the root page before `initialize`** (author's
  ruling, 2026-09-20). A document that declares `initialize` starts in stages and
  does not discover its body's includes until `initialize` has run. Before it
  runs, scan the root document's own text, frontmatter, lifecycle blocks, and
  body alike, for `ctx` mentions and capture those evidence groups then. That is
  within the staged-boot contract, which permits "parsing the root document and
  capturing its request context" before initialization and forbids only
  following a body include. The scan reads text; it composes nothing, follows no
  `::file`, and runs no shell.
- **Same-run extension is not a refresh.** The post-`initialize` reread remains
  in the same composition run. Collect the stabilized transclusion tree's
  requirements without replacing already observed values. A property from a group
  that was already observed is derived from that retained observation, not from a
  second working-tree scan.
- **The one exception is an include.** A transcluded file cannot be read before
  `initialize`, which may be what creates it. If such a file is the first thing
  in the run to mention a volatile group, that group is captured on first demand,
  after `initialize`, and held for the rest of the run. State this exception in
  the reference and pin it with a regression test, so it stays an exception and
  does not become the general rule again.
- **`current` refreshes once per event** (author's ruling, 2026-09-20): every
  read of `current` within one event's notification fields and stack sees the
  same value, and the next event sees a fresh one. That is today's behavior.
  Correct `docs/topics/composition.md` and the `claudine` skill, which say each
  `current.<key>` "is evaluated when it is referenced". Revisit this rule when
  `prep` lands: a flow-control action that leaves an event and re-enters it
  makes "one event" span work done elsewhere.

Update both passages of `docs/topics/composition.md` named in F1, the loop
wording, and the `claudine` skill's "Binding time" paragraph so they state the
per-composition-run rule once, in the same words.

### R2. A proxy from a looping document's terminal event hands off

A `proxy` control produced by `success`, `failure`, or `finalize` of a looping
document ends the loop and returns the handoff to the coordinator, per
`2026-07-13-proxy-with` R7. Specifically:

- the target is adopted and entered at its own `initialize`;
- no further iteration of the source runs, and the source's `loop` gate does
  not fire for the abandoned iteration;
- the run ledger gains exactly one chain entry, at the moment the handoff
  commits. A request that does not become a handoff must leave the chain
  untouched, so that a later legitimate request cannot be rejected against it.

The last point is a separate invariant and should be asserted separately: the
ledger records committed handoffs, never requests. A commit means the coordinator
has resolved the target and accepted its cycle/hop checks, then atomically
adopted it. It does not mean the target eventually succeeds: an adopted target
that fails initialization or validation remains in the chain. Resolution,
overlay-evaluation, cycle, and hop-limit refusals do not append an entry.
A handoff from `success` or `failure` skips the source's ordinary `finalize`;
a handoff from `finalize` does not run that event again.

### R3. Shell discovery and execution resolve the same command bytes

Discovery of a proxy target's shell surface interpolates against the same
effective state its execution uses, including the immediate `with:` overlay and
retained caller overrides, across the whole transclusion graph. The invariant
is the one `docs/topics/pre-flight-checks.md` already states
(`execution_set ⊆ approval_set`); this restores it for transcluded commands in
an overlaid target.

`NotPreApproved` must remain a bug sentinel. It must not become a soft warning
that lets the run continue with a fact silently missing, which is what happens
today when the block carries `when_error`. Neither `when_error` nor lifecycle
`no_error` may suppress a pre-approval invariant failure. Ordinary command
failures retain their documented recovery behavior.

Use the existing state-precedence rules, including target defaults, inherited
state, immediate proxy overlays, explicit caller overrides, reserved sequence
inputs, and transclusion-local `set.*`. Discovery must use the same resolver and
source-relative base directory as execution. Discover commands in untaken
branches without executing them. A fresh target still receives its full audit;
reusing an approval never authorizes newly changed command bytes.

### R4. A lifecycle downgrade reaches the process outcome

When an `error` control converts a terminal event to failure and no recovery
action takes it, the run:

- exits non-zero;
- renders the error once, through the same effective-diagnostic selection every
  other failure uses (`docs/topics/error-architecture.md`), so `err.*`, the
  terminal block, and machine output describe one error;
- reports the failure to its owner: a `sequence` step fails under the existing
  `fail_fast` rules, and a loop iteration counts as a failed iteration.

This holds on a first attempt and on a retried or resumed attempt alike.
Preserve already emitted success communications, the existing failure/finalize
event order, and recovery budgets. A successful retry, resume, or proxy recovery
reports its eventual outcome rather than a stale error from the abandoned
attempt. “Once” means one canonical diagnostic, not suppression of an author's
explicit `warn` or failure notification.

### R5. `ctx` requirements are collected across the transclusion tree

The set of `ctx` properties evaluated for a composition run is the set mentioned
anywhere in the composed tree: the root document and every file it transcludes,
recursively, including a file chosen by an interpolated `::file` reference. A
transclusion tree is one composition run with one `ctx` (F1), so a partial and
its parent observe the same value. "Only what is found on the page" continues to
hold, with "the page" meaning the composed tree.

Use the composition parser's rules for executable spans: fenced examples and
interpolation literals must not trigger context discovery, and raw code
inclusions must not be scanned as executable Markdown. Preserve transclusion
conditions, per-file overrides, file-resolution policy, and existing cycle and
depth limits. Discovery itself must not execute shell commands or side effects.
Resolve context-dependent include paths in stages using already captured
requirements; if expansion cannot finish safely before execution, return a
source-attributed preparation error rather than silently supplying empty values.
The scan must include lifecycle expressions and included frontmatter, not only
rendered body text.

### R6. Remove the workarounds from the shipped PR prompts

These exist only because of F1–F4. Remove each once its requirement lands, in
the same change, and re-run the PR flow rehearsal:

| Prompt | Workaround | Removed by |
|---|---|---|
| `prompts/_pr/dirty.md`, `prompts/_pr/fix.md` | the "staged-file list … is out of date" text passed to `commit.md` through `message` | R1 |
| `prompts/_pr/push.md` | git working state read through a `::shell-block` instead of `ctx.dirty_files`, and the second half of the explanatory comment | R1 |
| `prompts/_pr/push.md` | the facts `::shell-block`s are inline instead of a shared `_pr/_facts.md` partial | R3 |
| `prompts/_pr/fix.md` | bounded `retry: 2` plus a `fix_attempts` counter in the report, instead of a `loop:` | R2 (optional; `retry` is a reasonable design on its own, so re-evaluate rather than revert) |
| `prompts/_pr/fix.md` | a `warn` duplicating the exhaustion `error`, added so the outcome is visible | R4 |

`prompts/commit.md`'s `initialize` gate was changed to `current.ctx.staged_files`
alongside this draft. That is ordinary use of the designed binding times, not a
workaround, and it migrates to `current.staged_files` with the rest of the
prompts when R29–R33 land.

### R7. One loop engine, one description of its timing

**Ruling** (author, 2026-09-20): the loop is post-checked, and there must not be
two implementations. Delete the pre-checked engine outright. The project has no
users, so there is no deprecation period.

**Delete** `execute_loop` and `execute_loop_with_config` from
`lib/src/composition/looping/engine.rs`, together with whatever only they use
(`compute_is_last` is one candidate; confirm before removing). Both functions
carry a "Scheduled for deletion" note naming this requirement. The sites to
clear, from a whole-workspace search:

| Site | What to do |
|---|---|
| `lib/src/composition/mod.rs` | drop both names from the `pub use looping::{…}` re-export |
| `lib/src/composition/looping/engine/tests/iteration_actions.rs` (15 mentions) | port to `execute_loop_with_lifecycle` |
| `lib/src/composition/looping/engine/tests/rate_limits.rs` (7) | port |
| `lib/src/composition/looping/engine/tests/seed_state.rs` (3) | port |
| `cli/src/commands/compose/loop_run.rs` | its doc comment links to `execute_loop`; reword |

Port the tests rather than dropping them. They cover action atomicity,
rate-limit policy, and loop seeding, none of which depends on when the condition
is checked, but every expected **iteration count** in them was written for a
pre-checked loop, so expect most of them to move by one. That was not checked
test by test. Re-derive each count from the rule in F6; do
not adjust an assertion until it passes.

**Documentation.** `looping.md`, `composition.md`, and `lifecycle.md`, and the
`claudine` skill's copies of the latter two, were brought to one description of
the post-checked loop alongside this draft, including the F6 counting table and
the `skip`-from-`initialize` route to zero iterations. `lifecycle.md`'s loop
example was corrected at the same time: it promised three iterations and ran
four, and it read `_loop_count` from inside the `loop:` block (F8). When the
engine is deleted, remove the "Implementation note" from `looping.md`.

### R8. A sequence task contains a proxy within its step

A `proxy` raised by a `prompt:` task's document is adopted within that step, as
the reference describes: the target runs, the step's result is the final
target's, the step does not advance early or restart, and the hop joins that
step's chain. Implement that documented behavior; changing the reference to
permit refusal does not satisfy this fix.

Preserve task inputs, task identity, timing, output attribution, setup/teardown,
and budget accounting across the handoff. Setup and teardown still run once per
task. Commit the final target's result to the sequence output once. A failed
target follows the existing `fail_fast` policy. Parallel siblings have independent
handoff chains, so two siblings may legitimately adopt the same target; cycle
and hop checks still apply within each chain. Direct provider wrappers continue
to reject proxy requests because they do not own a document coordinator.

### R9. A lifecycle step can read a command's result

Included at the author's direction (2026-09-20). It is the one requirement here
that adds authoring surface rather than repairing behavior. Today a lifecycle `shell`
action returns nothing, and a `$(…)` inside a lifecycle `set` is stored as the
literal string. The only way to branch on a command is a frontmatter `$(…)`,
which runs at compose time, cannot observe anything a lifecycle step did, and is
rejected in a document that declares `initialize`.

```yaml
start:
    stack:
        - action:
              - set:
                    sha: "$(git rev-parse --short HEAD)"    # stored verbatim today
```

Make that assignment run the command and store its trimmed stdout. Two
constraints carry over unchanged: the command is discovered and approved during
preflight, byte for byte, so it may reference only early-binding values; and it
remains forbidden in `initialize`.

**Three result suffixes** (author's ruling, 2026-09-20) join the existing
`::timeout:<n>` and `::no-cache`. Each answers a different question about the
same command, and each turns a non-zero exit from a composition failure into a
value:

| Suffix | Value | Reads as |
|---|---|---|
| `::ok` | boolean: the command exited `0` | `when: "tree_is_clean"` |
| `::exit-code` | number, or null for an allowed timeout: the exit status | `when: "lint == 2"` |
| `::result` | object: `{ ok, code, stdout, stderr }` | `when: "diff.ok"`, `{{ diff.stderr }}` |

```yaml
tree_is_clean: "$(git diff --quiet)::ok"
lint:          "$(just lint)::exit-code::timeout:120"
diff:          "$(git diff --quiet)::result"
```

Rules:

- **At most one result suffix per expression.** Two is a parse error naming
  both. A result suffix combines with `::timeout:<n>` and `::no-cache` in any
  order.
- **No suffix means what it means today**: the value is trimmed stdout, and a
  non-zero exit fails the composition.
- **The command text is untouched.** Discovery and execution use the same early-resolved command bytes
  after argument interpolation and existing command parsing. A suffix is never part of the
  approved bytes, so approving `$(cmd)` covers `$(cmd)::ok`.
- **Only an exit status is forgiven.** A command that is not on the host, is
  blacklisted, or is denied still fails as it does today. A timeout still fails
  too (there is no status to report) unless `--allow-shell-timeout` is set, in
  which case `::ok` is `false`, `::exit-code` is `null`, and `::result` has
  `ok: false` and `code: null`.
- **Values are typed.** They follow the whole-value rule, so `::ok` is a real
  boolean and `::result` a real object whose members are reachable with dotted
  access. `stdout` and `stderr` are trimmed the way plain stdout is today.
- **The cache stores the whole outcome, not a suffix-specific value.** Within
  one execution context and timeout, the per-compose cache is keyed on the command, so `$(cmd)` and `$(cmd)::result` in one document run the
  command once. The cached entry therefore has to hold status, stdout, and
  stderr, not just the text. Reuse is valid only under equivalent execution
  context and timeout; a timeout override cannot silently reuse an outcome
  obtained with a different deadline. Concurrent identical requests must share
  one execution. `::no-cache` neither reads nor populates this cache. An unsuffixed
  reader of a cached non-zero result still fails.
- **`$schema` sees the expanded value.** A property declared `boolean` validates
  against what `::ok` produced. Existing validation defers values still holding
  `$(…)`; require a concrete post-expansion validation test, including a type
  mismatch, rather than assuming deferral alone validates the new result types.
- **Supported locations** are a top-level frontmatter value and the lifecycle
  `set` form above. The body's `::shell` directives splice text and are not
  affected.

**A result suffix applies to every shape a `$()` can take** (author's ruling,
2026-09-20). Darkmatter parses the expression and launches each executable
itself, so these rules do not depend on the host's shell. The shapes are a single
command, an `&&`/`||` chain, and a ternary; `|` and `;` are rejected at parse
time today ("Shell pipes are not allowed", "Command chaining (;) is not
allowed") and stay rejected.

| Shape | `code` | `stdout` / `stderr` |
|---|---|---|
| single command | its exit status | its streams |
| `a && b`, `a \|\| b`, longer chains | the status of the **last command that actually ran**, which is what `$?` would hold | the streams of every command that ran, in execution order, joined as an unsuffixed chain joins stdout today |
| ternary, selected branch is a command or chain | as the rows above | as the rows above |
| ternary, selected branch is a **literal** | `0` | `stdout` is the literal's text; `stderr` is empty |

`ok` is `code == 0` in every row.

The literal row is deliberate. The ternary is what ran: its condition was
evaluated, it selected a branch, and nothing failed, so `0` is the truthful
status and the selected text is its output. `$( has_command('just') ? just lint
: 'skipped' )::result` is `{ ok: true, code: 0, stdout: "skipped", stderr: "" }`
on a host without `just`.

So `$(git diff --quiet ref || echo handled)::ok` is `true` when the fallback
ran, exactly as the unsuffixed form yields `handled` today. An author who needs
to know *which* command in a chain failed gives each its own property.

Three things are never turned into a value. A ternary whose **condition**
raises is an expression error, as today. A command ended by a **signal** has no
exit status and stays an execution failure; no number is invented for it. A
**user interruption** keeps its cancellation outcome and must never surface as a
recoverable `ok: false`. The existing rule that a `$()` must contain at least one
real command in an executed position is unchanged.

This lives in Darkmatter's frontmatter shell expansion. Update
`darkmatter/docs/inline/fm-shell-expansion.md` and the suffix handling in the
language server (completion, hover, and the unrecognized-suffix diagnostic,
which should now list all five).

**Lifecycle execution boundary.** Expand only whole-value shell expressions
in mapping-based `set` assignments when that action actually executes, after
its guard passes. They are discovered but never executed during preflight or a
lifecycle-free dry run. A `set` in `start` is legal even when the document has
`initialize`; a top-level bootstrap shell value in such a document remains
forbidden. Reject shell-bearing assignments in `initialize`, including dead
branches and catches, and retain the runtime prohibition in early
blocked/failure/finalize routes before shell approval.

Resolve command arguments at preflight using early-binding values and retain
those approved bytes. Do not re-interpolate them against later runtime writes.
All mapping values read the existing pre-write state, and no destination is
updated unless every value succeeds. Executed external effects cannot be rolled
back; document that limitation. A later action may read the typed result after
the complete mapping commits. Nested objects and arrays are not a new recursive
shell-execution surface.

Each executed lifecycle assignment gets a fresh result-cache scope; duplicate
commands within that assignment can share work. A command observed in `start`
and again in `success` must execute again, since the agent may have changed its
answer. Approval reuse and result reuse are separate lifetimes.

**Reader's note:** the earlier `git push --dry-run` example conflicted with the
unchanged blacklist. A read-only `git diff` example demonstrates result capture
without suggesting that suffixes grant permission to run a blocked command.
**`capture:` on the lifecycle `shell` action is deferred** (author's ruling,
2026-09-20). `set: { tree: "$(cmd)::result" }` is the one way a lifecycle step
reads a command's result in this fix. A `capture:` option would be a second
state-write contract needing rules of its own (destination collisions, reserved
names, value lifetime, timeout contents, and its relationship to `no_error`,
which forgives a non-zero exit where `capture` would report one). It is additive
and can be specified when a real prompt reads awkwardly without it.

### R10. The prompt-authoring guide reads this spec's state

`prompts/_prompt.md` warns agents about each open defect here, and gates every
warning on this specification's state. References identify it by
`2026-09-20-lifecycle-handoff-gaps`, independently of its lifecycle directory.
A warning renders while its finding id is absent from `fixed:` and the
specification has not reached `completed`. Replace the guide's current literal
active-path dependency as part of this requirement; resolve the directory
identity through the existing file-resolution facilities. A missing or ambiguous
specification must be reported, not treated as proof every defect was fixed.

- When a requirement lands, add its finding id to `fixed:` in the same change.
  That is the whole maintenance step; the guide needs no edit.
- Only the author closes the review cycle and moves this specification. After
  completion, remove the obsolete guide section as documentation maintenance;
  an implementation agent leaves the specification ready for review.
- D1 and D2 are recorded the same way as the F findings. The guide currently
  lacks warnings for the loop-engine documentation discrepancy and terminal
  rendering defect; add relevant outstanding warnings or explicitly document
  why they do not affect prompt authoring. Do not claim every defect is gated
  unless the guide actually covers it.

### R11. The loop gate's lifecycle concerns see the loop's ambient values

The `loop:` block's notification fields and stack resolve `_loop_count`,
`_loop_is_first`, `_loop_is_last`, `_loop_last_output`, and
`_loop_last_exit_code` with the values the gate's condition sees: the iteration
that just finished. The `lifecycle.md` example runs as written.

### R12. Interpolation literals convert in every file (D1)

A `{{{ … }}}` literal composes to `{{ … }}` whether or not the file containing
it has any span to evaluate, under `md compose` and under Claudine, directly and
when transcluded. Remove the real span `prompts/_prompt.md` carries only to work
around this.

### R13. A `description` renders as written (D2)

In the run header, and in any diagnostic that quotes author text, an underscore
inside an inline code span is literal, an underscore that does not open valid
emphasis is literal, and no markup token such as `</i>` reaches the terminal.
File names in a description, and variable names in an error, read exactly as
the author wrote them. The component that renders the header's Markdown has not
been traced; find it first, because the same defect is likely wherever that
component is used.

## Spike Before Planning

R1 and R5 both change how much context work a run does, and context capture is
where `2026-08-01-faster-compose` spent its effort. Measure before planning, and
turn what is learned into budgets the plan has to meet.

**What R1 adds.** One fresh working-tree observation per composition run that
mentions a git-state property, where today there is one per invocation. A first
reading, on macOS, in this repository (11,742 tracked files): a cold
`sniff repo dirty-files` process takes about 0.06 s, and `git status --porcelain`
about 0.02 s. Against a composition run that launches an agent, that is noise.
It may not be noise everywhere, which is the point of the spike.

**What R5 adds.** A requirements scan over the transclusion tree instead of the
root page. The tree is already walked for shell discovery, so the question is
whether the two walks can be one.

Measure, with `--perf`, the sequence performance integration tests, and the
invocation work counters (`launch_context_constructions` and the sniff request
counters), on **macOS, Linux, native Windows, and WSL2** — the `os` skill records
workspace walks of 20–77 s per test on WSL2, so that host decides the answer:

1. a 10-iteration loop whose body mentions `ctx.dirty_files`, before and after;
2. a 5-hop proxy chain in which every page mentions a git-state property;
3. a 10-step sequence with a parallel group of 4;
4. a run that mentions **no** git-state property, which must incur no additional
   git-state requests and no statistically meaningful regression, since only
   properties found on the page should be evaluated;
5. a prompt that transcludes ten partials, for R5.

Report each as wall time and as counter deltas. The outcome is either "within
noise on every host, proceed", or a named budget (for example, "a composition
run's context capture stays under N ms on WSL2") that the plan carries as an
acceptance criterion. Use repeated warm and cold runs and record host, sample
count, and variance;
a single wall-clock sample is not a budget. Optimize discovery and evidence reuse
within a run if capture is costly. Do not refresh only when Claudine knows it
performed a mutation: another process can change the working tree, so that
optimization would violate the required freshness contract.

## Open Questions

None. Every question raised while drafting and during review has an author's
ruling dated 2026-09-20, recorded in the requirement it governs:

| Ruling | Where |
|---|---|
| `ctx` is evaluated once for each composition run | F1, R1 |
| a serial group evaluates once per task; a parallel group shares at launch and refreshes on re-entry | R1 |
| `ctx` requirements are read from the root page before `initialize`; a transcluded file is the one exception | R1 |
| `current` refreshes once per event; revisit when `prep` lands | R1 |
| one post-checked loop engine; the pre-checked one is deleted without a deprecation period | R7 |
| result suffixes `::ok`, `::exit-code`, `::result`, on every `$()` shape, a literal branch yielding `code: 0` | R9 |
| `capture:` on the lifecycle `shell` action is deferred | R9 |

Two spellings considered for the exit status and not adopted were `$(cmd > $?)`,
which the parser rejects as output redirection and which would put non-command
text inside the approved bytes, and `$(cmd) > $?`, which borrows redirection for
something that is not redirection.

## Verification

Add coverage at the lowest level that can observe each behavior. Every
reproduction above is already a minimal fixture.

- **R1:** one row per vehicle in the F1 table (`proxy`, `sequence`, `loop`,
  `retry`, `resume`), each asserting the later run observes the staged file.
  Run the `proxy` row both ways — with and without the source mentioning the
  property — and assert the same value, which pins first-mention independence.
  Assert a stable property (`ctx.cwd`, `ctx.repo_root`) is identical across the
  same hop. Require one prepared-context construction per new run and no
  repeated identity, topology, or host discovery; volatile observations may
  increase once per requested evidence group per run. Add a transclusion row
  asserting a `::file`d partial shares its parent's value.
- **R2:** a looping source that proxies from `success`. Assert one iteration,
  the target's `initialize` fired, and the chain has exactly two entries. Add a
  row proving a non-committed request leaves the chain untouched. Cover
  `failure` and `finalize` as sources as well.
- **R3:** the five-row matrix in F3 as a table-driven test, asserting the
  approval set contains the executed bytes in every row.
- **R4:** extend `level2_lifecycle_success_stack_error_downgrades_keeps_success_comm`,
  or add an L1 row beside it, to assert a non-zero exit and one rendered error.
  Add rows for a retried attempt, a `sequence` step under `fail_fast: true`, and
  a loop iteration.
- **R5:** the F5 table as a test, asserting Claudine's output equals
  Darkmatter's for the same tree. Add a row where the partial is two levels
  deep, and one where the `::file` reference is itself interpolated.
- **R7:** the F6 table as a table-driven characterization test against
  `execute_loop_with_lifecycle`, asserting both the iteration count and the value
  each body observes. After the deletion, a workspace search for `execute_loop`
  that is not `execute_loop_with_lifecycle` finds nothing, and
  `claudine::composition` exports one loop entry point.
- **R8:** a two-step sequence whose first task's document proxies. Assert the
  target ran, the step succeeded once, and step 2 ran after it.
- **R9:** table-driven, in Darkmatter: each suffix against a command that exits
  `0`, one that exits non-zero, and one that times out with and without
  `--allow-shell-timeout`, asserting value and type; two result suffixes on one
  expression fail to parse; one row per shape in the R9 table, including an
  `||` fallback that runs and a ternary selecting a literal branch; a suffix
  combined with `::timeout` and `::no-cache`
  in both orders; `$(cmd)` and `$(cmd)::result` in one document execute once. In
  Claudine: a `start` stack that assigns with `set` and gates its next item on
  the value; a row proving the command was approved in preflight under its bare
  text; a row proving the same assignment in `initialize` is still rejected; the
  typed `::result` assignment exposing `ok`, `code`, `stdout`, and `stderr`
  to a later `when:`.
- **R11:** the F8 document, asserting the gate message renders the iteration
  number on every pass, including the pass that ends the loop.
- **R12:** a file containing only a literal, composed directly and transcluded,
  through `md compose` and through `claudine compose`; four rows, one expected
  output.
- **R13:** the D2 description as a rendering test asserting the exact text, plus
  the F8 diagnostic asserting the root is reported as `_loop_count`.
- **R10:** compose a prompt that transcludes `prompts/_prompt.md` with `fixed: []`
  and again with one id listed, and assert that id's warning is the only one
  that disappears.
- **R6:** after the workarounds are removed, the PR flow still completes every
  route against a stub provider: clean tree; dirty tree with `uncommitted=commit`,
  `uncommitted=abort`, and no answer without a TTY; starting on the base branch;
  a blocked push under each of `on_failure=ask`, `fix`, and `stop`; a fix that
  verifies on a later attempt; a fix that never verifies.
  `shipped_prompt_contract` stays green.

Additional acceptance coverage:

- Context: serial tasks observe earlier mutations; parallel tasks share initial
  repository evidence but retain their own provider identity. Add recovery rows
  for the parallel-group ruling (a sibling's retry sees its siblings' changes), a
  branch change, a body-only property captured before `initialize`, and the
  include exception captured after it. Count volatile observations separately from stable
  discovery rather than requiring every sniff counter to remain unchanged.
- Handoffs: refused resolution and cycle checks leave the chain unchanged;
  an adopted target that fails still remains recorded. Sequence task teardown
  and output publication happen once, including failure with `fail_fast: false`.
- Shell results: failed mappings leave all destinations unchanged; false guards
  and dry runs execute no lifecycle assignment; successive events recapture a
  command's changed result. Test missing executables, denial, blacklist,
  cancellation, timeout policy, concurrent cache reuse, and post-expansion type
  errors. Approved bytes remain unchanged after an intervening runtime `set`.
- Transclusion: local overlays, conditional and nested includes, cycles, fenced
  examples, literal spans, and requirements used only in included lifecycle
  frontmatter retain their existing parsing and policy boundaries.
- Guide: test relocation by directory identity, completed status, and missing
  or ambiguous lookup, using temporary copies rather than changing this spec's
  `fixed:` list. Rendering: assert preserved text after stripping terminal
  styling, at narrow widths and with color disabled, not exact ANSI sequences.

Use canonical `just test`/`just test-l2` recipes and nextest for affected package
areas. L1 CLI fixtures use Claudine CLI’s
[`CliProcessFixture`](../../cli/tests/common/mod.rs), which isolates spawned
commands from the developer’s environment, plus private repositories and audio
spools, fake providers, and no real network or messaging. Real-terminal tests
must not gain focus. Include macOS, Linux, native Windows, and WSL2 evidence;
use portable test helpers for exit codes and timeouts instead of assuming POSIX
shell commands work on Windows. Use only disposable local repositories for commit or push rehearsals; no real
provider, remote push, or audible notification is needed. This review itself changes only the
specification, so implementation tests are acceptance work, not review evidence.

## Out of Scope

- A flow-control action that runs another prompt and returns (`prep`). The PR
  flow needs a second invocation after a fix because a proxy chain visits a
  document once and every commit goes through `commit.md`. Nothing here changes
  that, and cycle protection is working as designed.
- Changing which commands the built-in shell blacklist rejects.
- Network policy for `branch_exists_on_remote()` and `pr_list()`.
- Making `current` available to a document body. It is lifecycle-only by design,
  and in a body it is an undefined variable. Whether static tooling should warn
  about that reference belongs to `2026-09-17-remove-strict-mode` R7.
