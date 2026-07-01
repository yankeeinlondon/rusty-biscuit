//! A hierarchical, unit-aligned metrics report (`MetricsTree`).
//!
//! Renders a tree of labelled measurements as box-drawing rows with a
//! unit-aligned value column, a right-aligned share column, and an optional
//! single highlight marker on the dominant row. The component is value-generic
//! ([`MetricValue::Duration`] / [`MetricValue::Bytes`] / [`MetricValue::Count`])
//! so it can back duration reports (claudine `--perf`), byte-size breakdowns, or
//! plain counts from one renderer.
//!
//! Layout, connectors, and markers are owned here; the *content* of each node
//! (which value, which share, which row is hot) is decided by the caller. The
//! component folds its own glyphs to ASCII and lets [`Prose`] handle color
//! degradation, so it adapts to non-Unicode / `NO_COLOR` terminals via the
//! shared [`Terminal`] plumbing — consistent with [`Prose`] and
//! [`BlockQuote`](crate::components::block_quote::BlockQuote).

use std::any::Any;
use std::time::Duration;

use crate::components::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

use renderable::tree::{RenderNode, TreeRenderable};

/// A single measured value attached to a [`MetricNode`].
///
/// The value is split into a mantissa and a unit suffix at render time so the
/// value column can align at the unit boundary across the whole tree (e.g.
/// `538.4` + `ms`). [`MetricValue::Placeholder`] renders an em dash (folded to
/// `-` on ASCII terminals) for rows that carry no value, such as a skipped
/// dry-run stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricValue {
    /// A wall-clock duration, formatted `µs` / `ms` / `s` by magnitude.
    Duration(Duration),
    /// A byte size, formatted `B` / `KB` / `MB` / `GB` / `TB`.
    Bytes(u64),
    /// A plain count with no unit suffix.
    Count(u64),
    /// No value — rendered as an em dash.
    Placeholder,
}

impl MetricValue {
    /// Split the value into `(mantissa, unit)` for the unit-aligned column.
    ///
    /// When `unicode` is false the micro sign (`µ`) folds to `u` and the
    /// placeholder em dash folds to `-`, so the value stays legible on a
    /// terminal that cannot render the glyphs.
    fn parts(&self, unicode: bool) -> (String, String) {
        match self {
            MetricValue::Duration(d) => {
                let (num, unit) = split_unit(&fmt_duration(*d));
                let unit = if unicode { unit } else { unit.replace('µ', "u") };
                (num, unit)
            }
            MetricValue::Bytes(b) => split_unit(&fmt_bytes(*b)),
            MetricValue::Count(n) => (n.to_string(), String::new()),
            MetricValue::Placeholder => {
                ((if unicode { "—" } else { "-" }).to_string(), String::new())
            }
        }
    }
}

/// A node's share of the report total, rendered in the right-aligned percent
/// column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricShare {
    /// The report root — always rendered `100%`.
    Full,
    /// A fraction in `0.0..=1.0` of the report total. Sub-one-percent slivers
    /// render `<1%` (so the column never fills with `0%`); a measured share
    /// caps at `99%`, reserving `100%` for the root.
    Of(f64),
    /// Share is not applicable (e.g. a placeholder row) — rendered as an em
    /// dash.
    Unknown,
}

impl MetricShare {
    fn render(&self, unicode: bool) -> String {
        match self {
            MetricShare::Full => "100%".to_string(),
            MetricShare::Of(fraction) => {
                if *fraction <= 0.0 {
                    return "0%".to_string();
                }
                let pct = fraction * 100.0;
                if pct < 1.0 {
                    "<1%".to_string()
                } else {
                    format!("{}%", (pct.round() as u64).min(99))
                }
            }
            MetricShare::Unknown => (if unicode { "—" } else { "-" }).to_string(),
        }
    }
}

/// A marker flagging one node for emphasis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricMarker {
    /// The single dominant row — rendered with the tree's highlight glyph and
    /// label (default `▇ HOT`).
    Highlight,
}

/// One node in a [`MetricsTree`].
///
/// `children` nest one level deeper with box-drawing connectors derived from
/// depth. The optional decorations — `marker`, `calls`, `emphasize`, `note` —
/// are layered on after the column math so they never skew alignment.
#[derive(Debug, Clone)]
pub struct MetricNode {
    /// The row label (rendered verbatim after the connector prefix).
    pub label: String,
    /// The measured value for the value column.
    pub value: MetricValue,
    /// The node's share of the report total.
    pub share: MetricShare,
    /// Flags this node for the single highlight marker.
    pub marker: Option<MetricMarker>,
    /// A run count surfaced as `×N` when `> 1`.
    pub calls: Option<usize>,
    /// Render the label bold (typically the root).
    pub emphasize: bool,
    /// A dim trailing annotation (e.g. `(dry run)`).
    pub note: Option<String>,
    /// Child rows.
    pub children: Vec<MetricNode>,
}

impl MetricNode {
    /// A childless node carrying a value and share.
    pub fn leaf(label: impl Into<String>, value: MetricValue, share: MetricShare) -> Self {
        Self {
            label: label.into(),
            value,
            share,
            marker: None,
            calls: None,
            emphasize: false,
            note: None,
            children: Vec::new(),
        }
    }

    /// A node with children.
    pub fn branch(
        label: impl Into<String>,
        value: MetricValue,
        share: MetricShare,
        children: Vec<MetricNode>,
    ) -> Self {
        Self {
            children,
            ..Self::leaf(label, value, share)
        }
    }

    /// Flag this node for the highlight marker.
    pub fn with_marker(mut self, marker: MetricMarker) -> Self {
        self.marker = Some(marker);
        self
    }

    /// Attach a run count (surfaced as `×N` when `> 1`).
    pub fn with_calls(mut self, calls: usize) -> Self {
        self.calls = Some(calls);
        self
    }

    /// Render this node's label bold.
    pub fn emphasized(mut self) -> Self {
        self.emphasize = true;
        self
    }

    /// Attach a dim trailing annotation.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// A hierarchical metrics report rendered as an aligned, connector-bearing
/// tree.
///
/// ## Examples
///
/// ```
/// use std::time::Duration;
/// use biscuit_terminal::components::metrics_tree::{
///     MetricsTree, MetricNode, MetricValue, MetricShare,
/// };
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// let root = MetricNode::branch(
///     "Total",
///     MetricValue::Duration(Duration::from_millis(100)),
///     MetricShare::Full,
///     vec![
///         MetricNode::leaf(
///             "parse",
///             MetricValue::Duration(Duration::from_millis(40)),
///             MetricShare::Of(0.4),
///         ),
///         MetricNode::leaf(
///             "render",
///             MetricValue::Duration(Duration::from_millis(60)),
///             MetricShare::Of(0.6),
///         ),
///     ],
/// )
/// .emphasized();
///
/// let rendered = MetricsTree::new(root).render_optimistic(Some(80));
/// assert!(rendered.contains("Total"));
/// assert!(rendered.contains("100%"));
/// assert!(rendered.contains("├─"));
/// ```
///
/// ## Layout & Style Contract
///
/// `MetricsTree` is a structured-text component whose output composes Prose
/// markup (column-aligned rows with `<b>` / `<dim>` / `<red>` tags). Per
/// spec D5 / Task 3.5 the preferred path is a tree projection; MetricsTree
/// achieves this by **delegating to `Prose`**, which is a `TreeRenderable`
/// whose projection is lowered through the shared render-tree fold. The
/// outer `Layout` (`margin`, `alignment`, `max_width`, `word_wrap`) is
/// propagated onto the inner `Prose` so the fold applies the box model
/// (spec C1). The intra-row column math (label / value / share alignment)
/// is computed against the available width before the markup is built, so
/// the row geometry stays inside the box the fold hands it.
///
/// The honored subset is therefore the full block surface:
///
/// | Property | Status | Rationale |
/// |----------|--------|-----------|
/// | `Layout::margin` | **Honored** | Propagated to the inner `Prose`; the fold applies it. |
/// | `Layout::alignment` | **Honored** | Propagated to the inner `Prose`. |
/// | `Layout::max_width` | **Honored** | Propagated to the inner `Prose`; caps the row width. |
/// | `Layout::width` | **Honored** | Propagated to the inner `Prose`. |
/// | `Layout::word_wrap` | **Honored** | Propagated to the inner `Prose`; long labels are truncated by `build_markup` before wrapping is needed. |
/// | `Style::color` / `emphasis` | **Honored via Prose markup** | The `<red>` / `<b>` / `<dim>` tags in the built markup are lowered to `Style::emphasis` / `Style::color` by Prose. |
/// | `Style::background` | **N/A** | A metrics tree has no padding-box background; per-row coloring is done via the `<red>` / `<dim>` markup tags. |
/// | `Style::border` | **N/A** | The connector glyphs (`├─`, `└─`) are themselves structural drawing; a CSS-style border cannot frame them without disrupting the tree. |
///
/// Parity for the honored subset is pinned in `metrics_tree_parity.rs`.
#[derive(Debug, Clone)]
pub struct MetricsTree {
    root: MetricNode,
    /// Italic trailing notes rendered below the tree.
    notes: Vec<String>,
    /// The highlight label following the marker glyph (default `HOT`).
    highlight_label: String,
    layout: Layout,
}

impl MetricsTree {
    /// Build a metrics tree from its root node.
    pub fn new(root: MetricNode) -> Self {
        Self {
            root,
            notes: Vec::new(),
            highlight_label: "HOT".to_string(),
            layout: Layout::default(),
        }
    }

    /// Attach italic trailing notes rendered below the tree.
    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes = notes;
        self
    }

    /// Override the highlight label that follows the marker glyph.
    pub fn with_highlight_label(mut self, label: impl Into<String>) -> Self {
        self.highlight_label = label.into();
        self
    }

    /// Assemble the Prose-markup body for the whole report.
    ///
    /// Connectors and glyphs fold to ASCII when `unicode` is false; the visible
    /// column widths are computed from the pre-markup strings so styling never
    /// shifts alignment. `available_width` is the visible width the row must fit
    /// (already net of any outer border); the label column is capped to it so a
    /// single long label cannot push the value column past the wrap point.
    fn build_markup(&self, unicode: bool, available_width: usize) -> String {
        let glyphs = Glyphs::for_terminal(unicode);

        let mut rows = Vec::new();
        collect_rows(&self.root, String::new(), String::new(), true, unicode, &glyphs, &mut rows);

        let raw_label_w = rows.iter().map(|r| r.label.chars().count()).max().unwrap_or(0);
        let num_w = rows.iter().map(|r| r.num.chars().count()).max().unwrap_or(0);
        let unit_w = rows.iter().map(|r| r.unit.chars().count()).max().unwrap_or(0);
        let share_w = rows.iter().map(|r| r.share.chars().count()).max().unwrap_or(0);

        // The right-hand columns are fixed: two-space gap, value (mantissa +
        // unit), two-space gap, share, then the widest row decoration (calls /
        // hot marker / note). Cap the label column so even the longest label
        // leaves room for them on one line — otherwise one verbose label (a
        // `::shell` command, say) inflates the shared column and every row's
        // value wraps. `MIN_LABEL_W` keeps labels legible on absurdly narrow
        // widths rather than collapsing to the ellipsis.
        const MIN_LABEL_W: usize = 8;
        let decoration_w = rows
            .iter()
            .map(|r| row_decoration_width(r, &glyphs, &self.highlight_label))
            .max()
            .unwrap_or(0);
        let fixed_cols = 2 + num_w + unit_w + 2 + share_w + decoration_w;
        let max_label_w = available_width.saturating_sub(fixed_cols).max(MIN_LABEL_W);
        let label_w = raw_label_w.min(max_label_w);

        let mut body = String::new();
        for row in &rows {
            let truncated = truncate_to_width(&row.label, label_w, unicode);
            let mut label = format!("{truncated:<label_w$}");
            if row.emphasize {
                label = format!("<b>{label}</b>");
            }
            // Pad the mantissa and unit to their column widths BEFORE wrapping
            // in markup: widths were computed from the bare strings, so any tag
            // counted into them would skew the value column.
            let num = format!("{:>num_w$}", row.num);
            // Microsecond units read grey rather than dim — at µs granularity the
            // row is negligible, and grey keeps the suffix legible where dim can
            // wash out entirely. `us` is the ASCII fold of `µs`. Coarser units
            // stay dim. The trailing pad spaces sit inside the tag harmlessly.
            let unit_tag = if matches!(row.unit.as_str(), "µs" | "us") {
                "grey"
            } else {
                "dim"
            };
            let unit = format!("<{unit_tag}>{:<unit_w$}</{unit_tag}>", row.unit);
            // A hot row's whole value (mantissa and unit) reads red so the
            // dominant cost is scannable. The styled unit nests inside the red so
            // the markup stays well-formed (`<red>…<dim>…</dim>…</red>`).
            let value = if row.hot {
                format!("<red>{num}{unit}</red>")
            } else {
                format!("{num}{unit}")
            };
            // Escape the literal `<` in `<1%` so Prose's tag tokenizer treats it
            // as text rather than an opening tag (the backslash is invisible, so
            // the column width computed above still holds). Sub-percent slivers
            // are dimmed to recede; the dim wraps the padded+escaped cell so the
            // right-align spaces stay outside the visible glyphs.
            let share = format!("{:>share_w$}", row.share).replace('<', "\\<");
            let share = if row.share == "<1%" {
                format!("<dim>{share}</dim>")
            } else {
                share
            };
            body.push_str(&format!("{label}  {value}  {share}"));
            if let Some(calls) = row.calls.filter(|c| *c > 1) {
                body.push_str(&format!("  <dim>{}{calls}</dim>", glyphs.times));
            }
            if row.hot {
                body.push_str(&format!(
                    " <b>{} {}</b>",
                    glyphs.highlight, self.highlight_label
                ));
            }
            if let Some(note) = &row.note {
                body.push_str(&format!(" <dim>{note}</dim>"));
            }
            body.push('\n');
        }

        for note in &self.notes {
            body.push_str(&format!("\n<i>{note}</i>\n"));
        }

        body.trim_end().to_string()
    }
}

/// One fully-prepared row of the rendered tree (visible text only).
struct RenderRow {
    /// Connector prefix plus the node label.
    label: String,
    /// Value mantissa.
    num: String,
    /// Value unit suffix (empty for counts and placeholders).
    unit: String,
    /// Share-of-total cell.
    share: String,
    /// Run count, surfaced when `> 1`.
    calls: Option<usize>,
    /// This row carries the highlight marker.
    hot: bool,
    /// Render the label bold.
    emphasize: bool,
    /// A dim trailing annotation.
    note: Option<String>,
}

/// The connector and inline glyphs for one terminal capability tier.
struct Glyphs {
    /// Non-final child connector.
    tee: &'static str,
    /// Final child connector.
    elbow: &'static str,
    /// Continuation under a non-final child.
    pipe: &'static str,
    /// Continuation under a final child.
    blank: &'static str,
    /// The `calls` multiplier sign.
    times: &'static str,
    /// The highlight marker glyph.
    highlight: &'static str,
}

impl Glyphs {
    /// ASCII fallbacks avoid every Prose-special character (`<`, `>`, `{`, `*`,
    /// `_`, `[`, `]`, `(`, `)`, `\`) so the folded markup never needs escaping.
    fn for_terminal(unicode: bool) -> Self {
        if unicode {
            Self {
                tee: "├─ ",
                elbow: "└─ ",
                pipe: "│  ",
                blank: "   ",
                times: "×",
                highlight: "▇",
            }
        } else {
            Self {
                tee: "+- ",
                elbow: "+- ",
                pipe: "|  ",
                blank: "   ",
                times: "x",
                highlight: "#",
            }
        }
    }
}

/// Walk the tree depth-first, emitting one [`RenderRow`] per node with
/// box-drawing connectors derived from depth.
fn collect_rows(
    node: &MetricNode,
    label_prefix: String,
    children_base: String,
    is_root: bool,
    unicode: bool,
    glyphs: &Glyphs,
    rows: &mut Vec<RenderRow>,
) {
    let (num, unit) = node.value.parts(unicode);
    let share = if is_root {
        MetricShare::Full.render(unicode)
    } else {
        node.share.render(unicode)
    };
    rows.push(RenderRow {
        label: format!("{label_prefix}{}", node.label),
        num,
        unit,
        share,
        calls: node.calls,
        hot: node.marker == Some(MetricMarker::Highlight),
        emphasize: node.emphasize,
        note: node.note.clone(),
    });

    let last = node.children.len().saturating_sub(1);
    for (i, child) in node.children.iter().enumerate() {
        let connector = if i == last { glyphs.elbow } else { glyphs.tee };
        let cont = if i == last { glyphs.blank } else { glyphs.pipe };
        collect_rows(
            child,
            format!("{children_base}{connector}"),
            format!("{children_base}{cont}"),
            false,
            unicode,
            glyphs,
            rows,
        );
    }
}

/// Format a duration using the most readable unit for the magnitude.
///
/// Every unit carries exactly one decimal place so the mantissae line up in the
/// value column regardless of magnitude.
fn fmt_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros < 1_000 {
        format!("{:.1}µs", micros as f64)
    } else if micros < 1_000_000 {
        format!("{:.1}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

/// Format a byte size using the most readable binary unit for the magnitude.
fn fmt_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes}B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1}{}", UNITS[unit])
}

/// Split a formatted value into `(mantissa, unit)` at the unit boundary so the
/// value column can align units: `"538.4ms"` → `("538.4", "ms")`.
fn split_unit(value: &str) -> (String, String) {
    let boundary = value
        .find(|c: char| c != '.' && !c.is_ascii_digit())
        .unwrap_or(value.len());
    (value[..boundary].to_string(), value[boundary..].to_string())
}

/// Visible width of the trailing decorations appended after a row's share cell:
/// the run-count multiplier, the hot marker, and the dim note. Mirrors exactly
/// what `build_markup` emits so the label cap reserves the right amount.
fn row_decoration_width(row: &RenderRow, glyphs: &Glyphs, highlight_label: &str) -> usize {
    let mut width = 0;
    if let Some(calls) = row.calls.filter(|c| *c > 1) {
        // "  {times}{calls}"
        width += 2 + glyphs.times.chars().count() + calls.to_string().chars().count();
    }
    if row.hot {
        // " {highlight} {label}"
        width += 1 + glyphs.highlight.chars().count() + 1 + highlight_label.chars().count();
    }
    if let Some(note) = &row.note {
        // " {note}"
        width += 1 + note.chars().count();
    }
    width
}

/// Truncate a label to at most `max` visible chars, appending an ellipsis when
/// it overflows. The connector prefix sits at the head of `label`, so trimming
/// the tail keeps the tree structure intact. Plain text only — called before
/// any Prose markup is wrapped around the result.
fn truncate_to_width(label: &str, max: usize, unicode: bool) -> String {
    if label.chars().count() <= max {
        return label.to_string();
    }
    let ellipsis = if unicode { "…" } else { "..." };
    let ellipsis_w = ellipsis.chars().count();
    if max <= ellipsis_w {
        return label.chars().take(max).collect();
    }
    let kept: String = label.chars().take(max - ellipsis_w).collect();
    format!("{kept}{ellipsis}")
}

impl TerminalRenderable for MetricsTree {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80) as usize;
        // Propagate the outer `Layout` onto the inner `Prose` so the shared
        // render-tree fold honors `margin` / `alignment` / `max_width` /
        // `word_wrap` (spec C1/C5). The markup text is identical; only the
        // outer box placement is delegated.
        let mut prose = Prose::new(self.build_markup(true, width));
        *prose.layout_mut() = self.layout.clone();
        prose.render_optimistic(term_width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width() as usize;
        // See `render_optimistic`: propagate the outer Layout so the fold
        // applies the box model.
        let mut prose = Prose::new(self.build_markup(term.supports_unicode, width));
        *prose.layout_mut() = self.layout.clone();
        prose.render(term)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }
}

impl TreeRenderable for MetricsTree {
    /// Projects the metric tree through [`Prose`]'s tree projection so the
    /// markup (`<b>`, `<dim>`, `<red>`) lowers to proper inline nodes.
    ///
    /// The outer [`Layout`] is propagated onto the inner `Prose`, matching
    /// the terminal render path so both surfaces carry the same box model.
    fn render_tree(&self) -> RenderNode {
        // Render at a generous width so the markup text is not pre-truncated;
        // the fold re-resolves the box against the real terminal width.
        let markup = self.build_markup(true, 240);
        let mut prose = Prose::new(markup);
        *prose.layout_mut() = self.layout.clone();
        prose.render_tree()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strip ANSI CSI escapes so layout assertions ignore styling.
    fn strip_ansi(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            break;
                        }
                    }
                }
                continue;
            }
            out.push(ch);
        }
        out
    }

    fn sample_tree() -> MetricNode {
        MetricNode::branch(
            "Performance",
            MetricValue::Duration(Duration::from_millis(100)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    "parse",
                    MetricValue::Duration(Duration::from_micros(45)),
                    MetricShare::Of(0.0005),
                ),
                MetricNode::branch(
                    "render",
                    MetricValue::Duration(Duration::from_millis(60)),
                    MetricShare::Of(0.6),
                    vec![MetricNode::leaf(
                        "layout",
                        MetricValue::Duration(Duration::from_millis(58)),
                        MetricShare::Of(0.58),
                    )],
                ),
            ],
        )
        .emphasized()
    }

    #[test]
    fn long_label_does_not_overflow_or_wrap_other_rows() {
        // A single verbose label (e.g. a `::shell` command) must be truncated so
        // it cannot inflate the shared label column and push every row's value
        // past the wrap point. Regression for the perf tree rendering every row
        // on two lines once a long command appeared.
        let long = "shell · sniff repo staged-files -v --plain --on-error \"No staged files; nothing to do\"";
        let root = MetricNode::branch(
            "Performance",
            MetricValue::Duration(Duration::from_millis(100)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    long,
                    MetricValue::Duration(Duration::from_millis(80)),
                    MetricShare::Of(0.8),
                ),
                MetricNode::leaf(
                    "short",
                    MetricValue::Duration(Duration::from_millis(20)),
                    MetricShare::Of(0.2),
                ),
            ],
        );
        let width = 80usize;
        let plain = strip_ansi(&MetricsTree::new(root).render_optimistic(Some(width as u32)));
        for line in plain.lines().filter(|l| !l.is_empty()) {
            assert!(
                line.chars().count() <= width,
                "row exceeds available width ({} cols):\n{line:?}",
                line.chars().count()
            );
        }
        // The long label is trimmed with an ellipsis but its value stays on the
        // same row (no wrap), and the short row keeps its full value too.
        let long_row = plain
            .lines()
            .find(|l| l.contains("shell ·"))
            .expect("long row present");
        assert!(long_row.contains('…'), "long label should be truncated:\n{long_row}");
        assert!(long_row.contains("80.0ms"), "long row value wrapped away:\n{long_row}");
    }

    #[test]
    fn renders_box_drawing_connectors() {
        let plain = strip_ansi(&MetricsTree::new(sample_tree()).render_optimistic(Some(80)));
        assert!(plain.contains("├─ parse"), "missing tee connector:\n{plain}");
        assert!(plain.contains("└─ render"), "missing elbow connector:\n{plain}");
        // The nested leaf sits one level deeper, under the elbow's blank gutter.
        assert!(
            plain.contains("   └─ layout"),
            "nested connector missing or misindented:\n{plain}"
        );
    }

    #[test]
    fn aligns_value_units_in_a_shared_column() {
        let plain = strip_ansi(&MetricsTree::new(sample_tree()).render_optimistic(Some(80)));
        // Every value's unit suffix begins at the same column: locate the unit
        // start (first alpha after the mantissa) on each data row.
        let unit_columns: Vec<usize> = plain
            .lines()
            .filter_map(|l| l.find("ms").or_else(|| l.find("µs")).map(|idx| (l, idx)))
            .map(|(l, idx)| l[..idx].chars().count())
            .collect();
        assert!(unit_columns.len() >= 2, "expected multiple value rows:\n{plain}");
        assert!(
            unit_columns.iter().all(|c| *c == unit_columns[0]),
            "value units are not column-aligned: {unit_columns:?}\n{plain}"
        );
    }

    #[test]
    fn root_reads_full_share_and_sub_percent_renders_marker() {
        let plain = strip_ansi(&MetricsTree::new(sample_tree()).render_optimistic(Some(80)));
        let root = plain.lines().find(|l| l.contains("Performance")).unwrap();
        assert!(root.contains("100%"), "root must read 100%; got: {root:?}");
        // The 45µs leaf is well under 1% of 100ms.
        let parse = plain.lines().find(|l| l.contains("parse")).unwrap();
        assert!(parse.contains("<1%"), "sub-percent share must read <1%; got: {parse:?}");
    }

    #[test]
    fn highlight_marker_flags_the_chosen_row() {
        let root = MetricNode::branch(
            "Total",
            MetricValue::Duration(Duration::from_millis(100)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    "hot stage",
                    MetricValue::Duration(Duration::from_millis(90)),
                    MetricShare::Of(0.9),
                )
                .with_marker(MetricMarker::Highlight),
                MetricNode::leaf(
                    "cool stage",
                    MetricValue::Duration(Duration::from_millis(10)),
                    MetricShare::Of(0.1),
                ),
            ],
        );
        let plain = strip_ansi(&MetricsTree::new(root).render_optimistic(Some(80)));
        let hot = plain.lines().find(|l| l.contains("▇ HOT")).unwrap();
        assert!(hot.contains("hot stage"), "marker on wrong row: {hot:?}");
        assert_eq!(plain.matches("▇ HOT").count(), 1, "exactly one marker:\n{plain}");
    }

    #[test]
    fn custom_highlight_label() {
        let root = MetricNode::branch(
            "Total",
            MetricValue::Count(2),
            MetricShare::Full,
            vec![
                MetricNode::leaf("a", MetricValue::Count(1), MetricShare::Of(0.5))
                    .with_marker(MetricMarker::Highlight),
            ],
        );
        let plain = strip_ansi(
            &MetricsTree::new(root)
                .with_highlight_label("MAX")
                .render_optimistic(Some(80)),
        );
        assert!(plain.contains("▇ MAX"), "custom label missing:\n{plain}");
    }

    #[test]
    fn calls_surface_only_when_greater_than_one() {
        let root = MetricNode::branch(
            "Total",
            MetricValue::Count(3),
            MetricShare::Full,
            vec![
                MetricNode::leaf("once", MetricValue::Count(1), MetricShare::Of(0.3)).with_calls(1),
                MetricNode::leaf("thrice", MetricValue::Count(2), MetricShare::Of(0.6))
                    .with_calls(3),
            ],
        );
        let plain = strip_ansi(&MetricsTree::new(root).render_optimistic(Some(80)));
        let once = plain.lines().find(|l| l.contains("once")).unwrap();
        let thrice = plain.lines().find(|l| l.contains("thrice")).unwrap();
        assert!(!once.contains('×'), "calls == 1 must not render: {once:?}");
        assert!(thrice.contains("×3"), "calls > 1 must render ×N: {thrice:?}");
    }

    #[test]
    fn placeholder_and_note_render_as_dash_and_annotation() {
        let root = MetricNode::branch(
            "Total",
            MetricValue::Duration(Duration::from_millis(50)),
            MetricShare::Full,
            vec![
                MetricNode::leaf("skipped", MetricValue::Placeholder, MetricShare::Unknown)
                    .with_note("(dry run)"),
            ],
        );
        let plain = strip_ansi(&MetricsTree::new(root).render_optimistic(Some(80)));
        let skipped = plain.lines().find(|l| l.contains("skipped")).unwrap();
        assert!(skipped.contains('—'), "placeholder must render em dash: {skipped:?}");
        assert!(skipped.contains("(dry run)"), "note missing: {skipped:?}");
    }

    #[test]
    fn bytes_format_with_binary_units() {
        let root = MetricNode::branch(
            "Disk",
            MetricValue::Bytes(2 * 1024 * 1024),
            MetricShare::Full,
            vec![MetricNode::leaf(
                "cache",
                MetricValue::Bytes(512),
                MetricShare::Of(0.01),
            )],
        );
        let plain = strip_ansi(&MetricsTree::new(root).render_optimistic(Some(80)));
        assert!(plain.contains("2.0MB"), "byte total mis-formatted:\n{plain}");
        assert!(plain.contains("512B"), "small byte value mis-formatted:\n{plain}");
    }

    #[test]
    fn notes_render_below_the_tree() {
        let plain = strip_ansi(
            &MetricsTree::new(sample_tree())
                .with_notes(vec!["partial metrics".to_string()])
                .render_optimistic(Some(80)),
        );
        assert!(plain.contains("partial metrics"), "trailing note missing:\n{plain}");
    }

    #[test]
    fn no_color_terminal_drops_color_sgr() {
        use crate::discovery::detection::ColorDepth;
        use crate::terminal::Terminal;

        // A node carrying an explicit color marker still degrades: under a
        // no-color terminal Prose emits no foreground/background color SGR
        // (`38;`/`48;`). Text attributes such as bold legitimately survive —
        // NO_COLOR concerns color only.
        let term = Terminal::builder()
            .color_depth(ColorDepth::None)
            .width(80)
            .supports_unicode(true)
            .build();
        let rendered = MetricsTree::new(sample_tree()).render(&term);
        assert!(
            !rendered.contains("38;") && !rendered.contains("48;"),
            "no-color render must not emit color SGR: {rendered:?}"
        );
        // The structure still renders — Unicode connectors stay, color drops.
        assert!(rendered.contains("├─"), "connectors lost under NO_COLOR:\n{rendered}");
    }

    #[test]
    fn ascii_terminal_folds_unicode_glyphs() {
        use crate::terminal::Terminal;

        let term = Terminal::builder()
            .width(80)
            .supports_unicode(false)
            .build();
        let root = MetricNode::branch(
            "Total",
            MetricValue::Duration(Duration::from_micros(900)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    "hot",
                    MetricValue::Duration(Duration::from_micros(800)),
                    MetricShare::Of(0.9),
                )
                .with_marker(MetricMarker::Highlight)
                .with_calls(2),
                MetricNode::leaf("gap", MetricValue::Placeholder, MetricShare::Unknown),
            ],
        );
        let plain = strip_ansi(&MetricsTree::new(root).render(&term));
        assert!(!plain.contains('├'), "box-drawing must fold to ASCII:\n{plain}");
        assert!(!plain.contains('│'), "box-drawing must fold to ASCII:\n{plain}");
        assert!(!plain.contains('▇'), "highlight glyph must fold to ASCII:\n{plain}");
        assert!(!plain.contains('×'), "times sign must fold to ASCII:\n{plain}");
        assert!(!plain.contains('µ'), "micro sign must fold to ASCII:\n{plain}");
        assert!(!plain.contains('—'), "em dash must fold to ASCII:\n{plain}");
        assert!(plain.contains("+- "), "ASCII connector missing:\n{plain}");
        assert!(plain.contains("# HOT"), "ASCII marker missing:\n{plain}");
        assert!(plain.contains("x2"), "ASCII calls missing:\n{plain}");
        assert!(plain.contains("900.0us"), "ASCII duration unit missing:\n{plain}");
    }

    #[test]
    fn microsecond_unit_renders_grey_not_dim() {
        // µs units read grey; coarser units stay dim. Assert the µs row carries a
        // foreground-color SGR (grey is a web color → `38;...`) on its unit,
        // distinct from the bare `\x1b[2m` dim used for ms.
        let root = MetricNode::branch(
            "Total",
            MetricValue::Duration(Duration::from_millis(100)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    "fast",
                    MetricValue::Duration(Duration::from_micros(45)),
                    MetricShare::Of(0.01),
                ),
                MetricNode::leaf(
                    "slow",
                    MetricValue::Duration(Duration::from_millis(60)),
                    MetricShare::Of(0.6),
                ),
            ],
        );
        let colored = MetricsTree::new(root).render_optimistic(Some(80));
        let micro_line = colored
            .lines()
            .find(|l| strip_ansi(l).contains("fast"))
            .expect("µs row present");
        let ms_line = colored
            .lines()
            .find(|l| strip_ansi(l).contains("slow"))
            .expect("ms row present");
        // The µs unit is a foreground color (grey), not the plain dim attribute.
        assert!(
            micro_line.contains("38;"),
            "µs unit must carry a grey fg-color SGR:\n{micro_line:?}"
        );
        // The ms unit still uses dim.
        assert!(
            ms_line.contains("\u{1b}[2m"),
            "ms unit must stay dim:\n{ms_line:?}"
        );
    }

    #[test]
    fn unit_suffix_renders_dim() {
        // build_markup is private; render WITH color and inspect the raw SGR.
        // The dim attribute is `\x1b[2m` and must wrap every unit suffix.
        let colored = MetricsTree::new(sample_tree()).render_optimistic(Some(80));
        assert!(
            colored.contains("\u{1b}[2m"),
            "unit suffix must emit dim SGR (\\x1b[2m):\n{colored:?}"
        );
    }

    #[test]
    fn hot_row_value_renders_red() {
        // Per repo lore `<red>` emits 3-bit `\x1b[31m`, never truecolor `38;2;`.
        let root = MetricNode::branch(
            "Total",
            MetricValue::Duration(Duration::from_millis(100)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    "hot stage",
                    MetricValue::Duration(Duration::from_millis(90)),
                    MetricShare::Of(0.9),
                )
                .with_marker(MetricMarker::Highlight),
            ],
        );
        let colored = MetricsTree::new(root).render_optimistic(Some(80));
        assert!(
            colored.contains("\u{1b}[31m"),
            "hot value must emit 3-bit red SGR (\\x1b[31m):\n{colored:?}"
        );
        assert!(
            !colored.contains("38;2;"),
            "red must stay 3-bit, not truecolor:\n{colored:?}"
        );
        // The red must enclose the hot row's value: the red SGR appears on the
        // line carrying the value, ahead of its reset, on the hot label's row.
        let hot_line = colored
            .lines()
            .find(|l| strip_ansi(l).contains("hot stage"))
            .expect("hot row present");
        assert!(
            hot_line.contains("\u{1b}[31m"),
            "red SGR must sit on the hot row:\n{hot_line:?}"
        );
    }

    #[test]
    fn sub_percent_share_renders_dim() {
        // The 45µs parse leaf is well under 1% of 100ms, so its share reads
        // `<1%` and must carry the dim attribute. Assert the `1%` glyphs are
        // preceded by a dim SGR with no intervening reset — i.e. the sliver text
        // itself is emitted while dim is active, not merely that dim appears
        // somewhere on the (unit-dimmed) row.
        let colored = MetricsTree::new(sample_tree()).render_optimistic(Some(80));
        let parse_line = colored
            .lines()
            .find(|l| strip_ansi(l).contains("parse"))
            .expect("parse row present");
        assert!(
            strip_ansi(parse_line).contains("<1%"),
            "parse share must read <1%:\n{parse_line:?}"
        );
        let pct_at = parse_line.find('%').expect("percent glyph present");
        let prefix = &parse_line[..pct_at];
        let last_dim = prefix.rfind("\u{1b}[2m");
        let last_reset = prefix
            .rfind("\u{1b}[0m")
            .or_else(|| prefix.rfind("\u{1b}[22m"));
        assert!(
            matches!((last_dim, last_reset), (Some(d), reset) if reset.is_none_or(|r| r < d)),
            "sub-percent sliver must render under an open dim SGR:\n{parse_line:?}"
        );
    }

    #[test]
    fn nested_red_dim_on_hot_unit_renders_without_corrupting_columns() {
        // The hot row's unit is both red (value) and dim (unit), nested
        // `<red>…<dim>…</dim>…</red>`. Stripping ANSI must still leave the unit
        // column aligned with the non-hot rows.
        let root = MetricNode::branch(
            "Total",
            MetricValue::Duration(Duration::from_millis(100)),
            MetricShare::Full,
            vec![
                MetricNode::leaf(
                    "hot stage",
                    MetricValue::Duration(Duration::from_millis(90)),
                    MetricShare::Of(0.9),
                )
                .with_marker(MetricMarker::Highlight),
                MetricNode::leaf(
                    "cool stage",
                    MetricValue::Duration(Duration::from_millis(10)),
                    MetricShare::Of(0.1),
                ),
            ],
        );
        let plain = strip_ansi(&MetricsTree::new(root).render_optimistic(Some(80)));
        let unit_columns: Vec<usize> = plain
            .lines()
            .filter_map(|l| l.find("ms").map(|idx| l[..idx].chars().count()))
            .collect();
        assert!(unit_columns.len() >= 2, "expected multiple ms rows:\n{plain}");
        assert!(
            unit_columns.iter().all(|c| *c == unit_columns[0]),
            "nested red+dim hot unit broke column alignment: {unit_columns:?}\n{plain}"
        );
    }
}
