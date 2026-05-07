use serde_json::Value;

use crate::actions::{HookDecision, HookResponse};

pub(super) fn should_replace_selected(current: Option<&HookResponse>, candidate: &HookResponse) -> bool {
    match current {
        None => true,
        Some(existing) => {
            let existing_is_continue = matches!(existing.decision, Some(HookDecision::Continue));
            let candidate_is_continue = matches!(candidate.decision, Some(HookDecision::Continue));
            existing_is_continue && !candidate_is_continue
        }
    }
}

pub(super) fn parse_decision(value: &Value) -> Option<HookDecision> {
    let text = value.as_str()?.to_ascii_lowercase();
    match text.as_str() {
        "allow" | "approved" | "approve" => Some(HookDecision::Allow),
        "deny" | "denied" | "reject" | "rejected" => Some(HookDecision::Deny),
        "ask" => Some(HookDecision::Ask),
        "continue" => Some(HookDecision::Continue),
        _ => None,
    }
}

pub(super) fn dot_lookup<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(value, |acc, key| acc.get(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_overrides_continue() {
        let continue_response = HookResponse {
            decision: Some(HookDecision::Continue),
            ..HookResponse::default()
        };
        let deny_response = HookResponse {
            decision: Some(HookDecision::Deny),
            ..HookResponse::default()
        };

        assert!(should_replace_selected(
            Some(&continue_response),
            &deny_response
        ));
        assert!(!should_replace_selected(
            Some(&deny_response),
            &continue_response
        ));
    }
}
