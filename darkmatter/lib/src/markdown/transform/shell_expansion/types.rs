//! Type definitions for shell expansion.

use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use crate::markdown::transform::TransformSource;

/// Parsed `::shell` directive with raw command, executable, args, span, and line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDirective {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub span: std::ops::Range<usize>,
    pub line: usize,
}

/// Options for shell expansion behavior.
pub struct ShellExpansionOptions {
    pub timeout: std::time::Duration,
    pub policy_root: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
}

impl Clone for ShellExpansionOptions {
    fn clone(&self) -> Self {
        Self {
            timeout: self.timeout,
            policy_root: self.policy_root.clone(),
            working_directory: self.working_directory.clone(),
            approval_handler: self.approval_handler.clone(),
        }
    }
}

impl fmt::Debug for ShellExpansionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShellExpansionOptions")
            .field("timeout", &self.timeout)
            .field("policy_root", &self.policy_root)
            .field("working_directory", &self.working_directory)
            .field(
                "approval_handler",
                if self.approval_handler.is_some() {
                    &"Some(..)"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

impl Default for ShellExpansionOptions {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(10),
            policy_root: None,
            working_directory: None,
            approval_handler: None,
        }
    }
}

/// Trait for approval callbacks. The library never prompts directly.
pub trait ShellApprovalHandler: Send + Sync {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError>;
}

/// Information provided to the approval handler.
#[derive(Debug, Clone)]
pub struct ShellApprovalRequest {
    pub source: TransformSource,
    pub line: usize,
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub normalized_exact: String,
    pub whitelist_path: PathBuf,
    pub blacklist_path: PathBuf,
}

/// Possible decisions from approval handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellApprovalDecision {
    AllowExactPersist,
    AllowCommandPersist,
    AllowOnce,
    Deny,
    BlacklistPersist,
}

/// Errors from shell expansion operations.
#[derive(Error, Debug)]
pub enum ShellExpansionError {
    #[error("Shell directive parse error on line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    #[error("Command not found: '{command}' on line {line}")]
    CommandNotFound { command: String, line: usize },

    #[error("Blacklisted command '{command}' on line {line}: {reason}")]
    Blacklisted {
        command: String,
        reason: String,
        line: usize,
    },

    #[error("Approval required for '{command}' on line {line}")]
    ApprovalRequired {
        command: String,
        whitelist_path: PathBuf,
        blacklist_path: PathBuf,
        line: usize,
    },

    #[error("Command denied: '{command}' on line {line}")]
    Denied { command: String, line: usize },

    #[error("Command timed out after {timeout:?}: '{command}' on line {line}")]
    Timeout {
        command: String,
        timeout: std::time::Duration,
        line: usize,
    },

    #[error("Command failed (exit {code}): '{command}' on line {line}")]
    ExecutionFailed {
        command: String,
        code: i32,
        stdout: String,
        stderr: String,
        line: usize,
    },

    #[error("Policy I/O error for {path}: {source}")]
    PolicyIo {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Paths to whitelist and blacklist policy files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellPolicyPaths {
    pub whitelist: PathBuf,
    pub blacklist: PathBuf,
}

/// A single rule in a whitelist or user blacklist file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellRuleEntry {
    Exact(String),
    Prefix(String),
}

/// A loaded set of rules from a policy file.
#[derive(Debug, Clone, Default)]
pub struct ShellRuleSet {
    pub entries: Vec<ShellRuleEntry>,
}

impl ShellRuleSet {
    /// Returns `true` when `normalized` exactly matches a [`ShellRuleEntry::Exact`] rule.
    pub fn matches_exact(&self, normalized: &str) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e, ShellRuleEntry::Exact(s) if s == normalized))
    }

    /// Returns `true` when `executable` matches a [`ShellRuleEntry::Prefix`] rule.
    pub fn matches_prefix(&self, executable: &str) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e, ShellRuleEntry::Prefix(s) if s == executable))
    }
}

/// Runtime state for shell expansion across recursive document composition.
#[derive(Debug)]
pub struct ShellExpansionRuntime {
    pub allow_once: HashSet<String>,
    pub whitelist: ShellRuleSet,
    pub user_blacklist: ShellRuleSet,
    pub policy_paths: Option<ShellPolicyPaths>,
    pub approvals_used: usize,
}

impl ShellExpansionRuntime {
    pub fn new() -> Self {
        Self {
            allow_once: HashSet::new(),
            whitelist: ShellRuleSet::default(),
            user_blacklist: ShellRuleSet::default(),
            policy_paths: None,
            approvals_used: 0,
        }
    }

    /// Returns the number of approvals used since the last call and resets the counter.
    pub fn take_recent_approval_count(&mut self) -> usize {
        let count = self.approvals_used;
        self.approvals_used = 0;
        count
    }

    /// Ensures policy files are loaded (loads on first call, skips subsequent calls).
    pub fn ensure_loaded(&mut self, paths: &ShellPolicyPaths) -> Result<(), ShellExpansionError> {
        if self.policy_paths.is_some() {
            return Ok(());
        }
        self.whitelist = super::store::load_ruleset(&paths.whitelist)?;
        self.user_blacklist = super::store::load_ruleset(&paths.blacklist)?;
        self.policy_paths = Some(paths.clone());
        Ok(())
    }
}

/// Built-in blacklist rule types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlacklistRule {
    Executable(&'static str),
    ExecutablePrefix(&'static str),
    SubcommandPrefix {
        executable: &'static str,
        prefix: &'static [&'static str],
    },
    ArgExact {
        executable: &'static str,
        arg: &'static str,
    },
    ArgPrefix {
        executable: &'static str,
        arg_prefix: &'static str,
    },
    RawToken(&'static str),
}

/// Combined pipeline runtime for recursive composition.
#[derive(Debug)]
pub(crate) struct PipelineRuntime {
    pub transclusion: crate::markdown::transform::transclusion::TransclusionRuntime,
    pub shell: ShellExpansionRuntime,
}

impl PipelineRuntime {
    pub fn new(max_depth: usize) -> Self {
        Self {
            transclusion: crate::markdown::transform::transclusion::TransclusionRuntime::new(
                max_depth,
            ),
            shell: ShellExpansionRuntime::new(),
        }
    }
}
