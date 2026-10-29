//! The shell seam: the [`ShellRunner`] a host supplies to run approved
//! lifecycle commands, and the `set` value executor its default uses.

use std::path::Path;

use serde_json::Value;

use crate::markdown::compose::shell_expansion::template::{
    FilledChain, HoleEvaluator, ShellPreApprovals, TemplateShape, TemplateTargetError,
};
use crate::markdown::compose::shell_expansion::{ShellChildEnvironment, ShellExpansionError};
use crate::markdown::compose::{
    ComposeContext, ComposeOptions, ComposeRequest, ContextBuildError, ResolvedShellValue,
    execute_shell_values_with,
};

/// A shell command refused by the lifecycle boundary or unable to start.
/// A command that ran and exited nonzero is reported through
/// [`ShellRunner::run`]'s (or [`ShellRunner::run_structured`]'s) `Ok` arm.
/// Spawn failures retain their I/O source.
#[derive(Debug, thiserror::Error)]
pub enum ShellRunError {
    /// The lifecycle has not crossed the preflight boundary.
    #[error("shell commands are forbidden during initialize and before preflight completes; move the command to start or a later event")]
    BeforePreflight,
    /// A `set` value's command did not produce a value: it is missing,
    /// blacklisted, denied, not pre-approved, timed out, ended by a signal, or
    /// (without a result suffix) exited non-zero.
    #[error("{source}")]
    Value {
        /// The Darkmatter shell failure.
        #[source]
        source: ShellExpansionError,
    },
    /// The shell process could not be spawned or waited on.
    #[error("command `{command}` failed to run: {source}")]
    Spawn {
        /// The approved command that was attempted.
        command: String,
        /// The underlying spawn/wait failure.
        #[source]
        source: std::io::Error,
    },
    /// The request's file-resolution context failed validation, so no value
    /// could be evaluated against it.
    #[error("{source}")]
    Context {
        /// The builder's validation failure.
        #[source]
        source: ContextBuildError,
    },
    /// A template-mode program is not a native executable on the child's
    /// search path, so no segment of its command started.
    #[error("{reason}")]
    Target {
        /// The program word as authored.
        program: String,
        /// Why it cannot run without an interpreter.
        #[source]
        reason: Box<TemplateTargetError>,
    },
}

impl ShellRunError {
    /// [`Self::BeforePreflight`]'s message naming the event where shells
    /// unlock by `unlock_event`, the host's spelling of `composed`. The
    /// `Display` text names Claudine's `start`, which Claudine's recorded
    /// traces pin.
    pub fn before_preflight_message(unlock_event: &str) -> String {
        format!(
            "shell commands are forbidden during initialize and before preflight completes; move the command to {unlock_event} or a later event"
        )
    }
}

/// Abstraction over running an approved shell command.
///
/// Lifecycle shell actions are audited against the host's command policy
/// during pre-flight; this trait runs an already-approved command. Injectable
/// so tests can assert command dispatch without spawning real processes.
pub trait ShellRunner: Sync {
    /// Return the exit code, or [`ShellRunError`] when execution is prohibited
    /// or the process could not be started.
    fn run(&self, command: &str) -> Result<i32, ShellRunError>;

    /// Run a template-mode command whose every argument is already filled,
    /// validated, and checked against the blacklists, and return the exit code
    /// of the last segment that ran.
    ///
    /// Each program is started directly, never through a shell, under the
    /// same contract as [`Self::run`]: inherited standard streams, the same
    /// working directory and environment, and no deadline.
    ///
    /// ## Errors
    ///
    /// [`ShellRunError::BeforePreflight`] where shell execution is prohibited,
    /// [`ShellRunError::Target`] when a program is not a native executable
    /// (nothing starts then), or [`ShellRunError::Spawn`].
    fn run_structured(&self, chain: &FilledChain) -> Result<i32, ShellRunError>;

    /// Whether this runner may start a command at all.
    ///
    /// A template-mode site asks before it reads any placeholder, so a
    /// prohibited route reports the prohibition rather than a value error.
    fn ensure_permitted(&self) -> Result<(), ShellRunError> {
        Ok(())
    }

    /// Execute one `set` action's whole-value `$( … )` assignments and return
    /// each destination's value, in order.
    ///
    /// The default runs them through Darkmatter's frontmatter shell executor
    /// ([`execute_set_shell_values`]); a runner that prohibits shell execution
    /// overrides it.
    fn run_values(&self, request: &ShellValueRequest<'_>) -> Result<Vec<(String, Value)>, ShellRunError> {
        execute_set_shell_values(request, None)
    }

    /// This runner, starting its children from `environment` instead of this
    /// process's: the shell route of a document entered under its own map.
    ///
    /// `None` keeps `self`, which is right for a runner that starts no process
    /// of its own (a recorder, [`DisabledShellRunner`]).
    fn for_child_environment(&self, _environment: &ShellChildEnvironment) -> Option<Box<dyn ShellRunner>> {
        None
    }
}

/// One `set` action's shell assignments, ready to execute.
pub struct ShellValueRequest<'a> {
    /// The approved values, in authored order.
    pub values: &'a [&'a ResolvedShellValue],
    /// The pre-write state for conditions, value branches, and template-mode
    /// placeholders. Resolved-byte commands stay fixed at preflight.
    pub state: &'a crate::markdown::compose::SharedFrontmatter,
    /// The composition source, for diagnostics and file references.
    pub source_path: &'a Path,
    /// The early-binding `ctx.*`/`env.*` snapshot.
    pub context: ComposeContext,
    /// The request's file-resolution context.
    pub file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// Reads a template-mode value's placeholders against [`Self::state`]
    /// with the event's globals, exactly as the action's other values read.
    pub holes: Option<&'a dyn HoleEvaluator>,
    /// The shell policy root a template-mode value was approved under; its
    /// user blacklist judges the filled command.
    pub policy_root: Option<&'a Path>,
}

/// Runs a `set` action's shell values through Darkmatter in a fresh result
/// cache, so a command two values name runs once and nothing is reused from an
/// earlier event.
///
/// A resolved-byte value's commands are exactly the bytes preflight approved,
/// so they are its pre-approved set; a lifecycle `shell` action runs its
/// approved bytes on the same terms. A template-mode value
/// ([`ResolvedShellValue::template`]) is pre-approved by its shape instead:
/// its placeholders are read through [`ShellValueRequest::holes`], each fills
/// one argument, and the filled command meets both blacklists before it
/// starts. Commands run in the request's launch directory, as a
/// lifecycle `shell` action does, and from `child_environment` when one is
/// given (otherwise they inherit this process's).
///
/// ## Errors
///
/// [`ShellRunError::Value`] carrying the first value's Darkmatter failure, or
/// [`ShellRunError::Context`] when the request's context fails validation.
pub fn execute_set_shell_values(
    request: &ShellValueRequest<'_>,
    child_environment: Option<&ShellChildEnvironment>,
) -> Result<Vec<(String, Value)>, ShellRunError> {
    let mut approved = ShellPreApprovals::new();
    for value in request.values {
        match value.template() {
            Some(template) => {
                approved.insert_template(TemplateShape::from_template(template));
            }
            None => approved.extend_exact(value.commands()),
        }
    }
    let mut options = ComposeOptions::new_with_context(request.context.clone())
        .with_source_file(request.source_path)
        .with_pre_approved_commands(approved)
        .with_shell_working_directory(request.file_resolution_context.request_cwd());
    if let Some(policy_root) = request.policy_root {
        options = options.with_shell_policy_root(policy_root);
    }
    if let Some(environment) = child_environment {
        options = options.with_shell_child_environment(environment.clone());
    }
    let compose_request = compose_request(options, request.file_resolution_context.clone())
        .map_err(|source| ShellRunError::Context { source })?;
    // Cloning the view shares its base (an inline `sequence:` list included)
    // and copies only the values it owns.
    crate::note_work!(ShellValueStateNodes, crate::work_counters::value_nodes(request.state.owned()));
    let state = request.state.clone();
    execute_shell_values_with(request.values, state, &compose_request, request.holes)
        .map_err(|source| ShellRunError::Value { source })
}

/// Pairs `options` with the request's `file_resolution_context`.
///
/// Darkmatter aligns the options' `ctx.*` environment with the context's, so
/// the variables a host layers onto the captured `ctx` for a run are added to
/// the context's environment first. An expression, `env.*`, and a `{{VAR}}`
/// file reference then read one environment, and none of the run's variables
/// is lost.
///
/// ## Errors
///
/// Returns the [`ContextBuildError`] when `file_resolution_context` fails
/// validation.
pub fn compose_request(
    options: ComposeOptions,
    file_resolution_context: biscuit_file::FileResolutionContext,
) -> Result<ComposeRequest, ContextBuildError> {
    let layered = options.context().env();
    let context = if layered
        .iter()
        .all(|(key, value)| file_resolution_context.env().get(key) == Some(value))
    {
        file_resolution_context
    } else {
        let mut env = file_resolution_context.env().clone();
        env.extend(layered.iter().map(|(key, value)| (key.clone(), value.clone())));
        file_resolution_context.with_env(env)
    };
    ComposeRequest::with_context(options, context)
}

/// Runner for initialization and its early catch handlers. Approvals cannot
/// enable shell execution on this route.
#[derive(Debug, Default, Clone, Copy)]
pub struct DisabledShellRunner;

impl ShellRunner for DisabledShellRunner {
    fn run(&self, _command: &str) -> Result<i32, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }

    fn run_structured(&self, _chain: &FilledChain) -> Result<i32, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }

    fn ensure_permitted(&self) -> Result<(), ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }

    fn run_values(&self, _request: &ShellValueRequest<'_>) -> Result<Vec<(String, Value)>, ShellRunError> {
        Err(ShellRunError::BeforePreflight)
    }
}
