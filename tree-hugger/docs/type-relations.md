# Declared Type Relations

Tree Hugger records which types a class, interface, trait or other type
declares as its bases, as written in its header:

```typescript
interface Shape {}
class Base {}
class Square extends Base implements Shape, Comparable<Square> {}
```

```console
$ hug types --tree shapes.ts
shapes.ts
  interface Shape [1:1]
  class Base [2:1]
  class Square extends Base; implements Shape, Comparable<Square> [3:1]
```

In JSON, `Square`'s record carries the same facts in `relations`:

```json
"relations": {
  "extends":    [{ "id": "b3603d06…", "name": "Base",  "qualified_name": "Base" }],
  "implements": [{ "id": "54f39a55…", "name": "Shape", "qualified_name": "Shape" },
                 { "name": "Comparable<Square>" }]
}
```

`Base` and `Shape` are declared in this file, so their entries carry the
`id` of that declaration, which lets a tool follow the edge to the base's own
record. `Comparable` is not declared here, so its entry keeps only the
written text.

This page explains what is recorded and the rules in brief. Three detail
pages hold the per-language rules:

- [Name Lookup for Base Types](./type-relations/name-lookup.md): exactly when
  a written base is proved to be one declaration of the file.
- [Rust Impls](./type-relations/rust-impls.md): how `impl` blocks add trait
  relations and decide which type their members belong to.
- [C#, Swift and Scala Base Lists](./type-relations/mixed-base-lists.md): how
  a list that mixes a superclass and interfaces is classified, and built-in
  type names in Go and Swift.

## What is recorded

Every type-like record (class, interface, trait, type, enum) has three
lists:

- `relations.extends`: classes, interfaces and traits it inherits from;
- `relations.implements`: interfaces and traits it declares conformance to;
- `relations.unclassified_bases`: written bases whose category is unknown
  (see [C#, Swift and Scala](#c-swift-and-scala)).

Each entry is a `SymbolRef`. Its `name` is always the base **as written**. It
gets an `id` and an in-file `qualified_name` only when the name provably
refers to exactly one declaration in the same file (see
[When a target gets an ID](#when-a-target-gets-an-id)). An empty list is left
out of the JSON.

These facts come from the syntax tree alone, during the parse pass. A
declarations-only query returns them, and no references, semantic, docs or
diagnostics pass is involved. `hug types --tree` and `hug symbols --tree`
show nonempty lists at the end of each type's row (see
[Symbol Reports](./symbol-reports/terminal-output.md#declared-bases-on-type-rows)).

In every list:

- **Text as written.** Outer whitespace is trimmed. Generic arguments and
  call arguments stay (`pkg.Base<T>`, `mix(Base)`, `ns::TB<int>`, `G[int]`).
- **Source order, no repeats.** An expression written twice appears once.
  Different instantiations of the same base stay separate: `Mixin<string>`
  and `Mixin<number>` are two entries with the same `id`.
- **No inference.** Inherited members and transitive bases are never added.
- **Declared in the type's own header**, plus two additions written
  elsewhere: Rust trait `impl`s and same-file Swift extension conformances.

## What is recorded per language

| Language | `extends` | `implements` | Not a relation |
|---|---|---|---|
| Rust | trait supertraits (`trait A: B + c::D<u8>`) | trait impls, on each member and on the resolved self type (see [Rust impls](#rust-impls)) | lifetime bounds (`'a`), `?Sized` |
| TypeScript | class `extends`, interface `extends` | class `implements` | |
| JavaScript | the class `extends` expression, as written | — | |
| Go | named types embedded in an `interface` (`Base`, `fmt.Stringer`, `G[int]`) | — | unions, `~T` terms, unshadowed predeclared non-interface types (`int`, `string`, …; see [Built-in spellings](./type-relations/mixed-base-lists.md#built-in-spellings)), embedded struct fields (composition) |
| Python | positional class bases, `*bases` kept as written | — | `metaclass=` and other keyword arguments, `**options` |
| Java | class `extends`, interface `extends` | class, enum and record `implements` | |
| C++ | every base specifier, private and `virtual` bases included | — | the `public`/`private`/`virtual` keywords (not part of the text) |
| PHP | class and interface `extends` | class and enum `implements` | trait `use` (composition) |
| C# | interface inheritance; proven class bases | struct/record-struct interfaces; entries after a class's first base; proven interface targets | enum underlying types |
| Swift | protocol inheritance; proven class bases | structs/actors, local-owner extensions, entries after a class's first base, proven protocol targets | proven enum raw-value types |
| Scala | trait inheritance; proven class bases | mixins after the first parent; proven trait targets on concrete owners | |
| C, Bash, Zsh | — (no inheritance) | — | |
| Perl | — | — | `@ISA`, `use parent` |
| Lua | — | — | metatables |

## When a target gets an ID

A written base resolves to a same-file declaration only when its syntax,
qualification and lexical scope identify exactly one. Anything that cannot be
proved keeps its text alone. That counts as a successful result, not a gap:
an entry without an `id` means "not proved in this file", never "wrong".

The lookup works like a compiler's name lookup, restricted to one file:

1. **The first segment of the path is decided first**, by the nearest scope
   around the base that binds that name. Only names that can be a type count
   (a value parameter called `Base` does not hide `trait Base`), except in
   Go, Python and JavaScript, where any binding of the name hides it. Once a
   scope binds the name, outer scopes are never tried.
2. **Each further segment is looked up inside the declaration the previous
   one selected**: `ns::B` is `B` inside the one `ns` the first step found.
3. **The result must be exactly one declaration** of a class, interface,
   trait or type. Anything else, such as two declarations of one name, a
   type alias or a variable, keeps the text.

The common cases:

| Written base | Example | Result |
|---|---|---|
| A bare name with a declaration in an enclosing scope | Python `class Outer: class Base` … `class Inner(Base)` inside `Outer` | `Outer.Base`, not a module-level `Base` |
| A qualified name matching an in-file qualified name | C++ `struct D : ns::B` | `ns::B` |
| A path to another file | `extends other.Root` next to a local `Root` | text only: it never falls back to its last segment |
| An imported name | Rust `use m::V; impl Tr for V` | text only |
| A wildcard import in scope | C++ `namespace n { using namespace other; struct C : B {}; }` next to a global `B` | text only: the import may bind `B` |
| A type parameter of the same spelling | C++ `template <class T> struct P : T` next to a `struct T` | text only |
| An alias | TypeScript `implements Alias` where `type Alias = Root` | text only: aliases are not expanded |
| Two declarations of one name | Rust `#[cfg(a)] trait Amb` / `#[cfg(not(a))] trait Amb` | text only |
| A dynamic expression | JavaScript `class D extends mix(Base)` | text only |
| A rooted path | Rust `trait U: crate::Base`, C++ `::B`, C# `global::Base`, PHP `\App\Other` | the declaration at the file root |

A resolved target's `qualified_name` is the in-file name. The scan-root or
module prefix that reports add (`file.rs#Base`) never enters a stored
target. The lookup reads every declaration in the file, including kinds a
query did not select, so `hug classes` still resolves a base that is an
interface.

[Name Lookup for Base Types](./type-relations/name-lookup.md) gives every
rule with an example, including lexical scopes and name groups per language,
competing owners of one name, C# `partial` types and generic arity, Python
class members rebound after their declaration, and the declarations that
PHP, Scala and C++ bind under a namespace their displayed name does not
spell.

## Rust impls

A Rust type's trait implementations are written in separate `impl` blocks,
not in its header. Tree Hugger reads each `impl` and resolves its self type
by the same lookup as a base:

| Impl | Members' `implements` | Self type's `implements` | Members belong to (with Containment) |
|---|---|---|---|
| `impl fmt::Display for Local { fn fmt… }` | `fmt::Display` | `fmt::Display` added | `Local` |
| `impl Local { fn new… }` (inherent) | none | nothing added | `Local` |
| `impl Other for &Local { fn by_ref… }` | `Other` | nothing added | a textual container `&Local`, no parent |

So `hug types --tree` shows `public struct Local implements fmt::Display`,
and a query that returns methods alone still shows which trait supplied
each method. Qualified names and IDs never change: `by_ref` is still
`Local::by_ref`. [Rust Impls](./type-relations/rust-impls.md) gives the full
rules, including blanket impls, impls inside functions and the effect on the
cache.

## C#, Swift and Scala

These languages write a superclass and its interfaces (protocols, traits) in
one list. Classification comes from syntax first, then from an unambiguous
same-file target declaration; what neither proves goes in
`unclassified_bases`:

```csharp
interface I {}
class Child : External, I {}   // implements I; unclassified bases External
struct Value : IDisposable {}  // implements IDisposable: a struct only has interfaces
```

Swift extensions add their conformances to a uniquely resolved local owner:
`struct Point: P {}` with `extension Point: Codable {}` records `P` and
`Codable` in `implements`.
[C#, Swift and Scala Base Lists](./type-relations/mixed-base-lists.md) gives
the rules per language.

## Limitations

- Swift extension conformances and members on external, aliased, imported
  or ambiguous owners cannot be attached to a local record. Conditional
  conformance requirements are not retained on the relation edge.
- An external first base in a mixed list remains unclassified when syntax
  cannot establish its category. This preserves the source fact.
- Go: a named type embedded in an interface constraint cannot be told apart
  from an embedded interface by syntax. Only the predeclared non-interface
  types (`bool`, `int`, `string`, …) are excluded, and only when no
  same-file declaration redeclares the name; a redeclaration in another file
  of the package is not seen.
- Name lookup is a bounded source lookup, not compiler resolution. Imports
  are read only as bindings that hide a declaration further out: an imported
  base never resolves, even when an unrelated local declaration has the same
  last segment. An import whose bound names cannot all be read counts as a
  wildcard, so lookup through its scope stays unresolved rather than
  guessing. A Java import binds its last segment (a type, or any group for a
  `static` import) and a Go import its alias, else the package name Go tools
  assume for its path (the last element without a `.vN` suffix, or the one
  before a `/vN` element); a Go package whose name differs from that is not
  seen. So `import java.util.Map.Entry` keeps `Entry.Base` as written in a
  file whose own package is `Entry`.
- Python, TypeScript, JavaScript, Rust, Swift and Scala names are looked up
  without regard to statement order within their scope: a binding later in a
  scope hides an outer declaration just as an earlier one does. Go, C++ and
  Java local declarations follow declaration order; their named-scope
  (package, namespace, class member) declarations do not.
- Older-model adapters (`SymbolInfo`) carry no relations. Converting a record
  to `SymbolInfo` drops them.

## Related pages

- [Name Lookup for Base Types](./type-relations/name-lookup.md): every rule
  for proving a target's identity.
- [Rust Impls](./type-relations/rust-impls.md): trait impls, self types and
  member ownership.
- [C#, Swift and Scala Base Lists](./type-relations/mixed-base-lists.md):
  classifying mixed lists and built-in spellings.
- [Symbol Records](./symbol-records.md): the record and its
  `relations` facet.
- [Symbol Reports](./symbol-reports.md): how relations are shown in tree
  and JSON output.
