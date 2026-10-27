//! The `md` host: Darkmatter's core-only lifecycle profile, its seams, and the
//! request/reply driver `md compose` runs a document's lifecycle through.
//!
//! `md` registers no event of its own and no outcome pair: it fires only
//! Darkmatter's `initialize`, `composed`, `failed`, `cancelled`, and
//! `finalize`, under those names. Any other key (`start`, `success`, `loop`,
//! …) is ordinary frontmatter data. The profile parses tolerantly: a verb
//! `md` provides no service for ([`UNPROVIDED_VERBS`], or a verb outside the
//! inventory) is kept as an opaque [`IgnoredAction`](super::actions::IgnoredAction),
//! and the executor warns once for each occurrence it reaches. Everything
//! `md` does run (expressions, `shell`, the terminal communication verbs, and
//! `stop`/`skip`/`error`) parses and validates strictly.
//!
//! Nothing here runs unless a host calls [`drive`]: parsing, the shell audit,
//! and the seams are passive.
//!
//! ## The run
//!
//! ```mermaid
//! flowchart TD
//!     I[initialize] -->|skip| X[no document, exit 0]
//!     I -->|error| F[failed]
//!     I --> P[prepare: references, shell approval, composition]
//!     P -->|failure| F
//!     P --> C[composed]
//!     C --> W[write the document]
//!     C -->|error| Z
//!     W --> Z[finalize]
//!     F --> Z
//! ```
//!
//! A `composed` error or a failed write ends the run failed without `failed`:
//! the run's `failure` slot, which `md` does not expose, holds it. A
//! cancellation runs `cancelled` (when authored) and `finalize`.

use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use biscuit_terminal::errors::SourceContext;
use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::effects::EffectEngine;
use crate::markdown::schemas::EffectiveSchema;
use crate::markdown::{Frontmatter, Markdown};
use crate::markdown::compose::expression::{
    AuthoredMode, Availability, BindingError, BindingView, ExpressionError, ResolutionContext, RuntimeBinding,
    ScopeId, UnavailabilityReason, UnavailableBinding, is_reserved_namespace, prepare_value, static_variable_reads,
    validate_prepared,
};
use crate::markdown::compose::preflight::ExtraShellSources;
use crate::markdown::compose::shell_expansion::executor::{ShellStatus, execute_directive_outcome};
use crate::markdown::compose::shell_expansion::store::{load_ruleset, resolve_policy_paths};
use crate::markdown::compose::shell_expansion::template::{FilledChain, ShellPreApprovals};
use crate::markdown::compose::shell_expansion::tokenize::{parse_pipeline, tokenize};
use crate::markdown::compose::shell_expansion::types::{
    CommandAction, ErrorHandling, PipelineAction, ShellPipeline,
};
use crate::markdown::compose::shell_expansion::{
    ShellCommandEntry, ShellCommandOrigin, ShellDirective, ShellExpansionError, ShellRuleSet, check_filled_chain,
    normalize_command,
};
use crate::markdown::compose::{ComposeContext, ComposeRequest, ContextBuildError};

use super::context::{LifecycleErrorInfo, LifecycleTiming};
use super::envelope::{DiagnosticCatalog, DiagnosticSnapshot};
use super::error::LifecycleError;
use super::executor::{
    LifecycleEventOutcome, LifecycleHost, LifecycleRunFailure, PrepError, PrepRequest, ShellRunError, ShellRunner,
    StackControl, StackExecutor, StackInputs,
};
use super::effects::LifecycleEffects;
use super::runtime::EvaluationMode;
use super::preflight::{LifecycleShellSites, PreflightError, ShellAuditBindings, collect_lifecycle_shell_audit};
use super::registry::EventRegistry;
use super::run::{
    ApprovalEpoch, AuthoredEvents, EntryReason, FailureStage, Finish, HostFailure, OperationReport, Preparation,
    RecoveryReply, Reply, RequestKind, Run, RunError, RunSettings, SessionAvailability, ShellRefusal,
};
use super::run::CompletionVerdict;
use super::{LifecycleConfig, LifecycleParseOptions, LifecycleSignal, UnknownVerbPolicy};

#[cfg(test)]
mod tests;

/// The verbs of the lifecycle inventory `md` has no service for: audio,
/// desktop notification, and every recovery and loop directive.
pub const UNPROVIDED_VERBS: &[&str] =
    &["say", "speak", "effect", "notify", "retry", "resume", "proxy", "prep", "break", "defer"];

/// The roots a lifecycle shell cannot read at preflight, because their value
/// exists only once an event fires.
const LATE_ROOTS: [&str; 3] = ["err", "timing", "current"];

/// The reason a late root is unavailable to the shell audit.
const PREFLIGHT_UNAVAILABLE: &str = "lifecycle.resolved-at-preflight";

/// `md`'s event registry: Darkmatter's five events under their own names.
pub fn registry() -> &'static Arc<EventRegistry> {
    static REGISTRY: OnceLock<Arc<EventRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Arc::new(EventRegistry::md()))
}

/// The parse options of the `md` profile.
pub fn parse_options(authored: Option<&Frontmatter>) -> LifecycleParseOptions<'_> {
    LifecycleParseOptions {
        // `effect` is unprovided, so no effect name is ever checked.
        is_known_effect: &|_| false,
        authored,
        unknown_verbs: UnknownVerbPolicy::IgnoreUnprovided(UNPROVIDED_VERBS.into()),
        registry: registry(),
    }
}

/// Whether `frontmatter` authors any of `md`'s event keys, which makes the
/// document's compose a lifecycle run.
pub fn authors_lifecycle(frontmatter: &Value) -> bool {
    frontmatter
        .as_object()
        .is_some_and(|map| registry().authored_event_keys().iter().any(|key| map.contains_key(*key)))
}

/// Parses and statically validates a root document's lifecycle under the
/// `md` profile, before anything runs.
///
/// ## Errors
///
/// The [`LifecycleError`] authoring family: a malformed stack or
/// `when`/`then`/`else`, an invalid operand of a provided verb, a provided
/// verb its event forbids (`shell` in `initialize`, `skip` outside it, any
/// flow verb but `stop` in `cancelled`), or a nested span in a literal.
pub fn parse_lifecycle(
    frontmatter: &Value,
    source_file: &Path,
    authored: Option<&Frontmatter>,
) -> Result<LifecycleConfig, LifecycleError> {
    let config = super::parse_lifecycle_config_with(frontmatter, source_file, &parse_options(authored))?;
    super::validate::validate_no_nested_spans_in_literals_in(frontmatter, &config, source_file, registry())?;
    Ok(config)
}

// ── Shell audit ──────────────────────────────────────────────────────────

/// `md`'s answers to the lifecycle shell walk: `err`, `timing`, and `current`
/// are late; everything else reads its preflight value.
#[derive(Debug, Default, Clone, Copy)]
pub struct MdShellAuditBindings;

impl MdShellAuditBindings {
    fn view() -> Arc<BindingView> {
        static VIEW: OnceLock<Arc<BindingView>> = OnceLock::new();
        VIEW.get_or_init(|| {
            Arc::new(
                BindingView::builder(ScopeId::new("md.lifecycle-shell-approval"))
                    .declare("err", "The failure the run is handling.", unavailable_at_preflight())
                    .declare("timing", "Observed durations.", unavailable_at_preflight())
                    .build()
                    .expect("two unreserved roots, each declared once"),
            )
        })
        .clone()
    }
}

fn unavailable_at_preflight() -> Availability {
    Availability::Unavailable(UnavailabilityReason::new(PREFLIGHT_UNAVAILABLE))
}

impl ShellAuditBindings for MdShellAuditBindings {
    fn approval_findings(&self, raw: &str) -> Vec<ExpressionError> {
        let view = Self::view();
        let Ok(prepared) = prepare_value(&Value::String(raw.to_string()), AuthoredMode::InterpolatedValue) else {
            return Vec::new();
        };
        let mut findings: Vec<ExpressionError> =
            validate_prepared(&prepared, &view).into_iter().map(|diagnostic| diagnostic.error).collect();
        for expression in prepared.expressions() {
            for read in static_variable_reads(expression.expr()) {
                // The view already refuses the roots it declares.
                if LATE_ROOTS.contains(&read.root()) && view.global(read.root()).is_none() {
                    findings.push(ExpressionError::Binding(Box::new(BindingError::Unavailable(Box::new(
                        UnavailableBinding {
                            root: read.root().to_string(),
                            path: read.path.to_string(),
                            scope: view.scope().clone(),
                            reason: UnavailabilityReason::new(PREFLIGHT_UNAVAILABLE),
                            span: None,
                        },
                    )))));
                }
            }
        }
        findings
    }

    fn approval_bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        let view = Self::view();
        let entries = view
            .globals()
            .map(|global| {
                (global.root().to_string(), RuntimeBinding::unavailable(UnavailabilityReason::new(PREFLIGHT_UNAVAILABLE)))
            })
            .collect();
        (view, entries)
    }

    fn task_setup_bindings(
        &self,
        _group: Option<&Map<String, Value>>,
        _seq: Option<&Value>,
    ) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        self.approval_bindings()
    }

    fn loop_pending(&self) -> bool {
        false
    }
}

/// Fixes every lifecycle shell site's bytes or typed template against
/// `effective_frontmatter`, in `request`'s captured context.
///
/// A placeholder is typed by `document`'s effective schema, as a body
/// `::shell` placeholder is, so a template-mode command refuses a value that
/// schema rejects when it runs.
///
/// ## Errors
///
/// [`PreflightError`] for a late, missing, or malformed placeholder.
pub fn stamp_lifecycle_shells(
    config: &mut LifecycleConfig,
    effective_frontmatter: &Value,
    document: &Markdown,
    request: &ComposeRequest,
    source_path: &Path,
) -> Result<(), PreflightError> {
    let owner_schema = super::shell_template::lifecycle_shell_has_placeholders(config)
        .then(|| document_schema(document, request))
        .flatten();
    let sites = LifecycleShellSites {
        effective_frontmatter,
        context: request.options().context(),
        source_path,
        registry: registry(),
        owner_schema: owner_schema.as_ref(),
        bindings: &MdShellAuditBindings,
        options: None,
    };
    sites.stamp(config, || Ok(request.local_expression_resolution_context()))
}

/// `document`'s effective schema under `request`, or `None` when it has none
/// or it does not resolve; composition reports that.
fn document_schema(document: &Markdown, request: &ComposeRequest) -> Option<EffectiveSchema> {
    crate::markdown::compose::schema_validation::prepare_schemas(document, request, &mut None)
        .ok()
        .and_then(|schemas| schemas.effective_for(document))
}

/// The lifecycle's shell commands and sites as extra sources of the
/// document's preflight approval, condition-blind across every event.
///
/// A command is one entry per chain segment, so its approval matches what
/// [`MdShellRunner`] checks before it runs. Text that does not tokenize is
/// one entry of its own bytes, which approval refuses like any other.
pub fn lifecycle_shell_sources(config: &LifecycleConfig, source_path: &Path) -> ExtraShellSources {
    let audit = collect_lifecycle_shell_audit(config, &LifecycleSignal::ALL, source_path);
    let mut entries = Vec::new();
    for (command, property) in audit.commands {
        let origin = ShellCommandOrigin::Frontmatter { key: property, line: None };
        match command_pipeline(&command, source_path) {
            Ok(pipeline) => {
                for action in &pipeline.actions {
                    entries.push(ShellCommandEntry {
                        raw_command: command.clone(),
                        executable: action.command.executable.clone(),
                        args: action.command.args.clone(),
                        normalized: normalize_command(&action.command.executable, &action.command.args),
                        source_file: source_path.to_path_buf(),
                        origin: origin.clone(),
                    });
                }
            }
            Err(_) => entries.push(ShellCommandEntry {
                raw_command: command.clone(),
                executable: command.clone(),
                args: Vec::new(),
                normalized: command.clone(),
                source_file: source_path.to_path_buf(),
                origin,
            }),
        }
    }
    ExtraShellSources { entries, candidates: audit.candidates }
}

/// Records on each lifecycle shell site the mode `approved` grants it, under
/// `request`'s shell policy root.
pub fn settle_lifecycle_shells(config: &LifecycleConfig, approved: &ShellPreApprovals, request: &ComposeRequest) {
    let policy_root = request.options().shell_options().policy_root;
    super::preflight::settle_lifecycle_shell_modes(config, &LifecycleSignal::ALL, approved, policy_root.as_deref());
}

fn command_pipeline(command: &str, source_path: &Path) -> Result<ShellPipeline, ShellExpansionError> {
    let ctx = source_context(command, source_path);
    let tokens = tokenize(command, &ctx)?;
    parse_pipeline(&tokens, &ctx)
}

fn source_context(command: &str, source_path: &Path) -> SourceContext {
    SourceContext::new(source_path.to_path_buf(), source_path.to_path_buf(), command.to_string())
}

// ── Execution seams ──────────────────────────────────────────────────────

/// `md`'s diagnostic catalog: it registers no codes, so every failure is
/// labeled by the action that raised it.
#[derive(Debug, Default, Clone, Copy)]
pub struct MdDiagnostics;

impl DiagnosticCatalog for MdDiagnostics {
    fn select(&self, _: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }

    fn snapshot_for_code(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }

    fn snapshot_for_error_kind(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }

    fn event_name(&self, event: LifecycleSignal) -> &str {
        md_event_name(event)
    }
}

/// `event`'s authored name under the `md` host.
fn md_event_name(event: LifecycleSignal) -> &'static str {
    registry().name_of(event).unwrap_or_else(|| event.property_name())
}

/// The `set` destinations `initialize` and later events committed, in commit
/// order; the first composition reads them as data.
#[derive(Debug, Default)]
pub struct MdRuntimeState(Mutex<IndexMap<String, Value>>);

impl MdRuntimeState {
    /// Every committed destination and its latest value.
    pub fn snapshot(&self) -> IndexMap<String, Value> {
        self.0.lock().expect("runtime state lock").clone()
    }
}

/// The executor's [`LifecycleHost`] for one `md` event.
pub struct MdStackHost<'a> {
    /// The event being evaluated.
    pub signal: LifecycleSignal,
    /// The error the event reads as `err`, where its view admits it.
    pub err: Option<&'a LifecycleErrorInfo>,
    /// The run's observed durations.
    pub timing: &'a LifecycleTiming,
    /// The run's one request; every expression resolves in its context.
    pub request: &'a ComposeRequest,
    /// Where `set` commits.
    pub state: &'a MdRuntimeState,
    /// The user blacklist a template-mode command also meets.
    pub user_blacklist: &'a ShellRuleSet,
    /// The cancellation probe.
    pub interrupted: &'a (dyn Fn() -> bool + Sync),
}

impl LifecycleHost for MdStackHost<'_> {
    fn registry(&self) -> &EventRegistry {
        registry()
    }

    fn bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        let view = registry()
            .id_of(self.signal)
            .map(|id| registry().globals(id).clone())
            .unwrap_or_else(|| Arc::new(BindingView::baseline()));
        let entries = view
            .globals()
            .map(|global| {
                let entry = match global.availability() {
                    Availability::Unavailable(reason) => RuntimeBinding::unavailable(reason.clone()),
                    Availability::Available | Availability::ExecutionDependent => RuntimeBinding::eager(match global.root() {
                        "err" => self.err.map_or(Value::Null, LifecycleErrorInfo::to_value),
                        "timing" => self.timing.to_value(),
                        _ => Value::Null,
                    }),
                };
                (global.root().to_string(), entry)
            })
            .collect();
        (view, entries)
    }

    fn name_coercion_keys(&self) -> Vec<String> {
        Vec::new()
    }

    fn resolution_context(
        &self,
        _source_path: &Path,
        _prepared_context: Option<&ComposeContext>,
        _file_resolution_context: &biscuit_file::FileResolutionContext,
        _file_ref_fallback_dir: Option<&Path>,
    ) -> Result<ResolutionContext, ContextBuildError> {
        Ok(self.request.local_expression_resolution_context())
    }

    fn capture_overlay_context(&self, _base: &Path, _content: &str) -> ComposeContext {
        // Only `proxy`/`retry`/`prep` overlays ask, and `md` ignores them.
        self.request.options().context().clone()
    }

    fn persisted_data(&self, value: &Value) -> Value {
        value.clone()
    }

    fn commit_set(
        &self,
        _engine: &EffectEngine,
        updates: &IndexMap<String, Value>,
        prior_base: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, Box<dyn StdError + Send + Sync>> {
        if let Some(key) = updates.keys().find(|key| is_reserved_namespace(key.split('.').next().unwrap_or(key))) {
            return Err(format!("`set` cannot write `{key}`, which is a reserved root").into());
        }
        let mut state = self.state.0.lock().expect("runtime state lock");
        let prior = updates
            .keys()
            .map(|key| (key.clone(), state.get(key).or_else(|| prior_base.get(key)).cloned().unwrap_or(Value::Null)))
            .collect();
        state.extend(updates.iter().map(|(key, value)| (key.clone(), value.clone())));
        Ok(prior)
    }

    fn in_loop(&self) -> bool {
        false
    }

    fn record_break(&self, _reason: Option<String>, _code: u8) {}

    fn prep(&self, _call: &PrepRequest<'_>) -> Result<(), PrepError> {
        Err(PrepError::Unavailable)
    }

    fn user_blacklist(&self, _policy_root: Option<&Path>) -> ShellRuleSet {
        self.user_blacklist.clone()
    }

    fn interrupted(&self) -> bool {
        (self.interrupted)()
    }

    /// An interrupt ends the normal work of `initialize` and `composed`; the
    /// cleanup events it leads to run whole.
    fn stops_on_interrupt(&self) -> bool {
        matches!(self.signal, LifecycleSignal::Initialize | LifecycleSignal::Start)
    }

    fn is_known_effect(&self, _name: &str) -> bool {
        false
    }

    fn failure(&self, failure: LifecycleRunFailure) -> LifecycleErrorInfo {
        let (verb, message) = failure_message(failure);
        LifecycleErrorInfo::from_action_failure_with(&MdDiagnostics, verb, message)
    }
}

/// The action label and message of a failure `md` classifies.
fn failure_message(failure: LifecycleRunFailure) -> (&'static str, String) {
    match failure {
        LifecycleRunFailure::UnknownEffect { event, name } => ("effect", format!("unknown effect `{name}` in `{event}`")),
        LifecycleRunFailure::ShellTemplateRefused { property, message, .. } => {
            ("shell", format!("`{property}` was refused: {message}"))
        }
        LifecycleRunFailure::EvaluationFailed { event, surface, message, property, .. } => (
            "set",
            match property {
                Some(property) => format!("`{property}` in `{event}` failed to evaluate {surface}: {message}"),
                None => format!("`{surface}` in `{event}` failed to evaluate: {message}"),
            },
        ),
        LifecycleRunFailure::BreakWithoutLoop { property, .. } => ("break", format!("`{property}` ran outside a loop")),
        LifecycleRunFailure::PrepUnavailable { property, .. } => ("prep", format!("`{property}` has no runner")),
        LifecycleRunFailure::PrepFailed { property, message, .. } => ("prep", format!("`{property}` failed: {message}")),
        LifecycleRunFailure::WithEvaluationFailed { property, path, message, .. } => {
            ("with", format!("`{property}.{path}` failed to evaluate: {message}"))
        }
        LifecycleRunFailure::CancellationRecovery { action, .. } => {
            ("cancelled", format!("cancellation cannot `{action}`"))
        }
    }
}

/// Where a lifecycle command's captured output goes.
pub trait CommandOutput: Sync {
    /// The command's standard output.
    fn stdout(&self, text: &str);
    /// The command's standard error.
    fn stderr(&self, text: &str);
}

/// Runs an approved lifecycle command for `md`.
///
/// Every chain segment must be an exact member of the approval set (a
/// template-mode site arrives filled and checked through
/// [`ShellRunner::run_structured`]), and the whole chain meets both
/// blacklists before its first segment starts. The command runs under the
/// document's shell options (timeout, working directory) with its output
/// captured and handed to [`CommandOutput`].
pub struct MdShellRunner<'a> {
    /// The run's answer for the event being evaluated.
    pub permission: Result<ApprovalEpoch, ShellRefusal>,
    /// The approval set preflight produced.
    pub approved: &'a ShellPreApprovals,
    /// The user blacklist.
    pub user_blacklist: &'a ShellRuleSet,
    /// The run's one request: shell options, source, and file context.
    pub request: &'a ComposeRequest,
    /// The root document.
    pub source_path: &'a Path,
    /// Where command output goes.
    pub output: &'a dyn CommandOutput,
}

impl MdShellRunner<'_> {
    fn origin(&self) -> ShellCommandOrigin {
        ShellCommandOrigin::Frontmatter { key: "lifecycle".to_string(), line: None }
    }

    fn execute(&self, raw: &str, pipeline: ShellPipeline) -> Result<i32, ShellRunError> {
        let first = pipeline.actions.first().map(|action| action.command.clone()).unwrap_or(CommandAction {
            executable: String::new(),
            args: Vec::new(),
            redirection: Default::default(),
        });
        let directive = ShellDirective {
            raw_command: raw.to_string(),
            executable: first.executable,
            args: first.args,
            span: 0..0,
            indent: String::new(),
            origin: self.origin(),
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            no_cache: true,
            pipeline: Some(pipeline),
            ctx: source_context(raw, self.source_path),
        };
        let options = self.request.options();
        let shell_options = options.shell_options();
        let outcome = execute_directive_outcome(
            &directive,
            &shell_options,
            options.source(),
            self.request.resolution_context(),
            None,
        )
        .map_err(|source| ShellRunError::Value { source })?;
        if !outcome.stdout.is_empty() {
            self.output.stdout(&outcome.stdout);
        }
        if !outcome.stderr.is_empty() {
            self.output.stderr(&outcome.stderr);
        }
        match outcome.status {
            ShellStatus::Exited(code) => Ok(code),
            ShellStatus::TimedOut => Err(ShellRunError::Value {
                source: ShellExpansionError::Timeout {
                    ctx: Box::new(directive.ctx.clone()),
                    command: raw.to_string(),
                    timeout: shell_options.timeout,
                    origin: self.origin(),
                },
            }),
            ShellStatus::Signaled => Err(ShellRunError::Spawn {
                command: raw.to_string(),
                source: std::io::Error::other("ended by a signal"),
            }),
        }
    }
}

impl ShellRunner for MdShellRunner<'_> {
    fn ensure_permitted(&self) -> Result<(), ShellRunError> {
        self.permission.map(|_| ()).map_err(|_| ShellRunError::BeforePreflight)
    }

    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.ensure_permitted()?;
        let pipeline = command_pipeline(command, self.source_path).map_err(|source| ShellRunError::Value { source })?;
        let ctx = source_context(command, self.source_path);
        for action in &pipeline.actions {
            let normalized = normalize_command(&action.command.executable, &action.command.args);
            if !self.approved.contains_exact(&normalized) {
                return Err(ShellRunError::Value {
                    source: ShellExpansionError::NotPreApproved {
                        ctx: Box::new(ctx),
                        command: command.to_string(),
                        origin: self.origin(),
                        source_desc: self.source_path.display().to_string(),
                    },
                });
            }
        }
        if let Some(reason) = check_filled_chain(&FilledChain::from_pipeline(&pipeline), self.user_blacklist) {
            return Err(ShellRunError::Value {
                source: ShellExpansionError::Blacklisted {
                    ctx: Box::new(ctx),
                    command: command.to_string(),
                    reason,
                    origin: self.origin(),
                },
            });
        }
        self.execute(command, pipeline)
    }

    fn run_structured(&self, chain: &FilledChain) -> Result<i32, ShellRunError> {
        self.ensure_permitted()?;
        let pipeline = ShellPipeline {
            actions: chain
                .segments
                .iter()
                .map(|segment| PipelineAction {
                    operator: segment.operator,
                    command: CommandAction {
                        executable: segment.program.clone(),
                        args: segment.args.clone(),
                        redirection: segment.redirection.clone(),
                    },
                })
                .collect(),
        };
        let raw = chain
            .segments
            .iter()
            .map(|segment| normalize_command(&segment.program, &segment.args))
            .collect::<Vec<_>>()
            .join(" && ");
        self.execute(&raw, pipeline)
    }

    fn run_values(
        &self,
        request: &super::executor::ShellValueRequest<'_>,
    ) -> Result<Vec<(String, Value)>, ShellRunError> {
        self.ensure_permitted()?;
        super::executor::execute_set_shell_values(request, None)
    }
}

/// Everything one `md` event's stack evaluates against.
pub struct MdStackContext<'a> {
    /// The run's one request.
    pub request: &'a ComposeRequest,
    /// The root document.
    pub source_path: &'a Path,
    /// The document's effective frontmatter, the base of every expression.
    pub frontmatter: &'a Map<String, Value>,
    /// The frontmatter as earlier events left it.
    pub live_frontmatter: &'a Mutex<Map<String, Value>>,
    /// Where `set` commits.
    pub state: &'a MdRuntimeState,
    /// The approval set preflight produced; empty before it.
    pub approved: &'a ShellPreApprovals,
    /// The user blacklist.
    pub user_blacklist: &'a ShellRuleSet,
    /// The side-effect engine.
    pub effect_engine: &'a EffectEngine,
    /// Terminal communication.
    pub effects: &'a dyn LifecycleEffects,
    /// Where lifecycle command output goes.
    pub output: &'a dyn CommandOutput,
    /// The run's observed durations.
    pub timing: &'a LifecycleTiming,
    /// The cancellation probe.
    pub interrupted: &'a (dyn Fn() -> bool + Sync),
}

/// Runs `event`'s stack of `config` in `context`.
pub fn evaluate(context: &MdStackContext<'_>, config: &LifecycleConfig, event: &MdEvent) -> LifecycleEventOutcome {
    let host = MdStackHost {
        signal: event.signal,
        err: event.error.as_ref(),
        timing: context.timing,
        request: context.request,
        state: context.state,
        user_blacklist: context.user_blacklist,
        interrupted: context.interrupted,
    };
    let shell_runner = MdShellRunner {
        permission: event.shells,
        approved: context.approved,
        user_blacklist: context.user_blacklist,
        request: context.request,
        source_path: context.source_path,
        output: context.output,
    };
    let options = context.request.options();
    let inputs = StackInputs {
        signal: event.signal,
        frontmatter: context.frontmatter,
        live_frontmatter: Some(context.live_frontmatter),
        current: Some(options.current_authority()),
        base_dir: context.source_path.parent(),
        ctx_base_dir: None,
        prepared_context: Some(options.context()),
        file_resolution_context: context.request.resolution_context(),
        effect_engine: context.effect_engine,
        shell_runner: &shell_runner,
        source_path: context.source_path,
        host: &host,
    };
    StackExecutor::with_services(inputs, context.effects, &MdDiagnostics).execute_event(config)
}

/// The user blacklist of `request`'s shell policy; an unreadable policy file
/// reads as empty here because preflight already refused it.
pub fn user_blacklist(request: &ComposeRequest) -> ShellRuleSet {
    let options = request.options();
    resolve_policy_paths(&options.shell_options(), options.source(), request.resolution_context())
        .and_then(|paths| load_ruleset(&paths.blacklist))
        .unwrap_or_default()
}

// ── The driver ───────────────────────────────────────────────────────────

/// One event the driver asks the host to evaluate.
#[derive(Debug, Clone)]
pub struct MdEvent {
    /// The engine slot.
    pub signal: LifecycleSignal,
    /// The event's `md` name.
    pub name: &'static str,
    /// The error the event reads as `err`.
    pub error: Option<LifecycleErrorInfo>,
    /// Whether a lifecycle shell may run now, and under which approval.
    pub shells: Result<ApprovalEpoch, ShellRefusal>,
}

/// What `md compose` does for a lifecycle run; [`drive`] calls it.
pub trait MdRunHost {
    /// Runs `event`'s stack.
    fn event(&mut self, event: MdEvent) -> LifecycleEventOutcome;

    /// References, shell approval, and composition, after `initialize`.
    ///
    /// ## Errors
    ///
    /// The failure `failed` reads as `err`.
    fn prepare(&mut self) -> Result<ApprovalEpoch, LifecycleErrorInfo>;

    /// Writes the composed document, after `composed`.
    ///
    /// ## Errors
    ///
    /// The write failure, which `finalize` reads as `err`.
    fn write(&mut self) -> Result<(), LifecycleErrorInfo>;

    /// Whether the user interrupted the run.
    fn interrupted(&self) -> bool;
}

/// How a driven run ended.
#[derive(Debug, Clone, PartialEq)]
pub struct MdRunEnd {
    #[allow(missing_docs)]
    pub finish: Finish,
    /// The run's selected error.
    pub error: Option<LifecycleErrorInfo>,
    /// The `md` events that ran, in order.
    pub fired: Vec<&'static str>,
    /// Whether the document was written.
    pub written: bool,
    /// Replies the run refused because a cancellation outdated their
    /// request; their work's outcome was dropped.
    pub late_replies: usize,
}

impl MdRunEnd {
    /// The process exit status: `0` for a written or skipped document, `130`
    /// for a cancellation, `1` otherwise.
    pub fn exit_code(&self) -> i32 {
        match self.finish {
            Finish::Succeeded | Finish::Skipped => 0,
            Finish::Cancelled => 130,
            Finish::Failed => 1,
        }
    }
}

/// The error a cancelled run selects.
pub fn cancellation_error() -> LifecycleErrorInfo {
    LifecycleErrorInfo::from_action_failure_with(&MdDiagnostics, "interrupt", "interrupted by the user")
}

/// Drives `config`'s lifecycle through a [`Run`], performing each request on
/// `host`.
///
/// The host is polled for an interrupt before and after every unit of work.
/// An interrupt cancels the run: work that was in flight keeps its result
/// out of the run (its reply is refused as outdated), no further normal
/// event or document write starts, and the cleanup the run asks for
/// (`cancelled` when authored, then `finalize`) is evaluated.
pub fn drive(host: &mut impl MdRunHost, config: &LifecycleConfig) -> MdRunEnd {
    let settings = RunSettings::new(Arc::new(MdDiagnostics)).with_cancellation_error(cancellation_error());
    let mut run = Run::start(settings, AuthoredEvents::from_config(config));
    let mut fired = Vec::new();
    let mut written = false;
    let mut late_replies = 0;
    // An `initialize` failure is the document's setup failure under `md`:
    // `failed` reads it, then `finalize`. The run reaches that route through
    // the preparation it would otherwise have asked for.
    let mut initialize_failure: Option<LifecycleErrorInfo> = None;
    loop {
        cancel_if_interrupted(&mut run, host);
        let request = run.pending();
        let ticket = request.ticket();
        let reply = match request.kind() {
            RequestKind::Done(finish) => {
                return MdRunEnd {
                    finish: *finish,
                    error: run.selected_error().and_then(RunError::reported).cloned(),
                    fired,
                    written,
                    late_replies,
                };
            }
            RequestKind::EvaluateEvent(event) => match registry().name_of(event.signal) {
                Some(name) => {
                    let name = core_name(name);
                    fired.push(name);
                    let mut outcome = host.event(MdEvent {
                        signal: event.signal,
                        name,
                        error: event.error.as_ref().and_then(RunError::reported).cloned(),
                        shells: run.shell_permission(&ticket),
                    });
                    if event.signal == LifecycleSignal::Initialize && event.mode == EvaluationMode::NormalRecovery {
                        initialize_failure = setup_failure(&outcome);
                        if initialize_failure.is_some() {
                            outcome = LifecycleEventOutcome::default();
                        }
                    }
                    Reply::Event(outcome)
                }
                // A slot `md` does not expose (the outcome pair, the loop
                // gate) has no stack.
                None => Reply::Event(LifecycleEventOutcome::default()),
            },
            RequestKind::Prepare { entry } => Reply::Prepared(match initialize_failure.take().map_or_else(|| host.prepare(), Err) {
                Ok(epoch) => Preparation::Ready { epoch },
                Err(error) => Preparation::Failed(HostFailure { stage: FailureStage::Preflight, entry: *entry, error }),
            }),
            RequestKind::PerformOperation { .. } | RequestKind::AwaitCompletion => {
                let verdict = match host.write() {
                    Ok(()) => {
                        written = true;
                        CompletionVerdict::Succeeded
                    }
                    Err(error) => CompletionVerdict::Failed(error),
                };
                Reply::Operation(OperationReport::Completed { verdict, session: SessionAvailability::Unavailable })
            }
            // `md` parses no recovery directive, so the run never asks.
            RequestKind::Recover(_) => Reply::Recovery(RecoveryReply::Aborted(HostFailure {
                stage: FailureStage::RecoveryRefused,
                entry: EntryReason::Initial,
                error: LifecycleErrorInfo::from_action_failure_with(&MdDiagnostics, "recover", "`md` runs no recovery"),
            })),
        };
        let cancelled = cancel_if_interrupted(&mut run, host);
        if run.reply(ticket, reply).is_err() && cancelled {
            late_replies += 1;
        }
    }
}

/// The error an `initialize` outcome fails the document with: an evaluation
/// error, an unsuppressed action error, or an explicit `error`.
fn setup_failure(outcome: &LifecycleEventOutcome) -> Option<LifecycleErrorInfo> {
    if let Some(error) = outcome.evaluation_error.as_ref().or(outcome.action_error.as_ref()) {
        return Some(error.clone());
    }
    match &outcome.control {
        Some(StackControl::Error { reason }) => Some(LifecycleErrorInfo::from_action_failure_with(
            &MdDiagnostics,
            "error",
            reason.clone().unwrap_or_else(|| "lifecycle initialize error".to_string()),
        )),
        _ => None,
    }
}

/// Cancels `run` the first time `host` reports an interrupt; `true` once the
/// run is cancelled.
fn cancel_if_interrupted(run: &mut Run, host: &impl MdRunHost) -> bool {
    host.interrupted() && run.finish().is_none() && {
        let _ = run.cancel();
        true
    }
}

/// The registry's names are `'static` for `md`, which renames nothing.
fn core_name(name: &str) -> &'static str {
    super::registry::CoreEvent::ALL
        .into_iter()
        .map(super::registry::CoreEvent::darkmatter_name)
        .find(|candidate| *candidate == name)
        .expect("md exposes only Darkmatter's events under their own names")
}
