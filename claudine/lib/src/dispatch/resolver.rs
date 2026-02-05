use crate::events::{EventAction, EventBinding};

/// Resolve which actions to execute for an event binding.
///
/// Returns `(enabled, actions, matcher)`.
pub fn resolve_actions(binding: &EventBinding) -> (bool, Vec<&EventAction>, Option<&str>) {
    if !binding.enabled {
        return (false, vec![], None);
    }

    let actions = binding.actions.iter().collect();
    let matcher = binding.matcher.as_deref();
    (true, actions, matcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventAction;

    fn binding_with_actions(actions: Vec<EventAction>) -> EventBinding {
        EventBinding {
            enabled: true,
            actions,
            matcher: None,
        }
    }

    #[test]
    fn enabled_binding_returns_actions() {
        let binding = binding_with_actions(vec![EventAction::Speak {
            message: "hello".to_string(),
        }]);
        let (enabled, actions, matcher) = resolve_actions(&binding);
        assert!(enabled);
        assert_eq!(actions.len(), 1);
        assert!(matcher.is_none());
    }

    #[test]
    fn disabled_binding_returns_empty() {
        let binding = EventBinding {
            enabled: false,
            actions: vec![EventAction::Speak {
                message: "should not fire".to_string(),
            }],
            matcher: None,
        };
        let (enabled, actions, _) = resolve_actions(&binding);
        assert!(!enabled);
        assert!(actions.is_empty());
    }

    #[test]
    fn binding_with_matcher_returns_matcher() {
        let binding = EventBinding {
            enabled: true,
            actions: vec![],
            matcher: Some("Bash|Edit".to_string()),
        };
        let (_, _, matcher) = resolve_actions(&binding);
        assert_eq!(matcher, Some("Bash|Edit"));
    }
}
