# Attributes & Modeling Types

## Container attributes (struct/enum-level)

Common controls:

* `#[ts(export)]`: export this type during tests
* `#[ts(export_to = "api/types.ts")]`: custom output path (relative to export dir)
* `#[ts(rename = "UserName")]`: rename generated TS type
* `#[ts(rename_all = "camelCase")]`: rename fields/variants
* `#[ts(optional_fields)]`: `Option<T>` fields become optional properties (`t?: T`) rather than `t: T | null`
* `#[ts(tag = "type")]`, `#[ts(content = "data")]`, `#[ts(untagged)]`: enum representation controls
* `#[ts(concrete(T = String))]`: fix generics to concrete types for export
* `#[ts(bound = "T: TS")]`: explicit bounds when inference fails

## Field attributes (struct fields)

* `#[ts(skip)]`: exclude from TS output (use for internal/cache fields)
* `#[ts(rename = "id")]`: rename field
* `#[ts(optional)]`: make this `Option<T>` optional property
* `#[ts(flatten)]`: flatten nested struct into parent shape (matches serde flatten patterns)
* `#[ts(inline)]`: inline referenced type definition in place
* `#[ts(type = "any")]`: override TS type string (escape hatch)
* `#[ts(as = "Vec<_>")]`: generate TS as though it were a different Rust type

## Serde compatibility guidance

* Prefer putting JSON-shaping concerns in Serde attributes; ts-rs will usually follow.
* If a Serde attribute is ignored/unsupported, use `#[ts(...)]` equivalents (e.g., `#[ts(skip)]`).

---