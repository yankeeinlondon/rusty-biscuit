//! JSON null stripping utility.

use serde_json::Value;
use tracing::warn;

pub(super) fn strip_nulls(value: &mut Value) {
    let mut warned = false;
    strip_nulls_recursive(value, 0, &mut warned);
}

const STRIP_NULLS_MAX_DEPTH: u32 = 64;

fn strip_nulls_recursive(value: &mut Value, depth: u32, warned: &mut bool) {
    if depth >= STRIP_NULLS_MAX_DEPTH {
        if !*warned {
            *warned = true;
            warn!(
                max_depth = STRIP_NULLS_MAX_DEPTH,
                "null stripping reached maximum depth; deeper nulls will not be removed"
            );
        }
        return;
    }
    match value {
        Value::Object(map) => {
            let mut to_remove = Vec::new();
            for (key, nested) in map.iter_mut() {
                strip_nulls_recursive(nested, depth + 1, warned);
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
                strip_nulls_recursive(nested, depth + 1, warned);
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
    use tracing_test::traced_test;

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
            "g": 1.5,
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
                "g": 1.5,
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

    #[traced_test]
    #[test]
    fn warns_once_when_max_depth_is_reached() {
        let mut value = json!({"leaf": null});
        for _ in 0..(STRIP_NULLS_MAX_DEPTH + 10) {
            value = json!({"nested": value});
        }
        strip_nulls(&mut value);

        logs_assert(|logs| {
            let warnings: Vec<_> = logs
                .iter()
                .filter(|l| l.contains("null stripping reached maximum depth"))
                .collect();
            assert_eq!(warnings.len(), 1, "expected one warning, got: {:?}", logs);
            Ok(())
        });
    }
}
