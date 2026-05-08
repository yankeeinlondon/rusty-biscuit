//! Dependency-graph rendering for `sniff repo deps`.

use std::fmt::Write;
use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::repo::Package;

use super::packages::select_repo_packages;

/// Build a Mermaid flowchart from workspace package dependencies.
///
/// Packages are grouped into subgraphs by `package_area`. Edges are drawn
/// from each package to its `depends_on` entries.
///
/// Returns `None` when there are no internal dependency edges.
pub(super) fn build_deps_mermaid(packages: &[Package]) -> Option<String> {
    use std::collections::HashMap;

    // Assign each package a stable node ID and build name→id lookup
    let mut node_ids: HashMap<&str, String> = HashMap::new();
    for (i, pkg) in packages.iter().enumerate() {
        node_ids.insert(pkg.name.as_str(), format!("n{i}"));
    }

    // Check if there are any edges at all
    let has_edges = packages.iter().any(|p| !p.depends_on.is_empty());
    if !has_edges {
        return None;
    }

    let mut lines = vec!["flowchart TD".to_string()];

    // Group packages by area, preserving discovery order
    let mut areas: Vec<&str> = Vec::new();
    let mut area_packages: HashMap<&str, Vec<&Package>> = HashMap::new();
    for pkg in packages {
        let area = pkg.package_area.as_str();
        if !area_packages.contains_key(area) {
            areas.push(area);
        }
        area_packages.entry(area).or_default().push(pkg);
    }

    // Emit subgraphs
    for area in &areas {
        let pkgs = &area_packages[area];
        if pkgs.len() == 1 && *area == "root" {
            // Single root-level package doesn't need a subgraph
            let pkg = pkgs[0];
            let id = &node_ids[pkg.name.as_str()];
            lines.push(format!("    {id}[\"{}\"]", pkg.name));
        } else {
            lines.push(format!("    subgraph {area}"));
            for pkg in pkgs {
                let id = &node_ids[pkg.name.as_str()];
                lines.push(format!("        {id}[\"{}\"]", pkg.name));
            }
            lines.push("    end".to_string());
        }
    }

    // Emit edges
    for pkg in packages {
        let from = &node_ids[pkg.name.as_str()];
        for dep_name in &pkg.depends_on {
            if let Some(to) = node_ids.get(dep_name.as_str()) {
                lines.push(format!("    {from} --> {to}"));
            }
        }
    }

    Some(lines.join("\n"))
}

/// Render an internal dependency diagram for the repository as a Mermaid image.
///
/// Builds a Mermaid flowchart from package dependency data and renders it
/// inline using `MermaidRenderer`. Falls back to a code block if the
/// terminal cannot display images or mmdc is not available.
pub fn render_repo_deps_visual(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
) -> String {
    if !repo.is_monorepo {
        return String::from("deps requires a monorepo (no workspace packages found)");
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            return String::from("No packages found in workspace");
        }
    };

    let filtered: Vec<Package> = select_repo_packages(packages, repo_filter, package, package_area)
        .into_iter()
        .cloned()
        .collect();

    let mermaid = match build_deps_mermaid(&filtered) {
        Some(m) => m,
        None => {
            return String::from("No internal dependencies found between workspace packages");
        }
    };

    let diagram = MermaidDiagram::new(&mermaid);
    let term = Terminal::default();
    diagram.render(&term)
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
