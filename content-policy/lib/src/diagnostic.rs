//! Validation diagnostics and reader warnings.
//!
//! Field names and code spellings serialize into the JSON report and are
//! public contract once the CLI ships them.

use std::fmt;

use serde::Serialize;

/// Stable code of a validation diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    /// The policy value is `null` (`content_policy:` with no value).
    NullPolicy,
    /// The policy value is a single string instead of a list.
    NotAList,
    /// The policy value, an entry, or a field has the wrong type.
    WrongType,
    /// An explicit empty list.
    EmptyPolicy,
    /// An entry is `null` (`- ` with no value).
    NullEntry,
    /// A rule string that is not `Name` or `Name(arguments)`.
    InvalidRuleSyntax,
    /// A rule string whose parentheses do not balance, usually a flow list
    /// split at a comma.
    UnbalancedParentheses,
    /// A rule name this grammar does not define.
    UnknownRule,
    /// The wrong number or kind of rule arguments.
    InvalidArguments,
    /// A duration that is not a positive whole number with one of `d`, `wk`,
    /// `mo`, `yr`.
    InvalidDuration,
    /// A date that is not a real `YYYY-MM-DD` calendar date.
    InvalidDate,
    /// A timestamp where only a date is accepted.
    DatesOnly,
    /// A computed date outside the supported calendar range.
    DateOutOfRange,
    /// An `@name` reference that is not a valid property name.
    InvalidReference,
    /// An `@a.b` reference; only top-level properties can be referenced.
    NestedReference,
    /// `Evergreen` combined with another entry.
    EvergreenCombined,
    /// A long-form entry without `rule`.
    MissingRule,
    /// A long-form entry without `action`.
    MissingAction,
    /// An action other than `refresh`, `archive`, or `remove`.
    UnknownAction,
    /// A key other than `rule` and `action` in a long-form entry, or an
    /// unknown field in a serialized policy.
    UnknownField,
    /// A key defined twice in one mapping.
    DuplicateKey,
    /// A serialized policy from a newer grammar than this library reads.
    UnsupportedGrammarVersion,
    /// A serialized policy that is not the normalized JSON shape.
    MalformedSerialization,
}

/// Which part of a long-form entry a diagnostic points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryField {
    Rule,
    Action,
}

/// Where a diagnostic applies. Entry indexes are zero-based positions in the
/// declared policy list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    /// The policy value as a whole.
    Policy,
    /// One entry of the policy list.
    Entry { index: usize },
    /// The `rule` or `action` of a long-form entry.
    EntryField { index: usize, field: EntryField },
    /// An evidence property, as resolved for the entry at `entry`.
    Property { name: String, entry: Option<usize> },
    /// The document's frontmatter as a whole.
    Frontmatter,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub location: Location,
    pub message: String,
}

impl Diagnostic {
    #[allow(missing_docs)]
    #[must_use]
    pub fn new(code: DiagnosticCode, location: Location, message: impl Into<String>) -> Self {
        Self {
            code,
            location,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Location::Policy => write!(f, "policy: {}", self.message),
            Location::Entry { index } => write!(f, "entry {}: {}", index + 1, self.message),
            Location::EntryField { index, field } => {
                let field = match field {
                    EntryField::Rule => "rule",
                    EntryField::Action => "action",
                };
                write!(f, "entry {} {field}: {}", index + 1, self.message)
            }
            Location::Property { name, entry } => match entry {
                Some(index) => write!(f, "entry {} property `{name}`: {}", index + 1, self.message),
                None => write!(f, "property `{name}`: {}", self.message),
            },
            Location::Frontmatter => write!(f, "frontmatter: {}", self.message),
        }
    }
}

/// Stable code of a non-fatal reader warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    /// Tab-indented frontmatter was repaired in memory before parsing.
    TabIndentationRepaired,
}

/// A non-fatal finding carried by a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub code: WarningCode,
    pub message: String,
}

/// A declaration or evidence record that cannot produce a verdict.
///
/// It lists every invalid entry, never just the first: a verdict built from
/// the valid remainder could understate the action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Invalid {
    pub document: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub warnings: Vec<Warning>,
}

impl Invalid {
    pub(crate) fn new(diagnostics: Vec<Diagnostic>) -> Self {
        debug_assert!(!diagnostics.is_empty());
        Self {
            document: None,
            diagnostics,
            warnings: Vec::new(),
        }
    }

    /// Returns `true` when any diagnostic has `code`.
    #[must_use]
    pub fn has(&self, code: DiagnosticCode) -> bool {
        self.diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(document) = &self.document {
            write!(f, "{document}: ")?;
        }
        write!(f, "invalid content policy")?;
        for diagnostic in &self.diagnostics {
            write!(f, "\n  {diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Invalid {}
