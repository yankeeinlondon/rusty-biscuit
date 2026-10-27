//! A top-level frontmatter mapping whose bulk is shared and whose writes are
//! private.
//!
//! [`SharedFrontmatter`] is the document state one evaluation reads: a base
//! mapping held behind an [`Arc`] plus an owned overlay of the top-level keys
//! this view wrote or removed. Cloning a view copies only that overlay, so two
//! views derived from one seed read the same authored values (a large inline
//! `sequence:` list included) while neither can observe the other's writes.

use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A frontmatter mapping: a shared, immutable base under a private overlay.
///
/// A write replaces the whole top-level value in the overlay, as
/// [`Map::insert`] does; a removal hides the base key; an explicit `null` is
/// a present value, never a removal. Reads see the overlay first. Writing one
/// key never copies another, and [`Clone`] shares the base.
///
/// Iteration yields the base's keys in base order, a written value in its base
/// key's place, then keys the base never had in the order they were first
/// written. A base key removed and written again keeps its base position.
#[derive(Debug, Clone, Default)]
pub struct SharedFrontmatter {
    base: Arc<Map<String, Value>>,
    overlay: Map<String, Value>,
    /// Base keys this view removed. Never holds a key that is in `overlay`.
    removed: HashSet<String>,
}

impl SharedFrontmatter {
    /// A view over `base` with nothing written yet.
    pub fn new(base: Arc<Map<String, Value>>) -> Self {
        Self {
            base,
            overlay: Map::new(),
            removed: HashSet::new(),
        }
    }

    /// The value of a top-level key, overlay first.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self.overlay.get(key) {
            Some(value) => Some(value),
            None if self.removed.contains(key) => None,
            None => self.base.get(key),
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Writes a whole top-level value, shadowing any base value of that key.
    pub fn insert(&mut self, key: String, value: Value) {
        self.removed.remove(&key);
        self.overlay.insert(key, value);
    }

    /// Removes a top-level key; returns whether it was present.
    pub fn remove(&mut self, key: &str) -> bool {
        let written = self.overlay.shift_remove(key).is_some();
        let in_base = self.base.contains_key(key);
        if in_base {
            let newly_hidden = self.removed.insert(key.to_string());
            written || newly_hidden
        } else {
            written
        }
    }

    /// A mutable borrow of a top-level value.
    ///
    /// A base value is first copied into the overlay; only that one value is
    /// copied.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        if !self.overlay.contains_key(key) {
            let inherited = self.get(key)?.clone();
            self.overlay.insert(key.to_string(), inherited);
        }
        self.overlay.get_mut(key)
    }

    /// A mutable borrow of a top-level value, written from `default` when the
    /// key is absent.
    pub fn get_or_insert_with(&mut self, key: &str, default: impl FnOnce() -> Value) -> &mut Value {
        if !self.contains_key(key) {
            self.insert(key.to_string(), default());
        }
        self.get_mut(key).expect("the key was just made present")
    }

    /// The number of present top-level keys.
    pub fn len(&self) -> usize {
        let base_present = self.base.len() - self.removed.len();
        let added = self.overlay.keys().filter(|key| !self.base.contains_key(key.as_str())).count();
        base_present + added
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The present entries, in the order described on [`SharedFrontmatter`].
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        let base = self
            .base
            .iter()
            .filter(|(key, _)| !self.removed.contains(key.as_str()))
            .map(|(key, value)| (key, self.overlay.get(key).unwrap_or(value)));
        let added = self.overlay.iter().filter(|(key, _)| !self.base.contains_key(key.as_str()));
        base.chain(added)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.iter().map(|(key, _)| key)
    }

    /// Copies the present entries into an owned mapping.
    ///
    /// This deep-copies every value, the shared base included; call it only
    /// where a caller needs an owned whole document.
    pub fn to_map(&self) -> Map<String, Value> {
        self.iter().map(|(key, value)| (key.clone(), value.clone())).collect()
    }

    /// [`Self::to_map`], into a hash map.
    pub fn to_hash_map(&self) -> HashMap<String, Value> {
        self.iter().map(|(key, value)| (key.clone(), value.clone())).collect()
    }

    /// Whether `self` and `other` read the same shared base allocation.
    pub fn shares_base_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.base, &other.base)
    }

    /// The number of top-level values this view owns: written or copied for
    /// mutation, not inherited from the base.
    pub fn owned_len(&self) -> usize {
        self.overlay.len()
    }

    /// The top-level values this view owns (see [`Self::owned_len`]): what
    /// [`Clone`] copies.
    pub fn owned(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.overlay.iter()
    }
}

impl From<Map<String, Value>> for SharedFrontmatter {
    fn from(base: Map<String, Value>) -> Self {
        Self::new(Arc::new(base))
    }
}

impl From<HashMap<String, Value>> for SharedFrontmatter {
    fn from(base: HashMap<String, Value>) -> Self {
        base.into_iter().collect()
    }
}

impl FromIterator<(String, Value)> for SharedFrontmatter {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(entries: I) -> Self {
        Self::from(entries.into_iter().collect::<Map<String, Value>>())
    }
}

impl Extend<(String, Value)> for SharedFrontmatter {
    fn extend<I: IntoIterator<Item = (String, Value)>>(&mut self, entries: I) {
        for (key, value) in entries {
            self.insert(key, value);
        }
    }
}

/// Equal when both hold the same present entries, whatever their order or
/// how they are split between base and overlay.
impl PartialEq for SharedFrontmatter {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(key, value)| other.get(key) == Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seed() -> SharedFrontmatter {
        let base = json!({
            "title": "Release",
            "nested": { "a": 1, "b": { "c": 2 } },
            "sequence": [ { "id": "one", "shell": "echo 1" }, { "id": "two", "shell": "echo 2" } ],
            "cleared": "set"
        });
        let Value::Object(map) = base else { unreachable!() };
        SharedFrontmatter::from(map)
    }

    #[test]
    fn a_clone_reads_the_seed_without_owning_any_value() {
        let seed = seed();
        let view = seed.clone();

        assert!(view.shares_base_with(&seed));
        assert_eq!(view.owned_len(), 0);
        assert_eq!(view.get("title"), Some(&json!("Release")));
        assert_eq!(view.get("sequence").and_then(Value::as_array).map(Vec::len), Some(2));
        assert_eq!(view, seed);
    }

    #[test]
    fn a_small_write_owns_only_that_key_and_keeps_the_sequence_shared() {
        let seed = seed();
        let mut view = seed.clone();

        view.insert("status".to_string(), json!("running"));

        assert_eq!(view.owned_len(), 1);
        assert!(view.shares_base_with(&seed));
        assert_eq!(view.get("status"), Some(&json!("running")));
        assert_eq!(view.get("sequence"), seed.get("sequence"));
        assert_eq!(seed.get("status"), None);
    }

    #[test]
    fn sibling_views_never_observe_each_others_writes() {
        let seed = seed();
        let mut left = seed.clone();
        let mut right = seed.clone();

        left.insert("title".to_string(), json!("left"));
        right.remove("title");
        right.insert("only_right".to_string(), json!(true));

        assert_eq!(left.get("title"), Some(&json!("left")));
        assert_eq!(left.get("only_right"), None);
        assert_eq!(right.get("title"), None);
        assert_eq!(seed.get("title"), Some(&json!("Release")));
        assert!(left.shares_base_with(&right));
    }

    #[test]
    fn an_explicit_null_is_present_and_shadows_the_base_value() {
        let mut view = seed();

        view.insert("cleared".to_string(), Value::Null);

        assert!(view.contains_key("cleared"));
        assert_eq!(view.get("cleared"), Some(&Value::Null));
        assert_eq!(view.to_map().get("cleared"), Some(&Value::Null));
        assert_eq!(view.len(), seed().len());
    }

    #[test]
    fn a_write_replaces_a_whole_nested_value_rather_than_merging() {
        let mut view = seed();

        view.insert("nested".to_string(), json!({ "b": { "d": 3 } }));

        assert_eq!(view.get("nested"), Some(&json!({ "b": { "d": 3 } })));
    }

    #[test]
    fn a_removal_hides_the_base_key_and_a_rewrite_restores_its_place() {
        let mut view = seed();

        assert!(view.remove("title"));
        assert!(!view.remove("title"));
        assert!(!view.contains_key("title"));
        assert_eq!(view.len(), 3);
        assert!(!view.keys().any(|key| key == "title"));

        view.insert("title".to_string(), json!("again"));
        assert_eq!(view.keys().next().map(String::as_str), Some("title"));
        assert_eq!(view.get("title"), Some(&json!("again")));
        assert_eq!(view.len(), 4);
    }

    #[test]
    fn removing_a_key_only_the_overlay_held_forgets_it() {
        let mut view = seed();
        view.insert("added".to_string(), json!(1));

        assert!(view.remove("added"));
        assert!(!view.contains_key("added"));
        assert!(!view.remove("missing"));
        assert_eq!(view.len(), 4);
    }

    #[test]
    fn get_mut_copies_only_the_mutated_value() {
        let seed = seed();
        let mut view = seed.clone();

        if let Some(Value::Object(nested)) = view.get_mut("nested") {
            nested.insert("a".to_string(), json!(10));
        }

        assert_eq!(view.owned_len(), 1);
        assert_eq!(view.get("nested").and_then(|n| n.get("a")), Some(&json!(10)));
        assert_eq!(seed.get("nested").and_then(|n| n.get("a")), Some(&json!(1)));
        assert_eq!(view.get_mut("missing"), None);
        assert_eq!(view.owned_len(), 1);
    }

    #[test]
    fn get_or_insert_with_writes_only_an_absent_key() {
        let mut view = seed();

        if let Value::Array(items) = view.get_or_insert_with("log", || json!([])) {
            items.push(json!("first"));
        }
        let existing = view.get_or_insert_with("title", || json!("unused")).clone();

        assert_eq!(view.get("log"), Some(&json!(["first"])));
        assert_eq!(existing, json!("Release"));
    }

    #[test]
    fn iteration_keeps_base_order_then_appends_new_keys() {
        let mut view = seed();
        view.insert("zeta".to_string(), json!(1));
        view.insert("alpha".to_string(), json!(2));
        view.insert("nested".to_string(), json!("replaced"));

        let keys: Vec<&str> = view.keys().map(String::as_str).collect();
        assert_eq!(keys, ["title", "nested", "sequence", "cleared", "zeta", "alpha"]);
        assert_eq!(view.to_map().get("nested"), Some(&json!("replaced")));
        assert_eq!(view.to_hash_map().len(), 6);
    }

    #[test]
    fn equality_ignores_how_entries_are_layered() {
        let mut layered = seed();
        layered.insert("extra".to_string(), json!(1));
        let flat = SharedFrontmatter::from(layered.to_map());

        assert_eq!(layered, flat);
        assert!(!layered.shares_base_with(&flat));
        layered.remove("extra");
        assert_ne!(layered, flat);
    }
}
