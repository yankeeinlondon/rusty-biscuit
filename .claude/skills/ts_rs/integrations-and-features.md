# Integrations & Feature Flags (ts-rs)

## serde (default)

ts-rs is designed to align with serde’s serialization model.
What you get:

* `rename`, `rename_all`, tagging strategies (when supported), flatten alignment (when you use ts-rs attrs accordingly)

Guideline:

* If the wire format is “the contract”, make DTO structs mirror the serialized JSON shape.

## chrono

Enable feature to map chrono time types to TS-friendly representations (commonly `string`).
If you want strict control, override with:

* `#[ts(type = "string")]`

## uuid

Enable feature to map `Uuid` to `string`.

## serde_json

Enable feature (`serde-json`) to map:

* `serde_json::Value` to a TS type (commonly `any`/`unknown` depending on version/settings)
  If you want stricter typing, prefer explicit DTO enums/structs.

## url

Enable feature to map `url::Url` to `string`.

## format

Enable `format` to auto-format generated TypeScript with dprint’s TypeScript plugin.
Use this when:

* you commit generated types and want stable diffs

## no-serde-warnings

Enable when:

* you use serde attributes not supported by ts-rs and want a quieter build
  But prefer:
* explicitly shaping output using `#[ts(...)]` so TS matches actual API behavior

## Choosing alternatives (quick guidance)

* Specta: bigger projects, type-collection workflows, more export targets
* Typeshare: TS + Swift + Kotlin output needed
* Tsify: wasm-bindgen + wasm-pack `.d.ts` first
* Schemars: OpenAPI/JSON Schema pipeline and validation-oriented workflows