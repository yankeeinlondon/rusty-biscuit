//! Pins the single array rendering at every expression string-output boundary.
//!
//! A bare array embedded in text renders as compact JSON, identically to the
//! `+` operator's rendering. Typed whole-value boundaries are unaffected and
//! continue to resolve to `Value::Array`.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use serde_json::{Value, json};

fn compose(source: &str) -> darkmatter::markdown::Markdown {
    let markdown: Markdown = source.to_string().into();
    let (composed, _report) = markdown
        .compose_with(ComposeOptions::new())
        .expect("compose pipeline should succeed");
    composed
}

fn frontmatter_value(composed: &Markdown, key: &str) -> Value {
    composed
        .frontmatter()
        .as_map()
        .get(key)
        .cloned()
        .unwrap_or(Value::Null)
}

#[test]
fn bare_array_in_body_renders_compact_json() {
    let composed = compose("---\nitems:\n  - a\n  - b\n  - c\n---\n\n{{ items }}\n");

    assert!(
        composed.content().contains(r#"["a","b","c"]"#),
        "bare body interpolation of an array must render compact JSON, got: {}",
        composed.content()
    );
}

/// The two renderers must agree, asserted against each other rather than
/// against a shared literal, so neither can drift independently.
#[test]
fn plus_path_and_interpolation_path_render_arrays_identically() {
    let composed = compose(
        "---\nitems:\n  - a\n  - \"b \\\"quoted\\\"\"\n  - 3\n---\n\nBARE:{{ items }}:END\nPLUS:{{ \"\" + items }}:END\n",
    );
    let body = composed.content();

    let extract = |marker: &str| -> String {
        let start = body
            .find(marker)
            .unwrap_or_else(|| panic!("missing {marker} marker in body: {body}"))
            + marker.len();
        let rest = &body[start..];
        let end = rest
            .find(":END")
            .unwrap_or_else(|| panic!("missing :END terminator in body: {body}"));
        rest[..end].to_string()
    };

    let bare = extract("BARE:");
    let plus = extract("PLUS:");

    assert_eq!(
        bare, plus,
        "the interpolation path and the `+` path must render the same array identically"
    );
}

/// Objects already rendered as JSON on both paths; this proves the fix reached
/// arrays only.
#[test]
fn bare_object_rendering_is_unchanged() {
    let composed = compose("---\nconfig:\n  a: 1\n  b: two\n---\n\n{{ config }}\n");

    assert!(
        composed.content().contains(r#"{"a":1,"b":"two"}"#),
        "object rendering must be unchanged, got: {}",
        composed.content()
    );
}

/// An exact whole-value interpolation is a typed boundary: it must keep the
/// array rather than stringifying it.
#[test]
fn exact_whole_value_frontmatter_array_stays_typed() {
    let composed = compose("---\nitems:\n  - a\n  - b\nexact: \"{{ items }}\"\n---\n");

    assert_eq!(
        frontmatter_value(&composed, "exact"),
        json!(["a", "b"]),
        "an exact whole-value frontmatter interpolation must stay a typed array"
    );
}

/// The same array inside a mixed string is a text boundary and renders JSON.
#[test]
fn mixed_frontmatter_string_renders_compact_json() {
    let composed = compose("---\nitems:\n  - a\n  - b\nmixed: \"values: {{ items }}\"\n---\n");

    assert_eq!(
        frontmatter_value(&composed, "mixed"),
        json!(r#"values: ["a","b"]"#),
        "a mixed frontmatter string must render the array as compact JSON"
    );
}

/// `as_json(x)` is the explicit spelling of the new default, so at the
/// document boundary it must be byte-identical to bare interpolation of `x`.
#[test]
fn as_json_is_byte_identical_to_bare_interpolation() {
    let composed = compose(
        "---\nitems:\n  - a\n  - \"b \\\"quoted\\\"\"\n  - 3\n  - [1, 2]\nbare: \"{{ '' + items }}\"\nexplicit: \"{{ as_json(items) }}\"\n---\n",
    );

    assert_eq!(
        frontmatter_value(&composed, "bare"),
        frontmatter_value(&composed, "explicit"),
        "as_json must be byte-identical to the bare-array default"
    );
}

/// The migration path must be real: the newline-joined form still exists.
#[test]
fn as_line_separated_still_joins_with_newlines() {
    let composed = compose(
        "---\nitems:\n  - a\n  - b\njoined: \"OUT:{{ as_line_separated(items) }}:END\"\n---\n",
    );

    assert_eq!(
        frontmatter_value(&composed, "joined"),
        json!("OUT:a\nb:END"),
        "as_line_separated must remain newline-joined"
    );
}
