//! Repo-declared test runner usage types and detection.
//!
//! This module is the "repo surface" counterpart to
//! [`crate::programs::test_runner`] (the host surface). Where the host surface
//! answers "is this runner installed on the host?", this module answers "which
//! runner does this package *declare*?" using manifest dependency keys,
//! config files, ecosystem defaults, and naming conventions.
//!
//! See `sniff/features/2026-06-14-more-repo/test-runner-strategy.md` §4 for
//! the detection algorithm and signal priority.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::toml_crate;
use serde::{Deserialize, Serialize};

use crate::filesystem::repo::detection::{ManifestStore, probe_exists, probe_is_dir};
use crate::performance;
use crate::performance::counters;
use crate::programs::contract::CategoryEnum;
use crate::programs::enums::TestRunner;
use crate::programs::test_runner_spec::{TEST_RUNNER_SPEC, TestRunnerEcosystem};

/// Evidence source for a detected test runner.
///
/// Variants are ordered by signal strength (strongest first):
/// [`TestRunnerSource::Config`] disambiguates between runners that share a
/// manifest dependency key; [`TestRunnerSource::Manifest`] is a strong
/// declaration; [`TestRunnerSource::EcosystemDefault`] marks an implicit
/// built-in; [`TestRunnerSource::Convention`] is the weakest (naming
/// convention only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TestRunnerSource {
    /// A config file owned by the runner was found in the package directory
    /// (or, for workspace-root-scoped runners, at the repo root).
    Config {
        /// The config filename or glob that matched (e.g. `vitest.config.ts`).
        filename: String,
        /// Repo-root-relative path to the config file that actually matched
        /// (e.g. `.config/nextest.toml`, `crates/app/vitest.config.ts`). For a
        /// wildcard glob this is the concrete file, not the pattern.
        path: String,
    },
    /// The runner appears as a dependency key in a package manifest.
    Manifest {
        /// The matching dependency key (e.g. `vitest`, `phpunit/phpunit`).
        key: String,
    },
    /// The runner is the implicit built-in for its ecosystem (e.g. `cargo
    /// test`, `go test`, `unittest`, `mix test`).
    EcosystemDefault,
    /// The runner is inferred from naming convention only (e.g. `tests/` +
    /// `*_test.rb`), with no dedicated config or manifest marker.
    Convention,
}

/// A test runner declared by a package, together with the evidence that
/// triggered the detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunnerUsage {
    /// The detected runner variant.
    pub runner: TestRunner,
    /// Why the runner was attributed to the package.
    pub source: TestRunnerSource,
}

impl TestRunnerUsage {
    /// Returns the runner's display name (e.g. `nextest`, `cargo test`).
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        crate::programs::schema::ProgramMetadata::info(&self.runner).display_name
    }
}

/// Detect the test runner(s) a package actually uses, rooted at `pkg_dir`.
///
/// Signals are evaluated in priority order (`test-runner-strategy.md` §4):
///
/// 1. **Config file present** in the package dir — strongest, disambiguates.
/// 2. **Manifest dependency key** — strong.
/// 3. **Ecosystem default** — the implicit built-in (`cargo test`, `go test`, …).
/// 4. **Convention** — weakest; naming only, for stdlib runners.
///
/// The return is collapsed to the **strongest tier present**: an explicitly
/// configured or declared runner supersedes the ecosystem default, so a package
/// that configures nextest reports `nextest` alone, not `nextest` *and* the
/// `cargo test` fallback. The ecosystem default is returned only when it is the
/// sole signal (a package with no explicit runner). This gives callers a single
/// answer wherever one exists; see [`prioritize`].
///
/// `repo_root` is the enclosing repository (or Cargo workspace) root. A few
/// runners keep a single config at that root that governs every member rather
/// than one config per package dir — see [`root_scoped_config`]. For those, the
/// config search extends to `repo_root` so a workspace-root marker is not
/// missed when scanning an individual member directory. The recorded
/// [`TestRunnerSource::Config::path`] is always repo-root-relative.
pub(crate) fn detect_test_runners(
    pkg_dir: &Path,
    repo_root: &Path,
    manifests: &ManifestStore,
) -> Vec<TestRunnerUsage> {
    let ecosystems = ecosystems_present(pkg_dir);
    if ecosystems.is_empty() {
        return Vec::new();
    }

    // Collect the package's declared dependencies per ecosystem so manifest
    // matching is exact: a runner is attributed only when its dependency key
    // appears as an actual dependency declaration, never as an incidental
    // substring of a package name, test script, or comment.
    let declared = collect_declared_deps(pkg_dir, &ecosystems, manifests);

    let mut found: Vec<TestRunnerUsage> = Vec::new();
    let mut seen: HashSet<TestRunner> = HashSet::new();

    // The static catalog is keyed by TestRunner variant ordinal, so the
    // enumerated iteration maps spec index -> runner variant exactly.
    let indexed: Vec<(
        usize,
        &'static crate::programs::test_runner_spec::TestRunnerSpec,
    )> = TEST_RUNNER_SPEC.iter().enumerate().collect();

    // Pass 1: config files (strongest signal).
    for (idx, spec) in &indexed {
        if !ecosystems.contains(&spec.ecosystem) {
            continue;
        }
        let runner = runner_at(*idx);
        // Most runners are configured per package dir; a few (e.g. nextest) keep
        // one config at the workspace root that governs every member, so their
        // search extends to `repo_root`.
        let search_dirs: &[&Path] = if root_scoped_config(runner) && repo_root != pkg_dir {
            &[pkg_dir, repo_root]
        } else {
            &[pkg_dir]
        };
        'globs: for glob in spec.config_globs {
            for dir in search_dirs {
                let located = if *dir == repo_root && root_scoped_config(runner) {
                    locate_root_config(dir, runner, glob, manifests)
                } else {
                    locate_config(dir, runner, glob, manifests)
                };
                if let Some(located) = located {
                    push_unique(
                        &mut found,
                        &mut seen,
                        runner,
                        TestRunnerSource::Config {
                            filename: (*glob).to_string(),
                            path: repo_relative(&located, repo_root),
                        },
                    );
                    break 'globs;
                }
            }
        }
    }

    // Pass 2: manifest dependency keys.
    for (idx, spec) in &indexed {
        if !ecosystems.contains(&spec.ecosystem) {
            continue;
        }
        let runner = runner_at(*idx);
        if seen.contains(&runner) {
            continue;
        }
        for key in spec.manifest_dep_keys {
            if key_declared(&declared, key) {
                push_unique(
                    &mut found,
                    &mut seen,
                    runner,
                    TestRunnerSource::Manifest {
                        key: (*key).to_string(),
                    },
                );
                break;
            }
        }
    }

    // Pass 3: ecosystem defaults.
    for (idx, spec) in &indexed {
        if !ecosystems.contains(&spec.ecosystem) || !spec.is_ecosystem_default {
            continue;
        }
        push_unique(
            &mut found,
            &mut seen,
            runner_at(*idx),
            TestRunnerSource::EcosystemDefault,
        );
    }

    // Pass 4: convention (weakest) — only for stdlib runners with no dedicated
    // marker. We emit these only when the package has tests but no explicit
    // runner was found.
    if !ecosystems.is_empty() && has_convention_tests(pkg_dir) {
        for (idx, spec) in &indexed {
            if !ecosystems.contains(&spec.ecosystem) {
                continue;
            }
            let runner = runner_at(*idx);
            if matches!(
                runner,
                TestRunner::Unittest | TestRunner::Minitest | TestRunner::NodeTest
            ) && !seen.contains(&runner)
            {
                push_unique(&mut found, &mut seen, runner, TestRunnerSource::Convention);
            }
        }
    }

    found.sort_by_key(|u| u.runner.variant_index());
    prioritize(found)
}

/// Signal strength of a source; lower is stronger.
fn source_rank(source: &TestRunnerSource) -> u8 {
    match source {
        TestRunnerSource::Config { .. } => 0,
        TestRunnerSource::Manifest { .. } => 1,
        TestRunnerSource::EcosystemDefault => 2,
        TestRunnerSource::Convention => 3,
    }
}

/// Keep only the runners attributed by the strongest signal tier present.
///
/// An explicitly configured (`Config`) or declared (`Manifest`) runner
/// supersedes the ecosystem default and convention fallbacks, collapsing a
/// package to the single runner it actually uses wherever the evidence allows.
/// When the only evidence is the ecosystem default (no explicit runner), that
/// default is what survives.
fn prioritize(mut found: Vec<TestRunnerUsage>) -> Vec<TestRunnerUsage> {
    if let Some(best) = found.iter().map(|u| source_rank(&u.source)).min() {
        found.retain(|u| source_rank(&u.source) == best);
    }
    found
}

/// Detect declared test runners for a single project directory, building a
/// fresh manifest store.
///
/// This is the entry point for non-monorepo contexts where no `Package` is
/// produced by repo detection (e.g. a standalone Cargo crate). For monorepo
/// packages, prefer reading [`Package::test_runners`] which is populated
/// during repo detection.
#[must_use]
pub fn detect_test_runners_for_dir(dir: &Path) -> Vec<TestRunnerUsage> {
    let manifests = ManifestStore::default();
    detect_test_runners(dir, dir, &manifests)
}

/// Returns `true` when `runner`'s config file lives once at the workspace/repo
/// root and governs every member, rather than sitting in each package dir.
///
/// nextest reads a single `.config/nextest.toml` at the Cargo workspace root; it
/// is never duplicated per crate. A package-dir-only scan therefore misses it
/// and a workspace would report only the `cargo test` ecosystem default, so the
/// config search for these runners extends to the repo root.
fn root_scoped_config(runner: TestRunner) -> bool {
    matches!(runner, TestRunner::Nextest)
}

/// Returns the absolute path of the config file matching `glob` under `pkg_dir`,
/// or `None` when no qualifying file is present.
///
/// Wildcard patterns (`*.suite.yml`, `features/*.feature`, `spec/**/*_spec.rb`)
/// are resolved by [`locate_glob_file`]; literal patterns use a direct
/// `exists()` check.
///
/// `tox.ini` and `setup.cfg` are each claimed by several Python runners
/// (`tox.ini` by both pytest and tox; `setup.cfg` by both pytest and nose2).
/// Bare file presence would mis-attribute a `tox.ini` containing only `[tox]`
/// to pytest, or a nose2 `setup.cfg` to pytest. For those shared files we
/// require the section header named in `test-runner-strategy.md` §5; every
/// other config glob is runner-exclusive and matches on existence alone.
/// Wildcard patterns are always runner-exclusive, so they never need a section
/// check.
fn locate_config(
    pkg_dir: &Path,
    runner: TestRunner,
    glob: &str,
    manifests: &ManifestStore,
) -> Option<PathBuf> {
    if is_glob_pattern(glob) {
        return locate_glob_file(pkg_dir, glob);
    }
    let path = pkg_dir.join(glob);
    if !probe_exists(&path) {
        return None;
    }
    match required_config_section(runner, glob) {
        Some(section) => manifests
            .raw_text(&path)
            .is_some_and(|text| ini_has_section(&text, section))
            .then_some(path),
        None => Some(path),
    }
}

/// Locate a config whose workspace-root path is shared by every package.
fn locate_root_config(
    repo_root: &Path,
    runner: TestRunner,
    glob: &str,
    manifests: &ManifestStore,
) -> Option<PathBuf> {
    if is_glob_pattern(glob) {
        return locate_config(repo_root, runner, glob, manifests);
    }
    let path = repo_root.join(glob);
    if !manifests.root_config_exists(&path) {
        return None;
    }
    match required_config_section(runner, glob) {
        Some(section) => manifests
            .raw_text(&path)
            .is_some_and(|text| ini_has_section(&text, section))
            .then_some(path),
        None => Some(path),
    }
}

/// Render `path` relative to `repo_root` with `/` separators. Falls back to the
/// original path when it is not under `repo_root`.
fn repo_relative(path: &Path, repo_root: &Path) -> String {
    relative_slashes(path.strip_prefix(repo_root).unwrap_or(path))
}

/// Maximum directory depth descended when resolving a recursive (`**`) config
/// glob, so a pathological tree cannot turn config detection into a deep walk.
const GLOB_MAX_DEPTH: usize = 16;

/// Returns `true` when `glob` contains a glob metacharacter.
fn is_glob_pattern(glob: &str) -> bool {
    glob.bytes().any(|b| matches!(b, b'*' | b'?' | b'['))
}

/// Returns the absolute path of the first file under `pkg_dir` matching the glob
/// `pattern`, or `None`.
///
/// The walk is rooted at the pattern's literal directory prefix and bounded in
/// depth (non-recursive patterns descend only as far as their segment count
/// requires; `**` patterns are capped at [`GLOB_MAX_DEPTH`]), so a glob never
/// triggers an unbounded scan of the package tree.
fn locate_glob_file(pkg_dir: &Path, pattern: &str) -> Option<PathBuf> {
    let matcher = globset::GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .ok()?
        .compile_matcher();

    let segments: Vec<&str> = pattern.split('/').collect();
    let mut root = pkg_dir.to_path_buf();
    let mut literal_segments = 0;
    for segment in &segments {
        if is_glob_pattern(segment) {
            break;
        }
        root.push(segment);
        literal_segments += 1;
    }
    if !probe_exists(&root) {
        return None;
    }

    let descent = if pattern.contains("**") {
        GLOB_MAX_DEPTH
    } else {
        // The final wildcard segment is the file name, so directory descent
        // below the literal prefix is one less than the remaining segments.
        segments.len().saturating_sub(literal_segments + 1)
    };
    glob_walk(&root, pkg_dir, &matcher, descent)
}

/// Depth-bounded search for a file whose `base`-relative path matches
/// `matcher`, returning its absolute path. Descends at most `max_descent`
/// directory levels below `dir`.
fn glob_walk(
    dir: &Path,
    base: &Path,
    matcher: &globset::GlobMatcher,
    max_descent: usize,
) -> Option<PathBuf> {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_file() {
            if let Ok(relative) = path.strip_prefix(base)
                && matcher.is_match(relative_slashes(relative))
            {
                return Some(path);
            }
        } else if file_type.is_dir()
            && max_descent > 0
            && let Some(found) = glob_walk(&path, base, matcher, max_descent - 1)
        {
            return Some(found);
        }
    }
    None
}

/// Render a relative path with `/` separators so glob matching behaves the same
/// on Windows as on Unix.
fn relative_slashes(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The INI section that must be present for `runner` to claim a config file
/// shared with other runners, or `None` when `glob` is runner-exclusive.
fn required_config_section(runner: TestRunner, glob: &str) -> Option<&'static str> {
    match (runner, glob) {
        (TestRunner::Pytest, "tox.ini") => Some("[pytest]"),
        (TestRunner::Pytest, "setup.cfg") => Some("[tool:pytest]"),
        (TestRunner::Tox, "tox.ini") => Some("[tox]"),
        (TestRunner::Nose2, "setup.cfg") => Some("[unittest]"),
        _ => None,
    }
}

/// Returns `true` when `text` contains the INI section header `section`
/// (e.g. `[pytest]`) on its own line, ignoring surrounding whitespace.
fn ini_has_section(text: &str, section: &str) -> bool {
    text.lines().any(|line| line.trim() == section)
}

/// Append a usage to `found` iff its runner has not been recorded yet.
fn push_unique(
    found: &mut Vec<TestRunnerUsage>,
    seen: &mut HashSet<TestRunner>,
    runner: TestRunner,
    source: TestRunnerSource,
) {
    if seen.insert(runner) {
        found.push(TestRunnerUsage { runner, source });
    }
}

/// Returns the ecosystems that have a manifest present in `pkg_dir`.
fn ecosystems_present(pkg_dir: &Path) -> HashSet<TestRunnerEcosystem> {
    let mut set = HashSet::new();
    if probe_exists(&pkg_dir.join("Cargo.toml")) {
        set.insert(TestRunnerEcosystem::Rust);
    }
    if probe_exists(&pkg_dir.join("go.mod")) {
        set.insert(TestRunnerEcosystem::Go);
    }
    if probe_exists(&pkg_dir.join("package.json")) {
        set.insert(TestRunnerEcosystem::Node);
    }
    if probe_exists(&pkg_dir.join("pyproject.toml"))
        || probe_exists(&pkg_dir.join("requirements.txt"))
        || probe_exists(&pkg_dir.join("setup.py"))
    {
        set.insert(TestRunnerEcosystem::Python);
    }
    if probe_exists(&pkg_dir.join("composer.json")) {
        set.insert(TestRunnerEcosystem::Php);
    }
    if probe_exists(&pkg_dir.join("Gemfile")) || has_gemspec(pkg_dir) {
        set.insert(TestRunnerEcosystem::Ruby);
    }
    if probe_exists(&pkg_dir.join("pom.xml"))
        || probe_exists(&pkg_dir.join("build.gradle"))
        || probe_exists(&pkg_dir.join("build.gradle.kts"))
    {
        set.insert(TestRunnerEcosystem::Jvm);
    }
    if has_csproj(pkg_dir) {
        set.insert(TestRunnerEcosystem::DotNet);
    }
    if probe_exists(&pkg_dir.join("mix.exs")) {
        set.insert(TestRunnerEcosystem::Elixir);
    }
    set
}

/// A package's declared dependencies, partitioned by match style.
#[derive(Default)]
struct DeclaredDeps {
    /// Plain dependency names: npm packages, PyPI distributions, Ruby gems,
    /// Composer packages, Go module paths, .NET package ids, Elixir dep atoms.
    names: HashSet<String>,
    /// Maven/Gradle `groupId:artifactId` coordinates.
    coordinates: HashSet<String>,
}

/// Returns `true` when `key` is declared as a dependency of the package.
///
/// Keys carrying a `:` are Maven/Gradle coordinates and match the coordinate
/// set; all other keys match the plain-name set. Either way the test is exact
/// set membership — what separates a real `devDependencies.jest` from a package
/// merely *named* `jest-helper`.
fn key_declared(deps: &DeclaredDeps, key: &str) -> bool {
    if key.contains(':') {
        deps.coordinates.contains(key)
    } else {
        deps.names.contains(key)
    }
}

/// Collect the package's declared dependencies from every manifest of every
/// ecosystem present. Structured manifests (JSON/TOML) are parsed; the
/// line/markup formats (go.mod, Gemfile, pom.xml, Gradle DSL, .csproj, mix.exs)
/// are scanned for their dependency-declaration syntax. The shared cache reads
/// each file at most once.
fn collect_declared_deps(
    pkg_dir: &Path,
    ecosystems: &HashSet<TestRunnerEcosystem>,
    manifests: &ManifestStore,
) -> DeclaredDeps {
    let mut deps = DeclaredDeps::default();
    if ecosystems.contains(&TestRunnerEcosystem::Node) {
        collect_node_deps(pkg_dir, manifests, &mut deps.names);
    }
    if ecosystems.contains(&TestRunnerEcosystem::Php) {
        collect_php_deps(pkg_dir, manifests, &mut deps.names);
    }
    if ecosystems.contains(&TestRunnerEcosystem::Python) {
        collect_python_deps(pkg_dir, manifests, &mut deps.names);
    }
    if ecosystems.contains(&TestRunnerEcosystem::Go) {
        collect_go_deps(pkg_dir, manifests, &mut deps.names);
    }
    if ecosystems.contains(&TestRunnerEcosystem::Ruby) {
        collect_ruby_deps(pkg_dir, manifests, &mut deps.names);
    }
    if ecosystems.contains(&TestRunnerEcosystem::Jvm) {
        collect_jvm_coordinates(pkg_dir, manifests, &mut deps.coordinates);
    }
    if ecosystems.contains(&TestRunnerEcosystem::DotNet) {
        collect_dotnet_deps(pkg_dir, manifests, &mut deps.names);
    }
    if ecosystems.contains(&TestRunnerEcosystem::Elixir) {
        collect_elixir_deps(pkg_dir, manifests, &mut deps.names);
    }
    deps
}

/// Insert every npm dependency key (across the four standard sections) into
/// `out`.
fn collect_node_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    let Some(val) = manifests.npm(&pkg_dir.join("package.json")) else {
        return;
    };
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(obj) = val.get(section).and_then(serde_json::Value::as_object) {
            out.extend(obj.keys().cloned());
        }
    }
}

/// Insert every Composer `require` / `require-dev` package name into `out`.
fn collect_php_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    let parsed = manifests
        .raw_text(&pkg_dir.join("composer.json"))
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let Some(val) = parsed else {
        return;
    };
    for section in ["require", "require-dev"] {
        if let Some(obj) = val.get(section).and_then(serde_json::Value::as_object) {
            out.extend(obj.keys().cloned());
        }
    }
}

/// Insert every declared PyPI distribution name (lowercased) from
/// `pyproject.toml` (PEP 621, PEP 735, and Poetry layouts) and any top-level
/// `requirements*.txt` into `out`.
fn collect_python_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    if let Some(val) = manifests.pyproject(&pkg_dir.join("pyproject.toml")) {
        // PEP 621 [project].dependencies + [project.optional-dependencies]
        if let Some(project) = val.get("project") {
            if let Some(arr) = project
                .get("dependencies")
                .and_then(toml_crate::Value::as_array)
            {
                insert_python_specs(arr, out);
            }
            if let Some(tbl) = project
                .get("optional-dependencies")
                .and_then(toml_crate::Value::as_table)
            {
                for group in tbl.values() {
                    if let Some(arr) = group.as_array() {
                        insert_python_specs(arr, out);
                    }
                }
            }
        }
        // PEP 735 [dependency-groups]
        if let Some(tbl) = val
            .get("dependency-groups")
            .and_then(toml_crate::Value::as_table)
        {
            for group in tbl.values() {
                if let Some(arr) = group.as_array() {
                    insert_python_specs(arr, out);
                }
            }
        }
        // Poetry [tool.poetry.dependencies | dev-dependencies | group.*.dependencies]
        if let Some(poetry) = val.get("tool").and_then(|tool| tool.get("poetry")) {
            for key in ["dependencies", "dev-dependencies"] {
                if let Some(tbl) = poetry.get(key).and_then(toml_crate::Value::as_table) {
                    insert_poetry_names(tbl, out);
                }
            }
            if let Some(groups) = poetry.get("group").and_then(toml_crate::Value::as_table) {
                for group in groups.values() {
                    if let Some(tbl) = group
                        .get("dependencies")
                        .and_then(toml_crate::Value::as_table)
                    {
                        insert_poetry_names(tbl, out);
                    }
                }
            }
        }
    }
    for path in files_matching(pkg_dir, |name| {
        name.starts_with("requirements") && name.ends_with(".txt")
    }) {
        if let Some(text) = manifests.raw_text(&path) {
            for line in text.lines() {
                if let Some(name) = python_requirement_name(line) {
                    out.insert(name);
                }
            }
        }
    }
}

/// Insert the distribution name of every PEP 508 spec string in `arr`.
fn insert_python_specs(arr: &[toml_crate::Value], out: &mut HashSet<String>) {
    for item in arr {
        if let Some(name) = item.as_str().and_then(python_dist_name) {
            out.insert(name);
        }
    }
}

/// Insert every Poetry dependency-table key (excluding the `python` constraint)
/// into `out`, lowercased.
fn insert_poetry_names(table: &toml_crate::value::Table, out: &mut HashSet<String>) {
    out.extend(
        table
            .keys()
            .filter(|key| *key != "python")
            .map(|key| key.to_ascii_lowercase()),
    );
}

/// Insert every Go module path declared in a `require` directive into `out`.
fn collect_go_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    let Some(text) = manifests.go_mod(&pkg_dir.join("go.mod")) else {
        return;
    };
    let mut in_block = false;
    for raw in text.lines() {
        let line = raw.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if in_block {
            if line == ")" {
                in_block = false;
            } else if let Some(module) = line.split_whitespace().next() {
                out.insert(module.to_string());
            }
        } else if line == "require (" || line == "require(" {
            in_block = true;
        } else if let Some(rest) = line.strip_prefix("require ")
            && let Some(module) = rest.split_whitespace().next()
        {
            out.insert(module.to_string());
        }
    }
}

/// Insert every gem named by a `gem "..."` line (Gemfile) or an
/// `add_*dependency "..."` call (gemspec) into `out`.
fn collect_ruby_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    let mut paths = vec![pkg_dir.join("Gemfile")];
    paths.extend(files_matching(pkg_dir, |name| name.ends_with(".gemspec")));
    for path in paths {
        let Some(text) = manifests.raw_text(&path) else {
            continue;
        };
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let is_dependency = line.starts_with("gem ")
                || line.contains("add_dependency")
                || line.contains("add_development_dependency")
                || line.contains("add_runtime_dependency");
            if is_dependency && let Some(name) = first_quoted(line) {
                out.insert(name);
            }
        }
    }
}

/// Insert every Maven/Gradle `groupId:artifactId` coordinate into `out`.
fn collect_jvm_coordinates(
    pkg_dir: &Path,
    manifests: &ManifestStore,
    out: &mut HashSet<String>,
) {
    if let Some(text) = manifests.raw_text(&pkg_dir.join("pom.xml")) {
        for block in text.split("<dependency>").skip(1) {
            let block = &block[..block.find("</dependency>").unwrap_or(block.len())];
            if let (Some(group), Some(artifact)) = (
                xml_tag_text(block, "groupId"),
                xml_tag_text(block, "artifactId"),
            ) {
                out.insert(format!("{group}:{artifact}"));
            }
        }
    }
    for name in ["build.gradle", "build.gradle.kts"] {
        let Some(text) = manifests.raw_text(&pkg_dir.join(name)) else {
            continue;
        };
        for literal in quoted_strings(&text) {
            let mut parts = literal.splitn(3, ':');
            if let (Some(group), Some(artifact)) = (parts.next(), parts.next())
                && is_coordinate_token(group)
                && is_coordinate_token(artifact)
            {
                out.insert(format!("{group}:{artifact}"));
            }
        }
    }
}

/// Insert every `<PackageReference Include="...">` package id from each
/// `.csproj` in the package dir into `out`.
fn collect_dotnet_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    for path in files_matching(pkg_dir, |name| name.ends_with(".csproj")) {
        let Some(text) = manifests.raw_text(&path) else {
            continue;
        };
        for element in text.split("PackageReference").skip(1) {
            if let Some(after) = element
                .find("Include=")
                .map(|i| &element[i + "Include=".len()..])
                && let Some(name) = first_quoted(after)
            {
                out.insert(name);
            }
        }
    }
}

/// Insert every Mix dependency atom (`{:dep, ...}`) into `out`.
fn collect_elixir_deps(pkg_dir: &Path, manifests: &ManifestStore, out: &mut HashSet<String>) {
    let Some(text) = manifests.raw_text(&pkg_dir.join("mix.exs")) else {
        return;
    };
    let mut rest = text.as_str();
    while let Some(pos) = rest.find("{:") {
        let after = &rest[pos + 2..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        if end > 0 {
            out.insert(after[..end].to_string());
        }
        rest = &after[end..];
    }
}

/// Return the package-dir entries whose file name satisfies `pred`. The scan is
/// shallow (no recursion), so it is cheap and bounded.
fn files_matching(pkg_dir: &Path, pred: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    let Ok(entries) = std::fs::read_dir(pkg_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            pred(name.to_str()?).then(|| entry.path())
        })
        .collect()
}

/// Extract the leading distribution name from a PEP 508 requirement string,
/// lowercased; `None` when the string does not start with a name character.
fn python_dist_name(spec: &str) -> Option<String> {
    let spec = spec.trim();
    let end = spec
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .unwrap_or(spec.len());
    (end > 0).then(|| spec[..end].to_ascii_lowercase())
}

/// Extract the distribution name from one `requirements.txt` line, skipping
/// comments, blank lines, and `-`-prefixed options (`-r`, `-e`, `--hash`, …).
fn python_requirement_name(line: &str) -> Option<String> {
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() || line.starts_with('-') {
        return None;
    }
    python_dist_name(line)
}

/// Return the contents of the first single- or double-quoted string in `s`.
fn first_quoted(s: &str) -> Option<String> {
    for (i, b) in s.bytes().enumerate() {
        if b == b'"' || b == b'\'' {
            let rest = &s[i + 1..];
            return rest.find(b as char).map(|end| rest[..end].to_string());
        }
    }
    None
}

/// Collect the contents of every single- or double-quoted string in `text`.
fn quoted_strings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if (b == b'"' || b == b'\'')
            && let Some(rel) = text[i + 1..].find(b as char)
        {
            out.push(text[i + 1..i + 1 + rel].to_string());
            i += rel + 2;
            continue;
        }
        i += 1;
    }
    out
}

/// Return the trimmed text content of the first `<tag>…</tag>` in `chunk`.
fn xml_tag_text(chunk: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = chunk.find(&open)? + open.len();
    let end = chunk[start..].find(&close)? + start;
    Some(chunk[start..end].trim().to_string())
}

/// Returns `true` when `token` looks like a Maven coordinate component
/// (non-empty; only alphanumerics and `.`/`-`/`_`).
fn is_coordinate_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Returns `true` when the package dir contains a `.gemspec` file.
fn has_gemspec(pkg_dir: &Path) -> bool {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    if let Ok(entries) = std::fs::read_dir(pkg_dir) {
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.ends_with(".gemspec"))
            {
                return true;
            }
        }
    }
    false
}

/// Returns `true` when the package dir contains a `.csproj` file.
fn has_csproj(pkg_dir: &Path) -> bool {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    if let Ok(entries) = std::fs::read_dir(pkg_dir) {
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.ends_with(".csproj"))
            {
                return true;
            }
        }
    }
    false
}

/// Returns `true` when the package dir has convention-style test files
/// (`tests/`, `test/`, `spec/`, or `*_test.*` / `*_spec.*` naming).
fn has_convention_tests(pkg_dir: &Path) -> bool {
    for dir in ["tests", "test", "spec"] {
        if probe_is_dir(&pkg_dir.join(dir)) {
            return true;
        }
    }
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    if let Ok(entries) = std::fs::read_dir(pkg_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && (name.contains("_test.")
                    || name.contains("_spec.")
                    || name.contains(".test.")
                    || name.contains(".spec."))
            {
                return true;
            }
        }
    }
    false
}

/// Resolve a catalog index back to its `TestRunner` variant.
///
/// `TEST_RUNNER_SPEC` is keyed by variant ordinal (enforced by
/// `test_runner_count_matches_info` and the `variant_index` invariant in
/// `programs/enums`), so the nth spec corresponds to the nth variant.
fn runner_at(idx: usize) -> TestRunner {
    TestRunner::iter()
        .nth(idx)
        .expect("TEST_RUNNER_SPEC ordinal maps to a valid TestRunner variant")
}

use strum::IntoEnumIterator;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(dir: &Path, name: &str, contents: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn detect(dir: &Path) -> Vec<TestRunnerUsage> {
        let manifests = ManifestStore::default();
        detect_test_runners(dir, dir, &manifests)
    }

    fn detect_in_workspace(pkg_dir: &Path, root: &Path) -> Vec<TestRunnerUsage> {
        let manifests = ManifestStore::default();
        detect_test_runners(pkg_dir, root, &manifests)
    }

    fn runners_of(usage: &[TestRunnerUsage]) -> Vec<TestRunner> {
        usage.iter().map(|u| u.runner).collect()
    }

    // ------------------------------------------------------------------------
    // Rust
    // ------------------------------------------------------------------------

    #[test]
    fn rust_cargo_reports_cargo_test_default() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1\"\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::CargoTest
                && u.source == TestRunnerSource::EcosystemDefault),
            "expected CargoTest as ecosystem default, got {usage:?}"
        );
    }

    #[test]
    fn rust_nextest_config_wins_over_default() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1\"\n",
        );
        write(dir.path(), ".config/nextest.toml", "[profile.default]\n");

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::Nextest
            && matches!(u.source, TestRunnerSource::Config { .. })));
        // The configured runner supersedes the cargo test ecosystem default.
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::CargoTest),
            "cargo test default must be superseded by configured nextest, got {usage:?}"
        );
    }

    #[test]
    fn rust_nextest_config_at_workspace_root_is_detected_for_member_crate() {
        // The Cargo workspace keeps a single `.config/nextest.toml` at the root;
        // member crates do not duplicate it. Scanning a member dir must still
        // surface nextest by extending the config search to the workspace root.
        let root = tempdir().unwrap();
        write(root.path(), ".config/nextest.toml", "[profile.default]\n");

        let member = root.path().join("crates/lib");
        fs::create_dir_all(&member).unwrap();
        write(&member, "Cargo.toml", "[package]\nname = \"x\"\nversion = \"0.1\"\n");

        let usage = detect_in_workspace(&member, root.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Nextest
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "nextest config at the workspace root should attribute to the member crate, got {usage:?}"
        );
        // The configured runner supersedes the cargo test ecosystem default,
        // and its recorded path is repo-root-relative.
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::CargoTest),
            "cargo test default must be superseded by workspace-root nextest, got {usage:?}"
        );
        let nextest = usage
            .iter()
            .find(|u| u.runner == TestRunner::Nextest)
            .expect("nextest detected");
        assert!(
            matches!(&nextest.source, TestRunnerSource::Config { path, .. } if path == ".config/nextest.toml"),
            "nextest config path should be repo-root-relative, got {:?}",
            nextest.source
        );
    }

    #[test]
    fn rust_no_nextest_config_anywhere_reports_only_cargo_test() {
        // Without a nextest config in the member dir or the workspace root, the
        // member must report only the cargo test ecosystem default.
        let root = tempdir().unwrap();
        let member = root.path().join("crates/lib");
        fs::create_dir_all(&member).unwrap();
        write(&member, "Cargo.toml", "[package]\nname = \"x\"\nversion = \"0.1\"\n");

        let usage = detect_in_workspace(&member, root.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Nextest),
            "nextest must not be reported without a config marker, got {usage:?}"
        );
        assert!(usage.iter().any(|u| u.runner == TestRunner::CargoTest));
    }

    // ------------------------------------------------------------------------
    // Go
    // ------------------------------------------------------------------------

    #[test]
    fn go_reports_go_test_default() {
        let dir = tempdir().unwrap();
        write(dir.path(), "go.mod", "module example.com/x\ngo 1.21\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::GoTest
                && u.source == TestRunnerSource::EcosystemDefault)
        );
    }

    #[test]
    fn go_gotestsum_via_manifest_key() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "go.mod",
            "module example.com/x\nrequire gotest.tools/gotestsum v1.10.1\n",
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::Gotestsum
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
    }

    // ------------------------------------------------------------------------
    // JS/TS
    // ------------------------------------------------------------------------

    #[test]
    fn node_vitest_config_and_dep() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"name":"x","devDependencies":{"vitest":"^1.0.0"}}"#,
        );
        write(dir.path(), "vitest.config.ts", "export default {};\n");

        let usage = detect(dir.path());
        // Config signal should be present (strongest).
        assert!(usage.iter().any(|u| u.runner == TestRunner::Vitest
            && matches!(u.source, TestRunnerSource::Config { .. })));
    }

    #[test]
    fn node_jest_dep_key_without_config() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"name":"x","devDependencies":{"jest":"^29.0.0"}}"#,
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::Jest
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
    }

    #[test]
    fn node_reports_node_test_default_when_no_explicit_runner() {
        let dir = tempdir().unwrap();
        write(dir.path(), "package.json", r#"{"name":"x"}"#);

        let usage = detect(dir.path());
        // node --test is the ecosystem default.
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::NodeTest
                && u.source == TestRunnerSource::EcosystemDefault)
        );
    }

    // ------------------------------------------------------------------------
    // Python
    // ------------------------------------------------------------------------

    #[test]
    fn python_pytest_config() {
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(dir.path(), "pytest.ini", "[pytest]\n");

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::Pytest
            && matches!(u.source, TestRunnerSource::Config { .. })));
        // pytest (config) supersedes the unittest ecosystem default.
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Unittest),
            "unittest default must be superseded by configured pytest, got {usage:?}"
        );
    }

    #[test]
    fn python_tox_orchestrator_via_config() {
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(dir.path(), "tox.ini", "[tox]\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Tox
                && matches!(u.source, TestRunnerSource::Config { .. }))
        );
    }

    #[test]
    fn python_bare_tox_ini_is_not_attributed_to_pytest() {
        // `tox.ini` is shared by tox and pytest. A file with only `[tox]` must
        // attribute to tox, never pytest (false positive before section checks).
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(dir.path(), "tox.ini", "[tox]\nenvlist = py311\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Tox
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "tox should be detected via its [tox] section, got {usage:?}"
        );
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Pytest),
            "pytest must NOT be detected from a [tox]-only tox.ini, got {usage:?}"
        );
    }

    #[test]
    fn python_tox_ini_pytest_section_attributes_to_pytest() {
        // The same shared file with a `[pytest]` section legitimately signals
        // pytest in addition to tox.
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(
            dir.path(),
            "tox.ini",
            "[tox]\nenvlist = py311\n\n[pytest]\naddopts = -ra\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Tox),
            "tox should still be detected, got {usage:?}"
        );
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Pytest
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "pytest should be detected via its [pytest] section, got {usage:?}"
        );
    }

    #[test]
    fn python_nose2_setup_cfg_is_not_attributed_to_pytest() {
        // `setup.cfg` is shared by pytest (`[tool:pytest]`) and nose2
        // (`[unittest]`). A nose2 setup.cfg must not be read as pytest.
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(
            dir.path(),
            "setup.cfg",
            "[unittest]\nplugins = nose2.plugins\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Nose2
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "nose2 should be detected via its [unittest] section, got {usage:?}"
        );
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Pytest),
            "pytest must NOT be detected from a [unittest]-only setup.cfg, got {usage:?}"
        );
    }

    #[test]
    fn python_setup_cfg_tool_pytest_section_attributes_to_pytest() {
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(
            dir.path(),
            "setup.cfg",
            "[metadata]\nname = x\n\n[tool:pytest]\naddopts = -ra\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Pytest
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "pytest should be detected via its [tool:pytest] section, got {usage:?}"
        );
    }

    #[test]
    fn python_unittest_default_reported() {
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Unittest
                && u.source == TestRunnerSource::EcosystemDefault)
        );
    }

    // ------------------------------------------------------------------------
    // PHP
    // ------------------------------------------------------------------------

    #[test]
    fn php_phpunit_config_and_dep() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "composer.json",
            r#"{"name":"x/x","require-dev":{"phpunit/phpunit":"^10.0"}}"#,
        );
        write(dir.path(), "phpunit.xml.dist", "<phpunit/>\n");

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::PhpUnit
            && matches!(u.source, TestRunnerSource::Config { .. })));
    }

    #[test]
    fn php_pest_config_disambiguates() {
        let dir = tempdir().unwrap();
        // Pest carries phpunit/phpunit in many setups; the Pest.php config
        // file is what disambiguates.
        write(
            dir.path(),
            "composer.json",
            r#"{"name":"x/x","require-dev":{"phpunit/phpunit":"^10.0","pestphp/pest":"^2.0"}}"#,
        );
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        write(dir.path(), "tests/Pest.php", "<?php\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Pest
                && matches!(u.source, TestRunnerSource::Config { .. }))
        );
    }

    // ------------------------------------------------------------------------
    // Ruby
    // ------------------------------------------------------------------------

    #[test]
    fn ruby_rspec_dep_and_minitest_default() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Gemfile",
            "source 'https://rubygems.org'\ngem 'rspec'\n",
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::RSpec
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
        // rspec (manifest) supersedes the Minitest ecosystem default.
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Minitest),
            "Minitest default must be superseded by declared rspec, got {usage:?}"
        );
    }

    // ------------------------------------------------------------------------
    // JVM
    // ------------------------------------------------------------------------

    #[test]
    fn jvm_junit5_via_maven_pom() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "pom.xml",
            "<project>\n<dependencies>\n<dependency>\n<groupId>org.junit.jupiter</groupId>\n\
             <artifactId>junit-jupiter</artifactId>\n</dependency>\n</dependencies>\n</project>\n",
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::JUnit5
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
    }

    #[test]
    fn jvm_testng_via_gradle() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "build.gradle.kts",
            "dependencies { testImplementation(\"org.testng:testng:7.10.0\") }\n",
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::TestNg
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
    }

    // ------------------------------------------------------------------------
    // .NET
    // ------------------------------------------------------------------------

    #[test]
    fn dotnet_xunit_via_csproj() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Tests.csproj",
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n<ItemGroup>\n\
             <PackageReference Include=\"xunit\" Version=\"2.6.0\" />\n\
             <PackageReference Include=\"xunit.runner.visualstudio\" Version=\"2.5.0\" />\n\
             </ItemGroup>\n</Project>\n",
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::XUnit
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
    }

    // ------------------------------------------------------------------------
    // Elixir
    // ------------------------------------------------------------------------

    #[test]
    fn ellixir_ex_unit_default() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "mix.exs",
            "defmodule X.MixProject do\nuse Mix.Project\nend\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::ExUnit
                && u.source == TestRunnerSource::EcosystemDefault)
        );
    }

    #[test]
    fn elixir_espec_dep() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "mix.exs",
            "defp deps do\n[{:espec, \"~> 1.0\", only: :test}]\nend\n",
        );

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::ESpec
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
    }

    // ------------------------------------------------------------------------
    // No ecosystem / empty
    // ------------------------------------------------------------------------

    #[test]
    fn empty_package_has_no_runners() {
        let dir = tempdir().unwrap();
        let usage = detect(dir.path());
        assert!(
            usage.is_empty(),
            "no ecosystem => no runners, got {usage:?}"
        );
    }

    // ------------------------------------------------------------------------
    // Priority ordering
    // ------------------------------------------------------------------------

    #[test]
    fn config_signal_outranks_manifest_for_same_runner() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"name":"x","devDependencies":{"vitest":"^1.0.0"}}"#,
        );
        write(dir.path(), "vitest.config.ts", "export default {};\n");

        let usage = detect(dir.path());
        let vitest = usage
            .iter()
            .find(|u| u.runner == TestRunner::Vitest)
            .expect("vitest detected");
        assert!(
            matches!(vitest.source, TestRunnerSource::Config { .. }),
            "config signal should win, got {:?}",
            vitest.source
        );
    }

    // ------------------------------------------------------------------------
    // Exact dependency matching (no substring false positives)
    // ------------------------------------------------------------------------

    #[test]
    fn node_package_name_containing_runner_is_not_a_dependency() {
        // A package merely *named* `jest-helper` declares no jest dependency.
        let dir = tempdir().unwrap();
        write(dir.path(), "package.json", r#"{"name":"jest-helper"}"#);

        let usage = detect(dir.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Jest),
            "a package named jest-helper must not register Jest, got {usage:?}"
        );
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::NodeTest
                && u.source == TestRunnerSource::EcosystemDefault),
            "node --test default should still be reported, got {usage:?}"
        );
    }

    #[test]
    fn node_test_script_invoking_runner_is_not_a_dependency() {
        // A `test` script that shells out to jest is not a dependency key.
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"name":"x","scripts":{"test":"jest --ci"}}"#,
        );

        let usage = detect(dir.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Jest),
            "a test script invoking jest is not a dependency declaration, got {usage:?}"
        );
    }

    #[test]
    fn node_dependency_with_runner_prefix_does_not_match() {
        // `vitest-environment-jsdom` must not satisfy the exact key `vitest`.
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"name":"x","devDependencies":{"vitest-environment-jsdom":"^1.0.0"}}"#,
        );

        let usage = detect(dir.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Vitest),
            "vitest-environment-jsdom must not register Vitest, got {usage:?}"
        );
    }

    #[test]
    fn python_runner_via_requirements_txt() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "requirements.txt",
            "pytest==7.4.0\nrequests>=2\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Pytest
                && matches!(u.source, TestRunnerSource::Manifest { .. })),
            "pytest in requirements.txt should be detected, got {usage:?}"
        );
    }

    #[test]
    fn python_plugin_distribution_does_not_match_runner() {
        // `pytest-cov` is a plugin, not pytest itself.
        let dir = tempdir().unwrap();
        write(dir.path(), "requirements.txt", "pytest-cov==4.1.0\n");

        let usage = detect(dir.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Pytest),
            "pytest-cov must not register pytest, got {usage:?}"
        );
    }

    #[test]
    fn jvm_jupiter_pom_does_not_match_junit4_coordinate() {
        // org.junit.jupiter:junit-jupiter must not satisfy junit:junit.
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "pom.xml",
            "<project>\n<dependencies>\n<dependency>\n<groupId>org.junit.jupiter</groupId>\n\
             <artifactId>junit-jupiter</artifactId>\n</dependency>\n</dependencies>\n</project>\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::JUnit5),
            "JUnit 5 should be detected, got {usage:?}"
        );
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::JUnit4),
            "junit:junit must not match an org.junit.jupiter coordinate, got {usage:?}"
        );
    }

    #[test]
    fn jvm_junit4_via_gradle_coordinate() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "build.gradle",
            "dependencies { testImplementation 'junit:junit:4.13.2' }\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::JUnit4
                && matches!(u.source, TestRunnerSource::Manifest { .. })),
            "junit:junit coordinate should detect JUnit 4, got {usage:?}"
        );
    }

    #[test]
    fn dotnet_unrelated_package_does_not_match_runner() {
        // Include="MyXunitHelpers" must not satisfy the exact key `xunit`.
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Tests.csproj",
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n<ItemGroup>\n\
             <PackageReference Include=\"MyXunitHelpers\" Version=\"1.0.0\" />\n\
             </ItemGroup>\n</Project>\n",
        );

        let usage = detect(dir.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::XUnit),
            "Include=MyXunitHelpers must not register xUnit, got {usage:?}"
        );
    }

    // ------------------------------------------------------------------------
    // Glob / convention config signals
    // ------------------------------------------------------------------------

    #[test]
    fn php_codeception_suite_glob_matches() {
        // `*.suite.yml` is a glob; literal-only matching missed it entirely.
        let dir = tempdir().unwrap();
        write(dir.path(), "composer.json", r#"{"name":"x/x"}"#);
        write(dir.path(), "api.suite.yml", "actor: ApiTester\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Codeception
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "*.suite.yml should signal Codeception, got {usage:?}"
        );
    }

    #[test]
    fn php_unrelated_yaml_does_not_match_suite_glob() {
        let dir = tempdir().unwrap();
        write(dir.path(), "composer.json", r#"{"name":"x/x"}"#);
        write(dir.path(), "config.yml", "x: 1\n");

        let usage = detect(dir.path());
        assert!(
            !usage.iter().any(|u| u.runner == TestRunner::Codeception),
            "config.yml must not match *.suite.yml, got {usage:?}"
        );
    }

    #[test]
    fn php_behat_feature_glob_matches() {
        let dir = tempdir().unwrap();
        write(dir.path(), "composer.json", r#"{"name":"x/x"}"#);
        write(dir.path(), "features/login.feature", "Feature: login\n");

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::Behat
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "features/*.feature should signal Behat, got {usage:?}"
        );
    }

    #[test]
    fn ruby_rspec_recursive_spec_glob_matches() {
        // `spec/**/*_spec.rb` requires a recursive, depth-bounded walk.
        let dir = tempdir().unwrap();
        write(dir.path(), "Gemfile", "source 'https://rubygems.org'\n");
        write(
            dir.path(),
            "spec/models/user_spec.rb",
            "RSpec.describe User do\nend\n",
        );

        let usage = detect(dir.path());
        assert!(
            usage.iter().any(|u| u.runner == TestRunner::RSpec
                && matches!(u.source, TestRunnerSource::Config { .. })),
            "spec/**/*_spec.rb should signal RSpec, got {usage:?}"
        );
    }

    #[test]
    fn each_runner_appears_at_most_once() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "[package]\nname = \"x\"\nversion = \"0.1\"\n",
        );
        write(dir.path(), ".config/nextest.toml", "[profile.default]\n");

        let usage = detect(dir.path());
        let runners = runners_of(&usage);
        let mut deduped = runners.clone();
        deduped.sort_by_key(|r| r.variant_index());
        deduped.dedup();
        assert_eq!(deduped.len(), runners.len(), "duplicate runners detected");
    }
}
