//! The data values of `md`'s commands, one struct per command.
//!
//! Each file under `darkmatter/cli/schemas/output/` defines one of these
//! shapes, and a conformance test keeps the two honest. `outcome` and
//! `diagnostics` are not fields here: the CLI's run delivery adds them, so a
//! command cannot report a value that disagrees with its exit status.

use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use crate::markdown::dsl::CodeBlockMeta;
use crate::markdown::hash::{
    ComparisonDetail, ComputedHash, HashComparison, IgnorePolicyAdvisory, MdHashKind, StoredHash, StoredHashValue,
};
use crate::markdown::schemas::{
    InvalidSchemasDir, InvalidTriggerFinding, SchemaRootKind, SchemaRootState, SchemaSearchRoot, TriggerTrace,
};

/// `md set`.
#[derive(Debug, Clone, Serialize)]
pub struct SetData {
    /// The file as given; null for stdin.
    pub file: Option<String>,
    /// The property set.
    pub property: String,
    /// The value it now holds.
    pub value: Value,
    /// The resulting Markdown, with or without `--save`.
    pub document: Option<String>,
    /// Whether the file was written.
    pub saved: bool,
}

/// `md rm`: today's `--json` value.
#[derive(Debug, Clone, Serialize)]
pub struct RemovedData {
    /// The properties removed.
    pub removed: Vec<String>,
    /// The properties left.
    pub remaining: Vec<String>,
    /// The file written.
    pub filename: String,
}

/// `md edit`.
#[derive(Debug, Clone, Serialize)]
pub struct EditData {
    /// The resolved absolute path; null when it could not be resolved.
    pub file: Option<String>,
    /// The selected editor; null when none was launched.
    pub editor: Option<String>,
    /// Whether `md edit` created the file.
    pub created: bool,
}

/// Which `md hash` operation ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HashMode {
    /// Print the document's hash.
    Compute,
    /// Compare the document with its stored hash.
    Diff,
    /// Write the hash back into the document.
    Save,
    /// Aggregate a directory's documents.
    Directory,
}

/// A hash and the kind it was computed for. A `detailed` hash's `value` is its
/// nested object; every other kind's is the flat string it is stored as.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HashValueData {
    /// The hash kind.
    pub kind: MdHashKind,
    /// The value.
    pub value: Value,
    /// The extra properties a stored hash recorded as ignored; `None` for a
    /// computed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored: Option<Vec<String>>,
}

impl From<&ComputedHash> for HashValueData {
    fn from(computed: &ComputedHash) -> Self {
        let value = match computed {
            ComputedHash::Detailed(detailed) => serde_json::to_value(detailed).expect("a detailed hash is JSON"),
            flat => Value::String(flat.flat_string().expect("every other kind has a flat form")),
        };
        Self { kind: computed.kind(), value, ignored: None }
    }
}

impl From<&StoredHash> for HashValueData {
    fn from(stored: &StoredHash) -> Self {
        let value = match &stored.value {
            StoredHashValue::Flat(flat) => Value::String(flat.clone()),
            StoredHashValue::Detailed(detailed) => serde_json::to_value(detailed).expect("a detailed hash is JSON"),
        };
        Self { kind: stored.kind, value, ignored: Some(stored.ignored.clone()) }
    }
}

/// The stored ignore-set's difference from the current one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IgnorePolicyData {
    /// The extra properties the current options ignore.
    pub now_ignoring: Vec<String>,
    /// The extra properties the stored hash recorded.
    pub previously: Vec<String>,
}

impl From<&IgnorePolicyAdvisory> for IgnorePolicyData {
    fn from(advisory: &IgnorePolicyAdvisory) -> Self {
        Self { now_ignoring: advisory.now_ignoring.clone(), previously: advisory.previously.clone() }
    }
}

/// A document compared with its stored hash.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparisonData {
    /// The kind the comparison ran at: the stored hash's.
    pub kind: MdHashKind,
    /// Whether the frontmatter changed.
    pub frontmatter_changed: bool,
    /// Whether the body changed.
    pub body_changed: bool,
    /// The finer detail the kind supports, tagged by `level`.
    pub detail: Value,
    /// A changed ignore-set, which never counts as a content change.
    pub ignore_policy: Option<IgnorePolicyData>,
}

impl From<&HashComparison> for ComparisonData {
    fn from(comparison: &HashComparison) -> Self {
        let detail = match comparison.detail {
            ComparisonDetail::Coarse => serde_json::json!({ "level": "coarse" }),
            ComparisonDetail::Structured { fm_keys_changed, body_structure_changed } => serde_json::json!({
                "level": "structured",
                "fm_keys_changed": fm_keys_changed,
                "body_structure_changed": body_structure_changed,
            }),
            ComparisonDetail::Detailed { fm_keys_changed, body_structure_changed, preamble_changed, sections_changed } => {
                serde_json::json!({
                    "level": "detailed",
                    "fm_keys_changed": fm_keys_changed,
                    "body_structure_changed": body_structure_changed,
                    "preamble_changed": preamble_changed,
                    "sections_changed": sections_changed,
                })
            }
        };
        Self {
            kind: comparison.kind,
            frontmatter_changed: comparison.frontmatter_changed,
            body_changed: comparison.body_changed,
            detail,
            ignore_policy: comparison.ignore_policy.as_ref().map(IgnorePolicyData::from),
        }
    }
}

/// `md hash`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HashData {
    /// The operation.
    pub mode: HashMode,
    /// The file as given; null for stdin.
    pub file: Option<String>,
    /// The kind computed, when one was.
    pub kind: Option<MdHashKind>,
    /// The computed hash.
    pub computed: Option<HashValueData>,
    /// The hash stored in the document.
    pub stored: Option<HashValueData>,
    /// The comparison, for `--diff` against a stored hash.
    pub comparison: Option<ComparisonData>,
    /// Whether the file was written.
    pub saved: bool,
}

/// An inclusive range of highlighted lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CodeRange {
    /// The first line.
    pub start: usize,
    /// The last line.
    pub end: usize,
}

/// `md code-block`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodeBlockData {
    /// The file the code came from; null for literal content.
    pub file: Option<String>,
    /// The code.
    pub code: Option<String>,
    /// The resolved language.
    pub language: Option<String>,
    /// The block's title.
    pub title: Option<String>,
    /// Whether lines are numbered.
    pub line_numbering: bool,
    /// The highlighted lines, in authored order.
    pub highlight: Vec<CodeRange>,
}

impl CodeBlockData {
    /// The value for `code` under `meta`.
    #[must_use]
    pub fn new(file: Option<String>, code: &str, language: Option<String>, meta: &CodeBlockMeta) -> Self {
        Self {
            file,
            code: Some(code.to_string()),
            language,
            title: meta.title.clone(),
            line_numbering: meta.line_numbering,
            highlight: meta.highlight.iter().map(|range| CodeRange { start: range.start(), end: range.end() }).collect(),
        }
    }
}

/// One schema root in `md schema triggers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerRootData {
    /// Which root: its label.
    pub kind: &'static str,
    /// `searched`, `absent`, `duplicate`, `not_applicable`, or `invalid`.
    pub state: &'static str,
    /// Its folder, when it has one.
    pub path: Option<PathBuf>,
    /// The earlier root searching the same folder.
    pub duplicate_of: Option<&'static str>,
    /// The raw value of an invalid root.
    pub value: Option<String>,
    /// Why the root is invalid.
    pub reason: Option<&'static str>,
}

impl From<&SchemaSearchRoot> for TriggerRootData {
    fn from(root: &SchemaSearchRoot) -> Self {
        let mut row = Self {
            kind: root.kind.label(),
            state: "not_applicable",
            path: None,
            duplicate_of: None,
            value: None,
            reason: None,
        };
        match &root.state {
            SchemaRootState::Searched(path) => {
                row.state = "searched";
                row.path = Some(path.clone());
            }
            SchemaRootState::Absent(path) => {
                row.state = "absent";
                row.path = Some(path.clone());
            }
            SchemaRootState::Duplicate { path, of } => {
                row.state = "duplicate";
                row.path = Some(path.clone());
                row.duplicate_of = Some(SchemaRootKind::label(*of));
            }
            SchemaRootState::NotApplicable => {}
            SchemaRootState::Invalid { value, reason } => {
                row.state = "invalid";
                row.value = Some(value.clone());
                row.reason = Some(match reason {
                    InvalidSchemasDir::Empty => "empty",
                    InvalidSchemasDir::Relative => "relative",
                });
            }
        }
        row
    }
}

/// An envelope a same-named earlier one shadows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerShadowData {
    /// The shadowed file.
    pub path: PathBuf,
    /// The file that wins.
    pub winner: PathBuf,
}

/// A trigger discovery skipped as invalid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerFindingData {
    /// Its identity.
    pub source: String,
    /// The stable code.
    pub code: String,
    /// Why it was skipped.
    pub message: String,
}

/// One trigger condition's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerConditionData {
    /// The zero-based authored index.
    pub index: usize,
    /// Whether it matched.
    pub matched: bool,
    /// What defeated it.
    pub defeat: Option<String>,
}

/// One trigger's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerTraceData {
    /// Its identity.
    pub source: String,
    /// Whether it matched.
    pub matched: bool,
    /// Its conditions, in authored order.
    pub conditions: Vec<TriggerConditionData>,
}

/// `md schema triggers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggersData {
    /// The file as given.
    pub file: Option<String>,
    /// The five roots, in search order.
    pub roots: Vec<TriggerRootData>,
    /// Shadowed envelopes.
    pub shadowed: Vec<TriggerShadowData>,
    /// Triggers skipped as invalid.
    pub skipped: Vec<TriggerFindingData>,
    /// Every discovered trigger.
    pub triggers: Vec<TriggerTraceData>,
}

impl TriggersData {
    /// The value for a trace of `file`.
    #[must_use]
    pub fn new(file: String, trace: &TriggerTrace, findings: &[InvalidTriggerFinding]) -> Self {
        Self {
            file: Some(file),
            roots: trace.roots.entries().iter().map(TriggerRootData::from).collect(),
            shadowed: trace
                .shadowed
                .iter()
                .map(|(path, winner)| TriggerShadowData { path: path.clone(), winner: winner.clone() })
                .collect(),
            skipped: findings
                .iter()
                .map(|finding| TriggerFindingData {
                    source: finding.source().to_string(),
                    code: finding.code().to_string(),
                    message: finding.message(),
                })
                .collect(),
            triggers: trace
                .triggers
                .iter()
                .map(|trigger| TriggerTraceData {
                    source: trigger.source.to_string(),
                    matched: trigger.matched,
                    conditions: trigger
                        .conditions
                        .iter()
                        .map(|condition| TriggerConditionData {
                            index: condition.index,
                            matched: condition.matched,
                            defeat: condition.defeat.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// `md schema validate`: today's per-file records, in input order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidateData {
    /// One record per file argument.
    pub files: Vec<Value>,
}

/// One input's result in a multi-file `md schema detect`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DetectedFile {
    /// The file as given.
    pub file: String,
    /// The detected schema.
    pub schema: Value,
}

/// `md schema detect`: one schema, or one per input.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DetectData {
    /// One input, or `--merge`: the schema itself.
    Single(Value),
    /// Several inputs, in input order.
    Files {
        /// One entry per input.
        files: Vec<DetectedFile>,
    },
}

/// A diagram's notation and source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphNotation {
    /// `mermaid` or `dot`.
    pub format: &'static str,
    /// The diagram source.
    pub source: String,
}

/// `md validate refs --graph`: the diagram, never mixed with a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphNotationData {
    /// The diagram.
    pub graph: GraphNotation,
}
