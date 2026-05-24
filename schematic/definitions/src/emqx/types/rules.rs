use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Rule action configuration.
///
/// ## Example
///
/// ```json
/// {
///   "function": "republish",
///   "args": {"topic": "alerts/temp"}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RuleAction {
    /// Action function name.
    pub function: String,

    /// Action arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

/// Rule definition.
///
/// ## Example
///
/// ```json
/// {
///   "id": "temp_alert",
///   "sql": "SELECT * FROM \"sensors/+/temp\" WHERE payload.value > 100",
///   "actions": [{"function": "republish", "args": {"topic": "alerts/temp"}}],
///   "enabled": true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RuleInfo {
    /// Rule identifier.
    pub id: String,

    /// SQL query for the rule.
    pub sql: String,

    /// List of actions to execute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<RuleAction>>,

    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Rule description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Rule metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Request body for creating or updating a rule.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CreateRuleBody {
    /// Rule identifier.
    pub id: String,

    /// SQL query for the rule.
    pub sql: String,

    /// List of actions to execute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<RuleAction>>,

    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Rule description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Rule metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response for rules list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListRulesResponse {
    /// List of rules.
    pub data: Vec<RuleInfo>,
}

/// Rule test request body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TestRuleBody {
    /// SQL query to test.
    pub sql: String,

    /// Context for testing (topic, payload, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// Rule test response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TestRuleResponse {
    /// Test result data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_info_deserialization() {
        let json = r#"{
            "id": "temp_alert",
            "sql": "SELECT * FROM \"sensors/#\"",
            "enabled": true
        }"#;
        let rule: RuleInfo = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "temp_alert");
        assert_eq!(rule.enabled, Some(true));
    }
}
