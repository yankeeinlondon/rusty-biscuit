pub fn validate_no_interpolation_leaks(
    config: &LifecycleConfig,
    source_path: &Path,
    warnings: &[darkmatter::markdown::compose::ComposeWarning],
) -> Result<(), CompositionError> {
    for signal in LifecycleSignal::ALL {
        let Some(notification) = config.get(signal) else {
            continue;
        };

        for (field_name, value) in notification_comm_fields(notification) {
            let Some(text) = value else { continue };
            if text.is_empty() {
                continue;
            }

            let spans = ExpressionFinder::find_all_plain(text);
            if let Some(first) = spans.first() {
                let property = format!("{}.{}", signal.property_name(), field_name);
                let expression = first.expression.clone();
                let reason = find_matching_warning_reason(&expression, warnings);
                return Err(CompositionError::LifecycleInterpolationLeak {
                    source_path: source_path.to_path_buf(),
                    property,
                    expression,
                    reason,
                });
            }
        }
    }

    // Stack expression surfaces: scan string literals inside parsed Expr
    // trees for surviving `{{ … }}` spans. A string literal in a parsed
    // expression is passed through verbatim to the evaluated result, so a
    // literal containing template syntax would leak the raw braces into
    // user-visible output.
    for surface in iter_stack_expression_surfaces(config) {
        let mut found: Option<(String, String)> = None;
        visit_string_literals(surface.expr, &mut |literal| {
            if found.is_some() {
                return;
            }
            if let Some(span) = ExpressionFinder::find_all_plain(literal).first() {
                found = Some((span.expression.clone(), literal.to_string()));
            }
        });
        if let Some((expression, _literal)) = found {
            let reason = find_matching_warning_reason(&expression, warnings);
            return Err(CompositionError::LifecycleInterpolationLeak {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                expression,
                reason,
            });
        }
    }

    Ok(())
}

/// Best-effort extraction of a warning reason mentioning the leaked expression.
fn find_matching_warning_reason(
    expression: &str,
    warnings: &[darkmatter::markdown::compose::ComposeWarning],
) -> String {
    let inner = expression
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();

    for warning in warnings {
        if warning.message.contains(expression) || warning.message.contains(inner) {
            return warning.message.clone();
        }
    }

    String::new()
}

/// A single expression surface discovered by [`iter_stack_expression_surfaces`].
struct LifecycleExpressionSurface<'a> {
    /// Dotted property path for diagnostics, e.g. `start.stack[1].when` or
    /// `failure.stack[0].action`.
    property: String,
    /// The owning event — used by the `err` static scan to decide whether
    /// `err` references are permitted.
    signal: LifecycleSignal,
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
        let event_name = signal.property_name();
        for (idx, item) in stack.iter().enumerate() {
            let prefix = format!("{event_name}.stack[{idx}]");
            if let Some(when) = &item.when {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.when"),
                    signal,
                    expr: when,
                });
            }
            for (action_idx, action) in item.actions.iter().enumerate() {
                let action_prefix = format!("{prefix}.action[{action_idx}]");
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
    prefix: &str,
    signal: LifecycleSignal,
    surfaces: &mut Vec<LifecycleExpressionSurface<'a>>,
) {
    match kind {
        LifecycleActionKind::LifecycleControl(control) => match control {
            LifecycleControlAction::Error { reason } => {
                if let Some(reason) = reason {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.reason"),
                        signal,
                        expr: reason,
                    });
                }
            }
            LifecycleControlAction::Proxy { target, with } => {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.target"),
                    signal,
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
                        &format!("{prefix}.with.{key}"),
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
                        property: format!("{prefix}.max_attempts"),
                        signal,
                        expr: max_attempts,
                    });
                }
                if let Some(delay) = delay {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.delay"),
                        signal,
                        expr: delay,
                    });
                }
            }
            LifecycleControlAction::Resume {
                message,
                max_attempts,
            } => {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.message"),
                    signal,
                    expr: message,
                });
                if let Some(max_attempts) = max_attempts {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.max_attempts"),
                        signal,
                        expr: max_attempts,
                    });
                }
            }
            LifecycleControlAction::Defer { delay, reason } => {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.delay"),
                    signal,
                    expr: delay,
                });
                if let Some(reason) = reason {
                    surfaces.push(LifecycleExpressionSurface {
                        property: format!("{prefix}.reason"),
                        signal,
                        expr: reason,
                    });
                }
            }
            LifecycleControlAction::Stop | LifecycleControlAction::Skip => {}
        },
        LifecycleActionKind::Communication(comm) => {
            surfaces.push(LifecycleExpressionSurface {
                property: format!("{prefix}.message"),
                signal,
                expr: &comm.message,
            });
            if let Some(route) = &comm.route {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.route"),
                    signal,
                    expr: route,
                });
            }
        }
        LifecycleActionKind::Shell(shell) => {
            surfaces.push(LifecycleExpressionSurface {
                property: format!("{prefix}.command"),
                signal,
                expr: &shell.command,
            });
            if let Some(on_error) = &shell.on_error {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.on_error"),
                    signal,
                    expr: on_error,
                });
            }
        }
        LifecycleActionKind::SideEffect(effect) => {
            for (i, arg) in effect.args.iter().enumerate() {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.arg[{i}]"),
                    signal,
                    expr: arg,
                });
            }
        }
        LifecycleActionKind::ExpressionFunction(func) => {
            for (i, arg) in func.args.iter().enumerate() {
                surfaces.push(LifecycleExpressionSurface {
                    property: format!("{prefix}.arg[{i}]"),
                    signal,
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
    prefix: &str,
    signal: LifecycleSignal,
    surfaces: &mut Vec<LifecycleExpressionSurface<'a>>,
) {
    match value {
        ProxyWithValue::Null => {}
        ProxyWithValue::Scalar(expr) => surfaces.push(LifecycleExpressionSurface {
            property: prefix.to_string(),
            signal,
            expr,
        }),
        ProxyWithValue::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                iter_with_value_expressions(item, &format!("{prefix}[{i}]"), signal, surfaces);
            }
        }
        ProxyWithValue::Object(map) => {
            for (key, item) in map {
                iter_with_value_expressions(item, &format!("{prefix}.{key}"), signal, surfaces);
            }
        }
    }
}

/// Visit every `Expr::StringLiteral` reachable in the expression tree,
/// depth-first, calling `visitor` with the literal's value.
///
/// Used by the leak scan to detect surviving `{{ … }}` spans inside parsed
/// expression literals (e.g. `say('leaked {{ expr }}')`).
fn visit_string_literals<F: FnMut(&str)>(expr: &Expr, visitor: &mut F) {
    match expr {
        Expr::StringLiteral(s) => visitor(s),
        Expr::Variable(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            visit_string_literals(inner, visitor);
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            visit_string_literals(left, visitor);
            visit_string_literals(right, visitor);
        }
        Expr::Index { base, index } => {
            visit_string_literals(base, visitor);
            visit_string_literals(index, visitor);
        }
        Expr::MemberAccess { base, .. } => visit_string_literals(base, visitor),
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                visit_string_literals(arg, visitor);
            }
        }
        Expr::Fallback { primary, fallback } => {
            visit_string_literals(primary, visitor);
            visit_string_literals(fallback, visitor);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_string_literals(condition, visitor);
            visit_string_literals(then_branch, visitor);
            visit_string_literals(else_branch, visitor);
        }
    }
}

/// Validates that no raw lifecycle string references a bare variable that is
/// undefined after composition, and that no lifecycle stack expression
/// references an undefined bare variable.
///
/// Darkmatter resolves an unknown bare variable to an empty string with no
/// warning and no error — even in fail-fast mode (see
/// `frontmatter_interpolation::missing_variable_resolves_to_empty`). So the
/// post-compose [`validate_no_interpolation_leaks`] guard, which only scans
/// the *rendered* string for surviving spans, never sees the collapsed
/// reference. This guard closes that gap by inspecting the **raw**
/// (pre-composition) lifecycle strings, where the `{{ … }}` span is still
/// present, and resolving each bare variable against the composed frontmatter.
///
/// The stack-expression half walks the parsed `Expr` trees on
/// [`LifecycleConfig::stacks`]. Bare names in stack expressions resolve
/// against the composed frontmatter plus the lifecycle globals
/// (`err`, `timing`, `current`) and the runtime namespaces (`ctx`, `env`,
/// `doc`); a bare name not in any of those is reported as undefined.
///
/// Every bare variable reachable in the parsed expression tree is checked, not
/// just spans that are exactly `{{ variable }}`: a missing operand buried in a
/// function argument (`{{ parent_dir(missing) }}`), comparison, or arithmetic
/// node is rejected the same way a top-level `{{ missing }}` is. A ternary
/// condition (`{{ missing ? 'a' : 'b' }}`) is descended because it is evaluated,
/// but the ternary branch operands and fallback (`{{ x || 'y' }}`) subtrees
/// intentionally tolerate undefined operands, so they are skipped. `ctx.*` /
/// `env.*` / `doc` references resolve from outside the frontmatter and are
/// skipped — a bare name resolves only against top-level frontmatter keys.
///
/// Iterates events in [`LifecycleSignal::ALL`] order and communication fields
/// in [`LIFECYCLE_COMM_FIELDS`] order; the first undefined variable aborts
/// with [`CompositionError::LifecycleUndefinedVariable`].
///
/// ## Arguments
///
/// * `raw_frontmatter` — the pre-composition frontmatter holding the original
///   lifecycle strings (`{{ … }}` spans intact).
/// * `effective_frontmatter` — the composed frontmatter object; a bare
///   variable is "defined" when its root segment is one of these keys.
/// * `lifecycle` — the parsed lifecycle configuration with typed stacks.
/// * `source_path` — prompt file, used for the diagnostic.
pub fn validate_no_undefined_lifecycle_variables(
    raw_frontmatter: &darkmatter::markdown::Frontmatter,
    effective_frontmatter: &serde_json::Value,
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let raw_map = raw_frontmatter.as_map();
    let defined = effective_frontmatter.as_object();

    // Top-level communication fields across all seven events.
    for signal in LifecycleSignal::ALL {
        let Some(serde_json::Value::Object(notification)) = raw_map.get(signal.property_name())
        else {
            continue;
        };

        for field in LIFECYCLE_COMM_FIELDS {
            let Some(serde_json::Value::String(text)) = notification.get(*field) else {
                continue;
            };

            for span in ExpressionFinder::find_all_plain(text) {
                let Ok(expr) = parse(&span.expression) else {
                    continue;
                };
                if let Some(variable) = find_undefined_top_level_variable(&expr, defined) {
                    return Err(CompositionError::LifecycleUndefinedVariable {
                        source_path: source_path.to_path_buf(),
                        property: format!("{}.{}", signal.property_name(), field),
                        variable: variable.to_string(),
                    });
                }
            }
        }
    }

    // Stack expression surfaces: walk parsed Expr trees for bare undefined
    // references. The lifecycle globals (err, timing, current) and runtime
    // namespaces (ctx, env, doc) are always considered defined here —
    // bare `err` misuse in no-error events is caught separately by
    // [`validate_no_err_in_no_error_events`].
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if let Some(variable) = find_undefined_stack_variable(surface.expr, defined) {
            return Err(CompositionError::LifecycleUndefinedVariable {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                variable: variable.to_string(),
            });
        }
    }

    Ok(())
}

/// Recursively walks `expr`, returning the first frontmatter-scoped bare
/// variable whose root key is undefined in the composed frontmatter.
///
/// Used for top-level communication fields. The runtime namespaces
/// (`ctx`/`env`/`doc`) and the lifecycle late-binding globals
/// ([`LATE_BINDING_ROOTS`]: `err`/`timing`/`current`) are known roots — they
/// resolve at event-time, not against frontmatter — so a bare `err`/`timing`/
/// `current` is not flagged. Only genuinely-unknown roots (typos) are reported.
///
/// A ternary condition is descended because it is evaluated during composition,
/// but the ternary branch operands and fallback (`||`) subtrees are not: those
/// forms exist precisely to tolerate an undefined operand, so a miss inside them
/// is intentional, not a leak. Every other node — function-call arguments,
/// comparisons, arithmetic, indexing, member access, unary, parens — is
/// descended so an undefined variable buried in `parent_dir(missing)` is caught
/// like a top-level `{{ missing }}`. The returned reference borrows from `expr`.
fn find_undefined_top_level_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    match expr {
        Expr::Variable(path) => undefined_bare_variable(path, defined),
        // Ternary conditions are evaluated, but the branches intentionally
        // tolerate undefined operands by design.
        Expr::Ternary { condition, .. } => {
            find_undefined_top_level_variable(condition, defined)
        }
        Expr::Fallback { .. } => None,
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            find_undefined_top_level_variable(inner, defined)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            find_undefined_top_level_variable(left, defined)
                .or_else(|| find_undefined_top_level_variable(right, defined))
        }
        Expr::Index { base, index } => {
            find_undefined_top_level_variable(base, defined)
                .or_else(|| find_undefined_top_level_variable(index, defined))
        }
        Expr::MemberAccess { base, .. } => find_undefined_top_level_variable(base, defined),
        Expr::FunctionCall { args, .. } => args
            .iter()
            .find_map(|arg| find_undefined_top_level_variable(arg, defined)),
    }
}

/// Like [`find_undefined_top_level_variable`] but for stack expression
/// surfaces, where the lifecycle globals (`err`, `timing`, `current`) and
/// the runtime namespaces (`ctx`, `env`, `doc`) are always defined.
///
/// Stack `when:` clauses are parsed in condition mode, so `||`/`&&` lower to
/// `or(...)`/`and(...)` function calls rather than `Expr::Fallback`. Those two
/// functions get the same skip-the-operands tolerance a `Fallback` does — they
/// exist to guard an undefined operand.
fn find_undefined_stack_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    match expr {
        Expr::Variable(path) => undefined_stack_variable(path, defined),
        Expr::Ternary { condition, .. } => find_undefined_stack_variable(condition, defined),
        Expr::Fallback { .. } => None,
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            find_undefined_stack_variable(inner, defined)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            find_undefined_stack_variable(left, defined)
                .or_else(|| find_undefined_stack_variable(right, defined))
        }
        Expr::Index { base, index } => {
            find_undefined_stack_variable(base, defined)
                .or_else(|| find_undefined_stack_variable(index, defined))
        }
        Expr::MemberAccess { base, .. } => find_undefined_stack_variable(base, defined),
        // `or`/`and` are the condition-parse-mode (`parse_condition`) lowering of
        // `||`/`&&`. Like an interpolation-mode `Expr::Fallback`, they exist to
        // tolerate an undefined/falsy operand by design (`maybe_missing || false`
        // is a guarded optional, not a typo), so their operands are not scanned.
        Expr::FunctionCall { name, .. } if name == "or" || name == "and" => None,
        Expr::FunctionCall { args, .. } => args
            .iter()
            .find_map(|arg| find_undefined_stack_variable(arg, defined)),
    }
}

/// Returns the first frontmatter-scoped bare variable in `expr` whose root key
/// is undefined, applying the lifecycle-stack tolerance (`ctx`/`env`/`doc` and
/// the late-binding globals `err`/`timing`/`current` are known roots; `||`
/// fallbacks are skipped and only a ternary's condition is descended).
///
/// Exposed for the executor's event-time `when:` guard, which fails closed on a
/// genuinely-unknown root rather than silently treating the guard as false.
/// Returns `None` when every root resolves. The reference borrows from `expr`.
pub(crate) fn first_undefined_stack_variable<'a>(
    expr: &'a Expr,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    find_undefined_stack_variable(expr, defined)
}

/// Whether `path`'s root resolves outside top-level frontmatter — the runtime
/// namespaces (`ctx.*` / `env.*` / `doc`) or a lifecycle late-binding global
/// ([`LATE_BINDING_ROOTS`]: `err`/`timing`/`current`).
///
/// Such a reference is never an undefined *frontmatter* variable, so the
/// undefined scan skips it. A bare `err` *misuse* in a no-error event is caught
/// separately by [`validate_no_err_in_no_error_events`].
fn resolves_outside_frontmatter(path: &str) -> bool {
    if path.starts_with("ctx.")
        || path.starts_with("env.")
        || path == "doc"
        || path.starts_with("doc.")
    {
        return true;
    }
    let root = path.split('.').next().unwrap_or(path);
    LATE_BINDING_ROOTS.contains(&root)
}

/// Returns the bare variable name when `path` is a frontmatter-scoped reference
/// whose root segment is absent from the composed frontmatter, or `None` when
/// it resolves elsewhere ([`resolves_outside_frontmatter`]) or its root key
/// exists.
///
/// Nested misses (`{{ a.b }}` where `a` exists but `b` does not) are treated as
/// defined: only the bare-root contract the spec describes is enforced.
pub(super) fn undefined_bare_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    if resolves_outside_frontmatter(path) {
        return None;
    }
    let root = path.split('.').next().unwrap_or(path);
    if root.is_empty() {
        return None;
    }
    match defined {
        Some(map) if map.contains_key(root) => None,
        _ => Some(root),
    }
}

/// Identical to [`undefined_bare_variable`]: stack expression surfaces and
/// top-level fields now share one known-root contract (`ctx`/`env`/`doc` plus
/// the late-binding globals are known; only typos are flagged).
fn undefined_stack_variable<'a>(
    path: &'a str,
    defined: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<&'a str> {
    undefined_bare_variable(path, defined)
}

/// Validates that the lifecycle-stack-only `err` global is not referenced
/// in events that never carry an error.
///
/// Per the spec's `err` static-scan rule:
/// - `initialize`, `start`, `success`, and `loop` never carry an error, so
///   any reference to the bare `err` global (or `err.*` member access) in
///   their stack surfaces is faulty logic.
/// - `blocked` and `failure` always carry an error; `finalize` optionally
///   carries one. References in those events are allowed.
/// - The `doc.err` escape hatch is exempt everywhere — it reaches a literal
///   frontmatter property named `err`, not the lifecycle global.
///
/// Two kinds of surface are scanned in events that cannot carry an error:
///
/// - **Communication/action strings** (top-level `say`/`message`/`stderr`/…
///   fields and single-parameter action message bodies) are literal text whose
///   only path to the `err` global is a `{{ … }}` interpolation span. Each span
///   is parsed and rejected when it references bare `err`.
/// - **Expression surfaces** (`when:` clauses, multi-argument expression-verb
///   args, control-action operands) evaluate the whole expression, so a bare
///   `err` reference anywhere in the tree is rejected.
///
/// `timing`/`current` are allowed everywhere; `doc.err` remains the escape hatch
/// (it reaches a literal frontmatter `err` property, not the lifecycle global).
/// The first violation aborts with [`CompositionError::LifecycleErrNotAvailable`].
pub fn validate_no_err_in_no_error_events(
    lifecycle: &LifecycleConfig,
    source_path: &Path,
) -> Result<(), CompositionError> {
    // Top-level communication fields: `err` reaches them only through a
    // `{{ … }}` span, so scan each span rather than the whole string.
    for signal in LifecycleSignal::ALL {
        if signal.can_carry_error() {
            continue;
        }
        let Some(notification) = lifecycle.get(signal) else {
            continue;
        };
        for (field_name, value) in notification_comm_fields(notification) {
            let Some(text) = value else { continue };
            if literal_spans_reference_err(text) {
                return Err(CompositionError::LifecycleErrNotAvailable {
                    source_path: source_path.to_path_buf(),
                    property: format!("{}.{}", signal.property_name(), field_name),
                    event: signal.property_name().to_string(),
                });
            }
        }
    }

    // Stack surfaces: an expression surface is rejected for a bare `err`
    // anywhere in its tree; a string literal (a single-parameter message body)
    // is rejected for a bare `err` inside any of its `{{ … }}` spans.
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if surface.signal.can_carry_error() {
            continue;
        }
        if surface_references_err(surface.expr) {
            return Err(CompositionError::LifecycleErrNotAvailable {
                source_path: source_path.to_path_buf(),
                property: surface.property,
                event: surface.signal.property_name().to_string(),
            });
        }
    }
    Ok(())
}

/// Whether an expression surface references the lifecycle `err` global, either
/// as a bare expression reference or inside a `{{ … }}` span of a string literal
/// embedded in the tree (a single-parameter action message body).
fn surface_references_err(expr: &Expr) -> bool {
    if references_bare_err(expr) {
        return true;
    }
    let mut found = false;
    visit_string_literals(expr, &mut |literal| {
        if !found && literal_spans_reference_err(literal) {
            found = true;
        }
    });
    found
}

/// Whether any `{{ … }}` span inside a literal communication/action string
/// references the bare lifecycle `err` global.
fn literal_spans_reference_err(literal: &str) -> bool {
    ExpressionFinder::find_all_plain(literal).iter().any(|span| {
        parse(&span.expression)
            .map(|expr| references_bare_err(&expr))
            .unwrap_or(false)
    })
}

/// Returns `true` when the expression tree references the lifecycle `err`
/// global as a bare name or member-access base.
///
/// `doc.err` (and any `doc.*` path) is exempt: the `doc` namespace reaches
/// literal frontmatter, so `doc.err` is a property lookup, not a lifecycle
/// global reference.
fn references_bare_err(expr: &Expr) -> bool {
    match expr {
        Expr::Variable(path) => {
            let root = path.split('.').next().unwrap_or(path);
            root == "err"
        }
        Expr::MemberAccess { base, .. } => {
            // `doc.anything` (including `doc.err`) is not a bare err
            // reference — it reaches the frontmatter.
            if let Expr::Variable(base_path) = base.as_ref() {
                let root = base_path.split('.').next().unwrap_or(base_path);
                if root == "doc" {
                    return false;
                }
            }
            references_bare_err(base)
        }
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            references_bare_err(inner)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            references_bare_err(left) || references_bare_err(right)
        }
        Expr::Index { base, index } => references_bare_err(base) || references_bare_err(index),
        Expr::FunctionCall { args, .. } => args.iter().any(references_bare_err),
        Expr::Fallback { primary, fallback } => {
            references_bare_err(primary) || references_bare_err(fallback)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            references_bare_err(condition)
                || references_bare_err(then_branch)
                || references_bare_err(else_branch)
        }
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => false,
    }
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
/// non-zero exit. Each entry's property path names the source location
/// (e.g. `start.stack[1].action.command`).
pub fn collect_lifecycle_shell_commands(
    lifecycle: &LifecycleConfig,
) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    for surface in iter_stack_expression_surfaces(lifecycle) {
        if let Some(literal) = expr_as_string_literal(surface.expr) {
            if surface.property.ends_with(".command") || surface.property.ends_with(".on_error") {
                commands.push((literal, surface.property));
            }
        }
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

/// Normalizes empty or whitespace-only strings to `None`.
use super::*;
use super::actions::ProxyWithValue;
