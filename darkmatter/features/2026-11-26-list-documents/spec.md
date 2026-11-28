---
kind: feature
created: 2026-11-26
status: draft-spec
area: darkmatter
needs-rulings: true
related:
    - 2026-11-26-markdown-sequence-states
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

# List documents (`kind: list`) and loading them with `load_data`

## Problem

A YAML file that holds a list of records with a `template:` and a `$state`
can only be read correctly by Claudine's sequence runner. Everything else
gets the raw file:

- `load_data(file)` returns the whole mapping (`kind`, `$state`, `template`,
  `sequence`). The loading-data docs make that a rule: "a YAML mapping with a
  `kind` or `$schema` key selects no schema."
- `load_data(file).sequence` returns the rows as authored: scalar shorthand
  not expanded, no template fields, no generated fields, no `$state` check.

The repository already has a document broken by this.
`biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
declares `apps` as an array of apps and writes
`apps: "{{ load_data('biscuit-terminal/docs/terminal-apps.yaml') }}"`.
`md compose` on it fails today with
`type apps: {"kind":"sequence",…} is not of type "array"`, because
`terminal-apps.yaml` is a `kind: sequence` file whose rows sit under
`sequence:` and whose `slug`/`file`/`intro` fields come from its `template:`.

The list rules (shorthand, template, generated fields, `$state`) live in
Claudine's sequence code, and Darkmatter cannot depend on Claudine, so
`load_data` cannot apply them today.

## Summary of the change

- **Darkmatter owns lists.** A new document kind, `kind: list`, and the one
  set of rules that turns a list document into rows (its **rendering**) move
  into Darkmatter.
- **One rendering everywhere.** `load_data`, Claudine's Markdown sequences,
  and Claudine's YAML sequences all render a list through the same Darkmatter
  function. Nothing is done differently for any one of them: given the same
  file and the same outer values, every consumer gets the same rows.
- **Hosts can register list kinds.** Claudine registers `kind: sequence`
  (rows under `sequence:`) through the existing host registration, so
  `load_data` renders a sequence file too, in `md` as well as in Claudine.
  A registration names the kind, its rows property, and the row keys that are
  not state (for Claudine: the action keys and task options). Rendering keeps
  those keys in the row but `$state` never judges them, as a sequence's
  `$state` ignores them today.
- **Typed results.** When the file argument is a literal path and the file
  exists, `load_data` takes its result type from the list's `$state` and
  `template:`. A missing file gives today's untyped result.

## Behavior

### A list document

```yaml
# terminal-apps.yaml
kind: list
$state:
    name: string(required) -> name of the app
    url: url -> primary URL for the app
    os: enum(linux, macos, windows)[](required)
template:
    slug: "{{ dasherize(name) }}"
    file: "{{ slug }}.md"
list:
    - name: Wezterm
      url: https://wezterm.org/
      os: [linux, macos, windows]
    - name: Ghostty
      os: [linux, macos]
```

Root keys: `kind: list` (required), the rows (see
[Decision 1](#decision-1-the-property-that-holds-a-lists-rows)),
`template`, `$state`, `$schema`, and the documentation keys every document
may carry (`description`, …). The root is closed: any other key, a lifecycle
event or `loop:` included, is a schema error naming the key. A list has no
values of its own and runs nothing.

### Rendering a list

Rendering turns a list document into rows. It runs in a fixed order, the
order Claudine's sequences use today:

1. **Shorthand.** A scalar row `- claude` becomes `{ name: claude }`. An
   object row must have `name`.
2. **Template.** Each `template:` key the row does not already have is
   added; the template acts as the row's default values. A row value that
   is `null` counts as absent, by the repository's absent-equals-null rule.
   A string template is interpolated; any other value (a number, a list, a
   map) is a literal default.
3. **Generated fields.** `id` (the dasherized `name`, suffixed `-2`, `-3`, …
   on collision), `index` (one-based), `count`, `is_first`, `is_last`. They
   are properties of the list, so every consumer gets them. A row that
   authors one is an error, as it is in a sequence today.
4. **`$state` check.** The finished row is checked against `$state`
   passively: a failure is reported, but no default is applied and no value
   is converted (a row holding `"3"` where `$state` says `number` passes and
   keeps the text, as numeric text does for any schema).

Given the file above, every consumer gets:

```json5
[
    { name: "Wezterm", url: "https://wezterm.org/", os: ["linux", "macos", "windows"],
      slug: "wezterm", file: "wezterm.md",
      id: "wezterm", index: 1, count: 2, is_first: true, is_last: false },
    { name: "Ghostty", os: ["linux", "macos"],
      slug: "ghostty", file: "ghostty.md",
      id: "ghostty", index: 2, count: 2, is_first: false, is_last: true },
]
```

What rendering does **not** include:

- **`run_id`** and the other run values (`seq.previous`, `seq.next`,
  `seq.outputs`). They describe one sequence run, not the list, and the
  Claudine runner adds them as it does today.
- **Children.** A row field that holds another list (in a `kind: sequence`
  file, a nested `sequence: [ … ]`) is a row field like any other. The nested
  list is rendered by whoever runs it.
- **Action handling.** Rendering knows nothing of actions. A `kind: sequence`
  row's `prompt:` or `shell:` is a field of the rendered row. A consumer may
  then use the rows its own way: Claudine runs them as actions in a YAML
  sequence, or removes them (with a warning) when a Markdown document uses the
  file as its states. `load_data` returns them as data.

### What a template reads

A template reads, lowest precedence first:

1. the **outer values**: whatever the consumer is evaluating in;
2. the row's own fields, which shadow the outer values.

Only the outer values differ by consumer, and each consumer passes the values
it already has. The rule is the same:

| Consumer | Outer values |
| --- | --- |
| `load_data(f)` in a document | the values the `load_data` call's own expression can read |
| a Claudine Markdown sequence | the Markdown document's prepared values, invocation setters, and its `initialize` writes |
| a Claudine YAML sequence run directly | the sequence document's own prepared values, setters, and `initialize` writes |

So `load_data('terminal-apps.yaml')` in `osc-9-conemu.md` and a Markdown
sequence over the same file give the same rows whenever the outer values the
templates read are the same.

Template expressions are `{{ … }}` interpolation only. A `$( … )` shell
expansion in a template is an error (`dm.list.template_shell`), because
loading data never runs a command. Files a template reads join the call's
recorded reads, and the `load_data` memo key includes the outer values the
templates read, so two calls whose templates see different values never share
a cached result.

Relative paths in a list (a `$state` file, a named-type import, a template's
file read) resolve from the list's own folder.

### A list's `$schema`

A list has no values of its own, so its `$schema` is not a document schema.
It states what the list needs from its outer values and is checked against
them before rendering, passively (no default, no conversion):

```yaml
kind: list
$schema:
    research_prompt: file(required) -> the prompt each row researches
list: [claude, codex]
```

A missing or wrong-typed value is `dm.list.requirement`, naming the list and
the consumer. There is no `$use`/`$with` form: a list reads the outer values
like every other child, and a second way to pass state is not added.

### `load_data` on a list

- `load_data(f)` on a file whose `kind` is `list`, or a kind a host
  registered as a list kind (Claudine: `sequence`), returns the **rendered
  rows**. Any other file is loaded as today.
- `as` still applies, to the rendered rows: it converts and checks them, and
  a mismatch is `dm.data.contract_mismatch` as today.
- To read the file as plain data (the whole mapping), pass the option
  `raw: true`: `load_data(f, null, { raw: true })`. Then `kind` selects
  nothing, exactly as today.
- Rendering failures (`$state`, `$schema`, a template error) are reported by
  `load_data` with their `dm.list.*` code and a position in the list file, as
  other `load_data` failures point into the data file.
- The loading-data docs' rule "a mapping with a `kind` or `$schema` key
  selects no schema" changes to: it selects nothing **unless** `kind` is a
  list kind.

### Typing the result

When the file argument is a literal path, the file exists, and it is a list,
the design-time call check and DMLS type the result as an array of rows
whose fields are:

| Field | Type |
| --- | --- |
| each `$state` property | its declared type |
| `name` | `string` |
| each template key | `string` for a string template; the literal's type otherwise; a `$state` declaration of the same key wins |
| generated fields | `id: string`, `index: number`, `count: number`, `is_first: boolean`, `is_last: boolean` |

```md
---
apps: "{{ load_data('terminal-apps.yaml') }}"
---
{{ map(apps, a => a.slug) }}      <- `a.slug` is known; `a.slgu` is dm.expression.undeclared_member
```

- **A missing file** gives today's untyped (provisional) result, with no
  warning. It may be created later.
- **An explicit `as`** types the result as `as` does today, whatever the file
  holds.
- **Limits:** a `$state` given as a file reference or `Name@file` import types
  as `unknown` where the reference appears, as named references do for `as`
  today; inline structure keeps its type. The type is read when the check
  runs. The run renders the file again and checks it, so a file edited after
  the check is never trusted on the strength of that check.
- Reading the file at check time is a file read like any other: it needs
  read permission, and DMLS reads it through its observation worker rather
  than inside a completion request.

## Decisions for the author

### Decision 1: the property that holds a list's rows

```yaml
# Option A: `list:`                 # Option B: `sequence:`
kind: list                          kind: list
list:                               sequence:
    - claude                            - claude
```

**Option A (recommended): `list:`.** The key matches the kind, so a reader
can tell a list from a YAML sequence without checking `kind:`, and a file
whose `kind:` is missing or misspelled is never mistaken for a sequence.
Claudine's sequence trigger (`sequence-document.yaml`) matches any document
with a non-null `sequence`, so a list never picks up the sequence contract
(lifecycle events, `loop:`, row actions) in a run or in DMLS. A host that
registers a list kind names its own rows property (Claudine: `sequence`).

- Cost: `kind: sequence` with `list:` is the form Claudine retired on
  2026-07-12; its error tells authors to rename `list:` to `sequence:`. That
  message changes to offer both fixes.

**Option B: `sequence:`.** One rows property everywhere.

- Cost: Claudine's sequence trigger and formal-sequence classifier must both
  exclude `kind: list`, and DMLS offers sequence completions inside a list
  until they do. A missing or misspelled `kind:` silently makes a list a
  sequence.

### Decision 2: the option that returns the raw file

`load_data(f)` changes from the whole mapping to the rendered rows for list
files. A caller that needs the mapping (a tool that edits the roster) needs a
way to ask for it.

**Option A (recommended): `raw: true` in `options`.** Explicit, and it
leaves every other file's behavior alone.

**Option B: no option; read the rows property instead**
(`load_data(f).list` cannot work, since the result is now an array). The
caller would have to use another function, such as `frontmatter`, which does
not read YAML files.

- Cost: no way to read a list file as plain data from an expression.

## Scope of the work

- `schemas/kind.yaml`: add `list`.
- `darkmatter/schemas/`: the `kind: list` definition, its DMLS schema, and
  host registration of further list kinds (name, rows property).
- A Darkmatter `list` module that renders a list (the four steps, the
  template scope, `$schema`) with registered `dm.list.*` codes and typed
  detail. The rules move from Claudine's
  `claudine/lib/src/composition/sequence/` (`normalize`, `formal`) and
  Claudine calls the Darkmatter function; no second copy stays behind.
- `load_data`: list detection, the `raw` option, rendering, typing from the
  file, the memo key, and the read set.
- Docs: `darkmatter/docs/topics/loading-data.md` (a "Loading a list" section
  and the changed `kind` rule), a new `darkmatter/docs/topics/lists.md`
  topic page, the `use-darkmatter` and `use-dmls` caller skills, and the
  `darkmatter` repo skill.
- `osc-9-conemu.md` keeps its `load_data` call unchanged and composes.

## Success criteria

- One test renders the same list file through `load_data`, a Claudine
  Markdown sequence, and a Claudine YAML sequence (for a `kind: sequence`
  file) with the same outer values, and asserts identical rows, apart from
  the actions a Markdown sequence removes.
- `md compose biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
  gets past the `apps` type error. Its other missing inputs are supplied by
  the sequence that runs it.
- `load_data` on a list file with an inline `$state` is typed in the call
  check and in DMLS; with a missing file it is untyped and reports nothing.
- `raw: true` returns the mapping as today.
- Every `dm.list.*` failure points to a line in the list file.
