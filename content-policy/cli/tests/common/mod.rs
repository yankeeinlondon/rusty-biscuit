//! The one way these tests spawn `policy`: in a temporary working directory,
//! with no inherited `CONTENT_POLICY_*` configuration or color forcing.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The lifecycle example document from the spec.
pub const LIFECYCLE: &str = "---
title: Lifecycle
last_updated: 2026-09-28
content_policy:
  - rule: ValidFor(3mo, @last_updated)
    action: refresh
  - rule: ValidUntil(2027-01-01)
    action: archive
---

Body.
";

const CONFIG_VARIABLES: [&str; 3] = [
    "CONTENT_POLICY_KEY",
    "CONTENT_POLICY_DEFAULT",
    "CONTENT_POLICY_DATE_PROPERTY",
];

pub struct Workspace {
    dir: tempfile::TempDir,
    env: Vec<(&'static str, String)>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("temporary workspace"),
            env: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Writes `name` into the workspace and returns its path.
    pub fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.dir.path().join(name);
        fs::write(&path, contents).expect("write fixture document");
        path
    }

    pub fn read(&self, name: &str) -> String {
        fs::read_to_string(self.dir.path().join(name)).expect("read fixture document")
    }

    /// Sets an environment variable for every later run.
    pub fn env(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.env.push((name, value.into()));
        self
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(biscuit_test_harness::bin_exe!("policy"));
        command.current_dir(self.dir.path());
        for name in CONFIG_VARIABLES {
            command.env_remove(name);
        }
        command
            .env_remove("FORCE_COLOR")
            .env_remove("CLICOLOR_FORCE")
            .env_remove("COMPLETE")
            .env("NO_COLOR", "1")
            .env("COLUMNS", "100");
        for (name, value) in &self.env {
            command.env(name, value);
        }
        command
    }

    pub fn run(&self, args: &[&str]) -> Run {
        let output = self.command().args(args).output().expect("spawn policy");
        Run::from(output)
    }
}

/// A finished run with its streams decoded.
#[derive(Debug)]
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl From<Output> for Run {
    fn from(output: Output) -> Self {
        Self {
            code: output.status.code().expect("policy exited normally"),
            stdout: String::from_utf8(output.stdout).expect("utf-8 stdout"),
            stderr: String::from_utf8(output.stderr).expect("utf-8 stderr"),
        }
    }
}

impl Run {
    #[track_caller]
    pub fn success(self) -> Self {
        assert_eq!(self.code, 0, "expected exit 0; stderr:\n{}", self.stderr);
        self
    }

    #[track_caller]
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|error| panic!("stdout is not JSON ({error}):\n{}", self.stdout))
    }
}
