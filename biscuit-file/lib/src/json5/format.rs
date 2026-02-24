//! JSON5 formatter that outputs idiomatic JSON5 syntax.
//!
//! - Unquoted object keys when they are valid ECMAScript 5 identifiers
//! - Single-quoted strings
//! - Pretty mode: trailing commas, 2-space indentation, newlines
//! - Compact mode: single line, no trailing commas, minimal whitespace

use serde_json::Value;

/// Format a `serde_json::Value` as a pretty-printed JSON5 string.
pub fn to_json5_pretty(value: &Value) -> String {
    let mut buf = String::new();
    write_value(&mut buf, value, 0, false);
    buf.push('\n');
    buf
}

/// Format a `serde_json::Value` as a compact single-line JSON5 string.
pub fn to_json5_compact(value: &Value) -> String {
    let mut buf = String::new();
    write_value(&mut buf, value, 0, true);
    buf
}

fn write_value(buf: &mut String, value: &Value, depth: usize, compact: bool) {
    match value {
        Value::Null => buf.push_str("null"),
        Value::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => buf.push_str(&n.to_string()),
        Value::String(s) => write_single_quoted(buf, s),
        Value::Array(arr) => write_array(buf, arr, depth, compact),
        Value::Object(map) => write_object(buf, map, depth, compact),
    }
}

fn write_single_quoted(buf: &mut String, s: &str) {
    buf.push('\'');
    for ch in s.chars() {
        match ch {
            '\'' => buf.push_str("\\'"),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            '\u{0008}' => buf.push_str("\\b"),
            '\u{000C}' => buf.push_str("\\f"),
            '\0' => buf.push_str("\\0"),
            c if c.is_control() => {
                // \uXXXX for other control characters
                for unit in c.encode_utf16(&mut [0; 2]) {
                    buf.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => buf.push(c),
        }
    }
    buf.push('\'');
}

fn write_array(buf: &mut String, arr: &[Value], depth: usize, compact: bool) {
    if arr.is_empty() {
        buf.push_str("[]");
        return;
    }

    if compact {
        buf.push('[');
        for (i, item) in arr.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            write_value(buf, item, 0, true);
        }
        buf.push(']');
    } else {
        buf.push_str("[\n");
        let child_indent = depth + 1;
        for item in arr {
            push_indent(buf, child_indent);
            write_value(buf, item, child_indent, false);
            buf.push_str(",\n");
        }
        push_indent(buf, depth);
        buf.push(']');
    }
}

fn write_object(buf: &mut String, map: &serde_json::Map<String, Value>, depth: usize, compact: bool) {
    if map.is_empty() {
        buf.push_str("{}");
        return;
    }

    if compact {
        buf.push('{');
        for (i, (key, val)) in map.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            write_key(buf, key);
            buf.push_str(": ");
            write_value(buf, val, 0, true);
        }
        buf.push('}');
    } else {
        buf.push_str("{\n");
        let child_indent = depth + 1;
        for (key, val) in map {
            push_indent(buf, child_indent);
            write_key(buf, key);
            buf.push_str(": ");
            write_value(buf, val, child_indent, false);
            buf.push_str(",\n");
        }
        push_indent(buf, depth);
        buf.push('}');
    }
}

fn write_key(buf: &mut String, key: &str) {
    if is_valid_identifier(key) {
        buf.push_str(key);
    } else {
        write_single_quoted(buf, key);
    }
}

/// Check if a string is a valid ECMAScript 5.1 identifier name.
///
/// Valid identifiers start with a letter, `_`, or `$`, and continue with
/// letters, digits, `_`, or `$`. This also allows ES5 reserved words as
/// keys per the JSON5 spec.
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !is_id_start(first) {
        return false;
    }

    chars.all(is_id_continue)
}

fn is_id_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_id_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

fn push_indent(buf: &mut String, depth: usize) {
    for _ in 0..depth {
        buf.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_object() {
        let v = json!({"name": "Bob", "age": 32});
        let out = to_json5_pretty(&v);
        assert!(out.contains("name: 'Bob'"));
        assert!(out.contains("age: 32"));
        // Keys should NOT be quoted
        assert!(!out.contains("\"name\""));
        assert!(!out.contains("\"age\""));
    }

    #[test]
    fn nested_object() {
        let v = json!({"person": {"name": "Bob", "age": 32}});
        let out = to_json5_pretty(&v);
        assert!(out.contains("person: {"));
        assert!(out.contains("    name: 'Bob'"));
    }

    #[test]
    fn array_values() {
        let v = json!({"tags": ["rust", "cli"]});
        let out = to_json5_pretty(&v);
        assert!(out.contains("tags: ["));
        assert!(out.contains("  'rust',"));
    }

    #[test]
    fn trailing_commas() {
        let v = json!({"a": 1, "b": 2});
        let out = to_json5_pretty(&v);
        // Every value line should end with a comma
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.contains(':') && !trimmed.starts_with('{') {
                assert!(trimmed.ends_with(','), "missing trailing comma: {trimmed}");
            }
        }
    }

    #[test]
    fn empty_object() {
        let v = json!({});
        assert_eq!(to_json5_pretty(&v).trim(), "{}");
    }

    #[test]
    fn empty_array() {
        let v = json!({"items": []});
        let out = to_json5_pretty(&v);
        assert!(out.contains("items: []"));
    }

    #[test]
    fn special_key_needs_quoting() {
        let v = json!({"with space": 1, "with-hyphen": 2});
        let out = to_json5_pretty(&v);
        assert!(out.contains("'with space': 1"));
        assert!(out.contains("'with-hyphen': 2"));
    }

    #[test]
    fn single_quote_in_string_escaped() {
        let v = json!({"msg": "it's fine"});
        let out = to_json5_pretty(&v);
        assert!(out.contains(r"'it\'s fine'"));
    }

    #[test]
    fn null_and_bool() {
        let v = json!({"a": null, "b": true, "c": false});
        let out = to_json5_pretty(&v);
        assert!(out.contains("a: null"));
        assert!(out.contains("b: true"));
        assert!(out.contains("c: false"));
    }

    #[test]
    fn numeric_key_gets_quoted() {
        let v = json!({"123": "value"});
        let out = to_json5_pretty(&v);
        assert!(out.contains("'123': 'value'"));
    }

    #[test]
    fn underscore_and_dollar_keys() {
        let v = json!({"_private": 1, "$special": 2});
        let out = to_json5_pretty(&v);
        assert!(out.contains("_private: 1"));
        assert!(out.contains("$special: 2"));
    }

    // ── Compact mode ──────────────────────────────────────────────

    #[test]
    fn compact_simple_object() {
        let v = json!({"name": "Bob", "age": 32});
        let out = to_json5_compact(&v);
        assert_eq!(out, "{age: 32, name: 'Bob'}");
    }

    #[test]
    fn compact_nested() {
        let v = json!({"person": {"name": "Bob"}});
        let out = to_json5_compact(&v);
        assert_eq!(out, "{person: {name: 'Bob'}}");
    }

    #[test]
    fn compact_array() {
        let v = json!({"tags": ["a", "b"]});
        let out = to_json5_compact(&v);
        assert_eq!(out, "{tags: ['a', 'b']}");
    }

    #[test]
    fn compact_no_trailing_commas() {
        let v = json!({"a": 1, "b": 2});
        let out = to_json5_compact(&v);
        assert!(!out.ends_with(",}"));
        assert!(!out.contains(",}"));
    }

    #[test]
    fn compact_single_line() {
        let v = json!({"deep": {"nested": {"value": 42}}});
        let out = to_json5_compact(&v);
        assert!(!out.contains('\n'));
    }

    #[test]
    fn compact_empty_containers() {
        assert_eq!(to_json5_compact(&json!({})), "{}");
        assert_eq!(to_json5_compact(&json!({"a": []})), "{a: []}");
    }

    #[test]
    fn compact_special_keys_quoted() {
        let v = json!({"with space": 1});
        let out = to_json5_compact(&v);
        assert_eq!(out, "{'with space': 1}");
    }

    // ── Identifier validation ───────────────────────────────────────

    #[test]
    fn identifier_validation() {
        assert!(is_valid_identifier("foo"));
        assert!(is_valid_identifier("_bar"));
        assert!(is_valid_identifier("$baz"));
        assert!(is_valid_identifier("camelCase"));
        assert!(is_valid_identifier("snake_case"));
        assert!(is_valid_identifier("a1"));

        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("1starts_with_digit"));
        assert!(!is_valid_identifier("has space"));
        assert!(!is_valid_identifier("has-hyphen"));
        assert!(!is_valid_identifier("has.dot"));
    }
}
