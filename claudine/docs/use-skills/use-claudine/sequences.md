# Sequences and groups

`claudine sequence workflow.md` runs a list of tasks from one document, in
order. A task can be:

- a prompt;
- a shell command;
- a side effect;
- a nested sequence;
- a **group** of tasks that run at the same time.

Every shell command in the whole tree is approved before the first task
starts.

```yaml
---
agent: claude
sequence:
  - name: research                 # no executable: composes this document's own body
  - name: review
    prompt: "@prompts/review.md"
    when: "length(seq.outputs) > 0"
    with: { topic: "{{ seq.name }}", agent: codex }
  - name: checks
    group:
      name: checks
      fail_fast: false
      max_parallel: 2
      tasks:
        - { name: lint,  shell: just lint }
        - { name: tests, shell: just test }
  - name: mark-done
    side_effect: { set_frontmatter: ["status.md", "state", "done"] }
finalize:
  - info: "Workflow settled"
---
Body used by the `research` step.
```

## Tasks

Each task has exactly one executable:

| Key | Runs |
|---|---|
| `prompt: <file-ref>` | a prompt document, through the full compose pipeline and lifecycle |
| `shell: "cmd"` or `["cmd1", "cmd2"]` | shell commands (default `timeout:` 30s each) |
| `side_effect: { verb: … }` | one lifecycle side-effect verb |
| `sequence: [...]` or `<file-ref>` | a nested sequence; it may have its own `loop:` and lifecycle events |
| `group: {...}` or a file, or `name@catalog.yaml` | tasks that run concurrently |
| `task: <file-ref>` | a `kind: task` file |
| *(none)* | compose this document's body |

Other task keys:
- `name`;
- `when` (false means the task is skipped, not failed);
- `with` (overrides for prompt tasks only);
- `$schema`, `$state`, `template` (inline nested sequences only; see below);
- `timeout`;
- `operation`.

A relative path in a task (`prompt: ./draft.md`, `task:`, `sequence:`,
`group:`, and a `with:` value the prompt types as a file) resolves from the
folder of the file that wrote the task, never the launch folder. A shared
`sub/child.yaml` finds its own neighbors whether it runs directly, is named
by a root (`sequence: sub/child.yaml`), or is nested as a task.

## Document inputs, step types, and templates

A sequence document has three declaration keys, the same in YAML and
Markdown, run directly or referenced:

| Key | Describes |
|---|---|
| `$schema` | the document: its properties and the caller's inputs (setters) |
| `$state` | every step's state in this document's inline list |
| `template:` | values merged into every step (a string is an expression) |

```yaml
kind: sequence
$schema:
    spec: file(required;eager) -> the spec to complete   # a caller input
$state:
    color: string(required)                              # every step needs it
template:
    desc: "{{ color }} for {{ spec }}"                   # step fields shadow document values
sequence:
    - { name: blue, color: blue }
```

- `claudine sequence colors.yaml spec=work/spec.md` supplies the input
  (before or after the file). Without it, a terminal asks once for every
  missing input of the root and each referenced sequence document; anything
  else fails with one report and starts no step. A required `$state` field
  never asks. A value the root's `initialize` writes (`- set: { spec: … }`)
  counts as supplied for the root and every document it references; a dry
  run fires no `initialize`, so preview such a file with the setter.
- `$schema` used to type the steps in a YAML sequence. That is now
  `$state`; no warning flags an old file, so rename the key.
- Each sequence owns its declarations; nothing is inherited. A parent's
  `$state` judges a nested container step's own fields, never its children.
  An inline nested `sequence:` task may declare its own `$state` and
  `template:`, plus a `$schema` that checks the values it sees at its turn
  (after earlier tasks' `set` writes, after its `when:`, before its
  `initialize`), as its file reads then, so an earlier task's edit to the
  declaration applies. A failed check is that task's failure, and it never asks
  the caller.
- `$state`/`template:` belong beside an inline list. Beside a file reference
  or a dynamic source, or on a prompt/shell task, they are an error. No
  setter, `with:`, or `set` may write them.
- A template, a root's or an inline container's, reads its document's
  prepared values: `$schema` defaults (for an absent or `null` value),
  derived properties, and converted values, never the raw file text.
- Templates are filled once, when the task graph is built. Writes from the
  root's `initialize` are seen, later writes and on-disk edits are not.
- A referenced sequence document's `template:` reads its own properties,
  setters, and the root's `initialize` writes, never the referring
  document's properties. To parameterize a shared step list, write the value
  in the referrer's `initialize` (`- set: { research_prompt: … }`). A dry
  run fires no events, so preview it with the value as a setter.
- A referenced sequence document's events and steps read its prepared
  values (`$schema` defaults and conversions, then setters, then runtime
  writes), and its `$schema` judges them each time it runs: a nested task,
  group leg, or `kind: task` file when the container starts (after its own
  `initialize`; again each loop pass and retry), a root
  `sequence: child.yaml` at each root step. An earlier `set` that breaks the
  contract fails the document, naming the property and the file, before any
  of its rows run.
- Direct, root-reference (`sequence: child.yaml`), nested, group-leg,
  task-file, and container-`proxy` runs agree: a step authored in a document
  reads that document's values (its `when:`, its pre-flight `shell` binding,
  its run state), never the invoking root's. A directly run document's own
  events read its prepared values too (`initialize` under the caller's
  values, each `start` again with every write so far), and pre-flight binds
  `shell: "echo {{{ who }}}"` under `who: string(default(ada))` as `echo ada`.
- An empty `sequence: []` still validates its `$schema` and `$state`.
- A dry run judges inputs, `$schema`, and `$state`; an inline container's
  `$schema` is judged only at its turn in a real run.

## Results

- **`seq.outputs`** holds the completed results. A prompt or shell task
  adds a string, a nested sequence adds an array, and a group adds a
  dictionary keyed by leg name. Skipped tasks add nothing.
  `last(seq.outputs)` is the previous result.
- **The current row** is described by `seq.name`, `seq.index`, `seq.count`,
  `seq.is_first`, `seq.is_last`, `seq.previous`, `seq.next`, and
  `seq.run_id`.
- **Nested sequences** keep their own `seq`. Results publish when the
  container finishes.

## Groups

- **Isolation.** Every leg starts from a snapshot of the state taken when
  the group starts. Their in-memory writes are merged in declaration order.
  File and shell effects are shared and never undone.
- **`fail_fast`** (default `true`). One unhandled leg failure stops new legs
  from starting and cancels running legs that declare `cancelled:` (even
  `cancelled: []`). The group then publishes nothing. With
  `fail_fast: false`, failed legs' results are kept and the run continues.
- **Inside the group's own events,** `group.legs`, `group.succeeded`,
  `group.failed`, and `group.cancelled` are available.
- `execution:` is removed: groups are always concurrent. Use a nested
  `sequence:` when steps must run one after another.

## Failure, recovery, and exit codes

- A sequence stops at the first unhandled task failure. The container's
  `failure` handler can `retry` or `proxy` it. `resume` isn't valid on a
  container.
- Sequences, groups, and documents all loop the same way, with `loop:` (see
  [lifecycle.md](lifecycle.md#loop-repeat-a-document)).
- Exit codes:
  - `0`: success;
  - `1`: unhandled failure;
  - the `break` code, for a looping root sequence that ends with `break`;
  - `130`: Ctrl-C, on every OS. `cancelled` and then `finalize` run once,
    and cleanup goes from children to parents.

## Choosing providers per task

A sequence has no single provider. For each prompt task, the provider comes
from the highest of these that sets one:
1. the CLI provider flag;
2. the caller's `agent=`;
3. the task's `with.agent`;
4. the prompt document's `agent`;
5. the sequence document's `agent`;
6. a review screen, on a terminal only.

An unusable hint fails before any task starts.

## Previewing

```bash
claudine sequence workflow.md --dry-run          # report on stderr, each composed prompt on stdout
claudine sequence workflow.md --dry-run --json   # one JSON report on stdout
```

A dry run runs nothing: no provider, `shell:`/`side_effect:` task, `$(…)`
expansion, lifecycle event, loop pass, or question. It reads the whole tree
and reports every problem under its task (`composition.provider_unresolved`,
`composition.no_executable`, a missing `prompt:` file, a missing input, a
file with no `sequence` key (`composition.sequence_undefined`, which a real run
refuses too), …),
then exits `2` if any is an error or warning, else `0`. On a terminal a
choice the run would ask about is a warning; without one it is an error.
A dry run never exits `1`; a refused command line (`--json` without
`--dry-run`, `--dry-run-shell`) exits `2` with a usage message and no
report. In `--json`, match findings by `code`, never by message text.

A real run checks the same things at preflight. If one stops it before the
first task, it prints the same report under `Sequence preflight found …; no
task ran`, ends with `The run stopped at: …` (unless a row already says
it), and exits `1`; lifecycle `err.*` describes that returned error, not the
report's first row. If preflight passes, new warnings (a step's own
`agent:`/`model:`, a near-miss key) print once before task one.

## Removed spellings

| Removed | Use instead |
|---|---|
| `kind: sequence` + `list:` | `sequence:` |
| `execution:` on a task or group (`serial` or `parallel`) | omit it; a nested `sequence:` for ordered work |
| sequence `fail_fast:`, `--fail-fast` | `fail_fast` on a group object, or lifecycle handlers |
| task `fail_fast:` | `fail_fast` on a `group:` object |
| task `setup:` | a preceding `side_effect:` or `shell:` task |
| task `teardown:` | a group's `finalize:` (a group of one for a single task) |
| `state`, `outputs`, `step`, `total_steps` | `seq.*` |
| `interactive: true` in a sequence document | `-i` |
| `--dry-run-shell` | run the sequence; a dry run runs no command |

These are declared `removed(…)` in Claudine's schemas, so the editor, `md
schema validate`, a dry run, and a run all refuse them, `null` included, with
the replacement above. A step key one edit from a task field (`promt`) is a
warning, and the step still runs.

A runnable nesting example is in
`claudine/docs/examples/sequence-containers.md`.
