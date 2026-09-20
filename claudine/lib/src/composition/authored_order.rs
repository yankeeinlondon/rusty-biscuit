//! A moving JSON-Pointer cursor over one document's authored mapping order.
//!
//! Authored key order is recorded per document by Darkmatter as a
//! [`MappingOrders`] index keyed by JSON Pointer. Claudine reaches the mappings
//! that need it — a lifecycle `set:` — by walking *into* a document: a sequence
//! step, a group's member task, a stack item's action. Rebuilding the full
//! pointer at each of those sites is how the order channel came to cover only
//! the one shape that had a hard-coded pointer, so the walk carries this cursor
//! instead: the order source and the current position travel together, and
//! crossing into another document replaces both.

use std::sync::Arc;

use darkmatter::markdown::MappingOrders;

/// One document's authored mapping order, positioned at a node inside it.
///
/// A default cursor carries no order source. That is the correct state for a
/// document whose format records no key order (JSONL/NDJSON) and for a
/// task assembled in memory: lookups return `None` and the consumer keeps the
/// canonical `serde_json::Map` order it has always had.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthoredOrder {
    source: Option<Arc<MappingOrders>>,
    pointer: String,
}

impl AuthoredOrder {
    /// A cursor over `orders`, positioned at `pointer`.
    ///
    /// `pointer` is `""` for the document root.
    #[must_use]
    pub fn new(orders: Arc<MappingOrders>, pointer: impl Into<String>) -> Self {
        Self {
            source: Some(orders),
            pointer: pointer.into(),
        }
    }

    /// A cursor over the document root of `orders`.
    #[must_use]
    pub fn at_root(orders: Arc<MappingOrders>) -> Self {
        Self::new(orders, String::new())
    }

    /// The cursor positioned at the mapping key `segment` of the current node.
    #[must_use]
    pub fn child(&self, segment: &str) -> Self {
        Self {
            source: self.source.clone(),
            pointer: MappingOrders::child_pointer(&self.pointer, segment),
        }
    }

    /// The cursor positioned at list element `index` of the current node.
    #[must_use]
    pub fn at(&self, index: usize) -> Self {
        self.child(&index.to_string())
    }

    /// The JSON Pointer of the current node.
    #[must_use]
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// Authored key order of the mapping at the current node.
    #[must_use]
    pub fn key_order(&self) -> Option<&[String]> {
        self.source.as_ref()?.get(&self.pointer)
    }

    /// Authored key order of the mapping at the current node, owned.
    #[must_use]
    pub fn owned_key_order(&self) -> Option<Vec<String>> {
        self.key_order().map(<[String]>::to_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_file::serde_yaml_ng;

    fn orders(yaml: &str) -> Arc<MappingOrders> {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        Arc::new(MappingOrders::collect(&value))
    }

    #[test]
    fn walking_into_a_document_reaches_the_authored_order() {
        let cursor = AuthoredOrder::at_root(orders(
            "tasks:\n  - side_effect:\n      set:\n        z: 1\n        a: 2\n",
        ));

        let set = cursor.child("tasks").at(0).child("side_effect").child("set");

        assert_eq!(set.pointer(), "/tasks/0/side_effect/set");
        assert_eq!(
            set.key_order(),
            Some(["z".to_string(), "a".to_string()].as_slice())
        );
    }

    #[test]
    fn a_sourceless_cursor_reports_no_order_at_any_pointer() {
        let cursor = AuthoredOrder::default();

        assert_eq!(cursor.pointer(), "");
        assert_eq!(cursor.key_order(), None);
        assert_eq!(cursor.child("setup").at(3).owned_key_order(), None);
    }
}
