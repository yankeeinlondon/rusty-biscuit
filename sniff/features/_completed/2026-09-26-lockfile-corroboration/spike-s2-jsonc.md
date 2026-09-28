# Spike S2: JSONC parser for `bun.lock`

Phase 1 spike for `2026-09-26-lockfile-corroboration`. The throwaway harness
lives outside the repository at `/tmp/jsonc-spike/bench` (not committed).
Timings are in [measurements.md](./measurements.md#s2-jsonc-parser-throughput-phase-1-spike).

## Recommendation

Use **`jsonc-parser` 0.33.2** with only its `serde` feature, called through
`jsonc_parser::parse_to_serde_value::<BunLock>` with an explicit strict
`ParseOptions` (comments and trailing commas on, every other extension off).

- It is a **new direct dependency of the `sniff` crate** and a new crate in
  the workspace graph (`jsonc-parser` itself; its only other dependencies,
  `serde` with `derive`, are already present). Phase 5 must update
  `sniff/docs/dependencies.md` and the root `docs/dependencies.md`.
- Suggested manifest line:
  `jsonc-parser = { version = "0.33.2", features = ["serde"] }`. The crate
  has no default features.
- `jsonc-parser`'s defaults are lenient: missing commas, single-quoted strings,
  unquoted keys, hexadecimal numbers, and unary plus are all accepted. Never use
  `Default::default()`. Always construct the options literally:

```rust
jsonc_parser::ParseOptions {
    allow_comments: true,
    allow_trailing_commas: true,
    allow_loose_object_property_names: false,
    allow_missing_commas: false,
    allow_single_quoted_strings: false,
    allow_hexadecimal_numbers: false,
    allow_unary_plus_numbers: false,
}
```

  `ParseOptions` is not `#[non_exhaustive]`, so a literal breaks the build if a
  future release adds a field. Take that as a prompt to review the new field.

### Why not `json-five` (already in the graph)

`json-five` 0.3.1 fails criterion 2. `json_five::from_str` first builds the
complete `JSONValue` AST (`parser::from_str`, then `JSONValueDeserializer`
walks it; see `src/de.rs:762-778`). On the 15.7 MiB input, peak allocation
was 149 MiB (9.5x the input) and it ran about 3.5x slower. Its JSON5 grammar also
cannot be narrowed, so single-quoted strings and unquoted keys are always
accepted.

On the coordinator's two questions:

- **How it would be wired if chosen.** `sniff/lib/Cargo.toml` depends on
  `biscuit-file` without `default-features = false`, so `json-five` 0.3.1 is
  already compiled into sniff (confirmed with `cargo tree -p sniff`), but
  `biscuit-file` does not re-export it. The better wiring would be (b): add
  `pub use json_five;` to `biscuit-file` under `#[cfg(feature = "json5")]`,
  matching the `toml_crate` and `serde_yaml_ng` re-exports, and have sniff name
  `"json5"` in its explicit `biscuit-file` feature list instead of relying on
  defaults. That touches `biscuit-file`'s lib and README/skill re-export list,
  but no dependency docs, because no crate is added. Option (a), a direct
  `json-five` line in sniff, adds a `sniff/docs/dependencies.md` entry and
  allows version skew. **Neither is recommended**, because the AST cost is the
  problem.
- **Does JSON5 permissiveness matter?** It matters only a little. Bun never
  writes those forms. Bun 1.3.3's own reader is also looser than strict JSONC:
  `bun install --frozen-lockfile` accepted a `bun.lock` whose first key was
  `'lockfileVersion'` (single-quoted), and it accepted `//` and `/* */`
  comments. It rejected a missing comma ("Ignoring lockfile"). The risk
  is that a malformed, hand-edited file gets reported as corroborated rather
  than as a parse failure. The strict `jsonc-parser` options above reject those
  forms. Relaxing `allow_single_quoted_strings` to match Bun exactly is an
  open author decision; the spec's "permitted comments and trailing commas"
  reads as strict.

### Reference: `serde_json` after `json-strip-comments`

`json-strip-comments` 3.1.2 (oxc project) blanks comments and trailing commas in
place, then strict `serde_json` does a streaming typed parse. It is almost as
fast, but it needs an owned mutable copy of the input (1.0x input peak). It
also **silently accepts an unterminated `/* ...` block comment**, which fails
criterion 1's intent. That would be two new crates (`json-strip-comments`,
already-present `memchr`), with no advantage over `jsonc-parser`.
`json_comments` 0.2.2 was discarded: it strips comments but not trailing
commas.

## Criteria

All cases ran through the typed partial struct below with
`#[serde(default)] packages: IgnoredAny`. The raw output is at
`/tmp/jsonc-spike/criteria-serde228.txt`, built against the workspace's
`serde` 1.0.228 and `serde_json` 1.0.149.

| # | Criterion | `jsonc-parser` 0.33.2 (strict opts) | `json-five` 0.3.1 | `json-strip-comments` 3.1.2 + `serde_json` |
|---|-----------|------------------------------------|-------------------|--------------------------------------------|
| 1 | `//`, `/* */`, and trailing commas in objects and arrays; real Bun 1.3.3 `bun.lock` | **Pass** | **Pass** | **Partial**: accepts all of them, but also accepts an unterminated `/*` comment |
| 1b | Rejects non-JSONC extensions (missing comma, single quotes, unquoted key, `,,`) | **Pass** (default opts accept missing comma, single quotes, and unquoted key) | **Fail**: accepts single quotes and unquoted keys | **Pass** |
| 2 | Typed serde partial struct plus `IgnoredAny` with no whole-tree value | **Pass**: `parse_to_serde_value` drives `impl Deserializer for &mut JsoncParser` straight from the scanner (`src/serde.rs:61-90`, `ScannerMapAccess`); keys are `Cow<'de, str>`. Peak allocation is 0.03 MiB and does not grow with input size. | **Fail**: builds the full `JSONValue` AST first (`src/de.rs:762-778`); peak is 7-9.5x the input | **Pass** for the parse; the strip needs a 1.0x owned copy |
| 3 | Rejects trailing garbage (`}}} x`, and a second `{}` document) | **Pass**: `MultipleRootJsonValues` ("Text cannot contain more than one JSON value") | **Pass** | **Pass**: "trailing characters" |
| 4 | Duplicate `workspaces` key | Plain `BTreeMap` field: **last-wins, silently** (serde's map impl, the same for all three). Custom `Visitor`: **detectable**, with the error at line and column. Duplicate struct field `lockfileVersion`: **error** (serde derive). | Same last-wins behavior; a custom `Visitor` detects duplicates, with no position | Same last-wins behavior; a custom `Visitor` detects duplicates, with the position |
| 5 | Release throughput on a 15.71 MiB input (200 workspaces, 50,000 packages, comments, trailing commas), median of 7 runs | **22.6 ms, about 695 MiB/s, peak 0.03 MiB, 633 allocations** | 78.3 ms, about 201 MiB/s, peak 149 MiB, 3.67 M allocations | 24.2 ms, about 650 MiB/s, peak 15.7 MiB, 635 allocations |
| 6a | Edition and MSRV | Edition 2024, no `rust-version` (so at least 1.85); compiles on 1.98.1 | Edition 2024, no `rust-version` | Edition 2024, `rust-version` 1.85.0 |
| 6b | License (sniff is AGPL-3.0-only) | MIT: compatible | MIT: compatible | Apache-2.0 from 3.1.2 (MIT before): compatible |
| 6c | Maintenance | dprint project; 0.33.2 released 2026-09-12, with 5 releases in 2026; about 10.8 M downloads, 3.2 M recent | Single maintainer; 0.3.1 released 2026-01-08; about 1.2 M downloads | oxc project; 3.1.2 released 2026-07-27; about 5.5 M downloads |
| 6d | Dependency footprint (`cargo tree -p X -e normal`, unique crates including itself) | 8 (`serde`, `serde_core`, `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`): **only `jsonc-parser` is new** | 9 (adds `unicode-general-category`): already in the graph | 2 (`memchr`) plus the existing `serde_json` |
| - | Nesting-depth guard (100,000 nested `[` inside `packages`) | Error: `NestingDepthExceeded`, a fixed limit of 512 | Error: depth 3000 | `serde_json`'s `IgnoredAny` path skipped it without error |

Notes:

- The real `bun.lock` came from `bun install` (Bun 1.3.3) in
  `/tmp/jsonc-spike/bunws`: a root plus `packages/a` and `packages/b`, with
  `packages/a` depending on `@x/b` as `workspace:*` and the root depending on
  `is-odd`. Its top-level keys are `lockfileVersion: 1`, `configVersion: 1`,
  `workspaces`, and `packages`. Workspace keys are `""`, `packages/a`, and
  `packages/b`, and each value carries `name`, an optional `version`, and
  dependency maps. Bun writes trailing commas but no comments.
- The 512 nesting limit is far above `bun.lock`'s real depth of about 5.
- `jsonc-parser` treats empty input as `null`. The typed struct then fails with
  "invalid type: unit value", which is an error, as it should be.

## Typed partial struct sketch

```rust
use std::collections::BTreeMap;
use serde::Deserialize;
use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};

#[derive(Deserialize)]
struct BunLock {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: u32,
    #[serde(default, deserialize_with = "unique_workspaces")]
    workspaces: BTreeMap<String, BunWorkspace>,
    // Explicit only to document intent: serde derive already skips unknown
    // fields through IgnoredAny, so `packages` is scanned and never stored.
    #[serde(default)]
    packages: IgnoredAny,
}

/// Dependency maps are skipped as unknown fields.
#[derive(Deserialize)]
struct BunWorkspace {
    name: Option<String>,
    version: Option<String>,
}

/// serde's `BTreeMap` impl is last-wins; a duplicate workspace path must be an
/// error instead.
fn unique_workspaces<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, BunWorkspace>, D::Error> {
    struct UniqueKeys;
    impl<'de> Visitor<'de> for UniqueKeys {
        type Value = BTreeMap<String, BunWorkspace>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map of workspace paths")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut out = BTreeMap::new();
            while let Some(path) = map.next_key::<String>()? {
                let record = map.next_value()?;
                if out.insert(path.clone(), record).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate workspace key `{path}`"
                    )));
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(UniqueKeys)
}
```

Version gating (the accepted `lockfileVersion` values) belongs after this
parse. `lockfileVersion` is a required `u32`, so a string or a missing value
is a parse error, not an "unsupported version".
