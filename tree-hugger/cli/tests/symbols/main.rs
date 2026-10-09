//! `hug` checks for `2026-10-03-visibility-and-kind`: representative CLI wiring
//! of kinds, visibility and static members (test 7), and the warm-cache
//! upgrade across a schema and an extractor revision change (test 8), the
//! persistent-cache reader contract for malformed entry and symbol fields,
//! concurrent `hug` processes sharing one entry and `--verbose` cache
//! reporting, plus the declaration reports: the `hug symbols` goldens over
//! the 16-language `multi/` fixture tree, the JSON envelope and its errors,
//! `--sort` and grouping, `classes`, the record selectors (`filters`), the
//! report formats and views (`reports`: JSON Lines, count, stats, table,
//! outline, signature modes, wrapping),
//! `--prelude`, `refs` (reference reports), and equivalent spellings
//! of one file sharing its cache entry and listing; and the marker-scoped
//! report reader the `level2_*` terminal tests share.

mod cache_lookup;
mod cache_lexical;
mod cache_package_boundaries;
mod cache_robustness;
mod cache_upgrade;
mod classes;
mod filters;
mod golden;
mod grouping_acceptance;
mod grouping_core;
mod grouping_reports;
mod order;
mod outline;
mod prelude;
mod refs;
mod report;
mod reports;
mod export_plumbing;
mod export_tsv;
mod export_ctags;
mod export_ctags_paths;
mod export_controls;
mod export_order;
mod export_prose_markdown;
mod spelling;
mod terminal_scope;
mod wiring;

// The `level2_*` binaries' report reader; `terminal_scope` tests it here,
// where no terminal is needed. Parts it does not exercise go unused.
#[allow(dead_code)]
#[path = "../support/terminal_report.rs"]
mod terminal_report;

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

pub const TYPESCRIPT_FIXTURE: &str =
    include_str!("../../../lib/tests/fixtures/symbols/typescript.ts");
pub const RUST_FIXTURE: &str = include_str!("../../../lib/tests/fixtures/symbols/rust.rs");
pub const CPP_FIXTURE: &str = include_str!("../../../lib/tests/fixtures/symbols/cpp.cpp");
pub const JAVA_FIXTURE: &str = include_str!("../../../lib/tests/fixtures/symbols/java.java");
pub const CSHARP_FIXTURE: &str = include_str!("../../../lib/tests/fixtures/symbols/c_sharp.cs");
pub const PYTHON_FIXTURE: &str = include_str!("../../../lib/tests/fixtures/symbols/python.py");
pub const BASH_FIXTURE: &str = include_str!("../../../lib/tests/fixtures/symbols/bash.sh");

/// A scratch Git repository plus test-owned home, config and temp
/// directories. The temp directory holds `hug`'s persistent cache, so no run
/// sees the caller's cache, checkout or configuration.
pub struct Scratch {
    pub repo: TempDir,
    pub home: TempDir,
    pub temp: TempDir,
}

impl Scratch {
    /// Creates the repository and writes each `(file name, contents)` into it.
    pub fn new(files: &[(&str, &str)]) -> Self {
        let scratch = Self {
            repo: TempDir::new().expect("scratch repo"),
            home: TempDir::new().expect("scratch home"),
            temp: TempDir::new().expect("scratch temp"),
        };
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(scratch.repo.path())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("HOME", scratch.home.path())
            .env("USERPROFILE", scratch.home.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git must be on PATH to create the scratch repository");
        assert!(status.success(), "git init failed in the scratch repository");
        for (name, contents) in files {
            let path = scratch.repo.path().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create fixture directory");
            }
            std::fs::write(path, contents).expect("write fixture");
        }
        scratch
    }

    /// Runs `hug` in the repository with the cache under `temp`.
    pub fn hug(&self, args: &[&str]) -> Output {
        self.run(self.temp.path(), args, &[])
    }

    /// Runs `hug` with the persistent cache under `temp`.
    pub fn hug_with_temp(&self, temp: &Path, args: &[&str]) -> Output {
        self.run(temp, args, &[])
    }

    /// Runs `hug` with extra environment variables set after the isolation ones.
    pub fn hug_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.run(self.temp.path(), args, env)
    }

    /// A `hug` process command with the same isolation as [`Self::hug`], for
    /// tests that drive the child's pipes themselves.
    #[allow(deprecated)] // `cargo_bin` is the area's binary-launch helper
    pub fn process(&self, args: &[&str]) -> std::process::Command {
        let config = self.home.path().join(".config");
        let temp = self.temp.path();
        let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin("hug"));
        command
            .args(args)
            .current_dir(self.repo.path())
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("XDG_CONFIG_HOME", &config)
            .env("APPDATA", &config)
            .env("LOCALAPPDATA", &config)
            .env("TMPDIR", temp)
            .env("TMP", temp)
            .env("TEMP", temp)
            .env("NO_COLOR", "1")
            .env_remove("CLICOLOR_FORCE")
            .env("RAYON_NUM_THREADS", "2");
        command
    }

    #[allow(deprecated)] // `cargo_bin` is the area's binary-launch helper
    fn run(&self, temp: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let config = self.home.path().join(".config");
        let mut command = Command::cargo_bin("hug").expect("hug binary");
        command
            .args(args)
            .current_dir(self.repo.path())
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("XDG_CONFIG_HOME", &config)
            .env("APPDATA", &config)
            .env("LOCALAPPDATA", &config)
            .env("TMPDIR", temp)
            .env("TMP", temp)
            .env("TEMP", temp)
            .env("NO_COLOR", "1")
            .env_remove("CLICOLOR_FORCE")
            // Still parallel, without one worker per core in every
            // concurrently running test process.
            .env("RAYON_NUM_THREADS", "2")
            .envs(env.iter().copied());
        command.output().expect("run hug")
    }

    /// Runs `hug` and returns stdout, failing with stderr when it exits non-zero.
    pub fn stdout(&self, args: &[&str]) -> String {
        stdout_of(self.hug(args), args)
    }
}

/// Returns stdout of a successful run, failing with stderr otherwise.
pub fn stdout_of(output: Output, args: &[&str]) -> String {
    assert!(
        output.status.success(),
        "hug {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// Parses `hug --json` output.
pub fn json_of(stdout: &str) -> Value {
    serde_json::from_str(stdout).unwrap_or_else(|error| panic!("invalid JSON ({error}):\n{stdout}"))
}

/// Every `.json` entry file under `root`, sorted.
pub fn cache_entries(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "json") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(root, &mut found);
    found.sort();
    found
}
