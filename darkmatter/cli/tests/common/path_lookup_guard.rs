//! Shared engine for every package's `path_lookup_guard.rs`: a source guard
//! that keeps filesystem canonicalization and home-directory lookup on
//! biscuit-file's shared helpers.
//!
//! - **Canonicalize.** `std::fs::canonicalize`, `fs::canonicalize` (std or
//!   tokio), `Path::canonicalize` / `PathBuf::canonicalize`, the
//!   zero-argument method `.canonicalize()`, and `dunce::canonicalize`. On
//!   Windows these return verbatim (`\\?\C:\…`) spellings that the
//!   file-reference grammar rejects and that compare unequal to the legacy
//!   spelling another producer wrote, so a result that leaves a private
//!   comparison goes through `biscuit_file::canonicalize_simplified`.
//! - **Home lookup.** `dirs::home_dir`, `std::env::home_dir`, and
//!   `home::home_dir`. These disagree on native Windows (`dirs` ignores
//!   `USERPROFILE`), so a package under this rule reads its home through
//!   `biscuit_file::home_dir`, the one approved process-capture point, which
//!   owns the policy (environment first, relative home is no home).
//!
//! Each form is recognized when written qualified (`std::fs::canonicalize`),
//! as a function reference (`.map(std::fs::canonicalize)`) as well as a call,
//! and through the file's `use` statements: module aliases
//! (`use std::fs as filesystem;` then `filesystem::canonicalize`), bare and
//! renamed imports, chains of them (`use filesystem::canonicalize as canon;`,
//! `use canon as c;`), grouped and nested trees
//! (`use std::{fs::{self as f}};`), globs from a provider (`use std::fs::*;`),
//! and `self::`/`super::` paths into the file's own inline modules. Imports
//! are scoped as Rust scopes them: a `fn` body sees its own `use` statements
//! and those of the module around it; an inline `mod` sees only its own, plus
//! what a glob such as `use super::*` brings in. An import in one module never
//! decides a name in another.
//!
//! **Scope.** A package's production source: the module tree of every
//! `[lib]` and `[[bin]]` root its manifest declares or Cargo infers, and of its
//! build script, followed through `mod` declarations wherever they lead
//! (`#[path]` destinations outside the root's directory included, every
//! `#[cfg_attr(.., path = ..)]` branch included), plus every file under `src/`
//! so an orphan there is not missed. Nothing else is walked, so a root at the
//! package top does not pull in `tests/`. All of it is read under
//! `source_scan`'s production-scope rule (comments, literals, and
//! `#[cfg(test)]` code blanked; test-only module files never reached). Every
//! other `#[cfg(..)]` branch is scanned. A `[[bin]]` is production wherever it
//! lives; only one the guard call lists as a [`FixtureBin`] is skipped, and the
//! listing is checked against the manifest. The manifest is read with a TOML
//! parser, so any spelling Cargo accepts selects the same targets; a manifest
//! that is not TOML, a wrong-typed or empty `[lib].path`, `[[bin]].path`, or
//! `[package].build`, a target whose source is missing, a declared module with
//! no file, or a scan that finds no file fails: an empty scan cannot pass.
//!
//! **Exceptions.** A remaining direct call is an [`Exception`] keyed by rule,
//! repository-relative path, enclosing item (`Type::method`, `outer::inner`),
//! and operation, with its exact occurrence count and a reason. Line numbers
//! appear only in diagnostics, so moving a call keeps its exception. A new
//! call, a changed count, an unused (stale) or duplicate entry, an entry
//! without a reason, and an entry whose [`Kind`] does not fit the site all
//! fail, each reported with `path:line`.
//!
//! **Unresolved candidates.** A `canonicalize` or `home_dir` the scanner
//! cannot attribute is reported for explicit review rather than ignored: an
//! unknown qualifier (`descriptor::canonicalize`), a name or alias imported
//! from a module it cannot place (`use other::home_dir as home;` then
//! `home()`, `use crate::util::canonicalize;`), a chain too deep or cyclic to
//! follow, and a called bare name with no import or definition in scope. A
//! reviewed one is listed as [`Kind::Reviewed`] with what it actually calls.
//! Definitions (`fn canonicalize`), and calls that resolve to a free function
//! the file defines in scope, are not candidates; that is what keeps
//! Darkmatter's style-name normalizer out.
//!
//! **Limits.** This is a source guard, not proof about compiled code. Calls
//! produced by macro expansion (`macro_rules!` bodies invoked elsewhere,
//! procedural macros), modules a macro declares outside `src/`, files pulled
//! in with `include!` (build-script output included), and a function
//! re-exported under a new name by another file and called through that file's
//! module (`pub use std::fs::canonicalize as canon;` in one file,
//! `util::canon(..)` in another) are not followed; a re-export inside the same
//! file is. A method named `canonicalize` that takes arguments is taken to be
//! a non-filesystem method (the permission backends' policy
//! canonicalization): `Path::canonicalize` takes none. A local variable that
//! shadows an imported guarded name is reported like the import.
//!
//! A package other than `darkmatter-cli` that includes this file declares it
//! and `source_scan.rs` in `[package.metadata.ci.tests] source-inputs` and
//! spells both with `include_str!` inside its guard test, because CI's
//! test-input index does not count a `#[path]` include as a reference.

// Each including test binary uses a different subset.
#![allow(dead_code)]

// A binary's other source-scan gate may load `source_scan.rs` too; the copies
// are stateless, so loading it twice is harmless.
#[allow(clippy::duplicate_mod)]
#[path = "source_scan.rs"]
mod source_scan;

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use source_scan::{ident_offsets, is_ident, line_at, matching_brace, module_tree_sources, normalize_relative, production_sources};

/// What a guarded call is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    Canonicalize,
    HomeLookup,
}

impl Rule {
    fn helper(self) -> &'static str {
        match self {
            Self::Canonicalize => "biscuit_file::canonicalize_simplified",
            Self::HomeLookup => "biscuit_file::home_dir",
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Canonicalize => "canonicalize",
            Self::HomeLookup => "home-lookup",
        })
    }
}

/// Why an exception is allowed to stay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A direct call whose result stays inside a private comparison (both
    /// operands canonicalized identically; only a bool, a dedupe entry, or a
    /// relative suffix leaves), or the shared helper's own call. The reason
    /// names that invariant.
    Invariant,
    /// An unresolved candidate a reviewer judged not to be the guarded
    /// function; the reason names what it calls.
    Reviewed,
}

/// One allowed `(rule, path, item, operation)` with its exact occurrence
/// count. `path` is repository-relative and `/`-separated; `item` and
/// `operation` are the labels the scan reports (see the failure message for a
/// new site).
#[derive(Clone, Copy, Debug)]
pub struct Exception {
    pub rule: Rule,
    pub kind: Kind,
    pub path: &'static str,
    pub item: &'static str,
    pub operation: &'static str,
    pub count: usize,
    pub reason: &'static str,
}

/// One guarded call or candidate in production source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub rule: Rule,
    pub path: String,
    pub item: String,
    pub operation: String,
    pub line: usize,
    /// `false` for an unresolved candidate.
    pub resolved: bool,
}

/// The spellings one rule recognizes.
struct Target {
    rule: Rule,
    name: &'static str,
    /// Last path segments that make `segment::name` the guarded function.
    modules: &'static [&'static str],
    /// Last path segments that make `segment::name` the approved helper.
    helpers: &'static [&'static str],
    /// Whether a zero-argument `.name()` method call is the guarded function.
    method: bool,
}

const CANONICALIZE: Target = Target {
    rule: Rule::Canonicalize,
    name: "canonicalize",
    modules: &["fs", "dunce", "Path", "PathBuf"],
    helpers: &[],
    method: true,
};

const HOME_LOOKUP: Target = Target {
    rule: Rule::HomeLookup,
    name: "home_dir",
    modules: &["dirs", "env", "home"],
    helpers: &["biscuit_file"],
    method: false,
};

fn target(rule: Rule) -> &'static Target {
    match rule {
        Rule::Canonicalize => &CANONICALIZE,
        Rule::HomeLookup => &HOME_LOOKUP,
    }
}

/// A `[[bin]]` the guard does not scan because it is a test fixture, named by
/// the package's guard call. Its path alone never decides this: a `[[bin]]`
/// under `tests/` is production unless listed here. A listed bin must exist,
/// live under `tests/`, and declare `required-features`, so a default build and
/// `cargo install` skip it; `reason` names what drives it.
#[derive(Clone, Copy, Debug)]
pub struct FixtureBin {
    pub name: &'static str,
    pub reason: &'static str,
}

/// Fails with every problem the guard finds in the package at `manifest_dir`,
/// which `package_dir` names relative to the repository root (`claudine/lib`).
pub fn assert_guarded(package_dir: &str, manifest_dir: &Path, rules: &[Rule], exceptions: &[Exception]) {
    assert_guarded_with_fixtures(package_dir, manifest_dir, rules, exceptions, &[]);
}

/// [`assert_guarded`] for a package with test-fixture binaries.
pub fn assert_guarded_with_fixtures(
    package_dir: &str,
    manifest_dir: &Path,
    rules: &[Rule],
    exceptions: &[Exception],
    fixtures: &[FixtureBin],
) {
    let problems = problems_with_fixtures(package_dir, manifest_dir, rules, exceptions, fixtures);
    assert!(
        problems.is_empty(),
        "{package_dir} path lookup guard ({}):\n  {}",
        manifest_dir.display(),
        problems.join("\n  ")
    );
}

pub fn problems(package_dir: &str, manifest_dir: &Path, rules: &[Rule], exceptions: &[Exception]) -> Vec<String> {
    problems_with_fixtures(package_dir, manifest_dir, rules, exceptions, &[])
}

pub fn problems_with_fixtures(
    package_dir: &str,
    manifest_dir: &Path,
    rules: &[Rule],
    exceptions: &[Exception],
    fixtures: &[FixtureBin],
) -> Vec<String> {
    let (sources, mut problems) = package_sources_with_fixtures(package_dir, manifest_dir, fixtures);
    let sites = scan(&sources, rules);
    problems.extend(check(&sites, exceptions));
    problems
}

pub fn package_sources(package_dir: &str, manifest_dir: &Path) -> (BTreeMap<String, String>, Vec<String>) {
    package_sources_with_fixtures(package_dir, manifest_dir, &[])
}

/// Sanitized production source per repository-relative path, plus a problem
/// for every unreadable or malformed manifest field, missing root or module,
/// invalid fixture entry, or an empty scan.
///
/// The source is the module tree of every target root (declared, inferred
/// under `src/`, and the build script) except listed fixtures, followed by
/// [`module_tree_sources`], plus every file under `src/` by
/// [`production_sources`], so a file no root reaches there (an orphan, or one
/// only a macro declares) is still scanned. Nothing outside `src/` is walked.
pub fn package_sources_with_fixtures(
    package_dir: &str,
    manifest_dir: &Path,
    fixtures: &[FixtureBin],
) -> (BTreeMap<String, String>, Vec<String>) {
    let mut problems = Vec::new();
    let expected: PathBuf = package_dir.split('/').collect();
    if !manifest_dir.ends_with(&expected) {
        problems.push(format!("{} is not the package directory `{package_dir}`", manifest_dir.display()));
    }
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(error) => {
            problems.push(format!("required manifest {} is unreadable: {error}", manifest_path.display()));
            return (BTreeMap::new(), problems);
        }
    };
    let targets = match manifest_targets(&manifest, package_dir, manifest_dir) {
        Ok(targets) => targets,
        Err(problem) => {
            problems.push(problem);
            return (BTreeMap::new(), problems);
        }
    };
    problems.extend(targets.problems);
    problems.extend(fixture_problems(package_dir, &targets.roots, fixtures));

    let mut roots = Vec::new();
    let mut needs_src = true;
    for root in &targets.roots {
        if root.bin.as_deref().is_some_and(|name| fixtures.iter().any(|fixture| fixture.name == name)) {
            continue;
        }
        if !manifest_dir.join(&root.path).is_file() {
            problems.push(format!("declared source root {package_dir}/{} is missing", root.path));
            continue;
        }
        needs_src &= root.path.starts_with("src/");
        roots.push(root.path.clone());
    }
    // A target root Cargo infers needs no manifest entry: `src/lib.rs`,
    // `src/main.rs`, and `src/bin/*`. Scanned here even when the manifest
    // turns inference off, which only over-scans `src/`.
    roots.extend(["src/lib.rs", "src/main.rs"].into_iter().filter(|root| manifest_dir.join(root).is_file()).map(String::from));
    if let Ok(entries) = std::fs::read_dir(manifest_dir.join("src/bin")) {
        let mut inferred: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = entry.path();
                if path.is_file() && name.ends_with(".rs") {
                    Some(format!("src/bin/{name}"))
                } else {
                    path.join("main.rs").is_file().then(|| format!("src/bin/{name}/main.rs"))
                }
            })
            .collect();
        inferred.sort();
        roots.extend(inferred);
    }
    if let Some(script) = &targets.build_script {
        if manifest_dir.join(script).is_file() {
            roots.push(script.clone());
        } else {
            problems.push(format!("declared build script {package_dir}/{script} is missing"));
        }
    }

    let (mut sources, module_problems) = module_tree_sources(manifest_dir, package_dir, &roots);
    problems.extend(module_problems);
    sources = sources.into_iter().map(|(key, text)| (normalize_relative(&format!("{package_dir}/{key}")), text)).collect();
    let src = manifest_dir.join("src");
    if src.is_dir() {
        for (key, text) in production_sources(&src) {
            sources.entry(normalize_relative(&format!("{package_dir}/src/{key}"))).or_insert(text);
        }
    } else if needs_src {
        problems.push(format!("required source root {package_dir}/src is missing"));
    }
    if sources.is_empty() {
        problems.push(format!("scanned no production source under {package_dir}; an empty scan cannot pass"));
    }
    (sources, problems)
}

/// A problem per listed fixture that is unnamed in the manifest, outside
/// `tests/`, built by default, unexplained, or listed twice.
fn fixture_problems(package_dir: &str, roots: &[Root], fixtures: &[FixtureBin]) -> Vec<String> {
    let mut problems = Vec::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let label = format!("fixture bin `{}` of {package_dir}", fixture.name);
        if fixtures[..index].iter().any(|earlier| earlier.name == fixture.name) {
            problems.push(format!("{label} is listed twice"));
            continue;
        }
        if fixture.reason.trim().is_empty() {
            problems.push(format!("{label} has no reason"));
        }
        match roots.iter().find(|root| root.bin.as_deref() == Some(fixture.name)) {
            None => problems.push(format!("stale {label}: the manifest declares no such `[[bin]]`")),
            Some(root) if !root.path.starts_with("tests/") => {
                problems.push(format!("{label} is at {}, not under tests/, so it is production source", root.path));
            }
            Some(root) if !root.gated => problems.push(format!(
                "{label} declares no `required-features`, so a default build and `cargo install` compile it; it is production source"
            )),
            Some(_) => {}
        }
    }
    problems
}

/// One `[lib]` or `[[bin]]` source root.
struct Root {
    /// Package-relative and lexically normalized, as written or as Cargo
    /// infers it.
    path: String,
    /// The `[[bin]]` name; `None` for `[lib]`.
    bin: Option<String>,
    /// Whether the target declares a non-empty `required-features`.
    gated: bool,
}

/// The `[lib]` and `[[bin]]` roots and the build script a manifest selects,
/// read as Cargo reads them, plus a problem for each field the guard cannot
/// trust.
struct ManifestTargets {
    roots: Vec<Root>,
    /// Normalized like `roots`. Required to exist: an implicit `build.rs`
    /// appears only when present.
    build_script: Option<String>,
    problems: Vec<String>,
}

/// Reads the manifest with a TOML parser, so every spelling Cargo accepts
/// (literal strings, `[ lib ]`, inline and dotted tables, `bin = [{ .. }]`)
/// selects the same file. `Err` is a document that is not TOML at all
/// (duplicate key, trailing garbage, `null`).
///
/// A wrong-typed or empty `[lib].path`, `[[bin]].path`, or `[package].build`
/// is a problem rather than an absent field, as is a `[lib]` or `[[bin]]`
/// whose inferred source does not exist. Absent fields follow Cargo: `[lib]`
/// defaults to `src/lib.rs`; a `[[bin]]` to `src/main.rs` (when named after
/// the package), `src/bin/<name>.rs`, or `src/bin/<name>/main.rs`; the build
/// script to `build.rs` when it exists, `build = true` to `build.rs`, and
/// `build = false` to none. Auto-discovered binaries need no entry: `src/` is
/// always scanned whole. An array `build` (Cargo's unstable multiple build
/// scripts) is not read and fails as a wrong type. Declared paths are
/// normalized lexically (`./x.rs` and `y/../x.rs` are `x.rs`), so a file has
/// one site identity however the manifest spells it; an absolute path fails,
/// because sites are keyed by repository-relative path.
fn manifest_targets(text: &str, package_dir: &str, manifest_dir: &Path) -> Result<ManifestTargets, String> {
    let label = format!("{package_dir}/Cargo.toml");
    let manifest: toml::Table = toml::from_str(text).map_err(|error| {
        let line = error.span().map_or(String::new(), |span| format!(" at line {}", line_at(text, span.start)));
        format!("{label} is not valid TOML{line}: {}", error.message().trim_end())
    })?;
    let mut targets = ManifestTargets { roots: Vec::new(), build_script: None, problems: Vec::new() };
    let exists = |path: &str| manifest_dir.join(path).is_file();

    let package = match manifest.get("package") {
        Some(toml::Value::Table(package)) => Some(package),
        Some(other) => {
            targets.problems.push(format!("{label}: `package` must be a table, found {}", describe(other)));
            None
        }
        None => {
            targets.problems.push(format!("{label} has no `[package]` table"));
            None
        }
    };
    let package_name = package.and_then(|package| package.get("name")).and_then(toml::Value::as_str);

    match manifest.get("lib") {
        None => {}
        Some(toml::Value::Table(lib)) => match path_field(lib.get("path"), &label, "[lib].path") {
            Ok(Some(path)) => targets.roots.push(Root { path, bin: None, gated: false }),
            Ok(None) if exists("src/lib.rs") => {}
            Ok(None) => targets
                .problems
                .push(format!("{label}: `[lib]` has no `path` and its default {package_dir}/src/lib.rs is missing")),
            Err(problem) => targets.problems.push(problem),
        },
        Some(other) => targets.problems.push(format!("{label}: `lib` must be a table, found {}", describe(other))),
    }

    match manifest.get("bin") {
        None => {}
        Some(toml::Value::Array(entries)) => {
            for (index, entry) in entries.iter().enumerate() {
                let Some(bin) = entry.as_table() else {
                    targets.problems.push(format!(
                        "{label}: `[[bin]]` entry {} must be a table, found {}",
                        index + 1,
                        describe(entry)
                    ));
                    continue;
                };
                let name = match bin.get("name") {
                    None => None,
                    Some(toml::Value::String(name)) if !name.is_empty() => Some(name.as_str()),
                    Some(other) => {
                        targets.problems.push(format!(
                            "{label}: `[[bin]].name` of entry {} must be a non-empty string, found {}",
                            index + 1,
                            describe(other)
                        ));
                        continue;
                    }
                };
                let field = match name {
                    Some(name) => format!("[[bin]].path` of `{name}"),
                    None => format!("[[bin]].path` of entry `{}", index + 1),
                };
                let gated = bin.get("required-features").and_then(toml::Value::as_array).is_some_and(|features| !features.is_empty());
                let root = |path: String| Root { path, bin: name.map(String::from), gated };
                match (path_field(bin.get("path"), &label, &field), name) {
                    (Ok(Some(path)), _) => targets.roots.push(root(path)),
                    (Ok(None), Some(name)) => {
                        let mut candidates = Vec::new();
                        if package_name == Some(name) {
                            candidates.push("src/main.rs".to_string());
                        }
                        candidates.push(format!("src/bin/{name}.rs"));
                        candidates.push(format!("src/bin/{name}/main.rs"));
                        match candidates.iter().find(|candidate| exists(candidate)) {
                            Some(found) => targets.roots.push(root(found.clone())),
                            None => targets.problems.push(format!(
                                "{label}: `[[bin]]` `{name}` has no `path` and none of its inferred sources exists ({})",
                                candidates.join(", ")
                            )),
                        }
                    }
                    (Ok(None), None) => targets.problems.push(format!(
                        "{label}: `[[bin]]` entry {} has neither `name` nor `path`, so it has no source",
                        index + 1
                    )),
                    (Err(problem), _) => targets.problems.push(problem),
                }
            }
        }
        Some(other) => {
            targets.problems.push(format!("{label}: `bin` must be an array of tables, found {}", describe(other)));
        }
    }

    targets.build_script = match package.and_then(|package| package.get("build")) {
        None => exists("build.rs").then(|| "build.rs".to_string()),
        Some(toml::Value::Boolean(false)) => None,
        Some(toml::Value::Boolean(true)) => Some("build.rs".to_string()),
        Some(value @ toml::Value::String(_)) => match path_field(Some(value), &label, "[package].build") {
            Ok(path) => path,
            Err(problem) => {
                targets.problems.push(problem);
                None
            }
        },
        Some(other) => {
            targets.problems.push(format!(
                "{label}: `[package].build` must be `true`, `false`, or a path string, found {}",
                describe(other)
            ));
            None
        }
    };
    Ok(targets)
}

/// An optional path field, normalized: absent is `Ok(None)`; anything but a
/// non-empty, relative string is a problem naming the manifest and `field`.
fn path_field(value: Option<&toml::Value>, label: &str, field: &str) -> Result<Option<String>, String> {
    match value {
        None => Ok(None),
        Some(toml::Value::String(path)) if path.is_empty() => {
            Err(format!("{label}: `{field}` is an empty string; give a path or remove it"))
        }
        Some(toml::Value::String(path)) if path.starts_with(['/', '\\']) || path.as_bytes().get(1) == Some(&b':') => Err(
            format!("{label}: `{field}` must be relative to the package, found absolute `{path}`"),
        ),
        Some(toml::Value::String(path)) => Ok(Some(normalize_relative(path))),
        Some(other) => Err(format!("{label}: `{field}` must be a path string, found {}", describe(other))),
    }
}

fn describe(value: &toml::Value) -> String {
    format!("{} `{value}`", value.type_str())
}

/// Every site of `rules` in already-sanitized production sources.
pub fn scan(sources: &BTreeMap<String, String>, rules: &[Rule]) -> Vec<Site> {
    let mut sites = Vec::new();
    for (path, text) in sources {
        let items = item_spans(text.as_bytes());
        let uses = use_statements(text);
        let bindings: Vec<Binding> = uses.iter().flat_map(|&(start, end)| use_bindings(&text[start..end], scope_of(&items, start))).collect();
        let definitions = definitions(&items);
        for &rule in rules {
            let resolver = Resolver { items: &items, bindings: &bindings, definitions: &definitions, target: target(rule) };
            scan_target(path, text, &uses, &resolver, &mut sites);
        }
    }
    sites
}

fn scan_target(path: &str, text: &str, uses: &[(usize, usize)], resolver: &Resolver<'_>, sites: &mut Vec<Site>) {
    let bytes = text.as_bytes();
    let target = resolver.target;
    let in_use = |offset: usize| uses.iter().any(|&(start, end)| (start..end).contains(&offset));
    let mut push = |operation: String, offset: usize, resolved: bool| {
        sites.push(Site {
            rule: target.rule,
            path: path.to_string(),
            item: enclosing_item(resolver.items, offset),
            operation,
            line: line_at(text, offset),
            resolved,
        });
    };

    for name in resolver.candidate_names() {
        for offset in ident_offsets(text, &name) {
            if in_use(offset) || is_definition(bytes, offset) {
                continue;
            }
            let open = skip_ws(bytes, offset + name.len());
            let called = bytes.get(open) == Some(&b'(');
            if previous_non_ws(bytes, offset) == Some(b'.') {
                let no_arguments = called && bytes.get(skip_ws(bytes, open + 1)) == Some(&b')');
                if name == target.name && target.method && no_arguments {
                    push(format!(".{}()", target.name), offset, true);
                }
                continue;
            }
            let scope = scope_of(resolver.items, offset);
            if let Some(qualifier) = qualifier_path(bytes, offset) {
                let mut segments = qualifier.split("::").map(str::to_string).collect::<Vec<_>>();
                segments.push(name.clone());
                let operation = format!("{qualifier}::{name}");
                match resolver.resolve_path(&segments, false, scope, 0, None) {
                    Resolution::Local => {}
                    Resolution::Path(path) => match resolver.classify(&path) {
                        Provider::Helper => {}
                        Provider::Guarded => push(operation, offset, true),
                        // An alias name reached through a module this file
                        // cannot see into (`util::canon` re-exported by
                        // another file) is not followed; see the module docs.
                        Provider::Other if name != target.name => {}
                        Provider::Unknown | Provider::Other => push(operation, offset, false),
                    },
                    Resolution::Module(_) if name != target.name => {}
                    Resolution::Module(_) => push(operation, offset, false),
                }
                continue;
            }
            let relation = if name == target.name { "imported" } else { "alias of" };
            match resolver.lookup(&name, scope, 0, None) {
                Some(Resolution::Path(path)) => match resolver.classify(&path) {
                    // Bound to something that is not the target's name.
                    Provider::Helper | Provider::Other => {}
                    Provider::Guarded => push(format!("{name} ({relation} {})", source_label(&path)), offset, true),
                    Provider::Unknown => push(format!("{name} ({relation} {})", source_label(&path)), offset, false),
                },
                // A function this file defines in scope, or a module name.
                Some(Resolution::Local | Resolution::Module(_)) => {}
                // Without a binding in scope, a called bare target name may
                // still come from an unknown glob or the prelude of another
                // crate; anything else (a variable, an alias imported only in
                // another module) is not the guarded function.
                None if name == target.name && called => push(name.clone(), offset, false),
                None => {}
            }
        }
    }
}

/// The last two segments of a resolved import (`fs::canonicalize`).
fn source_label(path: &[String]) -> String {
    path[path.len().saturating_sub(2)..].join("::")
}

/// A lexical scope: `None` is the file's root module, `Some(index)` an inline
/// `mod` or a `fn` body in the file's item spans.
type Scope = Option<usize>;

/// A name a `use` statement binds in one scope.
struct Binding {
    /// `None` for a glob import.
    local: Option<String>,
    path: Vec<String>,
    /// Written with a leading `::`, so never resolved through local names.
    absolute: bool,
    scope: Scope,
}

/// A `fn` or inline `mod` the file defines, in the scope that can name it.
struct Definition {
    name: String,
    scope: Scope,
    /// `Some(span)` for an inline module.
    module: Option<usize>,
}

/// What a name or path refers to after following the file's imports.
enum Resolution {
    /// An item outside the file, or one the scan cannot place.
    Path(Vec<String>),
    /// A function this file defines.
    Local,
    /// An inline module of this file (or the file's root module).
    Module(Scope),
}

/// How a resolved path relates to one target.
enum Provider {
    /// The guarded function.
    Guarded,
    /// The approved helper.
    Helper,
    /// The target's name from a module the scan cannot place.
    Unknown,
    /// A path whose last segment is not the target's name.
    Other,
}

/// Bounds alias chains and glob recursion; a cyclic or deeper chain stays an
/// unattributed path, so it is reported rather than dropped.
const MAX_DEPTH: usize = 8;

/// Resolves names in one file for one target. Imports are scoped like Rust's:
/// a `fn` body sees its own `use` statements and those of the blocks and
/// module around it, and an inline `mod` sees only its own (plus what a glob
/// such as `use super::*` brings in).
struct Resolver<'a> {
    items: &'a [ItemSpan],
    bindings: &'a [Binding],
    definitions: &'a [Definition],
    target: &'static Target,
}

impl Resolver<'_> {
    /// The target's name plus every local name an import chain can bind to
    /// it (`use std::fs::canonicalize as canon; use canon as c;`).
    fn candidate_names(&self) -> Vec<String> {
        let mut names = vec![self.target.name.to_string()];
        loop {
            let before = names.len();
            for binding in self.bindings {
                if let (Some(local), Some(last)) = (&binding.local, binding.path.last())
                    && names.contains(last)
                    && !names.contains(local)
                {
                    names.push(local.clone());
                }
            }
            if names.len() == before {
                return names;
            }
        }
    }

    fn classify(&self, path: &[String]) -> Provider {
        let Some((last, qualifier)) = path.split_last() else { return Provider::Other };
        if last != self.target.name {
            return Provider::Other;
        }
        let module = qualifier.last().map_or("", String::as_str);
        if self.target.helpers.contains(&module) {
            Provider::Helper
        } else if self.target.modules.contains(&module) {
            Provider::Guarded
        } else {
            Provider::Unknown
        }
    }

    /// The innermost `Fn`/`Mod` span strictly containing `scope`'s span.
    fn parent(&self, scope: Scope) -> Scope {
        let index = scope?;
        let inner = &self.items[index];
        self.items
            .iter()
            .enumerate()
            .filter(|&(other, span)| {
                other != index && span.kind != SpanKind::Other && span.start <= inner.start && inner.end <= span.end
            })
            .max_by_key(|(_, span)| span.start)
            .map(|(other, _)| other)
    }

    /// The scopes whose imports are visible from `scope`, innermost first,
    /// ending at the enclosing module.
    fn chain(&self, scope: Scope) -> Vec<Scope> {
        let mut chain = vec![scope];
        let mut current = scope;
        while let Some(index) = current {
            if self.items[index].kind == SpanKind::Mod {
                break;
            }
            current = self.parent(current);
            chain.push(current);
        }
        chain
    }

    fn module_of(&self, scope: Scope) -> Scope {
        *self.chain(scope).last().expect("a chain holds its own scope")
    }

    /// Follows `segments` from `scope` through the file's imports, inline
    /// modules, and `self::`/`super::` prefixes. `exclude` is the binding whose
    /// own path is being resolved: a `use` never refers to itself.
    fn resolve_path(
        &self,
        segments: &[String],
        absolute: bool,
        scope: Scope,
        depth: usize,
        exclude: Option<usize>,
    ) -> Resolution {
        let Some((head, rest)) = segments.split_first() else { return Resolution::Path(Vec::new()) };
        if absolute || depth > MAX_DEPTH {
            return Resolution::Path(segments.to_vec());
        }
        let head = match head.as_str() {
            "self" => Some(Resolution::Module(self.module_of(scope))),
            "super" => self
                .module_of(scope)
                .map(|module| Resolution::Module(self.module_of(self.parent(Some(module))))),
            // `crate::` and an unknown head stay as written.
            "crate" => None,
            name => self.lookup(name, scope, depth + 1, exclude),
        };
        match head {
            None => Resolution::Path(segments.to_vec()),
            Some(Resolution::Path(mut path)) => {
                path.extend_from_slice(rest);
                Resolution::Path(path)
            }
            Some(resolution) if rest.is_empty() => resolution,
            Some(Resolution::Module(module)) => self.resolve_path(rest, false, module, depth + 1, None),
            Some(Resolution::Local) => Resolution::Path(segments.to_vec()),
        }
    }

    /// What `name` refers to in `scope`: an explicit import or definition in
    /// the innermost scope that has one, else a glob import that provides it.
    /// `None` when nothing in the file binds it.
    fn lookup(&self, name: &str, scope: Scope, depth: usize, exclude: Option<usize>) -> Option<Resolution> {
        if depth > MAX_DEPTH {
            return None;
        }
        let chain = self.chain(scope);
        for &visible in &chain {
            let binding = self.bindings.iter().enumerate().find(|&(index, binding)| {
                Some(index) != exclude && binding.scope == visible && binding.local.as_deref() == Some(name)
            });
            if let Some((index, binding)) = binding {
                return Some(self.resolve_path(&binding.path, binding.absolute, visible, depth + 1, Some(index)));
            }
            if let Some(definition) = self.definitions.iter().find(|definition| definition.scope == visible && definition.name == name) {
                return Some(definition.module.map_or(Resolution::Local, |module| Resolution::Module(Some(module))));
            }
        }
        for &visible in &chain {
            for (index, glob) in self.bindings.iter().enumerate() {
                if glob.local.is_some() || glob.scope != visible || Some(index) == exclude {
                    continue;
                }
                match self.resolve_path(&glob.path, glob.absolute, visible, depth + 1, Some(index)) {
                    Resolution::Module(module) => {
                        if let Some(found) = self.lookup(name, module, depth + 1, None) {
                            return Some(found);
                        }
                    }
                    // A provider module's glob brings in the target's name;
                    // what an unknown module's glob brings in is unknowable.
                    Resolution::Path(mut path)
                        if name == self.target.name
                            && path.last().is_some_and(|module| {
                                self.target.modules.contains(&module.as_str())
                                    || self.target.helpers.contains(&module.as_str())
                            }) =>
                    {
                        path.push(name.to_string());
                        return Some(Resolution::Path(path));
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// The innermost `fn` body or inline `mod` holding `offset`.
fn scope_of(items: &[ItemSpan], offset: usize) -> Scope {
    items
        .iter()
        .enumerate()
        .filter(|(_, span)| span.kind != SpanKind::Other && span.start <= offset && offset <= span.end)
        .max_by_key(|(_, span)| span.start)
        .map(|(index, _)| index)
}

/// Free functions and inline modules, each in the scope that can name it.
/// A `fn` inside an `impl` or `trait` is a method, which a bare name never
/// calls.
fn definitions(items: &[ItemSpan]) -> Vec<Definition> {
    let mut definitions = Vec::new();
    for (index, span) in items.iter().enumerate() {
        if span.kind == SpanKind::Other {
            continue;
        }
        let enclosing = items
            .iter()
            .enumerate()
            .filter(|&(other, outer)| other != index && outer.start <= span.start && span.end <= outer.end)
            .max_by_key(|(_, outer)| outer.start);
        if enclosing.is_some_and(|(_, outer)| outer.kind == SpanKind::Other) {
            continue;
        }
        definitions.push(Definition {
            name: span.name.clone(),
            scope: scope_of(items, span.start.saturating_sub(1)).filter(|&outer| outer != index),
            module: (span.kind == SpanKind::Mod).then_some(index),
        });
    }
    definitions
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UseToken<'a> {
    Ident(&'a str),
    Separator,
    Open,
    Close,
    Comma,
    Star,
}

/// The bindings one `use` statement (`use` through `;`) makes in `scope`,
/// groups, nested groups, `self`, renames, and globs included.
fn use_bindings(statement: &str, scope: Scope) -> Vec<Binding> {
    let bytes = statement.as_bytes();
    let mut tokens = Vec::new();
    let mut index = "use".len();
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() || byte == b';' {
            index += 1;
        } else if let Some(ident) = ident_at(bytes, index) {
            tokens.push(UseToken::Ident(ident));
            index += ident.len();
        } else if bytes[index..].starts_with(b"::") {
            tokens.push(UseToken::Separator);
            index += 2;
        } else {
            tokens.push(match byte {
                b'{' => UseToken::Open,
                b'}' => UseToken::Close,
                b',' => UseToken::Comma,
                b'*' => UseToken::Star,
                // Not a use tree this parser knows; bind nothing rather than
                // guess.
                _ => return Vec::new(),
            });
            index += 1;
        }
    }
    let mut bindings = Vec::new();
    let mut position = 0;
    use_tree(&tokens, &mut position, &[], false, scope, &mut bindings);
    bindings
}

fn use_tree(
    tokens: &[UseToken<'_>],
    position: &mut usize,
    prefix: &[String],
    mut absolute: bool,
    scope: Scope,
    bindings: &mut Vec<Binding>,
) {
    let mut path = prefix.to_vec();
    if path.is_empty() && tokens.get(*position) == Some(&UseToken::Separator) {
        absolute = true;
        *position += 1;
    }
    loop {
        match tokens.get(*position) {
            Some(&UseToken::Ident(ident)) if ident != "as" => {
                path.push(ident.to_string());
                *position += 1;
                if tokens.get(*position) == Some(&UseToken::Separator) {
                    *position += 1;
                } else {
                    break;
                }
            }
            Some(UseToken::Star) => {
                *position += 1;
                bindings.push(Binding { local: None, path, absolute, scope });
                return;
            }
            Some(UseToken::Open) => {
                *position += 1;
                while let Some(token) = tokens.get(*position) {
                    if *token == UseToken::Close {
                        *position += 1;
                        return;
                    }
                    let before = *position;
                    use_tree(tokens, position, &path, absolute, scope, bindings);
                    match tokens.get(*position) {
                        Some(UseToken::Comma) => *position += 1,
                        Some(UseToken::Close) => {}
                        _ if *position == before => return,
                        _ => {}
                    }
                }
                return;
            }
            _ => break,
        }
    }
    let alias = match tokens.get(*position..*position + 2) {
        Some([UseToken::Ident("as"), UseToken::Ident(alias)]) => {
            *position += 2;
            Some(*alias)
        }
        _ => None,
    };
    if path.last().is_some_and(|last| last == "self") {
        path.pop();
    }
    let local = alias.map(str::to_string).or_else(|| path.last().cloned());
    if path.is_empty() || local.as_deref().is_none_or(|local| local == "_") {
        return;
    }
    bindings.push(Binding { local, path, absolute, scope });
}

/// Compares the scanned sites with the exceptions.
pub fn check(sites: &[Site], exceptions: &[Exception]) -> Vec<String> {
    type Key<'a> = (Rule, &'a str, &'a str, &'a str);
    let mut problems = Vec::new();
    let mut allowed: BTreeMap<Key<'_>, &Exception> = BTreeMap::new();
    for entry in exceptions {
        if entry.reason.trim().is_empty() || entry.count == 0 {
            problems.push(format!(
                "exception for {} `{}` in {} ({}) needs a reason and a non-zero count",
                entry.rule, entry.operation, entry.item, entry.path
            ));
        }
        if allowed.insert((entry.rule, entry.path, entry.item, entry.operation), entry).is_some() {
            problems.push(format!(
                "duplicate exception: {} `{}` in {} ({})",
                entry.rule, entry.operation, entry.item, entry.path
            ));
        }
    }

    let mut found: BTreeMap<Key<'_>, (bool, Vec<usize>)> = BTreeMap::new();
    for site in sites {
        found
            .entry((site.rule, &site.path, &site.item, &site.operation))
            .or_insert_with(|| (site.resolved, Vec::new()))
            .1
            .push(site.line);
    }
    for (&(rule, path, item, operation), (resolved, lines)) in &found {
        let locations = lines.iter().map(|line| format!("{path}:{line}")).collect::<Vec<_>>().join(", ");
        let suggested_kind = if *resolved { "Kind::Invariant" } else { "Kind::Reviewed" };
        let suggestion = format!(
            "Exception {{ rule: Rule::{rule:?}, kind: {suggested_kind}, path: \"{path}\", item: \"{item}\", \
             operation: \"{operation}\", count: {}, reason: \"..\" }}",
            lines.len()
        );
        match allowed.get(&(rule, path, item, operation)) {
            None if *resolved => problems.push(format!(
                "new {rule} call `{operation}` in {item} at {locations}; use {}, or, when the result stays inside a \
                 private comparison, add {suggestion}",
                rule.helper()
            )),
            None => problems.push(format!(
                "unresolved {rule} candidate `{operation}` in {item} at {locations}: the scanner cannot tell what it \
                 calls; review it and add {suggestion} naming what it calls"
            )),
            Some(entry) => {
                if entry.count != lines.len() {
                    problems.push(format!(
                        "changed count for {rule} `{operation}` in {item}: excepted {}, found {} at {locations}",
                        entry.count,
                        lines.len()
                    ));
                }
                if (entry.kind == Kind::Reviewed) == *resolved {
                    problems.push(format!(
                        "exception for {rule} `{operation}` in {item} at {locations} is {:?}, but the site is {}",
                        entry.kind,
                        if *resolved { "a resolved call (use Invariant)" } else { "unresolved (use Reviewed)" }
                    ));
                }
            }
        }
    }
    for (key, entry) in &allowed {
        if !found.contains_key(key) {
            problems.push(format!(
                "stale exception: {} `{}` in {} ({}) no longer occurs",
                entry.rule, entry.operation, entry.item, entry.path
            ));
        }
    }
    problems
}

/// A named item with a brace body: `fn`, `impl`, `mod`, or `trait`.
struct ItemSpan {
    start: usize,
    end: usize,
    name: String,
    kind: SpanKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpanKind {
    Fn,
    Mod,
    /// An `impl` or `trait` body.
    Other,
}

fn item_spans(bytes: &[u8]) -> Vec<ItemSpan> {
    let text = std::str::from_utf8(bytes).expect("sanitized source is UTF-8");
    let mut spans = Vec::new();
    for (keyword, kind) in [("fn", SpanKind::Fn), ("mod", SpanKind::Mod), ("trait", SpanKind::Other)] {
        for offset in ident_offsets(text, keyword) {
            let name_at = skip_ws(bytes, offset + keyword.len());
            let Some(name) = ident_at(bytes, name_at) else { continue };
            if let Some(open) = body_open(bytes, name_at + name.len()) {
                spans.push(ItemSpan { start: offset, end: matching_brace(bytes, open), name: name.to_string(), kind });
            }
        }
    }
    for offset in ident_offsets(text, "impl") {
        let item_position = match previous_non_ws(bytes, offset) {
            None | Some(b';' | b'}' | b'{' | b']') => true,
            Some(_) => bytes[..offset].trim_ascii_end().ends_with(b"unsafe"),
        };
        if !item_position {
            continue;
        }
        if let Some(open) = body_open(bytes, offset + "impl".len()) {
            let name = impl_type_name(&text[offset + "impl".len()..open]);
            spans.push(ItemSpan { start: offset, end: matching_brace(bytes, open), name, kind: SpanKind::Other });
        }
    }
    spans.sort_by_key(|span| span.start);
    spans
}

/// The `{` opening the body of the item whose header continues at `from`, or
/// `None` for a body-less declaration (`fn f();`, `mod m;`).
fn body_open(bytes: &[u8], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(from) {
        match byte {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b'{' if depth == 0 => return Some(offset),
            b';' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// The self type's last path segment in an `impl` header
/// (`<T> Trait for a::Type<T> where ..` names `Type`).
fn impl_type_name(header: &str) -> String {
    let mut header = header.trim_start();
    if header.starts_with('<') {
        let mut depth = 0usize;
        for (offset, character) in header.char_indices() {
            match character {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        header = &header[offset + 1..];
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let for_at = ident_offsets(header, "for").into_iter().next();
    let self_type = for_at.map_or(header, |offset| &header[offset + 3..]);
    let self_type = ident_offsets(self_type, "where").first().map_or(self_type, |&offset| &self_type[..offset]);
    let path = self_type.split('<').next().unwrap_or_default();
    path.rsplit("::").next().unwrap_or_default().trim().trim_start_matches(['&', ' ']).trim_start_matches("dyn ").to_string()
}

/// The `::`-joined names of the items enclosing `offset`, outermost first.
fn enclosing_item(items: &[ItemSpan], offset: usize) -> String {
    let names = items
        .iter()
        .filter(|item| item.start <= offset && offset <= item.end)
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>();
    if names.is_empty() { "(module)".to_string() } else { names.join("::") }
}

/// `[start, end)` of every `use` statement, through its `;`.
fn use_statements(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    ident_offsets(text, "use")
        .into_iter()
        .filter(|&offset| bytes.get(skip_ws(bytes, offset + 3)) != Some(&b'<'))
        .filter_map(|offset| {
            let end = bytes[offset..].iter().position(|&byte| byte == b';')?;
            Some((offset, offset + end + 1))
        })
        .collect()
}

/// The `::`-joined path before the `::` that precedes `offset`
/// (`std::fs` for `std::fs::canonicalize`), or `<..>` for a qualified-self
/// path whose qualifier is not an identifier.
fn qualifier_path(bytes: &[u8], offset: usize) -> Option<String> {
    let mut segments = Vec::new();
    let mut cursor = offset;
    loop {
        let before = bytes[..cursor].trim_ascii_end();
        if !before.ends_with(b"::") {
            break;
        }
        let segment_end = before.len() - 2;
        let trimmed = bytes[..segment_end].trim_ascii_end();
        let mut start = trimmed.len();
        while start > 0 && is_ident(trimmed[start - 1]) {
            start -= 1;
        }
        if start == trimmed.len() {
            segments.push("<..>".to_string());
            break;
        }
        segments.push(String::from_utf8_lossy(&trimmed[start..]).into_owned());
        cursor = start;
    }
    (!segments.is_empty()).then(|| {
        segments.reverse();
        segments.join("::")
    })
}

fn is_definition(bytes: &[u8], offset: usize) -> bool {
    let trimmed = bytes[..offset].trim_ascii_end();
    trimmed.ends_with(b"fn") && trimmed.len().checked_sub(3).is_none_or(|at| !is_ident(trimmed[at]))
}

fn skip_ws(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(|byte| byte.is_ascii_whitespace()) {
        index += 1;
    }
    index
}

fn previous_non_ws(bytes: &[u8], offset: usize) -> Option<u8> {
    bytes[..offset].iter().rev().copied().find(|byte| !byte.is_ascii_whitespace())
}

/// The identifier starting at `index`, if one does.
fn ident_at(bytes: &[u8], index: usize) -> Option<&str> {
    let length = bytes.get(index..)?.iter().take_while(|&&byte| is_ident(byte)).count();
    (length > 0).then(|| std::str::from_utf8(&bytes[index..index + length]).expect("ASCII identifier"))
}
