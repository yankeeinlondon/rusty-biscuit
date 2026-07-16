# Setup & Export Automation (ts-rs)

## Cargo.toml

Minimum:

* `ts-rs = "11.1"`

Common feature flags (enable as needed):

* `chrono` (chrono types)
* `uuid` (Uuid → string)
* `url` (Url → string)
* `serde-json` (serde_json::Value, etc.)
* `format` (format output TS via dprint plugin)
* `no-serde-warnings` (silence warnings for unsupported serde attrs)

Example patterns:

* `ts-rs = { version = "11.1", features = ["chrono", "uuid", "serde-json", "format"] }`

## Where files are written

Default output directory:

* `./bindings/`

Configure via environment:

* `TS_RS_EXPORT_DIR="./frontend/src/types"`

Or via `.cargo/config.toml`:

* `[env]`
* `TS_RS_EXPORT_DIR = { value = "frontend/src/types", relative = true }`

Configure import extension (ESM workflows):

* `TS_RS_IMPORT_EXTENSION="js"` (or `"mjs"`)

## Recommended export workflow

### A) Attribute-driven export

Add `#[ts(export)]` on types you want written out.

Pros:

* Minimal boilerplate

Cons:

* Export is still triggered when tests run; you need to run `cargo test`.

### B) Central export test (recommended for monorepos)

Create a single test that exports all relevant types to a known directory.

Pattern:

* Put it in `#[cfg(test)] mod tests { ... }`
* Call `YourType::export_all_to("./frontend/src/types")?;`

Why `export_all_to`:

* Automatically exports dependencies/nested types

## CI suggestions

* Add a CI step: `cargo test -p your_crate` (or workspace-wide)
* Optionally verify generated files are committed:
  * Run export
  * `git diff --exit-code` to ensure no drift

## Path semantics

* `#[ts(export_to = "api/types.ts")]` is interpreted relative to `TS_RS_EXPORT_DIR` (default `./bindings/`).
* Ensure the export directory exists and is writable.