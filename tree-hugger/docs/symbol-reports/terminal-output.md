# Terminal Output

Part of [Symbol Reports](../symbol-reports.md).

The terminal listing is what the declaration commands print when no other
output mode is chosen. This page covers the line format, wrapping, signature
modes, package sections, and the three alternative terminal layouts: plain,
table and outline.

## The listing line

```text
  - public struct Point { x: f64, y: f64 } src/shapes.rs:2:1
  - public field x [in type Point] src/shapes.rs:3:9
  - public method norm(self) -> f64 [in type Point] src/shapes.rs:9:5
```

Each line shows the visibility (omitted when not known), a label, the name
with its signature or fields, the container (`[in type Point]`) and the
location. The label is the declaring keyword for types, variables, fields
and constants (`struct`, `class`, `let`, `const`, `my`, `local`, …) and the
kind for everything else (`function`, `method`, `enum variant`, …). Both
come from the record; nothing is guessed from the source line.

`--plain` drops colors and hyperlinks. `--comments` adds each symbol's doc
comment under its line.

### Visibility labels

The label is the access declared or implied at the declaration:
`public`, `protected`, `private`, `internal`, `package`, `crate`,
`module-private`, `protected internal` and `private protected`. When the
level is unknown (parameters and locals, Python/Perl/shell code, a Rust
`impl` of a trait declared in another file) no label is printed rather than
a guess. The rules per language are in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

### Containers

A member ends with `[in <kind> <name>]` naming its container, or
`[in <name>]` when the container's kind is not known from the file. Members of
a Rust `impl` block are grouped under the implemented type, so a method in
`impl Point` or `impl Display for Point` prints `[in type Point]`, never
`[in impl Point]`.

### Kind colors

| Kind | Display | Color |
|------|---------|-------|
| Function, Method, Constructor | `function`, `method`, `constructor` | Green |
| Type, TypeAlias, Class, Interface | `type`, `type alias`, `class`, `interface` | Magenta |
| Enum, EnumVariant | `enum`, `enum variant` | Cyan |
| Field, Property | `field`, `property` | Cyan |
| Trait | `trait` | Yellow |
| Module, Namespace | `module`, `namespace` | Yellow |
| Variable, Parameter | `variable`, `parameter` | Blue |
| Constant | `constant` | Bright blue |
| Macro | `macro` | Red |

A line that shows the declaring keyword (`struct`, `union`, `const`,
`static`, `let`) uses its kind's color.

## Wrapping

Output to an interactive terminal wraps at the terminal's
width, and a wrapped line continues under the item's text (a hanging
indent). A token longer than the space left is broken with a hyphen.
Redirected output, `--plain` and `--format plain` never wrap. A positive
`COLUMNS` environment variable sets the width instead of the detected one,
and forced color (`CLICOLOR_FORCE=1`) counts as interactive:

```text
$ COLUMNS=40 hug functions wide.py
  - function summarize_quarterly_revenue(
    regional_breakdown: dict[str, float],
    …
```

## Signatures

`--signatures` sets how much of each signature a text
format shows:

| Mode | Shows | Example |
|---|---|---|
| `short` (default) | the compact form; a type's field or variant list past four entries ends in `… +N` | `public struct Wide { a: u8, b: u8, c: u8, d: u8, … +1 }` |
| `full` | the declaration header as written (multi-line headers on one line), in place of visibility and label | `pub fn grow(&mut self, by: u32) [in type Widget]` |
| `none` | names only, with the container and location | `public method grow [in type Widget]` |

`full` falls back to the short form for a declaration whose header was not
extracted.

## Source text is shown, never interpreted

Names, signatures, comments and
paths come from the scanned files. Every control character in them (an
escape sequence, a bell, a tab) is printed as its `\u{..}` escape, and markup
such as `**x**` or `<red>` prints as written, so a file cannot restyle the
terminal, clear the screen or plant a link:

```text
  - function paint(color = "\u{1b}[31m<red>**x**</red>") paint.py:1:1
```

## Package sections

When the scanned files belong to two or more packages
(directories with a `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`
or another manifest), the listing is split into one section per package, each
under its path, and a package without declarations shows `(no symbols)`.
Files outside every package form the scan root's own section, whose heading
is empty. A scan within one package is one flat list without headings.

The decision depends only on which packages the scanned files belong to,
never on what they declare, so it is made before anything is analyzed and no
package waits for a later one. A scan in which only one of several packages
declares anything is therefore still sectioned:

```text

core

  - public function parse(input: &str) -> Ast core/src/lib.rs:1:1


docs-site

(no symbols)

```

Every `--sort` and grouping follows the same rule.

## Plain (`--format plain`)

The terminal listing without colors, hyperlinks or wrapping, one declaration
per line; the same text `--plain` gives, and the form to pipe into `grep` or
a diff:

```text
$ hug functions --format plain
  - public method norm(self) -> f64 [in type Point] src/shapes.rs:9:5
  - public method scale(self, by: f64, origin: Point) [in type Point] src/shapes.rs:13:5
  - public function load(path: &str) -> Result<Point, String> src/shapes.rs:17:1
```

## Table (`--format table`)

One biscuit-terminal table per file with a selection, one row per
declaration, in `--sort` order:

```text
┌────────────┬──────────┬────────────┬──────────────┬─────────────────────┐
│ Location   │ Kind     │ Visibility │ Name         │ Signature           │
├────────────┼──────────┼────────────┼──────────────┼─────────────────────┤
│ rust.rs:9  │ method   │ public     │ new          │ (size: u32) -> Self │
│ rust.rs:25 │ function │ public     │ build_widget │ () -> Widget        │
└────────────┴──────────┴────────────┴──────────────┴─────────────────────┘
```

- **Location** is `file:line`, a link to the file in an interactive terminal
  that supports OSC 8 links. The link is made from the file's native path, so
  a name shown with `�` still links to the declaration file.
- **Signature** follows `--signatures` (`full` shows the header as written,
  `none` drops the column) and is the only column that wraps. A terminal too
  narrow for the other columns gets the table at its natural width.
- `classes` lists each selected container followed by its displayed members.
- A table per file keeps the output streaming; a scan with no selected
  declaration prints `(no symbols)`.

## Outline (`--tree`)

`functions`, `types` and `symbols` accept `--tree`, which nests each
declaration under the container that holds it, matched by the container's
record ID. Methods of out-of-body Rust `impl` blocks and Go receiver methods
therefore sit under their type, and every `impl` block of a type shares it.
The outline replaces the flat listing's `[in …]` suffix:

```text
lib.rs
  public struct Widget { size: u32 } [2:1]
    public field size [3:9]
    public method new(size: u32) -> Self [7:5]
    public method grow(self, by: u32) [13:5]
  [in Elsewhere]
    public method visit(self) [19:5]
```

When a declaration's container is not listed (the command does not select
its kind, a filter dropped it, or it is declared in another file), the
declaration sits under a marked heading such as `[in type Widget]` or
`[in Elsewhere]`. A member with no container in the file is headed by its
qualified owner as far as the source names it, so the methods of
`impl other::W` sit under `[in other::W]`, apart from a local `W` and from
the methods of `impl another::W`. The heading is context only: it is never a listed
declaration and `--count` or `--stats` never counts it. Overloads stay
separate rows in source order. Containment covers declarations the library
links; a parameter is placed under the container its record names, which
for a Rust method's parameters is the `impl` type.

### Declared bases on type rows

C#, Swift and Scala preserve bases with an unknown category in
`relations.unclassified_bases`, displayed as `unclassified bases External`.
Swift extension conformances follow the type's header entries. Under full
signatures, source-span evidence hides only entries written in the header,
so `struct Box: P` plus `extension Box: Q` shows
`struct Box: P implements Q`.

A type row (class, interface, trait, type, enum) ends with the bases its
record declares: `extends`, `implements`, then `unclassified bases`, each as written, in source
order, joined by `; `. A list that is empty is left out, so a type with no
bases looks as it always did. The row shows its bases even when no member
of it is listed, as in `hug types --tree`:

```text
shapes.ts
  interface Shape [1:1]
  class Base [2:1]
  class Square extends Base<number>; implements Shape, Comparable<Square> [3:1]
lib.rs
  public trait Shown extends Base, std::fmt::Debug [2:1]
  public struct W { 0: T } implements std::fmt::Display, Base [3:1]
```

- Only type rows show bases. A Rust method keeps the trait of its `impl` in
  its JSON record, but its row does not show it, and a `[in …]` heading
  never shows bases. `hug functions --tree` lists no type row, so it shows
  none.
- Selectors are not widened: a base is named on the row, never added as a
  row of its own, and a type a selector dropped is not brought back to show
  its bases.
- `--signatures none` and `short` show both lists. `--signatures full` shows
  the declaration header as written, which already writes most base lists
  (`class Square extends Base<number> implements Shape, …`); a list the
  header writes is not repeated. Two lists are read from elsewhere and so
  still follow the header: a Rust type's `implements`, which comes from its
  `impl` blocks (`pub struct W<T: std::fmt::Display> implements
  std::fmt::Display, Base`), and a Go interface's `extends`, which comes from
  the interfaces embedded in its body.
- Bases wrap with the row, under the same hanging indent, and are text: the
  `<…>` of `Comparable<Square>` is never read as markup.
- `--count` and `--stats` gain no category for bases. The same lists are in
  `relations.extends` and `relations.implements` of the `--json`, `--jsonl`
  and `--json-full` records; see [Declared Type Relations](../type-relations.md)
  for what each language records.

| Combination | Behavior |
|---|---|
| `--tree` alone | files in path order; containers and members in source order |
| `--tree --sort name\|kind` | within each file, containers ordered, and separately the members under each container; a heading sorts by its container's name |
| `--tree --group-by-file` | accepted, no extra effect: the outline is already per file |
| `--tree --group-by-module` | a directory heading where the directory changes |
| `--tree --format plain` | the outline without styles or wrapping |
| `--tree --format table` | argument error |
| `--tree` with `--json`, `--json-full`, `--jsonl`, `--count` or `--stats` | argument error |
| `classes --tree` | argument error: `classes` already nests members |
