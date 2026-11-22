# Objects and Named Types

Inline object literals type a nested mapping in one string; named types let one definition be reused, imported from another file, or refer to itself; [object composition](#object-composition-name-and-extends) copies the members of one named object into another.

## Inline Object Literals

The type-expression grammar accepts an **inline object literal** — a single string scalar that declares the shape of a nested object. This is the way to type object-typed properties and arrays of objects without dropping down to an external JSON Schema file. The motivating case is the typed object array:

```yaml
$schema:
    authors: "{ name: string(required), email: email }[]"
```

### Forms

An inline object appears wherever a type expression is valid: as a single property, a property-level union arm, or the item type of an array.

```yaml
$schema:
    # Array of typed objects — the motivating case
    entries: "{ foo: string(required), bar: string }[]"

    # Optional single typed object
    config: "{ host: string(required), port: number(default(8080)) }"

    # Required single typed object — the postfix constraint applies to config
    config_required: "{ host: string }(required)"

    # Multi-line for readability (whitespace inside braces is ignored)
    endpoints: "{
        url: url(scheme(https); required),
        method: enum(GET, POST, PUT, DELETE; required),
        timeout: number(default(30))
    }[]"

    # Nested objects
    database: "{
        primary: { host: string(required), port: number(default(5432)) },
        replicas: { host: string, port: number }[]
    }"

    # Required non-empty array — postfix constraints after [] apply to replicas
    replicas: "{ host: string }[](min(1); required)"

    # Inline object as a union arm
    payload:
      - "{ type: enum(foo; required), foo_id: string(required) }"
      - "{ type: enum(bar; required), bar_count: number(required) }"

    # Mixed: inline object array or a plain string fallback
    metadata:
      - "{ key: string(required), value: string(required) }[]"
      - "string"
```

The inline object body parses identically in single-line and multi-line form because the parser strips whitespace after `{`, around `,` and `:`, and before `}`. Braces are not allowed inside constraint argument lists, so there is no ambiguity with the existing constraint syntax.

### Identifier Rules

Inline object property names are always **unquoted string-layer identifiers** — unquoted ASCII alphanumeric characters plus `-` and `_`, including leading digits. The same scanner is used elsewhere in the grammar.

Accepted: `name`, `foo_id`, `x-custom`, `api2_version`, `123abc`.

Rejected: `display name`, `@type`, `x.custom`, `"x-custom"`. There is no quoted-property-name form in this feature; rename the property to a valid identifier or drop down to a JSON Schema file for richer naming needs.

### Descriptions

Each property inside `{ ... }` may carry a `-> description` suffix that follows the same four syntax forms as top-level properties. Inside an inline object, **descriptions terminate at the next top-level comma or closing brace** at the current nesting level. Commas inside an inline property description are not supported by this feature — keep descriptions comma-free inside `{ ... }`. A description is plain prose: quotes and square brackets in it mean nothing, so `{ a: string -> (it's fine), b: number }` and `{ a: string -> plain [x, b: number }` both end `a`'s description at the comma before `b`. Top-level descriptions (outside any inline object) still consume the rest of the scalar string after `->` exactly as before.

### Postfix Constraints

Inline objects support the same postfix constraints as primitive atoms.

- A **single inline object** with a postfix constraint — `{ host: string }(required)` — applies the constraint to the containing property. `required` hoists to the parent `required` array; `default(...)` sets a default on the object property.
- An **inline object array** with constraints after `[]` — `{ name: string }[](min(1); required)` — applies those constraints to the array property itself (`minItems`, parent `required`). Constraints on nested properties remain inside the inline object fragment and become JSON Schema constraints on each `items` object.
- A trailing comma after the last property is allowed: `{ foo: string, }` is identical to `{ foo: string }`.

### Nesting Depth

The inline object parser enforces a hard maximum of **32 nesting levels** of inline objects. Exceeding that depth returns `SchemaError::Grammar`. The limit is the same for single inline objects, inline object arrays, and unions whose arms contain inline objects — depth is counted at every level of nested `{ ... }`.

```yaml
$schema:
    # OK: 3 levels of nesting
    a: "{ b: { c: { d: string } } }"

    # Error at depth 33: SchemaError::Grammar
    deep: "{ a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: { a: string } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } }"
```

The cap limits how deeply one type string is written, not how deeply a document may nest. For a structure that repeats at every level, such as a tree, declare a [recursive named type](#recursive-named-types). Otherwise, if you need deeper typing, reference an external JSON Schema file with `$schema: ./path/to/schema.yaml` (or use a root-level union whose arm is a file reference).

### `additionalProperties: false`

Every inline object compiles to a JSON Schema fragment that sets `additionalProperties: false`. This is the intended default — declaring an inline object is a signal that the author wants shape restriction. It differs from the root schema default (`additionalProperties: true`, which a root opts out of with [`closed`](#closed-roots-closed)) and from the opaque `object` type, which still compiles to `{ "type": "object" }` with no `additionalProperties: false`. Authors used to the root default may be surprised; this is documented behavior. A future `lenient` constraint on the inline object body may opt back to `true`.

### YAML Mapping vs. Inline Object

A nested object can also be written as a YAML mapping instead of a quoted string. A mapping at a property position, as a property-union arm, or as a named type's definition is the same object shape as the equivalent inline object literal: each entry is parsed as a property definition (a type string, another mapping, a sequence union, or `null`), and the object is closed like any inline object.

```yaml
$schema:
    # These two properties have the same shape
    config: "{ host: string(required), port: number(default(8080)) }"
    config_block:
        host: string(required)
        port: number(default(8080))
```

- **The mapping form holds what a one-line string cannot:** a
  [`$constraints` block](#object-arity-constraints-min-keys--max-keys),
  quoted [pattern keys](#pattern--dictionary-keys), and a
  [`$extends` directive](#object-composition-name-and-extends).
  `required: true` or `generated: true` in its `$constraints` block applies
  to the property, as the `(required)` postfix does on a string.
- **Every key is a property.** A key named `type` declares a property called
  `type`; there is no long-form `type:` descriptor.
- **The nesting cap counts mappings too.** Each nested mapping is one level
  of the 32-level [nesting depth](#nesting-depth).
- **Export uses this form.** `to_simplified_yaml()` writes a spread-free
  inline object as a block mapping.

### JSON Schema Output

An inline object compiles to the same Draft 2020-12 JSON Schema shape a hand-written `{ "type": "object", "properties": ..., "required": ..., "additionalProperties": false }` would produce, with `required` populated from per-property `required` constraints and the inline object's own `required` postfix hoisted to the parent `required` array.

For an array of inline objects, the `items` sub-schema is the inline object fragment and `minItems` / `maxItems` / `uniqueItems` / `required` come from the postfix constraints after `[]`.

## Composition Primitives

Four primitives compose named types, dictionaries, and content-format strings on top of the base grammar. They are additive — a schema that uses none of them parses and compiles exactly as before.

### `example(...)` Constraint

`example(...)` attaches one or more **example artifacts** to a property. Examples are documentation, like a richer `description` — they do **not** constrain the annotated property's value. Comma-separated file references resolve relative to the referencing schema file (magic paths and `this` are honored, mirroring `$schema` resolution).

```yaml
$schema:
    today: "date(required; example(./ctx/today-example.yaml)) -> Local date"
    demo:  "string(example(./a.yaml, ./b.yaml))"
```

Each referenced file is validated at **schema-load time** — a missing, malformed, or invalid example is a schema-load error (fail loud; the point is trustworthy examples). Validation has two layers:

1. The common **envelope** validates against the built-in example schema (`kind`, `invocation`, `returns`, `description`).
2. Target-specific fields such as `parameters` validate against the inherited target shape (for expression-function examples, an array of single-key `name → value` maps).

Resolved example objects are emitted onto the JSON Schema as the `x-darkmatter-example` extension so downstream consumers (`md schema about`, DMLS hover) read them without re-reading disk. Unchanged example files are cached by content hash across warm loads.

A `function` type also takes inline examples, `example(call => result)`; see
[Function Signatures](unions.md#function-signatures). The `=>` tells the two forms apart,
and the inline form on any other type is a schema-load error.

### Cross-File Named-Type Imports (`Name@file`)

`Name@fileref` gives the use site the definition of the named type `Name` from the referenced file — structural substitution: each use validates exactly as if the definition were written there. In the generated JSON Schema most uses are such a copy. Two kinds of type are stored once and referenced instead: a type that refers back to itself (see [Recursive Named Types](#recursive-named-types)), and a structured type that two or more places use (see [Shared Named Types](#shared-named-types)). A schema file's **`types` entries are its named types** when that table is present beside an exported `$schema`; without a separate table, the mapping payload's top-level entries serve as both properties and named types (see [Standalone Schema Documents](schema-documents.md#standalone-schema-documents)). Each is a full SimplifiedSchema definition (scalar, enum, union, or an inline object with literal or pattern keys).

A type defined in the same schema may be referenced by its **bare name**: `type` and `type@this` are the same reference. See [Local Type References](local-type-references.md) for where local names live, built-in collisions, and why a bare name inside an imported definition still resolves in its defining file.

```yaml
$schema:
    type:       type@./types.yaml        # inlines types.yaml's `type` definition
    parameters: parameter[]@./types.yaml # array of the inlined `parameter` map
    value:      kind                     # the `kind` defined in *this* schema
    other:      kind@this                # the same reference, spelled explicitly
    kind:       enum(a, b)
```

Grammar: `type_ref := ident ('(' constraints ')')? ('[]' ('(' constraints ')')?)? ('@' fileref)?`. A bare `ident` that is a built-in type keyword is the built-in; any other bare `ident` is a local reference. Postfix composes **outward**:

- `Name[]@file` — an array of the inlined `Name`.
- `Name(constraints)@file` — the inlined type with `constraints` merged into its own (see [Merging use-site constraints](#merging-use-site-constraints)).
- `Name[](constraints)@file` — an array of the inlined `Name`, with `constraints` on the array.
- `Name -> description` — a use-site description replaces the inherited one; omitting it keeps the inherited description.
- `@this` (or no `@` at all) — the current schema (self-target), including inline top-level documents.
- `@darkmatter:<stem>` — a type from one of the schema partials Darkmatter ships, wherever the importing schema lives (see [Reserved Darkmatter References](#reserved-darkmatter-references-namedarkmatterstem)).

A constraint list before `[]` (`Name(c)[]`) parses, but only a union-typed named type accepts one, and only its property markers; on any other named type it is a schema error, so write the list's constraints after `[]`. Adding `[]` to a named type that is already an array is an error. A reference to a **union-typed** named type accepts `[]` with list constraints, the `required`, `generated`, and `eager` markers (before or after `[]`), and a description, in all three forms (bare, `@this`, and `Name@file`); any other constraint on it is an error. See [Named Union References](unions.md#named-union-references).

The file reference is read as written up to the next `->`, `,`, or `}`, whichever comes first. No other punctuation in it has meaning, so `Name@./a(b.yaml` names the file `./a(b.yaml`, and `Name@./a{b.yaml -> the shared shape` names `./a{b.yaml` and describes it. Inside a YAML flow sequence, quote an arm whose filename contains `[` or `{`, because YAML ends a plain flow scalar there: `['Name@./a{b.yaml', string]`.

Rules:

- The right side resolves as a `biscuit_file::FileReference` whose `cwd` is the schema file's directory, in the request's file-resolution context when one is supplied — the same resolution as root-union file refs and `$schema`.
- The target must be a SimplifiedSchema file whose payload is a mapping with a matching named type. Importing from a raw JSON Schema file, from a root-union schema file, from a file whose whole payload is a reference to another file, or from a missing file is a schema error. A name the target does not define is `SchemaError::UnknownNamedType`, naming the type, the schema searched, and the property that holds the reference.
- **Every constraint of the definition survives the reference**, including `required` and `generated`. An inherited `required` keeps its scope: on `people: person[]` it makes `people` required, and inside one arm of a property-level union it is hoisted to the property like any arm's `required`. Requiring a field *inside* an object type does not require the property that holds the object.
- Expansion is **eager, bounded, and cycle-checked**. A named type may refer back to itself, directly or through other named types in the same or other files, as long as every cycle passes through a child value; see [Recursive Named Types](#recursive-named-types). A type that does not recurse is either copied into each use or, when it is structured and used from two or more places, stored once and referenced (see [Shared Named Types](#shared-named-types)); either way it accepts and rejects the same values. A chain of non-repeating references deeper than 32 is rejected (`SchemaError::ImportCycle`) — the same protection as the inline-object nesting cap.
- Each import of another file is a **dependency edge** recorded on the resolved schema (`ResolvedSchema.imports`) so the schema cache and DMLS index can invalidate when an imported file changes. A local reference adds no edge.
- An inherited `example(...)` reference resolves from the file that declared it, not from the importing schema.
- `to_json_schema` rejects any unresolved import that reaches conversion, exactly as it rejects unresolved root-union file arms — imports are always expanded by the resolver first.

#### Shared Tables and Independent Imports

Importing several names from one file does not copy that file's entire type
library for each reference. Within one import-expansion operation, the resolver
keeps one immutable table of the file's unexpanded definitions. The first
lookup, later lookups, and `@this` lookups inside that file share the same
table. The root document has its own table, copied once before expansion so
local references can still read its original definitions. Declaration order is
preserved.

Each use receives its own selected definition before its constraints,
description, and array modifier are applied. For example, given a type library
whose `name` is an optional string:

```yaml
# types.yaml
kind: schema
types:
    name: string
```

```yaml
$schema:
    primary: name(required)@./types.yaml
    backup:  name@./types.yaml
```

`primary` is required, while `backup` remains optional. Neither use changes
`name` in the library. A reference written inside an imported definition reads
that definition's own file for `@this` and bare local names, even if the root
document declares a type with the same name. Recursive types and repeated
structured types share their compiled bodies as described in
[Recursive Named Types](#recursive-named-types) and
[Shared Named Types](#shared-named-types); use-site modifiers still do not
mutate those bodies.

There are two cache lifetimes. The expansion's namespace cache ends with that
operation. Separately, the running process keeps one entry per imported file
path: the hash of the contents it last parsed successfully and the parsed
table. Every new expansion still reads and hashes the file first, so edits and
deletions are observed; a matching hash saves parsing, rather than suppressing
the read.

- **An unchanged file is reused, however many files the process imports.**
  Editing one file reparses that file alone; other entries are untouched.
- **A changed file replaces its entry.** An operation that already holds the
  earlier table keeps using it until it finishes.
- **A failure is never served.** A file that cannot be read, decoded, or
  parsed fails the import with its own error and loses its entry; fixing the
  file parses it again.
- **Entries are not evicted.** Memory grows with the number of distinct file
  paths a process imports, not with how often they are edited.

```text
resolve (a, b, c imported)   parses a, b, c
resolve again                parses nothing
edit b; resolve              parses b only
break c; resolve             error for c; c's entry dropped
repair c; resolve            parses c
```

Expansion state, dependency tracking, and use-site modifications remain local
to each operation.

This sharing reduces copying and repeated parsing. It does not remove file
resolution, selected-definition copying, or expansion work, and it does not
promise a fixed end-to-end speedup. Schemas without named references skip the
import-expansion machinery.

#### Reserved Darkmatter References (`Name@darkmatter:<stem>`)

`Name@darkmatter:<stem>` names the type `Name` in the schema partial
`darkmatter/schemas/partials/<stem>.yaml` that Darkmatter ships. Like `this`,
`darkmatter:<stem>` is a reserved word, not a file reference, so it means the
same thing whoever writes it and wherever the schema lives:

```yaml
# schemas/notify.yaml, in any repository
kind: schema
types:
    talk: communicate@darkmatter:lifecycle   # exactly one communicate action
    tally: count@darkmatter:lifecycle        # a non-negative integer or a whole-value {{ … }}
    steps: md-stack@darkmatter:md-lifecycle -> what md runs for this event
```

- **The disk is never read.** The partial comes from the copy compiled into
  the library, whether the importing schema is a file on disk, a schema
  Darkmatter embeds, or a document in a host's embedded schema bundle. The
  reference therefore works the same in a checkout, in an installed binary,
  and in a directory that holds only the host's own files.
- **The partial's own references stay inside the shipped partials.**
  `md-lifecycle.yaml` writes `info-short@./lifecycle.yaml`; that always
  reaches Darkmatter's `lifecycle.yaml`, even when the importing schema's
  folder holds a `lifecycle.yaml` of its own.
- **It is not a file dependency.** A shipped partial adds no edge to
  `ResolvedSchema.imports`, and DMLS does not watch it.
- **`<stem>` is a bare file stem**, with no folder and no `.yaml`. A stem no
  shipped partial has (`darkmatter:bogus`, `darkmatter:fm.yaml`,
  `darkmatter:../fm`) is `SchemaError::NoMatch`, whose message names the whole
  reference. A known stem without the type is `SchemaError::UnknownNamedType`.
- Use-site constraints, `[]`, and a `->` description compose exactly as they
  do on `Name@file`.

Darkmatter ships the partials `fm`, `style`, `context-variables`, `error`,
`timing`, `lifecycle`, and `md-lifecycle`. The two lifecycle partials are the
ones written for reuse by other schemas; see
[Lifecycle Event Typing](../../../../darkmatter/docs/topics/schemas/definition.md#lifecycle-event-typing).

#### Merging use-site constraints

Constraints written at a use site combine with the constraints the named type
already carries. This is a merge inside one reused type; it is unrelated to
[schema layering](../../../../darkmatter/docs/topics/schemas/authoring-schemas.md#schema-layering), where the winning
schema's whole property definition replaces lower layers.

```yaml
# types.yaml
kind: schema
types:
    label: string(min(3))
    site: url(scheme(http, https))
```

```yaml
$schema:
    title: label(max(80))@./types.yaml       # 3..80 characters
    short: label(min(5))@./types.yaml        # at least 5 characters
    home: site(scheme(https))@./types.yaml   # https only
```

| Situation | Result | Example |
|---|---|---|
| Different constraints | Both apply | `label` + `max(80)` → 3..80 characters |
| Repeated lower bound | The larger (stricter) wins | `min(3)` + `min(5)` → `min(5)` |
| Repeated upper bound | The smaller (stricter) wins | `max(80)` + `max(50)` → `max(50)` |
| Contradictory bounds | `SchemaError::ConstraintConflict` | `min(10)` + `max(5)` |
| Flags (`not-empty`, `integer`, `unique`, `eager`, `required`, `generated`) | Idempotent | `required` twice = `required` |
| Repeated `pattern(…)` | Every pattern applies | `pattern(^a)` + `pattern(z$)` accepts `az`, rejects `ab` and `bz` |
| Repeated `match(…)` | Groups intersect | `file(match('*.md'))` + `match('guide*')` offers `guide-intro.md`, not `notes.md` or `guide.txt` |
| Repeated `scheme(…)` | Intersection; empty is a `ConstraintConflict` | `scheme(http, https)` + `scheme(https)` → `https` |
| Repeated `default(…)` | Must be equal; different values are a `ConstraintConflict` | `default(3)` + `default(3.0)` is fine; `default(3)` + `default("3")` is not |
| Repeated `suggest(…)` | Error: at most one `suggest` per property | |
| Repeated `example(…)` | All kept, inherited first | each resolves from the file that declared it |
| `alphanumeric` options | The same canonical set is accepted; a different set is a `ConstraintConflict` | `slug(allow-dashes; max(10))` over `slug: alphanumeric(allow-dashes)` is fine; `slug(allow-periods)` is not (see [Alphanumeric Strings](types.md#alphanumeric-strings)) |

The rules in detail:

- **Bounds compare per kind:** string length (`min`/`max` on a string),
  number (`min`/`max` on a number), item count (`min`/`max` on an array), and
  key count (`min-keys`/`max-keys`). Equal lower and upper bounds are valid.
- **Bounds are read against the resolved target.** Parsing a reference does
  not know its target type, so `label(max(80))` means a length when `label`
  is a string and a numeric maximum when it is a number. On a named type that
  is already an array, such as `tags: string[](max(5))`, a use-site `min` or
  `max` is an item count: `tags(min(6))` conflicts with the inherited
  `max(5)`. A fractional length bound on a string target is an error.
- **Item and container constraints never merge.** A named `string(min(3))`
  used as `label[]` requires each item to have at least three characters;
  `label[](min(1))` also requires at least one item. The two minima stay
  separate.
- **Patterns are conjunctive.** `not-empty` counts as a pattern. With a single
  pattern-producing constraint the JSON Schema keeps one `pattern` keyword;
  with several, each becomes its own entry in an `allOf` beside the base
  `type`, `format`, bounds, and `x-darkmatter-*` annotations. Patterns are
  never combined into one regular expression, and Darkmatter does not try to
  prove that they can all match.
- **`match` groups intersect.** Each `match(...)` keeps its own positive and
  `!` exclusion list, and a path must satisfy every group. An exclusion in one
  group never applies to another group's positives. Groups are not
  concatenated, because a combined list would widen the choices instead of
  narrowing them. As with a single `match`, the groups only shape completion
  in a single schema and reject no value there; in a
  [root union](types.md#files) an existing file must satisfy every group of an arm
  for that arm to apply. An empty intersection is not detected at schema-load
  time, which would need a file search: the schema loads, and completion
  offers no files.
- **Defaults compare after number normalization**, so `3` equals `3.0`, while
  `"3"` and `3` differ. Existing type-specific default checks still apply.
- **One `suggest` per property.** The limit covers the whole property,
  including the arms of a union reached through different named types.
  Use-site `suggest(...)` candidates are checked against the resolved target,
  which must be exactly `string` or `number`.
- **The definition owns the target type.** A use site cannot replace enum
  members or a literal value, or change the type. A constraint the resolved
  type does not accept is rejected, as on any property.

A `ConstraintConflict` names the property, the kind of conflict (`Bounds`,
`Scheme`, `Default`, or `Options`), and every declaration involved as a
`SchemaConstraintParticipant`, in merge order (inherited first). Each
participant has two fields:

- `origin: SchemaConstraintOrigin`: where the constraint was declared: its
  source (a file or the inline document), path, keyword, occurrence, and, for
  a file, the byte span of the constraint. In a multi-hop chain the origin
  points at the declaration that introduced the value.
- `constraint: Constraint`: the complete value that took part in the conflict.

With `label: string(min(10))` in `./types.yaml` and
`title: label(max(5))@./types.yaml` in the document, the error reads:

```text
title: contradictory bounds on named type `label`: `min(10)` in `./types.yaml` and `max(5)` in the inline `$schema`
```

The terminal status block lists the same values, one line per participant
(`` `min(10)` at label in ./types.yaml ``), and DMLS relates each declaration
as `` `max(5)` is declared here `` (see
[DMLS Schema Support](../../../../darkmatter/docs/topics/schemas/dmls-schema-support.md#named-type-references)).

Which value a participant holds depends on the kind:

| Kind | Participant values |
|---|---|
| `Bounds` | the surviving (strictest) bound of each side, such as `min(10)` and `max(5)`; equal bounds keep the first origin |
| `Default` | each `default(…)` as declared, keeping its JSON type: `default('3')` and `default(3)` are different values, as are `default('null')` and `default(null)` |
| `Options` | the named type's option set and the use site's, such as `alphanumeric(allow-dashes)` and `alphanumeric(allow-periods)` |
| `Scheme` | the inherited scheme list as it stood just before the failing intersection, and the use site's list |

Two values need care when you read them:

- **Effective schemes.** An inherited scheme list can already have been
  narrowed by earlier, successful merges. With `link: url(scheme(http, https))`
  and the use `narrow: link(scheme(https, ftp); scheme(ftp))`, the first
  intersection leaves `https` and the second is empty. The inherited
  participant is `scheme(https)`, even though its origin still points at the
  `scheme(http, https)` declaration that introduced the group.
  The value is the effective constraint at the failing merge, not the
  authored text, and DMLS words every scheme participant that way.
- **Empty options.** A named type declared as bare `alphanumeric` takes part
  as `Constraint::Options` with an empty set, rendered `alphanumeric`. It
  has no option token, so its origin has no span and points at the type's
  declaration instead.

Messages never format values by hand. Each one is
`Constraint::diagnostic_excerpt()`: the constraint's canonical schema text
(`Constraint::to_schema_text(None)`), with line breaks, tabs, and other
control characters escaped (`\n`, `\t`, `\u{1b}`), shortened to at most 160
Unicode scalar values including a final `…`. The same excerpt appears in
`Display`, the terminal block, and DMLS, always inside a code span, so schema
text cannot become terminal markup. Paths and sources are not part of the
limit, and the structured `constraint` keeps the full value. A list or
mapping default is rendered as compact JSON (`default({"a":1})`), the same
spelling the grammar reads.

When the conflict arises inside a schema the document references,
`SchemaError::ReferencedSchema` wraps it.

The same contradictions on a single atom, with no reuse involved, are errors
too: `string(min(10); max(5))`, `url(scheme(http); scheme(ftp))`, and two
different defaults are each a `SchemaError::Convert` naming the property.

Because a named type can be *any* definition and `@` substitutes it, composition (named types + `@` + `[]` + unions + pattern keys) **is** the type system — SimplifiedSchema's ergonomic answer to JSON Schema `$ref` / `$defs`.

### Recursive Named Types

A named type can describe a structure that nests to any depth — a tree, a
linked list, a grammar whose items hold more items — by referring to itself.
There is no new syntax: the bare name, `Name@this`, and `Name@file` spellings
work in a cycle exactly as they do anywhere else.

```yaml
$schema:
  item: "{ value: string(required), children: item[](required) }"
  tree: item(required)
tree:
  value: root
  children:
    - value: a
      children: []      # an empty list is a finite leaf
```

Every level is checked against the same definition: each `item` needs a
`value` and a `children` list, and `children: []` ends the branch. A missing
`children` or `children: null` four levels down fails at that node. The schema
has no depth setting; a document may nest as deeply as its values do. A tool
that needs a nesting limit enforces it in its own parser.

This describes finite values, not unlimited resources. Existing parser and
conversion limits still apply, and exhausting a traversal limit is an error;
it never makes an unchecked remainder valid. Recursion stores each cyclic body
once; a type outside any cycle follows the rules in
[Shared Named Types](#shared-named-types). Neither promises compact output for
every possible schema.

#### Supported cycles

| Cycle | Example |
|---|---|
| Direct | `item: "{ children: item[] }"` |
| Mutual, in one file | `group: "{ members: member[] }"` and `member: "{ name: string, group: group }"` |
| Across two files | `a.yaml`: `node: "{ name: string(required), next: leaf(required)@./b.yaml }"`; `b.yaml`: `leaf: "{ back: node[](required)@./a.yaml }"` |
| Alias into a cycle | `node: "{ kids: node[] }"`, then `tree: node -> A tree` |

Both files of a two-file cycle are dependency edges of the resolved schema
(`ResolvedSchema.imports`), including the one reached only by the reference
that closes the cycle, so a change to either file is seen by the schema
cache and by DMLS. A
repeated import of the same recursive type shares one definition.

#### The child-value rule

Every cycle must pass through a **child value**, because only then does each
round of the recursion judge a smaller part of the document. Four positions
are child values:

| Position | Example |
|---|---|
| Object property | `item: "{ next: item }"` |
| Dictionary value | `item:` with the YAML mapping `<string>: item` |
| List item | `item: "{ children: item[] }"` |
| Tuple slot | `item: "tuple([string, item?])"` |

Nothing else counts. An alias and a union arm stand for the same value, and a
type inside a `function(...)` signature describes a parameter, not part of the
document. A cycle made only of those would ask the same question about the
same value forever, so it is rejected with `SchemaError::NonDescendingCycle`:

| Rejected schema | Why |
|---|---|
| `a: b` and `b: a` | An alias cycle never reaches a value. |
| `a: [a, string]` | A self-union, rejected even though the `string` arm could accept a value. |
| `f: "function(parameters(x: f); returns(string))"` | A cycle through a callable signature. |

Admission looks at the whole group of types that reach each other: after
setting aside every child-value edge, no cycle may remain. Finding one
child-value edge somewhere in the group is not enough.

```mermaid
flowchart TD
    A["Collect every named-type reference<br/>reachable from the schema"] --> B{"Does the reference sit at a child value?<br/>property, dictionary value, list item, tuple slot"}
    B -->|yes| D[descending edge]
    B -->|"no: alias, union arm, function signature"| N[non-descending edge]
    D --> C{"Is there a cycle made of<br/>non-descending edges only?"}
    N --> C
    C -->|yes| E["SchemaError::NonDescendingCycle<br/>names each reference of that cycle"]
    C -->|"no, but the types form a cycle"| R["stored once in the root $defs;<br/>every use becomes a $ref"]
    C -->|no cycle at all| I["copied into each use, or shared<br/>(see Shared Named Types)"]
```

So a group holding a good cycle and a bad one reports only the bad one.
`a: [b, "{ x: a }"]` with `b: a` is rejected, and the error names the two
references `b → a` and `a[0] → b`; the descending `x: a` is not mentioned.
Change `b` to `"{ y: a }"` and the same group resolves.

The error names every reference of each rejected cycle, with the schema it is
written in and the schema of its target. `md schema validate` prints:

```text
SchemaError: non-descending type cycle
┃ b in the inline $schema refers to a from the inline $schema
┃ a in the inline $schema refers to b from the inline $schema
```

Each entry is a `CycleReference` carrying the reference site (source and
path), the target name and source, and, for a reference written in a schema
file, the byte span of the name, so a cycle between two files has a span in
each. DMLS shows the error as a schema diagnostic on the cycle's first
reference in the open document and lists the others as related locations.
When the cycle sits in a schema the document references,
`SchemaError::ReferencedSchema` wraps it, as for other named-type errors.

#### Definitions and use sites

A recursive type is stored once, as a **definition**, and each property that
uses it is a **use site**. The definition holds the value's shape: its
properties, item types, and its own bounds. Everything about the property
holding the value belongs to the use site:

- **presence:** `required`, `generated`, and `eager`;
- **nullability:** an optional property accepts a missing value or `null`,
  while a list element of the same type does not;
- **annotations:** the description and `default`;
- **extra bounds** such as `min-keys(1)` or, on a list, `min(1)`.

```yaml
$schema:
  item: "{ value: string(required), children: item[](required) }"
  root: item(required)       # must be present and not null
  optional: item             # may be missing or null
  list: item[]               # elements must be items; [null] is rejected
  wide: item(min-keys(1))    # a bound beside the shared definition
```

Markers written on the definition are inherited by every use, exactly as for
an inlined type, so `node: "{ kids: node[] }(required)"` makes every property
typed `node` required. They apply to the property, never to the shared body.
The other inherited rules hold too:

- Two uses with different constraints share one unchanged definition.
- A use-site bound that contradicts the definition's own bound is a
  `SchemaError::ConstraintConflict` naming the definition first, then the use.
- A definition that admits `null` (`link: [null, "{ next: link(required) }"]`)
  accepts a written `null` at every use; a `required` marker, inherited or
  written at the use, still demands the key.
- `[]` on a definition that is already a list is an error, as for any named
  type, and `suggest(...)` at a use site of a recursive type is a
  `SchemaError::Grammar`.

A definition's identity is the schema that declares it plus its name. The
`item` in a document's inline `$schema` and the `item` in `types.yaml` are two
different definitions, and each inline arm of a root union is its own
namespace. A bare name inside a definition still resolves in its defining
file (see [Local Type References](local-type-references.md)).

In JSON Schema, a use lowers to a `$ref` into the schema's root `$defs`, with
the use site's annotations and bounds beside it. With `node: "{ kids: node[]
}"` and `tree: node -> A tree`, the property `root: tree(required)` lowers to
`{ "$ref": "#/$defs/node", "description": "A tree" }`, `root` joins the
root's `required` list, and the `node` body appears once under `$defs`. An
optional use wraps the reference with a `null` arm. Generated `$defs` keys
follow the type name (`item`, then `item-2` for a second, different `item`),
are the same on every run, and never contain a file path.

#### Where recursive types reach

- **Validation** follows a reference at every depth, and an error deep inside
  keeps its JSON path and its line and column in the document:
  `tree/children/0/children/0/value [42] is not of type "string" (at line 10, column 18)`.
- **Phases.** Launch and Completion apply each property's own markers at
  every level below a recursive reference; one definition can serve an eager
  and a deferred property at once. See
  [Universal Constraints](types.md#universal-constraints).
- **Conversion** continues at every depth, up to the conversion engine's
  64-level limit; see [Recursive Types](../conversion.md#recursive-types) under Type Coercion.
- **Merges.** Each schema keeps its own `$defs`: a root-union arm that names
  another schema file, a trigger payload, and a baseline each bring theirs to
  the merged root, and a key two of them share with different bodies is
  renamed in the schema that joins (`item` becomes `item-2`) along with that
  schema's own references, so a reference never binds to another layer's
  type. A key two of them share with an equal body is kept once. Shared
  acyclic definitions (see [Shared Named Types](#shared-named-types)) merge by
  the same rules. A raw JSON Schema baseline or trigger payload stays a simple
  object schema with no root `$ref` or root union; see
  [JSON Schema Baseline Restrictions](../../../../darkmatter/docs/topics/schemas/definition.md#json-schema-baseline-restrictions).
- **Function types** are still not allowed anywhere a document schema can
  reach: a `function(...)` property inside a recursive definition is a
  `SchemaError::Convert` at the use-site path, such as `root.f`.
- **Editors.** Completion and hover descend through a recursive type one level
  at a time, following the cursor's path, and never expand recursive branches
  the cursor is not in. A whole-type display names the back-reference
  (`kids: node[]`) instead of expanding it forever. See
  [Completing inside a union value](../../../../darkmatter/docs/topics/schemas/definition.md#completing-inside-a-union-value) for the
  library API and [DMLS Schema Support](../../../../darkmatter/docs/topics/schemas/dmls-schema-support.md) for the
  editor.

### Shared Named Types

A structured named type that a schema uses in two or more places is stored
once in the generated JSON Schema, and each use points to it, even when the
type never refers back to itself. This keeps a schema that reuses large
objects or unions from growing with every reference. It is automatic: there is
no syntax to request or prevent it, and it never changes which values a schema
accepts, where an error is reported, or what an editor offers. Only the shape
of the JSON Schema changes, which matters if you read `md schema` output or a
resolved schema's JSON directly.

```yaml
# types.yaml
kind: schema
types:
    contact: "{ name: string(required), email: email }"
    slug: alphanumeric(allow-dashes)
```

```yaml
$schema:
    author:   contact@./types.yaml              # stored once, referenced here
    reviewer: contact(required)@./types.yaml    # ... and here
    handle:   slug(min(3))@./types.yaml         # copied: a primitive alias
    nickname: slug@./types.yaml                 # copied
```

`contact` is an object used twice, so its body appears once under the root
`$defs` and both properties refer to it. Each use keeps its own presence,
description, and added bounds beside the reference, exactly as for a
[recursive type](#definitions-and-use-sites): `reviewer` is required, while
`author` is optional and wraps its reference with a `null` arm.
`slug` is a primitive alias, so each use is copied and stays flat and readable
with its own narrowing (`minLength: 3` on `handle` only):

```json
{
  "properties": {
    "author":   { "anyOf": [{ "type": "null" }, { "$ref": "#/$defs/contact" }] },
    "reviewer": { "$ref": "#/$defs/contact" },
    "handle":   { "type": "string", "minLength": 3, "pattern": "…" },
    "nickname": { "anyOf": [{ "type": "null" }, { "type": "string", "pattern": "…" }] }
  },
  "required": ["reviewer"],
  "$defs": { "contact": { "type": "object", "properties": { … }, "required": ["name"] } }
}
```

#### Which types are shared

A named type outside any reference cycle is shared when **both** hold:

1. **Its value is structured.** Arrays and unions are judged by what they
   contain, and an alias by the type it names.

   | Shared | Never shared |
   |---|---|
   | an object, such as `"{ name: string }"` | a primitive or primitive alias: `alphanumeric(allow-dashes)`, `string(min(3))` |
   | a pattern-key dictionary (`<string>: number`) | a list of primitives: `string[]` |
   | a tuple: `tuple([string, number])` | a union of primitives only: `[string, number]`, and a list of one |
   | a list of structured items: `"{ n: string }[]"` | a `function(...)` signature |
   | a union with at least one structured arm: `["{ n: string }", number]` | |

2. **Two or more distinct authored references reach it** (see
   [Counting uses](#counting-uses)).

Some structured types are still never shared, because what they mean depends
on the place that uses them and a reference beside an unchanged body could not
carry it:

- a type whose top level admits `null`, such as `[null, "{ n: string }"]`, or
  carries `operands(...)` or `deferred`;
- a union whose arms carry `required`, `generated`, `default(...)`, or
  `suggest(...)`, admit `null`, include a `file` arm (a list of files is
  fine), or refer to a recursive type.

A `file(match(...))` in the body does not stop sharing. Its glob keeps
suggesting at every use, also in a [root union](types.md#files) arm, where only a
recursive type's globs are enforced.

[Global catalog](../../../../darkmatter/docs/topics/schemas/definition.md#the-built-in-baseline) declarations, the built-in baseline
`fm` among them, follow the same rules: a type a declaration uses at two or
more sites is stored once in that declaration's definition table. The
lifecycle stacks `composed`, `failed`, `cancelled`, and `finalize` of the
built-in baseline all use `md-stack`, so it is stored once and referenced four
times.

#### Counting uses

A use is one reference written in a schema file or inline `$schema`,
identified by where it is written and the type it names:

- **Two places that write the reference count twice**, whether both are
  properties (`author` and `reviewer` above), two references inside one type's
  body, or two aliases of the type.
- **A reference inside another type counts once**, however often that type is
  used. With `team: "{ lead: contact }"` used by three properties, `contact`
  has one use from `team`'s body; `team` itself has three.
- **Unused declarations and callable signatures count for nothing.** A library
  type no reference reaches, and a type named only inside a `function(...)`
  signature, add no uses.
- **A type is its file plus its name.** `entry@./a.yaml` and `entry@./b.yaml`
  are two different types with one use each, so neither is shared. Two
  spellings of one file (`entry@./a.yaml` and `entry@a.yaml`) name the same
  type, so it has two uses and is shared. Each inline arm of a root union is
  its own namespace.
- **Counts belong to one resolution.** The baseline, each trigger payload, and
  the document's `$schema` are resolved separately and never pool their uses.
  Two triggers that each import `contact` once do not share it, even when both
  apply to the same document.

#### When one use is copied instead

Being shared is decided once per type. Each use then keeps the reference only
when the reference plus the use's own additions behaves exactly as a copy
would; otherwise that one use is copied, and the other uses stay references.
The schema stays exactly as valid or as invalid as before: a contradictory
constraint is still a `SchemaError::ConstraintConflict`, never turned into a
copy to make it pass.

A use keeps its reference when it adds only presence and annotation markers
(`required`, `generated`, `eager`, `default(...)`, `example(...)`, a
description) and, on an object, `min-keys`/`max-keys`, or on a list,
`min`/`max`/`unique`. A use is copied when it:

- adds `suggest(...)` or any other value constraint;
- adds `uncounted-keys(...)` that changes how the body's own key bounds count;
- is an arm of a union and either carries a postfix or names a union, a list,
  or a type with presence markers. Copying keeps the enclosing union's
  alternatives one flat list, with markers hoisted as usual (see
  [Named Union References](unions.md#named-union-references));
- names an alias of a shared type with a postfix the reference cannot carry.
  The whole alias chain down to the body is copied.

Key exclusion shows why a reference cannot always stand in for a copy. In a
type library:

```yaml
# types.yaml
kind: schema
types:
    entry: "{ value: string, note: string }(max-keys(1))"
```

```yaml
$schema:
    ordinary:  entry@./types.yaml
    annotated: entry(uncounted-keys(note))@./types.yaml
```

| Value | `ordinary` | `annotated` |
|---|---|---|
| `{ value: v }` | valid | valid |
| `{ value: v, note: n }` | invalid: two keys, above `max-keys(1)` | valid: `note` is not counted |

Copied, `annotated` merges its `uncounted-keys(note)` with the inherited
`max-keys(1)` into one count. A reference cannot: the stored body keeps
counting `note`, and a sibling constraint cannot loosen a bound inside the
definition. So `annotated` is copied and `ordinary` stays a reference. Had the
body already declared `uncounted-keys(note)`, the same use would keep its
reference.

```mermaid
flowchart TD
    T["named type"] --> C{"member of a<br/>reference cycle?"}
    C -- yes --> R["stored: recursive rules"]
    C -- no --> S{"structured value, eligible,<br/>and two or more authored uses?"}
    S -- no --> I["copied into every use"]
    S -- yes --> D["selected for storage"]
    D --> U{"for each use: does a reference<br/>plus this use's additions<br/>behave exactly like a copy?"}
    U -- yes --> REF["$ref to the stored body"]
    U -- no --> CP["this use copied;<br/>the others keep their $ref"]
```

#### What the output looks like

- Shared bodies sit in the root `$defs` beside recursive ones. Keys follow the
  type name (`contact`, then `contact-2` for a different `contact`), are the
  same on every run, and never contain a file path.
- A body is emitted only if some surviving reference reaches it. When every
  use of a selected type is copied, the schema has no `$defs` entry for it.
- Validation, phases, conversion, and editors follow a shared reference as
  they follow a recursive one (see
  [Where recursive types reach](#where-recursive-types-reach)), so problem
  codes, document paths, and line and column positions are those of the copy.
  Only internal JSON Schema keyword paths can move into `$defs`.
- Merged layers keep their own `$defs`: an equal body under the same key is
  kept once, and a different body is renamed with the references that layer
  wrote. Two layers that each store the same imported type therefore end up
  with one body.
- The 32-level import-chain limit applies exactly as for copied types, so
  sharing never makes a rejected chain valid.

### Pattern / Dictionary Keys

Inside an inline object (or a YAML-block schema object), keys may follow a **pattern** instead of a fixed literal set. This types dictionaries — objects whose keys are data, not a known vocabulary.

```yaml
$schema:
    headers: "{ <string>: string }"                 # any string key → string value
    parameter:
        "<string>": unknown                         # any string key → any value
```

Key forms:

| Form                    | Meaning                                              | JSON Schema                          |
|-------------------------|------------------------------------------------------|--------------------------------------|
| `<string>`              | Any string key (catch-all).                          | `additionalProperties: <valueType>`  |
| `<starting::PREFIX>`    | Keys beginning with the **literal** string `PREFIX`. | `patternProperties` (`^PREFIX`)      |
| `<ending::SUFFIX>`      | Keys ending with the **literal** string `SUFFIX`.    | `patternProperties` (`SUFFIX$`)      |
| `<pattern::RE>`         | A raw ECMA-262 regex escape hatch.                   | `patternProperties` (`RE`)           |

Rules:

- **Literal keys win** over pattern keys. Because JSON Schema's `patternProperties` also applies to declared `properties`, the converter subtracts literal names from each emitted non-catch-all pattern with a negative lookahead. For example `<starting::x->` alongside literal key `x-kind` emits a pattern equivalent to `^(?!(?:x-kind)$)x-`. If wrapping would make a user-supplied `<pattern::RE>` invalid, conversion fails loudly rather than silently double-validating.
- Multiple pattern keys are allowed (multiple `patternProperties`).
- A pattern-keyed object is **closed** by default (`additionalProperties: false`) unless a `<string>` catch-all is present, in which case the catch-all lowers to `additionalProperties: <valueType>`.
- A pattern-keyed object whose value type is a `@`-import sidesteps the 32-level inline-object nesting cap — cross-file named types are the depth escape hatch.

Schemas that emit a lookaround-bearing pattern are validated with `jsonschema`'s `fancy-regex` engine **per schema**; every other schema keeps the ReDoS-safe linear engine.

### Object Arity Constraints (`min-keys` / `max-keys`)

Pattern-keyed objects match `0..N` keys; some shapes need a bounded count. `min-keys(n)` / `max-keys(n)` are the object analog of the array `min` / `max`, lowering to JSON Schema `minProperties` / `maxProperties`. They apply only to object atoms — using them on a non-object type or an array-level constraint position is a schema error.

They can be authored as **postfix constraints** or via the reserved **`$constraints`** block key — one canonical model, two surfaces:

```yaml
$schema:
    # Postfix form — exactly one key/value pair
    parameter: "{ <string>: unknown }(min-keys(1); max-keys(1))"

    # $constraints block form — identical desugaring
    parameter:
        "<string>": unknown
        $constraints:
            min-keys: 1
            max-keys: 1
```

`$constraints` (dollar-prefixed, mirroring `$schema`) is **reserved only inside authored schema objects**: it is stripped before shape assembly and never participates in literal/pattern key matching. Flag constraints are written `name: true`. This does **not** reserve `$constraints` in user frontmatter data — only the schema-authoring language uses the sentinel.

A `types` table, and a value typed `schema(mapping)`, is a table of named declarations rather than an object, so `$constraints` written directly in it is a grammar error ("a `types` table holds named declarations; `$constraints` belongs inside a declaration's object"); so is a pattern key such as `"<string>"`. Put the block inside the declaration it constrains: `types: { pair: { "<string>": unknown, $constraints: { max-keys: 1 } } }`.

Key counts judge the value after [defaults are applied](../validation.md#applying-defaults):
a key a default inserts counts like a written one, so adding a default to a
property of a counted object can change whether a value passes. Use
`uncounted-keys` (below) for companion keys that carry a default.

The block and the postfix are one scope. Key bounds from a `$constraints` block, from the object's postfix, and from a use site that reuses the object as a named type merge under the [merge rules](#merging-use-site-constraints): the stricter bound wins, and a minimum above the maximum is an error whichever spelling declared it.

#### Keys the count leaves out (`uncounted-keys`)

`uncounted-keys(a, b)` names keys that `min-keys` and `max-keys` do not count.
Use it when an object holds exactly one entry of one kind with optional
companion keys beside it. A lifecycle action, for example, is one verb, with
`when` and `no_error` allowed beside it:

```yaml
$schema:
    action:
        info: string
        stop: null
        when: string
        no_error: boolean
        $constraints:
            min-keys: 1
            max-keys: 1
            uncounted-keys: [when, no_error]
```

| `action` value | Valid? | Why |
|---|---|---|
| `{ info: hi }` | yes | one counted key |
| `{ info: hi, when: x, no_error: true }` | yes | `when` and `no_error` are not counted |
| `{ when: x }` | no | no counted key: below `min-keys(1)` |
| `{ info: hi, stop: null, when: x }` | no | two counted keys: above `max-keys(1)` |

The postfix spelling is `{ … }(min-keys(1); max-keys(1); uncounted-keys(when, no_error))`.
It needs `min-keys` or `max-keys` on the same object and names one to eight
distinct keys. JSON Schema counts every key, so it lowers to an `allOf` with one
`if`/`then` condition for each combination of the named keys present, each
raising both bounds by the number present, plus an `x-darkmatter-uncounted-keys`
annotation (`{ keys, min, max }`) from which Darkmatter's conversion engine
reads the same count directly.

What else a block may hold depends on where it sits:

| Block on | Accepted keys | Anything else |
|---|---|---|
| an object-typed property | `min-keys`, `max-keys`, `uncounted-keys`, `required`, `not-empty`, `generated`, `closed` (a no-op: the object is already closed) | schema error ("`$constraints` key `…` is not valid on an inline object") |
| the root schema, or one arm of a root union | `min-keys`, `max-keys`, `uncounted-keys`, [`closed`](#closed-roots-closed) | schema error ("… is not valid on the root object") |

On a property's object, `required: true`, `not-empty: true`, and `generated: true` mean exactly the same as the postfix spelling: they apply to the property, so this `author` is required:

```yaml
$schema:
    author:
        name: string
        $constraints:
            required: true
```

### Closed Roots (`closed`)

A root schema is open: a document may carry keys the schema does not
declare. Add `closed: true` to the root's `$constraints` block when every key
must be one the schema knows:

```yaml
$schema:
    name: string
    "<starting::x->": number
    $constraints: { closed: true }
```

| Document | Valid? | Why |
|---|---|---|
| `{ name: Ada }` | yes | a declared property |
| `{ name: Ada, x-count: 3 }` | yes | matches the pattern key |
| `{ name: Ada, extra: 1 }` | no | an unknown key, reported at `extra` |

- **What counts as declared.** The root's literal properties, its pattern-key
  matches, and properties merged in from another
  [layer](../simplified-schema-struct.md#layering). A
  [`<string>` catch-all](#pattern--dictionary-keys) admits every key, so
  closing a root that has one changes nothing.
- **How it fails.** An undeclared key is the same unknown-key problem a nested
  object raises (`dm.schema.unknown_key`, `ValidationProblemCode::UnknownKey`),
  located at the key, under strict and soft validation alike.
- **Root unions.** `closed` goes on one arm, and closes only that arm; an
  open sibling arm still accepts extra keys:

  ```yaml
  $schema:
      - kind: literal(a; required)
        name: string
        $constraints: { closed: true }
      - kind: literal(b; required)
        name: string
  ```

  `{ kind: b, name: Ada, extra: 1 }` is valid; `{ kind: a, name: Ada, extra: 1 }`
  is not, and the problem carries arm index `0`.
- **Nested objects** are always closed, so `closed` there is accepted and
  changes nothing.
- **Flag only.** `closed` must be `true`; `closed: false` is a schema error
  (roots are open without it).
- **JSON Schema.** A closed root lowers to `additionalProperties: false`, and a
  raw JSON Schema root with `additionalProperties: false` is judged the same
  way.

### Content-Format Types (`yaml` / `json`)

`yaml` and `json` type a **string whose content must parse** as a structured document. They join the string-with-format family (`date`, `datetime`, `time`, `email`, `url`, `file`) — the same custom-format seam, not a new mechanism.

```yaml
$schema:
    invocation:
        - string(required)
        - { frontmatter: yaml }   # a frontmatter block expressed as a YAML string
    config: json                  # a string of strict JSON
```

- They compile to `{ "type": "string", "format": "darkmatter-yaml" }` / `"darkmatter-json"`, with validators that parse the value through biscuit-file's YAML / strict-JSON facilities. A parse failure is a validation error.
- **String or native.** The value may be a YAML/JSON **string** *or* a **native** mapping, sequence, number, or boolean, so `frontmatter: yaml` accepts both `"title: Foo"` and a native `{ title: Foo }`. How a native value becomes text, and why text is kept as written, is part of [Type Coercion](../conversion.md#conversion-rules).
- **`json` is strict; `yaml` is a superset.** JSON is valid YAML, so `yaml` accepts JSON; `json` rejects YAML-only syntax.
- Validation-only APIs stay **non-mutating** — they validate against a transient converted copy and leave the caller's frontmatter untouched. Only the composing/write-back path exposes the serialized value.
- Embedded sub-schema constraints (`yaml(schema(<ref>))`) are deferred to a future extension.

## Object Composition (`...name` and `$extends`)

Object composition lets one object reuse the properties of a named object
type, and override the ones it needs to change. Use it when several objects
share a set of properties, so the declarations are written once and cannot
drift apart. There are two spellings: `...name` inside an inline object, and
a `$extends` directive inside a YAML schema mapping.

```yaml
kind: schema
$schema:
    $extends: events
    source: string
    snapshot: "{ ...events, price: number(min(0)) }"
types:
    events: "{ id: string(required), price: number }"
```

The root receives `id` and `price` from `events`. The nested `snapshot`
receives the same required `id` and replaces `price` with its own
non-negative one. Each receiving object validates, lowers, and answers
queries exactly as if its final properties had been written there:

```yaml
kind: schema
$schema:
    id: string(required)
    price: number
    source: string
    snapshot: "{ id: string(required), price: number(min(0)) }"
types:
    events: "{ id: string(required), price: number }"
```

Composition copies **members** (properties and pattern keys). It never
removes an inherited property, and it does not compose arrays, unions, or
document values. The `...` of a `{{ … }}` object literal is a different
feature; see [Object literals and spread](../expression-syntax.md#object-literals-and-spread).

### The two forms

**`...name` in an inline object.** A spread may appear at the start, middle,
or end of an inline object body, any number of times, including an object
made only of spreads, and the usual trailing comma is allowed. It works
wherever an inline object does: a property, an array item type, a union arm,
a nested object.

```yaml
kind: schema
$schema:
    priced:  "{ ...events, price: number(min(0)) }"
    tagged:  "{ tag: string, ...events, }"
    history: "{ ...events, ...audit }[]"
    either:  "{ ...events } | string"
types:
    events: "{ id: string(required), price: number }"
    audit:  "{ changed_by: string, changed_at: datetime }"
```

**`$extends` in a YAML schema mapping.** The value is one named reference or
a nonempty YAML sequence of them, in flow or block style:

```yaml
kind: schema
$schema:
    $extends: events
    order:
        $extends: [events, audit]
        total: number
    refund:
        $extends:
            - events
            - audit
        reason: string
types:
    events: "{ id: string(required), price: number }"
    audit:  "{ changed_by: string, changed_at: datetime }"
```

`$extends` is schema metadata, like `$constraints`, and is accepted in every
mapping the schema grammar reads as an object:

| Position | Example |
| --- | --- |
| the root `$schema` mapping | `$schema: { $extends: events, source: string }` |
| one mapping arm of a root union | `$schema: [{ $extends: events, kind: literal(a; required) }, …]` |
| a mapping arm of a property union | `choice: [string, { $extends: events }]` |
| a mapping-valued `types` entry | `types: { order: { $extends: events, total: number } }` |
| a nested property mapping | `order: { $extends: events, total: number }` |

- **Quoting the key changes nothing.** `"$extends":` and `'$extends':` are
  the same directive.
- **Placement among the keys changes nothing.** `$extends` written last
  still applies before the mapping's own properties (see
  [Order and replacement](#order-and-replacement)).
- **Wrong values are grammar errors at the value or item.** `null`, an empty
  value, a number, a boolean, a mapping, `[]`, and a non-string list item
  are rejected. A comma inside one string (`$extends: "a, b"`) is rejected
  with the hint to write a YAML sequence, `[a, b]`.
- **The `types` table itself is not an object.** `$extends` placed directly
  under `types:` is the name of a type, not a directive, whatever its
  definition is: `types: { $extends: string }` declares a string type. The
  envelope check of a schema file reads the table the same way. It is a poor
  type name all the same: `$` is not an identifier character, so no
  reference can name it. A mapping entry of the table is an object again, so
  `types: { order: { $extends: base } }` composes. Unlike `$extends`,
  `$constraints` and pattern keys directly under `types:` are refused (see
  [Object Arity Constraints](#object-arity-constraints-min-keys--max-keys)).

### Which names an operand can use

An operand is a named-type reference, looked up by the same rules as any
other reference (see [Local Type References](local-type-references.md)):

```yaml
kind: schema
$schema:
    a: "{ ...events }"                     # bare local name
    b: "{ ...events@this }"                # the same, spelled explicitly
    c: "{ ...contact@./people.yaml }"      # a type from another file
    d: "{ ...string@this }"                # the local type named `string`
    e: "{ ...later }"                      # forward references work
    f: "{ ...alias }"                      # aliases are followed to the object
types:
    events: "{ id: string(required) }"
    string: "{ s: number }"
    alias:  events(required) -> an alias of events
    later:  "{ x: string }"
```

- **Built-in names need `@this`.** `...string` names the built-in `string`,
  which has no members, and is a grammar error that suggests
  `...string@this`. The same holds for `$extends: string`.
- **Aliases are followed, their qualifiers are not.** `...alias` copies the
  members of `events`; the alias's own `required` and description stay with
  the alias.
- **Every import form works:** a file next to the schema, a magic path, an
  embedded document in a host's schema bundle, and local names a host
  supplies beside the schema. An imported operand records its file, and the
  files that file imports, as dependencies of the resolved schema, exactly
  as `Name@file` does. A schema built from text alone
  (`SimplifiedSchema::try_from(&str)`) reads no file, so an operand that
  names one fails with `ResolutionUnavailable`.
- **An empty object qualifies.** `empty: "{}"` is a valid target that
  contributes nothing.

**Copied keys never become type names.** The local names of a schema are the
ones its author wrote. When a schema has no `types` table, its top-level
`$schema` entries are its local names; a property brought in by `$extends`
is a property of the root but not a new name:

```yaml
$schema:
    $extends: base
    base: "{ helper: { q: string } }"
    x: "{ ...helper }"     # error: unknown named type `helper`
```

For the same reason `select("helper")` cannot find it. Each inline arm of a
root union keeps its own local names, so the same operand can mean a
different type in each arm:

```yaml
$schema:
    - $extends: shape
      shape: "{ a: string }"     # this arm receives `a`
      kind: literal(a; required)
    - $extends: shape
      shape: "{ b: number }"     # this arm receives `b`
      kind: literal(b; required)
```

**A copied definition keeps its own scope.** References inside a copied
property resolve in the file (or root-union arm) that declared it, never in
the receiver:

```yaml
# lib.yaml
kind: schema
types:
    child: "{ a: string }"
    base: "{ c: child }"
```

```yaml
# main.yaml
kind: schema
$schema:
    $extends: base@./lib.yaml
    mine: child                  # this file's `child`
types:
    child: "{ z: number }"
```

`c` is `lib.yaml`'s `child` (`{ a: string }`), while `mine` is
`main.yaml`'s.

### Order and replacement

Entries apply **left to right**, and for each key the **last declaration
wins**. A spread applies where it is written; every `$extends` operand
applies, in list order, before the mapping's own properties.

| Object | `id` is |
| --- | --- |
| `{ id: number, ...events }` | `string(required)`, from `events` |
| `{ ...events, id: number }` | `number`, written here |
| `{ ...events, ...other }` | `other`'s `id`, when it has one |
| `$extends: events` plus `id: number`, in any key order | `number` |

For properties that both spellings can express, `$extends: [a, b]` with
`total: number` means the same as `"{ ...a, ...b, total: number }"`.

- **A property is replaced as a whole.** Its type, constraints, `required`,
  `default(...)`, description, and examples all come from the winning
  declaration. Overriding `id: string(required; default('x')) -> the id`
  with `id: number` gives an optional `id` with no default and no
  description.
- **A replaced key keeps its first position.** In
  `{ price: string, ...events }` the winning `price` from `events` stays
  first, so output order is deterministic.
- **Two written properties with one name are still an error,** even with a
  spread between them: `{ id: string, ...events, id: number }` is a
  `SchemaError::Grammar` at the second `id`.
- **Every operand must resolve,** even one whose members are all replaced
  later. `{ ...nope, a: number }` fails on `nope`.

### What an operand cannot carry

An operand names an object whose members are copied, so it takes nothing a
property reference can add. Each of these is a grammar error at the
offending text:

| Written | Why it fails |
| --- | --- |
| `...events(required)`, `$extends: events(required)` | no constraint list; constrain the receiving object |
| `...events[]` | no `[]` |
| `...events -> a description` | no description; describe the receiving property |
| `...{{ shape }}`, `...events{{ x }}` | no template |
| `...`, `...@./a.yaml`, `...a@` | no name, or no path after `@` |

A postfix **after the object** still belongs to the containing property:

```yaml
$schema:
    rows:    "{ ...events }[](min(1))"     # a non-empty list of events
    current: "{ ...events }(required) -> the current event"
```

The file path after `@` is read with the usual
[reference delimiters](#cross-file-named-type-imports-namefile): up to the
next `,`, `}`, or `->` (and `)` inside a group), so
`...a@./dir (x)/a[1].yaml` names that file. In a YAML flow sequence, quote
an item whose path holds `[` or `{`: `$extends: ['a@./x{1}.yaml']`.

### Origins: examples, defaults, and templates

A copied property keeps the origin of the file that declared it:

- A relative `example(./ex.yaml)` resolves from the declaring file's
  folder, and an example on a replaced declaration is never read.
- A default the copied property inserts is attributed to the declaring file
  (`ResolvedSchema::default_files`).
- A `{{ … }}` template inside a copied member evaluates in its declaring
  file.
- If the receiver replaces an imported property, its own definition and
  origin replace the imported ones together.

A schema error inside a copied member, such as a `function(...)` property
that cannot be lowered, is `SchemaError::ComposedMember`, which names the
member's declaration. A validation problem in a supplied value still points
at that value in the document.

### Constraints and pattern keys

Only the target's members are copied. Its own object-level constraints stay
with it: `$constraints`, a postfix on the object, its description, and a
`required` or `default` on the object itself. The receiver's own
constraints, including `closed` and key counts, apply to the **merged**
members:

```yaml
kind: schema
$schema:
    copy: "{ ...pair }"          # accepts { a, b }: max-keys(1) is not copied
    use: pair                    # rejects { a, b }: an ordinary use keeps it
    bounded:
        $extends: pair
        c: string
        $constraints: { max-keys: 2 }   # judges a, b, and c together
types:
    pair:
        a: string
        b: string
        $constraints: { max-keys: 1 }
```

[Pattern keys](#pattern--dictionary-keys) are copied too:

- **A later entry for the same pattern replaces it.** Two patterns are the
  same when they lower to the same JSON Schema key for the receiving object,
  so `<starting::a>` and `<pattern::^a>` are one pattern. The `<string>`
  catch-all is one identity of its own.
- **Distinct patterns both apply,** even when they overlap: with
  `<starting::a>: number` and `<ending::z>: string`, the key `az` must
  satisfy both.
- **Literal keys keep precedence** over every pattern. Patterns are lowered
  against the final set of literal keys, so a newly added literal is
  excluded from the patterns exactly as in a handwritten object.
- **An inherited pattern can close a root.** A root that receives a pattern
  key and no catch-all is closed by the usual pattern rule, and an inherited
  `<string>` catch-all types every extra key.

### Eligible targets and errors

A target must be **one object with a declared shape**, reached directly or
through aliases. Anything else fails at the operand:

| Case | Example | Error |
| --- | --- | --- |
| Primitive | `...label` with `label: string` | `InvalidCompositionTarget`, "it is a primitive type, not an object" |
| Opaque object | `...meta` with `meta: object` | `InvalidCompositionTarget`, "it is `object`, which declares no members" |
| Union, even of objects | `...shape` with `shape: ["{ a: string }", "{ b: number }"]` | `InvalidCompositionTarget`, "it is a union, not a single object" |
| List or tuple | `...rows` with `rows: "{ a: string }[]"` | `InvalidCompositionTarget`, "it is a list, not an object" |
| Function signature | `...greet` with `greet: "function(…)"` | `InvalidCompositionTarget`, "it is a function signature, not an object" |
| Shape supplied by a template | `...dynamic` with `dynamic: "{{ shape_of() }}"` | `InvalidCompositionTarget`, "it is supplied by a template, so its members are unknown" |
| Unknown name | `...missing` | `UnknownNamedType` at the operand |
| File that cannot be loaded | `...base@./absent.yaml` | `CompositionReference`, holding the original cause (such as `NoMatch`) |
| Chain deeper than 32 | `t1` spreads `t2`, … `t33` | `CompositionReference`, holding an `ImportCycle` |
| Loop with no child value | `a: "{ ...b }"`, `b: "{ ...a }"` | `NonDescendingCycle`, listing every hop, aliases included |

`md schema validate` prints, for example:

```text
$schema.order[...0]: `label` cannot be composed into an object: it is a primitive type, not an object
```

`InvalidCompositionTarget` names the operand as written, its site, the
reason (`CompositionTargetKind`), and the declaration that decided it.
Through an alias that is the type the alias reaches. A declared object with
a template inside one member is a valid target; only a shape supplied
entirely by a template is not. The [diagnostics guide](../diagnostics.md#where-a-definition-failure-points)
describes where each error points.

### Recursion through composition

A composition reference describes the object at the receiving position: it
does not descend into a child value. Composition and recursion therefore
mix under the usual [child-value rule](#the-child-value-rule): every loop
must pass through a child value at least once.

```yaml
kind: schema
$schema:
    tree: branch
types:
    base:   "{ children: branch[] }"
    branch: "{ ...base, label: string }"
```

`branch` receives `children` from `base`, and its children are `branch`es.
Resolving `branch` needs only the member list of `base`, with `children`
kept as a reference to `branch`; checking a document then reaches `branch`
again only on a smaller value. This document is valid; had `c` been written
as a list instead of text, the problem would be reported at
`tree/children/0/children/0/label`:

```yaml
tree:
    label: a
    children:
        - label: b
          children:
              - label: c
```

The same cycle split across two files resolves too. A recursive target can
also be composed from outside its cycle: with `node: "{ children: node[] }"`,
`labeled: "{ ...node, label: string }"` adds a `label` to the receiving
object, while its `children` are still plain `node`s.

A loop made **only** of composition and alias hops never reaches a value,
so it is a `SchemaError::NonDescendingCycle`:

```yaml
kind: schema
$schema:
    value: a
types:
    a: "{ ...b }"
    b: "{ ...a }"
```

```text
named types form a cycle that never descends into a child value: types.b[...0] -> `a`, types.a[...0] -> `b`
```

Composition and alias hops count toward the same 32-level limit as other
named-type references.

### How a composed object is assembled

```mermaid
flowchart TD
    R["receiving object"] --> E["entries in written order:<br/>spreads in place, $extends operands first"]
    E --> T{"operand: follow aliases<br/>to a declared object?"}
    T -->|no| X["InvalidCompositionTarget,<br/>UnknownNamedType, or CompositionReference"]
    T -->|"loops back with no child value"| C["NonDescendingCycle"]
    T -->|yes| M["the target's own assembled members<br/>(child references not expanded)"]
    M --> W["winner per literal key and per emitted pattern key:<br/>last declaration wins, first position kept"]
    E --> W
    W --> D["each winner resolves its references<br/>and origins in its declaring file"]
    D --> O["one object: validation, queries,<br/>and JSON Schema see the merged members"]
```

### Output: JSON Schema and YAML export

**JSON Schema.** A composed object lowers to one object shape with the
merged `properties`, `required`, and pattern keys, never an `allOf` of the
parts (an `allOf` of closed objects would reject the properties each part
adds). Recursive and shared named types still use `$defs` and `$ref` as
usual. Subtyping (`is_subtype`) gives the same answer for a composed object
and its handwritten equivalent, in both directions. Lowering a shape whose
operands were never resolved fails with `SchemaError::Convert`.

**YAML export.** `to_simplified_yaml()` writes composition **as authored**,
never as the merged members, so reparsing the export with the same
file-resolution context gives the same composition:

- `$extends` keeps its scalar or sequence form and is written **first** in
  its mapping, whatever its authored placement.
- An inline object with spreads stays one type-expression string, with each
  spread at its position, written compactly (`'{...base,label: string(required)}'`).
  Properties written between two spreads come out literals first.
- A property-union arm that uses `$extends` is written as a mapping, so the
  directive survives; other arms keep their string form.
- **A selected named type** (`schema.select("branch")`) whose definition uses
  spreads exports as a root that extends it, beside the authored `types`
  table:

  ```yaml
  $schema:
      $extends: branch@this
  types:
      base: '{ id: string(required), children: branch[] }'
      branch: '{...base,label: string(required)}'
  ```

  A selected name that cannot be written as an operand (such as `type.v2`)
  is `ExportUnsupported`. Selecting from a root that has no `types` table and
  uses `$extends` drops the root's directive from the copied table, because
  `$extends` under `types:` would read back as a type name.
- **Layered schemas** with a composing root in any layer, or a lower layer's
  property that composes, cannot be flattened into one document, so their
  export is `ExportUnsupported`. They still validate together; only the
  single authored YAML export is unavailable. A nested composition in the
  top layer is exported.

### Migrating a literal `$extends` property

`$extends` is now reserved inside authored schema mappings. A schema that
used it as an ordinary property name changes meaning:

- A value that is not a valid operand, such as `$extends: string`, is a
  grammar error that explains the reservation:

  ```text
  `$extends` is reserved for object composition and needs a named object type, but `string` is the built-in `string` type; write `string@this` to compose a local type named `string`, or use raw JSON Schema to declare a literal `$extends` property
  ```

- A value that happens to be a valid operand, such as `$extends: base`, now
  composes `base`. Check existing schemas for a `$extends` key before
  upgrading.
- To declare a property literally named `$extends`, use a raw JSON Schema
  file; SimplifiedSchema cannot.

Only schema mappings are affected. A document's **data** may still hold a
`$extends` key wherever its schema admits it, for example under an open
root:

```yaml
---
$schema:
    "$extends": base
    base: "{ id: string }"
$extends: kept as ordinary data
id: a
base: {}
---
```

### In an editor

DMLS completes operands after `...`, in a `$extends` value, and in each
`$extends` list item, and offers hover and go-to-definition on them. A failing operand is
underlined as a whole, with the target's declaration as related
information. See [Diagnostics](../diagnostics.md#composition-in-the-editor).
