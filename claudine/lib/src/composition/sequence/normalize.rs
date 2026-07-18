//! Strict authored-state normalization for Sequence Plus.
//!
//! Turns a raw list of authored items (scalars or objects) into a
//! [`SequencePlan`] of generated [`StepState`]s: every scalar becomes
//! `{name: <string>}`, every step gains a collision-free `id`, one-based
//! `index`, `count`, `is_first`/`is_last`, and the shared `sequence_id`
//! invocation token. Executable fields are detected and validated for
//! exclusivity; authored state colliding with a reserved key is rejected.
//!
//! Foreign-data leniency (numeric/boolean coercion, generated names) is a
//! phase-4 concern; this module implements the strict path used by inline and
//! formal `kind: sequence` lists.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::json_util::json_type_name;
use super::model::{
    ExecutableField, SequencePlan, SequenceSource, SequenceStep, StepExecutable, StepState,
};
use super::reserved;

/// A process-local monotonic counter folded into every `sequence_id` payload so
/// two invocations sharing an identical path/state/timestamp still differ.
static INVOCATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// An item parsed from the authored list, before identity/position generation.
struct ParsedStep {
    name: String,
    raw_state: Value,
    extra: Map<String, Value>,
    executable: Option<StepExecutable>,
}

/// Normalize an authored list into a [`SequencePlan`].
///
/// `invocation_path` is the resolved path of the document that invoked the
/// sequence; it seeds the `sequence_id` payload.
///
/// ## Errors
///
/// Returns typed [`CompositionError`] variants for an empty list, a non-scalar
/// non-object item, an object missing a string `name`, more than one executable
/// field, a task option meaningless for the chosen executable, or an authored
/// state key colliding with the reserved catalog.
pub fn normalize_plan(
    items: &[Value],
    source: SequenceSource,
    invocation_path: &Path,
    document_fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    if items.is_empty() {
        return Err(CompositionError::SequenceEmpty);
    }

    let parsed: Vec<ParsedStep> = items
        .iter()
        .enumerate()
        .map(|(index, item)| parse_item(index, item))
        .collect::<Result<_, _>>()?;

    let ids = generate_ids(parsed.iter().map(|p| p.name.as_str()));
    let sequence_id = generate_sequence_id(invocation_path, &ids);
    let count = parsed.len();

    let steps = parsed
        .into_iter()
        .zip(ids)
        .enumerate()
        .map(|(index, (parsed, id))| {
            let state = StepState {
                name: parsed.name.clone(),
                id,
                sequence_id: sequence_id.clone(),
                is_first: index == 0,
                is_last: index + 1 == count,
                index: index + 1,
                count,
                extra: parsed.extra,
            };
            SequenceStep {
                index,
                name: parsed.name,
                raw_state: parsed.raw_state,
                state,
                executable: parsed.executable,
            }
        })
        .collect();

    Ok(SequencePlan {
        source,
        steps,
        document_fail_fast,
        sequence_id,
    })
}

/// Parse a single authored item into its name, executable, and state portion.
fn parse_item(index: usize, item: &Value) -> Result<ParsedStep, CompositionError> {
    match item {
        Value::String(s) => Ok(ParsedStep {
            name: s.clone(),
            raw_state: item.clone(),
            extra: Map::new(),
            executable: None,
        }),
        Value::Object(map) => parse_object_step(index, map, item),
        other => Err(CompositionError::SequenceInvalid(format!(
            "step at index {index} must be a string or object, got {}",
            json_type_name(other)
        ))),
    }
}

/// Parse an object step: extract `name`, the (at most one) executable, its task
/// options, and the arbitrary state keys — rejecting reserved-key collisions.
fn parse_object_step(
    index: usize,
    map: &Map<String, Value>,
    raw: &Value,
) -> Result<ParsedStep, CompositionError> {
    // Strict authored objects require a string `name`.
    let name = map
        .get("name")
        .ok_or(CompositionError::SequenceStepNameMissing { index })?;
    let name = name
        .as_str()
        .ok_or_else(|| CompositionError::SequenceStepNameWrongType {
            index,
            found: json_type_name(name).to_string(),
        })?
        .to_string();

    // Exactly-one executable field.
    let executable_keys: Vec<&str> = map
        .keys()
        .map(String::as_str)
        .filter(|k| reserved::is_executable_key(k))
        .collect();
    if executable_keys.len() > 1 {
        return Err(CompositionError::SequenceExclusiveExecutable {
            index,
            fields: executable_keys.iter().map(|s| (*s).to_string()).collect(),
        });
    }
    let executable_field = executable_keys
        .first()
        .and_then(|k| ExecutableField::from_key(k));

    // Partition the remaining keys into task options and arbitrary state.
    let mut options = Map::new();
    let mut extra = Map::new();
    for (key, value) in map {
        if key == "name" || reserved::is_executable_key(key) {
            continue;
        }
        if reserved::is_task_option_key(key) {
            options.insert(key.clone(), value.clone());
        } else if reserved::is_reserved_state_key(key) {
            return Err(CompositionError::SequenceReservedStateKey {
                index,
                key: key.clone(),
            });
        } else {
            extra.insert(key.clone(), value.clone());
        }
    }

    let executable = match executable_field {
        Some(field) => {
            let allowed = reserved::allowed_task_options(field);
            for option_key in options.keys() {
                if !allowed.contains(&option_key.as_str()) {
                    return Err(CompositionError::SequenceInvalidTaskField {
                        index,
                        field: option_key.clone(),
                        executable: field.key().to_string(),
                    });
                }
            }
            Some(StepExecutable {
                field,
                value: map.get(field.key()).cloned().unwrap_or(Value::Null),
                options,
            })
        }
        None => None,
    };

    Ok(ParsedStep {
        name,
        raw_state: raw.clone(),
        extra,
        executable,
    })
}

/// Dasherize a display name into an id token: lowercase alphanumerics, other
/// runs collapse to a single `-`, and leading/trailing dashes are trimmed. A
/// name with no alphanumerics falls back to `state` so an id is never empty.
fn dasherize(name: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.extend(ch.to_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        "state".to_string()
    } else {
        out
    }
}

/// Generate collision-free ids: the first use of a dasherized base keeps it,
/// each later collision takes the lowest free `<base>-<n>` starting at `-2`.
fn generate_ids<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut used: HashSet<String> = HashSet::new();
    let mut ids = Vec::new();
    for name in names {
        let base = dasherize(name);
        let mut candidate = base.clone();
        let mut n = 2u64;
        while used.contains(&candidate) {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        used.insert(candidate.clone());
        ids.push(candidate);
    }
    ids
}

/// Generate the lowercase xxHash invocation token from a canonical payload:
/// the resolved source path, the ordered state ids, a high-resolution UTC
/// timestamp, the process id, and the process-local monotonic counter.
///
/// The token is collision-resistant for correlation and logging; it is **not**
/// a uniqueness or security guarantee (xxHash is non-cryptographic).
fn generate_sequence_id(source_path: &Path, ids: &[String]) -> String {
    let counter = INVOCATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();

    let mut payload: Vec<u8> = Vec::new();
    payload.extend_from_slice(source_path.as_os_str().as_encoded_bytes());
    payload.push(0);
    for id in ids {
        payload.extend_from_slice(id.as_bytes());
        payload.push(0);
    }
    payload.extend_from_slice(&nanos.to_le_bytes());
    payload.extend_from_slice(&pid.to_le_bytes());
    payload.extend_from_slice(&counter.to_le_bytes());

    format!("{:016x}", biscuit_hash::xx_hash_bytes(&payload))
}
