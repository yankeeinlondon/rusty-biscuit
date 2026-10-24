//! Canonical JSON rendering and hashing.
//!
//! [`canonical_json`] renders a [`serde_json::Value`] as compact JSON with
//! every object's keys sorted by `str` order, at every depth. Array elements
//! keep their order, because element order is meaningful. Two values that
//! differ only in object key order therefore render to the same bytes, and
//! hashing those bytes gives a key-order-insensitive identity.
//!
//! The rendering does not depend on serde_json's `preserve_order` feature and
//! never reorders the input map. See `biscuit-hash/docs/json.md`.

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq};
use serde_json::Value;

/// Renders `value` as compact JSON with object keys sorted at every depth.
///
/// The output is byte-identical to `serde_json::to_string` for a value whose
/// objects are already sorted, so it can replace that call wherever key order
/// must not affect the result.
///
/// ## Examples
///
/// ```rust
/// use biscuit_hash::canonical_json;
/// use serde_json::json;
///
/// let value = json!({"b": {"y": 1, "x": 2}, "a": [3, 1]});
/// assert_eq!(canonical_json(&value), r#"{"a":[3,1],"b":{"x":2,"y":1}}"#);
/// ```
pub fn canonical_json(value: &Value) -> String {
    let mut bytes = Vec::new();
    // Cannot fail: the writer is in memory and every `Value` key is a string.
    serde_json::to_writer(&mut bytes, &CanonicalJson(value))
        .expect("serializing a serde_json::Value into memory is infallible");
    // serde_json only ever writes valid UTF-8.
    String::from_utf8(bytes).expect("serde_json output is valid UTF-8")
}

/// XXH64 of [`canonical_json`]'s bytes (seed 0, as [`crate::xx_hash_bytes`]).
#[cfg(feature = "xx_hash")]
pub fn xx_hash_json(value: &Value) -> u64 {
    crate::xx_hash_bytes(canonical_json(value).as_bytes())
}

/// BLAKE3 hex digest of [`canonical_json`]'s bytes (as [`crate::blake3_hash`]).
#[cfg(feature = "blake3")]
pub fn blake3_hash_json(value: &Value) -> String {
    crate::blake3_hash(&canonical_json(value))
}

struct CanonicalJson<'a>(&'a Value);

impl Serialize for CanonicalJson<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Number(value) => value.serialize(serializer),
            Value::String(value) => serializer.serialize_str(value),
            Value::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(&CanonicalJson(value))?;
                }
                seq.end()
            }
            Value::Object(map) => {
                let mut entries: Vec<_> = map.iter().collect();
                entries.sort_by(|left, right| left.0.cmp(right.0));

                let mut object = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    object.serialize_entry(key, &CanonicalJson(value))?;
                }
                object.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    /// Builds an object with exactly the given insertion order. The crate's
    /// tests enable `preserve_order`, so this order is what iteration sees.
    fn object(entries: &[(&str, Value)]) -> Value {
        let mut map = Map::new();
        for (key, value) in entries {
            map.insert((*key).to_string(), value.clone());
        }
        Value::Object(map)
    }

    #[test]
    fn canonical_json_sorts_keys() {
        let value = json!({
            "zebra": 1,
            "alpha": 2,
            "middle": {"z": true, "a": false}
        });
        assert_eq!(
            canonical_json(&value),
            r#"{"alpha":2,"middle":{"a":false,"z":true},"zebra":1}"#
        );
    }

    #[test]
    fn canonical_json_preserves_array_order() {
        let value = json!({"items": [3, 1, 2]});
        assert_eq!(canonical_json(&value), r#"{"items":[3,1,2]}"#);
    }

    #[test]
    fn canonical_json_handles_nested_objects() {
        let value = json!({
            "b": {"d": 4, "c": 3},
            "a": {"f": 6, "e": 5}
        });
        assert_eq!(canonical_json(&value), r#"{"a":{"e":5,"f":6},"b":{"c":3,"d":4}}"#);
    }

    #[test]
    fn canonical_json_empty_containers() {
        assert_eq!(canonical_json(&json!({})), "{}");
        assert_eq!(canonical_json(&json!([])), "[]");
        assert_eq!(canonical_json(&json!({"a": {}, "b": []})), r#"{"a":{},"b":[]}"#);
    }

    #[test]
    fn canonical_json_scalars() {
        assert_eq!(canonical_json(&json!(null)), "null");
        assert_eq!(canonical_json(&json!(true)), "true");
        assert_eq!(canonical_json(&json!("hello")), r#""hello""#);
        assert_eq!(canonical_json(&json!(42)), "42");
        assert_eq!(canonical_json(&json!(-1.5)), "-1.5");
    }

    #[test]
    fn object_key_order_is_ignored_at_every_depth() {
        let first = object(&[
            ("title", json!("T")),
            (
                "meta",
                object(&[
                    ("owner", json!("ken")),
                    ("tags", json!([object(&[("y", json!(1)), ("x", json!(2))])])),
                ]),
            ),
        ]);
        let second = object(&[
            (
                "meta",
                object(&[
                    ("tags", json!([object(&[("x", json!(2)), ("y", json!(1))])])),
                    ("owner", json!("ken")),
                ]),
            ),
            ("title", json!("T")),
        ]);
        assert_ne!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            "fixture must differ in insertion order"
        );
        assert_eq!(canonical_json(&first), canonical_json(&second));
        assert_eq!(
            canonical_json(&first),
            r#"{"meta":{"owner":"ken","tags":[{"x":2,"y":1}]},"title":"T"}"#
        );
    }

    #[test]
    fn array_element_order_is_significant() {
        let first = json!({"list": [{"a": 1}, {"b": 2}]});
        let second = json!({"list": [{"b": 2}, {"a": 1}]});
        assert_ne!(canonical_json(&first), canonical_json(&second));
    }

    #[test]
    fn unicode_keys_sort_by_str_order() {
        // `str` order is UTF-8 byte order: "Z" < "a" < "é" < "日".
        let value = object(&[
            ("日", json!(4)),
            ("é", json!(3)),
            ("a", json!(2)),
            ("Z", json!(1)),
        ]);
        assert_eq!(canonical_json(&value), r#"{"Z":1,"a":2,"é":3,"日":4}"#);
    }

    #[test]
    fn strings_are_escaped_like_serde_json() {
        let value = json!({"k\"ey": "line\nbreak \"quoted\" \\ tab\t \u{0001}"});
        assert_eq!(canonical_json(&value), serde_json::to_string(&value).unwrap());
        assert_eq!(
            canonical_json(&value),
            r#"{"k\"ey":"line\nbreak \"quoted\" \\ tab\t \u0001"}"#
        );
    }

    #[test]
    fn sorted_input_matches_compact_serde_json() {
        let value = object(&[
            ("a", json!([1, "two", null, false])),
            ("b", object(&[("c", json!(1.25)), ("d", json!({}))])),
        ]);
        assert_eq!(canonical_json(&value), serde_json::to_string(&value).unwrap());
    }

    #[test]
    fn input_iteration_order_is_unchanged() {
        let value = object(&[
            ("z", object(&[("y", json!(1)), ("b", json!(2))])),
            ("a", json!(0)),
        ]);
        let before = serde_json::to_string(&value).unwrap();
        let _ = canonical_json(&value);
        assert_eq!(serde_json::to_string(&value).unwrap(), before);
        assert_eq!(before, r#"{"z":{"y":1,"b":2},"a":0}"#);
    }

    #[cfg(feature = "xx_hash")]
    #[test]
    fn xx_hash_json_hashes_the_canonical_bytes() {
        let value = object(&[("b", json!(1)), ("a", json!(2))]);
        let reordered = object(&[("a", json!(2)), ("b", json!(1))]);
        assert_eq!(
            xx_hash_json(&value),
            crate::xx_hash_bytes(br#"{"a":2,"b":1}"#)
        );
        assert_eq!(xx_hash_json(&value), xx_hash_json(&reordered));
        assert_ne!(xx_hash_json(&json!([1, 2])), xx_hash_json(&json!([2, 1])));
    }

    #[cfg(feature = "blake3")]
    #[test]
    fn blake3_hash_json_hashes_the_canonical_bytes() {
        let value = object(&[("b", json!(1)), ("a", json!(2))]);
        let reordered = object(&[("a", json!(2)), ("b", json!(1))]);
        assert_eq!(blake3_hash_json(&value), crate::blake3_hash(r#"{"a":2,"b":1}"#));
        assert_eq!(blake3_hash_json(&value), blake3_hash_json(&reordered));
        assert_ne!(blake3_hash_json(&json!([1, 2])), blake3_hash_json(&json!([2, 1])));
    }
}
