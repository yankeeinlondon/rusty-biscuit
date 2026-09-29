---
created: 2026-09-27
status: draft-spec
clarified: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-09-29
review_iterations: 2
completed: true
implemented: true
human_review: false
message_to_agent: |-
    All five phases are done; the fix is implemented and ready for review. Phase 5 was validation only and changed no source, docs, or skills. Read "## Phase 5" in implementation-log.md for the criteria-to-test mapping.

    - Gates: biscuit-tui `just test` 1012 passed (includes the `question` CLI tests), `just lint` clean; claudine `NEXTEST_TEST_THREADS=6 just test` 8051 passed, `just lint` clean.
    - No L2 test was added in any phase, so `just test-l2` was not run.
    - Not touched: `claudine/docs/rollout-strategy.md` (the author's roadmap) still lists this fix as "Draft; not reviewed". That status is now out of date.
    - Windows and WSL2 are only exercised in CI after merge unless the pull request gets the `ci:all-os` label. None of the fix's code has platform conditionals.
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

When `claudine sequence` opens its review screen, step labels fit when space
allows and show an ellipsis when clipped. Shell and side-effect steps no
longer have provider or model pickers. Choices still apply to the original
step, even when some steps have no row.

This fixes presentation and row mapping. The screen continues to show a
planned target; a prompt document's own provider can override that target
at launch, as described under Non-goals.

## Report

`prompts/review-loop.md` has no top-level `agent`, so running it from a
terminal opens the sequence review screen: one row per step, with a provider
column and a model column.

```sh
claudine sequence ../prompts/review-loop.md \
    spec=fixes/2026-09-27-graph-merged-branch/spec.md
```

Three things are wrong with that screen:

1. **Step names are clipped to four terminal cells.** `1 implement` shows
   as `1 im` and `12 stage-3` as `12 s`. The provider and model columns
   share all the remaining width, leaving a wide empty gap between them.
2. **Shell steps get a row.** `stage-N` steps (`shell: git add .`) launch no
   provider, but each still shows a provider and model picker. In the
   reported 21-step sequence, five such steps add five five-line rows.
3. **The first row shows stray horizontal rules** across the provider
   column that the other rows don't have. This was seen in WezTerm on macOS.

Separately, all rows start from the sequence's shared default provider,
which was Claude in the reported run, even for steps that ask for another
provider through `params.agent`. At launch, a prompt document's `agent`
can override the planned target. Correcting that provider-selection behavior
is outside this fix.

## Cause

### Step names

In the `biscuit-tui` package,
[`compute_column_widths`](../../../biscuit-tui/lib/src/components/input_table/table.rs)
allocates table column widths. It sizes a `StaticText` column using its
schema text (minimum three terminal cells), ignoring the actual row values.
There is no separate header row in the current renderer. The schema text
also seeds cells when no row value is supplied.

In the `claudine-cli` package,
[`build_initial_rows`](../../cli/src/commands/wrap/selection_ui.rs)
constructs labels from the original one-based step position and name.
Its `Step` schema text therefore limits labels to four cells in a wide table.
Static cells clip without an overflow marker.

### Shell steps

The `claudine-cli` package's
[sequence target preparation](../../cli/src/commands/wrap/sequence/mod.rs)
creates a draft for every normalized step. Its
[`review_sequence`](../../cli/src/commands/wrap/selection_ui.rs)
turns every supplied draft into an editable row and returns targets in row
order. Simply filtering rows would shorten that result and shift targets
onto the wrong steps.

### Stray rules

The `biscuit-tui` package's
[`paint_focus_background`](../../../biscuit-tui/lib/src/components/input_table/table.rs)
styles the focused cell before drawing its widget. It adds `UNDERLINED`
across the whole rectangle, including blank cells. This is a concrete
candidate for the reported horizontal rules; confirm it by inspecting the
rendered buffer's style modifiers, not only its text. A text snapshot cannot
show an underline on a space.

## Requirements

### Static columns fit their content

For each `StaticText` column, use the maximum display width of its schema
text and all current row values, with the existing three-cell floor, as its
preferred width. Include off-screen rows so scrolling does not change the
column width. Recalculate from the current state when rendering after a
resize or a caller's row update. An empty table uses schema text alone.

Measure terminal display cells using the existing Unicode width support,
not byte or character counts. Keep the existing single-line static-cell
behavior; this fix does not add wrapping or interpret ANSI escape sequences.
Use sufficiently wide or saturating arithmetic so very long labels cannot
wrap the width calculation.

Allocate width with these rules:

- When all preferred widths fit, static columns get their preferred widths.
  Share remaining space among focusable columns as today, with remainder
  cells assigned left to right. An all-static table may leave unused space.
- The current focusable preferred widths are protected budgets, not hard
  minimums: eight cells for switches, twenty for text and choice inputs,
  and the configured preferred width for text areas. This fix introduces
  no new public width configuration.
- As space decreases, remove focusable columns' extra space first. If the
  preferred widths still do not fit, shrink static columns toward three
  cells, sharing reductions evenly and assigning remainders left to right.
- If three cells per static column plus the focusable budgets cannot fit,
  divide the available width evenly among all columns, assigning remainder
  cells left to right and capping static columns at their preferred widths.
  Zero-width columns are allowed at extremely small widths. The allocated
  widths must never exceed the available width; hard minimums cannot be
  promised when the terminal itself is smaller than their sum.

A clipped static cell ends with `…`, reserving its display width within the
allocation. At one cell wide, show only `…`; at zero, draw nothing. Keep
visible character clusters intact (Unicode graphemes), so a letter with a
combining accent or a joined emoji is not split. Ensure wide characters do
not overlap the next column. Preserve
the full original string in returned row values; ellipsis is presentation
only.

> **Review note:** The original draft referred to editable-column minimums,
> but the component defines preferred widths, not minimum widths. The rules
> above reuse those budgets and specify a fallback for terminals too narrow
> to honor them, avoiding a new configuration API for this display fix.

This intentionally changes layout for every `InputTable` consumer. Preserve
column identifiers, row validation, keyboard navigation, and returned values.
Review affected rendering expectations; changes can come from content-based
widths, narrow layouts, or the focus-style correction below, rather than only
from previously clipped static cells.

### Only eligible steps get review rows

Classify the already-normalized step's outer executable. Do not compose
prompt files or run commands to decide whether to show a row.

| Outer executable | Show a row? | Meaning of its target |
| --- | --- | --- |
| `prompt` | Yes | Planned fallback for the prompt document |
| `task` | Yes | Planned fallback for the referenced task |
| `group` | Yes | Planned fallback for the group; no nested task rows |
| None (body step) | Yes | Planned target for the sequence body |
| `shell` | No | Retained target entry for existing execution bookkeeping |
| `side_effect` | No | Retained target entry for existing execution bookkeeping |

A task or group remains eligible even if its referenced work contains only
shell or side-effect actions. Recursively determining whether it will launch
a provider is outside this fix. This table defines eligibility more precisely
than saying every displayed step necessarily launches a provider.

Keep drafts and a baseline target entry for every original step. Construct
baseline targets with the same deterministic draft-to-target conversion used
when review is bypassed, including provider/model reasons. Pass only eligible
drafts to the review UI, retaining their original step indices. Merge the
submitted targets back into the baseline vector at those indices. Never map
by name (names may repeat) or by the filtered row number.

For example, `prompt`, `shell`, body steps appear as rows `1` and `3`.
Changing row `3` changes the third target, while the second target and its
resolution reasons remain untouched. The execution-facing target vector
retains one entry per original step, in original order. Reject an unexpected
submitted row count before merging rather than silently dropping choices.

If no eligible drafts remain, return the baseline targets without opening
an empty input table. This is the sole change to when the review UI opens.
Preserve the existing document-level provider-resolution gate, including its
failure without a terminal when a provider choice is required. In particular,
this fix does not make a shell-only sequence with no resolvable top-level
provider executable in a headless session. An explicit provider still bypasses
that gate as today. Dry-run behavior is unchanged.

Submitting with `Ctrl+S` commits all reviewed choices. `Esc` and `Ctrl+C`
retain their existing cancellation behavior and start no sequence work.
Filtering must not bypass schema checks, shell approval, lifecycle work, or
execution of hidden steps; it changes only the review rows and their mapping.

### Focus does not draw rules through blank space

Reproduce the focused choice cell using the default component theme in a
headless render test. Inspect the style of blank buffer cells, then move focus
to a later row and render again in the same buffer to check cleanup.

If blanket underlining explains the rules, remove that blanket modifier
while preserving the choice widget's active-option styling and a visible
focus cue. Do not remove intentional underlining from caller-supplied themes
or links. Verify another editable cell type as well, since the focus painting
is shared by all editable cells.

If the reported appearance remains unexplained after style inspection,
record the evidence and limitation in the implementation log. An inability
to reproduce is not proof of a fix: report this requirement as unresolved
rather than closing it without evidence. A real-terminal check is warranted
only if the headless style evidence cannot answer the question; it must use
the repository's terminal test harness without taking window focus.

## Non-goals

- **Per-step provider resolution.** Making rows reflect a prompt document's
  own provider, making those rows read-only, or suppressing review when
  every eligible step declares a provider is separate work. The related
  `2026-09-27-sequence-improvements` changes sequence execution, but this
  fix does not assume that it has resolved provider-selection behavior.
- **Per-row model catalogs.** Keep the current shared-column behavior:
  a shared provider uses its catalog; mixed draft providers use free text.
  Changing the model editor after an interactive provider change is separate
  work.
- **Other review-opening rules.** Preserve the current gate except for
  bypassing an empty filtered table, as specified above.
- **New layout options or performance studies.** Reuse current components
  and width support. No benchmark or performance spike is needed for this fix.

## Acceptance criteria

1. **Content widths and resizing.** With schema text `Step` and a row value
   `12 review-5`, a headless `InputTable` render at 80 columns shows the full
   label. Include a longer off-screen row and prove scrolling does not change
   widths. Resize down and back up to demonstrate clipping and restoration.
2. **Narrow and Unicode labels.** Cover static-column shrinking while
   editable budgets still fit, the emergency allocation, and widths of zero
   and one. Include a wide character and a combining sequence. Rendering stays
   inside column bounds, clipping shows `…`, and submission returns full text.
   Include multiple static columns and an all-static table.
3. **Row mapping.** Exercise target preparation, filtering, and merging with
   interleaved prompt, shell, side-effect, task, group, and body steps. Verify
   displayed original positions, distinct submitted provider/model choices,
   unchanged hidden targets and reasons, and the full target-vector length.
   Include repeated names, a hidden first/last step, an all-hidden sequence,
   and rejection of an unexpected returned row count.
4. **Opening and cancellation.** Use an injected review callback or synthetic
   events to prove the empty table never opens, existing explicit-provider
   and headless gates remain unchanged, dry-run never prompts, and cancellation
   starts no steps. Do not invoke the real standalone event loop from ordinary
   unit tests or launch real providers, shell actions, or lifecycle audio.
5. **Focus rendering.** A buffer-style assertion verifies the default focused
   choice cell has no blanket underline on blank cells, that moving focus
   clears the old styling, and that focus remains visible. If this does not
   explain the original report, record it as unresolved as described above.
6. **Existing consumers.** Run `just test` and `just lint` in both `claudine`
   and `biscuit-tui`; these include the `question` CLI unit tests. Update only
   rendering expectations explained by this fix. Use `just test-l2` only if
   real-terminal coverage is added or changed, keeping windows out of focus.
   The implementation and tests must remain portable across macOS, Linux,
   native Windows, and WSL2.
7. **Docs and comments.** Update Claudine's
   [sequence guide](../../docs/cli/sequence.md) and
   [execution flow](../../docs/topics/execution-flow.md) with row eligibility,
   original numbering, and the empty-table exception. Update biscuit-tui's
   [InputTable guide](../../../biscuit-tui/docs/components/input_table.md) with
   sizing and display-only clipping, and the relevant README summaries if
   they describe this behavior. Correct touched symbol documentation that
   implies every draft is displayed or that static text cannot vary by row.
   Update skill guidance if it describes the changed workflow. Current docs
   describe behavior directly and do not refer back to this dated fix.

## Open Questions

None requiring an author decision for the scope above. If headless style
inspection does not explain the reported rules, their cause remains an
implementation investigation, not permission to mark the defect fixed.
