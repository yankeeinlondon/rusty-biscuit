//! Sequence Plus static preflight: the recursive task graph.
//!
//! Preflight answers one question before anything is launched: *what could this
//! sequence possibly execute?* It walks every step, expands every external
//! reference — `kind: task` files, `kind: group` files, `kind: group-catalog`
//! entries, and `prompt:` documents — and returns an immutable
//! [`PreflightGraph`] describing all potentially executable work.
//!
//! Three properties make the walk trustworthy:
//!
//! - **Condition-blind.** Conditional branches are traversed as *potential*
//!   work. A `setup:` action guarded by a `when:` that is false today still
//!   contributes its shell command, because the guard may read differently at
//!   its turn.
//! - **Origin-relative.** Every reference resolves from the directory of the
//!   document that authored it, never the process CWD and never the top-level
//!   sequence directory. The authoring directory travels with each node.
//! - **Approved == executed.** A task's shell bytes are resolved once here
//!   against an early-binding-only lookup and stored on the node. That resolved
//!   string is the only executable representation; nothing re-interpolates it
//!   later.
//!
//! Every failure raised here is abort-all: `fail_fast` governs *execution*
//! outcomes, and preflight happens before execution exists. `--dry-run` runs
//! this identical walk.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md` → *Phase 1 — Static
//! Preflight*.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::ResolutionContext;
use darkmatter::markdown::compose::subtree::SubtreeCompose;
use darkmatter::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};
use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::json_util::json_type_name;
use super::super::types::ResolvedCompositionSource;
use super::model::{ExecutableField, SequencePlan, StepExecutable};
use super::{reserved, source as source_resolution};

mod shape;

pub use shape::{GroupExecution, PreflightAction, PreflightGroup, PreflightStep, PreflightTask};

/// A shell command discovered during preflight, with the location that
/// contributed it. The `command` is the fully resolved byte string that will be
/// executed — never the authored template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCommand {
    /// The resolved command bytes.
    pub command: String,
    /// The document that authored it.
    pub source_file: PathBuf,
    /// A human label locating the command within that document.
    pub origin: String,
}

/// A `prompt:` document reached from the task graph.
#[derive(Debug, Clone)]
pub struct PromptDocument {
    /// Canonical path to the document.
    pub path: PathBuf,
    /// The parsed document, retained so approval and JIT composition never
    /// re-read it during this preflight.
    pub markdown: Markdown,
    /// `true` when the document's own frontmatter carries a string `prompt:`,
    /// which makes composing it an inline-compose run that rewrites its body.
    pub inline_compose: bool,
    /// `true` when the document declares a `$schema`, so its required-property
    /// gaps join this run's aggregated collection pass.
    pub has_schema: bool,
}

/// The complete statically-knowable picture of one sequence invocation.
#[derive(Debug, Clone, Default)]
pub struct PreflightGraph {
    /// One node per step, in sequence order.
    pub steps: Vec<PreflightStep>,
    /// Every prompt document reachable from the graph, deduplicated by
    /// canonical path in first-encounter order.
    pub prompt_documents: Vec<PromptDocument>,
    /// Every shell command reachable from the graph, in discovery order.
    pub shell_commands: Vec<DiscoveredCommand>,
}

impl PreflightGraph {
    /// The prompt document at `path`, if the graph reached one.
    pub fn prompt_document(&self, path: &Path) -> Option<&PromptDocument> {
        self.prompt_documents.iter().find(|d| d.path == path)
    }
}

/// Roots whose values do not exist when shell commands are approved.
///
/// `outputs` is preflight-specific: it is the accumulator later tasks push
/// onto, so no shell string may depend on it (spec → *Shell approval with
/// strict byte-parity*). The other three are the standard lifecycle
/// late-binding globals.
const SHELL_UNAVAILABLE_ROOTS: &[&str] = &["outputs", "err", "timing", "current"];

/// Reject invoking a non-sequence document kind directly.
///
/// `kind: group` documents in particular are never directly executable — a
/// group runs only as a sequence task (spec → *Groups*). Catching this at the
/// entry point produces an actionable error instead of "does not define a
/// `sequence` frontmatter property".
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceUnsupportedConstruct`] when the source's
/// `kind:` is `group`, `group-catalog`, or `task`.
pub fn reject_non_sequence_kind(source: &ResolvedCompositionSource) -> Result<(), CompositionError> {
    let Some(Value::String(kind)) = source.markdown.frontmatter().as_map().get("kind") else {
        return Ok(());
    };
    let detail = match kind.as_str() {
        "group" | "group-catalog" => {
            "a group executes only as a sequence task; reference it from a step's `group:`"
        }
        "task" => "a task executes only inside a sequence or a group; reference it with `task:`",
        _ => return Ok(()),
    };
    Err(CompositionError::SequenceUnsupportedConstruct {
        construct: format!("running a `kind: {kind}` document directly"),
        detail: detail.to_string(),
    })
}

/// Walk `plan` and build the complete preflight graph.
///
/// ## Errors
///
/// Every construct rejection, reference failure, cycle, collision, and shell
/// late-binding violation surfaces as its typed [`CompositionError`] variant.
/// All of them are abort-all: preflight never degrades to best-effort.
pub fn build_preflight_graph(
    plan: &SequencePlan,
    source: &ResolvedCompositionSource,
) -> Result<PreflightGraph, CompositionError> {
    let mut loader = Loader::new(source)?;
    loader.walk_plan(plan)?;
    Ok(loader.graph)
}

/// Walk state: the graph under construction plus the ancestry stack and the
/// parsed-document cache shared by independent branches.
struct Loader<'a> {
    graph: PreflightGraph,
    /// Canonical paths currently being expanded, innermost last.
    ancestry: Vec<PathBuf>,
    /// Immutable parsed data files, reusable across independent branches
    /// (spec → *Task Resolution*: sharing parsed data is allowed; effective
    /// parameters and runtime state stay invocation-local).
    documents: HashMap<PathBuf, Value>,
    source: &'a ResolvedCompositionSource,
    /// The invoking document's canonical path, used for write-back collisions.
    source_path: PathBuf,
    /// Captured once: rebuilding it per shell command would re-probe the
    /// environment for every string in the graph.
    context: ComposeContext,
}

impl<'a> Loader<'a> {
    fn new(source: &'a ResolvedCompositionSource) -> Result<Self, CompositionError> {
        Ok(Self {
            graph: PreflightGraph::default(),
            ancestry: Vec::new(),
            documents: HashMap::new(),
            source,
            source_path: canonical(&source.resolved_path),
            context: ComposeContext::capture(),
        })
    }

    fn walk_plan(&mut self, plan: &SequencePlan) -> Result<(), CompositionError> {
        let source_path = self.source.resolved_path.clone();
        // The invoking document roots the ancestry, so a task or prompt that
        // references the sequence source itself closes a cycle rather than
        // recursing forever.
        self.ancestry.push(self.source_path.clone());
        for (index, step) in plan.steps.iter().enumerate() {
            let label = format!("step {} (`{}`)", index + 1, step.state.id);
            let state = self.step_state(plan, index)?;

            let task = match &step.executable {
                Some(executable) => Some(self.load_step_task(
                    executable,
                    &source_path,
                    &label,
                    &state,
                    None,
                )?),
                None => None,
            };

            self.graph.steps.push(PreflightStep {
                index,
                id: step.state.id.clone(),
                name: step.name.clone(),
                task,
            });
        }
        Ok(())
    }

    /// Build the early-binding lookup for one step: the invoking document's
    /// frontmatter under this step's reserved overlay.
    ///
    /// This is exactly the state a shell command sees at its turn, minus the
    /// runtime mutation layer — which is precisely why a command referencing
    /// that layer is rejected rather than approved against a stale value.
    fn step_state(
        &self,
        plan: &SequencePlan,
        index: usize,
    ) -> Result<EffectiveState, CompositionError> {
        let overlay = super::build_step_overlay(plan, index);
        let mut frontmatter: HashMap<String, Value> = self
            .source
            .markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if let Value::Object(overrides) = overlay.as_set_overrides(None) {
            frontmatter.extend(overrides);
        }

        EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter)
            .with_context(self.context.clone())
            .with_allow_ctx_override(true)
            .build()
            .map_err(|e| CompositionError::PreFlightStateBuildFailed { source: e })
    }

    /// Expand a step's declared executable into a task node.
    fn load_step_task(
        &mut self,
        executable: &StepExecutable,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
        group_defaults: Option<&GroupDefaults>,
    ) -> Result<PreflightTask, CompositionError> {
        self.load_task(
            executable.field,
            &executable.value,
            &executable.options,
            origin,
            label,
            state,
            group_defaults,
        )
    }

    /// Expand one executable field plus its options into a task node.
    #[allow(clippy::too_many_arguments)]
    fn load_task(
        &mut self,
        field: ExecutableField,
        value: &Value,
        options: &Map<String, Value>,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
        group_defaults: Option<&GroupDefaults>,
    ) -> Result<PreflightTask, CompositionError> {
        let origin_dir = origin
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let action = match field {
            ExecutableField::Prompt => {
                let reference = expect_reference(value, "prompt", label)?;
                let document = self.load_prompt(&reference, origin)?;
                PreflightAction::Prompt {
                    path: document,
                    reference,
                }
            }
            ExecutableField::Shell => {
                let commands = self.resolve_shell_commands(value, origin, label, state)?;
                PreflightAction::Shell { commands }
            }
            ExecutableField::SideEffect => PreflightAction::SideEffect {
                action: value.clone(),
            },
            ExecutableField::Group => {
                if group_defaults.is_some() {
                    return Err(CompositionError::SequenceUnsupportedConstruct {
                        construct: "a nested group".to_string(),
                        detail: format!(
                            "the task in {label} declares `group:`, but groups do not nest in v1; \
                             flatten the inner group's tasks into the outer one"
                        ),
                    });
                }
                let group = self.load_group(value, origin, label, state)?;
                PreflightAction::Group(group)
            }
            ExecutableField::Task => {
                let reference = expect_reference(value, "task", label)?;
                return self.load_external_task(&reference, origin, label, state, group_defaults);
            }
        };

        // Group `operation`/`flow` are defaults; a task-level value overrides
        // them. Both are meaningless outside a prompt task, and the option
        // catalog already rejects an explicit one — so a group default is
        // simply not propagated to non-prompt siblings rather than erroring.
        let prompt_task = matches!(action, PreflightAction::Prompt { .. });
        let operation = option_string(options, "operation")
            .or_else(|| prompt_task.then(|| group_defaults.and_then(|d| d.operation.clone())).flatten());
        let flow = option_string(options, "flow")
            .or_else(|| prompt_task.then(|| group_defaults.and_then(|d| d.flow.clone())).flatten());

        let mut task = PreflightTask {
            name: option_string(options, "name"),
            label: label.to_string(),
            action,
            params: option_map(options, "params"),
            timeout: options.get("timeout").cloned(),
            operation,
            flow,
            setup: options.get("setup").cloned(),
            teardown: options.get("teardown").cloned(),
            origin_dir,
            origin_path: origin.to_path_buf(),
        };

        self.collect_lifecycle_shell(&mut task, origin, label, state)?;
        Ok(task)
    }

    /// Expand a `task: <file-ref>` reference into the task it names.
    ///
    /// The reference is exclusive and immutable at the referencing site: the
    /// referenced file supplies the executable *and* every option, so nothing
    /// from the referencing step is merged in.
    fn load_external_task(
        &mut self,
        reference: &str,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
        group_defaults: Option<&GroupDefaults>,
    ) -> Result<PreflightTask, CompositionError> {
        let path = self.resolve_reference(reference, origin)?;
        let document = self.load_data_document(&path)?;
        let root = expect_object(&document, "task file", &path)?;

        expect_kind(&root, "task", &path)?;
        let (field, value, options) = parse_task_object(&root, &path)?;

        let nested_label = format!("{label} → task `{}`", path.display());
        self.enter(&path)?;
        let result = self.load_task(
            field,
            &value,
            &options,
            &path,
            &nested_label,
            state,
            group_defaults,
        );
        self.leave();
        result
    }

    /// Resolve a `group:` value into a group node.
    fn load_group(
        &mut self,
        value: &Value,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
    ) -> Result<PreflightGroup, CompositionError> {
        match value {
            Value::Object(map) => self.build_group(map, origin, label, state),
            Value::String(raw) => {
                let (name, reference) = split_catalog_reference(raw);
                let path = self.resolve_reference(&reference, origin)?;
                let document = self.load_data_document(&path)?;
                let root = expect_object(&document, "group file", &path)?;

                let group_map = match name {
                    Some(name) => select_catalog_group(&root, &name, &path)?,
                    None => {
                        expect_kind(&root, "group", &path)?;
                        root.clone()
                    }
                };

                let nested_label = format!("{label} → group `{}`", path.display());
                self.enter(&path)?;
                let result = self.build_group(&group_map, &path, &nested_label, state);
                self.leave();
                result
            }
            other => Err(CompositionError::SequenceGroupInvalid {
                group: "<inline>".to_string(),
                origin: origin.to_path_buf(),
                problem: format!(
                    "`group:` must be an inline group object or a file reference, got {}",
                    json_type_name(other)
                ),
            }),
        }
    }

    /// Validate a group definition and expand its tasks.
    fn build_group(
        &mut self,
        map: &Map<String, Value>,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
    ) -> Result<PreflightGroup, CompositionError> {
        let name = match map.get("name") {
            Some(Value::String(s)) => s.clone(),
            _ => "<inline>".to_string(),
        };
        let invalid = |problem: String| CompositionError::SequenceGroupInvalid {
            group: name.clone(),
            origin: origin.to_path_buf(),
            problem,
        };

        // Group `loop` commit semantics are unratified (spec → *Open
        // Questions*). Rejecting here keeps an implementer from inventing them.
        if map.contains_key("loop") {
            return Err(CompositionError::SequenceUnsupportedConstruct {
                construct: format!("group `loop` (in group `{name}`)"),
                detail: "iteration commit semantics for group loops are not ratified yet; \
                         remove `loop:` or express the repetition as separate sequence steps"
                    .to_string(),
            });
        }

        let execution = match map.get("execution") {
            None | Some(Value::Null) => GroupExecution::Serial,
            Some(Value::String(s)) if s == "serial" => GroupExecution::Serial,
            Some(Value::String(s)) if s == "parallel" => GroupExecution::Parallel,
            Some(other) => {
                return Err(invalid(format!(
                    "`execution` must be `serial` or `parallel`, got {}",
                    render_scalar(other)
                )));
            }
        };

        let max_parallel = match map.get("max_parallel") {
            None | Some(Value::Null) => None,
            Some(value) => {
                if execution == GroupExecution::Serial {
                    return Err(invalid(
                        "`max_parallel` is only valid on a parallel group".to_string(),
                    ));
                }
                let cap = value.as_u64().filter(|n| *n >= 1).ok_or_else(|| {
                    invalid(format!(
                        "`max_parallel` must be an integer >= 1, got {}",
                        render_scalar(value)
                    ))
                })?;
                Some(cap as usize)
            }
        };

        let variables = option_map(map, "variables");
        let defaults = GroupDefaults {
            operation: option_string(map, "operation"),
            flow: option_string(map, "flow"),
        };

        let raw_tasks = match map.get("tasks") {
            Some(Value::Array(items)) if !items.is_empty() => items.clone(),
            Some(Value::Array(_)) | None => {
                return Err(invalid("`tasks` must declare at least one task".to_string()));
            }
            Some(other) => {
                return Err(invalid(format!(
                    "`tasks` must be a list, got {}",
                    json_type_name(other)
                )));
            }
        };

        let mut tasks = Vec::with_capacity(raw_tasks.len());
        for (index, raw) in raw_tasks.iter().enumerate() {
            let task_map = raw.as_object().ok_or_else(|| {
                invalid(format!(
                    "task {} must be an object, got {}",
                    index + 1,
                    json_type_name(raw)
                ))
            })?;
            let (field, value, options) = parse_task_object(task_map, origin)?;
            let task_label = format!("{label} → task {} of group `{name}`", index + 1);
            tasks.push(self.load_task(
                field,
                &value,
                &options,
                origin,
                &task_label,
                state,
                Some(&defaults),
            )?);
        }

        let group = PreflightGroup {
            name,
            execution,
            max_parallel,
            variables,
            tasks,
            origin_path: origin.to_path_buf(),
        };

        if group.execution == GroupExecution::Parallel {
            self.check_write_back_collisions(&group)?;
        }
        Ok(group)
    }

    /// Reject two siblings in a parallel group that would rewrite one file.
    ///
    /// The sequence source document counts as an occupied target: an
    /// inline-compose sequence rewrites its own body between steps, so a
    /// concurrent task pointed at it would race the orchestrator itself.
    fn check_write_back_collisions(
        &self,
        group: &PreflightGroup,
    ) -> Result<(), CompositionError> {
        let mut claimed: HashMap<PathBuf, String> = HashMap::new();
        if self.source_inline_compose() {
            claimed.insert(self.source_path.clone(), "the sequence source".to_string());
        }

        for task in &group.tasks {
            let PreflightAction::Prompt { path, .. } = &task.action else {
                continue;
            };
            let Some(document) = self.graph.prompt_document(path) else {
                continue;
            };
            if !document.inline_compose {
                continue;
            }
            let owner = task.name.clone().unwrap_or_else(|| task.label.clone());
            if let Some(first) = claimed.get(path) {
                return Err(CompositionError::SequenceWriteBackCollision {
                    group: group.name.clone(),
                    target: path.clone(),
                    first: first.clone(),
                    second: owner,
                });
            }
            claimed.insert(path.clone(), owner);
        }
        Ok(())
    }

    fn source_inline_compose(&self) -> bool {
        matches!(
            self.source.markdown.frontmatter().as_map().get("prompt"),
            Some(Value::String(_))
        )
    }

    /// Load a `prompt:` document as a graph leaf.
    ///
    /// Prompt documents are leaves by construction: one that declares its own
    /// `sequence:` is a nested sequence, which v1 rejects.
    fn load_prompt(&mut self, reference: &str, origin: &Path) -> Result<PathBuf, CompositionError> {
        let path = self.resolve_reference(reference, origin)?;
        if let Some(existing) = self.graph.prompt_document(&path) {
            let _ = existing;
            return Ok(path);
        }
        self.check_cycle(&path)?;

        // The referenced document is loaded through the ordinary Markdown load
        // error so a malformed prompt reports the same typed parse failure it
        // would when composed directly, rather than a preflight-shaped string.
        let markdown = Markdown::try_from(path.as_path()).map_err(|e| {
            CompositionError::MarkdownLoad {
                path: path.clone(),
                source: super::super::error::MarkdownLoadCause::Parse(Box::new(e)),
            }
        })?;

        let frontmatter = markdown.frontmatter();
        if frontmatter.as_map().contains_key("sequence") {
            return Err(CompositionError::SequenceNestedSequence {
                path: path.clone(),
                referenced_from: origin.to_path_buf(),
            });
        }
        let inline_compose = matches!(frontmatter.as_map().get("prompt"), Some(Value::String(_)));
        let has_schema = frontmatter.as_map().contains_key("$schema");

        self.graph.prompt_documents.push(PromptDocument {
            path: path.clone(),
            markdown,
            inline_compose,
            has_schema,
        });
        Ok(path)
    }

    /// Resolve a task's `shell:` value into its executable byte strings.
    fn resolve_shell_commands(
        &mut self,
        value: &Value,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
    ) -> Result<Vec<String>, CompositionError> {
        let raw: Vec<String> = match value {
            Value::String(s) => vec![s.clone()],
            Value::Array(items) => items
                .iter()
                .map(|item| match item {
                    Value::String(s) => Ok(s.clone()),
                    other => Err(CompositionError::SequenceReferenceInvalid {
                        context: format!("shell task in {label}"),
                        path: origin.to_path_buf(),
                        problem: format!(
                            "every `shell` entry must be a string, got {}",
                            json_type_name(other)
                        ),
                    }),
                })
                .collect::<Result<_, _>>()?,
            other => {
                return Err(CompositionError::SequenceReferenceInvalid {
                    context: format!("shell task in {label}"),
                    path: origin.to_path_buf(),
                    problem: format!(
                        "`shell` must be a string or a list of strings, got {}",
                        json_type_name(other)
                    ),
                });
            }
        };

        let mut resolved = Vec::with_capacity(raw.len());
        for command in raw {
            let bytes = self.resolve_shell_bytes(&command, origin, label, state)?;
            self.graph.shell_commands.push(DiscoveredCommand {
                command: bytes.clone(),
                source_file: origin.to_path_buf(),
                origin: label.to_string(),
            });
            resolved.push(bytes);
        }
        Ok(resolved)
    }

    /// Collect shell actions out of a task's `setup:`/`teardown:` stacks.
    ///
    /// Traversal is condition-blind: a `when:`-guarded action contributes its
    /// command regardless of the guard, because the guard may read differently
    /// at the task's turn.
    fn collect_lifecycle_shell(
        &mut self,
        task: &mut PreflightTask,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
    ) -> Result<(), CompositionError> {
        for (stage, stack) in [("setup", task.setup.clone()), ("teardown", task.teardown.clone())] {
            let Some(stack) = stack else { continue };
            for command in shape::collect_stack_shell_commands(&stack) {
                let stage_label = format!("{label} `{stage}`");
                let bytes = self.resolve_shell_bytes(&command, origin, &stage_label, state)?;
                self.graph.shell_commands.push(DiscoveredCommand {
                    command: bytes,
                    source_file: origin.to_path_buf(),
                    origin: stage_label,
                });
            }
        }
        Ok(())
    }

    /// Resolve one shell string's `{{ … }}` spans against early-binding values.
    ///
    /// The returned string is the *only* executable representation of the
    /// command: it is what preflight approves and what execution runs, so the
    /// two can never drift.
    fn resolve_shell_bytes(
        &self,
        raw: &str,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
    ) -> Result<String, CompositionError> {
        if !raw.contains("{{") {
            return Ok(raw.to_string());
        }
        if let Some(root) = shape::first_unavailable_root(raw, SHELL_UNAVAILABLE_ROOTS) {
            return Err(CompositionError::SequenceShellLateBinding {
                command: raw.to_string(),
                root,
                task: label.to_string(),
            });
        }

        let base = origin
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let resolution_ctx = ResolutionContext::new(base);

        let composed = SubtreeCompose::new(&Value::String(raw.to_string()), state)
            .with_resolution_context(resolution_ctx)
            .strict()
            .compose()
            .map_err(|e| CompositionError::SequenceShellResolution {
                command: raw.to_string(),
                task: label.to_string(),
                message: e.to_string(),
                source: Some(Box::new(e)),
            })?;

        Ok(match composed {
            Value::String(s) => s,
            other => other.to_string(),
        })
    }

    /// Resolve a reference authored inside `origin` to an absolute path.
    fn resolve_reference(
        &self,
        reference: &str,
        origin: &Path,
    ) -> Result<PathBuf, CompositionError> {
        source_resolution::resolve_sequence_reference(reference, origin).map(|p| canonical(&p))
    }

    /// Load and cache a data document (`kind: task`/`group`/`group-catalog`).
    fn load_data_document(&mut self, path: &Path) -> Result<Value, CompositionError> {
        if let Some(cached) = self.documents.get(path) {
            return Ok(cached.clone());
        }
        let document = super::data::load_document(path)?;
        self.documents.insert(path.to_path_buf(), document.clone());
        Ok(document)
    }

    /// Reject a reference back to a document already being expanded, reporting
    /// the whole chain rather than only the repeated file.
    fn check_cycle(&self, path: &Path) -> Result<(), CompositionError> {
        if self.ancestry.iter().any(|p| p == path) {
            let mut chain = self.ancestry.clone();
            chain.push(path.to_path_buf());
            return Err(CompositionError::SequenceReferenceCycle { chain });
        }
        Ok(())
    }

    /// Push `path` onto the ancestry stack after a cycle check.
    fn enter(&mut self, path: &Path) -> Result<(), CompositionError> {
        if self.ancestry.iter().any(|p| p == path) {
            let mut chain = self.ancestry.clone();
            chain.push(path.to_path_buf());
            return Err(CompositionError::SequenceReferenceCycle { chain });
        }
        self.ancestry.push(path.to_path_buf());
        Ok(())
    }

    fn leave(&mut self) {
        self.ancestry.pop();
    }
}

/// Group-level `operation`/`flow` defaults handed down to member tasks.
struct GroupDefaults {
    operation: Option<String>,
    flow: Option<String>,
}

/// Split `{group-name}@{file-ref}` into its parts.
///
/// The split is on the *first* `@`, so `build-group@@catalogs/all.yaml` names
/// group `build-group` in the magic reference `@catalogs/all.yaml`. A leading
/// `@` is a magic file reference with no group name, and a string with no `@`
/// at all is a plain `kind: group` file reference.
fn split_catalog_reference(raw: &str) -> (Option<String>, String) {
    let trimmed = raw.trim();
    match trimmed.find('@') {
        Some(0) | None => (None, trimmed.to_string()),
        Some(at) => (
            Some(trimmed[..at].trim().to_string()),
            trimmed[at + 1..].trim().to_string(),
        ),
    }
}

/// Find the one group named `name` in a `kind: group-catalog` document.
fn select_catalog_group(
    root: &Map<String, Value>,
    name: &str,
    path: &Path,
) -> Result<Map<String, Value>, CompositionError> {
    expect_kind(root, "group-catalog", path)?;

    let groups = match root.get("groups") {
        Some(Value::Array(items)) => items,
        other => {
            return Err(CompositionError::SequenceReferenceInvalid {
                context: "group catalog".to_string(),
                path: path.to_path_buf(),
                problem: format!(
                    "a `kind: group-catalog` document must declare a `groups:` list, got {}",
                    other.map_or("nothing", json_type_name)
                ),
            });
        }
    };

    let available: Vec<String> = groups
        .iter()
        .filter_map(|g| g.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .collect();

    let matches: Vec<&Value> = groups
        .iter()
        .filter(|g| g.get("name").and_then(Value::as_str) == Some(name))
        .collect();

    let lookup_failure = |problem: &str| CompositionError::SequenceGroupCatalogLookup {
        name: name.to_string(),
        path: path.to_path_buf(),
        problem: problem.to_string(),
        available: available.clone(),
    };

    match matches.as_slice() {
        [only] => only
            .as_object()
            .cloned()
            .ok_or_else(|| lookup_failure("is not an object")),
        [] => Err(lookup_failure("is not defined")),
        _ => Err(lookup_failure("is defined more than once")),
    }
}

/// Parse a task object into its single executable field, that field's value,
/// and its validated option map.
fn parse_task_object(
    map: &Map<String, Value>,
    origin: &Path,
) -> Result<(ExecutableField, Value, Map<String, Value>), CompositionError> {
    let invalid = |problem: String| CompositionError::SequenceReferenceInvalid {
        context: "task".to_string(),
        path: origin.to_path_buf(),
        problem,
    };

    let executables: Vec<&str> = map
        .keys()
        .map(String::as_str)
        .filter(|k| reserved::is_executable_key(k))
        .collect();
    let field = match executables.as_slice() {
        [only] => ExecutableField::from_key(only).expect("filtered to executable keys"),
        [] => {
            return Err(invalid(format!(
                "a task must declare exactly one of {}",
                reserved::EXECUTABLE_KEYS.join(", ")
            )));
        }
        many => {
            return Err(invalid(format!(
                "a task declares exactly one executable; found {}",
                many.join(", ")
            )));
        }
    };

    let allowed = reserved::allowed_task_options(field);
    let mut options = Map::new();
    for (key, value) in map {
        if reserved::is_executable_key(key) || key == "kind" {
            continue;
        }
        if !reserved::is_task_option_key(key) {
            return Err(invalid(format!(
                "`{key}` is not a task field; expected one of {}",
                reserved::TASK_OPTION_KEYS.join(", ")
            )));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(format!(
                "`{key}` is meaningless for a `{}` task",
                field.key()
            )));
        }
        options.insert(key.clone(), value.clone());
    }

    Ok((field, map[field.key()].clone(), options))
}

/// Require a document's `kind:` to be `expected`.
fn expect_kind(
    root: &Map<String, Value>,
    expected: &str,
    path: &Path,
) -> Result<(), CompositionError> {
    match root.get("kind") {
        Some(Value::String(kind)) if kind == expected => Ok(()),
        other => Err(CompositionError::SequenceReferenceInvalid {
            context: format!("`kind: {expected}` document"),
            path: path.to_path_buf(),
            problem: match other {
                Some(Value::String(kind)) => format!("declares `kind: {kind}`"),
                Some(other) => format!("`kind` must be a string, got {}", json_type_name(other)),
                None => format!("does not declare `kind: {expected}`"),
            },
        }),
    }
}

fn expect_object(
    document: &Value,
    context: &str,
    path: &Path,
) -> Result<Map<String, Value>, CompositionError> {
    document
        .as_object()
        .cloned()
        .ok_or_else(|| CompositionError::SequenceReferenceInvalid {
            context: context.to_string(),
            path: path.to_path_buf(),
            problem: format!("root must be an object, got {}", json_type_name(document)),
        })
}

fn expect_reference(value: &Value, key: &str, label: &str) -> Result<String, CompositionError> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        other => Err(CompositionError::SequenceReferenceInvalid {
            context: format!("`{key}:` in {label}"),
            path: PathBuf::new(),
            problem: format!(
                "must be a non-empty file reference, got {}",
                json_type_name(other)
            ),
        }),
    }
}

fn option_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn option_map(map: &Map<String, Value>, key: &str) -> Map<String, Value> {
    match map.get(key) {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::new(),
    }
}

/// Render a scalar for an error message without the quoting `Display` on
/// `Value` would add for strings.
fn render_scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Canonicalize when the filesystem allows it, so two spellings of one file
/// compare equal in the ancestry stack and the collision map.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests;
