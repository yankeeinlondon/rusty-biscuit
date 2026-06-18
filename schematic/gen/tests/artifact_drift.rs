//! Artifact drift detection tests.
//!
//! These tests verify that committed OpenAPI and Postman artifacts match
//! what the generator would produce. They iterate every module reported by
//! [`apis_by_module`] and compare the in-memory regeneration against the
//! files committed under `schematic/openapi/` and `schematic/postman/`.
//!
//! Run with:
//!
//! ```sh
//! cargo test -p schematic-gen --test artifact_drift
//! ```
//!
//! The tests run unconditionally on every `cargo test` invocation so CI
//! catches drift on every PR. Each test is in-process — no `cargo` is
//! invoked — so the entire suite completes in a few seconds for ~16
//! modules.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use schematic_define::openapi::ExportOptions;
use schematic_definitions::apis_by_module;
use schematic_definitions::registry::get_registries_for_module;
use schematic_gen::export::openapi::write_openapi_grouped;
use schematic_gen::export::postman::{write_postman, write_postman_grouped};

/// Returns the absolute path to the `schematic/` package area, anchored on
/// the gen crate's `CARGO_MANIFEST_DIR` so the tests work regardless of the
/// caller's current working directory.
fn schematic_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at `<repo>/schematic/gen`. The committed
    // artifacts live in `<repo>/schematic/openapi` and `<repo>/schematic/postman`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
