//! Deterministic relational validation for steering research.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::errors::GenError;
use crate::inputs;

const TOPIC: &str = "steering";
const CONCRETE_EVIDENCE: &[&str] = &[
    "official_docs",
    "source_code",
    "local_inspection",
    "disposable_test",
];

/// Result of validating one provider's steering research document.
#[derive(Debug, Clone, Serialize)]
pub struct SteeringValidation {
    /// Provider slug validated.
    pub provider: String,
    /// Number of declared launch profiles.
    pub profiles: usize,
    /// Number of declared cases.
    pub cases: usize,
    /// Number of declared mechanisms.
    pub mechanisms: usize,
    /// Number of evidence records.
    pub evidence: usize,
    /// Number of live verification records, regardless of outcome.
    pub verification_records: usize,
    /// Number of live verification records whose outcome is `passed`.
    pub passed_verification_records: usize,
    /// Deterministically ordered validation findings.
    pub errors: Vec<String>,
}

impl SteeringValidation {
    /// Whether all deterministic checks passed.
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate one schema-v3 steering document, including cross-record references
/// and the case products that a shape schema cannot express.
pub fn check_provider(area: &Path, slug: &str) -> Result<SteeringValidation, GenError> {
    let path = area.join(format!("docs/research/{TOPIC}/{slug}.md"));
    let document = inputs::load_validated_frontmatter(&path)?;
    let mut result = evaluate(slug, &document);
    let execution_path = area.join(format!(
        "docs/research/non-interactive-sessions/{slug}.md"
    ));
    if execution_path.is_file() {
        let execution = inputs::load_validated_frontmatter(&execution_path)?;
        validate_execution_references(slug, &execution, &document, &mut result.errors);
        result.errors.sort();
        result.errors.dedup();
    }
    Ok(result)
}

/// Validate every active research provider in roster order.
pub fn check_fleet(area: &Path) -> Result<Vec<SteeringValidation>, GenError> {
    inputs::roster_active_slugs(area)?
        .into_iter()
        .map(|slug| check_provider(area, &slug))
        .collect()
}

fn evaluate(slug: &str, document: &Value) -> SteeringValidation {
    let mut errors = Vec::new();
    let revision = document.get("schema_revision").and_then(Value::as_u64);
    if revision != Some(3) {
        errors.push(format!("schema_revision must be 3, found {revision:?}"));
    }
    if document.get("provider").and_then(Value::as_str) != Some(slug) {
        errors.push(format!("provider must match document slug `{slug}`"));
    }

    let profiles = rows(document, "launch_profiles");
    let evidence = rows(document, "evidence");
    let discovery = rows(document, "discovery");
    let mechanisms = rows(document, "mechanisms");
    let compatibility = rows(document, "compatibility");
    let verification = rows(document, "verification");
    let cases = rows(document, "cases");
    let access = rows(document, "access_findings");
    let receipts = rows(document, "receipt_guarantees");
    let delivery = rows(document, "delivery_states");
    let observations = rows(document, "receipt_observations");
    let inventory = rows(document, "interface_inventory");
    let discovery_gaps = rows(document, "discovery_gaps");

    let profile_map = id_map("launch_profiles", profiles, &mut errors);
    let evidence_map = id_map("evidence", evidence, &mut errors);
    let discovery_map = id_map("discovery", discovery, &mut errors);
    let mechanism_map = id_map("mechanisms", mechanisms, &mut errors);

    for (group, group_rows) in [
        ("launch_profiles", profiles),
        ("discovery", discovery),
        ("mechanisms", mechanisms),
        ("compatibility", compatibility),
        ("verification", verification),
        ("cases", cases),
        ("access_findings", access),
        ("receipt_guarantees", receipts),
        ("delivery_states", delivery),
        ("receipt_observations", observations),
        ("interface_inventory", inventory),
    ] {
        check_references(group, group_rows, &profile_map, &mechanism_map, &evidence_map, &mut errors);
    }

    validate_case_products(profiles, cases, &mut errors);
    validate_mechanism_coverage("receipt_guarantees", receipts, &mechanism_map, &mut errors);
    validate_mechanism_coverage("delivery_states", delivery, &mechanism_map, &mut errors);
    validate_mechanism_coverage("receipt_observations", observations, &mechanism_map, &mut errors);
    validate_inventory(inventory, &profile_map, &evidence_map, &mut errors);
    validate_cases(
        cases,
        access,
        compatibility,
        discovery_gaps,
        &discovery_map,
        &mechanism_map,
        &evidence_map,
        &mut errors,
    );

    errors.sort();
    errors.dedup();
    SteeringValidation {
        provider: slug.to_string(),
        profiles: profiles.len(),
        cases: cases.len(),
        mechanisms: mechanisms.len(),
        evidence: evidence.len(),
        verification_records: verification.len(),
        passed_verification_records: verification
            .iter()
            .filter(|row| row.get("outcome").and_then(Value::as_str) == Some("passed"))
            .count(),
        errors,
    }
}

fn rows<'a>(document: &'a Value, key: &str) -> &'a [Value] {
    document.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn id_map<'a>(group: &str, rows: &'a [Value], errors: &mut Vec<String>) -> BTreeMap<&'a str, &'a Value> {
    let mut result = BTreeMap::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue };
        if result.insert(id, row).is_some() {
            errors.push(format!("{group}: duplicate id `{id}`"));
        }
    }
    result
}

fn strings<'a>(row: &'a Value, key: &str) -> Vec<&'a str> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn check_references(
    group: &str,
    rows: &[Value],
    profiles: &BTreeMap<&str, &Value>,
    mechanisms: &BTreeMap<&str, &Value>,
    evidence: &BTreeMap<&str, &Value>,
    errors: &mut Vec<String>,
) {
    for (index, row) in rows.iter().enumerate() {
        for id in strings(row, "evidence_ids") {
            if !evidence.contains_key(id) {
                errors.push(format!("{group}[{index}]: unresolved evidence_id `{id}`"));
            }
        }
        if let Some(id) = row.get("profile_id").and_then(Value::as_str)
            && !profiles.contains_key(id)
        {
            errors.push(format!("{group}[{index}]: unresolved profile_id `{id}`"));
        }
        for id in strings(row, "profile_ids") {
            if !profiles.contains_key(id) {
                errors.push(format!("{group}[{index}]: unresolved profile_id `{id}`"));
            }
        }
        if let Some(id) = row.get("mechanism_id").and_then(Value::as_str)
            && !mechanisms.contains_key(id)
        {
            errors.push(format!("{group}[{index}]: unresolved mechanism_id `{id}`"));
        }
        for id in strings(row, "mechanism_ids") {
            if !mechanisms.contains_key(id) {
                errors.push(format!("{group}[{index}]: unresolved mechanism_id `{id}`"));
            }
        }
    }
}

type CaseKey = (String, String, String, String, String);

fn case_key(row: &Value) -> Option<CaseKey> {
    Some((
        row.get("profile_id")?.as_str()?.to_string(),
        row.get("os")?.as_str()?.to_string(),
        row.get("launch_mode")?.as_str()?.to_string(),
        row.get("origin")?.as_str()?.to_string(),
        row.get("session_state")?.as_str()?.to_string(),
    ))
}

fn validate_case_products(profiles: &[Value], cases: &[Value], errors: &mut Vec<String>) {
    let actual: Vec<_> = cases.iter().filter_map(case_key).collect();
    let actual_set: BTreeSet<_> = actual.iter().cloned().collect();
    if actual.len() != actual_set.len() {
        errors.push("cases: duplicate profile/OS/launch/origin/state tuple".into());
    }
    let mut baseline = Vec::new();
    for profile in profiles {
        let Some(id) = profile.get("id").and_then(Value::as_str) else { continue };
        for field in ["applicable_os", "applicable_launch_modes", "applicable_origins"] {
            let values = strings(profile, field);
            let unique: BTreeSet<_> = values.iter().copied().collect();
            if values.is_empty() || values.len() != unique.len() {
                errors.push(format!("profile `{id}`: `{field}` must be nonempty and unique"));
            }
        }
        let mut expected = BTreeSet::new();
        for os in strings(profile, "applicable_os") {
            for launch in strings(profile, "applicable_launch_modes") {
                for origin in strings(profile, "applicable_origins") {
                    for state in ["working", "idle"] {
                        expected.insert((id.into(), os.into(), launch.into(), origin.into(), state.into()));
                    }
                }
            }
        }
        let found: BTreeSet<_> = actual_set.iter().filter(|key| key.0 == id).cloned().collect();
        if expected != found {
            errors.push(format!("profile `{id}` cases do not equal its declared Cartesian product"));
        }
        if profile.get("baseline").and_then(Value::as_bool) == Some(true) {
            baseline.extend(found);
        }
    }
    let expected_baseline: BTreeSet<_> = ["macos", "linux", "windows"]
        .into_iter()
        .flat_map(|os| ["interactive", "non_interactive"].into_iter().flat_map(move |launch| {
            ["native", "claudine"].into_iter().flat_map(move |origin| {
                ["working", "idle"].into_iter().map(move |state| (os, launch, origin, state))
            })
        }))
        .collect();
    let projected: BTreeSet<_> = baseline.iter().map(|k| (k.1.as_str(), k.2.as_str(), k.3.as_str(), k.4.as_str())).collect();
    if baseline.len() != 24 || projected != expected_baseline {
        errors.push("baseline profiles must cover each of the 24 OS/launch/origin/state tuples exactly once".into());
    }
}

fn validate_mechanism_coverage(group: &str, rows: &[Value], mechanisms: &BTreeMap<&str, &Value>, errors: &mut Vec<String>) {
    let ids: Vec<_> = rows.iter().filter_map(|r| r.get("mechanism_id")?.as_str()).collect();
    let found: BTreeSet<_> = ids.iter().copied().collect();
    let expected: BTreeSet<_> = mechanisms.keys().copied().collect();
    if ids.len() != found.len() || found != expected {
        errors.push(format!("{group}: requires exactly one row per mechanism"));
    }
}

fn validate_inventory(inventory: &[Value], profiles: &BTreeMap<&str, &Value>, evidence: &BTreeMap<&str, &Value>, errors: &mut Vec<String>) {
    let mut ids = BTreeSet::new();
    let mut included = BTreeSet::new();
    for row in inventory {
        if let Some(id) = row.get("id").and_then(Value::as_str)
            && !ids.insert(id)
        {
            errors.push(format!("interface_inventory: duplicate id `{id}`"));
        }
        if row.get("disposition").and_then(Value::as_str) == Some("included") {
            let profile_ids = strings(row, "profile_ids");
            if profile_ids.is_empty() {
                errors.push("interface_inventory: included row must name at least one profile".into());
            }
            included.extend(profile_ids);
        }
        require_concrete_evidence("interface_inventory", row, evidence, errors);
    }
    if included != profiles.keys().copied().collect() {
        errors.push("interface_inventory: included rows must cover every launch profile".into());
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_cases(
    cases: &[Value], access: &[Value], compatibility: &[Value], gaps: &[Value],
    discovery: &BTreeMap<&str, &Value>, mechanisms: &BTreeMap<&str, &Value>, evidence: &BTreeMap<&str, &Value>, errors: &mut Vec<String>,
) {
    let access_keys = unique_relation_keys("access_findings", access, errors);
    let compatibility_keys = unique_relation_keys("compatibility", compatibility, errors);
    let gap_keys: BTreeSet<_> = gaps.iter().filter_map(case_key).collect();
    if gap_keys.len() != gaps.len() {
        errors.push("discovery_gaps: duplicate or incomplete case key".into());
    }
    let case_keys: BTreeSet<_> = cases.iter().filter_map(case_key).collect();
    for gap in &gap_keys {
        if !case_keys.contains(gap) {
            errors.push(format!("discovery_gaps: orphan case key {gap:?}"));
        }
    }
    for (index, case) in cases.iter().enumerate() {
        let Some(key) = case_key(case) else { continue };
        for discovery_id in strings(case, "discovery_ids") {
            let Some(record) = discovery.get(discovery_id) else {
                errors.push(format!("cases[{index}]: unresolved discovery_id `{discovery_id}`"));
                continue;
            };
            for (field, expected) in [("profile_id", key.0.as_str()), ("os", key.1.as_str()), ("origin", key.3.as_str())] {
                if record.get(field).and_then(Value::as_str) != Some(expected) {
                    errors.push(format!("cases[{index}]: discovery `{discovery_id}` mismatches `{field}`"));
                }
            }
        }
        for mechanism_id in strings(case, "mechanism_ids") {
            let relation = (mechanism_id, key.0.as_str(), key.1.as_str());
            if !access_keys.contains(&relation) { errors.push(format!("cases[{index}]: missing access finding for {relation:?}")); }
            if !compatibility_keys.contains(&relation) { errors.push(format!("cases[{index}]: missing compatibility row for {relation:?}")); }
        }
        let support = case.get("support").and_then(Value::as_str).unwrap_or("unknown");
        if matches!(support, "non_interrupting" | "interruption_required") {
            if strings(case, "mechanism_ids").is_empty() { errors.push(format!("cases[{index}]: supported case lacks a mechanism")); }
            require_concrete_evidence(&format!("cases[{index}]"), case, evidence, errors);
            if strings(case, "discovery_ids").is_empty() && !gap_keys.contains(&key) {
                errors.push(format!("cases[{index}]: supported case without discovery needs an exact discovery_gap"));
            }
            validate_operation_support(index, case, mechanisms, errors);
        }
    }
}

fn unique_relation_keys<'a>(group: &str, rows: &'a [Value], errors: &mut Vec<String>) -> BTreeSet<(&'a str, &'a str, &'a str)> {
    let keys: Vec<_> = rows.iter().filter_map(|row| Some((row.get("mechanism_id")?.as_str()?, row.get("profile_id")?.as_str()?, row.get("os")?.as_str()?))).collect();
    let unique: BTreeSet<_> = keys.iter().copied().collect();
    if unique.len() != keys.len() {
        errors.push(format!("{group}: duplicate mechanism/profile/OS row"));
    }
    unique
}

fn require_concrete_evidence(label: &str, row: &Value, evidence: &BTreeMap<&str, &Value>, errors: &mut Vec<String>) {
    let concrete = strings(row, "evidence_ids").into_iter().any(|id| {
        evidence.get(id).and_then(|record| record.get("method")).and_then(Value::as_str).is_some_and(|method| CONCRETE_EVIDENCE.contains(&method))
    });
    if !concrete { errors.push(format!("{label}: requires at least one non-inference evidence record")); }
}

fn validate_operation_support(index: usize, case: &Value, mechanisms: &BTreeMap<&str, &Value>, errors: &mut Vec<String>) {
    let support = case.get("support").and_then(Value::as_str).unwrap_or("unknown");
    let state = case.get("session_state").and_then(Value::as_str).unwrap_or("unknown");
    let mut eligible = false;
    for mechanism_id in strings(case, "mechanism_ids") {
        let Some(mechanism) = mechanisms.get(mechanism_id) else { continue };
        let intent = mechanism.get("operation_intent").and_then(Value::as_str).unwrap_or("unknown");
        let effect = mechanism.get("conversation_effect").and_then(Value::as_str).unwrap_or("unknown");
        let valid = match (support, state) {
            ("non_interrupting", "working") => matches!(intent, "steer_active_turn" | "queue_follow_up") && effect == "preserve_running_turn",
            ("non_interrupting", "idle") => intent == "start_idle_turn" && matches!(effect, "preserve_running_turn" | "resume_same_conversation"),
            ("interruption_required", "working") => intent == "interrupt_then_submit" && effect == "cancel_turn_same_conversation",
            _ => false,
        };
        eligible |= valid;
    }
    if !eligible { errors.push(format!("cases[{index}]: no listed mechanism semantics support {support} for {state} state")); }
}

fn validate_execution_references(
    slug: &str,
    execution: &Value,
    steering: &Value,
    errors: &mut Vec<String>,
) {
    let interfaces = rows(execution, "execution_interfaces");
    let interface_map = id_map("execution_interfaces", interfaces, errors);
    let mechanism_ids: BTreeSet<_> = rows(steering, "mechanisms")
        .iter()
        .filter_map(|row| row.get("id")?.as_str())
        .collect();

    if let Some(selection) = execution.get("execution_selection") {
        validate_interface_reference(
            "execution_selection.preferred",
            selection.get("preferred").and_then(Value::as_str),
            false,
            &interface_map,
            errors,
        );
        validate_interface_reference(
            "execution_selection.fallback",
            selection.get("fallback").and_then(Value::as_str),
            true,
            &interface_map,
            errors,
        );
    }
    for group in ["unattended_requests", "settlement", "steering_mechanisms"] {
        for (index, row) in rows(execution, group).iter().enumerate() {
            validate_interface_reference(
                &format!("{group}[{index}].interface"),
                row.get("interface").and_then(Value::as_str),
                false,
                &interface_map,
                errors,
            );
        }
    }

    let expected_reference = format!("../steering/{slug}.md");
    for (index, row) in rows(execution, "steering_mechanisms").iter().enumerate() {
        let mechanism = row.get("mechanism").and_then(Value::as_str).unwrap_or("");
        if !mechanism_ids.contains(mechanism) {
            errors.push(format!(
                "steering_mechanisms[{index}]: unresolved same-provider mechanism `{mechanism}`"
            ));
        }
        let reference = row.get("reference").and_then(Value::as_str).unwrap_or("");
        if reference.split('#').next() != Some(expected_reference.as_str()) {
            errors.push(format!(
                "steering_mechanisms[{index}]: reference must target `{expected_reference}`"
            ));
        }
    }
}

fn validate_interface_reference(
    label: &str,
    reference: Option<&str>,
    empty_allowed: bool,
    interfaces: &BTreeMap<&str, &Value>,
    errors: &mut Vec<String>,
) {
    let reference = reference.unwrap_or("");
    if reference.is_empty() && empty_allowed {
        return;
    }
    if reference.is_empty() || !interfaces.contains_key(reference) {
        errors.push(format!("{label}: unresolved execution interface `{reference}`"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_next_turn_as_non_interrupting_follow_up() {
        let mechanism = serde_json::json!({"operation_intent":"queue_follow_up","conversation_effect":"preserve_running_turn","delivery_boundary":"next_turn"});
        let case = serde_json::json!({"support":"non_interrupting","session_state":"working","mechanism_ids":["follow"]});
        let mechanisms = BTreeMap::from([("follow", &mechanism)]);
        let mut errors = Vec::new();
        validate_operation_support(0, &case, &mechanisms, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn accepts_active_turn_preserving_delivery() {
        let mechanism = serde_json::json!({"operation_intent":"steer_active_turn","conversation_effect":"preserve_running_turn","delivery_boundary":"next_tool_boundary"});
        let case = serde_json::json!({"support":"non_interrupting","session_state":"working","mechanism_ids":["steer"]});
        let mechanisms = BTreeMap::from([("steer", &mechanism)]);
        let mut errors = Vec::new();
        validate_operation_support(0, &case, &mechanisms, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn reports_missing_mechanism_evidence_and_profile_references() {
        let row = serde_json::json!({
            "mechanism_id":"missing-mechanism",
            "profile_ids":["missing-profile"],
            "evidence_ids":["missing-evidence"]
        });
        let mut errors = Vec::new();
        check_references(
            "interface_inventory",
            &[row],
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &mut errors,
        );
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn requires_an_exact_discovery_gap_for_a_supported_case() {
        let mechanism = serde_json::json!({
            "operation_intent":"steer_active_turn",
            "conversation_effect":"preserve_running_turn",
            "delivery_boundary":"during_generation"
        });
        let evidence_record = serde_json::json!({"method":"source_code"});
        let case = serde_json::json!({
            "profile_id":"managed", "os":"macos", "launch_mode":"interactive",
            "origin":"native", "session_state":"working", "support":"non_interrupting",
            "discovery_ids":[], "mechanism_ids":["steer"], "evidence_ids":["source"]
        });
        let wrong_gap = serde_json::json!({
            "profile_id":"managed", "os":"macos", "launch_mode":"interactive",
            "origin":"native", "session_state":"idle"
        });
        let access = serde_json::json!({"mechanism_id":"steer","profile_id":"managed","os":"macos"});
        let compatibility = access.clone();
        let mechanisms = BTreeMap::from([("steer", &mechanism)]);
        let evidence = BTreeMap::from([("source", &evidence_record)]);
        let mut errors = Vec::new();
        validate_cases(
            &[case], &[access], &[compatibility], &[wrong_gap],
            &BTreeMap::new(), &mechanisms, &evidence, &mut errors,
        );
        assert!(errors.iter().any(|error| error.contains("exact discovery_gap")));
    }

    #[test]
    fn receipt_observations_require_exact_mechanism_coverage() {
        let first = serde_json::json!({"mechanism_id":"steer"});
        let duplicate = first.clone();
        let steer = serde_json::json!({});
        let idle = serde_json::json!({});
        let mechanisms = BTreeMap::from([("steer", &steer), ("idle", &idle)]);
        let mut errors = Vec::new();
        validate_mechanism_coverage(
            "receipt_observations",
            &[first, duplicate],
            &mechanisms,
            &mut errors,
        );
        assert_eq!(errors, ["receipt_observations: requires exactly one row per mechanism"]);
    }

    #[test]
    fn execution_contract_rejects_invalid_interface_and_steering_foreign_keys() {
        let execution = serde_json::json!({
            "execution_interfaces":[{"id":"exec"}],
            "execution_selection":{"preferred":"missing","fallback":""},
            "unattended_requests":[{"interface":"missing"}],
            "settlement":[{"interface":"exec"}],
            "steering_mechanisms":[{
                "interface":"exec",
                "mechanism":"foreign",
                "reference":"../steering/other.md"
            }]
        });
        let steering = serde_json::json!({"mechanisms":[{"id":"local"}]});
        let mut errors = Vec::new();
        validate_execution_references("codex", &execution, &steering, &mut errors);
        assert_eq!(errors.len(), 4);
    }
}
