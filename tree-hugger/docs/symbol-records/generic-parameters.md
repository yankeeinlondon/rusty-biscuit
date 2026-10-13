Part of [Symbol Records](../symbol-records.md).

# Generic Parameters

`type_info.generic_params` lists the generic parameters a declaration writes
in its own parameter list, in source order. `type_info.constraints` holds the
requirements whose subject is not one of those parameters. Both are read from
syntax alone, and both are filled on every selected record without any extra
capability.

## Example

```rust
pub struct Bag<T: Clone = String> { value: T }
impl<T: Clone> Bag<T> {
    pub fn get<U>(&self, value: U) -> U where U: Copy, T: Default { value }
}
```

`hug --json symbols bag.rs` gives these `type_info` facets (the `views` of
`get` left out):

```json
"Bag": { "generic_params": [{ "name": "T", "kind": "Type", "constraints": ["Clone"], "default": "String", "position": 0 }] }
"get": { "generic_params": [{ "name": "U", "kind": "Type", "constraints": ["Copy"], "position": 0 }],
         "constraints": ["T: Default"] }
```

Each entry of `generic_params` has:

| Field | Holds |
|---|---|
| `name` | the parameter's name |
| `kind` | `Type`, `Lifetime` or `Const` |
| `constraints` | the parameter's bounds, as written |
| `default` | its default, as written, without the `=` |
| `variance` | the variance marker, as written (C# `in`/`out`, Scala `+`/`-`) |
| `position` | its zero-based index in the written list |

## Rules

Each rule is visible in the example above.

- **Only the declaration's own list.** `get` does not receive `Bag`'s `T`,
  and `Bag` does not receive the `impl` block's `T`. A Rust `impl` block has
  no record of its own. A C++ member template reads only the `template`
  directly around it.
- **A requirement the declaration writes is its own, with or without a
  parameter list.** A member that constrains its enclosing type's parameter
  keeps that requirement and still owns no parameters; a requirement written
  on the enclosing template stays with the template:

  ```cpp
  template <class T> requires Small<T> struct Bound { void plain(); };
  template <class T> struct Gate {
      void check() requires Small<T>;   // constraints ["Small<T>"], generic_params []
      Gate() requires Small<T> {}       // constraints ["Small<T>"], generic_params []
  };
  ```

  `Bound` has `T` and `["Small<T>"]`; `plain` has neither. The same holds for
  a Rust or Swift method's `where` on its type's `T`, and for a C# `where`
  clause on a declaration with no list (which the compiler rejects but the
  grammar accepts).
- **Bounds attach to their parameter, in source order.** An inline bound
  (`T: Clone`) comes first, then bounds written in a `where` clause on the
  same bare name (`where U: Copy`). A requirement on anything else (`T` of
  the enclosing `impl`, `T::Item: Copy`, `Vec<T>: Send`, `for<'b> &'b T: …`,
  Swift `U == Int`, a C++ `requires` clause) goes to `type_info.constraints`
  as written, once. Nothing is inferred from how a parameter is used.
- **Texts are as written.** Bounds and defaults keep their source text
  (generic arguments, qualification and inner whitespace included) with
  outer whitespace trimmed. A bound list is split at its grammar entries
  (Rust `+`, Java `&`, C# `,`), never inside a nested type; comments between
  entries are dropped.
- **Kinds.** Rust lifetimes are `Lifetime` and keep their apostrophe (`'a`).
  Rust `const N: usize` and C++ non-type parameters are `Const`, and their
  declared type is not a constraint (it stays in `source.declaration_text`).
  Everything else is `Type`, including packs (C++ `class... Ts`, Python `*Ts`
  and `**P` named without the stars, Swift `each T` named without `each`) and
  higher-kinded parameters (Scala `F[_]`, C++ `template <class> class TT`,
  whose default in `template <class> class TT = Box` is `Box`).
- **Variance** is recorded as written: C# `in`/`out`, Scala `+`/`-`;
  otherwise absent.
- **Positions** are zero-based indexes in the written list. An unnamed C++
  parameter (`template <typename, int N>`) has no entry but keeps its slot,
  so `N` is position 1. A Rust attribute on a parameter is not a parameter:
  in `<#[cfg(feature = "x")] T, U>`, `T` is position 0.

### More than 65,536 parameters

Positions are 16-bit and are never truncated. A selected declaration with
more than 65,536 generic parameters fails its whole file with
`TreeHuggerError::TooManyGenericParameters`, which `hug` reports as
`too_many_generic_parameters`. A query that does not select that declaration
succeeds, and its parameter names still shadow same-named declarations when
relations are resolved.

## Per language

| Language | Parameters read from | Bounds | Defaults | Variance | `constraints` |
|---|---|---|---|---|---|
| Rust | items, functions, methods, traits, aliases | `T: A + B`, `where T: …` | `T = X`, `const N: u8 = 3` | — | other `where` predicates |
| TypeScript | classes, interfaces, aliases, functions, methods, arrow functions | `extends X` (one entry) | `= X` | — | — |
| Go | types (each spec of a `type` group), functions | the whole constraint (`~int \| ~float64`), shared by `K, V any` | — | — | — |
| Python | `class C[T]`, `def f[T]` | `T: int`, `T: (int, str)` | — | — | — |
| Java | classes, interfaces, records, methods, constructors | `extends A & B` | — | — | — |
| C# | classes, structs, interfaces, records, methods | `where T : class, new()` | — | `in`, `out` | a `where` clause naming no declared parameter |
| C++ | the `template` directly around the declaration | — | `= int`, `= 3`, `template <class> class TT = Box` | — | `requires` after that template's parameters; a function's own trailing `requires`, wrapped or not |
| Swift | types, functions, aliases | `T: P`, `where T: P` (after the list, or inside it: `<T where T: P>`) | — | — | `where` requirements on anything else (`U == Int`) |
| Scala | classes, traits, enums, functions, type aliases | `>: L`, `<: U`, `: Ctx`, operator kept | — | `+`, `-` | — |
| JavaScript, C, PHP, Perl, Bash, Zsh, Lua | — (no declared generic parameters; JSDoc and PHPDoc `@template` are not read) | | | | |

Scala keeps each bound's operator because one list mixes lower, upper and
context bounds; the other languages list only upper bounds and drop their
keyword (`extends`, `:`).

## Grammar and extractor limits

- The pinned Python grammar rejects a type-parameter default
  (`class C[T = str]`, PEP 696), and the `type A[T] = …` statement produces
  no record, so neither carries generics.
- The pinned TypeScript grammar rejects variance annotations
  (`interface K<in T>`).
- The pinned C++ grammar parses a concept-constrained type parameter
  (`template <std::integral T>`) exactly like a non-type parameter of a named
  type (`template <MyEnum E>`), so it is read as `Const` with no constraint.
  A `typename`/`class` parameter with a `requires` clause is read exactly.
- C# delegates produce no record, so their parameters are not reported.

The library's older `SymbolInfo` model has its own, separate list of type
parameter names (`TypeMetadata::type_parameters`), with its own coverage;
converting between the two models does not carry `generic_params` across
(see [Schema versions](./versions.md)).
