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
//! guard proves the detectors fire.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;

use source_scan::{ident_offsets, is_ident, line_at, production_sources, sanitize};

/// Claudine's package names; any `rendezvous-*` package is Claudine's too.
const CLAUDINE_PACKAGES: &[&str] =
    &["claudine", "claudine-cli", "claudine-contract", "claudine-gen", "claudine-catalog-types"];

/// The crate names a Rust path would spell for those packages.
const CLAUDINE_CRATES: &[&str] = &[
    "claudine",
    "claudine_cli",
    "claudine_contract",
    "claudine_gen",
    "claudine_catalog_types",
    "rendezvous_core",
    "rendezvous_daemon",
    "rendezvous_client",
];

const ALL_TABLES: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];
const BUILT_TABLES: &[&str] = &["dependencies", "build-dependencies"];

fn is_claudine_package(name: &str) -> bool {
    CLAUDINE_PACKAGES.contains(&name) || name.starts_with("rendezvous-")
}

/// `path` with `.` and `..` resolved lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// A manifest graph rooted in a repository: `read` returns a manifest's text.
struct Graph<'a> {
    repo_root: PathBuf,
    workspace: toml::Table,
    read: &'a dyn Fn(&Path) -> Option<String>,
}

impl Graph<'_> {
    fn under_claudine(&self, manifest_dir: &Path) -> bool {
        manifest_dir.strip_prefix(&self.repo_root).is_ok_and(|relative| relative.starts_with("claudine"))
    }

    fn shown(&self, manifest: &Path) -> String {
        manifest.strip_prefix(&self.repo_root).unwrap_or(manifest).to_string_lossy().replace('\\', "/")
    }

    /// Every Claudine dependency reachable from `roots` (manifest paths), as
    /// `"<declaring manifest>: <dependency>"`.
    fn violations(&self, roots: &[PathBuf]) -> Vec<String> {
        self.walk(roots).0
    }

    /// The violations and every manifest the walk read.
    fn walk(&self, roots: &[PathBuf]) -> (Vec<String>, BTreeSet<String>) {
        let mut violations = Vec::new();
        let mut visited = BTreeSet::new();
        let mut pending: Vec<(PathBuf, &[&str])> = roots.iter().map(|root| (normalize(root), ALL_TABLES)).collect();
        while let Some((manifest, tables)) = pending.pop() {
            if !visited.insert(manifest.clone()) {
                continue;
            }
            let Some(text) = (self.read)(&manifest) else {
                violations.push(format!("{}: manifest is unreadable", self.shown(&manifest)));
                continue;
            };
            let document: toml::Table = match toml::from_str(&text) {
                Ok(document) => document,
                Err(error) => {
                    violations.push(format!("{}: manifest does not parse: {error}", self.shown(&manifest)));
                    continue;
                }
            };
            let manifest_dir = manifest.parent().expect("a manifest has a directory").to_path_buf();
            let package = document.get("package").and_then(|package| package.get("name")).and_then(toml::Value::as_str);
            if package.is_some_and(is_claudine_package) || self.under_claudine(&manifest_dir) {
                violations.push(format!("{}: is a Claudine package ({})", self.shown(&manifest), package.unwrap_or("?")));
                continue;
            }
            for (key, spec, base) in self.dependencies(&document, tables, &manifest_dir) {
                let name = spec.get("package").and_then(toml::Value::as_str).unwrap_or(&key);
                let path = spec.get("path").and_then(toml::Value::as_str).map(|path| normalize(&base.join(path)));
                if is_claudine_package(name) || path.as_deref().is_some_and(|dir| self.under_claudine(dir)) {
                    violations.push(format!("{}: depends on `{key}` ({name})", self.shown(&manifest)));
                } else if let Some(dir) = path {
                    pending.push((dir.join("Cargo.toml"), BUILT_TABLES));
                }
            }
        }
        violations.sort();
        (violations, visited.iter().map(|manifest| self.shown(manifest)).collect())
    }

    /// `(key, spec, base directory for its path)` for every entry of `tables`,
    /// top-level and under `target.*`, with `workspace = true` resolved.
    fn dependencies(&self, document: &toml::Table, tables: &[&str], manifest_dir: &Path) -> Vec<(String, toml::Table, PathBuf)> {
        let mut scopes = vec![document];
        if let Some(targets) = document.get("target").and_then(toml::Value::as_table) {
            scopes.extend(targets.values().filter_map(toml::Value::as_table));
        }
        let mut entries = Vec::new();
        for scope in scopes {
            for table in tables {
                let Some(dependencies) = scope.get(*table).and_then(toml::Value::as_table) else { continue };
                for (key, value) in dependencies {
                    let mut spec = match value {
                        toml::Value::Table(spec) => spec.clone(),
                        // `name = "1.0"`: a registry version.
                        _ => toml::Table::new(),
                    };
                    let mut base = manifest_dir.to_path_buf();
                    // A version-only workspace entry has no path or rename to inherit.
                    if spec.get("workspace").and_then(toml::Value::as_bool) == Some(true)
                        && let Some(toml::Value::Table(inherited)) = self.workspace.get(key)
                    {
                        for (field, inherited) in inherited {
                            spec.entry(field.clone()).or_insert_with(|| inherited.clone());
                        }
                        // A workspace entry's `path` is relative to the root.
                        base.clone_from(&self.repo_root);
                    }
                    entries.push((key.clone(), spec, base));
                }
            }
        }
        entries
    }
}

/// The root `[workspace.dependencies]` table, empty when there is none.
fn workspace_dependencies(root_manifest: &str) -> toml::Table {
    let root: toml::Table = toml::from_str(root_manifest).expect("the root Cargo.toml parses");
    root.get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default()
}

/// `"<file>:<line>: <crate>"` for every Claudine crate a sanitized source
/// names in a path, a `use`, or an `extern crate`.
fn source_violations(file: &str, sanitized: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for krate in CLAUDINE_CRATES {
        for offset in ident_offsets(sanitized, krate) {
            let before = sanitized[..offset].trim_end();
            let after = sanitized[offset + krate.len()..].trim_start();
            // `self::claudine::…` or `field.claudine` is a local name, not a crate.
            let qualified = before.strip_suffix("::").is_some_and(|rest| rest.bytes().last().is_some_and(is_ident));
            let member = before.ends_with('.');
            let names_crate = after.starts_with("::")
                || before.ends_with("extern crate")
                || (before.strip_suffix("use").is_some_and(|rest| !rest.bytes().last().is_some_and(is_ident)));
            if names_crate && !qualified && !member {
                violations.push(format!("{file}:{}: names `{krate}`", line_at(sanitized, offset)));
            }
        }
    }
    violations.sort();
    violations
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
    let graph = Graph { repo_root, workspace: workspace_dependencies(root_manifest), read: &read };
    let roots: Vec<PathBuf> = seeded.keys().cloned().collect();

    let (violations, reached) = graph.walk(&roots);
    assert!(violations.is_empty(), "Darkmatter must not depend on Claudine:\n{}", violations.join("\n"));
    // The walk follows path dependencies past the three roots.
    assert!(reached.contains("biscuit-file/lib/Cargo.toml"), "reached only {reached:?}");
}

#[test]
fn darkmatter_sources_never_name_a_claudine_crate() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above.
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
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
    let graph = Graph { repo_root: repo.clone(), workspace, read: &read };

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
