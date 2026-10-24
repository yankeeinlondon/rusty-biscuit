/// Rejects a `{{ … }}` span authored inside a quoted string literal on a
/// single-pass lifecycle surface.
///
/// Covers every surface a host evaluates exactly once: `loop.while` /
/// `loop.until`, and every surface [`walk_stack`] yields — `when` predicates
/// in both branches, action operands, and `set`/`with` values. A mixed string
/// (`"a {{ … }} b"`) rescans and resolves, and a short-form action body is a
/// synthesized literal, so neither is examined. Surfaces read authored text
/// from [`LifecycleSourceMap`], never the parsed `Expr` tree (spec D2,
/// Invariant 3).
///
/// Events are walked in [`LifecycleSignal::ALL`] order and the first lint
/// aborts.
///
/// ## Errors
///
/// - [`LifecycleError::LifecycleNestedSpanInLiteral`] for the first defect.
/// - [`LifecycleError::LifecycleInvalid`] when a parsed stack surface has no
///   authored source record — an internal inconsistency between the parser and
///   the source map, never a silent skip.
pub fn validate_no_nested_spans_in_literals(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), LifecycleError> {
    let sources = LifecycleSourceMap::from_frontmatter(frontmatter);
    reject_nested_spans(&sources, lifecycle, source_path, |signal| Some(signal.property_name()))
}

/// [`validate_no_nested_spans_in_literals`] for a lifecycle parsed under
/// `registry`, whose events carry the host's names.
///
/// ## Errors
///
/// As [`validate_no_nested_spans_in_literals`].
pub fn validate_no_nested_spans_in_literals_in(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
    registry: &super::registry::EventRegistry,
) -> Result<(), LifecycleError> {
    let sources = LifecycleSourceMap::from_frontmatter_in(frontmatter, registry);
    reject_nested_spans(&sources, lifecycle, source_path, |signal| registry.name_of(signal))
}

fn reject_nested_spans<'n>(
    sources: &LifecycleSourceMap,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
    name: impl Fn(LifecycleSignal) -> Option<&'n str>,
) -> Result<(), LifecycleError> {
    let surfaces = event_surfaces(lifecycle);
    for signal in LifecycleSignal::ALL {
        let Some(event) = name(signal) else {
            continue;
        };
        if signal == LifecycleSignal::Loop {
            for predicate in ["while", "until"] {
                let path = LifecyclePath::root(event).field(predicate);
                if let Some(AuthoredValue::Text(text)) = sources.get(&path) {
                    reject_nested_span(source_path, &path.to_string(), text, true)?;
                }
            }
        }
        for (_, path, surface) in surfaces.iter().filter(|(owner, ..)| *owner == signal) {
            match sources.get(path) {
                Some(AuthoredValue::Text(text)) => {
                    reject_nested_span(source_path, &path.to_string(), text, surface.predicate)?;
                }
                Some(AuthoredValue::NonText) => {}
                None => {
                    return Err(LifecycleError::LifecycleInvalid {
                        property: path.to_string(),
                        message: "internal error: no authored source was recorded for this \
                                  lifecycle surface, so it cannot be checked for nested \
                                  interpolation"
                            .to_string(),
                        source_file: source_path.to_path_buf(),
                        unknown_field: None,
                        expected_fields: Vec::new(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Lint one authored scalar: a predicate is condition text as a whole; any
/// other surface is examined only when it is exactly one `{{ … }}` span.
fn reject_nested_span(
    source_path: &Path,
    property: &str,
    text: &str,
    predicate: bool,
) -> Result<(), LifecycleError> {
    let (source, mode) = if predicate {
        (text.to_string(), ParseMode::Condition)
    } else {
        if !is_whole_value_span(text) {
            return Ok(());
        }
        let Some(span) = ExpressionFinder::find_all_plain(text).into_iter().next() else {
            return Ok(());
        };
        (span.expression, ParseMode::Interpolation)
    };
    let Some(lint) = lint_expression(&source, mode).into_iter().next() else {
        return Ok(());
    };
    let ExpressionLintKind::NestedSpanInStringLiteral { literal, .. } = &lint.kind;
    Err(LifecycleError::LifecycleNestedSpanInLiteral {
        source_path: source_path.to_path_buf(),
        property: property.to_string(),
        literal: source[literal.clone()].to_string(),
        nested: source[lint.span.clone()].to_string(),
        suggestion: lint.suggestion,
    })
}

/// Rejects the first lifecycle call the design-time check proves wrong
/// (category 5), exactly as composition rejects the same call in the body.
///
/// Every surface [`walk_stack`] yields is checked, `when` predicates and
/// operands alike, in both branches of every conditional, from its authored
/// text. Its expressions are typed by the declared layer
/// [`lifecycle_declared_types`](super::shell_template::lifecycle_declared_types)
/// builds from `schema` and `registry`, with each event's globals as the
/// event declares them: an unavailable global has no type. Passive: nothing
/// is evaluated and no handler runs.
///
/// Guards are not applied. A guard only removes alternatives from a type, so
/// it never turns a call no value can satisfy into one some value can.
///
/// ## Errors
///
/// [`LifecycleError::LifecycleCallRejected`] for the first rejected call, in
/// [`LifecycleSignal::ALL`] order and then authored order.
pub fn reject_incompatible_calls(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
    registry: &super::registry::EventRegistry,
    schema: Option<&crate::markdown::schemas::EffectiveSchema>,
) -> Result<(), LifecycleError> {
    let sources = LifecycleSourceMap::from_frontmatter_in(frontmatter, registry);
    let declared = super::shell_template::lifecycle_declared_types(schema, registry);
    for (signal, path, surface) in event_surfaces(lifecycle) {
        let Some(AuthoredValue::Text(text)) = sources.get(&path) else {
            continue;
        };
        let mode = if surface.predicate {
            AuthoredMode::Expression(ParseMode::Condition)
        } else {
            AuthoredMode::InterpolatedValue
        };
        // A surface that does not parse is the parser's to report.
        let Ok(prepared) = prepare_value(&serde_json::Value::String(text.to_string()), mode) else {
            continue;
        };
        let event = match registry.id_of(signal) {
            Some(id) => declared.with_binding_view(registry.globals(id)),
            None => declared,
        };
        let rejected = check_prepared_calls(&prepared, &TypeScope::unguarded(&event))
            .into_iter()
            .find(|found| found.check.is_rejected());
        if let Some(found) = rejected {
            return Err(LifecycleError::LifecycleCallRejected {
                source_path: source_path.to_path_buf(),
                property: path.to_string(),
                expression: found.expression,
                source: Box::new(ExpressionError::CallCheck(Box::new(found.check))),
            });
        }
    }
    Ok(())
}

/// One expression surface [`walk_stack`] reaches.
#[derive(Clone, Copy)]
pub struct Surface<'a> {
    /// The parsed expression tree.
    pub expr: &'a Expr,
    /// A `when` predicate: condition text rather than an action value.
    pub predicate: bool,
}

/// Visit every expression surface of a stack, in authored order, through both
/// branches of every conditional.
///
/// This is the one walker every static check shares — nested-span lints,
/// `err` availability, binding validation, retired `_loop_*` reads, and shell
/// collection — so an
/// expression inside a branch, a `set` value, or a `with` value is never
/// missed. Each surface is named by its authored path: a guard is
/// `{item}.when`; an operand hangs off its action's verb path
/// (`failure[0].else[1].shell.command`, `start[0].say.text`,
/// `success[0].set.key`, `failure[0].retry.with.reason`).
pub fn walk_stack<'a>(
    nodes: &'a [LifecycleNode],
    visit: &mut impl FnMut(&LifecyclePath, Surface<'a>),
) {
    for node in nodes {
        match node {
            LifecycleNode::Action(action) => walk_action(&action.kind, &action.path, visit),
            LifecycleNode::Conditional(conditional) => {
                visit(
                    &conditional.path.field("when"),
                    Surface {
                        expr: &conditional.when,
                        predicate: true,
                    },
                );
                walk_stack(&conditional.then, visit);
                walk_stack(&conditional.otherwise, visit);
            }
        }
    }
}

/// Visit every operand of one action body under its verb path `prefix`.
fn walk_action<'a>(
    kind: &'a LifecycleActionKind,
    prefix: &LifecyclePath,
    visit: &mut impl FnMut(&LifecyclePath, Surface<'a>),
) {
    let mut operand = |path: LifecyclePath, expr: &'a Expr| {
        visit(
            &path,
            Surface {
                expr,
                predicate: false,
            },
        );
    };
    match kind {
        LifecycleActionKind::LifecycleControl(control) => match control {
            LifecycleControlAction::Error { reason } => {
                if let Some(reason) = reason {
                    operand(prefix.field("reason"), reason);
                }
            }
            LifecycleControlAction::Proxy { target, with } => {
                operand(prefix.field("target"), target);
                // `with` values resolve when the directive fires, against the
                // same event-time surface as every other operand, so they are
                // scoped by the same static scans.
                for (key, value) in with.iter() {
                    walk_with_value(value, &prefix.field("with").map_key(key.as_str()), &mut operand);
                }
            }
            LifecycleControlAction::Retry {
                max_attempts,
                delay,
                with,
                ..
            } => {
                if let Some(max_attempts) = max_attempts {
                    operand(prefix.field("max_attempts"), max_attempts);
                }
                if let Some(delay) = delay {
                    operand(prefix.field("delay"), delay);
                }
                for (key, value) in with.iter() {
                    walk_with_value(value, &prefix.field("with").map_key(key.as_str()), &mut operand);
                }
            }
            LifecycleControlAction::Resume {
                message,
                max_attempts,
            } => {
                operand(prefix.field("message"), message);
                if let Some(max_attempts) = max_attempts {
                    operand(prefix.field("max_attempts"), max_attempts);
                }
            }
            LifecycleControlAction::Defer { delay, reason } => {
                operand(prefix.field("delay"), delay);
                if let Some(reason) = reason {
                    operand(prefix.field("reason"), reason);
                }
            }
            LifecycleControlAction::Break { reason, .. } => {
                if let Some(reason) = reason {
                    operand(prefix.field("reason"), reason);
                }
            }
            LifecycleControlAction::Prep { target, with } => {
                operand(prefix.field("target"), target);
                for (key, value) in with.iter() {
                    walk_with_value(value, &prefix.field("with").map_key(key.as_str()), &mut operand);
                }
            }
            LifecycleControlAction::Stop | LifecycleControlAction::Skip => {}
        },
        LifecycleActionKind::Communication(comm) => operand(prefix.field("text"), &comm.text),
        LifecycleActionKind::Shell(shell) => {
            operand(prefix.field("command"), &shell.command);
            if let Some(on_error) = &shell.on_error {
                operand(prefix.field("on_error"), on_error);
            }
        }
        LifecycleActionKind::SideEffect(SideEffectAction { args, .. })
        | LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction { args, .. }) => {
            for (i, arg) in args.iter().enumerate() {
                operand(prefix.field("arg").index(i), arg);
            }
        }
        LifecycleActionKind::RuntimeSet(set) => {
            for (key, value) in set.iter() {
                walk_with_value(value, &prefix.map_key(key.as_str()), &mut operand);
            }
        }
        // An ignored verb's operand was never parsed, so it has no surface.
        LifecycleActionKind::Ignored(_) => {}
    }
}

/// Visit every expression leaf of one `set`/`with` value tree, naming each by
/// its path below the key (e.g. `…with.metadata.area`, `…with.files[0]`).
fn walk_with_value<'a>(
    value: &'a WithValue,
    prefix: &LifecyclePath,
    operand: &mut impl FnMut(LifecyclePath, &'a Expr),
) {
    match value {
        // A shell value's `{{ … }}` spans are resolved, and late-binding roots
        // refused, when preflight fixes its command bytes.
        WithValue::Null | WithValue::Shell(_) => {}
        WithValue::Scalar(expr) => operand(prefix.clone(), expr),
        WithValue::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk_with_value(item, &prefix.index(i), operand);
            }
        }
        WithValue::Object(map) => {
            for (key, item) in map {
                walk_with_value(item, &prefix.map_key(key.as_str()), operand);
            }
        }
    }
}

/// Every surface of every configured event, with its owning signal, in
/// [`LifecycleSignal::ALL`] order.
pub fn event_surfaces(config: &LifecycleConfig) -> Vec<(LifecycleSignal, LifecyclePath, Surface<'_>)> {
    let mut surfaces = Vec::new();
    for signal in LifecycleSignal::ALL {
        walk_stack(config.stack(signal), &mut |path, surface| {
            surfaces.push((signal, path.clone(), surface));
        });
    }
    surfaces
}

/// Collects the shell commands reachable from every lifecycle stack, for
/// inclusion in the pre-flight shell whitelist audit.
///
/// Returns `(command, property_path)` pairs in deterministic order: events in
/// [`LifecycleSignal::ALL`] order, then authored order through both branches
/// of every conditional — a guard never hides a command from approval. Only
/// commands with statically known text are collected; an expression-driven
/// command depends on runtime state preflight cannot see.
///
/// `on_error` warning text retains its existing audit behavior; it is not an
/// executed command. Also collected are the preflight-resolved commands of every `set` shell
/// value. A command whose resolved bytes no longer read as a command (a value
/// inserted command syntax) is not collected: it may run only as its template,
/// which approval sees as a site. Each property path names the authored
/// location (e.g. `start[1].shell.command`).
pub fn collect_lifecycle_shell_commands(lifecycle: &LifecycleConfig) -> Vec<(String, String)> {
    collect_lifecycle_shell_commands_for(lifecycle, &LifecycleSignal::ALL)
}

/// [`collect_lifecycle_shell_commands`], restricted to `signals`.
pub fn collect_lifecycle_shell_commands_for(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    for signal in LifecycleSignal::ALL.into_iter().filter(|signal| signals.contains(signal)) {
        for_each_action(lifecycle.stack(signal), &mut |action| match &action.kind {
            LifecycleActionKind::Shell(shell) => {
                let template_only = shell.template.as_ref().is_some_and(|site| site.resolved().is_none());
                let fields = (!template_only)
                    .then_some(("command", &shell.command))
                    .into_iter()
                    .chain(shell.on_error.as_ref().map(|on_error| ("on_error", on_error)));
                for (field, expr) in fields {
                    if let Some(literal) = expr_as_string_literal(expr) {
                        commands.push((literal, action.path.field(field).to_string()));
                    }
                }
            }
            // Only a value preflight resolved has known bytes; an unresolved
            // one contributes nothing and refuses to run.
            LifecycleActionKind::RuntimeSet(set) => {
                for (key, shell) in set.shell_values() {
                    let Some(resolved) = shell.resolved.as_ref() else {
                        continue;
                    };
                    let property = action.path.map_key(key.as_str()).to_string();
                    for command in resolved.commands() {
                        commands.push((command, property.clone()));
                    }
                }
            }
            _ => {}
        });
    }
    commands
}

/// Render an [`Expr`] to its literal string value when it is a string
/// literal, a bare variable, or a number/bool literal. `None` otherwise
/// (complex expressions are not collected — they depend on runtime state
/// not visible at pre-flight time).
fn expr_as_string_literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::StringLiteral(s) => Some(s.clone()),
        Expr::Variable(v) => Some(v.clone()),
        Expr::NumberLiteral(n) => Some(n.to_string()),
        Expr::BoolLiteral(b) => Some(b.to_string()),
        _ => None,
    }
}

use super::*;
use super::actions::{WithValue, for_each_action};
use super::source_map::{AuthoredValue, LifecycleSourceMap};
use crate::markdown::compose::expression::{
    AuthoredMode, ExpressionError, ExpressionLintKind, ParseMode, TypeScope, check_prepared_calls,
    is_whole_value_span, lint_expression, prepare_value,
};
