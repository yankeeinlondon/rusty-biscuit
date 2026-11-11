//! Strict readers for the JSON documents a push review decides from: the open
//! pull requests `gh pr list` reports, the planner's resolved plan, the
//! constraint reader's minimal view of that plan, and one constraint record.
//!
//! Every invalid shape is an error naming the document and the JSON path; none
//! is read as absent or empty. These validators decide; the plan renderer's
//! lenient defaults are display-only and never stand in for them.

use std::fmt;

use chrono::NaiveDate;
use serde_json::Value;

use super::json::{self, Located};

/// A document `ci-push` could not trust, and where in it the problem is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentError {
    /// What the document is, such as `` `gh pr list` output `` or a file path.
    pub document: String,
    pub location: String,
    pub problem: String,
}

impl DocumentError {
    fn at(document: &str, located: Located) -> Self {
        Self {
            document: document.to_owned(),
            location: located.location,
            problem: located.problem,
        }
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.location.is_empty() {
            write!(formatter, "{} {}", self.document, self.problem)
        } else {
            write!(formatter, "{}: `{}` {}", self.document, self.location, self.problem)
        }
    }
}

impl std::error::Error for DocumentError {}

/// One open pull request whose head is the pushed branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub base_branch: String,
}

/// Reads `gh pr list --json number,baseRefName`.
///
/// `[]` means no open pull request. Fields other than the two read are
/// tolerated: `gh` owns this document's shape.
pub fn parse_pull_requests(bytes: &[u8], document: &str) -> Result<Vec<PullRequest>, DocumentError> {
    let error = |located| DocumentError::at(document, located);
    let value = json::load(bytes).map_err(error)?;
    let Value::Array(entries) = &value else {
        return Err(error(Located::new(
            "",
            format!("must be an array, found {}", json::kind(&value)),
        )));
    };
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let at = json::index_path("", index);
            let object = json::object(entry, &at)?;
            Ok(PullRequest {
                number: json::positive_integer(object, &at, "number")?,
                base_branch: json::required_text(object, &at, "baseRefName")?.to_owned(),
            })
        })
        .collect::<Result<_, Located>>()
        .map_err(error)
}

/// The identity and status of one planned cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCell {
    pub package: String,
    pub environment: String,
    pub gate: String,
    pub execution: String,
    pub state: String,
}

/// A resolved plan, validated for the fields orchestration binds and renders.
///
/// `document` keeps the whole value, unknown fields included, because the
/// planner adds fields between schema versions and a reader that only passes
/// the plan on must not drop them.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPlan {
    pub base: String,
    pub head: String,
    pub cells: Vec<PlanCell>,
    pub document: Value,
}

/// Reads a resolved plan written by `affected_scope.py`.
pub fn parse_resolved_plan(bytes: &[u8], document: &str) -> Result<ResolvedPlan, DocumentError> {
    let error = |located| DocumentError::at(document, located);
    let value = json::load(bytes).map_err(error)?;
    let parsed = (|| {
        let root = json::object(&value, "")?;
        let base = json::required_text(root, "", "base")?.to_owned();
        let head = json::required_text(root, "", "head")?.to_owned();
        let cells = json::required_array(root, "", "cells")?
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let at = json::index_path("cells", index);
                let cell = json::object(cell, &at)?;
                Ok(PlanCell {
                    package: json::required_text(cell, &at, "package")?.to_owned(),
                    environment: json::required_text(cell, &at, "environment")?.to_owned(),
                    gate: json::required_text(cell, &at, "gate")?.to_owned(),
                    execution: json::required_text(cell, &at, "execution")?.to_owned(),
                    state: json::required_text(cell, &at, "state")?.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, Located>>()?;
        Ok::<_, Located>((base, head, cells))
    })()
    .map_err(error)?;
    let (base, head, cells) = parsed;
    Ok(ResolvedPlan {
        base,
        head,
        cells,
        document: value,
    })
}

/// One cell as the constraint check reads it.
///
/// `state` is `None` only when the field is absent: plans older than the
/// `state` field carry none, and absence means no prohibited-state override.
/// It is never synthesized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintCell {
    pub environment: String,
    pub gate: String,
    pub execution: String,
    pub state: Option<String>,
}

/// Reads the minimal plan view the constraint check needs, matching what
/// `constraints.py read_plan` accepts: `cells`, each with `environment`,
/// `gate`, and `execution`, and an optional `state`.
pub fn parse_constraint_plan(bytes: &[u8], document: &str) -> Result<Vec<ConstraintCell>, DocumentError> {
    let error = |located| DocumentError::at(document, located);
    let value = json::load(bytes).map_err(error)?;
    let root = json::object(&value, "").map_err(error)?;
    json::required_array(root, "", "cells")
        .map_err(error)?
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            let at = json::index_path("cells", index);
            let cell = json::object(cell, &at)?;
            let state = match cell.get("state") {
                None => None,
                Some(_) => Some(json::required_text(cell, &at, "state")?.to_owned()),
            };
            Ok(ConstraintCell {
                environment: json::required_text(cell, &at, "environment")?.to_owned(),
                gate: json::required_text(cell, &at, "gate")?.to_owned(),
                execution: json::required_text(cell, &at, "execution")?.to_owned(),
                state,
            })
        })
        .collect::<Result<_, Located>>()
        .map_err(error)
}

/// One persisted execution constraint.
///
/// `gate`, `repository`, and `branch` are `""` when the record leaves them
/// out, which means "every gate/repository/branch".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintRecord {
    pub environment: String,
    pub reason: String,
    pub owner: String,
    /// The last day the record binds; it is expired only after this day.
    pub expiry: NaiveDate,
    pub gate: String,
    pub repository: String,
    pub branch: String,
}

const CONSTRAINT_FIELDS: [&str; 7] = [
    "environment",
    "reason",
    "owner",
    "expiry",
    "gate",
    "repository",
    "branch",
];

/// Reads one constraint record. Unlike the documents other tools own, an
/// unknown field is an error: the store is written by hand, and a misspelled
/// `branchh` would otherwise widen the record to every branch.
pub fn parse_constraint_record(bytes: &[u8], document: &str) -> Result<ConstraintRecord, DocumentError> {
    let error = |located| DocumentError::at(document, located);
    let value = json::load(bytes).map_err(error)?;
    let parsed = (|| {
        let record = json::object(&value, "")?;
        if let Some(unknown) = record.keys().find(|key| !CONSTRAINT_FIELDS.contains(&key.as_str())) {
            return Err(Located::new(unknown.clone(), "is not a constraint field"));
        }
        Ok(ConstraintRecord {
            environment: json::required_text(record, "", "environment")?.to_owned(),
            reason: json::required_text(record, "", "reason")?.to_owned(),
            owner: json::required_text(record, "", "owner")?.to_owned(),
            expiry: json::calendar_date(record, "", "expiry")?,
            gate: json::optional_wildcard_text(record, "", "gate")?.to_owned(),
            repository: json::optional_wildcard_text(record, "", "repository")?.to_owned(),
            branch: json::optional_wildcard_text(record, "", "branch")?.to_owned(),
        })
    })();
    parsed.map_err(error)
}
