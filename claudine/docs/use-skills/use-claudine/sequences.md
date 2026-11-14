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
- `timeout`;
- `operation`.

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
claudine sequence workflow.md --dry-run              # one loop pass; no events; shell/side_effect tasks only rendered
claudine sequence workflow.md --dry-run --dry-run-shell   # also run shell: tasks for real
```

`$(…)` composition expansions run for real even in a dry run.

## Removed spellings

| Removed | Use instead |
|---|---|
| `kind: sequence` + `list:` | `sequence:` |
| `execution: serial` | a nested `sequence:` |
| sequence `fail_fast:`, `--fail-fast` | lifecycle handlers |
| task `setup:`/`teardown:` | an inline sequence, or the prompt's own events |
| `state`, `outputs`, `step`, `total_steps` | `seq.*` |
| `interactive: true` in a sequence document | `-i` |

A runnable nesting example is in
`claudine/docs/examples/sequence-containers.md`.
