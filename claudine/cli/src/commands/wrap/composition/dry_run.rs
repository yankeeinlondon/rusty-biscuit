//! Dry-run render core for the composition commands.
//!
//! Pure, unit-testable helpers that turn a prepared
//! [`CompositionExecutionRequest`] into the two artifacts a `--dry-run`
//! emits: the composed body (destined for stdout) and the finalized
//! frontmatter plus a metadata table (destined for stderr). Nothing here
//! performs I/O or launches a provider; the command paths (wired in later
//! phases) own the stdout/stderr split.
//!
//! These helpers are the foundation for the compose, inline-compose, and
//! sequence dry-run paths; the executor wires them in at the post-preflight
//! dry-run seam.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::horizontal_rule::{
    HorizontalRule, RuleAlignment, RuleStyle, RuleWeight,
};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue};
use claudine::composition::{
    AgentResolutionState, CompositionExecutionRequest, ProviderResolutionReason,
    SessionInteractivitySource, agent_state_breakdown, classify_agent_resolution,
};
use darkmatter::markdown::highlighting::{
    ColorMode as DmColorMode, ThemePair, detect_prose_theme, highlight_yaml_lines_with_theme,
};
use serde_json::Value;

use crate::table_utils::base_table;

/// Everything the dry-run renderer needs, extracted up front from a
/// [`CompositionExecutionRequest`] so rendering stays a pure function of
/// this struct.
#[derive(Debug, Clone)]
pub(crate) struct DryRunRender {
    /// The composed document body (what would be sent to the provider).
    pub body: String,
    /// Finalized frontmatter: merged + interpolated + shell-expanded.
    pub frontmatter: Value,
    /// Frontmatter `name`, when set and non-empty.
    pub name: Option<String>,
    /// Frontmatter `description`, when set and non-empty.
    pub description: Option<String>,
    /// Classified agent-resolution state used by both the dry-run table
    /// and the live execution gate.
    pub agent: AgentResolutionState,
    /// Resolved model, or `None` to use the provider default.
    pub model: Option<String>,
    /// Whether YOLO / auto-approval mode is active.
    pub yolo: bool,
    /// Whether the provider session is interactive.
    pub session_interactive: bool,
    /// Why the session is interactive or non-interactive.
    pub session_source: SessionInteractivitySource,
    /// Focused monorepo area (`ctx.area`), present only inside a monorepo.
    pub area: Option<String>,
    /// Absolute path to the source document, used for the OSC8 link.
    pub document_path: PathBuf,
    /// Lifecycle event keys Darkmatter deferred from compose-time resolution
    /// (DM1 metadata). Sorted. When non-empty, the metadata table labels them
    /// as interpolated at event-time so their raw `{{ }}` spans in the
    /// frontmatter block read as intentional, not as unresolved-variable bugs.
    pub deferred_lifecycle_keys: Vec<String>,
    /// Forwarded provider-argument tail, already run through the sensitive-arg
    /// redaction policy, so a `--dry-run` can audit the proposed launch without
    /// exposing secret values. Empty when no tail was forwarded.
    pub provider_args: Vec<String>,
}

impl DryRunRender {
    /// Extract the render inputs from a prepared execution request.
    ///
    /// Field origins:
    /// - `body` ← `prepared.prompt` (composed body)
    /// - `frontmatter` ← `prepared.effective_frontmatter`
    /// - `name` / `description` ← frontmatter string fields
    /// - `agent` / `model` ← `resolved_target` (None ⇒ interactive / default)
    /// - `yolo` ← `request.yolo`
    /// - `session_interactive` / `session_source` ← `request.session_interactive` /
    ///   `request.session_interactive_source`
    /// - `area` ← `prep_launch_workspace.package_context.package_area`
    /// - `document_path` ← `prepared.resolved_path`
    pub(crate) fn from_request(request: &CompositionExecutionRequest) -> Self {
        let frontmatter = request.prepared.effective_frontmatter.clone();
        let name = string_field(&frontmatter, "name");
        let description = string_field(&frontmatter, "description");
        // An explicit `--<provider>` flag has no frontmatter state to render,
        // so it shows as the selected provider. For every other resolved
        // target (a frontmatter-driven `Selected` or `ListOneInstalled`
        // auto-select), reclassify from the snapshot so the cell carries the
        // full breakdown — otherwise an auto-selected one-installed list would
        // collapse to a bare provider name and lose its auto-select header.
        let explicit_flag = request.resolved_target.as_ref().is_some_and(|t| {
            matches!(t.provider_reason, ProviderResolutionReason::ExplicitFlag)
        });
        let agent = if explicit_flag {
            AgentResolutionState::Selected {
                provider: request.resolved_target.as_ref().unwrap().provider,
            }
        } else if let Some(snapshot) = &request.installed_snapshot {
            classify_agent_resolution(&request.prepared.selection_hints, snapshot)
        } else if let Some(target) = &request.resolved_target {
            AgentResolutionState::Selected {
                provider: target.provider,
            }
        } else {
            AgentResolutionState::NoAgent
        };
        let model = request
            .resolved_target
            .as_ref()
            .and_then(|t| t.model.clone());
        let area = request
            .prep_launch_workspace
            .as_ref()
            .and_then(|w| w.package_context.as_ref())
            .map(|p| p.package_area.clone())
            .filter(|a| !a.is_empty());

        Self {
            body: request.prepared.prompt.clone(),
            frontmatter,
            name,
            description,
            agent,
            model,
            yolo: request.yolo,
            session_interactive: request.session_interactive,
            session_source: request.session_interactive_source,
            area,
            document_path: request.prepared.resolved_path.clone(),
            deferred_lifecycle_keys: request.prepared.deferred_lifecycle_keys.clone(),
            provider_args: crate::commands::wrap::env::redact_sensitive_args(&request.provider_args),
        }
    }
}

/// Read a non-empty string field from a JSON object frontmatter.
fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// Render a full-width horizontal rule.
pub(crate) fn render_hr(term: &Terminal) -> String {
    HorizontalRule::new()
        .style(RuleStyle::Dashes)
        .alignment(RuleAlignment::Full)
        .weight(RuleWeight::Medium)
        .render(term)
}

/// Render the `Frontmatter (resolved):` heading.
pub(crate) fn render_frontmatter_heading(term: &Terminal) -> String {
    Prose::new("<b>Frontmatter</b> (<i>resolved</i>):").render(term)
}

/// Render the finalized frontmatter as syntax-highlighted YAML.
///
/// The JSON `Value` is converted to YAML through `biscuit-file`'s
/// `serde_yaml_ng` re-export, then highlighted with darkmatter's YAML
/// highlighter using the **inverse** theme so the code block stands out
/// from the terminal background. Each line receives a `1ch` left margin.
/// Output targets stderr.
pub(crate) fn render_frontmatter(frontmatter: &Value, term: &Terminal) -> String {
    let yaml = frontmatter_to_yaml(frontmatter);
    let theme_pair = detect_prose_theme();
    // Invert the terminal's detected color mode so code blocks contrast
    // against the page (dark terminal → light code theme, and vice versa).
    let color_mode = match term.color_mode() {
        biscuit_terminal::discovery::detection::ColorMode::Light => DmColorMode::Dark,
        _ => DmColorMode::Light,
    };
    let highlighted =
        highlight_yaml_lines_with_theme(yaml.trim_end(), theme_pair, color_mode);
    // 1ch left margin on every line.
    highlighted
        .into_iter()
        .map(|line| format!(" {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a frontmatter JSON `Value` to a YAML string.
///
/// Falls back to an empty string if serialization fails (a non-object
/// `Value` still serializes; failure is effectively unreachable for
/// frontmatter shapes).
fn frontmatter_to_yaml(value: &Value) -> String {
    biscuit_file::serde_yaml_ng::to_string(value).unwrap_or_default()
}

/// Produce a `cwd`-relative display string when possible, falling back to
/// the absolute path.
fn relative_or_abs(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&cwd)
    {
        return PathBuf::from(rel).display().to_string();
    }
    path.display().to_string()
}

/// Render the classified `agent` resolution state into a multi-line cell.
///
/// All content stays inside the single `Agent` table row; the table
/// component is responsible for preserving embedded newlines and bullet
/// indentation. The markup is shared with the live no-TTY abort body via
/// [`agent_state_breakdown`] so the dry-run table stays a faithful
/// prediction of the live path.
fn render_agent_cell(state: &AgentResolutionState, term: &Terminal) -> String {
    Prose::new(agent_state_breakdown(state)).render(term)
}

/// Render the dry-run metadata table to a terminal string.
///
/// Rows, in order: Document (blue OSC8 link), Description (only when set),
/// Agent, Model, YOLO, Session (interactive/non-interactive with source),
/// Area (only inside a monorepo). Output targets stderr.
/// The table has a `1ch` top margin to separate it from the YAML block above.
pub(crate) fn render_metadata_table(render: &DryRunRender, term: &Terminal) -> String {
    let mut table = base_table(vec![TableColumn::new("Field"), TableColumn::new("Value")]);
    table.layout_mut().margin.top = TargetValue::universal(Length::ch(1));

    // Document: frontmatter `name` if set, else the file name; rendered
    // in blue as an OSC8 hyperlink to the document.
    let document_label = render.name.clone().unwrap_or_else(|| {
        render
            .document_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_string())
            .unwrap_or_else(|| relative_or_abs(&render.document_path))
    });
    let href = format!("file://{}", render.document_path.display());
    let document_cell =
        Prose::new(format!("<blue><a href=\"{href}\">{document_label}</a></blue>")).render(term);
    table.add_row(vec!["Document".into(), document_cell.into()]);

    // Description: only when present, italic + dim.
    if let Some(description) = &render.description {
        let cell = Prose::new(format!("<i><dim>{description}</dim></i>")).render(term);
        table.add_row(vec!["Description".into(), cell.into()]);
    }

    // Agent: classified resolution state rendered as a multi-line cell.
    let agent_cell = render_agent_cell(&render.agent, term);
    table.add_row(vec!["Agent".into(), agent_cell.into()]);

    // Model: resolved model, else a default placeholder.
    let model_cell = match &render.model {
        Some(model) => Prose::new(model.clone()).render(term),
        None => Prose::new("<i><dim>default</dim></i>").render(term),
    };
    table.add_row(vec!["Model".into(), model_cell.into()]);

    // YOLO: green true / red false.
    let yolo_cell = if render.yolo {
        Prose::new("<green>true</green>").render(term)
    } else {
        Prose::new("<red>false</red>").render(term)
    };
    table.add_row(vec!["YOLO".into(), yolo_cell.into()]);

    // Session: interactive / non-interactive with resolved source.
    let session_label = if render.session_interactive {
        "interactive"
    } else {
        "non-interactive"
    };
    let session_cell = Prose::new(format!(
        "{session_label} ({source})",
        source = render.session_source
    ))
    .render(term);
    table.add_row(vec!["Session".into(), session_cell.into()]);

    // Area: only inside a monorepo (Decision 3).
    if let Some(area) = &render.area {
        table.add_row(vec!["Area".into(), area.clone().into()]);
    }

    // Provider args: the forwarded (redacted) agent tail, shown so a dry-run
    // can audit exactly what would reach the child. Only when non-empty.
    if !render.provider_args.is_empty() {
        let joined = render.provider_args.join(" ");
        let cell = Prose::new(format!("<dim>{joined}</dim>")).render(term);
        table.add_row(vec!["Provider args".into(), cell.into()]);
    }

    // Deferred: lifecycle event keys left raw in the frontmatter block above
    // because they interpolate at event-time, not during compose (C5). Only
    // shown when at least one such key is present, so a reader sees that a raw
    // `{{err.msg}}` span there is intentional.
    if !render.deferred_lifecycle_keys.is_empty() {
        let keys = render.deferred_lifecycle_keys.join(", ");
        let cell = Prose::new(format!(
            "<i><dim>interpolated at event-time:</dim></i> {keys}"
        ))
        .render(term);
        table.add_row(vec!["Deferred".into(), cell.into()]);
    }

    table.render(term)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::prelude::strip_escape_codes;
    use claudine::provider::Provider;
    use serde_json::json;

    /// Build a minimal `DryRunRender` for table assertions.
    fn render_with(
        name: Option<&str>,
        description: Option<&str>,
        agent: AgentResolutionState,
        model: Option<&str>,
        yolo: bool,
        area: Option<&str>,
    ) -> DryRunRender {
        render_with_session(
            name,
            description,
            agent,
            model,
            yolo,
            false,
            SessionInteractivitySource::Default,
            area,
        )
    }

    /// Build a `DryRunRender` with explicit session interactivity state.
    #[allow(clippy::too_many_arguments)]
    fn render_with_session(
        name: Option<&str>,
        description: Option<&str>,
        agent: AgentResolutionState,
        model: Option<&str>,
        yolo: bool,
        session_interactive: bool,
        session_source: SessionInteractivitySource,
        area: Option<&str>,
    ) -> DryRunRender {
        DryRunRender {
            body: "body".to_string(),
            frontmatter: json!({}),
            name: name.map(str::to_string),
            description: description.map(str::to_string),
            agent,
            model: model.map(str::to_string),
            yolo,
            session_interactive,
            session_source,
            area: area.map(str::to_string),
            document_path: PathBuf::from("/tmp/doc.md"),
            deferred_lifecycle_keys: Vec::new(),
            provider_args: Vec::new(),
        }
    }

    /// Plain (escape-stripped) render of the metadata table for semantic
    /// assertions that survive SGR-collapsing across capture surfaces.
    fn plain_table(render: &DryRunRender) -> String {
        let term = Terminal::default();
        strip_escape_codes(render_metadata_table(render, &term))
    }

    /// Plain (escape-stripped) render of just the agent cell, without table
    /// borders or word-wrap, so multi-line assertions are stable.
    fn plain_agent_cell(state: &AgentResolutionState) -> String {
        let term = Terminal::default();
        strip_escape_codes(render_agent_cell(state, &term))
    }

    /// Extract the trimmed value cell for a single-row label from the plain
    /// metadata table. Returns `None` if the label is not found.
    fn plain_row_value(plain: &str, label: &str) -> Option<String> {
        for line in plain.lines() {
            let cells: Vec<&str> = line.split('│').collect();
            if cells.len() >= 3 && cells[1].trim() == label {
                return Some(cells[2].trim().to_string());
            }
        }
        None
    }

    #[test]
    fn table_omits_description_when_absent() {
        let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
        let plain = plain_table(&render);
        assert!(!plain.contains("Description"));
    }

    #[test]
    fn table_shows_description_when_present() {
        let render = render_with(
            Some("doc"),
            Some("a helpful doc"),
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        assert!(plain.contains("Description"));
        assert!(plain.contains("a helpful doc"));
    }

    #[test]
    fn table_omits_area_when_absent() {
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        assert!(!plain.contains("Area"));
    }

    #[test]
    fn table_shows_area_when_present() {
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            Some("claudine"),
        );
        let plain = plain_table(&render);
        assert!(plain.contains("Area"));
        assert!(plain.contains("claudine"));
    }

    #[test]
    fn table_omits_provider_args_when_empty() {
        let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
        let plain = plain_table(&render);
        assert!(!plain.contains("Provider args"));
    }

    #[test]
    fn table_shows_redacted_provider_args_when_present() {
        let mut render =
            render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
        render.provider_args = vec!["-c".to_string(), "model_reasoning_effort=low".to_string()];
        let plain = plain_table(&render);
        let value = plain_row_value(&plain, "Provider args").expect("Provider args row should exist");
        assert!(value.contains("-c"));
        assert!(value.contains("model_reasoning_effort=low"));
    }

    #[test]
    fn table_omits_deferred_row_when_no_keys() {
        let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
        let plain = plain_table(&render);
        assert!(!plain.contains("Deferred"));
    }

    #[test]
    fn table_shows_deferred_keys_labeled_event_time() {
        let mut render =
            render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
        render.deferred_lifecycle_keys = vec!["failure".to_string(), "finalize".to_string()];
        let plain = plain_table(&render);
        let value = plain_row_value(&plain, "Deferred").expect("Deferred row should exist");
        assert!(value.contains("event-time"), "row should label event-time: {value}");
        assert!(value.contains("failure"));
        assert!(value.contains("finalize"));
    }

    #[test]
    fn agent_falls_back_to_interactive() {
        let render = render_with(Some("doc"), None, AgentResolutionState::NoAgent, None, false, None);
        let plain = plain_table(&render);
        assert!(plain.contains("interactive"));
    }

    #[test]
    fn agent_shows_resolved_provider() {
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::Selected {
                provider: Provider::Claude,
            },
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        let agent_value = plain_row_value(&plain, "Agent").expect("Agent row should exist");
        assert!(agent_value.contains(&Provider::Claude.to_string()));
        assert!(!agent_value.contains("interactive"));
    }

    #[test]
    fn model_falls_back_to_default() {
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        let model_value = plain_row_value(&plain, "Model").expect("Model row should exist");
        assert!(model_value.contains("default"));
    }

    #[test]
    fn model_shows_resolved_model() {
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            Some("opus-4.8"),
            false,
            None,
        );
        let plain = plain_table(&render);
        let model_value = plain_row_value(&plain, "Model").expect("Model row should exist");
        assert!(model_value.contains("opus-4.8"));
        assert!(!model_value.contains("default"));
    }

    #[test]
    fn yolo_true_and_false() {
        let yes = plain_table(&render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            true,
            None,
        ));
        assert!(yes.contains("true"));
        let no = plain_table(&render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        ));
        assert!(no.contains("false"));
    }

    #[test]
    fn session_row_renders_for_each_source() {
        for (interactive, source, expected) in [
            (true, SessionInteractivitySource::InteractiveFlag, "interactive (--interactive)"),
            (true, SessionInteractivitySource::Frontmatter, "interactive (frontmatter)"),
            (false, SessionInteractivitySource::NoInteractiveFlag, "non-interactive (--no-interactive)"),
            (false, SessionInteractivitySource::Default, "non-interactive (default)"),
        ] {
            let render = render_with_session(
                Some("doc"),
                None,
                AgentResolutionState::NoAgent,
                None,
                false,
                interactive,
                source,
                None,
            );
            let plain = plain_table(&render);
            assert!(
                plain.contains("Session"),
                "Session row should appear for source {source}"
            );
            assert!(
                plain.contains(expected),
                "expected '{expected}' for source {source}; got:\n{plain}"
            );
        }
    }

    #[test]
    fn session_row_no_interactive_overrides_frontmatter_true() {
        let render = render_with_session(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            false,
            SessionInteractivitySource::NoInteractiveFlag,
            None,
        );
        let plain = plain_table(&render);
        assert!(plain.contains("Session"));
        assert!(plain.contains("non-interactive (--no-interactive)"));
    }

    #[test]
    fn document_uses_name_when_set() {
        let render = render_with(
            Some("My Document"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        assert!(plain.contains("My Document"));
    }

    #[test]
    fn document_uses_path_when_name_absent() {
        let render = render_with(
            None,
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        // No `name` ⇒ the document label is derived from the path.
        assert!(plain.contains("doc.md"));
    }

    #[test]
    fn frontmatter_renders_yaml_for_keys() {
        let fm = json!({ "name": "demo", "agent": "claude" });
        let term = Terminal::default();
        let plain = strip_escape_codes(render_frontmatter(&fm, &term));
        assert!(plain.contains("name:"));
        assert!(plain.contains("demo"));
        assert!(plain.contains("agent:"));
    }

    #[test]
    fn frontmatter_has_one_ch_left_margin() {
        let fm = json!({ "key": "value" });
        let term = Terminal::default();
        let plain = strip_escape_codes(render_frontmatter(&fm, &term));
        // Every non-empty line should start with a single space (the 1ch margin).
        for line in plain.lines() {
            if !line.trim().is_empty() {
                assert!(
                    line.starts_with(' '),
                    "line should have 1ch left margin: {line:?}"
                );
            }
        }
    }

    #[test]
    fn frontmatter_heading_contains_bold_and_italic() {
        let term = Terminal::default();
        let rendered = render_frontmatter_heading(&term);
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("Frontmatter"), "heading should contain 'Frontmatter': {plain}");
        assert!(plain.contains("resolved"), "heading should contain 'resolved': {plain}");
    }

    #[test]
    fn hr_renders_non_empty() {
        let term = Terminal::default();
        let rendered = render_hr(&term);
        let plain = strip_escape_codes(&rendered);
        assert!(!plain.trim().is_empty(), "horizontal rule should not be empty");
    }

    #[test]
    fn frontmatter_yaml_round_trips_a_value() {
        let original = json!({
            "name": "demo",
            "tags": ["a", "b"],
            "nested": { "k": "v" },
        });
        let yaml = frontmatter_to_yaml(&original);
        let parsed: Value =
            biscuit_file::serde_yaml_ng::from_str(&yaml).expect("yaml round-trip parses");
        assert_eq!(parsed, original);
    }

    // -- Agent-resolution state rendering (Phase 4) --------------------------

    #[test]
    fn agent_no_agent_renders_unordered_list() {
        let plain = plain_agent_cell(&AgentResolutionState::NoAgent);
        assert!(plain.contains("didn't specify the Agent"));
        assert!(plain.contains("didn't suggest any Agents"));
        assert!(plain.contains("interactive"));
    }

    #[test]
    fn agent_selected_renders_provider_name() {
        let plain = plain_agent_cell(&AgentResolutionState::Selected {
            provider: Provider::Codex,
        });
        assert!(plain.contains(&Provider::Codex.to_string()));
        assert!(!plain.contains("Invalid Agent"));
        assert!(!plain.contains("interactive"));
    }

    #[test]
    fn agent_single_invalid_renders_error_and_explanation() {
        let plain = plain_agent_cell(&AgentResolutionState::SingleInvalid {
            hint: "unknown-provider".into(),
        });
        assert!(plain.contains("Invalid Agent"));
        assert!(plain.contains("unknown-provider"));
        assert!(plain.contains("Frontmatter"));
    }

    #[test]
    fn agent_single_not_installed_renders_warning_and_explanation() {
        let plain = plain_agent_cell(&AgentResolutionState::SingleNotInstalled {
            provider: Provider::Gemini,
        });
        assert!(plain.contains("Agent Not Installed"));
        assert!(plain.contains(&Provider::Gemini.to_string()));
        assert!(plain.contains("not installed on this host"));
    }

    #[test]
    fn agent_list_multiple_installed_renders_choice_header_and_list() {
        let plain = plain_agent_cell(&AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude, Provider::Codex],
            not_installed: vec![Provider::Gemini],
            invalid: vec![],
        });
        assert!(plain.contains("choose interactively between suggested Agents"));
        assert!(plain.contains(&Provider::Claude.to_string()));
        assert!(plain.contains(&Provider::Codex.to_string()));
        assert!(plain.contains(&Provider::Gemini.to_string()));
    }

    #[test]
    fn agent_list_multiple_installed_dimms_not_installed() {
        // Semantic test: the plain text contains the not-installed provider;
        // L2 tests assert the actual dim SGR.
        let plain = plain_agent_cell(&AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude],
            not_installed: vec![Provider::Gemini],
            invalid: vec![],
        });
        assert!(plain.contains(&Provider::Gemini.to_string()));
    }

    #[test]
    fn agent_list_multiple_installed_shows_invalid_suggestions() {
        let plain = plain_agent_cell(&AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude],
            not_installed: vec![],
            invalid: vec!["bad-agent".into()],
        });
        assert!(plain.contains("NOT"));
        assert!(plain.contains("valid Agents"));
        assert!(plain.contains("bad-agent"));
    }

    #[test]
    fn agent_list_one_installed_renders_auto_select_header() {
        let plain = plain_agent_cell(&AgentResolutionState::ListOneInstalled {
            selected: Provider::Claude,
            not_installed: vec![Provider::Gemini],
            invalid: vec![],
        });
        assert!(plain.contains(&Provider::Claude.to_string()));
        assert!(plain.contains("without the need for interactive prompting"));
        assert!(plain.contains("only"));
        assert!(plain.contains("is installed on this host"));
    }

    #[test]
    fn agent_list_one_installed_with_invalid_shows_invalid_list() {
        let plain = plain_agent_cell(&AgentResolutionState::ListOneInstalled {
            selected: Provider::Claude,
            not_installed: vec![],
            invalid: vec!["bad-agent".into(), "worse-agent".into()],
        });
        assert!(plain.contains("NOT"));
        assert!(plain.contains("valid Agents"));
        assert!(plain.contains("bad-agent"));
        assert!(plain.contains("worse-agent"));
    }

    #[test]
    fn agent_zero_installed_list_renders_header_and_dimmed_suggestions() {
        let plain = plain_agent_cell(&AgentResolutionState::ZeroInstalledList {
            not_installed: vec![Provider::Gemini, Provider::Goose],
            invalid: vec![],
        });
        assert!(plain.contains("None of the suggested agents are installed/valid"));
        assert!(plain.contains(&Provider::Gemini.to_string()));
        assert!(plain.contains(&Provider::Goose.to_string()));
    }

    #[test]
    fn agent_zero_installed_list_with_invalid_shows_invalid_list() {
        let plain = plain_agent_cell(&AgentResolutionState::ZeroInstalledList {
            not_installed: vec![Provider::Gemini],
            invalid: vec!["bad-agent".into()],
        });
        assert!(plain.contains("None of the suggested agents are installed/valid"));
        assert!(plain.contains(&Provider::Gemini.to_string()));
        assert!(plain.contains("NOT"));
        assert!(plain.contains("valid Agents"));
        assert!(plain.contains("bad-agent"));
    }

    #[test]
    fn agent_cell_preserves_single_table_row() {
        // Every agent state must keep its breakdown inside the single Agent
        // value cell; no extra metadata rows are added.
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::ListMultipleInstalled {
                installed: vec![Provider::Claude, Provider::Codex],
                not_installed: vec![Provider::Gemini],
                invalid: vec!["bad".into()],
            },
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        // Count lines that contain the Agent row label after stripping table
        // border characters. There should be exactly one.
        let agent_label_lines: Vec<&str> = plain
            .lines()
            .filter(|l| {
                let cleaned: String = l.chars().filter(|&c| c != '│').collect();
                cleaned.trim().starts_with("Agent")
            })
            .collect();
        assert_eq!(
            agent_label_lines.len(),
            1,
            "Agent label should appear exactly once (single row); plain:\n{plain}"
        );
    }

    #[test]
    fn agent_cell_multiline_preserves_table_alignment() {
        // A multi-line Agent cell must keep the table's two-column alignment:
        // every `│` separator appears at the same positions across all rows.
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        let lines: Vec<&str> = plain.lines().collect();

        // Find the table region by its border characters.
        let table_start = lines
            .iter()
            .position(|l| l.contains('┌'))
            .expect("table top border should exist");
        let table_end = lines
            .iter()
            .skip(table_start)
            .position(|l| l.contains('└'))
            .map(|i| table_start + i)
            .expect("table bottom border should exist");

        let mut expected_positions: Option<Vec<usize>> = None;
        for line in &lines[table_start..=table_end] {
            let positions: Vec<usize> = line
                .char_indices()
                .filter(|(_, c)| *c == '│')
                .map(|(i, _)| i)
                .collect();
            if positions.len() >= 2 {
                if let Some(ref expected) = expected_positions {
                    assert_eq!(
                        positions, *expected,
                        "table column separators misaligned.\nline: {line:?}\nplain:\n{plain}"
                    );
                } else {
                    expected_positions = Some(positions);
                }
            }
        }
    }

    #[test]
    fn table_preserves_one_ch_left_margin() {
        // The table is rendered with `Edges::x(Length::ch(1))`, so every
        // table line must start with a single space (the 1ch left offset).
        let render = render_with(
            Some("doc"),
            None,
            AgentResolutionState::NoAgent,
            None,
            false,
            None,
        );
        let plain = plain_table(&render);
        let lines: Vec<&str> = plain.lines().collect();

        let table_start = lines
            .iter()
            .position(|l| l.contains('┌'))
            .expect("table top border should exist");
        let table_end = lines
            .iter()
            .skip(table_start)
            .position(|l| l.contains('└'))
            .map(|i| table_start + i)
            .expect("table bottom border should exist");

        for line in &lines[table_start..=table_end] {
            assert!(
                line.starts_with(' '),
                "every table line must have a 1ch left margin (start with space).\n\
                 line: {line:?}\nplain:\n{plain}"
            );
        }
    }
}
