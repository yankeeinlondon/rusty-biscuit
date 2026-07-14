# Enums, Tagging, and TypeScript Output

## Default behavior (unit enums)

A unit enum becomes a string-literal union by default:

* Rust: `enum Status { Pending, Completed }`
* TS: `type Status = "Pending" | "Completed"`

Use `#[ts(rename_all = "...")]` to control literal casing.

## Data-carrying enums (external tagging by default)

Rust enums with payloads become unions of objects, matching Serde’s typical external tagging:

* Example shape: `| { "Variant": PayloadType }`

## Discriminated unions (recommended for TS ergonomics)

Use `#[ts(tag = "type")]` to create `{ type: "Variant", ... }` patterns.

### Critical gotcha: do NOT use tuple/newtype variants in tagged enums

Problematic:

* `enum E { Ok(String) }` with `#[ts(tag="type")]`

Prefer struct variants:

* `enum E { Ok { value: String } }`

## Generating a TypeScript `enum` (opt-in)

If you want `export enum Role { ... }`, add:

* `#[ts(repr(enum))]`
  Commonly combined with `rename_all` (e.g., `UPPERCASE`).

Use this only when you truly need TS runtime enum objects; otherwise string unions are often better.

---