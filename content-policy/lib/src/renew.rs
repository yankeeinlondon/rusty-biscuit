//! Renewal: record a content update by advancing every renewable baseline.
//!
//! [`plan_renewal`] decides every edit before anything is written and returns
//! a [`RenewalPlan`] a caller can preview. [`RenewalPlan::apply_to`] and
//! [`apply_renewal`] refuse bytes that changed since planning, apply the
//! edits, and re-read the result as a safety net: when anything other than the
//! target values (and a listed tab repair) would change, nothing is written.
//!
//! Edits are byte spans in the original document, so every byte outside the
//! edited values is preserved: comments, quoting, line endings, and the body.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use biscuit_file::yaml::{
    YamlPathSegment, YamlRepair, YamlValueLocation, apply_edit_set, locate_yaml_key,
    locate_yaml_value,
};
use chrono::{NaiveDate, Utc};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::diagnostic::Invalid;
use crate::evaluate::{
    EvaluationContext, NaiveDateText, PolicySummary, duplicate_key_invalid, evaluate_record,
};
use crate::model::{Baseline, Deadline, EvidenceRecord, Policy, PolicyOptions, Rule};
use crate::reader::{Frontmatter, LineEnding, ReadError, ReadOutcome, read_frontmatter};
use crate::time::start_of_day;

/// Everything a renewal needs besides the document.
#[derive(Debug, Clone)]
pub struct RenewalContext {
    today: NaiveDate,
    on: Option<NaiveDate>,
    options: PolicyOptions,
    document: Option<String>,
}

impl RenewalContext {
    /// A context whose current UTC date is `today`, with the built-in
    /// options. The update date defaults to `today`.
    #[must_use]
    pub fn new(today: NaiveDate) -> Self {
        Self {
            today,
            on: None,
            options: PolicyOptions::default(),
            document: None,
        }
    }

    /// A context for the current UTC date from the system clock.
    #[must_use]
    pub fn now() -> Self {
        Self::new(Utc::now().date_naive())
    }

    /// Records the update as made on `date`, such as the day regeneration
    /// ran. A date after today is rejected when planning.
    #[must_use]
    pub fn on(mut self, date: NaiveDate) -> Self {
        self.on = Some(date);
        self
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn with_options(mut self, options: PolicyOptions) -> Self {
        self.options = options;
        self
    }

    /// Labels the plan and its errors with the document identity as the
    /// caller spells it. It is never canonicalized.
    #[must_use]
    pub fn with_document(mut self, label: impl Into<String>) -> Self {
        self.document = Some(label.into());
        self
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn today(&self) -> NaiveDate {
        self.today
    }

    /// The date written into every renewed baseline.
    #[must_use]
    pub fn update_date(&self) -> NaiveDate {
        self.on.unwrap_or(self.today)
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn options(&self) -> &PolicyOptions {
        &self.options
    }
}

/// The baseline a [`BaselineChange`] writes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BaselineTarget {
    /// The date inside the rule string of entry `entry`,
    /// `ValidFor(3mo, 2026-09-28)`.
    Inline { entry: usize },
    /// A top-level frontmatter property, such as `last_updated`.
    Property { name: String },
}

/// How a renewal changes one baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// A recorded date advances to the update date.
    Renewed,
    /// First capture: the property was absent or `null`.
    NewBaseline,
    /// The baseline already holds the update date; no bytes change.
    Unchanged,
}

/// One baseline a renewal writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaselineChange {
    pub target: BaselineTarget,
    /// Zero-based policy entries this baseline serves. A property shared by
    /// several rules is written once and lists every one.
    pub entries: Vec<usize>,
    pub kind: ChangeKind,
    /// The date recorded before renewal; `None` for a new baseline.
    pub previous: Option<NaiveDateText>,
    pub value: NaiveDateText,
}

/// A byte-range replacement in the whole document. An empty span is an
/// insertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextEdit {
    pub span: Range<usize>,
    pub replacement: String,
}

/// A planned renewal: what changes, and the exact edits that change it.
///
/// A plan with no [`changes`](Self::changes) means the policy has nothing to
/// renew (`Evergreen`, `TimeSensitive`, or only `ValidUntil` rules); applying
/// it writes nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenewalPlan {
    pub document: Option<String>,
    pub update_date: NaiveDateText,
    /// `xxh64:` plus 16 lowercase hex digits of the bytes the plan was made
    /// from. Applying refuses any other bytes. It is never stored in the
    /// document.
    pub fingerprint: String,
    pub policy: PolicySummary,
    pub changes: Vec<BaselineChange>,
    /// The edits that make [`changes`](Self::changes), in document offsets.
    pub edits: Vec<TextEdit>,
    /// The tab-indentation repair, one edit per tab-indented line, applied
    /// together with [`edits`](Self::edits). Listed separately because it is
    /// the one change outside the baseline values.
    pub tab_repair: Vec<TextEdit>,
    #[serde(skip)]
    key: String,
}

/// Why a renewal target cannot be edited in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    /// The value sits inside a one-line `[...]` policy list.
    FlowList,
    /// The value is a block scalar or spans more than one line.
    MultiLineValue,
    /// A double-quoted string containing escape sequences.
    EscapedString,
    /// The value, or the policy list holding it, carries an anchor, alias,
    /// or tag; editing it would change its aliases too.
    AnchorAliasOrTag,
    /// The value sits inside a `{rule, action}` flow mapping.
    FlowMapping,
    /// The frontmatter block has no closing `---` (including one closed by
    /// `...`).
    UnterminatedBlock,
    /// The first line is a near-miss fence such as `----`.
    NearMissFence,
    /// The located source text does not decode to the parsed value.
    SpanMismatch,
    /// The value cannot be found in the frontmatter source.
    NotLocatable,
    /// An inline baseline in the caller's default policy, which the document
    /// does not contain.
    NotInDocument,
}

/// A renewal target the planner will not edit, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refusal {
    pub reason: RefusalReason,
    /// `None` when the whole frontmatter block is refused.
    pub target: Option<BaselineTarget>,
    pub message: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Why two planned writes cannot both happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Two different values for one property.
    DifferentWrites,
    /// The property is also a `ValidUntil` deadline, which renewal never
    /// moves.
    MovesDeadline,
}

/// Planned writes that contradict each other or a nonrenewable rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Conflict {
    pub kind: ConflictKind,
    pub property: String,
    /// Every entry involved, zero-based.
    pub entries: Vec<usize>,
    pub message: String,
}

/// Why a renewal could not be planned or applied. Nothing is written in any
/// of these cases.
#[derive(Debug)]
pub enum RenewalError {
    /// The document's frontmatter could not be read.
    Read { document: Option<String>, error: ReadError },
    /// The declaration or an evidence value is invalid.
    Invalid(Invalid),
    /// The update date is after today's UTC date.
    FutureDate {
        document: Option<String>,
        on: NaiveDate,
        today: NaiveDate,
    },
    /// Some target has a source shape renewal will not edit.
    Refused {
        document: Option<String>,
        refusals: Vec<Refusal>,
    },
    Conflict {
        document: Option<String>,
        conflicts: Vec<Conflict>,
    },
    /// The document's bytes differ from the ones the plan was made from.
    ModifiedSincePlan { document: Option<String> },
    /// The edited text would change something other than its targets.
    SafetyNet {
        document: Option<String>,
        detail: String,
    },
    Io { path: PathBuf, error: io::Error },
}

impl fmt::Display for RenewalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = |f: &mut fmt::Formatter<'_>, document: &Option<String>| match document {
            Some(document) => write!(f, "{document}: "),
            None => Ok(()),
        };
        match self {
            Self::Read { document, error } => {
                prefix(f, document)?;
                write!(f, "{error}")
            }
            Self::Invalid(invalid) => write!(f, "{invalid}"),
            Self::FutureDate { document, on, today } => {
                prefix(f, document)?;
                write!(
                    f,
                    "the update date {on} is after today's UTC date {today}; a future baseline \
                     would evaluate as inconsistent"
                )
            }
            Self::Refused { document, refusals } => {
                prefix(f, document)?;
                f.write_str("renewal refused; nothing was written")?;
                for refusal in refusals {
                    write!(f, "\n  {refusal}")?;
                }
                Ok(())
            }
            Self::Conflict { document, conflicts } => {
                prefix(f, document)?;
                f.write_str("renewal has conflicting writes; nothing was written")?;
                for conflict in conflicts {
                    write!(f, "\n  {}", conflict.message)?;
                }
                Ok(())
            }
            Self::ModifiedSincePlan { document } => {
                prefix(f, document)?;
                f.write_str(
                    "the document changed after the renewal was planned; nothing was written",
                )
            }
            Self::SafetyNet { document, detail } => {
                prefix(f, document)?;
                write!(f, "renewal stopped by its safety net; nothing was written: {detail}")
            }
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for RenewalError {}

/// The plan fingerprint of `bytes`: `xxh64:` plus 16 lowercase hex digits.
#[must_use]
pub fn plan_fingerprint(bytes: &[u8]) -> String {
    format!("xxh64:{:016x}", biscuit_hash::xx_hash_bytes(bytes))
}

/// Plans the renewal of every renewable entry in a document's policy, or in
/// the caller's default policy when the document declares none.
///
/// Nothing is written. Missing baseline properties, and a missing
/// frontmatter block, become first captures.
///
/// ## Errors
///
/// Returns [`RenewalError`] for a future update date, unreadable or invalid
/// frontmatter, a present baseline value that is not a date, conflicting
/// writes, or a source shape renewal will not edit. Every refusal and
/// conflict is listed, not just the first.
pub fn plan_renewal(bytes: &[u8], context: &RenewalContext) -> Result<RenewalPlan, RenewalError> {
    let document = context.document.clone();
    let update = context.update_date();
    if update > context.today {
        return Err(RenewalError::FutureDate {
            document,
            on: update,
            today: context.today,
        });
    }
    let frontmatter = match read_frontmatter(bytes) {
        Ok(ReadOutcome::NoFrontmatter) => None,
        Ok(ReadOutcome::Found(frontmatter)) => Some(frontmatter),
        Err(ReadError::DuplicateKey { key, message }) => {
            return Err(RenewalError::Invalid(duplicate_key_invalid(key, &message, document)));
        }
        Err(error @ (ReadError::Unterminated { .. } | ReadError::NearMissFence)) => {
            let reason = if error == ReadError::NearMissFence {
                RefusalReason::NearMissFence
            } else {
                RefusalReason::UnterminatedBlock
            };
            let refusal = Refusal {
                reason,
                target: None,
                message: format!("{error}; renewal never creates a second block in front of it"),
            };
            return Err(RenewalError::Refused {
                document,
                refusals: vec![refusal],
            });
        }
        Err(error) => return Err(RenewalError::Read { document, error }),
    };
    let record = frontmatter
        .as_ref()
        .map(|frontmatter| frontmatter.record().clone())
        .unwrap_or_default();

    // Evaluating at the update date validates the declaration and every
    // evidence value, so a present malformed baseline is an error here and
    // renewal never repairs it.
    let mut evaluation =
        EvaluationContext::new(start_of_day(update)).with_options(context.options.clone());
    if let Some(label) = &document {
        evaluation = evaluation.with_document(label.clone());
    }
    let report = evaluate_record(&record, &evaluation).map_err(RenewalError::Invalid)?;
    let key = context.options.key();
    let declaration = record.get(key);
    let policy = match declaration {
        Some(value) => Policy::from_declaration(value).map_err(RenewalError::Invalid)?,
        None => context.options.default_policy().clone(),
    };

    let value = update.to_string();
    let mut refusals = Vec::new();
    let mut writes = Vec::new();
    for (index, entry) in policy.entries().iter().enumerate() {
        let Rule::ValidFor { baseline, .. } = &entry.rule else {
            continue;
        };
        let target = match baseline {
            Baseline::Inline(_) if declaration.is_none() => {
                refusals.push(Refusal {
                    reason: RefusalReason::NotInDocument,
                    target: Some(BaselineTarget::Inline { entry: index }),
                    message: format!(
                        "entry {} of the default policy, `{}`, has an inline baseline that is \
                         not written in the document; declare the policy in the document to \
                         renew it",
                        index + 1,
                        entry.rule
                    ),
                });
                continue;
            }
            Baseline::Inline(_) => BaselineTarget::Inline { entry: index },
            Baseline::Reference(name) => BaselineTarget::Property { name: name.clone() },
            Baseline::Defaulted => BaselineTarget::Property {
                name: context.options.date_property().to_string(),
            },
        };
        writes.push(Write {
            target,
            entry: index,
            value: value.clone(),
        });
    }

    let (groups, mut conflicts) = consolidate(writes);
    conflicts.extend(deadline_conflicts(&policy, &groups));
    if !conflicts.is_empty() {
        return Err(RenewalError::Conflict {
            document,
            conflicts,
        });
    }

    let source = std::str::from_utf8(bytes).expect("the reader accepted the bytes as UTF-8");
    let yaml = frontmatter.as_ref().map(|frontmatter| Yaml::new(source, frontmatter));
    let mut changes = Vec::new();
    let mut edits = Vec::new();
    let mut appended = Vec::new();
    for group in &groups {
        let planned = match &group.target {
            BaselineTarget::Property { name } => {
                plan_property(yaml.as_ref(), name, record.get(name), &group.value)
            }
            BaselineTarget::Inline { entry } => {
                let yaml = yaml.as_ref().expect("an inline baseline is declared in frontmatter");
                let item = declaration
                    .and_then(|declaration| declaration.get(*entry))
                    .expect("the parsed policy has this entry");
                plan_inline(yaml, key, *entry, item, &group.value)
            }
        };
        let (kind, previous, edit) = match planned {
            Ok(planned) => planned,
            Err(refusal) => {
                refusals.push(refusal);
                continue;
            }
        };
        match edit {
            Planned::Edit(edit) => edits.push(edit),
            Planned::Append => appended.push(group),
            Planned::Nothing => {}
        }
        changes.push(BaselineChange {
            target: group.target.clone(),
            entries: group.entries.clone(),
            kind,
            previous: previous.map(NaiveDateText),
            value: NaiveDateText(update),
        });
    }
    if !refusals.is_empty() {
        return Err(RenewalError::Refused {
            document,
            refusals,
        });
    }
    if !appended.is_empty() {
        edits.push(append_properties(source, frontmatter.as_ref(), yaml.as_ref(), &appended));
    }
    edits.sort_by_key(|edit| edit.span.start);
    let tab_repair = match (&frontmatter, changes.is_empty()) {
        (Some(frontmatter), false) => frontmatter
            .tab_repair()
            .iter()
            .map(|repair| TextEdit {
                span: repair.span.clone(),
                replacement: repair.replacement.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };

    let plan = RenewalPlan {
        document,
        update_date: NaiveDateText(update),
        fingerprint: plan_fingerprint(bytes),
        policy: report.policy,
        changes,
        edits,
        tab_repair,
        key: key.to_string(),
    };
    // Run the safety net at planning time too, so a preview never shows an
    // edit that `apply` would refuse.
    plan.render(bytes)?;
    Ok(plan)
}

/// Applies a plan to the file at `path` through an atomic replace: the file
/// is re-read, its bytes must match the plan's fingerprint, and the edited
/// text must pass the safety net. A plan with nothing to renew writes
/// nothing.
///
/// ## Errors
///
/// Returns [`RenewalError::ModifiedSincePlan`] when the file changed after
/// planning, [`RenewalError::SafetyNet`] when the edits would change anything
/// but their targets, and [`RenewalError::Io`] for read or write failures.
/// The file is unchanged in every case.
pub fn apply_renewal(path: &Path, plan: &RenewalPlan) -> Result<(), RenewalError> {
    let io_error = |error| RenewalError::Io {
        path: path.to_path_buf(),
        error,
    };
    let bytes = fs::read(path).map_err(io_error)?;
    let edited = plan.apply_to(&bytes)?;
    if edited != bytes {
        write_atomic(path, &edited).map_err(io_error)?;
    }
    Ok(())
}

impl RenewalPlan {
    /// `true` when the policy has no renewable entry.
    #[must_use]
    pub fn nothing_to_renew(&self) -> bool {
        self.changes.is_empty()
    }

    /// Returns the renewed document for `bytes`, which must be the bytes the
    /// plan was made from.
    ///
    /// ## Errors
    ///
    /// Returns [`RenewalError::ModifiedSincePlan`] for any other bytes, and
    /// [`RenewalError::SafetyNet`] when the edited text would change anything
    /// but the planned values and the listed tab repair.
    pub fn apply_to(&self, bytes: &[u8]) -> Result<Vec<u8>, RenewalError> {
        if plan_fingerprint(bytes) != self.fingerprint {
            return Err(RenewalError::ModifiedSincePlan {
                document: self.document.clone(),
            });
        }
        self.render(bytes)
    }

    fn render(&self, bytes: &[u8]) -> Result<Vec<u8>, RenewalError> {
        if self.nothing_to_renew() {
            return Ok(bytes.to_vec());
        }
        let source = std::str::from_utf8(bytes).map_err(|_| self.safety("the document is not UTF-8"))?;
        let repairs: Vec<YamlRepair> = self
            .edits
            .iter()
            .chain(&self.tab_repair)
            .map(|edit| YamlRepair {
                span: edit.span.clone(),
                replacement: edit.replacement.clone(),
                explanation: String::new(),
            })
            .collect();
        let outcome = apply_edit_set(source, &repairs);
        if !outcome.audit.rejected.is_empty() {
            return Err(self.safety("an edit overlaps another edit or falls outside the document"));
        }
        self.verify(source, &outcome.source)?;
        Ok(outcome.source.into_bytes())
    }

    fn safety(&self, detail: impl Into<String>) -> RenewalError {
        RenewalError::SafetyNet {
            document: self.document.clone(),
            detail: detail.into(),
        }
    }

    /// Re-reads the edited text: it must parse without a repair, keep every
    /// byte outside the frontmatter block, and hold exactly the original
    /// record with the planned values written.
    fn verify(&self, before: &str, after: &str) -> Result<(), RenewalError> {
        let (record, block) = match read_frontmatter(before.as_bytes()) {
            Ok(ReadOutcome::Found(frontmatter)) => {
                (frontmatter.record().clone(), frontmatter.block())
            }
            Ok(ReadOutcome::NoFrontmatter) => {
                let bom = bom_len(before);
                (EvidenceRecord::new(), bom..bom)
            }
            Err(error) => return Err(self.safety(format!("the original does not read: {error}"))),
        };
        let edited = match read_frontmatter(after.as_bytes()) {
            Ok(ReadOutcome::Found(frontmatter)) => frontmatter,
            Ok(ReadOutcome::NoFrontmatter) => {
                return Err(self.safety("the edited document has no frontmatter"));
            }
            Err(error) => {
                return Err(self.safety(format!("the edited frontmatter does not read: {error}")));
            }
        };
        if !edited.tab_repair().is_empty() {
            return Err(self.safety("the edited frontmatter still needs the tab repair"));
        }
        if before[..block.start] != after[..edited.block().start]
            || before[block.end..] != after[edited.block().end..]
        {
            return Err(self.safety("bytes outside the frontmatter block would change"));
        }
        let expected = self.expected(record.into_map())?;
        let actual = edited.record().as_map();
        if actual != &expected {
            let mut changed: Vec<&str> = expected
                .iter()
                .filter(|(name, value)| actual.get(*name) != Some(value))
                .map(|(name, _)| name.as_str())
                .chain(actual.keys().filter(|name| !expected.contains_key(*name)).map(String::as_str))
                .collect();
            changed.dedup();
            return Err(self.safety(format!(
                "the edit would change {} beyond the planned values",
                changed.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(())
    }

    /// The record the edited document must read as.
    fn expected(&self, mut record: Map<String, Value>) -> Result<Map<String, Value>, RenewalError> {
        for change in &self.changes {
            let value = change.value.0.to_string();
            match &change.target {
                BaselineTarget::Property { name } => {
                    record.insert(name.clone(), Value::String(value));
                }
                BaselineTarget::Inline { entry } => {
                    let previous = change.previous.map(|date| date.0.to_string());
                    let rule = record
                        .get_mut(&self.key)
                        .and_then(|policy| policy.get_mut(*entry))
                        .and_then(|item| match item {
                            Value::Object(map) => map.get_mut("rule"),
                            other => Some(other),
                        });
                    let renewed = match (rule, previous) {
                        (Some(Value::String(rule)), Some(previous)) => rule
                            .rfind(&previous)
                            .map(|at| rule.replace_range(at..at + previous.len(), &value)),
                        _ => None,
                    };
                    if renewed.is_none() {
                        return Err(self.safety(format!(
                            "entry {} has no inline date to renew",
                            entry + 1
                        )));
                    }
                }
            }
        }
        Ok(record)
    }
}

struct Write {
    target: BaselineTarget,
    entry: usize,
    value: String,
}

struct Group {
    target: BaselineTarget,
    entries: Vec<usize>,
    value: String,
}

/// Merges writes to one target into a single write listing every entry.
/// Differing values for one property are a conflict.
fn consolidate(writes: Vec<Write>) -> (Vec<Group>, Vec<Conflict>) {
    let mut groups: Vec<Group> = Vec::new();
    let mut differing: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for write in writes {
        match groups.iter_mut().find(|group| group.target == write.target) {
            Some(group) => {
                if group.value != write.value
                    && let BaselineTarget::Property { name } = &write.target
                {
                    differing
                        .entry(name.clone())
                        .or_insert_with(|| group.entries.clone())
                        .push(write.entry);
                }
                group.entries.push(write.entry);
            }
            None => groups.push(Group {
                target: write.target,
                entries: vec![write.entry],
                value: write.value,
            }),
        }
    }
    let conflicts = differing
        .into_iter()
        .map(|(property, entries)| Conflict {
            kind: ConflictKind::DifferentWrites,
            message: format!(
                "`{property}` would receive different values from entries {}",
                entry_list(&entries)
            ),
            property,
            entries,
        })
        .collect();
    (groups, conflicts)
}

/// A renewed property that is also a `ValidUntil` deadline would silently
/// extend that deadline.
fn deadline_conflicts(policy: &Policy, groups: &[Group]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();
    for group in groups {
        let BaselineTarget::Property { name } = &group.target else {
            continue;
        };
        let deadlines: Vec<usize> = policy
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                matches!(&entry.rule, Rule::ValidUntil { deadline: Deadline::Reference(deadline) } if deadline == name.as_str())
            })
            .map(|(index, _)| index)
            .collect();
        if deadlines.is_empty() {
            continue;
        }
        let mut entries = group.entries.clone();
        entries.extend(&deadlines);
        entries.sort_unstable();
        conflicts.push(Conflict {
            kind: ConflictKind::MovesDeadline,
            message: format!(
                "renewing `{name}` for entries {} would also move the `ValidUntil` deadline of \
                 entries {}, which renewal never extends; give the deadline its own property",
                entry_list(&group.entries),
                entry_list(&deadlines)
            ),
            property: name.clone(),
            entries,
        });
    }
    conflicts
}

fn entry_list(entries: &[usize]) -> String {
    entries
        .iter()
        .map(|entry| (entry + 1).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

enum Planned {
    Edit(TextEdit),
    /// A new property line appended to the block (or a new block).
    Append,
    Nothing,
}

type PlannedChange = (ChangeKind, Option<NaiveDate>, Planned);

/// The frontmatter YAML as renewal locates values in it: the tab-repaired
/// text when the reader repaired tabs, since the locator works on valid YAML.
/// Offsets found in it map back to the document through the repair.
struct Yaml {
    text: String,
    start: usize,
    /// The tab repair in YAML-local offsets, ordered by position.
    repairs: Vec<YamlRepair>,
}

impl Yaml {
    fn new(source: &str, frontmatter: &Frontmatter) -> Self {
        let range = frontmatter.yaml();
        let start = range.start;
        let original = &source[range];
        let mut repairs: Vec<YamlRepair> = frontmatter
            .tab_repair()
            .iter()
            .map(|repair| YamlRepair {
                span: repair.span.start - start..repair.span.end - start,
                ..repair.clone()
            })
            .collect();
        repairs.sort_by_key(|repair| repair.span.start);
        let text = if repairs.is_empty() {
            original.to_string()
        } else {
            apply_edit_set(original, &repairs).source
        };
        Self {
            text,
            start,
            repairs,
        }
    }

    /// Maps an offset in the located text to a document offset. Targets are
    /// never inside a repaired (leading whitespace) region.
    fn to_document(&self, offset: usize) -> usize {
        let mut shift: isize = 0;
        for repair in &self.repairs {
            let repaired_end = repair.span.start.saturating_add_signed(shift) + repair.replacement.len();
            if repaired_end > offset {
                break;
            }
            shift += repair.replacement.len() as isize - repair.span.len() as isize;
        }
        self.start + offset.saturating_add_signed(-shift)
    }

    fn edit(&self, span: Range<usize>, replacement: impl Into<String>) -> TextEdit {
        TextEdit {
            span: self.to_document(span.start)..self.to_document(span.end),
            replacement: replacement.into(),
        }
    }

    fn locate(&self, path: &[YamlPathSegment]) -> Option<YamlValueLocation> {
        locate_yaml_value(&self.text, path)
    }

    /// The offset just after the `:` of top-level key `name`.
    fn after_colon(&self, name: &str) -> Option<usize> {
        let key = locate_yaml_key(&self.text, &[YamlPathSegment::Key(name.to_string())])?;
        let rest = &self.text[key.end..];
        let gap = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        rest[gap..].starts_with(':').then_some(key.end + gap + 1)
    }

    /// The start of the last top-level entry when that entry holds a block
    /// scalar header without the strip indicator (`|`, `>`, `|+`, `>2`).
    /// The reader parses the block without its final line terminator, so
    /// such a scalar's value changes when any line follows it.
    fn trailing_block_scalar_entry(&self) -> Option<usize> {
        let mut offset = 0;
        let mut entry = None;
        for line in self.text.split_inclusive(['\n', '\r']) {
            if line.starts_with(|c: char| !matches!(c, ' ' | '\t' | '#' | '\n' | '\r')) {
                entry = Some(offset);
            }
            offset += line.len();
        }
        let entry = entry?;
        let keeps_break = self.text[entry..].lines().any(|line| {
            let code = line.find(" #").map_or(line, |comment| &line[..comment]).trim_end();
            let header = code.rsplit(' ').next().unwrap_or("");
            let before = &code[..code.len() - header.len()];
            is_block_scalar_header(header)
                && !header.contains('-')
                && (before.ends_with(": ") || before.ends_with("- "))
        });
        keeps_break.then_some(entry)
    }

    /// The rest of the line after `offset`, without its terminator.
    fn rest_of_line(&self, offset: usize) -> &str {
        let rest = &self.text[offset..];
        &rest[..rest.find(['\n', '\r']).unwrap_or(rest.len())]
    }
}

fn plan_property(
    yaml: Option<&Yaml>,
    name: &str,
    current: Option<&Value>,
    value: &str,
) -> Result<PlannedChange, Refusal> {
    let target = || BaselineTarget::Property {
        name: name.to_string(),
    };
    let refuse = |reason, message: String| Refusal {
        reason,
        target: Some(target()),
        message,
    };
    let Some(yaml) = yaml else {
        return Ok((ChangeKind::NewBaseline, None, Planned::Append));
    };
    let path = [YamlPathSegment::Key(name.to_string())];
    let unlocatable = || {
        if yaml.after_colon(name).is_some() {
            refuse(
                RefusalReason::MultiLineValue,
                format!(
                    "`{name}` is a block scalar or spans more than one line; write it on one \
                     line, such as `{name}: {value}`"
                ),
            )
        } else {
            refuse(
                RefusalReason::NotLocatable,
                format!("renewal cannot find the top-level property `{name}` in the frontmatter source"),
            )
        }
    };
    match current {
        None => Ok((ChangeKind::NewBaseline, None, Planned::Append)),
        Some(Value::Null) => {
            if let Some(location) = yaml.locate(&path) {
                // A `~` or `null` spelling: replace it.
                check_properties(&yaml.text, &location, &format!("`{name}`"))
                    .map_err(|message| refuse(RefusalReason::AnchorAliasOrTag, message))?;
                if !location.plain {
                    return Err(span_mismatch(target(), &yaml.text[location.span], "null"));
                }
                let edit = yaml.edit(location.span, value);
                return Ok((ChangeKind::NewBaseline, None, Planned::Edit(edit)));
            }
            let colon = yaml.after_colon(name).ok_or_else(unlocatable)?;
            let rest = yaml.rest_of_line(colon).trim_start_matches([' ', '\t']);
            if !(rest.is_empty() || rest.starts_with('#')) {
                return Err(unlocatable());
            }
            // `last_updated:` or `last_updated:   # todo`: the date goes
            // right after the colon, before any comment.
            let edit = yaml.edit(colon..colon, format!(" {value}"));
            Ok((ChangeKind::NewBaseline, None, Planned::Edit(edit)))
        }
        Some(Value::String(text)) => {
            let previous = NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .expect("evaluation accepted this baseline as a date");
            let location = yaml.locate(&path).ok_or_else(unlocatable)?;
            check_properties(&yaml.text, &location, &format!("`{name}`"))
                .map_err(|message| refuse(RefusalReason::AnchorAliasOrTag, message))?;
            let authored = &yaml.text[location.span.clone()];
            if is_block_scalar_header(authored) {
                return Err(unlocatable());
            }
            let (inner, offset) = unquote(authored, location.plain).map_err(|reason| match reason {
                RefusalReason::EscapedString => refuse(
                    reason,
                    format!(
                        "`{name}` is a double-quoted string with escape sequences, so its \
                         source bytes do not match its value; write it unquoted, such as \
                         `{name}: {value}`"
                    ),
                ),
                _ => span_mismatch(target(), authored, text),
            })?;
            if inner != text {
                return Err(span_mismatch(target(), authored, text));
            }
            if text == value {
                return Ok((ChangeKind::Unchanged, Some(previous), Planned::Nothing));
            }
            let start = location.span.start + offset;
            let edit = yaml.edit(start..start + inner.len(), value);
            Ok((ChangeKind::Renewed, Some(previous), Planned::Edit(edit)))
        }
        Some(_) => unreachable!("evaluation rejects a non-date baseline"),
    }
}

fn plan_inline(
    yaml: &Yaml,
    key: &str,
    entry: usize,
    item: &Value,
    value: &str,
) -> Result<PlannedChange, Refusal> {
    let target = || BaselineTarget::Inline { entry };
    let refuse = |reason, message: String| Refusal {
        reason,
        target: Some(target()),
        message,
    };
    let (rule, long_form) = match item {
        Value::String(rule) => (rule.as_str(), false),
        Value::Object(map) => (
            map.get("rule").and_then(Value::as_str).expect("a valid entry has a rule string"),
            true,
        ),
        _ => unreachable!("a valid entry is a string or a mapping"),
    };
    let block_list = format!(
        "write the policy as a block list, one entry per line:\n    {key}:\n      - {rule}"
    );
    let number = entry + 1;

    let Some(colon) = yaml.after_colon(key) else {
        return Err(refuse(
            RefusalReason::NotLocatable,
            format!("renewal cannot find the top-level `{key}` key in the frontmatter source"),
        ));
    };
    let policy_value = yaml.rest_of_line(colon).trim_start_matches([' ', '\t']);
    if policy_value.starts_with('[') {
        return Err(refuse(
            RefusalReason::FlowList,
            format!(
                "entry {number}'s date is inside a one-line `[...]` policy list, which renewal \
                 cannot edit in place; {block_list}"
            ),
        ));
    }
    if policy_value.starts_with(['&', '!', '*']) {
        return Err(refuse(
            RefusalReason::AnchorAliasOrTag,
            format!(
                "the `{key}` list carries an anchor, alias, or tag, so editing it would change \
                 every alias of it too; write the list without one"
            ),
        ));
    }

    let entry_path = [YamlPathSegment::Key(key.to_string()), YamlPathSegment::Index(entry)];
    let mut path = entry_path.to_vec();
    if long_form {
        if yaml
            .locate(&entry_path)
            .is_some_and(|location| yaml.text[location.span].starts_with('{'))
        {
            return Err(refuse(
                RefusalReason::FlowMapping,
                format!(
                    "entry {number} is a one-line `{{rule, action}}` mapping, which renewal \
                     cannot edit in place; write it as a block mapping:\n    - rule: {rule}\n      \
                     action: ..."
                ),
            ));
        }
        path.push(YamlPathSegment::Key("rule".to_string()));
    }
    let multi_line = || {
        refuse(
            RefusalReason::MultiLineValue,
            format!(
                "entry {number}'s rule is a block scalar or spans more than one line, which \
                 renewal cannot edit in place; {block_list}"
            ),
        )
    };
    let location = yaml.locate(&path).ok_or_else(multi_line)?;
    if is_block_scalar_header(&yaml.text[location.span.clone()]) {
        return Err(multi_line());
    }
    check_properties(&yaml.text, &location, &format!("entry {number}'s rule"))
        .map_err(|message| refuse(RefusalReason::AnchorAliasOrTag, message))?;
    let authored = &yaml.text[location.span.clone()];
    let (inner, offset) = unquote(authored, location.plain).map_err(|reason| match reason {
        RefusalReason::EscapedString => refuse(
            reason,
            format!(
                "entry {number}'s rule is a double-quoted string with escape sequences, so its \
                 source bytes do not match its value; {block_list}"
            ),
        ),
        _ => span_mismatch(target(), authored, rule),
    })?;
    if inner != rule {
        return Err(span_mismatch(target(), authored, rule));
    }
    let Some((position, previous)) = inline_date(rule) else {
        return Err(span_mismatch(target(), authored, rule));
    };
    if previous.to_string() == value {
        return Ok((ChangeKind::Unchanged, Some(previous), Planned::Nothing));
    }
    let start = location.span.start + offset + position;
    let edit = yaml.edit(start..start + value.len(), value);
    Ok((ChangeKind::Renewed, Some(previous), Planned::Edit(edit)))
}

/// Refuses a located value that carries an anchor, tag, or alias.
fn check_properties(text: &str, location: &YamlValueLocation, what: &str) -> Result<(), String> {
    let properties = &location.properties;
    if properties.is_empty() {
        return Ok(());
    }
    let token = [&properties.anchor, &properties.tag, &properties.alias]
        .into_iter()
        .flatten()
        .map(|span| &text[span.clone()])
        .collect::<Vec<_>>()
        .join(" ");
    Err(format!(
        "{what} carries an anchor, alias, or tag (`{token}`); editing it would change every alias \
         of it too, so renewal will not; write the plain value instead"
    ))
}

/// `true` for a block scalar header such as `|`, `>-`, or `|2+`. The locator
/// can report the header of a block-scalar sequence item as a one-line value.
fn is_block_scalar_header(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('|' | '>'))
        && text.len() <= 3
        && chars.all(|c| matches!(c, '+' | '-' | '1'..='9'))
}

/// The value text inside a scalar's quotes and its offset in `authored`.
fn unquote(authored: &str, plain: bool) -> Result<(&str, usize), RefusalReason> {
    if plain {
        return Ok((authored, 0));
    }
    let quote = match authored.as_bytes().first() {
        Some(quote @ (b'\'' | b'"')) => *quote,
        _ => return Err(RefusalReason::SpanMismatch),
    };
    if authored.len() < 2 || authored.as_bytes()[authored.len() - 1] != quote {
        return Err(RefusalReason::SpanMismatch);
    }
    let inner = &authored[1..authored.len() - 1];
    if quote == b'"' && inner.contains('\\') {
        return Err(RefusalReason::EscapedString);
    }
    Ok((inner, 1))
}

fn span_mismatch(target: BaselineTarget, authored: &str, parsed: &str) -> Refusal {
    Refusal {
        reason: RefusalReason::SpanMismatch,
        target: Some(target),
        message: format!(
            "the source text `{authored}` does not decode to the parsed value `{parsed}`, so \
             renewal cannot edit it byte-exactly"
        ),
    }
}

/// The offset and value of the inline baseline date in a
/// `ValidFor(<duration>, <date>)` rule as authored.
fn inline_date(rule: &str) -> Option<(usize, NaiveDate)> {
    let close = rule.strip_suffix(')')?.len();
    let comma = rule[..close].rfind(',')?;
    let argument = &rule[comma + 1..close];
    let start = comma + 1 + (argument.len() - argument.trim_start_matches(' ').len());
    let date = rule[start..close].trim_end_matches(' ');
    Some((start, NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?))
}

/// One edit that appends every new property: at the end of the frontmatter
/// block with the terminator of the line before it, or as a new block after
/// any BOM with the terminator the body uses.
///
/// When the block ends with a block scalar that keeps its final line break
/// (`note: |`), a line after it would add that break to its value, so the
/// properties go in front of that last top-level entry instead.
fn append_properties(
    source: &str,
    frontmatter: Option<&Frontmatter>,
    located: Option<&Yaml>,
    groups: &[&Group],
) -> TextEdit {
    let lines = |ending: &str| -> String {
        groups
            .iter()
            .map(|group| match &group.target {
                BaselineTarget::Property { name } => format!("{name}: {}{ending}", group.value),
                BaselineTarget::Inline { .. } => unreachable!("only properties are appended"),
            })
            .collect()
    };
    match frontmatter {
        Some(frontmatter) => {
            let at = located
                .and_then(Yaml::trailing_block_scalar_entry)
                .map_or(frontmatter.yaml().end, |offset| located.expect("checked").to_document(offset));
            let ending = if at == frontmatter.yaml().start {
                frontmatter.fence_line_ending().as_str()
            } else {
                trailing_line_ending(&source[..at])
            };
            TextEdit {
                span: at..at,
                replacement: lines(ending),
            }
        }
        None => {
            let bom = bom_len(source);
            let ending = first_line_ending(&source[bom..]).as_str();
            TextEdit {
                span: bom..bom,
                replacement: format!("---{ending}{}---{ending}", lines(ending)),
            }
        }
    }
}

fn bom_len(source: &str) -> usize {
    if source.starts_with('\u{feff}') { 3 } else { 0 }
}

fn trailing_line_ending(text: &str) -> &'static str {
    if text.ends_with("\r\n") {
        "\r\n"
    } else if text.ends_with('\r') {
        "\r"
    } else {
        "\n"
    }
}

fn first_line_ending(text: &str) -> LineEnding {
    match text.find(['\n', '\r']) {
        Some(index) if text[index..].starts_with("\r\n") => LineEnding::CrLf,
        Some(index) if text.as_bytes()[index] == b'\r' => LineEnding::Cr,
        _ => LineEnding::Lf,
    }
}

/// Replaces `path` with `bytes` through a sibling temporary file and a
/// rename, so a reader sees either the old or the new document, never a
/// partial one. A symlinked document is written through to its target.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let target = if fs::symlink_metadata(path)?.file_type().is_symlink() {
        fs::canonicalize(path)?
    } else {
        path.to_path_buf()
    };
    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?
        .to_string_lossy()
        .into_owned();
    let permissions = fs::metadata(&target)?.permissions();
    let (temp, mut file) = loop {
        let temp = parent.join(format!(
            ".{name}.{}-{}.renew.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (temp, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    };
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| {
            drop(file);
            fs::set_permissions(&temp, permissions)
        })
        .and_then(|()| fs::rename(&temp, &target));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(target: BaselineTarget, entry: usize, value: &str) -> Write {
        Write {
            target,
            entry,
            value: value.to_string(),
        }
    }

    fn property(name: &str) -> BaselineTarget {
        BaselineTarget::Property {
            name: name.to_string(),
        }
    }

    #[test]
    fn identical_writes_consolidate_and_different_ones_conflict() {
        let (groups, conflicts) = consolidate(vec![
            write(property("last_updated"), 0, "2026-12-29"),
            write(BaselineTarget::Inline { entry: 1 }, 1, "2026-12-29"),
            write(property("last_updated"), 2, "2026-12-29"),
        ]);
        assert!(conflicts.is_empty());
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].entries, vec![0, 2]);

        // Time rules always write the update date; a later rule kind that
        // writes something else to the same property must conflict.
        let (_, conflicts) = consolidate(vec![
            write(property("fingerprint"), 0, "2026-12-29"),
            write(property("fingerprint"), 3, "blake3-lf:00"),
        ]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, ConflictKind::DifferentWrites);
        assert_eq!(conflicts[0].entries, vec![0, 3]);
        assert!(conflicts[0].message.contains("entries 1, 4"), "{}", conflicts[0].message);
    }

    #[test]
    fn inline_date_finds_the_authored_baseline() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 28).unwrap();
        assert_eq!(inline_date("ValidFor(3mo, 2026-09-28)"), Some((14, date)));
        assert_eq!(inline_date("ValidFor(3mo,2026-09-28)"), Some((13, date)));
        assert_eq!(inline_date("ValidFor(3mo,   2026-09-28  )"), Some((16, date)));
        assert_eq!(inline_date("ValidFor(3mo, @last_updated)"), None);
        assert_eq!(inline_date("ValidFor(3mo)"), None);
    }

    #[test]
    fn located_offsets_map_back_through_the_tab_repair() {
        let source = "---\na:\n\t- x\nb:\n\t\tc: 2026-01-01\nd: 1\n---\n";
        let ReadOutcome::Found(frontmatter) = read_frontmatter(source.as_bytes()).unwrap() else {
            panic!("no frontmatter");
        };
        let yaml = Yaml::new(source, &frontmatter);
        assert!(!yaml.text.contains('\t'));
        for needle in ["x", "2026-01-01", "d: 1"] {
            let local = yaml.text.find(needle).unwrap();
            let document = yaml.to_document(local);
            assert_eq!(&source[document..document + needle.len()], needle);
        }
        assert_eq!(yaml.to_document(yaml.text.len()), frontmatter.yaml().end);
    }

    /// Valid input always decodes to its parsed value, so the guard is
    /// exercised by pairing a source with a record it did not produce.
    #[test]
    fn a_span_that_does_not_decode_to_the_parsed_value_is_refused() {
        let source = "---\nlast_updated: '2026-01-01'\ncontent_policy:\n  - ValidFor(3mo, 2026-01-01)\n---\n";
        let ReadOutcome::Found(frontmatter) = read_frontmatter(source.as_bytes()).unwrap() else {
            panic!("no frontmatter");
        };
        let yaml = Yaml::new(source, &frontmatter);
        let other_date = Value::from("2026-02-02");
        let refusal = plan_property(Some(&yaml), "last_updated", Some(&other_date), "2026-09-28").err().unwrap();
        assert_eq!(refusal.reason, RefusalReason::SpanMismatch);
        assert!(refusal.message.contains("'2026-01-01'"), "{}", refusal.message);
        let other_rule = Value::from("ValidFor(3mo, 2026-02-02)");
        let refusal = plan_inline(&yaml, "content_policy", 0, &other_rule, "2026-09-28").err().unwrap();
        assert_eq!(refusal.reason, RefusalReason::SpanMismatch);
    }

    #[test]
    fn block_scalar_headers_are_recognized() {
        for header in ["|", ">", "|-", ">+", "|2", ">2-", "|+1"] {
            assert!(is_block_scalar_header(header), "{header}");
        }
        for text in ["2026-01-01", "|x", "| a |", "", "-"] {
            assert!(!is_block_scalar_header(text), "{text}");
        }
    }

    #[test]
    fn write_atomic_replaces_the_file_and_leaves_no_temporary() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        fs::write(&path, "old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        let names: Vec<_> = fs::read_dir(directory.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("note.md")]);
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_writes_through_a_symlink() {
        let directory = tempfile::tempdir().unwrap();
        let real = directory.path().join("real.md");
        let link = directory.path().join("link.md");
        fs::write(&real, "old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, b"new").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "new");
    }
}
