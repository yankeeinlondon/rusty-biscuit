//! Authored lifecycle scalars keyed by canonical typed surface path.
//!
//! The parsed stack keeps only [`darkmatter::markdown::compose::expression::Expr`]
//! trees, but a nested-span lint must read the authored text: a synthesized
//! literal and an authored quoted literal are indistinguishable once parsed.
//! [`LifecycleSourceMap`] projects the frontmatter the lifecycle config was
//! parsed from onto the paths `iter_stack_expression_surfaces` emits, so a
//! validator can look up the authored scalar for each surface the one canonical
//! iterator yields. Structural fields, arbitrary overlay keys, and array
//! indices are distinct path segments; display punctuation is never used as
//! identity. The naming rules mirror the parser's verb dispatch in
//! `action_shape.rs`; a surface the map cannot name is reported as an internal
//! validation error rather than skipped.

use std::collections::HashMap;
use std::fmt;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LifecyclePathSegment {
    Field(String),
    MapKey(String),
    Index(usize),
}

/// Structural identity for one lifecycle expression surface.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct LifecycleSurfacePath(Vec<LifecyclePathSegment>);

impl LifecycleSurfacePath {
    pub(super) fn root(field: impl Into<String>) -> Self {
        Self(vec![LifecyclePathSegment::Field(field.into())])
    }

    pub(super) fn field(&self, field: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(LifecyclePathSegment::Field(field.into()));
        path
    }

    pub(super) fn map_key(&self, key: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(LifecyclePathSegment::MapKey(key.into()));
        path
    }

    pub(super) fn index(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.0.push(LifecyclePathSegment::Index(index));
        path
    }
}

impl fmt::Display for LifecycleSurfacePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, segment) in self.0.iter().enumerate() {
            match segment {
                LifecyclePathSegment::Field(field) => {
                    if position > 0 {
                        f.write_str(".")?;
                    }
                    f.write_str(field)?;
                }
                LifecyclePathSegment::MapKey(key)
                    if key.starts_with(|c: char| c == '_' || c.is_ascii_alphabetic())
                        && key.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) =>
                {
                    write!(f, ".{key}")?;
                }
                LifecyclePathSegment::MapKey(key) => write!(f, "[{key:?}]")?,
                LifecyclePathSegment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

/// Authored lifecycle scalars keyed by canonical typed paths.
#[derive(Debug, Default)]
pub(super) struct LifecycleSourceMap {
    entries: HashMap<LifecycleSurfacePath, AuthoredValue>,
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
                    map.record(
                        LifecycleSurfacePath::root(event).field(predicate),
                        block.get(predicate),
                    );
                }
            }
            let Some(Value::Array(stack)) = block.get("stack") else {
                continue;
            };
            for (idx, item) in stack.iter().enumerate() {
                let Value::Object(item) = item else { continue };
                let prefix = LifecycleSurfacePath::root(event).field("stack").index(idx);
                map.record(prefix.field("when"), item.get("when"));
                match item.get("action") {
                    Some(Value::Array(actions)) => {
                        for (action_idx, action) in actions.iter().enumerate() {
                            map.record_action(&prefix.field("action").index(action_idx), action);
                        }
                    }
                    Some(action) => map.record_action(&prefix.field("action").index(0), action),
                    None => {}
                }
            }
        }
        map
    }

    /// The authored value at `path`, or `None` when nothing was recorded.
    pub(super) fn get(&self, path: &LifecycleSurfacePath) -> Option<&AuthoredValue> {
        self.entries.get(path)
    }

    fn record(&mut self, path: LifecycleSurfacePath, value: Option<&Value>) {
        let authored = match value {
            Some(Value::String(text)) => AuthoredValue::Text(text.clone()),
            Some(Value::Number(_) | Value::Bool(_)) => AuthoredValue::NonText,
            _ => return,
        };
        self.entries.insert(path, authored);
    }

    /// Record one action object's operands under `prefix` (`…action[j]`).
    fn record_action(&mut self, prefix: &LifecycleSurfacePath, action: &Value) {
        let Value::Object(obj) = action else { return };
        if let Some(value) = obj.get("set")
            && obj.keys().all(|key| matches!(key.as_str(), "set" | "no_error"))
        {
            self.record_with_value(&prefix.field("set"), value);
            return;
        }
        if let Some(Value::String(verb)) = obj.get("action") {
            self.record_key_value_action(prefix, verb, obj);
        } else if let [(verb, value)] = obj.iter().collect::<Vec<_>>().as_slice() {
            self.record_positional_action(prefix, verb, value);
        }
    }

    /// `{verb: value}` or `{verb: [values…]}`: argument `k` is the scalar or
    /// the array's `k`th element.
    fn record_positional_action(
        &mut self,
        prefix: &LifecycleSurfacePath,
        verb: &str,
        value: &Value,
    ) {
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
            let path = match single_field {
                Some(field) => prefix.field(field),
                None => prefix.field("arg").index(k),
            };
            self.record(path, Some(arg));
        }
    }

    /// `{action: verb, param: value, …}`, named the way `build_action_from_params`
    /// stores each parameter.
    fn record_key_value_action(
        &mut self,
        prefix: &LifecycleSurfacePath,
        verb: &str,
        obj: &serde_json::Map<String, Value>,
    ) {
        let params: Vec<(&String, &Value)> = obj
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "action" | "no_error" | "with"))
            .collect();
        if let Some(with) = obj.get("with") {
            self.record_with_value(&prefix.field("with"), with);
        }

        let is_control = matches!(
            verb,
            "stop" | "skip" | "error" | "proxy" | "retry" | "resume" | "defer"
        );
        if is_control || verb == "shell" {
            for (key, value) in params {
                self.record(prefix.field(key), Some(value));
            }
            return;
        }
        if CommunicationChannel::from_verb(verb).is_some() {
            let message = ["message", "text", "sound"]
                .into_iter()
                .find_map(|key| obj.get(key));
            self.record(prefix.field("message"), message);
            self.record(prefix.field("route"), obj.get("route"));
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
            self.record(prefix.field("arg").index(k), Some(value));
        }
    }

    fn record_with_value(&mut self, path: &LifecycleSurfacePath, value: &Value) {
        match value {
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.record_with_value(&path.index(i), item);
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    self.record_with_value(&path.map_key(key), item);
                }
            }
            scalar => self.record(path.clone(), Some(scalar)),
        }
    }
}
