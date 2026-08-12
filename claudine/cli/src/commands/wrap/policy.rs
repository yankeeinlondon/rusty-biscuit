use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::log;

#[derive(Clone)]
pub(crate) struct StructuredCodexOutput {
    pub(crate) last_message_path: PathBuf,
}

impl StructuredCodexOutput {
    pub(crate) fn prepare(args: &mut Vec<String>) -> Self {
        let path = std::env::temp_dir().join(format!(
            "claudine-codex-last-message-{}.txt",
            uuid::Uuid::new_v4()
        ));
        args.push("--output-last-message".to_string());
        args.push(path.to_string_lossy().into_owned());
        Self {
            last_message_path: path,
        }
    }

    pub(crate) fn apply_to_summary(
        &self,
        summary: &mut claudine::stream::summary::StreamExecutionSummary,
    ) {
        if let Ok(text) = fs::read_to_string(&self.last_message_path)
            && !text.trim().is_empty()
        {
            summary.assistant_text = text;
        }
        let _ = fs::remove_file(&self.last_message_path);
    }

    /// Recover the final assistant message on the interactive-Codex path:
    /// read the `--output-last-message` file (empty when Codex wrote
    /// nothing) and remove it. Unlike [`Self::apply_to_summary`], the raw
    /// text is returned even when blank — the interactive caller treats an
    /// empty response as "no final message" itself.
    pub(crate) fn take_last_message(&self) -> String {
        let text = fs::read_to_string(&self.last_message_path).unwrap_or_default();
        let _ = fs::remove_file(&self.last_message_path);
        text
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuredSummaryDetails {
    pub(crate) tool_names: Vec<String>,
    /// The agent's **final response** — output text emitted after the last
    /// tool call. Accumulated from `OutputText` semantic events and reset
    /// whenever a `ToolCall` is observed, so interstitial narration between
    /// tool calls (e.g. "Let me read the docs…") is dropped and only the
    /// closing turn survives. `inline-compose` writes this (not the
    /// full accumulated `assistant_text`) into the document body so process
    /// narration never leaks into the artifact.
    pub(crate) final_response: String,
}

impl StructuredSummaryDetails {
    pub(crate) fn record_tool_name(&mut self, tool_name: &str) {
        if !tool_name.is_empty() && !self.tool_names.iter().any(|name| name == tool_name) {
            self.tool_names.push(tool_name.to_string());
        }
    }

    /// Append a streamed `OutputText` chunk to the in-progress final response.
    pub(crate) fn push_final_response(&mut self, text: &str) {
        self.final_response.push_str(text);
    }

    /// Discard any accumulated final-response text because a new tool call
    /// began — anything said before the last tool call is process narration,
    /// not the agent's closing answer.
    pub(crate) fn reset_final_response(&mut self) {
        self.final_response.clear();
    }
}

/// Build the per-run provider-attributed [`SignalHub`], with unmatched-event
/// harvesting enabled when opted in (E6): `CLAUDINE_HARVEST` env override
/// wins over the user config's `harvest_unmatched`, default off.
///
/// Hub creation is also the once-per-run seam for the listing-drift
/// check: the cached dynamic listing is diffed against the expected
/// baseline and any drift lands in this hub before the child spawns.
///
/// [`SignalHub`]: claudine::signals::SignalHub
pub(crate) fn provider_signal_hub(provider: Provider) -> Arc<claudine::signals::SignalHub> {
    let hub = Arc::new(claudine::signals::SignalHub::new(
        claudine::signals::for_provider(provider),
    ));
    if harvest_enabled() {
        hub.enable_harvest();
    }
    super::catalog_drift::emit_listing_drift(provider, &hub);
    hub
}

fn harvest_enabled() -> bool {
    if let Some(enabled) = claudine::signals::harvest::env_override() {
        return enabled;
    }
    match claudine::dispatch::loader::load_claudine_config(None, None) {
        Ok(config) => config.harvest_unmatched,
        // A missing or broken config never blocks a run; harvest stays off.
        Err(_) => false,
    }
}

/// Build the structured-stream parser builder plus an optional stderr bridge.
///
/// All providers share the same stdout parser construction pattern, but
/// OpenCode additionally wires an [`OpenCodeLogBridge`] into the stderr
/// reader thread so classified `--print-logs` records flow through the
/// same semantic sink as stdout events. When a bridge is returned, the
/// caller is responsible for threading it through
/// [`exec::run_child_stream_semantic`] so the stderr thread can consume
/// classified lines and the final summary can merge the bridge's
/// accumulated diagnostics.
///
/// ## Returns
///
/// * `build_parser` — parser-builder closure passed to
///   [`exec::run_child_stream_semantic`].
/// * `stderr_bridge` — `Some` for OpenCode, `None` otherwise. The bridge
///   owns its own shared state clone so the finalizer closure can merge
///   stderr-derived diagnostics into the summary after the reader threads
///   join.
///
/// [`OpenCodeLogBridge`]: claudine::stream::logs::opencode::OpenCodeLogBridge
pub(crate) fn build_structured_plumbing(
    provider: Provider,
    sink: super::live_semantic_sink::LiveSemanticSink,
    parser_config: claudine::stream::ParserConfig,
    stall_timeout: Option<std::time::Duration>,
    signal_hub: &Arc<claudine::signals::SignalHub>,
) -> (
    super::exec::SemanticParserBuilder,
    Option<claudine::stream::logs::StderrBridgeHandle>,
    Option<std::sync::mpsc::Receiver<claudine::stream::logs::EarlyTermination>>,
) {
    use claudine::stream::logs::StderrBridgeHandle;
    use claudine::stream::logs::codex::CodexLogBridge;
    use claudine::stream::logs::opencode::{OpenCodeLogBridge, merge_stderr_state_into_summary};
    use claudine::stream::semantic::{
        ObservedSemanticSink, SharedSemanticSink, StalledProgressObserverSink,
    };
    use std::sync::atomic::AtomicBool;

    // Whether the run wants a content-detector trip channel (Phase 6). True
    // when a detector is already armed OR when a re-scope source could arm
    // one on a provider-reported `SessionStart` model (the launch-time
    // compile can legitimately produce no detector yet still need a channel).
    // When true, this function owns the unified early-termination channel:
    // the detector's trip sender and (for OpenCode) the stderr bridge's
    // sender are clones feeding one receiver the wait loop polls.
    let detector_armed = sink.wants_content_channel();

    // The stalled-generation backstop is OpenCode-only. A resolved budget on
    // any other provider is inert config (the key is provider-neutral for
    // portable prompt files); trace it at debug level and never warn.
    if provider != Provider::OpenCode && stall_timeout.is_some() {
        tracing::debug!(
            %provider,
            "stall_timeout is OpenCode-scoped; ignored for this provider",
        );
    }

    if provider == Provider::OpenCode {
        let shared = SharedSemanticSink::new(sink);
        let live_sink_inner = Arc::clone(shared.inner());
        let stdout_seen = Arc::new(AtomicBool::new(false));

        let (early_tx, early_rx) = std::sync::mpsc::channel();
        // The content detector shares the bridge's early-termination sender
        // so a runaway trip and a usage-cap abort converge on one receiver.
        if detector_armed
            && let Ok(mut inner) = live_sink_inner.lock()
        {
            inner.set_trip_sender(early_tx.clone());
        }
        // The resolved stalled-generation backstop budget (CLI > frontmatter
        // > env > built-in `10m`). `None` disables the guard.
        // The signal hub is shared with the stdout reader thread (spawn.rs)
        // so classified stderr records and stdout NDJSON feed one dedup sink
        // — this is the glue-mode runtime shim's wiring point (E5).
        let bridge = OpenCodeLogBridge::new(
            shared.clone(),
            Arc::clone(&stdout_seen),
            Some(early_tx),
            stall_timeout,
        )
        .with_signal_hub(Arc::clone(signal_hub));
        let bridge_state = bridge.shared_state();
        // Share the bridge's stalled-generation progress clocks with the stdout
        // path so stdout-origin progress (OutputText/ToolCall/…) resets the same
        // counter the stderr `llm_call_start` churn accumulates against.
        let stalled_progress = bridge.stalled_generation_progress();
        let finalize: claudine::stream::logs::SummaryFinalizer = Box::new(move |summary| {
            merge_stderr_state_into_summary(&bridge_state, summary);
        });
        let stderr_bridge = Some(StderrBridgeHandle {
            bridge: Box::new(bridge),
            finalize,
            early_terminate: Some(early_rx),
        });

        let progress_observed = StalledProgressObserverSink::new(shared, stalled_progress);
        let stdout_sink = ObservedSemanticSink::new(progress_observed, stdout_seen);
        let build_parser: super::exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb, agent_pid| {
                if let Ok(mut inner) = live_sink_inner.lock() {
                    inner.set_output_text_sink(output_cb);
                    inner.set_agent_pid(agent_pid);
                }
                claudine::stream::create_semantic_parser(provider, stdout_sink, parser_config)
            });
        // The receiver reaches the wait loop via the bridge handle.
        (build_parser, stderr_bridge, None)
    } else if provider == Provider::Codex {
        // Codex emits `tracing-subscriber` records on stderr that we'd
        // rather render inline through the live sink (as an orange
        // BlockQuote) than leak raw to the terminal. Share the sink so the
        // stdout parser and the stderr bridge feed one rendering pipeline.
        let shared = SharedSemanticSink::new(sink);
        let live_sink_inner = Arc::clone(shared.inner());

        // Codex's bridge has no early-termination of its own, so the
        // detector gets a dedicated channel whose receiver is returned
        // separately and routed into the wait loop alongside the bridge.
        let content_rx = if detector_armed {
            let (tx, rx) = std::sync::mpsc::channel();
            if let Ok(mut inner) = live_sink_inner.lock() {
                inner.set_trip_sender(tx);
            }
            Some(rx)
        } else {
            None
        };

        let bridge = CodexLogBridge::new(shared.clone());
        let stderr_bridge = Some(StderrBridgeHandle {
            bridge: Box::new(bridge),
            finalize: Box::new(|_summary| {}),
            early_terminate: None,
        });

        let stdout_sink = shared;
        let build_parser: super::exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb, agent_pid| {
                if let Ok(mut inner) = live_sink_inner.lock() {
                    inner.set_output_text_sink(output_cb);
                    inner.set_agent_pid(agent_pid);
                }
                claudine::stream::create_semantic_parser(provider, stdout_sink, parser_config)
            });
        (build_parser, stderr_bridge, content_rx)
    } else {
        // No stderr bridge for this provider; the detector gets its own
        // channel and the receiver is returned for the wait loop.
        let (content_rx, sink) = if detector_armed {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut sink = sink;
            sink.set_trip_sender(tx);
            (Some(rx), sink)
        } else {
            (None, sink)
        };
        let build_parser: super::exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb, agent_pid| {
                let mut sink = sink.with_output_text_sink(output_cb);
                sink.set_agent_pid(agent_pid);
                claudine::stream::create_semantic_parser(provider, sink, parser_config)
            });
        (build_parser, None, content_rx)
    }
}

/// Emit stderr summaries and write synthetic JSONL event after a structured stream session.
///
/// When a [`SectionStream`](live_semantic_sink::SectionStream) handle is
/// supplied, every trailer line is routed through it as
/// [`Section::TrailerMetadata`] so the section-separator blank between the
/// final stdout and the trailer is inserted exactly once (and only when
/// the prior section actually emitted non-blank content). When the handle
/// is absent (legacy / test call sites), emission falls back to plain
/// `eprintln!`.
pub(crate) struct StreamSummaryContext<'a> {
    pub(crate) summary: &'a claudine::stream::summary::StreamExecutionSummary,
    pub(crate) profile: &'a dyn super::profile::WrapperProfile,
    pub(crate) env_context: &'a EnvironmentContext,
    pub(crate) verbosity: Verbosity,
    pub(crate) verbose: bool,
    pub(crate) details: &'a StructuredSummaryDetails,
    pub(crate) section_stream: Option<&'a super::section::SectionStream>,
    /// Immediate child PID captured by the wrapper after a successful
    /// spawn. `None` for failed-spawn paths or paths that never spawn a
    /// provider child. Threaded through to the synthetic summary event
    /// so `EventMeta.agent_pid` carries the spawned child PID.
    pub(crate) agent_pid: Option<u32>,
    /// The run's drained signal observations, destined for
    /// `extra["signals"]` on the SessionEnd summary row only. Passed as
    /// an explicit parameter (not via `context_extra`) because the sink
    /// mirrors `context_extra` onto every live semantic tool row.
    pub(crate) signals: &'a [claudine::signals::ObservedSignal],
    /// The model string the user/frontmatter requested (`--model` or the
    /// `MODEL`/frontmatter value) — NOT the provider-reported model.
    /// When it names a marked rolling alias, the resolved
    /// `family_latest` stamp is written to the summary row.
    pub(crate) requested_model: Option<&'a str>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_stream_summary(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn super::profile::WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
    section_stream: Option<&super::section::SectionStream>,
    agent_pid: Option<u32>,
    signals: &[claudine::signals::ObservedSignal],
    requested_model: Option<&str>,
) {
    emit_stream_summary_inner(
        StreamSummaryContext {
            summary,
            profile,
            env_context,
            verbosity,
            verbose,
            details,
            section_stream,
            agent_pid,
            signals,
            requested_model,
        },
        None,
    );
}

fn emit_stream_summary_inner(
    ctx: StreamSummaryContext<'_>,
    context_extra: Option<&HashMap<String, serde_json::Value>>,
) {
    let StreamSummaryContext {
        summary,
        profile,
        env_context,
        verbosity,
        verbose,
        details,
        section_stream,
        agent_pid,
        signals,
        requested_model,
    } = ctx;
    let primary_markup = if verbosity == Verbosity::Silent {
        None
    } else {
        format_summary_prose(summary)
    };
    let secondary_markup = if verbosity == Verbosity::Silent || !verbose {
        None
    } else {
        format_verbose_summary_details_prose(summary, details)
    };
    if primary_markup.is_some() || secondary_markup.is_some() {
        use super::section::Section;
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::TerminalRenderable;

        let term = crate::log::terminal();
        if let Some(section_stream) = section_stream {
            // Route every trailer line through the shared section tracker.
            // The tracker inserts the section-separator blank exactly once
            // when transitioning into `TrailerMetadata`, so callers do not
            // need any ad-hoc newline bookkeeping.
            if let Some(markup) = primary_markup {
                let rendered = Prose::new(markup).render(&term);
                section_stream.emit_stderr(Section::TrailerMetadata, &rendered);
            }
            if let Some(markup) = secondary_markup {
                let rendered = Prose::new(markup).render(&term);
                section_stream.emit_stderr(Section::TrailerMetadata, &format!("  {rendered}"));
            }
        } else {
            // Legacy / test fallback: keep the original spacing heuristic
            // so callers that do not own a section stream still emit a
            // reasonable separator between stdout text and the trailer.
            if !summary.assistant_text.is_empty() {
                if summary.assistant_text.ends_with("\n\n") {
                    // Already has a trailing blank line — no separator needed.
                } else if summary.assistant_text.ends_with('\n') {
                    eprintln!();
                } else {
                    eprint!("\n\n");
                }
            }
            if let Some(markup) = primary_markup {
                let rendered = Prose::new(markup).render(&term);
                eprintln!("{rendered}");
            }
            if let Some(markup) = secondary_markup {
                let rendered = Prose::new(markup).render(&term);
                eprintln!("  {rendered}");
            }
        }
    }

    // Write synthetic summary event to JSONL (best-effort)
    if let Some(protocol) = profile.stream_protocol() {
        // Stamp only when the REQUESTED model is a marked rolling alias;
        // provider-reported models never match (they are concrete ids).
        let family_latest = requested_model.and_then(|model| {
            claudine::model_catalog::family_latest_stamp(summary.provider, model)
        });
        let meta = claudine::stream::reporting::summary_to_event_meta_with_context(
            summary,
            protocol,
            env_context,
            context_extra,
            agent_pid,
            signals,
            family_latest.as_ref(),
        );
        if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
            tracing::warn!("Failed to write stream summary event: {e}");
        }
    }
}

pub(crate) fn format_summary_prose(
    summary: &claudine::stream::summary::StreamExecutionSummary,
) -> Option<String> {
    use claudine::stream::stderr::{format_cost, format_duration, format_number};

    let prefix = if summary.is_error {
        "\u{2717}"
    } else {
        "\u{2713}"
    };
    let mut parts = Vec::new();

    if let Some(ms) = summary.duration_ms {
        parts.push(format_duration(ms));
    }

    if let Some(usage) = &summary.token_usage {
        if let Some(input) = usage.input {
            parts.push(format!("{} <i>input tokens</i>", format_number(input)));
        }
        if let Some(output) = usage.output {
            parts.push(format!("{} <i>output tokens</i>", format_number(output)));
        }
        if let Some(cache) = usage.cache_read
            && cache > 0
        {
            parts.push(format!("{} <i>cached tokens</i>", format_number(cache)));
        }
    }

    if let Some(cost) = summary.cost_usd {
        parts.push(format!("{} <i>cost basis</i>", format_cost(cost)));
    }

    match summary.tool_calls {
        Some(tc) => parts.push(format!(
            "{tc} <i>tool call{}</i>",
            if tc == 1 { "" } else { "s" }
        )),
        None => parts.push("<i>no tool calls</i>".to_string()),
    }

    if let Some(pp) = summary.permission_prompts {
        parts.push(format!(
            "{pp} <i>permission prompt{}</i>",
            if pp == 1 { "" } else { "s" }
        ));
    }

    if let Some(uip) = summary.user_input_prompts {
        parts.push(format!(
            "{uip} <i>user input prompt{}</i>",
            if uip == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        return None;
    }

    let mut out = format!("<dim>{prefix} {}</dim>", parts.join(" \u{00b7} "));
    for badge in &summary.badges {
        let color = match badge.severity {
            claudine::stream::badges::BadgeSeverity::Error => "red",
            claudine::stream::badges::BadgeSeverity::Warning => "yellow",
            claudine::stream::badges::BadgeSeverity::Info => "cyan",
        };
        out.push('\n');
        out.push_str(&format!(
            "<{color}>\u{26a0} <bold>{}</bold> \u{2014} {}</{color}>",
            badge.label, badge.message
        ));
        if let Some(url) = &badge.remediation_url {
            out.push('\n');
            out.push_str(&format!("  <dim>\u{2192} {url}</dim>"));
        }
    }
    Some(out)
}

pub(crate) fn format_verbose_summary_details_prose(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    details: &StructuredSummaryDetails,
) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(sid) = &summary.session_id {
        parts.push(format!("<i>session</i>: {sid}"));
    }

    if !details.tool_names.is_empty() {
        parts.push(format!(
            "<i>tools used</i>: {}",
            details.tool_names.join(", ")
        ));
    }

    if let Some(model) = &summary.model {
        parts.push(format!("<i>model</i>: {model}"));
    }

    if let Some(turns) = summary.num_turns {
        parts.push(format!("<i>turns</i>: {turns}",));
    }

    if let Some(stop_reason) = &summary.provider_status {
        parts.push(format!("<i>stop reason</i>: {stop_reason}"));
    }

    if summary.is_error
        && let Some(msg) = &summary.error_message
    {
        parts.push(format!("<red>{msg}</red>"));
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("<dim>{}</dim>", parts.join(" \u{00b7} ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::provider::Provider;
    use claudine::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
    use claudine::stream::summary::StreamExecutionSummary;

    #[test]
    fn format_summary_prose_appends_badge_markup() {
        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            duration_ms: Some(1000),
            badges: vec![SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing".into(),
                message: "Insufficient credits".into(),
                remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
            }],
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("Billing"));
        assert!(rendered.contains("Insufficient credits"));
        assert!(rendered.contains("https://console.anthropic.com/settings/billing"));
    }

    #[test]
    fn format_summary_prose_without_badges_has_no_badge_markup() {
        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            duration_ms: Some(1000),
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(!rendered.contains("Billing"));
        assert!(!rendered.contains("\u{26a0}"));
    }

    #[test]
    fn format_summary_prose_renders_permission_prompts_singular() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("1 <i>permission prompt</i>"));
        assert!(!rendered.contains("permission prompts"));
    }

    #[test]
    fn format_summary_prose_renders_permission_prompts_plural() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(3),
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("3 <i>permission prompts</i>"));
    }

    #[test]
    fn format_summary_prose_renders_user_input_prompts_singular() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("1 <i>user input prompt</i>"));
        assert!(!rendered.contains("user input prompts"));
    }

    #[test]
    fn format_summary_prose_renders_both_counters() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(41_000),
            tool_calls: Some(12),
            permission_prompts: Some(2),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("2 <i>permission prompts</i>"));
        assert!(rendered.contains("1 <i>user input prompt</i>"));
    }

    #[test]
    fn format_summary_prose_omits_permission_clauses_when_unset() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            ..Default::default()
        };
        let rendered = format_summary_prose(&summary).unwrap();
        assert!(!rendered.contains("permission"));
        assert!(!rendered.contains("user input"));
    }
}
