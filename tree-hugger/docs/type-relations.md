# Declared Type Relations

Every type-like record (class, interface, trait, type, enum) carries the base
types its declaration writes in its own header:

- `relations.extends`: classes, interfaces and traits it inherits from;
- `relations.implements`: interfaces and traits it declares conformance to.

Each entry is a `SymbolRef`. Its `name` is always the base **as written**. It
gets an `id` and an in-file `qualified_name` only when the name provably
refers to exactly one declaration in the same file. Rust `impl` blocks add a
type-level `implements` summary and decide which type their members belong
to.

These facts come from the syntax tree alone, during the parse pass. A
declarations-only query returns them, and no references, semantic, docs or
diagnostics pass is involved. `hug types --tree` and `hug symbols --tree`
show both lists at the end of each type's row (see
[Symbol Reports](./symbol-reports.md#declared-bases-on-type-rows)).

```typescript
interface Shape {}
class Base {}
class Square extends Base implements Shape, Comparable<Square> {}
```

```json
"relations": {
  "extends":    [{ "id": "<Base>",  "name": "Base",  "qualified_name": "Base" }],
  "implements": [{ "id": "<Shape>", "name": "Shape", "qualified_name": "Shape" },
                 { "name": "Comparable<Square>" }]
}
```

`Comparable` is not declared in this file, so its entry keeps only the
written text.

## What is recorded per language

| Language | `extends` | `implements` | Not a relation |
|---|---|---|---|
| Rust | trait supertraits (`trait A: B + c::D<u8>`) | trait impls, on each member and on the resolved self type (see [Rust impls](#rust-impls)) | lifetime bounds (`'a`), `?Sized` |
| TypeScript | class `extends`, interface `extends` | class `implements` | |
| JavaScript | the class `extends` expression, as written | — | |
| Go | named types embedded in an `interface` (`Base`, `fmt.Stringer`, `G[int]`) | — | unions, `~T` terms, predeclared non-interface types (`int`, `string`, …), embedded struct fields (composition) |
| Python | positional class bases, `*bases` kept as written | — | `metaclass=` and other keyword arguments, `**options` |
| Java | class `extends`, interface `extends` | class, enum and record `implements` | |
| C++ | every base specifier, private and `virtual` bases included | — | the `public`/`private`/`virtual` keywords (not part of the text) |
| PHP | class and interface `extends` | class and enum `implements` | trait `use` (composition) |
| C#, Swift, Scala | **planned** (see [below](#c-swift-and-scala)): nothing is recorded yet | | |
| C, Bash, Zsh | — (no inheritance) | — | |
| Perl | — | — | `@ISA`, `use parent` |
| Lua | — | — | metatables |

In every list:

- **Text as written.** Outer whitespace is trimmed. Generic arguments and
  call arguments stay (`pkg.Base<T>`, `mix(Base)`, `ns::TB<int>`, `G[int]`).
- **Source order, no repeats.** An expression written twice appears once.
  Different instantiations of the same base stay separate: `Mixin<string>`
  and `Mixin<number>` are two entries with the same `id`.
- **No inference.** Inherited members and transitive bases are never added.

## When a target gets an ID

A written base resolves to a same-file declaration only when its syntax,
qualification and lexical scope identify exactly one. Anything that cannot
be proved keeps its text alone. That counts as a successful result, not a
gap.

A path is resolved one segment at a time. Its **first segment** is decided
first, by the nearest scope that binds that name. Only then are the remaining
segments looked up, each inside the one before it.

```mermaid
flowchart TD
    W["written base"] --> P{"a plain name path?<br/>(Base, pkg.Base, crate::m::W)"}
    P -- "no: call, reference, tuple, dyn, higher-ranked bound" --> U["text only"]
    P -- yes --> A{"how does it start?"}
    A -- "Rust ::name (external crate)" --> U
    A -- "rooted: crate::, C++ ::, PHP leading backslash" --> G["first segment at the file root"]
    A -- "Rust self:: / super::" --> M["first segment in that module"]
    A -- relative --> T{"first segment is a<br/>visible type parameter?"}
    T -- yes --> U
    T -- no --> S["look the first segment up in each<br/>enclosing scope, innermost first"]
    S --> B{"this scope binds it?"}
    B -- "an import, or a wildcard import /<br/>using-directive in this scope" --> U
    B -- "no, try the next scope out" --> S
    B -- "no scope left, or a scope<br/>the lookup cannot pass" --> U
    B -- "yes: a declaration of any kind" --> G2["first segment decided here"]
    G --> R2
    M --> R2
    G2 --> R2{"each further segment declared inside<br/>a namespace, module or type?"}
    R2 -- "no: through a function, variable,<br/>parameter, or not declared" --> U
    R2 -- yes --> E{"exactly one declaration, and a<br/>class/interface/trait/type?"}
    E -- yes --> R["id + qualified_name"]
    E -- "no: duplicate, type alias, variable" --> U
```

Once a scope binds the first segment, outer scopes are never tried, even when
the full path does not exist there. A scope binds a name when it declares it
(a type, namespace, module, function, variable or parameter), when the name
qualifies a declaration there (a package or namespace that has no record of
its own), or when one of its imports names it.

The rules, with an example of each:

| Rule | Example | Result |
|---|---|---|
| A bare name resolves in the nearest enclosing scope that declares it | Python `class Outer: class Base` … `class Inner(Base)`, with `Inner` directly in `Outer`'s body | `Outer.Base`, not a module-level `Base` |
| A qualified name must match an in-file qualified name exactly | C++ `struct D : ns::B` | `ns::B` |
| A qualified path to another file never falls back to its last segment | `extends other.Root` next to a local `Root` | text only |
| A nearer binding of the first segment decides, even when it lacks the rest | C++ `namespace inner { namespace ns {} struct C : ns::B {}; }` next to a global `ns::B` | text only |
| A parameter is a binding | TypeScript `function f(ns: any) { class C extends ns.Inner {} }` next to `namespace ns` | text only |
| A function is not a namespace: its local classes are not reachable through its name | JavaScript `function outer() { class Local {} }` … `class C extends outer.Local {}` | text only |
| A type parameter shadows a local type, the enclosing declarations' included | C++ `template <class T> struct P : T` next to a `struct T` | text only |
| An alias would need expansion | TypeScript `implements Alias` where `type Alias = Root` | text only |
| Two declarations share the name (ambiguity) | Rust `#[cfg(a)] trait Amb` / `#[cfg(not(a))] trait Amb` | text only |
| An imported name has no declaration at its scope | Rust `use m::V; impl Tr for V` | text only |
| An import binds its name in the scope that holds it | Python `def g(): from lib import Base` then `class C(Base)` inside `g`, next to a module-level `Base` | text only |
| A wildcard import or C++ `using namespace` may bind any name | C++ `namespace n { using namespace other; struct C : B {}; }` next to a global `B` | text only |
| A Java or C++ type with bases may inherit the name, so lookup stops there | Java `class Sub extends A { class C extends Base {} }`, where `A` declares `Base` | text only |
| A class body is never a lexical scope in JS/TS | TypeScript `class Outer { Base = 1; m() { class C extends Base {} } }` | the module-level `Base` |
| A Python class body is a lexical scope only for code written directly in it: a nested class, method or function skips every enclosing class body, with its classes, variables and imports, and continues to the enclosing functions and the module | Python `class Outer: class Base` … `class Middle: class C(Base)`, or `def m(self): class C(Base)` | the module-level `Base`; text only when the module has none |
| A dynamic expression | JavaScript `class D extends mix(Base)` | text only |
| Rust names are looked up only as far as the nearest module | `mod b { trait T: Base {} }` with `Base` at the crate root | text only (`super::Base` resolves) |
| Rust `crate::`, `self::` and `super::` anchor the path | `trait U: crate::Base` | the crate-root `Base` |
| A Rust leading `::` names an external crate | `trait U: ::a::Dup` next to a local `mod a { trait Dup {} }` | text only |
| A C++ leading `::` anchors at the global namespace | `namespace ns { struct C : ::B {}; }` next to `ns::B` | the global `B` |
| A PHP leading `\` anchors at the file root | `implements \App\Other` | `App\Other` |
| A PHP name without a leading `\` is relative to its namespace only | `namespace Other; class C extends App\Root {}` next to `App\Root` | text only (it names `Other\App\Root`) |

A resolved target's `qualified_name` is the in-file name. The scan-root or
module prefix that reports add (`file.rs#Base`) never enters a stored
target.

The lookup uses one per-file index of every declaration by in-file qualified
name. The index is built once per file from every record, including records
of kinds a query did not select. So a query for classes alone still resolves
a base that is an interface. The file's imports are collected in one walk of
its syntax tree, only when a lookup needs them.

## Rust impls

Each `impl` block in a Rust file is read once, empty ones included
(`impl Marker for Local {}` has no members but still says something). Its
self type is resolved by the rules above, using the full written path.
`Gen<T>` and `crate::m::W<T>` match their base declaration. A reference,
tuple, `dyn` or other composite self type (`&Local`) attaches to no
component type. Every generic parameter in view of the impl counts, not only
its own: those of the functions, impls and traits around it too, so
`fn outer<T>() { impl Base for T {} }` never attaches to a file-level
`struct T`.

| Impl | Members' `implements` | Self type's `implements` | Members belong to (with Containment) |
|---|---|---|---|
| `impl fmt::Display for Local { fn fmt… }` | `fmt::Display` | `fmt::Display` added | `Local` |
| `impl Local { fn new… }` (inherent) | none | nothing added | `Local` |
| `impl !Send for Local {}` (negative) | none | nothing added | `Local` |
| `impl Other for &Local { fn by_ref… }` | `Other` | nothing added | a textual container `&Local`, no parent |
| `impl Other for other::Local { fn ext… }` | `Other` | nothing added | a textual container `other::Local`, no parent |
| `impl<T> Base for T { fn blanket… }` | `Base` | nothing added (`T` is the impl's parameter) | a textual container `T`, no parent |
| `fn outer<T>() { impl Base for T { fn probe… } }` | `Base` | nothing added (`T` is `outer`'s parameter) | a textual container `T`, no parent |
| `impl ::ext::B { fn probe… }` next to a local `mod ext { struct B; }` | none | nothing added (an external crate's type) | a textual container `::ext::B`, no parent |

- The self type's `implements` lists every positive trait impl in file
  order, with repeats of the same written trait removed.
- Each member keeps its own trait relation, so a query that returns methods
  alone still shows which trait supplied each method.
- With Containment, each member (and anything the container search places on
  the impl's type from inside it, such as a method's parameters) belongs to
  the resolved type. It is added to `relations.members` next to the type's
  ordinary members and the members of its other impls, in record order.
- When the self type does not resolve, the member's `relations.container` is
  the self type as written, with no `id`; it has no `parent`, and no local
  type lists it.
- A member's `identity.container_kind` follows the same decision, with or
  without Containment: the resolved type's kind, and no kind when the self
  type does not resolve, even beside a same-named local type.
- Qualified names, `identity.container_name` and symbol IDs are not changed
  by any of this (`impl Other for &Local { fn by_ref }` is still
  `Local::by_ref`).

The self type's resolution exists only while the file is parsed; no record
retains it. Widening a cached Rust entry by Containment therefore parses the
file once (`CacheLookup::WidenedCapabilities { parsed: true }`) instead of
guessing a container from records. Other languages still add Containment
from stored records. See [Cache design](./cache-design.md).

## C#, Swift and Scala

**Planned.** These three languages write a type's base class and the
interfaces, protocols or traits it conforms to in one list:

```csharp
class Child : Base, IDisposable {}   // C#
```

```swift
struct Point: Codable {}             // Swift
extension Point: Equatable {}        // Swift: conformance added elsewhere
```

```scala
class Child extends Base with Logging   // Scala
```

When a listed type is declared in another file, the source alone does not
say whether it is a class (inheritance) or an interface/protocol/trait
(conformance), and the meaning of `extends` for such an entry has not been
decided. Until it is, records in these languages have empty
`relations.extends` and `relations.implements`, and their outline rows show
no bases. Every other fact (generics, modifiers, attributes, body lines) is
read for them as for any other language.

## Limitations

- C#, Swift and Scala base lists are not read yet (**planned**, see
  [above](#c-swift-and-scala)).
- Go: a named type embedded in an interface constraint cannot be told apart
  from an embedded interface by syntax. Only the predeclared non-interface
  types (`bool`, `int`, `string`, …) are excluded.
- Name lookup is a bounded source lookup, not compiler resolution. Imports
  are read only as bindings that hide a declaration further out: an imported
  base never resolves, even when an unrelated local declaration has the same
  last segment. Java and Go imports are not read, since a same-file
  declaration of an imported name does not compile.
- Python and TypeScript names are looked up without regard to statement
  order: a binding later in a scope hides an outer declaration just as an
  earlier one does.
- Older-model adapters (`SymbolInfo`) carry no relations. Converting a record
  to `SymbolInfo` drops them.
