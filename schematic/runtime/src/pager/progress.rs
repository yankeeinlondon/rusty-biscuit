//! Progress checks: the history that catches repeated exclusive
//! continuations, and the tracker that filters re-delivered records at an
//! inclusive cursor boundary.

use std::collections::HashSet;

use serde_json::Value;

use super::json::lookup;
use super::spec::{Boundary, CursorLeg, CursorSource};

/// Every continuation one direction of one traversal has sent, kept exactly
/// (not hashed), so any repeat, however far back, is a loop.
#[derive(Debug, Default)]
pub struct History {
    seen: HashSet<Vec<String>>,
}

impl History {
    /// Records `key`; `false` when it was already recorded.
    pub fn insert(&mut self, key: &[String]) -> bool {
        self.seen.insert(key.to_vec())
    }

    /// Forgets every key: the caller turned around, so a later run in this
    /// direction may revisit pages on purpose.
    pub fn clear(&mut self) {
        self.seen.clear();
    }
}

/// Why a page made no progress at an inclusive boundary, or could not be
/// checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryError {
    /// A record lacks a value of the cursor key.
    MissingKey,
    /// Every record of a non-empty page was already delivered at the key.
    NoProgress,
}

/// Drops records an inclusive boundary re-delivers.
///
/// The key of a record is its values at the leg's item-derived cursor
/// parts. Records at the last page's final key whose identity was already
/// delivered are dropped; a non-empty page made only of such records is no
/// progress.
#[derive(Debug)]
pub struct BoundaryTracker {
    key_paths: Vec<&'static [&'static str]>,
    identity: &'static [&'static str],
    last_key: Option<Vec<Value>>,
    delivered_at_key: Vec<Value>,
}

impl BoundaryTracker {
    /// A tracker for `leg`, or `None` when its boundary is exclusive or a
    /// part is not read from an item.
    pub fn new(leg: &CursorLeg) -> Option<Self> {
        let Boundary::Inclusive { identity } = leg.boundary else {
            return None;
        };
        let key_paths = leg
            .parts
            .iter()
            .map(|part| match part.source {
                CursorSource::LastItem(path) | CursorSource::FirstItem(path) => Some(path),
                CursorSource::Field(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            key_paths,
            identity,
            last_key: None,
            delivered_at_key: Vec::new(),
        })
    }

    fn key_of(&self, record: &Value) -> Result<Vec<Value>, BoundaryError> {
        self.key_paths
            .iter()
            .map(|path| lookup(record, path).cloned().ok_or(BoundaryError::MissingKey))
            .collect()
    }

    /// The indexes of `records` to deliver.
    ///
    /// ## Errors
    ///
    /// Returns [`BoundaryError`] when a record has no key, or when a
    /// non-empty page delivers nothing new.
    pub fn accept(&mut self, records: &[&Value]) -> Result<Vec<usize>, BoundaryError> {
        let mut kept = Vec::new();
        for (index, record) in records.iter().enumerate() {
            let key = self.key_of(record)?;
            let identity = lookup(record, self.identity).cloned().unwrap_or(Value::Null);
            let duplicate = self.last_key.as_ref() == Some(&key) && self.delivered_at_key.contains(&identity);
            if !duplicate {
                kept.push(index);
            }
        }
        if !records.is_empty() && kept.is_empty() {
            return Err(BoundaryError::NoProgress);
        }
        let Some(last) = records.last() else {
            return Ok(kept);
        };
        let new_key = self.key_of(last)?;
        if self.last_key.as_ref() != Some(&new_key) {
            self.delivered_at_key.clear();
        }
        for record in records {
            if self.key_of(record)? == new_key
                && let Some(identity) = lookup(record, self.identity)
                && !self.delivered_at_key.contains(identity)
            {
                self.delivered_at_key.push(identity.clone());
            }
        }
        self.last_key = Some(new_key);
        Ok(kept)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from the pagination prototype: g2.

    use serde_json::json;

    use super::*;
    use crate::pager::spec::{CursorPart, Input};

    const SINCE: CursorLeg = CursorLeg {
        parts: &[CursorPart {
            param: Input::Query("since"),
            source: CursorSource::LastItem(&["t"]),
        }],
        boundary: Boundary::Inclusive { identity: &["id"] },
    };

    fn page(value: &Value) -> Vec<&Value> {
        value.as_array().expect("array").iter().collect()
    }

    #[test]
    fn the_tracker_drops_redelivered_ties_and_detects_no_progress() {
        let mut tracker = BoundaryTracker::new(&SINCE).expect("inclusive item-keyed leg");

        let first = json!([{ "id": 1, "t": 10 }, { "id": 2, "t": 20 }, { "id": 3, "t": 20 }]);
        assert_eq!(tracker.accept(&page(&first)), Ok(vec![0, 1, 2]));

        let second = json!([{ "id": 2, "t": 20 }, { "id": 3, "t": 20 }, { "id": 4, "t": 20 }, { "id": 5, "t": 30 }]);
        assert_eq!(tracker.accept(&page(&second)), Ok(vec![2, 3]));

        let only_ties = json!([{ "id": 5, "t": 30 }]);
        assert_eq!(tracker.accept(&page(&only_ties)), Err(BoundaryError::NoProgress));
    }

    #[test]
    fn a_new_identity_at_the_same_key_is_progress() {
        let mut tracker = BoundaryTracker::new(&SINCE).expect("tracker");
        assert_eq!(tracker.accept(&page(&json!([{ "id": 1, "t": 5 }]))), Ok(vec![0]));
        assert_eq!(tracker.accept(&page(&json!([{ "id": 1, "t": 5 }, { "id": 2, "t": 5 }]))), Ok(vec![1]));
        assert_eq!(tracker.accept(&page(&json!([]))), Ok(vec![]), "an empty page is not a stall");
    }

    #[test]
    fn a_record_without_its_key_cannot_be_checked() {
        let mut tracker = BoundaryTracker::new(&SINCE).expect("tracker");
        assert_eq!(tracker.accept(&page(&json!([{ "id": 1 }]))), Err(BoundaryError::MissingKey));
    }

    #[test]
    fn only_inclusive_item_keyed_legs_are_tracked() {
        let exclusive = CursorLeg {
            boundary: Boundary::Exclusive,
            ..SINCE
        };
        assert!(BoundaryTracker::new(&exclusive).is_none());
        let field = CursorLeg {
            parts: &[CursorPart {
                param: Input::Query("since"),
                source: CursorSource::Field(crate::pager::spec::Source::Body(&["next"])),
            }],
            ..SINCE
        };
        assert!(BoundaryTracker::new(&field).is_none());
    }

    #[test]
    fn the_history_keeps_every_key_it_saw() {
        let mut history = History::default();
        assert!(history.insert(&["a".to_owned()]));
        assert!(history.insert(&["b".to_owned()]));
        assert!(!history.insert(&["a".to_owned()]), "a repeat far back is still a repeat");
        assert!(history.insert(&["a".to_owned(), "b".to_owned()]), "every part counts");
    }
}
