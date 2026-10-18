//! Per-task provider prediction: what preflight can say about the provider
//! each `prompt:` task launches before anything launches.
//!
//! A task's `agent`/`model` come, lowest first, from its prompt document's
//! frontmatter, the task's `with`, and the caller's setters; runtime `set`
//! writes outrank all three at the task's turn. Prediction reads those layers
//! the way the launch will, but only what is knowable now: a value is
//! evaluated against early-binding state only, and one that needs a later
//! output, loop or group values, `current.*`, a runtime-written key, or a shell
//! substitution is left for the launch. Nothing here runs a lifecycle action, a
//! shell, or a provider, composes a body, or refreshes a model catalog.
//!
//! The launch-time rebuild stays authoritative: a prediction gates the run,
//! names the provider on review and preflight rows, and fails a task naming an
//! unusable provider before any task starts. See
//! `claudine/docs/topics/flow-control/sequences.md` → *Phase 1 — Static
//! preflight*.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::shell_expansion::template::{
    DeferredShellRoots, derived_deferred_keys,
};
use serde_json::{Map, Value};

use super::shape::first_target_identity_root;

/// The selection keys a prediction reads.
pub const SELECTION_KEYS: [&str; 2] = ["agent", "model"];

/// The roots no value is known for at preflight: [`super::SEQUENCE_LATE_ROOTS`]
/// plus `current`, which is read when the task runs.
const PREDICTION_LATE_ROOTS: [&str; 4] = ["seq", crate::composition::reserved::LOOP_ROOT, "group", "current"];

/// Which layer supplied a task's hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintLayer {
    /// The prompt document's own frontmatter.
    Document,
    /// The task's `with:`.
    With,
    /// A caller `key=value` / `--set` setter.
    CallerSetter,
}

/// Where a hint was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintSource {
    #[allow(missing_docs)]
    pub layer: HintLayer,
    /// The file that wrote the hint; the sequence source for a caller setter.
    pub source_path: PathBuf,
    /// The source-rooted property (`agent`, `tasks[1].with.agent`).
    pub property: String,
}

/// Why a hint waits for the launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDecision {
    /// It reads `seq`, `loop`, `group`, or `current`, or a value derived from one.
    LateValue,
    /// It reads a key a lifecycle `set` or `side_effect` may write first, or is
    /// such a key.
    RuntimeWritten,
    /// It, or a value it reads, is a `$( … )` shell substitution.
    ShellSubstitution,
}

/// What preflight knows about one selection key of one task.
#[derive(Debug, Clone, PartialEq)]
pub enum HintState {
    /// No layer sets the key.
    Absent,
    /// Known now: the value the launch reads unless a runtime write replaces it.
    Static {
        /// The evaluated value, in the shape `agent:`/`model:` accept.
        value: Value,
        #[allow(missing_docs)]
        source: HintSource,
    },
    /// Decided by the composition at the task's turn.
    DecidedAtLaunch {
        #[allow(missing_docs)]
        source: HintSource,
        #[allow(missing_docs)]
        reason: LaunchDecision,
    },
    /// An otherwise static value whose evaluation failed.
    Failed {
        #[allow(missing_docs)]
        source: HintSource,
        /// The typed evaluation failure.
        cause: crate::composition::lifecycle_context::LifecycleCause,
    },
    /// The value reads the provider identity it selects (`env.AGENT`,
    /// `env.MODEL`, `ctx.agent`, `ctx.model`).
    Circular {
        #[allow(missing_docs)]
        source: HintSource,
        /// The identity path the value reads.
        variable: String,
    },
}

/// A task's predicted `agent` and `model`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderPrediction {
    #[allow(missing_docs)]
    pub agent: HintState,
    #[allow(missing_docs)]
    pub model: HintState,
    /// Every frontmatter key the chosen `agent` and `model` values read,
    /// directly or through another key. Preflight compares it with the keys
    /// the graph's lifecycle writes once the whole graph is known.
    pub(crate) reads: BTreeSet<String>,
}

/// The three classes a prompt task's provider falls into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskProviderClass {
    /// The task's own static `agent`.
    Predicted,
    /// The task's own `agent`, decided when it launches.
    DecidedAtLaunch,
    /// No `agent` of its own: an enclosing sequence's, a review choice, or none.
    Missing,
}

impl ProviderPrediction {
    /// The task's class. A failed or circular `agent` is still its own hint,
    /// so it is [`TaskProviderClass::Predicted`]; [`Self::failure`] reports it.
    #[must_use]
    pub fn class(&self) -> TaskProviderClass {
        match &self.agent {
            HintState::Absent => TaskProviderClass::Missing,
            HintState::DecidedAtLaunch { .. } => TaskProviderClass::DecidedAtLaunch,
            HintState::Static { .. } | HintState::Failed { .. } | HintState::Circular { .. } => {
                TaskProviderClass::Predicted
            }
        }
    }

    /// The first `agent`/`model` hint that can never select a provider.
    #[must_use]
    pub fn failure(&self) -> Option<&HintState> {
        [&self.agent, &self.model]
            .into_iter()
            .find(|state| matches!(state, HintState::Failed { .. } | HintState::Circular { .. }))
    }

    /// The task's own static hints as a frontmatter mapping, which the
    /// canonical selection-hint parser reads exactly as it reads a document's.
    /// A key decided at launch or absent is omitted.
    #[must_use]
    pub fn static_hints(&self) -> Map<String, Value> {
        let mut hints = Map::new();
        for (key, state) in SELECTION_KEYS.iter().zip([&self.agent, &self.model]) {
            if let HintState::Static { value, .. } = state {
                hints.insert((*key).to_string(), value.clone());
            }
        }
        hints
    }

    /// Move a static or failed hint that reads a key the graph writes at
    /// runtime, or that is such a key, to the launch.
    pub(crate) fn defer_runtime_written(&mut self, written: &HashSet<String>) {
        let reads_written = self.reads.iter().any(|key| written.contains(key));
        for (key, state) in SELECTION_KEYS.iter().zip([&mut self.agent, &mut self.model]) {
            if !(reads_written || written.contains(*key)) {
                continue;
            }
            if let HintState::Static { source, .. } | HintState::Failed { source, .. } = state {
                *state = HintState::DecidedAtLaunch {
                    source: source.clone(),
                    reason: LaunchDecision::RuntimeWritten,
                };
            }
        }
    }
}

/// The layers one prompt task's hints are read from.
pub(crate) struct HintLayers<'a> {
    /// The prompt document's authored frontmatter.
    pub(crate) document: &'a Map<String, Value>,
    pub(crate) document_path: &'a Path,
    /// The task's `with`, as authored.
    pub(crate) with: &'a Map<String, Value>,
    /// The `with` entries preflight evaluated against early-binding state.
    pub(crate) with_evaluated: &'a Map<String, Value>,
    /// The `with` keys that read a late value.
    pub(crate) with_deferred: &'a [String],
    /// The file that authored `with`, and the task's source-rooted property.
    pub(crate) with_origin: &'a Path,
    pub(crate) task_property: &'a str,
    /// The early-binding state the `with` values read.
    pub(crate) step_frontmatter: &'a HashMap<String, Value>,
    pub(crate) caller_setters: &'a Map<String, Value>,
    /// The sequence source, which a caller setter is attributed to.
    pub(crate) sequence_path: &'a Path,
}

/// Predict one prompt task's `agent` and `model` from `layers`.
///
/// `evaluate` resolves a document value's `{{ … }}` spans against the
/// document's early-binding state (its frontmatter under the evaluated `with`
/// and the caller's setters); it is called only for a value that reads
/// nothing late, nothing circular, and no shell substitution. Its inner error
/// is the value's evaluation failure, which the prediction records; its outer
/// error is a resolution context that could not be built, which fails
/// preflight.
///
/// ## Errors
///
/// `evaluate`'s outer error, unchanged.
pub(crate) fn predict(
    layers: &HintLayers<'_>,
    evaluate: &mut Evaluator<'_>,
) -> Result<ProviderPrediction, crate::composition::CompositionError> {
    let mut reads = BTreeSet::new();
    let mut document_state: HashMap<String, Value> = layers
        .document
        .iter()
        .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
        .collect();
    document_state.extend(layers.with_evaluated.iter().map(|(key, value)| (key.clone(), value.clone())));
    document_state.extend(layers.caller_setters.iter().map(|(key, value)| (key.clone(), value.clone())));
    let agent = predict_key("agent", layers, &document_state, &mut reads, evaluate)?;
    let model = predict_key("model", layers, &document_state, &mut reads, evaluate)?;
    Ok(ProviderPrediction { agent, model, reads })
}

/// What [`predict`] evaluates a document value with.
pub(crate) type Evaluator<'a> = dyn FnMut(
        &Value,
        &HashMap<String, Value>,
    ) -> Result<
        Result<Value, crate::composition::lifecycle_context::LifecycleCause>,
        crate::composition::CompositionError,
    > + 'a;

fn predict_key(
    key: &str,
    layers: &HintLayers<'_>,
    document_state: &HashMap<String, Value>,
    reads: &mut BTreeSet<String>,
    evaluate: &mut Evaluator<'_>,
) -> Result<HintState, crate::composition::CompositionError> {
    if let Some(value) = layers.caller_setters.get(key) {
        return Ok(HintState::Static {
            value: value.clone(),
            source: HintSource {
                layer: HintLayer::CallerSetter,
                source_path: layers.sequence_path.to_path_buf(),
                property: key.to_string(),
            },
        });
    }
    if let Some(value) = layers.with.get(key) {
        let source = HintSource {
            layer: HintLayer::With,
            source_path: layers.with_origin.to_path_buf(),
            property: super::property_child(&super::property_child(layers.task_property, "with"), key),
        };
        // `with` values read the enclosing sequence's state, never the
        // document's; a synthetic key keeps the entry apart from a step key of
        // the same name.
        let entry = format!("\u{0}with.{key}");
        let mut state: HashMap<String, Value> = layers.step_frontmatter.clone();
        state.insert(entry.clone(), value.clone());
        let decided = classify_reads(&entry, &state, &HashSet::new(), reads);
        if let Some(state) = decided.into_state(source.clone()) {
            return Ok(state);
        }
        if layers.with_deferred.iter().any(|deferred| deferred == key) {
            return Ok(HintState::DecidedAtLaunch { source, reason: LaunchDecision::LateValue });
        }
        return Ok(match layers.with_evaluated.get(key) {
            Some(value) => HintState::Static { value: value.clone(), source },
            None => HintState::DecidedAtLaunch { source, reason: LaunchDecision::LateValue },
        });
    }
    let Some(value) = layers.document.get(key) else {
        return Ok(HintState::Absent);
    };
    let source = HintSource {
        layer: HintLayer::Document,
        source_path: layers.document_path.to_path_buf(),
        property: key.to_string(),
    };
    let mut raw_state: HashMap<String, Value> = layers.document.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    raw_state.extend(layers.with.iter().map(|(k, v)| (k.clone(), v.clone())));
    raw_state.extend(layers.caller_setters.iter().map(|(k, v)| (k.clone(), v.clone())));
    let late_with: HashSet<String> = layers.with_deferred.iter().cloned().collect();
    let decided = classify_reads(key, &raw_state, &late_with, reads);
    if let Some(state) = decided.into_state(source.clone()) {
        return Ok(state);
    }
    if !contains_expression(value) {
        return Ok(HintState::Static { value: value.clone(), source });
    }
    Ok(match evaluate(value, document_state)? {
        Ok(value) => HintState::Static { value, source },
        Err(cause) => HintState::Failed { source, cause },
    })
}

/// What a key's value reads that keeps it from being evaluated now.
enum ReadClass {
    Evaluable,
    Circular(String),
    Late(LaunchDecision),
}

impl ReadClass {
    fn into_state(self, source: HintSource) -> Option<HintState> {
        match self {
            Self::Evaluable => None,
            Self::Circular(variable) => Some(HintState::Circular { source, variable }),
            Self::Late(reason) => Some(HintState::DecidedAtLaunch { source, reason }),
        }
    }
}

/// Classify `key` of `state` by what it reads, directly or through other keys
/// of `state`, and add those keys to `reads`. `late` names keys already known
/// to be late.
fn classify_reads(
    key: &str,
    state: &HashMap<String, Value>,
    late: &HashSet<String>,
    reads: &mut BTreeSet<String>,
) -> ReadClass {
    // A literal reads nothing.
    if !state.get(key).and_then(value_text).is_some_and(|text| text.contains("{{") || text.contains("$(")) {
        return ReadClass::Evaluable;
    }
    let no_roots = DeferredShellRoots::new(std::iter::empty::<&str>());
    let reaches = |marked: &HashSet<String>| -> bool {
        marked.contains(key) || derived_deferred_keys(state.iter(), &no_roots, marked).contains(key)
    };
    // Each key `key` derives from, for the runtime-write check.
    for candidate in state.keys() {
        if candidate != key && reaches(&HashSet::from([candidate.clone()])) {
            reads.insert(candidate.trim_start_matches("\u{0}with.").to_string());
        }
    }
    let circular: HashSet<String> = state
        .iter()
        .filter(|(_, value)| value_text(value).is_some_and(|text| first_target_identity_root(&text).is_some()))
        .map(|(name, _)| name.clone())
        .collect();
    if reaches(&circular) {
        let variable = std::iter::once(key)
            .chain(reads.iter().map(String::as_str))
            .filter_map(|name| state.get(name).or_else(|| state.get(&format!("\u{0}with.{name}"))))
            .filter_map(value_text)
            .find_map(|text| first_target_identity_root(&text))
            .unwrap_or_else(|| "env.AGENT".to_string());
        return ReadClass::Circular(variable);
    }
    let shells: HashSet<String> = state
        .iter()
        .filter(|(_, value)| value_text(value).is_some_and(|text| text.contains("$(")))
        .map(|(name, _)| name.clone())
        .collect();
    if reaches(&shells) {
        return ReadClass::Late(LaunchDecision::ShellSubstitution);
    }
    let late_roots = DeferredShellRoots::new(PREDICTION_LATE_ROOTS);
    if derived_deferred_keys(state.iter(), &late_roots, late).contains(key) {
        return ReadClass::Late(LaunchDecision::LateValue);
    }
    ReadClass::Evaluable
}

/// Every string of `value`, joined: the text an expression scan reads.
fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
        _ => None,
    }
}

fn contains_expression(value: &Value) -> bool {
    value_text(value).is_some_and(|text| text.contains("{{"))
}

/// Every key a lifecycle `set` (or a `side_effect`'s `set`) in `value` writes,
/// at any depth, with an `fm.` prefix removed. Over-collects on purpose: a
/// mapping named `set` that is plain data only moves a hint to the launch.
pub(crate) fn collect_written_keys(value: &Value, written: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(set)) = map.get("set") {
                written.extend(set.keys().map(|key| key.strip_prefix("fm.").unwrap_or(key).to_string()));
            }
            map.values().for_each(|nested| collect_written_keys(nested, written));
        }
        Value::Array(items) => items.iter().for_each(|nested| collect_written_keys(nested, written)),
        _ => {}
    }
}

#[cfg(test)]
mod tests;
