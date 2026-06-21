use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPromptMode {
    Passthrough,
    Inline,
    Compose,
}

pub(crate) fn harness_prompt_mode_label(mode: HarnessPromptMode) -> &'static str {
    match mode {
        HarnessPromptMode::Passthrough => "passthrough",
        HarnessPromptMode::Inline => "inline",
        HarnessPromptMode::Compose => "compose",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HarnessPromptState {
    pub(crate) mode: HarnessPromptMode,
    pub(crate) source_path: PathBuf,
    /// The original file reference string (for reporting).
    pub(crate) original_ref: String,
    pub(crate) base_prompt: Option<String>,
    pub(crate) overlay: indexmap::IndexMap<String, serde_json::Value>,
    pub(crate) prompt_tail: Vec<String>,
    pub(crate) next_prompt_override: Option<String>,
    pub(crate) next_resume_session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedHarnessPrompt {
    pub(crate) frontmatter: serde_json::Value,
    pub(crate) prompt: String,
    pub(crate) env_overrides: Vec<(String, String)>,
    pub(crate) inline_closure_plan: Option<claudine::composition::InlineClosurePlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct AttemptLaunch {
    pub(crate) args: Vec<String>,
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) stdin_seed: Option<String>,
    /// Wire-mode JSON-RPC prompt body, when the provider's prompt
    /// delivery requested transport via [`super::super::wire_io::run_kimi_wire_session`]
    /// instead of stdin / argv. Mutually exclusive with `stdin_seed` for
    /// the same launch.
    pub(crate) wire_prompt: Option<String>,
    /// Unified timeout configuration resolved through the full precedence
    /// chain (CLI > frontmatter > env > built-in default). Drives the
    /// timeout watchdog ticker for `timeout` (wall-clock) and
    /// `step_timeout` (stream silence). Carries the supporting knobs
    /// `kill_grace` and `interval` from `CLAUDINE_KILL_GRACE` /
    /// `CLAUDINE_WATCHDOG_INTERVAL`.
    pub(crate) timeout_config: super::super::subagent_watchdog::TimeoutConfig,
    /// True when `step_timeout` came from CLI, frontmatter, or a valid env
    /// value instead of Claudine's built-in default.
    pub(crate) step_timeout_user_configured: bool,
}
