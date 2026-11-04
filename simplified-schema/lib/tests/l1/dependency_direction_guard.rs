//! The schema language sits below its hosts: `simplified-schema` never reaches
//! Darkmatter, DMLS, Claudine, Content Policy, or any CLI, through its
//! manifests (every feature and table, `dev-dependencies` included, followed
//! through path dependencies) or by naming one of their crates in production
//! source. The walk and the source detector are Darkmatter's shared engine
//! (`darkmatter/lib/tests/common/dependency_graph.rs`), run here with the
//! library's own forbidden set; the self-tests prove the set is load-bearing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[allow(clippy::duplicate_mod)]
#[path = "../../../../darkmatter/cli/tests/common/source_scan.rs"]
mod source_scan;
#[path = "../../../../darkmatter/lib/tests/common/dependency_graph.rs"]
mod dependency_graph;

use dependency_graph::{Forbidden, Graph, normalize, workspace_dependencies};
use source_scan::{production_sources, sanitize};

/// The hosts and tools above the language: Darkmatter and its language server
/// and editor packages, Claudine (and its `rendezvous-*` packages), Content
/// Policy's runtime, and every CLI.
const HOSTS: Forbidden = Forbidden {
    packages: &[
        "darkmatter",
        "dmls",
        "content-policy",
        "claudine",
        "claudine-contract",
        "claudine-gen",
        "claudine-catalog-types",
    ],
    prefixes: &["rendezvous-", "zed-dmls"],
    suffixes: &["-cli"],
    dirs: &["darkmatter", "claudine", "content-policy"],
    crates: &[
        "darkmatter",
        "darkmatter_cli",
        "dmls",
        "content_policy",
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

fn repo_root() -> PathBuf {
    normalize(biscuit_test_harness::manifest_dir!().join("../..").as_path())
}

#[test]
fn the_library_manifest_never_reaches_a_host_package() {
    let root_manifest = include_str!("../../../../Cargo.toml");
    let manifest = normalize(&biscuit_test_harness::manifest_dir!().join("Cargo.toml"));
    let seeded: HashMap<PathBuf, &str> = [(manifest.clone(), include_str!("../../Cargo.toml"))].into_iter().collect();
    let read = |path: &Path| seeded.get(path).map(|text| (*text).to_string()).or_else(|| std::fs::read_to_string(path).ok());
    let graph = Graph { repo_root: repo_root(), workspace: workspace_dependencies(root_manifest), read: &read, forbidden: &HOSTS };

    let (violations, reached) = graph.walk(&[manifest]);
    assert!(violations.is_empty(), "simplified-schema must not depend on a host:\n{}", violations.join("\n"));
    // The walk followed path dependencies, optional ones included.
    for expected in ["biscuit-file/lib/Cargo.toml", "sniff/lib/Cargo.toml", "biscuit-terminal/lib/Cargo.toml"] {
        assert!(reached.contains(expected), "{expected} was not reached; reached only {reached:?}");
    }
}

#[test]
fn library_sources_never_name_a_host_crate() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    let _ = include_str!("../../../../darkmatter/lib/tests/common/dependency_graph.rs");
    let mut scanned = 0;
    let mut violations = Vec::new();
    for (file, text) in production_sources(&biscuit_test_harness::manifest_dir!().join("src")) {
        scanned += 1;
        violations.extend(dependency_graph::source_violations(&format!("simplified-schema/lib/src/{file}"), &text, HOSTS.crates));
    }
    assert!(scanned > 50, "the scan reached the library's sources ({scanned} files)");
    assert!(violations.is_empty(), "simplified-schema source must not name a host crate:\n{}", violations.join("\n"));
}

/// An in-memory repository: the library reaches Darkmatter through a helper's
/// `cfg(unix)` table, a CLI through a dev-dependency, Content Policy through a
/// renamed workspace entry, and a package that lives under `claudine/`.
#[test]
fn an_injected_host_dependency_is_detected() {
    let repo = PathBuf::from("/repo");
    let files: HashMap<PathBuf, &str> = [
        (
            "simplified-schema/lib/Cargo.toml",
            "[package]\nname = \"simplified-schema\"\n[dependencies]\nhelper = { path = \"../../helper\" }\n\
             policy = { workspace = true }\n[dev-dependencies]\nmd = { package = \"darkmatter-cli\", path = \"../../tools/md\" }\n",
        ),
        (
            "helper/Cargo.toml",
            "[package]\nname = \"helper\"\n[target.'cfg(unix)'.dependencies]\nhost = { package = \"darkmatter\", path = \"../host\" }\n\
             neutral = { path = \"../claudine/neutral\" }\n",
        ),
    ]
    .into_iter()
    .map(|(path, text)| (repo.join(path), text))
    .collect();
    let read = |path: &Path| files.get(path).map(|text| (*text).to_string());
    let workspace =
        workspace_dependencies("[workspace.dependencies]\npolicy = { package = \"content-policy\", path = \"policy\" }\n");
    let graph = Graph { repo_root: repo.clone(), workspace, read: &read, forbidden: &HOSTS };

    assert_eq!(
        graph.violations(&[repo.join("simplified-schema/lib/Cargo.toml")]),
        [
            "helper/Cargo.toml: depends on `host` (darkmatter)",
            "helper/Cargo.toml: depends on `neutral` (neutral)",
            "simplified-schema/lib/Cargo.toml: depends on `md` (darkmatter-cli)",
            "simplified-schema/lib/Cargo.toml: depends on `policy` (content-policy)",
        ]
    );
}

#[test]
fn an_injected_host_path_in_source_is_detected() {
    let source = r#"
use darkmatter::markdown::Markdown;
extern crate content_policy;
fn f() { dmls::serve(); }
// darkmatter::in_a_comment
const TEXT: &str = "darkmatter::in_a_string";
fn register_darkmatter_formats(darkmatter: u8) -> u8 { self::darkmatter::x(); darkmatter }
"#;
    let sanitized = String::from_utf8(sanitize(source)).expect("sanitizing keeps UTF-8");

    assert_eq!(
        dependency_graph::source_violations("injected.rs", &sanitized, HOSTS.crates),
        ["injected.rs:2: names `darkmatter`", "injected.rs:3: names `content_policy`", "injected.rs:4: names `dmls`"]
    );
}
