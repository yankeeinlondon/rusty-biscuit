//! Constraint violations: what was refused and where, never the value.

use std::fmt;

use crate::error::InvalidArgument;
use crate::redact::REDACTED;

/// One step of a [`Violation`]'s path.
///
/// Names are `&'static str`, written by generated code or taken from a
/// [`Plan`](super::Plan), so a path can never hold a submitted string. An
/// arbitrary map key, which may be a secret, is the payload-free
/// [`PathSegment::RedactedKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PathSegment {
    /// A location (`query`, `body`), a parameter name, or a declared
    /// property's serialized name.
    Name(&'static str),
    /// An array position.
    Index(usize),
    /// A map entry whose key is not shown.
    RedactedKey,
}

/// A supported validation keyword, named as in JSON Schema.
///
/// The variant order is the order in which one value's violations are
/// reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Keyword {
    MinLength,
    MaxLength,
    Pattern,
    Minimum,
    ExclusiveMinimum,
    Maximum,
    ExclusiveMaximum,
    MultipleOf,
    MinItems,
    MaxItems,
    UniqueItems,
    Enum,
    Const,
}

impl Keyword {
    /// The JSON Schema spelling, such as `minLength`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MinLength => "minLength",
            Self::MaxLength => "maxLength",
            Self::Pattern => "pattern",
            Self::Minimum => "minimum",
            Self::ExclusiveMinimum => "exclusiveMinimum",
            Self::Maximum => "maximum",
            Self::ExclusiveMaximum => "exclusiveMaximum",
            Self::MultipleOf => "multipleOf",
            Self::MinItems => "minItems",
            Self::MaxItems => "maxItems",
            Self::UniqueItems => "uniqueItems",
            Self::Enum => "enum",
            Self::Const => "const",
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One refused constraint: where, which keyword, and why.
///
/// The reason is fixed text plus declared limits (lengths, counts, bounds,
/// array positions). It never quotes the submitted value, an allowed-value
/// set, a constant, or a pattern, so `Display` and `Debug` are safe to log.
/// Only this crate's checks construct one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    path: Vec<PathSegment>,
    keyword: Keyword,
    reason: String,
}

#[allow(missing_docs)]
impl Violation {
    pub(crate) fn new(path: Vec<PathSegment>, keyword: Keyword, reason: String) -> Self {
        Self { path, keyword, reason }
    }

    pub fn path(&self) -> &[PathSegment] {
        &self.path
    }

    pub fn keyword(&self) -> Keyword {
        self.keyword
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The path in dotted form, such as `body.items[2].displayName` or
    /// `body.labels[[redacted]]`; `value` when the path is empty.
    pub fn path_text(&self) -> String {
        let mut text = String::new();
        for segment in &self.path {
            match segment {
                PathSegment::Name(name) => {
                    if !text.is_empty() {
                        text.push('.');
                    }
                    text.push_str(name);
                }
                PathSegment::Index(index) => text.push_str(&format!("[{index}]")),
                PathSegment::RedactedKey => text.push_str(&format!("[{REDACTED}]")),
            }
        }
        if text.is_empty() {
            text.push_str("value");
        }
        text
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} violates {}: {}", self.path_text(), self.keyword, self.reason)
    }
}

/// Every violation of one check, in report order.
///
/// Checks append in a deterministic order: the order in which generated
/// code checks its parameters (their declaration order), then for each
/// value its own keywords in [`Keyword`] order, its object entries by key
/// (bytewise), and its array items by index. Converting into
/// [`InvalidArgument`] keeps the aggregate text and drops the structure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Violations {
    list: Vec<Violation>,
}

#[allow(missing_docs)]
impl Violations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Violation> {
        self.list.iter()
    }

    /// `Ok` when nothing was refused, otherwise `Err(self)`.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }

    pub(crate) fn push(&mut self, violation: Violation) {
        self.list.push(violation);
    }
}

impl<'a> IntoIterator for &'a Violations {
    type Item = &'a Violation;
    type IntoIter = std::slice::Iter<'a, Violation>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}

impl IntoIterator for Violations {
    type Item = Violation;
    type IntoIter = std::vec::IntoIter<Violation>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
}

impl fmt::Display for Violations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.list.as_slice() {
            [] => f.write_str("no constraint violations"),
            [only] => write!(f, "{only}"),
            all => {
                write!(f, "{} constraint violations: ", all.len())?;
                for (position, violation) in all.iter().enumerate() {
                    if position > 0 {
                        f.write_str("; ")?;
                    }
                    write!(f, "{violation}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Violations {}

/// The aggregate text becomes the reason; it is value-free like every
/// violation.
impl From<Violations> for InvalidArgument {
    fn from(violations: Violations) -> Self {
        InvalidArgument::new(violations.to_string())
    }
}
