use serde_json::{Map, Value};

pub(crate) fn ensure_json_value<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let mut current = root;
    for key in path {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let obj = current
            .as_object_mut()
            .expect("ensure_json_value: current was just set to Object above");
        current = obj.entry((*key).to_owned()).or_insert(Value::Null);
    }
    current
}

pub(crate) fn ensure_json_array<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Vec<Value> {
    let value = ensure_json_value(root, path);
    if !value.is_array() {
        *value = Value::Array(Vec::new());
    }
    value
        .as_array_mut()
        .expect("ensure_json_array: value was just set to Array above")
}
