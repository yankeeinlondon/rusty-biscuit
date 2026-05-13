//! Dependency-graph rendering for `sniff repo deps`.

use std::collections::HashSet;
use std::fmt::Write;
use std::rc::Rc;

use biscuit_terminal::components::graph_expression::{
    GraphExpression, GraphInputSyntax, GraphOrientation,
};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{BrowserRenderable, Renderable, RenderableContent};
use biscuit_terminal::components::terminal_image::{ImageWidth, parse_width_spec};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::repo::Package;

use super::packages::select_repo_packages;

/// Default rendered width for `sniff repo deps --ui`.
///
/// Chosen to give the diagram noticeably more room than the
/// biscuit-terminal 50% default so dependency graphs are legible.
const DEFAULT_DEPS_WIDTH: ImageWidth = ImageWidth::Percent(0.75);

/// Parse an `--orientation` CLI value into a `GraphOrientation`.
///
/// Accepts `lr`/`left-to-right`/`horizontal` and `tb`/`top-to-bottom`/`vertical`.
fn parse_orientation(spec: &str) -> Result<GraphOrientation, String> {
    match spec.trim().to_ascii_lowercase().as_str() {
        "lr" | "left-to-right" | "horizontal" => Ok(GraphOrientation::LeftToRight),
        "tb" | "top-to-bottom" | "vertical" => Ok(GraphOrientation::TopToBottom),
        other => Err(format!(
            "invalid --orientation value `{other}`; expected `lr` or `tb`"
        )),
    }
}

/// Build a DOT digraph source from workspace package dependencies.
///
/// In-focus packages are grouped into a single `cluster_<area>` subgraph per
/// area. External (out-of-focus) packages are rendered as bare nodes outside
/// any cluster so the diagram stays visually anchored on the focus area.
///
/// When `focus_names` is `Some`, only edges with at least one endpoint in the
/// focus set are emitted, and non-focus nodes are drawn dashed.
///
/// Returns `None` when there are no internal dependency edges.
pub(super) fn build_deps_dot(
    packages: &[Package],
    focus_names: Option<&HashSet<&str>>,
) -> Option<String> {
    use std::collections::HashMap;

    // Assign each package a stable node ID. DOT identifiers must not collide
    // with DOT keywords, so we use a synthetic `n<index>` prefix.
    let mut node_ids: HashMap<&str, String> = HashMap::new();
    for (i, pkg) in packages.iter().enumerate() {
        node_ids.insert(pkg.name.as_str(), format!("n{i}"));
    }

    let in_focus = |name: &str| -> bool {
        focus_names.is_none_or(|set| set.contains(name))
    };

    // Check if there are any edges that will actually be rendered.
    let has_edges = packages.iter().any(|p| {
        p.depends_on.iter().any(|dep| {
            node_ids.contains_key(dep.as_str())
                && (in_focus(p.name.as_str()) || in_focus(dep.as_str()))
        })
    });
    if !has_edges {
        return None;
    }

    // Partition into focus (grouped by area) and external (bare nodes).
    let mut focus_areas: Vec<&str> = Vec::new();
    let mut focus_area_packages: HashMap<&str, Vec<&Package>> = HashMap::new();
    let mut external_packages: Vec<&Package> = Vec::new();
    for pkg in packages {
        if in_focus(pkg.name.as_str()) {
            let area = pkg.package_area.as_str();
            if !focus_area_packages.contains_key(area) {
                focus_areas.push(area);
            }
            focus_area_packages.entry(area).or_default().push(pkg);
        } else {
            external_packages.push(pkg);
        }
    }

    let mut dot = String::from("digraph G {\n");

    // Emit focus subgraphs as DOT clusters (one nesting level only — the
    // biscuit-visualized validator rejects nested subgraphs).
    for (idx, area) in focus_areas.iter().enumerate() {
        let pkgs = &focus_area_packages[area];
        // Suppress the cluster wrapper for a single root-level package when
        // no focus filter is active (matches prior behavior).
        if focus_names.is_none() && pkgs.len() == 1 && *area == "root" {
            let pkg = pkgs[0];
            let id = &node_ids[pkg.name.as_str()];
            writeln!(dot, "    {id} [label=\"{}\"];", escape_dot(&pkg.name)).unwrap();
            continue;
        }

        let cluster_name = format!("cluster_{idx}_{}", sanitize_id(area));
        writeln!(dot, "    subgraph {cluster_name} {{").unwrap();
        writeln!(dot, "        label=\"{}\";", escape_dot(area)).unwrap();
        for pkg in pkgs {
            let id = &node_ids[pkg.name.as_str()];
            writeln!(dot, "        {id} [label=\"{}\"];", escape_dot(&pkg.name)).unwrap();
        }
        writeln!(dot, "    }}").unwrap();
    }

    // Emit external (out-of-focus) packages as bare nodes outside any
    // cluster, drawn dashed so they read as context rather than focus.
    for pkg in &external_packages {
        let id = &node_ids[pkg.name.as_str()];
        writeln!(
            dot,
            "    {id} [label=\"{}\", style=dashed];",
            escape_dot(&pkg.name)
        )
        .unwrap();
    }

    // Emit edges (filtered to those touching the focus set when applicable).
    for pkg in packages {
        let from_in_focus = in_focus(pkg.name.as_str());
        let from = &node_ids[pkg.name.as_str()];
        for dep_name in &pkg.depends_on {
            if let Some(to) = node_ids.get(dep_name.as_str())
                && (from_in_focus || in_focus(dep_name.as_str()))
            {
                writeln!(dot, "    {from} -> {to};").unwrap();
            }
        }
    }

    dot.push_str("}\n");
    Some(dot)
}

fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Produce a DOT-safe identifier suffix from an arbitrary string.
fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Result of preparing the deps graph for rendering.
///
/// Either the DOT source + orientation are ready to feed to a renderer,
/// or we have a pre-baked message string to emit instead (no packages,
/// no edges, invalid CLI value, etc.).
enum DepsGraphPrep {
    Graph {
        dot: String,
        orientation: GraphOrientation,
    },
    Message(String),
}

/// Validate inputs and produce the DOT source (with applied orientation) for
/// the focus-aware deps graph. Shared by the terminal (`--ui`) and SVG
/// (`--svg`) renderers so they always agree on the underlying graph.
fn prepare_deps_graph(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
    orientation_spec: Option<&str>,
) -> DepsGraphPrep {
    if !repo.is_monorepo {
        return DepsGraphPrep::Message(String::from(
            "deps requires a monorepo (no workspace packages found)",
        ));
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            return DepsGraphPrep::Message(String::from("No packages found in workspace"));
        }
    };

    // Dependency graphs are typically hub-and-spoke, so a left-to-right
    // layout produces a near-square canvas that scrolls vertically rather
    // than a wide-thin one that loses text fidelity when downscaled to
    // terminal width. `--orientation tb` reverts to top-to-bottom for
    // deep, chain-like graphs.
    let orientation = match orientation_spec {
        Some(spec) => match parse_orientation(spec) {
            Ok(o) => o,
            Err(e) => return DepsGraphPrep::Message(e),
        },
        None => GraphOrientation::LeftToRight,
    };

    let focus: Vec<&Package> = select_repo_packages(packages, repo_filter, package, package_area);
    let has_focus_filter =
        !repo_filter.is_empty() || package.is_some() || package_area.is_some();

    let (visible, focus_names): (Vec<Package>, Option<HashSet<String>>) = if has_focus_filter {
        let focus_set: HashSet<&str> = focus.iter().map(|p| p.name.as_str()).collect();
        let mut visible_names: HashSet<String> =
            focus.iter().map(|p| p.name.to_string()).collect();

        // 1-hop outbound: deps of focused packages
        for pkg in &focus {
            for dep in &pkg.depends_on {
                visible_names.insert(dep.clone());
            }
        }
        // 1-hop inbound: packages that depend on a focused package
        for pkg in packages {
            if pkg.depends_on.iter().any(|d| focus_set.contains(d.as_str())) {
                visible_names.insert(pkg.name.clone());
            }
        }

        let visible: Vec<Package> = packages
            .iter()
            .filter(|p| visible_names.contains(p.name.as_str()))
            .cloned()
            .collect();

        let names: HashSet<String> = focus.iter().map(|p| p.name.to_string()).collect();
        (visible, Some(names))
    } else {
        (focus.into_iter().cloned().collect(), None)
    };

    let focus_refs: Option<HashSet<&str>> = focus_names
        .as_ref()
        .map(|set| set.iter().map(String::as_str).collect());

    match build_deps_dot(&visible, focus_refs.as_ref()) {
        Some(dot) => DepsGraphPrep::Graph { dot, orientation },
        None => DepsGraphPrep::Message(String::from(
            "No internal dependencies found between workspace packages",
        )),
    }
}

/// Render an internal dependency diagram for the repository as a graph image.
///
/// Builds a DOT digraph from package dependency data and renders it inline
/// using biscuit-terminal's [`GraphExpression`] (which layers caching and
/// terminal-image output over biscuit-visualized). Falls back to a code
/// block if the terminal cannot display images.
///
/// `width_spec` accepts the same syntax as [`parse_width_spec`]: a percentage
/// (`"75%"`), a column count (`"120"` or `"120ch"`), or `"fill"`. When `None`,
/// defaults to [`DEFAULT_DEPS_WIDTH`] (75%).
pub fn render_repo_deps_visual(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
    width_spec: Option<&str>,
    orientation_spec: Option<&str>,
) -> String {
    let width = match width_spec {
        Some(spec) => match parse_width_spec(spec) {
            Ok(w) => w,
            Err(e) => {
                return format!(
                    "invalid --width value `{spec}`: {e}\n\
                     expected a percentage (e.g. `75%`), a column count \
                     (e.g. `120` or `120ch`), or `fill`"
                );
            }
        },
        None => DEFAULT_DEPS_WIDTH,
    };

    let (dot, orientation) = match prepare_deps_graph(
        repo,
        repo_filter,
        package,
        package_area,
        orientation_spec,
    ) {
        DepsGraphPrep::Graph { dot, orientation } => (dot, orientation),
        DepsGraphPrep::Message(msg) => return msg,
    };

    let graph = match GraphExpression::for_terminal(&dot, GraphInputSyntax::Dot) {
        Ok(g) => g.with_width(width).with_orientation(orientation),
        Err(e) => return format!("Failed to build dependency graph: {e}"),
    };
    let term = Terminal::default();
    graph.render(&term)
}

/// Render the dependency diagram as a standalone SVG document.
///
/// Returns the raw SVG source (no HTML wrapper, no terminal escape codes)
/// so callers can pipe the output to a file, send it to a browser, or
/// embed it in a static site. The graph respects the same orientation
/// rules as `--ui`.
pub fn render_repo_deps_svg(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
    orientation_spec: Option<&str>,
) -> String {
    let (dot, orientation) = match prepare_deps_graph(
        repo,
        repo_filter,
        package,
        package_area,
        orientation_spec,
    ) {
        DepsGraphPrep::Graph { dot, orientation } => (dot, orientation),
        DepsGraphPrep::Message(msg) => return msg,
    };

    let graph = match GraphExpression::for_terminal(&dot, GraphInputSyntax::Dot) {
        Ok(g) => g.with_orientation(orientation),
        Err(e) => return format!("Failed to build dependency graph: {e}"),
    };
    graph.render_to_browser()
}

/// Render an internal dependency list for the repository as styled text.
///
/// Each package with dependencies or dependents is shown as a top-level item
/// with `depends-on` and `used-by` sub-items. Isolates (packages with neither)
/// are omitted unless an explicit filter is set.
pub fn render_repo_deps_text(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    let mut out = String::new();

    if !repo.is_monorepo {
        return String::from("deps requires a monorepo (no workspace packages found)");
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            return String::from("No packages found in workspace");
        }
    };

    let filtered = select_repo_packages(packages, repo_filter, package, package_area);
    let has_explicit_filter =
        !repo_filter.is_empty() || package.is_some() || package_area.is_some();

    // Collect only packages that participate in dependency relationships
    // (unless an explicit filter is set, in which case show all matched)
    let relevant: Vec<&&Package> = filtered
        .iter()
        .filter(|pkg| has_explicit_filter || !pkg.depends_on.is_empty() || !pkg.used_by.is_empty())
        .collect();

    if relevant.is_empty() {
        return String::from("No internal dependencies found between workspace packages");
    }

    let title = if has_explicit_filter {
        format!(
            "<b><u>Dependencies</u></b> <dim>(showing {} of {} packages)</dim>",
            filtered.len(),
            packages.len(),
        )
    } else {
        format!(
            "<b><u>Dependencies</u></b> <dim>({} packages with dependencies)</dim>",
            relevant.len(),
        )
    };
    let term = Terminal::default();
    writeln!(out, "\n{}\n", Prose::new(&title).render(&term)).unwrap();

    let mut outer_items: Vec<RenderableContent> = Vec::new();
    for pkg in &relevant {
        let label = Prose::new(format!("<b><blue>{}</blue></b>", pkg.name)).render(&term);
        outer_items.push(RenderableContent::String(label));

        let mut detail_items: Vec<String> = Vec::new();
        if !pkg.depends_on.is_empty() {
            detail_items.push(
                Prose::new(format!("<b>depends-on:</b> {}", pkg.depends_on.join(", ")))
                    .render(&term),
            );
        }
        if !pkg.used_by.is_empty() {
            detail_items.push(
                Prose::new(format!("<b>used-by:</b> {}", pkg.used_by.join(", "))).render(&term),
            );
        }

        if !detail_items.is_empty() {
            let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
            outer_items.push(RenderableContent::Component(Rc::new(detail_list)));
        }
    }

    let list = UnorderedList::from(outer_items).with_indent_children(Some(4));
    write!(out, "{}", list.render(&term)).unwrap();

    writeln!(
        out,
        "\n{}",
        Prose::new(
            "<dim><i>use the <blue>--ui</blue> flag to show this in a visual format</i></dim>"
        )
        .render(&term)
    )
    .unwrap();

    out
}
