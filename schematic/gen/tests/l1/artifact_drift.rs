//! Artifact drift detection tests.
//!
//! These tests verify that committed Rust, OpenAPI, and Postman artifacts match
//! what the generator would produce. They iterate every module reported by
//! [`apis_by_module`] and compare the in-memory regeneration against the
//! files committed under `schematic/schema/`, `schematic/openapi/`, and
//! `schematic/postman/`, and
//! scan the committed OpenAPI corpus for `operationId` integrity.
//!
//! Run with:
//!
//! ```sh
//! cargo nextest run -p schematic-gen --test l1 -E 'test(/^artifact_drift::/)'
//! ```
//!
//! The tests are ordinary L1 tests, so every `just test` run and every CI
//! run catches drift. Each test is in-process — no `cargo` is
//! invoked — so the entire suite completes in a few seconds for ~16
//! modules.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use schematic_define::openapi::ExportOptions;
use schematic_definitions::apis_by_module;
use schematic_definitions::registry::get_registries_for_module;
use schematic_gen::export::openapi::{operation_id_diagnostics, write_openapi_grouped};
use schematic_gen::export::postman::{write_postman, write_postman_grouped};
use schematic_gen::pipeline::{
    BatchScope, GenerateOpts, OpenApiFormat, generate_batch, resolve_all_apis,
};

/// Returns the absolute path to the `schematic/` package area, anchored on
/// the gen crate's manifest directory so the tests work regardless of the
/// caller's current working directory.
fn schematic_root() -> PathBuf {
    // The manifest directory is `<repo>/schematic/gen`. The committed artifacts
    // live in `<repo>/schematic/openapi` and `<repo>/schematic/postman`.
    biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("schematic/gen has a parent")
        .to_path_buf()
}

#[test]
fn generated_openapi_artifacts_are_up_to_date() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let openapi_committed = schematic_root().join("openapi");
    let grouped = apis_by_module();

    for (module_name, apis) in grouped.iter() {
        let Some(registry) = get_registries_for_module(module_name) else {
            // Modules without a registry are not exported as OpenAPI; skip.
            eprintln!("Skipping module '{module_name}' for OpenAPI drift: no registry available");
            continue;
        };

        // Use the version of the first API as the document version (matches
        // the CLI driver's behavior for the grouped writer).
        let version = apis
            .first()
            .and_then(|api| api.version.as_deref())
            .unwrap_or("0.1.0");
        let options = ExportOptions::new().with_version(version);

        let api_refs: Vec<&_> = apis.iter().collect();
        let generated_path =
            write_openapi_grouped(&api_refs, module_name, &registry, &options, temp_dir.path())
                .unwrap_or_else(|e| {
                    panic!("Failed to generate OpenAPI for module '{module_name}': {e}")
                });

        let generated = fs::read_to_string(&generated_path).unwrap();

        let committed_path = openapi_committed.join(generated_path.file_name().unwrap());
        assert!(
            committed_path.exists(),
            "Missing committed OpenAPI artifact for module '{module_name}' at {}",
            committed_path.display()
        );

        let committed = fs::read_to_string(&committed_path).unwrap_or_else(|e| {
            panic!("Failed to read committed OpenAPI for module '{module_name}': {e}")
        });

        assert_eq!(
            generated,
            committed,
            "\nOpenAPI artifact drift detected for module '{}'.\n\
             Run `just -f schematic/justfile generate` to update the committed artifacts.\n\
             Diff: {} vs {}",
            module_name,
            generated_path.display(),
            committed_path.display(),
        );
    }
}

#[test]
fn generated_postman_artifacts_are_up_to_date() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let postman_committed = schematic_root().join("postman");
    let grouped = apis_by_module();

    for (module_name, apis) in grouped.iter() {
        let generated_path = if apis.len() == 1 {
            write_postman(&apis[0], temp_dir.path(), false).unwrap_or_else(|e| {
                panic!(
                    "Failed to generate Postman for {} in module {}: {}",
                    apis[0].name, module_name, e
                )
            })
        } else {
            let api_refs: Vec<&_> = apis.iter().collect();
            write_postman_grouped(&api_refs, module_name, temp_dir.path(), false).unwrap_or_else(
                |e| panic!("Failed to generate grouped Postman for module {module_name}: {e}"),
            )
        };

        let generated = fs::read_to_string(&generated_path).unwrap();

        let committed_path = postman_committed.join(generated_path.file_name().unwrap());
        assert!(
            committed_path.exists(),
            "Missing committed Postman artifact for module '{module_name}' at {}",
            committed_path.display()
        );

        let committed = fs::read_to_string(&committed_path).unwrap_or_else(|e| {
            panic!("Failed to read committed Postman for module {module_name}: {e}")
        });

        assert_eq!(
            generated,
            committed,
            "\nPostman artifact drift detected for module '{}'.\n\
             Run `just -f schematic/justfile generate` to update the committed artifacts.\n\
             Diff: {} vs {}",
            module_name,
            generated_path.display(),
            committed_path.display(),
        );
    }

    let generated = source_files(temp_dir.path());
    let committed = source_files(&postman_committed);
    assert_eq!(
        generated.keys().collect::<Vec<_>>(),
        committed.keys().collect::<Vec<_>>(),
        "Postman artifact file-set drift; run `just -f schematic/justfile generate`",
    );
}

/// Regression guard: the seven legacy per-API OpenAPI filenames produced by
/// the pre-grouping export must never reappear under `schematic/openapi/`.
///
/// If grouping is reverted, the writer would re-emit one of these names and
/// this test will catch it before the artifact lands in a PR.
#[test]
fn generated_openapi_no_legacy_per_api_files() {
    const LEGACY_FILENAMES: &[&str] = &[
        "emqxbasic.json",
        "emqxbearer.json",
        "huggingfacehub.json",
        "ollamanative.json",
        "ollamaopenai.json",
        "samsungsmarttv.json",
        "unfoldedcirclecorerest.json",
    ];

    let openapi_dir = schematic_root().join("openapi");
    assert!(
        openapi_dir.exists(),
        "OpenAPI directory missing: {}",
        openapi_dir.display(),
    );

    let present: HashSet<String> = fs::read_dir(&openapi_dir)
        .expect("read openapi dir")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();

    let leaks: Vec<&str> = LEGACY_FILENAMES
        .iter()
        .copied()
        .filter(|name| present.contains(*name))
        .collect();

    assert!(
        leaks.is_empty(),
        "Legacy per-API OpenAPI files found in {}: {:?}.\n\
         These files were replaced by module-grouped artifacts. Delete them and re-run \
         `just -f schematic/justfile generate`.",
        openapi_dir.display(),
        leaks,
    );

    // Belt-and-braces: also assert no remaining `*.json` file collides with
    // the legacy pattern by scanning for any filename that looks like the
    // pre-grouping `<api.name>.json` form for our split modules.
    let suspicious: Vec<String> = present
        .iter()
        .filter(|name| {
            name.starts_with("ollama") && *name != "ollama.json"
                || name.starts_with("emqx") && *name != "emqx.json"
                || name.starts_with("huggingface") && *name != "huggingface.json"
                || name.starts_with("samsung") && *name != "samsung_smart_tv.json"
                || name.starts_with("unfolded") && *name != "unfolded_circle_core_rest.json"
        })
        .cloned()
        .collect();

    assert!(
        suspicious.is_empty(),
        "Found suspicious OpenAPI filenames that may indicate filename drift: {:?}",
        suspicious,
    );
}

/// Regression: the Artificial Analysis API terms require that all usage
/// include attribution to <https://artificialanalysis.ai/>. The generated
/// rustdoc, OpenAPI grouped document, and Postman grouped collection MUST
/// surface that attribution so downstream consumers see it. If any artifact
/// is regenerated in a way that drops the attribution sentence from the
/// upstream API description, this test fires.
#[test]
fn artificial_analysis_attribution_present_in_all_artifacts() {
    const ATTRIBUTION_URL: &str = "https://artificialanalysis.ai/";
    let root = schematic_root();
    let schema_path = {
        let single = root.join("schema/src/artificial_analysis.rs");
        let split = root.join("schema/src/artificial_analysis/mod.rs");
        if single.exists() { single } else { split }
    };
    let artifacts: [(&str, PathBuf); 3] = [
        ("schema rustdoc", schema_path),
        (
            "OpenAPI grouped doc",
            root.join("openapi/artificial_analysis.json"),
        ),
        (
            "Postman grouped collection",
            root.join("postman/artificial_analysis.postman_collection.json"),
        ),
    ];

    for (label, path) in artifacts {
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {label} at {} failed: {e}", path.display()));
        assert!(
            contents.contains(ATTRIBUTION_URL),
            "{label} ({}) is missing the required Artificial Analysis \
             attribution ({ATTRIBUTION_URL}). Update the `description` field \
             on the Artificial Analysis API definitions in \
             `schematic/definitions/src/artificial_analysis/mod.rs` to embed \
             the attribution sentence, then regenerate the artifact via \
             `just -f schematic/justfile generate`.",
            path.display(),
        );
    }
}

/// Every committed OpenAPI document, read back from disk, passes the same
/// `operationId` scan the writers run: no id is declared twice and every link
/// names an operation of its own document. This is the passive guard for the
/// committed corpus; the drift test above proves the generator would write
/// the same bytes.
#[test]
fn committed_openapi_documents_have_unique_operation_ids() {
    let openapi_dir = schematic_root().join("openapi");
    let mut documents: Vec<PathBuf> = fs::read_dir(&openapi_dir)
        .expect("read openapi dir")
        .map(|entry| entry.expect("openapi dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    documents.sort();
    assert!(
        !documents.is_empty(),
        "no committed OpenAPI documents under {}",
        openapi_dir.display()
    );

    let mut failures = Vec::new();
    for path in &documents {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {} failed: {e}", path.display()));
        let doc: openapiv3::OpenAPI = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not an OpenAPI document: {e}", path.display()));
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        failures.extend(
            operation_id_diagnostics(&doc, &[name.as_str()])
                .iter()
                .map(ToString::to_string),
        );
    }

    assert!(
        failures.is_empty(),
        "committed OpenAPI documents violate operationId integrity:\n{}\n\
         Run `just -f schematic/justfile generate` to regenerate them.",
        failures.join("\n")
    );
}

/// Verify the committed `schematic/openapi/` directory contains exactly the
/// module-named files the generator should produce. Catches both removed
/// modules (committed file present but no longer generated) and missing
/// regeneration (module present but no committed file).
#[test]
fn committed_openapi_filenames_match_modules() {
    let openapi_dir = schematic_root().join("openapi");
    let grouped = apis_by_module();

    let expected: HashSet<String> = grouped
        .iter()
        .filter_map(|(module_name, _)| {
            // Only modules with a registry are exported as OpenAPI.
            get_registries_for_module(module_name).map(|_| format!("{module_name}.json"))
        })
        .collect();

    let actual: HashSet<String> = fs::read_dir(&openapi_dir)
        .expect("read openapi dir")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json"))
        .collect();

    let missing: Vec<&String> = expected.difference(&actual).collect();
    let unexpected: Vec<&String> = actual.difference(&expected).collect();

    assert!(
        missing.is_empty() && unexpected.is_empty(),
        "schematic/openapi/ does not match generator output.\n\
         Missing (should be present): {missing:?}\n\
         Unexpected (should be removed): {unexpected:?}\n\
         Run `just -f schematic/justfile generate` to reconcile.",
    );
}

fn source_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                collect(root, &path, files);
            } else {
                assert!(
                    entry.file_type().unwrap().is_file(),
                    "unexpected source entry: {}",
                    path.display(),
                );
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

#[test]
fn generated_rust_artifacts_are_up_to_date() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let output = temp_dir.path().join("schema/src");
    let output_text = output.to_str().unwrap();
    let opts = GenerateOpts {
        output: output_text,
        dry_run: false,
        verbose: 0,
        openapi_out: None,
        openapi_format: OpenApiFormat::Json,
        openapi_version: None,
        postman_out: None,
        no_openapi: true,
        no_postman: true,
    };
    generate_batch(
        &resolve_all_apis(),
        &BatchScope::Complete,
        &opts,
        &get_registries_for_module,
    )
    .into_result()
    .unwrap();

    let committed = schematic_root().join("schema/src");
    let expected = source_files(&committed);
    let generated = source_files(&output);
    assert!(!expected.is_empty(), "committed Rust source tree is empty");
    assert_eq!(
        generated.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "Rust artifact file-set drift; run `just -f schematic/justfile generate`",
    );
    for (path, bytes) in &generated {
        assert!(
            bytes == &expected[path],
            "Rust artifact byte drift in {}; run `just -f schematic/justfile generate`",
            path.display(),
        );
    }
    assert!(
        fs::read(output.parent().unwrap().join("Cargo.toml")).unwrap()
            == fs::read(schematic_root().join("schema/Cargo.toml")).unwrap(),
        "Rust manifest byte drift; run `just -f schematic/justfile generate`",
    );
}

#[test]
fn qualified_typed_split_modules_need_no_unused_response_imports() {
    let api = schematic_gen::pipeline::resolve_api("emqx-basic").unwrap();
    let out = tempfile::TempDir::new().unwrap();
    schematic_gen::output::generate_and_write(&api, out.path(), false).unwrap();
    let requests = std::fs::read_to_string(out.path().join("emqx/requests.rs")).unwrap();
    assert!(!requests.contains("use super::responses::*;"), "typed paths already name their defining crate");
    assert!(requests.contains("::schematic_definitions::emqx::NodeInfo"));
}
