//! JSON null stripping utility.

use serde_json::Value;

pub(super) fn strip_nulls(value: &mut Value) {
    strip_nulls_recursive(value, 0);
}

const STRIP_NULLS_MAX_DEPTH: u32 = 64;

fn strip_nulls_recursive(value: &mut Value, depth: u32) {
    if depth >= STRIP_NULLS_MAX_DEPTH {
        return;
    }
    match value {
        Value::Object(map) => {
            let mut to_remove = Vec::new();
            for (key, nested) in map.iter_mut() {
                strip_nulls_recursive(nested, depth + 1);
                if nested.is_null() {
                    to_remove.push(key.clone());
                }
            }
            for key in to_remove {
                map.remove(&key);
            }
        }
        Value::Array(items) => {
            for nested in items.iter_mut() {
                strip_nulls_recursive(nested, depth + 1);
            }
            items.retain(|item| !item.is_null());
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn removes_nulls_at_top_level() {
        let mut value = json!({"a": 1, "b": null, "c": "keep"});
        strip_nulls(&mut value);
        assert_eq!(value, json!({"a": 1, "c": "keep"}));
    }

    #[test]
    fn removes_nulls_inside_arrays() {
        let mut value = json!([1, null, "keep", null, {"x": null, "y": 2}]);
        strip_nulls(&mut value);
        assert_eq!(value, json!([1, "keep", {"y": 2}]));
    }

    #[test]
    fn preserves_non_null_leaves_and_nested_structure() {
        let mut value = json!({
            "a": {
                "b": {
                    "c": 42,
                    "d": null
                },
                "e": [true, false, null]
            },
            "f": "string",
            "g": 3.14,
            "h": false
        });
        strip_nulls(&mut value);
        assert_eq!(
            value,
            json!({
                "a": {
                    "b": {
                        "c": 42
                    },
                    "e": [true, false]
                },
                "f": "string",
                "g": 3.14,
                "h": false
            })
        );
    }

    #[test]
    fn bottoms_out_past_max_depth_without_panicking() {
        // Build a deeply nested object that exceeds STRIP_NULLS_MAX_DEPTH.
        let mut value = json!({"leaf": null});
        for _ in 0..(STRIP_NULLS_MAX_DEPTH + 10) {
            value = json!({"nested": value});
        }
        // Should not panic and should still return a valid Value.
        strip_nulls(&mut value);
        // The deep nesting means we hit the depth limit before reaching the
        // null leaf, so the null is never stripped.
        assert!(value.is_object());
    }
}
