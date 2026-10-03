//! Preflight node shapes and the raw-value traversals that feed them.
//!
//! The node types here are deliberately inert: they describe *what could run*,
//! not how. Execution consumes them; preflight only fills them in.

use std::path::PathBuf;

use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, parse};
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
    /// Values passed to a prompt document as user setters, unevaluated —
    /// `params` bind just in time against the caller's effective state.
    pub params: Map<String, Value>,
    /// The authored `timeout:` value, parsed as a duration at execution time.
    pub timeout: Option<Value>,
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

/// A command whose exact bytes sequence approval fixed. Execution runs
/// [`Self::command`]; the authored text is never evaluated again.
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

/// The five executable shapes, resolved.
#[derive(Debug, Clone)]
pub enum PreflightAction {
    /// Compose and execute a referenced document.
    Prompt {
        /// The resolved, canonical document path.
        path: PathBuf,
        /// The reference as authored, retained for diagnostics.
        reference: String,
    },
    /// Run one or more commands. These are resolved bytes: approved == executed.
    Shell {
        /// The resolved commands, in declaration order.
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
}

/// A resolved group and its member tasks.
#[derive(Debug, Clone)]
pub struct PreflightGroup {
    /// The group's name, or `<inline>` for an unnamed inline group.
    pub name: String,
    /// Serial (the default) or parallel.
    pub execution: GroupExecution,
    /// Optional concurrency cap; only ever set on a parallel group.
    pub max_parallel: Option<usize>,
    /// Group-scoped variables exposed to member tasks on `group.*`.
    pub variables: Map<String, Value>,
    /// Member tasks in declaration order.
    pub tasks: Vec<PreflightTask>,
    /// The document the group was authored in.
    pub origin_path: PathBuf,
}

/// How a group's tasks are scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupExecution {
    /// Declaration order, one at a time.
    Serial,
    /// Concurrent, bounded by `max_parallel` when set.
    Parallel,
}

/// Collect every `shell` action command from a raw `setup:`/`teardown:` stack.
///
/// The stack is walked as raw JSON rather than through the lifecycle parser
/// because preflight wants *potential* commands: an entry is collected whether
/// or not its `when:` guard holds, and whether it is written positionally
/// (`shell: "…"`) or in key/value form (`{action: shell, command: "…"}`), plus
/// any `on_error:` companion.
pub fn collect_stack_shell_commands(stack: &Value) -> Vec<String> {
    let mut commands = Vec::new();
    collect_from_value(stack, &mut commands);
    commands
}

fn collect_from_value(value: &Value, out: &mut Vec<String>) {
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

fn collect_from_object(map: &Map<String, Value>, out: &mut Vec<String>) {
    // Positional form: `shell: "cmd"` or `shell: ["a", "b"]`.
    if let Some(shell) = map.get("shell") {
        push_strings(shell, out);
    }
    // Key/value form: `{action: shell, command: "cmd", on_error: "cmd"}`.
    if map.get("action").and_then(Value::as_str) == Some("shell") {
        if let Some(command) = map.get("command") {
            push_strings(command, out);
        }
        if let Some(on_error) = map.get("on_error") {
            push_strings(on_error, out);
        }
    }
    // Nested stacks: `action:` lists, `when:`-guarded entries, `on_error:` stacks.
    for (key, value) in map {
        if key == "shell" || key == "command" {
            continue;
        }
        collect_from_value(value, out);
    }
}

fn push_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
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
/// would expand a pre-selection value, and sequence preflight resolves bytes
/// once — execution runs exactly those bytes — so the reference is rejected
/// rather than expanded. The bare late-binding roots are the binding catalog's
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

