//! Test-runner metadata and host-resolution types.
//!
//! Test runners violate the usual "one program == one global binary on PATH"
//! model. A runner may be a standalone binary, an ecosystem subcommand, a
//! runtime module, or a project-local vendored executable. This module defines
//! the static catalog (`TestRunnerSpec`) and the host-resolution result
//! (`Availability`) used by the `TestRunner` category.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How a test runner is invoked on the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationClass {
    /// Standalone executable discoverable on PATH or in a project bin dir.
    PathBinary,
    /// Subcommand built into a parent tool (e.g. `cargo test`, `go test`).
    EcosystemSubcommand,
    /// Invoked through a language runtime (e.g. `python -m unittest`).
    RuntimeModule,
    /// Vendored binary under the project tree (e.g. `node_modules/.bin`).
    ProjectLocal,
}

/// Logical role of a test runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    /// Plain test runner.
    Runner,
    /// Environment orchestrator that delegates to another runner.
    Orchestrator,
    /// BDD-style test framework.
    Bdd,
}

/// Ecosystem that owns a test runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunnerEcosystem {
    Rust,
    Go,
    Node,
    Python,
    Php,
    Ruby,
    Jvm,
    DotNet,
    Elixir,
}

/// Static metadata for a single test runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestRunnerSpec {
    /// Ecosystem that owns this runner.
    pub ecosystem: TestRunnerEcosystem,
    /// How the runner is invoked on the host.
    pub invocation: InvocationClass,
    /// Logical role of the runner.
    pub kind: RunnerKind,
    /// Parent binary for class B/C host detection (e.g. `cargo`, `go`, `python3`).
    pub parent_binary: Option<&'static str>,
    /// Exact dependency names that signal this runner in a package manifest.
    pub manifest_dep_keys: &'static [&'static str],
    /// Exact config filenames/globs that signal this runner in the package dir.
    pub config_globs: &'static [&'static str],
    /// Whether the runner is implicitly available for its ecosystem.
    pub is_ecosystem_default: bool,
}

/// Result of resolving a test runner on the current host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "availability")]
pub enum Availability {
    /// Found as a global executable on PATH.
    Installed { path: PathBuf },
    /// Found in a project-local bin directory.
    Local { path: PathBuf, root: PathBuf },
    /// Available via a parent binary (ecosystem subcommand or runtime module).
    ViaParent { parent: String },
    /// Not found on the host.
    NotFound,
}

impl Availability {
    /// Returns `true` when the runner is runnable from the host.
    #[must_use]
    pub fn is_found(&self) -> bool {
        !matches!(self, Self::NotFound)
    }
}

/// Static catalog keyed by `TestRunner` ordinal.
pub static TEST_RUNNER_SPEC: &[TestRunnerSpec] = &[
    // Rust
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Rust,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("cargo"),
        manifest_dep_keys: &[],
        config_globs: &[],
        is_ecosystem_default: true,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Rust,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &[],
        config_globs: &[".config/nextest.toml", "nextest.toml"],
        is_ecosystem_default: false,
    },
    // Go
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Go,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("go"),
        manifest_dep_keys: &[],
        config_globs: &[],
        is_ecosystem_default: true,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Go,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["gotest.tools/gotestsum"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Go,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Bdd,
        parent_binary: None,
        manifest_dep_keys: &["github.com/onsi/ginkgo/v2"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    // JS/TS
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["vitest"],
        config_globs: &[
            "vitest.config.ts",
            "vitest.config.js",
            "vitest.config.mjs",
            "vitest.config.mts",
            "vitest.config.cts",
            "vitest.config.cjs",
        ],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["jest", "@jest/core"],
        config_globs: &[
            "jest.config.js",
            "jest.config.ts",
            "jest.config.mjs",
            "jest.config.cjs",
            "jest.config.json",
        ],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["mocha"],
        config_globs: &[
            ".mocharc.js",
            ".mocharc.cjs",
            ".mocharc.mjs",
            ".mocharc.yaml",
            ".mocharc.yml",
            ".mocharc.json",
            ".mocharc.jsonc",
        ],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["ava"],
        config_globs: &["ava.config.js", "ava.config.cjs", "ava.config.mjs"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::RuntimeModule,
        kind: RunnerKind::Runner,
        parent_binary: Some("node"),
        manifest_dep_keys: &[],
        config_globs: &[],
        is_ecosystem_default: true,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["jasmine", "jasmine-core"],
        config_globs: &["spec/support/jasmine.json"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["tap"],
        config_globs: &[".taprc"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Node,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["uvu"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    // Python
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Python,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["pytest"],
        config_globs: &["pytest.ini", "conftest.py", "tox.ini", "setup.cfg"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Python,
        invocation: InvocationClass::RuntimeModule,
        kind: RunnerKind::Runner,
        parent_binary: Some("python3"),
        manifest_dep_keys: &[],
        config_globs: &[],
        is_ecosystem_default: true,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Python,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["nose2"],
        config_globs: &["unittest.cfg", "nose2.cfg", "setup.cfg"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Python,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Orchestrator,
        parent_binary: None,
        manifest_dep_keys: &["tox"],
        config_globs: &["tox.ini", "tox.toml"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Python,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Orchestrator,
        parent_binary: None,
        manifest_dep_keys: &["nox"],
        config_globs: &["noxfile.py"],
        is_ecosystem_default: false,
    },
    // PHP
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Php,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["phpunit/phpunit"],
        config_globs: &["phpunit.xml", "phpunit.xml.dist"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Php,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["pestphp/pest"],
        config_globs: &["tests/Pest.php"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Php,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["codeception/codeception"],
        config_globs: &["codeception.yml", "*.suite.yml"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Php,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Bdd,
        parent_binary: None,
        manifest_dep_keys: &["behat/behat"],
        config_globs: &["behat.yml", "behat.yml.dist", "behat.dist.yml", "behat.php"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Php,
        invocation: InvocationClass::ProjectLocal,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["atoum/atoum"],
        config_globs: &[".atoum.php"],
        is_ecosystem_default: false,
    },
    // Ruby
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Ruby,
        invocation: InvocationClass::PathBinary,
        kind: RunnerKind::Runner,
        parent_binary: None,
        manifest_dep_keys: &["rspec"],
        config_globs: &[".rspec", "spec/spec_helper.rb"],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Ruby,
        invocation: InvocationClass::RuntimeModule,
        kind: RunnerKind::Runner,
        parent_binary: Some("ruby"),
        manifest_dep_keys: &["minitest"],
        config_globs: &[],
        is_ecosystem_default: true,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Ruby,
        invocation: InvocationClass::RuntimeModule,
        kind: RunnerKind::Runner,
        parent_binary: Some("ruby"),
        manifest_dep_keys: &["test-unit"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    // JVM
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Jvm,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("mvn"),
        manifest_dep_keys: &["org.junit.jupiter:junit-jupiter"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Jvm,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("mvn"),
        manifest_dep_keys: &["junit:junit"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Jvm,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("mvn"),
        manifest_dep_keys: &["org.testng:testng"],
        config_globs: &["testng.xml"],
        is_ecosystem_default: false,
    },
    // .NET
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::DotNet,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("dotnet"),
        manifest_dep_keys: &["xunit", "xunit.runner.visualstudio"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::DotNet,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("dotnet"),
        manifest_dep_keys: &["NUnit", "NUnit3TestAdapter"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::DotNet,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("dotnet"),
        manifest_dep_keys: &["MSTest", "MSTest.TestFramework", "MSTest.TestAdapter"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
    // Elixir
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Elixir,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Runner,
        parent_binary: Some("mix"),
        manifest_dep_keys: &[],
        config_globs: &[],
        is_ecosystem_default: true,
    },
    TestRunnerSpec {
        ecosystem: TestRunnerEcosystem::Elixir,
        invocation: InvocationClass::EcosystemSubcommand,
        kind: RunnerKind::Bdd,
        parent_binary: Some("mix"),
        manifest_dep_keys: &["espec"],
        config_globs: &[],
        is_ecosystem_default: false,
    },
];
