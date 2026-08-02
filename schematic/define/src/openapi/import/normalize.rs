//! Source-text normalization applied before an OpenAPI document is deserialized.
//!
//! Some published specifications carry numeric schema bounds that no Rust
//! integer type can hold. Deserialization of the whole document fails on the
//! first such value, so the repair has to happen on the raw text.

use super::diagnostics::OpenApiDiagnostic;

/// Schema keywords whose values are clamped into `i64` range.
///
/// Restricted to these five because `openapiv3::IntegerType` types
/// `minimum`, `maximum`, and `multiple_of` as `Option<i64>`; a value outside
/// that range is unrepresentable no matter how the document is parsed.
/// Integers under any other key are left alone, where a widening `u64` is
/// still meaningful and a blanket clamp would silently corrupt data.
const CLAMPED_KEYS: [&str; 5] = [
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
];

/// Clamps out-of-`i64`-range integer schema bounds in raw specification text.
///
/// Returns the (possibly rewritten) text and one warning diagnostic per
/// clamped value. Text without such bounds is returned unchanged.
///
/// Handles both YAML (`maximum: 123`) and JSON (`"maximum": 123,`) spellings,
/// preserving indentation, trailing commas, and trailing comments.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::clamp_numeric_bounds;
///
/// let (text, diagnostics) = clamp_numeric_bounds("            maximum: 9223372036854776000");
///
/// assert_eq!(text, "            maximum: 9223372036854775807");
/// assert_eq!(diagnostics.len(), 1);
/// ```
///
/// ## Notes
///
/// OpenAI's published specification is the motivating case: it writes the
/// `seed` bounds as `-9223372036854776000` / `9223372036854776000`, the `f64`
/// roundings of `i64::MIN` / `i64::MAX`. `serde_yaml_ng` routes integers
/// outside `i64` to `Visitor::visit_i128`, which its own `Value` visitor does
/// not implement, so even a permissive parse into `Value` fails.
pub fn clamp_numeric_bounds(text: &str) -> (String, Vec<OpenApiDiagnostic>) {
    let mut diagnostics = Vec::new();

    // A document with no candidate keyword cannot need rewriting, and specs are
    // large enough (OpenAI's is 2.8 MB) that the line scan is worth skipping.
    if !CLAMPED_KEYS.iter().any(|key| text.contains(key)) {
        return (text.to_string(), diagnostics);
    }

    let mut out = String::with_capacity(text.len());
    let mut first = true;

    for (index, line) in text.lines().enumerate() {
        if !first {
            out.push('\n');
        }
        first = false;

        match clamp_line(line) {
            Some((rewritten, key, original, clamped)) => {
                diagnostics.push(OpenApiDiagnostic::warn(
                    format!("line {}", index + 1),
                    format!(
                        "`{key}` value {original} is outside i64 range; clamped to {clamped}"
                    ),
                ));
                out.push_str(&rewritten);
            }
            None => out.push_str(line),
        }
    }

    if text.ends_with('\n') {
        out.push('\n');
    }

    (out, diagnostics)
}

/// Rewrites a single line, yielding `(line, key, original, clamped)` on a hit.
fn clamp_line(line: &str) -> Option<(String, &'static str, String, i64)> {
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);

    let (key, after_key) = match_key(rest)?;
    let (value, suffix) = split_value(after_key);

    let literal = value.trim();
    if !is_integer_literal(literal) {
        return None;
    }

    // Only literals that overflow i64 are touched; in-range values must round-trip
    // byte-for-byte so ordinary specs are unaffected.
    if literal.parse::<i64>().is_ok() {
        return None;
    }

    let clamped = if literal.starts_with('-') {
        i64::MIN
    } else {
        i64::MAX
    };

    // Rebuilt piecewise rather than by substitution so the line round-trips
    // byte-for-byte apart from the digits themselves.
    let key_part = &rest[..rest.len() - after_key.len()];
    let leading = &value[..value.len() - value.trim_start().len()];
    let trailing = &value[value.len() - (value.trim_start().len() - literal.len())..];

    let rewritten = format!("{indent}{key_part}{leading}{clamped}{trailing}{suffix}");
    Some((rewritten, key, literal.to_string(), clamped))
}

/// Matches a leading `key:` or `"key":`, returning the key and the text after the colon.
fn match_key(rest: &str) -> Option<(&'static str, &str)> {
    for key in CLAMPED_KEYS {
        for prefix in [key.to_string(), format!("\"{key}\"")] {
            if let Some(tail) = rest.strip_prefix(&prefix)
                && let Some(after) = tail.strip_prefix(':')
            {
                return Some((key, after));
            }
        }
    }
    None
}

/// Splits the text after a colon into its scalar value and trailing suffix.
///
/// The suffix carries a JSON trailing comma or a YAML `#` comment so the
/// rewritten line keeps them.
fn split_value(after_key: &str) -> (&str, &str) {
    let end = after_key
        .find('#')
        .into_iter()
        .chain(after_key.rfind(',').filter(|index| {
            // A trailing comma belongs to the enclosing JSON object, not the number.
            after_key[index + 1..].trim().is_empty()
        }))
        .min()
        .unwrap_or(after_key.len());

    after_key.split_at(end)
}

/// Reports whether a scalar is a bare decimal integer.
///
/// Floats, booleans, quoted scalars, and `null` are excluded, so only values
/// that `openapiv3` would decode into `i64` reach the clamp.
fn is_integer_literal(value: &str) -> bool {
    let digits = value.strip_prefix(['-', '+']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_documents_without_bounds_untouched() {
        let text = "openapi: 3.1.0\ninfo:\n  title: Test\n";
        let (out, diagnostics) = clamp_numeric_bounds(text);

        assert_eq!(out, text);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn leaves_in_range_bounds_untouched() {
        let text = "  minimum: -100\n  maximum: 9223372036854775807\n";
        let (out, diagnostics) = clamp_numeric_bounds(text);

        assert_eq!(out, text);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn clamps_the_openai_seed_bounds() {
        let text = "              minimum: -9223372036854776000\n              maximum: 9223372036854776000\n";
        let (out, diagnostics) = clamp_numeric_bounds(text);

        assert_eq!(
            out,
            "              minimum: -9223372036854775808\n              maximum: 9223372036854775807\n"
        );
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].location, "line 1");
        assert!(diagnostics[0].message.contains("clamped to"));
    }

    #[test]
    fn clamps_json_spelling_and_keeps_trailing_comma() {
        let text = "    \"maximum\": 9223372036854776000,\n";
        let (out, _) = clamp_numeric_bounds(text);

        assert_eq!(out, "    \"maximum\": 9223372036854775807,\n");
    }

    #[test]
    fn keeps_trailing_comments() {
        let text = "  minimum: -9223372036854776000 # i64::MIN\n";
        let (out, _) = clamp_numeric_bounds(text);

        assert_eq!(out, "  minimum: -9223372036854775808 # i64::MIN\n");
    }

    #[test]
    fn ignores_non_integer_bound_values() {
        let text = "  exclusiveMinimum: true\n  minimum: 1.5\n  maximum: null\n";
        let (out, diagnostics) = clamp_numeric_bounds(text);

        assert_eq!(out, text);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn ignores_out_of_range_integers_under_other_keys() {
        let text = "  default: 99999999999999999999\n";
        let (out, diagnostics) = clamp_numeric_bounds(text);

        assert_eq!(out, text);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn does_not_match_keys_that_merely_share_a_prefix() {
        let text = "  minimumLength: 99999999999999999999\n";
        let (out, diagnostics) = clamp_numeric_bounds(text);

        assert_eq!(out, text);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn preserves_absence_of_a_trailing_newline() {
        let text = "  maximum: 9223372036854776000";
        let (out, _) = clamp_numeric_bounds(text);

        assert!(!out.ends_with('\n'));
        assert_eq!(out, "  maximum: 9223372036854775807");
    }
}
