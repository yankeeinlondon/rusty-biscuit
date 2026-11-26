---
kind: feature
created: 2026-11-26
status: draft-spec
area: claudine
needs-rulings: true
related:
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

# Markdown sequences are state lists, and `kind: list`

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
- **formal file**: a new `kind: list` document, with `template:`, `$state`,
  and `$schema`
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

### A `kind: list` document

```yaml
# providers.yaml
kind: list
$state:
    slug: string(required) -> the canonical provider slug
    binary: string(required)
template:
    desc: "{{ name }} ({{ site }})"
list:
    - name: Claude Code
      slug: claude
      binary: claude
    - name: Codex CLI
      slug: codex
      binary: codex
```

```yaml
---
# _fleet.md, the owner
agent: claude
research_prompt: "@claudine/docs/research/hooks/_research.md"
sequence: ./providers.yaml
---
Research {{ seq.name }} ({{ seq.binary }}) using {{ research_prompt }}.
```

`claudine sequence _fleet.md` composes the body twice, once per row.

Rules:

- **Root keys.** `kind: list` is required, and the rows live under `list:`
  (see [The property that holds a list's rows](#the-property-that-holds-a-lists-rows)). The root is closed: `kind`, `list`, `template`, `$state`,
  `$schema`, and the documentation keys every document may carry
  (`description`, …). Any other key, such as a value (`rank: 1`), a
  lifecycle event, or `loop:`, is a schema error naming the key. A list has
  no values of its own and runs nothing.
- **Rows** normalize exactly as an inline Markdown list does: scalar
  shorthand (`- claude` becomes `{ name: claude }`), then `template:`, then
  the generated fields (`id`, `index`, …), then `$state` judges the finished
  state passively. A row with an action key is
  `composition.markdown_row_action`, naming the list file and its owner.
- **Values in scope.** The list is the owner's step data. Its `template:`
  reads the **owner's** prepared values, then invocation setters, then the
  owner's `initialize` writes, then the row's own fields, the same layers an
  inline list in the owner reads. The `initialize: set research_prompt`
  workaround is no longer needed.
- **Relative paths** inside the list (a `$state` file, a template's file
  read) resolve from the list's own folder, as declarations do today.
- **A list has no meaning without an owner.** Anywhere other than a Markdown
  document's root `sequence:` it is a typed error:

  | Site | Error |
  | --- | --- |
  | `claudine sequence list.yaml` | `composition.list_not_runnable` |
  | a YAML sequence's root `sequence:`, or any row's `sequence:` / `prompt:` / `task:` / `group:` | `composition.list_not_runnable` |

  The message says that a list holds states for a Markdown document and shows
  the `sequence: ./list.yaml` line to write in one.
- **Data selections are unchanged.** `providers.yaml -> list` or
  `providers.yaml::name(slug)` reads the file as foreign data (lenient
  rules; the list's `template:`, `$state`, and `$schema` are not applied).
- **DMLS** validates `kind: list` files against their registered schema
  (completion and hover for root keys, the closed root, `$state` row
  diagnostics) with the codes the run reports.

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

What is used and what is ignored:

| Part of the `kind: sequence` file | Used? |
| --- | --- |
| row state fields | yes |
| `template:` | yes, filled before stripping; an action key it supplies is stripped like an authored one |
| `$state` | yes, judges the stripped state |
| `$schema` | yes, read as a list's `$schema` (a requirements check against the owner) |
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

### A list's `$schema` is a requirements check against its owner

A list has no values of its own, so its `$schema` is not a document schema
(nothing to prepare, default, or convert). It states what the list needs from
the Markdown document that uses it, and is judged when the owner's rows are
built, against the owner's prepared values, setters, and `initialize`
writes. It is passive, like an inline container's `$schema`.

```yaml
kind: list
$schema:
    research_prompt: file(required) -> the prompt each provider row researches
list: [claude, codex]
```

```text
error[composition.list_requirement]: claudine/docs/providers.yaml requires
`research_prompt` from _fleet.md: "research_prompt" is a required property
```

A list's required property is not a caller input that Claudine asks for on a
terminal: the owner must declare or supply it, so the owner's `$schema` stays
the complete list of what running it needs. No `$use`/`$with` form is added;
the owner's state already reaches the list, and a parameterized form would be
a second way to pass state. It can be added later without changing this rule.

## Decision for the author

### The property that holds a list's rows

A YAML sequence file keeps its rows under the root property `sequence:`. A
`kind: list` file needs some root property for its rows too, and it can be
the same name or a new one:

```yaml
# Option A: `list:`                 # Option B: `sequence:`
kind: list                          kind: list
template:                           template:
    desc: "{{ name }}"                  desc: "{{ name }}"
list:                               sequence:
    - claude                            - claude
    - codex                             - codex
```

**Option A (recommended): `list:`.** The key matches the kind, and a reader
can tell a list from a YAML sequence without looking at `kind:`. Claudine's
sequence schema trigger (`sequence-document.yaml`) matches any document with
a non-null `sequence`, so a list using another key never picks up the
sequence contract (lifecycle events, `loop:`, row actions) in a run or in
DMLS, with no trigger change.

- Cost: `kind: sequence` with `list:` is the form retired on 2026-07-12, and
  its error (`composition/sequence/source.rs`) tells authors to rename `list:`
  to `sequence:`. That message is reworded to offer both fixes: rename to
  `sequence:` for a YAML sequence, or set `kind: list` for a list of states.

**Option B: `sequence:`.** One name for rows everywhere.

- Cost: the sequence trigger and the formal-sequence classifier
  (`is_formal_sequence`) must both learn to exclude `kind: list`, and DMLS
  offers sequence-document completions inside a list until they do. A file
  whose `kind:` is missing or misspelled silently becomes a YAML sequence.

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

- `schemas/kind.yaml`: add `list` to the `kind` enum.
- `claudine/registration/`: a `list-file.yaml` definition, registered like
  `task-file.yaml`; Markdown frontmatter's `sequence` rows lose the action
  keys; run the root `just shipped-schemas`.
- Sequence source loading (`composition/sequence/source.rs`): classify a
  whole file by `kind` as list, sequence, or informal; normalize a list or a
  stripped sequence against its owner (the owner's values, the file's
  declaring path).
- Errors and warnings: register `composition.markdown_row_action`,
  `composition.markdown_sequence_without_action`,
  `composition.list_not_runnable`, `composition.list_requirement`, and
  `composition.sequence_actions_ignored` in `lib/src/diagnostics/registry.rs`
  with typed detail; DMLS guardrails for each; a `docs/` example of reading
  `composition.list_requirement` in a `failure` handler.
- Dry run: the passive check reports all of them from the same walk.
- Migration (no install base, so no deprecation period):
    - the six rosters listed above become `kind: list` with `list:`; their
      fleets declare `research_prompt` and similar values directly instead of
      writing them in `initialize`
    - `_provider-tasks.yaml` and `_local-runner-tasks.yaml` stay
      `kind: sequence`; their fleets keep working through stripping (with the
      warning) only if their bodies are the research prompt, so each fleet is
      rewritten either to put the action in its body or document `prompt`, or
      to become a YAML sequence that calls a prompt document
    - Markdown sequences with row actions become YAML sequences:
      `claudine/docs/examples/sequence-containers.md`,
      `darkmatter/docs/research/parsing/_fleet_future.md`, about 34 test
      fixtures (`claudine/lib/tests/fixtures/sequence-schema/`,
      `claudine/cli/tests/fixtures/lifecycle_traces/`, …), and about 32 Rust
      test files that write such documents inline; the frozen prompt
      `review-loop.md` is a frozen copy and is rewritten with its test
- Docs: `claudine/docs/topics/flow-control/sequences.md` ("Steps and tasks",
  "Container grammar", "What a referenced sequence reads", "Sequence
  documents", a new "Lists" section), the `use-claudine` caller skill, and
  the `claudine` repo skill.

## Success criteria

- A Markdown document gives the same steps, `seq` values, provider, outputs,
  and diagnostics whether its states are inline, in a `kind: list`, or in a
  `kind: sequence` file holding the same states (the last with its one
  warning). One test runs all three and compares them.
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
