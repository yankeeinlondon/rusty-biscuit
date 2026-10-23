//! Acceptance: `import --definitions-out` followed by assembly rebuilds every
//! value rule the direct import keeps, compared by meaning.
//!
//! The direct import/export route is `schematic-define`'s
//! `constraint_preservation` test, over the same fixture and with the same
//! meaning comparison (`define/tests/support/constraint_meaning.rs`), which
//! the generated crate's test includes as source.

use biscuit_test_harness::manifest_dir;
use schematic_define::openapi::{OpenApiImport, OpenApiSource};
use schematic_gen::import_pipeline::{ImportOptions, run_import};

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
    let components = ["Item", "Node", "Stats"];
    let direct = meaning::rule_map(&expected, &components);
    let rebuilt = meaning::rule_map(&assembled, &components);
    assert!(direct.contains_key("schema:Item/properties/quantity"), "{direct:#?}");
    assert_eq!(rebuilt, direct);
}
"#;
    let meaning = meaning.lines().filter(|line| !line.starts_with("//!")).collect::<Vec<_>>().join("\n");
    fresh.add_test_file(
        "constraint_preservation",
        &format!("const EXPECTED: &str = {expected:?};\nmod meaning {{\n{meaning}\n}}\n{child}"),
    );
    fresh.nextest_run(&[]).assert_success();
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
