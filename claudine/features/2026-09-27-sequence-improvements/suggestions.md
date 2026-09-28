# Sequence improvements

## Context

A review/fix cycle for one feature took nine review iterations, each started
by hand. Automating it as a Claudine sequence (`prompts/review-loop.md`)
worked for the review loop, but the draft needed changes to two shipped
prompts, a rename of a prompt's own frontmatter key, several dry-run cycles,
an unrolled 21-step list to express a five-cycle loop, and it could not run a
multi-phase plan at all. Each item below names the gap behind one of those
costs, shows what had to be written, and shows what would be written instead.

Owner decisions recorded on 2026-09-27, which the items below assume:

- every loop behaves identically, or as close to identically as possible,
  whichever primitive carries it
- the loop condition is checked after the primitive has run, as the document
  loop does today; `when:` decides whether a primitive runs, `loop:` decides
  whether it runs again
- leaving one iteration stays `skip`; a new `break` flow-control action leaves
  the loop
- a new `prep` flow-control action calls another prompt and returns
- built-in state gets tidied so userland names cannot collide with it

## 1. `loop:` on every executable primitive

**Problem.** A document, a group, and a sequence are the three executable
primitives, and only the document can loop. A cycle of two or more documents
therefore has to be unrolled by hand: the draft is 21 steps for a cap of five,
and the cap is a sentence in the description because nothing can read it.
Worse, a looping document placed in a sequence silently runs one iteration.

**Current behavior**, reproduced with a five-line document, a one-step
sequence, and a fake agent that only records the prompt text it receives:

```yaml
# count.md
---
n: 1
loop:
    until: "n >= 3"
    action: "increment(n)"
---
Iteration {{ n }}
```

```yaml
# seq.md
---
sequence:
    - name: counting
      prompt: "@count.md"
---
```

| Invocation | Prompts the agent received |
|---|---|
| `claudine compose @count.md` | Iteration 1, Iteration 2, Iteration 3 |
| `claudine sequence @seq.md` | Iteration 1 |

The sequence reports the step as succeeded and exits 0. This is the defect
that cut the plan implementation short in the rehearsal: `implement-plan.md`
is a looping document, and it ran phase 1 and stopped.

**Proposed.** One `loop:` block, one set of semantics, accepted on all three
primitives. A document's loop keeps working when the document is a sequence
task. A group carries the block on its own object. A sequence document carries
it at the root beside `sequence:`, and presence of `sequence:` is what decides
that the root block belongs to the sequence rather than to a Markdown body.

```yaml
---
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> spec
    agent: string(required) -> agent
max: 5
sequence:
    - name: implement
      prompt: "@prompts/_implement/implement-plan.md"     # its own loop runs the phases
    - name: review-repair
      group:
          execution: serial
          loop:
              until: "frontmatter(latest_review, 'ready') == true"
              max: "{{ max }}"
          tasks:
              - prompt: "@prompts/_reviews/feature-review.md"
              - prompt: "@prompts/_implement/implement-suggestions.md"
              - shell: git add .
              - prompt: "@prompts/commit.md"
---
```

Semantics to pin in the spec, each answered the way the document loop already
answers it:

- **Condition timing.** Checked after each iteration, against the state that
  iteration ran with, so a loop always runs at least once. Zero iterations is
  what a false `when:` on the primitive means (item 2).
- **`action` target.** A document's loop mutates its frontmatter. A group's or
  sequence's loop mutates the runtime `set` layer, so the same `increment(n)`
  spelling works and later tasks read the new value.
- **`outputs` shape.** A group already pushes one entry. A looped group pushes
  one entry per iteration, each shaped as a group entry, so `last(outputs)` is
  still the most recent task and a consumer can count iterations.
- **`initialize` once, `start` per iteration.** As the document loop does. For
  a group, each task's own `initialize` fires on its first turn only.
- **`max` exhaustion is its own outcome**, distinct from "condition met" and
  from `fail_fast` failure. The document loop has no exhaustion event today
  and the guide works around it with a frontmatter counter; this is where to
  add one, reported in the summary and reflected in the exit code.
- **Top-level sequence loop is defined as a group.** A root `loop:` on a
  sequence document is observably identical to wrapping every step in one
  serial group with that block. Writing the equivalence down gives one code
  path and keeps the group the only place loop semantics are specified.

## 2. `when:` decides execution, `loop:` decides continuation

**Problem.** A sequence step has no condition of its own. The only way to make
one conditional is to put `skip` in the *target* document's `initialize`. The
draft therefore put the loop's stop rule inside two shipped prompts that are
also run standalone, guarded by an `in_loop` flag whose only job is to say
"you are inside a sequence".

**What had to be written**, in a prompt that is also composed directly:

```yaml
# feature-review.md
initialize:
    stack:
        - when: "in_loop && previous_review && frontmatter(previous_review, 'ready') == true"
          action: [skip]
```

**Proposed.** `when:` on any executable primitive decides whether it runs at
all. A false `when:` records the primitive as skipped, appends nothing to
`outputs`, runs no iteration of its loop, and leaves the target document
untouched.

```yaml
sequence:
    - name: review-2
      prompt: "@prompts/_reviews/feature-review.md"
      when: "frontmatter(latest_review, 'ready') != true"
```

`setup:` cannot serve this purpose: a failing setup marks the task failed,
which under `fail_fast` ends the run.

## 3. `break` leaves the loop

**Problem.** `skip` leaves one iteration and nothing leaves the loop. A
skipped step is a success, so a sequence cannot distinguish "the loop reached
its goal" from "a review reported a recurring finding class and a human should
look before another cycle runs". Both exit 0 with every remaining step skipped.
A bash loop can exit 2 for the second case.

**Proposed.** A `break` flow-control action, valid in every event, that exits
the innermost enclosing loop from wherever it fires, including a task's
lifecycle inside a looped group, the way `proxy` reaches the coordinator from
inside a document today.

```yaml
# in the review prompt's success stack
- when: "frontmatter(review, 'recurrence') == true"
  action:
      - warn: "a finding class recurred; stopping for a human"
      - break: { reason: "recurring finding class" }
```

- A broken loop is a third outcome next to "condition met" and "`max`
  reached", named in the summary with its `reason`, and available to the
  process exit code.
- `break` outside any loop is a typed error at parse time, like `skip` outside
  `initialize`.
- `skip` keeps its meaning: this iteration, or this primitive, is done.

## 4. `prep` calls a prompt and returns

**Problem.** `proxy` is a tail call: the target takes over the run. There is no
way for a document to run another prompt as a subroutine and continue. Every
commit in this repository goes through `prompts/commit.md`, which commits only
what is staged, so every repair needs a separate `shell: git add .` step before
it. Three steps where one was meant, and the extra one is the step that makes
dry-run unsafe (item 6).

**Proposed.** A `prep` flow-control action that composes and runs another
prompt document, then returns control to the caller's event stack when the
prep completes successfully.

```yaml
# commit.md
initialize:
    stack:
        - when: "stage == 'all'"
          action:
              - prep: "@prompts/_git/stage-all.md"
        - when: "length(current.staged_files) == 0"
          action: [skip]
```

Rules that follow from "call, not tail call":

- nothing flows back except what the prep wrote to disk, unless its final
  stdout is appended to `outputs`
- a prep that fails fails the caller's current event; `fail_fast` then applies
  as for any failure
- the prep's shell work is approved at the caller's preflight, transitively,
  as a sequence already does for every referenced prompt
- `prep` chains obey the `proxy` cycle rule and hop limit
- `prep` accepts the same `with:` overlay as `proxy`

`prep` overlaps with a task's `setup:` stack. The clean split is that `setup:`
holds actions and `prep` runs a document, so `setup: [{ prep: "@x.md" }]` is
the natural spelling when a step needs a whole prompt run first.

## 5. State management: built-ins in a namespace

**Problem.** Inside a sequence, `state`, `previous`, `next`, `outputs`, and
`sequence_id` are injected at the frontmatter root and outrank every other
layer. `previous` and `next` are natural names for a prompt's own keys. The
review prompt had `previous:` meaning the previous review file. Inside the
sequence it became the previous step's state object, with no warning, and
failed only when `basename()` received an object on step 5. The `loop`
parameter collided with the lifecycle key too, but that one failed loudly.

**What happened:**

```yaml
# feature-review.md
previous: "{{ iteration > 1 ? decrement_file_index(review) : null }}"
# inside a sequence, `previous` is {id: "commit-1", name: "commit-1", index: 4, …}
```

**Proposed.** Built-in state lives under namespaces; userland stays flat.

```md
{{ seq.state.name }}  {{ seq.previous }}  {{ seq.outputs }}  {{ seq.id }}
{{ loop.count }}      {{ loop.is_first }} {{ loop.is_last }}
{{ previous }}        <!-- still the document's own key -->
```

- the current `_loop_*` ambient values fold into `loop.*` at the same time
- preflight refuses a document that authors a key under a built-in namespace,
  naming the file and the key, since preflight already loads every referenced
  prompt
- `state`, `previous`, `next`, `outputs`, and `sequence_id` at the root are
  kept for one release as aliases that warn, then removed
- the name-coercion rule (`{{ seq.state }}` renders the state's `name` in
  string context) carries over unchanged

Two related gaps found while rehearsing, which the same pass should close:

- **Caller setters are invisible to shell tasks.** A sequence `shell:` string
  can read `state`, `doc.*`, `ctx.*`, and `env.*`, but not a value the caller
  passed as `key=value`, and authored step state cannot carry one in either
  (`agent: "{{ agent }}"` renders empty). A shell task therefore cannot be
  written generically against the sequence's own inputs.
- **Path functions disagree on their anchor.** `decrement_file_index` and
  `increment_file_index` re-anchor a relative path on the document's
  directory, while `file_exists`, `frontmatter`, and `absolute` resolve it
  against the repository root, and `absolute` accepts only a path that exists.
  The review prompt's previous-review path broke on this and now uses
  `replace(spec, basename(spec), …)` instead.

## 6. A dry-run that does not run `shell:` tasks

**Problem.** `--dry-run` executes `$(…)` expansions and `shell:` tasks for
real, because their output may feed composition. That is true of `$(…)`. A
`shell:` *task's* stdout only lands in `outputs`, which dry-run already leaves
empty. Rehearsing the draft sequence staged the working tree five times, and
each run had to be followed by `git reset`.

**Proposed:**

```text
$ claudine sequence --dry-run @prompts/review-loop.md …
 [3/21] stage-1   rendered (dry-run): would run `git add .`
```

Render the task, print the approved bytes, and push an empty entry onto
`outputs`. A `--dry-run-shell` opt-in keeps today's behavior for a sequence
that genuinely needs the side effect during rehearsal.

## 7. Prompts that opt out should `skip`, not `stop`

**Problem.** `commit.md` ends its `initialize` stack with `stop` when nothing
is staged. `stop` ends the stack; the agent still launches. In the rehearsal,
every commit step after the loop had closed launched an agent that found
nothing to do: four wasted launches per run.

**Proposed.** `skip` in `commit.md`, and a note in the lifecycle guide that
`stop` inside `initialize` is almost never what an opt-out wants. With `when:`
from item 2 the sequence can also decline the step before the prompt is
composed.

## Evidence

The rehearsal that produced these items ran the draft sequence in a scratch
repository with a fake agent on `PATH` that recognizes each prompt by its
heading and edits files the way an agent would. With the spec unimplemented:
the plan step ran phase 1 of 2 and stopped (item 1), review 1 was not ready,
the repair and commit ran, review 2 was ready, and every later review and
repair step skipped without launching. With the spec already implemented: the
plan step skipped, review 1 was ready, and the loop closed. The commit steps
after closure each launched an agent (item 7).

The draft is in the working tree as `prompts/review-loop.md`, with `in_loop`
gates in `prompts/_reviews/feature-review.md`,
`prompts/_implement/implement-suggestions.md`, and
`prompts/_implement/implement-plan.md`, and the `previous_review` path fix in
the first. `claudine sequence --dry-run` over a completed feature passes every
step, and the claudine prompt contract, provenance, acceptance, and route-drift
tests pass. The loop reproduction in item 1 reruns in seconds.
