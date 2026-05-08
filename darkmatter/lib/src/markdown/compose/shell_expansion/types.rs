//! Type definitions for shell expansion.

use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;
use crate::markdown::toc::MarkdownTocNode;
use crate::markdown::types::MarkdownResult;

/// Where a shell command originates in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommandOrigin {
    /// Body `::shell` directive at the given 1-indexed line.
    Body { line: usize },
    /// Frontmatter property with a `$(...)` shell expression.
    Frontmatter { key: String },
    /// Command inside a `::shell-block` / `::end-block` region.
    ///
    /// `start_line` is the 1-indexed line of the `::shell-block` opener.
    /// `command_line` is the 1-indexed line of the specific command within
    /// the block that would execute.
    ShellBlock {
        start_line: usize,
        command_line: usize,
    },
}

impl fmt::Display for ShellCommandOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body { line } => write!(f, "line {line}"),
            Self::Frontmatter { key } => write!(f, "frontmatter.{key}"),
            Self::ShellBlock {
                start_line,
                command_line,
            } => write!(
                f,
                "shell block starting at line {start_line}, command at line {command_line}"
            ),
        }
    }
}

impl ShellCommandOrigin {
    /// Returns the line number if this is a body or shell-block origin,
    /// or 0 for frontmatter.
    pub fn line_number(&self) -> usize {
        match self {
            Self::Body { line } => *line,
            Self::Frontmatter { .. } => 0,
            Self::ShellBlock { command_line, .. } => *command_line,
        }
    }
}

/// Parsed `::shell` directive with raw command, executable, args, span, and origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDirective {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub span: std::ops::Range<usize>,
    pub origin: ShellCommandOrigin,
    pub error_handling: ErrorHandling,
    /// Per-command timeout override. When Some, takes precedence over the global timeout.
    pub timeout_override: Option<std::time::Duration>,
    /// Parsed pipeline (supports chaining with && and ||).
    pub pipeline: Option<ShellPipeline>,
}

impl ShellDirective {
    /// Returns the first executable in the directive (for backward compatibility).
    pub fn first_executable(&self) -> &str {
        if let Some(ref pipeline) = self.pipeline {
            &pipeline.actions[0].command.executable
        } else {
            &self.executable
        }
    }

    pub fn first_args(&self) -> &[String] {
        if let Some(ref pipeline) = self.pipeline {
            &pipeline.actions[0].command.args
        } else {
            &self.args
        }
    }

    /// Returns true if this directive contains a chain (multiple actions).
    pub fn is_chain(&self) -> bool {
        self.pipeline
            .as_ref()
            .is_some_and(|p| p.actions.len() > 1)
    }
}

/// A parsed pipeline of commands connected by chain operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellPipeline {
    /// Ordered list of actions with their preceding operators.
    /// The first action always has `ChainOperator::None`.
    pub actions: Vec<PipelineAction>,
}

impl ShellPipeline {
    /// Creates a simple single-command pipeline.
    pub fn single(executable: String, args: Vec<String>) -> Self {
        Self {
            actions: vec![PipelineAction {
                operator: ChainOperator::None,
                command: CommandAction {
                    executable,
                    args,
                    redirection: RedirectionConfig::default(),
                },
            }],
        }
    }

    /// Returns all executables in the pipeline.
    pub fn executables(&self) -> Vec<&str> {
        self.actions.iter().map(|a| a.command.executable.as_str()).collect()
    }

    /// Returns a display string for the whole pipeline including chain
    /// operators and redirection tokens.
    pub fn display_string(&self) -> String {
        let mut parts = Vec::new();
        for action in &self.actions {
            let op_str = match action.operator {
                ChainOperator::None => "",
                ChainOperator::And => " && ",
                ChainOperator::Or => " || ",
            };
            let mut cmd = action.command.executable.clone();
            for arg in &action.command.args {
                if arg.contains(' ') || arg.contains('"') || arg.contains('\'') {
                    cmd.push_str(&format!(" \"{}\"", arg.replace('"', "\\\"")));
                } else {
                    cmd.push_str(&format!(" {arg}"));
                }
            }
            cmd.push_str(&render_redirection(&action.command.redirection));
            parts.push(format!("{op_str}{cmd}"));
        }
        parts.join("")
    }
}

/// A single action in a pipeline, with its chain operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineAction {
    /// The chain operator preceding this action (None for the first).
    pub operator: ChainOperator,
    /// The command to execute.
    pub command: CommandAction,
}

/// Chain operator connecting pipeline actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOperator {
    /// First command in the pipeline (no preceding operator).
    None,
    /// `&&` — execute next only if previous succeeded.
    And,
    /// `||` — execute next only if previous failed.
    Or,
}

/// A single command with its redirection configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAction {
    pub executable: String,
    pub args: Vec<String>,
    pub redirection: RedirectionConfig,
}

/// Configures stdout/stderr redirection for a command action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RedirectionConfig {
    pub stdout: StdoutTarget,
    pub stderr: StderrTarget,
}

/// Where stdout should be directed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StdoutTarget {
    /// Capture stdout normally (default).
    #[default]
    Capture,
    /// Discard stdout (`> /dev/null`).
    Null,
    /// Route stdout to stderr (`>&2`).
    ToStderr,
}

/// Renders a [`RedirectionConfig`] as the trailing redirection tokens that
/// would appear after a command (with a leading space when non-empty).
fn render_redirection(redir: &RedirectionConfig) -> String {
    let mut out = String::new();
    match redir.stdout {
        StdoutTarget::Capture => {}
        StdoutTarget::Null => out.push_str(" > /dev/null"),
        StdoutTarget::ToStderr => out.push_str(" >&2"),
    }
    match redir.stderr {
        StderrTarget::Capture => {}
        StderrTarget::Null => out.push_str(" 2> /dev/null"),
        StderrTarget::ToStdout => out.push_str(" 2>&1"),
    }
    out
}

/// Where stderr should be directed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StderrTarget {
    /// Capture stderr normally (default).
    #[default]
    Capture,
    /// Discard stderr (`2> /dev/null`).
    Null,
    /// Merge stderr into stdout (`2>&1`).
    ToStdout,
}

/// A shell command discovered during document graph analysis.
///
/// Returned by [`collect_shell_commands`] for pre-flight approval.
/// Contains enough information for the caller to check policy and
/// prompt the user without needing to understand the document graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandEntry {
    /// The raw command as written in the directive (e.g., "sniff repo packages").
    pub raw_command: String,
    /// The resolved executable name (e.g., "sniff").
    pub executable: String,
    /// The argument list (e.g., ["repo", "packages"]).
    pub args: Vec<String>,
    /// Normalized form for whitelist matching (e.g., "sniff repo packages").
    pub normalized: String,
    /// Source file where this directive was found.
    pub source_file: PathBuf,
    /// Where this command originates (body line or frontmatter key).
    pub origin: ShellCommandOrigin,
}

/// Error handling options parsed from `::shell` directive flags.
///
/// These options control how non-zero exit codes are handled, allowing
/// directives to gracefully handle expected failures instead of aborting
/// the pipeline.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::shell_expansion::types::{ErrorHandling, ErrorHandlingOutcome};
///
/// let handling = ErrorHandling {
///     when_error: Some("fallback text".to_string()),
///     ..Default::default()
/// };
/// let outcome = handling.resolve(1, "");
/// assert!(matches!(outcome, ErrorHandlingOutcome::Replace(ref s) if s == "fallback text"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorHandling {
    /// Catch-all: replace any non-zero exit with this text.
    pub when_error: Option<String>,
    /// Replace specific exit codes with text.
    pub when_exit_code: Vec<(i32, String)>,
    /// Replace all exit codes except the specified one with text.
    pub except_exit_code: Vec<(i32, String)>,
    /// If stderr contains `find`, replace with `replace_with`.
    pub stderr_contains: Vec<(String, String)>,
    /// If stderr lacks `find`, replace with `replace_with`.
    pub stderr_lacks: Vec<(String, String)>,
    /// Enrich error message for any failure.
    pub enrich_error: Option<String>,
    /// Enrich error message for specific exit code.
    pub enrich_error_on: Vec<(i32, String)>,
}

/// Outcome of applying error handling rules to a failed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorHandlingOutcome {
    /// Replace the directive with this text (error suppressed).
    Replace(String),
    /// Propagate the error but add this enrichment context.
    Enrich(String),
    /// Propagate the error as-is.
    Propagate,
}

impl ErrorHandling {
    /// Returns `true` if no error handling options are configured.
    pub fn is_empty(&self) -> bool {
        self.when_error.is_none()
            && self.when_exit_code.is_empty()
            && self.except_exit_code.is_empty()
            && self.stderr_contains.is_empty()
            && self.stderr_lacks.is_empty()
            && self.enrich_error.is_none()
            && self.enrich_error_on.is_empty()
    }

    /// Resolves error handling for a failed command.
    ///
    /// Checks rules in priority order:
    /// 1. `--when-exit-code` (specific code match)
    /// 2. `--except-exit-code` (all codes except one)
    /// 3. `--stderr-contains` (stderr content match)
    /// 4. `--stderr-lacks` (stderr content absence)
    /// 5. `--when-error` (catch-all replacement)
    /// 6. `--enrich-error-on` (specific code enrichment)
    /// 7. `--enrich-error` (catch-all enrichment)
    pub fn resolve(&self, code: i32, stderr: &str) -> ErrorHandlingOutcome {
        // 1. Specific exit code handlers
        for (target_code, replacement) in &self.when_exit_code {
            if code == *target_code {
                return ErrorHandlingOutcome::Replace(replacement.clone());
            }
        }

        // 2. Except-exit-code handlers
        for (except_code, replacement) in &self.except_exit_code {
            if code != *except_code {
                return ErrorHandlingOutcome::Replace(replacement.clone());
            }
        }

        // 3. Stderr-contains handlers
        for (find, replacement) in &self.stderr_contains {
            if stderr.contains(find.as_str()) {
                return ErrorHandlingOutcome::Replace(replacement.clone());
            }
        }

        // 4. Stderr-lacks handlers
        for (find, replacement) in &self.stderr_lacks {
            if !stderr.contains(find.as_str()) {
                return ErrorHandlingOutcome::Replace(replacement.clone());
            }
        }

        // 5. Catch-all replacement
        if let Some(ref replacement) = self.when_error {
            return ErrorHandlingOutcome::Replace(replacement.clone());
        }

        // 6. Specific code enrichment
        for (target_code, enrichment) in &self.enrich_error_on {
            if code == *target_code {
                return ErrorHandlingOutcome::Enrich(enrichment.clone());
            }
        }

        // 7. Catch-all enrichment
        if let Some(ref enrichment) = self.enrich_error {
            return ErrorHandlingOutcome::Enrich(enrichment.clone());
        }

        ErrorHandlingOutcome::Propagate
    }
}

/// What happens when a shell command exceeds its timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellTimeoutBehavior {
    /// Compose aborts with a shell-expansion error (default).
    #[default]
    Error,
    /// The shell result is replaced with an empty string and a warning is emitted.
    EmptyString,
}

/// Options for shell expansion behavior.
pub struct ShellExpansionOptions {
    pub timeout: std::time::Duration,
    pub timeout_behavior: ShellTimeoutBehavior,
    pub policy_root: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    /// Whether to strip ANSI escape codes and set `NO_COLOR=1`.
    pub strip_ansi: bool,
}

impl Clone for ShellExpansionOptions {
    fn clone(&self) -> Self {
        Self {
            timeout: self.timeout,
            timeout_behavior: self.timeout_behavior,
            policy_root: self.policy_root.clone(),
            working_directory: self.working_directory.clone(),
            approval_handler: self.approval_handler.clone(),
            strip_ansi: self.strip_ansi,
        }
    }
}

impl fmt::Debug for ShellExpansionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShellExpansionOptions")
            .field("timeout", &self.timeout)
            .field("timeout_behavior", &self.timeout_behavior)
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
            .field("strip_ansi", &self.strip_ansi)
            .finish()
    }
}

impl Default for ShellExpansionOptions {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(10),
            timeout_behavior: ShellTimeoutBehavior::Error,
            policy_root: None,
            working_directory: None,
            approval_handler: None,
            strip_ansi: true,
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
    pub source: ComposeSource,
    pub origin: ShellCommandOrigin,
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub normalized_exact: String,
    pub whitelist_path: PathBuf,
    pub blacklist_path: PathBuf,
    /// If the command was resolved from a shell alias, the original alias name.
    pub alias_name: Option<String>,
    /// When the request covers a chained command (e.g. `a && b`), this lists
    /// the unique executables in the chain in the order they first appear.
    /// For non-chained requests this is empty; the CLI prompt should fall
    /// back to `executable` in that case.
    pub chain_executables: Vec<String>,
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
    #[error("Shell directive parse error at {origin}: {message}")]
    ParseDirective {
        origin: ShellCommandOrigin,
        message: String,
    },

    #[error("Command not found: '{command}' at {origin}")]
    CommandNotFound {
        command: String,
        origin: ShellCommandOrigin,
    },

    #[error("Blacklisted command '{command}' at {origin}: {reason}")]
    Blacklisted {
        command: String,
        reason: String,
        origin: ShellCommandOrigin,
    },

    #[error("Approval required for '{command}' at {origin}")]
    ApprovalRequired {
        command: String,
        whitelist_path: PathBuf,
        blacklist_path: PathBuf,
        origin: ShellCommandOrigin,
    },

    #[error("Command denied: '{command}' at {origin}")]
    Denied {
        command: String,
        origin: ShellCommandOrigin,
    },

    #[error(
        "Command '{command}' at {origin} was not pre-approved{source_desc}. \
         This is a bug in the pre-flight scanner -- please report it."
    )]
    NotPreApproved {
        command: String,
        origin: ShellCommandOrigin,
        source_desc: String,
    },

    #[error("Command timed out after {timeout:?}: '{command}' at {origin}")]
    Timeout {
        command: String,
        timeout: std::time::Duration,
        origin: ShellCommandOrigin,
    },

    #[error("Command failed (exit {code}): '{command}' at {origin}")]
    ExecutionFailed {
        command: String,
        code: i32,
        stdout: String,
        stderr: String,
        origin: ShellCommandOrigin,
    },

    #[error("Policy I/O error for {path}: {source}")]
    PolicyIo {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl biscuit_terminal::errors::BlockError for ShellExpansionError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            ShellExpansionError::ParseDirective { origin, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "ShellExpansionError",
                    "directive parse failed",
                ))
                .body(format!(
                    "<dim>Origin:</dim> {origin}\n<dim>Message:</dim> {message}"
                ))
                .hint("Body syntax: <cyan>::shell \"command\"</cyan>. Frontmatter syntax: <cyan>key: $(command)</cyan>."),

            ShellExpansionError::CommandNotFound { command, origin } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellExpansionError", "command not found"))
                .body(format!(
                    "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}"
                ))
                .hint("Install the binary or update <cyan>$PATH</cyan> so it is discoverable."),

            ShellExpansionError::Blacklisted { command, reason, origin } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellExpansionError", "command blacklisted"))
                .body(format!(
                    "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}\n<dim>Reason:</dim> {reason}"
                ))
                .hint("Remove the entry from your blacklist file if you trust this command."),

            ShellExpansionError::ApprovalRequired {
                command,
                whitelist_path,
                blacklist_path,
                origin,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellExpansionError", "approval required"))
                .body(format!(
                    "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}\n<dim>Whitelist:</dim> {}\n<dim>Blacklist:</dim> {}",
                    whitelist_path.display(),
                    blacklist_path.display()
                ))
                .hint("Re-run with <cyan>--approve-shell</cyan> or add the command to your whitelist."),

            ShellExpansionError::Denied { command, origin } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellExpansionError", "command denied"))
                .body(format!(
                    "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}"
                ))
                .hint("The user declined to approve this shell expansion."),

            ShellExpansionError::NotPreApproved { command, origin, source_desc } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "ShellExpansionError",
                    "command not pre-approved",
                ))
                .body(format!(
                    "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}\n<dim>Source:</dim> {source_desc}"
                ))
                .hint("This is a bug in the pre-flight scanner — please report it."),

            ShellExpansionError::Timeout { command, timeout, origin } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellExpansionError", "command timed out"))
                .body(format!(
                    "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}\n<dim>Timeout:</dim> {timeout:?}"
                ))
                .hint("Raise the timeout, or pass <cyan>--allow-shell-timeout</cyan> to warn instead of fail."),

            ShellExpansionError::ExecutionFailed {
                command,
                code,
                stdout,
                stderr,
                origin,
            } => {
                let stdout_section = if stdout.trim().is_empty() {
                    String::new()
                } else {
                    format!("\n<dim>stdout:</dim>\n{}", truncate_output(stdout))
                };
                let stderr_section = if stderr.trim().is_empty() {
                    String::new()
                } else {
                    format!("\n<dim>stderr:</dim>\n{}", truncate_output(stderr))
                };
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ShellExpansionError", "execution failed"))
                    .body(format!(
                        "<dim>Command:</dim> <cyan>{command}</cyan>\n<dim>Origin:</dim> {origin}\n<dim>Exit code:</dim> {code}{stdout_section}{stderr_section}"
                    ))
                    .hint("Run the command directly to reproduce the failure.")
            }

            ShellExpansionError::PolicyIo { path, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellExpansionError", "policy I/O error"))
                .body(format!(
                    "<dim>Path:</dim> {}\n<dim>Kind:</dim> {:?}\n{source}",
                    path.display(),
                    source.kind()
                ))
                .hint("Verify the policy file exists and the process can read it."),
        }
    }
}

/// Truncate output to a maximum number of lines/bytes for block rendering.
fn truncate_output(text: &str) -> String {
    const MAX_LINES: usize = 20;
    const MAX_BYTES: usize = 2048;

    let truncated_bytes = if text.len() > MAX_BYTES {
        let cut = text
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= MAX_BYTES)
            .last()
            .unwrap_or(0);
        format!("{}\n<dim>… output truncated</dim>", &text[..cut])
    } else {
        text.to_string()
    };

    let lines: Vec<&str> = truncated_bytes.lines().collect();
    if lines.len() > MAX_LINES {
        let head = lines[..MAX_LINES].join("\n");
        format!(
            "{head}\n<dim>… {} more lines</dim>",
            lines.len() - MAX_LINES
        )
    } else {
        truncated_bytes
    }
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
#[derive(Debug, Clone)]
pub struct ShellExpansionRuntime {
    shared: Arc<std::sync::Mutex<SharedShellExpansionRuntime>>,
    pub approvals_used: usize,
}

#[derive(Debug, Clone, Default)]
struct SharedShellExpansionRuntime {
    allow_once: HashSet<String>,
    pending_allow_once: HashSet<String>,
    whitelist: ShellRuleSet,
    user_blacklist: ShellRuleSet,
    policy_paths: Option<ShellPolicyPaths>,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellRuntimeSnapshot {
    pub allow_once: HashSet<String>,
    pub whitelist: ShellRuleSet,
    pub user_blacklist: ShellRuleSet,
}

impl Default for ShellExpansionRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellExpansionRuntime {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(std::sync::Mutex::new(SharedShellExpansionRuntime::default())),
            approvals_used: 0,
        }
    }

    /// Creates a child runtime inheriting allow-once, whitelist, and
    /// blacklist state so that approvals persist across recursive transclusion.
    pub fn clone_for_child(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
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
        let mut shared = self.shared.lock().unwrap();
        if shared.policy_paths.is_some() {
            return Ok(());
        }
        shared.whitelist = super::store::load_ruleset(&paths.whitelist)?;
        shared.user_blacklist = super::store::load_ruleset(&paths.blacklist)?;
        shared.policy_paths = Some(paths.clone());
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> ShellRuntimeSnapshot {
        let shared = self.shared.lock().unwrap();
        ShellRuntimeSnapshot {
            allow_once: shared.allow_once.clone(),
            whitelist: shared.whitelist.clone(),
            user_blacklist: shared.user_blacklist.clone(),
        }
    }

    /// Attempts to reserve a command for allow-once approval.
    ///
    /// Returns `true` if the reservation succeeded (caller should proceed with
    /// the approval handler), or `false` if the command is already allowed or
    /// another thread is already handling it.
    pub(crate) fn try_reserve_allow_once(&mut self, normalized: &str) -> bool {
        let mut shared = self.shared.lock().unwrap();
        if shared.allow_once.contains(normalized) {
            return false;
        }
        shared.pending_allow_once.insert(normalized.to_string())
    }

    /// Completes an allow-once reservation.
    ///
    /// Removes the command from the pending set. If `approved` is `true`, also
    /// inserts it into the allow-once set.
    pub(crate) fn complete_allow_once(&mut self, normalized: &str, approved: bool) {
        let mut shared = self.shared.lock().unwrap();
        shared.pending_allow_once.remove(normalized);
        if approved {
            shared.allow_once.insert(normalized.to_string());
        }
    }

    pub(crate) fn persist_whitelist_exact(&mut self, normalized: String) {
        let mut shared = self.shared.lock().unwrap();
        shared
            .whitelist
            .entries
            .push(ShellRuleEntry::Exact(normalized));
    }

    pub(crate) fn persist_whitelist_prefix(&mut self, executable: String) {
        let mut shared = self.shared.lock().unwrap();
        shared
            .whitelist
            .entries
            .push(ShellRuleEntry::Prefix(executable));
    }

    pub(crate) fn persist_blacklist_exact(&mut self, normalized: String) {
        let mut shared = self.shared.lock().unwrap();
        shared
            .user_blacklist
            .entries
            .push(ShellRuleEntry::Exact(normalized));
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
    pub transclusion: crate::markdown::compose::transclusion::TransclusionRuntime,
    pub shell: ShellExpansionRuntime,
    pub cache: crate::markdown::compose::cache::RunLocalCache,
    dependencies: Vec<crate::markdown::compose::cache::types::DependencyRef>,
}

impl PipelineRuntime {
    pub fn new(
        max_depth: usize,
        cache_access_mode: crate::markdown::compose::cache::CacheAccessMode,
        cache_root: Option<std::path::PathBuf>,
    ) -> Self {
        let mut cache = crate::markdown::compose::cache::RunLocalCache::new(cache_access_mode);
        if let Some(root) = cache_root {
            cache = cache.with_persistent(root);
        }
        Self {
            transclusion: crate::markdown::compose::transclusion::TransclusionRuntime::new(
                max_depth,
            ),
            shell: ShellExpansionRuntime::new(),
            cache,
            dependencies: Vec::new(),
        }
    }

    /// Creates a child runtime sharing cycle detection and inheriting
    /// shell expansion state (whitelists, allow-once, blacklists).
    pub fn clone_for_child(&self) -> Self {
        Self {
            transclusion: self.transclusion.clone_for_child(),
            shell: self.shell.clone_for_child(),
            cache: self.cache.clone(),
            dependencies: Vec::new(),
        }
    }

    /// Merges a child runtime's stats back into this runtime.
    pub fn merge_child(&mut self, child: &Self) {
        self.transclusion.merge_child(&child.transclusion);
    }

    pub fn record_dependency(
        &mut self,
        dependency: crate::markdown::compose::cache::types::DependencyRef,
    ) {
        self.dependencies.push(dependency);
    }

    pub fn dependencies(&self) -> &[crate::markdown::compose::cache::types::DependencyRef] {
        &self.dependencies
    }

    pub fn load_markdown(&self, path: &std::path::Path) -> MarkdownResult<Markdown> {
        self.cache.load_markdown(path)
    }

    pub fn load_toc_headings(
        &self,
        path: &std::path::Path,
    ) -> std::io::Result<Vec<MarkdownTocNode>> {
        self.cache.load_toc_headings(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_when_exit_code_matches() {
        let handling = ErrorHandling {
            when_exit_code: vec![(42, "caught 42".to_string())],
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(42, ""),
            ErrorHandlingOutcome::Replace("caught 42".to_string())
        );
    }

    #[test]
    fn resolve_when_exit_code_no_match() {
        let handling = ErrorHandling {
            when_exit_code: vec![(42, "caught".to_string())],
            ..Default::default()
        };
        assert_eq!(handling.resolve(1, ""), ErrorHandlingOutcome::Propagate);
    }

    #[test]
    fn resolve_except_exit_code_catches_others() {
        let handling = ErrorHandling {
            except_exit_code: vec![(99, "caught".to_string())],
            ..Default::default()
        };
        // Code 1 is not 99, so it should be caught
        assert_eq!(
            handling.resolve(1, ""),
            ErrorHandlingOutcome::Replace("caught".to_string())
        );
        // Code 99 is the exception, so it should propagate
        assert_eq!(handling.resolve(99, ""), ErrorHandlingOutcome::Propagate);
    }

    #[test]
    fn resolve_stderr_contains_matches() {
        let handling = ErrorHandling {
            stderr_contains: vec![("warning".to_string(), "found warning".to_string())],
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(1, "warning: something bad"),
            ErrorHandlingOutcome::Replace("found warning".to_string())
        );
    }

    #[test]
    fn resolve_stderr_contains_no_match() {
        let handling = ErrorHandling {
            stderr_contains: vec![("fatal".to_string(), "found fatal".to_string())],
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(1, "warning: minor issue"),
            ErrorHandlingOutcome::Propagate
        );
    }

    #[test]
    fn resolve_stderr_lacks_matches_when_absent() {
        let handling = ErrorHandling {
            stderr_lacks: vec![("fatal".to_string(), "non-fatal".to_string())],
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(1, "error: minor"),
            ErrorHandlingOutcome::Replace("non-fatal".to_string())
        );
    }

    #[test]
    fn resolve_stderr_lacks_no_match_when_present() {
        let handling = ErrorHandling {
            stderr_lacks: vec![("fatal".to_string(), "non-fatal".to_string())],
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(1, "fatal error occurred"),
            ErrorHandlingOutcome::Propagate
        );
    }

    #[test]
    fn resolve_when_error_catches_all() {
        let handling = ErrorHandling {
            when_error: Some("fallback".to_string()),
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(1, "anything"),
            ErrorHandlingOutcome::Replace("fallback".to_string())
        );
        assert_eq!(
            handling.resolve(127, ""),
            ErrorHandlingOutcome::Replace("fallback".to_string())
        );
    }

    #[test]
    fn resolve_enrich_error_on_matches_code() {
        let handling = ErrorHandling {
            enrich_error_on: vec![(127, "command not found hint".to_string())],
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(127, ""),
            ErrorHandlingOutcome::Enrich("command not found hint".to_string())
        );
        assert_eq!(handling.resolve(1, ""), ErrorHandlingOutcome::Propagate);
    }

    #[test]
    fn resolve_enrich_error_catches_all() {
        let handling = ErrorHandling {
            enrich_error: Some("check your setup".to_string()),
            ..Default::default()
        };
        assert_eq!(
            handling.resolve(1, ""),
            ErrorHandlingOutcome::Enrich("check your setup".to_string())
        );
    }

    #[test]
    fn resolve_priority_exit_code_before_when_error() {
        let handling = ErrorHandling {
            when_exit_code: vec![(42, "specific".to_string())],
            when_error: Some("generic".to_string()),
            ..Default::default()
        };
        // Specific code match takes priority
        assert_eq!(
            handling.resolve(42, ""),
            ErrorHandlingOutcome::Replace("specific".to_string())
        );
        // Non-matching code falls through to when_error
        assert_eq!(
            handling.resolve(1, ""),
            ErrorHandlingOutcome::Replace("generic".to_string())
        );
    }

    #[test]
    fn resolve_priority_replace_before_enrich() {
        let handling = ErrorHandling {
            when_error: Some("replaced".to_string()),
            enrich_error: Some("enriched".to_string()),
            ..Default::default()
        };
        // Replace takes priority over enrich
        assert_eq!(
            handling.resolve(1, ""),
            ErrorHandlingOutcome::Replace("replaced".to_string())
        );
    }

    #[test]
    fn resolve_empty_handling_propagates() {
        let handling = ErrorHandling::default();
        assert_eq!(
            handling.resolve(1, "any stderr"),
            ErrorHandlingOutcome::Propagate
        );
    }

    #[test]
    fn is_empty_checks_all_fields() {
        assert!(ErrorHandling::default().is_empty());
        assert!(
            !ErrorHandling {
                when_error: Some("x".to_string()),
                ..Default::default()
            }
            .is_empty()
        );
        assert!(
            !ErrorHandling {
                when_exit_code: vec![(1, "x".to_string())],
                ..Default::default()
            }
            .is_empty()
        );
        assert!(
            !ErrorHandling {
                enrich_error: Some("x".to_string()),
                ..Default::default()
            }
            .is_empty()
        );
    }

    #[test]
    fn shell_timeout_behavior_default_is_error() {
        assert_eq!(ShellTimeoutBehavior::default(), ShellTimeoutBehavior::Error);
    }

    #[test]
    fn shell_expansion_options_default_timeout_behavior_is_error() {
        let opts = ShellExpansionOptions::default();
        assert_eq!(opts.timeout_behavior, ShellTimeoutBehavior::Error);
    }

    #[test]
    fn shell_command_origin_body_display() {
        let origin = ShellCommandOrigin::Body { line: 42 };
        assert_eq!(format!("{origin}"), "line 42");
    }

    #[test]
    fn shell_command_origin_frontmatter_display() {
        let origin = ShellCommandOrigin::Frontmatter {
            key: "files".to_string(),
        };
        assert_eq!(format!("{origin}"), "frontmatter.files");
    }

    #[test]
    fn shell_command_origin_shell_block_display() {
        let origin = ShellCommandOrigin::ShellBlock {
            start_line: 10,
            command_line: 12,
        };
        assert_eq!(
            format!("{origin}"),
            "shell block starting at line 10, command at line 12"
        );
    }

    #[test]
    fn shell_command_origin_line_number_body() {
        let origin = ShellCommandOrigin::Body { line: 42 };
        assert_eq!(origin.line_number(), 42);
    }

    #[test]
    fn shell_command_origin_line_number_frontmatter() {
        let origin = ShellCommandOrigin::Frontmatter {
            key: "x".to_string(),
        };
        assert_eq!(origin.line_number(), 0);
    }

    #[test]
    fn shell_command_origin_line_number_shell_block() {
        let origin = ShellCommandOrigin::ShellBlock {
            start_line: 10,
            command_line: 12,
        };
        assert_eq!(origin.line_number(), 12);
    }
}
