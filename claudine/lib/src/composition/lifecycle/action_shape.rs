/// Parse a short-form action, `{verb: value}`, into its typed body.
///
/// The value is classified into arguments: a scalar is one argument, an array
/// is one argument per element, and `null` or `[]` is none. A direct object is
/// rejected (object data passes through a whole-value interpolation). Arity
/// is validated against the verb's signature. `set` is not parsed here: its
/// mapping is always mutation data.
pub(super) fn parse_short_form(
    signal: LifecycleSignal,
    verb: &str,
    value: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<LifecycleActionKind, CompositionError> {
    // `break` and `prep` take one string operand at most; the generic
    // one-argument-per-element list reading would accept `break: [reason]`.
    if matches!(verb, "break" | "prep")
        && let serde_json::Value::Array(items) = value
        && !items.is_empty()
    {
        return Err(CompositionError::LifecycleInvalidArgs {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            action: verb.to_string(),
            message: format!(
                "`{verb}` takes a single string, not a list; use `{verb}: \"…\"` or the long form"
            ),
        });
    }
    let args = classify_positional_value(verb, value, source_file, property)?;
    validate_positional_arity_and_build(signal, verb, args, source_file, property)
}

/// Classify a positional action value into zero or more [`Expr`] arguments.
fn classify_positional_value(
    verb: &str,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<Vec<Expr>, CompositionError> {
    // The message names the operand the way the expression walker does
    // (`start[0].info.text`, `start[1].ensure_file.arg[1]`).
    let convert = |index: usize, item: &serde_json::Value| {
        action_value_to_expr(item).map_err(|source| {
            let operand = match positional_operand_field(verb) {
                Some(field) => format!("{property_name}.{field}"),
                None => format!("{property_name}.arg[{index}]"),
            };
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb.to_string(),
                message: format!("`{operand}` is not a valid value: {source}"),
                source: Some(source),
            }
        })
    };
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_) => Ok(vec![convert(0, value)?]),
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| convert(index, item))
            .collect(),
        serde_json::Value::Object(_) => Err(
            CompositionError::LifecycleObjectDataThroughInterpolationPositional {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                verb: verb.to_string(),
            },
        ),
    }
}

/// The named field a short-form verb's single positional operand fills, or
/// `None` when its operands are numbered `arg[k]` (side effects and
/// expression functions).
pub(super) fn positional_operand_field(verb: &str) -> Option<&'static str> {
    match verb {
        "error" => Some("reason"),
        "proxy" => Some("target"),
        "retry" => Some("max_attempts"),
        "resume" => Some("message"),
        "defer" => Some("delay"),
        "break" => Some("reason"),
        "prep" => Some("target"),
        "shell" => Some("command"),
        _ if CommunicationChannel::from_verb(verb).is_some() => Some("text"),
        _ => None,
    }
}

/// Validate the positional argument count for `verb` and build the typed
/// action body.
pub(super) fn validate_positional_arity_and_build(
    signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleActionKind, CompositionError> {
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
                // Short form stays compact and carries no overlay; an author
                // who needs `with:` opts into long form.
                with: WithOverlay::default(),
            }
        }
        "retry" => {
            check_optional_positional_arity(verb, &args, 0, 1, source_file, property_name)?;
            A::Retry {
                max_attempts: args.into_iter().next(),
                backoff: None,
                delay: None,
                with: WithOverlay::default(),
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
        "break" => {
            check_optional_positional_arity(verb, &args, 0, 1, source_file, property_name)?;
            A::Break {
                reason: args
                    .into_iter()
                    .next()
                    .map(|reason| break_reason(reason, source_file, property_name))
                    .transpose()?,
                code: 0,
            }
        }
        "prep" => {
            check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
            A::Prep {
                target: prep_target(args.into_iter().next().expect("arity checked"), source_file, property_name)?,
                with: WithOverlay::default(),
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

    Ok(LifecycleActionKind::LifecycleControl(control))
}

/// Build a positional action for verbs that are not lifecycle controls.
fn build_non_control_positional_action(
    _signal: LifecycleSignal,
    verb: &str,
    args: Vec<Expr>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleActionKind, CompositionError> {
    // Communication channels.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
        return Ok(LifecycleActionKind::Communication(CommunicationAction {
            channel,
            text: args.into_iter().next().expect("arity checked"),
        }));
    }

    // Shell.
    if verb == "shell" {
        check_exact_positional_arity(verb, &args, 1, source_file, property_name)?;
        return Ok(LifecycleActionKind::Shell(ShellAction {
            authored_command: None,
            command: args.into_iter().next().expect("arity checked"),
            on_error: None,
            pre_resolved: false,
            template: None,
        }));
    }

    // Side-effect verbs.
    if let Some(signature) = side_effect_signature(verb) {
        check_positional_signature(verb, &args, &signature, source_file, property_name)?;
        return Ok(LifecycleActionKind::SideEffect(SideEffectAction {
            verb: verb.to_string(),
            args,
        }));
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
    Ok(LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
        function: verb.to_string(),
        args,
    }))
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
    closest_spelling(verb, all_lifecycle_verbs())
}

/// The candidate closest to the unknown `word` by case-insensitive edit
/// distance, within a third of the word's length (at least 2). The first of
/// equally close candidates wins.
pub(crate) fn closest_spelling(
    word: &str,
    candidates: impl IntoIterator<Item = &'static str>,
) -> Option<&'static str> {
    use darkmatter::catalog::levenshtein;

    let normalized = word.to_lowercase();
    let threshold = (normalized.chars().count() / 3).max(2);
    let mut best: Option<(usize, &'static str)> = None;
    for candidate in candidates {
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
/// - Mixed text (`"status {{ fm.title }}"`) stays a literal template, but every
///   span in it is parsed here, so a syntax error is an authoring error in any
///   branch rather than an event-time surprise. Spans are never evaluated: a
///   span that parses but would fail to evaluate is accepted.
/// - YAML numeric and boolean scalars become [`Expr::NumberLiteral`] and
///   [`Expr::BoolLiteral`].
/// - Direct YAML objects and arrays are rejected; object/array data must be
///   passed through a whole-value interpolation span such as `{{ payload }}`.
///
/// The `when`/`until`/`while` keys are **not** action parameters — they remain
/// boolean expressions parsed by [`parse_condition`].
pub(crate) fn action_value_to_expr(value: &serde_json::Value) -> Result<Expr, ActionExprError> {
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
                // Whole-value expansion: preserve the expression's typed value,
                // and its typed parse error when it fails.
                Ok(parse(&span.expression).map_err(|e| ActionExprError::Parse(Box::new(e)))?)
            } else {
                for span in spans {
                    parse(&span.expression).map_err(|source| ActionExprError::SpanParse {
                        expression: span.expression.trim().to_string(),
                        source: Box::new(source),
                    })?;
                }
                Ok(Expr::StringLiteral(s.clone()))
            }
        }
        serde_json::Value::Number(n) => {
            let f = n
                .as_f64()
                .ok_or_else(|| ActionExprError::Invalid(format!("unsupported number `{n}`")))?;
            Ok(Expr::NumberLiteral(f))
        }
        serde_json::Value::Bool(b) => Ok(Expr::BoolLiteral(*b)),
        serde_json::Value::Null => Err(ActionExprError::Invalid(
            "null values are not supported as action parameters; use a whole-value `{{ null }}` interpolation to pass null"
                .to_string(),
        )),
        other => Err(ActionExprError::Invalid(format!(
            "{} values are not supported as action parameters; pass object/array data through a whole-value `{{{{ ... }}}}` interpolation",
            json_type_name(other)
        ))),
    }
}

/// Build a typed action body from a long-form verb and its named parameters.
///
/// Long form exposes per-verb named parameters the short form cannot carry
/// (e.g. `shell`'s `on_error`, `retry`'s `backoff`, `set_frontmatter`'s
/// `file`/`prop`/`value`). `property` is the action's authored path.
pub(super) fn build_action_from_params(
    signal: LifecycleSignal,
    verb: &str,
    params: Vec<(String, Expr)>,
    with: Option<WithOverlay>,
    source_file: &Path,
    property: &str,
) -> Result<LifecycleActionKind, CompositionError> {
    // Params are consumed via `params_map.remove(...)` as each verb reads its
    // known keys; any leftovers are reported by `reject_extra_params`.
    let mut params_map = params.into_iter().collect::<std::collections::HashMap<_, _>>();

    // Lifecycle control actions.
    if let Some(control) =
        parse_lifecycle_control_long(verb, &mut params_map, with, signal, source_file, property)?
    {
        reject_extra_params(control.verb(), &params_map, property, source_file)?;
        return Ok(LifecycleActionKind::LifecycleControl(control));
    }

    // Communication actions: one payload parameter, `text`.
    if let Some(channel) = CommunicationChannel::from_verb(verb) {
        let text = params_map.remove("text").ok_or_else(|| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                action: verb.to_string(),
                message: format!("`{verb}` requires a `text` parameter"),
                source: None,
            }
        })?;
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleActionKind::Communication(CommunicationAction { channel, text }));
    }

    // Shell action: `command` required, `on_error` optional.
    if verb == "shell" {
        let command = params_map.remove("command").ok_or_else(|| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property.to_string(),
                action: verb.to_string(),
                message: "`shell` requires a `command` parameter".to_string(),
                source: None,
            }
        })?;
        let on_error = params_map.remove("on_error");
        reject_extra_params(verb, &params_map, property, source_file)?;
        return Ok(LifecycleActionKind::Shell(ShellAction {
            authored_command: None,
            command,
            on_error,
            pre_resolved: false,
            template: None,
        }));
    }

    // Expression-function actions with concrete named parameters. Variadic
    // functions (`and`, `or`) are positional-only and reject long form.
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
        return Ok(LifecycleActionKind::ExpressionFunction(ExpressionFunctionAction {
            function: verb.to_string(),
            args,
        }));
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
        return Ok(LifecycleActionKind::SideEffect(SideEffectAction {
            verb: verb.to_string(),
            args,
        }));
    }

    let mut named: Vec<(String, Expr)> = params_map.into_iter().collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let args = named.into_iter().map(|(_, v)| v).collect();
    Ok(LifecycleActionKind::SideEffect(SideEffectAction {
        verb: verb.to_string(),
        args,
    }))
}

/// Parse a long-form lifecycle control action by named parameters.
fn parse_lifecycle_control_long(
    verb: &str,
    params: &mut std::collections::HashMap<String, Expr>,
    with: Option<WithOverlay>,
    _signal: LifecycleSignal,
    source_file: &Path,
    property: &str,
) -> Result<Option<LifecycleControlAction>, CompositionError> {
    use LifecycleControlAction as A;
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
            // An omitted `with:` and an authored `with: {}` are the same empty
            // overlay.
            with: with.unwrap_or_default(),
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
                with: with.unwrap_or_default(),
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
                invalid_args("`defer` requires a `delay` parameter".to_string())
            })?,
            reason: params.remove("reason"),
        },
        "break" => A::Break {
            reason: params
                .remove("reason")
                .map(|reason| break_reason(reason, source_file, property))
                .transpose()?,
            code: match params.remove("code") {
                None => 0,
                Some(code) => break_code(&code).ok_or_else(|| {
                    invalid_args(format!("`break.code` must be an integer from 0 to 255, got `{code}`"))
                })?,
            },
        },
        "prep" => A::Prep {
            target: prep_target(
                params.remove("target").ok_or_else(|| {
                    invalid_args("`prep` requires a `target` parameter".to_string())
                })?,
                source_file,
                property,
            )?,
            // An omitted `with:` and an authored `with: {}` are the same empty
            // overlay, as for `proxy`.
            with: with.unwrap_or_default(),
        },
        _ => return Ok(None),
    };
    Ok(Some(control))
}

/// A `break` reason is text: a literal (`""` included) or an interpolation.
fn break_reason(reason: Expr, source_file: &Path, property: &str) -> Result<Expr, CompositionError> {
    match reason {
        Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => Err(CompositionError::LifecycleInvalidArgs {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            action: "break".to_string(),
            message: format!("`break.reason` must be a string, got `{reason}`"),
        }),
        reason => Ok(reason),
    }
}

/// The `0..=255` integer a `break.code` expression is, if it is one. The long
/// form only passes a YAML number here; a string, `null`, or a collection was
/// refused before it became an expression.
fn break_code(code: &Expr) -> Option<u8> {
    match code {
        Expr::NumberLiteral(value) if value.fract() == 0.0 && (0.0..=255.0).contains(value) => {
            Some(*value as u8)
        }
        _ => None,
    }
}

/// A `prep` target is a non-empty file reference or an interpolation.
fn prep_target(target: Expr, source_file: &Path, property: &str) -> Result<Expr, CompositionError> {
    let message = match &target {
        Expr::StringLiteral(text) if text.trim().is_empty() => {
            "`prep.target` cannot be empty; name the document, group, or sequence to run".to_string()
        }
        Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {
            format!("`prep.target` must be a file reference string, got `{target}`")
        }
        _ => return Ok(target),
    };
    Err(CompositionError::LifecycleInvalidArgs {
        source_path: source_file.to_path_buf(),
        property: property.to_string(),
        action: "prep".to_string(),
        message,
    })
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
            source: None,
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
        source: None,
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

use super::*;
use super::super::json_util::json_type_name;
