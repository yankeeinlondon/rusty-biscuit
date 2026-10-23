//! Type errors from binding: one [`ValueFailure`] per failing path.
//!
//! The data is public so a caller can wrap it in its own context (function,
//! signature, parameter, property, source location) without parsing text.

use std::fmt;

use serde_json::Value;

use super::shape::{IntegerRule, compact};
use crate::markdown::numeric::{NumericAdmissionError, format_number};

/// One step into a container value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    Index(usize),
    Key(String),
}

/// Renders a path relative to `base`: `values` + `[2]` is `values[2]`,
/// `meta` + `inner`, `count` is `meta.inner.count`. A key that is not an
/// identifier is bracketed and quoted (`labels["a b"]`). An empty `base`
/// with an empty path renders as the empty string.
pub fn render_path(base: &str, path: &[PathSegment]) -> String {
    let mut out = base.to_string();
    for segment in path {
        match segment {
            PathSegment::Index(index) => out.push_str(&format!("[{index}]")),
            PathSegment::Key(key) if is_identifier(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            PathSegment::Key(key) => out.push_str(&format!("[{}]", compact(&Value::String(key.clone())))),
        }
    }
    out
}

fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-')
}

/// The type name of a runtime value, as messages spell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Text,
    Number,
    Boolean,
    Null,
    List,
    Object,
}

impl ValueKind {
    pub fn of(value: &Value) -> Self {
        match value {
            Value::String(_) => Self::Text,
            Value::Number(_) => Self::Number,
            Value::Bool(_) => Self::Boolean,
            Value::Null => Self::Null,
            Value::Array(_) => Self::List,
            Value::Object(_) => Self::Object,
        }
    }
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
            Self::List => "list",
            Self::Object => "object",
        })
    }
}

/// A constraint a value failed.
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Minimum(f64),
    Maximum(f64),
    ExclusiveMinimum(f64),
    ExclusiveMaximum(f64),
    MultipleOf(f64),
    Integer(IntegerRule),
    /// In Unicode scalar values.
    MinLength(usize),
    /// In Unicode scalar values.
    MaxLength(usize),
    Pattern(String),
    Members(Vec<String>),
    Literal(Value),
    MinItems(usize),
    MaxItems(usize),
    UniqueItems,
    MinProperties(usize),
    MaxProperties(usize),
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minimum(bound) => write!(f, "min({})", format_number(*bound)),
            Self::Maximum(bound) => write!(f, "max({})", format_number(*bound)),
            Self::ExclusiveMinimum(bound) => write!(f, "exclusive-min({})", format_number(*bound)),
            Self::ExclusiveMaximum(bound) => write!(f, "exclusive-max({})", format_number(*bound)),
            Self::MultipleOf(step) => write!(f, "multiple-of({})", format_number(*step)),
            Self::Integer(IntegerRule::Whole) => f.write_str("integer"),
            Self::Integer(IntegerRule::WholeBelow2Pow64) => f.write_str("integer (whole, below 2^64)"),
            Self::MinLength(length) => write!(f, "min({length})"),
            Self::MaxLength(length) => write!(f, "max({length})"),
            Self::Pattern(source) => write!(f, "pattern({source})"),
            Self::Members(members) => {
                let members: Vec<String> = members.iter().map(|m| compact(&Value::String(m.clone()))).collect();
                write!(f, "enum({})", members.join(", "))
            }
            Self::Literal(value) => write!(f, "literal({})", compact(value)),
            Self::MinItems(count) => write!(f, "min({count}) elements"),
            Self::MaxItems(count) => write!(f, "max({count}) elements"),
            Self::UniqueItems => f.write_str("unique elements"),
            Self::MinProperties(count) => write!(f, "min-keys({count})"),
            Self::MaxProperties(count) => write!(f, "max-keys({count})"),
        }
    }
}

/// Why one union option failed, for [`FailureReason::NoOptionAccepts`].
#[derive(Debug, Clone, PartialEq)]
pub struct OptionFailure {
    /// The option's expected-type text.
    pub expected: String,
    pub failures: Vec<ValueFailure>,
}

/// Why a value failed to bind.
#[derive(Debug, Clone, PartialEq)]
pub enum FailureReason {
    /// No conversion leads from the received type to the expected one
    /// (`null` to anything, boolean to number, text to a list, ...).
    NoConversion,
    /// The value would need a conversion, and an exact check performs none.
    ConversionNotAllowed,
    /// Text that the conversion or content check could not read: not a
    /// number, not a boolean word, or invalid `json`/`yaml`.
    MalformedText { detail: String },
    /// A number spelling or native number Darkmatter cannot hold.
    NumberNotAdmitted(NumericAdmissionError),
    /// The value, after any conversion, fails a constraint.
    ConstraintNotMet(Constraint),
    /// A required tuple slot or object property is missing.
    MissingRequired,
    /// A list element beyond what a tuple declares.
    UnexpectedElement,
    /// An object key the shape does not allow.
    UnexpectedProperty,
    /// Several union options were reachable with different results and no
    /// tie-breaker chose one; every reading, in option order.
    AmbiguousUnion { readings: Vec<Value> },
    /// A key under several overlapping declarations, whose converted readings
    /// do not satisfy all of them; every reading.
    ConflictingDeclarations { readings: Vec<Value> },
    /// No union option accepts the value; each option and why it failed.
    NoOptionAccepts { options: Vec<OptionFailure> },
    AssertionFailed { description: String },
    /// The value nests deeper than the engine walks.
    ResourceLimit { max_depth: usize },
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConversion => f.write_str("no conversion from that type"),
            Self::ConversionNotAllowed => f.write_str("not an exact match, and this check performs no conversion"),
            Self::MalformedText { detail } => write!(f, "conversion failed: {detail}"),
            Self::NumberNotAdmitted(reason) => reason.fmt(f),
            Self::ConstraintNotMet(constraint) => write!(f, "fails {constraint}"),
            Self::MissingRequired => f.write_str("required but missing"),
            Self::UnexpectedElement => f.write_str("unexpected element"),
            Self::UnexpectedProperty => f.write_str("unexpected property"),
            Self::AmbiguousUnion { readings } => {
                write!(f, "ambiguous union: the value reads as {}", join_readings(readings))
            }
            Self::ConflictingDeclarations { readings } => write!(
                f,
                "overlapping declarations disagree: the value reads as {}",
                join_readings(readings)
            ),
            Self::NoOptionAccepts { options } => {
                f.write_str("no option accepts it")?;
                for option in options {
                    write!(f, "\n    {}:", option.expected)?;
                    for failure in &option.failures {
                        let line = failure.to_string().replace('\n', "\n    ");
                        write!(f, "\n      {line}")?;
                    }
                }
                Ok(())
            }
            Self::AssertionFailed { description } => write!(f, "fails {description}"),
            Self::ResourceLimit { max_depth } => write!(f, "nested deeper than {max_depth} levels"),
        }
    }
}

fn join_readings(readings: &[Value]) -> String {
    readings.iter().map(compact).collect::<Vec<_>>().join(" or ")
}

/// One failing value.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueFailure {
    /// Relative to the value bound; empty for the value itself.
    pub path: Vec<PathSegment>,
    /// The expected-type text, constraints included.
    pub expected: String,
    /// The value found at `path`; `None` when it is missing.
    pub received: Option<Value>,
    /// The converted value that then failed, when a conversion ran.
    pub converted: Option<Value>,
    pub reason: FailureReason,
}

impl ValueFailure {
    /// The received value's type; `None` when it is missing.
    pub fn received_kind(&self) -> Option<ValueKind> {
        self.received.as_ref().map(ValueKind::of)
    }

    /// The path rendered without a base (`[2]`, `meta.inner.count`).
    pub fn path_text(&self) -> String {
        render_path("", &self.path)
    }
}

impl fmt::Display for ValueFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path_text();
        if !path.is_empty() {
            write!(f, "{path}: ")?;
        }
        write!(f, "expected {}; ", self.expected)?;
        match &self.received {
            Some(Value::Null) => f.write_str("received null")?,
            Some(received) => write!(f, "received {} {}", ValueKind::of(received), compact(received))?,
            None => f.write_str("received nothing")?,
        }
        if let Some(converted) = &self.converted {
            write!(f, ", converted to {}", compact(converted))?;
        }
        write!(f, "; {}", self.reason)
    }
}

/// A type error: every value that failed, not only the first.
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionError {
    pub failures: Vec<ValueFailure>,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.failures.as_slice() {
            [only] => write!(f, "{only}"),
            failures => {
                write!(f, "{} values failed:", failures.len())?;
                for failure in failures {
                    let line = failure.to_string().replace('\n', "\n  ");
                    write!(f, "\n  {line}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConversionError {}
