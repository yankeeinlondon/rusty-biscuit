const REMOVED_VALIDATION_KEYS: &[(&str, &str)] = &[
    (
        "pre_checks",
        "use the `initialize` or `start` lifecycle stack instead",
    ),
    (
        "post_checks",
        "use the `success` or `finalize` lifecycle stack instead",
    ),
    (
        "handle",
        "use a lifecycle `shell` action or other lifecycle action instead",
    ),
    (
        "deviate",
        "use a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) instead",
    ),
];

/// Prefix used for subject-specific handler keys that are also removed.
const HANDLE_PREFIX: &str = "handle_";

/// Scan frontmatter top-level keys for removed validation/handler DSL keys.
///
/// Returns the lexicographically first removed key found together with its
/// replacement guidance. This is called from composition preparation before
/// lifecycle event blocks are parsed so the diagnostic names the removed DSL
/// key rather than falling through to generic unknown-field handling.
pub fn scan_removed_validation_keys(
    frontmatter: &serde_json::Value,
) -> Option<(String, &'static str)> {
    let obj = frontmatter.as_object()?;
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    for key in keys {
        if let Some((_, replacement)) = REMOVED_VALIDATION_KEYS.iter().find(|(k, _)| *k == key) {
            return Some((key.to_string(), *replacement));
        }
        if let Some(suffix) = key.strip_prefix(HANDLE_PREFIX) {
            if !suffix.is_empty() {
                return Some((
                    key.to_string(),
                    "use the `blocked` or `failure` lifecycle recovery actions instead",
                ));
            }
        }
    }
    None
}

/// Parses lifecycle configuration from composition frontmatter.
///
/// Walks every event block (`initialize`, `start`, `success`, `blocked`,
/// `failure`, `finalize`) and extracts the top-level communication
/// properties into [`LifecycleConfig`]. Lifecycle concerns authored inside
/// the `loop:` block (alongside the iteration controls parsed by
/// [`super::looping::resolve_loop_config`]) are extracted into
/// [`LifecycleConfig::loop_concerns`].
///
/// For each event the raw `stack:` is parsed into typed
/// [`LifecycleStackItem`] values stored on [`LifecycleConfig::stacks`].
/// Stack parsing enforces the spec's cardinality rule (at most one
/// lifecycle control action per item; it must be last) and the per-event
/// "Where valid" matrix for control actions.
///
/// Validates mutual exclusivity of `say` and `say_first` and validates sound
/// effect names against the embedded `playa` catalog.
///
/// ## Returns
///
/// Returns `Ok(LifecycleConfig)` on success, or a `CompositionError` if
/// validation fails.
///
/// ## Errors
///
/// - [`CompositionError::LifecycleSayConflict`]: Both `say` and `say_first`
///   are present in the same event.
/// - [`CompositionError::LifecycleUnknownEffect`]: An unknown sound effect
///   name is referenced.
/// - [`CompositionError::LifecycleInvalid`]: A property failed to
///   deserialize (typically an unknown field).
/// - [`CompositionError::LifecycleStackInvalidShape`]: A stack item is
///   malformed (not an object, missing `action`, unknown key).
/// - [`CompositionError::LifecycleWhenExpressionInvalid`]: A stack item's
///   `when:` clause is not a valid condition expression.
/// - [`CompositionError::LifecycleActionInvalidShortForm`] /
///   [`CompositionError::LifecycleActionInvalidLongForm`]: An action could
///   not be parsed.
/// - [`CompositionError::LifecycleActionPlacement`]: A control action
///   appears in an event where the spec's "Where valid" matrix forbids it.
/// - [`CompositionError::LifecycleMultipleLifecycleActions`] /
///   [`CompositionError::LifecycleActionOrder`]: Cardinality violation.
///
/// ## Examples
///
/// ```
/// # use serde_json::json;
/// # use claudine::composition::parse_lifecycle_config;
/// let frontmatter = json!({
///     "title": "My Composition",
///     "start": {
///         "message": "Starting..."
///     }
/// });
/// let config = parse_lifecycle_config(&frontmatter, std::path::Path::new("test.md")).unwrap();
/// assert!(config.start.is_some());
/// ```
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleConfig, CompositionError> {
    // Non-object frontmatter returns default
    let Some(fm_obj) = frontmatter.as_object() else {
        return Ok(LifecycleConfig::default());
    };

    let mut config = LifecycleConfig::default();

    // Top-level event blocks. Loop concerns are handled separately below
    // because they share the `loop:` block with iteration controls.
    for signal in [
        LifecycleSignal::Initialize,
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
        LifecycleSignal::Finalize,
    ] {
        let property_name = signal.property_name();
        let Some(value) = fm_obj.get(property_name) else {
            continue;
        };

        // Skip null values
        if value.is_null() {
            continue;
        }

        let (notification, stack) =
            parse_event_block(signal, value, source_file, property_name)?;

        *event_notification_field_mut(signal, &mut config) = Some(notification);
        *event_stack_field_mut(signal, &mut config) = stack.filter(|s| !s.is_empty());
    }

    // Loop concerns: extract lifecycle concern keys from inside `loop:`.
    if let Some(loop_value) = fm_obj.get("loop")
        && !loop_value.is_null()
    {
        let property_name = LifecycleSignal::Loop.property_name();
        let Some(loop_obj) = loop_value.as_object() else {
            return Err(CompositionError::LifecycleInvalid {
                property: property_name.to_string(),
                message: format!(
                    "`loop` must be an object, got {}",
                    json_type_name(loop_value)
                ),
                source_file: source_file.to_path_buf(),
                unknown_field: None,
                expected_fields: LIFECYCLE_NOTIFICATION_FIELDS
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            });
        };

        let mut concerns_obj = serde_json::Map::new();
        for key in LIFECYCLE_CONCERN_KEYS {
            if let Some(value) = loop_obj.get(*key) {
                concerns_obj.insert((*key).to_string(), value.clone());
            }
        }
        if !concerns_obj.is_empty() {
            let concerns_value = serde_json::Value::Object(concerns_obj);
            let (notification, stack) = parse_event_block(
                LifecycleSignal::Loop,
                &concerns_value,
                source_file,
                property_name,
            )?;
            config.loop_concerns = Some(notification);
            config.stacks.loop_gate = stack.filter(|s| !s.is_empty());
        }
    }

    Ok(config)
}

/// The lifecycle-concern keys accepted inside the `loop:` block (and on
/// every top-level event block).
///
/// Kept distinct from [`LIFECYCLE_NOTIFICATION_FIELDS`] (the comm-field
/// list used by serde-driven diagnostics) because this list also includes
/// `stack`, which is a structured field rather than a string.
pub(crate) const LIFECYCLE_CONCERN_KEYS: &[&str] = &[
    "say",
    "say_first",
    "effect",
    "message",
    "stderr",
    "notify",
    "info",
    "warn",
    "success",
    "stdout",
    "stack",
];

/// Parse one event block into a (notification, stack) pair.
///
/// `signal` carries the event name needed for stack parsing (cardinality
/// and "Where valid" enforcement). `property_name` is the frontmatter key
/// used in diagnostics (it always matches `signal.property_name()` but is
/// passed in to avoid repeated lookups).
fn parse_event_block(
    signal: LifecycleSignal,
    value: &serde_json::Value,
    source_file: &Path,
    property_name: &str,
) -> Result<(LifecycleNotification, Option<Vec<LifecycleStackItem>>), CompositionError> {
    let mut notification: LifecycleNotification = serde_json::from_value(value.clone()).map_err(
        |e| {
            let (unknown_field, expected_fields) = parse_serde_unknown_field(&e);
            CompositionError::LifecycleInvalid {
                property: property_name.to_string(),
                message: e.to_string(),
                source_file: source_file.to_path_buf(),
                unknown_field,
                expected_fields,
            }
        },
    )?;

    // Normalize empty strings to None for every string field.
    normalize_empty_string(&mut notification.say);
    normalize_empty_string(&mut notification.say_first);
    normalize_empty_string(&mut notification.effect);
    normalize_empty_string(&mut notification.message);
    normalize_empty_string(&mut notification.stderr);
    normalize_empty_string(&mut notification.notify);
    normalize_empty_string(&mut notification.info);
    normalize_empty_string(&mut notification.warn);
    normalize_empty_string(&mut notification.success);
    normalize_empty_string(&mut notification.stdout);

    // Validate mutual exclusivity of say and say_first.
    if notification.say.is_some() && notification.say_first.is_some() {
        return Err(CompositionError::LifecycleSayConflict(
            property_name.to_string(),
        ));
    }

    // Validate effect name if present and free of interpolation. An
    // `effect: "{{name}}"` is deferred: its real name is only known after
    // event-time interpolation, so it is validated then (see
    // [`super::lifecycle_executor`]'s deferred effect validation), not here.
    if let Some(effect_name) = &notification.effect
        && !effect_name.contains("{{")
        && playa::SoundEffect::from_name(effect_name).is_none()
    {
        return Err(CompositionError::LifecycleUnknownEffect(
            property_name.to_string(),
            effect_name.clone(),
        ));
    }

    // Parse the raw stack into typed form, enforcing cardinality + "Where
    // valid" matrix for this event.
    let typed_stack = match notification.stack.take() {
        Some(raw_stack) if !raw_stack.is_empty() => Some(parse_lifecycle_stack(
            signal,
            &raw_stack,
            source_file,
        )?),
        _ => None,
    };

    Ok((notification, typed_stack))
}

/// Parse a task's `setup:` / `teardown:` value into a typed action stack.
///
/// A task stack is the same grammar as a lifecycle event's `stack:` — a list of
/// `{when?, action, no_error?}` items — with no surrounding event block, so this
/// is the entry point for callers that hold the bare list. `property` names the
/// authoring key in diagnostics (`setup` / `teardown`).
///
/// ## Errors
///
/// Returns the same [`CompositionError`] shape-violation family
/// [`parse_lifecycle_config`] raises for an event stack, plus
/// [`CompositionError::LifecycleStackInvalidShape`] when the value is not a
/// list.
pub fn parse_task_action_stack(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<Vec<LifecycleStackItem>, CompositionError> {
    let serde_json::Value::Array(items) = raw else {
        return Err(CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            message: format!(
                "`{property}` must be a list of action-stack items, got {}",
                json_type_name(raw)
            ),
        });
    };
    let mut parsed = Vec::with_capacity(items.len());
    for (idx, raw_item) in items.iter().enumerate() {
        let item = parse_lifecycle_stack_item(signal, raw_item, source_file)
            .map_err(|e| annotate_stack_error(e, property, idx))?;
        parsed.push(item);
    }
    Ok(parsed)
}

/// Parse a single action written in the standard positional/key-value grammar.
///
/// A task's `side_effect:` value is one action rather than a stack, so it is
/// parsed here instead of through [`parse_task_action_stack`]. `property` names
/// the authoring key in diagnostics.
///
/// ## Errors
///
/// Returns [`CompositionError::LifecycleStackInvalidShape`] when the value is
/// not an action object, and the standard unknown-verb / argument-shape
/// rejections otherwise.
pub fn parse_single_action(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<LifecycleAction, CompositionError> {
    let serde_json::Value::Object(obj) = raw else {
        return Err(CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            message: format!(
                "`{property}` must be an action object, got {}",
                json_type_name(raw)
            ),
        });
    };
    parse_stack_item_action_object(signal, obj, source_file, property)
}

/// Parse a raw stack (`Vec<Value>`) into typed form for the given event.
fn parse_lifecycle_stack(
    signal: LifecycleSignal,
    raw_stack: &[serde_json::Value],
    source_file: &Path,
) -> Result<Vec<LifecycleStackItem>, CompositionError> {
    let property_name = signal.property_name();
    let mut items = Vec::with_capacity(raw_stack.len());
    for (idx, raw_item) in raw_stack.iter().enumerate() {
        let item = parse_lifecycle_stack_item(signal, raw_item, source_file)
            .map_err(|e| annotate_stack_error(e, property_name, idx))?;
        items.push(item);
    }
    Ok(items)
}

/// Attach the stack-item index to a parse error so the diagnostic can name
/// `start.stack[2]` rather than just `start`.
fn annotate_stack_error(err: CompositionError, property: &str, idx: usize) -> CompositionError {
    let dotted = super::super::error::indexed_property(&format!("{property}.stack"), idx);
    match err {
        CompositionError::LifecycleStackInvalidShape {
            source_path,
            property: _,
            message,
        } => CompositionError::LifecycleStackInvalidShape {
            source_path,
            property: dotted,
            message,
        },
        CompositionError::LifecycleActionInvalidShortForm {
            source_path,
            property: _,
            raw,
            message,
        } => CompositionError::LifecycleActionInvalidShortForm {
            source_path,
            property: dotted,
            raw,
            message,
        },
        CompositionError::LifecycleActionInvalidLongForm {
            source_path,
            property: _,
            action,
            message,
            source,
        } => CompositionError::LifecycleActionInvalidLongForm {
            source_path,
            property: dotted,
            action,
            message,
            source,
        },
        CompositionError::LifecycleActionPlacement {
            source_path,
            property: _,
            action,
            event,
        } => CompositionError::LifecycleActionPlacement {
            source_path,
            property: dotted,
            action,
            event,
        },
        CompositionError::LifecycleMultipleLifecycleActions {
            source_path,
            property: _,
        } => CompositionError::LifecycleMultipleLifecycleActions {
            source_path,
            property: dotted,
        },
        CompositionError::LifecycleActionOrder {
            source_path,
            property: _,
        } => CompositionError::LifecycleActionOrder {
            source_path,
            property: dotted,
        },
        CompositionError::LifecycleInvalidArgs {
            source_path,
            property: _,
            action,
            message,
        } => CompositionError::LifecycleInvalidArgs {
            source_path,
            property: dotted,
            action,
            message,
        },
        other => other,
    }
}

/// Parse a single stack item.
///
/// Stack item schema (per the lifecycle spec):
///
/// ```yaml
/// - when: <optional condition>
///   action: <scalar string | array of object>
///   no_error: <optional boolean>
///   # A scalar `action` value must be a bare verb name with zero arguments
///   # (e.g. `stop`, `skip`). The universal `no_error` flag may appear as a
///   # sibling key. Key/value parameters must live inside an explicit object:
///   # `{ action: verb, ... }`. Array elements are self-contained positional
///   # single-key objects (`{verb: value}`) or key/value objects.
/// ```
fn parse_lifecycle_stack_item(
    signal: LifecycleSignal,
    raw_item: &serde_json::Value,
    source_file: &Path,
) -> Result<LifecycleStackItem, CompositionError> {
    let property_name = signal.property_name();

    let obj = raw_item.as_object().ok_or_else(|| {
        CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message: format!(
                "stack item must be an object, got {}",
                json_type_name(raw_item)
            ),
        }
    })?;

    // Parse `when:` as a Darkmatter condition expression.
    //
    // This is the intentional exception to the literal-default rule: `when`,
    // `until`, and `while` are always boolean expressions and must never be
    // routed through `action_value_to_expr`.
    let when = match obj.get("when") {
        Some(serde_json::Value::String(s)) => Some(parse_condition(s).map_err(|source| {
            // The one stack-shape rejection that holds a typed cause. It renders
            // and classifies exactly as `LifecycleStackInvalidShape` does — the
            // sibling variant adds only the recoverable `ParseError`.
            CompositionError::LifecycleWhenExpressionInvalid {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!("`when` is not a valid expression: {source}"),
                source,
            }
        })?),
        Some(other) => {
            return Err(CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!("`when` must be a string, got {}", json_type_name(other)),
            });
        }
        None => None,
    };

    let raw_action = obj.get("action").ok_or_else(|| {
        CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message: "stack item must have an `action` key".to_string(),
        }
    })?;

    // Collect the universal `no_error` flag. Sibling parameter keys are no
    // longer accepted at the stack-item level: a scalar `action: <verb>` must
    // be a bare verb with zero arguments, and key/value parameters must live
    // inside an explicit `{ action: verb, ... }` object.
    let mut stack_no_error: Option<bool> = None;
    let mut sibling_keys: Vec<&str> = Vec::new();
    for (key, value) in obj {
        if matches!(key.as_str(), "when" | "action") {
            continue;
        }
        if key == "no_error" {
            stack_no_error = Some(match value {
                serde_json::Value::Bool(b) => *b,
                other => {
                    return Err(CompositionError::LifecycleStackInvalidShape {
                        source_path: source_file.to_path_buf(),
                        property: property_name.to_string(),
                        message: format!(
                            "`no_error` must be a boolean, got {}",
                            json_type_name(other)
                        ),
                    });
                }
            });
            continue;
        }
        sibling_keys.push(key.as_str());
    }
    if !sibling_keys.is_empty() {
        sibling_keys.sort_unstable();
        let message = if let serde_json::Value::String(verb) = raw_action {
            format!(
                "scalar `action` value `{verb}` cannot take sibling parameter(s) ({}); \
                 use the explicit key/value form `{{{{ action: {verb}, ... }}}}``",
                sibling_keys.join(", ")
            )
        } else {
            format!(
                "stack item with an {} `action` cannot carry sibling parameter(s) ({}); \
                 move them into each array element",
                json_type_name(raw_action),
                sibling_keys.join(", ")
            )
        };
        return Err(CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message,
        });
    }

    let actions = match raw_action {
        // Scalar string action: only bare verb-name zero-arg form is accepted.
        // Any string containing `(` is the removed short-form grammar.
        serde_json::Value::String(s) => {
            let no_error = stack_no_error.unwrap_or(false);
            let action = parse_scalar_action(signal, s, no_error, source_file, property_name)?;
            vec![action]
        }
        // Array of actions, each self-contained.
        serde_json::Value::Array(items) => {
            if stack_no_error.is_some() {
                return Err(CompositionError::LifecycleStackInvalidShape {
                    source_path: source_file.to_path_buf(),
                    property: property_name.to_string(),
                    message: "stack item with an array `action` cannot carry a sibling `no_error`; \
                        move `no_error` into each array element"
                        .to_string(),
                });
            }
            let mut actions = Vec::with_capacity(items.len());
            for item in items {
                let action = match item {
                    serde_json::Value::String(s) => {
                        if s.contains('(') {
                            return Err(CompositionError::LifecycleShortFormRemoved {
                                source_path: source_file.to_path_buf(),
                                property: property_name.to_string(),
                                raw: s.clone(),
                                rewrite: rewrite_to_positional(s),
                            });
                        }
                        parse_bare_verb_string(signal, s, false, source_file, property_name)?
                    }
                    serde_json::Value::Object(inner) => {
                        parse_stack_item_action_object(
                            signal,
                            inner,
                            source_file,
                            property_name,
                        )?
                    }
                    other => {
                        return Err(CompositionError::LifecycleStackInvalidShape {
                            source_path: source_file.to_path_buf(),
                            property: property_name.to_string(),
                            message: format!(
                                "array `action` element must be a string or object, got {}",
                                json_type_name(other)
                            ),
                        });
                    }
                };
                actions.push(action);
            }
            actions
        }
        // Object form: either a positional single-key object (`{success: "x"}`)
        // or a nested key/value object (`{action: {action: verb, ...}}`).
        serde_json::Value::Object(obj) => {
            vec![parse_stack_item_action_object(
                signal,
                obj,
                source_file,
                property_name,
            )?]
        }
        other => {
            return Err(CompositionError::LifecycleStackInvalidShape {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!(
                    "`action` must be a string or array, got {}",
                    json_type_name(other)
                ),
            });
        }
    };

    // Cardinality: at most one lifecycle control action, and it must be last.
    let lifecycle_indices: Vec<usize> = actions
        .iter()
        .enumerate()
        .filter_map(|(i, a)| a.is_lifecycle_control().then_some(i))
        .collect();
    if lifecycle_indices.len() > 1 {
        return Err(CompositionError::LifecycleMultipleLifecycleActions {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
        });
    }
    if let Some(idx) = lifecycle_indices.first()
        && *idx != actions.len() - 1
    {
        return Err(CompositionError::LifecycleActionOrder {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
        });
    }

    // Per-event "Where valid" matrix for lifecycle control actions. Runs
    // after cardinality so the structural violation is reported first.
    for action in &actions {
        if let LifecycleActionKind::LifecycleControl(control) = &action.kind
            && !control.is_valid_for(signal)
        {
            return Err(CompositionError::LifecycleActionPlacement {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: control.verb().to_string(),
                event: signal.property_name().to_string(),
            });
        }
    }

    Ok(LifecycleStackItem { when, actions })
}

/// Parse an action object that appears either as an array element or as the
/// single value of a stack item's `action:` key.
///
/// Disambiguation (per the positional-and-key-value spec):
/// - Object with an `action:` key → key/value long form.
/// - Single-key object whose key is a known verb → positional form.
/// - Single-key object whose key is not a known verb → unknown-verb error.
/// - Multi-key object without an `action:` key → ambiguous error.
fn parse_stack_item_action_object(
    signal: LifecycleSignal,
    obj: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    if obj.contains_key("action") {
        return parse_long_form_action_object(signal, obj, source_file, property_name);
    }

    match obj.len() {
        0 => Err(CompositionError::LifecycleStackInvalidShape {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            message: "action object cannot be empty".to_string(),
        }),
        1 => {
            let (verb, value) = obj.iter().next().expect("single-key object");
            parse_positional_action(signal, verb, value, source_file, property_name)
        }
        _ => {
            let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
            keys.sort_unstable();
            let known_verb = keys.iter().copied().find(|k| is_known_lifecycle_verb(k));
            let positional_rewrite = known_verb.map(|verb| format!("`{verb}: ...`"));
            let kv_verb = known_verb.unwrap_or_else(|| keys.first().expect("multi-key object"));
            let kv_rewrite = format!("`{{{{ action: {kv_verb}, ... }}}}``");
            let mut rewrites: Vec<String> = Vec::new();
            if let Some(rw) = positional_rewrite {
                rewrites.push(rw);
            }
            rewrites.push(kv_rewrite);
            Err(CompositionError::LifecycleStackAmbiguous {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                message: format!(
                    "multi-key action object without an `action` key is ambiguous; \
                     did you mean {}?",
                    rewrites.join(" or ")
                ),
            })
        }
    }
}

/// Parse a scalar string `action` value.
///
/// Only the bare verb-name zero-arg form is accepted. A string containing
/// `(` is the removed short-form grammar and surfaces a typed
/// [`CompositionError::LifecycleShortFormRemoved`] with a positional rewrite.
fn parse_scalar_action(
    signal: LifecycleSignal,
    raw: &str,
    no_error: bool,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            message: "action cannot be empty".to_string(),
        });
    }

    if trimmed.contains('(') {
        return Err(CompositionError::LifecycleShortFormRemoved {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            rewrite: rewrite_to_positional(raw),
        });
    }

    parse_bare_verb_string(signal, trimmed, no_error, source_file, property_name)
}

/// Parse a bare verb-name string as a zero-arg positional action.
///
/// Validates that the verb is known and that its zero-arg form is arity-legal
/// (e.g. `stop` is accepted, `proxy` is rejected as wrong-arity).
fn parse_bare_verb_string(
    signal: LifecycleSignal,
    raw: &str,
    no_error: bool,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(CompositionError::LifecycleActionInvalidShortForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            raw: raw.to_string(),
            message: "action cannot be empty".to_string(),
        });
    }

    if !is_known_lifecycle_verb(trimmed) {
        let rewrite = did_you_mean_verb(trimmed)
            .map(|suggestion| format!("; did you mean `{suggestion}`?"))
            .unwrap_or_default();
        return Err(CompositionError::LifecycleUnknownVerb {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: trimmed.to_string(),
            rewrite,
        });
    }

    let mut action =
        validate_positional_arity_and_build(signal, trimmed, Vec::new(), source_file, property_name)?;
    action.no_error = no_error;
    Ok(action)
}

/// Parse one long-form action object (used inside an `action:` array).
///
/// The object's keys are the action's parameters, including `action:` as
/// the verb discriminator and the universal `no_error:` flag.
fn parse_long_form_action_object(
    signal: LifecycleSignal,
    obj: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    property_name: &str,
) -> Result<LifecycleAction, CompositionError> {
    let verb_value = obj.get("action").ok_or_else(|| {
        CompositionError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            action: "<missing>".to_string(),
            message: "long-form action object must have an `action` key".to_string(),
            source: None,
        }
    })?;
    let verb = match verb_value {
        serde_json::Value::String(s) => s.clone(),
        other => {
            return Err(CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: "<invalid>".to_string(),
                message: format!(
                    "`action` must be a string, got {}",
                    json_type_name(other)
                ),
                source: None,
            });
        }
    };

    if !is_known_lifecycle_verb(&verb) {
        let rewrite = did_you_mean_verb(&verb)
            .map(|suggestion| format!("; did you mean `{suggestion}`?"))
            .unwrap_or_default();
        return Err(CompositionError::LifecycleUnknownVerb {
            source_path: source_file.to_path_buf(),
            property: property_name.to_string(),
            verb: verb.clone(),
            rewrite,
        });
    }

    let no_error = match obj.get("no_error") {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(other) => {
            return Err(CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb,
                message: format!(
                    "`no_error` must be a boolean, got {}",
                    json_type_name(other)
                ),
                source: None,
            });
        }
        None => false,
    };

    let mut params: Vec<(String, Expr)> = Vec::new();
    for (key, value) in obj {
        if matches!(key.as_str(), "action" | "no_error") {
            continue;
        }
        // Direct YAML object literals are not accepted as parameter values;
        // object data must be passed through a whole-value `{{ ... }}` span.
        if let serde_json::Value::Object(_) = value {
            return Err(CompositionError::LifecycleObjectDataThroughInterpolationParameter {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                verb: verb.clone(),
                param: key.clone(),
            });
        }
        let expr = action_value_to_expr(value).map_err(|source| {
            CompositionError::LifecycleActionInvalidLongForm {
                source_path: source_file.to_path_buf(),
                property: property_name.to_string(),
                action: verb.clone(),
                message: format!("`{key}` is not a valid value: {source}"),
                source: Some(source),
            }
        })?;
        params.push((key.clone(), expr));
    }

    build_action_from_params(signal, &verb, params, no_error, source_file)
}

/// Parse a positional action: `{verb: value}`.
///
/// The value is classified into arguments per the spec:
/// - scalar (string/number/bool) → 1 argument
/// - array → N arguments, each element converted independently
/// - null or empty array → 0 arguments
/// - direct object → rejected as object-data-through-interpolation
///
/// Arity is validated against the verb's signature (communication, shell,
/// control, side-effect, or expression-function) and produces a typed
/// [`CompositionError::LifecycleWrongArity`] when it does not match.
fn parse_serde_unknown_field(err: &serde_json::Error) -> (Option<String>, Vec<String>) {
    let msg = err.to_string();

    if !msg.contains("unknown field") {
        return (None, Vec::new());
    }

    // Extract unknown field name between first pair of backticks.
    let unknown_field = extract_backtick_value(&msg, 0);

    // Extract expected fields from "expected one of `A`, `B`, `C`"
    // or "expected `A`" (single field).
    let expected = if let Some(idx) = msg.find("expected") {
        collect_backtick_values(&msg[idx..])
    } else {
        LIFECYCLE_NOTIFICATION_FIELDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    };

    (unknown_field, expected)
}

/// The canonical field names for [`LifecycleNotification`].
/// The canonical communication-field names for [`LifecycleNotification`].
///
/// Excludes `stack` because `stack` is a structured (list) field rather
/// than a string. Use [`LIFECYCLE_CONCERN_KEYS`] when the full set of
/// lifecycle concern keys (including `stack`) is needed.
pub(crate) const LIFECYCLE_NOTIFICATION_FIELDS: &[&str] = &[
    "say",
    "say_first",
    "effect",
    "message",
    "stderr",
    "notify",
    "info",
    "warn",
    "success",
    "stdout",
];

/// Extract the text inside the `n`th pair of backticks in `s`.
fn extract_backtick_value(s: &str, nth: usize) -> Option<String> {
    let mut start = 0;
    for i in 0..=nth {
        let open = s[start..].find('`')?;
        let abs_open = start + open;
        let close = s[abs_open + 1..].find('`')?;
        if i == nth {
            return Some(s[abs_open + 1..abs_open + 1 + close].to_string());
        }
        start = abs_open + 1 + close + 1;
    }
    None
}

/// Collect all backtick-delimited values from `s`.
fn collect_backtick_values(s: &str) -> Vec<String> {
    let mut vals = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some(&(i, ch)) = chars.peek() {
        if ch == '`' {
            chars.next();
            let start = i + 1;
            let mut end = start;
            for (j, c) in chars.by_ref() {
                if c == '`' {
                    end = j;
                    break;
                }
            }
            if end > start {
                vals.push(s[start..end].to_string());
            }
        } else {
            chars.next();
        }
    }
    if vals.is_empty() {
        LIFECYCLE_NOTIFICATION_FIELDS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        vals
    }
}

/// Build a `TtsConfig` from global settings.
use super::*;
use super::super::json_util::json_type_name;
