---
prompt: |-
    We are considering the use of the 'frontmatter-gen' crate to be a part of a LSP implementation for Darkmatter. Currently we use `serde-yaml-ng`. Your task is to research both and then:
    
    - list out each's functional footprint, 
    - all exported feature flags and what functionality is attached to each flag
    - strengths and weaknesses
    - when the package is a "good fit"
    - when the package is a "bad fit"
last_updated: 2026-05-02
---
# Frontmatter Parser Research: `frontmatter-gen` vs `serde_yaml_ng`

## Contents

- Summary
- serdeyamlng
- frontmatter-gen
- LSP-Specific Recommendation
- Decision Matrix

Use heading search to jump to the listed subsystem.


This compares `frontmatter-gen` `0.0.5` and `serde_yaml_ng` `0.10.0` as candidates for Darkmatter frontmatter handling in an LSP implementation.

Sources checked:

- [`frontmatter-gen` crate docs](https://docs.rs/frontmatter-gen/latest/frontmatter_gen/)
- [`frontmatter-gen` feature flags](https://docs.rs/crate/frontmatter-gen/latest/features)
- [`frontmatter-gen` `Cargo.toml.orig`](https://docs.rs/crate/frontmatter-gen/latest/source/Cargo.toml.orig)
- [`serde_yaml_ng` crate docs](https://docs.rs/serde_yaml_ng/latest/serde_yaml_ng/)
- [`serde_yaml_ng` feature flags](https://docs.rs/crate/serde_yaml_ng/latest/features)
- [`serde_yaml_ng` `Cargo.toml.orig`](https://docs.rs/crate/serde_yaml_ng/latest/source/Cargo.toml.orig)

## Summary

`serde_yaml_ng` is the better fit for Darkmatter’s LSP needs if the goal is accurate YAML parsing, Serde integration, and diagnostics with line/column locations. It is a YAML data-format crate, not a frontmatter extractor, so Darkmatter must continue owning delimiter detection, byte ranges, body slicing, fallback behavior, and LSP range mapping.

`frontmatter-gen` is broader but shallower. It bundles frontmatter extraction, multi-format parsing, conversion, validation, CLI support, and optional static-site-generation support. That breadth is useful for generic content tooling, but it is not obviously aligned with a markdown LSP that needs precise source spans, predictable delimiter semantics, and tight integration with Darkmatter’s existing frontmatter model.

## `serde_yaml_ng`

### Functional Footprint

`serde_yaml_ng` is a YAML implementation for Serde.

Its core functionality includes:

- Deserialize YAML from:

    - strings via `from_str`
    - byte slices via `from_slice`
    - readers via `from_reader`

- Serialize Rust values to:

    - strings via `to_string`
    - writers via `to_writer`

- Convert between Rust/Serde values and loosely typed YAML values:

    - `to_value`
    - `from_value`

- Expose a YAML value model:

    - `Value`
    - `Mapping`
    - `Sequence`
    - `Number`
    - tagged values

- Expose streaming-style serializer/deserializer types:

    - `Serializer`
    - `Deserializer`

- Provide YAML parse/serialization errors with optional source location:

    - `Error::location() -> Option<Location>`
    - `Location::index()`
    - `Location::line()`
    - `Location::column()`

- Support YAML 1.1 semantics.
- Support Serde derive workflows for typed structs/enums.

In Darkmatter today, this maps well to:

- parsing YAML between leading `---` delimiters;
- deserializing into Darkmatter’s ordered frontmatter map;
- producing user-facing diagnostics with line and column;
- preserving Darkmatter ownership of markdown/frontmatter boundaries;
- fallback parsing behavior such as leading-tab normalization and expression protection.

### Exported Feature Flags

`serde_yaml_ng` exports no feature flags.

The docs.rs feature page says this release has no feature flags. Its `Cargo.toml.orig` has no `[features]` section.

### Strengths

- Focused dependency: it does YAML and Serde, without frontmatter policy.
- Strong Serde fit: works naturally with Darkmatter’s existing typed accessors and `serde_json`-backed frontmatter model.
- Useful diagnostics: parse errors can expose `Location`, which is important for LSP diagnostics.
- Stable integration shape: Darkmatter can keep its own delimiter scanning, source slicing, fallback behavior, and rendering-specific error blocks.
- Supports YAML tags and YAML mappings through a general `Value` model.
- No feature-flag complexity.

### Weaknesses

- It is not a frontmatter parser. It does not detect `---` delimiters, split markdown body content, or report frontmatter block ranges.
- It does not preserve YAML formatting, comments, exact scalar style, or original key syntax for round-trip editing.
- YAML 1.1 behavior may surprise users expecting YAML 1.2 semantics.
- It is backed by `unsafe-libyaml`, which may matter for projects trying to avoid unsafe dependencies.
- LSP-grade incremental parsing still has to be built outside the crate.

### Good Fit

`serde_yaml_ng` is a good fit when:

- Darkmatter wants to keep frontmatter boundary detection under its own control.
- The LSP needs syntax diagnostics with line/column information.
- Frontmatter is primarily YAML.
- Parsed data should flow into existing Serde-based typed accessors.
- Darkmatter needs predictable ownership of markdown body ranges and frontmatter ranges.
- The implementation can tolerate non-round-tripping parsing.

### Bad Fit

`serde_yaml_ng` is a bad fit when:

- The desired dependency should parse complete markdown documents and split frontmatter/body itself.
- The LSP needs comment-preserving or formatting-preserving YAML edits.
- The LSP needs full YAML AST node spans for keys, values, comments, and nested structures.
- YAML 1.2 compliance is required.
- Unsafe transitive dependencies are unacceptable.

## `frontmatter-gen`

### Functional Footprint

`frontmatter-gen` is a frontmatter-oriented content metadata crate.

Its core functionality includes:

- Extract raw frontmatter from content:

    - YAML-style delimiters: `---`
    - TOML-style delimiters: `+++`
    - JSON object frontmatter via balanced braces

- Auto-detect frontmatter format:

    - YAML
    - TOML
    - JSON

- Parse frontmatter into its own data model:

    - `Frontmatter`
    - `Value`
    - `Format`

- Serialize frontmatter back to:

    - YAML
    - TOML
    - JSON

- Validate input:

    - maximum frontmatter size
    - maximum nesting depth
    - null-byte checks
    - path-traversal pattern checks

- Validate parsed frontmatter structure:

    - maximum nesting depth
    - maximum key count

- Provide a custom error enum covering:

    - parse errors
    - conversion errors
    - extraction errors
    - validation errors
    - format errors
    - nesting/size errors

- Provide optional CLI commands with the `cli` feature.
- Provide optional static-site-generator support with the `ssg` feature.

Internally, `frontmatter-gen` uses separate parsers for different formats:

- YAML through `serde_yml`
- JSON through `serde_json`
- TOML through `toml`

Its value model is intentionally format-neutral:

- `Null`
- `String`
- `Number(f64)`
- `Boolean`
- `Array`
- `Object`
- `Tagged`

### Exported Feature Flags

`frontmatter-gen` exports these feature flags:

| Feature   | Default?                               | Attached Functionality                                                                                                       |
|-----------|---------------------------------------:|------------------------------------------------------------------------------------------------------------------------------|
| `default` | Enabled by Cargo convention, but empty | Enables no additional features. The core library is available without optional features.                                     |
| `cli`     | No                                     | Enables the optional `clap` dependency and the `fmg` binary. The binary is declared with `required-features = ["cli"]`.      |
| `ssg`     | No                                     | Enables `cli` and optional dependencies `tera`, `pulldown-cmark`, `dtt`, and `url` for static-site-generation functionality. |

Important detail: the prose docs describe `default` as “core frontmatter parsing functionality,” but the actual Cargo metadata shows `default = []`. Core parsing is not attached to the `default` feature; it is simply part of the base crate.

### Strengths

- Broader frontmatter scope than `serde_yaml_ng`.
- Supports YAML, TOML, and JSON frontmatter behind one API.
- Provides frontmatter/body extraction out of the box.
- Provides format conversion APIs.
- Has size and nesting validation built in.
- Has a CLI feature if standalone frontmatter manipulation is useful.
- Has a low declared MSRV of Rust `1.56.0`.

### Weaknesses

- Poorer fit for LSP diagnostics. Its public API does not expose precise frontmatter delimiter spans, key spans, value spans, or reliable source ranges for diagnostics.
- Its extraction model is too coarse for a markdown LSP. It returns parsed frontmatter plus remaining content, but not enough range metadata for editor features.
- YAML parsing goes through `serde_yml`, not Darkmatter’s existing `serde_yaml_ng` path.
- Its neutral `Value::Number(f64)` loses integer precision and YAML/TOML numeric detail.
- It uses `HashMap`, so it does not preserve frontmatter key order. Darkmatter explicitly values order preservation for human-facing frontmatter.
- Its extraction heuristics are generic and may not match Darkmatter’s stricter rule that frontmatter must appear at the start of the document.
- It adds unrelated surface area for an LSP: CLI, SSG support, URL/template/markdown rendering dependencies behind features, and its own config/error/value abstractions.
- The validation behavior may be surprising for markdown content. For example, rejecting `../` patterns can be reasonable for some static-site contexts but is not inherently a YAML/frontmatter parse error.
- It does not remove the need for a real YAML parser for semantic validation, completions, schema checks, or diagnostics.

### Good Fit

`frontmatter-gen` is a good fit when:

- The application is a generic content-processing tool.
- YAML, TOML, and JSON frontmatter should be accepted through one simple API.
- Exact source ranges are not important.
- The consumer wants extraction, parsing, validation, and conversion bundled together.
- The caller is building a static-site or content-management workflow.
- The CLI or SSG features are directly useful.

### Bad Fit

`frontmatter-gen` is a bad fit when:

- The consumer is a language server that needs precise source locations.
- Diagnostics must point to exact YAML line/column or document ranges.
- The host application already owns markdown parsing and frontmatter delimiter semantics.
- Key order preservation matters.
- Numeric fidelity matters.
- The project wants one YAML stack instead of adding another YAML parser.
- The project needs to preserve formatting/comments for editor code actions.
- The implementation should avoid broad, unrelated content-tooling abstractions.

## LSP-Specific Recommendation

For Darkmatter’s LSP implementation, `serde_yaml_ng` should remain the semantic YAML parser unless the LSP moves to a dedicated syntax-tree YAML parser for editor-grade spans and round-trip editing.

`frontmatter-gen` should not replace `serde_yaml_ng` for LSP parsing. It solves a higher-level extraction/conversion problem, but Darkmatter already has domain-specific frontmatter extraction and fallback behavior. The LSP needs more precision than `frontmatter-gen` exposes, not less.

A pragmatic architecture is:

1. Keep Darkmatter-owned frontmatter delimiter scanning.
2. Track exact byte ranges for:

    - opening delimiter
    - YAML body
    - closing delimiter
    - markdown body

3. Continue using `serde_yaml_ng` for semantic parsing and diagnostics.
4. Add an LSP-facing wrapper that maps `serde_yaml_ng::Location` from YAML-local line/column into document ranges.
5. Consider a YAML CST/AST parser later if Darkmatter needs formatting-preserving edits, comments, completions, folding, or symbol ranges inside frontmatter.

## Decision Matrix

| Requirement                  | `serde_yaml_ng`                            | `frontmatter-gen`                        |
|------------------------------|-------------------------------------------:|-----------------------------------------:|
| YAML semantic parsing        | Strong                                     | Indirect, through `serde_yml`            |
| Serde integration            | Strong                                     | Limited to its own converted value model |
| Frontmatter/body splitting   | None                                       | Yes                                      |
| Exact source ranges          | Limited error locations only               | Not sufficient                           |
| LSP diagnostics              | Good base                                  | Weak base                                |
| Multi-format frontmatter     | No                                         | Yes: YAML, TOML, JSON                    |
| Key order preservation       | Possible if deserializing into ordered map | No, uses `HashMap`                       |
| Numeric fidelity             | Better than forced `f64` model             | Weaker, uses `f64`                       |
| Format/comment preservation  | No                                         | No                                       |
| Feature simplicity           | Strong, no features                        | Moderate, CLI/SSG features               |
| Darkmatter fit               | Strong                                     | Weak to moderate                         |
| Generic SSG/content-tool fit | Moderate                                   | Strong                                   |
