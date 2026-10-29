//! The runtime side of a collection source: one captured array or object,
//! walked in original order with a borrowed view of each entry.
//!
//! A source is captured once. Its entries are then viewed in place, so
//! visiting, filtering, and sorting copy nothing: a [`SourceEntry`] is a
//! `Copy` view holding the entry's original position, and a selection (or a
//! sort) stores only those views plus, for a sort, one key per selected
//! entry. No list of rejected entries exists. A value is cloned only when a
//! later body frame needs to retain it ([`SourceEntry::retained`]).
//!
//! Declared types for the same entries come from
//! [`scope::project_source`](super::scope::project_source).

use std::borrow::Cow;

use serde_json::{Map, Value};

use super::iteration::IterationPosition;

/// A non-null scalar reached where a collection was required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotIterable {
    kind: &'static str,
}

impl NotIterable {
    /// The kind of value found: `string`, `number`, or `boolean`.
    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

impl std::fmt::Display for NotIterable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a {} is not an array or object, so it cannot be iterated", self.kind)
    }
}

impl std::error::Error for NotIterable {}

#[derive(Debug, Clone, PartialEq)]
enum Captured {
    Empty,
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

/// One collection source, owned for the duration of its iteration.
///
/// `null` is an empty source, so a missing source renders nothing. Tuples are
/// JSON arrays at runtime, so they iterate as arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct IterableSource {
    captured: Captured,
}

impl IterableSource {
    /// Takes ownership of `value` as the source.
    ///
    /// ## Errors
    ///
    /// [`NotIterable`] for a string, number, or boolean.
    pub fn capture(value: Value) -> Result<Self, NotIterable> {
        let captured = match value {
            Value::Null => Captured::Empty,
            Value::Array(items) => Captured::Array(items),
            Value::Object(entries) => Captured::Object(entries),
            Value::String(_) => return Err(NotIterable { kind: "string" }),
            Value::Number(_) => return Err(NotIterable { kind: "number" }),
            Value::Bool(_) => return Err(NotIterable { kind: "boolean" }),
        };
        Ok(Self { captured })
    }

    /// [`Self::capture`] of a copy of `value`, for a source still owned
    /// elsewhere. The copy is the one capture; entries are never copied again.
    ///
    /// ## Errors
    ///
    /// [`NotIterable`] for a string, number, or boolean.
    pub fn capture_ref(value: &Value) -> Result<Self, NotIterable> {
        Self::capture(value.clone())
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        match &self.captured {
            Captured::Empty => 0,
            Captured::Array(items) => items.len(),
            Captured::Object(entries) => entries.len(),
        }
    }

    /// Whether the source has no entry.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the entries are keyed object entries rather than elements.
    pub fn is_keyed(&self) -> bool {
        matches!(self.captured, Captured::Object(_))
    }

    /// The entries in original order, each viewed in place.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = SourceEntry<'_>> + '_ {
        let (array, object) = match &self.captured {
            Captured::Empty => (None, None),
            Captured::Array(items) => (Some(items.iter()), None),
            Captured::Object(entries) => (None, Some(entries.iter())),
        };
        let len = self.len();
        let elements = array.into_iter().flatten().enumerate().map(|(index, value)| SourceEntry {
            index,
            key: None,
            value,
        });
        let keyed = object.into_iter().flatten().enumerate().map(|(index, (key, value))| SourceEntry {
            index,
            key: Some(key.as_str()),
            value,
        });
        Entries { inner: elements.chain(keyed), remaining: len }
    }
}

struct Entries<I> {
    inner: I,
    remaining: usize,
}

impl<I: Iterator> Iterator for Entries<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.next();
        if next.is_some() {
            self.remaining -= 1;
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<I: Iterator> ExactSizeIterator for Entries<I> {}

/// A borrowed view of one entry of an [`IterableSource`].
///
/// For an array it is the element; for an object it is the `{key, value}`
/// entry, which is built only when asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceEntry<'a> {
    index: usize,
    key: Option<&'a str>,
    value: &'a Value,
}

impl<'a> SourceEntry<'a> {
    /// The zero-based position in the original source, whatever filtering or
    /// sorting later does.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The original key of an object entry; `None` for an element.
    pub fn key(&self) -> Option<&'a str> {
        self.key
    }

    /// The element, or an object entry's value.
    pub fn value(&self) -> &'a Value {
        self.value
    }

    /// The original position as the budget and diagnostics name it: the key
    /// for an object entry, the index for an element.
    pub fn position(&self) -> IterationPosition {
        match self.key {
            Some(key) => IterationPosition::Key(key.to_string()),
            None => IterationPosition::Index(self.index as u64),
        }
    }

    /// The field `name` of the item a callback receives, without building it.
    /// An object entry has `key` and `value` (present even when the value is
    /// `null`); an element is read as an object. `None` when there is no such
    /// field, which is distinct from a field that holds `null`.
    pub fn field(&self, name: &str) -> Option<Cow<'a, Value>> {
        match (self.key, name) {
            (Some(key), "key") => Some(Cow::Owned(Value::String(key.to_string()))),
            (Some(_), "value") => Some(Cow::Borrowed(self.value)),
            (Some(_), _) => None,
            (None, name) => self.value.as_object()?.get(name).map(Cow::Borrowed),
        }
    }

    /// The item as a value: the element borrowed, or the `{key, value}` object
    /// built and owning a copy of the value.
    pub fn item(&self) -> Cow<'a, Value> {
        match self.key {
            None => Cow::Borrowed(self.value),
            Some(key) => {
                let mut entry = Map::new();
                entry.insert("key".to_string(), Value::String(key.to_string()));
                entry.insert("value".to_string(), self.value.clone());
                Cow::Owned(Value::Object(entry))
            }
        }
    }

    /// An owned copy of the item, for a body frame that outlives the source
    /// borrow. Take it only when a later frame needs it.
    pub fn retained(&self) -> Value {
        self.item().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn an_array_yields_elements_with_their_original_indexes() {
        let source = IterableSource::capture(json!(["a", null, "c"])).unwrap();
        let entries: Vec<_> = source.entries().collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(source.entries().len(), 3);
        assert!(!source.is_keyed());
        assert_eq!(entries[1].index(), 1);
        assert_eq!(entries[1].value(), &Value::Null, "a null element is preserved");
        assert_eq!(entries[1].key(), None);
        assert_eq!(entries[2].position(), IterationPosition::Index(2));
    }

    #[test]
    fn an_object_yields_key_value_entries_in_original_order() {
        let source = IterableSource::capture(json!({"b": 1, "a": null})).unwrap();
        assert!(source.is_keyed());
        let entries: Vec<_> = source.entries().collect();
        assert_eq!(entries.len(), 2);
        let first = entries[0];
        assert_eq!(first.position(), IterationPosition::Key(first.key().unwrap().to_string()));
        let null_entry = entries.iter().find(|entry| entry.key() == Some("a")).unwrap();
        assert_eq!(null_entry.field("key").unwrap().as_ref(), &json!("a"));
        assert_eq!(null_entry.field("value").unwrap().as_ref(), &Value::Null, "value exists even when null");
        assert!(null_entry.field("other").is_none());
        assert_eq!(null_entry.retained(), json!({"key": "a", "value": null}));
    }

    #[test]
    fn an_element_is_viewed_in_place_and_fields_read_through_it() {
        let source = IterableSource::capture_ref(&json!([{"name": "x", "gone": null}, 3])).unwrap();
        let entries: Vec<_> = source.entries().collect();
        assert!(matches!(entries[0].item(), Cow::Borrowed(_)));
        assert!(matches!(entries[0].field("name"), Some(Cow::Borrowed(_))));
        assert_eq!(entries[0].field("gone").unwrap().as_ref(), &Value::Null, "present null is not missing");
        assert!(entries[0].field("absent").is_none());
        assert!(entries[1].field("name").is_none(), "a scalar element has no fields");
    }

    #[test]
    fn null_is_an_empty_source_and_a_scalar_is_rejected() {
        let empty = IterableSource::capture(Value::Null).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.entries().count(), 0);
        for (value, kind) in [(json!("s"), "string"), (json!(1), "number"), (json!(true), "boolean")] {
            let error = IterableSource::capture(value).unwrap_err();
            assert_eq!(error.kind(), kind);
            assert!(error.to_string().contains(kind));
        }
    }

    #[test]
    fn a_selection_stores_views_not_copies() {
        let source = IterableSource::capture(json!([1, 2, 3, 4])).unwrap();
        let selected: Vec<SourceEntry<'_>> = source.entries().filter(|entry| entry.index() % 2 == 1).collect();
        assert_eq!(selected.iter().map(|entry| entry.index()).collect::<Vec<_>>(), [1, 3]);
        assert!(std::ptr::eq(selected[0].value(), source.entries().nth(1).unwrap().value()));
    }
}
