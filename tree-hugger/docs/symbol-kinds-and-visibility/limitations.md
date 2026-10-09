# Current Limitations

Part of [Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

These are limits of the extractor today, not of the grammars, except the
[grammar limits](#grammar-limits) below:

- **Swift** structs, classes and enums all report Type, and **Go** structs
  and interfaces both report Type. So a Go field's or a Swift initializer's
  container kind is Type (`[in type Point]`), and a Swift enum has no variant
  list.
- **Constructors beyond explicit instance constructors** keep their old
  kind or stay unreported: Java compact record constructors, C# static and
  primary constructors and destructors are not reported; a C++
  out-of-class `C::C() {}` is a Function with no container (its class is
  not resolved by name); PHP legacy class-name constructors and Python
  `__new__` are Methods.
- **Go struct members**: an embedded field (`type S struct { Base }`) and
  the fields of an anonymous struct type (`Inner` in
  `anon struct{ Inner int }`) are not reported; `anon` itself is. Go
  interface method elements (`M()` in `interface{ M() }`) are reported
  neither as symbols nor as entries of the interface's list.
- **Scala abstract type members** (`type Element`, with no `=`) are not
  reported; they declare no alias.
- **TypeScript `declare namespace D {}`** reports Unknown access, like every
  other `declare` declaration.
- **A standalone Field record's payload** (`kind_data`) has no type text,
  and its `is_mutable`/`is_readonly` are fixed at true/false whatever the
  declaration says; the type and the written annotation are in the owning
  type's field list.
- **C++ `= default;` and `= delete;` members** keep the clause in their
  header, terminator included (`C(const C&) = default;`).
- **Qualified names of out-of-class C++ definitions** do not include the
  class or namespace written in the definition's name (`void ns::W::f() {}`
  qualifies its locals as `k`, not `ns::W::f::k`).
- **Destructuring members**: a Python class-body target nested more than one
  pattern deep or starred (`a, (b, c) = …`, `a, *rest = …`), a Swift
  tuple-pattern property (`let (a, b) = (1, 2)`) and a Scala tuple-pattern
  member (`val (a, b) = (1, 2)`) are not reported, as symbols or field-list
  entries.
- **C and C++ field types** list the base type only: `int *ptrs[2]` is
  `ptrs: int`. Parameter and return types do the same: `char **argv` has
  type `char`, and `int (*f(int))(long)` returns `int`.
- **C and C++ unnamed parameters** (`int f(int);`) are not listed in the
  signature.
- **C++ out-of-class definitions** are reported behind one attribute or one
  pair of parentheses around the qualified name (`int S::f [[nodiscard]] ()`),
  not behind more (`int (S::f [[nodiscard]])()`).
- **Python `self.x`** attributes and **JavaScript `this.x`** properties
  assigned only in methods, never declared in the class body, are not
  reported. TypeScript never reported them.
- **Perl** coverage is narrow: subs, packages and `my`/`our`/`state`
  variables only; names keep their sigil.
- **Unresolved access** stays Unknown: Rust items implementing a trait
  declared elsewhere, Scala qualifiers that name a class, JS/TS script
  globals, and any language without access rules.
- **Cross-file facts** are not resolved: out-of-line C++ member definitions
  get no class container, and a Rust `impl` for a type declared in another
  file has no container kind.
- **Out-of-line `#[cfg(test)] mod tests;`** marks only the `mod`
  declaration; the other file's declarations are tests only when its path
  matches a file-name rule. `#[cfg(all(test, …))]` is not recognized either.
- **Test attributes mark only their own declaration**: the parameters and
  locals of a `#[test]` function are not tests (inside `#[cfg(test)]` they
  are).
- **Python test classes** (`unittest.TestCase` subclasses, `test_*`
  functions) count as tests only in a file whose name matches a rule; PHP's
  `#[\Deprecated]` attribute is not read (only the doc tag is).
- **Python `self`** counts as a parameter, since nothing in the syntax sets
  it apart.
- **C#, Swift and Scala base lists** retain inheritance in `extends`,
  conformance in `implements`, and uncertain entries in `unclassified_bases`.
  Swift extensions of external or ambiguous owners cannot attach to a local record. See
  [Declared Type Relations](../type-relations.md#c-swift-and-scala).
- **Some named declarations have no record**: Java annotation types
  (`@interface A`) and modules, C# events with accessors and `extern alias`
  names, Scala package clauses and package objects, C++ opaque enum
  declarations (`enum class E;`) and qualified unions (`union A::U {}`), and
  Swift macros and precedence groups. Their names are still never reference
  occurrences (see
  [Scope-Aware References](../references/language-support.md#declaration-names-in-the-other-languages)).
- **Declarations with no record carry no generics**: a Python
  `type A[T] = …` statement and a C# `delegate` produce no record, so their
  type parameters are not reported anywhere.
- **Adapter gaps**: converting a `SymbolInfo` to a record drops
  `TypeMetadata::type_parameters` and a TypeAlias's field metadata (C
  `typedef struct { … } Alias`). Converting a record to `SymbolInfo` drops
  its generic parameters and constraints, `extends`/`implements`,
  attributes, `metrics` and every modifier except `static` (both kinds) and
  `async`, since the older model has no field for them.

## Grammar limits

These forms cannot be represented by the pinned grammars, so they cannot be
reported correctly until the grammar changes. Each row is pinned by a test
that fails once a grammar upgrade lifts the limit (`symbols_kind_coverage`,
`symbols_generics`, `symbols_modifiers`):

| Snippet | Grammar | Observed | What is reported |
|---|---|---|---|
| `class Defaulted[T = str]: pass` (a type-parameter default, PEP 696) | tree-sitter-python 0.25.0 | `type_parameter` holds `ERROR(identifier "T", '=')` then `type "str"`; the file has a parse error | no default; Python generic parameters never carry one |
| `interface Variant<in T> {}` (also `out`, `in out`) | tree-sitter-typescript 0.23.2 | `type_parameter "in"` followed by an `ERROR`; the file has a parse error | no variance; TypeScript generic parameters never carry one |
| `template <std::integral T> void constrained(T)` | tree-sitter-cpp 0.23.4 | parses cleanly, but as `parameter_declaration(type: qualified_identifier, declarator: identifier)`, the same node as the non-type `template <MyEnum E>` | `T` is a `Const` parameter with no bound; a `typename T` with a `requires` clause is read exactly |
| `class A { @(flag ? one : two) m() {} }` | tree-sitter-typescript 0.23.2, tree-sitter-javascript 0.25.0 | the decorator becomes `parenthesized_expression(identifier, ERROR(…))`; the grammars accept only a path, a call of a path, or a parenthesized one of those | the file has a parse error; a TS/JS decorator is always a path or a call of one, so only Python keeps an arbitrary decorator expression as the attribute name |
| `object W { private enum Hidden { case H } }` (also `protected`, also at the top level) | tree-sitter-scala 0.24.0 | `ERROR(access_modifier)` followed by a separate `enum_definition`; the file has a parse error | `Hidden` is an Enum with access Public (the modifier is lost), and its case `H` an EnumVariant with the same Public access |

A Scala enum written without an access modifier is parsed and reported
normally, and its cases take the enum's access.

One more Scala form parses without an error but into the wrong shape. A
grouped declaration with no initializer (`val e, f: Int`) followed on the
next line by an unmodified `val` or `var` without a written type
(`var z = 1`, `val (p, q) = …`) is read by tree-sitter-scala 0.24.0 as one
definition whose type is the infix type `Int var z`: `e` and `f` are still
Fields, but their written type is that infix text and the following member
is lost. A single-name declaration, or a following member with a written
type, a modifier or another keyword (`val z: Int = 1`, `lazy val z = 1`,
`def`), parses as written.
