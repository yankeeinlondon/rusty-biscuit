//! Dual-target render for the mesh NOW view.
//!
//! [`DashboardReport`] mirrors `claudine::render::MetricsReport`: a
//! `TerminalRenderable` for the CLI and a `BrowserRenderable` so the
//! same figures compose into an HTML artifact almost for free
//! (dashboard spec D1/D6).

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{
    BrowserRenderable, RenderableTerminalContent, TerminalRenderable,
};
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Edges, Layout, Length};
use renderable::browser::fragment::{BrowserFragment, ComposableNode, Ready};
use renderable::html::tag::BlockTag;

use super::model::{HostView, Intervention, MeshSnapshot, SessionRow, Staleness};

/// A rendered mesh NOW view.
#[derive(Debug)]
pub struct DashboardReport {
    snapshot: MeshSnapshot,
    /// `mesh` or `local` — echoed in the heading so a `--local` render
    /// is self-describing.
    scope_label: String,
    /// Terminal used for inline colored spans, kept separate from the
    /// display target so badges stay colored even when piped (matching
    /// `MetricsReport`).
    inline: Terminal,
    layout: Layout,
}

impl DashboardReport {
    pub fn new(snapshot: MeshSnapshot, scope_label: impl Into<String>) -> Self {
        Self {
            snapshot,
            scope_label: scope_label.into(),
            inline: Terminal::new_optimistic(80),
            layout: Layout::default(),
        }
    }

    pub fn with_inline_terminal(mut self, inline: Terminal) -> Self {
        self.inline = inline;
        self
    }

    fn heading_text(&self) -> String {
        format!(
            "Rendezvous Dashboard ▸ {} · {} · {} · {}",
            self.scope_label,
            plural(self.snapshot.hosts.len(), "host", "hosts"),
            plural(self.snapshot.total_sessions(), "session", "sessions"),
            needs_input_phrase(self.snapshot.needs_input_count()),
        )
    }
}

impl TerminalRenderable for DashboardReport {
    fn render(&self, term: &Terminal) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(String::new());
        lines.push(
            Prose::new(format!("<blue><bold>{}</bold></blue>", self.heading_text())).render(term),
        );

        if self.snapshot.hosts.is_empty() {
            lines.push(String::new());
            lines.push(Prose::new("<dim>No hosts in the mesh yet.</dim>").render(term));
            return lines.join("\n");
        }

        for host in &self.snapshot.hosts {
            lines.push(String::new());
            lines.push(Prose::new(host_heading_markup(host)).render(term));

            if host.sessions.is_empty() {
                let hint = if host.sessions_trusted() {
                    "<dim>  no active sessions</dim>"
                } else {
                    "<dim>  sessions unknown (host is stale)</dim>"
                };
                lines.push(Prose::new(hint).render(term));
                continue;
            }

            let trusted = host.sessions_trusted();
            let mut table = base_table(vec![
                TableColumn::new("Session"),
                TableColumn::new("Agent"),
                TableColumn::new("Model"),
                TableColumn::new("Repo"),
                TableColumn::new("Status"),
                TableColumn::new("Needs?"),
            ]);
            for session in &host.sessions {
                table.add_row(vec![
                    short_id(&session.session_id).into(),
                    dash(session.agent.as_deref()).into(),
                    dash(session.model.as_deref()).into(),
                    dash(session.repo_root.as_deref().map(basename)).into(),
                    status_cell(session, trusted).into(),
                    needs_cell(session.intervention, &self.inline).into(),
                ]);
            }
            lines.push(table.render(term));
        }

        if self.snapshot.needs_input_count() > 0 {
            lines.push(String::new());
            let list = UnorderedList::from(vec![RenderableTerminalContent::from(Prose::new(
                "<yellow>⚠</yellow> sessions marked <bold>Needs?</bold> are waiting on a human.",
            ))])
            .with_bullet("  ");
            lines.push(list.render(term));
        }

        lines.join("\n")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

impl BrowserRenderable for DashboardReport {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let base = "claudine-dashboard-report";
        let mut section = BrowserFragment::new().define_as_block_tag(BlockTag::Section, base);

        section = section.add_component(block_text(
            BlockTag::H2,
            format!("{base}__title"),
            self.heading_text(),
        ));

        for host in &self.snapshot.hosts {
            section = section.add_component(block_text(
                BlockTag::H3,
                format!("{base}__host"),
                host_heading_plain(host),
            ));

            if host.sessions.is_empty() {
                let hint = if host.sessions_trusted() {
                    "no active sessions"
                } else {
                    "sessions unknown (host is stale)"
                };
                section = section.add_component(block_text(
                    BlockTag::P,
                    format!("{base}__host-empty"),
                    hint.to_string(),
                ));
                continue;
            }

            let trusted = host.sessions_trusted();
            let mut table = Table::new().with_columns(vec![
                TableColumn::new("Session"),
                TableColumn::new("Agent"),
                TableColumn::new("Model"),
                TableColumn::new("Repo"),
                TableColumn::new("Status"),
                TableColumn::new("Needs?"),
            ]);
            for session in &host.sessions {
                table.add_row(vec![
                    short_id(&session.session_id).into(),
                    dash(session.agent.as_deref()).into(),
                    dash(session.model.as_deref()).into(),
                    dash(session.repo_root.as_deref().map(basename)).into(),
                    status_cell(session, trusted).into(),
                    needs_plain(session.intervention).to_string().into(),
                ]);
            }
            section = section.add_component(BrowserRenderable::render_html_fragment(&table));
        }

        section.finalize()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Table with the x-margin `MetricsReport::base_table` uses so the
/// dashboard's tables share the report look.
fn base_table(columns: Vec<TableColumn>) -> Table {
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().margin = Edges::x(Length::ch(1));
    table
}

/// Build a block-level tag carrying a single escaped text child.
fn block_text(tag: BlockTag, class: String, text: String) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_block_tag(tag, class)
        .add_child(ComposableNode::TextFragment(text))
        .finalize()
}

fn host_heading_markup(host: &HostView) -> String {
    let label = host.hostname.as_deref().unwrap_or_else(|| short_node(host));
    let scope = if host.is_local {
        "<green>local</green>".to_string()
    } else {
        format!("remote · {}", staleness_markup(&host.staleness))
    };
    let mut parts = vec![format!("<bold>{label}</bold> <dim>({scope})</dim>")];
    if let Some(spec) = capability_spec(host) {
        parts.push(format!("<dim>{spec}</dim>"));
    }
    parts.join("  ·  ")
}

fn host_heading_plain(host: &HostView) -> String {
    let label = host.hostname.as_deref().unwrap_or_else(|| short_node(host));
    let scope = if host.is_local {
        "local".to_string()
    } else {
        format!("remote · {}", staleness_plain(&host.staleness))
    };
    let mut heading = format!("{label} ({scope})");
    if let Some(spec) = capability_spec(host) {
        heading.push_str(" · ");
        heading.push_str(&spec);
    }
    heading
}

fn staleness_markup(staleness: &Staleness) -> String {
    match staleness {
        Staleness::Local => "<green>fresh</green>".to_string(),
        Staleness::Fresh { age_ms } => format!("<green>synced {} ago</green>", format_age(*age_ms)),
        Staleness::Stale { age_ms } => format!(
            "<red>stale · last sync {} ago — sessions unknown</red>",
            format_age(*age_ms)
        ),
        Staleness::NeverSynced => "<red>never synced — sessions unknown</red>".to_string(),
    }
}

fn staleness_plain(staleness: &Staleness) -> String {
    match staleness {
        Staleness::Local => "fresh".to_string(),
        Staleness::Fresh { age_ms } => format!("synced {} ago", format_age(*age_ms)),
        Staleness::Stale { age_ms } => {
            format!("stale · last sync {} ago — sessions unknown", format_age(*age_ms))
        }
        Staleness::NeverSynced => "never synced — sessions unknown".to_string(),
    }
}

fn capability_spec(host: &HostView) -> Option<String> {
    let cap = host.capability.as_ref()?;
    let mut parts: Vec<String> = Vec::new();
    match (cap.os.as_deref(), cap.os_version.as_deref()) {
        (Some(os), Some(ver)) => parts.push(format!("{os} {ver}")),
        (Some(os), None) => parts.push(os.to_string()),
        _ => {}
    }
    if let Some(arch) = cap.arch.as_deref() {
        parts.push(arch.to_string());
    }
    if let Some(cores) = cap.cpu_cores {
        parts.push(format!("{cores} cores"));
    }
    if let Some(bytes) = cap.memory_bytes {
        parts.push(format_memory(bytes));
    }
    if let Some(gpu) = cap.gpu.as_deref().filter(|g| *g != "none") {
        parts.push(gpu.to_string());
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn status_cell(session: &SessionRow, trusted: bool) -> String {
    if !trusted {
        return "unknown".to_string();
    }
    session.status.clone().unwrap_or_else(|| "—".to_string())
}

fn needs_cell(intervention: Intervention, inline: &Terminal) -> String {
    match intervention {
        Intervention::NeedsInput => {
            Prose::new("<yellow><bold>⚠ input</bold></yellow>").render(inline)
        }
        Intervention::None => "—".to_string(),
    }
}

fn needs_plain(intervention: Intervention) -> &'static str {
    match intervention {
        Intervention::NeedsInput => "needs input",
        Intervention::None => "—",
    }
}

fn short_node(host: &HostView) -> &str {
    short_id(&host.node_id)
}

/// First 8 chars of a hex id — enough to disambiguate on one host.
fn short_id(id: &str) -> &str {
    let end = id.char_indices().nth(8).map_or(id.len(), |(i, _)| i);
    &id[..end]
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn dash(value: Option<&str>) -> String {
    value.filter(|s| !s.is_empty()).unwrap_or("—").to_string()
}

/// Coarse human age: `3s`, `5m`, `2h`, `4d`.
fn format_age(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

fn format_memory(bytes: i64) -> String {
    let gb = bytes as f64 / 1_073_741_824.0;
    if gb >= 1.0 {
        format!("{gb:.0} GB")
    } else {
        let mb = bytes as f64 / 1_048_576.0;
        format!("{mb:.0} MB")
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn needs_input_phrase(n: usize) -> String {
    match n {
        0 => "none need input".to_string(),
        1 => "1 needs input".to_string(),
        _ => format!("{n} need input"),
    }
}
