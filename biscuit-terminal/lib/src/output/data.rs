//! Serialization of a command's data for the data family.

use biscuit_file::Json5;
use serde::Serialize;
use serde_json::{Map, Value};

use super::{OutputError, OutputTarget};

/// Serializes `data` for a data target.
///
/// JSON is pretty-printed; JSON5, YAML, and TOML are converted from the same
/// JSON value by biscuit-file, so every format carries the same data. TOML
/// has no `null` and needs a table at its root:
///
/// - an object field whose value is `null` is omitted, at any depth;
/// - a `null` inside an array is an error naming its path (`/items/2`),
///   because omitting it would change the array;
/// - a root that is not an object (a scalar or an array) is written as
///   `value = …`.
///
/// Every output ends with a newline.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::output::{OutputTarget, serialize_data};
///
/// let data = serde_json::json!({ "name": "x", "note": null });
/// assert_eq!(serialize_data(&data, OutputTarget::Toml).unwrap(), "name = \"x\"\n");
/// assert_eq!(serialize_data(&3, OutputTarget::Toml).unwrap(), "value = 3\n");
/// ```
///
/// ## Errors
///
/// [`OutputError::Serialize`] when the data cannot be represented, or when
/// `target` is not a data target.
pub fn serialize_data<D: Serialize + ?Sized>(
    data: &D,
    target: OutputTarget,
) -> Result<String, OutputError> {
    let failure = |path: Option<String>, message: String| OutputError::Serialize {
        target,
        path,
        message,
    };
    let value = serde_json::to_value(data).map_err(|err| failure(None, err.to_string()))?;
    let mut text = match target {
        OutputTarget::Json => {
            serde_json::to_string_pretty(&value).map_err(|err| failure(None, err.to_string()))?
        }
        OutputTarget::Json5 => Json5::from_value(value).as_json5(),
        OutputTarget::Yaml => Json5::from_value(value)
            .as_yaml()
            .map_err(|err| failure(None, err.to_string()))?,
        OutputTarget::Toml => {
            let table = toml_ready(value).map_err(|path| {
                failure(
                    Some(path),
                    "TOML cannot represent a null inside an array".to_owned(),
                )
            })?;
            Json5::from_value(table)
                .as_toml()
                .map_err(|err| failure(None, err.to_string()))?
        }
        other => {
            return Err(failure(
                None,
                format!("`{other}` is not a data output"),
            ));
        }
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// Applies TOML's null and root rules; `Err` carries the path of a null
/// array element.
fn toml_ready(value: Value) -> Result<Value, String> {
    let root = match value {
        Value::Object(map) => Value::Object(map),
        Value::Null => Value::Object(Map::new()),
        other => {
            let mut map = Map::new();
            map.insert("value".to_owned(), other);
            Value::Object(map)
        }
    };
    let mut path = String::new();
    strip_nulls(root, &mut path)
}

fn strip_nulls(value: Value, path: &mut String) -> Result<Value, String> {
    match value {
        Value::Object(map) => {
            let mut kept = Map::new();
            for (key, child) in map {
                if child.is_null() {
                    continue;
                }
                let len = path.len();
                path.push('/');
                push_pointer_token(path, &key);
                let child = strip_nulls(child, path)?;
                path.truncate(len);
                kept.insert(key, child);
            }
            Ok(Value::Object(kept))
        }
        Value::Array(items) => {
            let mut kept = Vec::with_capacity(items.len());
            for (index, child) in items.into_iter().enumerate() {
                let len = path.len();
                path.push('/');
                path.push_str(&index.to_string());
                if child.is_null() {
                    return Err(path.clone());
                }
                let child = strip_nulls(child, path)?;
                path.truncate(len);
                kept.push(child);
            }
            Ok(Value::Array(kept))
        }
        scalar => Ok(scalar),
    }
}

/// Escapes a key as a JSON pointer token (`~` → `~0`, `/` → `~1`).
fn push_pointer_token(path: &mut String, key: &str) {
    for ch in key.chars() {
        match ch {
            '~' => path.push_str("~0"),
            '/' => path.push_str("~1"),
            ch => path.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn toml(value: Value) -> Result<String, OutputError> {
        serialize_data(&value, OutputTarget::Toml)
    }

    #[test]
    fn json_is_pretty_and_newline_terminated() {
        let text = serialize_data(&json!({ "a": [1, null] }), OutputTarget::Json).unwrap();
        assert_eq!(text, "{\n  \"a\": [\n    1,\n    null\n  ]\n}\n");
    }

    #[test]
    fn json5_and_yaml_keep_nulls() {
        let data = json!({ "a": null, "b": [null] });
        let json5 = serialize_data(&data, OutputTarget::Json5).unwrap();
        assert!(json5.contains("a: null"), "{json5}");
        let yaml = serialize_data(&data, OutputTarget::Yaml).unwrap();
        assert!(yaml.contains("a: null"), "{yaml}");
        assert!(yaml.ends_with('\n'));
    }

    #[test]
    fn toml_omits_null_fields_at_every_depth() {
        let text = toml(json!({ "a": 1, "b": null, "c": { "d": null, "e": "x" } })).unwrap();
        assert!(!text.contains('b'), "{text}");
        assert!(!text.contains("d ="), "{text}");
        assert!(text.contains("e = \"x\""), "{text}");
    }

    #[test]
    fn toml_rejects_a_null_array_element_with_its_path() {
        let err = toml(json!({ "outer": { "items/x": [1, { "k": [null] }] } })).unwrap_err();
        match err {
            OutputError::Serialize { target, path, .. } => {
                assert_eq!(target, OutputTarget::Toml);
                assert_eq!(path.as_deref(), Some("/outer/items~1x/1/k/0"));
            }
            other => panic!("unexpected {other:?}"),
        }
        let err = toml(json!([null])).unwrap_err();
        assert!(err.to_string().contains("`/value/0`"), "{err}");
    }

    #[test]
    fn toml_wraps_non_object_roots() {
        assert_eq!(toml(json!("hi")).unwrap(), "value = \"hi\"\n");
        assert_eq!(toml(json!([1, 2])).unwrap(), "value = [\n    1,\n    2,\n]\n");
        assert_eq!(toml(json!(null)).unwrap(), "\n");
    }

    #[test]
    fn toml_keeps_heterogeneous_arrays() {
        let text = toml(json!({ "row": ["a", 1, true, { "k": "v" }] })).unwrap();
        assert!(text.contains("row = ["), "{text}");
        assert!(text.contains("\"a\""), "{text}");
        assert!(text.contains("true"), "{text}");
    }

    #[test]
    fn a_formatted_target_is_not_a_data_output() {
        let err = serialize_data(&json!({}), OutputTarget::Plain).unwrap_err();
        assert!(matches!(err, OutputError::Serialize { path: None, .. }));
    }
}
