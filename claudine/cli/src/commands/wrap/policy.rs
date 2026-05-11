use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::log;

#[derive(Debug, Clone)]
pub(crate) struct WrapperHarnessPermissionProbe {
    provider: Provider,
    child_args: Vec<String>,
    repo_root: Option<PathBuf>,
}

impl WrapperHarnessPermissionProbe {
    pub(crate) fn new(
        provider: Provider,
        child_args: Vec<String>,
        repo_root: Option<&std::path::Path>,
    ) -> Self {
        Self {
            provider,
            child_args,
            repo_root: repo_root.map(std::path::Path::to_path_buf),
        }
    }

    fn sandbox_value(&self) -> Option<&str> {
        self.child_args
            .iter()
            .position(|arg| arg == "--sandbox")
            .and_then(|index| self.child_args.get(index + 1))
            .map(String::as_str)
    }

    fn workspace_root<'a>(
        &'a self,
        source_path: &'a std::path::Path,
    ) -> Option<&'a std::path::Path> {
        self.repo_root.as_deref().or_else(|| {
            source_path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
        })
    }
}

impl claudine::harness::HarnessPermissionProbe for WrapperHarnessPermissionProbe {
    fn can_write(
        &self,
        path: &std::path::Path,
        source_path: &std::path::Path,
    ) -> claudine::harness::PermissionAssessment {
        use claudine::harness::PermissionAssessment;

        if self.provider != Provider::Codex {
            return PermissionAssessment::Allowed;
        }

        if self
            .child_args
            .iter()
            .any(|arg| arg == "--dangerously-bypass-approvals-and-sandbox" || arg == "--yolo")
        {
            return PermissionAssessment::Allowed;
        }

        match self.sandbox_value() {
            Some("danger-full-access") => PermissionAssessment::Allowed,
            Some("read-only") => PermissionAssessment::Denied {
                reason: "Codex is running in read-only sandbox mode".to_string(),
            },
            Some("workspace-write") => {
                let Some(root) = self.workspace_root(source_path) else {
                    return PermissionAssessment::Unknown {
                        reason: "workspace-write mode is active, but no workspace root could be determined".to_string(),
                    };
                };
                if path.starts_with(root) {
                    PermissionAssessment::Allowed
                } else {
                    PermissionAssessment::Denied {
                        reason: format!(
                            "Codex workspace-write sandbox only allows writes under {}",
                            root.display()
                        ),
                    }
                }
            }
            Some(mode) => PermissionAssessment::Unknown {
                reason: format!("unrecognized Codex sandbox mode '{mode}'"),
            },
            None => PermissionAssessment::Allowed,
        }
    }
}

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
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StructuredSummaryDetails {
    pub(crate) tool_names: Vec<String>,
}

impl StructuredSummaryDetails {
    pub(crate) fn record_tool_name(&mut self, tool_name: &str) {
        if !tool_name.is_empty() && !self.tool_names.iter().any(|name| name == tool_name) {
            self.tool_names.push(tool_name.to_string());
        }
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
) -> (
    super::exec::SemanticParserBuilder,
    Option<claudine::stream::logs::StderrBridgeHandle>,
) {
    use claudine::stream::logs::StderrBridgeHandle;
    use claudine::stream::logs::codex::CodexLogBridge;
    use claudine::stream::logs::opencode::{OpenCodeLogBridge, merge_stderr_state_into_summary};
    use claudine::stream::semantic::{ObservedSemanticSink, SharedSemanticSink};
    use std::sync::atomic::AtomicBool;

    if provider == Provider::OpenCode {
        let shared = SharedSemanticSink::new(sink);
        let live_sink_inner = Arc::clone(shared.inner());
        let stdout_seen = Arc::new(AtomicBool::new(false));

        let (early_tx, early_rx) = std::sync::mpsc::channel();
        let bridge =
            OpenCodeLogBridge::new(shared.clone(), Arc::clone(&stdout_seen), Some(early_tx));
        let bridge_state = bridge.shared_state();
        let finalize: claudine::stream::logs::SummaryFinalizer = Box::new(move |summary| {
            merge_stderr_state_into_summary(&bridge_state, summary);
        });
        let stderr_bridge = Some(StderrBridgeHandle {
            bridge: Box::new(bridge),
            finalize,
            early_terminate: Some(early_rx),
        });

        let stdout_sink = ObservedSemanticSink::new(shared, stdout_seen);
        let build_parser: super::exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb| {
                if let Ok(mut inner) = live_sink_inner.lock() {
                    inner.set_output_text_sink(output_cb);
                }
                claudine::stream::create_semantic_parser(provider, stdout_sink, parser_config)
            });
        (build_parser, stderr_bridge)
    } else if provider == Provider::Codex {
        // Codex emits `tracing-subscriber` records on stderr that we'd
        // rather render inline through the live sink (as an orange
        // BlockQuote) than leak raw to the terminal. Share the sink so the
        // stdout parser and the stderr bridge feed one rendering pipeline.
        let shared = SharedSemanticSink::new(sink);
        let live_sink_inner = Arc::clone(shared.inner());
        let bridge = CodexLogBridge::new(shared.clone());
        let stderr_bridge = Some(StderrBridgeHandle {
            bridge: Box::new(bridge),
            finalize: Box::new(|_summary| {}),
            early_terminate: None,
        });

        let stdout_sink = shared;
        let build_parser: super::exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb| {
                if let Ok(mut inner) = live_sink_inner.lock() {
                    inner.set_output_text_sink(output_cb);
                }
                claudine::stream::create_semantic_parser(provider, stdout_sink, parser_config)
            });
        (build_parser, stderr_bridge)
    } else {
        let build_parser: super::exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb| {
                let sink = sink.with_output_text_sink(output_cb);
                claudine::stream::create_semantic_parser(provider, sink, parser_config)
            });
        (build_parser, None)
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
}

pub(crate) fn emit_stream_summary(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn super::profile::WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
    section_stream: Option<&super::section::SectionStream>,
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
        },
        None,
    );
}

pub(crate) fn emit_stream_summary_with_context(
    ctx: StreamSummaryContext<'_>,
    context_extra: &HashMap<String, serde_json::Value>,
) {
    emit_stream_summary_inner(ctx, Some(context_extra));
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
        use biscuit_terminal::components::renderable::Renderable;

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
        let meta = claudine::stream::reporting::summary_to_event_meta_with_context(
            summary,
            protocol,
            env_context,
            context_extra,
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
