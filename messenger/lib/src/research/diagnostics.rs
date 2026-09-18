//! Stable-order validation findings.
//!
//! Every finding names a rule code from `docs/research/platforms/_rules.md`
//! (or `SCHEMA` for a SimplifiedSchema problem), the repository-relative file,
//! a JSON Pointer into its frontmatter, and the stable ID it concerns.
//! Messages are built from IDs, field names, and enum values only; they never
//! echo fixture bodies, payloads, or other provider text.

use std::fmt;

use serde::Serialize;

use super::paths::RepoPath;

/// A rule code. `Schema` reports Darkmatter SimplifiedSchema problems; the
/// rest are the Rust-owned semantic rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Rule {
    #[serde(rename = "SCHEMA")]
    Schema,
    #[serde(rename = "SR-SCHEMA-BINDING")]
    SchemaBinding,
    #[serde(rename = "SR-TOP-LEVEL")]
    TopLevel,
    #[serde(rename = "SR-VERSION")]
    Version,
    #[serde(rename = "SR-STRICT-SCALARS")]
    StrictScalars,
    #[serde(rename = "SR-ROSTER")]
    Roster,
    #[serde(rename = "SR-CURATED")]
    Curated,
    #[serde(rename = "SR-UNIQUE")]
    Unique,
    #[serde(rename = "SR-REF")]
    Ref,
    #[serde(rename = "SR-EVIDENCE")]
    Evidence,
    #[serde(rename = "SR-STATE-VALUE")]
    StateValue,
    #[serde(rename = "SR-CONDITION")]
    Condition,
    #[serde(rename = "SR-APPLICABILITY")]
    Applicability,
    #[serde(rename = "SR-AGGREGATE")]
    Aggregate,
    #[serde(rename = "SR-KIND-UNIT")]
    KindUnit,
    #[serde(rename = "SR-COVERAGE")]
    Coverage,
    #[serde(rename = "SR-GAP")]
    Gap,
    #[serde(rename = "SR-CHANGE")]
    Change,
    #[serde(rename = "SR-FORMAT")]
    Format,
    #[serde(rename = "SR-IMAGE")]
    Image,
    #[serde(rename = "SR-ATTRIBUTION")]
    Attribution,
    #[serde(rename = "SR-LOCATION")]
    Location,
    #[serde(rename = "SR-EXPRESSION")]
    Expression,
    #[serde(rename = "SR-INTERACTIVITY")]
    Interactivity,
    #[serde(rename = "SR-ENVELOPE")]
    Envelope,
    #[serde(rename = "SR-MATCH")]
    Match,
    #[serde(rename = "SR-MATCH-OVERLAP")]
    MatchOverlap,
    #[serde(rename = "SR-ORIGIN-PHASE")]
    OriginPhase,
    #[serde(rename = "SR-FIXTURES")]
    Fixtures,
    #[serde(rename = "SR-OVERRIDE")]
    Override,
    #[serde(rename = "SR-MAPPING")]
    Mapping,
}

impl Rule {
    /// Every rule, in declaration order.
    pub const ALL: &'static [Rule] = &[
        Rule::Schema,
        Rule::SchemaBinding,
        Rule::TopLevel,
        Rule::Version,
        Rule::StrictScalars,
        Rule::Roster,
        Rule::Curated,
        Rule::Unique,
        Rule::Ref,
        Rule::Evidence,
        Rule::StateValue,
        Rule::Condition,
        Rule::Applicability,
        Rule::Aggregate,
        Rule::KindUnit,
        Rule::Coverage,
        Rule::Gap,
        Rule::Change,
        Rule::Format,
        Rule::Image,
        Rule::Attribution,
        Rule::Location,
        Rule::Expression,
        Rule::Interactivity,
        Rule::Envelope,
        Rule::Match,
        Rule::MatchOverlap,
        Rule::OriginPhase,
        Rule::Fixtures,
        Rule::Override,
        Rule::Mapping,
    ];

    /// The documented code, e.g. `SR-UNIQUE`.
    pub fn code(self) -> &'static str {
        match self {
            Rule::Schema => "SCHEMA",
            Rule::SchemaBinding => "SR-SCHEMA-BINDING",
            Rule::TopLevel => "SR-TOP-LEVEL",
            Rule::Version => "SR-VERSION",
            Rule::StrictScalars => "SR-STRICT-SCALARS",
            Rule::Roster => "SR-ROSTER",
            Rule::Curated => "SR-CURATED",
            Rule::Unique => "SR-UNIQUE",
            Rule::Ref => "SR-REF",
            Rule::Evidence => "SR-EVIDENCE",
            Rule::StateValue => "SR-STATE-VALUE",
            Rule::Condition => "SR-CONDITION",
            Rule::Applicability => "SR-APPLICABILITY",
            Rule::Aggregate => "SR-AGGREGATE",
            Rule::KindUnit => "SR-KIND-UNIT",
            Rule::Coverage => "SR-COVERAGE",
            Rule::Gap => "SR-GAP",
            Rule::Change => "SR-CHANGE",
            Rule::Format => "SR-FORMAT",
            Rule::Image => "SR-IMAGE",
            Rule::Attribution => "SR-ATTRIBUTION",
            Rule::Location => "SR-LOCATION",
            Rule::Expression => "SR-EXPRESSION",
            Rule::Interactivity => "SR-INTERACTIVITY",
            Rule::Envelope => "SR-ENVELOPE",
            Rule::Match => "SR-MATCH",
            Rule::MatchOverlap => "SR-MATCH-OVERLAP",
            Rule::OriginPhase => "SR-ORIGIN-PHASE",
            Rule::Fixtures => "SR-FIXTURES",
            Rule::Override => "SR-OVERRIDE",
            Rule::Mapping => "SR-MAPPING",
        }
    }

    /// Looks a rule up by its documented code.
    pub fn from_code(code: &str) -> Option<Rule> {
        Rule::ALL.iter().copied().find(|rule| rule.code() == code)
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// One validation finding. Field order is the stable sort order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Diagnostic {
    pub path: RepoPath,
    /// JSON Pointer into the file's frontmatter (`""` for the root).
    pub pointer: String,
    pub rule: Rule,
    /// The stable ID the finding concerns, when there is one.
    pub subject: Option<String>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.path, self.rule)?;
        if !self.pointer.is_empty() {
            write!(f, " at {}", self.pointer)?;
        }
        if let Some(subject) = &self.subject {
            write!(f, " ({subject})")?;
        }
        write!(f, ": {}", self.message)
    }
}

/// Collects findings for one file and returns them in stable order.
#[derive(Debug)]
pub(crate) struct Findings {
    path: RepoPath,
    items: Vec<Diagnostic>,
}

impl Findings {
    pub(crate) fn new(path: RepoPath) -> Self {
        Self { path, items: Vec::new() }
    }

    pub(crate) fn push(
        &mut self,
        rule: Rule,
        pointer: impl Into<String>,
        subject: Option<&str>,
        message: impl Into<String>,
    ) {
        self.items.push(Diagnostic {
            path: self.path.clone(),
            pointer: pointer.into(),
            rule,
            subject: subject.map(str::to_string),
            message: message.into(),
        });
    }

    pub(crate) fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.items.extend(diagnostics);
    }

    /// Sorted and deduplicated.
    pub(crate) fn finish(mut self) -> Vec<Diagnostic> {
        sort_diagnostics(&mut self.items);
        self.items
    }
}

/// Sorts into the stable order and removes exact duplicates.
pub fn sort_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.sort();
    diagnostics.dedup();
}
