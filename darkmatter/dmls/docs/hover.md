---
kind: canonical-documentation
---

# DMLS Hover Effects

Hover (`textDocument/hover`) is how DMLS explains the symbol under the cursor without navigating away from it — a schema property's type and description, a `ctx.*` variable, a directive's semantics, a wiki link's target, and so on. This page describes how hover content is produced, the hard limits on how it can look, and the formatting rules DMLS applies within those limits.

## The rendering contract

Every DMLS hover is an LSP `MarkupContent` with `kind = Markdown`. We hand the editor a **Markdown string**; the **editor** renders it using **its own theme**.

That split is the whole story of "how much control we have":

- We choose the **Markdown constructs**.
- The editor decides how each construct *looks* — font, weight, and especially   **color**.

We cannot set colors, backgrounds, opacity, or CSS, and editors strip inline HTML (`<span style="color:…">` and friends) from hover content. So there is **no way to make hover text a specific color, or to "dim" it** — those are theme decisions, identical across VS Code, Zed, Neovim, and Helix. This is an LSP/editor limitation, not a DMLS one.

What a construct becomes when the editor renders it:

| We emit                                         | Rendered as                                                |
|-------------------------------------------------|------------------------------------------------------------|
| `` `name` `` (inline code)                          | a boxed / inverted background (theme-colored)          |
| `**name**` (bold)                               | bold weight, no box                                        |
| `_name_` (italic)                               | italic                                                     |
| `# name` / `## name` (heading)                  | larger / heavier heading                                   |
| lists, block quotes, links, tables, fenced code | supported (fenced code is syntax-highlighted by the theme) |
| raw HTML, hex colors, ANSI codes                | **ignored / sanitized**                                    |

The only places color ever appears are constructs the *theme* colors — inline code, links (the accent color, but underlined and clickable), and syntax-highlighted fenced blocks. None of these is a reliable lever for "make this token blue," so DMLS does not try; it expresses hierarchy with weight and style instead.

## Formatting rule: frontmatter schema hover

Hovering a schema-declared frontmatter property renders its type, whether it is required, its enum values, its default, and its `->` description. The style rule is chosen to convey hierarchy using only the constructs above:

| Element                      | Markup                                  | Rationale                                       |
|------------------------------|-----------------------------------------|-------------------------------------------------|
| the property being described | `` `area` `` (inline-code box)              | the box marks *the subject* of the hover        |
| its type                     | `Type: **string**` (bold)               | emphasized, but distinct from the boxed subject |
| enum values                  | `Values: _draft_, _published_` (italic) | softer metadata                                 |
| default value                | `Default: _draft_` (italic)             | softer metadata                                 |
| required flag                | `Required` (plain)                      | a plain word needs no emphasis                  |
| description                  | plain paragraph                         | the prose the author wrote                      |
| `alphanumeric` character rules | `Options: …`, `Characters: …`, `First character: …`, `` `..`: … `` | the canonical options and the character sets they allow |

The mental model: **box = the thing being described; bold = its type; italic = its example/enum/default values.** (An earlier idea — blue type, dim default — was dropped because color and dim are not expressible in hover Markdown.)

The body is assembled by the pure `schema_hover_body` function in `src/providers/frontmatter.rs`, kept free of any LSP session state so the rule is unit-testable in isolation.

### Schema definitions

Hovering a definition in a schema (its key or its type text) shows what it
declares, then the catalog description of each built-in type it uses. A type
with an alias names it, so `whatever: any` hovers as `unknown`:

```markdown
**`whatever`**

Type: **type-definition**

Declares: **unknown**

**`unknown`** (alias `any`): Every value, including null, or omission; ...
```

An `alphanumeric` type adds its canonical options (redundant ones dropped)
and the effective character sets, as the library computes and renders them.
`alphanumeric(allow-dashes, block-leading-dash, block-double-periods)`
shows:

```markdown
Options: `allow-dashes`, `block-leading-dash`

Characters: `A-Z a-z 0-9 -`

First character: `A-Z a-z 0-9`

`..`: forbidden
```

A local type named like a built-in, referenced as `shell@this`, hovers and
goes to its local definition; the bare `shell` is the built-in.

### Union-typed properties and list items

A property typed by a reference to a union-typed named type with a postfix
(`policy[](min(1))@./content-policy.yaml`, `choice(required)`) has no single
type, so its hover lists every alternative, one line each, with the
alternative's own description:

```markdown
Type: **array**, each item one of:
- **string** — Never triggers
- **object** with `rule`, `action`
```

Hovering one item of such an array names an alternative only when the item
is valid against exactly that one. A missing (`null`) item, an item no
alternative accepts, and an item several alternatives accept all show the
whole list, with the reason first:

| Item                            | Hover shows                                           |
|---------------------------------|-------------------------------------------------------|
| `- ValidFor(3mo)`               | `Matches alternative 3 of 9`, then that alternative   |
| `- Duration(3mo)`               | `matches none of the 9 alternatives:` and the list    |
| `- null`                        | `no value; one of 9 alternatives:` and the list       |

On a field inside an object item, the field's own hover is followed by a
line naming the item's alternative (``Item `content_policy[1]` matches
alternative 9 of 9:``) or, when no single alternative accepts the item, the
list. Whether an alternative accepts an item is decided by the Darkmatter
library: the alternative is lowered to JSON Schema and checked with its
structural validator, which judges a file value by syntax alone.

A field that only some object forms of a value declare still hovers. In a
Claudine lifecycle item, `ensure_file`'s long form is one of three overloads,
and only the later two declare `content`; hovering `content` in
`- ensure_file: { file: notes.md, content: "# Notes" }` describes it with the
same item line as `file`.

### Recursive types

A property typed by a recursive named type is described by name, followed
by the definition once; inside it, each recursive use is again written by
name, so the hover is finite however deep the type goes:

```markdown
Type: **item[](required)**

`item` is defined as: `{children: item[](required),value: string(required) -> The node text}`
```

A property below the recursion (`value` four levels down) hovers exactly as
it does at the top level.

The same named form describes a property typed by a
[shared named type](../../../simplified-schema/docs/topics/language/objects-and-named-types.md#shared-named-types),
an acyclic object, dictionary, tuple, or list that the schema stores once
because two or more places use it: `` `entry` is defined as: … ``. A stored
**union**, recursive or shared, is the exception. It hovers exactly as the
inlined union would, listing its alternatives as described under
[Union-typed properties and list items](#union-typed-properties-and-list-items),
so the item line ("matches alternative N of M") numbers the same list
whether the union was stored or copied.

## Value versus key hover

In frontmatter, the key and the value of the same line answer different
questions. The **key** explains the property: the schema hover above. The
**value** of a `$schema` string, or of a property whose type includes
`file`, is a file reference, so it explains where that reference goes:

```yaml
$schema: review.yaml   # key: what `$schema` is · value: the file review.yaml selects
source: ./draft.md     # key: the `source` property · value: where ./draft.md resolves
```

Each hover's range is what it describes: the key's span for a key hover, the
authored value's span for a value hover, in the position encoding the
client negotiated. Inside the frontmatter, the first of these that applies
answers:

```mermaid
flowchart TD
    C[cursor in frontmatter] --> S{"on a $schema string?<br/>(the scalar, or a string item of a root union)"}
    S -- yes --> SV["$schema value hover"]
    S -- no --> M{"on a named-type reference, or inside<br/>an inline $schema mapping or arm?"}
    M -- yes --> MS[schema-authoring hover]
    M -- no --> E{"on an expression-typed value, or a<br/>{{ … }} / when inside a lifecycle stack?"}
    E -- yes --> EX[expression hover]
    E -- no --> F{"on the static string value of a<br/>file-typed property?"}
    F -- yes --> FV[file value hover]
    F -- no --> K["key hover: the property's schema hover<br/>(or the ctx.* annotation)"]
```

So an inline arm such as `{status: string}` in `$schema: [shape.yaml,
{status: string}]` keeps its schema-authoring hover, while `shape.yaml` in
the same list gets the value hover.

## Formatting rule: `$schema` value hover

Hovering the value of `$schema` shows the file the reference selects and
what it holds, or why it selects nothing. It is the same file the document
link, go-to-definition, and validation use; the lookup rules are in
[`$schema` references](./file-references.md#schema-references). Each string
item of a root union (`$schema: [a.yaml, b.yaml]`) hovers on its own, so one
missing item does not change the hover of another. A value that is not a
string (a mapping, `null`, a number) is not a reference and gets no value
hover.

### A valid schema file

With `repo/schemas/review.yaml`:

```yaml
kind: schema
description: A review of one feature.
$schema:
  # Who wrote it
  author: string(required)
  status: enum(draft, done)
```

hovering `review.yaml` in `$schema: review.yaml` shows:

````markdown
A review of one feature.

Schema Definition:

```yaml
# Who wrote it
author: string(required)
status: enum(draft, done)
```

Resolves to [schemas/review.yaml](file:///work/repo/schemas/review.yaml)
````

- **Description first.** A nonblank top-level string `description` in the
  target file leads the hover, trimmed and rendered as Markdown. Only the
  property counts: a leading YAML comment block is never used, and a
  missing, blank, or non-string `description` is simply left out.
- **The definition, not the file.** For a YAML target the block shows only
  the target's own `$schema` value as you wrote it: comments and line order
  kept, the key and the common indentation removed. Other top-level
  properties (`kind`, `description`, `types`) are not shown. A JSON target's
  whole document is the definition, shown in a `json` block.
- **At most 10 lines.** A longer definition shows its first ten lines, then
  `*Showing **10** of **{total}** lines*` after the block, where `{total}`
  counts every line of the definition (a final newline is not a line, and
  `\r\n` is shown as `\n`). The fence is longer than any run of backticks in
  the excerpt, so a definition containing a fence still renders.
- **Where it resolves.** `Resolves to` links the file. The label is
  relative to the document's repository when the file is inside it,
  otherwise an absolute path with `/` separators; labels and quoted error
  text are escaped, so a folder named `[x] (y)` reads literally.
- **Delegation is not expanded.** A target whose `$schema` is itself a
  reference, such as `alias.yaml` holding `$schema: ./real.yaml`, previews
  `./real.yaml` and links `alias.yaml`, the file the document named. Imports
  are not followed and no generated JSON Schema is shown.

The preview is built when DMLS prepares the document's schema, so a hover
reads no file and never composes. It is refreshed with the selection (see
[When a `$schema` selection is redone](./file-references.md#when-a-schema-selection-is-redone)).

### A file that is not a usable schema

The file was found, so the hover links it and says why it cannot be used.
It never calls an existing file missing.

| The selected file | Hover |
|---|---|
| Exists but is not an accepted schema format: a `types`-only library, YAML with no `$schema`, JSON without a dialect, a JSON array | ``The file [`schemas/types-only.yaml`](…) exists but it is **not** a valid schema definition!`` |
| JSON that declares a dialect other than Draft 2020-12 | ``The file [`schemas/draft7.json`](…) declares the unsupported JSON Schema dialect `http://json-schema.org/draft-07/schema#`. Only `https://json-schema.org/draft/2020-12/schema` is supported.`` |
| Malformed YAML or JSON, an invalid schema grammar, a missing or invalid import, a cycle, or a file that cannot be read | ``The file [`schemas/malformed.yaml`](…) could not be used as a schema: {reason}`` |

Which YAML and JSON files a `$schema` reference may point at is described
in [What a `$schema` Reference May Point At](../../docs/topics/schemas/definition.md#what-a-schema-reference-may-point-at).
The verdict is for this reference alone: in a root union, another item
failing does not mark this one unusable.

An unreadable file is reported this way when it is outside the schema
roots, such as `./locked.yaml`. An unreadable file inside a schema root
stops the schema-root scan itself, so its hover is the "schema roots could
not be read" one below, with no link.

### A reference that selects no file

A clean miss lists every location searched, in order, then the path the
document link opens, explicitly labeled as missing:

```markdown
`gone.yaml` does not match a schema file.

Searched, in order:

- `schemas/gone.yaml`
- `/home/ana/schemas/gone.yaml`

Missing: `schemas/gone.yaml` does not exist yet; following the link opens this path.
```

Only roots that exist are searched, and the creation path is in the first
of them. An explicit miss (`./gone.yaml`) reads the same, with the one
location beside the document. Neither the hover nor the link creates a file.

The first line changes with the kind of miss:

| Case | First line |
|---|---|
| Bare name or explicit path, not found | `` `gone.yaml` does not match a schema file. `` |
| Bare name with no schema root folder at all, so the ordinary lookup (document folder, then repository root) was used | `` `gone.yaml` was looked up like any other file reference (no schema root folder exists). `` |
| Bare name found only beside the document | `` `beside.yaml` is not in any schema root, but a file with that name is beside this document. Write `./beside.yaml` to reference it. `` |

For the sibling case the `Missing:` path is still in the first schema root:
the hover suggests the `./` spelling and never links the sibling as if the
bare name had resolved.

### A reference that cannot be resolved

These get one sentence and no link, because there is no honest path to
offer:

| Case | Hover |
|---|---|
| A remote URL | `` `https://example.com/s.yaml` is a remote reference. Remote schemas are not fetched. `` |
| A malformed reference, a missing anchor (`&x.yaml` outside a repository), or a location that could not be checked | `` `&x.yaml` could not be resolved: {reason} `` |
| A schema root that could not be read | `` `policy.yaml` could not be resolved because the schema roots could not be read: {reason} `` |
| A document without a file-resolution context | `` This document has no file-resolution context, so `shape.yaml` is not resolved. `` |

## Formatting rule: `file` value hover

Hovering the value of a property whose type includes `file` says where the
reference resolves, or why it does not:

```yaml
$schema:
  source: file(eager)
  notes: file
  pages: file(eager)[]
source: ./present.md     # Resolves to [`docs/present.md`](file:///work/repo/docs/present.md)
notes: ./later.md        # missing, not eager: not required even at run time
pages:
  - ./gone.md            # missing, eager: must exist at run time
```

| Value | Hover |
|---|---|
| Not a valid file reference | ``` `@` is not a valid file reference: {reason} ``` (also a `dm.schema.invalid_file_reference` diagnostic) |
| The file exists | ``Resolves to [`docs/present.md`](…)``, with the label rules of the `$schema` hover |
| Missing, property not `eager` | "This file reference is a valid file reference but the file does not exist yet in the filesystem. Because `notes` is not defined as being **eager** then the file reference is not required to exist even at run time." |
| Missing, property `eager` | "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `pages` is defined as being an **eager** file reference so that means the file reference must refer to a real file at runtime." |
| Missing, a union where an `eager` `file` arm is one of several possible types | "This file reference is a valid file reference but the file does not exist yet in the filesystem. The `either` has an **eager** `file` arm, so when this value is read as that file reference it must refer to a real file at runtime." |
| Needs an anchor the document lacks (`&x.md` outside a repository, an undefined `{{DIR}}`) | "This file reference needs context this document does not have (a home folder, repository, or package), so it cannot be resolved." |
| The filesystem could not be checked | "The filesystem could not be checked for this file reference (a permission or I/O failure)." |
| A URL | "This is a remote URL. It is a well-formed file reference, but the editor never fetches remote content and has not checked whether it exists." |

A missing file is never a diagnostic in the editor, `eager` or not; the
hover is where it is explained (see
[Missing `file` values are explained, not flagged](./file-references.md#missing-file-values-are-explained-not-flagged)).
None of these hovers offers a creation path.

Which values get the hover:

- **The selected arm decides.** A union arm that the value's other fields
  rule out does not make it a file: in
  `entry: "{ kind: literal(doc), path: file(eager) } | { kind: literal(text), path: string }"`,
  `path` under `kind: text` is a string and keeps its property hover. When
  several arms remain possible, any surviving `file` arm counts.
- **Any depth.** Nested mapping properties (`nested.main`) and scalar array
  items count. The hover names the property by its dotted path; an array
  item names its array (`pages`).
- **Static strings only.** A Darkmatter `{{ title }}.md` expression or a
  `$(echo x.md)` command is computed at compose time, so it keeps its
  property or expression hover. A biscuit-file environment reference such
  as `{{DIR}}/x.md` is a static reference and is hovered.
- **Not references:** `null`, numbers, inline mappings, sequence markers,
  and comments.
- **No context, no file hover.** A document without a file-resolution
  context assembles no schema, so none of its values is `file`-typed; its
  one `dm.context.build_failure` diagnostic explains why.

Resolution follows the ordinary `file` rules: an implicit path from the
document's folder, then the repository root; `./` and `../` from the
document's folder. The hover probes the local filesystem through the
document's context, like navigation, and never fetches a URL.

## Formatting rule: interpolation `ctx.*` hover

Inside a `{{ … }}` body interpolation, a cursor on an explicitly `ctx.`-qualified variable renders the shared catalog-backed block followed by an interpolation-only note. The variable counts as the root through member access and index access too (for example `ctx.packages[0].first`), because the lexer keeps a dotted `ctx.packages` as a single token.

The catalog-backed block is single-sourced through one adapter, `overlay::expressions::format_ctx_hover_block`, which the frontmatter `ctx.*` hover also renders — so the two surfaces produce the same bytes. The block carries the qualified name, the rendered type, the read-only / Darkmatter-owned ownership note, and the description:

| Element      | Markup                                         | Rationale                                        |
|--------------|------------------------------------------------|--------------------------------------------------|
| the variable | `` `ctx.<name>` ``(inline-code box) + `(type)` | the box marks the subject; the type rides inline |
| ownership    | `read-only, Darkmatter-owned` (plain)          | a passive reminder, not a captured value         |
| description  | plain paragraph                                | the prose the catalog author wrote               |

After the block, the interpolation surface appends one interpolation-only line:

> The `ctx` variable is evaluated at _compose_ time (rather than now).

The frontmatter surface does not emit that note — it is DMLS-owned because it describes passive editor behavior (DMLS never captures host context or evaluates the expression to build hover content).

Classification requires an explicit `ctx.` prefix (the D2 rule): a bare `{{ today }}` is a **frontmatter** variable even when `today` is also a known context-variable tail, and an unknown `ctx.<name>` keeps the generic expression hover without borrowing a similarly named bare key's value. The hover range remains the complete `{{ … }}` expression.

## Formatting rule: reserved-root and `fm.<key>` hover

A cursor on a bare reserved root (`fm`, `ctx`, `env`, `current`, and any other
name in Darkmatter's reserved-root catalog) appends one line, `` **`<root>`** — <description> ``,
with the catalog description. A reserved root shadows a frontmatter key of the
same name, so this line wins over any static value.

A cursor on `fm.<key>` hovers the frontmatter key `<key>` exactly as a bare
`{{ <key> }}` would: its static value, or the schema block for a declared but
unset property. `doc` and `hook` are ordinary properties: `{{ doc.title }}`
shows the static value of the nested key `doc.title`, never a root
description.

## Formatting rule: function-call hover

A cursor on a known function-name identifier — the name token of a `FunctionCall` such as `as_csv(...)` — renders the catalog typed signature and description through `overlay::expressions::format_function_block`.

| Element      | Markup                                     | Rationale                          |
|--------------|--------------------------------------------|------------------------------------|
| the function | `` `<typed signature>` ``(inline-code box) | the box marks the subject          |
| description  | plain paragraph                            | the prose the catalog author wrote |

Cursor precedence: the function name wins on its own name identifier; an offset on an argument, parenthesis, or comma falls through to that inner expression's `ctx.*` or frontmatter hover (`function_call_at` declines on a non-name offset, so `expression_at` serves the argument). Unknown functions keep the generic `**Expression**` hover. The hover range remains the complete `{{ … }}` expression.

## Formatting rule: bare-identifier (frontmatter variable) hover

A bare identifier inside `{{ … }}` is a **frontmatter variable** (the D2 rule above). When the document sets the key, hover shows its static value. When the key is *declared by the effective schema but unset in the document* — a caller-supplied parameter injected at compose time — hover falls back to the schema block: type, constraints, and `->` description, rendered by `frontmatter::schema_hover_details` (the schema body without the leading `` `key` `` heading, so the name is not duplicated under the `**Expression**` header). A set frontmatter value always wins over the schema block.

## Formatting rule: narrowed local type

When a guard narrows an identifier's type where the cursor is, the expression
hover ends with one more line giving the type there and the guards that
narrowed it. With `target: [string, number, "null"]` in the schema:

```md
::block when="is_string(target)"
Checking out {{ target }}.
::end-block
```

hovering `target` inside the block adds:

> Type here: `string` (narrowed by `::block when="is_string(target)" (line 5)`)

The type comes from Darkmatter's type scope, the same one composition uses for
shell template holes and transclusion targets, so the editor and `md compose`
always agree on it. DMLS does not decide any guard fact itself. A hover that no
guard narrows shows no extra line.

```mermaid
flowchart LR
    D[schema + static frontmatter] --> S[unguarded scope]
    B["enclosing ::block when guards"] -->|true side| S2[scope at the interpolation]
    S --> S2
    E["ternary / && / || around the cursor"] -->|each side| S3[scope at the identifier]
    S2 --> S3
    S3 --> H[Type here line]
```

Guards that apply:

- **Body `::block when="…"`**, every block that encloses the interpolation,
  outermost first. This covers prose interpolations, `::shell` holes, and
  `::shell-block` holes. On a `::shell` line or a `::shell-block` command the
  shell hover keeps precedence and appends the same line when the cursor is on
  a `{{ … }}` hole.
- **Inside the expression**: the branch of a ternary gets its side of the
  condition (`is_string(target) ? target : 'none'` shows `string` on the first
  `target`, `number | null` on the last), and a later `&&` / `and` operand gets
  the earlier operands' true side, a later `||` / `or` operand their false side.
  This also applies to an `expression`-typed frontmatter value.

Both branches are analyzed, each with its own side's type, because the editor
cannot know which one runs; this says nothing about which branch compose
reads. Guards are never evaluated: `file_exists(x)` makes `x` a present string
inside its block without checking any file or URL.

Not applied yet (**planned**):

- A lifecycle `when` (with its `then` / `else` actions) and a sequence task
  `when` in frontmatter do not narrow the hover of the actions they select: a
  value under `then:` hovers with its declared type. Call diagnostics in
  lifecycle stacks are narrowed (see
  [Diagnostics](./diagnostics.md)); sequence task bodies are not.
- Host globals (`err`, `timing`, `group`) and nested union members are not
  projected: a guard on `x.kind` narrows only `x.kind`, never `x`.

## Formatting rule: globals inside a lifecycle stack

Inside the value of one of the document's host's lifecycle events (md's
`initialize`, `composed`, `failed`, `cancelled`, `finalize`, or a host's from
its [lifecycle artifact](../../docs/topics/lifecycle/host-artifact.md), such as
Claudine's `start` and `success`), an expression is hovered with that
event's binding view, at any depth of the stack: a `{{ … }}` span, a `when`
condition, or an expression-typed value. A cursor on a global the event
declares, such as `err`, shows its declaration and whether the event can read
it, with the reason when it cannot. The event's global wins over a
frontmatter key of the same name, so the key's static value is not shown:

```yaml
err: shadow                       # a frontmatter key
composed:
    - info: "{{ err.msg }}"       # hover: err is unavailable in composed, and why
failed:
    - info: "{{ err.msg }}"       # hover: err is available in failed
```

Outside every stack, such as `{{ err }}` in the document body, `err` is the
frontmatter key again and hovers with its static value.

Hover on an event key itself is the ordinary schema hover, so it shows the
description of whichever schema types the key. Under Claudine's schema, for
example, `composed` and `failed` hover with the guidance to use Claudine's
`start` and `blocked`. A schema problem's diagnostic does not repeat that
description; hover is where it appears.

## Formatting rule: interpolation literal hover

A cursor anywhere inside an interpolation literal (`{{{ … }}}`) renders an `**Interpolation literal**` block showing the composed output — the inner content wrapped in `{{ … }}` as inline code (a fenced block when multiline, with backtick fences sized to survive content containing backticks) — followed by the note that the content is rendered as literal `{{ … }}` text and is not interpolated. The body is assembled by the pure `literal_hover_markdown` function in `src/providers/dsl.rs`. The hover range is the literal's complete outer `{{{ … }}}` span.

## Formatting rule: body-link hover to a Markdown document

Hovering a body link (`[setup](./setup.md)`, `[install](./setup.md#install)`)
that resolves to a Markdown document shows the target's title and path, the
heading line for a `#fragment`, and then the target's **summary**, so you
can tell what a document is about without opening it. With `docs/setup.md`:

```markdown
---
summary: Install the toolchain and run the first build.
---
# Setup

## Install
```

hovering `[install](./setup.md#install)` shows:

```markdown
**Setup**

→ `/work/repo/docs/setup.md`

Heading: Install

Install the toolchain and run the first build.
```

The summary is the first of the target's top-level frontmatter properties
`excerpt`, `summary`, `description`, and `about`, in that order, whose value
is a string that is nonempty after trimming. It is shown trimmed and
rendered as Markdown.

- A property whose value is `null`, a number or boolean, a list or mapping,
  a blank string, or a pending `{{ … }}` expression or `$(…)` command is
  skipped, never stringified, and the next property is tried. A quoted
  `"null"` or `"42"` is text and counts.
- Only top-level properties count; `meta.summary` does not. The target is
  never composed and its expressions are never run.
- With no usable property the hover is unchanged: title, path, and heading
  line only.
- Only body links to Markdown documents get a summary. A link to any other
  file shows only its path, and a `[[wiki]]` link keeps its own hover, which
  does not include the summary.

The summary is read when the target is parsed, the same way its headings
are, so the hover itself reads nothing from disk. A target is parsed on
demand the first time a link to it needs it, including a Markdown file
outside the workspace file list; DMLS does not parse every listed file to
collect summaries. An open buffer of the target wins over the file on disk;
editing it changes the summary, and closing it falls back to the disk copy.
A watched change or a save-triggered rescan refreshes it, deleting the
target removes it, and an unreadable target keeps the hover it had, without a summary.

## What answers a hover

Hover is a provider-chain capability: each provider may contribute, and the registry keeps the **first non-empty** result. Providers run in registration order, so a more specific overlay wins over the substrate for the same offset.

| Provider (in order)  | Hovers on                                            | Content                                                                                                   |
|----------------------|------------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| substrate (Markdown) | a heading or a link (not a `{{ }}` inside either)    | heading level and anchor; graph-sourced target preview, plus a Markdown target's frontmatter summary (an unopened target is read once, then cached) |
| wiki                 | `[[target]]` / `[[target#heading]]`                  | resolved target + heading preview (no summary)                                                            |
| frontmatter          | a frontmatter key or its value                       | key: schema type/required/enum/default/description, or a `ctx.*` generated-key annotation; value: the `$schema` or `file` value hover |
| DSL                  | a directive, `{{ }}` interpolation, `{{{ }}}` literal, or a `$()` value | directive semantics + resolved target; interpolation's static value, schema-property fallback, or `ctx.*` note, plus a guard-narrowed local type; literal composed-output + inert note; on a `$()` suffix (`::ok`, `::exit-code`, `::result`, `::timeout:<seconds>`, `::no-cache`), what the suffix does; elsewhere on a `$()` value, the shell policy verdict |

Because the chain stops at the first non-empty hover, a schema-defined frontmatter key whose value contains `{{ }}` is explained by the **frontmatter** provider (the schema description), while an *undefined* key's `{{ … }}` value falls through to the **DSL** provider's interpolation hover.

The substrate declines any offset inside a body `{{ … }}`, so an interpolation in a heading or link (`# Research on {{ seq.name }}`) shows the expression hover, while the rest of the heading still shows its anchor.

## Frontmatter typing and its reuse

Many frontmatter answers need to know what the schema says about a value:
whether it is an `expression`, a `file`, which union arm a list item
selected. DMLS works that out lazily and keeps it for as long as the buffer
and its schema inputs stay the same, so an editor's stream of hovers costs
the same on a large document as on a small one.

**A cursor request types only its own path.** Hover, completion, and
go-to-definition resolve the schema along the path of the entry under the
cursor (`initialize[3].then[0].warn`) and nothing else. On a hover inside a
lifecycle stack, only that scalar's `{{ … }}` spans and `when` guard are
read. A request never types the other entries to find its own, so a hover
on a 200-item lifecycle stack does the same work as on a one-item stack.

**Diagnostics types the whole frontmatter once.** Diagnostics and document
links need every scalar. The first of them classifies each scalar entry
(expression, file, or neither, with its decoded text) and keeps the list.
Every later request on the same buffer and schema reads that list instead
of walking again, including the next diagnostics pass. A cursor request
reads an entry from the list when it is already there, but never starts the
walk.

**Items share schema storage, not results.** Each list item still selects
its own union arm from its own authored values, exactly as before. What is
shared is the stored schema for an arm: every item that selects it reads the
same stored shape instead of a copy.

```yaml
$schema:
  step:
    - "{ kind: literal(say; required), text: string(required) }"
    - "{ kind: literal(shell; required), run: string(required) }"
  steps: step[]
steps:
  - kind: say      # selects the `say` arm
    text: hello
  - kind: shell    # selects the `shell` arm
    run: ls
  - kind: say      # selects `say` again: the same stored shape as item 0
    text: again
```

A hover on `text` in the third item describes the `say` arm; a hover on
`run` describes the `shell` arm. Items 0 and 2 share one stored `say` shape,
and item 1 has the `shell` one. An item whose `kind` is missing, unknown, or
not one of the literals sees the merged view of every arm, built once and
shared the same way. A recursive type (`item: "{ children: item[] }"`)
returns to the same stored shapes at every depth.

**When the stored analysis is replaced.** It belongs to one snapshot of the
document, and any change to what it was built from starts a new one:

| Change | Replaces the analysis |
| ------ | --------------------- |
| An edit, or a new buffer version with the same text | yes |
| A schema file the document's schema was built from changes on disk, even with no edit | yes, on the next request |
| A schema that failed to assemble because an imported file was malformed, when that file is fixed | yes, on the next request |
| A schema that failed because an imported file is missing, when the file is created | on the next watched-file event, rescan, or edit |
| The `.dmls.toml` or editor configuration, the trigger files, or the document's repository context | yes |
| Malformed YAML (the last good tree is kept) | yes: answers use the last good tree, never another buffer's text or ranges |
| Closing the document | dropped |

A request that is still running when the analysis is replaced finishes on
the one it started with; the replaced analysis is then dropped, and it never
becomes current again.

```mermaid
stateDiagram-v2
    [*] --> Snapshot: first request after open, edit, or input change
    Snapshot --> Snapshot: hover, completion, definition (cursor path only)
    Snapshot --> Inventoried: first diagnostics or document links (every scalar, once)
    Inventoried --> Inventoried: any later request reads the stored list
    Snapshot --> Replaced: edit or input change
    Inventoried --> Replaced: edit or input change
    Replaced --> [*]: last running request finishes
    Snapshot --> [*]: document closed
    Inventoried --> [*]: document closed
```

**Work kept across edits.** Two kinds of schema work are expensive enough
on large recursive types that DMLS keeps them across edits, keyed by the
schema content they came from rather than by a snapshot: the array-item
definition of a list (the union `- ` items are checked against), and, for
each union alternative, the prepared check that says whether an item matches
it (the `matches alternative … of …` line above). An edit that leaves the
schema as it was reuses both; a changed alternative is prepared again, and
an unchanged one beside it is not. The server keeps at most 256 of each and
starts over when that fills. The first hover over alternatives the server
has not seen pays for preparing them once.

## Passivity

Hover is read-only like every DMLS request: it resolves local files and reads the in-memory graph (parsing a link's unopened target the first time a hover needs it), but never executes a shell command (`$(...)`, `::shell`), fetches a remote URL, or mutates anything. A `$()` or `::shell` hover *explains* what compose would do and whether policy allows it — it never does it.

## Testing

The formatting rule is covered at two levels:

- **Unit** (`src/providers/frontmatter.rs`): `schema_hover_body` is called with   hand-built `PropertyDef`s and the emitted Markdown is asserted directly (type   is bold and not inline code; enum members and default are italic). No LSP   session is needed because the function is pure.
- **Integration** (`tests/l1/lsp_session.rs`): a full   `initialize → didOpen → textDocument/hover` session over the in-memory   connection asserts the rendered `contents.value` the editor would receive. The   `hover_markup(fixture, uri, line, character)` helper collapses the   request-and-extract boilerplate so hover assertions stay one line.

The value hovers' exact wording is unit-tested in `src/providers/reference_hover/tests.rs`, and every `$schema` and `file` outcome above is asserted end to end in `tests/l1/value_hovers.rs`.

## Collection locals

Inside a `::map` / `::filter` body, a read of a bound name shows the type the
library's scope gives it at the cursor:

> Local `person.name`: `string`

A narrowed local appends the guards (`(narrowed by …)`), and a local shadows a
frontmatter key or schema property of the same name, so no frontmatter value or
property description is shown for it. On the opener, hovering the source shows
its declared type and the type of one entry; a callback's expression and its
parameters show the types at that callback's own scope (another callback's
parameter is not visible there). The keyword still shows the directive
description.



## Inline callback locals

In `map(people, person => person.name)`, hover on a body read identifies it as
a callback local, names the owning function, and shows the projected item or
member type. A shallow `{name}` pattern projects that field directly. Bare
`person` and `local.person` refer to the same binding; `fm.person` still gets
document hover. Nested callbacks use the innermost matching local. The source
argument uses its enclosing scope and the local ends with the callback. Hover
uses shared passive type analysis and never runs a callback to obtain a value.
