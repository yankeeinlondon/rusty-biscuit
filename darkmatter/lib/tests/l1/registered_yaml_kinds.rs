//! Registered `kind: yaml-schema` definitions as the lowest tier of whole
//! YAML document selection (`DarkmatterSchemas::validate_yaml_document` with
//! registered hosts): a kind only a registered host claims is judged by its
//! definition, which names its host; shipped and local definitions keep
//! precedence and their failures; copies collapse; distinct definitions of
//! one kind refuse it; a selected kind outranks the root `$schema`; and a
//! YAML document gets no trigger layer.

use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::markdown::schemas::{DarkmatterSchemas, Route, YamlOutcome, YamlValidation};
use simplified_schema::{KindSelectionCause, SchemaError};
use simplified_schema::yaml_documents::LoadedYamlDocument;

const HOST: &str = "../host/darkmatter-host.yaml";

/// A registered definition of `kind: roster`.
const ROSTER: &str = "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: string[](required)\n";

/// `acme` with the `roster` definition.
fn roster_bundle() -> Bundle {
    Bundle::acme().with_file("schemas/roster.yaml", ROSTER)
}

/// The fixture with `acme` (plus `roster`) installed under `host/`.
fn roster_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.install_bundle("host", &roster_bundle());
    fixture
}

/// Validates `text`, written at `work/<name>`, in the repository context,
/// with `hosts` attached.
fn validate(
    fixture: &Fixture,
    hosts: &Arc<darkmatter::config::registration::RegisteredHosts>,
    name: &str,
    text: &str,
) -> Result<YamlOutcome, SchemaError> {
    let path = fixture.path(&format!("work/{name}"));
    fixture.write(&format!("work/{name}"), text);
    let context = fixture.repository(None).for_source(&path);
    DarkmatterSchemas::new(context).with_registered_hosts(Arc::clone(hosts)).validate_yaml_document(text, &path)
}

fn validated(outcome: Result<YamlOutcome, SchemaError>) -> YamlValidation {
    match outcome {
        Ok(YamlOutcome::Validated(validation)) => validation,
        other => panic!("a validated document: {other:?}"),
    }
}

fn problem_paths(validation: &YamlValidation) -> Vec<String> {
    validation.report.problems.iter().map(|problem| problem.path.clone()).collect()
}

fn refusal(outcome: Result<YamlOutcome, SchemaError>) -> (String, KindSelectionCause) {
    match outcome {
        Err(SchemaError::KindSelection { kind, cause }) => (kind, cause),
        other => panic!("a kind-selection refusal: {other:?}"),
    }
}

#[test]
fn a_kind_only_a_registered_host_claims_is_judged_by_its_definition_which_names_the_host() {
    let fixture = roster_fixture();
    let hosts = fixture.hosts(&[HOST]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());

    let valid = validated(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: [linux, macos]\n"));
    assert_eq!(valid.route, Route::Kind("roster".into()));
    assert!(valid.report.valid, "{:#?}", valid.report.problems);

    let invalid = validated(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: 3\n"));
    assert_eq!(problem_paths(&invalid), ["/platforms"]);
    // The problem is located in the document's own text.
    let problem = &invalid.report.problems[0];
    let range = invalid.byte_range(problem).expect("located");
    assert_eq!(&"kind: roster\nplatforms: 3\n"[range], "3");

    // The prepared contract carries the host and the definition's identity.
    let path = fixture.path("work/team.yaml");
    let text = "kind: roster\nplatforms: []\n";
    let document = LoadedYamlDocument::load(text, &path).unwrap();
    let contract = DarkmatterSchemas::new(fixture.repository(None).for_source(&path))
        .with_registered_hosts(Arc::clone(&hosts))
        .prepare_yaml_contract(&document)
        .unwrap()
        .expect("a contract");
    let registered = contract.registered().expect("a registered definition");
    assert_eq!(registered.host, "acme");
    assert_eq!(registered.path, "schemas/roster.yaml");
    assert_eq!(registered.file, fixture.path("host/schemas/roster.yaml"));
}

#[test]
fn without_registered_hosts_the_kind_is_unclaimed_and_nothing_is_checked() {
    let fixture = roster_fixture();
    let path = fixture.path("work/team.yaml");
    let context = fixture.repository(None).for_source(&path);
    let outcome = DarkmatterSchemas::new(context).validate_yaml_document("kind: roster\nplatforms: 3\n", &path).unwrap();
    assert!(matches!(outcome, YamlOutcome::NoSchema), "{outcome:?}");
}

#[test]
fn a_selected_registered_kind_outranks_the_root_schema_and_an_unclaimed_kind_falls_through_to_it() {
    let fixture = roster_fixture();
    let hosts = fixture.hosts(&[HOST]);
    // The root `$schema` would accept a number; the registered kind does not.
    let text = "kind: roster\n$schema:\n    platforms: number\nplatforms: 3\n";
    let over = validated(validate(&fixture, &hosts, "team.yaml", text));
    assert_eq!(over.route, Route::Kind("roster".into()));
    assert!(problem_paths(&over).contains(&"/platforms".to_string()), "{:#?}", over.report.problems);

    // No definition claims `crew`: the root `$schema` is the contract.
    let text = "kind: crew\n$schema:\n    platforms: number\nplatforms: 3\n";
    let through = validated(validate(&fixture, &hosts, "crew.yaml", text));
    assert_eq!(through.route, Route::RootSchema);
    assert!(through.report.valid, "{:#?}", through.report.problems);
}

#[test]
fn a_local_definition_claiming_the_kind_wins_over_the_registered_one() {
    let fixture = roster_fixture();
    let hosts = fixture.hosts(&[HOST]);
    fixture.write("work/schemas/roster.yaml", "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: number(required)\n");

    let validation = validated(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: 3\n"));
    assert!(validation.report.valid, "the local definition judges: {:#?}", validation.report.problems);
    assert_eq!(validation.definition, fixture.path("work/schemas/roster.yaml").display().to_string());
}

#[test]
fn a_shipped_kind_is_never_a_registered_hosts() {
    let fixture = Fixture::new();
    // A registered definition claiming the shipped `schema` kind.
    fixture.install_bundle(
        "host",
        &Bundle::acme().with_file("schemas/claims-schema.yaml", "kind: yaml-schema\n$schema:\n    kind: literal(schema; required)\n    extra: number(required)\n"),
    );
    let hosts = fixture.hosts(&[HOST]);

    let validation = validated(validate(&fixture, &hosts, "types.yaml", "kind: schema\ntypes:\n    name: string\n"));
    assert_eq!(validation.route, Route::Kind("schema".into()));
    assert!(validation.report.valid, "the shipped definition judges: {:#?}", validation.report.problems);
}

#[test]
fn an_incomplete_local_definition_keeps_the_kind_unavailable_over_a_registered_claim() {
    let fixture = roster_fixture();
    let hosts = fixture.hosts(&[HOST]);
    let broken = fixture.path("work/schemas/broken.yaml");
    fixture.write("work/schemas/broken.yaml", "kind: yaml-schema\n$schema: [unclosed\n");

    let (kind, cause) = refusal(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: []\n"));
    assert_eq!(kind, "roster");
    assert!(matches!(&cause, KindSelectionCause::Unavailable { file, .. } if *file == broken), "{cause:?}");
}

#[test]
fn an_identical_copy_of_the_host_collapses_into_one_claim() {
    let fixture = roster_fixture();
    fixture.install_bundle("moved/acme", &roster_bundle());
    let hosts = fixture.hosts(&[HOST, "../moved/acme/darkmatter-host.yaml"]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());

    let validation = validated(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: [linux]\n"));
    assert!(validation.report.valid, "{:#?}", validation.report.problems);
}

#[test]
fn distinct_definitions_of_one_kind_refuse_it_naming_each_within_and_across_hosts() {
    // Across hosts: `acme` and `beta` each define `roster` differently.
    let fixture = roster_fixture();
    let mut beta = Bundle::acme_named("beta").with_file("schemas/roster.yaml", &ROSTER.replace("string[]", "number[]"));
    beta.version = "0.1.0".into();
    fixture.install_bundle("beta", &beta);
    let hosts = fixture.hosts(&[HOST, "../beta/darkmatter-host.yaml"]);
    let (kind, cause) = refusal(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: []\n"));
    assert_eq!(kind, "roster");
    let KindSelectionCause::Duplicate { files } = cause else { panic!("a duplicate: {cause:?}") };
    assert_eq!(files, [fixture.path("host/schemas/roster.yaml"), fixture.path("beta/schemas/roster.yaml")], "host order");

    // Within one host: two of its files claim `roster`.
    let fixture = Fixture::new();
    fixture.install_bundle("host", &roster_bundle().with_file("schemas/roster-two.yaml", &ROSTER.replace("string[]", "number[]")));
    let hosts = fixture.hosts(&[HOST]);
    let (_, cause) = refusal(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: []\n"));
    let KindSelectionCause::Duplicate { files } = cause else { panic!("a duplicate: {cause:?}") };
    assert_eq!(files, [fixture.path("host/schemas/roster-two.yaml"), fixture.path("host/schemas/roster.yaml")], "path order");
}

#[test]
fn a_registered_yaml_document_is_judged_whole_with_no_trigger_layer() {
    let fixture = roster_fixture();
    let hosts = fixture.hosts(&[HOST]);
    // `gadget: 5` would break the registered gadget trigger's `string`; a
    // YAML document gets no trigger layer, and the definition is open.
    let validation = validated(validate(&fixture, &hosts, "team.yaml", "kind: roster\nplatforms: []\ngadget: 5\n"));
    assert!(validation.report.valid, "{:#?}", validation.report.problems);
    assert!(validation.effective().layers().triggers.is_empty());
    assert!(validation.effective().layers().registered.is_none());
}
