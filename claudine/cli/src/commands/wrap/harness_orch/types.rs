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
    /// The evaluated `proxy.with` mapping for this document — immutable,
    /// pre-schema, and never written to disk.
    ///
    /// Merged into the target's *authored* frontmatter at every read of it, so
    /// it participates in frontmatter interpolation, selection hints,
    /// `initialize` conditions, schema validation, lifecycle parsing, loop
    /// configuration, and the body alike. Pre-schema is what makes a refresh
    /// deterministic: schema coercion and invalid-optional drops shape the
    /// prepared effective frontmatter, and if they could write back here, each
    /// refresh would reapply a different overlay than the one the source
    /// authored.
    ///
    /// Empty for a directly invoked document. Replaced wholesale — never merged
    /// — when this document proxies on.
    pub(crate) overlay: indexmap::IndexMap<String, serde_json::Value>,
    pub(crate) prompt_tail: Vec<String>,
    /// The caller's input layers, reapplied at every canonical preparation of
    /// this document. Sourced from
    /// [`PreparedComposition::input_layers`][claudine::composition::PreparedComposition].
    /// Empty for direct-wrapper passthrough runs, which have no compose params.
    pub(crate) input_layers: claudine::composition::CallerInputLayers,
    /// Why the next preparation of this document is happening — stamped by the
    /// control dispatch that decided the re-entry, so preparation reads the
    /// stage row rather than re-deriving it from loop-local state.
    pub(crate) entry: claudine::composition::DocumentEntryReason,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedHarnessPrompt {
    pub(crate) frontmatter: serde_json::Value,
    pub(crate) prompt: String,
    pub(crate) env_overrides: Vec<(String, String)>,
    /// The provider/model selection hints canonical preparation parsed from this
    /// document's effective frontmatter.
    ///
    /// Carried so the R6 target launch rebuild can re-resolve a proxied target's
    /// provider/model from the target's *own* `agent:`/`model:` hints (honoring
    /// immutable CLI precedence) rather than inheriting the router's resolved
    /// identity. Empty for the passthrough seed and the synthesized empty prompt.
    pub(crate) selection_hints: claudine::composition::EffectiveSelectionHints,
    pub(crate) inline_closure_plan: Option<claudine::composition::InlineClosurePlan>,
    /// The lifecycle config canonical preparation parsed for this document,
    /// with its shell commands already C3-resolved.
    ///
    /// Carried rather than re-parsed from [`Self::frontmatter`]: a re-parse
    /// yields the *authored* commands with their `{{ }}` spans intact, so the
    /// bytes the bootstrap gate approves would not be the bytes the executor
    /// runs. `None` only for the passthrough seed and the synthesized empty
    /// prompt, neither of which has a document lifecycle.
    pub(crate) lifecycle: Option<claudine::composition::LifecycleConfig>,
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
    pub(crate) live_frontmatter: std::cell::RefCell<serde_json::Map<String, serde_json::Value>>,
    /// The MCP `#tag`s this document's **canonically composed** body selects,
    /// sorted and deduped.
    ///
    /// Captured here rather than re-lexed downstream because neither surviving
    /// text can stand in for it. [`Self::prompt`] is the provider input, which a
    /// resume deliberately replaces with the follow-up message; the document on
    /// disk is raw authored Markdown, so it predates `proxy.with`, caller
    /// overrides, and frontmatter interpolation — a body of `#{{ mcp_server }}`
    /// names a server there only after composition. The launch rebuild reads
    /// this field so retry and resume select the same servers a direct
    /// invocation of the composed document selects (review-11 finding 3).
    pub(crate) mcp_body_tags: Vec<String>,
}

impl MaterializedHarnessPrompt {
    /// The sorted, deduped MCP tag set of a composed body.
    ///
    /// Single-sources the normalization the invocation pipeline applies when it
    /// records its own launch facets (`composition::pipeline`), so a rebuilt
    /// facet set compares equal to the recorded one for an unchanged document.
    pub(crate) fn lex_body_mcp_tags(body: &str) -> Vec<String> {
        let (_cleaned, mut tags) = claudine::mcp::session::lex_tags(body);
        tags.sort();
        tags.dedup();
        tags
    }

    /// Build the per-attempt live-frontmatter cell from a frontmatter value.
    ///
    /// A non-object frontmatter (e.g. the synthesized `Null` used when
    /// materialization failed) seeds an empty map, matching the stack-context
    /// builder's empty-frontmatter fallback.
    pub(crate) fn live_cell_from(
        frontmatter: &serde_json::Value,
    ) -> std::cell::RefCell<serde_json::Map<String, serde_json::Value>> {
        std::cell::RefCell::new(frontmatter.as_object().cloned().unwrap_or_default())
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
