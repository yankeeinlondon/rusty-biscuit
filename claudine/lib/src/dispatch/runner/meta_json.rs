//! Utility module for JSON null stripping (retained after flattening removal).

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
