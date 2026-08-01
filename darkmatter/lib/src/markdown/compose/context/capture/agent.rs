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

    /// Helper: run a closure with the given env var set, restoring the
    /// previous value (or unset state) afterwards. Tests using this are
    /// marked `#[serial]` to avoid racing with other env-mutating tests.
    fn with_env_var<F, R>(key: &str, value: Option<&str>, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let previous = std::env::var(key).ok();
        // `set_var`/`remove_var` are unsafe in Rust 2024. Serial_test isolates
        // these tests from each other, and the helper restores the prior value
        // so the mutation does not leak past the closure.
        unsafe {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let result = f();
        unsafe {
            match previous {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        result
    }

    #[test]
    #[serial_test::serial(env_agent_model)]
    fn populate_agent_uses_env_values_with_trim() {
        with_env_var("AGENT", Some("  claude  "), || {
            with_env_var("MODEL", Some("  sonnet-4  "), || {
                let mut values = Map::new();
                populate_agent(&std::env::vars().collect(), &mut values);
                assert_eq!(
                    values.get("agent"),
                    Some(&Value::String("claude".to_string()))
                );
                assert_eq!(
                    values.get("model"),
                    Some(&Value::String("sonnet-4".to_string()))
                );
            })
        });
    }

    #[test]
    #[serial_test::serial(env_agent_model)]
    fn populate_agent_defaults_when_missing_or_empty() {
        with_env_var("AGENT", None, || {
            with_env_var("MODEL", Some("   "), || {
                let mut values = Map::new();
                populate_agent(&std::env::vars().collect(), &mut values);
                assert_eq!(
                    values.get("agent"),
                    Some(&Value::String("unknown".to_string()))
                );
                assert_eq!(
                    values.get("model"),
                    Some(&Value::String("default".to_string()))
                );
            })
        });
    }

    #[test]
    #[serial_test::serial(env_agent_model)]
    fn capture_runtime_context_includes_agent_group() {
        with_env_var("AGENT", Some("opencode"), || {
            with_env_var("MODEL", Some("glm-5.2"), || {
                let (values, _, _, _) =
                    capture_runtime_context_for_groups(Path::new("."), &[ContextGroup::Agent]);
                assert_eq!(
                    values.get("agent"),
                    Some(&Value::String("opencode".to_string()))
                );
                assert_eq!(
                    values.get("model"),
                    Some(&Value::String("glm-5.2".to_string()))
                );
            })
        });
    }
}
