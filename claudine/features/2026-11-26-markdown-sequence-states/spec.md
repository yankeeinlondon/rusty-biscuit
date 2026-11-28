---
kind: feature
created: 2026-11-26
status: draft-spec
area: claudine
needs-rulings: false
depends-on:
    - 2026-11-26-list-documents
related:
    - 2026-11-26-list-documents
    - 2026-11-18-ergonomic-sequences
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
    needs-rulings: boolean -> indicates whether open questions still need the author's ruling
---

# Markdown sequences are state lists

## The model

There are two kinds of sequence, and they differ in **where the action
lives**:

| | Markdown sequence | YAML sequence (`kind: sequence`) |
| --- | --- | --- |
| The action | the document itself: its body (compose) or its document-level `prompt` (inline-compose) | each row's own action (`prompt`, `shell`, `side_effect`, `group`, `task`, `sequence`, and `compose`/`inline-compose` once `2026-11-18-ergonomic-sequences` lands) |
| A row is | the **state** of one run of the document | state **plus** an action |
| Children | none: a Markdown document runs itself once per row | nested containers, groups, called documents |
| Can be called by | a YAML sequence row (`prompt:`, `compose:`, …) | a YAML sequence row, or run directly |

A Markdown document has no children. Its `sequence:` only ever supplies
states. It may be *called* by a YAML sequence, but it never calls anything
through its rows.

Where a Markdown document gets its states:

- **inline**: `sequence: [ … ]` in its frontmatter
- **informal file**: a bare array of strings (each becomes `seq.name`) or of
  objects (each must have `name`), or any selection reached with
  `-> offset` / `::operator`
- **formal file**: a `kind: list` document, with `template:`, `$state`,
  and `$schema`, defined and rendered by Darkmatter (`2026-11-26-list-documents`)
- **a `kind: sequence` file**: allowed, but its actions are ignored and only
  its states are used, with a warning

## What is wrong today

- **Markdown rows can carry actions.** A row with `shell:`, `prompt:`,
  `group:`, a nested `sequence:`, … runs that action instead of the body.
  This makes a Markdown document a program with children, against the model,
  and it is documented and tested as supported
  (`claudine/docs/topics/flow-control/sequences.md` → "Steps and tasks").
- **A `kind: sequence` file used as state data is ambiguous.** The repository
  uses one kind for two roles. Some files are programs whose rows carry
  actions (`prompts/complete.yaml`;
  `claudine/docs/research/_provider-tasks.yaml`, whose `template:` gives every
  row `prompt: "{{ research_prompt }}"`). Others are state rosters for a
  Markdown fleet (`claudine/docs/providers.yaml`,
  `claudine/docs/local-runners.yaml`,
  `biscuit-terminal/docs/multiplexers.yaml`,
  `biscuit-terminal/docs/terminal-apps.yaml`,
  `biscuit-tui/docs/style-guide/style-guide.yaml`,
  `darkmatter/docs/research/parsing/parsing-crates.yaml`). A roster is valid
  when a fleet references it and invalid when run directly, and no schema can
  say which role a file is meant for.
- **Rosters read the wrong values.** A referenced whole sequence document
  reads its own values, not the Markdown document's. That is why the research
  fleets pass `research_prompt` through
  `initialize: [{ set: { research_prompt: … } }]` instead of declaring it.
- **`2026-11-18-ergonomic-sequences` builds the wrong model.** That
  finalized spec lists a Markdown root `sequence: <file-ref>` among callees,
  dispatched as a "child sequence", and its per-executable table allows every
  action on every sequence step, Markdown included. See
  [Required corrections to `2026-11-18-ergonomic-sequences`](#required-corrections-to-2026-11-18-ergonomic-sequences).

## Behavior

### Markdown rows are states only

A row of a Markdown-authored sequence may not carry an action key. Writing one
is the error `composition.markdown_row_action`, reported for every offending
row before anything runs:

```yaml
---
sequence:
    - name: draft
    - name: test
      shell: just test
---
Work on {{ seq.name }}.
```

```text
error[composition.markdown_row_action]: sequence row 1 (`test`) of
review.md has `shell:`. A Markdown sequence runs its own body once per row,
so a row holds state only. Move the shell command into a YAML sequence
(`kind: sequence`) that calls review.md with `prompt: ./review.md`.
```

The action keys are `prompt`, `shell`, `side_effect`, `group`, `task`, and
`sequence` (and `compose`/`inline-compose` once
`2026-11-18-ergonomic-sequences` lands). The rule applies to rows from every
source: inline, informal file, and `kind: list`. Rows from a `kind: sequence`
file are stripped instead (below).

Task options that only make sense beside an action (`with:`, `when:`,
`timeout:`) are already refused on a body step and stay refused. A row field
with any other name is state.

Document-level keys are untouched: the Markdown document keeps its
`prompt` (inline-compose), its lifecycle events, its `loop:`, its `$schema`,
`$state`, and `template:`.

### A Markdown document that uses a `kind: list`

`kind: list`, how a list is rendered into rows (shorthand, template as
default values, generated fields, the passive `$state` check), what its
templates read, and what its `$schema` means are defined by Darkmatter in
`2026-11-26-list-documents`. Claudine renders a list through that one
Darkmatter function, so a Markdown sequence over a list gets exactly the rows
`load_data` returns for it. This spec adds only what is Claudine's:

```yaml
---
# _fleet.md
agent: claude
research_prompt: "@claudine/docs/research/hooks/_research.md"
sequence: ./providers.yaml      # kind: list
---
Research {{ seq.name }} ({{ seq.binary }}) using {{ research_prompt }}.
```

- **The outer values** a list's templates and `$schema` read are the Markdown
  document's prepared values, then invocation setters, then its `initialize`
  writes, the same layers its inline list reads today. The
  `initialize: set research_prompt` workaround is no longer needed.
- **A rendered row with an action key** is `composition.markdown_row_action`,
  naming the list file and the Markdown document. Rendering keeps such a key as
  data (it knows nothing of actions); refusing it is the Markdown sequence's
  rule.
- **A list has no meaning without a Markdown document.** Anywhere else in
  Claudine it is a typed error:

  | Site | Error |
  | --- | --- |
  | `claudine sequence list.yaml` | `composition.list_not_runnable` |
  | a YAML sequence's root `sequence:`, or any row's `sequence:` / `prompt:` / `task:` / `group:` | `composition.list_not_runnable` |

  The message says that a list holds states for a Markdown document and shows
  the `sequence: ./list.yaml` line to write in one.
- **Data selections are unchanged.** `providers.yaml -> list` or
  `providers.yaml::name(slug)` reads the file as foreign data, so it is not
  rendered as a list.
- **Run values** (`seq.run_id`, `seq.previous`, `seq.next`, `seq.outputs`)
  are added by the sequence run on top of the rendered rows, as today.

### A Markdown document that names a `kind: sequence` file

This is allowed. The file's rows are used as states, and everything that
would make it a program is ignored. Claudine reports one warning,
`composition.sequence_actions_ignored`:

```text
warning[composition.sequence_actions_ignored]: _fleet.md uses the YAML
sequence _provider-tasks.yaml as its list of states. Its actions are
ignored, and each row runs _fleet.md's body with only the row's state:
  - `prompt` on 12 rows (from `template:`)
Declare the file `kind: list` if it only holds states.
```

The warning's typed detail lists each ignored key with the rows it was on and
whether it came from a row or from `template:`, so an author can read it
from `err.detail` in a `failure`/`finalize` handler and DMLS can show the same
finding on the `sequence:` line.

The file is rendered by Darkmatter exactly as `load_data` renders it
(Claudine registers `kind: sequence` as a list kind whose rows are under
`sequence:`). The Markdown sequence then removes the actions from the
rendered rows. What is used and what is ignored:

| Part of the `kind: sequence` file | Used? |
| --- | --- |
| row state fields | yes |
| `template:` | yes, as rendering applies it; an action key it supplies is removed like an authored one |
| `$state` | yes, as rendering applies it (action keys and task options are not state, so it never judges them) |
| `$schema` | yes, as a list's `$schema`: a check of the Markdown document's values |
| row action keys, including nested `sequence:` / `group:` rows | ignored: the key is removed, and the row stays a state |
| task options (`with:`, `when:`, `timeout:`, …) | ignored with the action they belong to |
| root lifecycle events, `loop:`, root values | ignored, listed in the warning |

Values in scope are the owner's, exactly as for a `kind: list` (a row that
reads `research_prompt` reads the Markdown document's value).

The ignored keys are **removed**, not kept as state, so `seq.prompt` never
holds a row-level file reference next to the document-level `prompt`
(prose) that a Markdown template already interpolates.

### A Markdown sequence must have an action

Because a Markdown sequence's action is the document itself, the document
must have one. It has an action when **either** of these holds:

- its frontmatter defines `prompt` (and, once `2026-07-02-inline-sections`
  lands, `sections`): an inline-compose sequence; the prompt lives in
  frontmatter and the agent's target is the document. A frontmatter-only
  document is fine here;
- its body holds text: a compose sequence; the body is the prompt.

A document with neither (frontmatter only, no `prompt`/`sections`) is the
error `composition.markdown_sequence_without_action`:

```yaml
---
agent: claude
sequence: ./providers.yaml
---
```

```text
error[composition.markdown_sequence_without_action]: _fleet.md is a Markdown
sequence with nothing to run: it has no body and no `prompt`. Each row runs
the document, so write the prompt in its body, or in `prompt:` to edit the
document inline.
```

- The check reads the **authored** document (`prompt: null` counts as
  absent), so it fails before `initialize` and before any row is read, and
  `--dry-run` and DMLS report it the same way.
- It holds however the rows are supplied: inline, informal file,
  `kind: list`, or a stripped `kind: sequence` file. In the last case the
  `sequence_actions_ignored` warning is printed with it, so the author sees
  that the program they named is not what runs.
- A body that has text but composes to nothing at run time (for example only
  a `::block` whose condition is false) keeps today's empty-body refusal
  (Darkmatter `ComposedBodyEmpty`); this new check is structural and does not
  replace it.

## Required corrections to `2026-11-18-ergonomic-sequences`

That spec is finalized but not built. It must change before its Phase 4
(sequence planning) lands, or the fleets above break and Markdown rows keep
their actions:

- A Markdown root `sequence: <file-ref>` is **step data, never a callee**,
  whatever the file's kind. Remove it from the Decision 2.4 callee list and
  from the classification matrix's "dispatches to child sequence" wording for
  Markdown owners. Calling a Markdown sequence document from a YAML row
  (`prompt: ./fleet.md`) stays a call: a Markdown document can be a child; it
  has none.
- The per-executable table applies to YAML sequence rows, group legs, and
  `kind: task` files only. A Markdown row's only form is the zero-executable
  body step.
- Its "informal step lists are not callees" rule extends to `kind: list` and
  to a stripped `kind: sequence`.

## Scope of the work

- Depends on `2026-11-26-list-documents` (Darkmatter): `kind: list`, the
  shared rendering, and list-kind registration land first, or together.
- `claudine/registration/`: register `kind: sequence` as a Darkmatter list
  kind (rows under `sequence:`; action keys and task options are not state);
  Markdown frontmatter's `sequence` rows lose the action keys; run the root
  `just shipped-schemas`.
- Sequence source loading (`composition/sequence/source.rs`): classify a whole
  file by `kind` as list, sequence, or informal; render lists and sequence
  files through Darkmatter with the Markdown document's values as the outer
  values; remove `normalize`/`formal` code that the Darkmatter rendering
  replaces.
- Errors and warnings: register `composition.markdown_row_action`,
  `composition.markdown_sequence_without_action`,
  `composition.list_not_runnable`, and `composition.sequence_actions_ignored`
  in `lib/src/diagnostics/registry.rs` with typed detail; DMLS guardrails for
  each; a `docs/` example of reading `sequence_actions_ignored` detail in a
  `finalize` handler. List rendering failures keep Darkmatter's `dm.list.*`
  codes.
- Dry run: the passive check reports all of them from the same walk.
- Migration (no install base, so no deprecation period):
    - the six rosters listed above become `kind: list`; their fleets declare
      `research_prompt` and similar values directly instead of writing them in
      `initialize`
    - `_provider-tasks.yaml` and `_local-runner-tasks.yaml` stay
      `kind: sequence`. Their fleets would lose the roster's `prompt` to
      stripping, so each fleet is rewritten either to put the action in its
      body or document `prompt`, or to become a YAML sequence that calls a
      prompt document
    - Markdown sequences with row actions become YAML sequences:
      `claudine/docs/examples/sequence-containers.md`,
      `darkmatter/docs/research/parsing/_fleet_future.md`, about 34 test
      fixtures (`claudine/lib/tests/fixtures/sequence-schema/`,
      `claudine/cli/tests/fixtures/lifecycle_traces/`, …), and about 32 Rust
      test files that write such documents inline; the frozen prompt
      `review-loop.md` is rewritten with its test
- Docs: `claudine/docs/topics/flow-control/sequences.md` ("Steps and tasks",
  "Container grammar", "What a referenced sequence reads", "Sequence
  documents", a "Lists" section that links to Darkmatter's lists page), the
  `use-claudine` caller skill, and the `claudine` repo skill.

## Success criteria

- A Markdown document gives the same steps, `seq` values, provider, outputs,
  and diagnostics whether its states are inline, in a `kind: list`, or in a
  `kind: sequence` file holding the same states (the last with its one
  warning). One test runs all three and compares them, and compares the rows
  with `load_data` of the same file.
- Every Markdown row action, from every source but `kind: sequence`, fails
  with `composition.markdown_row_action` before anything runs.
- A frontmatter-only Markdown sequence runs when it defines `prompt` and
  fails with `composition.markdown_sequence_without_action` before
  `initialize` when it does not, for every row source.
- `claudine sequence list.yaml` and every other site in the list-site table
  fail with `composition.list_not_runnable` before any step starts.
- Each code is reported identically by a run, `--dry-run`, and DMLS.
- The migrated fleets plan and dry-run with the same steps as before.

## Unchanged rule worth stating

A Markdown document's own `$state` and `template:` declare its **inline**
list. Beside a `sequence:` that is a file reference they stay the existing
placement error; a list's states are typed and filled by the list's own
declarations.
