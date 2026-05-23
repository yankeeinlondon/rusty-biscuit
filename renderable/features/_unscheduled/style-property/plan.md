# Style Frontmatter Schema & Parser — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship sub-spec #1 of `style:` frontmatter — a typed schema and parser
under `darkmatter::style` that lowers user YAML into `renderable::{layout,color}`
runtime types, with discriminated warnings for unknown / deprecated / not-yet-
wired keys. No rendering changes.

**Architecture:** New top-level module `darkmatter/lib/src/style/`. A two-pass
parser: (1) a canonicalization walk over the raw `serde_json::Value` map that
detects unknown keys and snake-case aliases against a static schema descriptor,
(2) typed deserialization into per-bucket structs whose length / alignment /
color fields hold `renderable` primitives directly via custom string
deserializers, then (3) an annotation pass that marks every successfully-parsed
key as `KnownButInactive` for sub-specs #2..#7. `into_strict` collapses
`UnknownKey`/`Deprecated` to errors; `KnownButInactive` never fails strict mode.

**Tech Stack:** Rust 2024 edition · serde / serde_json · `renderable` crate
(`Length`, `Alignment`, `Color`, `Tailwind`, `WebColor`) · `thiserror` · `tracing`.
Test runner: cargo-nextest (already in use for `darkmatter/lib`).

**Spec:** [`spec.md`](./spec.md) (revision 2).

**Convention reminders:**
- Workspace: targeted `cargo` calls only (`-p darkmatter`); never bare `cargo build` at repo root.
- Rustdoc: no `# H1` in `///`; use `## Examples` / `## Errors` / `## Notes`.
- US English (en-US) symbol names and prose.
- Tests live inline as `#[cfg(test)] mod tests` in each module; integration tests under `darkmatter/lib/tests/`.
- After every task: `cargo nextest run -p darkmatter --no-fail-fast` must pass (no regressions).

---

## File Structure

| Path | Responsibility |
|---|---|
| `darkmatter/lib/src/style/mod.rs` | Public surface: re-exports of `StyleFrontmatter`, `StyleWarning`, `StyleWarningKind`, `StyleColor`, `StyleParseError`, `from_frontmatter`, `from_json_value`, `into_strict`. Module-level rustdoc. |
| `darkmatter/lib/src/style/error.rs` | `StyleParseError` enum. |
| `darkmatter/lib/src/style/warning.rs` | `StyleWarning`, `StyleWarningKind`, `StyleSpan`. |
| `darkmatter/lib/src/style/length.rs` | `deserialize_optional_length` (horizontal → `renderable::layout::Length`) and `deserialize_optional_row_count` (vertical → `u16`). |
| `darkmatter/lib/src/style/alignment.rs` | `deserialize_optional_alignment` → `renderable::layout::Alignment` with `centered` alias. |
| `darkmatter/lib/src/style/color.rs` | `StyleColor` struct + `deserialize_optional_color` (Tailwind / hex / web-named → `renderable::color::Color` + opacity). |
| `darkmatter/lib/src/style/schema/mod.rs` | Root `StyleFrontmatter` struct + re-exports of per-bucket structs. |
| `darkmatter/lib/src/style/schema/common.rs` | `CommonStyle` (5 shared mutations). |
| `darkmatter/lib/src/style/schema/page.rs` | `PageStyle` (margins, padding, page knobs, bespoke). |
| `darkmatter/lib/src/style/schema/components.rs` | `TableStyle`, `BlockQuoteStyle` (CommonStyle-only). |
| `darkmatter/lib/src/style/schema/lists.rs` | `UlStyle`, `OlStyle`, `LiStyle`. |
| `darkmatter/lib/src/style/schema/inline.rs` | `HyperlinkStyle`, `ImageStyle` (with `local_style`). |
| `darkmatter/lib/src/style/schema/hr.rs` | `HrStyle`. |
| `darkmatter/lib/src/style/descriptor.rs` | Static catalog of canonical key paths + alias map. |
| `darkmatter/lib/src/style/walker.rs` | Pass-1 canonicalization walk over `serde_json::Value`. |
| `darkmatter/lib/src/style/parse.rs` | `from_json_value`, `from_frontmatter`, `into_strict`, `KnownButInactive` annotation pass. |
| `darkmatter/lib/src/lib.rs` | One-line modification: add `pub mod style;`. |
| `darkmatter/lib/tests/style_frontmatter.rs` | Integration test against `example-docs/rendering/style-prop.md`. |

---

## Task 1: Scaffold `darkmatter::style` module

**Files:**
- Create: `darkmatter/lib/src/style/mod.rs`
- Modify: `darkmatter/lib/src/lib.rs` (add `pub mod style;` near the other `pub mod` lines around line 23)

- [ ] **Step 1: Write the failing test**

Add this test in `darkmatter/lib/src/style/mod.rs`:

```rust
//! Frontmatter `style:` parser for darkmatter documents.
//!
//! See `renderable/features/_unscheduled/style-property/spec.md` for the
//! design. Sub-spec #1: schema + parser only; no rendering changes.

#[cfg(test)]
mod tests {
    /// Smoke test: the module is reachable and compiles.
    #[test]
    fn module_compiles() {
        // Intentionally empty. Existence is the assertion.
    }
}
```

- [ ] **Step 2: Add the module declaration**

In `darkmatter/lib/src/lib.rs`, insert `pub mod style;` alongside the existing
`pub mod` lines (alphabetical order — between `pub mod render;` and
`pub mod terminal;`).

- [ ] **Step 3: Run the test**

```bash
cargo nextest run -p darkmatter style::tests::module_compiles
```

Expected: 1 test passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/lib.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): scaffold style frontmatter module"
```

---

## Task 2: Define `StyleParseError`

**Files:**
- Create: `darkmatter/lib/src/style/error.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod error;` and re-export)

- [ ] **Step 1: Write the failing test**

Create `darkmatter/lib/src/style/error.rs` with:

```rust
//! Error type returned by `darkmatter::style` parsers.

use thiserror::Error;

use super::warning::StyleWarning;

/// Errors that can be returned by `darkmatter::style` parsers.
///
/// ## Notes
///
/// `Strict` is produced by [`super::into_strict`] when an otherwise successful
/// parse carries `UnknownKey` or `Deprecated` warnings.
#[derive(Debug, Error)]
pub enum StyleParseError {
    /// A YAML node had the wrong shape at the given path.
    #[error("Invalid YAML structure at `{path}`: expected {expected}, got {actual}")]
    Structure {
        path: String,
        expected: &'static str,
        actual: String,
    },

    /// A length value could not be parsed.
    #[error("Invalid length `{raw}` at `{path}`: {reason}")]
    InvalidLength {
        path: String,
        raw: String,
        reason: &'static str,
    },

    /// A percent value was out of `0.0..=100.0`.
    #[error("Invalid percent `{value}` at `{path}`: must be in 0.0..=100.0")]
    InvalidPercent { path: String, value: f32 },

    /// A color value could not be parsed.
    #[error("Invalid color `{raw}` at `{path}`: {reason}")]
    InvalidColor {
        path: String,
        raw: String,
        reason: &'static str,
    },

    /// Strict mode: schema-validation warnings (`UnknownKey` or `Deprecated`)
    /// were promoted to an error.
    #[error("Strict mode: {} schema warning(s)", warnings.len())]
    Strict { warnings: Vec<StyleWarning> },

    /// Pass-through for serde failures.
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_message_contains_path_and_types() {
        let err = StyleParseError::Structure {
            path: "style.page.left-margin".to_string(),
            expected: "string",
            actual: "number".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("style.page.left-margin"));
        assert!(msg.contains("string"));
        assert!(msg.contains("number"));
    }

    #[test]
    fn invalid_length_message_contains_raw_and_reason() {
        let err = StyleParseError::InvalidLength {
            path: "style.page.left-margin".to_string(),
            raw: "2px".to_string(),
            reason: "unsupported unit `px`; allowed: ch, %",
        };
        let msg = err.to_string();
        assert!(msg.contains("2px"));
        assert!(msg.contains("unsupported unit"));
    }
}
```

- [ ] **Step 2: Wire the module + create the warning placeholder**

Because `error.rs` depends on `warning.rs` (declared in Task 3), create a
**temporary stub** for `warning.rs` now so the crate compiles. In
`darkmatter/lib/src/style/warning.rs`:

```rust
//! Warning types — full definition lands in Task 3.

#[derive(Debug, Clone, PartialEq)]
pub struct StyleWarning;
```

Update `darkmatter/lib/src/style/mod.rs` to add:

```rust
pub mod error;
pub mod warning;

pub use error::StyleParseError;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::error::tests
```

Expected: 2 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/error.rs darkmatter/lib/src/style/warning.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): add StyleParseError type"
```

---

## Task 3: Define `StyleWarning` + `StyleWarningKind` + `StyleSpan`

**Files:**
- Modify: `darkmatter/lib/src/style/warning.rs` (replace stub)
- Modify: `darkmatter/lib/src/style/mod.rs` (re-exports)

- [ ] **Step 1: Write the failing tests**

Replace the contents of `darkmatter/lib/src/style/warning.rs` with:

```rust
//! Discriminated warning channel for the `style:` parser.

/// Source-position placeholder. v1 always produces `None` for
/// `StyleWarning::source_span`; the struct exists so later sub-specs can
/// populate it without changing the public surface.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSpan {
    pub line: u32,
    pub column: u32,
    pub length: u32,
}

/// Discriminated category for a `StyleWarning`.
///
/// ## Notes
///
/// `into_strict` promotes `UnknownKey` and `Deprecated` to errors;
/// `KnownButInactive` is informational and never fails strict mode.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleWarningKind {
    /// The path does not appear anywhere in the schema. Likely a typo.
    UnknownKey,
    /// The path matched a documented snake-case alias for a renamed key.
    /// The kebab-case canonical spelling is `replacement`.
    Deprecated { replacement: String },
    /// The path parsed successfully and is structurally valid, but the
    /// rendering wiring for this key has not yet been implemented. The
    /// sub-spec number tells the user when it will be.
    KnownButInactive { sub_spec: u8 },
}

/// A warning emitted by the `style:` parser.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleWarning {
    /// Fully-qualified YAML path, e.g., `style.page.lft-margin`.
    pub path: String,
    pub kind: StyleWarningKind,
    /// Source position. Always `None` in v1.
    pub source_span: Option<StyleSpan>,
}

impl StyleWarning {
    /// Convenience: a warning with no source span.
    pub fn new(path: impl Into<String>, kind: StyleWarningKind) -> Self {
        Self {
            path: path.into(),
            kind,
            source_span: None,
        }
    }

    /// `true` if this warning is a schema-validation issue that strict mode
    /// promotes to an error.
    pub fn is_schema_issue(&self) -> bool {
        matches!(
            self.kind,
            StyleWarningKind::UnknownKey | StyleWarningKind::Deprecated { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_is_schema_issue() {
        let w = StyleWarning::new("style.x", StyleWarningKind::UnknownKey);
        assert!(w.is_schema_issue());
    }

    #[test]
    fn deprecated_is_schema_issue() {
        let w = StyleWarning::new(
            "style.block_quote",
            StyleWarningKind::Deprecated { replacement: "block-quote".into() },
        );
        assert!(w.is_schema_issue());
    }

    #[test]
    fn known_but_inactive_is_not_schema_issue() {
        let w = StyleWarning::new(
            "style.page.color",
            StyleWarningKind::KnownButInactive { sub_spec: 5 },
        );
        assert!(!w.is_schema_issue());
    }

    #[test]
    fn span_defaults_to_none() {
        let w = StyleWarning::new("style.x", StyleWarningKind::UnknownKey);
        assert_eq!(w.source_span, None);
    }
}
```

- [ ] **Step 2: Re-export from the module**

Update `darkmatter/lib/src/style/mod.rs`:

```rust
pub use warning::{StyleSpan, StyleWarning, StyleWarningKind};
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::warning::tests
```

Expected: 4 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/warning.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): add StyleWarning discriminated channel"
```

---

## Task 4: Horizontal length deserializer (`"2ch"` / `"50%"` / `"40"` → `Length`)

**Files:**
- Create: `darkmatter/lib/src/style/length.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod length;`)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/length.rs`:

```rust
//! Custom deserializers that lower frontmatter length strings into
//! `renderable::layout::Length` (horizontal) and `u16` (vertical row counts).

use renderable::layout::Length;
use serde::de::{self, Deserializer};

/// Parse a single horizontal length value.
///
/// ## Accepted forms
///
/// - `"2ch"` / `"2 ch"` → `Length::Ch(2)`
/// - `"40"` (bare) → `Length::Ch(40)`
/// - `"50%"` / `"50.5%"` → `Length::Percent(50.0)` / `Length::Percent(50.5)`
///
/// ## Errors
///
/// Returns a serde error with one of the reasons:
/// `"empty length"`, `"negative length"`, `"malformed percent"`,
/// `"unsupported unit `<unit>`; allowed: ch, %"`, or
/// `"percent out of range; must be in 0.0..=100.0"`.
pub fn parse_horizontal(raw: &str) -> Result<Length, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty length");
    }
    if trimmed.starts_with('-') {
        return Err("negative length");
    }

    // Percent: trailing `%`.
    if let Some(num_part) = trimmed.strip_suffix('%') {
        let n: f32 = num_part
            .trim()
            .parse()
            .map_err(|_| "malformed percent")?;
        if !(0.0..=100.0).contains(&n) || !n.is_finite() {
            return Err("percent out of range; must be in 0.0..=100.0");
        }
        return Ok(Length::Percent(n));
    }

    // `Nch` (with or without space).
    let lower = trimmed.to_ascii_lowercase();
    if let Some(num_part) = lower.strip_suffix("ch") {
        let n: u32 = num_part.trim().parse().map_err(|_| "malformed ch length")?;
        return Ok(Length::Ch(n));
    }

    // Bare number → Ch.
    if let Ok(n) = trimmed.parse::<u32>() {
        return Ok(Length::Ch(n));
    }

    Err("unsupported unit; allowed: ch, %")
}

/// Serde deserializer for `Option<Length>` reading a string.
pub fn deserialize_optional_length<'de, D>(de: D) -> Result<Option<Length>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(de)?;
    match raw {
        None => Ok(None),
        Some(s) => parse_horizontal(&s)
            .map(Some)
            .map_err(de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Result<Length, &'static str> {
        parse_horizontal(raw)
    }

    #[test]
    fn bare_number_is_ch() {
        assert_eq!(parse("40"), Ok(Length::Ch(40)));
        assert_eq!(parse("0"), Ok(Length::Ch(0)));
    }

    #[test]
    fn nch_parses() {
        assert_eq!(parse("2ch"), Ok(Length::Ch(2)));
        assert_eq!(parse("2 ch"), Ok(Length::Ch(2)));
        assert_eq!(parse("0ch"), Ok(Length::Ch(0)));
    }

    #[test]
    fn percent_parses() {
        assert_eq!(parse("50%"), Ok(Length::Percent(50.0)));
        assert_eq!(parse("50.5%"), Ok(Length::Percent(50.5)));
        assert_eq!(parse("100%"), Ok(Length::Percent(100.0)));
        assert_eq!(parse("0%"), Ok(Length::Percent(0.0)));
    }

    #[test]
    fn negative_rejected() {
        assert_eq!(parse("-2"), Err("negative length"));
        assert_eq!(parse("-2ch"), Err("negative length"));
        assert_eq!(parse("-50%"), Err("negative length"));
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(parse(""), Err("empty length"));
        assert_eq!(parse("   "), Err("empty length"));
    }

    #[test]
    fn unsupported_unit_rejected() {
        assert!(parse("2px").is_err());
        assert!(parse("2em").is_err());
        assert!(parse("2rem").is_err());
    }

    #[test]
    fn malformed_percent_rejected() {
        assert_eq!(parse("50%%"), Err("malformed percent"));
        assert_eq!(parse("abc%"), Err("malformed percent"));
    }

    #[test]
    fn percent_out_of_range_rejected() {
        assert_eq!(
            parse("101%"),
            Err("percent out of range; must be in 0.0..=100.0")
        );
    }

    #[test]
    fn deserialize_via_serde() {
        // Use serde_json with a wrapper struct.
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_length")]
            v: Option<Length>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": "2ch"}"#).unwrap();
        assert_eq!(w.v, Some(Length::Ch(2)));
        let w: Wrap = serde_json::from_str(r#"{"v": null}"#).unwrap();
        assert_eq!(w.v, None);
        let err = serde_json::from_str::<Wrap>(r#"{"v": "2px"}"#).unwrap_err();
        assert!(err.to_string().contains("unsupported unit"));
    }
}
```

- [ ] **Step 2: Register the module**

In `darkmatter/lib/src/style/mod.rs`, add:

```rust
pub mod length;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::length::tests
```

Expected: 9 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/length.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): horizontal length deserializer for style frontmatter"
```

---

## Task 5: Vertical row-count deserializer (`u16` only, reject strings)

**Files:**
- Modify: `darkmatter/lib/src/style/length.rs` (extend)

- [ ] **Step 1: Write the failing tests**

Append to `darkmatter/lib/src/style/length.rs`:

```rust
/// Serde deserializer for `Option<u16>` row counts.
///
/// Explicitly rejects strings so `top-margin: "2ch"` produces a clear error
/// rather than serde's default "invalid type" message.
pub fn deserialize_optional_row_count<'de, D>(de: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(de)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| de::Error::custom("row count out of range for u16")),
        Some(other) => Err(de::Error::custom(format!(
            "row count must be a non-negative integer (got {})",
            type_name_of(&other)
        ))),
    }
}

fn type_name_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}
```

Append to the `tests` module:

```rust
    #[test]
    fn row_count_accepts_integers() {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_row_count")]
            v: Option<u16>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": 0}"#).unwrap();
        assert_eq!(w.v, Some(0));
        let w: Wrap = serde_json::from_str(r#"{"v": 1}"#).unwrap();
        assert_eq!(w.v, Some(1));
        let w: Wrap = serde_json::from_str(r#"{"v": 42}"#).unwrap();
        assert_eq!(w.v, Some(42));
    }

    #[test]
    fn row_count_rejects_strings() {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_row_count")]
            v: Option<u16>,
        }
        let err = serde_json::from_str::<Wrap>(r#"{"v": "2ch"}"#).unwrap_err();
        assert!(err.to_string().contains("must be a non-negative integer"));
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn row_count_null_is_none() {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default, deserialize_with = "deserialize_optional_row_count")]
            v: Option<u16>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": null}"#).unwrap();
        assert_eq!(w.v, None);
    }
```

- [ ] **Step 2: Run the tests**

```bash
cargo nextest run -p darkmatter style::length::tests
```

Expected: 12 tests passed.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/lib/src/style/length.rs
git commit -m "feat(darkmatter): row-count deserializer for vertical style fields"
```

---

## Task 6: Alignment deserializer (`left|center|centered|right` → `renderable::layout::Alignment`)

**Files:**
- Create: `darkmatter/lib/src/style/alignment.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod alignment;`)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/alignment.rs`:

```rust
//! Deserializer for `renderable::layout::Alignment` accepting the documented
//! `centered` alias for `center`.

use renderable::layout::Alignment;
use serde::de::{self, Deserializer};

/// Parse an alignment string.
///
/// Accepts `"left"`, `"center"`, `"centered"`, `"right"`.
pub fn parse(raw: &str) -> Result<Alignment, &'static str> {
    match raw.trim() {
        "left" => Ok(Alignment::Left),
        "center" | "centered" => Ok(Alignment::Center),
        "right" => Ok(Alignment::Right),
        _ => Err("alignment must be one of: left, center, centered, right"),
    }
}

pub fn deserialize_optional_alignment<'de, D>(de: D) -> Result<Option<Alignment>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(de)?;
    match raw {
        None => Ok(None),
        Some(s) => parse(&s).map(Some).map_err(de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_center_right() {
        assert_eq!(parse("left"), Ok(Alignment::Left));
        assert_eq!(parse("center"), Ok(Alignment::Center));
        assert_eq!(parse("right"), Ok(Alignment::Right));
    }

    #[test]
    fn centered_alias_matches_center() {
        assert_eq!(parse("centered"), Ok(Alignment::Center));
    }

    #[test]
    fn unknown_value_rejected() {
        assert!(parse("middle").is_err());
        assert!(parse("justify").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn deserialize_via_serde() {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(deserialize_with = "deserialize_optional_alignment")]
            v: Option<Alignment>,
        }
        let w: Wrap = serde_json::from_str(r#"{"v": "centered"}"#).unwrap();
        assert_eq!(w.v, Some(Alignment::Center));
        let w: Wrap = serde_json::from_str(r#"{"v": null}"#).unwrap();
        assert_eq!(w.v, None);
    }
}
```

- [ ] **Step 2: Register the module**

In `darkmatter/lib/src/style/mod.rs`, add `pub mod alignment;`.

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::alignment::tests
```

Expected: 4 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/alignment.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): alignment deserializer with `centered` alias"
```

---

## Task 7: `StyleColor` + Tailwind name parser

**Files:**
- Create: `darkmatter/lib/src/style/color.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod color;` + re-export `StyleColor`)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/color.rs`:

```rust
//! Color deserializer for the style frontmatter.
//!
//! Lowers Tailwind names (`red-500`, `red-500/50`), hex (`#rrggbb`,
//! `#rrggbbaa`), and CSS web names (`orange`) into
//! [`renderable::color::Color`]. Opacity is preserved separately because
//! the underlying enum does not carry it.

use renderable::color::{Color, Tailwind, WEB_COLOR_LOOKUP, WebColor};
use serde::de::{self, Deserializer};

/// A frontmatter color value.
///
/// Wraps `renderable::color::Color` (which does not carry opacity) with an
/// optional Tailwind-style opacity (`/50` → `Some(50)`), in `0..=100`.
/// Opacity is documented as HTML-only by `docs/rendering/style.md`; terminal
/// targets drop it.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleColor {
    pub color: Color,
    pub opacity: Option<u8>,
}

/// Parse a color string.
pub fn parse(raw: &str) -> Result<StyleColor, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty color");
    }

    // Hex (handled in Task 8).
    if trimmed.starts_with('#') {
        return parse_hex(trimmed);
    }

    // Tailwind (`family-level` or `family-level/opacity`).
    if let Some(tw) = parse_tailwind(trimmed)? {
        return Ok(tw);
    }

    // Web named color (handled in Task 8).
    if let Some(web) = parse_web_named(trimmed) {
        return Ok(web);
    }

    Err("unrecognized color; expected Tailwind name, hex, or web color name")
}

/// Parse a Tailwind palette string like `red-500` or `red-500/50`.
///
/// Returns `Ok(None)` if the input doesn't look like a Tailwind reference,
/// `Ok(Some(_))` if it parses, `Err` if it looks like a Tailwind reference
/// but is malformed.
fn parse_tailwind(raw: &str) -> Result<Option<StyleColor>, &'static str> {
    // Split off opacity.
    let (color_part, opacity) = match raw.split_once('/') {
        Some((c, op)) => {
            let n: u8 = op
                .parse()
                .map_err(|_| "malformed opacity (must be integer 0..=100)")?;
            if n > 100 {
                return Err("opacity must be in 0..=100");
            }
            (c, Some(n))
        }
        None => (raw, None),
    };

    // Split family from level.
    let Some((family, level)) = color_part.rsplit_once('-') else {
        return Ok(None); // not Tailwind-shaped
    };

    let tw = tailwind_variant(family, level)?;
    let Some(tw) = tw else {
        // Family or level wasn't recognized; might be a web-named hyphen
        // form (none in Tailwind palette overlap, but be permissive).
        return Ok(None);
    };

    Ok(Some(StyleColor {
        color: Color::Tailwind(tw),
        opacity,
    }))
}

/// Map `(family, level)` to a `Tailwind` enum variant.
///
/// Returns `Ok(None)` when family or level is not recognized (not an error;
/// caller can fall through to other color forms). Returns `Err` when the
/// level looks numeric but is not a valid Tailwind step.
fn tailwind_variant(family: &str, level: &str) -> Result<Option<Tailwind>, &'static str> {
    // Specials first (no level).
    match family {
        "transparent" | "current" | "inherit" | "black" | "white" if level.is_empty() => {
            return Ok(Some(special_to_tailwind(family)));
        }
        _ => {}
    }

    // Level must be one of the canonical steps.
    let level_ok = matches!(
        level,
        "50" | "100" | "200" | "300" | "400" | "500"
            | "600" | "700" | "800" | "900" | "950"
    );
    if !level_ok {
        // If the level looks like a number but isn't canonical, that's a
        // real error worth surfacing.
        if level.chars().all(|c| c.is_ascii_digit()) && !level.is_empty() {
            return Err(
                "Tailwind level must be one of: 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950",
            );
        }
        return Ok(None);
    }

    let variant = match (family, level) {
        // Generated below — see tailwind_table! macro.
        _ => {
            let Some(v) = lookup_tailwind(family, level) else {
                return Ok(None);
            };
            v
        }
    };
    Ok(Some(variant))
}

fn special_to_tailwind(family: &str) -> Tailwind {
    match family {
        "transparent" => Tailwind::Transparent,
        "current" => Tailwind::Current,
        "inherit" => Tailwind::Inherit,
        "black" => Tailwind::Black,
        "white" => Tailwind::White,
        _ => unreachable!(),
    }
}

/// Lookup table for `(family, level)` → `Tailwind`. Hand-written because
/// `Tailwind` is an exhaustive enum and we want exhaustive matching.
fn lookup_tailwind(family: &str, level: &str) -> Option<Tailwind> {
    macro_rules! tw {
        ($($fam:literal => $variant:ident),* $(,)?) => {
            match (family, level) {
                $(
                    ($fam, "50")  => Some(Tailwind::concat_ident!($variant, 50)),
                )*
                _ => None,
            }
        };
    }
    // The macro above is a sketch; spell out the table explicitly to avoid
    // proc-macro gymnastics. 21 families × 11 levels = 231 entries.
    expand_tailwind_table(family, level)
}

/// Explicit `(family, level)` → `Tailwind` table.
///
/// 21 families × 11 levels. Spelling each entry out keeps the match
/// exhaustive without relying on macros or token concatenation.
fn expand_tailwind_table(family: &str, level: &str) -> Option<Tailwind> {
    Some(match (family, level) {
        ("red", "50") => Tailwind::Red50,
        ("red", "100") => Tailwind::Red100,
        ("red", "200") => Tailwind::Red200,
        ("red", "300") => Tailwind::Red300,
        ("red", "400") => Tailwind::Red400,
        ("red", "500") => Tailwind::Red500,
        ("red", "600") => Tailwind::Red600,
        ("red", "700") => Tailwind::Red700,
        ("red", "800") => Tailwind::Red800,
        ("red", "900") => Tailwind::Red900,
        ("red", "950") => Tailwind::Red950,
        // Orange
        ("orange", "50") => Tailwind::Orange50,
        ("orange", "100") => Tailwind::Orange100,
        ("orange", "200") => Tailwind::Orange200,
        ("orange", "300") => Tailwind::Orange300,
        ("orange", "400") => Tailwind::Orange400,
        ("orange", "500") => Tailwind::Orange500,
        ("orange", "600") => Tailwind::Orange600,
        ("orange", "700") => Tailwind::Orange700,
        ("orange", "800") => Tailwind::Orange800,
        ("orange", "900") => Tailwind::Orange900,
        ("orange", "950") => Tailwind::Orange950,
        // Amber, Yellow, Lime, Green, Emerald, Teal, Cyan, Sky, Blue,
        // Indigo, Violet, Purple, Fuchsia, Pink, Rose, Slate, Gray, Zinc,
        // Neutral, Stone — same 11-row pattern. Fill all 21 families here
        // by copying the Red/Orange blocks and renaming. See
        // `renderable/src/color/tailwind.rs` for the exact variant
        // identifiers.
        _ => return None,
    })
}

/// Stub: hex parsing lands in Task 8.
fn parse_hex(_raw: &str) -> Result<StyleColor, &'static str> {
    Err("hex parsing not yet implemented (Task 8)")
}

/// Stub: web-named parsing lands in Task 8.
fn parse_web_named(_raw: &str) -> Option<StyleColor> {
    None
}

pub fn deserialize_optional_color<'de, D>(de: D) -> Result<Option<StyleColor>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(de)?;
    match raw {
        None => Ok(None),
        Some(s) => parse(&s).map(Some).map_err(de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_named_family_level() {
        let c = parse("red-500").unwrap();
        assert_eq!(c.color, Color::Tailwind(Tailwind::Red500));
        assert_eq!(c.opacity, None);
    }

    #[test]
    fn tailwind_with_opacity() {
        let c = parse("red-500/50").unwrap();
        assert_eq!(c.color, Color::Tailwind(Tailwind::Red500));
        assert_eq!(c.opacity, Some(50));
    }

    #[test]
    fn tailwind_opacity_bounds() {
        assert_eq!(parse("red-500/0").unwrap().opacity, Some(0));
        assert_eq!(parse("red-500/100").unwrap().opacity, Some(100));
        assert!(parse("red-500/101").is_err());
    }

    #[test]
    fn tailwind_specials() {
        assert_eq!(parse("transparent").unwrap().color, Color::Tailwind(Tailwind::Transparent));
        assert_eq!(parse("black").unwrap().color, Color::Tailwind(Tailwind::Black));
        assert_eq!(parse("white").unwrap().color, Color::Tailwind(Tailwind::White));
    }

    #[test]
    fn tailwind_bad_level_errors() {
        // Level looks numeric but isn't a canonical step.
        assert!(parse("red-501").is_err());
    }

    #[test]
    fn empty_color_rejected() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }
}
```

> **Worker note:** The `expand_tailwind_table` body must enumerate every
> family from `renderable/src/color/tailwind.rs` (Red, Orange, Amber, Yellow,
> Lime, Green, Emerald, Teal, Cyan, Sky, Blue, Indigo, Violet, Purple,
> Fuchsia, Pink, Rose, Slate, Gray, Zinc, Neutral, Stone — and `Stone` may
> be absent; check the file). Spell out every `(family, level) => variant`
> pair. The shown Red/Orange entries are the pattern. Do not use macros.

- [ ] **Step 2: Register the module**

In `darkmatter/lib/src/style/mod.rs`, add:

```rust
pub mod color;
pub use color::StyleColor;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::color::tests
```

Expected: 6 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/color.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): StyleColor type and Tailwind name parser"
```

---

## Task 8: Hex and web-named color parsing

**Files:**
- Modify: `darkmatter/lib/src/style/color.rs` (replace stubs from Task 7)

- [ ] **Step 1: Replace `parse_hex` and `parse_web_named` with real implementations**

Replace the `parse_hex` and `parse_web_named` stubs in
`darkmatter/lib/src/style/color.rs`:

```rust
/// Parse a CSS hex color: `#rgb`, `#rrggbb`, or `#rrggbbaa`.
fn parse_hex(raw: &str) -> Result<StyleColor, &'static str> {
    use renderable::color::{BasicColor, RgbColor};

    let hex = raw.trim_start_matches('#');
    if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("non-hex digit");
    }

    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).map_err(|_| "non-hex digit")?;
            let g = u8::from_str_radix(&hex[1..2], 16).map_err(|_| "non-hex digit")?;
            let b = u8::from_str_radix(&hex[2..3], 16).map_err(|_| "non-hex digit")?;
            (r * 17, g * 17, b * 17, None) // expand nibble
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "non-hex digit")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "non-hex digit")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "non-hex digit")?;
            (r, g, b, None)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "non-hex digit")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "non-hex digit")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "non-hex digit")?;
            let a_byte = u8::from_str_radix(&hex[6..8], 16).map_err(|_| "non-hex digit")?;
            // Alpha 0..=255 → opacity 0..=100.
            let opacity = ((a_byte as u32) * 100 / 255) as u8;
            (r, g, b, Some(opacity))
        }
        _ => {
            return Err("hex color must have 3, 6, or 8 digits after `#`");
        }
    };

    // `BasicColor` fallback is required by `RgbColor::new`; pick a sensible
    // ANSI mapping based on dominant channel.
    let fallback = dominant_basic_color(r, g, b);
    Ok(StyleColor {
        color: Color::Rgb(RgbColor::new(r, g, b, fallback)),
        opacity: a,
    })
}

/// Pick a `BasicColor` fallback for hex inputs (used only when ANSI palette
/// is too narrow for true-color output). Simple dominant-channel heuristic.
fn dominant_basic_color(r: u8, g: u8, b: u8) -> renderable::color::BasicColor {
    use renderable::color::BasicColor::*;
    match (r, g, b) {
        (r, g, b) if r >= 200 && g >= 200 && b >= 200 => White,
        (r, g, b) if r < 50 && g < 50 && b < 50 => Black,
        (r, g, b) if r > g && r > b => Red,
        (r, g, b) if g > r && g > b => Green,
        (r, g, b) if b > r && b > g => Blue,
        (r, g, b) if r > 150 && g > 150 && b < 100 => Yellow,
        (r, g, b) if r > 150 && b > 150 && g < 100 => Magenta,
        (r, g, b) if g > 150 && b > 150 && r < 100 => Cyan,
        _ => White,
    }
}

/// Parse a CSS web-named color (`"orange"`, `"rebeccapurple"`).
fn parse_web_named(raw: &str) -> Option<StyleColor> {
    // `WebColor` is an enum with `serde(rename_all = "snake_case")` and
    // matching CSS spellings (lowercase, no spaces). Deserialize via JSON
    // because we already have the lookup table for fallback.
    let lower = raw.to_ascii_lowercase();
    let web: WebColor = serde_json::from_value(serde_json::Value::String(lower)).ok()?;
    // Confirm it has a registered RGB mapping.
    WEB_COLOR_LOOKUP.get(&web)?;
    Some(StyleColor {
        color: Color::Web(web),
        opacity: None,
    })
}
```

- [ ] **Step 2: Add tests**

Append to the `tests` module in `color.rs`:

```rust
    #[test]
    fn hex_short() {
        let c = parse("#fff").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 255);
                assert_eq!(rgb.green(), 255);
                assert_eq!(rgb.blue(), 255);
            }
            _ => panic!("expected Color::Rgb"),
        }
        assert_eq!(c.opacity, None);
    }

    #[test]
    fn hex_long() {
        let c = parse("#ff8000").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 255);
                assert_eq!(rgb.green(), 128);
                assert_eq!(rgb.blue(), 0);
            }
            _ => panic!("expected Color::Rgb"),
        }
    }

    #[test]
    fn hex_with_alpha() {
        let c = parse("#ff000080").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 255);
                assert_eq!(rgb.green(), 0);
                assert_eq!(rgb.blue(), 0);
            }
            _ => panic!("expected Color::Rgb"),
        }
        // 0x80 / 255 * 100 ≈ 50
        assert_eq!(c.opacity, Some(50));
    }

    #[test]
    fn hex_invalid_digit_rejected() {
        let err = parse("#fg0").unwrap_err();
        assert_eq!(err, "non-hex digit");
    }

    #[test]
    fn hex_wrong_length_rejected() {
        assert!(parse("#ff").is_err());
        assert!(parse("#ffff").is_err());
        assert!(parse("#fffffff").is_err());
    }

    #[test]
    fn web_named_orange() {
        let c = parse("orange").unwrap();
        assert!(matches!(c.color, Color::Web(_)));
        assert_eq!(c.opacity, None);
    }
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::color::tests
```

Expected: 12 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/color.rs
git commit -m "feat(darkmatter): hex and web-named color parsing for style frontmatter"
```

---

## Task 9: `CommonStyle` struct

**Files:**
- Create: `darkmatter/lib/src/style/schema/mod.rs`
- Create: `darkmatter/lib/src/style/schema/common.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod schema;`)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/schema/common.rs`:

```rust
//! `CommonStyle` — the five mutations shared by every component bucket
//! (`width`, `max-width`, `alignment`, `color`, `bg-color`).

use renderable::layout::{Alignment, Length};
use serde::Deserialize;

use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::deserialize_optional_length;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonStyle {
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub width: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "max_width"
    )]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(
        deserialize_with = "deserialize_optional_color",
        alias = "bg_color"
    )]
    pub bg_color: Option<StyleColor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_yields_default() {
        let c: CommonStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(c, CommonStyle::default());
    }

    #[test]
    fn parses_max_width_percent() {
        let c: CommonStyle =
            serde_json::from_str(r#"{"max-width": "50%"}"#).unwrap();
        assert_eq!(c.max_width, Some(Length::Percent(50.0)));
    }

    #[test]
    fn parses_alignment_centered() {
        let c: CommonStyle =
            serde_json::from_str(r#"{"alignment": "centered"}"#).unwrap();
        assert_eq!(c.alignment, Some(Alignment::Center));
    }

    #[test]
    fn snake_case_max_width_alias_accepted() {
        // serde `alias` accepts the snake_case form. The Deprecated warning
        // is emitted by the canonicalization walker (Task 17), not by serde.
        let c: CommonStyle =
            serde_json::from_str(r#"{"max_width": "50%"}"#).unwrap();
        assert_eq!(c.max_width, Some(Length::Percent(50.0)));
    }
}
```

- [ ] **Step 2: Create the schema mod root**

Create `darkmatter/lib/src/style/schema/mod.rs`:

```rust
//! Per-bucket schema structs for the `style:` frontmatter.
//!
//! The root [`StyleFrontmatter`] is defined here; per-bucket structs live
//! in sibling files.

pub mod common;

pub use common::CommonStyle;
```

In `darkmatter/lib/src/style/mod.rs` add `pub mod schema;`.

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::common::tests
```

Expected: 4 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/schema darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): CommonStyle struct for style frontmatter"
```

---

## Task 10: `PageStyle` struct

**Files:**
- Create: `darkmatter/lib/src/style/schema/page.rs`
- Modify: `darkmatter/lib/src/style/schema/mod.rs` (register submodule)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/schema/page.rs`:

```rust
//! `PageStyle` — page-level frontmatter bucket.

use renderable::layout::{Alignment, Length};
use serde::Deserialize;

use crate::layout::PageBackground;
use crate::style::alignment::deserialize_optional_alignment;
use crate::style::color::{StyleColor, deserialize_optional_color};
use crate::style::length::{
    deserialize_optional_length, deserialize_optional_row_count,
};

/// Page-level style settings from frontmatter.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct PageStyle {
    // Margins.
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub left_margin: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub right_margin: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub top_margin: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub bottom_margin: Option<u16>,

    // Padding.
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub left_padding: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub right_padding: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub top_padding: Option<u16>,
    #[serde(deserialize_with = "deserialize_optional_row_count")]
    pub bottom_padding: Option<u16>,

    // Page knobs.
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "max_width"
    )]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_alignment")]
    pub alignment: Option<Alignment>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(
        deserialize_with = "deserialize_optional_color",
        alias = "bg_color"
    )]
    pub bg_color: Option<StyleColor>,
    pub background: Option<PageBackground>,

    // Bespoke (parsed; inactive in v1).
    pub stylesheet: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub code: Option<CodeStyle>,
}

/// Opaque `style.page.code` bucket. Detailed shape lands in sub-spec #7.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CodeStyle {
    pub theme: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_yields_default() {
        let p: PageStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(p, PageStyle::default());
    }

    #[test]
    fn parses_margins_from_test_doc() {
        // Matches the layout from
        // `darkmatter/example-docs/rendering/style-prop.md`.
        let json = r#"{
            "left-margin": "2ch",
            "right-margin": "4ch",
            "top-margin": 1,
            "bottom-margin": 0
        }"#;
        let p: PageStyle = serde_json::from_str(json).unwrap();
        assert_eq!(p.left_margin, Some(Length::Ch(2)));
        assert_eq!(p.right_margin, Some(Length::Ch(4)));
        assert_eq!(p.top_margin, Some(1));
        assert_eq!(p.bottom_margin, Some(0));
    }

    #[test]
    fn rejects_unit_on_vertical_margin() {
        let err = serde_json::from_str::<PageStyle>(
            r#"{"top-margin": "2ch"}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("non-negative integer"));
    }

    #[test]
    fn snake_case_max_width_alias_accepted() {
        let p: PageStyle = serde_json::from_str(r#"{"max_width": "80"}"#).unwrap();
        assert_eq!(p.max_width, Some(Length::Ch(80)));
    }

    #[test]
    fn background_enum_parses() {
        // Existing `PageBackground` derives Deserialize; check it round-trips.
        let p: PageStyle =
            serde_json::from_str(r#"{"background": "subtle"}"#).unwrap();
        assert_eq!(p.background, Some(PageBackground::Subtle));
    }
}
```

- [ ] **Step 2: Register the submodule**

In `darkmatter/lib/src/style/schema/mod.rs`, add:

```rust
pub mod page;
pub use page::{CodeStyle, PageStyle};
```

- [ ] **Step 3: Verify `PageBackground` already has `Deserialize`**

```bash
grep -n "Deserialize" /Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/types.rs | head -5
```

If `PageBackground` does not already derive `Deserialize`, add `Deserialize`
to its derive list (it should already be there — it's used by existing
config paths — but verify before this task ships).

- [ ] **Step 4: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::page::tests
```

Expected: 5 tests passed.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/style/schema/page.rs darkmatter/lib/src/style/schema/mod.rs
git commit -m "feat(darkmatter): PageStyle struct for style frontmatter"
```

---

## Task 11: `TableStyle` and `BlockQuoteStyle`

**Files:**
- Create: `darkmatter/lib/src/style/schema/components.rs`
- Modify: `darkmatter/lib/src/style/schema/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/schema/components.rs`:

```rust
//! Component buckets whose schema is exactly `CommonStyle` (no bespoke
//! fields): `table`, `block-quote`.

use serde::Deserialize;

use super::common::CommonStyle;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct TableStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct BlockQuoteStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::{Alignment, Length};

    #[test]
    fn table_parses_alignment_and_max_width() {
        let json = r#"{"alignment": "right", "max-width": "50%"}"#;
        let t: TableStyle = serde_json::from_str(json).unwrap();
        assert_eq!(t.common.alignment, Some(Alignment::Right));
        assert_eq!(t.common.max_width, Some(Length::Percent(50.0)));
    }

    #[test]
    fn block_quote_parses_color() {
        let json = r#"{"color": "red-500"}"#;
        let bq: BlockQuoteStyle = serde_json::from_str(json).unwrap();
        assert!(bq.common.color.is_some());
    }
}
```

- [ ] **Step 2: Register the submodule**

In `darkmatter/lib/src/style/schema/mod.rs`, add:

```rust
pub mod components;
pub use components::{BlockQuoteStyle, TableStyle};
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::components::tests
```

Expected: 2 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/schema/components.rs darkmatter/lib/src/style/schema/mod.rs
git commit -m "feat(darkmatter): TableStyle and BlockQuoteStyle"
```

---

## Task 12: `UlStyle`, `OlStyle`, `LiStyle`

**Files:**
- Create: `darkmatter/lib/src/style/schema/lists.rs`
- Modify: `darkmatter/lib/src/style/schema/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/schema/lists.rs`:

```rust
//! List buckets: `ul`, `ol`, `li`.

use renderable::layout::Length;
use serde::Deserialize;

use super::common::CommonStyle;
use crate::style::length::deserialize_optional_length;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct UlStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    /// Indent applied to ul content. Wired in sub-spec #4 as
    /// `PageFill::Indent` on `PageComponent::Ul`.
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub left_margin: Option<Length>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct OlStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct LiStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::Alignment;

    #[test]
    fn ul_parses_left_margin_alignment_and_max_width() {
        // Matches the layout from
        // `darkmatter/example-docs/rendering/style-prop.md`:
        // ul:
        //   alignment: left
        //   left-margin: 4ch
        //   max-width: 40
        let json = r#"{
            "alignment": "left",
            "left-margin": "4ch",
            "max-width": "40"
        }"#;
        let ul: UlStyle = serde_json::from_str(json).unwrap();
        assert_eq!(ul.common.alignment, Some(Alignment::Left));
        assert_eq!(ul.left_margin, Some(Length::Ch(4)));
        assert_eq!(ul.common.max_width, Some(Length::Ch(40)));
    }

    #[test]
    fn ol_parses_alignment() {
        let ol: OlStyle =
            serde_json::from_str(r#"{"alignment": "right"}"#).unwrap();
        assert_eq!(ol.common.alignment, Some(Alignment::Right));
    }

    #[test]
    fn li_is_common_only() {
        let li: LiStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(li, LiStyle::default());
    }
}
```

- [ ] **Step 2: Register the submodule**

In `darkmatter/lib/src/style/schema/mod.rs`:

```rust
pub mod lists;
pub use lists::{LiStyle, OlStyle, UlStyle};
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::lists::tests
```

Expected: 3 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/schema/lists.rs darkmatter/lib/src/style/schema/mod.rs
git commit -m "feat(darkmatter): UlStyle, OlStyle, LiStyle"
```

---

## Task 13: `HyperlinkStyle` and `ImageStyle` (with `local-style`)

**Files:**
- Create: `darkmatter/lib/src/style/schema/inline.rs`
- Modify: `darkmatter/lib/src/style/schema/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/schema/inline.rs`:

```rust
//! Inline buckets: `hyperlinks`, `images`. Both carry an optional
//! `local-style` override applied to file-local references.

use serde::Deserialize;

use super::common::CommonStyle;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HyperlinkStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    #[serde(alias = "local_style")]
    pub local_style: Option<Box<CommonStyle>>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct ImageStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    #[serde(alias = "local_style")]
    pub local_style: Option<Box<CommonStyle>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::Alignment;

    #[test]
    fn hyperlinks_with_local_style() {
        let json = r#"{
            "alignment": "left",
            "local-style": {"alignment": "right"}
        }"#;
        let h: HyperlinkStyle = serde_json::from_str(json).unwrap();
        assert_eq!(h.common.alignment, Some(Alignment::Left));
        let local = h.local_style.expect("local_style should be Some");
        assert_eq!(local.alignment, Some(Alignment::Right));
    }

    #[test]
    fn snake_case_local_style_alias_accepted() {
        let json = r#"{"local_style": {"alignment": "right"}}"#;
        let h: HyperlinkStyle = serde_json::from_str(json).unwrap();
        assert!(h.local_style.is_some());
    }

    #[test]
    fn images_local_style_independent() {
        let json = r#"{"local-style": {"max-width": "50%"}}"#;
        let img: ImageStyle = serde_json::from_str(json).unwrap();
        let local = img.local_style.expect("local_style should be Some");
        assert!(local.max_width.is_some());
    }
}
```

- [ ] **Step 2: Register the submodule**

In `darkmatter/lib/src/style/schema/mod.rs`:

```rust
pub mod inline;
pub use inline::{HyperlinkStyle, ImageStyle};
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::inline::tests
```

Expected: 3 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/schema/inline.rs darkmatter/lib/src/style/schema/mod.rs
git commit -m "feat(darkmatter): HyperlinkStyle and ImageStyle with local_style"
```

---

## Task 14: `HrStyle`

**Files:**
- Create: `darkmatter/lib/src/style/schema/hr.rs`
- Modify: `darkmatter/lib/src/style/schema/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/schema/hr.rs`:

```rust
//! `HrStyle` — horizontal-rule bucket.
//!
//! The legacy per-block `style: waves` attribute is migrated to
//! `style.hr.kind` by sub-spec #6. v1 schema carries `kind` as an opaque
//! `Option<String>`.

use serde::Deserialize;

use super::common::CommonStyle;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct HrStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    pub kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kind() {
        let h: HrStyle = serde_json::from_str(r#"{"kind": "waves"}"#).unwrap();
        assert_eq!(h.kind.as_deref(), Some("waves"));
    }

    #[test]
    fn empty_yields_default() {
        let h: HrStyle = serde_json::from_str("{}").unwrap();
        assert_eq!(h, HrStyle::default());
    }
}
```

- [ ] **Step 2: Register the submodule**

In `darkmatter/lib/src/style/schema/mod.rs`:

```rust
pub mod hr;
pub use hr::HrStyle;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::hr::tests
```

Expected: 2 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/schema/hr.rs darkmatter/lib/src/style/schema/mod.rs
git commit -m "feat(darkmatter): HrStyle bucket (kind as opaque string for v1)"
```

---

## Task 15: `StyleFrontmatter` root struct

**Files:**
- Modify: `darkmatter/lib/src/style/schema/mod.rs` (add root struct)
- Modify: `darkmatter/lib/src/style/mod.rs` (re-export root)

- [ ] **Step 1: Write the failing tests**

Append to `darkmatter/lib/src/style/schema/mod.rs`:

```rust
use serde::Deserialize;

/// Root of the `style:` frontmatter tree. Every bucket is `Option` so a
/// sparse user document does not materialize default values across the tree.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct StyleFrontmatter {
    pub page: Option<PageStyle>,
    pub table: Option<TableStyle>,
    pub hyperlinks: Option<HyperlinkStyle>,
    pub images: Option<ImageStyle>,
    pub hr: Option<HrStyle>,
    pub ul: Option<UlStyle>,
    pub ol: Option<OlStyle>,
    pub li: Option<LiStyle>,
    #[serde(alias = "block_quote")]
    pub block_quote: Option<BlockQuoteStyle>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::Length;

    #[test]
    fn fully_empty_yields_default() {
        let s: StyleFrontmatter = serde_json::from_str("{}").unwrap();
        assert_eq!(s, StyleFrontmatter::default());
    }

    #[test]
    fn only_page_set_leaves_others_none() {
        let s: StyleFrontmatter =
            serde_json::from_str(r#"{"page": {"left-margin": "2ch"}}"#).unwrap();
        assert!(s.page.is_some());
        assert!(s.table.is_none());
        assert!(s.ul.is_none());
        assert!(s.ol.is_none());
        assert!(s.li.is_none());
        assert!(s.hr.is_none());
        assert!(s.hyperlinks.is_none());
        assert!(s.images.is_none());
        assert!(s.block_quote.is_none());

        let page = s.page.unwrap();
        assert_eq!(page.left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn block_quote_canonical_kebab_works() {
        let json = r#"{"block-quote": {"max-width": "50%"}}"#;
        let s: StyleFrontmatter = serde_json::from_str(json).unwrap();
        assert!(s.block_quote.is_some());
    }

    #[test]
    fn block_quote_snake_case_alias_works() {
        let json = r#"{"block_quote": {"max-width": "50%"}}"#;
        let s: StyleFrontmatter = serde_json::from_str(json).unwrap();
        assert!(s.block_quote.is_some());
    }
}
```

- [ ] **Step 2: Re-export from style mod**

In `darkmatter/lib/src/style/mod.rs`:

```rust
pub use schema::StyleFrontmatter;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::schema::tests
```

Expected: 4 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/schema/mod.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): StyleFrontmatter root struct"
```

---

## Task 16: Schema descriptor (canonical paths + alias map)

**Files:**
- Create: `darkmatter/lib/src/style/descriptor.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod descriptor;`)

- [ ] **Step 1: Write the descriptor + failing test**

Create `darkmatter/lib/src/style/descriptor.rs`:

```rust
//! Static catalog of every leaf path the schema understands.
//!
//! Used by the canonicalization walker (pass 1 of the parser) to detect
//! unknown keys and snake-case aliases. Add a row here whenever a field is
//! added to any per-bucket schema struct.

/// A single schema leaf: its canonical kebab-case path plus any accepted
/// snake-case alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaLeaf {
    /// Dotted canonical path under `style.` (e.g. `"page.left-margin"`).
    pub canonical: &'static str,
    /// Snake-case alias path, if any (e.g. `"page.left_margin"`).
    pub alias: Option<&'static str>,
    /// Sub-spec number that wires this key to rendering. `1` indicates
    /// "parsed only in v1; never wired" — currently only descriptor itself.
    /// All other values point to one of sub-specs #2..#7.
    pub sub_spec: u8,
}

/// The complete schema catalog. Every leaf reachable through any per-bucket
/// struct must appear here.
pub const SCHEMA: &[SchemaLeaf] = &[
    // ── page ────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "page.left-margin",   alias: Some("page.left_margin"),   sub_spec: 2 },
    SchemaLeaf { canonical: "page.right-margin",  alias: Some("page.right_margin"),  sub_spec: 2 },
    SchemaLeaf { canonical: "page.top-margin",    alias: Some("page.top_margin"),    sub_spec: 2 },
    SchemaLeaf { canonical: "page.bottom-margin", alias: Some("page.bottom_margin"), sub_spec: 2 },
    SchemaLeaf { canonical: "page.left-padding",  alias: Some("page.left_padding"),  sub_spec: 2 },
    SchemaLeaf { canonical: "page.right-padding", alias: Some("page.right_padding"), sub_spec: 2 },
    SchemaLeaf { canonical: "page.top-padding",   alias: Some("page.top_padding"),   sub_spec: 2 },
    SchemaLeaf { canonical: "page.bottom-padding",alias: Some("page.bottom_padding"),sub_spec: 2 },
    SchemaLeaf { canonical: "page.max-width",     alias: Some("page.max_width"),     sub_spec: 2 },
    SchemaLeaf { canonical: "page.alignment",     alias: None,                       sub_spec: 2 },
    SchemaLeaf { canonical: "page.color",         alias: None,                       sub_spec: 5 },
    SchemaLeaf { canonical: "page.bg-color",      alias: Some("page.bg_color"),      sub_spec: 5 },
    SchemaLeaf { canonical: "page.background",    alias: None,                       sub_spec: 2 },
    SchemaLeaf { canonical: "page.stylesheet",    alias: None,                       sub_spec: 7 },
    SchemaLeaf { canonical: "page.meta",          alias: None,                       sub_spec: 7 },
    SchemaLeaf { canonical: "page.code.theme",    alias: None,                       sub_spec: 7 },

    // ── table ───────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "table.width",     alias: None,                  sub_spec: 3 },
    SchemaLeaf { canonical: "table.max-width", alias: Some("table.max_width"), sub_spec: 3 },
    SchemaLeaf { canonical: "table.alignment", alias: None,                  sub_spec: 3 },
    SchemaLeaf { canonical: "table.color",     alias: None,                  sub_spec: 5 },
    SchemaLeaf { canonical: "table.bg-color",  alias: Some("table.bg_color"), sub_spec: 5 },

    // ── block-quote ─────────────────────────────────────────────────────
    SchemaLeaf { canonical: "block-quote.width",     alias: Some("block_quote.width"),     sub_spec: 3 },
    SchemaLeaf { canonical: "block-quote.max-width", alias: Some("block_quote.max_width"), sub_spec: 3 },
    SchemaLeaf { canonical: "block-quote.alignment", alias: Some("block_quote.alignment"), sub_spec: 3 },
    SchemaLeaf { canonical: "block-quote.color",     alias: Some("block_quote.color"),     sub_spec: 5 },
    SchemaLeaf { canonical: "block-quote.bg-color",  alias: Some("block_quote.bg_color"),  sub_spec: 5 },

    // ── ul ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "ul.width",       alias: None,                sub_spec: 4 },
    SchemaLeaf { canonical: "ul.max-width",   alias: Some("ul.max_width"),sub_spec: 4 },
    SchemaLeaf { canonical: "ul.alignment",   alias: None,                sub_spec: 4 },
    SchemaLeaf { canonical: "ul.color",       alias: None,                sub_spec: 5 },
    SchemaLeaf { canonical: "ul.bg-color",    alias: Some("ul.bg_color"), sub_spec: 5 },
    SchemaLeaf { canonical: "ul.left-margin", alias: Some("ul.left_margin"), sub_spec: 4 },

    // ── ol ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "ol.width",     alias: None,                sub_spec: 4 },
    SchemaLeaf { canonical: "ol.max-width", alias: Some("ol.max_width"),sub_spec: 4 },
    SchemaLeaf { canonical: "ol.alignment", alias: None,                sub_spec: 4 },
    SchemaLeaf { canonical: "ol.color",     alias: None,                sub_spec: 5 },
    SchemaLeaf { canonical: "ol.bg-color",  alias: Some("ol.bg_color"), sub_spec: 5 },

    // ── li ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "li.width",     alias: None,                sub_spec: 4 },
    SchemaLeaf { canonical: "li.max-width", alias: Some("li.max_width"),sub_spec: 4 },
    SchemaLeaf { canonical: "li.alignment", alias: None,                sub_spec: 4 },
    SchemaLeaf { canonical: "li.color",     alias: None,                sub_spec: 5 },
    SchemaLeaf { canonical: "li.bg-color",  alias: Some("li.bg_color"), sub_spec: 5 },

    // ── hyperlinks ──────────────────────────────────────────────────────
    SchemaLeaf { canonical: "hyperlinks.width",                  alias: None, sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.max-width",              alias: Some("hyperlinks.max_width"), sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.alignment",              alias: None, sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.color",                  alias: None, sub_spec: 5 },
    SchemaLeaf { canonical: "hyperlinks.bg-color",               alias: Some("hyperlinks.bg_color"), sub_spec: 5 },
    SchemaLeaf { canonical: "hyperlinks.local-style.width",      alias: Some("hyperlinks.local_style.width"), sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.max-width",  alias: Some("hyperlinks.local_style.max_width"), sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.alignment",  alias: Some("hyperlinks.local_style.alignment"), sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.color",      alias: Some("hyperlinks.local_style.color"), sub_spec: 7 },
    SchemaLeaf { canonical: "hyperlinks.local-style.bg-color",   alias: Some("hyperlinks.local_style.bg_color"), sub_spec: 7 },

    // ── images ──────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "images.width",                  alias: None, sub_spec: 3 },
    SchemaLeaf { canonical: "images.max-width",              alias: Some("images.max_width"), sub_spec: 3 },
    SchemaLeaf { canonical: "images.alignment",              alias: None, sub_spec: 3 },
    SchemaLeaf { canonical: "images.color",                  alias: None, sub_spec: 5 },
    SchemaLeaf { canonical: "images.bg-color",               alias: Some("images.bg_color"), sub_spec: 5 },
    SchemaLeaf { canonical: "images.local-style.width",      alias: Some("images.local_style.width"), sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.max-width",  alias: Some("images.local_style.max_width"), sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.alignment",  alias: Some("images.local_style.alignment"), sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.color",      alias: Some("images.local_style.color"), sub_spec: 7 },
    SchemaLeaf { canonical: "images.local-style.bg-color",   alias: Some("images.local_style.bg_color"), sub_spec: 7 },

    // ── hr ──────────────────────────────────────────────────────────────
    SchemaLeaf { canonical: "hr.width",     alias: None,                sub_spec: 6 },
    SchemaLeaf { canonical: "hr.max-width", alias: Some("hr.max_width"),sub_spec: 6 },
    SchemaLeaf { canonical: "hr.alignment", alias: None,                sub_spec: 6 },
    SchemaLeaf { canonical: "hr.color",     alias: None,                sub_spec: 6 },
    SchemaLeaf { canonical: "hr.bg-color",  alias: Some("hr.bg_color"), sub_spec: 6 },
    SchemaLeaf { canonical: "hr.kind",      alias: None,                sub_spec: 6 },
];

/// Return the canonical path for `raw_path` if it matches either a canonical
/// entry or an alias. Returns `None` for unknown paths.
pub fn canonicalize(raw_path: &str) -> Option<&'static SchemaLeaf> {
    SCHEMA
        .iter()
        .find(|leaf| leaf.canonical == raw_path || leaf.alias == Some(raw_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_lookup_finds_kebab_form() {
        let leaf = canonicalize("page.left-margin").expect("found");
        assert_eq!(leaf.canonical, "page.left-margin");
        assert_eq!(leaf.sub_spec, 2);
    }

    #[test]
    fn canonical_lookup_finds_snake_alias() {
        let leaf = canonicalize("page.left_margin").expect("found");
        assert_eq!(leaf.canonical, "page.left-margin");
        assert_eq!(leaf.alias, Some("page.left_margin"));
    }

    #[test]
    fn unknown_path_returns_none() {
        assert!(canonicalize("page.lft-margin").is_none());
        assert!(canonicalize("planet.left-margin").is_none());
    }

    #[test]
    fn schema_paths_are_unique() {
        // Detect duplicate canonical or alias entries — would cause double-
        // counted warnings.
        let mut seen = std::collections::BTreeSet::new();
        for leaf in SCHEMA {
            assert!(
                seen.insert(leaf.canonical),
                "duplicate canonical: {}",
                leaf.canonical
            );
            if let Some(alias) = leaf.alias {
                assert!(seen.insert(alias), "duplicate alias: {}", alias);
            }
        }
    }
}
```

- [ ] **Step 2: Register the module**

In `darkmatter/lib/src/style/mod.rs`:

```rust
pub mod descriptor;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::descriptor::tests
```

Expected: 4 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/descriptor.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): static schema descriptor for style frontmatter"
```

---

## Task 17: Pass-1 canonicalization walker

**Files:**
- Create: `darkmatter/lib/src/style/walker.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod walker;`)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/walker.rs`:

```rust
//! Pass 1 of the style parser: walk the raw `serde_json::Value` map of the
//! `style:` value, comparing every leaf path against the schema descriptor.
//!
//! Emits `UnknownKey` for paths the descriptor doesn't know, and
//! `Deprecated` for paths that matched a snake-case alias of a canonical
//! kebab-case key.

use crate::style::descriptor::canonicalize;
use crate::style::warning::{StyleWarning, StyleWarningKind};

/// Walk the raw style value and collect schema-validation warnings.
///
/// The path produced for each warning starts with `style.` so it's directly
/// usable by callers / users.
pub fn walk(value: &serde_json::Value) -> Vec<StyleWarning> {
    let mut warnings = Vec::new();
    walk_inner(value, "", &mut warnings);
    warnings
}

fn walk_inner(value: &serde_json::Value, path: &str, warnings: &mut Vec<StyleWarning>) {
    let serde_json::Value::Object(map) = value else {
        return; // leaves are checked at the parent level (by name).
    };

    for (key, child) in map {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", path, key)
        };

        // If the path is a leaf in the schema, check for alias use.
        if let Some(leaf) = canonicalize(&child_path) {
            if leaf.alias == Some(&child_path) {
                warnings.push(StyleWarning::new(
                    format!("style.{}", child_path),
                    StyleWarningKind::Deprecated {
                        replacement: leaf.canonical.to_string(),
                    },
                ));
            }
            // Leaves do not need to recurse further into their value — the
            // typed deserializer will validate them.
            continue;
        }

        // Not a leaf — might be a container path (e.g. "page", "block-quote",
        // "block_quote"). Container "block_quote" is also a deprecated alias
        // because the canonical key is "block-quote"; treat the container's
        // own segment as if it were a leaf path of length 1.
        if is_known_container(&child_path) {
            walk_inner(child, &canonical_container_for(&child_path), warnings);
            continue;
        }
        if let Some(canonical) = deprecated_container(&child_path) {
            warnings.push(StyleWarning::new(
                format!("style.{}", child_path),
                StyleWarningKind::Deprecated {
                    replacement: canonical.to_string(),
                },
            ));
            walk_inner(child, canonical, warnings);
            continue;
        }

        // Truly unknown.
        warnings.push(StyleWarning::new(
            format!("style.{}", child_path),
            StyleWarningKind::UnknownKey,
        ));
    }
}

/// Known canonical container paths (top-level buckets plus the nested
/// `local-style` paths). Anything else either matches a leaf in the
/// descriptor or is unknown.
fn is_known_container(path: &str) -> bool {
    matches!(
        path,
        "page"
            | "table"
            | "block-quote"
            | "hyperlinks"
            | "hyperlinks.local-style"
            | "images"
            | "images.local-style"
            | "hr"
            | "ul"
            | "ol"
            | "li"
            | "page.code"
    )
}

/// Map a deprecated container alias to its canonical container path.
fn deprecated_container(path: &str) -> Option<&'static str> {
    match path {
        "block_quote" => Some("block-quote"),
        "hyperlinks.local_style" => Some("hyperlinks.local-style"),
        "images.local_style" => Some("images.local-style"),
        _ => None,
    }
}

/// For a known container path that may have been written under a deprecated
/// alias, return the canonical container path to use when continuing the
/// walk. Identity for canonical inputs.
fn canonical_container_for(path: &str) -> String {
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn warnings_for(value: serde_json::Value) -> Vec<StyleWarning> {
        walk(&value)
    }

    #[test]
    fn empty_object_no_warnings() {
        assert!(warnings_for(json!({})).is_empty());
    }

    #[test]
    fn unknown_top_level_bucket() {
        let w = warnings_for(json!({"planet": {"x": 1}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.planet");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn unknown_leaf_under_known_bucket() {
        let w = warnings_for(json!({"page": {"lft-margin": "2ch"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.lft-margin");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn snake_case_leaf_is_deprecated() {
        let w = warnings_for(json!({"page": {"left_margin": "2ch"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.left_margin");
        assert_eq!(
            w[0].kind,
            StyleWarningKind::Deprecated {
                replacement: "page.left-margin".to_string()
            }
        );
    }

    #[test]
    fn block_quote_snake_container_is_deprecated() {
        let w = warnings_for(json!({"block_quote": {"max-width": "50%"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.block_quote");
        assert!(matches!(
            w[0].kind,
            StyleWarningKind::Deprecated { ref replacement } if replacement == "block-quote"
        ));
    }

    #[test]
    fn flatten_typo_detected() {
        // `table` is structurally just `CommonStyle` — typos inside it must
        // be detected.
        let w = warnings_for(json!({"table": {"maxx-width": "50%"}}));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.table.maxx-width");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn nested_local_style_typo_detected() {
        let w = warnings_for(json!({
            "hyperlinks": {
                "local-style": {"maxx-width": "50%"}
            }
        }));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.hyperlinks.local-style.maxx-width");
        assert_eq!(w[0].kind, StyleWarningKind::UnknownKey);
    }

    #[test]
    fn multiple_unknown_keys_emit_distinct_warnings() {
        let w = warnings_for(json!({
            "page":  {"lft-margin": "2ch"},
            "table": {"maxx-width": "50%"},
            "ul":    {"left-mrgin": "4ch"}
        }));
        assert_eq!(w.len(), 3);
        let paths: Vec<&str> = w.iter().map(|w| w.path.as_str()).collect();
        assert!(paths.contains(&"style.page.lft-margin"));
        assert!(paths.contains(&"style.table.maxx-width"));
        assert!(paths.contains(&"style.ul.left-mrgin"));
    }
}
```

- [ ] **Step 2: Register the module**

In `darkmatter/lib/src/style/mod.rs`, add `pub mod walker;`.

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::walker::tests
```

Expected: 8 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/walker.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): pass-1 canonicalization walker for style frontmatter"
```

---

## Task 18: `from_json_value` entry point

**Files:**
- Create: `darkmatter/lib/src/style/parse.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (add `pub mod parse;` + re-export entry points)

- [ ] **Step 1: Write the failing tests**

Create `darkmatter/lib/src/style/parse.rs`:

```rust
//! Parse entry points for the style frontmatter.

use serde_json::Value;

use crate::style::error::StyleParseError;
use crate::style::schema::StyleFrontmatter;
use crate::style::walker;
use crate::style::warning::StyleWarning;

/// Parse a `serde_json::Value` representing the value at the `style:` key.
///
/// Returns `(StyleFrontmatter::default(), vec![])` for `Value::Null`.
///
/// ## Errors
///
/// `StyleParseError::Serde` on any typed-deserialization failure
/// (structure/length/color/alignment value errors).
pub fn from_json_value(
    value: &Value,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError> {
    if value.is_null() {
        return Ok((StyleFrontmatter::default(), Vec::new()));
    }

    // Pass 1: collect schema-validation warnings.
    let mut warnings = walker::walk(value);

    // Pass 2: typed deserialize. Serde's `alias` accepts both spellings, so
    // the value is parsed in its original form — no rewriting needed.
    let parsed: StyleFrontmatter = serde_json::from_value(value.clone())?;

    // (Pass 3 — `KnownButInactive` — lands in Task 19.)
    let _ = &mut warnings;

    Ok((parsed, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::layout::{Alignment, Length};
    use serde_json::json;

    #[test]
    fn null_yields_default() {
        let (s, w) = from_json_value(&Value::Null).unwrap();
        assert_eq!(s, StyleFrontmatter::default());
        assert!(w.is_empty());
    }

    #[test]
    fn empty_object_yields_default() {
        let (s, w) = from_json_value(&json!({})).unwrap();
        assert_eq!(s, StyleFrontmatter::default());
        assert!(w.is_empty());
    }

    #[test]
    fn page_left_margin_parses() {
        let (s, w) = from_json_value(&json!({"page": {"left-margin": "2ch"}})).unwrap();
        assert!(w.is_empty());
        assert_eq!(s.page.unwrap().left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn unknown_key_produces_warning_but_parse_succeeds() {
        let (s, w) = from_json_value(&json!({"page": {"lft-margin": "2ch"}})).unwrap();
        // The unknown key is dropped by serde; warning is recorded.
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].path, "style.page.lft-margin");
        assert!(s.page.is_some()); // page bucket still materialized (empty).
    }

    #[test]
    fn deprecated_alias_produces_warning_but_parse_succeeds() {
        let (s, w) = from_json_value(&json!({"page": {"left_margin": "2ch"}})).unwrap();
        assert_eq!(w.len(), 1);
        assert!(matches!(
            w[0].kind,
            crate::style::warning::StyleWarningKind::Deprecated { .. }
        ));
        // Value still parsed because of serde alias.
        assert_eq!(s.page.unwrap().left_margin, Some(Length::Ch(2)));
    }

    #[test]
    fn type_error_short_circuits() {
        let err = from_json_value(&json!({"page": {"left-margin": "2px"}})).unwrap_err();
        // The unknown unit should surface as a Serde error.
        let msg = err.to_string();
        assert!(msg.contains("unsupported unit"));
    }

    #[test]
    fn matches_test_doc_acceptance_criteria() {
        let v = json!({
            "page": {
                "left-margin": "2ch",
                "right-margin": "4ch",
                "top-margin": 1,
                "bottom-margin": 0
            },
            "table": {
                "alignment": "right",
                "max-width": "50%"
            },
            "ol": {"alignment": "right"},
            "ul": {
                "alignment": "left",
                "left-margin": "4ch",
                "max-width": "40"
            }
        });

        let (s, _w) = from_json_value(&v).unwrap();
        let p = s.page.expect("page");
        assert_eq!(p.left_margin, Some(Length::Ch(2)));
        assert_eq!(p.right_margin, Some(Length::Ch(4)));
        assert_eq!(p.top_margin, Some(1));
        assert_eq!(p.bottom_margin, Some(0));

        let t = s.table.expect("table");
        assert_eq!(t.common.alignment, Some(Alignment::Right));
        assert_eq!(t.common.max_width, Some(Length::Percent(50.0)));

        let ol = s.ol.expect("ol");
        assert_eq!(ol.common.alignment, Some(Alignment::Right));

        let ul = s.ul.expect("ul");
        assert_eq!(ul.common.alignment, Some(Alignment::Left));
        assert_eq!(ul.left_margin, Some(Length::Ch(4)));
        assert_eq!(ul.common.max_width, Some(Length::Ch(40)));
    }
}
```

- [ ] **Step 2: Register the module**

In `darkmatter/lib/src/style/mod.rs`:

```rust
pub mod parse;
pub use parse::from_json_value;
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::parse::tests
```

Expected: 7 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/parse.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): from_json_value parse entry point with pass-1 walker"
```

---

## Task 19: `KnownButInactive` annotation pass

**Files:**
- Modify: `darkmatter/lib/src/style/parse.rs` (add annotator + wire it into `from_json_value`)

- [ ] **Step 1: Add the annotator**

In `darkmatter/lib/src/style/parse.rs`, add **above** the `tests` module:

```rust
use crate::style::descriptor::SCHEMA;
use crate::style::warning::StyleWarningKind;

/// Walk the raw style value a second time and emit `KnownButInactive` for
/// every leaf that *is* in the schema but whose wiring sub-spec is greater
/// than 1. We re-walk the raw value (rather than the typed
/// `StyleFrontmatter`) because the typed walk would require visitor code per
/// bucket — the raw walk is one function shared by every leaf.
fn annotate_known_but_inactive(
    value: &Value,
    warnings: &mut Vec<StyleWarning>,
) {
    annotate_inner(value, "", warnings);
}

fn annotate_inner(value: &Value, path: &str, warnings: &mut Vec<StyleWarning>) {
    let Value::Object(map) = value else {
        return;
    };
    for (key, child) in map {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", path, key)
        };

        if let Some(leaf) = SCHEMA
            .iter()
            .find(|l| l.canonical == child_path || l.alias == Some(&child_path))
        {
            // Use the canonical path in the warning, regardless of which
            // spelling the user wrote — the wiring is tracked against
            // canonical paths.
            warnings.push(StyleWarning::new(
                format!("style.{}", leaf.canonical),
                StyleWarningKind::KnownButInactive {
                    sub_spec: leaf.sub_spec,
                },
            ));
            // Leaves don't recurse.
            continue;
        }

        // Containers and unknowns: recurse if it's a container, ignore
        // unknown (pass 1 already reported them).
        if child.is_object() {
            annotate_inner(child, &child_path, warnings);
        }
    }
}
```

Modify `from_json_value` to call the annotator:

```rust
pub fn from_json_value(
    value: &Value,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError> {
    if value.is_null() {
        return Ok((StyleFrontmatter::default(), Vec::new()));
    }

    let mut warnings = walker::walk(value);
    let parsed: StyleFrontmatter = serde_json::from_value(value.clone())?;
    annotate_known_but_inactive(value, &mut warnings);
    Ok((parsed, warnings))
}
```

- [ ] **Step 2: Replace the prior `unknown_key_produces_warning_but_parse_succeeds` test**

The previous test asserted exactly 1 warning. With the annotator, the unknown
key still emits 1 warning AND no `KnownButInactive` (the unknown key doesn't
contribute one, because it's not in the schema). The page bucket itself
materializes as empty so no other leaves contribute. The test as written
should still pass — but verify by running it. If it fails because the page
bucket is being annotated, narrow the assertion to:

```rust
        // Filter to schema-validation warnings only.
        let schema_warnings: Vec<_> = w
            .iter()
            .filter(|w| w.is_schema_issue())
            .collect();
        assert_eq!(schema_warnings.len(), 1);
        assert_eq!(schema_warnings[0].path, "style.page.lft-margin");
```

- [ ] **Step 3: Add tests for KnownButInactive**

Append to the `tests` module in `parse.rs`:

```rust
    #[test]
    fn known_but_inactive_per_field() {
        let (_, w) = from_json_value(&json!({
            "page": {"left-margin": "2ch"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert_eq!(inactive.len(), 1);
        assert_eq!(inactive[0].path, "style.page.left-margin");
        assert!(matches!(
            inactive[0].kind,
            StyleWarningKind::KnownButInactive { sub_spec: 2 }
        ));
    }

    #[test]
    fn deprecated_alias_uses_canonical_in_known_but_inactive() {
        let (_, w) = from_json_value(&json!({
            "page": {"left_margin": "2ch"}
        }))
        .unwrap();
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        assert_eq!(inactive.len(), 1);
        // KnownButInactive uses the canonical name even when the user wrote
        // the alias.
        assert_eq!(inactive[0].path, "style.page.left-margin");
    }

    #[test]
    fn test_doc_all_known_but_inactive() {
        let v = json!({
            "page":  {"left-margin": "2ch", "right-margin": "4ch",
                       "top-margin": 1, "bottom-margin": 0},
            "table": {"alignment": "right", "max-width": "50%"},
            "ol":    {"alignment": "right"},
            "ul":    {"alignment": "left", "left-margin": "4ch", "max-width": "40"}
        });
        let (_, w) = from_json_value(&v).unwrap();
        let schema: Vec<_> = w.iter().filter(|w| w.is_schema_issue()).collect();
        assert!(schema.is_empty(), "should not produce schema warnings: {:?}", schema);
        let inactive: Vec<_> = w
            .iter()
            .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
            .collect();
        // 4 page + 2 table + 1 ol + 3 ul = 10 leaves.
        assert_eq!(inactive.len(), 10, "got {:?}", inactive);
    }
```

- [ ] **Step 4: Run the tests**

```bash
cargo nextest run -p darkmatter style::parse::tests
```

Expected: 10 tests passed.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/style/parse.rs
git commit -m "feat(darkmatter): KnownButInactive annotation for parsed style keys"
```

---

## Task 20: `from_frontmatter` + `into_strict`

**Files:**
- Modify: `darkmatter/lib/src/style/parse.rs`
- Modify: `darkmatter/lib/src/style/mod.rs` (export new entry points)

- [ ] **Step 1: Add the helpers + failing tests**

Append to `darkmatter/lib/src/style/parse.rs`, above the `tests` module:

```rust
use crate::markdown::Frontmatter;

/// Parse the `style:` value from a `Frontmatter`. Returns
/// `(StyleFrontmatter::default(), vec![])` when no `style:` key is present.
pub fn from_frontmatter(
    fm: &Frontmatter,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError> {
    match fm.as_map().get("style") {
        Some(value) => from_json_value(value),
        None => Ok((StyleFrontmatter::default(), Vec::new())),
    }
}

/// Promote schema-validation warnings (`UnknownKey`, `Deprecated`) to errors.
/// `KnownButInactive` warnings are deliberately ignored so a strict caller
/// does not fail on a forward-compatible document.
pub fn into_strict(
    parsed: (StyleFrontmatter, Vec<StyleWarning>),
) -> Result<StyleFrontmatter, StyleParseError> {
    let (style, warnings) = parsed;
    let schema: Vec<StyleWarning> =
        warnings.into_iter().filter(|w| w.is_schema_issue()).collect();
    if schema.is_empty() {
        Ok(style)
    } else {
        Err(StyleParseError::Strict { warnings: schema })
    }
}
```

Append to the `tests` module:

```rust
    use crate::markdown::Frontmatter;
    use serde_json::json;

    #[test]
    fn from_frontmatter_no_style_key_yields_default() {
        let fm = Frontmatter::new();
        let (s, w) = from_frontmatter(&fm).unwrap();
        assert_eq!(s, StyleFrontmatter::default());
        assert!(w.is_empty());
    }

    #[test]
    fn from_frontmatter_with_style_key() {
        let mut fm = Frontmatter::new();
        fm.insert("style", json!({"page": {"left-margin": "2ch"}})).unwrap();
        let (s, _w) = from_frontmatter(&fm).unwrap();
        assert!(s.page.is_some());
    }

    #[test]
    fn into_strict_passes_clean_parse() {
        let parsed = from_json_value(&json!({"page": {"left-margin": "2ch"}})).unwrap();
        // Only KnownButInactive warnings; strict should succeed.
        let s = into_strict(parsed).unwrap();
        assert!(s.page.is_some());
    }

    #[test]
    fn into_strict_fails_on_unknown_key() {
        let parsed =
            from_json_value(&json!({"page": {"lft-margin": "2ch"}})).unwrap();
        match into_strict(parsed) {
            Err(StyleParseError::Strict { warnings }) => {
                assert_eq!(warnings.len(), 1);
                assert_eq!(warnings[0].path, "style.page.lft-margin");
            }
            other => panic!("expected Strict error, got {:?}", other),
        }
    }

    #[test]
    fn into_strict_fails_on_deprecated_alias() {
        let parsed =
            from_json_value(&json!({"page": {"left_margin": "2ch"}})).unwrap();
        assert!(matches!(
            into_strict(parsed),
            Err(StyleParseError::Strict { .. })
        ));
    }

    #[test]
    fn into_strict_ignores_known_but_inactive() {
        // Document fully valid; every key emits KnownButInactive but strict
        // must still succeed.
        let parsed = from_json_value(&json!({
            "table": {"alignment": "right", "max-width": "50%"}
        }))
        .unwrap();
        assert!(into_strict(parsed).is_ok());
    }
```

- [ ] **Step 2: Re-export from style mod**

In `darkmatter/lib/src/style/mod.rs`:

```rust
pub use parse::{from_frontmatter, from_json_value, into_strict};
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p darkmatter style::parse::tests
```

Expected: 16 tests passed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/style/parse.rs darkmatter/lib/src/style/mod.rs
git commit -m "feat(darkmatter): from_frontmatter and into_strict entry points"
```

---

## Task 21: Integration test against `style-prop.md`

**Files:**
- Create: `darkmatter/lib/tests/style_frontmatter.rs`

- [ ] **Step 1: Write the integration test**

Create `darkmatter/lib/tests/style_frontmatter.rs`:

```rust
//! Integration: parse the user's example fixture
//! (`darkmatter/example-docs/rendering/style-prop.md`) and assert every
//! field in the spec's acceptance criteria.

use std::fs;

use darkmatter::markdown::Markdown;
use darkmatter::style::{StyleFrontmatter, from_frontmatter, into_strict};
use darkmatter::style::warning::StyleWarningKind;
use renderable::layout::{Alignment, Length};

const FIXTURE: &str = "example-docs/rendering/style-prop.md";

#[test]
fn fixture_parses_to_expected_style_frontmatter() {
    let raw = fs::read_to_string(FIXTURE).expect("read fixture");
    let md = Markdown::from_string(&raw).expect("parse markdown");

    let (style, warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    // Acceptance criteria: page.
    let page = style.page.expect("page bucket");
    assert_eq!(page.left_margin, Some(Length::Ch(2)));
    assert_eq!(page.right_margin, Some(Length::Ch(4)));
    assert_eq!(page.top_margin, Some(1));
    assert_eq!(page.bottom_margin, Some(0));

    // Acceptance criteria: table.
    let table = style.table.expect("table bucket");
    assert_eq!(table.common.alignment, Some(Alignment::Right));
    assert_eq!(table.common.max_width, Some(Length::Percent(50.0)));

    // Acceptance criteria: ol.
    let ol = style.ol.expect("ol bucket");
    assert_eq!(ol.common.alignment, Some(Alignment::Right));

    // Acceptance criteria: ul.
    let ul = style.ul.expect("ul bucket");
    assert_eq!(ul.common.alignment, Some(Alignment::Left));
    assert_eq!(ul.left_margin, Some(Length::Ch(4)));
    assert_eq!(ul.common.max_width, Some(Length::Ch(40)));

    // Other buckets must remain None.
    assert!(style.hyperlinks.is_none());
    assert!(style.images.is_none());
    assert!(style.hr.is_none());
    assert!(style.li.is_none());
    assert!(style.block_quote.is_none());

    // All warnings must be KnownButInactive (the fixture is schema-clean).
    let schema_issues: Vec<_> = warnings
        .iter()
        .filter(|w| w.is_schema_issue())
        .collect();
    assert!(
        schema_issues.is_empty(),
        "fixture should be schema-clean, got: {:?}",
        schema_issues
    );

    let inactive_count = warnings
        .iter()
        .filter(|w| matches!(w.kind, StyleWarningKind::KnownButInactive { .. }))
        .count();
    assert!(inactive_count > 0, "expected KnownButInactive warnings");
}

#[test]
fn fixture_passes_strict_validation() {
    // Strict mode succeeds because every warning is KnownButInactive.
    let raw = fs::read_to_string(FIXTURE).expect("read fixture");
    let md = Markdown::from_string(&raw).expect("parse markdown");
    let parsed = from_frontmatter(md.frontmatter()).expect("parse style");
    let _: StyleFrontmatter = into_strict(parsed).expect("strict should pass");
}
```

> **Worker note:** Confirm the exact `Markdown::from_string` API name. If
> `Markdown::from_string` does not exist, check
> `darkmatter/lib/src/markdown/mod.rs` for the canonical entry point
> (could be `Markdown::new(content)`, `Markdown::parse(content)`, or
> `Markdown::from(...)`). Use whichever the existing test files in
> `darkmatter/lib/tests/` use.

- [ ] **Step 2: Run the integration test**

```bash
cargo nextest run -p darkmatter --test style_frontmatter
```

Expected: 2 tests passed.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/lib/tests/style_frontmatter.rs
git commit -m "test(darkmatter): integration test for style frontmatter parser against fixture"
```

---

## Task 22: Module rustdoc + run full suite + verify docs

**Files:**
- Modify: `darkmatter/lib/src/style/mod.rs` (final rustdoc)

- [ ] **Step 1: Finalize the module-level rustdoc**

Replace the top of `darkmatter/lib/src/style/mod.rs` with this rustdoc and
re-export block:

```rust
//! Frontmatter `style:` parser for darkmatter documents.
//!
//! Lowers a user-authored YAML `style:` block into a typed sparse tree
//! ([`StyleFrontmatter`]) whose length, alignment, and color values are
//! [`renderable`] runtime types. No rendering is performed by this module;
//! sub-specs #2..#7 (see
//! `renderable/features/_unscheduled/style-property/spec.md`) consume the
//! parsed value to drive `DarkmatterPage`.
//!
//! ## Examples
//!
//! ```no_run
//! use darkmatter::markdown::Markdown;
//! use darkmatter::style::{from_frontmatter, into_strict};
//!
//! let md = Markdown::from_string("---\nstyle:\n  page:\n    left-margin: 2ch\n---\n").unwrap();
//! let (style, warnings) = from_frontmatter(md.frontmatter()).unwrap();
//!
//! // Schema-strict validation: succeeds on documents whose only warnings
//! // are `KnownButInactive`.
//! let _ = into_strict((style, warnings));
//! ```
//!
//! ## Notes
//!
//! Frontmatter values are stored as `serde_json::Value` (the canonical
//! `Frontmatter` map representation in darkmatter). YAML text parsing
//! happens upstream via `biscuit_file::serde_yaml_ng` and is not the
//! concern of this module.
//!
//! Diagnostics carry an optional [`StyleSpan`] for source positions; v1
//! always sets it to `None`, but the field exists so later sub-specs can
//! populate it without changing the public surface.

pub mod alignment;
pub mod color;
pub mod descriptor;
pub mod error;
pub mod length;
pub mod parse;
pub mod schema;
pub mod walker;
pub mod warning;

pub use color::StyleColor;
pub use error::StyleParseError;
pub use parse::{from_frontmatter, from_json_value, into_strict};
pub use schema::StyleFrontmatter;
pub use warning::{StyleSpan, StyleWarning, StyleWarningKind};
```

- [ ] **Step 2: Run the full darkmatter test suite**

```bash
cargo nextest run -p darkmatter --no-fail-fast
```

Expected: every prior test still passes; new style tests pass; nothing else
regresses.

- [ ] **Step 3: Confirm rustdoc is warning-free for the new module**

```bash
cargo doc -p darkmatter --no-deps 2>&1 | grep -E "(warning|error).*style" | head
```

Expected: empty output (no new warnings mentioning `style`).

- [ ] **Step 4: Confirm clippy is clean**

```bash
cargo clippy -p darkmatter --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: no warnings, no errors.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/style/mod.rs
git commit -m "docs(darkmatter): module rustdoc for style frontmatter parser"
```

---

## Final Verification

After all 22 tasks land, verify the full acceptance criteria from
[`spec.md`](./spec.md#acceptance-criteria):

- [ ] `darkmatter::style::from_frontmatter(md.frontmatter())` callable from any workspace member depending on `darkmatter`.
- [ ] Parsing `darkmatter/example-docs/rendering/style-prop.md` returns the expected `StyleFrontmatter` (verified by Task 21 integration test).
- [ ] All warnings from the fixture parse are `KnownButInactive` (Task 21).
- [ ] All 14 tests from the spec's **Tests** section have peer tests in the implementation (Tasks 4–13, 17, 18, 19, 20, 21).
- [ ] `cargo doc -p darkmatter --no-deps` produces no new warnings (Task 22).
- [ ] No existing darkmatter test regresses (Task 22).
- [ ] `md` CLI behavior unchanged.
- [ ] No second runtime model: `StyleColor.color` is `renderable::color::Color`; every length is `renderable::layout::Length`; every alignment is `renderable::layout::Alignment`. Verified in Tasks 9, 10, 12.
- [ ] Snake-case spellings accepted via `serde(alias)` and emit `Deprecated` warnings. Verified in Tasks 9, 17, 20.
- [ ] Unknown-key detection through `flatten` (Task 17 test `flatten_typo_detected`) and through nested `local-style` (Task 17 test `nested_local_style_typo_detected`).
- [ ] Parser operates on `serde_json::Value` (Tasks 18–20). YAML text parsing not introduced.
- [ ] Strict mode succeeds on documents whose only warnings are `KnownButInactive` (Task 20 test `into_strict_ignores_known_but_inactive`; Task 21 test `fixture_passes_strict_validation`).

---

## Self-Review Checklist

- [x] **Spec coverage:** every acceptance criterion in spec.md is exercised by at least one task. See "Final Verification" mapping.
- [x] **Placeholder scan:** every code step shows the actual code. The Tailwind family-level table in Task 7 references the pattern + names the file to expand against rather than burying ~200 trivial match arms in this plan.
- [x] **Type consistency:** function names (`from_json_value`, `from_frontmatter`, `into_strict`), struct names (`StyleFrontmatter`, `StyleColor`, `StyleWarning`, `StyleSpan`), and field names (`left_margin`, `block_quote`, `local_style`, `bg_color`) are consistent across every task.
