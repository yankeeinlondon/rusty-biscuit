use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{ClaudineError, Result};

use super::catalog::{RuleGroup, ScanSurface};

/// Flat configuration for the Protect service.
///
/// Supports shorthand (`"protect": true`) and expanded form
/// (`"protect": { "rules": { ... }, "custom_patterns": [...] }`).
#[derive(Debug, Clone, Serialize)]
pub struct ProtectConfig {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "ProtectRuleToggles::is_default")]
    pub rules: ProtectRuleToggles,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_patterns: Vec<CustomPattern>,
}

impl Default for ProtectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: ProtectRuleToggles::default(),
            custom_patterns: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_custom_surface() -> ScanSurface {
    ScanSurface::BashCommand
}

impl<'de> Deserialize<'de> for ProtectConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value =
            serde_json::Value::deserialize(deserializer).map_err(serde::de::Error::custom)?;

        // Shorthand: bool
        if let Some(b) = value.as_bool() {
            return Ok(Self {
                enabled: b,
                rules: ProtectRuleToggles::default(),
                custom_patterns: Vec::new(),
            });
        }

        // Expanded: object — reject unknown top-level keys
        if let Some(map) = value.as_object() {
            let known = &["enabled", "rules", "custom_patterns"];
            for key in map.keys() {
                if !known.contains(&key.as_str()) {
                    return Err(serde::de::Error::unknown_field(key, known));
                }
            }
        } else {
            return Err(serde::de::Error::custom(
                "protect must be a boolean or object",
            ));
        }

        #[derive(Deserialize)]
        struct Expanded {
            #[serde(default = "default_true")]
            enabled: bool,
            #[serde(default)]
            rules: ProtectRuleToggles,
            #[serde(default)]
            custom_patterns: Vec<CustomPattern>,
        }

        let expanded: Expanded = serde_json::from_value(value).map_err(serde::de::Error::custom)?;

        Ok(Self {
            enabled: expanded.enabled,
            rules: expanded.rules,
            custom_patterns: expanded.custom_patterns,
        })
    }
}

impl ProtectConfig {
    pub fn is_group_enabled(&self, group: RuleGroup) -> bool {
        if !self.enabled {
            return false;
        }
        match self.rules.get(group) {
            None => true,
            Some(RuleGroupConfig::Toggle(enabled)) => *enabled,
            Some(RuleGroupConfig::Detailed(d)) => d.enabled,
        }
    }

    pub fn get_allow_paths(&self, group: RuleGroup) -> Option<&Vec<String>> {
        match self.rules.get(group) {
            Some(RuleGroupConfig::Detailed(d)) if !d.allow_paths.is_empty() => Some(&d.allow_paths),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        // Only filesystem_destruction and sensitive_paths support allow_paths.
        let non_allow_path_groups = [
            ("disk_manipulation", &self.rules.disk_manipulation),
            ("remote_execution", &self.rules.remote_execution),
            ("git_destructive", &self.rules.git_destructive),
            ("system_sabotage", &self.rules.system_sabotage),
            ("network_sabotage", &self.rules.network_sabotage),
            ("container_cloud", &self.rules.container_cloud),
            ("database_nukes", &self.rules.database_nukes),
            ("obfuscated_execution", &self.rules.obfuscated_execution),
            ("prompt_injection", &self.rules.prompt_injection),
            (
                "credential_exfiltration",
                &self.rules.credential_exfiltration,
            ),
        ];

        for (name, toggle) in non_allow_path_groups {
            if let Some(RuleGroupConfig::Detailed(d)) = toggle
                && !d.allow_paths.is_empty()
            {
                return Err(ClaudineError::ConfigValidation(format!(
                    "allow_paths is not supported for group `{name}`"
                )));
            }
        }

        for cp in &self.custom_patterns {
            Regex::new(&cp.pattern).map_err(|source| ClaudineError::ProtectRuleParse {
                pattern: cp.pattern.clone(),
                source,
            })?;
            if cp.surface == ScanSurface::WritePath {
                return Err(ClaudineError::ConfigValidation(
                    "custom pattern surface 'write_path' is not supported".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectRuleToggles {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filesystem_destruction: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_manipulation: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_execution: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_destructive: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_sabotage: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_sabotage: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_cloud: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_nukes: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obfuscated_execution: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_injection: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_exfiltration: Option<RuleGroupConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitive_paths: Option<RuleGroupConfig>,
}

impl ProtectRuleToggles {
    fn is_default(&self) -> bool {
        self.filesystem_destruction.is_none()
            && self.disk_manipulation.is_none()
            && self.remote_execution.is_none()
            && self.git_destructive.is_none()
            && self.system_sabotage.is_none()
            && self.network_sabotage.is_none()
            && self.container_cloud.is_none()
            && self.database_nukes.is_none()
            && self.obfuscated_execution.is_none()
            && self.prompt_injection.is_none()
            && self.credential_exfiltration.is_none()
            && self.sensitive_paths.is_none()
    }

    pub fn get(&self, group: RuleGroup) -> Option<&RuleGroupConfig> {
        match group {
            RuleGroup::FilesystemDestruction => self.filesystem_destruction.as_ref(),
            RuleGroup::DiskManipulation => self.disk_manipulation.as_ref(),
            RuleGroup::RemoteExecution => self.remote_execution.as_ref(),
            RuleGroup::GitDestructive => self.git_destructive.as_ref(),
            RuleGroup::SystemSabotage => self.system_sabotage.as_ref(),
            RuleGroup::NetworkSabotage => self.network_sabotage.as_ref(),
            RuleGroup::ContainerCloud => self.container_cloud.as_ref(),
            RuleGroup::DatabaseNukes => self.database_nukes.as_ref(),
            RuleGroup::ObfuscatedExecution => self.obfuscated_execution.as_ref(),
            RuleGroup::PromptInjection => self.prompt_injection.as_ref(),
            RuleGroup::CredentialExfiltration => self.credential_exfiltration.as_ref(),
            RuleGroup::SensitivePaths => self.sensitive_paths.as_ref(),
            RuleGroup::Custom => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleGroupConfig {
    Toggle(bool),
    Detailed(RuleGroupDetailedConfig),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleGroupDetailedConfig {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CustomPattern {
    pub name: String,
    pub pattern: String,
    #[serde(default = "default_custom_surface")]
    pub surface: ScanSurface,
}

impl Default for CustomPattern {
    fn default() -> Self {
        Self {
            name: String::new(),
            pattern: String::new(),
            surface: ScanSurface::BashCommand,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_true_enables_all_defaults() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!(true)).unwrap();
        assert!(config.enabled);
        assert!(config.custom_patterns.is_empty());
    }

    #[test]
    fn shorthand_false_disables() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!(false)).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn expanded_form_with_group_toggles() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "rules": {
                "git_destructive": false,
                "filesystem_destruction": {
                    "enabled": true,
                    "allow_paths": ["node_modules", "target"]
                }
            }
        }))
        .unwrap();

        assert!(config.enabled);
        assert_eq!(
            config.rules.git_destructive,
            Some(RuleGroupConfig::Toggle(false))
        );
        match &config.rules.filesystem_destruction {
            Some(RuleGroupConfig::Detailed(d)) => {
                assert!(d.enabled);
                assert_eq!(d.allow_paths, vec!["node_modules", "target"]);
            }
            other => panic!("expected Detailed, got {other:?}"),
        }
    }

    #[test]
    fn custom_patterns_parse() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "custom_patterns": [
                { "name": "no_prod_deploy", "pattern": "deploy.*production" }
            ]
        }))
        .unwrap();

        assert_eq!(config.custom_patterns.len(), 1);
        assert_eq!(config.custom_patterns[0].name, "no_prod_deploy");
        assert_eq!(config.custom_patterns[0].surface, ScanSurface::BashCommand);
    }

    #[test]
    fn custom_pattern_mcp_surface_parses() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "custom_patterns": [
                { "name": "no_webhook", "pattern": "webhook\\.site/.*", "surface": "mcp_response" }
            ]
        }))
        .unwrap();

        assert_eq!(config.custom_patterns[0].surface, ScanSurface::McpResponse);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_write_path_custom_surface() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "custom_patterns": [
                { "name": "bad", "pattern": ".*", "surface": "write_path" }
            ]
        }))
        .unwrap();

        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_allow_paths_on_non_filesystem_group() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "rules": {
                "git_destructive": {
                    "enabled": true,
                    "allow_paths": ["something"]
                }
            }
        }))
        .unwrap();

        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_invalid_custom_pattern_regex() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "custom_patterns": [
                { "name": "bad", "pattern": "[invalid" }
            ]
        }))
        .unwrap();

        assert!(config.validate().is_err());
    }

    #[test]
    fn is_group_enabled_defaults_to_true() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!(true)).unwrap();
        assert!(config.is_group_enabled(RuleGroup::FilesystemDestruction));
        assert!(config.is_group_enabled(RuleGroup::GitDestructive));
    }

    #[test]
    fn is_group_enabled_respects_toggle() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "rules": { "git_destructive": false }
        }))
        .unwrap();

        assert!(!config.is_group_enabled(RuleGroup::GitDestructive));
        assert!(config.is_group_enabled(RuleGroup::FilesystemDestruction));
    }

    #[test]
    fn get_allow_paths_returns_configured_list() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "rules": {
                "filesystem_destruction": {
                    "enabled": true,
                    "allow_paths": ["node_modules", "target"]
                }
            }
        }))
        .unwrap();

        let paths = config.get_allow_paths(RuleGroup::FilesystemDestruction);
        assert_eq!(
            paths,
            Some(&vec!["node_modules".to_string(), "target".to_string()])
        );
    }

    #[test]
    fn serialization_round_trips() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "rules": { "git_destructive": false },
            "custom_patterns": [
                { "name": "test", "pattern": "test.*pattern" }
            ]
        }))
        .unwrap();

        let json = serde_json::to_value(&config).unwrap();
        let round_tripped: ProtectConfig = serde_json::from_value(json).unwrap();
        assert!(!round_tripped.is_group_enabled(RuleGroup::GitDestructive));
        assert_eq!(round_tripped.custom_patterns.len(), 1);
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let result = serde_json::from_value::<ProtectConfig>(serde_json::json!({
            "posture": "strict",
            "rules": {}
        }));
        assert!(
            result.is_err(),
            "unknown field 'posture' should be rejected"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("posture"),
            "error should mention the unknown field: {err}"
        );
    }

    #[test]
    fn validate_accepts_allow_paths_on_sensitive_paths() {
        let config: ProtectConfig = serde_json::from_value(serde_json::json!({
            "rules": {
                "sensitive_paths": {
                    "enabled": true,
                    "allow_paths": ["/etc/resolv.conf"]
                }
            }
        }))
        .unwrap();
        assert!(
            config.validate().is_ok(),
            "sensitive_paths should support allow_paths"
        );
    }
}
