# Patterns & Examples (ts-rs)

## Basic struct export

Use `#[derive(TS)]` and `#[ts(export)]`.

Typical DTO:

* IDs as `String`/`Uuid`
* `Option<T>` for nullable/optional data

## Optional fields

Two approaches:

1. Container-level:

* `#[ts(optional_fields)]` makes all `Option<T>` fields become `t?: T` (optional property)

2. Field-level:

* `#[ts(optional)]` for a single field

Choose based on wire format expectations:

* If your JSON omits missing fields: optional props are accurate
* If your JSON includes `null`: prefer nullable unions (or don’t mark optional)

## Flattening

`#[ts(flatten)]` on a nested struct field flattens its fields into the parent TS type.
Use when serde uses `#[serde(flatten)]` to match wire format.

## Enum representations

### Unit enums

Default is a **string-literal union** in TS:

* `type Status = "Pending" | "InProgress" | ...`

Use `#[ts(rename_all = "...")]` to match casing expectations (camelCase, etc.).

### Data-carrying enums (externally tagged by default)

Generated as unions like:

* `"Quit" | { "Move": { ... } } | { "Write": string }`

### Discriminated unions (recommended for TS ergonomics)

Use:

* `#[ts(tag = "type")]` (internally tagged)
  Optionally with:
* `#[ts(content = "data")]` (adjacently tagged)

Important constraint:

* Avoid tuple/newtype variants in tagged enums; prefer struct variants.

## Overrides & adapters

### `#[ts(type = "...")]`

Hard override TS type for a field.
Use for:

* timestamps (`string`)
* opaque JSON (`any` / `unknown`)

### `#[ts(as = "Vec<_>")]`

Generate TS as if the Rust type were another Rust type.
Useful for:

* sets/hashsets → arrays
* wrapper types

## Generics

ts-rs supports generics, but you may need:

* `#[ts(concrete(T = String))]` to export a concrete instantiation
* `#[ts(bound = "T: TS")]` when bounds aren’t inferred how you need

## Documentation export

Rust doc comments (`///` / `#[doc]`) are exported automatically as JSDoc/TSDoc in TS output.
Note:

* Regular `//` comments are not exported.