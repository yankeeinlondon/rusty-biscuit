//! Serde helpers for optional configuration whose absence and explicit null
//! mean different things.
//!
//! `Option<T>` alone reads both an omitted key and `null` as `None`. Vocabulary
//! fields pair `#[serde(default, deserialize_with = "non_null")]` so omission
//! is `None` while `null` is rejected by `T`'s own deserializer.

use serde::{Deserialize, Deserializer};

/// Deserialize a present value as `Some`; explicit null fails as a wrong type.
pub(crate) fn non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
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

    #[test]
    fn explicit_null_is_rejected() {
        let error = serde_json::from_str::<Probe>(r#"{"value":null}"#).unwrap_err();
        assert!(error.to_string().contains("invalid type: null"), "{error}");
    }
}
