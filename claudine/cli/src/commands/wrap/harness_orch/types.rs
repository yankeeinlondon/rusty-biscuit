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
    /// Caller-supplied compose inputs re-applied on every re-materialization
    /// (retry/resume/proxy), so a re-composed document keeps the caller's
    /// `--set` params, launch-area file-ref anchor, and pre-approved shell
    /// commands. Sourced from
    /// [`PreparedComposition::rematerialize`][claudine::composition::PreparedComposition].
    /// Empty for direct-wrapper passthrough runs, which have no compose params.
    pub(crate) rematerialize: claudine::composition::RematerializeInputs,
    /// The invocation-local runtime state cell.
    ///
    /// Lives here rather than on [`MaterializedHarnessPrompt`] because it must
    /// outlive every re-materialization: a `set` written in iteration 1 is
    /// still visible in iteration 5, and the `outputs` accumulator grows across
    /// the whole invocation. Each materialization clones the handle so the
    /// lifecycle executor writes through to this one cell.
    pub(crate) runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
    /// Withhold this run's `outputs` commit because the caller owns output
    /// timing (the sequence task executor appends only after `teardown`).
    /// [`Self::last_final_output`] still carries the captured text out.
    pub(crate) suppress_output_commit: bool,
    /// The most recent successful run's captured final text.
    ///
    /// Recorded whether or not the commit was suppressed, so a caller that
    /// withheld the commit can still read what the run produced.
    pub(crate) last_final_output: Option<String>,
}

impl HarnessPromptState {
    /// Adopt a proxy target that has already passed file-reference policy.
    ///
    /// The explicit trusted derivation preserves the immutable request inputs
    /// while allowing the accepted target's authored path identity to differ
    /// from the request root (including macOS `/var` → `/private/var` aliases).
    pub(crate) fn adopt_resolved_proxy_source(&mut self, source_path: PathBuf) {
        if let Some(context) = self.rematerialize.file_resolution_context.take() {
            self.rematerialize.file_resolution_context =
                Some(context.for_trusted_external_source(&source_path));
        }
        self.source_path = source_path;
    }
}

#[derive(Debug)]
pub(crate) struct MaterializedHarnessPrompt {
    pub(crate) frontmatter: serde_json::Value,
    pub(crate) prompt: String,
    pub(crate) env_overrides: Vec<(String, String)>,
    pub(crate) inline_closure_plan: Option<claudine::composition::InlineClosurePlan>,
    pub(crate) file_resolution_context: Option<biscuit_file::FileResolutionContext>,
    /// Shared cross-event live document frontmatter for the current attempt.
    ///
    /// Seeded from `frontmatter` when the prompt is materialized and threaded
    /// into every lifecycle event's [`claudine::composition::lifecycle_executor::StackExecutionContext`]
    /// for this iteration. A lifecycle frontmatter side effect that targets the
    /// document persists here so a *later* event in the same attempt
    /// (`start` → `success`/`finalize`) reads the mutated value, per the
    /// late-binding spec's "current effective document state at the moment the
    /// event fires" contract. Re-created each loop iteration (a retry
    /// re-materializes from disk), giving the correct per-attempt lifetime.
    pub(crate) live_frontmatter: std::sync::Mutex<serde_json::Map<String, serde_json::Value>>,
    /// Handle to the invocation-local runtime cell owned by
    /// [`HarnessPromptState`]. Threaded into every lifecycle event so a `set`
    /// accumulates across attempts instead of dying with `live_frontmatter`.
    pub(crate) runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
}

impl MaterializedHarnessPrompt {
    /// Build the per-attempt live-frontmatter cell from a frontmatter value.
    ///
    /// A non-object frontmatter (e.g. the synthesized `Null` used when
    /// materialization failed) seeds an empty map, matching the stack-context
    /// builder's empty-frontmatter fallback.
    pub(crate) fn live_cell_from(
        frontmatter: &serde_json::Value,
    ) -> std::sync::Mutex<serde_json::Map<String, serde_json::Value>> {
        std::sync::Mutex::new(frontmatter.as_object().cloned().unwrap_or_default())
    }
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
    /// Resolved OpenCode stalled-generation backstop budget (CLI > env >
    /// built-in `10m`). `None` disables the backstop.
    /// Honored only by the OpenCode bridge in structured-stream mode; passed
    /// to `build_structured_plumbing`, ignored on every other path.
    pub(crate) stall_timeout: Option<std::time::Duration>,
    /// True when `stall_timeout` came from CLI or a valid env value instead of
    /// Claudine's built-in `10m` default. Drives the
    /// "only enforced in structured-stream mode" warning on non-structured
    /// attempts.
    pub(crate) stall_timeout_user_configured: bool,
}
