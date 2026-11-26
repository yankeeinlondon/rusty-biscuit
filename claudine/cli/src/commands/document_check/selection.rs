//! The provider and model checks, decided by the selectors a real run uses:
//! `wrap/composition/target.rs::decide_target` for `compose` and
//! `inline-compose`, and `wrap/sequence/providers.rs::select_traced` for a
//! sequence task. Each selector records the sources it consulted, which the
//! provider finding lists as its chain. Nothing here asks, refreshes a
//! catalog, or starts a process: the installed providers come from the
//! read-only `PATH` snapshot, and the configuration from the passive load.

use claudine::composition::sequence::preflight::TaskIdentity;
use claudine::composition::sequence::preflight::providers::ProviderPrediction;
use claudine::composition::{
    AgentResolutionState, EffectiveSelectionHints, InstalledProviderSnapshot, ResolvedExecutionTarget,
    SequenceTaskProviderProblem,
};
use claudine::model_catalog::ModelCatalogService;

use super::*;
use crate::commands::wrap::composition::{ModelResolveMode, TargetDecision, TargetInputs, decide_target};
use crate::commands::wrap::selection_trace::{LinkState, SelectionTrace};
use crate::commands::wrap::sequence::providers::{
    self as sequence_providers, OwnHints, SelectionInputs, SelectionSite, TaskTarget,
};

/// Where a prompt task's own provider hints come from.
pub(super) enum TaskHints {
    /// `compose`/`inline-compose`: the document's frontmatter under the
    /// caller's setters.
    Document,
    /// A sequence `prompt:` task: the walk's prediction, the enclosing
    /// sequence documents' hints that name an `agent`, and every enclosing
    /// sequence document, nearest first.
    Task {
        prediction: Option<Box<ProviderPrediction>>,
        enclosing: Vec<(PathBuf, EffectiveSelectionHints)>,
        consulted: Vec<PathBuf>,
    },
    /// A sequence step that composes its document's body: the nearest
    /// sequence document's hints, when it names an `agent`.
    Body { document: Option<(PathBuf, EffectiveSelectionHints)> },
}

/// The selection observations every task shares: built once per check.
pub(super) struct SelectionEnv {
    snapshot: InstalledProviderSnapshot,
    catalog: ModelCatalogService,
    favorite: Option<Provider>,
}

impl SelectionEnv {
    pub(super) fn observe(excluded: &std::collections::BTreeSet<Provider>, config: Option<&claudine::config::claudine_config::ClaudineConfig>) -> Self {
        let clients = sniff::programs::InstalledAiClients::new();
        let installed = composition::detect_installed_providers(&clients);
        let snapshot = composition::build_installed_snapshot(&installed, excluded, &clients);
        let catalog = match config {
            Some(config) => ModelCatalogService::with_overrides(config.models.clone()),
            None => ModelCatalogService::new(),
        };
        Self { snapshot, catalog, favorite: config.and_then(|config| config.preferred_agent) }
    }
}

/// How a task's provider was decided.
enum Selected {
    /// The provider it launches.
    Provider(ResolvedExecutionTarget),
    /// Decided when the task runs.
    AtLaunch,
    /// No provider: the finding is recorded.
    None,
}

impl Check {
    /// Decides the task's provider as its real run would, recording a
    /// finding when that run would ask or refuse; returns the provider when
    /// it is known.
    pub(super) fn select_provider(&mut self, task: &PromptTask<'_>) -> Option<ResolvedExecutionTarget> {
        let selected = match &task.selection {
            TaskHints::Document => self.select_document(task),
            TaskHints::Task { .. } | TaskHints::Body { .. } => self.select_sequence_task(task),
        };
        match selected {
            Selected::Provider(target) => Some(target),
            Selected::AtLaunch => {
                self.sink.not_checked(task.subject, "provider", "the task's `agent` is decided when it runs", None);
                None
            }
            Selected::None => None,
        }
    }

    fn select_document(&mut self, task: &PromptTask<'_>) -> Selected {
        let Some(hints) = self.document_hints(task) else { return Selected::None };
        let environment = self.environment();
        self.selection_env();
        let env = self.selection_env.as_ref().expect("observed above");
        let inputs = TargetInputs {
            hints: &hints,
            snapshot: &env.snapshot,
            explicit_provider: self.shared.explicit_provider(),
            favorite: env.favorite,
            cli_model: self.shared.model.as_deref(),
            catalog: &env.catalog,
            source_path: &task.prompt.resolved_path,
            is_tty: self.stderr_is_tty,
            mode: ModelResolveMode::PASSIVE,
            environment: &environment,
        };
        let mut trace = SelectionTrace::default();
        let decision = decide_target(&inputs, &mut trace);
        let favorite = env.favorite;
        match decision {
            TargetDecision::Chosen(target) => self.installed(task, target, &trace),
            TargetDecision::Picker(state) => {
                let picker = match favorite {
                    Some(favorite) => format!("; the picker defaults to the configured `preferred_agent` ({favorite})"),
                    None => String::new(),
                };
                self.push_unresolved(task, &state, &trace, Severity::Warning, &format!("an interactive run asks you to choose{picker}"));
                Selected::None
            }
            TargetDecision::Refused(_) => {
                let state = classify_after_trace(&hints, &self.selection_env().snapshot);
                self.push_unresolved(task, &state, &trace, Severity::Error, "a non-interactive run fails here");
                Selected::None
            }
        }
    }

    fn select_sequence_task(&mut self, task: &PromptTask<'_>) -> Selected {
        let own = match &task.selection {
            TaskHints::Task { prediction: Some(prediction), .. } => OwnHints::Task(prediction.as_ref().clone()),
            TaskHints::Task { prediction: None, .. } => {
                self.sink.not_checked(task.subject, "provider", "the task's `agent` could not be predicted", None);
                return Selected::None;
            }
            TaskHints::Body { document } => OwnHints::Body(document.clone()),
            TaskHints::Document => unreachable!("a document has no sequence site"),
        };
        let (enclosing, consulted) = match &task.selection {
            TaskHints::Task { enclosing, consulted, .. } => (enclosing.clone(), consulted.clone()),
            _ => (Vec::new(), Vec::new()),
        };
        let site = SelectionSite {
            identity: TaskIdentity::default(),
            step: 0,
            name: task.label.clone(),
            label: task.label.clone(),
            task_path: task.task_path.clone(),
            prompt: Some(task.prompt.resolved_path.clone()),
            containers: Vec::new(),
            own,
            enclosing,
            consulted,
        };
        let environment = self.environment();
        self.selection_env();
        let env = self.selection_env.as_ref().expect("observed above");
        let inputs = SelectionInputs {
            snapshot: &env.snapshot,
            catalog: &env.catalog,
            cli_model: self.shared.model.as_deref(),
            explicit_provider: self.shared.explicit_provider(),
            favorite: env.favorite,
            environment: &environment,
            silent: true,
        };
        let (result, mut trace) = sequence_providers::select_traced(&site, &inputs);
        if task.step_agent.is_some() {
            trace.annotate("with.agent", "see `agent:` above");
        }
        match result {
            Ok(TaskTarget::Own(target) | TaskTarget::Fallback(target)) => self.installed(task, target, &trace),
            Ok(TaskTarget::AtLaunch(_)) => Selected::AtLaunch,
            Ok(TaskTarget::Unresolved { state, .. }) => {
                // A sequence reviews every unresolved task on a terminal
                // (`review::needs_review`) and fails without one.
                if self.stderr_is_tty {
                    self.push_unresolved(task, &state, &trace, Severity::Warning, "an interactive run asks you to choose");
                } else {
                    self.push_unresolved(task, &state, &trace, Severity::Error, "a non-interactive run fails here");
                }
                Selected::None
            }
            Err(error) => {
                // An unusable own hint fails the real run before any task
                // starts, on a terminal too.
                if let CompositionError::SequenceTaskProviderSelection { problem, .. } = &error
                    && let SequenceTaskProviderProblem::Unresolvable { state, .. } = problem.as_ref()
                {
                    self.push_unresolved(task, state, &trace, Severity::Error, "the run fails before any task starts");
                } else {
                    let code = claudine::diagnostics::as_diagnostic(&error).map_or(INTERNAL_BUG, |known| known.code());
                    let mut finding = Diagnostic::projected(code, Severity::Error, error.to_string(), None)
                        .with_trail(task.trail.clone());
                    for line in trace.lines() {
                        finding = finding.with_note(line);
                    }
                    self.sink.push(task.subject, finding);
                }
                Selected::None
            }
        }
    }

    /// `target` when its program is on `PATH`. A provider flag names a
    /// provider without classifying it, so a real run finds it missing only
    /// when it launches; the check reports it here, as an error.
    fn installed(&mut self, task: &PromptTask<'_>, target: ResolvedExecutionTarget, trace: &SelectionTrace) -> Selected {
        let runnable = self.selection_env.as_ref().is_some_and(|env| env.snapshot.runnable.contains(&target.provider));
        if runnable {
            return Selected::Provider(target);
        }
        let provider = target.provider;
        let source = trace
            .links()
            .iter()
            .find(|link| matches!(link.state, LinkState::Chose(_)))
            .map_or_else(|| "--provider flag".to_string(), |link| link.source.clone());
        let binary = provider_info(provider).binary;
        let mut finding = Diagnostic::actionable(
            PROVIDER_NOT_INSTALLED,
            Severity::Error,
            format!("the {provider} provider ({source}) is not installed: `{binary}` is not on PATH; the run fails when it launches"),
            format!("install {provider}, or choose another provider"),
        )
        .with_trail(task.trail.clone());
        for line in trace.lines() {
            finding = finding.with_note(line);
        }
        self.sink.push(task.subject, finding);
        Selected::None
    }

    /// The finding for a task no source chose a provider for: an unknown
    /// name, a provider not installed, or none at all.
    fn push_unresolved(
        &mut self,
        task: &PromptTask<'_>,
        state: &AgentResolutionState,
        trace: &SelectionTrace,
        severity: Severity,
        consequence: &str,
    ) {
        let noun = if self.mode == Mode::Sequence { "task" } else { "document" };
        let decisive = trace.links().iter().rev().find_map(|link| match &link.state {
            LinkState::Unusable { value, .. } => Some((link.source.clone(), value.clone())),
            _ => None,
        });
        let prompt_name = file_name(&task.prompt.resolved_path);
        let (code, message, hint) = match (state, &decisive) {
            (AgentResolutionState::SingleInvalid { hint: name }, Some((source, _))) => {
                let hint = match Provider::fuzzy_match_cli_name(name) {
                    Some(near) => format!("did you mean `{near}`?"),
                    None => format!(
                        "use one of: {}",
                        claudine::provider_id::PROVIDERS_DISPLAY_ORDER
                            .iter()
                            .map(|provider| provider.as_slug().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                };
                (PROVIDER_UNKNOWN, format!("`{name}` ({source}) is not a known provider; {consequence}"), hint)
            }
            (AgentResolutionState::SingleNotInstalled { provider }, Some((source, _))) => {
                let binary = provider_info(*provider).binary;
                (
                    PROVIDER_NOT_INSTALLED,
                    format!("the {provider} provider ({source}) is not installed: `{binary}` is not on PATH; {consequence}"),
                    format!("install {provider}, or choose another provider"),
                )
            }
            (AgentResolutionState::ZeroInstalledList { .. }, Some((source, _))) => (
                PROVIDER_NOT_INSTALLED,
                format!("no provider listed in {source} is installed; {consequence}"),
                "install one of the listed providers, or choose another provider".to_string(),
            ),
            (AgentResolutionState::ListMultipleInstalled { .. }, Some((source, _))) => (
                PROVIDER_UNRESOLVED,
                format!("more than one provider listed in {source} is installed; {consequence}"),
                format!("run with `--provider <provider>`, or list one installed provider in {prompt_name}"),
            ),
            _ => {
                let hint = match &task.step_agent {
                    Some(written) => format!("add `with: {{ agent: {written} }}`, or run with `--provider {written}`"),
                    None => format!("add `agent: <provider>` to {prompt_name}, or run with `--provider <provider>`"),
                };
                (PROVIDER_UNRESOLVED, format!("no provider for this {noun}; {consequence}"), hint)
            }
        };
        let mut finding = Diagnostic::actionable(code, severity, message, hint).with_trail(task.trail.clone());
        for line in trace.lines() {
            finding = finding.with_note(line);
        }
        self.sink.push(task.subject, finding);
        self.sink.not_checked(task.subject, "model requirement", "no provider", None);
    }

    /// Whether the chosen provider needs a model this launch cannot find,
    /// by the precedence `exec_prep::resolve_model_and_validate` applies: the
    /// resolved model (the `-m` flag, the provider's variables, `MODEL`, the
    /// hints), then the provider's own variables and configured default.
    pub(super) fn check_model(&mut self, task: &PromptTask<'_>, target: &ResolvedExecutionTarget) {
        let provider = target.provider;
        let info = provider_info(provider);
        if !(self.non_interactive(task) && info.model_required_in_non_tty) {
            return;
        }
        let environment = self.environment();
        let found = target.model.is_some()
            || crate::commands::wrap::profile::profile_for_provider(provider).is_some_and(|profile| {
                crate::commands::wrap::profile::resolve_model_source(
                    info.model_env_vars,
                    |var| environment.lookup(var),
                    || profile.configured_default_model(),
                )
                .is_some()
            })
            || self.shared.provider_tail.launch_args().iter().any(|arg| arg == "--model" || arg == "-m" || arg.starts_with("--model="));
        if found {
            return;
        }
        let variables = info.model_env_vars.iter().map(|var| format!("`{var}`")).collect::<Vec<_>>();
        let hint = if variables.is_empty() {
            "pass `-m <model>`, or add `model:` to the prompt's frontmatter".to_string()
        } else {
            format!("pass `-m <model>`, set {}, or add `model:` to the prompt's frontmatter", variables.join(" or "))
        };
        self.sink.push(
            task.subject,
            Diagnostic::actionable(
                MODEL_REQUIRED,
                Severity::Error,
                format!("{provider} needs a model in a non-interactive run, and none was found"),
                hint,
            )
            .with_trail(task.trail.clone()),
        );
    }

    /// Whether the task launches non-interactively: always in a sequence;
    /// for a document, as `SharedComposeArgs::resolve_session_interactivity`
    /// decides from the flags and the document's `interactive`.
    fn non_interactive(&self, task: &PromptTask<'_>) -> bool {
        if self.mode == Mode::Sequence {
            return true;
        }
        let interactive = task.prompt.markdown.frontmatter().as_map().get("interactive")
            .map(composition::parse_interactive_hint)
            .and_then(Result::ok)
            .flatten();
        !self.shared.resolve_session_interactivity(interactive).value
    }

    /// The document's selection hints under the caller's setters, as a real
    /// run reads them; a hint of the wrong type is recorded.
    fn document_hints(&mut self, task: &PromptTask<'_>) -> Option<EffectiveSelectionHints> {
        let mut frontmatter = task.prompt.markdown.frontmatter().clone();
        for key in ["agent", "model", "interactive"] {
            if let Some(value) = self.setters.get(key) {
                let _ = frontmatter.insert(key, value.clone());
            }
        }
        match composition::parse_selection_hints_from_frontmatter(&frontmatter) {
            Ok(hints) => Some(hints),
            Err(error) => {
                self.push_error(task.subject, &error, task.trail.clone());
                self.sink.not_checked(task.subject, "model requirement", "no provider", None);
                None
            }
        }
    }

    fn environment(&self) -> claudine::invocation_context::DocumentEnvironment {
        self.invocation.as_ref().map(|invocation| invocation.document_environment().clone()).unwrap_or_default()
    }

    fn selection_env(&mut self) -> &SelectionEnv {
        if self.selection_env.is_none() {
            let excluded = self.shared.excluded();
            self.selection_env = Some(SelectionEnv::observe(&excluded, self.config.as_ref()));
        }
        self.selection_env.as_ref().expect("built above")
    }
}

/// The classified state of `hints`, for a refusal the resolver returned.
fn classify_after_trace(hints: &EffectiveSelectionHints, snapshot: &InstalledProviderSnapshot) -> AgentResolutionState {
    composition::classify_agent_resolution(hints, snapshot)
}
