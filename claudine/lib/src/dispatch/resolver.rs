use crate::events::{EventAction, EventBinding, Provider};

/// Resolve which actions to execute for an event binding.
///
/// Checks provider-specific overrides first, falls back to binding defaults.
/// Returns `(enabled, actions, optional_matcher_override)`.
///
/// ## Returns
///
/// A tuple of:
/// - `enabled`: whether this binding should fire
/// - `actions`: references to the resolved action list
/// - `matcher`: optional regex override from the provider override
pub fn resolve_actions(
    binding: &EventBinding,
    provider: Provider,
) -> (bool, Vec<&EventAction>, Option<&str>) {
    // Binding disabled at top level: short-circuit
    if !binding.enabled {
        return (false, vec![], None);
    }

    // Check for a provider-specific override
    if let Some(override_) = binding.overrides.get(&provider) {
        if !override_.enabled {
            return (false, vec![], None);
        }
        let actions = override_.actions.iter().collect();
        let matcher = override_.matcher.as_deref();
        return (true, actions, matcher);
    }

    // Fall back to binding's own actions
    let actions = binding.actions.iter().collect();
    (true, actions, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventAction, EventBinding, Provider, ProviderOverride};
    use std::collections::HashMap;

    fn binding_with_actions(actions: Vec<EventAction>) -> EventBinding {
        EventBinding {
            enabled: true,
            actions,
            matcher: None,
            overrides: HashMap::new(),
        }
    }

    #[test]
    fn no_override_returns_binding_actions() {
        let binding = binding_with_actions(vec![
            EventAction::Speak {
                message: "hello".to_string(),
            },
        ]);
        let (enabled, actions, matcher) = resolve_actions(&binding, Provider::Claude);
        assert!(enabled);
        assert_eq!(actions.len(), 1);
        assert!(matcher.is_none());
    }

    #[test]
    fn override_present_returns_override_actions() {
        let mut binding = binding_with_actions(vec![EventAction::Speak {
            message: "default".to_string(),
        }]);
        binding.overrides.insert(
            Provider::Claude,
            ProviderOverride {
                enabled: true,
                actions: vec![EventAction::Speak {
                    message: "claude-specific".to_string(),
                }],
                matcher: Some("Bash".to_string()),
            },
        );

        let (enabled, actions, matcher) = resolve_actions(&binding, Provider::Claude);
        assert!(enabled);
        assert_eq!(actions.len(), 1);
        match actions[0] {
            EventAction::Speak { message } => assert_eq!(message, "claude-specific"),
            _ => panic!("Expected Speak action"),
        }
        assert_eq!(matcher, Some("Bash"));
    }

    #[test]
    fn disabled_binding_returns_enabled_false() {
        let binding = EventBinding {
            enabled: false,
            actions: vec![EventAction::Speak {
                message: "should not fire".to_string(),
            }],
            matcher: None,
            overrides: HashMap::new(),
        };
        let (enabled, actions, _) = resolve_actions(&binding, Provider::Claude);
        assert!(!enabled);
        assert!(actions.is_empty());
    }

    #[test]
    fn disabled_override_returns_enabled_false() {
        let mut binding = binding_with_actions(vec![EventAction::Speak {
            message: "default".to_string(),
        }]);
        binding.overrides.insert(
            Provider::Codex,
            ProviderOverride {
                enabled: false,
                actions: vec![EventAction::Speak {
                    message: "codex override".to_string(),
                }],
                matcher: None,
            },
        );

        let (enabled, actions, _) = resolve_actions(&binding, Provider::Codex);
        assert!(!enabled);
        assert!(actions.is_empty());
    }

    #[test]
    fn unrelated_provider_override_uses_binding_defaults() {
        let mut binding = binding_with_actions(vec![EventAction::Speak {
            message: "default".to_string(),
        }]);
        binding.overrides.insert(
            Provider::Claude,
            ProviderOverride {
                enabled: true,
                actions: vec![EventAction::Speak {
                    message: "claude-only".to_string(),
                }],
                matcher: None,
            },
        );

        // Query for Gemini -- no override, gets binding defaults
        let (enabled, actions, _) = resolve_actions(&binding, Provider::Gemini);
        assert!(enabled);
        assert_eq!(actions.len(), 1);
        match actions[0] {
            EventAction::Speak { message } => assert_eq!(message, "default"),
            _ => panic!("Expected Speak action"),
        }
    }

    #[test]
    fn override_without_matcher_returns_none() {
        let mut binding = binding_with_actions(vec![]);
        binding.overrides.insert(
            Provider::Claude,
            ProviderOverride {
                enabled: true,
                actions: vec![],
                matcher: None,
            },
        );
        let (_, _, matcher) = resolve_actions(&binding, Provider::Claude);
        assert!(matcher.is_none());
    }
}
