//! Authored lifecycle scalars keyed by canonical surface property path.
//!
//! The parsed stack keeps only [`darkmatter::markdown::compose::expression::Expr`]
//! trees, but a nested-span lint must read the authored text: a synthesized
//! literal and an authored quoted literal are indistinguishable once parsed.
//! [`LifecycleSourceMap`] projects the frontmatter the lifecycle config was
//! parsed from onto the property paths `iter_stack_expression_surfaces` emits,
//! so a validator can look up the authored scalar for each surface the one
//! canonical iterator yields. The naming rules mirror the parser's verb
//! dispatch in `action_shape.rs`; a surface the map cannot name is reported as
//! an internal validation error rather than skipped.

use std::collections::HashMap;

use serde_json::Value;

use super::LifecycleSignal;
use super::actions::{CommunicationChannel, expression_function_signature, side_effect_signature};

/// An authored lifecycle value at one canonical property path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AuthoredValue {
    /// A YAML string scalar, as decoded by the frontmatter parser.
    Text(String),
    /// A number or boolean scalar; it holds no authored literal.
    NonText,
}

/// Authored lifecycle scalars keyed by canonical property path
/// (`success.stack[0].action[1].message`, `loop.while`, …).
#[derive(Debug, Default)]
pub(super) struct LifecycleSourceMap {
    entries: HashMap<String, AuthoredValue>,
}

impl LifecycleSourceMap {
    /// Build the map from the frontmatter object `parse_lifecycle_config` read.
    pub(super) fn from_frontmatter(frontmatter: &Value) -> Self {
        let mut map = Self::default();
        let Some(root) = frontmatter.as_object() else {
            return map;
        };
        for signal in LifecycleSignal::ALL {
            let event = signal.property_name();
            let Some(Value::Object(block)) = root.get(event) else {
                continue;
            };
            if signal == LifecycleSignal::Loop {
                for predicate in ["while", "until"] {
                    map.record(format!("{event}.{predicate}"), block.get(predicate));
                }
            }
            let Some(Value::Array(stack)) = block.get("stack") else {
                continue;
            };
            for (idx, item) in stack.iter().enumerate() {
                let Value::Object(item) = item else { continue };
                let prefix = format!("{event}.stack[{idx}]");
                map.record(format!("{prefix}.when"), item.get("when"));
                match item.get("action") {
                    Some(Value::Array(actions)) => {
                        for (action_idx, action) in actions.iter().enumerate() {
                            map.record_action(&format!("{prefix}.action[{action_idx}]"), action);
                        }
                    }
                    Some(action) => map.record_action(&format!("{prefix}.action[0]"), action),
                    None => {}
                }
            }
        }
        map
    }

    /// The authored value at `property`, or `None` when nothing was recorded.
    pub(super) fn get(&self, property: &str) -> Option<&AuthoredValue> {
        self.entries.get(property)
    }

    fn record(&mut self, property: String, value: Option<&Value>) {
        let authored = match value {
            Some(Value::String(text)) => AuthoredValue::Text(text.clone()),
            Some(Value::Number(_) | Value::Bool(_)) => AuthoredValue::NonText,
            _ => return,
        };
        self.entries.insert(property, authored);
    }

    /// Record one action object's operands under `prefix` (`…action[j]`).
    fn record_action(&mut self, prefix: &str, action: &Value) {
        let Value::Object(obj) = action else { return };
        if let Some(Value::String(verb)) = obj.get("action") {
            self.record_key_value_action(prefix, verb, obj);
        } else if let [(verb, value)] = obj.iter().collect::<Vec<_>>().as_slice() {
            self.record_positional_action(prefix, verb, value);
        }
    }

    /// `{verb: value}` or `{verb: [values…]}`: argument `k` is the scalar or
    /// the array's `k`th element.
    fn record_positional_action(&mut self, prefix: &str, verb: &str, value: &Value) {
        let args: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            scalar => vec![scalar],
        };
        let single_field = match verb {
            "error" => Some("reason"),
            "proxy" => Some("target"),
            "retry" => Some("max_attempts"),
            "resume" => Some("message"),
            "defer" => Some("delay"),
            "shell" => Some("command"),
            _ if CommunicationChannel::from_verb(verb).is_some() => Some("message"),
            _ => None,
        };
        for (k, arg) in args.into_iter().enumerate() {
            let property = match single_field {
                Some(field) => format!("{prefix}.{field}"),
                None => format!("{prefix}.arg[{k}]"),
            };
            self.record(property, Some(arg));
        }
    }

    /// `{action: verb, param: value, …}`, named the way `build_action_from_params`
    /// stores each parameter.
    fn record_key_value_action(
        &mut self,
        prefix: &str,
        verb: &str,
        obj: &serde_json::Map<String, Value>,
    ) {
        let params: Vec<(&String, &Value)> = obj
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "action" | "no_error" | "with"))
            .collect();
        if let Some(with) = obj.get("with") {
            self.record_with_value(&format!("{prefix}.with"), with);
        }

        let is_control = matches!(
            verb,
            "stop" | "skip" | "error" | "proxy" | "retry" | "resume" | "defer"
        );
        if is_control || verb == "shell" {
            for (key, value) in params {
                self.record(format!("{prefix}.{key}"), Some(value));
            }
            return;
        }
        if CommunicationChannel::from_verb(verb).is_some() {
            let message = ["message", "text", "sound"]
                .into_iter()
                .find_map(|key| obj.get(key));
            self.record(format!("{prefix}.message"), message);
            self.record(format!("{prefix}.route"), obj.get("route"));
            return;
        }

        let signature = expression_function_signature(verb)
            .filter(|signature| !signature.variadic)
            .or_else(|| side_effect_signature(verb));
        let ordered: Vec<&Value> = match signature {
            // Absent optional parameters are skipped, so an argument's index
            // counts only the parameters actually present.
            Some(signature) => signature
                .params
                .iter()
                .filter_map(|name| obj.get(name))
                .collect(),
            None => {
                let mut sorted = params;
                sorted.sort_by(|a, b| a.0.cmp(b.0));
                sorted.into_iter().map(|(_, value)| value).collect()
            }
        };
        for (k, value) in ordered.into_iter().enumerate() {
            self.record(format!("{prefix}.arg[{k}]"), Some(value));
        }
    }

    fn record_with_value(&mut self, path: &str, value: &Value) {
        match value {
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.record_with_value(&format!("{path}[{i}]"), item);
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    self.record_with_value(&format!("{path}.{key}"), item);
                }
            }
            scalar => self.record(path.to_string(), Some(scalar)),
        }
    }
}
