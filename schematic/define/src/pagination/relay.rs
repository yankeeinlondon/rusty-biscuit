//! The Relay connection preset for GraphQL operations (R40, Default D-12).
//!
//! [`Relay`] expands to an ordinary cursor [`Strategy`] and the [`Collection`]
//! that reads its items, so a Relay operation pages through the same model
//! as a REST endpoint. Response paths are relative to the decoded `data` of
//! the reply; request inputs are body paths under `variables`.

use super::{Collection, CursorSource, InputParam, MoreSignal, PageSize, Strategy};

/// A Relay connection at `root` under the reply's `data`, paged in one
/// direction.
///
/// | | [`forward`](Self::forward) | [`backward`](Self::backward) |
/// | --- | --- | --- |
/// | cursor input | `variables.after` | `variables.before` |
/// | page-size input | `variables.first` | `variables.last` |
/// | cursor | `{root}.pageInfo.endCursor` | `{root}.pageInfo.startCursor` |
/// | more signal | `{root}.pageInfo.hasNextPage` | `{root}.pageInfo.hasPreviousPage` |
/// | items | `{root}.nodes`, or `{root}.edges[*].node` with [`with_edges`](Self::with_edges) | same |
///
/// The variable names are overridable. The pager writes its inputs into the
/// serialized variables, so they must be optional fields of the operation's
/// variables type; a `null` or omitted initial cursor is valid.
///
/// One pager pages one direction: the cursor model has a single page-size
/// input and end signal per strategy, so a Relay connection read both ways
/// is two operations.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::Relay;
///
/// let issues = Relay::forward("repository.issues").with_edges().max_page_size(100).collection("Issue");
/// assert_eq!(issues.items.to_string(), "repository.issues.edges[*].node");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relay {
    root: String,
    backward: bool,
    edges: bool,
    cursor_variable: String,
    size_variable: String,
    max_page_size: Option<u32>,
    sent_page_size: Option<u32>,
    resumable: bool,
}

impl Relay {
    /// Page forward with `first`/`after` until `hasNextPage` is false.
    pub fn forward(root: &str) -> Self {
        Self::new(root, false, "after", "first")
    }

    /// Page backward with `last`/`before` until `hasPreviousPage` is false.
    pub fn backward(root: &str) -> Self {
        Self::new(root, true, "before", "last")
    }

    fn new(root: &str, backward: bool, cursor: &str, size: &str) -> Self {
        Self {
            root: root.to_owned(),
            backward,
            edges: false,
            cursor_variable: cursor.to_owned(),
            size_variable: size.to_owned(),
            max_page_size: None,
            sent_page_size: None,
            resumable: false,
        }
    }

    /// Read items from `edges[*].node` instead of `nodes`.
    pub fn with_edges(mut self) -> Self {
        self.edges = true;
        self
    }

    /// The variable the cursor is written to, in place of `after` or
    /// `before`.
    pub fn cursor_variable(mut self, name: &str) -> Self {
        self.cursor_variable = name.to_owned();
        self
    }

    /// The variable the page size is written to, in place of `first` or
    /// `last`.
    pub fn size_variable(mut self, name: &str) -> Self {
        self.size_variable = name.to_owned();
        self
    }

    /// The largest page size the caller may set; larger sizes fail before
    /// sending.
    pub fn max_page_size(mut self, max: u32) -> Self {
        self.max_page_size = Some(max);
        self
    }

    /// The page size the pager sends when the caller sets none. Without it,
    /// the size the caller's variables hold is sent unchanged.
    pub fn sent_page_size(mut self, size: u32) -> Self {
        self.sent_page_size = Some(size);
        self
    }

    /// Let the caller start from a cursor it kept, through the pager's
    /// `.after(..)`. The pager's one direction is the connection's, so for a
    /// backward connection that cursor is sent as `before`.
    pub fn resumable(mut self) -> Self {
        self.resumable = true;
        self
    }

    fn path(&self, tail: &str) -> String {
        if self.root.is_empty() { tail.to_owned() } else { format!("{}.{tail}", self.root) }
    }

    /// The cursor strategy.
    pub fn strategy(&self) -> Strategy {
        let (cursor, more) = if self.backward {
            ("pageInfo.startCursor", "pageInfo.hasPreviousPage")
        } else {
            ("pageInfo.endCursor", "pageInfo.hasNextPage")
        };
        let mut size = PageSize::new(InputParam::body(&format!("variables.{}", self.size_variable)));
        if let Some(max) = self.max_page_size {
            size = size.max(max);
        }
        if let Some(sent) = self.sent_page_size {
            size = size.sent_default(sent);
        }
        let mut builder = Strategy::cursor(
            InputParam::body(&format!("variables.{}", self.cursor_variable)),
            CursorSource::body(&self.path(cursor)),
        )
        .more_when(MoreSignal::has_more(&self.path(more)))
        .page_size(size);
        if self.resumable {
            builder = builder.resumable();
        }
        builder.build()
    }

    /// The paginated collection of `item_type`, ready for
    /// [`GraphQlOperationBuilder::pagination`](crate::graphql::GraphQlOperationBuilder::pagination).
    pub fn collection(self, item_type: &str) -> Collection {
        let items = self.path(if self.edges { "edges[*].node" } else { "nodes" });
        Collection::at(&items).item_type(item_type).paginate(self.strategy())
    }
}
