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

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::CompositionExecutionRequest;
use claudine::provider::Provider;
use darkmatter::markdown::highlighting::highlight_yaml_lines;
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
    /// Resolved provider/agent, or `None` when selection is still
    /// interactive (no agent identified yet).
    pub agent: Option<Provider>,
    /// Resolved model, or `None` to use the provider default.
    pub model: Option<String>,
    /// Whether YOLO / auto-approval mode is active.
    pub yolo: bool,
    /// Focused monorepo area (`ctx.area`), present only inside a monorepo.
    pub area: Option<String>,
    /// Absolute path to the source document, used for the OSC8 link.
    pub document_path: PathBuf,
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
    /// - `area` ← `prep_launch_workspace.package_context.package_area`
    /// - `document_path` ← `prepared.resolved_path`
    pub(crate) fn from_request(request: &CompositionExecutionRequest) -> Self {
        let frontmatter = request.prepared.effective_frontmatter.clone();
        let name = string_field(&frontmatter, "name");
        let description = string_field(&frontmatter, "description");
        let agent = request.resolved_target.as_ref().map(|t| t.provider);
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
            area,
            document_path: request.prepared.resolved_path.clone(),
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

/// Render the finalized frontmatter as syntax-highlighted YAML.
///
/// The JSON `Value` is converted to YAML through `biscuit-file`'s
/// `serde_yaml_ng` re-export, then highlighted with darkmatter's YAML
/// highlighter. Output targets stderr.
pub(crate) fn render_frontmatter(frontmatter: &Value) -> String {
    let yaml = frontmatter_to_yaml(frontmatter);
    highlight_yaml_lines(yaml.trim_end()).join("\n")
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

/// Render the dry-run metadata table to a terminal string.
///
/// Rows, in order: Document (blue OSC8 link), Description (only when set),
/// Agent, Model, YOLO, Area (only inside a monorepo). Output targets stderr.
pub(crate) fn render_metadata_table(render: &DryRunRender, term: &Terminal) -> String {
    let mut table = base_table(vec![TableColumn::new("Field"), TableColumn::new("Value")]);

    // Document: frontmatter `name` if set, else the relative path; rendered
    // in blue as an OSC8 hyperlink to the document.
    let document_label = render
        .name
        .clone()
        .unwrap_or_else(|| relative_or_abs(&render.document_path));
    let href = format!("file://{}", render.document_path.display());
    let document_cell =
        Prose::new(format!("<blue><a href=\"{href}\">{document_label}</a></blue>")).render(term);
    table.add_row(vec!["Document".into(), document_cell.into()]);

    // Description: only when present, italic + dim.
    if let Some(description) = &render.description {
        let cell = Prose::new(format!("<i><dim>{description}</dim></i>")).render(term);
        table.add_row(vec!["Description".into(), cell.into()]);
    }

    // Agent: resolved provider, else an interactive placeholder.
    let agent_cell = match render.agent {
        Some(provider) => Prose::new(provider.to_string()).render(term),
        None => Prose::new("<i><yellow>interactive</yellow></i>").render(term),
    };
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

    // Area: only inside a monorepo (Decision 3).
    if let Some(area) = &render.area {
        table.add_row(vec!["Area".into(), area.clone().into()]);
    }

    table.render(term)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::prelude::strip_escape_codes;
    use serde_json::json;

    /// Build a minimal `DryRunRender` for table assertions.
    fn render_with(
        name: Option<&str>,
        description: Option<&str>,
        agent: Option<Provider>,
        model: Option<&str>,
        yolo: bool,
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
            area: area.map(str::to_string),
            document_path: PathBuf::from("/tmp/doc.md"),
        }
    }

    /// Plain (escape-stripped) render of the metadata table for semantic
    /// assertions that survive SGR-collapsing across capture surfaces.
    fn plain_table(render: &DryRunRender) -> String {
        let term = Terminal::default();
        strip_escape_codes(render_metadata_table(render, &term))
    }

    #[test]
    fn table_omits_description_when_absent() {
        let render = render_with(Some("doc"), None, None, None, false, None);
        let plain = plain_table(&render);
        assert!(!plain.contains("Description"));
    }

    #[test]
    fn table_shows_description_when_present() {
        let render = render_with(Some("doc"), Some("a helpful doc"), None, None, false, None);
        let plain = plain_table(&render);
        assert!(plain.contains("Description"));
        assert!(plain.contains("a helpful doc"));
    }

    #[test]
    fn table_omits_area_when_absent() {
        let render = render_with(Some("doc"), None, None, None, false, None);
        let plain = plain_table(&render);
        assert!(!plain.contains("Area"));
    }

    #[test]
    fn table_shows_area_when_present() {
        let render = render_with(Some("doc"), None, None, None, false, Some("claudine"));
        let plain = plain_table(&render);
        assert!(plain.contains("Area"));
        assert!(plain.contains("claudine"));
    }

    #[test]
    fn agent_falls_back_to_interactive() {
        let render = render_with(Some("doc"), None, None, None, false, None);
        let plain = plain_table(&render);
        assert!(plain.contains("interactive"));
    }

    #[test]
    fn agent_shows_resolved_provider() {
        let render = render_with(Some("doc"), None, Some(Provider::Claude), None, false, None);
        let plain = plain_table(&render);
        assert!(plain.contains(&Provider::Claude.to_string()));
        assert!(!plain.contains("interactive"));
    }

    #[test]
    fn model_falls_back_to_default() {
        let render = render_with(Some("doc"), None, None, None, false, None);
        let plain = plain_table(&render);
        assert!(plain.contains("default"));
    }

    #[test]
    fn model_shows_resolved_model() {
        let render = render_with(Some("doc"), None, None, Some("opus-4.8"), false, None);
        let plain = plain_table(&render);
        assert!(plain.contains("opus-4.8"));
        assert!(!plain.contains("default"));
    }

    #[test]
    fn yolo_true_and_false() {
        let yes = plain_table(&render_with(Some("doc"), None, None, None, true, None));
        assert!(yes.contains("true"));
        let no = plain_table(&render_with(Some("doc"), None, None, None, false, None));
        assert!(no.contains("false"));
    }

    #[test]
    fn document_uses_name_when_set() {
        let render = render_with(Some("My Document"), None, None, None, false, None);
        let plain = plain_table(&render);
        assert!(plain.contains("My Document"));
    }

    #[test]
    fn document_uses_path_when_name_absent() {
        let render = render_with(None, None, None, None, false, None);
        let plain = plain_table(&render);
        // No `name` ⇒ the document label is derived from the path.
        assert!(plain.contains("doc.md"));
    }

    #[test]
    fn frontmatter_renders_yaml_for_keys() {
        let fm = json!({ "name": "demo", "agent": "claude" });
        let plain = strip_escape_codes(render_frontmatter(&fm));
        assert!(plain.contains("name:"));
        assert!(plain.contains("demo"));
        assert!(plain.contains("agent:"));
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
}
