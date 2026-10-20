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

/// The tail expression of the emitted `define_petstore_api()`.
fn factory_tail(module: &str) -> syn::Expr {
    let file = syn::parse_file(module).expect("emitted mod.rs parses");
    let body = file
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "define_petstore_api" => {
                Some(function.block.stmts)
            }
            _ => None,
        })
        .expect("define_petstore_api is emitted");
    match body.last() {
        Some(syn::Stmt::Expr(tail, None)) => tail.clone(),
        _ => panic!("factory has no tail expression:\n{module}"),
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

    let syn::Expr::Call(call) = factory_tail(&first) else {
        panic!("tail is not the overlay call:\n{first}");
    };
    let syn::Expr::Path(callee) = &*call.func else { panic!("callee is not a path") };
    let callee: Vec<String> = callee.path.segments.iter().map(|s| s.ident.to_string()).collect();
    assert_eq!(callee, ["crate", "petstore_pagination", "apply"]);
    assert_eq!(first.matches("crate::petstore_pagination::apply(api)").count(), 1);
    assert_eq!(std::fs::read_to_string(&overlay_file).unwrap(), OVERLAY_SOURCE);
    assert_eq!(first, second, "re-import is not byte-identical");
}

#[test]
fn an_import_without_an_overlay_ends_with_the_literal() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let definitions = temp_dir.path().join("petstore");

    run_import(&petstore_options(Some(&definitions), None)).expect("import succeeds");
    let module = std::fs::read_to_string(definitions.join("mod.rs")).unwrap();

    assert!(
        matches!(factory_tail(&module), syn::Expr::Struct(literal) if literal.path.is_ident("RestApi")),
        "tail is not the `RestApi` literal:\n{module}"
    );
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
    assert!(
        first == std::fs::read_to_string(&committed).unwrap(),
        "re-importing changes {}; run `just import-openai`",
        committed.display()
    );
    assert!(
        types == std::fs::read_to_string(&committed_types).unwrap(),
        "re-importing changes {}; run `just import-openai`",
        committed_types.display()
    );
}
