//! Preflight node shapes and the raw-value traversals that feed them.
//!
//! The node types here are deliberately inert: they describe *what could run*,
//! not how. Execution consumes them; preflight only fills them in.

use std::path::PathBuf;

use darkmatter::markdown::compose::ResolvedShellValue;
use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, parse};
use darkmatter::markdown::compose::shell_expansion::template::{CommandTemplate, HoleTiming};
use serde_json::{Map, Value};

use crate::composition::authored_order::AuthoredOrder;
use crate::composition::sequence::task::TaskStage;

/// One sequence step's preflight node.
#[derive(Debug, Clone)]
pub struct PreflightStep {
    /// Zero-based position in the sequence.
    pub index: usize,
    /// The step's generated state id.
    pub id: String,
    /// The step's display name.
    pub name: String,
    /// The step's task, or `None` when the step runs the source document body.
    pub task: Option<PreflightTask>,
}

/// One atomic unit of potentially executable work.
#[derive(Debug, Clone)]
pub struct PreflightTask {
    /// The authored `name:`, when the task declared one.
    pub name: Option<String>,
    /// A generated label locating this task in diagnostics.
    pub label: String,
    /// What the task executes.
    pub action: PreflightAction,
    /// The prompt task's `with:` inputs, unevaluated: they bind just in time
    /// against the caller's effective state, from the file that authored them
    /// (`origin_path`).
    pub with: Map<String, Value>,
    /// The authored `timeout:` value, parsed as a duration at execution time.
    pub timeout: Option<Value>,
    /// The authored `when:` guard, already checked to parse as a condition.
    /// It is evaluated at the task's turn, never during preflight, so a task
    /// whose guard will be false is still validated and approved.
    pub when: Option<String>,
    /// Effective operation after group defaults and task overrides.
    pub operation: Option<String>,
    /// Effective flow after group defaults and task overrides.
    pub flow: Option<String>,
    /// The raw `setup:` action stack.
    pub setup: Option<Value>,
    /// The raw `teardown:` action stack.
    pub teardown: Option<Value>,
    /// The directory descendant references of this task resolve from.
    pub origin_dir: PathBuf,
    /// The document that authored this task.
    pub origin_path: PathBuf,
    /// Authored mapping order of the owning document, positioned at this task.
    ///
    /// `setup:`/`teardown:` are parsed at execution time from the raw values
    /// above, so the cursor that reaches their `set:` mappings has to be
    /// recorded here while preflight still knows where in the document the task
    /// came from. Default when the owning document records no key order.
    pub authored: AuthoredOrder,
    /// Source location used when an executable reports an authored-property
    /// diagnostic. This is fixed during preflight because external groups and
    /// task files change both halves independently of the invoking sequence.
    pub diagnostic: TaskDiagnosticProvenance,
    /// The `setup:`/`teardown:` shell commands sequence approval fixed, which
    /// execution runs instead of re-evaluating the authored text.
    pub approved_stack_commands: Vec<ApprovedCommand>,
    /// Every interpolating `shell:`, `setup:`, and `teardown:` command
    /// preflight resolved to bytes, with what it was resolved from, so the task
    /// can refuse one its own captured environment would change.
    pub preflight_approvals: Vec<super::PreflightApproval>,
    /// Where the task sits in its sequence. Static: skipped predecessors,
    /// retries, and resume never change it.
    pub position: TaskPosition,
    /// One record per shell command site of the task (`shell:`, stack `shell`
    /// actions, and stack whole-value `set` `$( … )`), in authored order.
    pub shell_sites: Vec<PreparedShellSite>,
    /// The stack whole-value `set` `$( … )` values sequence approval fixed as
    /// resolved bytes, which execution runs instead of resolving the authored
    /// text. A template-mode value is absent here.
    pub approved_set_values: Vec<ApprovedSetValue>,
    /// Where the task sits in the graph; see [`TaskIdentity`].
    pub identity: TaskIdentity,
    /// What preflight predicts this task's provider from, for a `prompt:` task.
    /// `None` for every executable that launches no provider of its own.
    pub provider: Option<super::providers::ProviderPrediction>,
}

/// A task's position in the preflight graph, from the root sequence down:
/// `tasks[1]`, `tasks[1].sequence[0]`, `tasks[2].group.tasks[1]`.
///
/// Unlike a task's source-rooted property, it never resets at a file boundary,
/// so two references to one task file, or two nested tasks with one name, have
/// different identities. Per-task provider choices are keyed by it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskIdentity(String);

impl TaskIdentity {
    /// The identity of root step `index`.
    #[must_use]
    pub fn root(index: usize) -> Self {
        Self(format!("tasks[{index}]"))
    }

    /// The identity of child `index` of the nested sequence this task runs.
    #[must_use]
    pub fn sequence_child(&self, index: usize) -> Self {
        Self(format!("{}.sequence[{index}]", self.0))
    }

    /// The identity of leg `index` of the group this task runs.
    #[must_use]
    pub fn group_leg(&self, index: usize) -> Self {
        Self(format!("{}.group.tasks[{index}]", self.0))
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TaskIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl PreflightTask {
    /// Every template site of this task whose placeholders include a late
    /// value, as `(site id, template)`: what the step barrier resolves before
    /// any of the task's commands run.
    pub fn late_sites(&self) -> impl Iterator<Item = (&str, &CommandTemplate)> {
        self.shell_sites
            .iter()
            .filter(|site| site.reads_late_value())
            .filter_map(|site| Some((site.id.as_str(), site.template.as_ref()?)))
    }
}

/// Where a task sits in its sequence.
///
/// A task is [`TaskPosition::First`] only along an all-first-step ancestry.
/// Every leg of a first group shares that position. A first task's commands may be
/// approved as resolved bytes; every other task's commands with placeholders
/// are approved only as templates, because earlier tasks may change the values
/// they read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPosition {
    First,
    Later,
}

impl TaskPosition {
    /// The position of the child at `index` of a sequence-like container at
    /// this position: first only when the container is first and the child is
    /// its first. A group's legs are not ordered and take the group's position.
    #[must_use]
    pub fn child(self, index: usize) -> Self {
        match (self, index) {
            (Self::First, 0) => Self::First,
            _ => Self::Later,
        }
    }
}

/// What a sequence shell site is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellSiteKind {
    /// A `shell:` command or a stack `shell` action's `command`/`on_error`.
    Command,
    /// A stack `set` value written as a whole-value `$( … )`; the site is the
    /// command inside it.
    SetValue {
        /// The destination key.
        key: String,
    },
}

/// How a site was approved, which is how it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteMode {
    /// Approved as these bytes, fixed at preflight. They run as they stand and
    /// are never re-evaluated.
    ResolvedBytes {
        /// The resolved command, as approval saw it.
        bytes: String,
    },
    /// Approved by its shape: each placeholder is read, checked against its
    /// type, and filled as one argument when the command runs.
    Template,
}

/// One shell command site of a sequence task, as preflight prepared it.
#[derive(Debug, Clone)]
pub struct PreparedShellSite {
    /// The site's identity within its task, the key of its [`LateValues`].
    ///
    /// [`LateValues`]: darkmatter::markdown::compose::shell_expansion::template::LateValues
    pub id: String,
    pub stage: super::super::task::TaskStage,
    pub kind: ShellSiteKind,
    /// The command as authored, `{{ … }}` spans included. For a `set` value
    /// this is the command inside `$( … )`.
    pub authored: String,
    /// The authored command typed against the owning document's schema, as
    /// the task's `when:` guard narrows it for a `shell:` command, with each
    /// hole's timing. `None` when it has no placeholder or, for a
    /// resolved-byte site, when it is not a valid template.
    pub template: Option<CommandTemplate>,
    pub mode: SiteMode,
    pub position: TaskPosition,
    /// The document that authored the command.
    pub source_file: PathBuf,
    /// The source-rooted property the command was authored at
    /// (`tasks[1].shell`, `tasks[0].setup`).
    pub property: String,
    /// The task label diagnostics name.
    pub label: String,
}

impl PreparedShellSite {
    /// Whether a placeholder reads a value an earlier step produces, so the
    /// step barrier reads it. A late placeholder the task's `when:` guard
    /// proves present ([`HoleTiming::Guarded`]) is read only by its command
    /// once the guard selects the task, so it does not count.
    pub fn reads_late_value(&self) -> bool {
        self.template.as_ref().is_some_and(|template| {
            template
                .placeholders()
                .any(|placeholder| matches!(placeholder.timing, HoleTiming::Deferred { .. }))
        })
    }
}

/// A stack `set` `$( … )` value sequence approval fixed as resolved bytes.
#[derive(Debug, Clone)]
pub struct ApprovedSetValue {
    /// The stage and the authored `$( … )` value.
    pub site: CommandSiteId,
    /// The destination key.
    pub key: String,
    /// The fixed value, whose [`ResolvedShellValue::commands`] were approved.
    pub value: ResolvedShellValue,
}

/// Where a `setup:`/`teardown:` shell command was authored: its stage and its
/// authored text. Two sites with the same text in the same stage resolve
/// against the same state at the same moment, so they share bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSiteId {
    stage: TaskStage,
    authored: String,
}

impl CommandSiteId {
    pub fn new(stage: TaskStage, authored: impl Into<String>) -> Self {
        Self {
            stage,
            authored: authored.into(),
        }
    }

    pub fn stage(&self) -> TaskStage {
        self.stage
    }

    pub fn authored(&self) -> &str {
        &self.authored
    }
}

/// A stack command whose bytes sequence preflight resolved. Execution runs
/// [`Self::command`]; the authored text is never evaluated again to produce
/// what runs. (A [`PreflightApproval`](super::PreflightApproval) re-resolves
/// it only to refuse a task whose environment would change it.)
///
/// Only resolved-byte sites are recorded here. A later task's command with
/// placeholders is a [`SiteMode::Template`] on its [`PreparedShellSite`]
/// instead, even when every value exists at approval time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedCommand {
    site: CommandSiteId,
    bytes: String,
}

impl ApprovedCommand {
    pub(crate) fn new(site: CommandSiteId, bytes: String) -> Self {
        Self { site, bytes }
    }

    pub fn command(&self) -> &str {
        &self.bytes
    }

    pub fn site(&self) -> &CommandSiteId {
        &self.site
    }
}

/// Authored source location for diagnostics raised while executing a task.
#[derive(Debug, Clone)]
pub struct TaskDiagnosticProvenance {
    /// The document that owns `action_property`.
    pub source_path: PathBuf,
    /// Exact source-rooted path of the task's executable property.
    pub action_property: String,
    /// Exact source-rooted path of the task itself, the root every other task
    /// property hangs off (`tasks[0]`, `tasks[1].group.tasks[0]`).
    ///
    /// Empty when the task *is* the document — an external `kind: task` file,
    /// where `setup` is already the source-rooted spelling.
    pub task_property: String,
}

/// The executable shapes, resolved.
#[derive(Debug, Clone)]
pub enum PreflightAction {
    /// Compose and execute a referenced document.
    Prompt {
        /// The resolved, canonical document path.
        path: PathBuf,
        /// The reference as authored, retained for diagnostics.
        reference: String,
    },
    /// Run one or more commands.
    Shell {
        /// The commands in declaration order: the bytes preflight resolved, or
        /// the authored text of a command with a late value, which has none.
        /// How each was approved is its [`PreparedShellSite`]'s [`SiteMode`].
        commands: Vec<String>,
    },
    /// Run one Darkmatter side effect, in the standard action grammar.
    SideEffect {
        /// The authored action, evaluated at execution time.
        action: Value,
        /// Authored destination order retained outside ordinary JSON maps.
        authored_set_order: Option<Vec<String>>,
    },
    /// Execute a group.
    Group(PreflightGroup),
    /// Execute a nested sequence as one native array result.
    Sequence(Box<PreflightSequence>),
}

/// A nested sequence with its frozen approval graph and authoring source.
#[derive(Debug, Clone)]
pub struct PreflightSequence {
    /// Normalized child positions and authored values.
    pub plan: super::super::model::SequencePlan,
    /// Children in declaration order.
    pub steps: Vec<PreflightStep>,
    /// Events belonging to this container, rather than its children.
    pub lifecycle: Box<crate::composition::LifecycleConfig>,
    /// Source used to resolve child references and compose body-only steps.
    pub origin_path: PathBuf,
    /// The referenced sequence document's authored root properties, for a
    /// reference that names a whole document (no offset or operator).
    ///
    /// When present it replaces the caller's frontmatter as the base state of
    /// this container's events and children; invocation setters and runtime
    /// mutations still layer above it. `None` (inline lists, offset and
    /// operator references) keeps the enclosing scope's state.
    pub frontmatter: Option<Map<String, Value>>,
    /// The sequence's `loop:` controls, when it repeats. Its gate is
    /// [`Self::lifecycle`]'s `loop` stack.
    pub loop_config: Option<PreflightLoop>,
}

/// A resolved group and its member tasks.
#[derive(Debug, Clone)]
pub struct PreflightGroup {
    /// The group's name, or `<inline>` for an unnamed inline group.
    pub name: String,
    /// Concurrent for authored groups; serial remains an internal runtime mode.
    pub execution: GroupExecution,
    /// Whether a leg failure fails the group; retained for container execution.
    pub fail_fast: bool,
    /// Validated container events, including explicit cancellation opt-in.
    pub lifecycle: Box<crate::composition::LifecycleConfig>,
    /// Optional concurrency cap; only ever set on a parallel group.
    pub max_parallel: Option<usize>,
    /// Group-scoped variables exposed to member tasks on `group.*`.
    pub variables: Map<String, Value>,
    /// Member tasks in declaration order.
    pub tasks: Vec<PreflightTask>,
    /// The document the group was authored in.
    pub origin_path: PathBuf,
    /// The group's `loop:` controls, when it repeats. Its gate is
    /// [`Self::lifecycle`]'s `loop` stack. Boxed so a group stays the size
    /// of the other task actions.
    pub loop_config: Option<Box<PreflightLoop>>,
}

/// A container's `loop:` controls, frozen at preflight for the invocation,
/// and where they were authored.
#[derive(Debug, Clone)]
pub struct PreflightLoop {
    /// The iteration controls; a templated `max` is still unresolved.
    pub config: crate::composition::LoopConfig,
    /// The authored `loop:` block, for diagnostics.
    pub location: crate::composition::LoopLocation,
}

/// How a group's tasks are scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupExecution {
    /// Declaration order, one at a time.
    Serial,
    /// Concurrent, bounded by `max_parallel` when set.
    Parallel,
}

/// Collect every shell command site from a raw `setup:`/`teardown:` stack: each
/// `shell` action's `command` and `on_error`, and each `set` value written as a
/// whole-value `$( … )` (returned as authored, `$( … )` included).
///
/// The stack is walked as raw JSON rather than through the lifecycle parser
/// because preflight wants *potential* commands: an entry is collected whether
/// or not its `when:` guard holds — both `then` and `else` bodies are walked —
/// and whether it is written in short form (`shell: "…"`) or long form
/// (`shell: {command: "…", on_error: "…"}`).
pub fn collect_stack_shell_sites(stack: &Value) -> Vec<(ShellSiteKind, String)> {
    let mut sites = Vec::new();
    collect_from_value(stack, &mut sites);
    sites
}

fn collect_from_value(value: &Value, out: &mut Vec<(ShellSiteKind, String)>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_from_value(item, out);
            }
        }
        Value::Object(map) => collect_from_object(map, out),
        _ => {}
    }
}

fn collect_from_object(map: &Map<String, Value>, out: &mut Vec<(ShellSiteKind, String)>) {
    match map.get("shell") {
        // Long form: `shell: {command: "cmd", on_error: "cmd"}`.
        Some(Value::Object(params)) => {
            for key in ["command", "on_error"] {
                if let Some(command) = params.get(key) {
                    push_strings(command, out);
                }
            }
        }
        // Short form: `shell: "cmd"` or `shell: ["a", "b"]`.
        Some(shell) => push_strings(shell, out),
        None => {}
    }
    // The lifecycle parser types the same values as shell values.
    if let Some(Value::Object(set)) = map.get("set") {
        for (key, value) in set {
            if let Value::String(text) = value
                && text.trim_start().starts_with("$(")
            {
                out.push((ShellSiteKind::SetValue { key: key.clone() }, text.trim().to_string()));
            }
        }
    }
    // Nested bodies: `then:`/`else:` lists and dictionaries.
    for (key, value) in map {
        if key == "shell" || key == "set" {
            continue;
        }
        collect_from_value(value, out);
    }
}

fn push_strings(value: &Value, out: &mut Vec<(ShellSiteKind, String)>) {
    match value {
        Value::String(s) => out.push((ShellSiteKind::Command, s.clone())),
        Value::Array(items) => {
            for item in items {
                push_strings(item, out);
            }
        }
        _ => {}
    }
}

/// The target-identity roots whose values do not exist when graph-phase shell
/// commands are resolved, because per-task target selection has not run yet.
///
/// Unlike the binding catalog's late globals, these are canonical paths, not
/// bare roots: `ctx` and `env` are legal in a graph-phase command, and only the
/// agent/model leaves of each are target-dependent. Static bracket access is
/// canonicalized to the same path as dotted access before this policy is
/// applied.
pub(crate) const TARGET_IDENTITY_ROOTS: &[&str] =
    &["ctx.agent", "ctx.model", "env.AGENT", "env.MODEL"];

/// The first target-identity path referenced by any `{{ … }}` span of `raw`.
///
/// A graph-phase shell command that references the resolved agent or model
/// would read a pre-selection value at approval, so the reference is rejected
/// rather than expanded. The bare late roots are the binding catalog's
/// sequence approval scope.
pub(crate) fn first_target_identity_root(raw: &str) -> Option<String> {
    for location in ExpressionFinder::find_all_plain(raw) {
        let Ok(expr) = parse(&location.expression) else {
            continue;
        };
        if let Some(path) = target_identity_path_in_expr(&expr) {
            return Some(path);
        }
    }
    None
}

fn target_identity_path_in_expr(expr: &Expr) -> Option<String> {
    if let Some(path) = static_member_path(expr)
        && let Some(root) = TARGET_IDENTITY_ROOTS
            .iter()
            .find(|root| path == **root || path.starts_with(&format!("{root}.")))
    {
        return Some((*root).to_string());
    }

    if let Expr::Index { base, index } = expr
        && !matches!(index.as_ref(), Expr::StringLiteral(_))
        && matches!(member_root(base), Some("ctx" | "env"))
    {
        return Some(expr.to_string());
    }

    match expr {
        Expr::Variable(_) | Expr::StringLiteral(_) | Expr::NumberLiteral(_)
        | Expr::BoolLiteral(_) => None,
        Expr::MemberAccess { base, .. } => target_identity_path_in_expr(base),
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            target_identity_path_in_expr(inner)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            target_identity_path_in_expr(left).or_else(|| target_identity_path_in_expr(right))
        }
        Expr::Index { base, index } => target_identity_path_in_expr(base)
            .or_else(|| target_identity_path_in_expr(index)),
        Expr::FunctionCall { args, .. } => {
            args.iter().find_map(target_identity_path_in_expr)
        }
        Expr::ArrayLiteral(items) => items.iter().find_map(target_identity_path_in_expr),
        Expr::ObjectLiteral(entries) => entries
            .iter()
            .find_map(|(_, value)| target_identity_path_in_expr(value)),
        Expr::Fallback { primary, fallback } => target_identity_path_in_expr(primary)
            .or_else(|| target_identity_path_in_expr(fallback)),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => target_identity_path_in_expr(condition)
            .or_else(|| target_identity_path_in_expr(then_branch))
            .or_else(|| target_identity_path_in_expr(else_branch)),
    }
}

fn static_member_path(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Variable(path) => Some(path.clone()),
        Expr::MemberAccess { base, name } => {
            Some(format!("{}.{}", static_member_path(base)?, name))
        }
        Expr::Index { base, index } => {
            let Expr::StringLiteral(key) = index.as_ref() else {
                return None;
            };
            Some(format!("{}.{}", static_member_path(base)?, key))
        }
        Expr::Paren(inner) => static_member_path(inner),
        _ => None,
    }
}

fn member_root(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Variable(path) => path.split('.').next(),
        Expr::MemberAccess { base, .. } | Expr::Index { base, .. } => member_root(base),
        Expr::Paren(inner) => member_root(inner),
        _ => None,
    }
}

