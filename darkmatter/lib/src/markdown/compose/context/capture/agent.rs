use super::*;

pub(super) const KEYS: &[&str] = &["agent", "model"];

pub(crate) fn populate_agent(
    environment: &std::collections::HashMap<String, String>,
    values: &mut Map<String, Value>,
) {
    let agent = environment
        .get("AGENT")
        .cloned()
        .map(|s| s.trim_ascii().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let model = environment
        .get("MODEL")
        .cloned()
        .map(|s| s.trim_ascii().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string());

    values.insert("agent".into(), Value::String(agent));
    values.insert("model".into(), Value::String(model));
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::context::capture::{capture_runtime_context_for_groups, ContextGroup};

    fn environment(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
    }

    #[test]
    fn populate_agent_uses_env_values_with_trim() {
        let mut values = Map::new();
        populate_agent(&environment(&[("AGENT", "  claude  "), ("MODEL", "  sonnet-4  ")]), &mut values);
        assert_eq!(values.get("agent"), Some(&Value::String("claude".to_string())));
        assert_eq!(values.get("model"), Some(&Value::String("sonnet-4".to_string())));
    }

    #[test]
    fn populate_agent_defaults_when_missing_or_empty() {
        let mut values = Map::new();
        populate_agent(&environment(&[("MODEL", "   ")]), &mut values);
        assert_eq!(values.get("agent"), Some(&Value::String("unknown".to_string())));
        assert_eq!(values.get("model"), Some(&Value::String("default".to_string())));
    }

    #[test]
    fn capture_runtime_context_includes_agent_group_from_the_supplied_environment() {
        let supplied = environment(&[("AGENT", "opencode"), ("MODEL", "glm-5.2")]);
        let (values, _, _, _, _) =
            capture_runtime_context_for_groups(Path::new("."), &[ContextGroup::Agent], &supplied);
        assert_eq!(values.get("agent"), Some(&Value::String("opencode".to_string())));
        assert_eq!(values.get("model"), Some(&Value::String("glm-5.2".to_string())));
    }
}
