pub(super) fn parse_positional_action(
    signal: LifecycleSignal,
    verb: &str,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    if !is_known_lifecycle_verb(verb) {
        let rewrite = did_you_mean_verb(verb)
            .map(|suggestion| format!("; did you mean `{suggestion}`?"))
            .unwrap_or_default();
        return Err(CompositionError::LifecycleUnknownVerb {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            rewrite,
        });
    }

    let args = classify_positional_value(verb, value, source_file, property_name)?;
    validate_positional_arity_and_build(signal, verb, args, source_file, property_name)
}

/// Classify a positional action value into zero or more [`Expr`] arguments.
fn classify_positional_value(
    verb: &str,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<Vec<Expr>, CompositionError> {
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_) => Ok(vec![action_value_to_expr(value).map_err(|message| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb.to_string(),
                message,
            }
        })?]),
        serde_json::Value::Array(items) => {
            let mut args = Vec::with_capacity(items.len());
            for item in items {
                args.push(action_value_to_expr(item).map_err(|message| {
                    CompositionError::LifecycleActionInvalidLongForm {
                        source_path: source_file.to_path_buf(),
                        property: property_name.to_string(),
                        action: verb.to_string(),
                        message,
                    }
                })?);
            }
            Ok(args)
        }
        serde_json::Value::Object(_) => Err(
            CompositionError::LifecycleObjectDataThroughInterpolationPositional {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                verb: verb.to_string(),
            },
        ),
    }
}

/// Validate the positional argument count for `verb` and build the typed
/// action.
pub(super) fn validate_positional_arity_and_build(
    signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    use LifecycleControlAction as A;

    // Lifecycle control verbs.
    let control = match verb {
        "stop" => {
            check_exact_positional_arity(verb, &args, 0, source_file, property_name)?;
            A::Stop
        }
        "skip" => {
            check_exact_positional_arity(verb, &args, 0, source_file, property_name)?;
            A::Skip
        }
        "error" => {
            check_optional_positional_arity(verb, &args, 0, 1, source_file, property_name)?;
            A::Error {
                reason: args.into_iter().next(),
            }
        }
        "proxy" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Proxy {
                target: args.into_iter().next().expect("arity checked"),
            }
        }
        "retry" => {
            check_optional_positional_arity(verb, &args, 0, 1, source_file, property_name)?;
            A::Retry {
                max_attempts: args.into_iter().next(),
                backoff: None,
                delay: None,
            }
        }
        "resume" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Resume {
                message: args.into_iter().next().expect("arity checked"),
                max_attempts: None,
            }
        }
        "defer" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Defer {
                delay: args.into_iter().next().expect("arity checked"),
                reason: None,
            }
        }
        _ => {
            // Not a control verb; fall through to communication / shell /
            // side-effect / expression-function dispatch.
            return build_non_control_positional_action(
                signal, verb, args, source_file, property_name,
            );
        }
    };

    Ok(LifecycleAction {
        kind: LifecycleActionKind::LifecycleControl(control),
        no_error: false,
    })
}

/// Build a positional action for verbs that are not lifecycle controls.
fn build_non_control_positional_action(
    _signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    // Communication channels.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Communication(CommunicationAction {
                channel,
                message: args.into_iter().next().expect("arity checked"),
                route: None,
            }),
            no_error: false,
        });
    }

    // Shell.
    if verb == "shell" {
        check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Shell(ShellAction {
                command: args.into_iter().next().expect("arity checked"),
                on_error: None,
            }),
            no_error: false,
        });
    }

    // Side-effect verbs.
    if let Some(signature) = side_effect_signature(verb) {
        check_positional_signature(verb, &args, &signature, source_file, property_name)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::SideEffect(SideEffectAction {
                verb: verb.to_string(),
                args,
            }),
            no_error: false,
        });
    }

    // Expression-function verbs.
    let signature = super::actions::expression_function_signature(verb)
        .unwrap_or_else(|| super::actions::Signature {
            verb: verb.to_string(),
            params: Vec::new(),
            optional_tail: 0,
            variadic: true,
        });
    check_positional_signature(verb, &args, &signature, source_file, property_name)?;
    Ok(LifecycleAction {
        kind: LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
            function: verb.to_string(),
            args,
        }),
        no_error: false,
    })
}

/// Enforce an exact positional arity.
fn check_exact_positional_arity(
    verb: &str,
    args: &[Expr],
    expected: usize,
    source_file: &Path,
    property_name: &str,
) -> Result<(), CompositionError> {
    if args.len() != expected {
        let message = if expected == 0 {
            format!("`{verb}` takes no arguments, got {}", args.len())
        } else if expected == 1 {
            format!("`{verb}` expects exactly 1 argument, got {}", args.len())
        } else {
            format!("`{verb}` expects exactly {expected} arguments, got {}", args.len())
        };
        return Err(CompositionError::LifecycleWrongArity {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            message,
        });
    }
    Ok(())
}

/// Enforce a positional arity in the inclusive range `[min, max]`.
fn check_optional_positional_arity(
    verb: &str,
    args: &[Expr],
    min: usize,
    max: usize,
    source_file: &Path,
    property_name: &str,
) -> Result<(), CompositionError> {
    if args.len() < min || args.len() > max {
        let message = if min == max {
            format!("`{verb}` expects exactly {min} argument(s), got {}", args.len())
        } else {
            format!(
                "`{verb}` expects {min} to {max} argument(s), got {}",
                args.len()
            )
        };
        return Err(CompositionError::LifecycleWrongArity {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            message,
        });
    }
    Ok(())
}

/// Enforce positional arity against a descriptor signature.
fn check_positional_signature(
    verb: &str,
    args: &[Expr],
    signature: &super::actions::Signature,
    source_file: &Path,
    property_name: &str,
) -> Result<(), CompositionError> {
    let min = signature.required_count();
    let max = signature.max_count();

    let too_few = args.len() < min;
    let too_many = max.is_some_and(|m| args.len() > m);
    if too_few || too_many {
        let message = match max {
            None => format!(
                "`{verb}` is variadic and requires at least {min} argument(s), got {}; \
                 expected {}",
                args.len(),
                signature.params.join(", ")
            ),
            Some(max) if min == max => format!(
                "`{verb}` expects exactly {min} argument(s) ({}), got {}",
                signature.params.join(", "),
                args.len()
            ),
            Some(max) => format!(
                "`{verb}` expects {min} to {max} argument(s) ({}), got {}",
                signature.params.join(", "),
                args.len()
            ),
        };
        return Err(CompositionError::LifecycleWrongArity {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.to_string(),
            message,
        });
    }
    Ok(())
}

/// Suggest a known lifecycle verb that is close to the unknown `verb`.
pub(super) fn did_you_mean_verb(verb: &str) -> Option<&'static str> {
    use darkmatter::catalog::levenshtein;

    let normalized = verb.to_lowercase();
    let threshold = (normalized.chars().count() / 3).max(2);
    let mut best: Option<(usize, &'static str)> = None;
    for candidate in all_lifecycle_verbs() {
        let distance = levenshtein(&normalized, &candidate.to_lowercase());
        if distance <= threshold {
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, candidate));
            }
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// Convert a raw action parameter value into a Darkmatter `Expr` using the
/// single lifecycle evaluation rule.
///
/// - Strings are stored as [`Expr::StringLiteral`] unless the trimmed value is
///   exactly one `{{ … }}` interpolation span, in which case the inner
///   expression is parsed and its typed result is preserved (e.g. `{{ true }}`
///   becomes a bool, `{{ 3 }}` a number, `{{ payload }}` a variable reference
///   whose runtime value may be an object/array).
/// - YAML numeric and boolean scalars become [`Expr::NumberLiteral`] and
///   [`Expr::BoolLiteral`].
/// - Direct YAML objects and arrays are rejected; object/array data must be
///   passed through a whole-value interpolation span such as `{{ payload }}`.
///
/// The `when`/`until`/`while` keys are **not** action parameters — they remain
/// boolean expressions parsed by [`parse_condition`].
pub(super) fn action_value_to_expr(value: &serde_json::Value) -> Result<Expr, String> {
    match value {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(Expr::StringLiteral(String::new()));
            }

            let spans = ExpressionFinder::find_all_plain(trimmed);
            if let Some(span) = spans.first()
                && spans.len() == 1
                && span.start == 0
                && span.end == trimmed.len()
            {
                // Whole-value expansion: preserve the expression's typed value.
                parse(&span.expression)
                    .map_err(|e| format!("whole-value expression is not valid: {e}"))
            } else {
                // Literal text, including mixed interpolation.
                Ok(Expr::StringLiteral(s.clone()))
            }
        }
        serde_json::Value::Number(n) => {
            let f = n
                .as_f64()
                .ok_or_else(|| format!("unsupported number `{n}`"))?;
            Ok(Expr::NumberLiteral(f))
        }
        serde_json::Value::Bool(b) => Ok(Expr::BoolLiteral(*b)),
        serde_json::Value::Null => Err(
            "null values are not supported as action parameters; use a whole-value `{{ null }}` interpolation to pass null"
                .to_string(),
        ),
        other => Err(format!(
            "{} values are not supported as action parameters; pass object/array data through a whole-value `{{{{ ... }}}}` interpolation",
            json_type_name(other)
        )),
    }
}

/// Build a typed action from an explicit key/value verb + named parameters.
///
/// Key/value form allows per-verb named parameters that don't fit the
/// positional `{verb: value}` shape (e.g. `shell`'s `on_error`, `retry`'s
/// `backoff`, `set_frontmatter`'s `file`/`prop`/`value`).
pub(super) fn build_action_from_params(
    signal: LifecycleSignal,
    verb: &str,
    params: Vec<(String, Expr)>,
    no_error: bool,
    source_file: &Path,
) -> Result<LifecycleAction, CompositionError> {
    let property = signal.property_name();

    // Params are consumed via `params_map.remove(...)` as each verb reads its
    // known keys; any leftovers are reported by `reject_extra_params`.
    let mut params_map = params.into_iter().collect::<std::collections::HashMap<_, _>>();

    // Lifecycle control actions.
    if let Some(control) =
        parse_lifecycle_control_long(verb, &mut params_map, signal, source_file)?
    {
        reject_extra_params(control.verb(), &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::LifecycleControl(control),
            no_error,
        });
    }

    // Communication actions.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        let message = params_map
            .remove("message")
            .or_else(|| params_map.remove("text"))
            .or_else(|| params_map.remove("sound"))
            .ok_or_else(|| CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                action: verb.to_string(),
                message: format!("`{verb}` requires a `message` parameter"),
            })?;
        let route = params_map.remove("route");
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Communication(CommunicationAction {
                channel,
                message,
                route,
            }),
            no_error,
        });
    }

    // Shell action: `command` required, `on_error` optional.
    if verb == "shell" {
        let command = params_map.remove("command").ok_or_else(|| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                action: verb.to_string(),
                message: "`shell` requires a `command` parameter".to_string(),
            }
        })?;
        let on_error = params_map.remove("on_error");
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::Shell(ShellAction { command, on_error }),
            no_error,
        });
    }


    // Expression-function actions with concrete named parameters. Variadic
    // functions (`and`, `or`) are positional-only and reject key/value form.
    if let Some(signature) = expression_function_signature(verb) {
        if signature.variadic {
            return Err(CompositionError::LifecycleExpressionFunctionKeyValueUnsupported {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                verb: verb.to_string(),
            });
        }
        let args =
            collect_named_signature_args(verb, &signature, &mut params_map, property, source_file)?;
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
                function: verb.to_string(),
                args,
            }),
            no_error,
        });
    }

    // Side-effect action: any remaining verb with named params. The
    // Darkmatter catalog is the authority. For a verb with a known signature
    // we reorder the named params into positional call order (and reject any
    // param the signature does not name) so the executor can dispatch
    // positionally. For an unknown verb we keep the params in alphabetical
    // key order for deterministic storage; the executor surfaces it as an
    // unknown side effect at runtime.
    if let Some(signature) = super::actions::side_effect_signature(verb) {
        let args =
            collect_named_signature_args(verb, &signature, &mut params_map, property, source_file)?;
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleAction {
            kind: LifecycleActionKind::SideEffect(SideEffectAction {
                verb: verb.to_string(),
                args,
            }),
            no_error,
        });
    }

    let mut named: Vec<(String, Expr)> = params_map.into_iter().collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let args = named.into_iter().map(|(_, v)| v).collect();
    Ok(LifecycleAction {
        kind: LifecycleActionKind::SideEffect(SideEffectAction {
            verb: verb.to_string(),
            args,
        }),
        no_error,
    })
}

/// Parse a long-form lifecycle control action by named parameters.
fn parse_lifecycle_control_long(
    verb: &str,
    params: &mut std::collections::HashMap<String, Expr>,
    signal: LifecycleSignal,
    source_file: &Path,
) -> Result<Option<LifecycleControlAction>, CompositionError> {
    use LifecycleControlAction as A;
    let property = signal.property_name();
    let invalid_args = |message: String| CompositionError::LifecycleInvalidArgs {
        source_path: source_file.to_path_buf(),
        property: property.to_string(),
        action: verb.to_string(),
        message,
    };

    let control = match verb {
        "stop" => A::Stop,
        "skip" => A::Skip,
        "error" => A::Error {
            reason: params.remove("reason"),
        },
        "proxy" => A::Proxy {
            target: params.remove("target").ok_or_else(|| {
                invalid_args("`proxy` requires a `target` parameter".to_string())
            })?,
        },
        "retry" => {
            let max_attempts = params.remove("max_attempts");
            let backoff = match params.remove("backoff") {
                Some(expr) => {
                    let raw = expr_to_string(&expr);
                    let parsed = RetryBackoff::parse(&raw).ok_or_else(|| {
                        invalid_args(format!(
                            "`retry.backoff` must be `fixed` or `exponential`, got `{raw}`"
                        ))
                    })?;
                    Some(parsed)
                }
                None => None,
            };
            let delay = params.remove("delay");
            A::Retry {
                max_attempts,
                backoff,
                delay,
            }
        }
        "resume" => A::Resume {
            message: params.remove("message").ok_or_else(|| {
                invalid_args("`resume` requires a `message` parameter".to_string())
            })?,
            max_attempts: params.remove("max_attempts"),
        },
        "defer" => A::Defer {
            delay: params.remove("delay").ok_or_else(|| {
                invalid_args("`requeue` requires a `delay` parameter".to_string())
            })?,
            reason: params.remove("reason"),
        },
        _ => return Ok(None),
    };
    Ok(Some(control))
}

/// Collect key/value action arguments in the descriptor's positional order,
/// enforcing the required-through-max parameter band at parse time.
///
/// Walks `signature.params` and consumes each named parameter present in
/// `params_map`. A parameter in the first [`required_count`] positions that is
/// absent is a parse-time error naming the missing required parameter(s);
/// optional-tail parameters being absent is allowed.
///
/// [`required_count`]: super::actions::Signature::required_count
fn collect_named_signature_args(
    verb: &str,
    signature: &super::actions::Signature,
    params_map: &mut std::collections::HashMap<String, Expr>,
    property: &str,
    source_file: &Path,
) -> Result<Vec<Expr>, CompositionError> {
    let required = signature.required_count();
    let mut args = Vec::with_capacity(signature.params.len());
    let mut missing: Vec<&str> = Vec::new();
    for (index, name) in signature.params.iter().enumerate() {
        match params_map.remove(name) {
            Some(expr) => args.push(expr),
            None if index < required => missing.push(name),
            None => {}
        }
    }
    if !missing.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            action: verb.to_string(),
            message: format!(
                "`{verb}` is missing required parameter(s): {}",
                missing.join(", ")
            ),
        });
    }
    Ok(args)
}

/// Reject leftover parameters after a long-form action consumes its known
/// keys (so typos surface as parse-time errors).
fn reject_extra_params(
    verb: &str,
    leftovers: &std::collections::HashMap<String, Expr>,
    property: &str,
    source_file: &Path,
) -> Result<(), CompositionError> {
    if leftovers.is_empty() {
        return Ok(());
    }
    let mut keys: Vec<&str> = leftovers.keys().map(String::as_str).collect();
    keys.sort_unstable();
    Err(CompositionError::LifecycleActionInvalidLongForm {
        source_path: source_file.to_path_buf(),
        property: property.to_string(),
        action: verb.to_string(),
        message: format!("unknown parameter(s): {}", keys.join(", ")),
    })
}

/// Render an [`Expr`] back to a string for cases where a parameter is parsed
/// from an expression but needs a known domain (e.g. `backoff: "fixed"`).
fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::StringLiteral(s) => s.clone(),
        Expr::Variable(v) => v.clone(),
        other => other.to_string(),
    }
}

/// Get a mutable reference to the notification slot for a signal.
pub(super) fn event_notification_field_mut(
    signal: LifecycleSignal,
    config: &mut LifecycleConfig,
) -> &mut Option<LifecycleNotification> {
    match signal {
        LifecycleSignal::Initialize => &mut config.initialize,
        LifecycleSignal::Start => &mut config.start,
        LifecycleSignal::Success => &mut config.success,
        LifecycleSignal::Blocked => &mut config.blocked,
        LifecycleSignal::Failure => &mut config.failure,
        LifecycleSignal::Finalize => &mut config.finalize,
        LifecycleSignal::Loop => &mut config.loop_concerns,
    }
}

/// Get a mutable reference to the typed stack slot for a signal.
pub(super) fn event_stack_field_mut(
    signal: LifecycleSignal,
    config: &mut LifecycleConfig,
) -> &mut Option<Vec<LifecycleStackItem>> {
    match signal {
        LifecycleSignal::Initialize => &mut config.stacks.initialize,
        LifecycleSignal::Start => &mut config.stacks.start,
        LifecycleSignal::Success => &mut config.stacks.success,
        LifecycleSignal::Blocked => &mut config.stacks.blocked,
        LifecycleSignal::Failure => &mut config.stacks.failure,
        LifecycleSignal::Finalize => &mut config.stacks.finalize,
        LifecycleSignal::Loop => &mut config.stacks.loop_gate,
    }
}

/// Validates that no rendered lifecycle string contains a surviving
/// `{{ … }}` interpolation span.
///
/// Walks every configured event in [`LifecycleSignal::ALL`] order and every
/// communication field on the notification (`say`, `say_first`, `message`,
/// `stderr`, `notify`, `info`, `warn`). Additionally walks every reachable
/// stack expression surface (when clauses, action arguments, communication
/// message bodies, shell commands, side-effect args, control-action
/// operands) and scans string literals inside those parsed expression trees
/// for surviving `{{ … }}` spans.
///
/// The first field or expression with a non-empty span list aborts with
/// [`CompositionError::LifecycleInterpolationLeak`].
///
/// This runs **after** composition, when expressions should have resolved.
/// It intentionally does *not* run inside [`parse_lifecycle_config`], which
/// is also used for raw frontmatter inspection where unresolved templates
/// are legitimate.
///
/// ## Arguments
///
/// * `config` — parsed lifecycle configuration.
/// * `source_path` — composed prompt file, used for the diagnostic.
/// * `warnings` — compose report warnings, used best-effort to enrich the
///   leak reason.
use super::*;
use super::super::json_util::json_type_name;
