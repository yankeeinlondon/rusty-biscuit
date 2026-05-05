//! Pricing metadata for LLM models.
//!
//! Captures per-token and per-request costs from provider APIs (e.g., OpenRouter).
//! All fields are optional since not all providers expose pricing data, and not all
//! models incur costs for every category.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Per-token and per-request pricing for a model.
///
/// Costs are expressed in USD. Providers like OpenRouter return pricing as
/// JSON strings (e.g., `"0.000005"`), so a custom deserializer handles
/// both string and numeric representations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelPricing {
    /// Cost per input token in USD.
    pub prompt_per_token: Option<f64>,

    /// Cost per output/completion token in USD.
    pub completion_per_token: Option<f64>,

    /// Cost per web search request in USD.
    pub web_search_per_request: Option<f64>,

    /// Cost per cached input token read in USD.
    pub input_cache_read_per_token: Option<f64>,
}

impl ModelPricing {
    /// Creates a new empty pricing instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
struct ModelPricingHelper {
    #[serde(
        rename = "prompt",
        default,
        deserialize_with = "deserialize_optional_string_or_f64"
    )]
    pub prompt_per_token: Option<f64>,

    #[serde(
        rename = "completion",
        default,
        deserialize_with = "deserialize_optional_string_or_f64"
    )]
    pub completion_per_token: Option<f64>,

    #[serde(
        rename = "request",
        default,
        deserialize_with = "deserialize_optional_string_or_f64"
    )]
    pub web_search_per_request: Option<f64>,

    #[serde(
        rename = "cache_read",
        default,
        deserialize_with = "deserialize_optional_string_or_f64"
    )]
    pub input_cache_read_per_token: Option<f64>,
}

impl Serialize for ModelPricing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let helper = ModelPricingHelper {
            prompt_per_token: self.prompt_per_token,
            completion_per_token: self.completion_per_token,
            web_search_per_request: self.web_search_per_request,
            input_cache_read_per_token: self.input_cache_read_per_token,
        };
        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModelPricing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = ModelPricingHelper::deserialize(deserializer)?;
        Ok(Self {
            prompt_per_token: helper.prompt_per_token,
            completion_per_token: helper.completion_per_token,
            web_search_per_request: helper.web_search_per_request,
            input_cache_read_per_token: helper.input_cache_read_per_token,
        })
    }
}

fn deserialize_optional_string_or_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrF64Visitor;

    impl<'de> Visitor<'de> for StringOrF64Visitor {
        type Value = Option<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, a number, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                return Ok(None);
            }
            v.parse::<f64>()
                .map(Some)
                .map_err(|_| de::Error::custom(format!("invalid float string: '{v}'")))
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v as f64))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v as f64))
        }
    }

    deserializer.deserialize_any(StringOrF64Visitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_default() {
        assert_eq!(ModelPricing::new(), ModelPricing::default());
    }

    #[test]
    fn test_deserialize_from_strings() {
        let json = r#"{
            "prompt": "0.000005",
            "completion": "0.000015",
            "request": "0.001",
            "cache_read": "0.000001"
        }"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, Some(0.000005));
        assert_eq!(pricing.completion_per_token, Some(0.000015));
        assert_eq!(pricing.web_search_per_request, Some(0.001));
        assert_eq!(pricing.input_cache_read_per_token, Some(0.000001));
    }

    #[test]
    fn test_deserialize_from_numbers() {
        let json = r#"{
            "prompt": 0.000005,
            "completion": 0.000015,
            "request": 0.001,
            "cache_read": 0.000001
        }"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, Some(0.000005));
        assert_eq!(pricing.completion_per_token, Some(0.000015));
        assert_eq!(pricing.web_search_per_request, Some(0.001));
        assert_eq!(pricing.input_cache_read_per_token, Some(0.000001));
    }

    #[test]
    fn test_deserialize_empty_strings() {
        let json = r#"{
            "prompt": "",
            "completion": ""
        }"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, None);
        assert_eq!(pricing.completion_per_token, None);
    }

    #[test]
    fn test_deserialize_nulls() {
        let json = r#"{
            "prompt": null,
            "completion": null
        }"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, None);
        assert_eq!(pricing.completion_per_token, None);
    }

    #[test]
    fn test_deserialize_partial() {
        let json = r#"{
            "prompt": "0.000005"
        }"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, Some(0.000005));
        assert_eq!(pricing.completion_per_token, None);
        assert_eq!(pricing.web_search_per_request, None);
        assert_eq!(pricing.input_cache_read_per_token, None);
    }

    #[test]
    fn test_deserialize_empty_object() {
        let json = r#"{}"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, None);
    }

    #[test]
    fn test_roundtrip() {
        let pricing = ModelPricing {
            prompt_per_token: Some(0.000005),
            completion_per_token: Some(0.000015),
            web_search_per_request: None,
            input_cache_read_per_token: Some(0.000001),
        };
        let json = serde_json::to_string(&pricing).unwrap();
        let deserialized: ModelPricing = serde_json::from_str(&json).unwrap();
        assert_eq!(pricing, deserialized);
    }

    #[test]
    fn test_zero_values() {
        let json = r#"{
            "prompt": "0",
            "completion": 0
        }"#;
        let pricing: ModelPricing = serde_json::from_str(json).unwrap();
        assert_eq!(pricing.prompt_per_token, Some(0.0));
        assert_eq!(pricing.completion_per_token, Some(0.0));
    }
}
