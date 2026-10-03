/// Rejects a `{{ … }}` span authored inside a quoted string literal on a
/// single-pass lifecycle surface.
///
/// Covers every surface Claudine evaluates exactly once: the whole-value
/// communication fields of all seven events, `loop.while` / `loop.until`, and
/// every surface [`iter_stack_expression_surfaces`] yields — stack `when`
/// predicates, action operands, and `proxy … with` values. A mixed string
/// (`"a {{ … }} b"`) rescans and resolves, and a positional action body is a
/// synthesized literal, so neither is examined. Stack surfaces and loop
/// predicates read authored text from [`LifecycleSourceMap`], never the parsed
/// `Expr` tree (spec D2, Invariant 3); communication fields are already stored
/// as authored strings.
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
    let surfaces = iter_stack_expression_surfaces(lifecycle);
    for signal in LifecycleSignal::ALL {
        let event = signal.property_name();
        if let Some(notification) = lifecycle.get(signal) {
            for (field, value) in notification_comm_fields(notification) {
                if let Some(text) = value {
                    reject_nested_span(source_path, &format!("{event}.{field}"), text, false)?;
                }
            }
        }
        if signal == LifecycleSignal::Loop {
            for predicate in ["while", "until"] {
                let path = LifecycleSurfacePath::root(event).field(predicate);
                if let Some(AuthoredValue::Text(text)) = sources.get(&path) {
                    reject_nested_span(source_path, &path.to_string(), text, true)?;
                }
            }
        }
        for surface in surfaces.iter().filter(|surface| surface.signal == signal) {
            match sources.get(&surface.path) {
                Some(AuthoredValue::Text(text)) => {
                    reject_nested_span(
                        source_path,
                        &surface.path.to_string(),
                        text,
                        surface.predicate,
                    )?;
                }
                Some(AuthoredValue::NonText) => {}
                None => {
                    return Err(CompositionError::LifecycleInvalid {
                        property: surface.path.to_string(),
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

/// A single expression surface discovered by [`iter_stack_expression_surfaces`].
struct LifecycleExpressionSurface<'a> {
    /// Structural source identity, rendered as a property path for diagnostics.
    path: LifecycleSurfacePath,
    /// The owning event — used by the `err` static scan to decide whether
    /// `err` references are permitted.
    signal: LifecycleSignal,
    /// A `when` predicate: condition text rather than an action value.
    predicate: bool,
    /// The parsed expression tree.
    expr: &'a Expr,
}

/// Walk every reachable expression surface in every configured lifecycle
/// stack and yield it for scanning.
///
/// Surfaces include:
/// - `stack_item.when` (the condition expression)
/// - communication-action message expressions
/// - shell `command` and `on_error` expressions
/// - side-effect positional arguments
/// - expression-function positional arguments
/// - lifecycle-control action operands (`reason`, `target`, `max_attempts`,
///   `delay`, `message`)
///
/// The iteration order is deterministic: events are walked in
/// [`LifecycleSignal::ALL`] order, stack items in array order, actions in
/// execution order.
fn iter_stack_expression_surfaces<'a>(
    config: &'a LifecycleConfig,
) -> Vec<LifecycleExpressionSurface<'a>> {
    let mut surfaces = Vec::new();
    for signal in LifecycleSignal::ALL {
        let Some(stack) = config.stack(signal) else {
            continue;
        };
        let event_path = LifecycleSurfacePath::root(signal.property_name());
        for (idx, item) in stack.iter().enumerate() {
            let prefix = event_path.field("stack").index(idx);
            if let Some(when) = &item.when {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("when"),
                    signal,
                    predicate: true,
                    expr: when,
                });
            }
            for (action_idx, action) in item.actions.iter().enumerate() {
                let action_prefix = prefix.field("action").index(action_idx);
                iter_action_expressions(&action.kind, &action_prefix, signal, &mut surfaces);
            }
        }
    }
    surfaces
}

/// Walk every parsed expression inside a single action body and append to
/// `surfaces`.
fn iter_action_expressions<'a>(
    kind: &'a LifecycleActionKind,
    prefix: &LifecycleSurfacePath,
    signal: LifecycleSignal,
    surfaces: &mut Vec<LifecycleExpressionSurface<'a>>,
) {
    match kind {
        LifecycleActionKind::LifecycleControl(control) => match control {
            LifecycleControlAction::Error { reason } => {
                if let Some(reason) = reason {
                    surfaces.push(LifecycleExpressionSurface {
                        path: prefix.field("reason"),
                        signal,
                        predicate: false,
                        expr: reason,
                    });
                }
            }
            LifecycleControlAction::Proxy { target, with } => {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("target"),
                    signal,
                    predicate: false,
                    expr: target,
                });
                // `with` values resolve at the source handoff against the same
                // event-time surface as every other operand, so they are scoped
                // by the same static scans — an `err` reference in a `with:`
                // value on a no-error event is the same authoring fault as one
                // in `target`.
                for (key, value) in with.iter() {
                    iter_with_value_expressions(
                        value,
                        &prefix.field("with").map_key(key),
                        signal,
                        surfaces,
                    );
                }
            }
            LifecycleControlAction::Retry {
                max_attempts,
                delay,
                ..
            } => {
                if let Some(max_attempts) = max_attempts {
                    surfaces.push(LifecycleExpressionSurface {
                        path: prefix.field("max_attempts"),
                        signal,
                        predicate: false,
                        expr: max_attempts,
                    });
                }
                if let Some(delay) = delay {
                    surfaces.push(LifecycleExpressionSurface {
                        path: prefix.field("delay"),
                        signal,
                        predicate: false,
                        expr: delay,
                    });
                }
            }
            LifecycleControlAction::Resume {
                message,
                max_attempts,
            } => {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("message"),
                    signal,
                    predicate: false,
                    expr: message,
                });
                if let Some(max_attempts) = max_attempts {
                    surfaces.push(LifecycleExpressionSurface {
                        path: prefix.field("max_attempts"),
                        signal,
                        predicate: false,
                        expr: max_attempts,
                    });
                }
            }
            LifecycleControlAction::Defer { delay, reason } => {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("delay"),
                    signal,
                    predicate: false,
                    expr: delay,
                });
                if let Some(reason) = reason {
                    surfaces.push(LifecycleExpressionSurface {
                        path: prefix.field("reason"),
                        signal,
                        predicate: false,
                        expr: reason,
                    });
                }
            }
            LifecycleControlAction::Stop | LifecycleControlAction::Skip => {}
        },
        LifecycleActionKind::Communication(comm) => {
            surfaces.push(LifecycleExpressionSurface {
                path: prefix.field("message"),
                signal,
                predicate: false,
                expr: &comm.message,
            });
            if let Some(route) = &comm.route {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("route"),
                    signal,
                    predicate: false,
                    expr: route,
                });
            }
        }
        LifecycleActionKind::Shell(shell) => {
            surfaces.push(LifecycleExpressionSurface {
                path: prefix.field("command"),
                signal,
                predicate: false,
                expr: &shell.command,
            });
            if let Some(on_error) = &shell.on_error {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("on_error"),
                    signal,
                    predicate: false,
                    expr: on_error,
                });
            }
        }
        LifecycleActionKind::SideEffect(effect) => {
            for (i, arg) in effect.args.iter().enumerate() {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("arg").index(i),
                    signal,
                    predicate: false,
                    expr: arg,
                });
            }
        }
        LifecycleActionKind::RuntimeSet(set) => {
            for (key, value) in set.iter() {
                iter_with_value_expressions(
                    value,
                    &prefix.field("set").map_key(key),
                    signal,
                    surfaces,
                );
            }
        }
        LifecycleActionKind::ExpressionFunction(func) => {
            for (i, arg) in func.args.iter().enumerate() {
                surfaces.push(LifecycleExpressionSurface {
                    path: prefix.field("arg").index(i),
                    signal,
                    predicate: false,
                    expr: arg,
                });
            }
        }
    }
}

/// Append every expression leaf of one `proxy.with` value tree, naming each by
/// its path below the overlay key (e.g. `…with.metadata.area`, `…with.files[0]`).
fn iter_with_value_expressions<'a>(
    value: &'a ProxyWithValue,
    prefix: &LifecycleSurfacePath,
    signal: LifecycleSignal,
    surfaces: &mut Vec<LifecycleExpressionSurface<'a>>,
) {
    match value {
        // A shell value's `{{ … }}` spans are resolved, and late-binding roots
        // refused, when preflight fixes its command bytes.
        ProxyWithValue::Null | ProxyWithValue::Shell(_) => {}
        ProxyWithValue::Scalar(expr) => surfaces.push(LifecycleExpressionSurface {
            path: prefix.clone(),
            signal,
            predicate: false,
            expr,
        }),
        ProxyWithValue::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                iter_with_value_expressions(item, &prefix.index(i), signal, surfaces);
            }
        }
        ProxyWithValue::Object(map) => {
            for (key, item) in map {
                iter_with_value_expressions(item, &prefix.map_key(key), signal, surfaces);
            }
        }
    }
}

/// Validates that no lifecycle expression reads a global its event declares
/// unavailable, in any branch.
///
/// A thin adapter over Darkmatter's passive validation: each event's
/// [`bindings::binding_view`] declares `err` unavailable in `initialize`,
/// `start`, `success`, and `loop` (`claudine.event-has-no-error`), and every
/// authored surface — top-level communication fields and the stack surfaces
/// [`iter_stack_expression_surfaces`] yields — is prepared from its authored
/// text and checked against that view. Group membership is not known yet, so
/// `group` is deferred to runtime. `doc.err` reads the document everywhere.
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
    let surfaces = iter_stack_expression_surfaces(lifecycle);
    for signal in LifecycleSignal::ALL {
        let view = bindings::binding_view(
            bindings::LifecycleScope::Event(signal),
            bindings::GroupMembership::Unknown,
        );
        let mut authored: Vec<(String, &str, AuthoredMode)> = Vec::new();
        if let Some(notification) = lifecycle.get(signal) {
            for (field, value) in notification_comm_fields(notification) {
                if let Some(text) = value {
                    authored.push((
                        format!("{}.{field}", signal.property_name()),
                        text,
                        AuthoredMode::InterpolatedValue,
                    ));
                }
            }
        }
        for surface in surfaces.iter().filter(|surface| surface.signal == signal) {
            if let Some(AuthoredValue::Text(text)) = sources.get(&surface.path) {
                let mode = if surface.predicate {
                    AuthoredMode::Expression(ParseMode::Condition)
                } else {
                    AuthoredMode::InterpolatedValue
                };
                authored.push((surface.path.to_string(), text, mode));
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
/// Returns `(command_expr, property_path)` pairs in deterministic order:
/// events in [`LifecycleSignal::ALL`] order, stack items in array order,
/// actions in execution order. The `command_expr` is the parsed expression
/// for the shell `command` field, which the caller renders to a string
/// (static literals are pre-known; expression-driven commands are also
/// gathered condition-blind, matching the existing template `::shell`
/// audit posture).
///
/// `on_error` commands are also collected because they execute on
/// non-zero exit, and so are the preflight-resolved commands of every `set`
/// shell value. Each entry's property path names the source location
/// (e.g. `start.stack[1].action.command`).
pub fn collect_lifecycle_shell_commands(
    lifecycle: &LifecycleConfig,
) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if let Some(literal) = expr_as_string_literal(surface.expr) {
            let property = surface.path.to_string();
            if property.ends_with(".command") || property.ends_with(".on_error") {
                commands.push((literal, property));
            }
        }
    }
    commands.extend(collect_set_shell_commands(lifecycle));
    commands
}

/// The commands every lifecycle `set` shell value can run, one entry per
/// chained command of each reachable pipeline, named by the value's property
/// (e.g. `start.stack[0].action[1].set.sha`).
///
/// Only a value preflight resolved has known bytes; an unresolved one
/// contributes nothing and refuses to run.
fn collect_set_shell_commands(lifecycle: &LifecycleConfig) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    for signal in LifecycleSignal::ALL {
        let Some(stack) = lifecycle.stack(signal) else {
            continue;
        };
        for (index, item) in stack.iter().enumerate() {
            for (action_index, action) in item.actions.iter().enumerate() {
                let LifecycleActionKind::RuntimeSet(set) = &action.kind else {
                    continue;
                };
                for (key, shell) in set.shell_values() {
                    let Some(resolved) = shell.resolved.as_ref() else {
                        continue;
                    };
                    let property = format!(
                        "{}.stack[{index}].action[{action_index}].set.{key}",
                        signal.property_name()
                    );
                    for command in resolved.commands() {
                        commands.push((command, property.clone()));
                    }
                }
            }
        }
    }
    commands
}

/// [`collect_lifecycle_shell_commands`], restricted to `signals`.
///
/// The property path a surface carries is rooted at its event
/// (`initialize.stack[0].action[1].command`), so the event name is the filter.
pub fn collect_lifecycle_shell_commands_for(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
) -> Vec<(String, String)> {
    let prefixes: Vec<String> = signals
        .iter()
        .map(|signal| format!("{}.", signal.property_name()))
        .collect();
    collect_lifecycle_shell_commands(lifecycle)
        .into_iter()
        .filter(|(_, property)| {
            prefixes
                .iter()
                .any(|prefix| property.starts_with(prefix.as_str()))
        })
        .collect()
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

/// Normalizes empty or whitespace-only strings to `None`.
use super::*;
use super::actions::ProxyWithValue;
use super::source_map::{AuthoredValue, LifecycleSourceMap, LifecycleSurfacePath};
use darkmatter::markdown::compose::expression::{
    AuthoredMode, ExpressionLintKind, ParseMode, is_whole_value_span, lint_expression,
    prepare_value, validate_prepared,
};
