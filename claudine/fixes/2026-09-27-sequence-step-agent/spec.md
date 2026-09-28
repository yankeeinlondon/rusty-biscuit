---
created: 2026-09-27
status: draft-spec
clarified: false
reviewed: false
review_iterations: 0
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
area: claudine
packages:
    - claudine
    - claudine-cli
    - biscuit-tui
related:
    - 2026-09-27-sequence-improvements
    - 2026-04-25-agent-selection
---

# A sequence step's own `agent` does not choose its provider

## Outcome

Each step of a sequence that launches a prompt runs on the provider that the
step names. The step can name it through `params.agent` or through the
prompt's own `agent` frontmatter. A sequence can then review with one agent
and repair with another, with no command-line flag and no interactive
picker. When a picker is needed, it shows each step's name in full and starts
with each step's own provider selected.

## Report

`prompts/review-loop.md` has no top-level `agent`. It passes a provider to
each step through `params`:

```yaml
review_agent: codex
repair_agent: claude
sequence:
    - name: review-1
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
    - name: repair-1
      prompt: "@prompts/_implement/implement-suggestions.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
```

Running it from a terminal:

```sh
claudine sequence ../prompts/review-loop.md \
    spec=fixes/2026-09-27-graph-merged-branch/spec.md -y
```

opens the sequence review screen instead of starting the run. There are two
problems with that screen:

1. **Every row defaults to Claude**, including the `review-N` rows that ask
   for `codex` and the `commit` rows whose prompt (`prompts/commit.md`)
   declares `agent: opencode`. If you accept the defaults, every step runs
   under Claude.
2. **The layout is broken.** The step column is 4 cells wide, so
   `1 implement` shows as `1 im`. The provider and model columns share the
   rest of the width, which leaves a wide empty gap between them. The first
   row also shows stray horizontal rules across the provider column.

## Cause

### Provider selection reads only the sequence document

The step list, including any provider it asks for, is resolved in Phase 1a/1b
of `run_sequence` (`cli/src/commands/wrap/sequence/mod.rs`):

1. `raw_hints` is parsed from the **sequence document's** frontmatter and
   the CLI setters (`mod.rs:277-279`).
2. `raw_hints` is classified once into a single `shared_state`
   (`mod.rs:398-404`). Every step's draft is built from those same hints
   (`mod.rs:427-504`).
3. When `shared_state` is a prompting state (here, "no agent"), a terminal
   gets the review screen and anything else gets `AgentResolutionFailed`
   (`mod.rs:411-421`, `:515-548`).
4. The result is a vector of `Some(target)`, one per step (`mod.rs:578`). When
   a step runs, `WrapperPromptRunner` hands that target to the child
   (`task_run.rs:455`). `resolve_execution_target` then returns it without
   looking at the child's hints (`cli/src/commands/wrap/composition/target.rs:91-93`).

`params` really do reach the child. They are the lowest `set_overrides` layer
(`lib/src/composition/sequence/task/mod.rs:827-842`), so `params.agent`
becomes the child's composed `agent`, just as it would in a direct
`claudine compose`. The planned target simply wins over it. Dry run makes the
mismatch visible: the per-step agent cell is reclassified from the child's
hints for display (`cli/src/commands/wrap/composition/dry_run.rs:105-106`), but the provider that
actually runs is still the planned one.

The docs claim more than the code does:

- `docs/cli/sequence.md` ("Example: Multi-Provider Research") shows steps
  with `provider: "claude"` and `provider: "gemini"`. `provider` there is
  ordinary step state (`{{ state.provider }}`) and selects nothing.
- `docs/topics/flow-control/sequences.md` says provider and model "resolve
  once, producing a per-step target vector". That describes the data shape.
  It does not describe where each step's provider comes from.

### The step column is sized by its header

`compute_column_widths` (`biscuit-tui/lib/src/components/input_table/table.rs:892`)
sizes a `StaticText` column by its header text (`"Step"`, with a minimum of 3
cells) and never looks at the row cells. All leftover width goes to the
focusable columns. The step label, `format!("{} {}", index, name)` in
`build_initial_rows` (`cli/src/commands/wrap/selection_ui.rs`), is therefore
clipped to 4 cells.

The cause of the stray rules on the first row has not been identified. They
may come from how the focused row's choice cells are styled.

## Requirements

### R1. A prompt step resolves its own agent hint

A step with a `prompt` executable resolves its provider from the child
prompt's composed `agent` and `model` hints. These are the same hints a
direct `claudine compose` of that prompt would classify, so the existing
layer order applies, lowest to highest:

1. the child prompt's own frontmatter,
2. the step's `params`,
3. CLI setters (`agent=`, `--set agent=`).

The full precedence for a prompt step, highest first:

| Source                                    | Scope      |
| ----------------------------------------- | ---------- |
| `--<provider>` / `--provider`             | every step |
| a choice made on the review screen        | that row   |
| the child's composed `agent` (above)      | that step  |
| the sequence document's `agent`           | every step |
| favorite / picker (today's prompting path) | that step  |

The last two rows are what happens today. The fix inserts the third.

A step without a `prompt` executable keeps today's behavior. A body step runs
the sequence document's body, so the document's hint applies. `shell` and
`side_effect` steps launch no provider.

### R2. Step hints are early-binding and resolved before any step runs

A step's provider is fixed before the first step launches, just as shell
commands are approved before the first step launches. The step's `agent` and
`model` are evaluated at Phase 1a/1b using the same early-binding context
that shell preflight uses (`state`, `params`, template values, `doc.*`,
`ctx.*`, `env.*`, and CLI setters). The child prompt's frontmatter is read at
the same point.

If a step's `agent` or `model` expression reads `outputs` or a value that an
earlier step changes at runtime, preflight fails with a typed error. The
error names the step and the expression. This mirrors the shell rule in
`docs/topics/flow-control/sequences.md` and keeps what dry run shows equal to
what runs.

At the step's turn, the child is composed again (JIT) and runs on the planned
target. R2 guarantees that the composed hint cannot differ from the plan, so
there is nothing to reconcile.

### R3. The picker appears only for steps that cannot resolve

Classification happens per step, not once for the whole document:

- If every step that launches a provider resolves to an auto-selectable
  state, the run starts without the review screen. `review-loop.md` falls in
  this case.
- If any such step is in a prompting state and stderr is a terminal, the
  review screen opens. Each row starts with its own step's resolved provider
  and model, and a row for a step in a prompting state starts with today's
  default.
- If any such step is in a prompting state and stderr is not a terminal, the
  run aborts with `AgentResolutionFailed`, naming each unresolved step.

The review screen lists only steps that launch a provider. `shell` steps such
as `stage-N` are left out.

### R4. The review screen shows each step's name

In `InputTable`, a `StaticText` column's preferred width is the widest of its
header and its row cells. When the table is narrower than the total preferred
width, the static column may shrink. It shrinks only after the focusable
columns reach their preferred widths, and a clipped label ends with `…`
instead of being cut silently. This change belongs to `biscuit-tui`, and every
`InputTable` consumer benefits from it.

The stray rules on the first row are reproduced in a render test and fixed,
or recorded in the implementation log as not reproducible along with the
terminal they appeared in.

### R5. Status and reports name the provider that ran

Step status lines, `env.AGENT`/`env.MODEL`, the dry-run table, and the
sequence report all use each step's own target. This is acceptance criterion
10 of `2026-09-27-sequence-improvements`, and this fix is what makes it
reachable. Whichever change lands second removes the duplicate requirement.

## Non-goals

- A step-level `agent:` key beside `prompt:`. `params.agent` and the prompt's
  own frontmatter already express this, and a second spelling would need its
  own precedence rule.
- Choosing a provider at the step's turn (late binding). R2 rules it out on
  purpose.
- Per-row model catalogs in the review screen. With mixed providers the
  model column already falls back to free text, which is acceptable.
- Rewriting `review-loop.md` for the loop primitives. That is
  `2026-09-27-sequence-improvements`.

## Consequence for `review-loop.md`

After this fix, the `commit-N` steps, which pass no `agent`, run on
`prompts/commit.md`'s own `agent: opencode`. That matches the intent recorded
in `bd1ae5f8c` ("commit steps fall back to the commit prompt's own default").
It differs from what a run does today, where every step runs under one
provider.

## Acceptance criteria

1. **Params choose the provider.** A Level 1 test runs a three-step sequence
   against stub providers. The sequence has no top-level `agent`. The steps
   pass `params.agent: codex`, pass `params.agent: claude`, and pass nothing to
   a prompt whose frontmatter says `agent: opencode`. The test asserts that
   each stub received exactly its own step's prompt and that no review screen
   or picker ran.
2. **Precedence.** A table-driven test covers each precedence row in R1 once:
   `--codex` overrides all three steps, `agent=gemini` overrides `params`, a
   document-level `agent` fills a step that names nothing, and a step whose
   hint is invalid falls through to the prompting path.
3. **Early binding.** A step whose `params.agent` reads `outputs` fails
   preflight with the typed error. No provider launches, and the message
   names the step.
4. **No terminal.** With stderr redirected, a sequence with one unresolved
   step aborts with `AgentResolutionFailed` naming that step. A sequence with
   every step resolved runs.
5. **Review screen seeding.** A test drives `review_sequence` with drafts
   for mixed providers and asserts that each row's starting provider matches
   its draft. It also asserts that `shell` steps have no row.
6. **Column width.** An `InputTable` render test with the header `"Step"` and
   the cell `"12 review-5"` shows the full label at 80 columns and a
   `…`-terminated label at a width too narrow for it.
7. **Dry run agrees with execution.** `--dry-run` on the fixture from
   criterion 1 reports the same three providers that criterion 1 asserts ran.
8. **Docs.** `docs/cli/sequence.md` replaces the misleading `provider:`
   example with a working `params.agent` example.
   `docs/topics/flow-control/sequences.md` states the precedence in R1 and the
   early-binding rule in R2. `docs/topics/execution-flow.md` describes when
   the review screen appears and how its rows are seeded. The `claudine`
   skill's composition page matches.
