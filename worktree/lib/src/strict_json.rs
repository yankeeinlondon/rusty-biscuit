//! Strict JSON parsing for the stored formats that serde alone reads
//! permissively.
//!
//! A typed `serde_json::from_slice` rejects a repeated key and an integer
//! enum tag, but two shapes lose that strictness:
//!
//! - a format whose top-level members are validated one by one goes through
//!   a [`Value`], which keeps the last of two repeated keys; [`members`] keeps
//!   the rejection, spoiling the member that repeats a key, or the whole
//!   document when the top level repeats one;
//! - a field inside an internally tagged enum variant is read from serde's
//!   buffer, which takes an integer `kind` as a variant index, so
//!   `{"kind": 1}` became a credentials failure never recorded; [`nested`]
//!   reads such a field without that buffer.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

/// The top-level members of `bytes`, or `None` when they are not one JSON
/// object or the object repeats a key.
pub(crate) fn members(bytes: &[u8]) -> Option<Members> {
    serde_json::from_slice::<Members>(bytes).ok()
}

pub(crate) struct Members(BTreeMap<String, Checked>);

impl Members {
    /// The member `key` as a `T`, or `None` when it is absent, repeats a key
    /// at any depth, or does not make a `T`.
    pub(crate) fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.0.get(key).filter(|member| member.clean).and_then(|member| T::deserialize(&member.value).ok())
    }

    /// Every member as one value, for a format read as a whole; `None` when
    /// any member repeats a key.
    pub(crate) fn into_value(self) -> Option<Value> {
        let mut object = Map::new();
        for (key, member) in self.0 {
            if !member.clean {
                return None;
            }
            object.insert(key, member.value);
        }
        Some(Value::Object(object))
    }
}

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MembersVisitor;

        impl<'de> Visitor<'de> for MembersVisitor {
            type Value = Members;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Members, A::Error> {
                let mut members = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let member = map.next_value::<Checked>()?;
                    if members.insert(key, member).is_some() {
                        return Err(A::Error::custom("duplicate key"));
                    }
                }
                Ok(Members(members))
            }
        }

        deserializer.deserialize_map(MembersVisitor)
    }
}

/// A `deserialize_with` for a field of an internally tagged enum variant:
/// the field must repeat no key and, when it is itself a tagged enum, carry a
/// string tag.
pub(crate) fn nested<'de, D: Deserializer<'de>, T: DeserializeOwned>(deserializer: D) -> Result<T, D::Error> {
    let Checked { value, clean } = Checked::deserialize(deserializer)?;
    if !clean {
        return Err(D::Error::custom("duplicate key"));
    }
    // Unlike serde's buffer, a `Value` refuses a number as a variant name.
    T::deserialize(value).map_err(D::Error::custom)
}

/// A JSON value and whether it is free of repeated keys.
struct Checked {
    value: Value,
    clean: bool,
}

impl From<Value> for Checked {
    fn from(value: Value) -> Self {
        Self { value, clean: true }
    }
}

impl<'de> Deserialize<'de> for Checked {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(CheckedVisitor)
    }
}

struct CheckedVisitor;

impl<'de> Visitor<'de> for CheckedVisitor {
    type Value = Checked;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Checked, E> {
        Ok(Value::Bool(value).into())
    }

    fn visit_i64<E>(self, value: i64) -> Result<Checked, E> {
        Ok(Value::Number(value.into()).into())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Checked, E> {
        Ok(Value::Number(value.into()).into())
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Checked, E> {
        Number::from_f64(value).map(|number| Value::Number(number).into()).ok_or_else(|| E::custom("not a JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Checked, E> {
        Ok(Value::String(value.to_string()).into())
    }

    fn visit_string<E>(self, value: String) -> Result<Checked, E> {
        Ok(Value::String(value).into())
    }

    fn visit_unit<E>(self) -> Result<Checked, E> {
        Ok(Value::Null.into())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Checked, A::Error> {
        let mut items = Vec::new();
        let mut clean = true;
        while let Some(item) = seq.next_element::<Checked>()? {
            clean &= item.clean;
            items.push(item.value);
        }
        Ok(Checked { value: Value::Array(items), clean })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Checked, A::Error> {
        let mut object = Map::new();
        let mut clean = true;
        while let Some(key) = map.next_key::<String>()? {
            let member = map.next_value::<Checked>()?;
            clean &= member.clean && !object.contains_key(&key);
            object.insert(key, member.value);
        }
        Ok(Checked { value: Value::Object(object), clean })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum Inner {
        First,
        Second { key: Option<String> },
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum Outer {
        Plain { inner: Inner },
        Strict {
            #[serde(deserialize_with = "nested")]
            inner: Inner,
        },
    }

    #[test]
    fn a_nested_tagged_field_needs_a_string_tag_and_no_repeated_key() {
        let read = |text: &str| serde_json::from_str::<Outer>(text).ok();
        assert_eq!(
            read(r#"{"kind":"plain","inner":{"kind":1,"key":null}}"#),
            Some(Outer::Plain { inner: Inner::Second { key: None } }),
            "what serde's buffer alone does"
        );
        assert_eq!(read(r#"{"kind":"strict","inner":{"kind":1,"key":null}}"#), None);
        assert_eq!(read(r#"{"kind":"strict","inner":{"kind":"second","key":null,"key":null}}"#), None);
        assert_eq!(
            read(r#"{"kind":"strict","inner":{"kind":"second","key":"K"}}"#),
            Some(Outer::Strict { inner: Inner::Second { key: Some("K".into()) } })
        );
    }

    #[test]
    fn a_repeated_member_key_spoils_only_that_member() {
        let members = members(br#"{"good":{"a":1},"repeats":{"a":1,"a":1},"deep":[{"b":1,"b":2}],"sibling":{"a":1,"c":{"a":1}}}"#)
            .expect("the top level repeats nothing");
        assert_eq!(members.get::<Value>("good"), Some(json!({"a": 1})));
        assert_eq!(members.get::<Value>("repeats"), None);
        assert_eq!(members.get::<Value>("deep"), None);
        assert_eq!(members.get::<Value>("sibling"), Some(json!({"a": 1, "c": {"a": 1}})), "one key in two objects");
        assert_eq!(members.get::<Value>("absent"), None);
        assert_eq!(members.into_value(), None, "read as a whole, a spoiled member spoils it");
    }

    #[test]
    fn a_repeated_top_level_key_or_anything_but_one_object_is_unreadable() {
        assert!(members(br#"{"a":1,"a":1}"#).is_none());
        assert!(members(b"{} trailing").is_none());
        assert!(members(b"[]").is_none());
        assert_eq!(
            members(br#"{"a":1.5,"b":[true,null,"s",-1]}"#).and_then(Members::into_value),
            Some(json!({"a": 1.5, "b": [true, null, "s", -1]}))
        );
    }
}
