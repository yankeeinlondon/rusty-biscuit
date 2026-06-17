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
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::filesystem::repo::detection::ManifestCache;
use crate::programs::contract::CategoryEnum;
use crate::programs::enums::TestRunner;
use crate::programs::test_runner_spec::{TestRunnerEcosystem, TEST_RUNNER_SPEC};

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
    /// A config file owned by the runner was found in the package directory.
    Config {
        /// The config filename or glob that matched (e.g. `vitest.config.ts`).
        filename: String,
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

/// Detect declared test runner usage for the package rooted at `pkg_dir`.
///
/// Follows the signal priority from `test-runner-strategy.md` §4:
///
/// 1. **Config file present** in the package dir — strongest, disambiguates.
/// 2. **Manifest dependency key** — strong.
/// 3. **Ecosystem default** — fallback when no explicit runner is found but
///    the ecosystem always ships one.
/// 4. **Convention** — weakest; emitted for stdlib runners with no marker.
///
/// A package is never empty for ecosystems with a built-in default: the
/// default is reported with [`TestRunnerSource::EcosystemDefault`] so
/// consumers can distinguish "explicitly configured" from "implicitly
/// available".
pub(crate) fn detect_test_runners(pkg_dir: &Path, cache: &mut ManifestCache) -> Vec<TestRunnerUsage> {
    let ecosystems = ecosystems_present(pkg_dir);
    if ecosystems.is_empty() {
        return Vec::new();
    }

    // Read the raw text of every manifest the package might declare a runner
    // in. Substring search over the raw text is robust across manifest
    // formats (TOML / JSON / XML / Gradle DSL / Elixir / Ruby) and avoids
    // bespoke parsers for v1; config-file presence is the stronger signal
    // and disambiguates the common cases. We collect owned strings because
    // each `raw_text` call mutably borrows the cache.
    let manifest_blobs: Vec<String> = collect_manifest_text(pkg_dir, cache);
    let manifest_refs: Vec<&str> = manifest_blobs.iter().map(String::as_str).collect();

    let mut found: Vec<TestRunnerUsage> = Vec::new();
    let mut seen: HashSet<TestRunner> = HashSet::new();

    // The static catalog is keyed by TestRunner variant ordinal, so the
    // enumerated iteration maps spec index -> runner variant exactly.
    let indexed: Vec<(usize, &'static crate::programs::test_runner_spec::TestRunnerSpec)> =
        TEST_RUNNER_SPEC.iter().enumerate().collect();

    // Pass 1: config files (strongest signal).
    for (idx, spec) in &indexed {
        if !ecosystems.contains(&spec.ecosystem) {
            continue;
        }
        let runner = runner_at(*idx);
        for glob in spec.config_globs {
            if config_glob_matches(pkg_dir, runner, glob, cache) {
                push_unique(
                    &mut found,
                    &mut seen,
                    runner,
                    TestRunnerSource::Config {
                        filename: (*glob).to_string(),
                    },
                );
                break;
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
            if manifest_key_present(&manifest_refs, key) {
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
                push_unique(
                    &mut found,
                    &mut seen,
                    runner,
                    TestRunnerSource::Convention,
                );
            }
        }
    }

    found.sort_by_key(|u| u.runner.variant_index());
    found
}

/// Detect declared test runners for a single project directory, building a
/// fresh manifest cache.
///
/// This is the entry point for non-monorepo contexts where no `Package` is
/// produced by repo detection (e.g. a standalone Cargo crate). For monorepo
/// packages, prefer reading [`Package::test_runners`] which is populated
/// during repo detection.
#[must_use]
pub fn detect_test_runners_for_dir(dir: &Path) -> Vec<TestRunnerUsage> {
    let mut cache = ManifestCache::default();
    detect_test_runners(dir, &mut cache)
}

/// Returns `true` when `glob` is present in `pkg_dir` and, for config files
/// shared by more than one runner, the runner-specific INI section is present.
///
/// `tox.ini` and `setup.cfg` are each claimed by several Python runners
/// (`tox.ini` by both pytest and tox; `setup.cfg` by both pytest and nose2).
/// Bare file presence would mis-attribute a `tox.ini` containing only `[tox]`
/// to pytest, or a nose2 `setup.cfg` to pytest. For those shared files we
/// require the section header named in `test-runner-strategy.md` §5; every
/// other config glob is runner-exclusive and matches on existence alone.
fn config_glob_matches(
    pkg_dir: &Path,
    runner: TestRunner,
    glob: &str,
    cache: &mut ManifestCache,
) -> bool {
    let path = pkg_dir.join(glob);
    if !path.exists() {
        return false;
    }
    match required_config_section(runner, glob) {
        Some(section) => cache
            .raw_text(&path)
            .is_some_and(|text| ini_has_section(text, section)),
        None => true,
    }
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
    if pkg_dir.join("Cargo.toml").exists() {
        set.insert(TestRunnerEcosystem::Rust);
    }
    if pkg_dir.join("go.mod").exists() {
        set.insert(TestRunnerEcosystem::Go);
    }
    if pkg_dir.join("package.json").exists() {
        set.insert(TestRunnerEcosystem::Node);
    }
    if pkg_dir.join("pyproject.toml").exists()
        || pkg_dir.join("requirements.txt").exists()
        || pkg_dir.join("setup.py").exists()
    {
        set.insert(TestRunnerEcosystem::Python);
    }
    if pkg_dir.join("composer.json").exists() {
        set.insert(TestRunnerEcosystem::Php);
    }
    if pkg_dir.join("Gemfile").exists() || has_gemspec(pkg_dir) {
        set.insert(TestRunnerEcosystem::Ruby);
    }
    if pkg_dir.join("pom.xml").exists()
        || pkg_dir.join("build.gradle").exists()
        || pkg_dir.join("build.gradle.kts").exists()
    {
        set.insert(TestRunnerEcosystem::Jvm);
    }
    if has_csproj(pkg_dir) {
        set.insert(TestRunnerEcosystem::DotNet);
    }
    if pkg_dir.join("mix.exs").exists() {
        set.insert(TestRunnerEcosystem::Elixir);
    }
    set
}

/// Collect the raw text of every manifest the package might declare a runner
/// in, using the shared cache to avoid re-reading files. Returns owned
/// strings because each `raw_text` call mutably borrows the cache and the
/// returned references cannot outlive a subsequent call.
fn collect_manifest_text(pkg_dir: &Path, cache: &mut ManifestCache) -> Vec<String> {
    let mut blobs: Vec<String> = Vec::new();
    let candidates = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "requirements.txt",
        "go.mod",
        "composer.json",
        "Gemfile",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "mix.exs",
    ];
    for name in candidates {
        let path = pkg_dir.join(name);
        if let Some(text) = cache.raw_text(&path) {
            blobs.push(text.to_string());
        }
    }
    // Ruby gemspec and .NET .csproj have arbitrary names — scan the package
    // dir for them. The package dir scan is shallow (no recursion) so it is
    // cheap and bounded.
    if let Ok(entries) = std::fs::read_dir(pkg_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && (name.ends_with(".gemspec") || name.ends_with(".csproj"))
            {
                let path = entry.path();
                if let Some(text) = cache.raw_text(&path) {
                    blobs.push(text.to_string());
                }
            }
        }
    }
    blobs
}

/// Returns `true` when `key` appears as a dependency declaration in any of the
/// manifest blobs.
///
/// Substring search is intentionally lenient: it matches Gradle DSL
/// (`testImplementation("org.junit.jupiter:junit-jupiter")`), Elixir
/// (`{:espec, …}`), and Ruby (`gem "rspec"`) uniformly. For Maven coordinates
/// of the form `groupId:artifactId` (e.g. `org.junit.jupiter:junit-jupiter`),
/// the blob may carry the two halves in separate XML elements
/// (`<groupId>…</groupId><artifactId>…</artifactId>`), so a colon key matches
/// when **both** halves appear as substrings; a plain key matches on a single
/// substring hit. The stronger config-file signal disambiguates the common
/// ambiguous cases.
fn manifest_key_present(blobs: &[&str], key: &str) -> bool {
    if key.contains(':') {
        let halves: Vec<&str> = key.split(':').filter(|s| !s.is_empty()).collect();
        blobs.iter().any(|blob| {
            halves
                .iter()
                .all(|half| blob.contains(half))
        })
    } else {
        blobs.iter().any(|blob| blob.contains(key))
    }
}

/// Returns `true` when the package dir contains a `.gemspec` file.
fn has_gemspec(pkg_dir: &Path) -> bool {
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
        if pkg_dir.join(dir).is_dir() {
            return true;
        }
    }
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
        let mut cache = ManifestCache::default();
        detect_test_runners(dir, &mut cache)
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
        write(dir.path(), "Cargo.toml", "[package]\nname = \"x\"\nversion = \"0.1\"\n");

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
        write(dir.path(), "Cargo.toml", "[package]\nname = \"x\"\nversion = \"0.1\"\n");
        write(dir.path(), ".config/nextest.toml", "[profile.default]\n");

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::Nextest
            && matches!(u.source, TestRunnerSource::Config { .. })));
        // The default cargo test is still reported alongside nextest.
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
        assert!(usage.iter().any(|u| u.runner == TestRunner::GoTest
            && u.source == TestRunnerSource::EcosystemDefault));
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
        assert!(usage.iter().any(|u| u.runner == TestRunner::NodeTest
            && u.source == TestRunnerSource::EcosystemDefault));
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
        // unittest is the ecosystem default and must still appear.
        assert!(usage.iter().any(|u| u.runner == TestRunner::Unittest));
    }

    #[test]
    fn python_tox_orchestrator_via_config() {
        let dir = tempdir().unwrap();
        write(dir.path(), "pyproject.toml", "[project]\nname = \"x\"\n");
        write(dir.path(), "tox.ini", "[tox]\n");

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::Tox
            && matches!(u.source, TestRunnerSource::Config { .. })));
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
        write(dir.path(), "setup.cfg", "[unittest]\nplugins = nose2.plugins\n");

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
        assert!(usage.iter().any(|u| u.runner == TestRunner::Unittest
            && u.source == TestRunnerSource::EcosystemDefault));
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
        assert!(usage.iter().any(|u| u.runner == TestRunner::Pest
            && matches!(u.source, TestRunnerSource::Config { .. })));
    }

    // ------------------------------------------------------------------------
    // Ruby
    // ------------------------------------------------------------------------

    #[test]
    fn ruby_rspec_dep_and_minitest_default() {
        let dir = tempdir().unwrap();
        write(dir.path(), "Gemfile", "source 'https://rubygems.org'\ngem 'rspec'\n");

        let usage = detect(dir.path());
        assert!(usage.iter().any(|u| u.runner == TestRunner::RSpec
            && matches!(u.source, TestRunnerSource::Manifest { .. })));
        // Minitest is the ecosystem default for Ruby.
        assert!(usage.iter().any(|u| u.runner == TestRunner::Minitest));
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
        assert!(usage.iter().any(|u| u.runner == TestRunner::ExUnit
            && u.source == TestRunnerSource::EcosystemDefault));
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
        assert!(usage.is_empty(), "no ecosystem => no runners, got {usage:?}");
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

    #[test]
    fn each_runner_appears_at_most_once() {
        let dir = tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "[package]\nname = \"x\"\nversion = \"0.1\"\n");
        write(dir.path(), ".config/nextest.toml", "[profile.default]\n");

        let usage = detect(dir.path());
        let runners = runners_of(&usage);
        let mut deduped = runners.clone();
        deduped.sort_by_key(|r| r.variant_index());
        deduped.dedup();
        assert_eq!(deduped.len(), runners.len(), "duplicate runners detected");
    }
}
