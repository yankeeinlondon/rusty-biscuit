use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Cache control settings for prompt caching.
///
/// Prompt caching reduces costs by up to 90% for repeated context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CacheControl {
    /// Cache type (currently only "ephemeral" is supported).
    #[serde(rename = "type")]
    pub cache_type: String,

    /// Time-to-live for the cache entry.
    ///
    /// Valid values: "5m" (default), "1h"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

impl CacheControl {
    /// Creates a new ephemeral cache control with default TTL.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        }
    }

    /// Creates a new ephemeral cache control with 1-hour TTL.
    pub fn ephemeral_1h() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
        }
    }
}

/// A tool definition for the model to use.
///
/// Tools are defined using JSON Schema for the input parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    /// Unique name for the tool.
    ///
    /// Must match the pattern `^[a-zA-Z0-9_-]{1,64}$`.
    pub name: String,

    /// Human-readable description of what the tool does.
    ///
    /// Include when and how to use the tool. Be specific about trigger conditions.
    pub description: String,

    /// JSON Schema defining the input parameters.
    pub input_schema: serde_json::Value,

    /// Optional cache control for the tool definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl Tool {
    /// Creates a new tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            cache_control: None,
        }
    }

    /// Creates a tool with an object schema.
    ///
    /// Helper for the common case of tools with object-typed inputs.
    pub fn with_object_schema(
        name: impl Into<String>,
        description: impl Into<String>,
        properties: serde_json::Value,
        required: Vec<String>,
    ) -> Self {
        Self::new(
            name,
            description,
            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            }),
        )
    }
}

/// How the model should choose which tools to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools.
    Auto,
    /// Model must use at least one tool.
    Any,
    /// Model must use the specified tool.
    Tool {
        /// Name of the required tool.
        name: String,
    },
    /// Model should not use any tools.
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition() {
        let tool = Tool::with_object_schema(
            "calculator",
            "Perform math operations",
            serde_json::json!({
                "a": {"type": "number"},
                "b": {"type": "number"},
                "op": {"type": "string", "enum": ["add", "subtract"]}
            }),
            vec!["a".to_string(), "b".to_string(), "op".to_string()],
        );

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"calculator\""));
        assert!(json.contains("\"type\":\"object\""));
    }

    #[test]
    fn tool_choice_serialization() {
        assert_eq!(
            serde_json::to_string(&ToolChoice::Auto).unwrap(),
            "{\"type\":\"auto\"}"
        );
        assert_eq!(
            serde_json::to_string(&ToolChoice::Any).unwrap(),
            "{\"type\":\"any\"}"
        );

        let specific = ToolChoice::Tool {
            name: "calculator".to_string(),
        };
        let json = serde_json::to_string(&specific).unwrap();
        assert!(json.contains("\"type\":\"tool\""));
        assert!(json.contains("\"name\":\"calculator\""));
    }

    #[test]
    fn cache_control_creation() {
        let ephemeral = CacheControl::ephemeral();
        assert_eq!(ephemeral.cache_type, "ephemeral");
        assert!(ephemeral.ttl.is_none());

        let one_hour = CacheControl::ephemeral_1h();
        assert_eq!(one_hour.ttl, Some("1h".to_string()));
    }
}
