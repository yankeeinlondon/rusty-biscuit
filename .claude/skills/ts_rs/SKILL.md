---
name: ts_rs
description: Expert knowledge for generating TypeScript type declarations from Rust types with ts-rs (derive macros, export workflows, serde compatibility, and ecosystem feature flags). Use when sharing Rust API/DTO types with TypeScript frontends, automating .ts/.d.ts generation in CI, or troubleshooting enum/tagging/generics/export path issues.
tools: [Read, Write, Edit, Grep, Glob, Bash]
---
# ts_rs

## What this skill does
Provides end-to-end guidance to implement and maintain Rust→TypeScript type generation using **ts-rs**:
- Add `#[derive(TS)]` and `#[ts(...)]` attributes correctly
- Automate exports (test-time export, output directories, CI usage)
- Ensure parity with **serde** serialization (rename/flatten/tag strategies)
- Handle generics, enums, and common “this generated weird TS” gotchas
- Enable feature flags for popular crate types (chrono/uuid/url/serde_json/etc.)

## When to use
Use this skill when you are:
- Building a Rust backend (axum/actix/rocket) with a TS/JS frontend and want **shared DTO types**
- Adding type exports to a monorepo (`frontend/src/types`, `packages/*`)
- Fixing incorrect enum representations (tagged/untagged) or optional/flatten behavior
- Introducing chrono/uuid/serde_json and need correct TS mappings
- Setting up CI to ensure generated TS stays up-to-date

## When not to use
Avoid ts-rs (or consider alternatives) when:
- You need runtime or non-test-time generation (ts-rs exports during `cargo test`)
- You need multi-language output (Swift/Kotlin) → consider **typeshare**
- You need WASM-first `.d.ts` integration → consider **tsify**
- You want a type-collection/export pipeline with more formats → consider **specta**
- Your schema is best expressed as OpenAPI/JSON Schema → consider **schemars** + converters

## Quick start checklist (project setup)
1. Add dependency:
   - `ts-rs = "11.1"` (optionally enable feature flags)
2. Add `#[derive(TS)]` and `#[ts(export)]` to exported DTOs
3. Ensure export directory exists (default `./bindings/` or configured)
4. Run `cargo test` to generate `.ts` files
5. Import generated types in your frontend

Detailed setup & automation: [setup-and-export.md](setup-and-export.md)

## Core patterns you’ll use constantly
- **Structs & enums**: `#[derive(TS)]`, `#[ts(export)]`
- **Serde alignment** (default `serde-compat` feature): field renames, tagging, flattening
- **Output control**: `#[ts(export_to = "...")]`, `TS_RS_EXPORT_DIR`
- **Field shaping**: `#[ts(optional_fields)]`, `#[ts(optional)]`, `#[ts(flatten)]`, `#[ts(type = "...")]`, `#[ts(as = "...")]`
- **Enum shaping**: `#[ts(tag = "...")]`, `#[ts(content = "...")]`, `#[ts(untagged)]`, `#[ts(repr(enum))]`

Examples and recipes: [patterns-and-examples.md](patterns-and-examples.md)

## Common gotchas (high-value fixes)
- Export happens at **test time**, not compile time → run `cargo test` in CI
- Tagged enums + tuple/newtype variants can produce invalid/undesired TS → use struct variants
- `#[serde(skip_deserializing)]` is ignored by ts-rs → use `#[ts(skip)]`
- Large integers default to `number` → set `TS_RS_LARGE_INT=bigint` if needed
- Generics may require `#[ts(concrete(...))]` and `#[ts(bound = "...")]`

Troubleshooting guide: [gotchas-and-troubleshooting.md](gotchas-and-troubleshooting.md)

## Integration notes
- **serde**: default compatibility; prefer DTO structs that match actual wire format
- **chrono/uuid/url/serde_json/etc.**: enable ts-rs features to map foreign types
- **Formatting**: optional `format` feature to format output TS

Integration details: [integrations-and-features.md](integrations-and-features.md)

---
## Operational advice (best practice)
- Keep exported types in a dedicated `api`/`dto` module to avoid leaking internal fields.
- Add a single “export all types” test to make generation deterministic.
- Consider pinning ts-rs versions and regenerating types on upgrades (major versions can change output).