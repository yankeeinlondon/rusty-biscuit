//! Authored key order for the mappings of one parsed document.

use std::collections::HashMap;
use std::fmt;

use biscuit_file::serde_yaml_ng;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

/// Authored key order of every mapping in one document, keyed by JSON
/// Pointer.
///
/// A YAML document is indexed from its parsed value with [`Self::collect`].
/// Any other self-describing serde format — JSON and JSON5 included, through
/// [`biscuit_file::Json5::deserialize_raw`] — is indexed by deserializing its
/// text straight into this type: the
/// [`Deserialize`](serde::Deserialize) impl records each mapping's keys in the
/// order the format's deserializer reports them, which for a streaming parser
/// is source order. Both routes produce the same pointers.
///
/// The source records the order an author wrote their keys in; the JSON value model
/// Darkmatter and its consumers pass around does not. This index is the narrow
/// side channel that carries that order without changing the representation of
/// every `serde_json::Map` in the graph: a consumer that needs authored order
/// at one boundary looks the mapping up by pointer, and everything else keeps
/// ordinary canonical-order behavior.
///
/// Only mappings with string keys are indexed, and only string keys are
/// recorded, because a JSON Pointer has no spelling for a non-string key.
/// Through [`Deserialize`](serde::Deserialize) a non-string key is an error
/// rather than a skip; JSON and JSON5 cannot author one.
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

impl<'de> serde::Deserialize<'de> for MappingOrders {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut orders = HashMap::new();
        OrderRecorder {
            pointer: String::new(),
            orders: &mut orders,
        }
        .deserialize(deserializer)?;
        Ok(Self { orders })
    }
}

/// Records the key order of the value at `pointer` and of everything below it.
struct OrderRecorder<'a> {
    pointer: String,
    orders: &'a mut HashMap<String, Vec<String>>,
}

impl OrderRecorder<'_> {
    fn child(&mut self, segment: &str) -> OrderRecorder<'_> {
        OrderRecorder {
            pointer: MappingOrders::child_pointer(&self.pointer, segment),
            orders: self.orders,
        }
    }
}

impl<'de> DeserializeSeed<'de> for OrderRecorder<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for OrderRecorder<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any self-describing document value")
    }

    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
        let mut keys = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            map.next_value_seed(self.child(&key))?;
            keys.push(key);
        }
        self.orders.insert(self.pointer, keys);
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(mut self, mut items: A) -> Result<(), A::Error> {
        let mut index = 0_usize;
        while items
            .next_element_seed(self.child(&index.to_string()))?
            .is_some()
        {
            index += 1;
        }
        Ok(())
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_i128<E: de::Error>(self, _: i128) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u128<E: de::Error>(self, _: u128) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_bytes<E: de::Error>(self, _: &[u8]) -> Result<(), E> {
        Ok(())
    }

    fn visit_none<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
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
    fn deserializing_json5_indexes_source_order_at_the_same_pointers_as_yaml() {
        let json5 = concat!(
            "{\n",
            "  // comments, unquoted keys, and trailing commas are JSON5\n",
            "  start: {stack: [{action: {set: {\n",
            "    z_last_lexically: true,\n",
            "    'a/first': [1, null, {y: 1, b: 2}],\n",
            "  }}}]},\n",
            "}\n",
        );
        let yaml = concat!(
            "start:\n",
            "  stack:\n",
            "    - action:\n",
            "        set:\n",
            "          z_last_lexically: true\n",
            "          a/first: [1, null, {y: 1, b: 2}]\n",
        );

        let orders: MappingOrders = biscuit_file::Json5::from_str(json5)
            .unwrap()
            .deserialize_raw()
            .unwrap();

        assert_eq!(
            orders.get("/start/stack/0/action/set"),
            Some(["z_last_lexically".to_string(), "a/first".to_string()].as_slice()),
        );
        assert_eq!(
            orders.get("/start/stack/0/action/set/a~1first/2"),
            Some(["y".to_string(), "b".to_string()].as_slice()),
        );
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(orders, MappingOrders::collect(&yaml));
    }

    #[test]
    fn deserializing_plain_json_indexes_source_order() {
        let orders: MappingOrders =
            biscuit_file::Json5::from_str(r#"{"z": {"y": 1, "b": 2}, "a": [3.5, "s"]}"#)
                .unwrap()
                .deserialize_raw()
                .unwrap();

        assert_eq!(orders.get(""), Some(["z".to_string(), "a".to_string()].as_slice()));
        assert_eq!(orders.get("/z"), Some(["y".to_string(), "b".to_string()].as_slice()));
    }

    #[test]
    fn a_scalar_document_indexes_nothing() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str("just a scalar").unwrap();

        assert!(MappingOrders::collect(&value).is_empty());
        assert_eq!(MappingOrders::default().get(""), None);
    }
}
