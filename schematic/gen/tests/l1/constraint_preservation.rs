//! Acceptance: `import --definitions-out` followed by assembly rebuilds every
//! value rule the direct import keeps, compared by meaning, and the rebuilt
//! definitions pass the generator's validation. The direct API's compiled
//! client accepts and refuses requests at every free-form position, and the
//! rebuilt API generates the same plans for those positions.
//!
//! The direct import/export route is `schematic-define`'s
//! `constraint_preservation` test, over the same fixture and with the same
//! meaning comparison (`define/tests/support/constraint_meaning.rs`), which
//! the generated crate's test includes as source.

use biscuit_test_harness::manifest_dir;
use schematic_define::constraints::referenced_component;
use schematic_define::openapi::{OpenApiImport, OpenApiSource};
use schematic_gen::import_pipeline::{ImportOptions, run_import};
use schematic_gen::output::generate_and_write_all;
use schematic_gen::validation::validate_rust_client;

use crate::common::fresh_output::{Definitions, FreshCrate};

#[test]
fn slow_definitions_out_rebuilds_every_rule_through_registration() {
    let fixture = manifest_dir!().join("../define/tests/fixtures/constraint_preservation.yaml");
    let meaning = include_str!("../../../define/tests/support/constraint_meaning.rs");

    let imported = OpenApiImport::new(OpenApiSource::path(&fixture))
        .api_name("ConstraintPreservation").module_path("constraint_preservation").build().unwrap();
    let temp = tempfile::TempDir::new().unwrap();
    run_import(&ImportOptions {
        input: fixture.display().to_string(),
        api_name: Some("ConstraintPreservation".to_string()),
        module_path: Some("constraint_preservation".to_string()),
        output: String::new(),
        definitions_out: Some(temp.path().display().to_string()),
        overlay: None,
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: false,
        strict: false,
        verbose: false,
    })
    .unwrap();
    let module = std::fs::read_to_string(temp.path().join("mod.rs")).unwrap();
    let types = std::fs::read_to_string(temp.path().join("types.rs")).unwrap();
    let module = module.replace("mod types;", &format!("mod types {{\n{types}\n}}"));
    // The stand-in defines this module inline rather than as a file.
    let module = module.lines().filter(|line| !line.starts_with("//!")).collect::<Vec<_>>().join("\n");
    let sources = [("constraint_preservation", module.as_str())];
    let lockfile = manifest_dir!().join("../schema/Cargo.lock");
    let mut generated = imported.api.clone();
    for component in &mut generated.schemas {
        component.rust_path = Some(component.name.clone());
    }
    let fresh = FreshCrate::generate(&[&generated], Definitions::Fixture(&sources), &lockfile);
    let expected = serde_json::to_string(&imported.api).unwrap();
    let child = r#"
#[test]
fn assembled_definitions_keep_every_imported_rule() {
    let assembled = schematic_definitions::constraint_preservation::module_group().unwrap().remove(0);
    let expected: schematic_define::RestApi = serde_json::from_str(EXPECTED).unwrap();
    let components = ["Item", "Node", "Stats", "Owner", "Settings", "Extras", "MixedExtras"];
    let direct = meaning::rule_map(&expected, &components);
    let rebuilt = meaning::rule_map(&assembled, &components);
    for key in [
        "schema:Item/properties/quantity",
        "schema:Item/properties/attributes/additionalProperties",
        "schema:Item/properties/free_value",
        "schema:Item/properties/identifier",
        "schema:Item/properties/palette",
        "schema:Item/properties/owner",
        "schema:Settings",
        "schema:Extras/additionalProperties",
        "schema:Item/properties/entries/items",
        "schema:Item/properties/owners/items",
        "schema:Item/properties/mixed_value/properties/inner/additionalProperties",
        "schema:MixedExtras/additionalProperties/properties/code",
    ] {
        assert!(rebuilt.contains_key(key), "rebuilt lost {key}: {rebuilt:#?}");
    }
    assert_eq!(rebuilt, direct);
    // The rule beside `owner`'s reference belongs to that occurrence; the
    // shared component keeps only its own.
    assert!(!rebuilt.contains_key("schema:Owner") && rebuilt.contains_key("schema:Owner/properties/name"), "{rebuilt:#?}");
    std::fs::write(std::env::var("REBUILT_API_OUT").unwrap(), serde_json::to_string(&assembled).unwrap()).unwrap();
}

use schematic_runtime::EndpointSpec;
use schematic_schema::constraint_preservation::CreateItemRequest;
use serde_json::{Value, json};

/// A `CreateItem` request whose body sets `field` to `value`.
fn request(field: &str, value: Value) -> CreateItemRequest {
    let mut body = json!({"display-name": "ok", "kind": "item", "owner": {"name": "boss"}});
    body[field] = value;
    CreateItemRequest::new(serde_json::from_value(body).unwrap())
}

/// The path and keyword of each violation `validate()` reports.
fn found(request: &CreateItemRequest) -> Vec<(String, String)> {
    match request.validate() {
        Ok(()) => vec![],
        Err(violations) => violations.iter().map(|v| (v.path_text(), v.keyword().as_str().to_owned())).collect(),
    }
}

/// `member` at every free-form position, with the field holding it and the
/// path a violation inside it is reported at.
fn positions(member: Value) -> [(&'static str, Value, &'static str); 7] {
    [
        ("mixed_value", member.clone(), "body.mixed_value"),
        ("mixed_value", json!({"inner": member}), "body.mixed_value.inner"),
        ("mixed_value", json!([member]), "body.mixed_value[0]"),
        ("mixed_map", json!({"key": member}), "body.mixed_map[[redacted]]"),
        ("mixed_attributes", json!({"key": member}), "body.mixed_attributes[[redacted]]"),
        ("mixed_entries", json!([member]), "body.mixed_entries[0]"),
        ("mixed_extras", json!({"key": member}), "body.mixed_extras[[redacted]]"),
    ]
}

/// A declared property without rules is never checked by its object's
/// extra-value rule, at every free-form position; a declared property's own
/// rule and a genuinely extra entry still refuse. The outer test checks
/// that the rebuilt API generates these same plans.
#[test]
fn every_free_form_position_exempts_its_declared_names_from_the_extra_value_rule() {
    let none: Vec<(String, String)> = vec![];
    let max_length = |path: String| vec![(path, "maxLength".to_owned())];
    for (field, value, at) in positions(json!({"label": "long declared label", "code": "ok"})) {
        let request = request(field, value);
        assert_eq!(found(&request), none, "{at}");
        assert!(request.into_parts().is_ok(), "{at}: preparation");
    }
    for (field, value, at) in positions(json!({"code": "long"})) {
        let request = request(field, value);
        assert_eq!(found(&request), max_length(format!("{at}.code")), "{at}");
        assert!(request.into_parts().is_err(), "{at}: preparation");
    }
    for (field, value, at) in positions(json!({"extra": "long"})) {
        let request = request(field, value);
        assert_eq!(found(&request), max_length(format!("{at}[[redacted]]")), "{at}");
        assert!(request.into_parts().is_err(), "{at}: preparation");
    }
    // A nominal struct's own declared name, beside a checked extra value.
    for field in ["extras", "mixed_extras"] {
        assert_eq!(found(&request(field, json!({"id": "long declared id"}))), none, "{field}");
    }
}

/// `Owner`'s rule is checked wherever a free-form value references it: as a
/// property at every free-form position, directly (`owner`), through a
/// nullable alternative that still admits null (`nullable_probe`), and
/// through a one-member `allOf` (`allof_probe`); as a free-form value's
/// items and extra values; and beside a rule. The outer test checks that
/// the rebuilt API generates these same plans.
#[test]
fn every_free_form_position_reaches_a_referenced_model_s_rule() {
    let long = || json!({"name": "longer than ten"});
    let short = || json!({"name": "short"});
    let mut refused: Vec<(&str, Value, String)> = Vec::new();
    let mut accepted: Vec<(&str, Value, String)> = Vec::new();
    for property in ["owner", "nullable_probe", "allof_probe"] {
        for (field, value, at) in positions(json!({property: long()})) {
            refused.push((field, value, format!("{at}.{property}")));
        }
        for (field, value, at) in positions(json!({property: short()})) {
            accepted.push((field, value, format!("{at}.{property}")));
        }
    }
    for (field, value, at) in positions(json!({"nullable_probe": null})) {
        accepted.push((field, value, format!("{at}.nullable_probe")));
    }
    for (member, at) in [
        (json!([long()]), "body.linked_value[0]"),
        (json!({"key": long()}), "body.linked_value[[redacted]]"),
    ] {
        refused.push(("linked_value", member, at.to_owned()));
    }
    for member in [json!([short()]), json!({"key": short()}), json!({"pinned": {"name": "boss"}})] {
        accepted.push(("linked_value", member, "body.linked_value".to_owned()));
    }
    for (field, value, at) in refused {
        let request = request(field, value);
        assert_eq!(found(&request), vec![(format!("{at}.name"), "maxLength".to_owned())], "{at}");
        assert!(request.into_parts().is_err(), "{at}: preparation");
    }
    for (field, value, at) in accepted {
        let request = request(field, value.clone());
        assert_eq!(found(&request), vec![], "{at}: {value}");
        assert!(request.into_parts().is_ok(), "{at}: {value}: preparation");
    }
    // The rule beside the reference applies too.
    let pinned = request("linked_value", json!({"pinned": short()}));
    assert_eq!(found(&pinned), vec![("body.linked_value.pinned".to_owned(), "const".to_owned())]);
    assert!(pinned.into_parts().is_err());
}
"#;
    let meaning = meaning.lines().filter(|line| !line.starts_with("//!")).collect::<Vec<_>>().join("\n");
    fresh.add_test_file(
        "constraint_preservation",
        &format!("const EXPECTED: &str = {expected:?};\nmod meaning {{\n{meaning}\n}}\n{child}"),
    );
    let rebuilt_out = temp.path().join("rebuilt.json");
    let run = fresh.nextest_run(&[("REBUILT_API_OUT", rebuilt_out.to_str().unwrap())]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for child in ["every_free_form_position_exempts_its_declared_names_from_the_extra_value_rule", "every_free_form_position_reaches_a_referenced_model_s_rule"] {
        assert!(report.contains(child), "{child}: {report}");
    }

    // The rebuilt rules are enforceable: nothing the generator refuses, such
    // as a rule beneath a composition, is introduced by the rebuild.
    let rebuilt: schematic_define::RestApi = serde_json::from_str(&std::fs::read_to_string(&rebuilt_out).unwrap()).unwrap();
    let errors: Vec<_> = validate_rust_client(&[&rebuilt]).into_iter().filter(|diagnostic| diagnostic.is_error()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    // The model a free-form value references is registered, and every
    // reference of the rebuilt API names a component.
    assert!(rebuilt.component("Owner").is_some(), "{:?}", rebuilt.schemas.iter().map(|c| &c.name).collect::<Vec<_>>());
    for component in &rebuilt.schemas {
        let mut references = Vec::new();
        collect_references(&component.schema, &mut references);
        for reference in references {
            assert!(referenced_component(&rebuilt, &reference, Some(&component.name)).is_some(), "{}: dangling {reference}", component.name);
        }
    }

    // The rebuilt API generates the plans the child just proved, so its
    // client accepts and refuses those requests too, without compiling a
    // second crate. Six objects check `code` and their extra values beside
    // the unchecked `label` (`mixed_value`'s items and `inner`, and the
    // values of `mixed_map`, `mixed_attributes`, `mixed_entries`, and
    // `MixedExtras`); `mixed_value` itself declares `label` as a seventh.
    // Other plans differ in spelling only (a `const` as a one-value `enum`,
    // an `Option`'s null alternative), so the module is not compared whole.
    // Each of the seven objects also reaches `Owner`'s plan through `owner`,
    // through the nullable alternative `nullable_probe`, and through the
    // one-member `allOf` `allof_probe`; `linked_value` reaches it as its
    // items and its extra values.
    const MIXED: &str = concat!(
        r#"Plan::new().properties(&[Property::new("allof_probe",&Plan::reference(&Plan::reference(&COMPONENT_OWNER))),"#,
        r#"Property::new("code",&Plan::new().max_length(2u64)),Property::new("label",&Plan::new()),"#,
        r#"Property::new("nullable_probe",&Plan::reference(&Plan::reference(&COMPONENT_OWNER)).nullable()),"#,
        r#"Property::new("owner",&Plan::reference(&COMPONENT_OWNER))]).values(&Plan::new().max_length(3u64))"#,
    );
    const LABEL: &str = r#"Property::new("label",&Plan::new())"#;
    const OWNER: &str = r#"Property::new("owner",&Plan::reference(&COMPONENT_OWNER))"#;
    const NULLABLE: &str = r#"Property::new("nullable_probe",&Plan::reference(&Plan::reference(&COMPONENT_OWNER)).nullable())"#;
    const ALL_OF: &str = r#"Property::new("allof_probe",&Plan::reference(&Plan::reference(&COMPONENT_OWNER)))"#;
    const LINKED: [&str; 2] = [
        r#"Property::new("linked_value",&Plan::new().items(&Plan::reference(&COMPONENT_OWNER)).properties(&[Property::new("pinned",&Plan::"#,
        r#")]).values(&Plan::reference(&COMPONENT_OWNER))"#,
    ];
    for (route, plans) in [("direct", value_rule_plans(&generated)), ("rebuilt", value_rule_plans(&rebuilt))] {
        let counts = [MIXED, LABEL, OWNER, NULLABLE, ALL_OF].map(|plan| plans.matches(plan).count());
        assert_eq!(counts, [6, 7, 7, 7, 7], "{route}: {plans}");
        for linked in LINKED {
            assert_eq!(plans.matches(linked).count(), 1, "{route}: {linked}: {plans}");
        }
    }
}

/// Every `$ref` string in `value`.
fn collect_references(value: &serde_json::Value, references: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::String(reference)) = object.get("$ref") {
                references.push(reference.clone());
            }
            object.values().for_each(|value| collect_references(value, references));
        }
        serde_json::Value::Array(values) => values.iter().for_each(|value| collect_references(value, references)),
        _ => {}
    }
}

/// The generated plans of `api`'s requests, without layout (no whitespace
/// and no trailing commas) and with each component plan named without its
/// index (`COMPONENT_OWNER`).
fn value_rule_plans(api: &schematic_define::RestApi) -> String {
    let out = tempfile::TempDir::new().unwrap();
    let module = generate_and_write_all(&[api], out.path(), false).unwrap();
    let start = module.find("mod constraint_preservation_value_rules {").expect("the API has request plans");
    let end = module[start..].find("\n}\n").expect("the plan module closes");
    let compact: String = module[start..start + end].split_whitespace().collect();
    let compact = compact.replace(",]", "]").replace(",)", ")");
    // A component plan's index is its compile order, which differs by route.
    let mut named = String::with_capacity(compact.len());
    let mut rest = compact.as_str();
    while let Some(at) = rest.find("COMPONENT_") {
        named.push_str(&rest[..at + "COMPONENT_".len()]);
        rest = rest[at + "COMPONENT_".len()..].trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches('_');
    }
    named.push_str(rest);
    named
}

/// Runs `schematic-gen import --definitions-out` on `source` and returns the
/// process output and the output directory (whether anything was written).
fn import_cli(source: &str) -> (std::process::Output, Vec<String>) {
    let temp = tempfile::TempDir::new().unwrap();
    let input = temp.path().join("spec.yaml");
    std::fs::write(&input, source).unwrap();
    let out = temp.path().join("definitions");
    let output = std::process::Command::new(biscuit_test_harness::bin_exe!("schematic-gen"))
        .args(["import", "--input", input.to_str().unwrap(), "--api-name", "Wiring", "--definitions-out", out.to_str().unwrap()])
        .env("NO_COLOR", "1")
        .output()
        .expect("run schematic-gen");
    let written = std::fs::read_dir(&out).map(|entries| entries.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    (output, written)
}

/// The OpenAPI loader's wiring through the CLI: the control document imports
/// and writes the module; each invalid numeric shape fails naming the
/// keyword's pointer in the rendered error and writes nothing. Every other
/// loader cell runs in-process (`define` `openapi_constraint_import`).
#[test]
fn the_cli_import_reports_an_unrepresentable_or_null_bound_at_its_pointer_and_writes_nothing() {
    let control = include_str!("../../../define/tests/fixtures/constraint_preservation.yaml");
    let (output, written) = import_cli(control);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(written.contains(&"mod.rs".to_string()), "{written:?}");

    for (edit, pointer) in [
        ("maximum: .inf", "#/paths/~1items/get/parameters/0/schema/maximum"),
        ("maximum: null", "#/paths/~1items/get/parameters/0/schema/maximum"),
    ] {
        let edited = control.replacen("maximum: 18446744073709551616", edit, 1);
        assert_ne!(edited, control, "the edit applies");
        let (output, written) = import_cli(&edited);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{edit}: the import fails");
        assert!(stderr.contains(pointer), "{edit}: {stderr}");
        assert!(written.is_empty(), "{edit}: nothing is written: {written:?}");
    }
}
