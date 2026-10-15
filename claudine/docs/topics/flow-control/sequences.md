# Sequences in Claudine

A **sequence** runs an ordered list of *steps*. Each step composes and executes
one unit of work, and later steps can see what earlier steps produced.

![sequence overview](sequence-overview.png)

```sh
claudine sequence <file> [key=value ...]
```

## Container grammar

A task can declare a nested `sequence:` list or a file reference using the
same `-> offset` and `::operator` suffixes as a sequence source. It is
exclusive with `prompt`, `shell`, `side_effect`, `group`, and `task`.

```yaml
sequence:
    - name: validate
      sequence:
          - name: check
            shell: just check
      failure:
          - warn: Validation failed
```

The grammar accepts these containers; **nested container execution is
planned**. Until the container runner is available, execution rejects a
nested sequence explicitly before any child runs. An empty nested list is
valid grammar. Only an inline list can carry container lifecycle events
on its task; a reference carries those events in its referenced document.

Groups are concurrent: omit `execution:`. Both `execution: parallel` and
`execution: serial` are removed grammar; ordered work uses a nested
`sequence:`. Every group leg requires a unique, nonempty string `name`.
Group `variables:` may not shadow `legs`, `succeeded`, `failed`, or
`cancelled`. Group `fail_fast` is a boolean, defaults to `true`, and rejects
null or quoted boolean strings.

Task `setup:` and `teardown:` are removed. Put `initialize`/`start` and
`finalize` on the prompt document or wrap work in an inline sequence.
A sequence document's `fail_fast:` and `--fail-fast` are removed; failure
handling belongs in lifecycle events. No `CLAUDINE_FAIL_FAST` value is
injected into child sessions. The `cancelled:` lifecycle event accepts an
empty stack as an explicit cancellation opt-in; null is an error.
Cancellation dispatch and container recovery are **planned** with the
container runner.

## The two meanings of `prompt`

Read this before anything else. `prompt` means different things at two adjacent
levels, and the distinction is load-bearing:

| Level | Value | Meaning |
|---|---|---|
| **Document** (frontmatter root) | prose string | Flips the whole sequence to **inline-compose**: the composed `prompt` is the agent prompt, and the provider's reply replaces the document body on disk. |
| **Step / task** | file reference | Names a **prompt document** to compose and execute for that step. |

```yaml
---
prompt: "Summarize {{ seq.name }}."   # ← document level: prose, inline-compose
sequence:
    - name: alpha
      prompt: "@prompts/review.md"      # ← step level: a file reference
---
```

The two never occur at the same level, and their value types differ, so
Claudine can always tell them apart. The name was kept because `prompt` is
entrenched in inline-compose throughout the CLI and docs.

## Steps and tasks

There is one execution model. A **task** is the atomic executable unit; a
**step** is an entry in the `sequence:` list — its state, plus optionally the
task fields.

A task declares **exactly one** executable field:

| Field | Runs |
|---|---|
| `prompt: <file-ref>` | The referenced Markdown document (inline-compose if *that* document is configured for it, otherwise a normal compose) |
| `shell: <string \| string[]>` | One or more shell commands |
| `side_effect: <action>` | One Darkmatter side effect, written as a one-verb lifecycle dictionary in short or long form (`side_effect: { set_frontmatter: ["state.md", "status", "done"] }`) |
| `group: <group-ref \| inline group>` | A [group](#groups) |
| `sequence: <list \| file-ref>` | An ordered nested container; execution is planned |
| `task: <file-ref>` | An externalized `kind: task` file |

Declaring two is a typed error naming both fields.

A step with **no** executable runs the **default action**: it composes the
source document's Markdown body. A step **with** one runs that instead of the
body. A source with no body — a `kind: sequence` YAML file invoked directly —
simply requires every step to carry an executable.

Optional task fields: `name`, `params:`
(user setters passed to a `prompt` document), `timeout:` (per shell command),
`operation:`, `flow:`. A field that cannot affect the chosen executable — say
`params:` on a `shell:` task — is a typed error, not a silent no-op.

```yaml
---
sequence:
    - name: alpha            # no executable → composes this document's body
      topic: parsing
    - name: run-tests
      shell: just test
    - name: review
      prompt: "@prompts/review.md"
      params:
          topic: "{{ seq.topic }}"
    - name: build-phase
      group: build-group@my-catalog.yaml
---
Work on {{ seq.topic }}.
```

## Step state

Every step gets a `state` object. A bare string is shorthand for `{name: <string>}`:

```yaml
sequence:
    - one
    - two
    - one
```

normalizes to:

```json5
[
    { id: "one",   name: "one", index: 1, count: 3, is_first: true,  is_last: false },
    { id: "two",   name: "two", index: 2, count: 3, is_first: false, is_last: false },
    { id: "one-2", name: "one", index: 3, count: 3, is_first: false, is_last: true  },
]
```

Generated fields — you may read them, never author them:

| Field | Meaning |
|---|---|
| `id` | Unique per step. The dasherized `name`, suffixed `-2`, `-3`, … on collision. |
| `index` | One-based position. |
| `count` | Total steps. |
| `is_first` / `is_last` | Boundary flags. |
| `run_id` | One collision-resistant token per invocation, shared by nested sequences. |

Object steps carry whatever else you want:

```yaml
sequence:
    - name: Bob
      age: 32
    - name: Sally
      age: 36
```

### The `seq` runtime root

Read the active step through `seq`, separately from frontmatter:

| Expression | Value |
|---|---|
| `seq.name`, `seq.age` | Authored fields of the current row |
| `seq.id`, `seq.index`, `seq.count` | Unique ID and one-based position |
| `seq.is_first`, `seq.is_last` | Boundary flags |
| `seq.previous`, `seq.next` | Shallow neighboring rows; `null` at the boundaries |
| `seq.outputs` | Results committed so far |
| `seq.run_id` | Invocation token |

In string text, `The person is {{ seq }}; next is {{ seq.next }}` renders the
rows' names. A whole-value expression still yields the typed row object.
Standalone `compose` and `inline-compose` expose `seq.outputs`; their other
`seq` members are `null`.

These roots are read-only: `set` cannot assign `seq`, `group`, or `loop`.
They are not inserted into frontmatter or saved by inline-compose.
`fm.seq`, `fm.group`, and `fm.loop` always read authored document properties.

The old runtime reads `state`, `previous`, `previous_state`, `next`,
`next_state`, `outputs`, `sequence_id`, `step`, `total_steps`, `is_first`, and
`is_last` raise typed errors naming the corresponding `seq` expression.
For example, replace `{{ state.name }}` with `{{ seq.name }}` and
`{{ last(outputs) }}` with `{{ last(seq.outputs) }}`. A document that declares
its own `previous: null` still reads `previous` as ordinary frontmatter;
property presence, including supplied null values, preserves that behavior.
Explicit `fm.previous`, unrelated members such as `ctx.state`, and quoted
strings never request a retired runtime global.

**Planned container execution:** each nested sequence, group, or loop
replaces its entire same-kind root and restores the enclosing root on exit.
A sequence's own events have null step positions and expose its own outputs
and invocation token. Inner empty outputs never fall back to outer outputs.

```mermaid
flowchart LR
    A[Outer root] --> B[Inner complete root]
    B --> C[Restore outer root]
```

A group's own terminal events and loop gate expose `group.legs`, a map from
leg name to `{name, outcome, output, err}`, and the declaration-ordered
`group.succeeded`, `group.failed`, and `group.cancelled` lists. Outcome is
`succeeded`, `failed`, `cancelled`, or `skipped`; skipped legs appear only in
the map. Failed output can contain partial stdout; an unadmitted leg has
null output and retains its cancellation cause. These result fields are
empty during initialize/start/blocked, and absent inside a leg, where only
`group` variables are visible. Group result construction is available to the
binding layer; dispatch of container events remains planned.

### Keys you cannot author as state

Rejected with a typed error naming the offending key:

- executable and task keys — `prompt`, `shell`, `side_effect`, `group`, `sequence`, `task`,
  `setup`, `teardown`, `params`, `timeout`, `operation`, `flow`
- generated state keys — `id`, `is_first`, `is_last`, `index`, `count`, `previous`, `next`, `outputs`, `run_id`
- runtime roots — `seq`, `group`, `loop`

## Sources

`sequence:` accepts a static list, a typed expression, a shell expansion, or a
file reference.

### Source grammar

```text
sequence: <file-ref> [-> <offset.path>] [::<operator>(<args>)]
```

- `<file-ref>` keeps its full powers — relative paths, `@` magic, `&`
  repository-root, `^` repository-scoped, `~`, `vault:`, and `{{ }}`
  interpolation. Interpolation resolves **before**
  the suffix is parsed, and resolution is relative to the **authoring
  document's** context: implicit paths check its directory before the repository
  root, while explicit `./`/`../` paths stay pinned to its directory.
- `->` is a dot-notation **offset** into the document. YAML/JSON/JSON5 only; a
  typed error on JSONL/NDJSON, whose root is always the list.
- `::` is an **operator** on the resolved list. Exactly one per reference.
- Expression (`{{ … }}`) and shell (`$( … )`) sources produce lists directly and
  take no suffix.
- A stored literal token (`"{{!data:v1:…}}"`, the form Claudine uses to save
  text an agent wrote) is refused with `SequenceInvalid`. It is data, and data
  must not choose which steps run. See
  [Values an Agent Writes](../frontmatter-properties.md#values-an-agent-writes).

The suffix parser respects quoted arguments, so a path containing a space or an
`@`, and an operator argument containing a comma, all survive.

### Operators

| Operator | Effect | Fails when |
|---|---|---|
| `map(from, to)` | **Renames** `from` to `to` (the original key is removed) | an item lacks `from`, or is a scalar — the error names the item index |
| `name(from)` | **Copies** `from` into `name` (the original is retained) | same as `map` |
| `template(expr)` | Computes `name` per item with a Darkmatter expression; the item's top-level fields shadow globals, each replacing the same-named frontmatter key whole (an item `config: {a: 1}` over frontmatter `config: {b: 2}` makes `config.b`, `fm.config.b`, and `fm['config']['b']` all `null`) | the result is null or empty |

```yaml
sequence: things.yaml -> colors.data                          # names become "1", "2", "3"
sequence: things.yaml -> colors.data::map(color, name)
sequence: things.yaml -> colors.data::name(color)
sequence: things.yaml -> colors.data::template(color + '-is-great')
```

### Data formats

YAML, JSON, JSON5, JSONL, and NDJSON all work and normalize identically.
JSONL/NDJSON take operators but never an offset:

```yaml
sequence: list.ndjson::map(color, name)
```

### String list formats

A string source is classified in this order: Markdown list markers (ordered or
unordered) → multiple lines → tabs (TSV) → commas (CSV, quote-aware) → spaces →
otherwise a single-item list. Whitespace-only entries are dropped; CSV/TSV
parsing is delimiter-aware, so quoted delimiters and escaped quotes survive.

```yaml
sequence: "{{ ctx.dirty_files }}"    # typed arrays bypass classification entirely
sequence: "$(ls -1 src)"             # the command must be approved at preflight
```

### Strictness depends on provenance

- **Authored for sequences** — inline lists and formal `kind: sequence`
  documents — stay **strict**. An object must carry a string `name`; a scalar
  must be a string. An omission here is a typo worth catching.
- **Foreign data** — arbitrary data files reached via offset/operator,
  JSONL/NDJSON, `{{ expr }}`, `$( shell )` — is **lenient**. Numbers and
  booleans coerce to string names, and a nameless object receives its one-based
  ordinal (`"1"`, `"2"`, …).

`null` items are a typed error either way.

### Empty lists

A static empty `sequence: []` is an authoring error. A **dynamic** source that
resolves to zero steps is a graceful no-op: Claudine prints a "resolved to 0
steps" notice to stderr and exits `0`. A clean repo legitimately makes
`{{ ctx.dirty_files }}` empty.

### Formal sequence documents

A YAML file using the `sequence:` property is a *formal* sequence document. It
may add `template:` (values merged into every step, with the item's own fields
in scope) and `$schema:`.

```yaml
kind: sequence
sequence:
    - name: blue
      color: blue
      rank: 5
template:
    desc: "{{ color }}({{ rank }})"
$schema:
    color: string(required) -> the color being evaluated
    rank: number(required)  -> a 1-5 ranking, 5 being best
    desc: string(required)
```

Normalization runs in a fixed order: scalar shorthand (`- blue`) expands to
`{name: blue}` first, then `template:` fills in, then the generated fields
(`id`, `index`, `is_first`, …), and `$schema` validates the finished state last.
So `desc` is ordinary authored state by the time `id` and `index` are made, and
a `template:` default reaches a scalar step exactly as it reaches an object one.
A schema failure names the step index, its id, and the failing property path.

A `template:` value is `any`, not just a string: a **string** is interpolated
with the item's fields in scope, and any **other** value (`rank: 5`, a list, a
map) is a literal default. A default never overwrites a value the item already
carries.

> **One shape, both entry modes.** `template:` and `$schema:` behave identically
> whether the file is **referenced** (`sequence: formal.yaml`) or invoked
> **directly** (`claudine sequence formal.yaml`) — both routes share a single
> normalization pipeline. In particular the root `$schema` is always the *step
> state* schema; on a directly invoked document it is not also validated as that
> document's own frontmatter schema. Parity is pinned by
> `a_formal_document_normalizes_identically_through_both_entry_paths` in
> `cli/tests/l1/sequence_sources_cli.rs`.

The retired external `kind: sequence` + `list:` form is **gone**. Use
`sequence:`.

## Execution

Sequence execution is two phases.

### Phase 1 — Static preflight

Before anything runs, Claudine walks the **entire task graph**:

- **Dynamic sources resolve exactly once.** The resulting step list is a
  snapshot; it never re-evaluates mid-run, even if the environment changes.
- **Every referenced document loads transitively** — `kind: group` files,
  `kind: group-catalog` entries, `kind: task` files, and every `prompt:`
  document. Reference cycles are rejected with the complete chain.
- **Schemas validate and missing properties aggregate** across all steps and all
  referenced prompt documents, with one interactive collection pass.
- **Every shell command is approved before the first step** — `shell:` tasks,
  lifecycle `shell` actions and whole-value `set` `$( … )` values,
  and the `::shell` and `$( … )` commands of every referenced document, across
  every step and group, *including branches that may never run*. How a command
  with a placeholder is approved depends on its task's position:

  - **The first task** (the first step's task, or every leg of a first-step
    group) resolves each command now, under the launch environment and the
    caller's values, and approves those bytes. They are what runs.
  - **Every later task** approves each command with a placeholder only by its
    typed shape (`just clean --out {{ string }}/build`), even when its values
    are known now, because an earlier task may change them. An `exact` rule for
    today's bytes does not cover it. Position is static: skipped predecessors,
    retries, and resume never make a task first.

  A placeholder reading a value an earlier step produces — `outputs`, `loop`,
  or, inside a group, `group` — is a **late value**: it is not read at
  preflight, and its command is approved only by its shape in any task. A step
  value or a prompt task's `param` derived from one is late too. Every other
  value a command reads (`seq`, template values, caller values, `fm.*`,
  `ctx.*`, `env.*`) is read now, and a `null` fails preflight before step 1. A
  lifecycle global that never exists at approval (`err`, `timing`, `current`,
  or `group` outside a group) is a typed preflight error
  (`SequenceShellLateBinding`), even beside a late value. `fm.group` still
  reads the document.

  When a task is reached, every late value its commands read is read and
  checked before any of them runs. A `null`, an array or
  object, or a value its type refuses fails that task
  (`SequenceShellLateValue`, never showing the value); earlier steps' effects
  stand. A task command approved by shape runs directly with structured
  arguments: late holes use the checked step-entry values; other holes read
  current state when their command runs. Whole chains
  meet the runtime blacklist before any segment starts. Prompt documents and
  their transclusions pass the same late-value check before task execution; each
  execution retains its own checked values. Sequence-wide document validation
  discovers shells without executing them. See
  [Pre-flight Checks — the first task and later tasks](../pre-flight-checks.md#the-first-task-and-later-tasks).
  Redirections preserve their authored order: `2>&1 > /dev/null` keeps stderr
  in the task output for the next step, while `> /dev/null 2>&1` discards both.
  Lifecycle shells use the same order over the parent process's inherited streams.

  A first-task `shell:` or prompt lifecycle command whose values are known
  offers exact approval and, for valid placeholders, shape approval. Saved
  exact rules take priority; shape approval fills each containing argument at dispatch. A first
  task reading a late value still requires a template.

  The group scheduler applies one late-value barrier across the entire group
  before any command runs. Referenced documents
  retain the checked late inputs; ordinary values still read current state at
  command dispatch. Nested sequences and container lifecycle execution are
  planned. Use `seq.outputs` for prior results.

  Each step and group leg later captures its own environment. Before any of
  its stages runs, a task resolves the commands preflight resolved to bytes
  again under that environment. Identical text runs; changed text is refused
  (`SequenceShellApprovalChanged`, naming the command and the variables that
  changed), and nothing in the task runs. Approving `deploy {{ env.TARGET }}`
  as `deploy staging` never authorizes `deploy prod`. See
  [Composition — Approval under a later document's map](../composition.md#approval-under-a-later-documents-map).
- **Git working state is observed once for the whole walk.** Static preflight
  is one discovery run: every referenced document (and every file it includes)
  that names a Git fact such as `ctx.staged_files` reads the same observation.
  Each step then observes Git state again when it runs, as its own composition
  run, so a step still sees what earlier steps changed.
- **Provider and model resolve once**, producing a per-step target vector.
  This plan reads the launch environment. When a step or group leg starts, it
  captures its own environment, and the model is selected again from it: a
  later step whose environment sets `MODEL=b` launches with `b`, even if the
  launch environment named another model. The provider stands as planned,
  because no environment variable selects it. A model given with `--model`, or
  confirmed on the review screen, also stands.

A preflight failure aborts the sequence. Preparation
never degrades to best-effort.

An include that a prompt's `initialize` would create must already exist at this
boundary. A missing include remains a typed preflight failure, with guidance to
create it **before starting the sequence**. An earlier task in the same sequence
cannot create it in time because no task starts before static preflight passes.
Live standalone `compose` and `inline-compose` support initialization-created
includes through [staged preparation](../composition.md#documents-that-declare-initialize).
A sequence task composes its prompt before its own initialization, so a change
to an existing include during that initialization does not alter the prompt
already composed for that task.


### Phase 2 — Just-in-time composition

Each step composes **at its turn**, not up front. At every step boundary
Claudine checks the interrupt flag, **captures the step's environment**,
**re-reads the live source file from disk**, re-layers state, composes,
validates, executes, appends the output, and folds in any mutations. The step's
composition, its task, its prompt document, its loop iterations, retries,
handlers, and `finalize` all read that one environment; a group leg captures
its own. See [Composition — One Environment per Document](../composition.md#one-environment-per-document).

State layering, lowest precedence to highest:

1. source document frontmatter (live, from disk)
2. user setters (`--set`, shorthand `key=value`)
3. accumulated runtime mutations (what `set` writes)

The lexical `seq` root travels beside these frontmatter layers.

Layers 1 and 2 are authored, so their `{{ … }}` and `$( … )` spans are
templates. Runtime mutations and lexical roots are produced by the run, so they reach composition as
**data**: a value holding `{{ ctx.repo }}` stays that text, through arrays and
objects too. Evaluated task `params` and group `variables` are data for the
same reason.

**Live-disk chaining.** Because each step re-reads the source, an
inline-compose sequence's body write-backs — and any frontmatter an agent edits
mid-run — are visible to later steps. Mid-run *external* edits take effect too;
that is both the feature and the hazard.

What is *not* re-derived at a step boundary: the step list. A
`sequence: $(ls)` that would enumerate differently mid-run does not change the
plan under execution.

**Mid-run failures.** A just-in-time composition failure is a failure of *that
step*: recorded in the summary and stopping the sequence. Container lifecycle
recovery is planned; document-level `fail_fast` is rejected.

## The output array

Every executed task pushes its final stdout onto `seq.outputs`, in execution order.
The previous task's output is `{{ last(seq.outputs) }}`.

- A task with no stdout pushes an empty string, so entries stay aligned with
  executed tasks.
- A **parallel group** pushes a single entry that is itself an array — one
  string per task, in **declaration** order, never completion order. The overall
  shape is `(string | string[])[]`.
- Single-document `compose` / `inline-compose` use the same key, so a prompt
  written with `{{ last(seq.outputs) }}` behaves identically standalone and
  mid-sequence.
- The generated `seq.outputs` member is read-only. An authored root property
  named `outputs` is ordinary frontmatter data.
- Entries are **data**. An entry is stored raw and typed, and reaches
  predicates, functions such as `last(seq.outputs)`, logs, and the next step's
  prompt exactly as the task printed it. If a task prints `{{ ctx.repo }}`,
  the next step's `{{ last(seq.outputs) }}` renders the characters `{{ ctx.repo }}`,
  not the repository name, and a printed `$(cmd)` is never offered for
  approval.

**What an entry contains.** The task's captured, undecorated stdout. Terminal
status rendering, stderr, lifecycle messages, color bars, and provider protocol
records are never included. For a prompt task it is the provider's final
assistant text; for a multi-command shell task, the commands' stdout
concatenated in declaration order; for a side effect, its returned text or an
empty string. One trailing transport newline is removed; other whitespace is
preserved.

**Lifecycle timing.** `initialize` and `start` see only prior entries. `success`
sees this run's output appended. `failure` sees no new entry — it has `err`
instead. `finalize` sees whatever has accumulated at that point.

Failed and interrupted tasks append no entry, except that each slot of a
completed parallel group stays positionally present and holds whatever partial
stdout that task captured.

## Mutating state with `set`

`set` is the state-mutation side effect. It works anywhere the lifecycle action
grammar applies — lifecycle events and `side_effect:` tasks:

```yaml
side_effect:
    set:
        ready: "{{ true }}"
```

- Lifecycle YAML accepts only the mapping form `set: {property: value}`. A
  one-entry mapping, a multi-entry mapping, and an empty mapping are all valid.
- a whole-value `{{ expr }}` span carries its typed value — the example above
  writes boolean `true`, not the string `"true"`
- all values in one mapping read the same pre-write snapshot, and the complete
  mapping commits atomically; use two consecutive `set` actions when the second
  assignment must observe the first
- it writes to the **in-memory runtime layer**, never to disk. That is what
  distinguishes it from `set_frontmatter`, which targets a file.
- the value it stores is the evaluation's result, so later steps read it as
  data, not as a template
- top-level keys only; reserved keys are rejected
- outside a sequence it still works, mutating the state visible to later
  lifecycle actions and loop iterations in the same run

Do not confuse this lifecycle YAML action with the capability and loop-control
API spelled `set(key, value)`. That positional API remains unchanged; only
Claudine lifecycle YAML uses `set: {property: value}`.

Mutations are visible to subsequent lifecycle actions, loop iterations, and later steps. `seq`, `seq.previous`, and `seq.next` remain immutable
runtime views; runtime mutation flows through ordinary frontmatter keys.

## Groups

A group bundles one or more tasks under a name, for semantic naming, reuse, and
concurrency. **A group cannot be executed directly** — it runs only as a
sequence task.

### Defining a group

Three places, all behaviorally equivalent once loaded:

**Inline:**

```yaml
sequence:
    - name: build-phase
      group:
          name: ICR
          tasks:
              - name: lint
                shell: just lint
              - name: tests
                shell: just test
```

**A `kind: group` file** — one group per file, normal file-reference grammar:

```yaml
sequence:
    - name: build-phase
      group: my-group-file.yaml
```

**A `kind: group-catalog` file** — a root `groups:` list. One group is
referenced `{name}@{file}`, name first, matching Darkmatter's named-type import
convention. Magic refs compose: `build-group@@catalogs/all.yaml` splits on the
first `@` after the group name.

```yaml
sequence:
    - name: build-phase
      group: build-group@my-catalog.yaml
```

### Group fields

| Field | Meaning |
|---|---|
| `name` (required) | Identifies the group |
| `description` | Optional longer prose |
| `variables` | Group-scoped values exposed to tasks on the `group.*` global, discarded when the group completes |
| `tasks` (required, ≥1) | Concurrent legs, each with a unique, nonempty string `name` |
| `max_parallel` | Cap on simultaneous tasks. Integer ≥ 1. |
| `fail_fast` | Boolean, default `true`; sibling cancellation is planned |
| `operation` / `flow` | Defaults for the group's tasks; task-level values override |

A single-task group is legitimate — it is the honest way to get a semantic name
or `variables` scoping around one task.

A group leg may itself declare a group or a sequence task. Nested sequence
execution is **planned**; the current runner rejects a nested sequence before
child work starts.

**Group `loop` is blocked.** Iteration commit semantics are unratified, so a
group carrying `loop` is rejected at preflight with a typed, actionable error
rather than left to implementation judgment.

### Task references and lifecycle events

An external `task:` reference is exclusive with the other executables and is an
immutable expansion of a `kind: task` file. Overrides at the referencing site
are rejected.

Task `setup:` and `teardown:` are removed grammar, even when their values are
empty or null. For a prompt task, put `initialize`, `start`, and `finalize` on
the prompt document. For ordered non-prompt work, use an inline `sequence:`;
its lifecycle execution is **planned**.

```yaml
sequence:
    - name: review
      prompt: review.md
```

The referenced `review.md` can declare its own lifecycle:

```yaml
---
start:
    - info: Starting review
finalize:
    - info: Review finished
---
Review the current changes.
```

A `side_effect` task resolves file-touching effects relative to the document
that authored the task. An externalized `task:` or `group:` file therefore
uses its own directory and repository, even when referenced from another
repository.

### Shell tasks

```yaml
sequence:
    - name: JustOne
      shell: just test
    - name: Multiple
      shell:
          - just test
          - just lint
```

Every command gets a **30s** default timeout, overridable with `timeout:` as a
duration (`45s`, `2m`) — a bare integer is rejected because the unit is part of
the grammar, and `0s` is rejected rather than read as unbounded. A task passes
when every command exits `0`. Stdout is concatenated in declaration order.

### Ordered work

Groups always run concurrently. `execution: serial` and `execution: parallel`
are rejected with replacement guidance. An inline `sequence:` expresses ordered
work; nested sequence execution is **planned**.

### Parallel groups

```yaml
group:
    name: fan-out
    max_parallel: 3
    tasks:
        - name: a
          prompt: "@prompts/a.md"
        - name: b
          prompt: "@prompts/b.md"
```

- **Scheduling** — all tasks launch at once unless `max_parallel` is set, in
  which case they admit in **declaration order** as slots free up.
- **Snapshot isolation** — every task receives the same state snapshot and the
  same `outputs` view, taken when the group starts. Live-disk re-reads happen
  between *steps*, never between sibling tasks, so a sibling's mid-group file
  edits are invisible inside the group.
- **State merge** — mutations fold in **task-declaration order**, never
  completion order. Disjoint keys merge cleanly; when two tasks wrote the same
  key the later-declared task wins and Claudine warns on stderr naming the key
  and both tasks.
- **Failure policy** — the grammar accepts group `fail_fast` (default `true`).
  Canceling siblings after a failure and dispatching `cancelled` lifecycle
  events are **planned**. The current runner lets started legs complete and
  fails the group if any leg failed. A failed leg's partial stdout remains in
  its output slot.
- **No interactivity** — a late required property becomes a task *failure*
  inside a parallel group, never a TTY prompt: N concurrent tasks cannot share
  one terminal. Serial contexts keep the standard interactive collection.
- **Write-back collisions** — two tasks whose `prompt` documents are
  inline-compose targets of the same file (or of the sequence source itself) is a
  typed **preflight** error. Racing write-backs are never legal.
- **Process isolation** — each task runs as an independent child with
  spawn-level environment and working directory. Claudine never mutates process
  environment or CWD on behalf of one task while siblings run. A leg's
  `shell:` commands, its prompt lifecycle `shell` actions, and the
  `::shell` and `$( … )` commands its documents run all start from the
  environment that leg captured, so `echo $X` prints the value that
  `{{ env.X }}` reads in the same leg.
- **Timeouts and guards** apply to each task independently. Ctrl+C fans out to
  all running children; the group is marked interrupted and the sequence exits
  `130`.

### Reading concurrent output

Parallel tasks render with line-interleaved color bars: each task takes a color
from a fixed cycling palette, announces itself with a `▶ <task-name>` header,
prefixes its lines with a colored vertical bar, and closes with a footer
carrying its outcome and duration. Lines interleave in arrival order —
attribution comes from the bar and the label, not from ordering.

Serial work renders in the same geometry with an **invisible** bar, so the
stream does not lurch sideways when execution switches between serial and
parallel.

Degradation is by design: without color, the header and footer still name the
task; without Unicode, `▶` becomes `>`. Task and provider data stay on stdout;
headers, footers, status, and warnings go to stderr. One synchronized sink
writes complete frames, so sibling writers cannot tear an ANSI sequence.

## Fail-fast, exit codes, and dry-run

An unhandled step failure stops the sequence. Document `fail_fast:` and the
`--fail-fast` flag are removed grammar. Claudine does not inject
`CLAUDINE_FAIL_FAST` into child processes. Container lifecycle recovery is
**planned**.

| Code | Meaning |
|---|---|
| `0` | Every executed step succeeded (or a dynamic source resolved to 0 steps) |
| `1` | At least one step failed |
| `130` | Ctrl+C interrupted the sequence, or `Esc`/Ctrl+C left the provider review screen before any step started |

`--dry-run` performs the **full preflight**, then just-in-time-composes every
step against the *initial* state — empty `outputs`, no runtime mutations —
without launching a provider and without inline-compose write-back.
Late-binding references render exactly as a first-step composition would see
them.

> **`--dry-run` still executes shell work.** `$( … )` expansions and `shell:`
> tasks run for real, because their output is what composition needs. A dry-run
> of a sequence containing `shell: just commit` will commit. This is inherited
> behavior, called out here because it is genuinely surprising.

## Requiring caller input

A document declaring `required` schema properties it does not define is asking
the caller for them. Well-designed sequences front-load that interaction, and
Claudine helps two ways:

1. Preflight approves **all** shell expansion across the entire sequence before
   the first step starts — including commands in conditional blocks that will
   never run. No exceptions. Once the sequence starts, nothing stops to ask.
2. Missing required properties aggregate across every step and every referenced
   prompt document into one interactive collection pass.

Claudine cannot guarantee that runtime mutations satisfy every later step. If
just-in-time validation still finds a required property missing, the normal
behavior applies: interactive collection in a serial TTY context, a typed error
otherwise. Arrange for earlier tasks to `set` what later tasks require.

**Each step is its own composition, so each step reaches its own completion
verdict.** A step that ends with a `required` property missing or a value of the
wrong type fails at that step and stops the sequence — the schema
describes the document and its lifecycle, not the sequence as a whole. A
property that is only meant to accumulate across steps must therefore not be
declared `required`. Values supplied transiently — a caller `key=value`, step
state, or a `proxy.with` overlay — count toward the verdict exactly as they
count everywhere else, and are still never written to the source. See
[Composition — Completion Verdict](../composition.md#completion-verdict).

## Operations

Any step may define `operation`, whose value becomes the `OPERATION`
environment variable for that step — the same dimension the `--operation` CLI
switch sets, and a reportable field in Claudine's logs.

## Planned container execution

The grammar supports nested sequence and group tasks, inline container
lifecycle events, and group `fail_fast`. Execution of nested containers,
container recovery, and cancellation event dispatch are **planned**. Group
`loop` remains rejected. Persisted checkpoint/resume is not available.
