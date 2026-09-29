---
created: 2026-09-27
amends: ./spec.md
origin_branch: fix/wt-ux
status: proposed
---

# Amendments: provider selection per task, and `params` → `with`

This file holds changes the owner agreed to on 2026-09-27 while `spec.md` was
being refined on another branch. It exists so they can be carried over
without conflicting with that work.

**How to apply.** Merge each amendment below into `spec.md` on the refining
branch, adjusting the wording to the spec's current state. Then delete this
file in the same commit. Section and criterion names are given by content,
not line number, because the spec is still moving. Where the spec's
**Decisions** / **Open questions** split applies, keep to it: amendment D is
an owner decision in principle, and its sub-questions stay open until the
owner answers them.

## A. Correct the provider probe note in **Evidence**

The **Rehearsal of the draft sequence** subsection says a probe "showed that
a sequence honors each prompt task's own `agent:` and a step's
`params: { agent: … }` when no provider flag is given". That is true only
**when the sequence document itself resolves an agent**. Replace it with:

> A follow-up probe with stub providers showed that each prompt task launches
> on its own document's `agent:`, including a value passed through
> `params: { agent: … }`, and that a command-line `--<provider>` overrides
> every task. This holds only when the sequence document's own `agent`
> resolves. A sequence document with no `agent` stops before any task runs:
> it opens the review screen on a terminal, or fails with
> `AgentResolutionFailed` without one, even when every task names a provider.
> Step status lines report the planned target rather than the provider that
> launched, so a mixed-provider sequence prints the wrong name.

Verified on 2026-09-27 with a debug build of this branch. There were two
sequences: one with no document `agent`, and one with `agent: claude`. Each
had a step passing `params: { agent: codex }` and a step whose prompt declares
`agent: opencode`. The first failed before any launch. In the second, step 1
launched `codex` but reported `succeeded (via Claude)`, and step 2 selected
`opencode`.

## B. Add a section **Provider selection per task** (before **Migration**)

### What happens today

A provider is chosen twice.

1. **Before the run**, one target is planned from the **sequence
   document's** `agent`/`model` and the caller's flags and setters
   (`cli/src/commands/wrap/sequence/mod.rs`, Phase 1a/1b). If that doesn't
   resolve, a terminal gets the review screen and anything else gets
   `AgentResolutionFailed`.
2. **At every launch**, the provider is rebuilt from the composed frontmatter
   of the document about to run (`select_rebuilt_provider`,
   `cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs`). The
   order is: explicit flag, then that document's `agent` (prompt frontmatter,
   then `params`, then caller setters), then the planned target as a
   fallback. This rebuild is deliberate: it lets a retry or a `proxy` target
   move `agent:`.

So each task already runs on its own `agent`. The planned target is used only
for tasks whose document names none. What's wrong is everything built on the
planned target:

- **The gate.** A sequence document with no `agent` blocks the run, or forces
  the review screen, even when every provider-launching task names one.
  `prompts/review-loop.md` hits this.
- **The review screen.** Every row starts on the planned default. A choice
  made for a task whose document names an `agent` is silently discarded at
  launch.
- **Reporting.** The step status line, and the `env.AGENT`/`env.MODEL`
  values the step's prompt is composed with
  (`cli/src/commands/wrap/sequence/jit.rs`, `step_env_overrides`), come from
  the planned target, not the provider that launched. This is the defect
  behind acceptance criterion 10. The launched process's own
  `AGENT`/`MODEL`/`YOLO` are already correct, because the launch rebuild
  replaces them (`apply_target_env_overrides`).
- **Late failure.** A task naming a provider that isn't installed is refused
  only at its turn, after earlier tasks have already run.

### Requirements

- **Predict each task's provider during preflight.** For every
  provider-launching task, preflight evaluates the task's `agent`/`model` in
  the same order the launch uses, in the early-binding context shell approval
  uses. The launch-time rebuild stays authoritative. The prediction exists to
  gate, report, and fail early, not to replace it.
  - If a task's `agent` can't be evaluated early (it reads `outputs`, a
    runtime-mutated value, or a per-pass `state.loop.*` / `state.seq.*`
    value), the task is reported as *decided at launch*. It does not gate
    the run, and the launch-time rule applies.
- **Gate per task.** The review screen opens, or the run fails without a
  terminal, only when some provider-launching task has no predictable
  provider of its own **and** the sequence document's `agent` doesn't
  resolve. A task whose predicted provider is invalid or not installed fails
  preflight with a typed error naming the task, before any task launches.
- **An honest review screen.** Tasks with a predicted provider of their own
  appear as read-only rows showing that provider. Only the remaining tasks
  can be edited, and a choice made there is what launches.
- **Report what launched.** Step status lines, composition-time
  `env.AGENT`/`env.MODEL`, the dry-run table, and the sequence report use the
  task's predicted provider, which the launch confirms. This covers acceptance criterion 10. Keep that criterion and
  point it here.

Resulting precedence for a prompt task, highest first:

| Source | Scope |
|---|---|
| `--<provider>` / `--provider`, `--model` | every task |
| the task document's composed `agent`/`model`: its frontmatter, then `with` (today `params`), then caller setters | that task |
| the sequence document's `agent`/`model`, or a review-screen choice for a task with no hint of its own | that task |
| favorite / picker | that task |

A body step composes the sequence document, so its own hints are the
sequence document's. A `group` has no `agent`: each prompt task inside it
resolves separately. `shell` and `side_effect` tasks launch no provider.

Because caller setters sit above `params`, `claudine sequence doc.md
agent=codex` forces every prompt task onto Codex. That is existing setter
layering, and the docs should say so.

## C. Add acceptance criteria

Number them after the spec's last criterion.

1. **Tasks name their providers.** A sequence document with no `agent` runs
   against stub providers, with stderr redirected. One task passes
   `agent: codex` through `with` (or `params`) and one runs a prompt that
   declares `agent: opencode`, inside a looped serial group. The run starts
   with no picker or error, each stub receives only its own task's prompts
   on every pass, and each status line names the stub that ran.
2. **Precedence.** A table-driven test covers each row of the precedence
   table once, including `agent=gemini` overriding a task's `with.agent`.
3. **Early failure.** A task naming a provider that isn't installed fails
   preflight, naming the task, before any task launches.
4. **Review screen.** With a terminal, and some tasks without a hint of their
   own, the screen shows tasks that have their own provider as read-only
   rows. A choice made on an editable row is the provider that launches.
5. **Decided at launch.** A task whose `agent` reads `outputs` is reported as
   decided at launch, does not gate the run, and launches on the value
   composed at its turn.

## D. Rename task `params` to `with` (owner decision in principle)

**Decision.** A task's `params:` becomes `with:`. Both carry values for the
document about to run. `with` is already that mechanism for `proxy`
(`docs/topics/flow-control/flow-control-reference.md`, "Passing values with
`with:`"), and this spec already gives `prep` the same `with`. Afterwards
Claudine has one word for "frontmatter values for the document I'm about to
run".

This is a hard rename with no alias, in line with the repository's
no-deprecation stance. A task that still says `params:` gets a typed error
that suggests `with:`.

The two already share their core contract:

| | `proxy.with` | task `params` |
|---|---|---|
| Target | the target document's top-level frontmatter | the prompt document's top-level frontmatter |
| Precedence (lowest → highest) | target frontmatter → `with` → caller setters | prompt frontmatter → `params` → caller setters (`lib/src/composition/sequence/task/mod.rs`, `layered_overrides`) |
| Evaluation | once, in the source; the target receives resolved values | in the sequence's state at the task's turn; the child receives resolved values |

**Open questions for the owner** (the recommendation is not a decision):

1. **File-path provenance.** Task `params` are recorded as caller input from
   the sequence file's location (`cli/src/commands/wrap/sequence/task_run.rs`,
   `params_source_context`), so a relative file value resolves from the
   sequence's directory. `docs/topics/composition.md` ("Caller File
   Provenance and Materialization") says `proxy.with` values are *not*
   recorded as caller input. If one keyword keeps both rules, the same
   relative path resolves differently depending on where it's written.
   *Recommendation:* one rule for every `with`. Values resolve from the
   location of the file that authored them, which is the current `params`
   behavior, because authors write paths relative to the file they are
   editing.
2. **Shape rules.** `proxy.with` rejects interpolated keys and a
   whole-mapping expression (`with: "{{ payload }}"`), with typed errors.
   *Recommendation:* apply the same rules to task `with` and rename the
   `LifecycleProxyWith*` errors so they aren't proxy-specific.
3. **Reserved keys.** `with` joins the task keys a step can't use as state
   (the "Keys you cannot author as state" list in
   `docs/topics/flow-control/sequences.md`), and `params` leaves that list. A
   sequence that uses `with` as a state key breaks. That's unlikely, but it
   should be stated.

**Carry-through.** Every `params` in `spec.md` is renamed, including the
migration sketch YAML, the overlay-precedence sentence ("including task
`params` …"), and amendments B and C. So is every mention in
`prompts/review-loop.md`, the shipped prompts, the claudine skill, and the
topic pages that name `params`. That includes `sequences.md` and
`docs/cli/sequence.md`.

## E. Topic-page note (per **Specs and docs**, rule 2)

When B is merged, add this **Planned** note to
`docs/topics/flow-control/sequences.md` under **Phase 1 — Static preflight**,
after the "Provider and model resolve once" bullet:

> **Planned:** preflight will predict each task's provider from its own
> document's `agent`, the same way the launch chooses it. A sequence whose
> tasks all name providers then runs without a document-level `agent` or a
> review screen. A task naming a provider that isn't installed will fail
> before any task launches, and status lines will name the provider that
> actually ran.
