//! Steering catalog stage: researched steering facts, execution interfaces,
//! and the reviewed activation policy → `lib/src/steering/generated.rs`.
//!
//! Like the stream vocabulary, this is a standalone full-scope artifact, not
//! a `ProviderInfo` field: steering facts are runtime selection inputs, so this
//! loader declares their ownership itself.
//!
//! ## Source ownership
//!
//! - `docs/research/steering/<slug>.md` owns discovery methods, mechanisms,
//!   cases, access, receipts, and live verification (with typed assertions).
//! - `docs/providers.yaml` owns the roster order listings sort by.
//! - `docs/research/non-interactive-sessions/<slug>.md` owns execution
//!   interfaces and the preferred/fallback selection.
//! - [`ACTIVATION_POLICY`] owns reviewed adapter bindings, activation grants,
//!   and profile blocks. It is hand-reviewed policy, never derived from
//!   research: a new passing verification record grants nothing until a
//!   reviewed grant names it, a grant is rejected unless research supports
//!   every part of it, and a blocked profile cannot be granted at all.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use claudine_catalog_types::steering::{
    AccessStatus, AssertionKind, CaseSupport, ConversationEffect, DeliveryBoundary, DeliveryState,
    DiscoveryMethod, ExecutionInterfaceKind, ExecutionState, GuaranteeLevel, HostOs, LaunchMode, LaunchOrigin,
    OperationIntent, ReceiptTiming, SteeringTransport, VerificationOutcome,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::emit::provider_variant;
use crate::errors::GenError;
use crate::generate::{CheckOutcome, diff_lines, provider_slugs};
use crate::inputs;

/// Research topic owning steering facts.
pub const RESEARCH_TOPIC: &str = "steering";
/// Research topic owning execution interfaces and their selection.
pub const EXECUTION_TOPIC: &str = "non-interactive-sessions";
/// Area-relative path of the reviewed activation policy.
pub const ACTIVATION_POLICY: &str = "docs/providers/steering-activation.yaml";

/// Path of the reviewed activation policy under `area`.
pub fn activation_policy_path(area: &Path) -> PathBuf {
    area.join(ACTIVATION_POLICY)
}

/// Path of the generated steering catalog under `area`.
pub fn steering_catalog_path(area: &Path) -> PathBuf {
    area.join("lib/src/steering/generated.rs")
}

// ---------------------------------------------------------------------------
// Reviewed activation policy
// ---------------------------------------------------------------------------

/// Deserializes a policy string that is written as a YAML string. A plain
/// `String` accepts any scalar (`7`, `0.84`, `true`) by its text, silently
/// coercing a mistyped value; this resolves the scalar's type first.
fn strict_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    StrictString::deserialize(deserializer).map(|value| value.0)
}

/// [`strict_string`] for every element of a required, non-null list.
fn strict_string_list<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    non_null_list::<D, StrictString>(deserializer).map(|items| items.into_iter().map(|item| item.0).collect())
}

struct StrictString(String);

impl<'de> Deserialize<'de> for StrictString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = StrictString;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<StrictString, E> {
                Ok(StrictString(value.to_string()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// Deserializes a required list, rejecting YAML null. Plain `Vec<T>` would
/// read an empty `key:` value as an empty list, conflating null with `[]`.
fn non_null_list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer)?
        .ok_or_else(|| serde::de::Error::custom("expected a list, found null"))
}

/// The hand-reviewed activation policy. Every list is a required key: an
/// absent or null list is malformed, never an empty policy.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationPolicy {
    /// Reviewed adapter implementations, by identifier and revision.
    #[serde(deserialize_with = "non_null_list")]
    pub adapters: Vec<ReviewedAdapter>,
    /// Exact activation grants.
    #[serde(deserialize_with = "non_null_list")]
    pub grants: Vec<PolicyGrant>,
    /// Launch profiles reviewed and refused for steering.
    #[serde(deserialize_with = "non_null_list")]
    pub blocks: Vec<PolicyBlock>,
}

/// One reviewed refusal: a launch profile no grant may activate.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyBlock {
    #[serde(deserialize_with = "strict_string")]
    pub provider: String,
    #[serde(deserialize_with = "strict_string")]
    pub profile_id: String,
    #[serde(deserialize_with = "strict_string")]
    pub reason: String,
}

/// One reviewed adapter binding.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedAdapter {
    #[serde(deserialize_with = "strict_string")]
    pub id: String,
    pub revision: u32,
    #[serde(deserialize_with = "strict_string")]
    pub provider: String,
    #[serde(deserialize_with = "strict_string_list")]
    pub mechanism_ids: Vec<String>,
}

/// Adapter identifier and revision named by a grant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyAdapterRef {
    #[serde(deserialize_with = "strict_string")]
    pub id: String,
    pub revision: u32,
}

/// One activation grant as authored.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyGrant {
    #[serde(deserialize_with = "strict_string")]
    pub provider: String,
    #[serde(deserialize_with = "strict_string")]
    pub mechanism_id: String,
    pub operation: OperationIntent,
    pub adapter: PolicyAdapterRef,
    #[serde(deserialize_with = "strict_string")]
    pub profile_id: String,
    pub os: HostOs,
    #[serde(deserialize_with = "strict_string")]
    pub provider_version: String,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub session_state: ExecutionState,
    #[serde(deserialize_with = "strict_string_list")]
    pub verification_ids: Vec<String>,
}

/// Parses the policy text. Duplicate keys, extra documents, unknown keys,
/// and wrong types are errors.
pub fn parse_activation_policy(path: &Path, text: &str) -> Result<ActivationPolicy, GenError> {
    serde_yaml_ng::from_str(text).map_err(|err| GenError::SteeringPolicyInvalid {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}

/// Loads the policy from `area`; a missing file is an error.
pub fn load_activation_policy(area: &Path) -> Result<ActivationPolicy, GenError> {
    let path = activation_policy_path(area);
    let text = std::fs::read_to_string(&path).map_err(|source| GenError::Io {
        path: path.clone(),
        source,
    })?;
    parse_activation_policy(&path, &text)
}

// ---------------------------------------------------------------------------
// Typed research projection
// ---------------------------------------------------------------------------

/// The typed subset of one provider's steering and execution research that
/// the runtime and the activation checker consume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchSteering {
    pub discovery: Vec<Discovery>,
    pub mechanisms: Vec<Mechanism>,
    pub cases: Vec<Case>,
    pub access: Vec<Access>,
    pub verification: Vec<Verification>,
    pub execution_interfaces: Vec<Interface>,
    pub preferred_interface: String,
    pub fallback_interface: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: String,
    pub transport: SteeringTransport,
    pub request_format: String,
    pub operation_intent: OperationIntent,
    pub conversation_effect: ConversationEffect,
    pub delivery_boundary: DeliveryBoundary,
    pub receipt_timing: ReceiptTiming,
    pub request_acceptance: GuaranteeLevel,
    pub persistence: GuaranteeLevel,
    pub scheduling: GuaranteeLevel,
    pub conversation_delivery: GuaranteeLevel,
    pub delivery_states: Vec<DeliveryState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Case {
    pub profile_id: String,
    pub os: HostOs,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub session_state: ExecutionState,
    pub support: CaseSupport,
    pub mechanism_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Discovery {
    pub id: String,
    pub profile_id: String,
    pub os: HostOs,
    pub origin: LaunchOrigin,
    pub method: DiscoveryMethod,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Access {
    pub mechanism_id: String,
    pub profile_id: String,
    pub os: HostOs,
    pub status: AccessStatus,
    pub prerequisite: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Verification {
    pub id: String,
    pub mechanism_id: String,
    pub profile_id: String,
    pub os: HostOs,
    pub provider_version: String,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub session_state: ExecutionState,
    pub outcome: VerificationOutcome,
    pub assertion_kinds: Vec<AssertionKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Interface {
    pub id: String,
    pub kind: ExecutionInterfaceKind,
}

#[derive(Deserialize)]
struct RawMechanism {
    id: String,
    transport: SteeringTransport,
    request_format: String,
    operation_intent: OperationIntent,
    conversation_effect: ConversationEffect,
    delivery_boundary: DeliveryBoundary,
}

#[derive(Deserialize)]
struct RawReceipt {
    mechanism_id: String,
    request_acceptance: String,
    persistence: String,
    scheduling: String,
    conversation_delivery: String,
}

#[derive(Deserialize)]
struct RawObservation {
    mechanism_id: String,
    timing: ReceiptTiming,
}

#[derive(Deserialize)]
struct RawDeliveryStates {
    mechanism_id: String,
    states: Vec<DeliveryState>,
}

#[derive(Deserialize)]
struct RawSelection {
    preferred: String,
    /// The execution contract spells "no fallback" as `""`; absence is
    /// malformed, not "no fallback".
    fallback: String,
}

fn invalid(slug: &str, message: impl Into<String>) -> GenError {
    GenError::SteeringResearchInvalid {
        slug: slug.to_string(),
        message: message.into(),
    }
}

/// Deserializes a required frontmatter key. Absent and null are both errors.
fn required<T: DeserializeOwned>(slug: &str, document: &Value, key: &str) -> Result<T, GenError> {
    let value = document
        .get(key)
        .filter(|value| !value.is_null())
        .ok_or_else(|| invalid(slug, format!("missing required `{key}`")))?;
    serde_json::from_value(value.clone()).map_err(|err| invalid(slug, format!("`{key}`: {err}")))
}

/// Maps a researched guarantee member to its normalized level. Unknown
/// members fail rather than silently becoming `unknown`.
fn guarantee(slug: &str, field: &str, member: &str) -> Result<GuaranteeLevel, GenError> {
    match member {
        "confirmed" => Ok(GuaranteeLevel::Confirmed),
        "rejected" | "not_persisted" | "not_scheduled" | "not_delivered" => Ok(GuaranteeLevel::Negative),
        "unknown" => Ok(GuaranteeLevel::Unknown),
        other => Err(invalid(slug, format!("receipt_guarantees.{field}: unknown member `{other}`"))),
    }
}

/// Projects already schema-validated frontmatter into the typed model. Each
/// mechanism requires exactly one receipt guarantee, observation, and
/// delivery-state row; the relational checker enforces that, and this
/// projection refuses rather than guesses when it does not hold.
pub fn project_research(slug: &str, steering: &Value, execution: &Value) -> Result<ResearchSteering, GenError> {
    let raw_mechanisms: Vec<RawMechanism> = required(slug, steering, "mechanisms")?;
    let receipts: Vec<RawReceipt> = required(slug, steering, "receipt_guarantees")?;
    let observations: Vec<RawObservation> = required(slug, steering, "receipt_observations")?;
    let delivery: Vec<RawDeliveryStates> = required(slug, steering, "delivery_states")?;

    let mut mechanisms = Vec::with_capacity(raw_mechanisms.len());
    for raw in raw_mechanisms {
        let one = |kind: &str, count: usize| {
            if count == 1 {
                Ok(())
            } else {
                Err(invalid(slug, format!("mechanism `{}` needs exactly one {kind} row, found {count}", raw.id)))
            }
        };
        let receipt: Vec<_> = receipts.iter().filter(|row| row.mechanism_id == raw.id).collect();
        one("receipt_guarantees", receipt.len())?;
        let observation: Vec<_> = observations.iter().filter(|row| row.mechanism_id == raw.id).collect();
        one("receipt_observations", observation.len())?;
        let states: Vec<_> = delivery.iter().filter(|row| row.mechanism_id == raw.id).collect();
        one("delivery_states", states.len())?;
        let receipt = receipt[0];
        mechanisms.push(Mechanism {
            request_acceptance: guarantee(slug, "request_acceptance", &receipt.request_acceptance)?,
            persistence: guarantee(slug, "persistence", &receipt.persistence)?,
            scheduling: guarantee(slug, "scheduling", &receipt.scheduling)?,
            conversation_delivery: guarantee(slug, "conversation_delivery", &receipt.conversation_delivery)?,
            receipt_timing: observation[0].timing,
            delivery_states: states[0].states.clone(),
            id: raw.id,
            transport: raw.transport,
            request_format: raw.request_format,
            operation_intent: raw.operation_intent,
            conversation_effect: raw.conversation_effect,
            delivery_boundary: raw.delivery_boundary,
        });
    }

    let selection: RawSelection = required(slug, execution, "execution_selection")?;
    Ok(ResearchSteering {
        discovery: required(slug, steering, "discovery")?,
        mechanisms,
        cases: required(slug, steering, "cases")?,
        access: required(slug, steering, "access_findings")?,
        verification: required(slug, steering, "verification")?,
        execution_interfaces: required(slug, execution, "execution_interfaces")?,
        preferred_interface: selection.preferred,
        fallback_interface: Some(selection.fallback).filter(|fallback| !fallback.is_empty()),
    })
}

/// Loads and projects one provider's research from `area`.
pub fn load_research(area: &Path, slug: &str) -> Result<ResearchSteering, GenError> {
    let steering = inputs::load_validated_frontmatter(
        &area.join(format!("docs/research/{RESEARCH_TOPIC}/{slug}.md")),
    )?;
    let execution = inputs::load_validated_frontmatter(
        &area.join(format!("docs/research/{EXECUTION_TOPIC}/{slug}.md")),
    )?;
    project_research(slug, &steering, &execution)
}

// ---------------------------------------------------------------------------
// Deterministic activation applicability
// ---------------------------------------------------------------------------

/// Whether `version` names exactly one provider release. Ranges, wildcards,
/// and placeholders cannot scope an activation grant.
fn is_exact_version(version: &str) -> bool {
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        && version.chars().next().is_some_and(|c| c.is_ascii_digit())
        && !version.split(['.', '-', '+']).any(|part| matches!(part, "" | "x" | "X"))
}

/// Validates the policy's adapters and grants for one provider against its
/// research. Returns sorted, deduplicated, specific blocking reasons.
pub fn activation_errors(slug: &str, research: &ResearchSteering, policy: &ActivationPolicy) -> Vec<String> {
    let mut errors = Vec::new();
    let mechanisms: BTreeMap<&str, &Mechanism> =
        research.mechanisms.iter().map(|m| (m.id.as_str(), m)).collect();

    let mut adapter_ids = BTreeSet::new();
    for adapter in policy.adapters.iter().filter(|adapter| adapter.provider == slug) {
        let label = format!("adapter `{}`@{}", adapter.id, adapter.revision);
        if !adapter_ids.insert(adapter.id.as_str()) {
            errors.push(format!("{label}: adapter id is reviewed more than once"));
        }
        if adapter.revision == 0 {
            errors.push(format!("{label}: revision must be at least 1"));
        }
        if adapter.mechanism_ids.is_empty() {
            errors.push(format!("{label}: binds no mechanism"));
        }
        for mechanism in &adapter.mechanism_ids {
            if !mechanisms.contains_key(mechanism.as_str()) {
                errors.push(format!("{label}: unresolved mechanism_id `{mechanism}`"));
            }
        }
    }

    let mut blocked = BTreeSet::new();
    for (index, block) in policy.blocks.iter().enumerate().filter(|(_, block)| block.provider == slug) {
        let label = format!("blocks[{index}] {slug}/{}", block.profile_id);
        if !blocked.insert(block.profile_id.as_str()) {
            errors.push(format!("{label}: profile is blocked more than once"));
        }
        if block.reason.trim().is_empty() {
            errors.push(format!("{label}: a block must state its reason"));
        }
        if !research.cases.iter().any(|case| case.profile_id == block.profile_id) {
            errors.push(format!("{label}: no researched case uses this profile"));
        }
    }

    let mut seen = BTreeSet::new();
    for (index, grant) in policy.grants.iter().enumerate().filter(|(_, grant)| grant.provider == slug) {
        let label = format!("grants[{index}] {slug}/{}", grant.mechanism_id);
        if blocked.contains(grant.profile_id.as_str()) {
            errors.push(format!("{label}: profile `{}` is blocked by the reviewed policy", grant.profile_id));
        }
        let key = (
            &grant.mechanism_id, &grant.profile_id, grant.os, &grant.provider_version,
            grant.launch_mode, grant.origin, grant.session_state,
        );
        if !seen.insert(key) {
            errors.push(format!("{label}: duplicate grant for the same exact case"));
        }
        errors.extend(grant_errors(&label, slug, grant, research, &mechanisms, policy));
    }
    errors.sort();
    errors.dedup();
    errors
}

fn grant_errors(
    label: &str,
    slug: &str,
    grant: &PolicyGrant,
    research: &ResearchSteering,
    mechanisms: &BTreeMap<&str, &Mechanism>,
    policy: &ActivationPolicy,
) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(mechanism) = mechanisms.get(grant.mechanism_id.as_str()) else {
        return vec![format!("{label}: unresolved mechanism_id `{}`", grant.mechanism_id)];
    };

    match policy.adapters.iter().find(|adapter| adapter.id == grant.adapter.id) {
        None => errors.push(format!("{label}: adapter `{}` is not a reviewed adapter", grant.adapter.id)),
        Some(adapter) => {
            if adapter.revision != grant.adapter.revision {
                errors.push(format!(
                    "{label}: adapter `{}` revision {} is unreviewed (reviewed revision is {})",
                    grant.adapter.id, grant.adapter.revision, adapter.revision
                ));
            }
            if adapter.provider != slug {
                errors.push(format!("{label}: adapter `{}` belongs to provider `{}`", adapter.id, adapter.provider));
            }
            if !adapter.mechanism_ids.contains(&grant.mechanism_id) {
                errors.push(format!("{label}: adapter `{}` does not bind this mechanism", adapter.id));
            }
        }
    }

    if grant.operation != mechanism.operation_intent {
        errors.push(format!(
            "{label}: operation `{}` does not match researched operation `{}`",
            grant.operation, mechanism.operation_intent
        ));
    }
    let semantic = mechanism
        .operation_intent
        .supports(grant.session_state, mechanism.conversation_effect);
    if semantic.is_none() {
        errors.push(format!(
            "{label}: operation `{}` with effect `{}` cannot serve a `{}` session",
            mechanism.operation_intent, mechanism.conversation_effect, grant.session_state
        ));
    }
    if !mechanism.receipt_timing.supports_prompt_return() {
        errors.push(format!(
            "{label}: receipt timing `{}` establishes no early acceptance",
            mechanism.receipt_timing
        ));
    }
    if !is_exact_version(&grant.provider_version) {
        errors.push(format!("{label}: provider_version `{}` is not one exact release", grant.provider_version));
    }

    let case = research.cases.iter().find(|case| {
        case.profile_id == grant.profile_id
            && case.os == grant.os
            && case.launch_mode == grant.launch_mode
            && case.origin == grant.origin
            && case.session_state == grant.session_state
    });
    match case {
        None => errors.push(format!(
            "{label}: no researched case for profile `{}` on {} ({}, {}, {})",
            grant.profile_id, grant.os, grant.launch_mode, grant.origin, grant.session_state
        )),
        Some(case) => {
            if !case.mechanism_ids.contains(&grant.mechanism_id) {
                errors.push(format!("{label}: researched case does not list this mechanism"));
            }
            if Some(case.support) != semantic || !matches!(case.support, CaseSupport::NonInterrupting | CaseSupport::InterruptionRequired) {
                errors.push(format!("{label}: researched case support is `{}`", case.support));
            }
        }
    }

    match research.access.iter().find(|access| {
        access.mechanism_id == grant.mechanism_id && access.profile_id == grant.profile_id && access.os == grant.os
    }) {
        None => errors.push(format!("{label}: no access finding for this profile and OS")),
        Some(access) if matches!(access.status, AccessStatus::Blocked | AccessStatus::Unknown) => {
            errors.push(format!("{label}: access is `{}`: {}", access.status, access.prerequisite));
        }
        Some(_) => {}
    }

    errors.extend(verification_errors(label, grant, research));
    errors
}

/// Required assertion coverage from exactly matching passed records.
/// A record documenting an expected loss never contributes coverage.
fn verification_errors(label: &str, grant: &PolicyGrant, research: &ResearchSteering) -> Vec<String> {
    let mut errors = Vec::new();
    if grant.verification_ids.is_empty() {
        errors.push(format!("{label}: names no verification record"));
    }
    let mut covered = BTreeSet::new();
    for id in &grant.verification_ids {
        let Some(record) = research.verification.iter().find(|record| &record.id == id) else {
            errors.push(format!("{label}: unresolved verification id `{id}`"));
            continue;
        };
        let mut mismatches = Vec::new();
        if record.mechanism_id != grant.mechanism_id { mismatches.push("mechanism"); }
        if record.profile_id != grant.profile_id { mismatches.push("profile"); }
        if record.os != grant.os { mismatches.push("OS"); }
        if record.provider_version != grant.provider_version { mismatches.push("provider version"); }
        if record.launch_mode != grant.launch_mode { mismatches.push("launch mode"); }
        if record.origin != grant.origin { mismatches.push("origin"); }
        if record.session_state != grant.session_state { mismatches.push("session state"); }
        if !mismatches.is_empty() {
            errors.push(format!("{label}: verification `{id}` does not apply ({} differ)", mismatches.join(", ")));
            continue;
        }
        if record.outcome != VerificationOutcome::Passed {
            errors.push(format!("{label}: verification `{id}` outcome is `{}`", record.outcome));
            continue;
        }
        if record.assertion_kinds.contains(&AssertionKind::ExpectedLoss) {
            errors.push(format!("{label}: verification `{id}` documents an expected loss and cannot activate delivery"));
            continue;
        }
        covered.extend(record.assertion_kinds.iter().copied());
    }
    match grant.operation.required_assertions() {
        None => errors.push(format!("{label}: operation `{}` can never be activated", grant.operation)),
        Some(required) => {
            let missing: Vec<_> = required.iter().filter(|kind| !covered.contains(kind)).map(|kind| kind.as_ref()).collect();
            if !missing.is_empty() {
                errors.push(format!("{label}: verification lacks required assertions: {}", missing.join(", ")));
            }
        }
    }
    errors
}

/// Policy entries naming a provider outside the active research roster.
pub fn orphan_policy_errors(active: &[String], policy: &ActivationPolicy) -> Vec<String> {
    let mut errors = Vec::new();
    for adapter in &policy.adapters {
        if !active.contains(&adapter.provider) {
            errors.push(format!("adapter `{}`: unknown provider `{}`", adapter.id, adapter.provider));
        }
    }
    for (index, grant) in policy.grants.iter().enumerate() {
        if !active.contains(&grant.provider) {
            errors.push(format!("grants[{index}]: unknown provider `{}`", grant.provider));
        }
    }
    for (index, block) in policy.blocks.iter().enumerate() {
        if !active.contains(&block.provider) {
            errors.push(format!("blocks[{index}]: unknown provider `{}`", block.provider));
        }
    }
    let mut ids = BTreeSet::new();
    for adapter in &policy.adapters {
        if !ids.insert(adapter.id.as_str()) {
            errors.push(format!("adapter `{}`: adapter id is reviewed more than once", adapter.id));
        }
    }
    errors.sort();
    errors.dedup();
    errors
}

// ---------------------------------------------------------------------------
// Generation: → lib/src/steering/generated.rs
// ---------------------------------------------------------------------------

/// Builds the generated steering catalog for every wired provider. Fails
/// when any policy entry does not pass activation applicability.
pub fn build_steering_catalog(area: &Path) -> Result<String, GenError> {
    let policy = load_activation_policy(area)?;
    let active = inputs::roster_active_slugs(area)?;
    let mut errors = orphan_policy_errors(&active, &policy);
    let mut providers = Vec::new();
    for slug in provider_slugs() {
        let research = load_research(area, slug)?;
        errors.extend(activation_errors(slug, &research, &policy).into_iter().map(|e| format!("{slug}: {e}")));
        providers.push((slug, research));
    }
    if !errors.is_empty() {
        return Err(GenError::SteeringActivationInvalid { errors: errors.join("\n") });
    }
    emit_file(&providers, &active, &policy)
}

/// Byte-compares [`build_steering_catalog`] with the committed file.
pub fn check_steering_catalog(area: &Path) -> Result<CheckOutcome, GenError> {
    let path = steering_catalog_path(area);
    let generated = build_steering_catalog(area)?;
    if !path.is_file() {
        return Ok(CheckOutcome::MissingCommitted { path });
    }
    let committed = std::fs::read_to_string(&path).map_err(|source| GenError::Io {
        path: path.clone(),
        source,
    })?;
    if committed == generated {
        return Ok(CheckOutcome::Clean);
    }
    Ok(CheckOutcome::Drift { details: diff_lines(&committed, &generated) })
}

fn variant<T: std::fmt::Debug>(ty: &str, value: T) -> String {
    format!("{ty}::{value:?}")
}

fn strs(items: &[String]) -> String {
    let rendered: Vec<String> = items.iter().map(|item| format!("{item:?}")).collect();
    format!("&[{}]", rendered.join(", "))
}

fn list<T: std::fmt::Debug + Copy>(ty: &str, items: &[T]) -> String {
    let rendered: Vec<String> = items.iter().map(|item| variant(ty, *item)).collect();
    format!("&[{}]", rendered.join(", "))
}

fn struct_slice(items: Vec<String>) -> String {
    if items.is_empty() {
        return "&[]".into();
    }
    let mut out = String::from("&[\n");
    for item in items {
        out.push_str(&item);
    }
    out.push_str("    ]");
    out
}

fn emit_provider(slug: &str, research: &ResearchSteering) -> Result<String, GenError> {
    let name = format!("{}_STEERING", slug.to_ascii_uppercase());
    let discovery = research.discovery.iter().map(|d| {
        format!(
            "        SteeringDiscovery {{ id: {:?}, profile_id: {:?}, os: {}, origin: {}, method: {}, prerequisites: {} }},\n",
            d.id,
            d.profile_id,
            variant("HostOs", d.os),
            variant("LaunchOrigin", d.origin),
            variant("DiscoveryMethod", d.method),
            strs(&d.prerequisites),
        )
    });
    let mechanisms = research.mechanisms.iter().map(|m| {
        format!(
            "        SteeringMechanism {{\n            id: {:?},\n            transport: {},\n            request_format: {:?},\n            operation_intent: {},\n            conversation_effect: {},\n            delivery_boundary: {},\n            receipt_timing: {},\n            receipts: ReceiptGuarantees {{\n                request_acceptance: {},\n                persistence: {},\n                scheduling: {},\n                conversation_delivery: {},\n            }},\n            delivery_states: {},\n        }},\n",
            m.id,
            variant("SteeringTransport", m.transport),
            m.request_format,
            variant("OperationIntent", m.operation_intent),
            variant("ConversationEffect", m.conversation_effect),
            variant("DeliveryBoundary", m.delivery_boundary),
            variant("ReceiptTiming", m.receipt_timing),
            variant("GuaranteeLevel", m.request_acceptance),
            variant("GuaranteeLevel", m.persistence),
            variant("GuaranteeLevel", m.scheduling),
            variant("GuaranteeLevel", m.conversation_delivery),
            list("DeliveryState", &m.delivery_states),
        )
    });
    let cases = research.cases.iter().map(|c| {
        format!(
            "        SteeringCase {{ profile_id: {:?}, os: {}, launch_mode: {}, origin: {}, state: {}, support: {}, mechanism_ids: {}, reason: {:?} }},\n",
            c.profile_id,
            variant("HostOs", c.os),
            variant("LaunchMode", c.launch_mode),
            variant("LaunchOrigin", c.origin),
            variant("ExecutionState", c.session_state),
            variant("CaseSupport", c.support),
            strs(&c.mechanism_ids),
            c.reason,
        )
    });
    let access = research.access.iter().map(|a| {
        format!(
            "        SteeringAccess {{ mechanism_id: {:?}, profile_id: {:?}, os: {}, status: {}, prerequisite: {:?} }},\n",
            a.mechanism_id,
            a.profile_id,
            variant("HostOs", a.os),
            variant("AccessStatus", a.status),
            a.prerequisite,
        )
    });
    let verification = research.verification.iter().map(|v| {
        format!(
            "        SteeringVerification {{\n            id: {:?},\n            mechanism_id: {:?},\n            profile_id: {:?},\n            os: {},\n            provider_version: {:?},\n            launch_mode: {},\n            origin: {},\n            state: {},\n            outcome: {},\n            assertions: {},\n        }},\n",
            v.id,
            v.mechanism_id,
            v.profile_id,
            variant("HostOs", v.os),
            v.provider_version,
            variant("LaunchMode", v.launch_mode),
            variant("LaunchOrigin", v.origin),
            variant("ExecutionState", v.session_state),
            variant("VerificationOutcome", v.outcome),
            list("AssertionKind", &v.assertion_kinds),
        )
    });
    let interfaces = research.execution_interfaces.iter().map(|i| {
        format!(
            "        ExecutionInterface {{ id: {:?}, kind: {} }},\n",
            i.id,
            variant("ExecutionInterfaceKind", i.kind),
        )
    });
    let fallback = match &research.fallback_interface {
        Some(fallback) => format!("Some({fallback:?})"),
        None => "None".into(),
    };
    Ok(format!(
        "static {name}: ProviderSteering = ProviderSteering {{\n    discovery: {},\n    mechanisms: {},\n    cases: {},\n    access: {},\n    verification: {},\n    execution_interfaces: {},\n    execution_selection: Some(ExecutionSelection {{ preferred: {:?}, fallback: {fallback} }}),\n}};\n",
        struct_slice(discovery.collect()),
        struct_slice(mechanisms.collect()),
        struct_slice(cases.collect()),
        struct_slice(access.collect()),
        struct_slice(verification.collect()),
        struct_slice(interfaces.collect()),
        research.preferred_interface,
    ))
}

fn emit_file(
    providers: &[(&str, ResearchSteering)],
    roster: &[String],
    policy: &ActivationPolicy,
) -> Result<String, GenError> {
    let mut out = String::from("// GENERATED by claudine-gen — DO NOT EDIT BY HAND.\n//\n// Inputs:\n");
    out.push_str("//   docs/providers.yaml (roster order)\n");
    for (slug, _) in providers {
        out.push_str(&format!("//   docs/research/{RESEARCH_TOPIC}/{slug}.md (researched steering facts)\n"));
        out.push_str(&format!("//   docs/research/{EXECUTION_TOPIC}/{slug}.md (execution interfaces)\n"));
    }
    out.push_str(&format!("//   {ACTIVATION_POLICY} (reviewed adapters, activation grants, and profile blocks)\n"));
    out.push_str(
        "// Regenerate with `cargo run -p claudine-gen -- generate`; drift-check with\n\
         // `cargo run -p claudine-gen -- check` (the same code path as the drift test).\n\n\
         //! Generated steering facts and the reviewed activation policy.\n\
         //!\n\
         //! [`provider_steering`] exposes researched facts. [`REVIEWED_ADAPTERS`],\n\
         //! [`ACTIVATION_GRANTS`], and [`PROFILE_BLOCKS`] are the separately reviewed\n\
         //! policy; a grant here has already passed the generator's exact\n\
         //! applicability check, but runtime eligibility still requires its adapter\n\
         //! to be implemented.\n\n\
         use claudine_catalog_types::steering::*;\n\n\
         use crate::provider_id::Provider;\n\n",
    );
    for (slug, research) in providers {
        out.push_str(&emit_provider(slug, research)?);
        out.push('\n');
    }
    out.push_str("/// Researched steering facts for `provider`.\npub(crate) fn provider_steering(provider: Provider) -> &'static ProviderSteering {\n    match provider {\n");
    for (slug, _) in providers {
        out.push_str(&format!(
            "        Provider::{} => &{}_STEERING,\n",
            provider_variant(slug)?,
            slug.to_ascii_uppercase()
        ));
    }
    out.push_str("    }\n}\n\n");

    // Roster entries not yet wired as a `Provider` variant are skipped; they
    // have no sessions to order.
    let wired: BTreeSet<&str> = providers.iter().map(|(slug, _)| *slug).collect();
    let ordered: Vec<String> = roster
        .iter()
        .filter(|slug| wired.contains(slug.as_str()))
        .map(|slug| Ok(format!("Provider::{}", provider_variant(slug)?)))
        .collect::<Result<_, GenError>>()?;
    out.push_str("/// Wired providers in `docs/providers.yaml` roster order.\n");
    out.push_str(&format!("pub(crate) static ROSTER_ORDER: &[Provider] = &[{}];\n\n", ordered.join(", ")));

    let mut adapters: Vec<_> = policy.adapters.iter().collect();
    adapters.sort_by(|a, b| (&a.id, a.revision).cmp(&(&b.id, b.revision)));
    let adapters: Vec<String> = adapters
        .iter()
        .map(|a| format!("    AdapterRef {{ id: {:?}, revision: {} }},\n", a.id, a.revision))
        .collect();
    out.push_str("/// Reviewed adapter implementations named by the activation policy.\n");
    out.push_str(&format!("pub(crate) static REVIEWED_ADAPTERS: &[AdapterRef] = {};\n\n", top_slice(adapters)));

    let grants: Vec<String> = policy
        .grants
        .iter()
        .map(|g| {
            format!(
                "    ActivationGrant {{\n        provider: {:?},\n        mechanism_id: {:?},\n        operation: {},\n        adapter: AdapterRef {{ id: {:?}, revision: {} }},\n        profile_id: {:?},\n        os: {},\n        provider_version: {:?},\n        launch_mode: {},\n        origin: {},\n        state: {},\n        verification_ids: {},\n    }},\n",
                g.provider,
                g.mechanism_id,
                variant("OperationIntent", g.operation),
                g.adapter.id,
                g.adapter.revision,
                g.profile_id,
                variant("HostOs", g.os),
                g.provider_version,
                variant("LaunchMode", g.launch_mode),
                variant("LaunchOrigin", g.origin),
                variant("ExecutionState", g.session_state),
                strs(&g.verification_ids),
            )
        })
        .collect();
    out.push_str("/// Reviewed exact activation grants.\n");
    out.push_str(&format!("pub(crate) static ACTIVATION_GRANTS: &[ActivationGrant] = {};\n\n", top_slice(grants)));

    let blocks: Vec<String> = policy
        .blocks
        .iter()
        .map(|b| {
            format!(
                "    ProfileBlock {{ provider: {:?}, profile_id: {:?}, reason: {:?} }},\n",
                b.provider, b.profile_id, b.reason
            )
        })
        .collect();
    out.push_str("/// Launch profiles the reviewed policy refuses to activate.\n");
    out.push_str(&format!("pub(crate) static PROFILE_BLOCKS: &[ProfileBlock] = {};\n", top_slice(blocks)));
    Ok(out)
}

fn top_slice(items: Vec<String>) -> String {
    if items.is_empty() {
        return "&[]".into();
    }
    format!("&[\n{}]", items.concat())
}
