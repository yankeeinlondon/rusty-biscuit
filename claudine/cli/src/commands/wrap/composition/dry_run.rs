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
mod tests;
