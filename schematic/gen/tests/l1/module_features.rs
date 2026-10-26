//! One Cargo feature per generated module group.
//!
//! The generated manifest declares a feature for every module group of the
//! whole registry, default all; `lib.rs` and `prelude.rs` gate each module,
//! re-export, and doc line on its feature, and the WebSocket-only
//! dependencies are enabled only by WebSocket module features. The fixture
//! tests here run the public generation entry point into temporary
//! directories; the `slow_` test builds the real APIs offline with feature
//! selections through the fresh-output crate.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use biscuit_file::toml_crate;
use schematic_define::{DefinitionContext, Endpoint, RestApi};
use schematic_definitions::registry::registrations;
use schematic_gen::cargo_gen::MANIFEST_OWNERSHIP_MARKER;
use schematic_gen::output::{RustOutputRequest, Selection, prepare_rust_output};
use schematic_gen::pipeline::resolve_all_apis;

use crate::common::fresh_output::{CargoRun, Definitions, FreshCrate};

fn api(name: &str, module: &str, suffix: &str) -> RestApi {
    let mut context = DefinitionContext::types_in("::fixture");
    context.register::<serde_json::Value>().unwrap();
    let endpoint = Endpoint::get("ListItems", format!("/{}/items", name.to_lowercase()))
        .describes("ListItems fixture endpoint")
        .returns_json::<serde_json::Value>(&mut context)
        .build()
        .unwrap();
    RestApi::builder(&context, name, "https://fixture.example/v1")
        .describes(format!("{name} fixture API"))
        .no_auth()
        .module_path(module)
        .request_suffix(suffix)
        .endpoints(vec![endpoint])
        .build()
        .unwrap()
}

/// Four APIs in three module groups: `alpha` holds two, like Ollama.
fn registry() -> Vec<RestApi> {
    vec![
        api("AlphaOne", "alpha", "OneRequest"),
        api("AlphaTwo", "alpha", "TwoRequest"),
        api("Beta", "beta", "Request"),
        api("Gamma", "gamma", "Request"),
    ]
}

/// Runs one generation with a manifest into `crate_dir`.
fn generate(registry: &[&RestApi], selection: Selection<'_>, crate_dir: &Path) {
    prepare_rust_output(&RustOutputRequest {
        registry,
        registrations: registrations(),
        selection,
        output_dir: &crate_dir.join("src"),
        manifest_dir: Some(crate_dir),
    })
    .expect("prepare")
    .plan
    .publish()
    .expect("publish");
}

fn manifest_features(manifest: &str) -> toml_crate::Table {
    let parsed: toml_crate::Table = toml_crate::from_str(manifest).expect("manifest is TOML");
    parsed["features"].as_table().expect("[features] table").clone()
}

/// Every feature name a generated source file's `cfg`s mention.
fn cfg_features(source: &str) -> BTreeSet<String> {
    source
        .split("feature = \"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_string())
        .collect()
}

#[test]
fn a_partial_run_writes_the_manifest_of_the_whole_registry() {
    let apis = registry();
    let registry: Vec<&RestApi> = apis.iter().collect();
    let temp = tempfile::tempdir().unwrap();
    let crate_dir = temp.path().join("schema");

    generate(&registry, Selection::All, &crate_dir);
    let full_manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    let full_lib = fs::read_to_string(crate_dir.join("src/lib.rs")).unwrap();

    // Still generator-owned, so the partial run may rewrite it; the stale
    // feature set proves the partial run writes the manifest itself.
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!("{MANIFEST_OWNERSHIP_MARKER}\n[features]\ndefault = [\"beta\"]\nbeta = []\n"),
    )
    .unwrap();
    generate(&registry, Selection::Apis(&[registry[2]]), &crate_dir);

    let partial_manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    assert_eq!(partial_manifest, full_manifest, "a partial run writes the full-inventory manifest");
    let features = manifest_features(&partial_manifest);
    for module in ["alpha", "beta", "gamma", "elevenlabs_ws"] {
        assert!(features.contains_key(module), "unselected sibling `{module}` keeps its feature");
    }
    assert_eq!(
        fs::read_to_string(crate_dir.join("src/lib.rs")).unwrap(),
        full_lib,
        "retained modules keep their gated declarations"
    );
    assert!(full_lib.contains("#[cfg(feature = \"alpha\")]\npub mod alpha;"), "{full_lib}");
}

#[test]
fn every_feature_the_generated_code_names_is_a_declared_module_feature() {
    let apis = registry();
    let registry: Vec<&RestApi> = apis.iter().collect();
    let temp = tempfile::tempdir().unwrap();
    let crate_dir = temp.path().join("schema");
    generate(&registry, Selection::All, &crate_dir);

    let manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    let features = manifest_features(&manifest);
    let declared: BTreeSet<String> = features.keys().filter(|key| *key != "default").cloned().collect();
    let default: BTreeSet<String> = features["default"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    assert_eq!(default, declared, "the default set enables every module");

    let lib = fs::read_to_string(crate_dir.join("src/lib.rs")).unwrap();
    let prelude = fs::read_to_string(crate_dir.join("src/prelude.rs")).unwrap();
    assert_eq!(cfg_features(&lib), declared, "lib.rs gates exactly the declared modules");
    assert!(cfg_features(&prelude).is_subset(&declared), "{prelude}");
    assert_eq!(
        lib.matches("#[cfg(feature = \"alpha\")]").count(),
        1,
        "the shared `alpha` group is one declaration under one feature"
    );
    assert!(prelude.contains("#[cfg(feature = \"alpha\")]\npub use crate::alpha::AlphaOne;"), "{prelude}");
    assert!(prelude.contains("#[cfg(feature = \"alpha\")]\npub use crate::alpha::AlphaTwo;"), "{prelude}");
    let row = lib.find("doc = \" | [`beta`]").expect("beta has a doc row");
    let attribute = &lib[lib[..row].rfind("#![cfg_attr(").expect("the row is an attribute")..row];
    assert!(attribute.contains("feature = \"beta\""), "the doc row is gated with its module: {attribute}");
}

/// One feature selection of the real crate: what must resolve, what must
/// not, and whether the WebSocket transport is in the dependency graph.
struct FeatureCase {
    features: &'static [&'static str],
    present: &'static [&'static str],
    absent: &'static [&'static str],
    tungstenite: bool,
}

const FEATURE_CASES: &[FeatureCase] = &[
    FeatureCase {
        features: &[],
        present: &["prelude::Call"],
        absent: &["anthropic", "prelude::Anthropic", "elevenlabs_ws", "ws_shared", "ollama"],
        tungstenite: false,
    },
    FeatureCase {
        features: &["anthropic"],
        present: &["anthropic::Anthropic", "prelude::Anthropic"],
        absent: &["openai", "prelude::OpenAI", "elevenlabs_ws", "ws_shared"],
        tungstenite: false,
    },
    FeatureCase {
        features: &["elevenlabs_ws"],
        present: &[
            "elevenlabs_ws::define_api",
            "prelude::define_elevenlabs_ws_api_definition",
            "ws_shared::WsError",
        ],
        absent: &["anthropic", "prelude::Anthropic", "unfolded_circle_core_ws"],
        tungstenite: true,
    },
    FeatureCase {
        features: &["ollama"],
        present: &["ollama::OllamaNative", "ollama::OllamaOpenAI", "prelude::OllamaNative", "prelude::OllamaOpenAI"],
        absent: &["openai", "anthropic", "ws_shared"],
        tungstenite: false,
    },
    FeatureCase {
        features: &["anthropic", "elevenlabs_ws"],
        present: &[
            "anthropic::Anthropic",
            "prelude::Anthropic",
            "elevenlabs_ws::define_api",
            "prelude::define_elevenlabs_ws_api_definition",
        ],
        absent: &["openai", "unfolded_circle_core_ws"],
        tungstenite: true,
    },
];

fn feature_args(features: &[&str]) -> Vec<String> {
    let mut args = vec!["--no-default-features".to_string()];
    if !features.is_empty() {
        args.extend(["--features".to_string(), features.join(",")]);
    }
    args
}

fn run_with(fresh: &FreshCrate, base: &[&str], features: &[&str]) -> CargoRun {
    let feature_args = feature_args(features);
    let args: Vec<&str> = base.iter().copied().chain(feature_args.iter().map(String::as_str)).collect();
    fresh.cargo_offline(&args)
}

/// Replaces the crate's `tests/` with one probe that must compile and one
/// probe per item that must be absent.
fn write_probes(fresh: &FreshCrate, case: &FeatureCase) {
    let tests = fresh.crate_dir.join("tests");
    let _ = fs::remove_dir_all(&tests);
    let present: String = std::iter::once("#![allow(unused_imports)]\n".to_string())
        .chain(case.present.iter().map(|path| format!("use schematic_schema::{path} as _;\n")))
        .collect();
    fresh.add_test_file("present", &present);
    for (index, path) in case.absent.iter().enumerate() {
        fresh.add_test_file(&format!("absent_{index}"), &format!("use schematic_schema::{path} as _;\n"));
    }
}

#[test]
fn slow_feature_selections_compile_only_their_module_groups() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let apis = resolve_all_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Checkout, &lockfile);

    for case in FEATURE_CASES {
        let label = format!("--features {:?}", case.features);

        let lib = run_with(&fresh, &["check", "--lib"], case.features);
        lib.assert_success();
        assert!(
            !lib.stderr.contains("`schematic-schema` (lib) generated"),
            "{label}: the selected library builds without warnings\n{}",
            lib.stderr
        );

        write_probes(&fresh, case);
        let probes = run_with(&fresh, &["check", "--tests", "--keep-going"], case.features);
        assert!(
            !probes.stderr.contains("could not compile `schematic-schema` (test \"present\")"),
            "{label}: enabled items resolve\n{}",
            probes.stderr
        );
        for (index, path) in case.absent.iter().enumerate() {
            assert!(
                probes.stderr.contains(&format!("could not compile `schematic-schema` (test \"absent_{index}\")"))
                    && probes.stderr.contains(&format!("unresolved import `schematic_schema::{path}`")),
                "{label}: `{path}` must be absent\n{}",
                probes.stderr
            );
        }

        let tree = run_with(&fresh, &["tree", "-e", "normal", "--prefix", "none"], case.features);
        tree.assert_success();
        assert_eq!(
            tree.stdout.lines().any(|line| line.starts_with("tokio-tungstenite ")),
            case.tungstenite,
            "{label}: tokio-tungstenite in the dependency graph\n{}",
            tree.stdout
        );
    }
}
