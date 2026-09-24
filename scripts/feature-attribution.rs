//! Attribute every workspace-crate recompile in a producer's owner tree to its
//! cause.
//!
//! The environment owner builds one Nextest archive per selected package, in
//! one target tree, as separate Cargo invocations. Cargo reuses a unit only when
//! its whole identity matches — its own features *and* every dependency's
//! identity — so a third-party flag near the base of the graph re-identifies
//! every workspace crate above it. This binary enumerates each
//! `(workspace crate, configuration)` the selection builds and, for each
//! configuration beyond a crate's first, names the root of the difference: the
//! crate's own features, a workspace dependency's features, or a third-party
//! dependency's features. A workspace crate here is a package with a library or
//! build script; binaries and test harnesses exist once per owner whatever the
//! features, so a bin- or test-only package is never one. Design and rationale:
//! `fixes/2026-09-21-ci-build-feature-divergence/spec.md`.
//!
//! Configurations come from `cargo tree` alone, so nothing is compiled. Compile
//! seconds are not derivable that way; `--events` joins the `ci-build` rustc
//! wrapper's event files from a timed local pass and charges each
//! configuration the seconds of the compile that built it (see [`join`]).
//! Under `--align`, a measured compile the aligned model no longer needs is
//! counted as removed, including one of two configurations a single owner
//! built that the alignment merges.
//!
//! ## Notes
//!
//! A local diagnostic behind `local-tools` (it reads the workspace through
//! `cargo_metadata`); nothing in CI runs it.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use biscuit_terminal::prelude::{Prose, Table, TableColumn, Terminal, TerminalRenderable};
use serde::{Deserialize, Serialize};

/// Bumped whenever the `--json` report's field set changes.
const REPORT_SCHEMA_VERSION: u32 = 1;

/// Target kinds (and rustc `--crate-type`s) that compile a library. With build
/// scripts, these are the only units whose identity features can multiply;
/// both the workspace model and the timed join are restricted to them.
const LIBRARY_KINDS: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

const USAGE: &str = "\
usage:
  feature-attribution --owners <pkg,pkg,...> [--target <triple>] [--workspace <dir>]
                      [--sidecars <file>] [--events <dir>] [--json <file>]
                      [--align <crate,crate,...>]

  --owners     the selected owner packages; archives are modeled in the
               producer's order (sorted by package name), whatever order is given
  --target     the triple the producer passes to Cargo (default: this host's)
  --workspace  the workspace root (default: the current directory)
  --sidecars   the sidecar table (default: <workspace>/.github/ci/sidecars.json
               when it exists)
  --events     a `ci-build` counter directory from a timed pass whose events
               are labeled with the owner package (BISCUIT_CI_BUILD_PACKAGE)
  --json       also write the machine-readable report to this file
  --align      what-if: model every owner resolving these third-party crates
               identically, as a reviewed alignment entry would; with
               --events, reports the timed seconds that would no longer compile.
               A workspace crate is refused
";

// ---------------------------------------------------------------------------
// `cargo tree` parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum EdgeKind {
    Normal,
    Build,
    Dev,
}

/// One package as `cargo tree --format '{p}|{f}'` displays it.
///
/// Keyed on `(name, version, source)`, never on the name alone: two versions of
/// one crate are two packages, and merging their feature sets fabricates a
/// divergence neither has.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PackageKey {
    name: String,
    version: String,
    /// The parenthesized source Cargo shows for non-crates.io packages (a path
    /// or git URL), or empty.
    source: String,
}

#[derive(Debug, Clone)]
struct TreeNode {
    package: PackageKey,
    features: BTreeSet<String>,
    /// Compiled for the host: a proc-macro, a build dependency, or anything
    /// beneath one. With an explicit `--target` those are separate units from
    /// the target-side ones even when their features match.
    host: bool,
    deps: BTreeSet<(usize, EdgeKind)>,
}

/// One owner invocation's resolved graph. `nodes[0]` is the root.
#[derive(Debug, Clone)]
struct Tree {
    nodes: Vec<TreeNode>,
}

/// Parse `cargo tree --charset ascii --format '{p}|{f}'` output.
///
/// Cargo prints a package's subtree once and marks later occurrences `(*)`; a
/// package with no dependencies is repeated without the mark. Both resolve to
/// the node first printed, keyed on the displayed package, its features, and
/// whether the context is host-side — Cargo's own node identity.
fn parse_tree(text: &str) -> Result<Tree> {
    let mut nodes: Vec<TreeNode> = Vec::new();
    let mut index: HashMap<(String, bool), usize> = HashMap::new();
    // One entry per depth: the node at that depth and the edge kind its
    // following children are listed under.
    let mut stack: Vec<(usize, EdgeKind)> = Vec::new();

    for (line_no, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let (depth, rest) = split_prefix(line);

        if let Some(header) = rest.strip_prefix('[') {
            let kind = match header.trim_end_matches(']') {
                "build-dependencies" => EdgeKind::Build,
                "dev-dependencies" => EdgeKind::Dev,
                other => bail!("line {}: unknown section header [{other}]", line_no + 1),
            };
            // A header at depth d introduces children of the node at depth d.
            stack.truncate(depth + 1);
            let Some(entry) = stack.get_mut(depth) else {
                bail!("line {}: section header with no parent", line_no + 1);
            };
            entry.1 = kind;
            continue;
        }

        let (display, _repeated) = match rest.strip_suffix(" (*)") {
            Some(display) => (display, true),
            None => (rest, false),
        };
        let (package_part, features_part) = display
            .rsplit_once('|')
            .with_context(|| format!("line {}: no '|' separator in {rest:?}", line_no + 1))?;
        let (package, proc_macro) = parse_package(package_part)
            .with_context(|| format!("line {}: unreadable package {package_part:?}", line_no + 1))?;

        let parent = if depth == 0 {
            if !nodes.is_empty() {
                bail!("line {}: a second root", line_no + 1);
            }
            None
        } else {
            stack.truncate(depth);
            Some(*stack.get(depth - 1).with_context(|| {
                format!("line {}: indentation skips a level", line_no + 1)
            })?)
        };
        let host = proc_macro
            || parent.is_some_and(|(parent, kind)| nodes[parent].host || kind == EdgeKind::Build);

        let key = (display.to_owned(), host);
        let node = match index.get(&key) {
            Some(&existing) => existing,
            None => {
                nodes.push(TreeNode {
                    package,
                    features: features_part
                        .split(',')
                        .filter(|feature| !feature.is_empty())
                        .map(str::to_owned)
                        .collect(),
                    host,
                    deps: BTreeSet::new(),
                });
                index.insert(key, nodes.len() - 1);
                nodes.len() - 1
            }
        };
        if let Some((parent, kind)) = parent {
            nodes[parent].deps.insert((node, kind));
        }
        stack.push((node, EdgeKind::Normal));
    }

    if nodes.is_empty() {
        bail!("cargo tree printed no packages");
    }
    Ok(Tree { nodes })
}

/// Consume the ASCII tree-drawing prefix: each level is four characters.
fn split_prefix(line: &str) -> (usize, &str) {
    let mut depth = 0;
    let mut rest = line;
    while let Some(next) = ["|   ", "    ", "|-- ", "`-- "]
        .iter()
        .find_map(|group| rest.strip_prefix(group))
    {
        depth += 1;
        rest = next;
    }
    (depth, rest)
}

/// Read `name vX.Y.Z[ (source)][ (proc-macro)]`.
fn parse_package(text: &str) -> Result<(PackageKey, bool)> {
    let (text, proc_macro) = match text.strip_suffix(" (proc-macro)") {
        Some(text) => (text, true),
        None => (text, false),
    };
    let (name, rest) = text.split_once(' ').context("no version")?;
    let (version, source) = match rest.split_once(' ') {
        Some((version, source)) => (version, source.trim_start_matches('(').trim_end_matches(')')),
        None => (rest, ""),
    };
    let version = version.strip_prefix('v').context("version lacks its 'v'")?;
    Ok((
        PackageKey {
            name: name.to_owned(),
            version: version.to_owned(),
            source: source.to_owned(),
        },
        proc_macro,
    ))
}

// ---------------------------------------------------------------------------
// Configurations
// ---------------------------------------------------------------------------

/// What makes two units of one package the same compile: its own features,
/// its host/target side, and the configurations of the dependencies it links.
/// Dev edges are excluded — they feed the root's test targets, never its
/// library — though their feature unification is already in the child nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConfigKey {
    package: PackageKey,
    features: BTreeSet<String>,
    host: bool,
    deps: Vec<usize>,
}

/// One package's `[features]` table and how its manifest names each
/// dependency, from `cargo metadata`.
#[derive(Debug, Clone, Default)]
struct PackageFeatures {
    features: BTreeMap<String, Vec<String>>,
    /// `(package name, name in this manifest)`: the second differs when the
    /// dependency is renamed, and is what `dep/feature` entries use.
    deps: Vec<(String, String)>,
}

/// Feature tables of every package in the resolve, keyed on `(name,
/// version)`: `cargo tree` and `cargo metadata` spell sources differently.
type FeatureTables = HashMap<(String, String), PackageFeatures>;

/// Every configuration seen across the selection, interned so that equal
/// configurations from different owners share one id.
#[derive(Debug, Default)]
struct Interner {
    configs: Vec<ConfigKey>,
    ids: HashMap<ConfigKey, usize>,
    /// The `--align` what-if: for each aligned third-party `(package, host)`,
    /// the dependencies every resolved variant of it shares.
    aligned: HashMap<(PackageKey, bool), BTreeSet<(PackageKey, bool)>>,
    /// The union of every variant's features, per aligned `(package, host)`.
    union: HashMap<(PackageKey, bool), BTreeSet<String>>,
    tables: FeatureTables,
}

impl Interner {
    /// An interner that models every owner resolving the `aligned` crates
    /// identically, as one alignment entry enabling the union of their
    /// features would.
    ///
    /// An aligned crate's identity keeps only the dependencies all of its
    /// variants share: a dependency only some variants pull in is one the
    /// union would give every owner. Features the union forwards to a
    /// dependency that is already in the graph (`feat = ["dep/x"]` or
    /// `"dep?/x"`) are added to that dependency, with everything they imply
    /// in its own table. An optional dependency the union newly enables is
    /// not modeled, so when its own configuration differs between owners the
    /// what-if can overstate the configurations that collapse.
    fn with_alignment(
        aligned: &BTreeSet<String>,
        invocations: &[Invocation],
        workspace: &BTreeSet<PackageKey>,
        tables: &FeatureTables,
    ) -> Result<Self> {
        if let Some(member) = workspace.iter().find(|package| aligned.contains(&package.name)) {
            bail!(
                "'{}' is a workspace crate: its own features are never aligned",
                member.name
            );
        }
        let mut shared: HashMap<(PackageKey, bool), BTreeSet<(PackageKey, bool)>> = HashMap::new();
        let mut union: HashMap<(PackageKey, bool), BTreeSet<String>> = HashMap::new();
        for invocation in invocations {
            let nodes = &invocation.tree.nodes;
            for node in nodes.iter().filter(|node| aligned.contains(&node.package.name)) {
                let deps: BTreeSet<_> = node
                    .deps
                    .iter()
                    .filter(|(_, kind)| *kind != EdgeKind::Dev)
                    .map(|(dep, _)| (nodes[*dep].package.clone(), nodes[*dep].host))
                    .collect();
                let key = (node.package.clone(), node.host);
                shared
                    .entry(key.clone())
                    .and_modify(|common| common.retain(|dep| deps.contains(dep)))
                    .or_insert(deps);
                union.entry(key).or_default().extend(node.features.iter().cloned());
            }
        }
        Ok(Self {
            aligned: shared,
            tables: if union.is_empty() { FeatureTables::new() } else { tables.clone() },
            union,
            ..Self::default()
        })
    }

    /// Features each node of `tree` gains from the aligned crates' unions,
    /// forwarded down `dep/feature` entries until nothing new is enabled.
    fn forwarded(&self, tree: &Tree) -> Vec<BTreeSet<String>> {
        let mut added = vec![BTreeSet::new(); tree.nodes.len()];
        let mut pending: Vec<(usize, BTreeSet<String>)> = tree
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(node, current)| {
                self.union
                    .get(&(current.package.clone(), current.host))
                    .map(|union| (node, union.clone()))
            })
            .collect();
        while let Some((node, enabled)) = pending.pop() {
            let current = &tree.nodes[node];
            let Some(table) = self.tables.get(&(current.package.name.clone(), current.package.version.clone()))
            else {
                continue;
            };
            let mut forwards: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
            let mut stack: Vec<&str> = enabled.iter().map(String::as_str).collect();
            let mut closed: BTreeSet<&str> = stack.iter().copied().collect();
            while let Some(feature) = stack.pop() {
                for entry in table.features.get(feature).into_iter().flatten() {
                    if let Some((dep, dep_feature)) = entry.split_once('/') {
                        forwards
                            .entry(dep.trim_end_matches('?'))
                            .or_default()
                            .insert(dep_feature.to_owned());
                    } else if !entry.starts_with("dep:") && closed.insert(entry) {
                        stack.push(entry);
                    }
                }
            }
            if !self.union.contains_key(&(current.package.clone(), current.host)) {
                let fresh: BTreeSet<String> = closed
                    .iter()
                    .filter(|feature| !current.features.contains(**feature))
                    .map(|feature| (*feature).to_owned())
                    .collect();
                added[node].extend(fresh);
            }
            for &(dep, kind) in &current.deps {
                let child = &tree.nodes[dep];
                if kind == EdgeKind::Dev || self.union.contains_key(&(child.package.clone(), child.host)) {
                    continue;
                }
                let gained: BTreeSet<String> = table
                    .deps
                    .iter()
                    .filter(|(package, _)| *package == child.package.name)
                    .filter_map(|(_, name)| forwards.get(name.as_str()))
                    .flatten()
                    .filter(|feature| !child.features.contains(*feature) && !added[dep].contains(*feature))
                    .cloned()
                    .collect();
                if !gained.is_empty() {
                    pending.push((dep, gained));
                }
            }
        }
        added
    }

    fn intern(&mut self, key: ConfigKey) -> usize {
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        self.configs.push(key.clone());
        self.ids.insert(key, self.configs.len() - 1);
        self.configs.len() - 1
    }

    /// Configuration ids for every node of `tree`, and the nodes the
    /// invocation builds (everything reachable from the root, dev edges
    /// included).
    fn resolve(&mut self, tree: &Tree) -> Result<(Vec<usize>, Vec<usize>)> {
        let mut ids: Vec<Option<usize>> = vec![None; tree.nodes.len()];
        let mut visiting = vec![false; tree.nodes.len()];
        let added = self.forwarded(tree);
        for node in 0..tree.nodes.len() {
            self.config_of(tree, &added, node, &mut ids, &mut visiting)?;
        }
        let mut reachable = BTreeSet::new();
        let mut pending = vec![0usize];
        while let Some(node) = pending.pop() {
            if reachable.insert(node) {
                pending.extend(tree.nodes[node].deps.iter().map(|(dep, _)| *dep));
            }
        }
        Ok((
            ids.into_iter().map(|id| id.expect("every node resolved")).collect(),
            reachable.into_iter().collect(),
        ))
    }

    fn config_of(
        &mut self,
        tree: &Tree,
        added: &[BTreeSet<String>],
        node: usize,
        ids: &mut Vec<Option<usize>>,
        visiting: &mut Vec<bool>,
    ) -> Result<usize> {
        if let Some(id) = ids[node] {
            return Ok(id);
        }
        if visiting[node] {
            bail!(
                "dependency cycle through {} outside dev edges",
                tree.nodes[node].package.name
            );
        }
        visiting[node] = true;
        let current = &tree.nodes[node];
        let shared = self.aligned.get(&(current.package.clone(), current.host)).cloned();
        let mut deps = Vec::new();
        for &(dep, kind) in &current.deps {
            let child = &tree.nodes[dep];
            let kept = shared
                .as_ref()
                .is_none_or(|shared| shared.contains(&(child.package.clone(), child.host)));
            if kind != EdgeKind::Dev && kept {
                deps.push(self.config_of(tree, added, dep, ids, visiting)?);
            }
        }
        deps.sort_unstable();
        deps.dedup();
        visiting[node] = false;
        let id = self.intern(ConfigKey {
            package: current.package.clone(),
            // Every owner resolves an aligned crate the same way.
            features: if shared.is_some() {
                BTreeSet::new()
            } else {
                current.features.union(&added[node]).cloned().collect()
            },
            host: current.host,
            deps,
        });
        ids[node] = Some(id);
        Ok(id)
    }
}

// ---------------------------------------------------------------------------
// Causes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Category {
    /// The divergent crate's own features differ.
    Own,
    /// A workspace dependency's own features differ.
    WorkspaceDependency,
    /// A third-party dependency's features differ.
    ThirdParty,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Category::Own => "own features",
            Category::WorkspaceDependency => "workspace dependency",
            Category::ThirdParty => "third-party",
        }
    }
}

/// The root of one divergence: the deepest crate whose own resolved features
/// differ between two configurations, not explained by anything beneath it.
///
/// A crate whose dependency *set* differs with identical features (a platform
/// or optional-edge difference Cargo resolved elsewhere) is reported with empty
/// `added`/`removed`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Cause {
    category: Category,
    #[serde(rename = "crate")]
    krate: String,
    version: String,
    host: bool,
    added: Vec<String>,
    removed: Vec<String>,
}

impl Cause {
    fn flag(&self) -> String {
        let mut parts = Vec::new();
        parts.extend(self.added.iter().map(|feature| format!("+{feature}")));
        parts.extend(self.removed.iter().map(|feature| format!("-{feature}")));
        if parts.is_empty() {
            parts.push("dependency set".to_owned());
        }
        let side = if self.host { " (host)" } else { "" };
        format!("{} {}{side}", self.krate, parts.join(" "))
    }
}

/// Roots of the difference between configurations `a` and `b` of one package.
///
/// Every workspace root is reported as [`Category::WorkspaceDependency`]; the
/// caller knows which crate is being compared and relabels its own root, which
/// keeps the memo valid across crates.
fn diff(
    interner: &Interner,
    workspace: &BTreeSet<PackageKey>,
    a: usize,
    b: usize,
    memo: &mut HashMap<(usize, usize), BTreeSet<Cause>>,
) -> BTreeSet<Cause> {
    if a == b {
        return BTreeSet::new();
    }
    if let Some(found) = memo.get(&(a, b)) {
        return found.clone();
    }
    let left = &interner.configs[a];
    let right = &interner.configs[b];
    let mut causes = BTreeSet::new();

    let by_package = |config: &ConfigKey| -> BTreeMap<(PackageKey, bool), usize> {
        config
            .deps
            .iter()
            .map(|&dep| {
                let key = &interner.configs[dep];
                ((key.package.clone(), key.host), dep)
            })
            .collect()
    };
    let left_deps = by_package(left);
    let right_deps = by_package(right);
    let unpaired = left_deps.keys().any(|key| !right_deps.contains_key(key))
        || right_deps.keys().any(|key| !left_deps.contains_key(key));

    if left.features != right.features || unpaired {
        let category = if workspace.contains(&left.package) {
            Category::WorkspaceDependency
        } else {
            Category::ThirdParty
        };
        causes.insert(Cause {
            category,
            krate: left.package.name.clone(),
            version: left.package.version.clone(),
            host: left.host,
            added: left.features.difference(&right.features).cloned().collect(),
            removed: right.features.difference(&left.features).cloned().collect(),
        });
    }
    for (key, &dep) in &left_deps {
        if let Some(&other) = right_deps.get(key) {
            causes.extend(diff(interner, workspace, dep, other, memo));
        }
    }

    memo.insert((a, b), causes.clone());
    causes
}

// ---------------------------------------------------------------------------
// Attribution
// ---------------------------------------------------------------------------

/// One Cargo invocation the owner performs for an owner package.
#[derive(Debug, Clone)]
struct Invocation {
    owner: String,
    label: String,
    tree: Tree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigReport {
    features: Vec<String>,
    host: bool,
    /// The owner whose archive first builds this configuration.
    first_owner: String,
    /// Every owner whose invocations use it.
    owners: Vec<String>,
    /// For every configuration beyond a crate's first: the index of the
    /// earlier configuration it is nearest to (fewest root causes), which is
    /// what `causes` are measured against.
    compared_with: Option<usize>,
    causes: Vec<Cause>,
    /// Compile seconds of this configuration's library and build-script units,
    /// when a timed pass was joined. Under `--align`, those of the as-built
    /// compile that provides it.
    seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrateReport {
    #[serde(rename = "crate")]
    krate: String,
    version: String,
    configurations: Vec<ConfigReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RankedCause {
    #[serde(flatten)]
    cause: Cause,
    /// Configurations beyond a crate's first that list this cause.
    configurations: usize,
    crates: Vec<String>,
    /// Seconds charged to this cause. A configuration with several causes
    /// splits its seconds evenly between them, so the column sums to the
    /// divergent total.
    seconds: f64,
    /// Seconds of the configurations this is the *only* cause of: what
    /// removing this cause alone is certain to save.
    sole_seconds: f64,
    /// Seconds of every configuration this cause takes part in: the most
    /// removing it could save, reached only if its co-causes go too.
    involved_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Variant {
    features: Vec<String>,
    owners: Vec<String>,
}

/// A third-party `(crate, version, side)` the selection resolves with more than
/// one feature set.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThirdPartyDivergence {
    #[serde(rename = "crate")]
    krate: String,
    version: String,
    host: bool,
    variants: Vec<Variant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OwnerReport {
    owner: String,
    features: Vec<String>,
    invocations: Vec<String>,
    /// Workspace-crate configurations this owner builds first.
    new_configurations: usize,
}

/// Workspace units a timed pass compiled that attribution says the owner did
/// not need to build — evidence the model and the build disagree.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Unexplained {
    owner: String,
    #[serde(rename = "crate")]
    krate: String,
    seconds: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Totals {
    workspace_crates: usize,
    configurations: usize,
    /// Configurations beyond each crate's first.
    divergent_configurations: usize,
    /// Divergent configurations whose causes are all third-party, all
    /// workspace-own (own or workspace-dependency), or both.
    divergent_by_origin: BTreeMap<String, usize>,
    seconds_first: Option<f64>,
    seconds_divergent: Option<f64>,
    /// Under `--align`: the timed seconds of compiles the aligned model no
    /// longer needs.
    seconds_removed: Option<f64>,
    /// Divergent seconds split by cause category.
    seconds_by_category: BTreeMap<Category, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Report {
    schema_version: u32,
    target: String,
    /// The third-party crates the `--align` what-if resolved identically for
    /// every owner; empty for the as-is build.
    aligned: Vec<String>,
    owners: Vec<OwnerReport>,
    totals: Totals,
    causes: Vec<RankedCause>,
    crates: Vec<CrateReport>,
    third_party_divergence: Vec<ThirdPartyDivergence>,
    unexplained_compiles: Vec<Unexplained>,
}

fn attribute(
    target: &str,
    owners: &[(String, Vec<String>)],
    invocations: &[Invocation],
    workspace: &BTreeSet<PackageKey>,
    tables: &FeatureTables,
    compiles: Option<&[Compile]>,
    aligned: &BTreeSet<String>,
) -> Result<Report> {
    // The timed pass built the as-is graph, so its compiles are matched
    // against that model; the report itself describes the `--align` one.
    let mut as_is = Interner::default();
    let mut interner = Interner::with_alignment(aligned, invocations, workspace, tables)?;
    // Per workspace package: configurations in first-built order, with owners.
    let mut per_crate: BTreeMap<PackageKey, Vec<(usize, String, BTreeSet<String>)>> =
        BTreeMap::new();
    let mut new_by_owner: BTreeMap<(String, PackageKey), Vec<usize>> = BTreeMap::new();
    let mut third_party: BTreeMap<(PackageKey, bool), BTreeMap<BTreeSet<String>, BTreeSet<String>>> =
        BTreeMap::new();
    let mut compiled: HashSet<usize> = HashSet::new();
    let mut built: Vec<Built> = Vec::new();

    for invocation in invocations {
        let (as_is_ids, _) = as_is.resolve(&invocation.tree)?;
        let (ids, reachable) = interner.resolve(&invocation.tree)?;
        for node in reachable {
            let tree_node = &invocation.tree.nodes[node];
            let id = ids[node];
            if !workspace.contains(&tree_node.package) {
                // The root's own display is never compared: it is the package
                // under test, not a dependency whose features could diverge.
                if node != 0 {
                    third_party
                        .entry((tree_node.package.clone(), tree_node.host))
                        .or_default()
                        .entry(tree_node.features.clone())
                        .or_default()
                        .insert(invocation.owner.clone());
                }
                continue;
            }
            let seen = per_crate.entry(tree_node.package.clone()).or_default();
            let provides = match seen.iter_mut().find(|(config, _, _)| *config == id) {
                Some((_, _, users)) => {
                    users.insert(invocation.owner.clone());
                    None
                }
                None => {
                    let position = seen.len();
                    seen.push((id, invocation.owner.clone(), BTreeSet::from([invocation.owner.clone()])));
                    new_by_owner
                        .entry((invocation.owner.clone(), tree_node.package.clone()))
                        .or_default()
                        .push(position);
                    Some((tree_node.package.clone(), position))
                }
            };
            // Alignment only merges configurations, so a configuration new to
            // the aligned model is always a new as-is compile too.
            if compiled.insert(as_is_ids[node]) {
                built.push(Built {
                    owner: invocation.owner.clone(),
                    package: tree_node.package.clone(),
                    host: tree_node.host,
                    provides,
                });
            }
        }
    }

    // Seconds per configuration: the measured seconds of the as-is compile
    // that provides it. Under `--align`, a compile that provides nothing is
    // one the alignment removes.
    let mut seconds: BTreeMap<(PackageKey, usize), f64> = BTreeMap::new();
    let mut unexplained = Vec::new();
    let mut removed = 0.0;
    if let Some(compiles) = compiles {
        let (spent, missing) = join(compiles, &built);
        unexplained = missing;
        for (entry, spent) in built.iter().zip(spent) {
            match &entry.provides {
                Some(config) => *seconds.entry(config.clone()).or_default() += spent,
                None => removed += spent,
            }
        }
    }

    let mut crates = Vec::new();
    let mut ranked: BTreeMap<Cause, RankedCause> = BTreeMap::new();
    let mut totals = Totals::default();
    let mut memo = HashMap::new();
    let mut first_seconds = 0.0;
    let mut divergent_seconds = 0.0;

    for (package, configs) in &per_crate {
        totals.workspace_crates += 1;
        let mut reports = Vec::new();
        for (position, (id, first_owner, users)) in configs.iter().enumerate() {
            totals.configurations += 1;
            let key = &interner.configs[*id];
            let spent = compiles.map(|_| seconds.get(&(package.clone(), position)).copied().unwrap_or(0.0));
            let (compared_with, causes) = if position == 0 {
                first_seconds += spent.unwrap_or(0.0);
                (None, Vec::new())
            } else {
                // Nearest earlier configuration: fewest roots, earliest on a tie.
                let (nearest, causes) = (0..position)
                    .map(|earlier| {
                        let causes: BTreeSet<Cause> = diff(&interner, workspace, *id, configs[earlier].0, &mut memo)
                            .into_iter()
                            .map(|mut cause| {
                                if cause.krate == package.name && cause.version == package.version {
                                    cause.category = Category::Own;
                                }
                                cause
                            })
                            .collect();
                        (earlier, causes)
                    })
                    .min_by_key(|(earlier, causes)| (causes.len(), *earlier))
                    .expect("position > 0 has an earlier configuration");
                totals.divergent_configurations += 1;
                let third = causes.iter().filter(|cause| cause.category == Category::ThirdParty).count();
                let origin = if third == causes.len() {
                    "third-party"
                } else if third == 0 {
                    "workspace-own"
                } else {
                    "mixed"
                };
                *totals.divergent_by_origin.entry(origin.to_owned()).or_default() += 1;
                let share = spent.unwrap_or(0.0) / causes.len().max(1) as f64;
                divergent_seconds += spent.unwrap_or(0.0);
                for cause in &causes {
                    let entry = ranked.entry(cause.clone()).or_insert_with(|| RankedCause {
                        cause: cause.clone(),
                        configurations: 0,
                        crates: Vec::new(),
                        seconds: 0.0,
                        sole_seconds: 0.0,
                        involved_seconds: 0.0,
                    });
                    entry.configurations += 1;
                    if !entry.crates.contains(&package.name) {
                        entry.crates.push(package.name.clone());
                    }
                    entry.seconds += share;
                    entry.involved_seconds += spent.unwrap_or(0.0);
                    if causes.len() == 1 {
                        entry.sole_seconds += spent.unwrap_or(0.0);
                    }
                    *totals.seconds_by_category.entry(cause.category).or_default() += share;
                }
                (Some(nearest), causes.into_iter().collect())
            };
            reports.push(ConfigReport {
                features: key.features.iter().cloned().collect(),
                host: key.host,
                first_owner: first_owner.clone(),
                owners: users.iter().cloned().collect(),
                compared_with,
                causes,
                seconds: spent,
            });
        }
        crates.push(CrateReport {
            krate: package.name.clone(),
            version: package.version.clone(),
            configurations: reports,
        });
    }
    if compiles.is_some() {
        totals.seconds_first = Some(first_seconds);
        totals.seconds_divergent = Some(divergent_seconds);
        totals.seconds_removed = (!aligned.is_empty()).then_some(removed);
    }

    let mut causes: Vec<RankedCause> = ranked.into_values().collect();
    causes.sort_by(|left, right| {
        right
            .seconds
            .total_cmp(&left.seconds)
            .then(right.configurations.cmp(&left.configurations))
            .then(left.cause.cmp(&right.cause))
    });

    let third_party_divergence = third_party
        .into_iter()
        .filter(|(_, variants)| variants.len() > 1)
        .map(|((package, host), variants)| ThirdPartyDivergence {
            krate: package.name,
            version: package.version,
            host,
            variants: variants
                .into_iter()
                .map(|(features, owners)| Variant {
                    features: features.into_iter().collect(),
                    owners: owners.into_iter().collect(),
                })
                .collect(),
        })
        .collect();

    let owners = owners
        .iter()
        .map(|(owner, features)| OwnerReport {
            owner: owner.clone(),
            features: features.clone(),
            invocations: invocations
                .iter()
                .filter(|invocation| &invocation.owner == owner)
                .map(|invocation| invocation.label.clone())
                .collect(),
            new_configurations: new_by_owner
                .iter()
                .filter(|((built_owner, _), _)| built_owner == owner)
                .map(|(_, positions)| positions.len())
                .sum(),
        })
        .collect();

    Ok(Report {
        schema_version: REPORT_SCHEMA_VERSION,
        target: target.to_owned(),
        aligned: aligned.iter().cloned().collect(),
        owners,
        totals,
        causes,
        crates,
        third_party_divergence,
        unexplained_compiles: unexplained,
    })
}

// ---------------------------------------------------------------------------
// Timed-pass events
// ---------------------------------------------------------------------------

/// The fields of a `ci-build` wrapper event this join reads.
#[derive(Debug, Deserialize)]
struct Event {
    package: String,
    #[serde(default)]
    crate_name: Option<String>,
    #[serde(default)]
    cargo_package: Option<String>,
    #[serde(default)]
    crate_types: Vec<String>,
    /// rustc's `--target`. The owner passes an explicit `--target`, so Cargo
    /// passes it to rustc for target-side units and omits it for host-side
    /// ones (proc-macros, build scripts, build dependencies).
    #[serde(default)]
    target: Option<String>,
    probe: bool,
    started_ms: u64,
    duration_ms: u64,
}

/// One workspace library or build-script compile a timed pass recorded.
#[derive(Debug, Clone, PartialEq)]
struct Compile {
    /// The owner package whose invocation ran it.
    owner: String,
    package: String,
    host: bool,
    build_script: bool,
    started_ms: u64,
    seconds: f64,
}

/// Every library and build-script compile of a workspace package in a timed
/// pass, in start order.
///
/// Test harnesses (no `--crate-type`) and binaries belong to the owner package
/// itself and exist once per owner whatever the features, so they are never
/// divergence and are left out.
fn read_compiles(dir: &Path, workspace: &BTreeSet<PackageKey>) -> Result<Vec<Compile>> {
    let names: BTreeSet<&str> = workspace.iter().map(|package| package.name.as_str()).collect();
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    // Ties on the millisecond keep the directory's own (sorted) order.
    paths.sort();
    let mut compiles = Vec::new();
    for path in paths {
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let event: Event =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let Some(package) = event.cargo_package.as_deref() else {
            continue;
        };
        if event.probe || !names.contains(package) {
            continue;
        }
        let build_script = event
            .crate_name
            .as_deref()
            .is_some_and(|name| name.starts_with("build_script_"));
        let library = event.crate_types.iter().any(|kind| LIBRARY_KINDS.contains(&kind.as_str()));
        if !(build_script || library) {
            continue;
        }
        compiles.push(Compile {
            owner: event.package,
            package: package.to_owned(),
            host: event.target.is_none(),
            build_script,
            started_ms: event.started_ms,
            seconds: event.duration_ms as f64 / 1000.0,
        });
    }
    compiles.sort_by_key(|compile| compile.started_ms);
    Ok(compiles)
}

/// A workspace library compile the as-is model says an owner performs.
#[derive(Debug, Clone)]
struct Built {
    owner: String,
    package: PackageKey,
    host: bool,
    /// The reported configuration (crate, position) this compile provides, or
    /// `None` when the `--align` model no longer needs it.
    provides: Option<(PackageKey, usize)>,
}

/// Charge every recorded compile to the modeled compile it performed.
///
/// Returns seconds per entry of `built`, and the recorded compiles the model
/// does not explain. An owner's Cargo invocations run one after another, and
/// one invocation compiles a `(package, side)` at most once, so the k-th
/// library compile an owner records for a `(package, side)` is the k-th one
/// `built` lists for it. A build script is charged with the next library
/// compile of its package by the same owner, which is the compile that reads
/// its output.
fn join(compiles: &[Compile], built: &[Built]) -> (Vec<f64>, Vec<Unexplained>) {
    let mut slots: HashMap<(&str, &str, bool), VecDeque<usize>> = HashMap::new();
    for (index, entry) in built.iter().enumerate() {
        slots
            .entry((entry.owner.as_str(), entry.package.name.as_str(), entry.host))
            .or_default()
            .push_back(index);
    }
    let mut spent = vec![0.0; built.len()];
    let mut unexplained: BTreeMap<(String, String), f64> = BTreeMap::new();
    // For each library compile, the `built` entry it matched.
    let mut matched: Vec<Option<usize>> = vec![None; compiles.len()];
    for (index, compile) in compiles.iter().enumerate().filter(|(_, compile)| !compile.build_script) {
        matched[index] = slots
            .get_mut(&(compile.owner.as_str(), compile.package.as_str(), compile.host))
            .and_then(|queue| queue.pop_front());
    }
    for (index, compile) in compiles.iter().enumerate() {
        let charged = if compile.build_script {
            compiles
                .iter()
                .enumerate()
                .skip(index + 1)
                .find(|(_, library)| {
                    !library.build_script && library.owner == compile.owner && library.package == compile.package
                })
                .and_then(|(library, _)| matched[library])
        } else {
            matched[index]
        };
        match charged {
            Some(entry) => spent[entry] += compile.seconds,
            None => {
                *unexplained
                    .entry((compile.owner.clone(), compile.package.clone()))
                    .or_default() += compile.seconds;
            }
        }
    }
    let unexplained = unexplained
        .into_iter()
        .map(|((owner, krate), seconds)| Unexplained { owner, krate, seconds })
        .collect();
    (spent, unexplained)
}

// ---------------------------------------------------------------------------
// Workspace inputs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SidecarTable {
    sidecars: BTreeMap<String, Sidecar>,
}

#[derive(Debug, Deserialize)]
struct Sidecar {
    package: String,
    #[serde(default)]
    features: Vec<String>,
}

/// What the producer reads for one owner package from its manifest.
#[derive(Debug, Clone, Default)]
struct OwnerSpec {
    features: Vec<String>,
    sidecars: Vec<String>,
}

fn string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|value| value.as_array())
        .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// The workspace crates the model tracks, every workspace package's owner
/// spec, and the feature tables of every package in the resolve.
///
/// A package with neither a library nor a build script (bin- or test-only)
/// is left out of the crate set: no other package can depend on it, so it only
/// ever appears as an owner's graph root, and the root's own display is never
/// a third-party row either. Its archive compiles only binaries and test
/// harnesses, which are never divergence, so counting it as a configuration
/// would report a compile that does not happen.
fn read_workspace(
    workspace: &Path,
) -> Result<(BTreeSet<PackageKey>, BTreeMap<String, OwnerSpec>, FeatureTables)> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(workspace.join("Cargo.toml"))
        .other_options(vec!["--locked".to_owned()])
        .exec()
        .context("running cargo metadata")?;
    let tables = metadata
        .packages
        .iter()
        .map(|package| {
            let features = PackageFeatures {
                features: package.features.clone().into_iter().collect(),
                deps: package
                    .dependencies
                    .iter()
                    .map(|dep| (dep.name.clone(), dep.rename.clone().unwrap_or_else(|| dep.name.clone())))
                    .collect(),
            };
            ((package.name.to_string(), package.version.to_string()), features)
        })
        .collect();
    let mut members = BTreeSet::new();
    let mut specs = BTreeMap::new();
    for package in metadata.workspace_packages() {
        let directory = package
            .manifest_path
            .parent()
            .context("a manifest path has a directory")?;
        let compiles_a_configuration = package.targets.iter().any(|target| {
            target.is_custom_build() || target.kind.iter().any(|kind| LIBRARY_KINDS.contains(&kind.as_str()))
        });
        if compiles_a_configuration {
            members.insert(PackageKey {
                name: package.name.to_string(),
                version: package.version.to_string(),
                source: directory.to_string(),
            });
        }
        let tests = package.metadata.pointer("/ci/tests");
        specs.insert(
            package.name.to_string(),
            OwnerSpec {
                features: string_list(tests.and_then(|tests| tests.get("features"))),
                sidecars: string_list(tests.and_then(|tests| tests.get("sidecars"))),
            },
        );
    }
    Ok((members, specs, tables))
}

fn cargo_tree(workspace: &Path, target: &str, package: &str, features: &[String], dev: bool) -> Result<Tree> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut command = Command::new(cargo);
    command
        .current_dir(workspace)
        .args(["tree", "--locked", "--color", "never", "--charset", "ascii"])
        .args(["--format", "{p}|{f}", "--target", target, "--package", package])
        .args(["--edges", if dev { "normal,build,dev" } else { "normal,build" }]);
    if !features.is_empty() {
        command.args(["--features", &features.join(",")]);
    }
    let output = command.output().context("running cargo tree")?;
    if !output.status.success() {
        bail!(
            "cargo tree --package {package} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    parse_tree(&String::from_utf8_lossy(&output.stdout))
        .with_context(|| format!("parsing cargo tree for {package}"))
}

fn host_triple() -> Result<String> {
    let output = Command::new("rustc").arg("-vV").output().context("running rustc -vV")?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .context("rustc -vV reported no host triple")
}

/// Everything `attribute` needs about one owner selection.
struct Selection {
    /// Workspace packages with a library or build script (see
    /// [`read_workspace`]).
    members: BTreeSet<PackageKey>,
    tables: FeatureTables,
    /// Each owner and its CI features, in the producer's order.
    owners: Vec<(String, Vec<String>)>,
    invocations: Vec<Invocation>,
}

/// Resolve every invocation the owner performs for `owners`, in the
/// producer's order: each package's archive, then its sidecar builds.
fn collect(
    workspace: &Path,
    target: &str,
    owners: &[String],
    sidecars: Option<&SidecarTable>,
) -> Result<Selection> {
    let (members, specs, tables) = read_workspace(workspace)?;
    let mut ordered: Vec<String> = owners.to_vec();
    ordered.sort();
    ordered.dedup();

    let mut owner_features = Vec::new();
    let mut invocations = Vec::new();
    for owner in &ordered {
        let spec = specs
            .get(owner)
            .with_context(|| format!("'{owner}' is not a workspace package"))?;
        owner_features.push((owner.clone(), spec.features.clone()));
        invocations.push(Invocation {
            owner: owner.clone(),
            label: "archive".to_owned(),
            tree: cargo_tree(workspace, target, owner, &spec.features, true)?,
        });
        for name in &spec.sidecars {
            let sidecar = sidecars
                .and_then(|table| table.sidecars.get(name))
                .with_context(|| format!("sidecar '{name}' declared by {owner} is not in the sidecar table"))?;
            invocations.push(Invocation {
                owner: owner.clone(),
                label: format!("sidecar {name}"),
                tree: cargo_tree(workspace, target, &sidecar.package, &sidecar.features, false)?,
            });
        }
    }
    Ok(Selection {
        members,
        tables,
        owners: owner_features,
        invocations,
    })
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn seconds_text(seconds: Option<f64>) -> String {
    seconds.map_or_else(|| "–".to_owned(), |seconds| format!("{seconds:.1}s"))
}

fn render(report: &Report, term: &Terminal) -> String {
    let mut out = String::new();
    let totals = &report.totals;
    out.push_str(
        &Prose::new(format!(
            "**Feature attribution** for {} owner(s) on `{}` — {} workspace crate(s) in {} \
             configuration(s), {} of them beyond a crate's first.",
            report.owners.len(),
            report.target,
            totals.workspace_crates,
            totals.configurations,
            totals.divergent_configurations,
        ))
        .render(term),
    );
    out.push('\n');
    if !report.aligned.is_empty() {
        out.push_str(
            &Prose::new(format!(
                "_What-if: every owner resolves {} identically; {} of the timed pass's compile \
                 time would no longer be needed. Forwarded features are modeled; an optional \
                 dependency the alignment newly enables is not, so the collapse can be \
                 overstated._",
                report.aligned.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", "),
                seconds_text(totals.seconds_removed),
            ))
            .render(term),
        );
        out.push('\n');
    }
    if let (Some(first), Some(divergent)) = (totals.seconds_first, totals.seconds_divergent) {
        let by_category = totals
            .seconds_by_category
            .iter()
            .map(|(category, seconds)| format!("{} {seconds:.1}s", category.label()))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(
            &Prose::new(format!(
                "First configurations took {first:.1}s; divergent ones {divergent:.1}s \
                 ({by_category})."
            ))
            .render(term),
        );
        out.push('\n');
    }

    let owner_rows = report
        .owners
        .iter()
        .map(|owner| {
            vec![
                owner.owner.clone().into(),
                owner.features.join(",").into(),
                owner.invocations.len().to_string().into(),
                owner.new_configurations.to_string().into(),
            ]
        })
        .collect::<Vec<_>>();
    out.push_str(
        &Table::new()
            .with_columns(
                ["owner", "ci features", "invocations", "new configs"]
                    .into_iter()
                    .map(TableColumn::new)
                    .collect::<Vec<_>>(),
            )
            .with_data(owner_rows)
            .render(term),
    );
    out.push('\n');

    let timed = totals.seconds_divergent.is_some();
    let cause_rows = report
        .causes
        .iter()
        .map(|ranked| {
            vec![
                ranked.cause.flag().into(),
                ranked.cause.category.label().into(),
                ranked.configurations.to_string().into(),
                seconds_text(timed.then_some(ranked.seconds)).into(),
                seconds_text(timed.then_some(ranked.sole_seconds)).into(),
                seconds_text(timed.then_some(ranked.involved_seconds)).into(),
                ranked.crates.len().to_string().into(),
            ]
        })
        .collect::<Vec<_>>();
    out.push_str(
        &Table::new()
            .with_columns(vec![
                TableColumn::new("cause"),
                TableColumn::new("category"),
                TableColumn::new("configs"),
                TableColumn::new("seconds"),
                TableColumn::new("sole"),
                TableColumn::new("involved"),
                // The crate names are in `--json`; a list here wraps every row.
                TableColumn::new("crates"),
            ])
            .with_data(cause_rows)
            .render(term),
    );
    out.push('\n');

    let crate_rows = report
        .crates
        .iter()
        .flat_map(|krate| {
            krate.configurations.iter().enumerate().map(move |(index, config)| {
                let causes = config
                    .causes
                    .iter()
                    .map(Cause::flag)
                    .collect::<Vec<_>>()
                    .join("; ");
                vec![
                    krate.krate.clone().into(),
                    format!("{}{}", index + 1, if config.host { " (host)" } else { "" }).into(),
                    config.first_owner.clone().into(),
                    seconds_text(config.seconds).into(),
                    if index == 0 { "first".to_owned() } else { causes }.into(),
                ]
            })
        })
        .collect::<Vec<_>>();
    out.push_str(
        &Table::new()
            .with_columns(
                ["crate", "config", "first owner", "seconds", "cause"]
                    .into_iter()
                    .map(TableColumn::new)
                    .collect::<Vec<_>>(),
            )
            .with_data(crate_rows)
            .render(term),
    );
    out.push('\n');

    if !report.unexplained_compiles.is_empty() {
        out.push_str(
            &Prose::new(format!(
                "**{} compile(s) the model does not explain** — see `unexplained_compiles` \
                 in `--json`.",
                report.unexplained_compiles.len()
            ))
            .render(term),
        );
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Options {
    owners: Vec<String>,
    target: Option<String>,
    workspace: Option<PathBuf>,
    sidecars: Option<PathBuf>,
    events: Option<PathBuf>,
    json: Option<PathBuf>,
    align: BTreeSet<String>,
}

fn parse_args(args: &[String]) -> Result<Options> {
    let mut options = Options::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || iter.next().cloned().with_context(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--owners" => {
                options.owners = value()?
                    .split(',')
                    .filter(|owner| !owner.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            "--target" => options.target = Some(value()?),
            "--workspace" => options.workspace = Some(PathBuf::from(value()?)),
            "--sidecars" => options.sidecars = Some(PathBuf::from(value()?)),
            "--events" => options.events = Some(PathBuf::from(value()?)),
            "--json" => options.json = Some(PathBuf::from(value()?)),
            "--align" => options
                .align
                .extend(value()?.split(',').filter(|name| !name.is_empty()).map(str::to_owned)),
            "-h" | "--help" => bail!("{USAGE}"),
            other => bail!("unknown argument {other}\n{USAGE}"),
        }
    }
    if options.owners.is_empty() {
        bail!("--owners is required\n{USAGE}");
    }
    Ok(options)
}

fn run(options: Options, term: &Terminal) -> Result<()> {
    let workspace = options.workspace.unwrap_or_else(|| PathBuf::from("."));
    let target = match options.target {
        Some(target) => target,
        None => host_triple()?,
    };
    let sidecar_path = options
        .sidecars
        .unwrap_or_else(|| workspace.join(".github/ci/sidecars.json"));
    let sidecars = if sidecar_path.exists() {
        let text = fs::read_to_string(&sidecar_path)
            .with_context(|| format!("reading {}", sidecar_path.display()))?;
        Some(serde_json::from_str::<SidecarTable>(&text).context("parsing the sidecar table")?)
    } else {
        None
    };

    let selection = collect(&workspace, &target, &options.owners, sidecars.as_ref())?;
    let compiles = options
        .events
        .as_deref()
        .map(|dir| read_compiles(dir, &selection.members))
        .transpose()?;
    let report = attribute(
        &target,
        &selection.owners,
        &selection.invocations,
        &selection.members,
        &selection.tables,
        compiles.as_deref(),
        &options.align,
    )?;

    if let Some(path) = options.json {
        fs::write(&path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    print!("{}", render(&report, term));
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let term = Terminal::new();
    match parse_args(&args).and_then(|options| run(options, &term)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("feature-attribution: {error:#}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "feature-attribution-tests.rs"]
mod tests;
