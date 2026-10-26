//! Serde helpers for optional configuration whose absence and explicit null
//! mean different things.
//!
//! `Option<T>` alone reads both an omitted key and `null` as `None`. Vocabulary
//! fields pair `#[serde(default, deserialize_with = "non_null")]` so omission
//! is `None` while `null` is rejected by `T`'s own deserializer.

//!
//! A derived struct also accepts a sequence of its fields (`[]` reads as an
//! all-default struct). Fields whose value must be an object use
//! [`map_only`] or [`non_null_map`], which accept only a map.

use std::fmt;
use std::marker::PhantomData;

use serde::de::{MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer};

/// Deserialize a present value as `Some`; explicit null fails as a wrong type.
pub(crate) fn non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Deserialize `T` from a map only; a sequence, null, or scalar is a wrong
/// type. Duplicate keys are still rejected by `T`'s own visitor.
pub(crate) fn map_only<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct MapOnly<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for MapOnly<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object")
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_map(MapOnly(PhantomData))
}

/// [`non_null`] for a field whose value must be an object.
pub(crate) fn non_null_map<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    map_only(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Probe {
        #[serde(default, deserialize_with = "super::non_null")]
        value: Option<String>,
    }

    #[test]
    fn absent_is_none_and_present_is_some() {
        assert_eq!(serde_json::from_str::<Probe>("{}").unwrap().value, None);
        assert_eq!(serde_json::from_str::<Probe>(r#"{"value":"x"}"#).unwrap().value.as_deref(), Some("x"));
    }

    #[derive(Debug, Default, PartialEq, Deserialize)]
    struct Inner {
        #[serde(default)]
        flag: bool,
    }

    #[derive(Debug, Deserialize)]
    struct MapProbe {
        #[serde(default, deserialize_with = "super::non_null_map")]
        inner: Option<Inner>,
    }

    #[test]
    fn map_fields_accept_only_objects() {
        let read = |text: &str| serde_json::from_str::<MapProbe>(text).map(|probe| probe.inner);
        assert_eq!(read("{}").unwrap(), None);
        assert_eq!(read(r#"{"inner":{}}"#).unwrap(), Some(Inner::default()));
        assert_eq!(read(r#"{"inner":{"flag":true}}"#).unwrap(), Some(Inner { flag: true }));
        for wrong in [r#"{"inner":[]}"#, r#"{"inner":[true]}"#, r#"{"inner":null}"#, r#"{"inner":1}"#] {
            assert!(read(wrong).is_err(), "{wrong}");
        }
        assert!(read(r#"{"inner":{"flag":true,"flag":false}}"#).is_err(), "duplicate key");
    }

    #[test]
    fn explicit_null_is_rejected() {
        let error = serde_json::from_str::<Probe>(r#"{"value":null}"#).unwrap_err();
        assert!(error.to_string().contains("invalid type: null"), "{error}");
    }
}
