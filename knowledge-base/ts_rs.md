# ts-rs (ts_rs) Deep Dive: The Definitive Reference (Rust → TypeScript Type Generation)

`ts-rs` is a Rust library that **generates TypeScript type declarations from Rust types**. It is designed to keep Rust backends (or shared libraries) and TypeScript/JavaScript consumers aligned with a **single source of truth**: your Rust type definitions.

It is commonly used in Rust web backends (Axum/Actix/Rocket), Tauri apps, shared Rust crates consumed by TS, and WASM projects (as a complement to `wasm-bindgen`).

---

## Table of Contents

1. [What ts-rs Is (and Why It Exists)](#what-ts-rs-is-and-why-it-exists)
1. [Installation & Feature Flags](#installation--feature-flags)
   1. [Common Feature Integrations](#common-feature-integrations)
   1. [Formatting Generated TypeScript](#formatting-generated-typescript)
1. [How Generation Works: Trait + Derive Macro](#how-generation-works-trait--derive-macro)
   1. [The `TS` Trait (Core API)](#the-ts-trait-core-api)
   1. [Derive Macro Basics](#derive-macro-basics)
   1. [Export Lifecycle: Why `cargo test` Matters](#export-lifecycle-why-cargo-test-matters)
1. [Quick Start: First Export](#quick-start-first-export)
1. [Type Coverage: What ts-rs Can Represent](#type-coverage-what-ts-rs-can-represent)
1. [Structs Deep Dive](#structs-deep-dive)
   1. [Field Renaming, Optionality, and Overrides](#field-renaming-optionality-and-overrides)
   1. [Inlining and Flattening](#inlining-and-flattening)
1. [Enums Deep Dive (Rust ADTs → TypeScript Unions)](#enums-deep-dive-rust-adts--typescript-unions)
   1. [Unit Enums → String-Literal Unions (Default)](#unit-enums--string-literal-unions-default)
   1. [Data-Carrying Enums → Union Shapes](#data-carrying-enums--union-shapes)
   1. [Discriminated Unions with `tag` / `content`](#discriminated-unions-with-tag--content)
   1. [Emitting a TypeScript `enum` with `repr(enum)`](#emitting-a-typescript-enum-with-reprenum)
1. [Generics and Concretization](#generics-and-concretization)
1. [Serde Compatibility (and What It Does *Not* Do)](#serde-compatibility-and-what-it-does-not-do)
1. [Export Configuration: Paths, Environment Variables, and ESM Imports](#export-configuration-paths-environment-variables-and-esm-imports)
1. [Programmatic Usage & Introspection](#programmatic-usage--introspection)
1. [Gotchas, Limitations, and Sharp Edges](#gotchas-limitations-and-sharp-edges)
1. [Version Notes & Evolution (Important Breaking Changes)](#version-notes--evolution-important-breaking-changes)
1. [Ecosystem & Alternatives](#ecosystem--alternatives)
1. [When to Use ts-rs (and When Not To)](#when-to-use-ts-rs-and-when-not-to)
1. [License](#license)

---

## What ts-rs Is (and Why It Exists)

In many systems, you define a data model twice:

* Rust backend types (for serialization, persistence, API payloads)
* TypeScript frontend types (for request/response handling, UI state)

This duplication is error-prone. `ts-rs` solves this by generating `.ts` type declarations directly from Rust, so the Rust definition becomes the canonical schema.

**Key characteristics:**

* Single trait-based design (`TS`)
* `#[derive(TS)]` macro for most use cases
* Automatic **dependency discovery/export** for nested types
* **Serde attribute compatibility** (default feature) to match actual JSON shape
* Handles structs, enums, generics, and many std/external types
* WebAssembly-friendly (generates types; does not generate runtime conversion glue)
* Export automation typically triggered during `cargo test`

---

## Installation & Feature Flags

Add to `Cargo.toml`:

````toml
[dependencies]
ts-rs = "11.1"
````

Enable optional integrations as needed:

````toml
[dependencies]
ts-rs = { version = "11.1", features = ["chrono", "serde-json", "format"] }
````

### Common Feature Integrations

`ts-rs` supports many “foreign” crate types via feature flags (to work around Rust orphan rules). Examples:

* `chrono`: maps chrono date/time types (commonly to TS `string`)
* `uuid`: maps UUIDs (commonly to TS `string`)
* `url`: maps `url::Url` (commonly to TS `string`)
* `serde-json`: support for `serde_json` types (e.g., `Value`)
* others: `bigdecimal`, `bson`, `bytes`, `indexmap`, `ordered-float`, `semver`, `smol-str`, `heapless`, etc.
* version note: `tokio` support exists behind a feature gate in later versions (notably mentioned in 10.1)

### Formatting Generated TypeScript

Enable:

* `format`: formats output using `dprint-plugin-typescript`

This is useful for committed, stable diffs in generated files.

---

## How Generation Works: Trait + Derive Macro

### The `TS` Trait (Core API)

The central abstraction is the `TS` trait. `#[derive(TS)]` implements it for your types.

Key methods you’ll see in practice:

* `decl()` / `decl_concrete()`: returns a full TS declaration string
* `name()`: TS type name
* `inline()`: inline type expression (useful for nested/anonymous shapes)
* `dependencies()`: collect referenced dependent types
* export-related helpers:
  * `export()`, `export_all()`, `export_all_to(...)`, `export_to_string()`

Conceptually:

* **Your type produces a TS declaration**
* **ts-rs walks dependencies** and exports them too (when configured)

### Derive Macro Basics

Basic usage:

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
struct MyStruct {
    id: i32,
}
````

The `#[ts(export)]` attribute marks a type for export in the typical “test-time export” workflow.

### Export Lifecycle: Why `cargo test` Matters

A frequently-missed point: **export commonly happens when you run `cargo test`**, not at normal compile time.

That’s not a conceptual limitation of type generation—it’s largely about practical macro/build ergonomics and when you choose to call `export()`.

You can also export manually (see below), but many projects wire it into tests so CI ensures generated TS bindings are always updated.

---

## Quick Start: First Export

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
pub struct User {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}
````

Generated output (typical):

````ts
export type User = { user_id: number, first_name: string, last_name: string, email: string };
````

 > 
 > Version note: modern `ts-rs` emits `type` aliases (not `interface`) by default (see [Version Notes](#version-notes--evolution-important-breaking-changes)).

---

## Type Coverage: What ts-rs Can Represent

`ts-rs` includes implementations for many Rust types out of the box:

* **Primitives**: integers, floats, `bool`, `char`, `()`
* **Strings & paths**: `String`, `str`, `Path`, `PathBuf`, `SmolStr`
* **Collections**: `Vec<T>`, maps/sets, slices/arrays (`[T; N]`), etc.
* **Smart pointers**: `Box`, `Rc`, `Arc`, `Cow`
* **Option/Result**
* **Network types**: `IpAddr`, `SocketAddr`, etc. (and `Url` via feature)
* **Wrappers/locks**: `Mutex`, `RwLock`, `Cell`, `RefCell`, etc.
* **Ranges**: `Range`, `RangeInclusive`

For external crate types (chrono, uuid, etc.), you typically enable the relevant `ts-rs` feature.

---

## Structs Deep Dive

### Field Renaming, Optionality, and Overrides

#### Optional fields (two models)

1. Default `Option<T>` tends to produce `T | null` (depending on config/version behavior).
1. `#[ts(optional_fields)]` makes *all* `Option<T>` fields become *optional properties* (`field?: T`), which is often nicer for idiomatic TS object modeling.

Example:

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export, optional_fields)]
struct UserProfile {
    username: String,
    display_name: Option<String>,
    bio: Option<String>,
}
````

TS:

````ts
export type UserProfile = {
  username: string,
  display_name?: string,
  bio?: string
};
````

You can also control individual fields:

````rust
#[derive(TS)]
#[ts(export)]
struct Example {
    required: String,
    #[ts(optional)]
    maybe: Option<String>,
}
````

#### Rename fields and types

````rust
#[derive(TS)]
#[ts(export, rename = "UserName")]
struct Username {
    #[ts(rename = "id")]
    user_id: i32,
}
````

#### Override a field’s TS type

````rust
use chrono::{DateTime, Utc};
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
struct Event {
    id: String,
    #[ts(type = "string")]
    created_at: DateTime<Utc>,
}
````

#### “Generate as if it were another Rust type” (`#[ts(as = "...")]`)

````rust
use std::collections::HashSet;
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
struct Tags {
    #[ts(as = "Vec<String>")]
    tags: HashSet<String>,
}
````

### Inlining and Flattening

#### Flattening (merging nested object fields)

````rust
use ts_rs::TS;

#[derive(TS)]
struct Address {
    street: String,
    city: String,
}

#[derive(TS)]
#[ts(export)]
struct Person {
    name: String,
    #[ts(flatten)]
    address: Address,
}
````

TS becomes a single object type containing `street`, `city` directly.

#### Inlining (embed type expression instead of referencing a named type)

````rust
#[derive(TS)]
#[ts(export)]
struct Outer {
    #[ts(inline)]
    inner: Inner,
}

#[derive(TS)]
struct Inner {
    x: i32,
}
````

Inlining can reduce imports but may affect doc propagation (see [Docs caveat](#documentation-comments-rust--jsdoc-is-automatic)).

---

## Enums Deep Dive (Rust ADTs → TypeScript Unions)

### Unit Enums → String-Literal Unions (Default)

A key behavior: **a Rust enum with only unit variants** becomes a **TypeScript string-literal union** by default.

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
enum Status {
    Pending,
    InProgress,
    Completed,
}
````

TS:

````ts
export type Status = "Pending" | "InProgress" | "Completed";
````

Use `rename_all` to control casing:

````rust
#[derive(TS)]
#[ts(export, rename_all = "camelCase")]
enum UserRole {
    AdminUser,
    Guest,
}
````

TS:

````ts
export type UserRole = "adminUser" | "guest";
````

### Data-Carrying Enums → Union Shapes

For mixed variants, ts-rs generates unions that match common serde representations.

````rust
#[derive(ts_rs::TS)]
#[ts(export)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}
````

A typical TS output shape is a union combining literals/objects.

### Discriminated Unions with `tag` / `content`

You can produce idiomatic discriminated unions (great for frontend pattern matching):

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export, tag = "type")]
enum ApiResponse<T> {
    Success { data: T },
    Error { message: String },
}
````

TS:

````ts
export type ApiResponse<T> =
  | { type: "Success", data: T }
  | { type: "Error", message: string };
````

You can also use `content = "data"` for adjacently-tagged shapes, and `untagged` to emulate serde untagged enums.

### Emitting a TypeScript `enum` with `repr(enum)`

If you want a TS `enum` object instead of unions, opt in:

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export, rename_all = "UPPERCASE")]
#[ts(repr(enum))]
enum Role {
    Admin,
    User,
    Guest,
}
````

TS:

````ts
export enum Role {
  Admin = "ADMIN",
  User = "USER",
  Guest = "GUEST"
}
````

If your goal is a string-literal union, **do not** use `repr(enum)`.

---

## Generics and Concretization

`ts-rs` supports generics:

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export)]
struct PaginatedResponse<T> {
    items: Vec<T>,
    total: u64,
}
````

TS:

````ts
export type PaginatedResponse<T> = { items: Array<T>, total: number };
````

### Concretizing generics for export (`#[ts(concrete(...))]`)

Sometimes you want to export a type with generics “fixed” to specific concrete types:

````rust
use ts_rs::TS;

#[derive(TS)]
#[ts(export, concrete(T = String))]
struct Wrapper<T> {
    value: T,
}
````

### Custom bounds (`#[ts(bound = "...")]`)

Complex generic bounds can require explicit TS trait bounds:

````rust
#[derive(TS)]
#[ts(export, concrete(T = String), bound = "T: TS")]
struct GenericStruct<T> {
    value: T,
}
````

---

## Serde Compatibility (and What It Does *Not* Do)

### Serde integration (default feature)

`ts-rs` is designed to **respect serde attributes** so the generated TS matches real serialized JSON.

Example:

````rust
use serde::{Serialize, Deserialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct User {
    pub id: u32,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}
````

This alignment is the main reason `ts-rs` is frequently paired with serde.

### Important limitations / caveats

* Not all serde attributes are supported; unsupported ones may trigger warnings.
  * You can suppress warnings via feature `no-serde-warnings`.
* A practical, recurring rule: **if you need to skip something in TS output, prefer `#[ts(skip)]`** rather than relying on serde skip behavior.
  * `skip_deserializing` is ignored (per gotchas noted in research).
  * `skip_serializing` only works in some cases (notably with `default`)—but ts-rs-specific skip is still recommended.

---

## Export Configuration: Paths, Environment Variables, and ESM Imports

### Export directory

You can control where exports land:

````bash
export TS_RS_EXPORT_DIR="./frontend/src/types"
````

Or set via `.cargo/config.toml`:

````toml
[env]
TS_RS_EXPORT_DIR = { value = "frontend/src/types", relative = true }
````

### Import extension for ESM

If your generated TS includes imports (depending on your structure), you may need JS extensions:

````bash
export TS_RS_IMPORT_EXTENSION="js"  # or "mjs"
````

### Per-type export path

````rust
#[derive(ts_rs::TS)]
#[ts(export, export_to = "api/types.ts")]
struct MyType {
  id: i32
}
````

 > 
 > Version-specific note: path semantics changed in v8.0.0—`export_to` is interpreted relative to `TS_RS_EXPORT_DIR` (default `./bindings/`). Validate your paths when upgrading.

---

## Programmatic Usage & Introspection

You can generate declarations as strings (useful for tooling, testing, or embedding):

````rust
use ts_rs::TS;

fn main() {
    println!("{}", User::decl());
    println!("inline: {}", User::inline());
    println!("name: {}", User::name());

    for dep in User::dependencies() {
        println!("dep: {:?}", dep);
    }
}
````

Manual export without `#[ts(export)]`:

````rust
#[cfg(test)]
mod tests {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_types() {
        User::export_all_to("./frontend/src/types").unwrap();
        Status::export_all_to("./frontend/src/types").unwrap();
    }
}
````

---

## Gotchas, Limitations, and Sharp Edges

### 1) Tagged enums + tuple/newtype variants can produce invalid/undesired TS

Internally tagged enums (`#[ts(tag = "...")]`) work best with **struct variants**:

````rust
// Avoid (tuple/newtype variant in tagged enum):
#[derive(ts_rs::TS)]
#[ts(export, tag = "type")]
enum BadEnum {
    Success(String),
}

// Prefer:
#[derive(ts_rs::TS)]
#[ts(export, tag = "type")]
enum GoodEnum {
    Success { message: String },
}
````

### 2) Associated types cannot be expressed as generic TS associated types

TypeScript does not have Rust-like associated types. Workarounds usually involve concretization.

### 3) Export happens “at test time” in many workflows

If your team rarely runs tests locally, generated TS can lag. Typically you:

* enforce generation in CI
* add a dedicated “bindings generation” test
* or run export in a custom step (still using Rust code to call `export_all_to`)

### 4) Large integers and precision

By default, large Rust integers may map to TS `number`, which can lose precision for 64-bit+ values.

There is an environment-based strategy:

````bash
export TS_RS_LARGE_INT=bigint
````

Then:

````rust
#[derive(ts_rs::TS)]
struct BigNumbers {
    large_id: i64, // becomes bigint when configured
}
````

### 5) Directory creation / permissions

The export directory must exist or be creatable. Ensure `TS_RS_EXPORT_DIR` points somewhere writable in dev and CI.

### 6) Serde attribute warnings and unsupported serde features

* Use feature `no-serde-warnings` if your serde usage is more complex than what ts-rs models and warnings are noisy.
* Prefer explicit ts-rs attributes (`#[ts(skip)]`, `#[ts(rename)]`, etc.) for TS-specific intent.

### 7) Re-exporting / custom macro paths

If `ts-rs` is re-exported from another path/crate, point the derive to the correct crate path:

````rust
#[derive(ts_rs::TS)]
#[ts(crate = "my_crate::ts_rs")]
struct MyType {
    id: i32,
}
````

---

## Version Notes & Evolution (Important Breaking Changes)

**Current version line referenced in research:** 11.x (latest noted: 11.1.0).

The most detailed breaking-change record in the provided research is **v8.0.0**, which included significant shifts:

### v8.0.0 highlights (not exhaustive)

* **Exports changed from `interface` → `type`** aliases.
  * This can impact TS patterns that rely on interface declaration merging.
* `#[ts(export)]` began **automatically exporting dependencies** (major ergonomics win).
* Substantial trait surface evolution (new generics/dependency-related methods; removal of `transparent()`).
* Export APIs reshaped (new export helpers).
* Output path semantics changed: `export_to` relative to `TS_RS_EXPORT_DIR`.

### v8.1.0 highlights

* `#[ts(crate = "..")]` support (macro crate override / re-exports)
* `serde_json` types behind feature
* map representation changes (string output differences for maps)

### v10.1.0 highlight (from research)

* Adds feature-gated **Tokio support** (`tokio-impl`)

### v9/v10/v11 major bumps

The research dataset includes version/date but not full per-release details for these majors. Practically:

* treat major bumps as potentially output-breaking,
* regenerate bindings,
* and run TS type-check in CI to catch output changes.

---

## Ecosystem & Alternatives

### Common integration partners

* **Serde**: primary alignment target; ts-rs tries to match real serialized shapes
* **Chrono** (feature-gated): date/time types → TS `string` (commonly ISO-8601)
* **Uuid** (feature-gated): UUID → TS `string`
* plus many other crate integrations via features

### Alternatives (when ts-rs isn’t the best fit)

* **Specta**: often considered the “successor” for large projects; strong type collection & multi-target export options (TS/OpenAPI/JSDoc). Great for deeply nested types and larger systems.
* **Typeshare (1Password)**: CLI-based; also exports Swift/Kotlin; good for multi-platform teams.
* **Tsify**: WASM-focused, integrates directly with `wasm-bindgen` output `.d.ts`.
* **Schemars** (+ json-schema-to-typescript): if you want JSON Schema as the canonical artifact (validation + multi-tooling), at the cost of verbosity and an extra conversion step.

---

## When to Use ts-rs (and When Not To)

### Strong fits

* Rust backend + TS frontend where **shared DTOs** matter
* Full-stack Rust architectures (Axum/Actix/Rocket + React/Vue/Svelte)
* Tauri apps (Rust commands ↔ TS invoke payloads)
* Shared Rust libraries distributed to TS consumers
* WASM projects needing TS shape declarations (ts-rs for types + another crate for conversion)

### Consider alternatives if…

* You need runtime/dynamic type generation
* You have very complex generics that TS cannot express cleanly
* You don’t use serde at all (ts-rs is strongly serde-aligned)
* Your canonical types are defined “frontend-first” in TypeScript
* You want multi-language outputs (Swift/Kotlin) → Typeshare may be better
* You want a broader schema system (OpenAPI/JSON Schema) → Specta/Schemars workflows may fit better

---

## Documentation Comments: Rust → JSDoc Is Automatic

`ts-rs` preserves Rust doc comments (`///` and `#[doc = ...]`) and emits them into generated TS as **JSDoc** comments. This is automatic when using `#[derive(TS)]`.

Example:

````rust
use ts_rs::TS;

/// Represents a user shown in the UI.
#[derive(TS)]
#[ts(export)]
pub struct User {
    /// Unique identifier.
    pub id: i32,

    /// Optional biography shown on profile pages.
    #[ts(optional)]
    pub bio: Option<String>,
}
````

This becomes JSDoc in TS output.

Caveat: normal `//` comments are not docs and will not export. Also, aggressive inlining can affect where comments appear.

---

## License

`ts-rs` is licensed under the **MIT License**, suitable for commercial and open-source use with the standard requirement to retain the license notice.

---

### Appendix: Practical “Project Pattern” for Reliable Generation

A common production setup:

1. Keep exported Rust DTOs in a dedicated module (avoid leaking internal-only fields).
1. Add a single test that exports all root types to your frontend directory.
1. Run `cargo test` in CI and fail the build if generated files differ (or commit generated files and verify clean git diff).
1. Run `tsc --noEmit` (or your frontend build) to catch output format changes on upgrades.

If you want, I can provide a “recommended repository layout” and CI snippets (GitHub Actions) tailored to monorepo vs split-repo setups.