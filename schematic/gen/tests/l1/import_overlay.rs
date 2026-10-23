//! The importer's overlay hook: `import --definitions-out <dir> --overlay
//! <RUST_PATH>` emits a factory that applies a hand-written overlay function,
//! which the importer never writes.

use std::path::Path;

use biscuit_test_harness::manifest_dir;
use schematic_gen::errors::GeneratorError;
use schematic_gen::import_pipeline::{ImportOptions, run_import};

const OVERLAY: &str = "crate::petstore_pagination::apply";

/// A hand-written overlay, placed where `schematic-definitions` keeps one:
/// beside the imported module's directory.
const OVERLAY_SOURCE: &str = "use schematic_define::RestApi;\n\n\
    pub fn apply(api: RestApi) -> RestApi {\n    api\n}\n";

fn petstore_options(definitions_out: Option<&Path>, overlay: Option<&str>) -> ImportOptions {
    ImportOptions {
        input: manifest_dir!().join("tests/fixtures/petstore.yaml").display().to_string(),
        api_name: Some("Petstore".to_string()),
        module_path: Some("petstore".to_string()),
        output: String::new(),
        definitions_out: definitions_out.map(|dir| dir.display().to_string()),
        overlay: overlay.map(str::to_string),
        exclude_paths: vec![],
        request_id_header: None,
        dry_run: false,
        strict: false,
        verbose: false,
    }
}

#[test]
fn an_overlay_import_emits_the_call_and_leaves_the_overlay_untouched() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let src = temp_dir.path().join("src");
    let definitions = src.join("petstore");
    let overlay_file = src.join("petstore_pagination.rs");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(&overlay_file, OVERLAY_SOURCE).unwrap();
    let options = petstore_options(Some(&definitions), Some(OVERLAY));

    run_import(&options).expect("first import succeeds");
    let first = std::fs::read_to_string(definitions.join("mod.rs")).unwrap();
    run_import(&options).expect("second import succeeds");
    let second = std::fs::read_to_string(definitions.join("mod.rs")).unwrap();

    assert_eq!(first.matches("crate::petstore_pagination::apply(api)").count(), 1);
    assert_eq!(std::fs::read_to_string(&overlay_file).unwrap(), OVERLAY_SOURCE);
    assert_eq!(first, second, "re-import is not byte-identical");
}

#[test]
fn an_import_without_an_overlay_uses_fallible_typed_assembly() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let definitions = temp_dir.path().join("petstore");

    run_import(&petstore_options(Some(&definitions), None)).expect("import succeeds");
    let module = std::fs::read_to_string(definitions.join("mod.rs")).unwrap();

    assert!(module.contains("RestApi::builder(ctx,"));
    assert!(module.contains(".returns_json::<"));
    assert!(module.contains("assemble_module_group"));
    assert!(!module.contains("let api = RestApi {"));
    assert!(!module.contains("--overlay"));
}

#[test]
fn an_invalid_overlay_is_rejected_before_anything_is_written() {
    for overlay in ["", "not a path", "crate::x::<T>", "crate::x::apply()"] {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let definitions = temp_dir.path().join("petstore");

        let error = run_import(&petstore_options(Some(&definitions), Some(overlay)))
            .expect_err("invalid overlay is rejected");

        assert!(
            matches!(&error, GeneratorError::ConfigError(message) if message.contains("--overlay")),
            "{overlay:?}: {error}"
        );
        assert!(!definitions.exists(), "{overlay:?}: output was written");
    }
}

#[test]
fn an_overlay_without_definitions_out_is_rejected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("out");
    let mut options = petstore_options(None, Some(OVERLAY));
    options.output = output.display().to_string();

    let error = run_import(&options).expect_err("overlay without --definitions-out is rejected");

    assert!(
        matches!(&error, GeneratorError::ConfigError(message) if message.contains("--definitions-out")),
        "{error}"
    );
    assert!(!output.exists(), "output was written");
}

/// Re-importing the vendored OpenAI document with the `import-openai` recipe's
/// options reproduces the committed definition byte for byte, overlay call
/// included, so a re-import cannot drop OpenAI's authored pagination (R35).
/// The options mirror `schematic/justfile`'s `import-openai`; a recipe change
/// that alters the output fails here until the committed module is re-imported.
#[test]
fn reimporting_the_vendored_openai_document_reproduces_the_committed_definition() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let definitions = temp_dir.path().join("openai");
    let options = ImportOptions {
        input: manifest_dir!().join("../specs/openai/openapi.yaml").display().to_string(),
        api_name: Some("OpenAI".to_string()),
        module_path: Some("openai".to_string()),
        output: String::new(),
        definitions_out: Some(definitions.display().to_string()),
        overlay: Some("crate::openai_pagination::apply".to_string()),
        exclude_paths: vec!["/assistants".to_string(), "/threads".to_string()],
        request_id_header: Some("x-request-id".to_string()),
        dry_run: false,
        strict: false,
        verbose: false,
    };

    run_import(&options).expect("first import succeeds");
    let first = std::fs::read_to_string(definitions.join("mod.rs")).unwrap();
    run_import(&options).expect("second import succeeds");
    let second = std::fs::read_to_string(definitions.join("mod.rs")).unwrap();
    let types = std::fs::read_to_string(definitions.join("types.rs")).unwrap();

    let committed = manifest_dir!().join("../definitions/src/openai/mod.rs");
    let committed_types = manifest_dir!().join("../definitions/src/openai/types.rs");
    assert_eq!(first, second, "a second import is not byte-identical");
    assert_eq!(first.matches("crate::openai_pagination::apply(api)").count(), 1);
    // Two importer improvements the committed module has not been
    // re-imported for, because each changes the definition and its generated
    // client beyond what this feature may edit: `info.version` is now read,
    // and the realtime call's `session` part declares `encoding.contentType:
    // application/json`. Running `just import-openai` adopts both; this test
    // then fails until the pins below are removed.
    let pending = std::fs::read_to_string(&committed).unwrap()
        .replacen("fn endpoint_183(_ctx: &mut DefinitionContext)", "fn endpoint_183(ctx: &mut DefinitionContext)", 1)
        .replacen(
            "                    FormField::text(\"session\")\n",
            "                    FormField::json_desc(\n                            \"session\",\n                            ctx.register::<RealtimeSessionCreateRequestGA>()?,\n                        )\n",
            1,
        )
        .replacen("        .module_path(\"openai\")\n", "        .module_path(\"openai\")\n        .version(\"2.3.0\")\n", 1);
    assert!(
        first == pending,
        "re-importing changes {} beyond the two pinned differences; run `just import-openai`",
        committed.display()
    );
    assert!(
        types == std::fs::read_to_string(&committed_types).unwrap(),
        "re-importing changes {}; run `just import-openai`",
        committed_types.display()
    );
}

#[test]
fn slow_imported_builders_compile_and_match_public_import() {
    use schematic_define::openapi::{OpenApiImport, OpenApiSource};
    use crate::common::fresh_output::{Definitions, FreshCrate};

    let fixture = manifest_dir!().join("tests/fixtures/petstore.yaml");
    let imported = OpenApiImport::new(OpenApiSource::path(&fixture))
        .api_name("Petstore").module_path("petstore").prefer_json().build().unwrap();
    let temp = tempfile::TempDir::new().unwrap();
    run_import(&petstore_options(Some(temp.path()), None)).unwrap();
    let module = std::fs::read_to_string(temp.path().join("mod.rs")).unwrap();
    let types = std::fs::read_to_string(temp.path().join("types.rs")).unwrap();
    let module = module.replace("mod types;", &format!("mod types {{\n{types}\n}}"));
    // The stand-in defines this module inline rather than as a file.
    let module = module.lines().filter(|line| !line.starts_with("//!"))
        .collect::<Vec<_>>().join("\n");
    let sources = [("petstore", module.as_str())];
    let lockfile = manifest_dir!().join("../schema/Cargo.lock");
    // The client is generated beside the definitions fixture, whose models it
    // glob-imports, so each component's Rust path is its bare name.
    let mut generated = imported.api.clone();
    for component in &mut generated.schemas {
        component.rust_path = Some(component.name.clone());
    }
    let fresh = FreshCrate::generate(&[&generated], Definitions::Fixture(&sources), &lockfile);
    let expected = serde_json::to_string(&imported.api).unwrap();
    let child = r#"
#[test]
fn imported_api_and_components_match_the_public_import() {
    use schematic_define::RestApi;
    let mut api = schematic_definitions::petstore::module_group().unwrap().remove(0);
    api.validate_references().unwrap();
    let expected: RestApi = serde_json::from_str(EXPECTED).unwrap();
    expected.validate_references().unwrap();
    let names = |api: &RestApi| -> Vec<(String, Option<String>)> {
        api.schemas.iter().map(|c| (c.name.clone(), c.rust_path.clone())).collect()
    };
    // The public import lists components in source order with Rust paths in
    // its module path. Assembly registers the same models, but flattens a
    // model's nested definitions directly after it (so `PetStatus` moves up
    // behind `Pet`), and places the types module in `schematic_definitions`.
    assert_eq!(names(&expected), [
        ("Pet".to_string(), Some("petstore::Pet".to_string())),
        ("CreatePetRequest2".to_string(), Some("petstore::CreatePetRequest2".to_string())),
        ("PetStatus".to_string(), Some("petstore::PetStatus".to_string())),
    ]);
    assert_eq!(names(&api), [
        ("Pet".to_string(), Some("::schematic_definitions::petstore::Pet".to_string())),
        ("PetStatus".to_string(), Some("::schematic_definitions::petstore::PetStatus".to_string())),
        ("CreatePetRequest2".to_string(), Some("::schematic_definitions::petstore::CreatePetRequest2".to_string())),
    ]);
    let mut qualified: Vec<_> = names(&expected).into_iter()
        .map(|(name, path)| (name, path.map(|path| format!("::schematic_definitions::{path}")))).collect();
    let mut assembled = names(&api);
    qualified.sort();
    assembled.sort();
    assert_eq!(assembled, qualified);
    // Schemas differ by source: the import keeps the OpenAPI source schema,
    // assembly carries the schemars output of the generated model (titles,
    // `nullable` optionals). Both were compared by name and path above, and
    // references are names, so the remaining definition must match exactly.
    api.schemas.clone_from(&expected.schemas);
    assert_eq!(api, expected);
    let client = schematic_schema::petstore::Petstore::new();
    let _ = client.list_pets().limit(2).build().unwrap();
    assert_eq!(api.endpoints.len(), 4);
}
"#;
    fresh.add_test_file("imported_builders", &format!("const EXPECTED: &str = {expected:?};\n{child}"));
    fresh.nextest_run(&[]).assert_success();
}

#[test]
fn empty_imports_are_not_usable_apis_in_either_format() {
    for (extension, source) in [
        ("json", r#"{"openapi":"3.0.3","info":{"title":"Empty","version":"1"},"paths":{}}"#),
        ("yaml", "openapi: '3.0.3'\ninfo: {title: Empty, version: '1'}\npaths: {}\n"),
    ] {
        let temp = tempfile::TempDir::new().unwrap();
        let input = temp.path().join(format!("empty.{extension}"));
        std::fs::write(&input, source).unwrap();
        let destination = temp.path().join("out");
        let mut options = petstore_options(Some(&destination), None);
        options.input = input.display().to_string();
        let error = run_import(&options).unwrap_err();
        assert!(error.to_string().contains("no usable endpoints"), "{extension}: {error}");
        assert!(!destination.exists());
    }
}
