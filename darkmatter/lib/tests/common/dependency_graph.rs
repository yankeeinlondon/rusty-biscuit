//! Shared engine for every package's `dependency_direction_guard.rs`: a
//! package must never reach a [`Forbidden`] package, through its manifests or
//! by naming the crate in production source.
//!
//! - **Manifests.** From each root manifest, every dependency table
//!   (`dependencies`, `dev-dependencies`, `build-dependencies`, and their
//!   `target.*` forms) is walked, `workspace = true` entries are resolved
//!   through the root `[workspace.dependencies]`, and every `path` dependency
//!   is followed to its manifest. A reached package fails the guard when its
//!   name or its manifest's directory is forbidden. A transitively reached
//!   package contributes only the tables Cargo builds for a dependency (not
//!   its `dev-dependencies`).
//! - **Sources.** Production source (comments, literals, and test-only code
//!   blanked by `source_scan`) must not name a forbidden crate in a path
//!   (`krate::…`), a `use`, or an `extern crate`.
//!
//! A package other than `darkmatter` that includes this file declares it and
//! `source_scan.rs` in `[package.metadata.ci.tests] source-inputs`, and spells
//! both with `include_str!` inside the guard test, because CI's test-input
//! index does not count a `#[path]` include as a reference.

// Each including test binary uses a different subset.
#![allow(dead_code)]

// The including binary's other source-scan gates may load `source_scan.rs`
// too; the copies are stateless, so loading it twice is harmless.
#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use source_scan::{ident_offsets, is_ident, line_at};

/// What a guarded package must never reach.
pub struct Forbidden {
    /// Package names (Cargo spelling).
    pub packages: &'static [&'static str],
    /// Package-name prefixes (`rendezvous-`).
    pub prefixes: &'static [&'static str],
    /// Package-name suffixes (`-cli`).
    pub suffixes: &'static [&'static str],
    /// Repository-relative directories whose manifests are forbidden whatever
    /// the package is called (`claudine`).
    pub dirs: &'static [&'static str],
    /// The crate names a Rust path would spell for those packages.
    pub crates: &'static [&'static str],
}

impl Forbidden {
    fn package(&self, name: &str) -> bool {
        self.packages.contains(&name)
            || self.prefixes.iter().any(|prefix| name.starts_with(prefix))
            || self.suffixes.iter().any(|suffix| name.ends_with(suffix))
    }
}

const ALL_TABLES: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];
const BUILT_TABLES: &[&str] = &["dependencies", "build-dependencies"];

/// `path` with `.` and `..` resolved lexically.
pub fn normalize(path: &Path) -> PathBuf {
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
pub struct Graph<'a> {
    pub repo_root: PathBuf,
    pub workspace: toml::Table,
    pub read: &'a dyn Fn(&Path) -> Option<String>,
    pub forbidden: &'a Forbidden,
}

impl Graph<'_> {
    fn under_forbidden_dir(&self, manifest_dir: &Path) -> bool {
        manifest_dir
            .strip_prefix(&self.repo_root)
            .is_ok_and(|relative| self.forbidden.dirs.iter().any(|dir| relative.starts_with(dir)))
    }

    fn shown(&self, manifest: &Path) -> String {
        manifest.strip_prefix(&self.repo_root).unwrap_or(manifest).to_string_lossy().replace('\\', "/")
    }

    /// Every forbidden dependency reachable from `roots` (manifest paths), as
    /// `"<declaring manifest>: <dependency>"`.
    pub fn violations(&self, roots: &[PathBuf]) -> Vec<String> {
        self.walk(roots).0
    }

    /// The violations and every manifest the walk read.
    pub fn walk(&self, roots: &[PathBuf]) -> (Vec<String>, BTreeSet<String>) {
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
            if package.is_some_and(|name| self.forbidden.package(name)) || self.under_forbidden_dir(&manifest_dir) {
                violations.push(format!("{}: is a forbidden package ({})", self.shown(&manifest), package.unwrap_or("?")));
                continue;
            }
            for (key, spec, base) in self.dependencies(&document, tables, &manifest_dir) {
                let name = spec.get("package").and_then(toml::Value::as_str).unwrap_or(&key);
                let path = spec.get("path").and_then(toml::Value::as_str).map(|path| normalize(&base.join(path)));
                if self.forbidden.package(name) || path.as_deref().is_some_and(|dir| self.under_forbidden_dir(dir)) {
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
pub fn workspace_dependencies(root_manifest: &str) -> toml::Table {
    let root: toml::Table = toml::from_str(root_manifest).expect("the root Cargo.toml parses");
    root.get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default()
}

/// `"<file>:<line>: <crate>"` for every one of `crates` a sanitized source
/// names in a path, a `use`, or an `extern crate`.
pub fn source_violations(file: &str, sanitized: &str, crates: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    for krate in crates {
        for offset in ident_offsets(sanitized, krate) {
            let before = sanitized[..offset].trim_end();
            let after = sanitized[offset + krate.len()..].trim_start();
            // `self::krate::…` or `field.krate` is a local name, not a crate.
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
