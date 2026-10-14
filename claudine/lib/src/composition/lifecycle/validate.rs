/// Rejects a `{{ … }}` span authored inside a quoted string literal on a
/// single-pass lifecycle surface.
///
/// Covers every surface Claudine evaluates exactly once: `loop.while` /
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
/// - [`CompositionError::LifecycleNestedSpanInLiteral`] for the first defect.
/// - [`CompositionError::LifecycleInvalid`] when a parsed stack surface has no
///   authored source record — an internal inconsistency between the parser and
///   the source map, never a silent skip.
pub fn validate_no_nested_spans_in_literals(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let sources = LifecycleSourceMap::from_frontmatter(frontmatter);
    let surfaces = event_surfaces(lifecycle);
    for signal in LifecycleSignal::ALL {
        let event = signal.property_name();
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
                    return Err(CompositionError::LifecycleInvalid {
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
) -> Result<(), CompositionError> {
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
    Err(CompositionError::LifecycleNestedSpanInLiteral {
        source_path: source_path.to_path_buf(),
        property: property.to_string(),
        literal: source[literal.clone()].to_string(),
        nested: source[lint.span.clone()].to_string(),
        suggestion: lint.suggestion,
    })
}

/// One expression surface [`walk_stack`] reaches.
#[derive(Clone, Copy)]
pub(crate) struct Surface<'a> {
    /// The parsed expression tree.
    pub(crate) expr: &'a Expr,
    /// A `when` predicate: condition text rather than an action value.
    pub(crate) predicate: bool,
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
pub(crate) fn walk_stack<'a>(
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

/// The first read of a retired `_loop_*` name in any lifecycle stack, with its
/// authored path and the retired name and `loop` member it names.
///
/// Stacks are walked in [`LifecycleSignal::ALL`] order through both branches.
/// A guard is read as the condition it is; an operand that stayed literal text
/// (`"pass {{ _loop_count }}"`) is read through its `{{ … }}` spans, so a quoted
/// literal or plain text naming `_loop_count` is data, never a read.
pub(crate) fn find_removed_loop_read(
    lifecycle: &LifecycleConfig,
) -> Option<(LifecyclePath, (&'static str, &'static str))> {
    for signal in LifecycleSignal::ALL {
        let mut found = None;
        walk_stack(lifecycle.stack(signal), &mut |path, surface| {
            if found.is_some() {
                return;
            }
            let retired = match surface.expr {
                Expr::StringLiteral(text) if !surface.predicate => {
                    bindings::removed_loop_read_in_text(text)
                }
                expr => bindings::removed_loop_read(expr),
            };
            found = retired.map(|retired| (path.clone(), retired));
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Every surface of every configured event, with its owning signal, in
/// [`LifecycleSignal::ALL`] order.
fn event_surfaces(config: &LifecycleConfig) -> Vec<(LifecycleSignal, LifecyclePath, Surface<'_>)> {
    let mut surfaces = Vec::new();
    for signal in LifecycleSignal::ALL {
        walk_stack(config.stack(signal), &mut |path, surface| {
            surfaces.push((signal, path.clone(), surface));
        });
    }
    surfaces
}

/// Validates that no lifecycle expression reads a global its event declares
/// unavailable, in any branch.
///
/// A thin adapter over Darkmatter's passive validation: each event's
/// [`bindings::binding_view`] declares `err` unavailable in `initialize`,
/// `start`, `success`, and `loop` (`claudine.event-has-no-error`), and every
/// authored surface [`walk_stack`] yields, in both branches, is prepared from
/// its authored text and checked against that view. Group membership is not known yet, so
/// `group` is deferred to runtime. `fm.err` reads the document everywhere.
/// Nothing is evaluated and no provider runs.
///
/// A surface whose authored text does not parse is left to the event-time
/// failure that reports it; an unknown function is likewise reported when the
/// event evaluates it.
///
/// ## Errors
///
/// [`CompositionError::LifecycleErrNotAvailable`] for the first surface, in
/// [`LifecycleSignal::ALL`] order, that reads an unavailable global, carrying
/// Darkmatter's typed unavailable-binding error as its source.
pub fn validate_no_err_in_no_error_events(
    frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let sources = LifecycleSourceMap::from_frontmatter(frontmatter);
    let surfaces = event_surfaces(lifecycle);
    for signal in LifecycleSignal::ALL {
        let view = bindings::binding_view(
            bindings::LifecycleScope::Event(signal),
            bindings::GroupMembership::Unknown,
        );
        let mut authored: Vec<(String, &str, AuthoredMode)> = Vec::new();
        for (_, path, surface) in surfaces.iter().filter(|(owner, ..)| *owner == signal) {
            if let Some(AuthoredValue::Text(text)) = sources.get(path) {
                let mode = if surface.predicate {
                    AuthoredMode::Expression(ParseMode::Condition)
                } else {
                    AuthoredMode::InterpolatedValue
                };
                authored.push((path.to_string(), text, mode));
            }
        }
        for (property, text, mode) in authored {
            let Ok(prepared) = prepare_value(&serde_json::Value::String(text.to_string()), mode)
            else {
                continue;
            };
            let unavailable = validate_prepared(&prepared, &view)
                .into_iter()
                .find(|diagnostic| bindings::unavailable_root(&diagnostic.error).is_some());
            if let Some(diagnostic) = unavailable {
                return Err(CompositionError::LifecycleErrNotAvailable {
                    source_path: source_path.to_path_buf(),
                    property,
                    event: signal.property_name().to_string(),
                    source: Some(Box::new(diagnostic.error)),
                });
            }
        }
    }
    Ok(())
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
/// `on_error` commands are also collected because they execute on non-zero
/// exit, and so are the preflight-resolved commands of every `set` shell
/// value. Each property path names the authored location (e.g.
/// `start[1].shell.command`).
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
                let fields = std::iter::once(("command", &shell.command))
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
use darkmatter::markdown::compose::expression::{
    AuthoredMode, ExpressionLintKind, ParseMode, is_whole_value_span, lint_expression,
    prepare_value, validate_prepared,
};
