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
        // The headline counts only trusted-live sessions; stale
        // last-known observations are reported separately so they are
        // never presented as live (dashboard spec D4).
        let mut heading = format!(
            "Rendezvous Dashboard ▸ {} · {} · {} · {}",
            self.scope_label,
            plural(self.snapshot.hosts.len(), "host", "hosts"),
            plural(self.snapshot.trusted_live_sessions(), "session", "sessions"),
            needs_input_phrase(self.snapshot.needs_input_count()),
        );
        let idle = self.snapshot.idle_count();
        if idle > 0 {
            heading.push_str(&format!(" · {}", idle_phrase(idle)));
        }
        let last_known = self.snapshot.last_known_sessions();
        if last_known > 0 {
            heading.push_str(&format!(" · {last_known} last-known"));
        }
        heading
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
            if !trusted {
                lines.push(
                    Prose::new(
                        "<dim>  last known — host is stale, not counted as live</dim>",
                    )
                    .render(term),
                );
            }
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
                    needs_cell(session, trusted, &self.inline).into(),
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
            if !trusted {
                section = section.add_component(block_text(
                    BlockTag::P,
                    format!("{base}__host-last-known"),
                    "last known — host is stale, not counted as live".to_string(),
                ));
            }
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
                    needs_plain(session, trusted).into(),
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
        Staleness::FreshnessUnknown => {
            "<yellow>freshness unknown · no direct peer — sessions unknown</yellow>".to_string()
        }
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
        Staleness::FreshnessUnknown => {
            "freshness unknown · no direct peer — sessions unknown".to_string()
        }
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
    let status = session.status.clone().unwrap_or_else(|| "—".to_string());
    // Local host only: surface how long ago the producer last stamped
    // this entry, so a crashed-producer phantom reads as stale.
    match session.reported_age_ms {
        Some(age) => format!("{status} · reported {} ago", format_age(age)),
        None => status,
    }
}

/// The honest three-way reading of a session that is NOT actively
/// waiting on the user (dashboard spec D5; review Finding 4 tail):
/// absence of a permission signal must read as "can't tell", never as a
/// negative observation.
enum Support {
    /// The provider can emit a permission signal and none is outstanding.
    NoIntervention,
    /// The provider cannot emit a permission signal at all — its silence
    /// carries no information, so the UI must not imply "all good".
    Unavailable,
    /// Untrusted host, or a provider that recorded no signal capability:
    /// we simply do not know.
    Unknown,
}

/// Classify a non-waiting session's support state. Untrusted hosts are
/// always [`Support::Unknown`] — a stale replica's "supported" claim is
/// no more trustworthy than its status.
fn support(session: &SessionRow, trusted: bool) -> Support {
    if !trusted {
        return Support::Unknown;
    }
    match session.permission_signal.as_deref() {
        Some("unsupported") => Support::Unavailable,
        Some("supported") => Support::NoIntervention,
        _ => Support::Unknown,
    }
}

/// Humanized "why · who" behind the winning status, or `None` when the
/// daemon projected neither (older entries) — so a legacy row renders
/// exactly as before rather than gaining an empty provenance tail.
fn needs_provenance(session: &SessionRow) -> Option<String> {
    let basis = session.status_basis.as_deref().map(basis_label);
    let producer = session.status_producer.as_deref().map(producer_label);
    match (basis, producer) {
        (Some(b), Some(p)) => Some(format!("{b} · via {p}")),
        (Some(b), None) => Some(b.to_string()),
        (None, Some(p)) => Some(format!("via {p}")),
        (None, None) => None,
    }
}

fn basis_label(basis: &str) -> &str {
    match basis {
        "permission_ask" => "permission ask",
        "interactive_turn_complete" => "turn complete",
        "process_heuristic" => "process heuristic",
        "lifecycle" => "lifecycle",
        // Forward-compat: show an unrecognized basis verbatim rather than
        // dropping the daemon's provenance.
        other => other,
    }
}

fn producer_label(producer: &str) -> &str {
    match producer {
        "sink" => "stream sink",
        "permission_hook" => "permission hook",
        "idle_hook" => "idle hook",
        "lifecycle" => "lifecycle",
        "process_monitor" => "process monitor",
        other => other,
    }
}

fn needs_cell(session: &SessionRow, trusted: bool, inline: &Terminal) -> String {
    match session.intervention {
        Intervention::NeedsInput => {
            let mut tail: Vec<String> = Vec::new();
            if let Some(prov) = needs_provenance(session) {
                tail.push(prov);
            }
            if let Some(age) = session.reported_age_ms {
                // Age the urgency: a needs-input backed by a very old
                // entry must not read as freshly urgent.
                tail.push(format!("{} ago", format_age(age)));
            }
            let badge = if tail.is_empty() {
                "<yellow><bold>⚠ input</bold></yellow>".to_string()
            } else {
                format!(
                    "<yellow><bold>⚠ input</bold></yellow> <dim>· {}</dim>",
                    tail.join(" · ")
                )
            };
            Prose::new(badge).render(inline)
        }
        Intervention::PossiblyIdle => {
            // Weaker than needs-input: a dim marker, plus the idle
            // duration when known (local host only — remote skew makes it
            // meaningless, so it renders without one).
            let badge = match session.reported_age_ms {
                Some(age) => format!("<dim>◦ idle {}</dim>", format_age(age)),
                None => "<dim>◦ idle</dim>".to_string(),
            };
            Prose::new(badge).render(inline)
        }
        Intervention::None => match support(session, trusted) {
            Support::NoIntervention => Prose::new("<dim>no intervention needed</dim>").render(inline),
            Support::Unavailable => {
                Prose::new("<dim>permission signal unavailable</dim>").render(inline)
            }
            Support::Unknown => "—".to_string(),
        },
    }
}

fn needs_plain(session: &SessionRow, trusted: bool) -> String {
    match session.intervention {
        Intervention::NeedsInput => {
            let mut parts = vec!["needs input".to_string()];
            if let Some(prov) = needs_provenance(session) {
                parts.push(prov);
            }
            if let Some(age) = session.reported_age_ms {
                parts.push(format!("{} ago", format_age(age)));
            }
            parts.join(" · ")
        }
        Intervention::PossiblyIdle => match session.reported_age_ms {
            Some(age) => format!("idle {}", format_age(age)),
            None => "idle".to_string(),
        },
        Intervention::None => match support(session, trusted) {
            Support::NoIntervention => "no intervention needed".to_string(),
            Support::Unavailable => "permission signal unavailable".to_string(),
            Support::Unknown => "—".to_string(),
        },
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

/// Idle-count phrase appended to the heading only when non-zero, so a
/// mesh with no idle sessions reads exactly as before.
fn idle_phrase(n: usize) -> String {
    format!("{n} idle")
}
