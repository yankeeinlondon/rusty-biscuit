use serde_json::{Map, Value};

pub(crate) fn ensure_json_value<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let mut current = root;
    for key in path {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let obj = current.as_object_mut().unwrap();
        current = obj.entry((*key).to_owned()).or_insert(Value::Null);
    }
    current
}

pub(crate) fn ensure_json_array<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Vec<Value> {
    let value = ensure_json_value(root, path);
    if !value.is_array() {
        *value = Value::Array(Vec::new());
    }
    debug_assert!(
        value.is_array(),
        "ensure_json_array: type mismatch after set — expected array, got {:?}",
        value
    );
    unsafe { value.as_array_mut().unwrap_unchecked() }
}
