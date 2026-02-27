# Better Use Of `biscuit-file`

## Goal

This review looks at packages that already depend on `biscuit-file`, but still mostly use its crate re-exports (`serde_yaml_ng`, `toml_crate`, `YamlParseError`, `TomlDeError`) instead of the package's own wrapper types and higher-level API.

The point is not to remove the re-exports. They are still useful as a migration bridge and will remain the right tool in some low-level cases. The goal here is to identify where the current wrappers are already good enough, where callers are clearly asking for more core API, and which additions would give the monorepo the best long-term leverage.

## Migration Scope

The migration set for this review is:

| Package | Status |
|--------|--------|
| `model-citizen` | Migrated (`TOML`) |
| `sniff` | Migrated (`TOML`, `YAML`) |
| `claudine` | Migrated (`YAML`) |
| `darkmatter` | Migrated (`YAML`) |
| `biscuit-terminal` | Removed unused dependency |
| `schematic/gen` | Migrated (`TOML`, `YAML`) |
| `schematic/define` | Migrated (`YAML`) |
| `unchained-ai` | Removed unused dependency |
| `research` | Migrated (`YAML`) |

`biscuit-terminal` and `unchained-ai` are out of scope for API recommendations because they no longer consume `biscuit-file`.

## Summary

The clearest pattern is this:

1. `Toml`, `Yaml`, and `Json5` are already useful for "parse and convert" flows.
2. Several packages still fall back to re-exports because the wrapper API stops one step too early.
3. The biggest missing pieces are:
   - typed deserialization helpers
   - serialization helpers from typed data or `serde_json::Value`
   - a library-level Markdown frontmatter API
   - lightweight document/path accessors for common inspection use-cases

If those four areas are covered, most current re-export usage in `sniff`, `darkmatter`, `claudine`, `research`, and `schematic` could move to the core `biscuit-file` surface naturally.

## What Is Already Working Well

### `model-citizen`

`model-citizen/lib/src/config.rs:161-164` already uses `Toml::new(path)?` before deserializing into `Config`. This is the right direction:

```rust
let toml = Toml::new(path)?;
let config: Self = serde::Deserialize::deserialize(toml.value().clone())?;
```

That is a good `biscuit-file` usage pattern today. The only issue is that callers still have to drop down to raw `toml::Value` and clone it in order to finish the job.

### `darkmatter` CLI

`darkmatter/cli/src/main.rs:348-388` already leans on `Json5` for output formatting and cross-format conversion. That is also directionally correct. The main limitation is that the wrapper only accepts string/bytes inputs, so `serde_json::Value` must first be serialized to JSON and then parsed back into `Json5`.

## Highest-Value API Additions

These are the best investments if the goal is to make `biscuit-file` the preferred API rather than just a crate aggregator.

### 1. Typed deserialization on wrapper types

This is the single most useful improvement.

Suggested shape:

```rust
impl Toml {
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, TomlError>;
}

impl Yaml {
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, YamlError>;
}

impl Json5 {
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, Json5Error>;
}
```

Why this matters:

- `model-citizen` can replace `deserialize(toml.value().clone())` with `toml.deserialize::<Config>()`.
- `research` can parse frontmatter into `SkillFrontmatter` and `ChangelogFrontmatter` without directly calling `serde_yaml_ng::from_str`.
- `schematic` can parse YAML OpenAPI documents into `openapiv3::OpenAPI` through `Yaml`.
- Callers stop depending on `TomlDeError` and `YamlParseError` directly.

Related cleanup:

- `model-citizen/lib/src/error.rs:57-64`
- `darkmatter/lib/src/markdown/types.rs:3-15`
- `claudine/lib/src/error.rs:1-21`
- `research/lib/src/validation/frontmatter.rs:7-40`

Once typed methods exist, those packages should prefer `TomlError` / `YamlError` or `BiscuitFileError` instead of the re-exported dependency error types.

### 2. A library-level Markdown frontmatter module

This is the biggest repeated gap in the repo.

There are at least five separate frontmatter implementations today:

- `biscuit-file/cli/src/main.rs:186-215`
- `darkmatter/lib/src/markdown/frontmatter.rs:170-208`
- `research/lib/src/validation/frontmatter.rs:101-239`
- `sniff/lib/src/filesystem/docs.rs:163-208`
- `claudine/lib/src/linking/compatibility.rs:162-244`

Those implementations all do some variation of:

- detect YAML or TOML frontmatter delimiters
- split body from frontmatter
- parse typed data or mappings
- serialize frontmatter back to Markdown

Suggested shape:

```rust
pub mod markdown {
    pub enum FrontmatterFormat {
        Yaml,
        Toml,
    }

    pub struct FrontmatterBlock {
        pub format: FrontmatterFormat,
        pub raw: String,
        pub body: String,
    }

    pub fn extract_frontmatter(input: &str) -> Result<Option<FrontmatterBlock>, FrontmatterError>;
    pub fn render_frontmatter<T: serde::Serialize>(
        body: &str,
        format: FrontmatterFormat,
        value: &T,
    ) -> Result<String, BiscuitFileError>;
}
```

Useful follow-on helpers:

```rust
impl FrontmatterBlock {
    pub fn parse_yaml<T: serde::de::DeserializeOwned>(&self) -> Result<T, YamlError>;
    pub fn parse_toml<T: serde::de::DeserializeOwned>(&self) -> Result<T, TomlError>;
}
```

Why this matters:

- `sniff` can drop its manual YAML extraction.
- `darkmatter` can use a shared parser/renderer.
- `research` can focus on validation, not delimiter handling.
- `claudine` can keep its lenient fallback behavior as an optional mode layered on top of a shared extractor.
- the CLI's frontmatter support moves into the library where other packages can use it.

### 3. Serialization helpers from typed data or `serde_json::Value`

Today several callers have to build `serde_yaml_ng::Mapping` by hand or round-trip through JSON strings.

Examples:

- `darkmatter/lib/src/markdown/output/string.rs:24-45`
- `darkmatter/lib/src/markdown/toc/mod.rs:317-324`
- `claudine/lib/src/linking/execution.rs:511-557`
- `schematic/define/src/openapi/options.rs:79-89`

Suggested additions:

```rust
impl Yaml {
    pub fn from_serializable<T: serde::Serialize>(value: &T) -> Result<Self, YamlError>;
    pub fn to_yaml_string(&self) -> Result<String, YamlError>;
}

impl Toml {
    pub fn from_serializable<T: serde::Serialize>(value: &T) -> Result<Self, TomlError>;
}

impl Json5 {
    pub fn from_json_value(value: serde_json::Value) -> Self;
}
```

Potential standalone helpers:

```rust
pub fn yaml_string_from<T: serde::Serialize>(value: &T) -> Result<String, YamlError>;
pub fn toml_string_from<T: serde::Serialize>(value: &T) -> Result<String, TomlError>;
```

Why this matters:

- `darkmatter` could serialize frontmatter without directly importing `serde_yaml_ng`.
- `claudine` could render derived YAML from regular Rust maps instead of building `serde_yaml_ng::Mapping` manually.
- `darkmatter` CLI could avoid `serde_json::Value -> String -> Json5::from_str(...)`.
- `schematic` YAML serialization becomes a `biscuit-file` concern instead of a re-export concern.

### 4. Lightweight path/query helpers for inspection-heavy callers

`sniff/lib/src/filesystem/repo.rs` uses `toml_crate::Value` as a dynamic tree for repeated point lookups:

- `CargoLockVersions::parse`: `repo.rs:202-221`
- workspace parsing: `repo.rs:288-316`
- package metadata reads: `repo.rs:1030-1069`

That is a valid use of raw TOML values, but it shows a missing ergonomic layer for "read a config and inspect a few paths".

Suggested shape:

```rust
impl Toml {
    pub fn get_path<'a>(&'a self, path: &[&str]) -> Option<&'a toml::Value>;
    pub fn get_str(&self, path: &[&str]) -> Option<&str>;
    pub fn get_array(&self, path: &[&str]) -> Option<&Vec<toml::Value>>;
    pub fn get_table(&self, path: &[&str]) -> Option<&toml::map::Map<String, toml::Value>>;
}

impl Yaml {
    pub fn get_path<'a>(&'a self, path: &[&str]) -> Option<&'a serde_yaml_ng::Value>;
}
```

This is lower priority than typed deserialization and frontmatter, but it would make `sniff` a much more natural consumer of `Toml` and `Yaml`.

## Package-By-Package Recommendations

### `model-citizen`

Current usage:

- Good use of `Toml` in `model-citizen/lib/src/config.rs:161-164`
- Still depends on `TomlDeError` in `model-citizen/lib/src/error.rs:57-60`

Recommendation:

- Add `Toml::deserialize<T>()`.
- After that, prefer:

```rust
let config: Config = Toml::new(path)?.deserialize()?;
```

- Once that exists, the `TomlDeError` re-export should no longer be needed here.

### `sniff`

Current usage:

- Uses `toml_crate::Value` heavily for Cargo and Python metadata inspection:
  - `sniff/lib/src/filesystem/repo.rs:202-221`
  - `sniff/lib/src/filesystem/repo.rs:288-316`
  - `sniff/lib/src/filesystem/repo.rs:1030-1069`
- Uses raw `serde_yaml_ng::Value` for `pnpm-workspace.yaml`:
  - `sniff/lib/src/filesystem/repo.rs:927-939`
- Reimplements Markdown frontmatter parsing:
  - `sniff/lib/src/filesystem/docs.rs:163-208`

Recommendation:

- Short term: keep re-exports for inspection-heavy TOML code if that is simplest.
- Medium term: add `Toml`/`Yaml` path accessors for config inspection.
- High value: move Markdown frontmatter extraction to `biscuit-file` and use it from `sniff`.

This is the package that most clearly benefits from both "inspection helpers" and a shared frontmatter API.

### `darkmatter`

Current usage:

- Frontmatter parsing uses direct YAML parsing:
  - `darkmatter/lib/src/markdown/frontmatter.rs:173-208`
- Frontmatter serialization uses direct `serde_yaml_ng::to_string`:
  - `darkmatter/lib/src/markdown/output/string.rs:24-45`
- TOC hashing also serializes frontmatter through the re-export:
  - `darkmatter/lib/src/markdown/toc/mod.rs:317-324`
- CLI conversion uses `Json5`, but only after reparsing JSON strings:
  - `darkmatter/cli/src/main.rs:348-388`

Recommendation:

- Add a frontmatter library API to `biscuit-file`; `darkmatter` should consume that instead of owning its own parser.
- Add `Yaml::from_serializable` + `Yaml::to_string` so frontmatter rendering stays inside `biscuit-file`.
- Add `Json5::from_json_value` so the CLI can skip the JSON string round-trip.

`darkmatter` is one of the best targets for making `biscuit-file` feel like a real document utility library, not just format wrappers.

### `claudine`

Current usage:

- Owns a full Markdown/frontmatter parser plus permissive fallback:
  - `claudine/lib/src/linking/compatibility.rs:162-244`
- Manually constructs and serializes YAML mappings:
  - `claudine/lib/src/linking/execution.rs:511-557`
- Reads YAML hashes by parsing raw YAML values:
  - `claudine/lib/src/linking/execution.rs:598-614`
- Still exposes re-exported parse errors:
  - `claudine/lib/src/error.rs:1-21`

Recommendation:

- `claudine` should remain free to keep its lenient compatibility logic, but delimiter handling and baseline parse/render flows should come from `biscuit-file`.
- Add a strict/lenient frontmatter parsing mode in `biscuit-file` if we want to absorb more of Claudine's behavior.
- Add `Yaml::from_serializable`, `Yaml::deserialize<T>`, and `Yaml::to_yaml_string`.
- Prefer wrapper-level errors over `YamlParseError`.

`claudine` is the strongest argument for a configurable frontmatter API rather than a one-size-fits-all helper.

### `research`

Current usage:

- Reimplements frontmatter extraction and typed YAML parsing:
  - `research/lib/src/validation/frontmatter.rs:101-239`
- Uses `serde_yaml_ng` and `YamlParseError` directly:
  - `research/lib/src/validation/frontmatter.rs:7-40`

Recommendation:

- Move delimiter extraction into `biscuit-file`.
- Use `Yaml::deserialize<T>()` for `SkillFrontmatter` and `ChangelogFrontmatter`.
- Prefer `YamlError` or a dedicated `FrontmatterError` from `biscuit-file` rather than the raw parser error re-export.

`research` is a very clean consumer candidate once `biscuit-file` grows a typed frontmatter path.

### `schematic`

Current usage:

- Parses OpenAPI YAML with `serde_yaml_ng::from_str`:
  - `schematic/define/src/openapi/import.rs:275-308`
- Serializes OpenAPI YAML with `serde_yaml_ng::to_string`:
  - `schematic/define/src/openapi/options.rs:79-89`
- Test code also validates YAML via the re-export:
  - `schematic/gen/src/openapi_output.rs:91-160`

Recommendation:

- Add `Yaml::deserialize<T>()` and `Yaml::from_serializable` / `Yaml::to_yaml_string`.
- Production code in `schematic/define` should prefer those wrapper methods.
- Test-only validation through re-exports is still fine; this is lower priority than production paths.

`schematic` does not need a frontmatter API, but it would benefit immediately from typed YAML read/write helpers.

## Recommended Implementation Order

If this work is done incrementally, this order gives the best return:

1. Add `deserialize<T>()` to `Toml`, `Yaml`, and `Json5`.
2. Add `from_serializable(...)` and `to_string()` helpers for `Yaml` and `Toml`.
3. Move Markdown frontmatter extraction/rendering from the CLI into a library module.
4. Add optional strict/lenient frontmatter parsing policies.
5. Add TOML/YAML path accessors for inspection-heavy callers like `sniff`.

## What Should Probably Stay As Re-Exports

Even after those improvements, some re-export usage will still be reasonable:

- test-only syntax validation
- very low-level `Value` manipulation
- cases where the underlying crate type is the real domain model

The main point is that most production code paths should not need to reach for the re-export just to finish common tasks like:

- parse file -> typed struct
- typed struct -> YAML/TOML string
- extract frontmatter -> typed metadata + body
- inspect a few keys in a TOML/YAML document

## Bottom Line

The repo is already showing what the optimal `biscuit-file` API should be.

The strongest next step is not removing re-exports. It is making the wrapper surface complete enough that callers no longer need the re-exports for ordinary work. The most important additions are typed deserialization, typed serialization, and a shared frontmatter module. Once those exist, `model-citizen`, `sniff`, `darkmatter`, `claudine`, `research`, and `schematic` can all use `biscuit-file` more directly without losing flexibility.
