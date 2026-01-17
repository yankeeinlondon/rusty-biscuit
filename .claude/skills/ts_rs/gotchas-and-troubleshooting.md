# Gotchas & Troubleshooting (ts-rs)

## 1) “Why didn’t my TS file update?”

Exports happen at **test time**.
Fix:

* Run `cargo test` (locally and in CI)
* Ensure your export test runs in the right crate/workspace member

## 2) “My export directory is empty / missing”

* Verify `TS_RS_EXPORT_DIR`
* Ensure directory exists or is creatable
* Check write permissions in CI container

## 3) Tagged enums producing invalid/undesired TS

Problem case:

* `#[ts(tag = "type")]` + tuple/newtype variant

Fix:

* Convert tuple/newtype variants to struct variants:
  * `Success(String)` → `Success { value: String }`

## 4) Serde skip attributes don’t do what I expect

Notably:

* `#[serde(skip_deserializing)]` is ignored by ts-rs
  Best practice:
* Use `#[ts(skip)]` to remove fields/variants from TS output
* Or create dedicated exported DTOs instead of exporting internal structs

## 5) Large integers & precision

By default, 64-bit and larger integers often map to TS `number` (precision risk).
If your frontend needs exact integers:

* Set `TS_RS_LARGE_INT=bigint`
* Ensure your JSON encoding supports bigints (often it won’t; consider strings)

## 6) Generic complexity & associated types

TypeScript can’t model Rust associated types cleanly.
Workaround:

* Use `#[ts(concrete(...))]` to pick concrete type parameters
* Add `#[ts(bound = "...")]` to satisfy export trait bounds

## 7) Import extension / ESM issues

If generated TS imports need `.js` extensions (ESM builds):

* Set `TS_RS_IMPORT_EXTENSION=js` (or `mjs`)

## Debug checklist

* Inspect `Type::decl()` / `Type::export_to_string()` in a quick test
* Grep for `#[ts(...)]` and `#[serde(...)]` mismatches
* Confirm the type being exported is the same type used in API responses