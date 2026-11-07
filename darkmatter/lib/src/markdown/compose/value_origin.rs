//! Whether a frontmatter value is an authored instruction or data.
//!
//! Darkmatter scans authored text for `{{ … }}`, `{{{ … }}}`, and whole-value
//! `$( … )` **once**. Everything an operation produces is data and is never
//! scanned again: an expression result, a file read, shell output, a literal
//! escape's output, a caller's data override, and the composed values a parent
//! hands to a transcluded child. Origin travels beside the value, never inside
//! it, so no in-band escaping is needed and no string is judged by how it looks.
//!
//! Body text carries the same distinction as byte ranges
//! ([`DataRanges`](super::body_origin::DataRanges)).

use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

/// Where a caller-supplied override value came from.
///
/// [`Authored`](Self::Authored) values are templates, exactly like text written
/// in the document: a person typed them (`md compose --set`, `key=value`).
/// [`Data`](Self::Data) values are inert: Darkmatter never evaluates their
/// `{{ … }}`, converts their `{{{ … }}}`, or runs their `$( … )`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OverrideOrigin {
    /// A person wrote the value; it is scanned once like authored frontmatter.
    #[default]
    Authored,
    /// An operation produced the value; it is never scanned.
    Data,
}

/// One layer of top-level frontmatter overrides with a single origin.
///
/// Layers are applied in order by
/// [`ComposeOptions::with_override_layers`](super::ComposeOptions::with_override_layers):
/// a later layer's key replaces an earlier layer's key, and the key takes the
/// origin of the layer that supplied it.
#[derive(Debug, Clone, PartialEq)]
pub struct OverrideLayer {
    /// Origin of every value in [`values`](Self::values).
    pub origin: OverrideOrigin,
    /// A JSON object of top-level frontmatter keys. Any other JSON value
    /// contributes no keys.
    pub values: Value,
}

impl OverrideLayer {
    /// A layer of templates a person wrote.
    #[must_use]
    pub fn authored(values: Value) -> Self {
        Self {
            origin: OverrideOrigin::Authored,
            values,
        }
    }

    /// A layer of inert values an operation produced.
    #[must_use]
    pub fn data(values: Value) -> Self {
        Self {
            origin: OverrideOrigin::Data,
            values,
        }
    }
}

/// One step of a path from the frontmatter root to a value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ValuePathSegment {
    Key(String),
    Index(usize),
}

/// The frontmatter values that are data, as paths from the root.
///
/// A path marks its whole subtree: `[replace]` makes every leaf under
/// `replace` data. Paths not covered by any entry are authored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct DataPaths {
    paths: BTreeSet<Vec<ValuePathSegment>>,
}

impl DataPaths {
    /// The marked paths, in sorted order.
    pub(crate) fn paths(&self) -> impl ExactSizeIterator<Item = &Vec<ValuePathSegment>> {
        self.paths.iter()
    }

    /// Marks `path` and everything below it as data.
    pub(crate) fn mark(&mut self, path: Vec<ValuePathSegment>) {
        if self.is_data(&path) {
            return;
        }
        self.paths.retain(|existing| !existing.starts_with(&path));
        self.paths.insert(path);
    }

    /// Marks the whole value of top-level `key` as data.
    pub(crate) fn mark_key(&mut self, key: &str) {
        self.mark(vec![ValuePathSegment::Key(key.to_string())]);
    }

    /// Marks each leaf `overlay` supplies below `prefix` as data.
    ///
    /// A non-empty object recurses, because a deep merge keeps the siblings it
    /// does not name; anything else (a scalar, an array, an empty object)
    /// replaces the value at its path wholesale.
    pub(crate) fn mark_leaves(&mut self, prefix: &mut Vec<ValuePathSegment>, overlay: &Value) {
        match overlay {
            Value::Object(map) if !map.is_empty() => {
                for (key, value) in map {
                    prefix.push(ValuePathSegment::Key(key.clone()));
                    self.mark_leaves(prefix, value);
                    prefix.pop();
                }
            }
            _ => self.mark(prefix.clone()),
        }
    }

    /// Makes top-level `key` authored again, as when an authored override
    /// replaces it.
    pub(crate) fn clear_key(&mut self, key: &str) {
        self.paths
            .retain(|path| path.first() != Some(&ValuePathSegment::Key(key.to_string())));
    }

    /// Makes the value at `path` authored again, keeping every other value's
    /// origin.
    ///
    /// A mark on an ancestor covers `path` too, so it is split into marks on the
    /// values beside the path; `root` is the frontmatter that names them.
    pub(crate) fn clear_path(&mut self, path: &[ValuePathSegment], root: &Value) {
        self.paths.retain(|existing| !existing.starts_with(path));
        let Some(ancestor) = self.paths.iter().find(|marked| path.starts_with(marked)).cloned() else {
            return;
        };
        self.paths.remove(&ancestor);
        let mut prefix = Vec::with_capacity(path.len());
        let mut current = Some(root);
        for step in path {
            let on_route = prefix.len() >= ancestor.len();
            if on_route {
                for sibling in children_of(current).filter(|sibling| sibling != step) {
                    let mut marked = prefix.clone();
                    marked.push(sibling);
                    self.paths.insert(marked);
                }
            }
            current = match step {
                ValuePathSegment::Key(key) => current.and_then(|value| value.get(key)),
                ValuePathSegment::Index(index) => current.and_then(|value| value.get(index)),
            };
            prefix.push(step.clone());
        }
    }

    /// Whether the value at `path` is data.
    pub(crate) fn is_data(&self, path: &[ValuePathSegment]) -> bool {
        self.paths.iter().any(|marked| path.starts_with(marked))
    }

    /// Whether top-level `key` is wholly data.
    pub(crate) fn is_data_key(&self, key: &str) -> bool {
        self.is_data(&[ValuePathSegment::Key(key.to_string())])
    }

    /// `value` at `path` with every data leaf replaced by `null`, so a caller
    /// that classifies or analyzes templates sees authored text only.
    pub(crate) fn authored_view(&self, path: &mut Vec<ValuePathSegment>, value: &Value) -> Value {
        if self.is_data(path) {
            return Value::Null;
        }
        match value {
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        path.push(ValuePathSegment::Index(index));
                        let view = self.authored_view(path, item);
                        path.pop();
                        view
                    })
                    .collect(),
            ),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, item)| {
                        path.push(ValuePathSegment::Key(key.clone()));
                        let view = self.authored_view(path, item);
                        path.pop();
                        (key.clone(), view)
                    })
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

fn children_of(value: Option<&Value>) -> impl Iterator<Item = ValuePathSegment> + '_ {
    let keys = value.and_then(Value::as_object).into_iter().flatten().map(|(key, _)| ValuePathSegment::Key(key.clone()));
    let indexes = value.and_then(Value::as_array).into_iter().flatten().enumerate().map(|(index, _)| ValuePathSegment::Index(index));
    keys.chain(indexes)
}

/// Origin of one document's frontmatter during composition.
///
/// Captured once, after caller overrides and inherited state are merged and
/// before any frontmatter scan.
#[derive(Debug, Clone, Default)]
pub(crate) struct FrontmatterProvenance {
    /// Each authored top-level string exactly as written, before any scan.
    /// Data keys are absent.
    authored: HashMap<String, String>,
    /// Leaves that are data. Grows as scans produce values.
    data: DataPaths,
}

impl FrontmatterProvenance {
    pub(crate) fn new(authored: HashMap<String, String>, data: DataPaths) -> Self {
        Self { authored, data }
    }

    /// The provenance of a frontmatter whose every value is authored.
    pub(crate) fn all_authored(frontmatter: &crate::markdown::frontmatter::Frontmatter) -> Self {
        let authored = frontmatter
            .as_map()
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|s| (key.clone(), s.to_string())))
            .collect();
        Self::new(authored, DataPaths::default())
    }

    /// The authored text of top-level `key` before any scan, when `key` is an
    /// authored string.
    pub(crate) fn authored_source(&self, key: &str) -> Option<&str> {
        self.authored.get(key).map(String::as_str)
    }

    /// Records `value` as the authored text of top-level `key`, which a person
    /// has just supplied; a non-string value has no authored text.
    pub(crate) fn set_authored_source(&mut self, key: &str, value: Option<&Value>) {
        match value.and_then(Value::as_str) {
            Some(text) => self.authored.insert(key.to_string(), text.to_string()),
            None => self.authored.remove(key),
        };
    }

    /// Whether `key`'s authored value is a whole-value `$( … )` shell
    /// candidate. See [`authored_shell_candidate`].
    pub(crate) fn is_authored_shell_candidate(&self, key: &str) -> bool {
        self.authored_source(key).is_some_and(authored_shell_candidate)
    }

    pub(crate) fn data(&self) -> &DataPaths {
        &self.data
    }

    pub(crate) fn data_mut(&mut self) -> &mut DataPaths {
        &mut self.data
    }
}

/// Whether an **authored** frontmatter string is a whole-value `$( … )` shell
/// candidate.
///
/// The one shell-shape predicate shared by the runtime scan, preflight
/// collection, and the interpolation deferral mark (R1.2). It is only ever
/// applied to the authored source value, so interpolation can supply a
/// command's arguments but never its shape, and shell output that looks like
/// `$( … )` stays data.
pub(crate) fn authored_shell_candidate(source: &str) -> bool {
    source.trim_start().starts_with("$(")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn key(k: &str) -> ValuePathSegment {
        ValuePathSegment::Key(k.to_string())
    }

    #[test]
    fn a_marked_path_covers_its_subtree_only() {
        let mut data = DataPaths::default();
        data.mark(vec![key("a"), key("b")]);
        assert!(data.is_data(&[key("a"), key("b")]));
        assert!(data.is_data(&[key("a"), key("b"), ValuePathSegment::Index(0)]));
        assert!(!data.is_data(&[key("a")]));
        assert!(!data.is_data(&[key("a"), key("c")]));
    }

    #[test]
    fn overlay_leaves_mark_only_what_the_overlay_names() {
        let mut data = DataPaths::default();
        data.mark_leaves(&mut Vec::new(), &json!({"a": {"x": "{{ y }}"}, "b": [1]}));
        assert!(data.is_data(&[key("a"), key("x")]));
        assert!(!data.is_data(&[key("a"), key("sibling")]));
        assert!(data.is_data_key("b"));
    }

    #[test]
    fn clearing_a_key_makes_it_authored_again() {
        let mut data = DataPaths::default();
        data.mark_key("a");
        data.mark(vec![key("b"), key("c")]);
        data.clear_key("a");
        assert!(!data.is_data_key("a"));
        assert!(data.is_data(&[key("b"), key("c")]));
    }

    #[test]
    fn clearing_a_path_keeps_the_values_beside_it() {
        let mut data = DataPaths::default();
        data.mark_key("a");
        let root = json!({"a": {"b": {"c": 1, "d": 2}, "e": 3, "items": [4, 5]}});
        data.clear_path(&[key("a"), key("b"), key("c")], &root);
        assert!(!data.is_data(&[key("a"), key("b"), key("c")]));
        assert!(data.is_data(&[key("a"), key("b"), key("d")]));
        assert!(data.is_data(&[key("a"), key("e")]));
        assert!(data.is_data(&[key("a"), key("items"), ValuePathSegment::Index(1)]));
        assert!(!data.is_data(&[key("a")]));

        let mut data = DataPaths::default();
        data.mark_key("items");
        data.clear_path(&[key("items"), ValuePathSegment::Index(1)], &json!({"items": [4, 5, 6]}));
        assert!(data.is_data(&[key("items"), ValuePathSegment::Index(0)]));
        assert!(!data.is_data(&[key("items"), ValuePathSegment::Index(1)]));
        assert!(data.is_data(&[key("items"), ValuePathSegment::Index(2)]));
    }

    #[test]
    fn clearing_a_path_drops_marks_below_it_and_leaves_others() {
        let mut data = DataPaths::default();
        data.mark(vec![key("a"), key("b"), key("c")]);
        data.mark(vec![key("z")]);
        data.clear_path(&[key("a"), key("b")], &json!({"a": {"b": {"c": 1}}, "z": 2}));
        assert!(!data.is_data(&[key("a"), key("b"), key("c")]));
        assert!(data.is_data_key("z"));
    }

    #[test]
    fn the_authored_view_nulls_data_leaves() {
        let mut data = DataPaths::default();
        data.mark(vec![key("a"), key("x")]);
        let view = data.authored_view(
            &mut vec![key("a")],
            &json!({"x": "{{ data }}", "y": "{{ authored }}"}),
        );
        assert_eq!(view, json!({"x": null, "y": "{{ authored }}"}));
    }

    #[test]
    fn the_shell_candidate_is_decided_on_the_trimmed_start() {
        assert!(authored_shell_candidate("$(echo x)"));
        assert!(authored_shell_candidate("  $(echo x)"));
        assert!(!authored_shell_candidate("see $(echo x)"));
        assert!(!authored_shell_candidate("{{ cmd }}"));
    }
}
