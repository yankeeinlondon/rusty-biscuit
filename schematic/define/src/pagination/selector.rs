//! Locations inside a JSON page: value paths and item selectors.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A path of object keys into a JSON document.
///
/// [`JsonPath::parse`] splits on `.`; use [`JsonPath::segments`] for a key
/// that itself contains a dot (`@odata.nextLink`). Parsing keeps empty
/// segments (`""`, `"a..b"`) so validation can reject them: the document root
/// is spelled only as [`JsonPath::root`].
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::JsonPath;
///
/// assert_eq!(JsonPath::parse("meta.hasnext").as_segments(), ["meta", "hasnext"]);
/// assert_eq!(JsonPath::segments(["@odata.nextLink"]).as_segments(), ["@odata.nextLink"]);
/// assert!(JsonPath::root().is_root());
/// assert!(!JsonPath::parse("").is_root());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonPath(Vec<String>);

impl JsonPath {
    /// The document itself.
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Splits `dotted` on every `.`.
    pub fn parse(dotted: &str) -> Self {
        Self(dotted.split('.').map(str::to_owned).collect())
    }

    /// Builds a path from whole keys, none of which is split.
    pub fn segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self(segments.into_iter().map(Into::into).collect())
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_segments(&self) -> &[String] {
        &self.0
    }

    pub(crate) fn has_empty_segment(&self) -> bool {
        self.0.iter().any(String::is_empty)
    }

    /// Whether `other` starts with every segment of `self`.
    pub(crate) fn is_prefix_of(&self, other: &JsonPath) -> bool {
        other.0.starts_with(&self.0)
    }
}

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("."))
    }
}

/// One step of an [`ItemSelector`]'s record path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// Descend into an object key.
    Key(String),
    /// Fan out over every element of an array (`[*]`).
    Each,
}

/// Selects the items of a page through any number of array fan-outs.
///
/// The element reached by the *last* `[*]` is the **record**; whatever follows
/// is a projection from record to the item handed to the caller. Item-derived
/// cursors read the record, so Elasticsearch's `sort` (on the hit) and Relay's
/// `cursor` (on the edge) work while the caller receives `_source` / `node`.
///
/// `parse("data")` means `data[*]`; `parse("[*]")` and [`ItemSelector::root`]
/// are a root array; `parse("responses[*].hits.hits[*]._source")` flattens two
/// levels. `parse("")` is not a root array: it names an empty key, which
/// validation rejects.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::{ItemSelector, JsonPath};
///
/// let relay = ItemSelector::parse("data.issues.edges[*].node");
/// assert_eq!(relay.projection, JsonPath::parse("node"));
/// assert_eq!(ItemSelector::parse("data"), ItemSelector::parse("data[*]"));
/// assert!(ItemSelector::parse("[*]").is_root_array());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSelector {
    /// Steps from the page root to each record; ends with [`Step::Each`].
    pub record: Vec<Step>,
    /// Path from a record to the item the caller receives.
    pub projection: JsonPath,
}

impl ItemSelector {
    /// The page body is the array of items.
    pub fn root() -> Self {
        Self {
            record: vec![Step::Each],
            projection: JsonPath::root(),
        }
    }

    /// Parses a selector expression; see the type documentation.
    pub fn parse(expression: &str) -> Self {
        let mut steps = Vec::new();
        for segment in expression.split('.') {
            match segment.strip_suffix("[*]") {
                Some("") => steps.push(Step::Each),
                Some(name) => {
                    steps.push(Step::Key(name.to_owned()));
                    steps.push(Step::Each);
                }
                None => steps.push(Step::Key(segment.to_owned())),
            }
        }
        match steps.iter().rposition(|step| *step == Step::Each) {
            None => {
                steps.push(Step::Each);
                Self {
                    record: steps,
                    projection: JsonPath::root(),
                }
            }
            Some(last_each) => {
                let projected = steps.split_off(last_each + 1);
                // A projection has no fan-out by construction: `last_each` is
                // the last one.
                let keys = projected.into_iter().filter_map(|step| match step {
                    Step::Key(key) => Some(key),
                    Step::Each => None,
                });
                Self {
                    record: steps,
                    projection: JsonPath::segments(keys),
                }
            }
        }
    }

    /// The number of `[*]` steps.
    pub fn fan_outs(&self) -> usize {
        self.record.iter().filter(|step| **step == Step::Each).count()
    }

    pub fn is_root_array(&self) -> bool {
        self.record == [Step::Each] && self.projection.is_root()
    }

    /// Why this selector cannot locate items, if it cannot.
    pub(crate) fn syntax_problem(&self) -> Option<&'static str> {
        if self.record.last() != Some(&Step::Each) {
            return Some("the record path must end with a `[*]` fan-out");
        }
        let bad_key = |key: &String| key.is_empty() || key.contains(['[', ']']);
        if self
            .record
            .iter()
            .any(|step| matches!(step, Step::Key(key) if bad_key(key)))
            || self.projection.as_segments().iter().any(bad_key)
        {
            return Some("a key is empty or contains `[` or `]`");
        }
        None
    }
}

impl fmt::Display for ItemSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut segments: Vec<String> = Vec::new();
        for step in &self.record {
            match step {
                // An empty key is invalid; quoting it keeps it from reading as
                // a root array in a diagnostic.
                Step::Key(key) if key.is_empty() => segments.push("\"\"".to_owned()),
                Step::Key(key) => segments.push(key.clone()),
                Step::Each => match segments.last_mut() {
                    Some(last) if !last.ends_with("[*]") => last.push_str("[*]"),
                    _ => segments.push("[*]".to_owned()),
                },
            }
        }
        segments.extend(self.projection.as_segments().iter().cloned());
        f.write_str(&segments.join("."))
    }
}
