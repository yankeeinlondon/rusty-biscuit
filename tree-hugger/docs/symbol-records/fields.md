Part of [Symbol Records](../symbol-records.md).

# Field Reference

This page lists every field of a symbol record, facet by facet, with what it
holds and when it is filled. The kind-specific `kind_data` payload has its own
page, [Kind-specific payloads](./kind-data.md), and the generic parameters
in `type_info` have [Generic parameters](./generic-parameters.md).

Two conventions apply throughout:

- An optional field without a value, and an empty list, is omitted from the
  JSON. A reader should treat a missing key as "none" or "empty".
- Some fields are **declared but not filled**: the schema reserves them for
  future analysis, and today they are always empty or always at their default
  value. Each section ends with a list of them, so a reader does not mistake
  a constant `false` for a measured one.

## Header

```json
"schema_version": { "major": 5, "minor": 1 },
"id": "ea8352547f52deac7db272b1651b1c84bfec409adcff861bcc69d19350062cd9",
"language": "Rust",
"kind": "Method",
"scope": "Member"
```

| Field | Holds |
|---|---|
| `schema_version` | the record format; see [Schema versions](./versions.md) |
| `id` | the record's stable ID: the BLAKE3 hash of `identity.stable_key`, 64 lowercase hex characters; see [IDs](#ids) |
| `language` | the source language, such as `Rust`, `TypeScript`, `Python` |
| `kind` | what the declaration is: `Function`, `Method`, `Constructor`, `Type`, `Class`, `Interface`, `Enum`, `Trait`, `Module`, `Namespace`, `Field`, `Property`, `Variable`, `Constant`, `Parameter`, `TypeAlias`, `EnumVariant`, `Macro` or `Unknown` |
| `scope` | where it sits: `Item` (file level, or in a namespace, package or module body), `Member` (in a type, trait, interface, object or `impl` body, or named on its owner, like a Go receiver method or a Lua `M.f`) or `Local` (in a function body, or a parameter) |

Which kind and scope each language construct gets is described in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

## Identity

```json
"identity": {
  "name": "norm",
  "display_name": "norm",
  "qualified_name": "shapes.rs#Point::norm",
  "is_namespace_rooted": false,
  "container_name": "Point",
  "container_kind": "Type",
  "keyword": "fn",
  "stable_key": "rust::shapes.rs::Method::Point::norm::0"
}
```

| Field | Holds |
|---|---|
| `name` | the declared name |
| `display_name` | the name to show; today always equal to `name` |
| `qualified_name` | the name with its enclosing declarations; never empty (see below) |
| `is_namespace_rooted` | `true` when `qualified_name` already starts at a `package` or `namespace` the file declares |
| `container_name` | the name of the nearest enclosing declaration (a class, trait, module, namespace, function, or the type a Rust `impl` implements); absent at file level |
| `container_kind` | that container's kind, read from its own declaration; absent when there is no container or the file cannot tell its kind |
| `keyword` | the token that introduces the declaration (`fn`, `struct`, `def`, `class`); absent when the construct has none |
| `stable_key` | the readable key the `id` is hashed from |

### Qualified names

The record stores an **in-file** qualified name, built from the file's own
source with the language's separators: `Record::load` in Rust,
`com.example.Point.area` in Java, `App\Models\User::find` in PHP. Because it
comes from the file alone, it does not depend on where the file is read from,
and both the cache and the `id` use this form.

A query result then adds a **prefix** that places the name in the project,
unless `is_namespace_rooted` is `true`. `hug` uses the file path relative to
the scan root and a `#`, so `Point::norm` is shown as
`shapes.rs#Point::norm`. A library caller can supply a module-style prefix
instead (`crate::shapes::Point::norm`); see
[Name prefixes](./file-index.md#name-prefixes). The `qualified_name` inside a
relation (`relations.parent`, an export's `symbol`) is always the in-file
form.

A name is namespace-rooted when it starts at a declared Java, Scala or Go
`package`, a C#, C++ or PHP `namespace`, or a Perl `package`. A block-form
namespace counts wherever it encloses the declaration: in Scala,
`package left { class W }` gives `left.W`, rooted, although the package itself
is not a record. A Rust `mod` or a type never roots a name. The per-language
prefix rules are in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

### Containers

`container_name` is a display name, and several records can share it (two
`Boxed` structs in different Rust `mod`s). The link to the actual owner
record is `relations.parent`, which identifies it by `id`; see
[Relations](#relations).

`container_kind` is omitted when the file cannot tell the container's kind,
for example a Rust `impl` for a type declared in another file. A Rust `impl`
member's `container_kind` is the kind of the declaration its self type
resolves to, by the rules in [Declared Type Relations](../type-relations.md),
and is omitted when the self type does not resolve: `impl ::ext::B` beside a
local `mod ext { struct B; }`, or `impl Base for T` inside `fn outer<T>()`
beside a file-level `struct T`.

### Keyword

`keyword` is read from the syntax tree, never from source text, so a comment
that names a keyword cannot supply one. It is a single token: Scala
`case class` reports `class`, C++ `enum class` reports `enum`, Swift
`class func` reports `func`. It is omitted when the construct has none, for
example a Java or C# method, a C variable or a TypeScript class member. For a
Go `const`/`var` spec and a Lua or shell `local`, the keyword belongs to the
statement that wraps the declaration, so it is reported even though
`source.declaration_text` does not contain it. The keywords of each language
are listed in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

### IDs

`stable_key` has five parts joined by `::`:

```text
rust :: /repo/src/shapes.rs :: Method :: Point::norm :: 0
lang    file identity          kind      qualified    sibling ordinal
                                         name
```

- **file identity** is the file's resolved absolute path (symlinks, `.` and
  `..` resolved), with `\` written as `/`; see
  [Cache Design](../cache-design.md) for how it is computed. `hug` prints this
  part relative to the scan root.
- **kind** is the `kind` value.
- **qualified name** is the in-file `qualified_name`, without any prefix.
- **sibling ordinal** counts the earlier records in the file with the same
  kind and qualified name, from `0`, so C++ overloads and repeated locals get
  distinct keys.

`id` is the BLAKE3 hash of the key as analyzed, in lowercase hex. The schema
version and extractor revision are not inputs, so a version bump keeps every
ID. Because `hug` rewrites the printed key but not the `id`, hashing the
printed key does not reproduce the `id`.

## Source

```json
"source": {
  "file_path": "shapes.rs",
  "declaration_span": { "start": { "line": 3, "column": 1 }, "end": { "line": 6, "column": 2 }, "start_byte": 48, "end_byte": 100 },
  "body_span": { … },
  "name_span": { … },
  "declaration_text": "pub struct Point",
  "signature_text": "pub struct Point"
}
```

| Field | Holds |
|---|---|
| `file_path` | the file the declaration is written in (scan-root-relative in `hug` output) |
| `declaration_span` | the whole declaration |
| `body_span` | the declaration's body, when it has one (see below) |
| `name_span` | the declared name token |
| `declaration_text` | **evidence**: the declaration up to its body |
| `signature_text` | **display**: the header a reader needs to recognize the symbol |

A span holds `start` and `end` points, each with a 1-based `line` and
`column`, plus 0-based `start_byte` and `end_byte` offsets into the file.

`body_span` of a function, method or constructor is its own implementation
body, the span `metrics.lines` counts: a block, or an expression body without
its `=>` or `=`, never a default value, a return type or a nested callable,
and never ending in whitespace. It is absent for a callable without a body (a
prototype, an abstract or interface method, a defaulted or deleted C++
member). Other declarations keep the body of their own node (a class body, an
enum body) when the grammar has one.

### Declaration text and display header

`declaration_text` runs from the declaration's start up to, but not
including, its body, with trailing whitespace trimmed; a declaration without
a body contributes its whole text. It keeps the modifiers, access keywords
and attributes the grammar parses inside the declaration node, so syntax that
`visibility.level` cannot express survives. Doc comments and syntax outside
the node (a Rust `#[…]` attribute, a Python decorator, a TypeScript `export`
wrapper) are not included.

`signature_text` is always a contiguous part of `declaration_text`:

- outer whitespace is stripped; internal whitespace and line breaks are kept
- it excludes initializers, accessor lists (C# `{ get; set; }`, Swift
  `{ get }`), the annotations, attributes and decorators that come before the
  first modifier or keyword, a `;` (Lua: `end`) terminator, and the `=`, `=>`
  or `:` that introduces a body
- an annotation written between modifiers stays, because removing it would
  break contiguity
- it is omitted when no contiguous header can be isolated: a C/C++
  declaration whose type carries a body (`typedef struct { … } Point;`), or a
  second name that an earlier name's initializer would cut off
  (`int a = 1, b = 2;` for `b`)
- a type alias's aliased type counts as its initializer, so
  `pub type Alias = u32;` has the header `pub type Alias`; a C `typedef`
  names the alias last, so `typedef unsigned long Count;` keeps its whole text
- each type of a grouped Go `type ( … )` block is declared by its own spec,
  so `Second` in `type ( First struct{ a int }; Second int )` has the
  declaration text, header and span `Second int`, with `type` still its
  `keyword`; a grouped alias is cut at its `=` like every alias

The two texts and `identity.keyword` have distinct contracts: the declaration
text is evidence, the header is for display, and the keyword comes from the
syntax tree rather than either text.

| Declaration | `declaration_text` | `signature_text` | `keyword` |
|---|---|---|---|
| `pub(in crate::a) fn f() { … }` | `pub(in crate::a) fn f()` | `pub(in crate::a) fn f()` | `fn` |
| `pub const TABLE: [u8; 256] = [0, 1, …];` | `pub const TABLE: [u8; 256] = [0, 1, …];` | `pub const TABLE: [u8; 256]` | `const` |
| `public private(set) var count = 0` | `public private(set) var count = 0` | `public private(set) var count` | `var` |
| `@Override public void run() { … }` | `@Override public void run()` | `public void run()` | omitted |
| `public @Deprecated static void m() { … }` | `public @Deprecated static void m()` | `public @Deprecated static void m()` | omitted |
| `private[Shape] def secret() = 2` | `private[Shape] def secret() =` | `private[Shape] def secret()` | `def` |
| `public int Count { get; private set; }` | `public int Count { get; private set; }` | `public int Count` | omitted |

`hug --json` and `--jsonl` drop `declaration_text` from their output;
`hug --json-full` and the library keep it.

**Not filled:** `doc_span`, `source_text` and `body_text` (**planned**).

## Visibility

```json
"visibility": {
  "level": "Public",
  "is_exported": true,
  "is_deprecated": false,
  "is_test": false,
  …
}
```

| Field | Holds |
|---|---|
| `level` | the access the declaration declares or implies: `Public`, `Protected`, `Private`, `Internal`, `Package`, `Crate`, `ModulePrivate`, `ProtectedInternal` (C# `protected internal`), `PrivateProtected` (C# `private protected`) or `Unknown` |
| `is_exported` | the declaration is exported by export syntax (`export` in JavaScript and TypeScript, a `pub` Rust item; never a member) |
| `is_deprecated` | the declaration is marked deprecated (an attribute, annotation or doc tag) |
| `is_test` | test-only code: a declaration in a recognized test file, with a test attribute, or inside a Rust `#[cfg(test)]` item |

`level` is the access written at the declaration, not the effective access
through enclosing modules or types. `Unknown` covers declarations without
access rules (parameters, locals, Python, Perl, shell) and access the file
cannot establish.

**Export is separate from visibility.** A `Public` level never implies an
export: a public Java class or a public TypeScript class method has
`is_exported: false`.

`is_test` and `is_deprecated` are computed only when the flags are requested
(every `hug` JSON report requests them). Without them both are `false`,
meaning "not computed"; with them a `false` means "computed and not set".
The rules for each language, and for each level, are in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

**Not filled** (always `false`): `is_reexported`, `is_default_export`,
`is_imported`, `is_external`, `is_generated`, `is_synthetic`,
`is_experimental`.

## Docs

```json
"docs": {
  "raw_doc": "Distance from the origin.",
  "comments": [
    { "kind": "Doc", "attachment": "Leading", "span": { … },
      "raw_text": "Distance from the origin.", "cleaned_text": "Distance from the origin." }
  ],
  "parsed": { "summary": "Distance from the origin." }
}
```

| Field | Holds |
|---|---|
| `raw_doc` | the doc comment attached to the declaration, with its comment markers removed |
| `comments` | the attached comments; today the doc comment, as one `Doc`, `Leading` entry |
| `parsed` | the doc comment split into parts (filled by the docs pass) |

`parsed` reads `raw_doc` line by line:

| Line | Goes to |
|---|---|
| `@param name description` | `params`, as `{ "name": …, "description": … }` |
| `@returns text` | `returns` |
| any other `@tag value` | `tags`, as `{ "name": "tag", "value": … }` |
| any other non-empty line | `summary`, all such lines joined with a space |

`parsed` is present only when it found a summary, a `@param` or a `@returns`.
Dedicated `description`, `throws`, `examples`, `remarks`,
`deprecated_message` and `since` fields are **planned**; today those tags
land in `tags`.

`hug --json` and `--jsonl` drop `comments` from their output and do not run
the docs pass, so they carry only `raw_doc`. Which comments count as
documentation in each language is described in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

**Not filled:** the `Line` and `Block` comment kinds, the `Trailing`,
`Inline` and `Detached` attachments, and `line_distance`.

## Attributes

```json
"attributes": [ { "name": "derive", "value": "Debug, Clone" } ]
```

`attributes` lists the declaration's own attributes, annotations and
decorators in source order, repeats kept. `name` is the path as written, with
whitespace and comments removed and without the marker (`#[…]`, `@`, `[…]`).
`value` is the raw text between the outer delimiters, trimmed: absent without
an argument list, and `""` for an explicit `()`.

| Source | `name` | `value` |
|---|---|---|
| Rust `#[tokio::test]` | `tokio::test` | absent |
| Java `@Deprecated(since = "9")` | `Deprecated` | `since = "9"` |
| Rust `#[allow()]` | `allow` | `""` |
| Rust `#[cfg_attr(test, derive(Debug))]` | `cfg_attr` | `test, derive(Debug)` (one attribute, never expanded) |
| Rust `#[doc = "a  b"]` | `doc` | `"a  b"` |
| Python `@(lambda f: f)` | `lambda f: f` | absent |

A decorator that is neither a path nor a call of a path keeps its whole
expression, as written, as the name. Attributes are read from Rust outer
attributes, C++ `[[…]]`, Java and Scala annotations, C# attribute lists,
Swift attributes, and Python, TypeScript and JavaScript decorators (an
exported class's decorator written before `export` belongs to the class; a
parameter's decorator belongs to the parameter). File-level forms (Rust
`#![…]`, C# `[assembly: …]`) and doc comments are never a declaration's
attributes. PHP `#[…]` attributes, Perl sub attributes and C `__attribute__`
are not read. The deprecation and test flags read the same list.

The pinned TypeScript and JavaScript grammars accept only a path, a call of a
path, or a parenthesized one of those as a decorator; `@(flag ? one : two)`
is a syntax error there, so a TypeScript or JavaScript decorator never takes
the whole-expression form.

## Modifiers

```json
"modifiers": [ { "type": "Static" }, { "type": "Async" },
               { "type": "Custom", "value": "abstract" }, { "type": "Custom", "value": "final" } ]
```

A modifier is a keyword the declaration itself writes. A keyword in its body,
a string, its type (TypeScript `size: readonly string[]`) or a parameter
(`fn f(mut x: u32)` marks the parameter `x`, not `f`) never marks it, and
none is inferred: Java interface methods are not `abstract`, and a Python
`@abstractmethod` adds no modifier.

| Modifier | Written as |
|---|---|
| `Static` | a member used without an instance: Java, C# and TypeScript `static`, Python `@classmethod`, a Rust associated function without `self`, a Java interface constant |
| `StaticStorage` | storage or linkage `static`: Rust `static` items, C and C++ file-scope, namespace-scope and function-local `static`; never implies `Static` |
| `Async` | an `async` function or method |
| `Const` | Rust `const fn` only; Java `final` and C++ `const` objects or member functions are not `Const` |
| `Readonly` | TypeScript `readonly` fields, interface properties and constructor parameter properties; C# `readonly` fields, structs and members |
| `Unsafe` | Rust `unsafe fn`, `unsafe trait` |
| `Mut` | Rust bindings (`let mut x`, `static mut S`, `mut x: T`, `mut self`) and a method whose receiver is written with `mut` (`&mut self`, `mut self`, `self: &mut Self`) |
| `Custom("abstract")` | TypeScript, Java, C#, Scala, PHP |
| `Custom("final")` | Java, C++ (class and member `final`), Swift, Scala, PHP |
| `Custom("override")` | TypeScript, C#, C++, Swift, Scala |
| `Custom("virtual")` | C#, C++ |
| `Custom("sealed")` | C# only (never renamed `final`) |
| `Custom("mutating")` | Swift only (never Rust's `Mut`) |

Go, C, JavaScript, Python, Perl, Bash, Zsh and Lua write none of the
`Const` to `Custom` modifiers. Other keywords (Scala and Java `sealed`, Swift
`open` and `nonmutating`, PHP `readonly`, C++ `constexpr`) are not read.
Further modifiers (`Open`, `Extern`, `Generator`) are **planned**.

A record lists each modifier once, in one fixed order: `Static`,
`StaticStorage`, `Async`, `Const`, `Readonly`, `Unsafe`, `Mut`, then distinct
`Custom` values in alphabetical order. `pub async unsafe fn poke(&mut self)`
gives `[Async, Unsafe, Mut]`. When `static` applies in each language is
described in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

## Type info

```json
"type_info": {
  "primary_type": "U",
  "views": [ { "kind": "Declared", "text": "U" }, { "kind": "Canonical", "text": "U" } ],
  "generic_params": [ { "name": "U", "kind": "Type", "constraints": ["Copy"], "position": 0 } ],
  "constraints": ["T: Default"]
}
```

| Field | Holds |
|---|---|
| `primary_type` | a function's declared return type text |
| `views` | the same type text as a `Declared` view and a `Canonical` view (the text trimmed) |
| `generic_params` | the parameters in the declaration's own generic parameter list |
| `constraints` | requirements whose subject is not one of those parameters |

`primary_type` and `views` are filled only for a function, method or
constructor that writes a return type. The generic fields are described in
[Generic parameters](./generic-parameters.md).

**Not filled:** the `Resolved` and `Expanded` views, a view's `ast` and
`referenced_symbol_ids`, and `nullability`.

## Relations

```json
"relations": {
  "parent":    { "id": "cad9ce0f…", "name": "Point", "qualified_name": "Point" },
  "container": { "id": "cad9ce0f…", "name": "Point", "qualified_name": "Point" }
}
```

Every relation entry is a **symbol reference**: `name`, plus the target's
`id` and in-file `qualified_name` when the target is a record in the same
file. A target outside the file (an import, a base type from a library) has
no `id`. A relation can also carry a `source_span`, used to tell a Swift
type's own header apart from a conformance added by an extension in the same
file.

| Field | Holds | Computed with |
|---|---|---|
| `parent`, `container` | the record that owns this one | containment |
| `members` | the records this one owns | containment |
| `extends`, `implements` | the base types the declaration writes in its own header | declarations |
| `unclassified_bases` | written bases whose kind (inheritance or conformance) cannot be decided | declarations |
| `references` | the in-file records and the imports that the declaration's code refers to | references |
| `dependencies` | the imports among those, alone | references |
| `referenced_by` | the records whose code refers to this one | references |

**Owner.** `parent` and `container` identify the owner by `id`: the smallest
same-named declaration around the member; else the same-named record of
`container_kind` whose qualified name qualifies the member's (`right::Boxed`
for `right::Boxed::work`); else the only such record whose qualified name
ends with the member's owner path. With several candidates and none
qualifying, the member has no parent. A lone same-named record never
overrides a qualification that names another owner: beside a local `W`, the
method of `impl other::W` is `other::W::probe` with no parent.

**Base types.** `extends` and `implements` hold each base as written
(`pkg.Base<T>`, `fmt::Display`), with an `id` only when the path names
exactly one eligible declaration in the file. A Rust trait impl adds its
trait to each of its members and to the resolved self type. C#, Swift and
Scala bases whose kind cannot be decided go to `unclassified_bases` rather
than changing the meaning of `extends`. The rules are in
[Declared Type Relations](../type-relations.md).

**Reference edges.** `references`, `dependencies` and `referenced_by` are
read from the file's reference occurrences whose target the language's scope
rules establish; a name-only or unresolved occurrence makes no edge. An edge
starts at the callable that owns the occurrence, so file-level code adds
none. Each
target is listed once: resolved targets repeat when their IDs are equal,
targets without an ID (an import) when their name and qualified name are
equal.

```ts
import { path } from "os";
import { argv } from "sys";

function run() {    // references: path, argv (both imports, path once)
    path.join("a");
    path.join("b");
    return argv;
}
```

How occurrences are found and resolved is described in
[Scope-Aware References](../references.md).

**Not filled:** `overrides` and `related`.

## Semantics

```json
"semantics": { "may_throw": "Unknown", "may_panic": "Unknown", "is_recursive": "Yes",
               "mutates_self": "Unknown", "mutates_arguments": "Unknown" }
```

The semantic pass fills this facet for functions, methods and constructors.
Each value is `Yes`, `No` or `Unknown`, and an absent key means "not
analyzed", which is different from an analyzed `Unknown`. Without the
semantic pass the facet is `{}`.

| Field | Value |
|---|---|
| `is_recursive` | direct recursion: `Yes` only for a call that resolves to the callable itself, `No` only when every call in its body was assessed and none might be to itself, otherwise `Unknown` |
| `may_panic` | `Yes` when the body refers to `panic` or `panic!`, otherwise `Unknown` |
| `may_throw`, `mutates_self`, `mutates_arguments` | always `Unknown` today |

```rust
fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }
// semantic pass ran:     "semantics": { "is_recursive": "Yes", … }
// semantic pass not run: "semantics": {}
```

The recursion rules, with examples, are in
[Scope-Aware References](../references.md).

**Not filled:** `is_pure`, `has_side_effects`, `reads_global_state`,
`writes_global_state`, `allocates` and `blocks`.

## Metrics

```json
"metrics": { "parameters": 2, "has_docs": false, "lines": 3, "cyclomatic": 2 }         // Rust, JavaScript, TypeScript: measured
"metrics": { "parameters": 2, "has_docs": false, "lines": 3, "cyclomatic": null }      // Python: no counting rules
"metrics": { "parameters": null, "has_docs": null, "lines": null, "cyclomatic": null } // Bash: unsupported
                                                                                       // no "metrics" key: not requested
```

`metrics` is present only when metrics were requested (`hug --json-full`,
`hug --stats`, and filters such as `--undocumented` and `--min-params`).
Inside a present facet every key is required: `null` means the metric does
not apply to this declaration or language, and a missing key makes the
record malformed. A filter never matches a `null` metric, so `--undocumented`
skips a Lua function rather than reporting it as undocumented.

| Field | Holds | `null` for |
|---|---|---|
| `parameters` | the length of the declaration's own parameter list (every list of a curried Scala `def`), counting a pattern-bound parameter like a named one, without a Rust `self`, a variadic parameter once | a declaration that is not a function, one with no parameter list (`def area: Int` in Scala), Perl, Bash and Zsh |
| `has_docs` | whether `docs.raw_doc` is present | Lua, Perl, Bash and Zsh, which have no doc-comment detection |
| `lines` | the physical lines a function, method or constructor's `source.body_span` touches (`0` for an empty Lua body) | every other kind, and a callable without a body |
| `cyclomatic` | the cyclomatic complexity of that same body: 1 plus one per decision | every other kind, a callable without a body, a language without counting rules (every language but Rust, JavaScript and TypeScript today), a syntax error in the body, a total beyond `u32` |

`cyclomatic` is never `0`, and `null` must never be read as zero. How it is
counted, with an example per rule, is in
[Cyclomatic Complexity](../cyclomatic-complexity.md); the other metric
definitions are in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

`metrics.parameters` and the `arity` of a function payload can differ:
`arity` counts every extracted parameter, a Rust `self` included, so
`fn norm(&self)` has `arity: 1` and `parameters: 0`.

## Provenance

```json
"provenance": { "extractor": "tree-sitter", "extractor_version": "0.1.0+rev.50", "parse_pass": "docs" }
```

| Field | Holds |
|---|---|
| `extractor` | always `tree-sitter` |
| `extractor_version` | the library version and its extraction-rules revision, `{version}+rev.{revision}`; the revision changes whenever the extraction rules do, which invalidates cached records without a schema change |
| `parse_pass` | the last analysis pass that changed the record: `parse`, `bind`, `semantic` or `docs` (the diagnostics pass changes no record) |

Records carry no timestamps, so two runs over the same files produce
identical records.

## Extensions

`extensions` is a free-form map of language-specific data. It is omitted when
empty, and no extractor fills it today.
