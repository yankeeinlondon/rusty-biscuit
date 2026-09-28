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
    - claudine-cli
    - biscuit-tui
related:
    - 2026-09-27-sequence-improvements
---

# The sequence review screen hides step names and lists steps with no provider

## Outcome

When `claudine sequence` opens its review screen, you can read which step each
row belongs to, and every row is a step you can actually choose a provider
for.

## Report

`prompts/review-loop.md` has no top-level `agent`, so running it from a
terminal opens the sequence review screen: one row per step, with a provider
column and a model column.

```sh
claudine sequence ../prompts/review-loop.md \
    spec=fixes/2026-09-27-graph-merged-branch/spec.md
```

Three things are wrong with that screen:

1. **Step names are clipped to four cells.** `1 implement` shows as `1 im`
   and `12 stage-3` as `12 s`. The provider and model columns share all the
   remaining width, which leaves a wide empty gap between them.
2. **Shell steps get a row.** `stage-N` steps (`shell: git add .`) launch no
   provider, but each one still shows a provider and model picker. That's
   five extra five-line rows in a 21-step sequence.
3. **The first row shows stray horizontal rules** across the provider
   column that the other rows don't have.

Separately, every row starts on Claude, even for steps that ask for another
provider through `params.agent`. Those steps would in fact launch on their own
agent: at launch, a step's document `agent` overrides whatever the screen
chose. So the screen misrepresents what will run. That is a
provider-selection problem, not a display defect, and it is out of scope here
(see Non-goals).

## Cause

### Step names

`compute_column_widths` (`biscuit-tui/lib/src/components/input_table/table.rs:892`)
sizes a `StaticText` column by its header text (`"Step"`, minimum 3 cells). It
never looks at the row cells. Leftover width then goes only to the focusable
columns. The step label that `build_initial_rows` produces
(`format!("{} {}", index, name)` in `claudine/cli/src/commands/wrap/selection_ui.rs`)
is therefore clipped to the header's width, with no sign that it was clipped.

### Shell steps

Phase 1a builds one `SequenceStepDraft` for every step, whatever its
executable (`claudine/cli/src/commands/wrap/sequence/mod.rs:427-504`).
`review_sequence` renders one row per draft.

### Stray rules

Not yet identified. They appear only on the first row, which starts with
focus, so the focused row's choice-cell styling is the likely place to look.

## Requirements

### R1. A static column fits its content

In `InputTable`, a `StaticText` column's preferred width is the widest of its
header and every row cell in that column. When the table is narrower than the
total preferred width:

- focusable columns shrink first, down to their preferred minimums;
- after that the static column shrinks, and a clipped cell ends with `…`.

This is a `biscuit-tui` change, so every `InputTable` consumer benefits from
it. No consumer may rely on the old header-only width.

### R2. Only provider-launching steps get a row

The review screen lists the steps that launch a provider: steps with a
`prompt`, `task`, or `group` executable, and body steps with no executable.
`shell` and `side_effect` steps get no row. The resolved target vector still
has one entry per step, so the execution code needs no change. A step without
a row gets the same target that the non-interactive path would give it.

### R3. The stray rules are fixed or explained

A render test reproduces the first-row rules and the fix removes them. If
they don't reproduce in a test backend, the implementation log records that,
along with the terminal they were seen in (WezTerm on macOS), and R3 closes
without a code change.

## Non-goals

- **What the rows offer.** Steps already launch on their own document's
  `agent`. Making the screen reflect that (read-only rows for steps that
  name a provider), and not opening it at all when every step names one,
  belongs to `2026-09-27-sequence-improvements`, which is redesigning how
  steps and tasks run.
- **Per-row model catalogs.** With mixed providers, the model column already
  falls back to free text.
- **Deciding when the screen opens.** It still opens exactly when it does today.

## Acceptance criteria

1. **Column width.** An `InputTable` render test with the header `"Step"` and
   the cell `"12 review-5"` shows the full label at 80 columns. At a width
   too narrow for it, the label ends with `…`, and the focusable columns keep
   their minimums.
2. **Existing consumers.** The `biscuit-tui` input-table tests, and the
   `question` CLI tests that render input tables, pass. Snapshots are updated
   only where a static column was previously clipped.
3. **Shell steps.** A test drives Phase 1a/1b with a sequence of
   `prompt`, `shell`, and body steps and asserts that the review screen
   receives rows for the `prompt` and body steps only. It also asserts that
   the resolved target vector still has one entry per step.
4. **First row.** Either a render test proves the first row has no stray
   rules, or the implementation log records that the rules don't reproduce
   (R3).
5. **Docs.** `docs/topics/execution-flow.md` and `docs/cli/sequence.md`
   describe which steps appear on the review screen.
