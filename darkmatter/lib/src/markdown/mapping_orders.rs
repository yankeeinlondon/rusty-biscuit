//! Authored key order for the mappings of one parsed YAML document.

use std::collections::HashMap;

use biscuit_file::serde_yaml_ng;

/// Authored key order of every mapping in one YAML document, keyed by JSON
/// Pointer.
///
/// YAML records the order an author wrote their keys in; the JSON value model
/// Darkmatter and its consumers pass around does not. This index is the narrow
/// side channel that carries that order without changing the representation of
/// every `serde_json::Map` in the graph: a consumer that needs authored order
/// at one boundary looks the mapping up by pointer, and everything else keeps
/// ordinary canonical-order behavior.
///
/// Only mappings with string keys are indexed, and only string keys are
/// recorded, because a JSON Pointer has no spelling for a non-string key.
///
/// ## Examples
///
/// ```
/// use biscuit_file::serde_yaml_ng;
/// use darkmatter::markdown::MappingOrders;
///
/// let value: serde_yaml_ng::Value =
///     serde_yaml_ng::from_str("outer:\n  z: 1\n  a: 2\n").unwrap();
/// let orders = MappingOrders::collect(&value);
///
/// assert_eq!(
///     orders.get("/outer"),
///     Some(["z".to_string(), "a".to_string()].as_slice()),
/// );
/// assert_eq!(orders.get("/missing"), None);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MappingOrders {
    orders: HashMap<String, Vec<String>>,
}

impl MappingOrders {
    /// Index every mapping reachable from `value`, rooted at the empty pointer.
    #[must_use]
    pub fn collect(value: &serde_yaml_ng::Value) -> Self {
        let mut orders = HashMap::new();
        collect_into(value, "", &mut orders);
        Self { orders }
    }

    /// Authored key order of the mapping at `pointer`.
    ///
    /// ## Returns
    ///
    /// `None` when no mapping was indexed at that pointer — which includes a
    /// document that carried no order metadata at all.
    #[must_use]
    pub fn get(&self, pointer: &str) -> Option<&[String]> {
        self.orders.get(pointer).map(Vec::as_slice)
    }

    /// `true` when no mapping was indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// Append one segment to a JSON Pointer, escaping `~` and `/` per RFC 6901.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::MappingOrders;
    ///
    /// assert_eq!(MappingOrders::child_pointer("", "start"), "/start");
    /// assert_eq!(MappingOrders::child_pointer("/a", "b/c"), "/a/b~1c");
    /// ```
    #[must_use]
    pub fn child_pointer(pointer: &str, segment: &str) -> String {
        let escaped = segment.replace('~', "~0").replace('/', "~1");
        format!("{pointer}/{escaped}")
    }
}

fn collect_into(
    value: &serde_yaml_ng::Value,
    pointer: &str,
    orders: &mut HashMap<String, Vec<String>>,
) {
    match value {
        serde_yaml_ng::Value::Mapping(map) => {
            let keys = map
                .keys()
                .filter_map(serde_yaml_ng::Value::as_str)
                .map(str::to_owned)
                .collect();
            orders.insert(pointer.to_string(), keys);
            for (key, child) in map {
                if let Some(key) = key.as_str() {
                    collect_into(child, &MappingOrders::child_pointer(pointer, key), orders);
                }
            }
        }
        serde_yaml_ng::Value::Sequence(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_into(
                    child,
                    &MappingOrders::child_pointer(pointer, &index.to_string()),
                    orders,
                );
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_indexes_nested_mappings_by_json_pointer() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(concat!(
            "start:\n",
            "  stack:\n",
            "    - action:\n",
            "        set:\n",
            "          z_last_lexically: true\n",
            "          a_first_lexically: false\n",
        ))
        .unwrap();

        let orders = MappingOrders::collect(&value);

        assert_eq!(
            orders.get("/start/stack/0/action/set"),
            Some(["z_last_lexically".to_string(), "a_first_lexically".to_string()].as_slice()),
        );
        assert_eq!(orders.get(""), Some(["start".to_string()].as_slice()));
        assert_eq!(orders.get("/start/stack/1"), None);
    }

    #[test]
    fn collect_escapes_pointer_segments_and_skips_non_string_keys() {
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str("\"a/b\":\n  \"c~d\":\n    z: 1\n1: {x: 2}\n").unwrap();

        let orders = MappingOrders::collect(&value);

        assert_eq!(orders.get("/a~1b/c~0d"), Some(["z".to_string()].as_slice()));
        // A non-string key has no pointer spelling, so neither it nor its
        // subtree is indexed — and it is absent from its parent's key order.
        assert_eq!(orders.get("/1"), None);
        assert_eq!(orders.get(""), Some(["a/b".to_string()].as_slice()));
    }

    #[test]
    fn a_scalar_document_indexes_nothing() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str("just a scalar").unwrap();

        assert!(MappingOrders::collect(&value).is_empty());
        assert_eq!(MappingOrders::default().get(""), None);
    }
}
