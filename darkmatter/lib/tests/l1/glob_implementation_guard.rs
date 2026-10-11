//! Acceptance criterion 2 of `2026-09-30-glob-reference`: biscuit-file's
//! `GlobReference` is the one glob implementation behind file references.
//! Production code of the six packages that consume it reaches a glob library
//! only from the files listed in [`SOURCE_ALLOWLIST`], at exactly the listed
//! counts; a new file, a moved count, or a stale entry fails.
//!
//! Two checks, because neither alone sees every glob library:
//!
//! - **Source.** Each package's `src/` is read under the production-scope rule
//!   of `source_scan::production_sources` (comments, literals, and test-only
//!   code blanked) and searched on identifier boundaries for the crate names and
//!   matcher types in [`GLOB_LIBRARY_IDENTIFIERS`], plus `Glob::` (globset's
//!   constructor path; a bare `Glob` is an enum variant in `FileLinksMode` and
//!   Claudine's `PathSegment`).
//! - **Manifest.** Each package's normal, build, and target dependencies may
//!   name a crate from [`GLOB_CRATES`] only as [`SCANNED`] lists. This is what
//!   catches the `glob` crate: its name is also biscuit-file's own
//!   `file_reference::glob` module, so the source check cannot search for it.
//!
//! The `ignore` crate is admitted as a directory walker (`WalkBuilder`, used by
//! DMLS's workspace discovery and Claudine's completion walks); its glob
//! matchers (`OverrideBuilder`, `GitignoreBuilder`, `TypesBuilder`) are not.
//! Other packages' allowlisted files are declared in this package's
//! `[package.metadata.ci.tests] source-inputs`, so changing one runs this test.

// `semantic_results_never_persist.rs` loads the same stateless file.
#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use source_scan::{ident_offsets, line_at, production_sources};

/// A package the guard scans, by repository-relative path.
struct ScannedPackage {
    src: &'static str,
    manifest: &'static str,
    /// Exactly the [`GLOB_CRATES`] its non-dev dependencies may name.
    glob_dependencies: &'static [&'static str],
}

const SCANNED: &[ScannedPackage] = &[
    ScannedPackage { src: "biscuit-file/lib/src", manifest: "biscuit-file/lib/Cargo.toml", glob_dependencies: &["globset"] },
    ScannedPackage { src: "darkmatter/lib/src", manifest: "darkmatter/lib/Cargo.toml", glob_dependencies: &["globset"] },
    ScannedPackage { src: "darkmatter/cli/src", manifest: "darkmatter/cli/Cargo.toml", glob_dependencies: &[] },
    ScannedPackage {
        src: "darkmatter/dmls/src",
        manifest: "darkmatter/dmls/Cargo.toml",
        glob_dependencies: &["globset", "ignore"],
    },
    ScannedPackage { src: "claudine/lib/src", manifest: "claudine/lib/Cargo.toml", glob_dependencies: &[] },
    ScannedPackage { src: "claudine/cli/src", manifest: "claudine/cli/Cargo.toml", glob_dependencies: &["ignore"] },
];

/// Crates that implement glob matching (Cargo spelling).
const GLOB_CRATES: &[&str] = &["globset", "glob", "wax", "globwalk", "wildmatch", "fast-glob", "glob-match", "ignore"];

/// Identifiers that only a glob library supplies: the crate roots a `use` or
/// path must name (Rust spelling) and the matcher types a re-export could
/// carry without naming the crate.
const GLOB_LIBRARY_IDENTIFIERS: &[&str] = &[
    "globset",
    "wax",
    "globwalk",
    "wildmatch",
    "fast_glob",
    "glob_match",
    "GlobBuilder",
    "GlobSet",
    "GlobSetBuilder",
    "GlobMatcher",
    "OverrideBuilder",
    "GitignoreBuilder",
    "TypesBuilder",
];

/// Production files that may use a glob library: repository-relative path,
/// exact hit count, and why the file is not a second glob implementation.
const SOURCE_ALLOWLIST: &[(&str, usize, &str)] = &[
    (
        "biscuit-file/lib/src/file_reference/glob/parse.rs",
        6,
        "`GlobReference` compiles every pattern payload here",
    ),
    (
        "biscuit-file/lib/src/file_reference/glob/roots.rs",
        3,
        "`GlobReference` holds each prepared root's compiled matcher",
    ),
    (
        "darkmatter/lib/src/markdown/compose/toc_linking/filter.rs",
        7,
        "`::toc-linking` keep/filter globs match heading text, not files",
    ),
    (
        "darkmatter/dmls/src/workspace/discover.rs",
        9,
        "workspace include/exclude globs are editor configuration, not references",
    ),
    (
        "darkmatter/dmls/src/overlay/schema.rs",
        4,
        "schema-extension activation globs are editor configuration, not references",
    ),
];

/// Offsets of every glob-library hit in sanitized source, with the identifier.
fn glob_library_hits(text: &str) -> Vec<(usize, &'static str)> {
    let mut hits: Vec<(usize, &'static str)> = GLOB_LIBRARY_IDENTIFIERS
        .iter()
        .flat_map(|ident| ident_offsets(text, ident).into_iter().map(move |offset| (offset, *ident)))
        .collect();
    hits.extend(
        ident_offsets(text, "Glob")
            .into_iter()
            .filter(|&offset| text[offset + "Glob".len()..].trim_start().starts_with("::"))
            .map(|offset| (offset, "Glob::")),
    );
    hits.sort_unstable();
    hits
}

/// Every source problem under `root` against `allowlist`.
fn check_sources(root: &Path, packages: &[ScannedPackage], allowlist: &[(&str, usize, &str)]) -> Vec<String> {
    let allowed: BTreeMap<&str, usize> = allowlist.iter().map(|&(path, count, _)| (path, count)).collect();
    assert_eq!(allowed.len(), allowlist.len(), "duplicate allowlist entry");
    let mut problems = Vec::new();
    let mut found = BTreeSet::new();
    for package in packages {
        for (relative, text) in production_sources(&root.join(package.src)) {
            let hits = glob_library_hits(&text);
            if hits.is_empty() {
                continue;
            }
            let path = format!("{}/{relative}", package.src);
            let located: Vec<String> =
                hits.iter().map(|&(offset, ident)| format!("{ident}@{}", line_at(&text, offset))).collect();
            match allowed.get(path.as_str()) {
                None => problems.push(format!(
                    "glob library used in {path} ({}); match file references through biscuit-file's `GlobReference`",
                    located.join(", ")
                )),
                Some(&expected) if expected != hits.len() => problems.push(format!(
                    "stale allowlist count for {path}: expected {expected}, found {} ({})",
                    hits.len(),
                    located.join(", ")
                )),
                Some(_) => {}
            }
            found.insert(path);
        }
    }
    for &(path, _, _) in allowlist {
        if !found.contains(path) {
            problems.push(format!("stale allowlist entry: {path} no longer uses a glob library"));
        }
    }
    problems
}

/// The [`GLOB_CRATES`] a manifest's normal, build, and target-specific
/// dependency tables name, by package name (a `package = ".."` rename counts as
/// the crate it renames). Dev-dependencies are tests' business.
fn glob_dependencies(manifest: &Path) -> BTreeSet<String> {
    let toml = biscuit_file::Toml::new(manifest).unwrap_or_else(|error| panic!("parse {}: {error}", manifest.display()));
    let value = toml.as_json_value().expect("TOML converts to JSON");
    let mut tables = Vec::new();
    for key in ["dependencies", "build-dependencies"] {
        tables.extend(value.get(key));
    }
    if let Some(targets) = value.get("target").and_then(|target| target.as_object()) {
        for target in targets.values() {
            for key in ["dependencies", "build-dependencies"] {
                tables.extend(target.get(key));
            }
        }
    }
    let mut names = BTreeSet::new();
    for table in tables {
        let table = table.as_object().unwrap_or_else(|| panic!("{}: a dependency table is a table", manifest.display()));
        for (key, spec) in table {
            let name = spec.get("package").and_then(|package| package.as_str()).unwrap_or(key);
            if GLOB_CRATES.contains(&name) {
                names.insert(name.to_string());
            }
        }
    }
    names
}

/// Every manifest whose glob-crate dependencies differ from [`SCANNED`].
fn check_manifests(root: &Path, packages: &[ScannedPackage]) -> Vec<String> {
    let mut problems = Vec::new();
    for package in packages {
        let found = glob_dependencies(&root.join(package.manifest));
        let expected: BTreeSet<String> = package.glob_dependencies.iter().map(|name| name.to_string()).collect();
        for added in found.difference(&expected) {
            problems.push(format!("{} depends on glob crate `{added}`, which the guard does not admit", package.manifest));
        }
        for removed in expected.difference(&found) {
            problems.push(format!("stale manifest entry: {} no longer depends on `{removed}`", package.manifest));
        }
    }
    problems
}

#[test]
fn file_references_have_one_glob_implementation() {
    let root = biscuit_test_harness::manifest_dir!()
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above darkmatter/lib")
        .to_path_buf();
    let mut problems = check_sources(&root, SCANNED, SOURCE_ALLOWLIST);
    problems.extend(check_manifests(&root, SCANNED));
    assert!(problems.is_empty(), "glob implementation guard:\n  {}", problems.join("\n  "));
}

/// The guard catches a planted glob-library use, a moved count, a stale entry,
/// and an undeclared glob crate, and ignores test code, comments, literals,
/// enum variants named `Glob`, and dev-dependencies, so it cannot pass
/// vacuously.
#[test]
fn the_guard_catches_planted_violations_and_honors_the_scope_rule() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "a/src/lib.rs",
        "// A GlobBuilder in a comment.\n\
         const HINT: &str = \"see globset::GlobBuilder\";\n\
         pub enum Mode { Glob(String) }\n\
         fn mode(m: Mode) { if let Mode::Glob(g) = m {} }\n\
         #[cfg(test)]\nmod tests {\n    fn t() { globset::GlobBuilder::new(\"*\"); }\n}\n\
         mod glob;\npub use glob::Thing;\n",
    );
    write("a/src/planted.rs", "fn p() { let _ = GlobBuilder::new(\"*.md\"); }\n");
    write("a/src/kept.rs", "use globset::{Glob, GlobSetBuilder};\nfn k() { Glob::new(\"*\"); }\n");
    write("a/src/moved.rs", "use globset::GlobMatcher;\n");
    write(
        "a/Cargo.toml",
        "[package]\nname = \"a\"\n\n[dependencies]\nglobset = \"0.4\"\nwalker = { package = \"ignore\", version = \"0.4\" }\n\n\
         [target.'cfg(unix)'.dependencies]\nwax = \"0.6\"\n\n[dev-dependencies]\nglob = \"0.3\"\n",
    );
    write("b/src/lib.rs", "pub fn b() {}\n");
    write("b/Cargo.toml", "[package]\nname = \"b\"\n");

    let packages = [
        ScannedPackage { src: "a/src", manifest: "a/Cargo.toml", glob_dependencies: &["globset", "ignore"] },
        ScannedPackage { src: "b/src", manifest: "b/Cargo.toml", glob_dependencies: &["globset"] },
    ];
    let allowlist = [("a/src/kept.rs", 3, "kept"), ("a/src/moved.rs", 1, "moved"), ("b/src/gone.rs", 1, "gone")];

    let mut problems = check_sources(root.path(), &packages, &allowlist);
    problems.extend(check_manifests(root.path(), &packages));
    problems.sort();
    let expected = [
        "a/Cargo.toml depends on glob crate `wax`, which the guard does not admit",
        "glob library used in a/src/planted.rs (GlobBuilder@1); match file references through biscuit-file's `GlobReference`",
        "stale allowlist count for a/src/moved.rs: expected 1, found 2 (globset@1, GlobMatcher@1)",
        "stale allowlist entry: b/src/gone.rs no longer uses a glob library",
        "stale manifest entry: b/Cargo.toml no longer depends on `globset`",
    ];
    assert_eq!(problems, expected);
}
