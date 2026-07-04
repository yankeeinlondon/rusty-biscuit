//! Render-policy shells consumed by functional render components.
//!
//! Placeholders for Phase G of the provider-metadata plan
//! (`design/render-components.md`): components read typed policy fields and
//! contain zero `match Provider`. The populated values become a generated
//! catalog section; only the type shells live here for now.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Per-provider display policy consumed by shared render components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DisplayPolicy {
    /// Whether tool results are summarized in live output.
    pub tool_result_summary: ToolResultSummary,
    /// Informational event classes suppressed for this provider.
    pub info_event_suppression: &'static [EventClass],
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
    Show,
    Suppress,
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
