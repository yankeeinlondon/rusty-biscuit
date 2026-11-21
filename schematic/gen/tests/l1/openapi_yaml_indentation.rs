//! A YAML OpenAPI export takes the indentation its destination file asks for
//! (the worktree's EditorConfig or formatter settings, resolved by Sniff),
//! four spaces when nothing states one; JSON is unaffected. The layout
//! changes no parsed value.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{canonicalize_simplified, serde_yaml_ng};
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_define::{DefinitionContext, Endpoint, RestApi};
use schematic_definitions::registry::assemble_module_group;
use schematic_gen::export::openapi::{
    build_openapi_grouped, render_openapi_document, render_openapi_document_for, write_openapi_document,
};
use schematic_gen::pipeline::{BatchScope, GenerateOpts, OpenApiFormat, generate_batch};

/// A Git worktree, with an `.editorconfig` asking for two-space YAML when
/// `two_space` is set.
fn repository(two_space: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = canonicalize_simplified(dir.path()).unwrap();
    git2::Repository::init(&root).unwrap();
    if two_space {
        fs::write(root.join(".editorconfig"), "root = true\n\n[*.yaml]\nindent_style = space\nindent_size = 2\n").unwrap();
    }
    (dir, root)
}

fn widgets() -> Vec<RestApi> {
    assemble_module_group("::fixture", |ctx: &mut DefinitionContext| {
        let list = Endpoint::get("ListWidgets", "/widgets").describes("List").returns_empty().build()?;
        Ok(vec![
            RestApi::builder(ctx, "Widgets", "https://fixture.test").describes("Fixture").no_auth().endpoints([list]).build()?,
        ])
    })
    .expect("fixture group builds")
}

fn document() -> openapiv3::OpenAPI {
    let apis = widgets();
    let refs: Vec<&RestApi> = apis.iter().collect();
    build_openapi_grouped(&refs, "widgets", &ExportOptions::new()).unwrap()
}

/// Runs the generate pipeline with YAML OpenAPI output under `root` and
/// returns the published document's text.
fn generated_yaml(root: &Path) -> String {
    let rust = root.join("schema/src").to_str().unwrap().to_owned();
    let openapi = root.join("openapi").to_str().unwrap().to_owned();
    let opts = GenerateOpts {
        output: &rust,
        dry_run: false,
        verbose: 0,
        openapi_out: Some(&openapi),
        openapi_format: OpenApiFormat::Yaml,
        openapi_version: None,
        postman_out: None,
        no_openapi: false,
        no_postman: true,
        postman_environment: false,
    };
    generate_batch(&widgets(), &BatchScope::Complete, &opts).into_result().expect("fixture batch generates");
    fs::read_to_string(root.join("openapi/widgets.yaml")).unwrap()
}

fn parsed(text: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(text).unwrap()
}

#[test]
fn the_pipeline_lays_yaml_out_for_its_destination() {
    let (_two_dir, two_root) = repository(true);
    let (_default_dir, default_root) = repository(false);

    let two = generated_yaml(&two_root);
    let default = generated_yaml(&default_root);

    assert!(two.contains("\ninfo:\n  title: widgets\n"), "{two}");
    assert!(two.contains("\npaths:\n  /widgets:\n    get:\n"), "{two}");
    assert!(default.contains("\ninfo:\n    title: widgets\n"), "{default}");
    assert!(default.contains("\npaths:\n    /widgets:\n        get:\n"), "{default}");
    assert_eq!(parsed(&two), parsed(&default));
}

#[test]
fn a_written_document_takes_its_destination_indentation_and_keeps_its_values() {
    let doc = document();
    let backend = serde_yaml_ng::to_value(&doc).unwrap();
    for (two_space, title) in [(true, "\ninfo:\n  title: widgets\n"), (false, "\ninfo:\n    title: widgets\n")] {
        let (_dir, root) = repository(two_space);
        let path = write_openapi_document(&doc, "widgets", ExportFormat::Yaml, &root).unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains(title), "{text}");
        assert_eq!(parsed(&text), backend);
    }
}

#[test]
fn json_output_ignores_the_destination_indentation() {
    let (_dir, root) = repository(true);
    let doc = document();
    assert_eq!(
        render_openapi_document_for(&doc, ExportFormat::Json, &root.join("widgets.json")).unwrap(),
        render_openapi_document(&doc, ExportFormat::Json).unwrap()
    );
}

/// A destination under a regular file cannot be identified, so the default
/// layout applies (with a warning on standard error).
#[test]
fn an_unidentifiable_destination_falls_back_to_four_spaces() {
    let (_dir, root) = repository(true);
    fs::write(root.join("blocker"), "").unwrap();
    let doc = document();
    assert_eq!(
        render_openapi_document_for(&doc, ExportFormat::Yaml, &root.join("blocker/widgets.yaml")).unwrap(),
        render_openapi_document(&doc, ExportFormat::Yaml).unwrap()
    );
}
