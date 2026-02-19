use regex::Regex;
#[cfg(test)]
use tracing::warn;

use crate::events::{AgenticEvent, EventMeta};

/// Check if an event matches using a precompiled regex.
pub fn matches_with_regex(matcher: Option<&Regex>, meta: &EventMeta) -> bool {
    let matcher = match matcher {
        Some(regex) => regex,
        None => return true,
    };

    match meta.event {
        AgenticEvent::BeforeTool | AgenticEvent::AfterTool | AgenticEvent::ToolError => {
            match &meta.tool_name {
                Some(name) => matcher.is_match(name),
                None => false,
            }
        }
        AgenticEvent::Notification => match &meta.notification_type {
            Some(ntype) => matcher.is_match(ntype),
            None => false,
        },
        // For all other events, matcher has no field to match against
        _ => true,
    }
}

/// Check if an event matches using an explicit pattern string.
///
/// Prefer [`matches_with_regex`] in runtime code so regex compilation is
/// performed at config-load time.
#[cfg(test)]
pub fn matches_with_pattern(pattern: Option<&str>, meta: &EventMeta) -> bool {
    let pattern = match pattern {
        Some(p) => p,
        None => return true,
    };

    let regex = match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(error) => {
            warn!(
                %pattern,
                %error,
                "Invalid regex in event matcher, skipping binding"
            );
            return false;
        }
    };

    matches_with_regex(Some(&regex), meta)
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

    #[test]
    fn no_matcher_returns_true() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(matches_with_pattern(None, &meta));
    }

    #[test]
    fn regex_matches_tool_name() {
        let meta_bash = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        let meta_edit = tool_meta(AgenticEvent::AfterTool, Some("Edit"));
        let meta_read = tool_meta(AgenticEvent::BeforeTool, Some("Read"));

        assert!(matches_with_pattern(Some("Bash|Edit"), &meta_bash));
        assert!(matches_with_pattern(Some("Bash|Edit"), &meta_edit));
        assert!(!matches_with_pattern(Some("Bash|Edit"), &meta_read));
    }

    #[test]
    fn regex_matches_tool_error() {
        let meta = tool_meta(AgenticEvent::ToolError, Some("Bash"));
        assert!(matches_with_pattern(Some("Bash"), &meta));
    }

    #[test]
    fn regex_matches_notification_type() {
        let meta_match = notification_meta(Some("permission_prompt"));
        let meta_nomatch = notification_meta(Some("info"));

        assert!(matches_with_pattern(
            Some("permission_prompt|ToolPermission"),
            &meta_match
        ));
        assert!(!matches_with_pattern(
            Some("permission_prompt|ToolPermission"),
            &meta_nomatch
        ));
    }

    #[test]
    fn tool_event_with_no_tool_name_returns_false() {
        let meta = tool_meta(AgenticEvent::BeforeTool, None);
        assert!(!matches_with_pattern(Some("Bash"), &meta));
    }

    #[test]
    fn notification_with_no_type_returns_false() {
        let meta = notification_meta(None);
        assert!(!matches_with_pattern(Some("info"), &meta));
    }

    #[test]
    fn invalid_regex_returns_false() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(!matches_with_pattern(Some("[invalid(regex"), &meta));
    }

    #[test]
    fn non_tool_event_with_matcher_returns_true() {
        let meta = tool_meta(AgenticEvent::SessionStart, None);
        assert!(matches_with_pattern(Some("anything"), &meta));
    }

    #[test]
    fn matches_with_pattern_function() {
        let meta = tool_meta(AgenticEvent::BeforeTool, Some("Bash"));
        assert!(matches_with_pattern(Some("Bash"), &meta));
        assert!(!matches_with_pattern(Some("Read"), &meta));
        assert!(matches_with_pattern(None, &meta));
    }
}
