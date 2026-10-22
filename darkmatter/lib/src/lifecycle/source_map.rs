//! Authored lifecycle scalars keyed by canonical typed surface path.
//!
//! The parsed stack keeps only [`crate::markdown::compose::expression::Expr`]
//! trees, but a nested-span lint must read the authored text: a synthesized
//! literal and an authored quoted literal are indistinguishable once parsed.
//! [`LifecycleSourceMap`] projects the frontmatter the lifecycle config was
//! parsed from onto the paths `walk_stack` emits, so a
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
use super::action_shape::positional_operand_field;
use super::actions::{
    CommunicationChannel, expression_function_signature, is_lifecycle_control_verb, side_effect_signature,
};
use super::parse::LOOP_ITERATION_KEYS;
use super::registry::EventRegistry;

/// An authored lifecycle value at one canonical property path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoredValue {
    /// A YAML string scalar, as decoded by the frontmatter parser.
    Text(String),
    /// A number or boolean scalar; it holds no authored literal.
    NonText,
}

/// One segment of a [`LifecyclePath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LifecyclePathSegment {
    /// A grammar key: an event, `then`, `else`, a verb, or a parameter.
    Field(String),
    /// An arbitrary authored mapping key, such as a `set` or `with` key.
    MapKey(String),
    /// A list position.
    Index(usize),
}

/// The authored location of a lifecycle node, action, or operand.
///
/// Paths name what the author wrote, never an execution position: expanding a
/// dictionary bundle keeps each action at its verb key, so the second action of
/// `failure[0].else` written as `- shell: …` is `failure[0].else[1].shell`, and
/// its command operand is `failure[0].else[1].shell.command`. Diagnostics carry
/// the rendered form; the source map and the expression walker key on the
/// structural form, so display punctuation is never identity.
///
/// A [`MapKey`](LifecyclePathSegment::MapKey) segment that is not an
/// identifier renders bracket-quoted (`with["a.b"]`) so a key containing a dot
/// cannot read as two segments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LifecyclePath(Vec<LifecyclePathSegment>);

impl LifecyclePath {
    /// A path rooted at `field`: an event name, or a task's source-rooted
    /// stack property such as `tasks[0].setup`.
    pub fn root(field: impl Into<String>) -> Self {
        Self(vec![LifecyclePathSegment::Field(field.into())])
    }

    #[allow(missing_docs)]
    pub fn field(&self, field: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(LifecyclePathSegment::Field(field.into()));
        path
    }

    #[allow(missing_docs)]
    pub fn map_key(&self, key: impl Into<String>) -> Self {
        let mut path = self.clone();
        path.0.push(LifecyclePathSegment::MapKey(key.into()));
        path
    }

    #[allow(missing_docs)]
    pub fn index(&self, index: usize) -> Self {
        let mut path = self.clone();
        path.0.push(LifecyclePathSegment::Index(index));
        path
    }

    /// The path without its last segment; the root is its own parent.
    pub fn parent(&self) -> Self {
        let mut path = self.clone();
        if path.0.len() > 1 {
            path.0.pop();
        }
        path
    }

    /// The first segment as authored: the event name or task stack property.
    pub fn root_name(&self) -> &str {
        match self.0.first() {
            Some(LifecyclePathSegment::Field(field) | LifecyclePathSegment::MapKey(field)) => field,
            _ => "",
        }
    }
}

impl fmt::Display for LifecyclePath {
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
pub struct LifecycleSourceMap {
    entries: HashMap<LifecyclePath, AuthoredValue>,
}

impl LifecycleSourceMap {
    /// Build the map from the frontmatter object `parse_lifecycle_config` read,
    /// with every event under its engine slot's name (Claudine's spelling).
    pub fn from_frontmatter(frontmatter: &Value) -> Self {
        Self::from_frontmatter_named(frontmatter, |signal| Some(signal.property_name()))
    }

    /// [`Self::from_frontmatter`] with each event under its name in
    /// `registry`; a slot the host does not expose is ordinary data.
    pub fn from_frontmatter_in(frontmatter: &Value, registry: &EventRegistry) -> Self {
        Self::from_frontmatter_named(frontmatter, |signal| registry.name_of(signal))
    }

    fn from_frontmatter_named<'n>(frontmatter: &Value, name: impl Fn(LifecycleSignal) -> Option<&'n str>) -> Self {
        let mut map = Self::default();
        let Some(root) = frontmatter.as_object() else {
            return map;
        };
        for signal in LifecycleSignal::ALL {
            let Some(event) = name(signal) else {
                continue;
            };
            let path = LifecyclePath::root(event);
            if signal != LifecycleSignal::Loop {
                if let Some(stack) = root.get(event) {
                    map.record_stack(&path, stack);
                }
                continue;
            }
            let Some(Value::Object(block)) = root.get(event) else {
                continue;
            };
            for predicate in ["while", "until"] {
                map.record(path.field(predicate), block.get(predicate));
            }
            match block.get("gate") {
                Some(gate) => map.record_stack(&path.field("gate"), gate),
                None => {
                    let concerns: serde_json::Map<String, Value> = block
                        .iter()
                        .filter(|(key, _)| !LOOP_ITERATION_KEYS.contains(&key.as_str()))
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect();
                    map.record_dictionary(&path, &concerns);
                }
            }
        }
        map
    }

    /// The authored value at `path`, or `None` when nothing was recorded.
    pub fn get(&self, path: &LifecyclePath) -> Option<&AuthoredValue> {
        self.entries.get(path)
    }

    fn record(&mut self, path: LifecyclePath, value: Option<&Value>) {
        let authored = match value {
            Some(Value::String(text)) => AuthoredValue::Text(text.clone()),
            Some(Value::Number(_) | Value::Bool(_)) => AuthoredValue::NonText,
            _ => return,
        };
        self.entries.insert(path, authored);
    }

    /// A stack or `then`/`else` body: a list of items or a dictionary.
    fn record_stack(&mut self, path: &LifecyclePath, value: &Value) {
        match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    if let Value::Object(item) = item {
                        self.record_item(&path.index(index), item);
                    }
                }
            }
            Value::Object(map) => self.record_dictionary(path, map),
            _ => {}
        }
    }

    /// One list item: an optional guard and branches, then its verbs.
    fn record_item(&mut self, path: &LifecyclePath, item: &serde_json::Map<String, Value>) {
        self.record(path.field("when"), item.get("when"));
        for branch in ["then", "else"] {
            if let Some(body) = item.get(branch) {
                self.record_stack(&path.field(branch), body);
            }
        }
        self.record_dictionary(path, item);
    }

    /// Every verb of a dictionary, each at `{path}.{verb}`.
    fn record_dictionary(&mut self, path: &LifecyclePath, map: &serde_json::Map<String, Value>) {
        for (verb, value) in map {
            if matches!(verb.as_str(), "when" | "then" | "else" | "no_error") {
                continue;
            }
            let verb_path = path.field(verb.as_str());
            match value {
                _ if verb == "set" => self.record_with_value(&verb_path, value),
                Value::Object(params) => self.record_long_form(&verb_path, verb, params),
                _ => self.record_short_form(&verb_path, verb, value),
            }
        }
    }

    /// `verb: value` or `verb: [values…]`: argument `k` is the scalar or the
    /// array's `k`th element, named the way the walker names the operand.
    fn record_short_form(&mut self, prefix: &LifecyclePath, verb: &str, value: &Value) {
        let args: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            scalar => vec![scalar],
        };
        let single_field = positional_operand_field(verb);
        for (k, arg) in args.into_iter().enumerate() {
            let path = match single_field {
                Some(field) => prefix.field(field),
                None => prefix.field("arg").index(k),
            };
            self.record(path, Some(arg));
        }
    }

    /// `verb: { param: value, … }`, named the way `build_action_from_params`
    /// stores each parameter.
    fn record_long_form(
        &mut self,
        prefix: &LifecyclePath,
        verb: &str,
        obj: &serde_json::Map<String, Value>,
    ) {
        let params: Vec<(&String, &Value)> = obj
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "no_error" | "with"))
            .collect();
        if let Some(with) = obj.get("with") {
            self.record_with_value(&prefix.field("with"), with);
        }

        if is_lifecycle_control_verb(verb) || verb == "shell" {
            for (key, value) in params {
                self.record(prefix.field(key.as_str()), Some(value));
            }
            return;
        }
        if CommunicationChannel::from_verb(verb).is_some() {
            self.record(prefix.field("text"), obj.get("text"));
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

    fn record_with_value(&mut self, path: &LifecyclePath, value: &Value) {
        match value {
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.record_with_value(&path.index(i), item);
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    self.record_with_value(&path.map_key(key.as_str()), item);
                }
            }
            scalar => self.record(path.clone(), Some(scalar)),
        }
    }
}
