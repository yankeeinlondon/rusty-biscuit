//! Type-safe CSS declaration container.
//!
//! A [`CssStyle`] is an ordered list of `property: value` declarations.
//! Insertion preserves order; re-adding an existing property replaces the
//! previous value in place without changing position. There is no concept of
//! selectors or rules — this is a flat declaration block, suitable for use as
//! an inline style attribute or as the body of a single rule.
//!
//! ## Construction modes
//!
//! - **Typed (preferred)** — chain [`CssStyle::add`] calls with a typed
//!   property and its matching value type. Mistypes are compile-time errors.
//! - **Dynamic** — use [`CssStyle::try_add`] with [`CssProp`] + [`CssValue`]
//!   for input whose property/value pairing isn't known at compile time.
//! - **Parsing** — [`CssStyle::try_from`] accepts a CSS-like declaration
//!   block (optionally wrapped in `{ }`).
//!
//! ## Rendering targets
//!
//! | Method                                | Output                              |
//! |---------------------------------------|-------------------------------------|
//! | [`to_css`](crate::stylesheet::CssStyle::to_css) / [`Display`] | Newline-joined `property: value;` declarations |
//! | [`to_json`](crate::stylesheet::CssStyle::to_json)   | Pretty JSON object (integers preserved) |
//! | [`to_json5`](crate::stylesheet::CssStyle::to_json5) | JSON5 with unquoted keys + trailing commas |
//!
//! ## Examples
//!
//! ```rust
//! use renderable::stylesheet::{CssColor, CssColorProp, CssSizing, CssSizingProp, CssStyle};
//!
//! let sheet = CssStyle::new()
//!     .add(CssSizingProp::Width, CssSizing::px(320.0))
//!     .add(CssColorProp::Color, CssColor::hex("#336699").unwrap());
//!
//! assert_eq!(sheet.len(), 2);
//! assert!(sheet.to_css().contains("width: 320px"));
//! ```

use std::fmt::{self, Display};

use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::stylesheet::error::StylesheetError;
use crate::stylesheet::prop::{CssProp, CssTypedProperty};
use crate::stylesheet::value::strip_surrounding_braces;
use crate::stylesheet::value::{
    CssColor, CssRaw, CssSizing, CssSizingMulti, CssValue, CssValueKind, IntoCssValue,
};

/// Internal storage unit for [`CssStyle`]: a single
/// `property: value` pair preserved in insertion order.
#[derive(Debug, Clone, PartialEq)]
struct CssDeclaration {
    prop: CssProp,
    value: CssValue,
}

/// Type-safe CSS declaration container.
///
/// A `CssStyle` is an ordered list of `property: value` declarations.
/// Insertion preserves order; re-adding an existing property replaces the
/// previous value in place without changing position. There is no concept of
/// selectors or rules — this is a flat declaration block, suitable for use as
/// an inline style attribute or as the body of a single rule.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CssStyle {
    declarations: Vec<CssDeclaration>,
}

impl CssStyle {
    /// Creates an empty style with no declarations.
    ///
    /// Equivalent to [`CssStyle::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of declarations currently stored.
    pub fn len(&self) -> usize {
        self.declarations.len()
    }

    /// Returns `true` when the style has no declarations.
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty()
    }

    /// Adds (or replaces) a typed property/value declaration and returns
    /// `self` for chaining.
    ///
    /// The signature is constrained by [`CssTypedProperty`]: each typed
    /// property subset (e.g. [`CssSizingProp`](crate::stylesheet::CssSizingProp),
    /// [`CssColorProp`](crate::stylesheet::CssColorProp)) only
    /// accepts its matching value type. Trying to pair, say, a
    /// [`CssSizingProp`](crate::stylesheet::CssSizingProp) with a [`CssColor`]
    /// is a compile-time error.
    ///
    /// If a declaration for the same [`CssProp`] already exists, its value is
    /// updated in place (insertion order is preserved). New properties are
    /// appended.
    ///
    /// ## Panics
    ///
    /// Panics in debug builds via `debug_assert!` if the value's kind does
    /// not match the property's [`CssProp::expected_kind`]. In release
    /// builds the assertion is elided; the type system already prevents the
    /// mismatch for typed properties, so this assertion exists only as
    /// defense-in-depth against bugs in [`CssTypedProperty`] impls.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use renderable::stylesheet::{CssIntegerProp, CssStyle};
    ///
    /// let sheet = CssStyle::new()
    ///     .add(CssIntegerProp::ZIndex, 1)
    ///     .add(CssIntegerProp::ZIndex, 10); // replaces in place
    /// assert_eq!(sheet.to_css(), "z-index: 10;");
    /// ```
    pub fn add<T, U>(mut self, prop: T, value: U) -> Self
    where
        T: CssTypedProperty<Value = U>,
        U: IntoCssValue,
    {
        let prop = prop.into_css_prop();
        let value = value.into_css_value();

        debug_assert!(
            is_value_allowed_for_prop(&prop, &value),
            "typed property/value pair should always be valid"
        );

        self.insert_or_replace(prop, value);
        self
    }

    /// Adds (or replaces) a dynamic property/value declaration with runtime
    /// validation.
    ///
    /// Use this when working with already-erased [`CssProp`] / [`CssValue`]
    /// values — typically when parsing user input or operating on data of
    /// unknown shape. For statically known pairings, [`CssStyle::add`] is
    /// stricter and infallible.
    ///
    /// Replacement semantics are identical to [`CssStyle::add`].
    ///
    /// ## Errors
    ///
    /// Returns [`StylesheetError::PropertyValueTypeMismatch`] when the
    /// property has a known [`CssProp::expected_kind`] that disagrees with
    /// the value's [`CssValue::kind`]. Properties whose `expected_kind`
    /// is `None` (i.e. [`CssProp::Other`]) accept any value.
    pub fn try_add(mut self, prop: CssProp, value: CssValue) -> Result<Self, StylesheetError> {
        ensure_valid_property_value_pair(&prop, &value)?;
        self.insert_or_replace(prop, value);
        Ok(self)
    }

    /// Renders the style as plain CSS declarations.
    ///
    /// Each declaration is emitted on its own line in `property: value;`
    /// form, joined by `\n` (no leading or trailing whitespace). Returns an
    /// empty string when the style has no declarations.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use renderable::stylesheet::{CssIntegerProp, CssSizing, CssSizingProp, CssStyle};
    ///
    /// let sheet = CssStyle::new()
    ///     .add(CssSizingProp::Width, CssSizing::px(80.0))
    ///     .add(CssIntegerProp::ZIndex, 5);
    /// assert_eq!(sheet.to_css(), "width: 80px;\nz-index: 5;");
    /// ```
    pub fn to_css(&self) -> String {
        self.declarations
            .iter()
            .map(|decl| {
                let name = decl.prop.css_name();
                format!("{name}: {};", decl.value)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Renders declarations as pretty-printed JSON.
    ///
    /// The output is a JSON object whose keys are CSS property names and
    /// whose values are:
    ///
    /// - JSON numbers for [`CssValue::Integer`] declarations.
    /// - JSON strings (the [`Display`] form of the value) for everything else.
    ///
    /// Declaration order is preserved by [`serde_json::Map`] when the
    /// `preserve_order` feature is enabled in `serde_json`; otherwise key
    /// order follows that implementation's default. Returns `"{}"` for an
    /// empty style (and as a fallback if serialization fails — which
    /// is unreachable in practice since all values stringify cleanly).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use renderable::stylesheet::{CssIntegerProp, CssSizing, CssSizingProp, CssStyle};
    ///
    /// let sheet = CssStyle::new()
    ///     .add(CssSizingProp::Width, CssSizing::rem(42.0))
    ///     .add(CssIntegerProp::ZIndex, 9);
    /// let json = sheet.to_json();
    /// assert!(json.contains("\"width\": \"42rem\""));
    /// assert!(json.contains("\"z-index\": 9"));
    /// ```
    pub fn to_json(&self) -> String {
        let mut map = Map::new();
        for decl in &self.declarations {
            map.insert(
                decl.prop.css_name().into_owned(),
                json_value_for_css_value(&decl.value),
            );
        }

        match serde_json::to_string_pretty(&Value::Object(map)) {
            Ok(json) => json,
            Err(_) => "{}".to_string(),
        }
    }

    /// Renders declarations as JSON5.
    ///
    /// JSON5 differs from JSON in two ways here:
    ///
    /// - Keys are emitted **unquoted** when they pass `is_json5_identifier`
    ///   (e.g. `width`, `zIndex`). Keys containing hyphens or other
    ///   characters that disqualify them as identifiers are JSON-quoted
    ///   (e.g. `"z-index"`).
    /// - Each declaration ends with a **trailing comma** (legal in JSON5).
    ///
    /// Value typing matches [`CssStyle::to_json`]: integers are emitted as
    /// JSON numbers, everything else is emitted as a quoted string.
    ///
    /// Returns `"{}"` for an empty style.
    pub fn to_json5(&self) -> String {
        if self.declarations.is_empty() {
            return "{}".to_string();
        }

        let mut lines = Vec::with_capacity(self.declarations.len());
        for decl in &self.declarations {
            let key = json5_key(&decl.prop.css_name());
            let value = json5_value(&decl.value);
            lines.push(format!("  {key}: {value},"));
        }

        format!("{{\n{}}}\n", lines.join("\n"))
    }

    /// Inserts a new declaration or, if a declaration with the same
    /// [`CssProp`] already exists, replaces its value in place.
    ///
    /// Preserves insertion order: replacing a value keeps the declaration's
    /// position; only genuinely new properties append to the end.
    fn insert_or_replace(&mut self, prop: CssProp, value: CssValue) {
        if let Some(existing) = self.declarations.iter_mut().find(|decl| decl.prop == prop) {
            existing.value = value;
            return;
        }

        self.declarations.push(CssDeclaration { prop, value });
    }
}

/// Delegates to [`CssStyle::to_css`]. Use `{style}` in format strings
/// to emit the plain CSS form.
impl Display for CssStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_css())
    }
}

/// Serializes the style as its canonical declaration string (the
/// [`to_css`](CssStyle::to_css) form), never as the internal declaration
/// storage. The string round-trips through [`Deserialize`] via
/// [`CssStyle::try_from`].
impl Serialize for CssStyle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_css())
    }
}

/// Deserializes a style from its canonical declaration string, validating it
/// through [`CssStyle::try_from`]. An invalid declaration block is a
/// deserialization error rather than a silently-empty style.
impl<'de> Deserialize<'de> for CssStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let declarations = String::deserialize(deserializer)?;
        CssStyle::try_from(declarations.as_str()).map_err(de::Error::custom)
    }
}

/// Parses a style from a CSS-like declaration block.
///
/// Accepted shapes:
///
/// - A bare list of declarations: `"color: red; margin: 8px 16px;"`
/// - The same list wrapped in braces: `"{ color: red; margin: 8px 16px; }"`
///
/// Trailing semicolons are optional; declarations are separated by top-level
/// `;` (parenthesized content is preserved so `calc(...)` and `rgba(...)`
/// arguments are not split). Empty declarations between `;` characters are
/// skipped.
///
/// Each declaration is split on the first `:`. The property name is parsed
/// via [`CssProp::from_css_name`]; the value is parsed via
/// `parse_value_for_prop`, which dispatches based on the property's
/// [`CssProp::expected_kind`]. Properties with no known kind (i.e.
/// [`CssProp::Other`]) accept any non-empty raw value not containing `;`.
///
/// ## Errors
///
/// Bubbles up the first [`StylesheetError`] returned by any of:
///
/// - [`CssProp::from_css_name`] (invalid property name)
/// - the per-kind value parser (`CssSizing::try_from`, `CssColor::try_from`,
///   `i32::parse`, `CssRaw::new`, ...)
/// - [`CssStyle::try_add`] (type mismatch, though this can only fire when
///   `parse_value_for_prop` returns a value of the wrong kind, which is
///   unreachable in normal use).
///
/// Returns [`StylesheetError::InvalidDeclaration`] when a declaration does
/// not contain `:`.
impl TryFrom<&str> for CssStyle {
    type Error = StylesheetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut style = CssStyle::new();
        let content = strip_surrounding_braces(value.trim());

        for declaration in split_declarations(content) {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }

            let (prop_text, value_text) =
                declaration
                    .split_once(':')
                    .ok_or_else(|| StylesheetError::InvalidDeclaration {
                        declaration: declaration.to_string(),
                    })?;

            let prop = CssProp::from_css_name(prop_text)?;
            let value = parse_value_for_prop(&prop, value_text.trim())?;
            style = style.try_add(prop, value)?;
        }

        Ok(style)
    }
}

/// Borrowed-string convenience wrapper over [`TryFrom<&str> for CssStyle`].
impl TryFrom<&String> for CssStyle {
    type Error = StylesheetError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Owned-string convenience wrapper over [`TryFrom<&str> for CssStyle`].
impl TryFrom<String> for CssStyle {
    type Error = StylesheetError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Parses a value string into the [`CssValue`] variant expected by `prop`.
///
/// Dispatches on [`CssProp::expected_kind`]:
/// - `Sizing` → [`CssSizing::try_from`]
/// - `SizingMulti` → [`CssSizingMulti::try_from`]
/// - `Color` → [`CssColor::try_from`]
/// - `Integer` → [`i32`]'s [`std::str::FromStr`] impl (wrapped in [`StylesheetError::InvalidInteger`])
/// - `Raw` or `None` (unknown property) → [`CssRaw::new`]
fn parse_value_for_prop(prop: &CssProp, value: &str) -> Result<CssValue, StylesheetError> {
    match prop.expected_kind() {
        Some(CssValueKind::Sizing) => Ok(CssSizing::try_from(value)?.into_css_value()),
        Some(CssValueKind::SizingMulti) => Ok(CssSizingMulti::try_from(value)?.into_css_value()),
        Some(CssValueKind::Color) => Ok(CssColor::try_from(value)?.into_css_value()),
        Some(CssValueKind::Integer) => {
            let parsed =
                value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| StylesheetError::InvalidInteger {
                        value: value.to_string(),
                    })?;
            Ok(parsed.into_css_value())
        }
        Some(CssValueKind::Raw) | None => Ok(CssRaw::new(value)?.into_css_value()),
    }
}

/// Validates that a value's [`CssValueKind`] matches its property's expected
/// kind, returning [`StylesheetError::PropertyValueTypeMismatch`] on conflict.
///
/// Properties with no known kind (i.e. [`CssProp::Other`]) accept any value.
fn ensure_valid_property_value_pair(
    prop: &CssProp,
    value: &CssValue,
) -> Result<(), StylesheetError> {
    if let Some(expected_kind) = prop.expected_kind() {
        let actual_kind = value.kind();
        if expected_kind != actual_kind {
            return Err(StylesheetError::PropertyValueTypeMismatch {
                property: prop.css_name().into_owned(),
                expected: expected_kind,
                actual: actual_kind,
                value: value.to_string(),
            });
        }
    }

    Ok(())
}

/// Boolean form of `ensure_valid_property_value_pair` used by the
/// debug-time assertion in [`CssStyle::add`].
fn is_value_allowed_for_prop(prop: &CssProp, value: &CssValue) -> bool {
    match prop.expected_kind() {
        Some(expected) => expected == value.kind(),
        None => true,
    }
}

/// Maps a [`CssValue`] to a [`serde_json::Value`] for [`CssStyle::to_json`].
/// Integers become JSON numbers; all other variants stringify via [`Display`].
fn json_value_for_css_value(value: &CssValue) -> Value {
    match value {
        CssValue::Integer(number) => Value::from(*number),
        _ => Value::from(value.to_string()),
    }
}

/// Emits a JSON5 object key: unquoted if it qualifies as an identifier per
/// `is_json5_identifier`, otherwise JSON-quoted.
fn json5_key(key: &str) -> String {
    if is_json5_identifier(key) {
        key.to_string()
    } else {
        json_quote(key)
    }
}

/// Emits a JSON5 value: bare number for [`CssValue::Integer`], quoted string
/// for everything else.
fn json5_value(value: &CssValue) -> String {
    match value {
        CssValue::Integer(number) => number.to_string(),
        _ => json_quote(&value.to_string()),
    }
}

/// JSON-escapes `value` and surrounds it with `"..."`.
///
/// Uses [`serde_json`] for proper Unicode escaping. The fallback branch
/// (manual escape of `\` and `"`) is unreachable in practice but provides a
/// safe default if `serde_json` ever rejects the input.
fn json_quote(value: &str) -> String {
    match serde_json::to_string(value) {
        Ok(serialized) => serialized,
        Err(_) => {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        }
    }
}

/// Returns `true` when `value` matches the JSON5 identifier grammar:
/// starts with `_`, `$`, or an ASCII letter, followed by any number of
/// `_`/`$`/ASCII alphanumerics. CSS property names with hyphens (`z-index`,
/// `margin-top`) return `false` and must be quoted in JSON5 output.
fn is_json5_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

/// Splits a CSS declaration block on top-level `;` characters.
///
/// Tracks parenthesis depth so that `;` inside `calc(...)`, `rgba(...)`, etc.
/// are not treated as declaration separators (CSS does not actually allow
/// `;` inside function arguments, but this defensive behavior keeps malformed
/// input from producing spurious extra tokens). Empty segments are skipped.
fn split_declarations(input: &str) -> Vec<String> {
    let mut declarations = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ';' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    declarations.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        declarations.push(trimmed.to_string());
    }

    declarations
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    use crate::stylesheet::{CssColor, CssColorProp, CssSizing, CssSizingProp};

    fn sample() -> CssStyle {
        CssStyle::new()
            .add(CssSizingProp::Width, CssSizing::px(320.0))
            .add(CssColorProp::Color, CssColor::rgb(0x33, 0x66, 0x99))
    }

    #[test]
    fn serializes_as_canonical_declaration_string() {
        let json = serde_json::to_string(&sample()).unwrap();
        assert_eq!(json, r#""width: 320px;\ncolor: rgb(51, 102, 153);""#);
    }

    #[test]
    fn deserializes_through_try_from() {
        let style: CssStyle =
            serde_json::from_str(r#""color: #336699; margin: 8px 16px;""#).unwrap();
        assert_eq!(style.len(), 2);
    }

    #[test]
    fn roundtrip_is_deterministic() {
        let original = sample();
        let json = serde_json::to_string(&original).unwrap();
        let back: CssStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
        // Re-serializing produces byte-identical output.
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
    }

    #[test]
    fn invalid_css_is_rejected() {
        // No `:` separator → InvalidDeclaration surfaces as a serde error.
        assert!(serde_json::from_str::<CssStyle>(r#""invalid-syntax""#).is_err());
        // A known property paired with an unparsable value is also rejected.
        assert!(serde_json::from_str::<CssStyle>(r#""width: notasize""#).is_err());
    }

    #[test]
    fn ordering_and_replacement_are_preserved() {
        let style = CssStyle::new()
            .add(CssSizingProp::Width, CssSizing::px(10.0))
            .add(CssColorProp::Color, CssColor::rgb(1, 2, 3))
            .add(CssSizingProp::Width, CssSizing::px(20.0)); // replaces in place
        let json = serde_json::to_string(&style).unwrap();
        let back: CssStyle = serde_json::from_str(&json).unwrap();
        // Width keeps its first position with the replaced value; color follows.
        assert_eq!(back.to_css(), "width: 20px;\ncolor: rgb(1, 2, 3);");
    }
}
