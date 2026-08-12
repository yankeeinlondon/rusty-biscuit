//! Render-policy types consumed by functional render components.
//!
//! Phase G of the provider-metadata plan (`design/render-components.md`
//! ruling 1): components read typed policy fields and contain zero
//! `match Provider`. The populated values are a generated catalog section
//! of `ProviderInfo` (per-provider, overridable like any field); this
//! crate owns only the type shells.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Per-provider display policy consumed by shared render components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DisplayPolicy {
    /// How tool results are summarized in live output.
    pub tool_result_summary: ToolResultSummary,
    /// Informational event classes suppressed for this provider.
    pub info_event_suppression: &'static [EventClass],
    /// Whether task-progress narration lines are held back and collapsed
    /// into the matching tool-call line that follows.
    pub collapse_task_progress: bool,
    /// Whether the generic rate-limit warning is suppressed when session
    /// metadata shows subscription (non-API-key) auth.
    pub suppress_subscription_rate_limit: bool,
    /// Provider-extension kinds whose stderr status line is suppressed.
    /// Suppressed events still flow through dispatch and the JSONL log.
    pub silent_extension_kinds: &'static [&'static str],
    /// Line prefixes on stdout that are provider noise, not content.
    pub stdout_noise_prefixes: &'static [&'static str],
    /// Line prefixes on stderr that are provider noise, not content.
    pub stderr_noise_prefixes: &'static [&'static str],
}

/// Whether a provider's tool results get a rendered summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ToolResultSummary {
    /// Always render the one-line tool-result summary.
    Show,
    /// When a successful non-file tool result carries a renderable body,
    /// suppress the one-line summary and render the body as a block quote
    /// at Normal verbosity.
    PreferBody,
}

/// Coarse classification of normalized stream events for policy dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventClass {
    ToolUse,
    Thinking,
    FinalMessage,
    McpCall,
    HookEvent,
    StepProgress,
    FileChange,
    PlanUpdate,
    SubagentActivity,
    Error,
}
