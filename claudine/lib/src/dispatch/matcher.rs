use regex::Regex;
use tracing::warn;

use crate::events::{AgenticEvent, EventBinding, EventMeta};

/// Check if an event binding matches the given event metadata.
///
/// Uses regex against `tool_name` for tool events and
/// `notification_type` for notifications. No matcher = always true.
///
/// ## Returns
///
/// `true` if the binding should fire for this event, `false` otherwise.
pub fn matches(binding: &EventBinding, meta: &EventMeta) -> bool {
    matches_with_pattern(binding.matcher.as_deref(), meta)
}

/// Check if an event matches using an explicit pattern string.
///
/// This is used by both the default binding matcher and by provider
/// override matcher strings.
pub fn matches_with_pattern(pattern: Option<&str>, meta: &EventMeta) -> bool {
    let pattern = match pattern {
        Some(p) => p,
        None => return true,
    };

    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            warn!(
                %pattern,
                %e,
                "Invalid regex in event matcher, skipping binding"
            );
            return false;
        }
    };

    match meta.event {
        AgenticEvent::BeforeTool | AgenticEvent::AfterTool | AgenticEvent::ToolError => {
            match &meta.tool_name {
                Some(name) => re.is_match(name),
                None => false,
            }
        }
        AgenticEvent::Notification => match &meta.notification_type {
            Some(ntype) => re.is_match(ntype),
            None => false,
        },
        // For all other events, matcher has no field to match against
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn tool_meta(event: AgenticEvent, tool_name: Option<&str>) -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event,
            timestamp: Utc::now(),
            session_id: None,
            cwd: None,
            tool_name: tool_name.map(String::from),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn notification_meta(ntype: Option<&str>) -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::Notification,
            timestamp: Utc::now(),
            session_id: None,
            cwd: None,
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: ntype.map(String::from),
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn binding_with_matcher(matcher: Option<&str>) -> EventBinding {
        EventBinding {
            enabled: true,
            actions: vec![],
            matcher: matcher.map(String::from),
            overrides: HashMap::new(),
        }
    }

    #[test]
    fn no_matcher_returns_true() {
        let binding = binding_with_matcher(None);
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(matches(&binding, &meta));
    }

    #[test]
    fn regex_matches_tool_name() {
        let binding = binding_with_matcher(Some("Bash|Edit"));
        let meta_bash = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        let meta_edit = tool_meta(AgenticEvent::AfterTool, Some("Edit"));
        let meta_read = tool_meta(AgenticEvent::BeforeTool, Some("Read"));

        assert!(matches(&binding, &meta_bash));
        assert!(matches(&binding, &meta_edit));
        assert!(!matches(&binding, &meta_read));
    }

    #[test]
    fn regex_matches_tool_error() {
        let binding = binding_with_matcher(Some("Bash"));
        let meta = tool_meta(AgenticEvent::ToolError, Some("Bash"));
        assert!(matches(&binding, &meta));
    }

    #[test]
    fn regex_matches_notification_type() {
        let binding = binding_with_matcher(Some("permission_prompt|ToolPermission"));
        let meta_match = notification_meta(Some("permission_prompt"));
        let meta_nomatch = notification_meta(Some("info"));

        assert!(matches(&binding, &meta_match));
        assert!(!matches(&binding, &meta_nomatch));
    }

    #[test]
    fn tool_event_with_no_tool_name_returns_false() {
        let binding = binding_with_matcher(Some("Bash"));
        let meta = tool_meta(AgenticEvent::BeforeTool, None);
        assert!(!matches(&binding, &meta));
    }

    #[test]
    fn notification_with_no_type_returns_false() {
        let binding = binding_with_matcher(Some("info"));
        let meta = notification_meta(None);
        assert!(!matches(&binding, &meta));
    }

    #[test]
    fn invalid_regex_returns_false() {
        let binding = binding_with_matcher(Some("[invalid(regex"));
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(!matches(&binding, &meta));
    }

    #[test]
    fn non_tool_event_with_matcher_returns_true() {
        let binding = binding_with_matcher(Some("anything"));
        let meta = tool_meta(AgenticEvent::SessionStart, None);
        assert!(matches(&binding, &meta));
    }

    #[test]
    fn matches_with_pattern_function() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(matches_with_pattern(Some("Bash"), &meta));
        assert!(!matches_with_pattern(Some("Read"), &meta));
        assert!(matches_with_pattern(None, &meta));
    }
}
