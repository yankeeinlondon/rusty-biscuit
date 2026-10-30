//! Darkmatter never depends on Claudine: Claudine adapts Darkmatter's
//! lifecycle engine, so the dependency points one way only.
//!
//! - **Manifests.** From `darkmatter`, `darkmatter-cli`, and `dmls`, every
//!   dependency table (`dependencies`, `dev-dependencies`,
//!   `build-dependencies`, and their `target.*` forms) is walked, `workspace =
//!   true` entries are resolved through the root `[workspace.dependencies]`,
//!   and every `path` dependency is followed to its manifest. A reached
//!   package fails the guard when its name is a Claudine-family crate or its
//!   manifest lives under `claudine/`. A transitively reached package
//!   contributes only the tables Cargo builds for a dependency (not its
//!   `dev-dependencies`).
//! - **Sources.** The three crates' production sources (comments, literals,
//!   and test-only code blanked by `source_scan`) must not name a Claudine
//!   crate in a path (`claudine::…`), a `use`, or an `extern crate`.
//!
//! Both checks run over in-memory inputs in the self-tests, so a passing
//! guard proves the detectors fire. The walk and the source detector are the
//! shared engine in `tests/common/dependency_graph.rs`, which
//! `simplified-schema`'s guard runs with its own forbidden set.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;
#[path = "../common/dependency_graph.rs"]
mod dependency_graph;

use dependency_graph::{Forbidden, Graph, normalize, workspace_dependencies};
use source_scan::{production_sources, sanitize};

/// Claudine's packages (any `rendezvous-*` package is Claudine's too), every
/// manifest under `claudine/`, and the crate names a Rust path would spell.
const CLAUDINE: Forbidden = Forbidden {
    packages: &["claudine", "claudine-cli", "claudine-contract", "claudine-gen", "claudine-catalog-types"],
    prefixes: &["rendezvous-"],
    suffixes: &[],
    dirs: &["claudine"],
    crates: &[
        "claudine",
        "claudine_cli",
        "claudine_contract",
        "claudine_gen",
        "claudine_catalog_types",
        "rendezvous_core",
        "rendezvous_daemon",
        "rendezvous_client",
    ],
};

/// `"<file>:<line>: <crate>"` for every Claudine crate a sanitized source
/// names in a path, a `use`, or an `extern crate`.
fn source_violations(file: &str, sanitized: &str) -> Vec<String> {
    dependency_graph::source_violations(file, sanitized, CLAUDINE.crates)
}

#[test]
fn darkmatter_manifests_never_reach_a_claudine_package() {
    let root_manifest = include_str!("../../../../Cargo.toml");
    let seeded: HashMap<PathBuf, &str> = [
        (biscuit_test_harness::manifest_dir!().join("Cargo.toml"), include_str!("../../Cargo.toml")),
        (biscuit_test_harness::manifest_dir!().join("../cli/Cargo.toml"), include_str!("../../../cli/Cargo.toml")),
        (biscuit_test_harness::manifest_dir!().join("../dmls/Cargo.toml"), include_str!("../../../dmls/Cargo.toml")),
    ]
    .into_iter()
    .map(|(path, text)| (normalize(&path), text))
    .collect();
    let repo_root = normalize(
        biscuit_test_harness::manifest_dir!().join("../../Cargo.toml").parent().expect("the root manifest's directory"),
    );
    let read = |path: &Path| seeded.get(path).map(|text| (*text).to_string()).or_else(|| std::fs::read_to_string(path).ok());
    let graph = Graph { repo_root, workspace: workspace_dependencies(root_manifest), read: &read, forbidden: &CLAUDINE };
    let roots: Vec<PathBuf> = seeded.keys().cloned().collect();

    let (violations, reached) = graph.walk(&roots);
    assert!(violations.is_empty(), "Darkmatter must not depend on Claudine:\n{}", violations.join("\n"));
    // The walk follows path dependencies past the three roots.
    assert!(reached.contains("biscuit-file/lib/Cargo.toml"), "reached only {reached:?}");
}

#[test]
fn darkmatter_sources_never_name_a_claudine_crate() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above.
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
    let _ = include_str!("../common/dependency_graph.rs");
    let crates: BTreeMap<&str, PathBuf> = [
        ("darkmatter/lib/src", biscuit_test_harness::manifest_dir!().join("src")),
        ("darkmatter/cli/src", biscuit_test_harness::manifest_dir!().join("../cli/src")),
        ("darkmatter/dmls/src", biscuit_test_harness::manifest_dir!().join("../dmls/src")),
    ]
    .into_iter()
    .collect();

    let mut scanned = 0;
    let mut violations = Vec::new();
    for (label, src) in &crates {
        for (file, text) in production_sources(src) {
            scanned += 1;
            violations.extend(source_violations(&format!("{label}/{file}"), &text));
        }
    }
    assert!(scanned > 100, "the scan reached the three crates' sources ({scanned} files)");
    assert!(violations.is_empty(), "Darkmatter source must not name a Claudine crate:\n{}", violations.join("\n"));
}

/// An in-memory repository: `darkmatter/lib` reaches `claudine-contract`
/// through a `cfg(unix)` path dependency of `helper`, and through a renamed
/// `workspace = true` entry; neither is a direct, top-level dependency.
#[test]
fn an_injected_claudine_dependency_is_detected() {
    let repo = PathBuf::from("/repo");
    let files: HashMap<PathBuf, &str> = [
        (
            "darkmatter/lib/Cargo.toml",
            "[package]\nname = \"darkmatter\"\n[dependencies]\nhelper = { path = \"../../helper\" }\n\
             [dev-dependencies]\ncontract = { workspace = true }\n",
        ),
        (
            "helper/Cargo.toml",
            "[package]\nname = \"helper\"\n[target.'cfg(unix)'.dependencies]\n\
             bridge = { package = \"claudine-contract\", path = \"../claudine/contract\" }\n\
             [dev-dependencies]\nhidden = { path = \"../claudine/lib\" }\n",
        ),
        ("clean/Cargo.toml", "[package]\nname = \"clean\"\n[dependencies]\nserde = \"1\"\nhelper = { path = \"../helper2\" }\n"),
        ("helper2/Cargo.toml", "[package]\nname = \"helper2\"\n[dev-dependencies]\nclaudine = { path = \"../claudine/lib\" }\n"),
    ]
    .into_iter()
    .map(|(path, text)| (repo.join(path), text))
    .collect();
    let read = |path: &Path| files.get(path).map(|text| (*text).to_string());
    let workspace = workspace_dependencies(
        "[workspace.dependencies]\ncontract = { package = \"claudine-contract\", path = \"claudine/contract\" }\n",
    );
    let graph = Graph { repo_root: repo.clone(), workspace, read: &read, forbidden: &CLAUDINE };

    assert_eq!(
        graph.violations(&[repo.join("darkmatter/lib/Cargo.toml")]),
        [
            "darkmatter/lib/Cargo.toml: depends on `contract` (claudine-contract)",
            "helper/Cargo.toml: depends on `bridge` (claudine-contract)",
        ]
    );
    // A dependency's own dev-dependencies are never built for Darkmatter.
    assert_eq!(graph.violations(&[repo.join("clean/Cargo.toml")]), Vec::<String>::new());
}

#[test]
fn an_injected_claudine_path_in_source_is_detected() {
    let source = r#"
use claudine::composition::LifecycleSignal;
extern crate rendezvous_core;
use claudine_contract;
// claudine::in_a_comment
const TEXT: &str = "claudine::in_a_string";
fn local(claudine: u8, this: &Self) -> u8 { self::claudine::x(); this.claudine; claudine }
"#;
    let sanitized = String::from_utf8(sanitize(source)).expect("sanitizing keeps UTF-8");

    assert_eq!(
        source_violations("injected.rs", &sanitized),
        ["injected.rs:2: names `claudine`", "injected.rs:3: names `rendezvous_core`", "injected.rs:4: names `claudine_contract`"]
    );
}
