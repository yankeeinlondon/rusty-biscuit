Part of [Symbol Records](../symbol-records.md).

# Kind-Specific Payloads

Most of a symbol record is the same for every kind. What only makes sense for
one kind, such as a function's parameter list or an enum's variants, lives in
`kind_data`. It is a tagged object: `kind_data_type` names the payload and
`data` holds it.

```json
"kind_data": {
  "kind_data_type": "Function",
  "data": {
    "parameters": [
      { "name": "x", "position": 0, "type_text": "number", "default_value": "1", "is_optional": true,
        "is_variadic": false, "is_keyword_only": false, "is_positional_only": false },
      { "name": "rest", "position": 1, "type_text": "string[]", "is_optional": false,
        "is_variadic": true, "is_keyword_only": false, "is_positional_only": false }
    ],
    "return_info": { "type_text": "void", "never_returns": false },
    "is_variadic": true,
    "arity": 2
  }
}
```

That is the payload of the TypeScript function
`function f(x: number = 1, ...rest: string[]): void {}`.

## Which kind carries which payload

| `kind` | `kind_data_type` |
|---|---|
| `Function`, `Method`, `Constructor` | `Function` |
| `Enum` | `Enum` |
| `Type`, `Class`, `Interface`, `Trait` | `Type` when the extractor read a member list for the declaration, otherwise `Unknown` (a Rust `trait` carries `Unknown`) |
| `TypeAlias` | `TypeAlias` |
| `Field`, `Property` | `Field` |
| `Parameter` | `Parameter` |
| `Variable`, `Constant`, `Module`, `Namespace`, `EnumVariant`, `Macro`, `Unknown` | `Unknown` |

An `Unknown` payload has no `data`. It only means "no payload"; it says
nothing about the record's `kind`. The schema also declares `Module` and
`Variable` payloads (a module's members; a variable's type, initializer and
const flag), but no extractor produces them today.

## Function

| Field | Holds |
|---|---|
| `parameters` | the parameter list, in order (see below) |
| `return_info.type_text` | the return type as written; absent when none is written |
| `is_variadic` | whether any parameter is variadic |
| `arity` | the number of entries in `parameters` |

Each parameter has:

| Field | Holds |
|---|---|
| `name` | the parameter's name; a Rust `self` receiver is listed as `self` |
| `position` | its zero-based index in `parameters` |
| `type_text` | its declared type, as written |
| `default_value` | its default, as written |
| `is_optional` | `true` exactly when a default was read |
| `is_variadic` | a rest or variadic parameter (`...rest`, `*args`) |

Type texts are what the source declares, never inferred: a Rust function
without `->`, a Go function without a result and every constructor have no
`return_info.type_text`.

A default is read even where the grammar places it beside the parameter
rather than inside it: Swift `init(x: Int, y: Int = 2)` gives `y` the default
`2`. A comment on either side of the `=` is never the default and never hides
it, so `int x /* note */ = /* note */ 1` (C#) and `$x = /* note */ 1` (PHP)
both have the default `1` and are optional.

`arity` counts a Rust `self`, while `metrics.parameters` does not:
`fn norm(&self) -> f64` has `arity: 1` and `parameters: 0`.

**Not filled:** `receiver`, `abi`, `calling_convention` and `overload_set`;
on a parameter, `label`, `type_ast`, `mutability`, `ownership_mode`, `docs`
and `attributes`, and `is_keyword_only` and `is_positional_only` are always
`false`; on `return_info`, `type_ast`, `error_type` and `yields_type`, and
`never_returns` is always `false`.

## Type

A `Type` payload lists the members a type declares in its own body:

```json
"kind_data": {
  "kind_data_type": "Type",
  "data": {
    "fields": [
      { "name": "x", "type_text": "f64", "visibility": "Public", "is_static": false,
        "is_readonly": false, "is_mutable": true, "is_optional": false },
      { "name": "y", "type_text": "f64", "visibility": "Public", "is_static": false,
        "is_readonly": false, "is_mutable": true, "is_optional": false }
    ],
    "is_abstract": false, "is_final": false, "is_sealed": false, "is_open": false, "is_partial": false
  }
}
```

`fields` lists each field, property and constant the type declares in its
body, once, in source order. Each entry is paired with the standalone
`Field`, `Property` or `Constant` record of the same name: it keeps the type
written in the source, and takes the record's access level, `static` flag and
(when the entry has none) doc comment. A member record no reader listed is
added without a type. A Go struct field or a Scala body `val` is therefore
both a `Field` record and a typed entry here.

A field entry has `name`, `type_text`, `visibility`, `is_static` and `docs`.
Its `is_readonly` is always `false` and `is_mutable` always `true`, and
`qualified_name`, `is_optional` and `default_value` are not filled, so
TypeScript `b?: string` reads `is_optional: false`.

Which members appear for each language is described in
[Symbol Kinds, Visibility and `static`](../symbol-kinds-and-visibility.md).

**Not filled:** `members`, `constructors`, `associated_types` and
`associated_constants`; `is_abstract`, `is_final`, `is_sealed`, `is_open`
and `is_partial` are always `false`. A type's constructors are the
`Constructor` records whose `relations.parent` is the type, and its members
are in its `relations.members`; its modifiers are in the record's
`modifiers`.

## Field

The standalone `Field` or `Property` record carries a thinner `Field`
payload than its owner's list: `name`, `visibility`, `is_static` and `docs`,
but no `type_text`. Read a field's type from its owner's `fields` list.

## Enum

An enum and its standalone `EnumVariant` records name the same variants:

```rust
pub enum Mode { On, Off = 2 }
```

```json
{ "kind": "Enum", "identity": { "name": "Mode" },
  "kind_data": { "kind_data_type": "Enum",
                 "data": { "variants": [ { "name": "On" }, { "name": "Off", "discriminant": "2" } ] } } }
{ "kind": "EnumVariant", "identity": { "name": "Off", "container_name": "Mode" },
  "kind_data": { "kind_data_type": "Unknown" } }
```

`variants` lists each variant once, in source order. Each entry is paired
with the variant record of the same name (the k-th entry with the k-th
record); a variant record no reader listed is added at its position, and the
entry takes the record's doc comment and value only when it has none.

| Field | Holds |
|---|---|
| `name` | the variant's name |
| `discriminant` | the variant's value as written (Rust, C, C++, C#, TypeScript, PHP: `Off = 2` gives `2`) |
| `tuple_fields` | the types of a tuple payload: Rust `Pair(i32, String)` |
| `struct_fields` | the named fields of a struct payload: Rust `Named { x: i32 }`, Scala `case Value(x: Int = 1)(using o: Ord)` (every parameter clause, in order) |
| `docs` | the variant's doc comment |

Arguments are not a value: Java `MERCURY(1.0)` lists `MERCURY` alone, and a
Scala default is never a `discriminant`. Scala's `x` and `o` above are still
`Parameter` records (`Color.Value.x`). Only an `Enum` has a variant list; a
Swift `enum` is a `Type`, whose payload holds no variants.

**Not filled:** `backing_type`.

## Type alias

```json
"kind_data": { "kind_data_type": "TypeAlias", "data": { "aliased_type_text": "u32", "is_recursive": false } }
```

`aliased_type_text` is the alias's right-hand side as written, generics
included: Go `type GenAlias[T any] = []T` gives `[]T`, Swift
`typealias G<T> = Array<T>` gives `Array<T>`. A comment after the `=` is
skipped, so `typealias A = /* note */ Int` gives `Int`. The target is never
resolved, and its fields are never copied onto the alias. An abstract Scala
`type Element` and a Swift `associatedtype` declare no alias and have no
record.

**Not filled:** `aliased_type_ast`; `is_recursive` is always `false`.

## Parameter

A `Parameter` record (shown by `hug symbols --all-scopes` and in
`hug --json-full`) carries `name` and `position`. Today `position` is always
`0` and `type_text` is not filled; the parameter's real position, type and
default are in the `parameters` list of the function that declares it.
