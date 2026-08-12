use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::rigging::providers::models::ProviderModel;

/// Canonical serialized capability token for function/tool calling.
pub const CAPABILITY_FUNCTION_CALLING: &str = "function_calling";
/// Canonical serialized capability token for structured output.
pub const CAPABILITY_STRUCTURED_OUTPUT: &str = "structured_output";
/// Canonical serialized capability token for reasoning models.
pub const CAPABILITY_REASONING: &str = "reasoning";
/// Canonical serialized capability token for file input support.
pub const CAPABILITY_FILE_INPUT: &str = "file_input";

/// Maps a models.dev boolean capability field to Claudine's canonical token.
#[must_use]
pub fn canonical_models_dev_capability(field: &str) -> Option<&'static str> {
    match field {
        "tool_call" => Some(CAPABILITY_FUNCTION_CALLING),
        "structured_output" => Some(CAPABILITY_STRUCTURED_OUTPUT),
        "reasoning" => Some(CAPABILITY_REASONING),
        "attachment" => Some(CAPABILITY_FILE_INPUT),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelCapability {
    /// Use this when you want a fast and cheap model and you're sure that
    /// your cost savings will not impact the success of the query.
    ///
    /// **Note:** there are plenty of prompts which do not need the latest
    /// models to perform their task.
    FastCheap,
    /// A stack of fast models which are optimized for capability not cost.
    ///
    /// The **default** stack will use the latest "fast models" for all providers
    /// and tend to stack the US providers before Chinese and local options.
    ///
    /// Example model: `claude-haiku`
    Fast,
    /// A stack of models which provide strong capability while avoiding the
    /// top-stack models which might break the bank or be too slow.
    ///
    /// Typically these models will be _non-thinking_ or _low-thinking_ to
    /// keep performance/speed relatively quick.
    ///
    /// Example model: `claude-sonnet`
    Normal,
    /// A variant of the "Normal" capability which stacks all the less expensive
    /// models first and either _excludes_ the most expensive models in this class
    /// or puts at the end of the stack so that they are marked as _less preferred_.
    ///
    /// **Note:** The **default** stacks will not exclude but instead just put the
    /// the more expensive models at the _end_ of the stack. This allows more expensive
    /// models in this class to still be used as a fallback.
    NormalCheap,
    /// A stack of models which uses a similar class models as "Normal" but which have
    /// pushed to "think".
    NormalThinking,
    /// A stack of models which use a similar class of models as "NormalCheap" but which have
    /// pushed to "think".
    NormalThinkingCheap,

    NormalUltrathink,
    NormalCheapUltrathink,

    /// A stack of top-tier models only. These model's will not be asked to think in this
    /// setting so you'll get reasonably fast but smart responses.
    ///
    /// Example model: `claude-opus` (non-thinking)
    Smart,
    SmartCheap,

    /// A stack of top-tier models only who have been asked to operate in thinking mode.
    ///
    /// Example model: `claude-opus` (think)
    SmartThink,
    SmartCheapThink,

    /// A stack of top-tier models only who have been asked to take as much time as possible
    /// to think through the prompt (aka, `ultrathink`).
    ///
    /// Example model: `claude-opus` (ultrathink)
    SmartUltrathink,
    SmartCheapUltrathink,

    // Below you'll find some specialist settings for some edge cases

    // Creative models will have their temperatures lowered from default settings
    // to make them more creative
    /// A fast model which has had it's temperature turned down from the baseline to make it
    /// more creative.
    CreativeFast,
    CreativeNormal,
    CreativeSmart,

    // Literal models will have their temperature increased, not to 1 but very close
    /// A fast model which has had it's temperature turned up from the baseline to make it
    /// more literal.
    LiteralFast,
    /// A normal model which has had it's temperature turned up from the baseline to make it
    /// more literal.
    LiteralNormal,
    /// A top tier model which has had it's temperature turned up from the baseline to make it
    /// more literal
    LiteralSmart,

    /// When you want to specify exactly which model you want to use rather
    /// than a category targeting some capability.
    Specific(ProviderModel),
}

impl ModelCapability {
    fn kind_str(&self) -> Option<&'static str> {
        match self {
            Self::FastCheap => Some("FastCheap"),
            Self::Fast => Some("Fast"),
            Self::Normal => Some("Normal"),
            Self::NormalCheap => Some("NormalCheap"),
            Self::NormalThinking => Some("NormalThinking"),
            Self::NormalThinkingCheap => Some("NormalThinkingCheap"),
            Self::NormalUltrathink => Some("NormalUltrathink"),
            Self::NormalCheapUltrathink => Some("NormalCheapUltrathink"),
            Self::Smart => Some("Smart"),
            Self::SmartCheap => Some("SmartCheap"),
            Self::SmartThink => Some("SmartThink"),
            Self::SmartCheapThink => Some("SmartCheapThink"),
            Self::SmartUltrathink => Some("SmartUltrathink"),
            Self::SmartCheapUltrathink => Some("SmartCheapUltrathink"),
            Self::CreativeFast => Some("CreativeFast"),
            Self::CreativeNormal => Some("CreativeNormal"),
            Self::CreativeSmart => Some("CreativeSmart"),
            Self::LiteralFast => Some("LiteralFast"),
            Self::LiteralNormal => Some("LiteralNormal"),
            Self::LiteralSmart => Some("LiteralSmart"),
            Self::Specific(_) => None,
        }
    }

    fn parse_kind(value: &str) -> Option<Self> {
        match normalize_kind(value).as_str() {
            "fastcheap" => Some(Self::FastCheap),
            "fast" => Some(Self::Fast),
            "normal" => Some(Self::Normal),
            "normalcheap" => Some(Self::NormalCheap),
            "normalthinking" => Some(Self::NormalThinking),
            "normalthinkingcheap" => Some(Self::NormalThinkingCheap),
            "normalultrathink" => Some(Self::NormalUltrathink),
            "normalcheapultrathink" => Some(Self::NormalCheapUltrathink),
            "smart" => Some(Self::Smart),
            "smartcheap" => Some(Self::SmartCheap),
            "smartthink" => Some(Self::SmartThink),
            "smartcheapthink" => Some(Self::SmartCheapThink),
            "smartultrathink" => Some(Self::SmartUltrathink),
            "smartcheapultrathink" => Some(Self::SmartCheapUltrathink),
            "creativefast" => Some(Self::CreativeFast),
            "creativenormal" => Some(Self::CreativeNormal),
            "creativesmart" => Some(Self::CreativeSmart),
            "literalfast" => Some(Self::LiteralFast),
            "literalnormal" => Some(Self::LiteralNormal),
            "literalsmart" => Some(Self::LiteralSmart),
            _ => None,
        }
    }
}

impl Serialize for ModelCapability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(kind) = self.kind_str() {
            return serializer.serialize_str(kind);
        }

        if let Self::Specific(model) = self {
            let wire_id = model.wire_id();
            return serializer.serialize_str(&format!("Specific:{}", wire_id));
        }

        Err(serde::ser::Error::custom("Unknown model capability"))
    }
}

impl<'de> Deserialize<'de> for ModelCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModelCapabilityVisitor;

        impl<'de> Visitor<'de> for ModelCapabilityVisitor {
            type Value = ModelCapability;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_str("a model capability string like 'Fast' or 'Specific:provider/model'")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_model_capability(value).map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let key: Option<String> = map.next_key()?;
                let Some(key) = key else {
                    return Err(de::Error::custom("expected map with a single key"));
                };

                if !key.eq_ignore_ascii_case("specific") {
                    return Err(de::Error::custom("expected key 'Specific'"));
                }

                let model: ProviderModel = map.next_value()?;

                if map.next_key::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom("expected map with a single key"));
                }

                Ok(ModelCapability::Specific(model))
            }
        }

        deserializer.deserialize_any(ModelCapabilityVisitor)
    }
}

fn normalize_kind(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn parse_model_capability(value: &str) -> Result<ModelCapability, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("ModelCapability cannot be empty".to_string());
    }

    if let Some(model_id) = strip_specific_prefix(trimmed) {
        return parse_specific_model(model_id);
    }

    if trimmed.contains('/') {
        return parse_specific_model(trimmed);
    }

    ModelCapability::parse_kind(trimmed)
        .ok_or_else(|| format!("Unknown model capability '{}'", trimmed))
}

fn strip_specific_prefix(value: &str) -> Option<&str> {
    let prefix = "specific:";
    if value.len() >= prefix.len() && value[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(value[prefix.len()..].trim())
    } else {
        None
    }
}

fn parse_specific_model(value: &str) -> Result<ModelCapability, String> {
    let model_id = value.trim();
    if model_id.is_empty() {
        return Err("Specific model id cannot be empty".to_string());
    }

    ProviderModel::parse_wire_id(model_id)
        .map(ModelCapability::Specific)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rigging::providers::models::openai::ProviderModelOpenAi;

    #[test]
    fn test_serialize_simple_kind() {
        let json = serde_json::to_string(&ModelCapability::Fast).expect("serialize should work");
        assert_eq!(json, "\"Fast\"");
    }

    #[test]
    fn test_specific_roundtrip_with_prefix() {
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let capability = ModelCapability::Specific(model.clone());

        let json = serde_json::to_string(&capability).expect("serialize should work");
        assert_eq!(json, "\"Specific:openai/o3\"");

        let parsed: ModelCapability =
            serde_json::from_str("\"Specific:openai/o3\"").expect("deserialize should work");
        assert_eq!(parsed, capability);
    }

    #[test]
    fn test_specific_roundtrip_without_prefix() {
        let parsed: ModelCapability =
            serde_json::from_str("\"openai/o3\"").expect("deserialize should work");

        let expected = ModelCapability::Specific(ProviderModel::OpenAi(ProviderModelOpenAi::O3));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_specific_map_form() {
        let parsed: ModelCapability =
            serde_json::from_str("{\"Specific\":\"openai/o3\"}").expect("deserialize should work");

        let expected = ModelCapability::Specific(ProviderModel::OpenAi(ProviderModelOpenAi::O3));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_canonical_models_dev_capability_mapping() {
        assert_eq!(
            canonical_models_dev_capability("tool_call"),
            Some(CAPABILITY_FUNCTION_CALLING)
        );
        assert_eq!(
            canonical_models_dev_capability("structured_output"),
            Some(CAPABILITY_STRUCTURED_OUTPUT)
        );
        assert_eq!(
            canonical_models_dev_capability("reasoning"),
            Some(CAPABILITY_REASONING)
        );
        assert_eq!(
            canonical_models_dev_capability("attachment"),
            Some(CAPABILITY_FILE_INPUT)
        );
        assert_eq!(canonical_models_dev_capability("unknown"), None);
    }
}
