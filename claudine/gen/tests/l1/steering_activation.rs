//! Deterministic activation applicability over the REAL Pi steering research.
//!
//! The control grant names Pi's passing active-steering fixture record and is
//! accepted; every other case edits exactly one dimension of it, so each
//! rejection is known to be caused by that edit. The policy parser is walked
//! through the input-robustness matrix separately.

use std::path::{Path, PathBuf};

use claudine_gen::steering_catalog::{
    ActivationPolicy, PolicyAdapterRef, PolicyGrant, ReviewedAdapter, activation_errors,
    load_research, orphan_policy_errors, parse_activation_policy, project_research,
};
use claudine_gen::GenError;

fn area() -> PathBuf {
    let root = biscuit_test_harness::manifest_dir!();
    // The repository inputs these tests read, spelled so CI schedules them.
    let _inputs = [
        root.join("../docs/research/steering"),
        root.join("../docs/research/non-interactive-sessions"),
        root.join("../docs/providers/steering-activation.yaml"),
        root.join("../docs/providers.yaml"),
    ];
    root.parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf()
}

fn adapter() -> ReviewedAdapter {
    ReviewedAdapter {
        id: "pi-rpc".into(),
        revision: 1,
        provider: "pi".into(),
        mechanism_ids: vec!["rpc-steer".into(), "rpc-idle-prompt".into()],
    }
}

fn control_grant() -> PolicyGrant {
    serde_yaml_ng::from_str(
        "provider: pi\n\
         mechanism_id: rpc-steer\n\
         operation: steer_active_turn\n\
         adapter: { id: pi-rpc, revision: 1 }\n\
         profile_id: retained-rpc\n\
         os: macos\n\
         provider_version: \"0.84.4\"\n\
         launch_mode: non_interactive\n\
         origin: native\n\
         session_state: working\n\
         verification_ids: [pi-rpc-steer-active-0844]\n",
    )
    .expect("control grant parses")
}

fn errors_for(grant: PolicyGrant, adapters: Vec<ReviewedAdapter>) -> Vec<String> {
    let research = load_research(&area(), "pi").expect("real Pi research projects");
    activation_errors("pi", &research, &ActivationPolicy { adapters, grants: vec![grant] })
}

fn assert_rejected(grant: PolicyGrant, needle: &str) {
    let errors = errors_for(grant, vec![adapter()]);
    assert!(
        errors.iter().any(|error| error.contains(needle)),
        "expected an error containing {needle:?}, got {errors:#?}"
    );
}

#[test]
fn control_grant_over_the_passing_fixture_is_accepted() {
    assert_eq!(errors_for(control_grant(), vec![adapter()]), Vec::<String>::new());
}

#[test]
fn every_single_dimension_edit_is_rejected_with_a_specific_reason() {
    let edit = |change: &dyn Fn(&mut PolicyGrant)| {
        let mut grant = control_grant();
        change(&mut grant);
        grant
    };
    assert_rejected(edit(&|g| g.verification_ids = vec!["no-such-record".into()]), "unresolved verification id `no-such-record`");
    assert_rejected(edit(&|g| g.verification_ids.clear()), "names no verification record");
    assert_rejected(edit(&|g| g.os = "windows".parse().unwrap()), "(OS differ)");
    assert_rejected(edit(&|g| g.origin = "claudine".parse().unwrap()), "(origin differ)");
    assert_rejected(edit(&|g| g.provider_version = "0.84.5".into()), "(provider version differ)");
    assert_rejected(edit(&|g| g.provider_version = "0.84.x".into()), "is not one exact release");
    assert_rejected(edit(&|g| g.provider_version = ">=0.84.4".into()), "is not one exact release");
    assert_rejected(edit(&|g| g.profile_id = "ordinary-cli".into()), "(profile differ)");
    assert_rejected(edit(&|g| g.session_state = "idle".parse().unwrap()), "cannot serve a `idle` session");
    assert_rejected(edit(&|g| g.operation = "queue_follow_up".parse().unwrap()), "does not match researched operation");
    assert_rejected(edit(&|g| g.adapter = PolicyAdapterRef { id: "pi-rpc".into(), revision: 2 }), "revision 2 is unreviewed");
    assert_rejected(edit(&|g| g.adapter = PolicyAdapterRef { id: "pi-ghost".into(), revision: 1 }), "is not a reviewed adapter");
    assert_rejected(edit(&|g| g.mechanism_id = "rpc-telepathy".into()), "unresolved mechanism_id `rpc-telepathy`");
}

#[test]
fn expected_loss_records_never_activate_delivery() {
    let mut grant = control_grant();
    grant.verification_ids = vec!["pi-rpc-steer-eof-0844".into()];
    let errors = errors_for(grant, vec![adapter()]);
    assert!(errors.iter().any(|e| e.contains("documents an expected loss")), "{errors:#?}");
    assert!(
        errors.iter().any(|e| e.contains("lacks required assertions: target_identity, acceptance_signal, conversation_delivery, running_work_preserved")),
        "{errors:#?}"
    );
}

#[test]
fn an_unrelated_passing_record_cannot_enable_delivery() {
    // The idle-prompt record passed, but it is a different mechanism and state.
    let mut grant = control_grant();
    grant.verification_ids = vec!["pi-rpc-idle-prompt-0844".into()];
    let errors = errors_for(grant, vec![adapter()]);
    assert!(errors.iter().any(|e| e.contains("does not apply (mechanism, session state differ)")), "{errors:#?}");
}

#[test]
fn adapter_must_bind_the_mechanism_and_belong_to_the_provider() {
    let mut narrow = adapter();
    narrow.mechanism_ids = vec!["rpc-idle-prompt".into()];
    let errors = errors_for(control_grant(), vec![narrow]);
    assert!(errors.iter().any(|e| e.contains("does not bind this mechanism")), "{errors:#?}");

    let mut foreign = adapter();
    foreign.provider = "codex".into();
    let errors = errors_for(control_grant(), vec![foreign]);
    assert!(errors.iter().any(|e| e.contains("belongs to provider `codex`")), "{errors:#?}");
}

#[test]
fn duplicate_grants_and_orphan_providers_are_rejected() {
    let research = load_research(&area(), "pi").unwrap();
    let policy = ActivationPolicy { adapters: vec![adapter()], grants: vec![control_grant(), control_grant()] };
    let errors = activation_errors("pi", &research, &policy);
    assert!(errors.iter().any(|e| e.contains("duplicate grant")), "{errors:#?}");

    let mut orphan = control_grant();
    orphan.provider = "copilot".into();
    let mut orphan_adapter = adapter();
    orphan_adapter.provider = "copilot".into();
    let errors = orphan_policy_errors(
        &["pi".to_string()],
        &ActivationPolicy { adapters: vec![orphan_adapter, adapter()], grants: vec![orphan] },
    );
    assert!(errors.iter().any(|e| e.contains("unknown provider `copilot`")), "{errors:#?}");
    assert!(errors.iter().any(|e| e.contains("reviewed more than once")), "{errors:#?}");
}

#[test]
fn malformed_research_references_refuse_projection() {
    let steering = claudine_gen::inputs::load_validated_frontmatter(
        &area().join("docs/research/steering/pi.md"),
    )
    .unwrap();
    let execution = claudine_gen::inputs::load_validated_frontmatter(
        &area().join("docs/research/non-interactive-sessions/pi.md"),
    )
    .unwrap();
    assert!(project_research("pi", &steering, &execution).is_ok(), "control projects");

    let mut missing_selection = execution.clone();
    missing_selection.as_object_mut().unwrap().remove("execution_selection");
    assert!(matches!(project_research("pi", &steering, &missing_selection), Err(GenError::SteeringResearchInvalid { .. })));

    let mut absent_fallback = execution.clone();
    absent_fallback["execution_selection"].as_object_mut().unwrap().remove("fallback");
    let error = project_research("pi", &steering, &absent_fallback).unwrap_err().to_string();
    assert!(error.contains("fallback"), "{error}");

    let mut null_interfaces = execution.clone();
    null_interfaces["execution_interfaces"] = serde_json::Value::Null;
    let error = project_research("pi", &steering, &null_interfaces).unwrap_err().to_string();
    assert!(error.contains("missing required `execution_interfaces`"), "{error}");

    let mut orphan_receipt = steering.clone();
    orphan_receipt["receipt_guarantees"].as_array_mut().unwrap().remove(0);
    let error = project_research("pi", &orphan_receipt, &execution).unwrap_err().to_string();
    assert!(error.contains("needs exactly one receipt_guarantees row, found 0"), "{error}");

    let mut bad_member = steering.clone();
    bad_member["receipt_guarantees"][0]["scheduling"] = "maybe".into();
    let error = project_research("pi", &bad_member, &execution).unwrap_err().to_string();
    assert!(error.contains("unknown member `maybe`"), "{error}");

    let mut bad_state = steering.clone();
    bad_state["cases"][0]["session_state"] = "asleep".into();
    assert!(project_research("pi", &bad_state, &execution).is_err());
}

/// Walks the input-robustness matrix for the projected `discovery` field and
/// its load-bearing members (`method`, `origin`, `os`, `prerequisites`), one
/// edit per cell on the real Pi research. Duplicate IDs are the relational
/// checker's cell (`steering_check::id_map`), which gates generation.
#[test]
fn discovery_projection_walks_the_input_robustness_matrix() {
    use serde_json::{Value, json};
    let steering = claudine_gen::inputs::load_validated_frontmatter(&area().join("docs/research/steering/pi.md")).unwrap();
    let execution =
        claudine_gen::inputs::load_validated_frontmatter(&area().join("docs/research/non-interactive-sessions/pi.md")).unwrap();

    // Control: every researched record projects with its typed method.
    let control = project_research("pi", &steering, &execution).expect("control projects");
    assert_eq!(control.discovery.len(), steering["discovery"].as_array().unwrap().len());
    let rpc = control.discovery.iter().find(|d| d.id == "rpc-macos-native").unwrap();
    assert_eq!(rpc.method, claudine_catalog_types::steering::DiscoveryMethod::ProviderApi);
    assert_eq!(rpc.prerequisites, vec!["owned RPC child".to_string()]);

    let edit = |mutate: &dyn Fn(&mut Value)| {
        let mut document = steering.clone();
        mutate(&mut document);
        project_research("pi", &document, &execution)
    };
    type Cell = (&'static str, Box<dyn Fn(&mut Value)>);
    let rejected: Vec<Cell> = vec![
        ("absent", Box::new(|d| drop(d.as_object_mut().unwrap().remove("discovery")))),
        ("explicit null", Box::new(|d| d["discovery"] = Value::Null)),
        ("wrong type, whole field", Box::new(|d| d["discovery"] = json!(123))),
        ("wrong type, one element", Box::new(|d| d["discovery"].as_array_mut().unwrap().push(json!(123)))),
        ("wrong type, every element", Box::new(|d| d["discovery"] = json!([123]))),
        ("method not in vocabulary", Box::new(|d| d["discovery"][0]["method"] = json!("guess"))),
        ("method null", Box::new(|d| d["discovery"][0]["method"] = Value::Null)),
        ("origin wrong type", Box::new(|d| d["discovery"][0]["origin"] = json!(1))),
        ("os absent", Box::new(|d| drop(d["discovery"][0].as_object_mut().unwrap().remove("os")))),
        ("prerequisites null", Box::new(|d| d["discovery"][0]["prerequisites"] = Value::Null)),
        ("prerequisites one wrong element", Box::new(|d| d["discovery"][0]["prerequisites"] = json!(["ok", 7]))),
        ("prerequisites every wrong element", Box::new(|d| d["discovery"][0]["prerequisites"] = json!([7]))),
    ];
    for (cell, mutate) in &rejected {
        assert!(
            matches!(edit(mutate.as_ref()), Err(GenError::SteeringResearchInvalid { .. })),
            "{cell} must be rejected, never read as empty or absent"
        );
    }

    // Empty is distinct from absent: no researched discovery, not an error.
    let empty = edit(&|d| d["discovery"] = json!([])).expect("empty list projects");
    assert!(empty.discovery.is_empty());
    let empty_prerequisites = edit(&|d| d["discovery"][0]["prerequisites"] = json!([])).unwrap();
    assert!(empty_prerequisites.discovery[0].prerequisites.is_empty());
}

const CONTROL_POLICY: &str = "\
adapters:
  - id: pi-rpc
    revision: 1
    provider: pi
    mechanism_ids: [rpc-steer]
grants:
  - provider: pi
    mechanism_id: rpc-steer
    operation: steer_active_turn
    adapter: { id: pi-rpc, revision: 1 }
    profile_id: retained-rpc
    os: macos
    provider_version: \"0.84.4\"
    launch_mode: non_interactive
    origin: native
    session_state: working
    verification_ids: [pi-rpc-steer-active-0844]
";

fn parse(text: &str) -> Result<ActivationPolicy, GenError> {
    parse_activation_policy(Path::new("steering-activation.yaml"), text)
}

/// Input-robustness matrix for the policy file. Load-bearing fields: the
/// `adapters` and `grants` lists, and each grant's `verification_ids`,
/// `provider_version`, and adapter `revision`. One edit per row.
#[test]
fn policy_parser_walks_the_input_robustness_matrix() {
    let control = parse(CONTROL_POLICY).expect("control policy parses");
    assert_eq!(control.grants.len(), 1);
    assert_eq!(control.grants[0].verification_ids, ["pi-rpc-steer-active-0844"]);

    let rejected: &[(&str, String)] = &[
        ("absent grants", CONTROL_POLICY.split("grants:").next().unwrap().to_string()),
        ("absent adapters", CONTROL_POLICY.replace("adapters:\n  - id: pi-rpc\n    revision: 1\n    provider: pi\n    mechanism_ids: [rpc-steer]\n", "")),
        ("null grants", "adapters: []\ngrants: null\n".into()),
        ("null grants (empty value)", "adapters: []\ngrants:\n".into()),
        ("grants wrong type", "adapters: []\ngrants: 123\n".into()),
        ("grants every element wrong type", "adapters: []\ngrants: [123]\n".into()),
        ("verification_ids one element wrong type", CONTROL_POLICY.replace("[pi-rpc-steer-active-0844]", "[pi-rpc-steer-active-0844, 7]")),
        ("verification_ids null", CONTROL_POLICY.replace("[pi-rpc-steer-active-0844]", "null")),
        ("provider_version as number", CONTROL_POLICY.replace("\"0.84.4\"", "0.84")),
        ("revision as string", CONTROL_POLICY.replace("revision: 1\n    provider", "revision: one\n    provider")),
        ("negative revision", CONTROL_POLICY.replace("revision: 1\n    provider", "revision: -1\n    provider")),
        ("unknown enum member", CONTROL_POLICY.replace("os: macos", "os: wsl")),
        ("unknown key", CONTROL_POLICY.replace("    os: macos\n", "    os: macos\n    trusted: true\n")),
        ("duplicate key", CONTROL_POLICY.replace("    os: macos\n", "    os: macos\n    os: linux\n")),
        ("trailing document", format!("{CONTROL_POLICY}---\nadapters: []\ngrants: []\n")),
        ("trailing garbage", format!("{CONTROL_POLICY}]]] not yaml")),
    ];
    for (label, text) in rejected {
        assert!(
            matches!(parse(text), Err(GenError::SteeringPolicyInvalid { .. })),
            "{label} must be rejected: {:?}",
            parse(text)
        );
    }

    // Empty lists are the defined "nothing reviewed" policy, distinct from
    // absence; an empty verification list parses but can never activate.
    let empty = parse("adapters: []\ngrants: []\n").unwrap();
    assert!(empty.adapters.is_empty() && empty.grants.is_empty());
    let unverified = parse(&CONTROL_POLICY.replace("[pi-rpc-steer-active-0844]", "[]")).unwrap();
    let research = load_research(&area(), "pi").unwrap();
    let errors = activation_errors("pi", &research, &unverified);
    assert!(errors.iter().any(|e| e.contains("names no verification record")), "{errors:#?}");
}

/// The committed policy file parses and every entry passes applicability
/// (it is also enforced by generation and the drift test).
#[test]
fn committed_policy_is_valid() {
    let policy = claudine_gen::load_activation_policy(&area()).expect("committed policy parses");
    let active = claudine_gen::inputs::roster_active_slugs(&area()).unwrap();
    assert!(orphan_policy_errors(&active, &policy).is_empty());
    for slug in &active {
        let research = load_research(&area(), slug).unwrap_or_else(|err| panic!("{slug}: {err}"));
        assert_eq!(activation_errors(slug, &research, &policy), Vec::<String>::new(), "{slug}");
    }
}
