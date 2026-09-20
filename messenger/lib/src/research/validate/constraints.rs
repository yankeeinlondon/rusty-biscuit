//! Constraint rules (SR-STATE-VALUE, SR-CONDITION, SR-APPLICABILITY,
//! SR-AGGREGATE, SR-KIND-UNIT) and the executable-eligibility projection
//! (SR-ENFORCEABLE).

use std::collections::BTreeMap;

use serde::Serialize;

use super::{Index, condition_lists, knowledge_blocks, push};
use crate::research::diagnostics::{Findings, Rule};
use crate::research::model::{
    AggregationScope, Condition, ConditionKind, Constraint, ConstraintKind, Enforcer, Member,
    Operand, OverflowBehavior, PlatformDocument, PlatformId, Stage, State, Surface, Unit,
};

pub(super) fn check(d: &PlatformDocument, findings: &mut Findings) {
    state_values(d, findings);
    conditions(d, findings);
    for (i, constraint) in d.constraints.iter().enumerate() {
        kind_unit(i, constraint, findings);
        aggregate(i, constraint, findings);
    }
    applicability(d, findings);
    lower_bounds(d, findings);
}

// ---- SR-STATE-VALUE ---------------------------------------------------------

fn state_values(d: &PlatformDocument, findings: &mut Findings) {
    for (pointer, id, knowledge) in knowledge_blocks(d) {
        match knowledge.state {
            State::Unknown | State::Conflicting if knowledge.gap.is_none() => push(
                findings,
                Rule::StateValue,
                format!("{pointer}/gap"),
                id,
                format!("a {} fact names the gap that investigates it", knowledge.state),
            ),
            _ => {}
        }
        if knowledge.state == State::Conflicting && knowledge.claims.len() < 2 {
            push(
                findings,
                Rule::StateValue,
                format!("{pointer}/claims"),
                id,
                "a conflicting fact keeps two or more competing claims",
            );
        }
        if knowledge.state == State::NotApplicable && knowledge.explanation.is_none() {
            push(
                findings,
                Rule::StateValue,
                format!("{pointer}/explanation"),
                id,
                "a not-applicable fact explains why",
            );
        }
    }
    for (i, c) in d.constraints.iter().enumerate() {
        let pointer = format!("/constraints/{i}/value");
        match (c.knowledge.state, c.value) {
            (State::Known, None) => push(findings, Rule::StateValue, pointer, &c.id, "a known bound carries its value"),
            (state, Some(_)) if state != State::Known => push(
                findings,
                Rule::StateValue,
                pointer,
                &c.id,
                format!("a {state} bound carries no value; it is never zero or unlimited"),
            ),
            _ => {}
        }
    }
    for (i, entry) in d.chronology.iter().enumerate() {
        if entry.release_date_state != State::Known && (entry.release_date.is_some() || entry.release_period.is_some()) {
            push(
                findings,
                Rule::StateValue,
                format!("/chronology/{i}/release_date_state"),
                &entry.id,
                "a release date or period needs release_date_state: known; a retrieval date is not a release date",
            );
        }
    }
}

// ---- SR-CONDITION -----------------------------------------------------------

fn conditions(d: &PlatformDocument, findings: &mut Findings) {
    for (pointer, id, list) in condition_lists(d) {
        for (c, condition) in list.iter().enumerate() {
            let pointer = format!("{pointer}/{c}");
            match condition.operand_forms() {
                0 if condition.gap.is_none() => push(
                    findings,
                    Rule::Condition,
                    pointer.clone(),
                    id,
                    format!("{} condition has no operand and names no gap", condition.kind),
                ),
                0 | 1 => {}
                _ => push(
                    findings,
                    Rule::Condition,
                    pointer.clone(),
                    id,
                    format!("{} condition mixes operand forms; use exactly one of equals, min/max, or one_of", condition.kind),
                ),
            }
            if condition.min.is_some() || condition.max.is_some() {
                if !is_ordered(condition.kind) {
                    push(
                        findings,
                        Rule::Condition,
                        pointer,
                        id,
                        format!("{} is not an ordered condition; min/max do not apply", condition.kind),
                    );
                }
            } else if condition.one_of.as_ref().is_some_and(Vec::is_empty) {
                push(findings, Rule::Condition, pointer, id, "one_of lists at least one value");
            }
        }
    }
}

/// Condition kinds with an ordered operand (versions and ages).
fn is_ordered(kind: ConditionKind) -> bool {
    matches!(
        kind,
        ConditionKind::ApiVersion | ConditionKind::SdkVersion | ConditionKind::BridgeVersion | ConditionKind::ReleaseAge
    )
}

/// Whether a condition resolved to a bounded operand without a gap.
fn resolved(condition: &Condition) -> bool {
    condition.gap.is_none() && condition.operand().is_some()
}

// ---- SR-KIND-UNIT -----------------------------------------------------------

fn kind_unit(i: usize, c: &Constraint, findings: &mut Findings) {
    let base = format!("/constraints/{i}");
    match c.kind {
        ConstraintKind::RecommendedMax => {
            if c.enforced_by.is_some() {
                push(findings, Rule::KindUnit, format!("{base}/enforced_by"), &c.id, "a recommendation is not enforced");
            }
            if c.recommended_by.is_none() {
                push(findings, Rule::KindUnit, format!("{base}/recommended_by"), &c.id, "a recommendation names its recommending source");
            }
        }
        ConstraintKind::HardMax | ConstraintKind::HardMin
            if c.knowledge.state == State::Known && c.enforced_by.is_none() =>
        {
            push(findings, Rule::KindUnit, format!("{base}/enforced_by"), &c.id, "a known hard bound names its enforcer")
        }
        _ => {}
    }
    if c.kind != ConstraintKind::RecommendedMax && c.recommended_by.is_some() {
        push(findings, Rule::KindUnit, format!("{base}/recommended_by"), &c.id, "only a recommendation names recommended_by");
    }
    let unit_ok = match c.unit {
        Unit::Bytes => c.kind.is_payload(),
        Unit::Items => c.kind.is_count() || c.kind == ConstraintKind::AggregateMax,
        Unit::Unknown => true,
        _ if c.kind.is_count() => false,
        Unit::Utf8Bytes => true,
        _ => !c.kind.is_payload(),
    };
    if !unit_ok {
        push(
            findings,
            Rule::KindUnit,
            format!("{base}/unit"),
            &c.id,
            format!("unit {} does not measure a {} bound", c.unit, c.kind),
        );
    }
}

// ---- SR-AGGREGATE -----------------------------------------------------------

fn aggregate(i: usize, c: &Constraint, findings: &mut Findings) {
    let base = format!("/constraints/{i}");
    if c.kind != ConstraintKind::AggregateMax {
        if !c.members.is_empty() || c.aggregation_scope.is_some() {
            push(findings, Rule::Aggregate, format!("{base}/members"), &c.id, "only an aggregate bound has members or an aggregation scope");
        }
        return;
    }
    if c.members.is_empty() || c.aggregation_scope.is_none() {
        push(findings, Rule::Aggregate, base.clone(), &c.id, "an aggregate bound needs members and an aggregation scope");
    }
    for (m, member) in c.members.iter().enumerate() {
        let compatible = if c.unit == Unit::Items {
            member.surface.is_collection()
        } else if c.unit.is_text() {
            !member.surface.is_collection()
        } else {
            true
        };
        if !compatible {
            push(
                findings,
                Rule::Aggregate,
                format!("{base}/members/{m}/surface"),
                &c.id,
                format!("member {} is incompatible with unit {}", member.surface, c.unit),
            );
        }
    }
}

// ---- SR-APPLICABILITY -------------------------------------------------------

fn applicability(d: &PlatformDocument, findings: &mut Findings) {
    let mut scoped: BTreeMap<(&str, &str, Surface, ConstraintKind), Vec<usize>> = BTreeMap::new();
    for (i, c) in d.constraints.iter().enumerate() {
        if c.knowledge.state == State::Known {
            scoped.entry((c.interface.as_str(), c.operation.as_str(), c.surface, c.kind)).or_default().push(i);
        }
    }
    for positions in scoped.values() {
        for (n, &later) in positions.iter().enumerate() {
            for &earlier in &positions[..n] {
                let (a, b) = (&d.constraints[earlier], &d.constraints[later]);
                if a.applies_when != b.applies_when
                    && a.value != b.value
                    && a.unit == b.unit
                    && a.measurement_stage == b.measurement_stage
                    && can_co_hold(&a.applies_when, &b.applies_when)
                {
                    push(
                        findings,
                        Rule::Applicability,
                        format!("/constraints/{later}/applies_when"),
                        &b.id,
                        format!("conditions can hold together with {}, so neither value is decided", a.id),
                    );
                }
            }
        }
    }
}

/// Two condition sets are exclusive only when some kind appears in both
/// with disjoint equality operands. Anything else might hold together.
fn can_co_hold(left: &[Condition], right: &[Condition]) -> bool {
    for a in left {
        for b in right.iter().filter(|b| b.kind == a.kind) {
            if let (Some(a), Some(b)) = (values(a), values(b))
                && a.iter().all(|value| !b.contains(value))
            {
                return false;
            }
        }
    }
    true
}

fn values(condition: &Condition) -> Option<Vec<&str>> {
    match condition.operand()? {
        Operand::Equals(value) => Some(vec![value]),
        Operand::OneOf(values) => Some(values.iter().map(String::as_str).collect()),
        Operand::Range { .. } => None,
    }
}

/// A known lower bound must not exceed the matching upper bound.
fn lower_bounds(d: &PlatformDocument, findings: &mut Findings) {
    for (i, min) in d.constraints.iter().enumerate() {
        if !min.kind.is_lower_bound() || min.knowledge.state != State::Known {
            continue;
        }
        let upper = match min.kind {
            ConstraintKind::ItemCountMin => ConstraintKind::ItemCountMax,
            _ => ConstraintKind::HardMax,
        };
        for max in d.constraints.iter().filter(|c| {
            c.kind == upper
                && c.knowledge.state == State::Known
                && c.interface == min.interface
                && c.operation == min.operation
                && c.surface == min.surface
                && c.unit == min.unit
                && c.applies_when == min.applies_when
        }) {
            if let (Some(low), Some(high)) = (min.value, max.value)
                && low > high
            {
                push(
                    findings,
                    Rule::StateValue,
                    format!("/constraints/{i}/value"),
                    &min.id,
                    format!("lower bound {low} exceeds {} ({high})", max.id),
                );
            }
        }
    }
}

// ---- SR-ENFORCEABLE ---------------------------------------------------------

/// Why a constraint has no executable projection. Ineligible facts stay in
/// catalogs and reports; they are never coerced to zero or unlimited.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum IneligibleReason {
    /// Only `known` facts are executable.
    NotKnown { state: State },
    /// `unspecified_characters` and `unknown` do not say how to count.
    UnresolvedUnit { unit: Unit },
    UnknownStage,
    /// A condition names a gap or has no bounded operand.
    UnresolvedCondition { kind: ConditionKind },
    NoEvidence,
    /// Secondary sources are leads, never executable evidence.
    SecondaryEvidenceOnly,
    /// Recommendations have no enforcing consumer.
    Advisory,
}

impl IneligibleReason {
    /// A stable code, e.g. `unresolved_unit`.
    pub fn code(&self) -> &'static str {
        match self {
            IneligibleReason::NotKnown { .. } => "not_known",
            IneligibleReason::UnresolvedUnit { .. } => "unresolved_unit",
            IneligibleReason::UnknownStage => "unknown_stage",
            IneligibleReason::UnresolvedCondition { .. } => "unresolved_condition",
            IneligibleReason::NoEvidence => "no_evidence",
            IneligibleReason::SecondaryEvidenceOnly => "secondary_evidence_only",
            IneligibleReason::Advisory => "advisory",
        }
    }
}

/// A resolved applicability condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ExecutableCondition {
    pub kind: ConditionKind,
    pub equals: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub one_of: Option<Vec<String>>,
}

/// A known constraint with resolved evidence, unit, stage, and conditions.
///
/// Constructed only by validation; the unit and stage are kept exactly as
/// researched (UTF-8 bytes, Unicode scalars, UTF-16 code units, grapheme
/// clusters, parsed text versus markup are distinct quantities).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ExecutableConstraint {
    pub platform_id: PlatformId,
    pub interface: String,
    pub operation: String,
    pub id: String,
    pub surface: Surface,
    pub native_locator: String,
    pub kind: ConstraintKind,
    pub value: u64,
    pub unit: Unit,
    pub stage: Stage,
    pub enforced_by: Option<Enforcer>,
    pub overflow_behavior: OverflowBehavior,
    pub conditions: Vec<ExecutableCondition>,
    pub aggregation_scope: Option<AggregationScope>,
    pub members: Vec<Member>,
}

/// One constraint's eligibility.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "eligibility", rename_all = "snake_case")]
pub enum ConstraintEligibility {
    Eligible(ExecutableConstraint),
    Ineligible {
        id: String,
        interface: String,
        operation: String,
        reasons: Vec<IneligibleReason>,
    },
}

impl ConstraintEligibility {
    pub fn id(&self) -> &str {
        match self {
            ConstraintEligibility::Eligible(constraint) => &constraint.id,
            ConstraintEligibility::Ineligible { id, .. } => id,
        }
    }

    pub fn executable(&self) -> Option<&ExecutableConstraint> {
        match self {
            ConstraintEligibility::Eligible(constraint) => Some(constraint),
            ConstraintEligibility::Ineligible { .. } => None,
        }
    }

    /// Empty for an eligible constraint.
    pub fn reasons(&self) -> &[IneligibleReason] {
        match self {
            ConstraintEligibility::Eligible(_) => &[],
            ConstraintEligibility::Ineligible { reasons, .. } => reasons,
        }
    }
}

pub(super) fn eligibility(d: &PlatformDocument) -> Vec<ConstraintEligibility> {
    let index = Index::new(d);
    d.constraints
        .iter()
        .map(|c| {
            let mut reasons = Vec::new();
            if c.knowledge.state != State::Known {
                reasons.push(IneligibleReason::NotKnown { state: c.knowledge.state });
            }
            if !c.unit.is_resolved() {
                reasons.push(IneligibleReason::UnresolvedUnit { unit: c.unit });
            }
            if c.measurement_stage == Stage::Unknown {
                reasons.push(IneligibleReason::UnknownStage);
            }
            for condition in c.applies_when.iter().filter(|condition| !resolved(condition)) {
                reasons.push(IneligibleReason::UnresolvedCondition { kind: condition.kind });
            }
            if c.knowledge.evidence.is_empty() {
                reasons.push(IneligibleReason::NoEvidence);
            } else if index.secondary_only(&c.knowledge.evidence) {
                reasons.push(IneligibleReason::SecondaryEvidenceOnly);
            }
            if c.kind == ConstraintKind::RecommendedMax {
                reasons.push(IneligibleReason::Advisory);
            }
            reasons.sort();
            reasons.dedup();
            match (reasons.is_empty(), c.value) {
                (true, Some(value)) => ConstraintEligibility::Eligible(ExecutableConstraint {
                    platform_id: d.platform_id,
                    interface: c.interface.clone(),
                    operation: c.operation.clone(),
                    id: c.id.clone(),
                    surface: c.surface,
                    native_locator: c.native_locator.clone(),
                    kind: c.kind,
                    value,
                    unit: c.unit,
                    stage: c.measurement_stage,
                    enforced_by: c.enforced_by,
                    overflow_behavior: c.overflow_behavior,
                    conditions: c
                        .applies_when
                        .iter()
                        .map(|condition| ExecutableCondition {
                            kind: condition.kind,
                            equals: condition.equals.clone(),
                            min: condition.min.clone(),
                            max: condition.max.clone(),
                            one_of: condition.one_of.clone(),
                        })
                        .collect(),
                    aggregation_scope: c.aggregation_scope,
                    members: c.members.clone(),
                }),
                _ => ConstraintEligibility::Ineligible {
                    id: c.id.clone(),
                    interface: c.interface.clone(),
                    operation: c.operation.clone(),
                    reasons: if reasons.is_empty() {
                        vec![IneligibleReason::NotKnown { state: c.knowledge.state }]
                    } else {
                        reasons
                    },
                },
            }
        })
        .collect()
}
