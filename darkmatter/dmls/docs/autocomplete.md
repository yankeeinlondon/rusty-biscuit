---
kind: canonical-documentation
---

# DMLS Autocomplete

Autocomplete (the LSP `textDocument/completion` capability) offers context-aware
suggestions as you type: link paths, heading anchors, fenced-code languages,
wiki targets, frontmatter schema keys and values, directives, and `{{ }}`
interpolation variables and functions.

## Eager by design

DMLS completion is **eager and self-contained**:

- Every item carries an explicit **`textEdit`** that replaces the exact token
  under the cursor — nothing is left to the client to compute.
- The server advertises **`resolveProvider: false`**: it never defers `detail`,
  `documentation`, or `textEdit` to a `completionItem/resolve` round-trip. All
  fields are present on the first response.
- Insertion is **plain text, not snippets** — no tab stops, and functions are
  inserted as a bare name with no synthesized parentheses.

This is deliberately **Zed-safe**: Zed does not resolve `completionItem.textEdit`
for performance, so a server that relies on resolve would silently misbehave
there. Eager `textEdit` works identically across VS Code, Zed, Neovim, and Helix.

## Trigger characters

Completion is offered on manual invocation and automatically after these
characters:

| Character | Triggers |
|-----------|----------|
| `/` | link and wiki path segments |
| `#` | heading anchors (`[...](doc.md#…)`, `[[target#…]]`) |
| `(` | directive options / expression function arguments |
| `.` | `ctx.` and `fm.` members inside an open `{{ }}` interpolation |

Each provider re-checks context, so a trigger character in ordinary prose
produces nothing. In particular, a `.` outside an open interpolation yields no
DSL completion items.

## What answers a completion

Completion is a provider-chain capability. Each provider contributes items for
the contexts it owns, and the registry **merges them (union, de-duplicated)** —
unlike hover, which stops at the first non-empty result. Providers run in
registration order (substrate → wiki → frontmatter → DSL).

| Provider | Context | Offers |
|----------|---------|--------|
| substrate (Markdown) | inside a link path / after `#` | document paths; `#`-anchor names for the target; fenced-code language tokens |
| wiki | inside `[[ … ]]` | wiki targets (path style configurable) and `#heading` names; never inserts a `.md` extension |
| frontmatter | inside the frontmatter block | schema keys (required-marked), enum values, boolish scaffolds, `file(...)` paths, `style.*` keys, `suggest(...)` value candidates, and `ctx.*` variables |
| DSL | on a `::` line, inside `{{ }}`, or after a frontmatter `$( … )` | directive names, per-family option keys/enum values, interpolation variables/functions, and `$()` suffixes |

### `suggest(...)` value candidates

A property whose effective schema carries a `suggest(...)` constraint offers its
candidates as **advisory** value completions — they aid discovery but never
validate (any value the type allows remains legal). Candidates are
prefix-filtered against the partial value, offered both for scalar values and
for block-sequence items (`- value`), and a suggest-bearing union arm wins over
the enum / boolish / file families for the same position. The candidates come
from the schema itself (via the library's `suggestions_for_path`), so DMLS
holds no parallel suggestion table.

### Items of an array of a union

When an array's items are a union (`policy[](min(1))@./content-policy.yaml`,
or a property union of several array types), completion inside an item
comes from the library's `alternatives_for_def`:

- On a new `- ` line, every object alternative's keys (`rule: `, marked
  `(required)` when every alternative offering it requires it) and every
  scalar alternative's values (`enum` members, `literal(...)` values, and
  `suggest(...)` candidates) are offered together.
- On a later line of an object item, the keys the item already has are left
  out. If a `literal(...)` tag in the item selects exactly one alternative
  (`kind: say`), only that alternative's keys are offered; a missing,
  unknown, or conflicting tag keeps every alternative's keys.

```yaml
$schema:
    step:
      - "{ kind: literal(say; required), text: string(required) }"
      - "{ kind: literal(shell; required), run: string(required) }"
    steps: step[]
steps:
  - kind: say
    # offers `text`, not `run`
```

### Recursive types and new list items

Below a recursive named type (`item: "{ value: string, children: item[] }"`),
completion offers the referenced definition's keys and values at every
depth. Each step of the cursor's path opens only the definition it stands
on, through the library's `resolved_alternatives`, so a type the path never
reaches is never read. On an empty `- ` line of a list of objects, recursive
or not, the object's keys are offered (marked `(required)` where they are),
as they are for an object that has no keys yet. See
[Recursive types](../../docs/topics/schemas/dmls-schema-support.md#recursive-types).

### Only the cursor's path is typed

Completion resolves the schema along the cursor's own path and reads the
document's stored schema shapes, so completing in one item of a long list
costs the same as in a short one. It never classifies the other entries;
that happens once, on the first diagnostics pass, and completion may read
its result when it exists. See
[Frontmatter typing and its reuse](./hover.md#frontmatter-typing-and-its-reuse).

### Lifecycle stacks

The items of md's lifecycle events (`initialize`, `composed`, `failed`,
`cancelled`, `finalize`) complete from the baseline's
[lifecycle typing](../../docs/topics/schemas/definition.md#lifecycle-event-typing),
so they offer md's verbs only (`info`, `stdout`, `shell`, `set`, `stop`, …).
A verb only another host provides, such as `say`, `retry`, `notify`, or
`proxy`, is accepted there but never offered, and `shell` is not offered in
`initialize`. A key only another host treats as an event, such as Claudine's
`start`, is untyped under the baseline and offers neither. `then` and `else`
are not offered inside these stacks, because md's conditionals type them with
one pattern key.

```yaml
composed:
  - |      # offers `info`, `warn`, `shell`, `set`, `stop`, ...; not `say`
```

Inside a `{{ … }}` span or a `when` condition in such a stack, the
interpolation candidates (described below) also include the globals that
event may read: `err` and its members in `failed`, but not in `composed`.
Under a host with a
[lifecycle artifact](../../docs/topics/lifecycle/host-artifact.md), the
host's events are stacks for this purpose too: under Claudine, `err` is
offered in `blocked` and `failure` and not in `start` or `success`.

### Schema definitions

Inside a schema definition (a standalone schema file, a frontmatter
`$schema`, or a `type-definition` value), completion reads what you are in
the middle of typing from the library's tolerant cursor parser:

- At a type position, including a union arm, it offers every built-in type
  keyword and its `[]` form, plus aliases labeled as such (`any`, detail
  ``Alias of `unknown` ``), and your named types.
- Inside `alphanumeric(`, before the first `;`, it offers the type options
  not yet written in that group, each with its effect as `detail`. While the
  group is still empty the constraints are offered too; once it holds an
  option, only options are; after `;`, only constraints are.

```yaml
types:
  slug: alphanumeric(allow-dashes, |   # offers the other eight options
  id: alphanumeric(allow-dashes; |     # offers `min`, `max`, `required`, ...
```

Options are not offered inside a reference to a named `alphanumeric` type:
a use site may only repeat its definition's options.

## Shell suffix completion (`$( … )::`)

After the closing `)` of a frontmatter `$( … )` value, completion offers the
five suffixes: `::ok`, `::exit-code`, `::result`, `::timeout:<seconds>`, and
`::no-cache`, filtered by the `::…` you have typed. A suffix the value already
carries is not offered again, and neither is a second result suffix, so after
`$(git diff --quiet)::ok::` only `::timeout:<seconds>` and `::no-cache` remain.
Each item's `detail` says what the suffix does; `::timeout:` is inserted
without a number.

## Interpolation completion (`{{ … }}`)

Inside an interpolation, the DSL provider offers these candidate families:

1. **Frontmatter keys** — the document's own top-level keys, plus the
   top-level properties its own `$schema` declares but leaves unset. A key
   named `doc`, `hook`, or `current_env` is offered like any other key.
2. **The `fm` root and its members** — `fm` itself, documented with its
   description from Darkmatter's reserved-root catalog, and, once the partial
   starts with `fm.`, each frontmatter key as `fm.<key>`. A key the dotted
   form cannot reach as one path (for example `$schema`) is not offered after
   `fm.`.
3. **`ctx.*` context variables** — offered fully qualified (`ctx.today`,
   `ctx.packages`, …).
4. **Expression functions** — every function in the catalog, including the
   eight list-formatting functions (`as_csv`, `as_tsv`, `as_space_separated`,
   `as_line_separated`, `as_unordered_list`, `as_ordered_list`, `as_json`,
   `as_json5`).
5. **Host globals** — every global a host catalog in the document's schema
   roots declares, such as Claudine's `seq`, `loop`, and `group`.
6. **Event globals** — inside a lifecycle stack only, the globals that
   event may read under the document's host: md's events, or a host's from
   its [lifecycle artifact](../../docs/topics/lifecycle/host-artifact.md),
   such as Claudine's `start` or `failure` (see
   [Lifecycle stacks](#lifecycle-stacks)).

Matching is **prefix-based and case-sensitive**. `{{ ctx.pa }}` offers matching
`ctx.*` variables; it does not offer the removed `*_list` aliases.

These candidates are names, not values: interpolation completion does not
offer a property's enum members, literals, or object members. An enclosing
guard such as `::block when="mode != 'release'"` therefore leaves the
candidate list unchanged; hover shows the narrowed type instead
([Hover](./hover.md#formatting-rule-narrowed-local-type)).

### `ctx.*` candidate discovery

A `ctx.*` variable is offered whenever its fully qualified name is a
case-sensitive prefix match of the current partial — including an empty partial.
`{{ ` (empty), `{{ ctx`, and `{{ ctx.pa` all offer matching `ctx.*` variables, so
`{{ ctx.pa }}` offers `ctx.packages`. This is the same prefix rule
(`starts_with(partial)`) the frontmatter-key and function families use in
`interpolation_candidates`.

The two namespaces never bleed into each other, but each can be offered
independently when its prefix matches: a bare `today` candidate is the
**frontmatter** key `today`, not an alias for `ctx.today`, while the qualified
`ctx.today` is a separate candidate offered whenever the `ctx.t…` prefix matches.

The explicit-`ctx.`-prefix requirement is a **hover classification** rule, not a
completion restriction: a bare `{{ today }}` must not receive `ctx.today` *hover*
metadata even when `today` is a known context-variable tail. See
[Hover](./hover.md) for that classification.

## Completion item metadata

Items populate the LSP fields intended for type and documentation so editors can
render rich completion popups without a resolve round-trip.

**`ctx.<name>` variable:**

| Field | Value |
|-------|-------|
| `label` / inserted text | the qualified `ctx.<name>` |
| `kind` | `VARIABLE` |
| `detail` | the descriptor's rendered type (e.g. `string[]`) |
| `documentation` | eager Markdown containing the descriptor's description |
| `textEdit` | eagerly replaces the current interpolation token |

**Expression function:**

| Field | Value |
|-------|-------|
| `label` | the untyped signature (e.g. `as_csv(list)`) |
| inserted text | the bare function name (no parentheses) |
| `detail` | the typed signature (e.g. `as_csv(list: any[]) -> string \| error`) |
| `documentation` | the descriptor description as eager Markdown |

The type and documentation come from the single-sourced Darkmatter catalogs via
one passive adapter — DMLS does not maintain a parallel table of variable types,
function signatures, or descriptions, so completion never drifts from the
library's semantics.

## Configuration

Via `.dmls.toml`, overridden by the editor's `workspace/didChangeConfiguration`
settings and reloadable without restart (see
[DMLS Configuration](../../docs/lsp/configuration.md)):

- **`wiki.path_style`** — `shortest` / `relative` / `root-relative`: how an
  inserted wiki target is spelled.
- **`wiki.heading_completion_style`** — how `#heading` completions are rendered.

Wiki completion never inserts a `.md` extension regardless of style.

## Passivity

Answering a completion is read-only: it reads open buffers and the in-memory
workspace graph (parsing an unopened link target the first time heading
completion needs its headings) and resolves local paths, but never executes a shell command,
evaluates an expression, fetches a remote URL, or mutates a file. Offering a
`ctx.*` variable or a function does **not** compute its value.

## Not included in v1

- LSP `textDocument/signatureHelp`, active-parameter tracking, and automatic
  insertion of parentheses or arguments.
- `completionItem/resolve` (everything is eager).
- Snippet insert text / tab stops.
- Compatibility completions for the removed `ctx.*_list` variables.

## See also

- [Hover](./hover.md) — the sibling capability; how `ctx.*`, functions, and
  directives are *explained*, and the Markdown-formatting limits DMLS works
  within.
- [Diagnostics](./diagnostics.md) — problem reporting and the stable code taxonomy.
- [Features](./features.md) — the full capability overview and per-editor matrix.
