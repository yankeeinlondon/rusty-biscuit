//! The record of a provider selection: each source the selector consulted,
//! in precedence order, and what it found there.
//!
//! The selectors (`sequence/providers.rs::select_target`,
//! `composition/target.rs::decide_target`) push a link in the branch that
//! consults a source, so the record is the decision as it was made, never a
//! second description of it. A dry run renders it under the provider
//! finding; a real run ignores it.

/// What one consulted source held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkState {
    /// A command-line source the caller did not give.
    NotGiven,
    /// A document or task source that does not write the key.
    NotSet,
    /// The source wrote this value, and it decided the provider.
    Chose(String),
    /// The source wrote this value, which the task's turn evaluates.
    AtLaunch(String),
    /// The source wrote this value, which selects no provider.
    Unusable { value: String, why: String },
}

/// One consulted source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceLink {
    /// The source as the report names it (`--provider flag`, `plan.md agent`).
    pub(crate) source: String,
    pub(crate) state: LinkState,
    /// A pointer the report appends to the state, such as where a value the
    /// selector does not read was written.
    pub(crate) note: Option<String>,
}

/// The consulted sources, in the order the selector read them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SelectionTrace {
    links: Vec<TraceLink>,
}

impl SelectionTrace {
    pub(crate) fn push(&mut self, source: impl Into<String>, state: LinkState) {
        self.links.push(TraceLink { source: source.into(), state, note: None });
    }

    /// Append `note` to the state of the link read from `source`, if any.
    pub(crate) fn annotate(&mut self, source: &str, note: impl Into<String>) {
        if let Some(link) = self.links.iter_mut().find(|link| link.source == source) {
            link.note = Some(note.into());
        }
    }

    pub(crate) fn links(&self) -> &[TraceLink] {
        &self.links
    }

    /// The chain as numbered lines with aligned states. Values pass through
    /// the secret masker, since a hint may carry any text.
    pub(crate) fn lines(&self) -> Vec<String> {
        let width = self.links.iter().map(|link| link.source.chars().count()).max().unwrap_or(0) + 2;
        self.links
            .iter()
            .enumerate()
            .map(|(index, link)| {
                let state = match &link.state {
                    LinkState::NotGiven => "not given".to_string(),
                    LinkState::NotSet => "not set".to_string(),
                    LinkState::Chose(value) => format!("`{}` (chosen)", masked(value)),
                    LinkState::AtLaunch(value) => format!("`{}` (decided when the task runs)", masked(value)),
                    LinkState::Unusable { value, why } => format!("`{}`: {why}", masked(value)),
                };
                match &link.note {
                    Some(note) => format!("{}. {:<width$}{state} ({note})", index + 1, link.source),
                    None => format!("{}. {:<width$}{state}", index + 1, link.source),
                }
            })
            .collect()
    }
}

fn masked(value: &str) -> String {
    claudine::secrets::mask_secrets(value).into_owned()
}

/// A hint value as written: a string bare, anything else as JSON.
pub(crate) fn hint_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The `agent` parsed `hints` hold, written back as a hint value.
pub(crate) fn hints_agent_value(hints: &claudine::composition::EffectiveSelectionHints) -> serde_json::Value {
    use claudine::composition::AgentHint;
    use serde_json::Value;
    let slug = |provider: &claudine::provider::Provider| Value::String(provider.as_slug().to_string());
    match &hints.agent {
        Some(AgentHint::Single(provider)) => slug(provider),
        Some(AgentHint::List(providers)) => Value::Array(
            providers.iter().map(slug).chain(hints.agent_invalid.iter().cloned().map(Value::String)).collect(),
        ),
        None if hints.agent_was_list => Value::Array(hints.agent_invalid.iter().cloned().map(Value::String).collect()),
        None => hints.agent_invalid.first().cloned().map_or(Value::Null, Value::String),
    }
}

/// Why a classified hint selected no provider without asking; `None` for a
/// state that selects one.
pub(crate) fn unusable_reason(state: &claudine::composition::AgentResolutionState) -> Option<String> {
    use claudine::composition::AgentResolutionState as State;
    Some(match state {
        State::Selected { .. } | State::ListOneInstalled { .. } => return None,
        State::NoAgent => "names no provider".to_string(),
        State::SingleInvalid { .. } => "not a known provider".to_string(),
        State::SingleNotInstalled { provider } => {
            format!("not installed: `{}` is not on PATH", claudine::provider::provider_info(*provider).binary)
        }
        State::ListMultipleInstalled { .. } => "more than one listed provider is installed, so a run asks".to_string(),
        State::ZeroInstalledList { .. } => "no listed provider is installed".to_string(),
    })
}

/// The link for a hint the selector classified: chosen, or why not.
pub(crate) fn classified(value: &serde_json::Value, state: &claudine::composition::AgentResolutionState) -> LinkState {
    match unusable_reason(state) {
        None => LinkState::Chose(hint_text(value)),
        Some(why) => LinkState::Unusable { value: hint_text(value), why },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_number_and_align_each_link_and_mask_secret_shaped_values() {
        let mut trace = SelectionTrace::default();
        trace.push("--provider flag", LinkState::NotGiven);
        trace.push("plan.md agent", LinkState::Unusable { value: "clade".into(), why: "not a known provider".into() });
        trace.push("x agent", LinkState::Chose("sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789".into()));
        let lines = trace.lines();
        assert_eq!(lines[0], "1. --provider flag  not given");
        assert_eq!(lines[1], "2. plan.md agent    `clade`: not a known provider");
        assert!(lines[2].starts_with("3. x agent          `"), "{}", lines[2]);
        assert!(!lines[2].contains("abcdefghijklmnop"), "a secret-shaped value is masked: {}", lines[2]);
    }
}
