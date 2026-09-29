//! Policy model: rules, actions, entries, policies, evidence, and options.

use std::fmt;

use chrono::NaiveDate;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::diagnostic::Invalid;

/// Version of the policy grammar this library reads and writes.
///
/// It is part of every serialized policy and of every policy identity, so a
/// grammar change invalidates artifacts cached under an older policy. A
/// serialized policy with a newer version is rejected (fail closed).
pub const GRAMMAR_VERSION: u32 = 1;

/// What a triggered entry asks the consumer to do with the content.
///
/// The ordering is the fixed precedence `Refresh < Archive < Remove`, so the
/// effective action of several triggers is their maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Refresh,
    Archive,
    Remove,
}

impl Action {
    /// Every action, lowest precedence first.
    pub const ALL: [Self; 3] = [Self::Refresh, Self::Archive, Self::Remove];

    #[allow(missing_docs)]
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Refresh => "refresh",
            Self::Archive => "archive",
            Self::Remove => "remove",
        }
    }

    /// Parses an action name. Names are case-sensitive.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.as_str() == text)
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Unit of a [`Duration`]. `Days` and `Weeks` are UTC day increments;
/// `Months` and `Years` are calendar arithmetic that clamps to the last day of
/// the destination month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DurationUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl DurationUnit {
    /// The unit as written in a rule: `d`, `wk`, `mo`, or `yr`.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Days => "d",
            Self::Weeks => "wk",
            Self::Months => "mo",
            Self::Years => "yr",
        }
    }
}

/// A positive whole number of one [`DurationUnit`], such as `3mo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration {
    count: u32,
    unit: DurationUnit,
}

#[allow(missing_docs)]
impl Duration {
    /// Returns `None` for a zero count; durations are positive.
    #[must_use]
    pub fn new(count: u32, unit: DurationUnit) -> Option<Self> {
        (count > 0).then_some(Self { count, unit })
    }

    #[must_use]
    pub fn count(self) -> u32 {
        self.count
    }

    #[must_use]
    pub fn unit(self) -> DurationUnit {
        self.unit
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.count, self.unit.suffix())
    }
}

/// Where a `ValidFor` rule's starting date comes from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Baseline {
    /// A date written inside the rule, `ValidFor(3mo, 2026-09-28)`.
    Inline(NaiveDate),
    /// A top-level evidence property, `ValidFor(3mo, @reviewed)`.
    Reference(String),
    /// The shorthand `ValidFor(3mo)`: the caller's default date property
    /// ([`PolicyOptions::date_property`]).
    Defaulted,
}

/// Where a `ValidUntil` rule's deadline comes from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Deadline {
    Inline(NaiveDate),
    Reference(String),
}

/// A condition that can trigger.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Rule {
    /// Never triggers.
    Evergreen,
    /// Always triggers.
    TimeSensitive,
    /// Triggers once evaluation time reaches `baseline + duration`.
    ValidFor { duration: Duration, baseline: Baseline },
    /// Triggers once evaluation time reaches the deadline. Renewal never moves it.
    ValidUntil { deadline: Deadline },
}

impl Rule {
    /// The rule's name as written, such as `ValidFor`.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Evergreen => "Evergreen",
            Self::TimeSensitive => "TimeSensitive",
            Self::ValidFor { .. } => "ValidFor",
            Self::ValidUntil { .. } => "ValidUntil",
        }
    }

    /// How renewal treats this rule.
    #[must_use]
    pub fn renewal(&self) -> Renewal {
        match self {
            Self::Evergreen | Self::TimeSensitive => Renewal::NoBaseline,
            Self::ValidFor { .. } => Renewal::Renewable,
            Self::ValidUntil { .. } => Renewal::Nonrenewable,
        }
    }
}

/// The canonical compact form, `ValidFor(3mo, @last_updated)`. Parsing it
/// back yields an equal rule.
impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evergreen | Self::TimeSensitive => f.write_str(self.name()),
            Self::ValidFor { duration, baseline } => match baseline {
                Baseline::Inline(date) => write!(f, "ValidFor({duration}, {date})"),
                Baseline::Reference(name) => write!(f, "ValidFor({duration}, @{name})"),
                Baseline::Defaulted => write!(f, "ValidFor({duration})"),
            },
            Self::ValidUntil { deadline } => match deadline {
                Deadline::Inline(date) => write!(f, "ValidUntil({date})"),
                Deadline::Reference(name) => write!(f, "ValidUntil(@{name})"),
            },
        }
    }
}

/// Renewal classification of a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Renewal {
    /// Renewal advances the rule's baseline (`ValidFor`).
    Renewable,
    /// Renewal never moves the rule's date (`ValidUntil`).
    Nonrenewable,
    /// The rule has no baseline (`Evergreen`, `TimeSensitive`).
    NoBaseline,
}

/// A rule plus the action it requests when triggered.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyEntry {
    pub rule: Rule,
    pub action: Action,
}

impl PolicyEntry {
    /// An entry with the compact form's default action, `refresh`.
    #[must_use]
    pub fn new(rule: Rule) -> Self {
        Self {
            rule,
            action: Action::Refresh,
        }
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn with_action(rule: Rule, action: Action) -> Self {
        Self { rule, action }
    }
}

/// A validated, non-empty list of policy entries.
///
/// Every constructor enforces the declaration rules: at least one entry, and
/// `Evergreen` only on its own. The grammar version is always
/// [`GRAMMAR_VERSION`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    entries: Vec<PolicyEntry>,
}

impl Policy {
    /// ## Errors
    ///
    /// Returns [`Invalid`] for an empty list or `Evergreen` combined with
    /// another entry.
    pub fn new(entries: Vec<PolicyEntry>) -> Result<Self, Invalid> {
        let diagnostics = crate::grammar::policy_shape_diagnostics(&entries);
        if diagnostics.is_empty() {
            Ok(Self { entries })
        } else {
            Err(Invalid::new(diagnostics))
        }
    }

    /// Parses a declaration value, as it appears under the policy key in an
    /// evidence record: a list of compact rule strings and `{rule, action}`
    /// mappings.
    ///
    /// ## Errors
    ///
    /// Returns [`Invalid`] listing every invalid entry, not just the first.
    pub fn from_declaration(value: &Value) -> Result<Self, Invalid> {
        crate::grammar::parse_declaration(value)
    }

    /// Parses a policy from command-line text: one compact rule (action
    /// `refresh`), or a YAML-style flow list when the text starts with `[`,
    /// such as `["ValidFor(3mo)", {rule: "ValidUntil(2027-01-01)", action: archive}]`.
    ///
    /// ## Errors
    ///
    /// Returns [`Invalid`] for empty, unparseable, or invalid text.
    pub fn from_text(text: &str) -> Result<Self, Invalid> {
        crate::grammar::parse_policy_text(text)
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn entries(&self) -> &[PolicyEntry] {
        &self.entries
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn grammar_version(&self) -> u32 {
        GRAMMAR_VERSION
    }

    pub(crate) fn from_valid_entries(entries: Vec<PolicyEntry>) -> Self {
        debug_assert!(crate::grammar::policy_shape_diagnostics(&entries).is_empty());
        Self { entries }
    }
}

/// The property map that `@name` references resolve against: frontmatter for
/// a Markdown document, a manifest for a cache artifact.
///
/// A present `null` counts as missing evidence wherever a date is expected.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvidenceRecord(Map<String, Value>);

#[allow(missing_docs)]
impl EvidenceRecord {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    pub fn insert(&mut self, name: impl Into<String>, value: Value) -> Option<Value> {
        self.0.insert(name.into(), value)
    }

    #[must_use]
    pub fn as_map(&self) -> &Map<String, Value> {
        &self.0
    }

    #[must_use]
    pub fn into_map(self) -> Map<String, Value> {
        self.0
    }
}

impl From<Map<String, Value>> for EvidenceRecord {
    fn from(map: Map<String, Value>) -> Self {
        Self(map)
    }
}

impl FromIterator<(String, Value)> for EvidenceRecord {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Caller configuration shared by evaluation and renewal.
///
/// There is always a default policy: it can be replaced but not removed, so
/// every valid document gets a status. A fail-closed consumer, such as a
/// cache, replaces it with `TimeSensitive`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyOptions {
    key: String,
    default_policy: Policy,
    date_property: String,
}

impl PolicyOptions {
    /// The built-in policy key.
    pub const DEFAULT_KEY: &'static str = "content_policy";
    /// The built-in default date property.
    pub const DEFAULT_DATE_PROPERTY: &'static str = "last_updated";

    /// The built-in default policy, `ValidFor(6mo)`.
    #[must_use]
    pub fn builtin_default_policy() -> Policy {
        let six_months = Duration::new(6, DurationUnit::Months).expect("6 is positive");
        Policy::from_valid_entries(vec![PolicyEntry::new(Rule::ValidFor {
            duration: six_months,
            baseline: Baseline::Defaulted,
        })])
    }

    /// Replaces the frontmatter key the policy is read from.
    #[must_use]
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    /// Replaces the policy used when a record declares none.
    #[must_use]
    pub fn with_default_policy(mut self, policy: Policy) -> Self {
        self.default_policy = policy;
        self
    }

    /// Replaces the property the `ValidFor(<duration>)` shorthand reads.
    #[must_use]
    pub fn with_date_property(mut self, name: impl Into<String>) -> Self {
        self.date_property = name.into();
        self
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn default_policy(&self) -> &Policy {
        &self.default_policy
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn date_property(&self) -> &str {
        &self.date_property
    }
}

impl Default for PolicyOptions {
    fn default() -> Self {
        Self {
            key: Self::DEFAULT_KEY.to_string(),
            default_policy: Self::builtin_default_policy(),
            date_property: Self::DEFAULT_DATE_PROPERTY.to_string(),
        }
    }
}

pub(crate) fn serialize_display<T: fmt::Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}
