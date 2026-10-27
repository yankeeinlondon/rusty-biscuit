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
impl schematic_define::Registration for RuleAction {
    const RUST_IDENTIFIER: Option<&'static str> = Some("RuleAction");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
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
impl schematic_define::Registration for RuleInfo {
    const RUST_IDENTIFIER: Option<&'static str> = Some("RuleInfo");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
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
impl schematic_define::Registration for CreateRuleBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("CreateRuleBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


/// Response for rules list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListRulesResponse {
    /// List of rules.
    pub data: Vec<RuleInfo>,
}
impl schematic_define::Registration for ListRulesResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ListRulesResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
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
impl schematic_define::Registration for TestRuleBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("TestRuleBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


/// Rule test response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TestRuleResponse {
    /// Test result data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
impl schematic_define::Registration for TestRuleResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("TestRuleResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
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
