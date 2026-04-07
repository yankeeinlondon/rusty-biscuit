use serde::{Deserialize, Serialize};

use super::catalog::{RuleGroup, ScanSurface};

/// Binary outcome of a protect evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectOutcome {
    Allow,
    Block,
}

/// Details of the matched rule when an action is blocked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectMatch {
    pub group: RuleGroup,
    pub rule_id: String,
    pub pattern: String,
    pub matched_text: String,
    pub surface: ScanSurface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
    pub config_key: String,
}

/// Result of a single protect evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<ProtectMatch>,
}

impl ProtectDecision {
    pub fn allow() -> Self {
        Self { outcome: ProtectOutcome::Allow, blocked: None }
    }

    pub fn blocked(m: ProtectMatch) -> Self {
        Self { outcome: ProtectOutcome::Block, blocked: Some(m) }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self.outcome, ProtectOutcome::Block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_decision_is_not_blocked() {
        let decision = ProtectDecision::allow();
        assert!(matches!(decision.outcome, ProtectOutcome::Allow));
        assert!(decision.blocked.is_none());
        assert!(!decision.is_blocked());
    }

    #[test]
    fn block_decision_carries_match_info() {
        let m = ProtectMatch {
            group: RuleGroup::FilesystemDestruction,
            rule_id: "rm_recursive_force".to_string(),
            pattern: r"rm\s+-rf".to_string(),
            matched_text: "rm -rf".to_string(),
            surface: ScanSurface::BashCommand,
            target_path: None,
            config_key: "protect.rules.filesystem_destruction".to_string(),
        };
        let decision = ProtectDecision::blocked(m);
        assert!(decision.is_blocked());
        assert_eq!(decision.blocked.as_ref().unwrap().rule_id, "rm_recursive_force");
    }
}
