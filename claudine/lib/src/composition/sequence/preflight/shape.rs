//! Preflight node shapes and the raw-value traversals that feed them.
//!
//! The node types here are deliberately inert: they describe *what could run*,
//! not how. Execution consumes them; preflight only fills them in.

use std::path::PathBuf;

use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, parse};
use serde_json::{Map, Value};

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

/// The first root in `roots` referenced by any `{{ … }}` span of `raw`.
///
/// `doc.*` is exempt from the walk for the same reason the lifecycle pre-flight
/// exempts it: it reaches a literal frontmatter property, not a global.
pub fn first_unavailable_root(raw: &str, roots: &[&str]) -> Option<String> {
    for location in ExpressionFinder::find_all_plain(raw) {
        let Ok(expr) = parse(&location.expression) else {
            continue;
        };
        if let Some(root) = root_in_expr(&expr, roots) {
            return Some(root);
        }
    }
    None
}

fn root_in_expr(expr: &Expr, roots: &[&str]) -> Option<String> {
    match expr {
        Expr::Variable(path) => {
            let root = path.split('.').next().unwrap_or(path);
            roots.contains(&root).then(|| root.to_string())
        }
        Expr::MemberAccess { base, .. } => {
            if let Expr::Variable(base_path) = base.as_ref() {
                let root = base_path.split('.').next().unwrap_or(base_path);
                if root == "doc" {
                    return None;
                }
            }
            root_in_expr(base, roots)
        }
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            root_in_expr(inner, roots)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            root_in_expr(left, roots).or_else(|| root_in_expr(right, roots))
        }
        Expr::Index { base, index } => {
            root_in_expr(base, roots).or_else(|| root_in_expr(index, roots))
        }
        Expr::FunctionCall { args, .. } => args.iter().find_map(|a| root_in_expr(a, roots)),
        Expr::Fallback { primary, fallback } => {
            root_in_expr(primary, roots).or_else(|| root_in_expr(fallback, roots))
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => root_in_expr(condition, roots)
            .or_else(|| root_in_expr(then_branch, roots))
            .or_else(|| root_in_expr(else_branch, roots)),
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
    }
}
