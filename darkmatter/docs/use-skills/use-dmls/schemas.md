# Schemas in DMLS

Schemas make frontmatter completion, hover, and diagnostics specific to
your documents. DMLS reads the same schema files as `md`, using the same
library, so the editor and `md schema validate` always agree. Schemas are
written in **SimplifiedSchema**, the compact type language:
`title: string(required)`, `tags: string[]`, `status: enum(draft, published)`.
Run `md schema about` for the full vocabulary.

All the examples below were checked with `md schema validate` in a scratch
repository.

## Which schema a document gets

```text
built-in baseline           always: types title, style, content_policy, lifecycle stacks, …
  + [schema.extensions.*]   from .dmls.toml, when a glob matches the file
  + matching trigger files  schema-trigger files in the document's schema roots
  + the document's $schema  wins over everything above
  = the effective schema    drives diagnostics, completion, and hover
```

A later layer replaces an earlier layer's definition of a top-level key as a
whole. So a key that the document's `$schema` redeclares is completed and
validated from the document's definition alone.

## Schema roots: where schema files live

DMLS and `md` look for schema files in exactly five folders, first match
wins:

| # | Root | Example |
|---|---|---|
| 1 | the folder `$SCHEMAS_DIR` names (absolute path, the folder itself) | `/opt/team/schemas/` |
| 2 | the document's package | `{repo}/area/pkg/schemas/` |
| 3 | the document's package area | `{repo}/claudine/schemas/` |
| 4 | the file tree root (the repository root) | `{repo}/schemas/` |
| 5 | your home folder | `~/schemas/` |

- The roots belong to **the document**, not to the folder you opened. In the
  editor they're searched for any document inside an open workspace folder.
- `schemas/` folders anywhere else, such as `docs/schemas/`, are never
  searched.
- A missing folder is skipped. A folder that exists but can't be read is an
  error. DMLS never falls back to a later root in that case.
- A file name in an earlier root **shadows** the same name in later roots,
  even when the earlier file is broken.
- `[workspace]` settings don't affect the roots.

`md schema triggers doc.md` prints the roots for a document, which triggers
matched (condition by condition), which files were shadowed, and which were
skipped as invalid. Use it whenever the editor applies a schema you didn't
expect, or doesn't apply one you did.

## The `$schema` property

```yaml
---
# 1. inline
$schema:
  title: string(required) -> shown in hover
  tags: string[]
---
```

```yaml
---
$schema: note.yaml          # 2. a bare name: looked up in the schema roots
# $schema: ./note.yaml      #    ./ or ../ : beside this document only
# $schema: ~/schemas/x.yaml #    ~ or an absolute path also work
---
```

A YAML list under `$schema` is a union: each item is a complete object
schema or a file. Hover on the value shows where it resolves. When the file
is missing, the link opens the path where it would be created.

A `$schema` file must be one of these:

- a YAML file with a top-level `$schema`;
- a `.json` JSON Schema object whose `$schema` is exactly
  `https://json-schema.org/draft/2020-12/schema`.

Anything else is an error, including a types-only library and a trigger
file.

### Required, `null`, and defaults in the editor

A missing key and `null` are the same value. `required` only asks that the
key be written; a written `null` is then judged by the type. A `default(...)`
fills a missing or `null` value, required or not.

```yaml
---
$schema:
  retries: number(default(3))   # never "missing", never null in expressions
  mode: number(required)        # `mode: null` → error: null is not a number
  note: string                  # optional, no default: may be null
---
```

- A defaulted key is never reported missing, even with `schema.strict`, and
  hover shows `Default: 3` on the key and its uses.
- Only a property with no default and no `required` is "possibly null" to
  expression checks.
- A default that breaks its own definition is reported on the `$schema`
  declaration. DMLS never writes the default into your file.
- `non-null` is no longer part of the grammar and is a schema error; use
  `not-empty` to refuse blank text, an empty list, or an empty mapping.

## The schema file kinds

| File | Top-level keys | Used how |
|---|---|---|
| informal schema (no `kind`) | `$schema` (required), `types`, `description`, `categories` | a `$schema` target |
| `kind: schema` | `$schema` and/or `types`, `description`, `categories` | a `$schema` target if it has `$schema`; otherwise a **type library** for imports |
| `kind: schema-trigger` | **only** `kind`, `match`, `$schema` | applies automatically to matching Markdown documents; never named in `$schema` |
| `kind: yaml-schema` | `kind`, `$schema` (required), `types`, `description`, `categories` | applies automatically to **YAML files** whose `kind` it claims |
| `.json` | a Draft 2020-12 JSON Schema | a `$schema` target. It validates, but adds no completion or hover |

Any other top-level key is a malformed-envelope error.

### A type library and an importing schema

```yaml
# schemas/people.yaml — a type library, never a $schema target
kind: schema
types:
    person:
        name: string(required)
        email: email
```

```yaml
# schemas/note.yaml — an informal schema, a $schema target
description: A note in docs/notes
$schema:
    title: string(required) -> the note's title
    status: enum(draft, published)
    author: person@people.yaml     # Name@file imports a named type
```

The `file` in `Name@file` uses the same roots: a bare name is searched for
in them, and `./` means beside the importing file.

### Reusing an object's properties (`...name`, `$extends`)

Copy the properties of a named object type into another object, then
override what differs. Write `...name` inside an inline object, or
`$extends` in a YAML schema mapping:

```yaml
kind: schema
$schema:
    $extends: events                                # the root gets id and price
    source: string
    snapshot: "{ ...events, price: number(min(0)) }"
    order:
        $extends: [events, person@people.yaml]     # a nonempty list, applied in order
        total: number
types:
    events: "{ id: string(required), price: number }"
```

- **Later wins, as a whole property.** Entries apply left to right, and
  `$extends` applies before the mapping's own keys wherever it is written.
  `{ ...events, id: number }` makes `id` an optional number: its
  `required`, default, and description are gone too.
- **Operands are plain names:** `events`, `events@this`, or
  `name@file.yaml`, looked up like any named type. Write `string@this` for a
  local type named after a built-in. No `(…)`, `[]`, `->`, or template on the
  operand; a postfix after the object (`"{ ...events }(required)"`) applies
  to the property.
- **Only one declared object composes.** A primitive, `object`, a union, a
  list, or a function is an error at the operand, and so is a name or file
  that does not resolve, even when every copied property is overridden.
  `a: "{ ...b }"` with `b: "{ ...a }"` is a cycle error, but recursion
  through a child value works: `base: "{ children: branch[] }"` with
  `branch: "{ ...base, label: string }"`.
- **The target's own object constraints are not copied;** the receiving
  object's `$constraints` judge the merged properties. Copied properties
  resolve names, examples, and defaults in the file that declared them, and
  never become new type names.
- **`$extends` is reserved in schema mappings.** A schema that used it as a
  property name, such as `$extends: string`, now gets an error explaining the
  reservation; declare a literal `$extends` property in a JSON Schema file
  instead. Documents may still hold a `$extends` data key.

In the editor:

- **Completion** works right after `...`, in a `$extends` value, and in each
  flow (`[a, b]`) or block (`- a`) list item, while the operand is still
  incomplete. It offers local type names (as `Name@this` when the name is
  also a built-in, such as `string@this`) and imported `Name@./file.yaml`
  names. It leaves out
  targets it knows cannot compose (non-objects, alias cycles), keeps names it
  cannot check, and never offers keywords or constraints there.
- **Hover and go-to-definition** work on operands in both forms.
- **A failing operand** is underlined as a whole, as
  `dm.schema.invalid_type_definition`, with the target's declaration as
  related information.
- **Editing an imported target,** even in an unsaved buffer, refreshes the
  receiving schema's diagnostics and its property completions.

## Schema triggers

A trigger applies a schema to documents that match its conditions, so
documents don't need a `$schema` of their own. Put it in a schema root:

```yaml
# schemas/notes.trigger.yaml
kind: schema-trigger
match:
    $path:
        - "&docs/notes/**"   # every note under the repo's docs/notes …
        - "!**/_*.md"        # … except files starting with _
$schema: note.yaml           # a payload file (bare name: schema roots), or an inline mapping
```

With this trigger, `docs/notes/a.md` is validated against `note.yaml`, and
`docs/notes/_draft.md` gets only the baseline.

### `match` conditions

```yaml
kind: schema-trigger
match:
    - kind: literal(report; required)     # top-level list = AND
    - group:                              # group = OR
          - prompt: string(required)
          - steps: unknown[](required)
$schema:
    title: string(required)
    summary: string
```

- **Combinators:**
  - a top-level list, `all`, and sibling keys in one mapping mean **AND**;
  - `group` means **OR**;
  - `none` means none of its entries may match;
  - `min-match: { count, of }` means at least `count` of `of` must match.

  OR is always an explicit `group`. A nested list with no combinator is an
  error, and so is the retired `any`.
- **Property conditions** are SimplifiedSchema types:
  - `kind: literal(report; required)` requires the value;
  - without `required`, the key may be absent, but if present it must match
    the type;
  - `required` proves presence on any type: `unknown(required)` needs the
    key written, with any value, `null` included. Write `unknown[](required)`
    to also require a list.
- **Only static constraints are allowed** in a match: `required`, `enum`,
  `pattern`, lengths, ranges, and counts. `eager`, `default`, `example`,
  `generated`, and imports are rejected.
- **A trigger can't match everything.** Every way to satisfy the match must
  require a property or match a path, and an empty condition is rejected.
- **`$path`** takes glob references:
  - positive patterns admit a path, and `!` patterns exclude one;
  - a pattern with no folder (`SKILL.md`) matches that file name at any
    depth;
  - bare, `./`, and `../` patterns are read from the folder that holds the
    trigger's `schemas/`. For a trigger in `~/schemas` or `SCHEMAS_DIR`, they
    are read from the document's repository root;
  - `&`, `^`, `~`, and absolute paths work. `~/notes/**` applies across all
    your notes;
  - `@`, `%`, `vault:`, and `{{{VAR}}}` are errors.
- Conditions on file existence, executables, OS, time, and arbitrary
  expressions are **planned, not supported**. A trigger that uses one is
  skipped as invalid.

### When a trigger is broken

A trigger file that fails to load is **skipped**, and the other triggers
still apply. Documents that would have used it get a
`dm.schema.invalid_trigger` **warning** naming the trigger. If the editor
attaches DMLS to YAML, opening the trigger file itself shows **errors** at
the exact node, as `md schema validate schemas/x.yaml` does.

### When changes take effect

- **Saved changes** to a trigger, a payload, or an import apply as soon as
  DMLS sees them: through a watched-file event, or on the next save in
  clients without a watcher.
- **Unsaved** edits in a trigger or `$schema` file reach Markdown documents
  only after you save.
- **Hover** on a document always shows its current effective schema.
- **If a schema fails to load mid-edit**, completion keeps serving the last
  good schema.

## YAML documents (`kind: yaml-schema`)

A family of YAML files can share one contract, with no `$schema` in each
file:

```yaml
# schemas/roster.yaml
kind: yaml-schema
description: The platforms a team publishes to
$schema:
    kind: literal(roster; required)   # the kind this definition claims
    roster_version: literal(1; required)
```

Any YAML file with `kind: roster` is now validated against it. A YAML file
whose `kind` no definition claims is judged by its root `$schema`, if it has
one. Otherwise it isn't validated.

- YAML files never get the baseline, extensions, or triggers.
- Missing and unknown keys are always errors, because a YAML file has no
  compose step.
- Every root's definitions count; there's no shadowing here. Two definitions
  claiming the same kind is an error.
- In the editor, this needs DMLS attached to YAML buffers; see
  [install.md](install.md#yaml-files).

## Always-on schemas for parts of the tree (`.dmls.toml`)

```toml
[schema.extensions.team]
path = "schemas/team-prompt.yaml"   # a document schema (relative to the first workspace folder)
globs = ["prompts/**"]              # relative to the workspace folder
```

Extensions apply in name order, and they are **editor-only**: `md` doesn't
read them. Use a trigger with `$path` when `md` should apply the schema too.

## Claudine

Claudine's schemas are ordinary trigger files in `claudine/schemas/`. DMLS
needs no configuration for them.

- **Under `claudine/`**, documents find them through the package-area root.
- **Elsewhere**, put the complete set of eight files in the repository
  root's `schemas/`, an absolute `SCHEMAS_DIR`, or `~/schemas`:
  `action.yaml`, `lifecycle.yaml`, `sequence-types.yaml`, `linking.yaml`,
  `document.yaml`, `claudine.yaml`, `inline-compose.yaml`, and
  `sequence.yaml`. They import each other, so all eight are needed.
- **Use regular files.** A symlinked trigger is ignored.
- **For Claudine's globals and lifecycle checks**, also copy `globals.yaml`
  and `globals.lifecycle.json`.

DMLS reads these only from disk. An installed `claudine` binary doesn't make
them available to the editor.

## Schema templates

A `$schema` or trigger payload containing `{{{ … }}}` slots is resolved in the
background. Until the first result arrives, diagnostics show
`dm.schema.slot_pending`. A slot that needs to read a file you haven't
approved shows `dm.schema.slot_approval_required`, with quick fixes that
grant the read.
