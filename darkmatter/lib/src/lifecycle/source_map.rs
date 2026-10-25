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

/// One step from the frontmatter root to an authored scalar: a mapping key
/// or a list position, exactly as the YAML holds it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AuthoredSegment {
    Key(String),
    Index(usize),
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
    /// Where each recorded scalar sits in the frontmatter, so an editor can
    /// range a surface's findings on its authored bytes.
    locations: HashMap<LifecyclePath, Vec<AuthoredSegment>>,
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
            let at = vec![AuthoredSegment::Key(event.to_string())];
            if signal != LifecycleSignal::Loop {
                if let Some(stack) = root.get(event) {
                    map.record_stack(&path, stack, &at);
                }
                continue;
            }
            let Some(Value::Object(block)) = root.get(event) else {
                continue;
            };
            for predicate in ["while", "until"] {
                map.record(path.field(predicate), block.get(predicate), &child_key(&at, predicate));
            }
            match block.get("gate") {
                Some(gate) => map.record_stack(&path.field("gate"), gate, &child_key(&at, "gate")),
                None => {
                    let concerns: serde_json::Map<String, Value> = block
                        .iter()
                        .filter(|(key, _)| !LOOP_ITERATION_KEYS.contains(&key.as_str()))
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect();
                    map.record_dictionary(&path, &concerns, &at);
                }
            }
        }
        map
    }

    /// The authored value at `path`, or `None` when nothing was recorded.
    pub fn get(&self, path: &LifecyclePath) -> Option<&AuthoredValue> {
        self.entries.get(path)
    }

    /// Where the authored scalar at `path` sits in the frontmatter, or
    /// `None` when nothing was recorded there.
    pub fn location(&self, path: &LifecyclePath) -> Option<&[AuthoredSegment]> {
        self.locations.get(path).map(Vec::as_slice)
    }

    fn record(&mut self, path: LifecyclePath, value: Option<&Value>, at: &[AuthoredSegment]) {
        let authored = match value {
            Some(Value::String(text)) => AuthoredValue::Text(text.clone()),
            Some(Value::Number(_) | Value::Bool(_)) => AuthoredValue::NonText,
            _ => return,
        };
        self.locations.insert(path.clone(), at.to_vec());
        self.entries.insert(path, authored);
    }

    /// A stack or `then`/`else` body: a list of items or a dictionary.
    fn record_stack(&mut self, path: &LifecyclePath, value: &Value, at: &[AuthoredSegment]) {
        match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    if let Value::Object(item) = item {
                        self.record_item(&path.index(index), item, &child_index(at, index));
                    }
                }
            }
            Value::Object(map) => self.record_dictionary(path, map, at),
            _ => {}
        }
    }

    /// One list item: an optional guard and branches, then its verbs.
    fn record_item(&mut self, path: &LifecyclePath, item: &serde_json::Map<String, Value>, at: &[AuthoredSegment]) {
        self.record(path.field("when"), item.get("when"), &child_key(at, "when"));
        for branch in ["then", "else"] {
            if let Some(body) = item.get(branch) {
                self.record_stack(&path.field(branch), body, &child_key(at, branch));
            }
        }
        self.record_dictionary(path, item, at);
    }

    /// Every verb of a dictionary, each at `{path}.{verb}`.
    fn record_dictionary(
        &mut self,
        path: &LifecyclePath,
        map: &serde_json::Map<String, Value>,
        at: &[AuthoredSegment],
    ) {
        for (verb, value) in map {
            if matches!(verb.as_str(), "when" | "then" | "else" | "no_error") {
                continue;
            }
            let verb_path = path.field(verb.as_str());
            let at = child_key(at, verb);
            match value {
                _ if verb == "set" => self.record_with_value(&verb_path, value, &at),
                Value::Object(params) => self.record_long_form(&verb_path, verb, params, &at),
                _ => self.record_short_form(&verb_path, verb, value, &at),
            }
        }
    }

    /// `verb: value` or `verb: [values…]`: argument `k` is the scalar or the
    /// array's `k`th element, named the way the walker names the operand.
    fn record_short_form(&mut self, prefix: &LifecyclePath, verb: &str, value: &Value, at: &[AuthoredSegment]) {
        let args: Vec<(&Value, Vec<AuthoredSegment>)> = match value {
            Value::Array(items) => items.iter().enumerate().map(|(k, item)| (item, child_index(at, k))).collect(),
            scalar => vec![(scalar, at.to_vec())],
        };
        let single_field = positional_operand_field(verb);
        for (k, (arg, arg_at)) in args.into_iter().enumerate() {
            let path = match single_field {
                Some(field) => prefix.field(field),
                None => prefix.field("arg").index(k),
            };
            self.record(path, Some(arg), &arg_at);
        }
    }

    /// `verb: { param: value, … }`, named the way `build_action_from_params`
    /// stores each parameter.
    fn record_long_form(
        &mut self,
        prefix: &LifecyclePath,
        verb: &str,
        obj: &serde_json::Map<String, Value>,
        at: &[AuthoredSegment],
    ) {
        let params: Vec<(&String, &Value)> = obj
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "no_error" | "with"))
            .collect();
        if let Some(with) = obj.get("with") {
            self.record_with_value(&prefix.field("with"), with, &child_key(at, "with"));
        }

        if is_lifecycle_control_verb(verb) || verb == "shell" {
            for (key, value) in params {
                self.record(prefix.field(key.as_str()), Some(value), &child_key(at, key));
            }
            return;
        }
        if CommunicationChannel::from_verb(verb).is_some() {
            self.record(prefix.field("text"), obj.get("text"), &child_key(at, "text"));
            return;
        }

        let keys: Vec<&str> = params.iter().map(|(key, _)| key.as_str()).collect();
        let overload = expression_function_signature(verb)
            .filter(|signature| !signature.is_variadic())
            .or_else(|| side_effect_signature(verb))
            .and_then(|signature| signature.select_named(&keys).ok());
        let ordered: Vec<(&str, &Value)> = match overload {
            // The overload the parser selects orders the arguments; absent
            // optional parameters are skipped, so an argument's index counts
            // only the parameters actually present.
            Some(overload) => overload.names().filter_map(|name| obj.get(name).map(|value| (name, value))).collect(),
            None => {
                let mut sorted = params;
                sorted.sort_by(|a, b| a.0.cmp(b.0));
                sorted.into_iter().map(|(key, value)| (key.as_str(), value)).collect()
            }
        };
        for (k, (key, value)) in ordered.into_iter().enumerate() {
            self.record(prefix.field("arg").index(k), Some(value), &child_key(at, key));
        }
    }

    fn record_with_value(&mut self, path: &LifecyclePath, value: &Value, at: &[AuthoredSegment]) {
        match value {
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.record_with_value(&path.index(i), item, &child_index(at, i));
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    self.record_with_value(&path.map_key(key.as_str()), item, &child_key(at, key));
                }
            }
            scalar => self.record(path.clone(), Some(scalar), at),
        }
    }
}

fn child_key(at: &[AuthoredSegment], key: &str) -> Vec<AuthoredSegment> {
    let mut path = at.to_vec();
    path.push(AuthoredSegment::Key(key.to_string()));
    path
}

fn child_index(at: &[AuthoredSegment], index: usize) -> Vec<AuthoredSegment> {
    let mut path = at.to_vec();
    path.push(AuthoredSegment::Index(index));
    path
}
