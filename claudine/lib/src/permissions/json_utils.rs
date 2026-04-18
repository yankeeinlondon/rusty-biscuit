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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn ensure_json_value_creates_nested_path() {
        let mut root = json!({});
        let result = ensure_json_value(&mut root, &["a", "b", "c"]);
        *result = json!(42);
        assert_eq!(root, json!({"a": {"b": {"c": 42}}}));
    }

    #[test]
    fn ensure_json_value_preserves_existing_objects() {
        let mut root = json!({"a": {"existing": true}});
        let result = ensure_json_value(&mut root, &["a", "new_key"]);
        *result = json!("hello");
        assert_eq!(
            root,
            json!({"a": {"existing": true, "new_key": "hello"}})
        );
    }

    #[test]
    fn ensure_json_value_overwrites_non_object_to_object() {
        let mut root = json!({"a": "not_an_object"});
        let result = ensure_json_value(&mut root, &["a", "b"]);
        *result = json!(99);
        assert_eq!(root, json!({"a": {"b": 99}}));
    }

    #[test]
    fn ensure_json_array_creates_new_array_at_path() {
        let mut root = json!({});
        let arr = ensure_json_array(&mut root, &["items"]);
        arr.push(json!(1));
        arr.push(json!(2));
        assert_eq!(root, json!({"items": [1, 2]}));
    }

    #[test]
    fn ensure_json_array_preserves_existing_array() {
        let mut root = json!({"items": [1, 2, 3]});
        let arr = ensure_json_array(&mut root, &["items"]);
        assert_eq!(arr.len(), 3);
        arr.push(json!(4));
        assert_eq!(root, json!({"items": [1, 2, 3, 4]}));
    }

    #[test]
    fn ensure_json_array_overwrites_non_array_to_array() {
        let mut root = json!({"items": "not_an_array"});
        let arr = ensure_json_array(&mut root, &["items"]);
        assert!(arr.is_empty());
        arr.push(json!("x"));
        assert_eq!(root, json!({"items": ["x"]}));
    }

    #[test]
    fn ensure_json_value_empty_path_returns_root() {
        let mut root = json!({"key": "value"});
        let result = ensure_json_value(&mut root, &[]);
        assert_eq!(*result, json!({"key": "value"}));
    }
}
