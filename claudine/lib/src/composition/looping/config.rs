//! `loop:` block parsing for every owner, and the runtime cap sources.
//!
//! A document's `loop:` is read by [`resolve_loop_config`]; a group's or a
//! sequence's by [`parse_loop_block`]. Both apply one grammar, differing only
//! where [`LoopOwner`] says so, and every error names the block's file.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, Expr, ExpressionFinder, FRONTMATTER_ROOT, evaluate, is_reserved_namespace, parse,
    static_access_reads,
};
use serde_json::Value;

use super::super::authored_order::AuthoredOrder;
use super::super::error::{CompositionError, LoopExpressionCause};
use super::super::lifecycle::actions::LifecycleNode;
use crate::invocation_context::DocumentEnvironment;
use super::super::json_util::json_type_name;
use super::super::types::{
    LoopAction, LoopCondition, LoopConfig, LoopLocation, LoopOwner, OnRateLimit,
    ResolvedCompositionSource,
};

use super::dsl::{LoopIssue, parse_actions, parse_condition_expression, parse_positive_usize};

/// Read the runtime cap override (`CLAUDINE_MAX_ITERATIONS`) from the
/// environment of the looping document at `source_path`.
///
/// ## Errors
///
/// [`CompositionError::LoopMaxIterationsEnvInvalid`] when the variable is set
/// to anything but a positive integer, so a mistyped override never falls back
/// to the authored or default cap.
pub fn resolve_max_iterations_from_env(
    environment: &DocumentEnvironment,
    source_path: &Path,
) -> Result<Option<usize>, CompositionError> {
    let Some(raw) = environment.get("CLAUDINE_MAX_ITERATIONS") else {
        return Ok(None);
    };
    let invalid = |reason: &str| CompositionError::LoopMaxIterationsEnvInvalid {
        source_path: source_path.to_path_buf(),
        value: raw.to_string(),
        reason: reason.to_string(),
    };
    match raw.trim().parse::<usize>() {
        Ok(0) => Err(invalid("the cap must be greater than zero")),
        Ok(cap) => Ok(Some(cap)),
        Err(_) => Err(invalid("expected a positive integer")),
    }
}

/// Read the rate-limit pause reset-margin override from a document's
/// environment.
///
/// Checks `CLAUDINE_PAUSE_RESET_MARGIN`, parsed with the shared timeout
/// grammar (e.g. `5s`, `2m`, `0.5s`). Invalid values are ignored so the
/// engine falls back to its built-in margin. Primarily a test knob to keep
/// pause-policy coverage fast; also lets operators tune skew tolerance.
pub fn resolve_pause_reset_margin_from_env(
    environment: &DocumentEnvironment,
) -> Option<std::time::Duration> {
    let raw = environment.get("CLAUDINE_PAUSE_RESET_MARGIN")?;
    crate::harness::parse_timeout(raw, std::path::Path::new("<CLAUDINE_PAUSE_RESET_MARGIN>")).ok()
}

/// Detect and parse a document's loop configuration from its authored
/// frontmatter.
///
/// Returns `Ok(None)` when the document has no `loop` key. The block's
/// lifecycle concerns are parsed with the document's other stacks; this reads
/// the iteration controls and checks every key.
///
/// ## Errors
///
/// The errors of [`parse_loop_block`] for a [`LoopOwner::Document`] loop
/// located at the document's root `loop`.
pub fn resolve_loop_config(
    source: &ResolvedCompositionSource,
) -> Result<Option<LoopConfig>, CompositionError> {
    let fm = source.markdown.frontmatter();
    let Some(loop_value) = fm.as_map().get("loop") else {
        return Ok(None);
    };
    let location = LoopLocation::document_root(&source.resolved_path);
    let loop_map = loop_object(loop_value, &location)?;
    parse_loop_controls(loop_map, LoopOwner::Document, &location).map(Some)
}

/// Check only the keys of a document's `loop:` block: unknown keys (with
/// their did-you-mean hints) and the removed `fail_fast`.
///
/// A launch runs this before the schema verdict, so a misspelled or removed
/// key gets the loop parser's diagnostic rather than the schema's generic
/// "additional property"; [`resolve_loop_config`] judges the values later. A
/// block that is absent or not a mapping passes here.
///
/// ## Errors
///
/// [`CompositionError::LoopInvalid`] for an unknown key and
/// [`CompositionError::RemovedGrammar`] for `fail_fast`.
pub fn check_loop_config_keys(source: &ResolvedCompositionSource) -> Result<(), CompositionError> {
    let fm = source.markdown.frontmatter();
    let Some(Value::Object(loop_map)) = fm.as_map().get("loop") else {
        return Ok(());
    };
    check_loop_keys(loop_map, LoopOwner::Document, &LoopLocation::document_root(&source.resolved_path))
}

/// A parsed `loop:` block: its iteration controls and its lifecycle gate.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopBlock {
    /// The iteration controls.
    pub config: LoopConfig,
    /// The block's lifecycle concerns (loose verb keys or `gate:`), run after
    /// each pass's `finalize`.
    pub gate: Vec<LifecycleNode>,
}

/// Parse and validate the `loop:` value `value` of an `owner` authored at
/// `location`.
///
/// Every owner shares the grammar of a document's `loop:`: `until` or
/// `while` (exactly one), `action` or `actions`, `max` (a positive integer
/// or a template resolved once by [`resolve_templated_loop_max`]), and
/// lifecycle concerns as loose verb keys or a `gate:` stack. Only a document
/// loop accepts `on_rate_limit`. Nothing is evaluated or run.
///
/// ## Errors
///
/// - [`CompositionError::LoopInvalid`] for a block that is not a mapping, an
///   unknown key (with a did-you-mean hint where one is close), a missing,
///   doubled, null, or empty condition, both action aliases, and a malformed
///   action or cap.
/// - [`CompositionError::RemovedGrammar`] for `fail_fast` in any shape.
/// - [`CompositionError::LoopRateLimitPolicyNotAccepted`] for
///   `on_rate_limit` on a container.
/// - The lifecycle authoring family for the gate.
pub fn parse_loop_block(
    value: &Value,
    owner: LoopOwner,
    location: &LoopLocation,
) -> Result<LoopBlock, CompositionError> {
    parse_loop_block_with_order(value, owner, location, &AuthoredOrder::default())
}

/// [`parse_loop_block`] with the block's authored key order, which gives a
/// gate `set:` mapping its authored order.
pub(crate) fn parse_loop_block_with_order(
    value: &Value,
    owner: LoopOwner,
    location: &LoopLocation,
    authored: &AuthoredOrder,
) -> Result<LoopBlock, CompositionError> {
    let loop_map = loop_object(value, location)?;
    let config = parse_loop_controls(loop_map, owner, location)?;
    let gate = super::super::lifecycle::parse_loop_gate(
        loop_map,
        &location.source_path,
        &location.property,
        authored,
    )?;
    Ok(LoopBlock { config, gate })
}

/// Resolve a templated `max` once, before the loop starts, and record the cap.
///
/// `lookup` reads the values the owner's frontmatter resolves with: its
/// defaults with the caller's setters applied. A configuration without a
/// template is left as it is. The template is one whole-value `{{ … }}`
/// interpolation; its result is a positive integer, or text that spells one.
///
/// ## Errors
///
/// - [`CompositionError::LoopMaxTemplateInvalid`] when the template fails to
///   parse or evaluate.
/// - [`CompositionError::LoopInvalid`] when it resolves to null, an empty
///   value, zero, a negative or fractional number, text that is not an
///   integer, or a cap too large for this platform.
pub fn resolve_templated_loop_max<L: EvaluationLookup>(
    config: &mut LoopConfig,
    lookup: &L,
    location: &LoopLocation,
) -> Result<(), CompositionError> {
    let Some(template) = config.max_template.take() else {
        return Ok(());
    };
    let property = location.key("max");
    let invalid = |message: String| CompositionError::LoopInvalid {
        source_path: location.source_path.clone(),
        property: property.clone(),
        message: format!("`{property}` template `{template}` {message}"),
    };
    let resolved = render_cap_template(&template, lookup).map_err(|source| {
        CompositionError::LoopMaxTemplateInvalid {
            source_path: location.source_path.clone(),
            property: property.clone(),
            template: template.clone(),
            source,
        }
    })?;
    let cap = match &resolved {
        Value::Null => {
            return Err(invalid(
                "resolved to null; define the value it reads or pass it as a setter".to_string(),
            ));
        }
        Value::Number(number) => number.as_u64(),
        Value::String(text) if text.trim().is_empty() => {
            return Err(invalid("resolved to an empty value".to_string()));
        }
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    };
    let Some(cap) = cap else {
        return Err(invalid(format!(
            "resolved to {}; expected a positive integer",
            compact(&resolved)
        )));
    };
    if cap == 0 {
        return Err(invalid("resolved to 0; the cap must be greater than zero".to_string()));
    }
    let cap = usize::try_from(cap)
        .map_err(|_| invalid(format!("resolved to {cap}, too large for this platform")))?;
    config.max_iterations = Some(cap);
    Ok(())
}

/// Evaluate a cap template. The parser accepts only one whole-value
/// interpolation; any other text is parsed as written and fails as a parse
/// error.
fn render_cap_template<L: EvaluationLookup>(
    template: &str,
    lookup: &L,
) -> Result<Value, LoopExpressionCause> {
    let expression = whole_template_expression(template).unwrap_or_else(|| template.to_string());
    let parsed = parse(&expression)?;
    evaluate(&parsed, lookup).map_err(|error| LoopExpressionCause::Evaluate(Box::new(error)))
}

fn compact(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| json_type_name(value).to_string())
}

/// The expression of a `max` written as one whole-value `{{ … }}`
/// interpolation, the only templated form a cap takes.
fn whole_template_expression(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    match ExpressionFinder::find_all_plain(trimmed).as_slice() {
        [location] if location.start == 0 && location.end == trimmed.len() => {
            Some(location.expression.trim().to_string())
        }
        _ => None,
    }
}

fn loop_object<'a>(
    value: &'a Value,
    location: &LoopLocation,
) -> Result<&'a serde_json::Map<String, Value>, CompositionError> {
    value.as_object().ok_or_else(|| CompositionError::LoopInvalid {
        source_path: location.source_path.clone(),
        property: location.property.clone(),
        message: format!(
            "`{}` must be an object with `until` or `while`, got {}",
            location.property,
            json_type_name(value)
        ),
    })
}

fn locate(issue: LoopIssue, location: &LoopLocation) -> CompositionError {
    CompositionError::LoopInvalid {
        source_path: location.source_path.clone(),
        property: issue.property,
        message: issue.message,
    }
}

/// What to write instead of a loop's `fail_fast`.
const FAIL_FAST_REPLACEMENT: &str = "a loop has no `fail_fast`: a failed iteration is an \
    unhandled failure unless a `failure` event handles it (the iteration's document's, or the \
    looping container's); `fail_fast` is a group key";

fn parse_loop_controls(
    loop_map: &serde_json::Map<String, Value>,
    owner: LoopOwner,
    location: &LoopLocation,
) -> Result<LoopConfig, CompositionError> {
    check_loop_keys(loop_map, owner, location)?;

    let condition = parse_condition(loop_map, location)?;
    let actions = match (loop_map.get("action"), loop_map.get("actions")) {
        (Some(_), Some(_)) => {
            return Err(CompositionError::LoopInvalid {
                source_path: location.source_path.clone(),
                property: location.key("actions"),
                message: format!(
                    "`{}` and `{}` are aliases; specify only one",
                    location.key("action"),
                    location.key("actions")
                ),
            });
        }
        (Some(value), None) => parse_actions(&location.key("action"), value),
        (None, Some(value)) => parse_actions(&location.key("actions"), value),
        (None, None) => Ok(Vec::new()),
    }
    .map_err(|issue| locate(issue, location))?;
    let (max_iterations, max_template) = match loop_map.get("max") {
        Some(Value::String(raw)) if whole_template_expression(raw).is_some() => {
            (None, Some(raw.clone()))
        }
        Some(value) => (
            Some(parse_positive_usize(&location.key("max"), value).map_err(|issue| locate(issue, location))?),
            None,
        ),
        None => (None, None),
    };
    let on_rate_limit = match loop_map.get("on_rate_limit") {
        None => None,
        Some(_) if owner != LoopOwner::Document => {
            return Err(CompositionError::LoopRateLimitPolicyNotAccepted {
                source_path: location.source_path.clone(),
                property: location.key("on_rate_limit"),
                owner,
            });
        }
        Some(value) => Some(parse_on_rate_limit(value, location)?),
    };

    Ok(LoopConfig {
        condition,
        actions,
        max_iterations,
        max_template,
        on_rate_limit,
    })
}

fn check_loop_keys(
    loop_map: &serde_json::Map<String, Value>,
    owner: LoopOwner,
    location: &LoopLocation,
) -> Result<(), CompositionError> {
    reject_unknown_loop_keys(loop_map, owner, location)?;
    if loop_map.contains_key("fail_fast") {
        return Err(CompositionError::RemovedGrammar {
            source_path: location.source_path.clone(),
            path: location.key("fail_fast"),
            removed: "fail_fast".to_string(),
            replacement: FAIL_FAST_REPLACEMENT.to_string(),
        });
    }
    Ok(())
}

fn parse_on_rate_limit(value: &Value, location: &LoopLocation) -> Result<OnRateLimit, CompositionError> {
    let property = location.key("on_rate_limit");
    let message = match value {
        Value::String(raw) => match OnRateLimit::parse(raw) {
            Ok(policy) => return Ok(policy),
            Err(why) => format!("`{property}` {why}"),
        },
        other => format!("`{property}` must be a string, got {}", json_type_name(other)),
    };
    Err(CompositionError::LoopInvalid {
        source_path: location.source_path.clone(),
        property,
        message,
    })
}

/// Extract the set of frontmatter keys the loop reads or writes.
///
/// Control variables are the only keys resolved once at seed time and
/// carried as typed state across iterations. Derived/presentation keys are
/// intentionally excluded so they re-resolve each iteration.
///
/// Sources:
/// - every action target (`increment`/`decrement`/`set`/`append`/
///   `prepend`/`merge` `prop`);
/// - the root of every variable the `while`/`until` condition reads;
/// - the root of every variable read inside action-value templates.
///
/// Reads come from Darkmatter's [`static_access_reads`], so every branch,
/// operand, and index expression counts. Roots supplied by the runtime rather
/// than by frontmatter are excluded: Darkmatter's reserved namespaces
/// ([`is_reserved_namespace`]) and the host-declared `loop` global. A
/// `fm.<head>` read lifts `<head>` (the `fm` namespace traverses the loop's
/// frontmatter state); bare `fm`, including a bracket read such as
/// `fm['x']`, owns no single key.
pub fn extract_control_variables(config: &LoopConfig) -> Vec<String> {
    let mut names = BTreeSet::new();

    for action in &config.actions {
        match action {
            LoopAction::Increment(prop) | LoopAction::Decrement(prop) => {
                names.insert(prop.clone());
            }
            LoopAction::Set { prop, value }
            | LoopAction::Append { prop, value }
            | LoopAction::Prepend { prop, value }
            | LoopAction::Merge { prop, value } => {
                names.insert(prop.clone());
                if let Value::String(raw) = value {
                    for location in ExpressionFinder::find_all_plain(raw) {
                        if let Ok(expr) = parse(&location.expression) {
                            insert_control_variables(&expr, &mut names);
                        }
                    }
                }
            }
        }
    }

    let condition_source = match &config.condition {
        LoopCondition::While(source) | LoopCondition::Until(source) => source,
    };
    if let Ok(expr) = darkmatter::markdown::compose::expression::parse_condition(condition_source) {
        insert_control_variables(&expr, &mut names);
    }

    Vec::from_iter(names)
}

/// Inserts the frontmatter key each read of `expr` names, per
/// [`extract_control_variables`].
fn insert_control_variables(expr: &Expr, names: &mut BTreeSet<String>) {
    for read in static_access_reads(expr) {
        let mut segments = read.variable.split('.');
        let head = segments.next().unwrap_or(read.variable);
        if head == FRONTMATTER_ROOT {
            if let Some(segment) = segments.next() {
                names.insert(segment.to_string());
            }
        } else if !is_reserved_namespace(head) && head != super::super::reserved::LOOP_ROOT {
            names.insert(head.to_string());
        }
    }
}

/// Iteration-control keys under the `loop:` frontmatter object.
///
/// `action` is the canonical key for action mutators; `actions` is accepted
/// as an alias. `fail_fast` is listed so it reaches its removed-key error.
/// Every other key of a `loop:` block must be a lifecycle concern
/// (see [`is_lifecycle_concern_key`]); anything else is rejected at parse time
/// so a typo surfaces as a clear error rather than being ignored.
const ITERATION_CONTROL_KEYS: &[&str] = super::super::lifecycle::LOOP_ITERATION_KEYS;

/// The iteration-control keys an `owner`'s loop accepts, as an error lists
/// and suggests them.
fn accepted_control_keys(owner: LoopOwner) -> impl Iterator<Item = &'static str> {
    ITERATION_CONTROL_KEYS.iter().copied().filter(move |key| {
        *key != "fail_fast" && (owner == LoopOwner::Document || *key != "on_rate_limit")
    })
}

/// Whether `key` belongs to the lifecycle parser: a lifecycle verb, `gate:`,
/// the sibling `no_error`, or a spelling the lifecycle parser rejects with a
/// typed diagnostic (`when` at the loop root, the removed `stack` and
/// `say_first`). The two parsers read disjoint key sets of one `loop:` block,
/// and this parser leaves those keys to the lifecycle parser.
fn is_lifecycle_concern_key(key: &str) -> bool {
    matches!(key, "gate" | "no_error" | "when" | "stack" | "say_first" | "set")
        || super::super::lifecycle::actions::is_known_lifecycle_verb(key)
}

fn reject_unknown_loop_keys(
    loop_map: &serde_json::Map<String, Value>,
    owner: LoopOwner,
    location: &LoopLocation,
) -> Result<(), CompositionError> {
    for key in loop_map.keys() {
        if !ITERATION_CONTROL_KEYS.contains(&key.as_str()) && !is_lifecycle_concern_key(key) {
            let suggestion_hint = suggest_loop_key(key, owner)
                .map(|suggestion| format!(" (did you mean `{suggestion}`?)"))
                .unwrap_or_default();
            let valid: Vec<&str> = accepted_control_keys(owner).collect();
            let property = location.key(key);
            return Err(CompositionError::LoopInvalid {
                source_path: location.source_path.clone(),
                message: format!(
                    "unknown `{property}` key{suggestion_hint}; valid keys are: {}, `gate`, \
                     or lifecycle verbs",
                    valid.join(", ")
                ),
                property,
            });
        }
    }
    Ok(())
}

/// A known alias, else the closest accepted key, `gate`, or lifecycle verb.
fn suggest_loop_key(unknown: &str, owner: LoopOwner) -> Option<&'static str> {
    let lower = unknown.to_ascii_lowercase();
    let alias = match lower.as_str() {
        "loops" | "iterations" | "max_iterations" | "max-iterations" => Some("max"),
        "whilst" => Some("while"),
        "onratelimit" | "on-rate-limit" | "rate_limit" | "ratelimit"
            if owner == LoopOwner::Document =>
        {
            Some("on_rate_limit")
        }
        _ => None,
    };
    alias.or_else(|| {
        let candidates = accepted_control_keys(owner)
            .chain(std::iter::once("gate"))
            .chain(super::super::lifecycle::actions::all_lifecycle_verbs());
        super::super::lifecycle::closest_spelling(unknown, candidates)
    })
}

fn parse_condition(
    loop_map: &serde_json::Map<String, Value>,
    location: &LoopLocation,
) -> Result<LoopCondition, CompositionError> {
    let while_key = location.key("while");
    let until_key = location.key("until");
    match (loop_map.get("while"), loop_map.get("until")) {
        (Some(_), Some(_)) => Err(CompositionError::LoopInvalid {
            source_path: location.source_path.clone(),
            message: format!("`{while_key}` and `{until_key}` are mutually exclusive; keep one"),
            property: location.property.clone(),
        }),
        (Some(value), None) => parse_condition_expression(&while_key, value)
            .map(LoopCondition::While)
            .map_err(|issue| locate(issue, location)),
        (None, Some(value)) => parse_condition_expression(&until_key, value)
            .map(LoopCondition::Until)
            .map_err(|issue| locate(issue, location)),
        (None, None) => Err(CompositionError::LoopInvalid {
            source_path: location.source_path.clone(),
            message: format!(
                "`{}` must define either `while` or `until`",
                location.property
            ),
            property: location.property.clone(),
        }),
    }
}

#[cfg(test)]
mod tests;
