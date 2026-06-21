//! Exit-expression + guard-settings config (Phase 3).
//!
//! This module owns the user/repo/frontmatter-facing types and the
//! three-layer resolution pipeline that turns them into the resolved set
//! the streaming path receives. It is the bridge between the
//! configuration layer ([`crate::config::ClaudineConfig`]) and the pure
//! detector ([`crate::runaway::detector`]).
//!
//! Notable design decisions, all from `claudine/features/2026-06-19-repetitive/spec.md`:
//! - **E1** — `exit_expressions` accepts either a bare array (uses the
//!   layer's default combine mode) or an object `{ mode, rules }`
//!   (explicit mode). The repo layer defaults to `override` (a repo is
//!   deterministic for every contributor); the frontmatter layer defaults
//!   to `merge` (the prompt author adds edge cases the general guards
//!   don't cover).
//! - **E2** — `scope` is `{agent}` or `{agent/model}`, split on the
//!   **first** `/` (model strings may contain `/`). Absent scope is a
//!   global wildcard. Scoping is additive: the run is checked against
//!   the union of every matching entry.
//! - **E3** — `kind: "literal"` (default) | `"regex"`; literal honors
//!   `ignore_case`; regex defers to inline flags (`(?i)`).
//! - **Cluster E1 note** — scalar guard settings (repetition + volume)
//!   do **not** carry a combine mode; they follow last-writer precedence
//!   (frontmatter > repo > user > built-in) like `timeout` /
//!   `step_timeout`. Only the list-typed `exit_expressions` carries a
//!   combine mode.
//!
//! The detector ([`crate::runaway::patterns::ExitExpressionInput`]) stays
//! decoupled from this module: [`ExitExpressionEntry::to_input`] is the
//! one-line bridge.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{ClaudineError, Result};
use crate::provider::Provider;
use crate::runaway::patterns::ExitExpressionInput;
use crate::runaway::{
    patterns::PatternKind, MAX_CYCLE_LENGTH, MAX_REPETITION_ALLOWED, VOLUME_BYTES, VOLUME_LINES,
};

// ============================================================================
// ExitExpressionEntry
// ============================================================================

/// One exit-expression entry as declared in user/repo/frontmatter config.
///
/// Accepts either `pattern: "..."` (single) or `patterns: [...]`
/// (array sharing the entry's `kind` / `scope`). All other fields are
/// optional with conservative defaults: `kind = Literal`, `ignore_case
/// = false`, `scope = None` (global wildcard). See Cluster E3.
///
/// Validation (compile-time regex check + scope parse) happens at
/// [`crate::config::ClaudineConfig::validate`], not at deserialization —
/// matching the `ProtectConfig::validate` precedent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExitExpressionEntry {
    /// One or more patterns sharing this entry's kind/scope. Built from
    /// either `pattern` (single) or `patterns` (array) at deserialize
    /// time.
    pub patterns: Vec<String>,
    /// Literal substring vs regex (E3a). Defaults to [`PatternKind::Literal`]
    /// to avoid metacharacter surprises (e.g. the regex `STOP.` matching
    /// `STOPS`).
    pub kind: PatternKind,
    /// Literal-only case folding (E3c). Regex defers to inline flags
    /// (`(?i)`); this flag is ignored when `kind == Regex`.
    pub ignore_case: bool,
    /// Optional scope string verbatim (e.g.
    /// `"opencode/kimi-for-coding/k2p7"`); `None` is the global wildcard
    /// (E2). Parsed into a [`ScopeSelector`] at validate time.
    pub scope: Option<String>,
}

impl<'de> Deserialize<'de> for ExitExpressionEntry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Intermediate shape that lets serde enforce `deny_unknown_fields`
        // while we coerce `pattern` / `patterns` into a single `patterns`
        // vec. Following the `ProtectConfig` precedent.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawExitExpressionEntry {
            #[serde(default)]
            pattern: Option<String>,
            #[serde(default)]
            patterns: Option<Vec<String>>,
            #[serde(default)]
            kind: PatternKind,
            #[serde(default)]
            ignore_case: bool,
            #[serde(default)]
            scope: Option<String>,
        }

        let raw = RawExitExpressionEntry::deserialize(deserializer)?;

        let patterns = match (raw.pattern, raw.patterns) {
            (Some(single), None) => vec![single],
            (None, Some(list)) => list,
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "exit-expression entry must declare `pattern` or `patterns`",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "exit-expression entry cannot declare both `pattern` and `patterns`",
                ));
            }
        };

        Ok(ExitExpressionEntry {
            patterns,
            kind: raw.kind,
            ignore_case: raw.ignore_case,
            scope: raw.scope,
        })
    }
}

impl ExitExpressionEntry {
    /// One-line bridge to the detector's input shape (Phase 2 → Phase 3
    /// wiring). Phase 6 calls this for every resolved entry before
    /// handing the set to [`crate::runaway::patterns::CompiledExitExpressions::compile`].
    pub fn to_input(&self) -> ExitExpressionInput {
        ExitExpressionInput {
            patterns: self.patterns.clone(),
            kind: self.kind,
            ignore_case: self.ignore_case,
            scope: self.scope.clone(),
        }
    }
}

// ============================================================================
// ExitExpressionsValue — array-or-object layer declaration (E1)
// ============================================================================

/// Per-layer combine mode for `exit_expressions` (Cluster E1).
///
/// - `Override` — the layer's rules fully replace the accumulated set.
/// - `Merge` — the layer's rules are appended to the accumulated set.
///
/// The repo layer defaults to `Override` (deterministic for every
/// contributor); the frontmatter layer defaults to `Merge` (the prompt
/// author adds edge cases). User-layer mode is irrelevant (no parent).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerMode {
    /// Append this layer's rules to the accumulated set.
    Merge,
    /// Replace the accumulated set with this layer's rules.
    #[default]
    Override,
}

/// Object form of a layer's `exit_expressions` declaration:
/// `{ mode: "merge"|"override", rules: [...] }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitExpressionsLayer {
    /// Combine mode for this layer.
    #[serde(default)]
    pub mode: LayerMode,

    /// The exit-expression entries.
    #[serde(default)]
    pub rules: Vec<ExitExpressionEntry>,
}

/// A layer's `exit_expressions` declaration in either form (Cluster E1):
/// - **Array shorthand** — `[{ entry }, ...]` — uses the layer's default
///   mode (repo = override, frontmatter = merge).
/// - **Object form** — `{ mode, rules }` — explicit mode.
///
/// Matches the `ProtectConfig` / `TtsValue` array-or-object house style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExitExpressionsValue {
    /// Bare array shorthand; the resolver applies the layer's default mode.
    Array(Vec<ExitExpressionEntry>),
    /// Explicit `{ mode, rules }` object form.
    Object(ExitExpressionsLayer),
}

impl ExitExpressionsValue {
    /// The rules declared by this layer.
    pub fn rules(&self) -> &[ExitExpressionEntry] {
        match self {
            Self::Array(rules) => rules,
            Self::Object(layer) => &layer.rules,
        }
    }

    /// Explicit mode declared by this layer, or `None` for the array
    /// shorthand (the resolver applies the layer's default mode).
    pub fn explicit_mode(&self) -> Option<LayerMode> {
        match self {
            Self::Array(_) => None,
            Self::Object(layer) => Some(layer.mode),
        }
    }
}

// ============================================================================
// ScopeSelector — E2
// ============================================================================

/// Parsed `scope` selector for an [`ExitExpressionEntry`].
///
/// A `scope` string is one of (E2-scope):
/// - **absent / empty** — global wildcard, matches any (provider, model);
/// - **`{agent}`** — matches any model under the given agent
///   (e.g. `"opencode"` → any OpenCode model);
/// - **`{agent}/{model}`** — matches exactly the given agent + model
///   (the model segment is split on the **first** `/`, so models may
///   themselves contain `/`).
///
/// Within the resolved set, scoping is **additive**: a run is checked
/// against the union of every entry whose selector matches
/// (global ∪ agent ∪ agent/model). See [`ScopeSelector::matches`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeSelector {
    /// No `scope` declared — matches every (provider, model).
    Global,
    /// Agent-scoped — matches any model under the given provider.
    Agent(Provider),
    /// Agent+model-scoped — matches exactly the given provider + model.
    AgentModel(Provider, String),
}

impl ScopeSelector {
    /// Parse a `scope` string per E2.
    ///
    /// ## Examples
    ///
    /// | Input                              | Result                                  |
    /// |------------------------------------|-----------------------------------------|
    /// | `""`                               | [`Global`](Self::Global)                |
    /// | `"opencode"`                       | [`Agent(OpenCode)`](Self::Agent)        |
    /// | `"opencode/kimi-for-coding/k2p7"`  | [`AgentModel`](Self::AgentModel)        |
    ///
    /// ## Errors
    ///
    /// Returns [`ClaudineError::ConfigValidation`] if the agent segment
    /// does not parse to a known [`Provider`] (the E2 validation rule).
    pub fn parse(scope: &str) -> Result<Self> {
        let trimmed = scope.trim();
        if trimmed.is_empty() {
            return Ok(Self::Global);
        }
        let (agent_segment, model_segment) = match trimmed.split_once('/') {
            Some((agent, model)) => (agent, Some(model)),
            None => (trimmed, None),
        };
        let provider = Provider::parse_cli_name(agent_segment).ok_or_else(|| {
            ClaudineError::ConfigValidation(format!(
                "exit-expression scope `{scope}` references unknown agent `{agent_segment}`"
            ))
        })?;
        match model_segment {
            None | Some("") => Ok(Self::Agent(provider)),
            Some(model) => Ok(Self::AgentModel(provider, model.to_string())),
        }
    }

    /// Whether this selector matches the run's `(provider, model)`.
    ///
    /// Used at resolve time to filter the resolved set to in-scope
    /// entries before compilation. Additive: the run is checked against
    /// the union of every entry whose `matches` returns `true`.
    pub fn matches(&self, provider: Provider, model: &str) -> bool {
        match self {
            Self::Global => true,
            Self::Agent(p) => *p == provider,
            Self::AgentModel(p, m) => *p == provider && m == model,
        }
    }
}

/// Split a scope string on the first `/`, returning `(agent, model)`
/// where `model` is `None` when no `/` is present. The model segment is
/// kept verbatim (models may themselves contain `/`).
///
/// Absent or whitespace-only input yields `(None, None)`.
///
/// This is the underlying parser used by [`ScopeSelector::parse`],
/// exposed for callers that want the raw segments without provider
/// validation (e.g. error messages that quote both halves).
pub fn parse_scope(scope: &str) -> (Option<&str>, Option<&str>) {
    let trimmed = scope.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    match trimmed.split_once('/') {
        Some((agent, model)) if !agent.is_empty() => (Some(agent), Some(model)),
        Some((_, model)) => (None, Some(model)),
        None => (Some(trimmed), None),
    }
}

// ============================================================================
// resolve_exit_expressions — three-layer pipeline (E1)
// ============================================================================

/// Resolve the effective exit-expression set across the three-layer
/// pipeline (Cluster E1):
///
/// ```text
/// effective = user_rules
/// repo present?        override (default) → effective = repo_rules
///                      merge              → effective = effective ∪ repo_rules
/// frontmatter present? merge (default)    → effective = effective ∪ fm_rules
///                      override           → effective = fm_rules
/// ```
///
/// Each layer is `Option<&ExitExpressionsValue>`; `None` means the layer
/// did not declare `exit_expressions` and is skipped. The user layer's
/// mode is irrelevant (it is the base) — only the repo and frontmatter
/// modes participate in the combine.
pub fn resolve_exit_expressions(
    user: Option<&ExitExpressionsValue>,
    repo: Option<&ExitExpressionsValue>,
    frontmatter: Option<&ExitExpressionsValue>,
) -> Vec<ExitExpressionEntry> {
    let mut effective: Vec<ExitExpressionEntry> =
        user.map(|layer| layer.rules().to_vec()).unwrap_or_default();

    if let Some(repo_layer) = repo {
        let mode = repo_layer.explicit_mode().unwrap_or(LayerMode::Override);
        match mode {
            LayerMode::Override => effective = repo_layer.rules().to_vec(),
            LayerMode::Merge => effective.extend(repo_layer.rules().iter().cloned()),
        }
    }

    if let Some(fm_layer) = frontmatter {
        let mode = fm_layer.explicit_mode().unwrap_or(LayerMode::Merge);
        match mode {
            LayerMode::Override => effective = fm_layer.rules().to_vec(),
            LayerMode::Merge => effective.extend(fm_layer.rules().iter().cloned()),
        }
    }

    effective
}

/// Extract the frontmatter-scoped `exit_expressions` from a composed
/// frontmatter object.
///
/// The composition layer parses the document frontmatter into a JSON
/// object ([`Map<String, Value>`]); this helper reads the
/// `exit_expressions` key and deserializes it into
/// [`ExitExpressionsValue`] using the same array-or-object
/// deserialization as the user/repo layers.
///
/// ## Errors
///
/// Returns [`ClaudineError::ConfigValidation`] if the frontmatter
/// declares `exit_expressions` but it does not deserialize cleanly.
pub fn extract_frontmatter_exit_expressions(
    frontmatter: &Map<String, Value>,
) -> Result<Option<ExitExpressionsValue>> {
    let Some(value) = frontmatter.get("exit_expressions") else {
        return Ok(None);
    };
    let parsed: ExitExpressionsValue = serde_json::from_value(value.clone()).map_err(|e| {
        ClaudineError::ConfigValidation(format!("frontmatter `exit_expressions` is invalid: {e}"))
    })?;
    Ok(Some(parsed))
}

// ============================================================================
// GuardSettings — scalar repetition + volume knobs (Cluster E1 note / F1 / F2)
// ============================================================================

/// Scalar knobs for the repetition + volume guards.
///
/// Last-writer precedence (frontmatter > repo > user > built-in), like
/// `timeout` / `step_timeout` — these do **not** carry a combine mode
/// (only the list-typed `exit_expressions` does).
///
/// Built-in defaults come from the spec's false-positive posture
/// (Cluster B3 / F2): conservative high thresholds so a wrongful kill
/// is essentially impossible for honest runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GuardSettings {
    /// Repetition (group-cycle) guard thresholds + kill-switch.
    pub repetition: RepetitionGuardSettings,
    /// Per-turn/per-run volume cap thresholds + kill-switch.
    pub volume: VolumeGuardSettings,
}

/// Repetition-guard scalar settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RepetitionGuardSettings {
    /// Whether the group-cycle repetition guard runs (F1 — default `true`).
    pub enabled: bool,
    /// Full-cycle count threshold at which the guard trips.
    /// Default [`MAX_REPETITION_ALLOWED`] (Cluster B3 — conservative).
    pub max_repeats: usize,
    /// Maximum cycle length `K` the detector attempts to recognize.
    /// Default [`MAX_CYCLE_LENGTH`] (Cluster B3 — must be ≥ 6 to catch
    /// the incident's 6-line block).
    pub max_cycle_length: usize,
}

impl Default for RepetitionGuardSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_repeats: MAX_REPETITION_ALLOWED,
            max_cycle_length: MAX_CYCLE_LENGTH,
        }
    }
}

/// Volume-cap scalar settings (F2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VolumeGuardSettings {
    /// Whether the volume cap runs (F2 — default `true`).
    pub enabled: bool,
    /// Per-turn line threshold. Default [`VOLUME_LINES`].
    pub max_lines: u64,
    /// Per-turn (streaming) / per-run (capture) byte threshold.
    /// Default [`VOLUME_BYTES`] (32 MiB).
    pub max_bytes: u64,
}

impl Default for VolumeGuardSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_lines: VOLUME_LINES,
            max_bytes: VOLUME_BYTES,
        }
    }
}

/// Resolve the effective [`GuardSettings`] via last-writer precedence.
///
/// frontmatter > repo > user > built-in. A set layer fully replaces
/// the lower-precedence layer's value (the sub-fields are *not*
/// individually merged).
pub fn resolve_guard_settings(
    user: &GuardSettings,
    repo: Option<&GuardSettings>,
    frontmatter: Option<&GuardSettings>,
) -> GuardSettings {
    frontmatter
        .cloned()
        .or_else(|| repo.cloned())
        .unwrap_or_else(|| user.clone())
}

// ============================================================================
// validate_exit_expressions — config-load validation (E3a / E2)
// ============================================================================

/// Validate a slice of resolved [`ExitExpressionEntry`] values.
///
/// Performed once at config-load (mirroring `ProtectConfig::validate`):
/// - rejects empty `patterns` (an entry with no patterns is meaningless);
/// - rejects an unknown agent in any `scope` (E2 validation rule);
/// - precompiles every regex-kind entry so a malformed pattern fails
///   here (never mid-stream — Cluster E3a).
pub fn validate_exit_expressions(entries: &[ExitExpressionEntry]) -> Result<()> {
    for entry in entries {
        if entry.patterns.is_empty() {
            return Err(ClaudineError::ConfigValidation(
                "exit-expression entry has an empty `patterns` list".to_string(),
            ));
        }
        if let Some(scope) = &entry.scope {
            // Validate the scope parses cleanly and references a known agent.
            let _ = ScopeSelector::parse(scope)?;
        }
        if entry.kind == PatternKind::Regex {
            for pattern in &entry.patterns {
                // Reuse the detector's compile path so a single code path
                // owns regex validation; surface as ConfigValidation to
                // match the spec's "errors at config-load" contract.
                let input = ExitExpressionInput {
                    patterns: vec![pattern.clone()],
                    kind: PatternKind::Regex,
                    ignore_case: entry.ignore_case,
                    scope: entry.scope.clone(),
                };
                crate::runaway::patterns::CompiledExitExpressions::compile(&[input])?;
            }
        }
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;
    use crate::runaway::patterns::PatternKind;

    // -------------------------------------------------------------------------
    // ExitExpressionEntry deserialization
    // -------------------------------------------------------------------------

    #[test]
    fn entry_accepts_single_pattern() {
        let entry: ExitExpressionEntry = serde_json::from_value(serde_json::json!({
            "pattern": "STOP."
        }))
        .unwrap();
        assert_eq!(entry.patterns, vec!["STOP.".to_string()]);
        assert_eq!(entry.kind, PatternKind::Literal);
        assert!(!entry.ignore_case);
        assert!(entry.scope.is_none());
    }

    #[test]
    fn entry_accepts_patterns_array_sharing_scope() {
        let entry: ExitExpressionEntry = serde_json::from_value(serde_json::json!({
            "patterns": ["STOP.", "Bye."],
            "kind": "literal",
            "scope": "opencode"
        }))
        .unwrap();
        assert_eq!(entry.patterns, vec!["STOP.", "Bye."]);
        assert_eq!(entry.scope.as_deref(), Some("opencode"));
    }

    #[test]
    fn entry_regex_kind_with_ignore_case_ignored() {
        let entry: ExitExpressionEntry = serde_json::from_value(serde_json::json!({
            "pattern": "(?i)stop",
            "kind": "regex",
            "ignore_case": true
        }))
        .unwrap();
        assert_eq!(entry.kind, PatternKind::Regex);
        // Stored as-is — the detector's compile path ignores
        // `ignore_case` when kind == Regex.
        assert!(entry.ignore_case);
    }

    #[test]
    fn entry_rejects_both_pattern_and_patterns() {
        let result = serde_json::from_value::<ExitExpressionEntry>(serde_json::json!({
            "pattern": "a",
            "patterns": ["b"]
        }));
        assert!(result.is_err());
    }

    #[test]
    fn entry_rejects_neither_pattern_nor_patterns() {
        let result = serde_json::from_value::<ExitExpressionEntry>(serde_json::json!({
            "kind": "literal"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn entry_rejects_unknown_field() {
        let result = serde_json::from_value::<ExitExpressionEntry>(serde_json::json!({
            "pattern": "STOP.",
            "posture": "strict"
        }));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("posture"), "error: {msg}");
    }

    #[test]
    fn entry_to_input_preserves_all_fields() {
        let entry = ExitExpressionEntry {
            patterns: vec!["a".to_string(), "b".to_string()],
            kind: PatternKind::Regex,
            ignore_case: true,
            scope: Some("opencode/k2".to_string()),
        };
        let input = entry.to_input();
        assert_eq!(input.patterns, vec!["a", "b"]);
        assert_eq!(input.kind, PatternKind::Regex);
        assert!(input.ignore_case);
        assert_eq!(input.scope.as_deref(), Some("opencode/k2"));
    }

    // -------------------------------------------------------------------------
    // ExitExpressionsValue — array / object layer deserialization (E1)
    // -------------------------------------------------------------------------

    #[test]
    fn layer_array_shorthand_has_no_explicit_mode() {
        let value: ExitExpressionsValue = serde_json::from_value(serde_json::json!([
            { "pattern": "a" },
            { "pattern": "b" }
        ]))
        .unwrap();
        assert!(value.explicit_mode().is_none());
        assert_eq!(value.rules().len(), 2);
    }

    #[test]
    fn layer_object_form_carries_explicit_mode() {
        let value: ExitExpressionsValue = serde_json::from_value(serde_json::json!({
            "mode": "merge",
            "rules": [{ "pattern": "a" }]
        }))
        .unwrap();
        assert_eq!(value.explicit_mode(), Some(LayerMode::Merge));
        assert_eq!(value.rules().len(), 1);
    }

    #[test]
    fn layer_object_defaults_to_override_mode() {
        let value: ExitExpressionsValue =
            serde_json::from_value(serde_json::json!({ "rules": [] })).unwrap();
        assert_eq!(value.explicit_mode(), Some(LayerMode::Override));
    }

    #[test]
    fn layer_object_rejects_unknown_field() {
        let result = serde_json::from_value::<ExitExpressionsValue>(serde_json::json!({
            "mode": "merge",
            "rules": [],
            "extra": true
        }));
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // ScopeSelector + parse_scope (E2)
    // -------------------------------------------------------------------------

    #[test]
    fn scope_absent_is_global() {
        assert_eq!(ScopeSelector::parse("").unwrap(), ScopeSelector::Global);
        assert_eq!(ScopeSelector::parse("   ").unwrap(), ScopeSelector::Global);
    }

    #[test]
    fn scope_agent_only_matches_any_model_under_provider() {
        let sel = ScopeSelector::parse("opencode").unwrap();
        assert_eq!(sel, ScopeSelector::Agent(Provider::OpenCode));
        assert!(sel.matches(Provider::OpenCode, "anything"));
        assert!(sel.matches(Provider::OpenCode, "kimi-for-coding/k2p7"));
        assert!(!sel.matches(Provider::Claude, "anything"));
    }

    #[test]
    fn scope_agent_plus_model_matches_exactly() {
        let sel = ScopeSelector::parse("opencode/kimi-for-coding/k2p7").unwrap();
        assert_eq!(
            sel,
            ScopeSelector::AgentModel(Provider::OpenCode, "kimi-for-coding/k2p7".to_string())
        );
        // Model keeps its inner `/` (split is on the FIRST `/`).
        assert!(sel.matches(Provider::OpenCode, "kimi-for-coding/k2p7"));
        assert!(!sel.matches(Provider::OpenCode, "other-model"));
        assert!(!sel.matches(Provider::Claude, "kimi-for-coding/k2p7"));
    }

    #[test]
    fn scope_accepts_alias_forms_of_provider() {
        // `opencode`, `open_code`, `open-code` all resolve to OpenCode.
        assert_eq!(
            ScopeSelector::parse("opencode").unwrap(),
            ScopeSelector::Agent(Provider::OpenCode)
        );
        assert_eq!(
            ScopeSelector::parse("open_code").unwrap(),
            ScopeSelector::Agent(Provider::OpenCode)
        );
        assert_eq!(
            ScopeSelector::parse("open-code").unwrap(),
            ScopeSelector::Agent(Provider::OpenCode)
        );
    }

    #[test]
    fn scope_trailing_slash_treated_as_agent_only() {
        // `"opencode/"` → Agent(OpenCode); empty model segment means any model.
        let sel = ScopeSelector::parse("opencode/").unwrap();
        assert_eq!(sel, ScopeSelector::Agent(Provider::OpenCode));
    }

    #[test]
    fn scope_unknown_agent_rejected() {
        let err = ScopeSelector::parse("nonsense/k2").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nonsense"), "error: {msg}");
    }

    #[test]
    fn scope_global_matches_everything() {
        let sel = ScopeSelector::Global;
        for provider in [Provider::Claude, Provider::OpenCode, Provider::KimiCode] {
            assert!(sel.matches(provider, "any-model"));
        }
    }

    #[test]
    fn parse_scope_returns_raw_segments() {
        assert_eq!(parse_scope(""), (None, None));
        assert_eq!(parse_scope("opencode"), (Some("opencode"), None));
        assert_eq!(
            parse_scope("opencode/kimi-for-coding/k2p7"),
            (Some("opencode"), Some("kimi-for-coding/k2p7"))
        );
    }

    // -------------------------------------------------------------------------
    // resolve_exit_expressions — three-layer pipeline (E1)
    // -------------------------------------------------------------------------

    fn entry(pattern: &str) -> ExitExpressionEntry {
        ExitExpressionEntry {
            patterns: vec![pattern.to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }
    }

    fn patterns_of(entries: &[ExitExpressionEntry]) -> Vec<&str> {
        entries
            .iter()
            .flat_map(|e| e.patterns.iter().map(String::as_str))
            .collect()
    }

    fn array(entries: &[ExitExpressionEntry]) -> ExitExpressionsValue {
        ExitExpressionsValue::Array(entries.to_vec())
    }

    fn object(mode: LayerMode, entries: &[ExitExpressionEntry]) -> ExitExpressionsValue {
        ExitExpressionsValue::Object(ExitExpressionsLayer {
            mode,
            rules: entries.to_vec(),
        })
    }

    #[test]
    fn resolve_with_only_user_returns_user_rules() {
        let user = array(&[entry("a"), entry("b")]);
        let resolved = resolve_exit_expressions(Some(&user), None, None);
        assert_eq!(patterns_of(&resolved), vec!["a", "b"]);
    }

    #[test]
    fn resolve_repo_default_override_replaces_user() {
        let user = array(&[entry("a"), entry("b")]);
        let repo = array(&[entry("c")]);
        let resolved = resolve_exit_expressions(Some(&user), Some(&repo), None);
        // repo present, array form → default override → effective = repo only.
        assert_eq!(patterns_of(&resolved), vec!["c"]);
    }

    #[test]
    fn resolve_repo_explicit_merge_adds_to_user() {
        let user = array(&[entry("a")]);
        let repo = object(LayerMode::Merge, &[entry("b")]);
        let resolved = resolve_exit_expressions(Some(&user), Some(&repo), None);
        assert_eq!(patterns_of(&resolved), vec!["a", "b"]);
    }

    #[test]
    fn resolve_frontmatter_default_merge_adds_on_top_of_repo() {
        let user = array(&[entry("a")]);
        let repo = array(&[entry("b")]); // default override → effective = [b]
        let fm = array(&[entry("c")]); // default merge → effective = [b, c]
        let resolved = resolve_exit_expressions(Some(&user), Some(&repo), Some(&fm));
        assert_eq!(patterns_of(&resolved), vec!["b", "c"]);
    }

    #[test]
    fn resolve_frontmatter_explicit_override_replaces_all() {
        let user = array(&[entry("a"), entry("b")]);
        let repo = object(LayerMode::Merge, &[entry("c")]); // effective = [a, b, c]
        let fm = object(LayerMode::Override, &[entry("d")]); // effective = [d]
        let resolved = resolve_exit_expressions(Some(&user), Some(&repo), Some(&fm));
        assert_eq!(patterns_of(&resolved), vec!["d"]);
    }

    #[test]
    fn resolve_skips_absent_layers() {
        // No user, no repo, no frontmatter → empty set.
        assert!(resolve_exit_expressions(None, None, None).is_empty());

        // No user; repo present → repo becomes the base.
        let repo = array(&[entry("x")]);
        let resolved = resolve_exit_expressions(None, Some(&repo), None);
        assert_eq!(patterns_of(&resolved), vec!["x"]);
    }

    #[test]
    fn resolve_frontmatter_only() {
        let user = array(&[entry("a")]);
        let fm = array(&[entry("z")]); // default merge → effective = [a, z]
        let resolved = resolve_exit_expressions(Some(&user), None, Some(&fm));
        assert_eq!(patterns_of(&resolved), vec!["a", "z"]);
    }

    // -------------------------------------------------------------------------
    // extract_frontmatter_exit_expressions
    // -------------------------------------------------------------------------

    #[test]
    fn frontmatter_extract_returns_none_when_absent() {
        let fm = Map::new();
        assert!(extract_frontmatter_exit_expressions(&fm).unwrap().is_none());
    }

    #[test]
    fn frontmatter_extract_returns_array_form() {
        let mut fm = Map::new();
        fm.insert(
            "exit_expressions".to_string(),
            serde_json::json!([{ "pattern": "STOP." }]),
        );
        let extracted = extract_frontmatter_exit_expressions(&fm).unwrap().unwrap();
        assert!(extracted.explicit_mode().is_none());
        assert_eq!(extracted.rules().len(), 1);
    }

    #[test]
    fn frontmatter_extract_returns_object_form() {
        let mut fm = Map::new();
        fm.insert(
            "exit_expressions".to_string(),
            serde_json::json!({ "mode": "override", "rules": [{ "pattern": "x" }] }),
        );
        let extracted = extract_frontmatter_exit_expressions(&fm).unwrap().unwrap();
        assert_eq!(extracted.explicit_mode(), Some(LayerMode::Override));
    }

    #[test]
    fn frontmatter_extract_errors_on_invalid_shape() {
        let mut fm = Map::new();
        fm.insert(
            "exit_expressions".to_string(),
            serde_json::json!({ "mode": "tuesday", "rules": [] }),
        );
        assert!(extract_frontmatter_exit_expressions(&fm).is_err());
    }

    // -------------------------------------------------------------------------
    // GuardSettings — defaults + last-writer resolution
    // -------------------------------------------------------------------------

    #[test]
    fn guard_settings_default_matches_spec_constants() {
        let gs = GuardSettings::default();
        assert!(gs.repetition.enabled);
        assert_eq!(gs.repetition.max_repeats, MAX_REPETITION_ALLOWED);
        assert_eq!(gs.repetition.max_cycle_length, MAX_CYCLE_LENGTH);
        assert!(gs.volume.enabled);
        assert_eq!(gs.volume.max_lines, VOLUME_LINES);
        assert_eq!(gs.volume.max_bytes, VOLUME_BYTES);
    }

    #[test]
    fn guard_settings_round_trip() {
        let gs = GuardSettings {
            repetition: RepetitionGuardSettings {
                enabled: false,
                max_repeats: 12,
                max_cycle_length: 8,
            },
            volume: VolumeGuardSettings {
                enabled: true,
                max_lines: 1000,
                max_bytes: 1024 * 1024,
            },
        };
        let json = serde_json::to_value(&gs).unwrap();
        let back: GuardSettings = serde_json::from_value(json).unwrap();
        assert_eq!(back, gs);
    }

    #[test]
    fn guard_settings_deserializes_partial_form() {
        // Only `repetition.enabled` set; everything else gets defaults.
        let gs: GuardSettings = serde_json::from_value(serde_json::json!({
            "repetition": { "enabled": false }
        }))
        .unwrap();
        assert!(!gs.repetition.enabled);
        assert_eq!(gs.repetition.max_repeats, MAX_REPETITION_ALLOWED);
        assert!(gs.volume.enabled);
        assert_eq!(gs.volume.max_bytes, VOLUME_BYTES);
    }

    #[test]
    fn guard_settings_rejects_unknown_field() {
        let result = serde_json::from_value::<GuardSettings>(serde_json::json!({
            "posture": "strict"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_guard_settings_user_only_returns_user() {
        let user = GuardSettings::default();
        let resolved = resolve_guard_settings(&user, None, None);
        assert_eq!(resolved, user);
    }

    #[test]
    fn resolve_guard_settings_repo_overrides_user() {
        let user = GuardSettings::default();
        let repo = GuardSettings {
            repetition: RepetitionGuardSettings {
                enabled: false,
                ..RepetitionGuardSettings::default()
            },
            ..GuardSettings::default()
        };
        let resolved = resolve_guard_settings(&user, Some(&repo), None);
        assert!(!resolved.repetition.enabled);
    }

    #[test]
    fn resolve_guard_settings_frontmatter_overrides_repo() {
        let user = GuardSettings::default();
        let repo = GuardSettings {
            repetition: RepetitionGuardSettings {
                enabled: false,
                ..RepetitionGuardSettings::default()
            },
            ..GuardSettings::default()
        };
        let fm = GuardSettings::default(); // re-enabled at frontmatter
        let resolved = resolve_guard_settings(&user, Some(&repo), Some(&fm));
        assert!(resolved.repetition.enabled);
    }

    // -------------------------------------------------------------------------
    // validate_exit_expressions
    // -------------------------------------------------------------------------

    #[test]
    fn validate_accepts_clean_literal_entry() {
        let entries = vec![ExitExpressionEntry {
            patterns: vec!["STOP.".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: Some("opencode".to_string()),
        }];
        assert!(validate_exit_expressions(&entries).is_ok());
    }

    #[test]
    fn validate_accepts_clean_regex_entry() {
        let entries = vec![ExitExpressionEntry {
            patterns: vec![r"^(STOP|Bye)\.$".to_string()],
            kind: PatternKind::Regex,
            ignore_case: false,
            scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
        }];
        assert!(validate_exit_expressions(&entries).is_ok());
    }

    #[test]
    fn validate_rejects_empty_patterns() {
        let entries = vec![ExitExpressionEntry {
            patterns: vec![],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        }];
        let err = validate_exit_expressions(&entries).unwrap_err();
        assert!(err.to_string().contains("empty"), "error: {err}");
    }

    #[test]
    fn validate_rejects_invalid_regex_at_load() {
        let entries = vec![ExitExpressionEntry {
            patterns: vec!["[".to_string()],
            kind: PatternKind::Regex,
            ignore_case: false,
            scope: None,
        }];
        assert!(validate_exit_expressions(&entries).is_err());
    }

    #[test]
    fn validate_rejects_unknown_agent_in_scope() {
        let entries = vec![ExitExpressionEntry {
            patterns: vec!["x".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: Some("nonsense/k2".to_string()),
        }];
        let err = validate_exit_expressions(&entries).unwrap_err();
        assert!(err.to_string().contains("nonsense"), "error: {err}");
    }

    #[test]
    fn validate_accepts_all_three_scope_kinds() {
        let entries = vec![
            ExitExpressionEntry {
                patterns: vec!["global".to_string()],
                kind: PatternKind::Literal,
                ignore_case: false,
                scope: None,
            },
            ExitExpressionEntry {
                patterns: vec!["agent".to_string()],
                kind: PatternKind::Literal,
                ignore_case: false,
                scope: Some("claude".to_string()),
            },
            ExitExpressionEntry {
                patterns: vec![r"am$".to_string()],
                kind: PatternKind::Regex,
                ignore_case: false,
                scope: Some("claude/sonnet-4".to_string()),
            },
        ];
        assert!(validate_exit_expressions(&entries).is_ok());
    }
}
