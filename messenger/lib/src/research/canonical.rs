//! Canonical JSON and xxh64 fingerprints.
//!
//! Fingerprints use `biscuit-hash` xxh64 (seed 0) and are spelled
//! `xxh64:` + 16 lowercase hex digits, matching the schema's `Xxh64` type.
//! Text inputs hash after CRLF-to-LF normalization so a Windows checkout with
//! `core.autocrlf` fingerprints the same as any other.

use serde_json::Value;

/// Key-sorted, compact JSON. Independent of `serde_json`'s
/// `preserve_order` feature, which workspace feature unification can enable.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

/// `xxh64:` + 16 lowercase hex digits of `bytes`.
pub fn xxh64_digest(bytes: &[u8]) -> String {
    format!("xxh64:{:016x}", biscuit_hash::xx_hash_bytes(bytes))
}

/// Fingerprint of text content after CRLF-to-LF normalization.
pub fn text_fingerprint(text: &str) -> String {
    xxh64_digest(text.replace("\r\n", "\n").as_bytes())
}

/// Fingerprint of one research record: its canonical JSON.
pub fn record_fingerprint(record: &Value) -> String {
    xxh64_digest(canonical_json(record).as_bytes())
}

/// Fingerprint of the document contract: `_schema.yaml` and `_types.yaml`
/// together, because a named-type change alters the contract as much as a
/// document-schema change does. The two normalized texts are joined by NUL.
pub fn schema_fingerprint(schema: &str, types: &str) -> String {
    let mut joined = schema.replace("\r\n", "\n");
    joined.push('\0');
    joined.push_str(&types.replace("\r\n", "\n"));
    xxh64_digest(joined.as_bytes())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_json_sorts_keys_at_every_depth() {
        let value = json!({"b": 1, "a": {"d": [2, {"z": true, "y": null}], "c": "x"}});
        assert_eq!(
            canonical_json(&value),
            r#"{"a":{"c":"x","d":[2,{"y":null,"z":true}]},"b":1}"#
        );
    }

    #[test]
    fn text_fingerprint_ignores_crlf_but_not_content() {
        assert_eq!(text_fingerprint("a\r\nb\n"), text_fingerprint("a\nb\n"));
        assert_ne!(text_fingerprint("a\nb\n"), text_fingerprint("a\nc\n"));
        let digest = text_fingerprint("a");
        assert!(digest.starts_with("xxh64:") && digest.len() == 22, "{digest}");
        assert!(digest[6..].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    }

    #[test]
    fn schema_fingerprint_changes_with_either_file() {
        let base = schema_fingerprint("s", "t");
        assert_ne!(base, schema_fingerprint("s2", "t"));
        assert_ne!(base, schema_fingerprint("s", "t2"));
        assert_ne!(schema_fingerprint("ab", ""), schema_fingerprint("a", "b"));
    }
}
