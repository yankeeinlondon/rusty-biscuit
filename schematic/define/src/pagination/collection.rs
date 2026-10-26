//! Endpoint-owned list shapes and pagination policies.

use serde::{Deserialize, Serialize};

use super::selector::ItemSelector;
use super::strategy::Strategy;

/// How a list paginates.
// Built once per definition at generation time; boxing would only make
// authoring and matching clumsier.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", content = "strategy", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Pagination {
    /// Use the API's default strategy; unpaginated when there is none.
    Inherit,
    /// The list has no pagination, whatever the API default.
    Unpaginated,
    /// This strategy, overriding the API default.
    Explicit(Strategy),
}

/// A named, caller-selectable alternative strategy (GitLab's `keyset`).
///
/// Alternates are modeled and validated; the generated client does not expose
/// them yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alternate {
    /// Lowercase ASCII letters, digits, and `_`.
    pub label: String,
    pub strategy: Strategy,
}

/// A list endpoint's shape: where its items are, their type, and how it
/// paginates.
///
/// `item_type` is a Rust type path spelled like a response type
/// (`WorkflowRun`, `serde_json::Value`). It may be omitted only for a root
/// array whose endpoint declares an explicit `Vec<T>` response, where `T` is
/// the item type.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::{Collection, Pagination};
///
/// let runs = Collection::at("workflow_runs").item_type("WorkflowRun");
/// assert_eq!(runs.items.to_string(), "workflow_runs[*]");
/// assert_eq!(runs.pagination, Pagination::Inherit);
///
/// let opt_out = Collection::root().unpaginated();
/// assert!(opt_out.items.is_root_array());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    pub items: ItemSelector,
    pub item_type: Option<String>,
    pub pagination: Pagination,
    pub alternates: Vec<Alternate>,
}

impl Collection {
    /// The response body is the array of items.
    pub fn root() -> Self {
        Self::with_items(ItemSelector::root())
    }

    /// Items at a selector expression: `at("data")`, `at("edges[*].node")`,
    /// `at("data.children[*].data")`. See [`ItemSelector`].
    pub fn at(expression: &str) -> Self {
        Self::with_items(ItemSelector::parse(expression))
    }

    fn with_items(items: ItemSelector) -> Self {
        Self {
            items,
            item_type: None,
            pagination: Pagination::Inherit,
            alternates: Vec::new(),
        }
    }

    pub fn item_type(mut self, type_path: &str) -> Self {
        self.item_type = Some(type_path.to_owned());
        self
    }

    pub fn paginate(mut self, strategy: impl Into<Strategy>) -> Self {
        self.pagination = Pagination::Explicit(strategy.into());
        self
    }

    pub fn unpaginated(mut self) -> Self {
        self.pagination = Pagination::Unpaginated;
        self
    }

    pub fn or_paginate(mut self, label: &str, strategy: impl Into<Strategy>) -> Self {
        self.alternates.push(Alternate {
            label: label.to_owned(),
            strategy: strategy.into(),
        });
        self
    }
}
