---
blast_radius:
  - biscuit-terminal/lib/Cargo.toml
  - biscuit-terminal/lib/src/components/mod.rs
  - biscuit-terminal/lib/src/components/git_graph/mod.rs
  - biscuit-terminal/lib/src/components/git_graph/builder.rs
  - biscuit-terminal/lib/src/components/git_graph/auto.rs
  - biscuit-terminal/lib/src/components/git_graph/browser.rs
  - biscuit-terminal/lib/src/components/git_graph/sanitize.rs
  - biscuit-terminal/lib/src/components/git_graph/collect.rs
  - biscuit-terminal/lib/src/components/git_graph/tests.rs
  - biscuit-terminal/lib/README.md
  - biscuit-terminal/cli/Cargo.toml
  - biscuit-terminal/cli/src/commands/git_graph.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/src/commands/git_graph.rs
reference:
  - biscuit-terminal/lib/src/components/mermaid.rs
  - biscuit-terminal/lib/src/components/renderable.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/browser.rs
  - worktree/cli/src/commands/git_graph.rs
  - worktree/cli/src/commands/list.rs
---

# `GitGraph` Component Specification

## Summary

Promote the inline git graph view currently implemented inside the `worktree` CLI
into a reusable, composable component on `biscuit-terminal`:

- `biscuit-terminal` owns a new `GitGraph` component in its `components` module.
- `GitGraph` implements [`Renderable`] (terminal output) and
  [`BrowserRenderable`] (HTML/SVG output) so the same value renders identically
  in terminals and in browser-facing tools such as `darkmatter`.
- The component is ergonomic for callers that already speak the MermaidJS
  `gitGraph` DSL — strings, fragments, or typed builders all map onto the same
  underlying Mermaid instruction set.
- A separate auto-configuration builder accepts repository state (current
  branch, default branch, worktree branches, recent commits) and produces a
  fully-configured `GitGraph` using the same heuristics that the `worktree`
  CLI uses today.
- This work explicitly **does not** add a git graph view to `sniff`. The
  previously-proposed `sniff repo git-status` integration is rescinded.

The end result is one component that `worktree`, `darkmatter`, and any future
caller can use to render branch topology — manually, semi-manually, or fully
auto-configured from git state.

## Goals

- Provide a single `GitGraph` component that renders branch topology consistently
  in both terminals (via Mermaid → image) and browsers (via Mermaid → SVG/HTML).
- Accept MermaidJS `gitGraph` input directly so callers that already know the
  DSL can wrap a string in a builder and render it.
- Offer a typed builder API for callers that prefer not to assemble raw Mermaid
  strings.
- Offer an auto-configuration builder that mirrors the worktree CLI's current
  scenario selection (focused branch vs base overview), commit caps, and
  placeholder rules.
- Keep the **default** build of `biscuit-terminal` free of any direct
  dependency on `git2` or on shelling out to `git`. The auto builder operates
  on caller-supplied data.
- Offer an **opt-in** companion path (Cargo feature `git`) that collects the
  required git state by shelling out to the system `git` binary, so that
  ergonomic callers — including the `biscuit-terminal` CLI itself — can render
  a graph from a path without re-implementing data collection.

## Non-Goals

- Do not modify `sniff` library or CLI in any way.
- Do not add a **mandatory** git data-collection layer to `biscuit-terminal`.
  The component's core path consumes structured input. The opt-in `git`
  feature is additive — disabled by default and gated so callers that do not
  want any git dependency pay zero cost.
- Do not pull in `git2`. The companion collector shells out to the system
  `git` binary via `std::process::Command`, matching what `worktree` does
  today and adding no new compile-time dependency.
- Do not attempt to render the full git DAG. The simplified two-lane rendering
  is preserved; commit collection still follows full reachability rather than
  filtering to first parents.
- Do not change the public API of `MermaidDiagram`. `GitGraph` may delegate to
  it but does not replace it.
- Do not introduce network calls or subprocess calls inside the component.

## Reference Behavior From `worktree`

The graph behavior being lifted out of the CLI has these user-facing semantics:

- Inline Mermaid `gitGraph` rendered through `biscuit-terminal`.
- Two scenarios:
  - **Focused branch** when the current checkout is a non-default branch:
    - up to 2 shared context commits ending at the selected merge-base
    - up to 5 commits unique to the feature tip (not reachable from the
      default tip)
    - up to 5 commits unique to the default tip (not reachable from the
      feature tip)
  - **Base overview** when the current checkout is the default branch:
    - up to 10 recent commits on the default branch
    - each active worktree branch anchored at its selected merge-base
    - up to 10 commits unique to each worktree branch tip (not reachable from
      the default tip)
- If a branch has no commits unique to its tip, emit a placeholder `HEAD`
  commit so the fork is still visible.
- Width chosen from commit-count thresholds (see [Width Policy](#width-policy)).
- Auto rendering suppressed when terminal width is narrower than 80 columns.
- A verbose text variant lists the selected merge-base commit on the default
  branch and the full oldest-first sequence of commits unique to the current
  branch tip.

These semantics survive — they move from `worktree`'s `git_graph.rs` into the
auto-configuration builder on the new component.

## Component Design

### Module Location

```
biscuit-terminal/lib/src/components/git_graph/
├── mod.rs         # GitGraph struct, Renderable impl, public API
├── builder.rs     # Typed builder API + Mermaid DSL ingestion
├── auto.rs        # Auto-configuration from caller-supplied git state
├── browser.rs     # BrowserRenderable impl
├── sanitize.rs    # Mermaid-safe branch identifier generation
└── tests.rs
```

Register it in `biscuit-terminal/lib/src/components/mod.rs`:

```rust
pub mod git_graph;

pub use git_graph::{GitGraph, GitGraphBuilder, GitGraphCommit, GitGraphBranch};
```

### Public Types

```rust
/// A composable git topology component.
///
/// A `GitGraph` is conceptually a structured form of a MermaidJS `gitGraph`
/// diagram, plus the metadata needed to render it sensibly to a terminal
/// (width policy, image fallback, verbose-detail block).
///
/// Construct via [`GitGraph::from_mermaid`], [`GitGraph::builder`], or
/// [`GitGraph::auto`]. All three converge on the same internal representation.
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub(crate) instructions: String,             // resolved Mermaid `gitGraph` text
    pub(crate) commit_count: usize,
    pub(crate) width: Option<ImageWidth>,        // override; None = use heuristic
    pub(crate) title: Option<String>,
    pub(crate) verbose_detail: Option<VerboseDetail>,
    pub(crate) layout: Layout,
}

/// Scenario hint surfaced for callers that want to branch on shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitGraphKind {
    FocusedBranch,
    BaseOverview,
    Manual,
}

/// One commit in the graph.
#[derive(Debug, Clone)]
pub struct GitGraphCommit {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub refs: Vec<String>,
}

/// One branch in the graph.
#[derive(Debug, Clone)]
pub struct GitGraphBranch {
    pub name: String,                  // displayed branch name
    pub mermaid_id: String,            // sanitized id used inside the diagram
    pub anchor_index: usize,           // selected merge-base position in mainline
    pub commits: Vec<GitGraphCommit>,  // commits unique to this tip, oldest first
    pub hidden_commits: usize,         // earlier unique commits omitted from view
    pub placeholder_head: bool,        // true => no unique commits; emit `HEAD`
}

/// Optional verbose-detail block printed under the graph in `-v` modes.
#[derive(Debug, Clone)]
pub struct VerboseDetail {
    pub merge_base: Option<GitGraphCommit>, // selected shared-context endpoint
    pub branch_commits: Vec<GitGraphCommit>, // full unique sequence, oldest first
}
```

### Three Construction Paths

#### 1. Raw Mermaid input

The most ergonomic path for callers that already have a `gitGraph` block:

```rust
let graph = GitGraph::from_mermaid(r#"
gitGraph
    commit id: "abc1234"
    commit id: "def5678"
    branch feature/x
    checkout feature/x
    commit id: "f00ba12"
"#);

print!("{}", graph.display(&Terminal::default()));
```

`from_mermaid` accepts anything `Into<String>`. It does not parse or validate
the diagram — invalid Mermaid surfaces as a Mermaid render error at render
time, identical to [`MermaidDiagram`](crate::components::mermaid::MermaidDiagram).
A best-effort commit count is computed for width-policy purposes by counting
lines matching `\s*commit\b` and `\s*branch\b`; that count is approximate but
sufficient for the width heuristic.

#### 2. Typed builder

For callers that prefer not to splice strings:

```rust
let graph = GitGraph::builder()
    .default_branch("main")
    .mainline_commits([c1, c2, c3])
    .branch(
        GitGraphBranch::new("feature/x")
            .anchor_at(2)
            .commits([f1, f2])
    )
    .build();
```

The builder owns Mermaid emission. Callers never write Mermaid syntax through
this path. Branch identifiers are run through the sanitizer (see
[Mermaid-Safe Identifiers](#mermaid-safe-identifiers)) so user-supplied branch
names like `feature/x` map to valid Mermaid IDs like `feature_x` while the
display name is preserved on the resulting [`GitGraphBranch`].

#### 3. Auto-configuration (caller-supplied data — always available)

For callers like the `worktree` CLI that have repository state on hand:

```rust
let graph = GitGraph::auto(GitGraphAuto {
    kind_hint: None,                        // None => infer from state
    current_branch: Some("feature/x".into()),
    default_branch: "main".into(),
    mainline: recent_main_commits,          // overview mainline, oldest first
    focused_context: shared_context,        // ends at selected merge-base
    focused_default_unique: default_unique, // not reachable from current tip
    focused_default_hidden: default_hidden, // omitted default-unique commits
    branches: worktree_branches,            // commits already unique vs default
    merge_base: Some(merge_base_commit),    // anchor/detail only
    caps: GitGraphCaps::default(),
})?;
```

`GitGraphCaps` carries the same numbers used by the worktree CLI today:

```rust
pub struct GitGraphCaps {
    pub focused_context_commits: usize,     // default 2
    pub focused_branch_commits: usize,      // default 5
    pub focused_default_commits: usize,     // default 5
    pub overview_default_commits: usize,    // default 10
    pub overview_branch_commits: usize,     // default 10
}
```

The auto builder applies the same scenario rules described in
[Reference Behavior From `worktree`](#reference-behavior-from-worktree),
trims input to the caps, and emits a `GitGraph` whose `instructions` are
identical in structure to what `worktree`'s current code produces. Crucially,
the auto builder takes pre-collected data — it does not open a repository or
derive lane membership from the merge-base. Focused default commits are
already filtered against the feature tip, and every branch's `commits` are
already filtered against the default tip. The selected merge-base is only the
shared-context endpoint, fork anchor, and verbose merge-base detail. This keeps
the default build of `biscuit-terminal` free of any git dependency and lets
every caller choose its own git access path (`git2`, `git` subprocess, mock
data in tests) without making merge-base selection part of lane membership.
Collectors also supply each lane's count of earlier omitted unique commits;
the auto builder renders that value directly as `+N` and never derives it from
the selected merge-base.

#### 4. Auto-from-repo (opt-in via `git` feature)

Many callers — including the `biscuit-terminal` CLI itself — only have a
path and want a rendered graph. Requiring each of them to re-implement git
data collection would push the same `git_command` boilerplate into every
consumer. To keep the library ergonomic without forcing a dependency on
graph-free callers, ship a thin collector behind a Cargo feature:

```toml
# biscuit-terminal/lib/Cargo.toml
[features]
default = []
git = []   # enables GitGraph::from_repo and friends; no new deps
```

The collector lives at `components::git_graph::collect` and is compiled in
only when the `git` feature is enabled. It uses `std::process::Command` to
shell out to the system `git` binary — there is no `git2` link, no new
runtime dependency, no measurable compile-time cost on the default build.

Public surface added behind the feature:

```rust
#[cfg(feature = "git")]
impl GitGraph {
    /// Auto-build a `GitGraph` by inspecting the repository at `path`.
    ///
    /// Shells out to the system `git` binary to enumerate the current
    /// branch, default branch, recent commits, tip-unique commits, worktree
    /// branches, and selected merge-base. The collected data is then fed through the same
    /// [`GitGraph::auto`] path, so output is identical to what a caller
    /// supplying the same data manually would produce.
    ///
    /// ## Errors
    /// Returns [`GitGraphCollectError`] when `git` is not on `$PATH`, when
    /// the path is not inside a git working tree, or when a required ref
    /// cannot be resolved.
    pub fn from_repo(
        path: impl AsRef<std::path::Path>,
        caps: GitGraphCaps,
    ) -> Result<Option<GitGraph>, GitGraphCollectError>;
}
```

The feature only exists so that callers who *want* a one-liner can have
one. The default-data path (`GitGraph::auto`) remains the canonical entry
point and is fully tested without the feature enabled.

##### Use inside the `biscuit-terminal` CLI

The `biscuit-terminal` CLI enables the `git` feature in its own
`Cargo.toml` and exposes a subcommand (e.g. `biscuit-terminal git-graph
[PATH]`) that:

1. resolves the path argument (default: current directory);
2. calls `GitGraph::from_repo(path, caps)`;
3. prints the rendered graph via `graph.display(&Terminal::default())`.

This gives `biscuit-terminal` users a way to view any repository's branch
topology directly from the binary, and serves as a living example for
downstream consumers of the library API.

##### Use inside the `worktree` CLI

`worktree` is the more complex consumer: it already has its own
`git_command` plumbing, custom error type, and policy logic that decides
*when* a graph should appear (focused vs. overview, narrow-terminal
suppression, verbose detail toggling). It therefore stays on the
**caller-supplied** `GitGraph::auto` path and continues to own its data
collection. The opt-in feature is for callers that do not have those
needs and want a one-call ergonomic entry point.

### Renderable Implementation

`GitGraph` implements [`Renderable`] by delegating to an internal
[`MermaidDiagram`] configured from its own state:

```rust
impl Renderable for GitGraph {
    fn render(&self, term: &Terminal) -> String {
        let mermaid = MermaidDiagram::new(&self.instructions)
            .with_width(self.resolved_width(term))
            .with_layout(self.layout.clone());

        let body = match mermaid.try_render(term) {
            Ok(result) => result.output,
            Err(_) => mermaid.fallback_code_block(),
        };

        let mut out = String::new();
        if let Some(title) = &self.title {
            out.push_str(&format!("{title}\n"));
        }
        out.push_str(&body);
        if let Some(detail) = &self.verbose_detail {
            out.push('\n');
            out.push_str(&detail.render(term));
        }
        self.layout.apply_layout(&out, term.width())
    }

    fn is_block_level(&self) -> bool { true }

    fn layout(&self) -> &Layout { &self.layout }
    fn layout_mut(&mut self) -> &mut Layout { &mut self.layout }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
```

Composition rules:

- `GitGraph` is block-level, like [`MermaidDiagram`].
- Width selection happens inside the component — see [Width Policy](#width-policy).
- Verbose detail is opt-in via `GitGraph::with_verbose_detail(...)` and is only
  populated by the auto builder when the focused-branch scenario applies.

### BrowserRenderable Implementation

```rust
impl BrowserRenderable for GitGraph {
    fn render_to_browser(&self) -> String { ... }
    fn render_to_browser_with_inline_variables(
        &self,
        variables: &HashMap<String, String>,
    ) -> String { ... }
    fn as_any(&self) -> &dyn std::any::Any { self }
}
```

The browser path goes through `biscuit-visualized`'s Mermaid SVG output (the
same pipeline that produces the cached PNG used for terminal rendering). The
emitted markup wraps the SVG in a `<figure>` with a sensible class and inline
CSS variables so callers like `darkmatter` can theme it. Variables:

- `--gitgraph-bg` — fill behind the diagram (default `transparent`)
- `--gitgraph-fg` — foreground/text color
- `--gitgraph-max-width` — outer width cap (default `100%`)

Patterns mirror [`HorizontalRule`'s `BrowserRenderable`
implementation](biscuit-terminal/lib/src/components/horizontal_rule/browser.rs):
declare variables on the root element with concrete fallbacks, and let
`render_to_browser_with_inline_variables` perform `var(--name)` substitution.

When SVG generation fails (missing system dependency, malformed Mermaid),
the browser path falls back to a `<pre><code class="language-mermaid">` block
so the source is still visible — the analogue of the terminal fenced-code
fallback.

## Width Policy

The width heuristic moves verbatim from worktree:

| Commits In Diagram | Width                                                       |
|--------------------|-------------------------------------------------------------|
| 1-4                | `60ch`                                                      |
| 5-8                | `80ch`                                                      |
| 9-15               | `120ch` if terminal width > 120, otherwise `100%`           |
| 16+                | `160ch` if terminal width >= 160, otherwise `100%`          |

Resolution order inside `resolved_width(term)`:

1. If the caller set an explicit `with_width(spec)`, use it verbatim.
2. Otherwise, apply the table above using `self.commit_count`.
3. Width parsing uses
   [`biscuit_terminal::components::terminal_image::parse_width_spec`](biscuit-terminal/lib/src/components/terminal_image/width.rs).

When the terminal width is narrower than 80 columns, `GitGraph::render` returns
the Mermaid fallback code block instead of attempting an inline image. This
preserves the "auto-suppress on narrow terminal" rule from worktree.

## Mermaid-Safe Identifiers

Raw branch names cannot be emitted directly as Mermaid `branch` identifiers
because Mermaid rejects `/`, `.`, and other punctuation found in real branch
names. The sanitizer rules are unchanged from the previous design:

- Allow only `[A-Za-z0-9_]`.
- Replace every other character with `_`.
- Collapse repeated `_`.
- Prefix with `b_` when the first character is numeric.
- Add numeric suffixes on collision within a single diagram.

The original branch name is preserved on `GitGraphBranch::name` and used for
the verbose-detail block. Only the `mermaid_id` is sanitized.

## Verbose Detail

The auto builder populates `VerboseDetail` only when:

- the scenario is `GitGraphKind::FocusedBranch`, and
- the caller passed `GitGraphAuto { verbose: true, .. }`.

`VerboseDetail::render(term)` prints two short sections under the graph:

1. The selected merge-base commit on the default branch.
2. The full sequence of commits unique to the current branch tip, oldest
   first.

Formatting reuses `worktree`'s current conventional-commit detection and
relative-day policy. That code moves into `biscuit-terminal` as a thin helper
inside `git_graph/auto.rs`, since the component already needs it.

## Mermaid Construction Algorithms

The auto builder retains the two algorithms from worktree's current
implementation:

### Focused Branch Graph

1. Take up to `focused_context_commits` from the caller-supplied
   shared context ending at the selected merge-base, oldest first.
2. Take up to `focused_branch_commits` from the caller-supplied commits unique
   to `current_branch` (not reachable from `default_branch`), oldest first.
3. Take up to `focused_default_commits` from the caller-supplied commits unique
   to `default_branch` (not reachable from `current_branch`), oldest first.
4. Before each lane's displayed commits, emit its supplied nonzero
   `hidden_commits` as a fork-adjacent `+N` marker.
5. Emit:
   ```
   gitGraph
       commit id: "ctx1"
       commit id: "ctx2"
       branch feature_x
       checkout feature_x
       commit id: "f1"
       commit id: "f2"
       checkout main
       commit id: "m1"
       commit id: "m2"
   ```
6. If the current branch has no commits unique to its tip, emit a single
   `commit id: "HEAD"` placeholder under the feature-branch checkout so the
   selected merge-base anchor remains visible. The placeholder is synthetic
   and is never counted as a unique commit.

### Base Overview Graph

1. Take up to `overview_default_commits` mainline commits oldest first.
2. For each worktree branch:
   - resolve its selected merge-base position within the mainline window
   - take up to `overview_branch_commits` caller-supplied commits unique to the
     branch tip (not reachable from the default tip), oldest first
   - emit the branch's supplied nonzero `hidden_commits` as a fork-adjacent
     `+N` marker before its displayed commits
   - if no unique commits exist, mark `placeholder_head = true`
3. If the selected merge-base falls outside the displayed mainline window,
   anchor that branch at index `0`.
4. Sort branches by `(anchor_index, name)`.
5. Emit the mainline commits, inserting each branch block at its anchor index.

These rules are identical to the current `worktree` behavior — they have
simply moved into a reusable, testable module. A collector must obtain branch
commits with the equivalent of `git log <branch> --not <default> --` and its
hidden count with `git rev-list --count <branch> --not <default> --` minus the
displayed count. For the focused default lane, both queries reverse the two
tips. Plain `git merge-base` may choose any one of multiple best bases, so its
result must never be used as the exclusion revision for lane commits, hidden
counts, or verbose branch details.

## Consumer Migration: `worktree` CLI

The existing worktree CLI code in `worktree/cli/src/commands/git_graph.rs`
becomes a **thin adapter**:

- it continues to be responsible for invoking `git` and assembling
  `GitGraphCommit` / `GitGraphBranch` values from the result;
- it constructs a `GitGraph` via `GitGraph::auto(...)`;
- it calls `graph.display(&term)` for rendering.

All Mermaid-emission logic (`worktree_graph`, `base_graph`, `BranchData`) is
deleted from worktree once the component is in place. The verbose-detail
helpers (`merge_base_commit`, `branch_commits_detail`, `format_commit`) are
deleted as well — the component owns that formatting now.

`worktree/cli/src/commands/list.rs` continues to choose when the graph is
shown; it is the policy layer. The component is the renderer.

## Consumer Migration: `darkmatter`

Once `GitGraph` implements `BrowserRenderable`, `darkmatter` can include git
graphs in HTML output without bespoke handling. The intended integration is:

- a Markdown extension or fenced code-block handler recognizes
  ```` ```gitgraph ```` blocks;
- the block content is fed to `GitGraph::from_mermaid(...)`;
- the value is rendered through `BrowserRenderable::render_to_browser()`.

This integration is out of scope for the initial component landing, but the
component must be shaped to make it ergonomic when it does land.

## Edge Cases

Handle these explicitly inside the component or its auto builder:

- empty `instructions` after construction: `try_render` returns the fallback
  code block (effectively empty), matching `MermaidDiagram` behavior;
- terminal width < 80 cols: emit fenced Mermaid code block rather than image;
- inline image support unavailable: emit fenced Mermaid code block;
- branch with zero tip-unique commits: emit a synthetic `HEAD` placeholder;
- zero hidden unique commits: omit the `+N` marker; otherwise render the exact
  supplied count once, immediately before the lane's displayed commits;
- selected merge-base older than displayed mainline window: anchor at index 0;
- multiple incomparable best merge-bases: lane membership and verbose branch
  details remain unchanged whichever base Git selects; only shared context and
  anchor placement may follow the selected base;
- collision between sanitized branch ids: numeric suffix on collision;
- invalid `ImageWidth` override: fall back to the commit-count heuristic;
- caller passes no branches in base-overview auto mode: returns `Ok(None)` from
  `GitGraph::auto`, signalling the caller to skip rendering;
- caller passes branches but no mainline: returns `Ok(None)`;
- **branch name collides with a working-tree path** (e.g. a `darkmatter`
  branch in a repo that also has a `darkmatter/` directory): every `git log`
  invocation that takes a revision argument MUST terminate its arg list with
  a `--` separator. Without it, git aborts with `fatal: ambiguous argument
  '<branch>': both revision and filename`, the call returns `Err`, the
  caller swallows it to `vec![]`, and the graph silently degrades to the
  `HEAD` placeholder. This applies to the opt-in `git` feature collector
  and to any caller-owned git plumbing (including the `worktree` CLI
  adapter).

## Performance Requirements

- The default build does no subprocess work — `GitGraph::auto` is pure data
  in, structured output out.
- No network calls in any build.
- No filesystem traversal beyond what `MermaidDiagram` already does for its
  PNG cache.
- Rendering cost is dominated by Mermaid → PNG conversion, which is already
  cached by `biscuit-visualized`.
- Auto-builder cost is `O(n)` in the supplied commits — there is no traversal,
  only filtering and capping.
- The opt-in `git` feature adds subprocess calls only when explicitly enabled,
  and only when `from_repo` is invoked. Expected cost there is comparable to
  what `worktree` already does today: one `git symbolic-ref`, a few
  `git rev-parse` / `git merge-base` / `git log` invocations per branch,
  capped by `GitGraphCaps`.

## Test Plan

### Unit Tests — Default Build (`biscuit-terminal/lib/src/components/git_graph/tests.rs`)

- `from_mermaid` round-trips its input as `instructions`.
- Typed builder emits the same Mermaid for an equivalent focused-branch shape
  as the raw Mermaid string would.
- Sanitizer rules: punctuation replacement, leading-digit prefix, collision
  numbering.
- Auto builder:
  - focused-branch scenario renders disjoint caller-supplied tip-unique lanes
    and the documented shared context
  - base-overview scenario renders only branch-tip-unique commits and inserts
    branches at the selected merge-base anchor indices
  - each lane's `+N` marker equals its supplied omitted-unique count and is
    fork-adjacent before the displayed unique commits
  - branch with zero tip-unique commits emits the synthetic `HEAD` placeholder
  - selected merge-base older than the displayed window anchors at index `0`
  - changing which incomparable best merge-base is selected does not change
    lane membership, hidden counts, or verbose branch details
  - returns `Ok(None)` when input data is insufficient
- Width policy: each row of the table resolves to the expected `ImageWidth`.
- `is_block_level()` returns `true`.

### Renderable Tests

- `render(term)` on a narrow terminal returns a fenced Mermaid block.
- `render(term)` on a wide terminal returns non-empty output (best-effort,
  may be skipped in CI without Mermaid deps, matching existing
  `MermaidDiagram` tests).

### BrowserRenderable Tests

- `render_to_browser` includes `<svg ...>` (or the fenced fallback) and the
  declared CSS variables.
- `render_to_browser_with_inline_variables` substitutes `var(--gitgraph-fg)`
  and `var(--gitgraph-bg)`.

### Feature-Gated Tests (`#[cfg(feature = "git")]`)

- `GitGraph::from_repo` against a temporary git repository with two linked
  worktrees produces a non-empty graph and the expected scenario kind.
- `GitGraph::from_repo` on a non-repository path returns
  `GitGraphCollectError`, not a panic.
- Output of `from_repo` is byte-identical to a manually-constructed
  `GitGraph::auto` call given the same caps and equivalent data.

### Consumer Integration

- A worktree CLI integration test exercises the focused-branch and
  base-overview paths through the component, asserting on the emitted
  Mermaid instructions string (not on rendered terminal bytes).
- A `biscuit-terminal` CLI smoke test exercises the new `git-graph`
  subcommand against a temporary repository (enabled only when the CLI
  builds with the `git` feature, which it does by default).

## Documentation Updates Required At Implementation Time

- `biscuit-terminal/lib/README.md` — list `GitGraph` alongside `MermaidDiagram`
  and `HorizontalRule`.
- `biscuit-terminal/docs/` — add a short component page if other components
  have one, or extend the existing components reference.
- `worktree/cli/README.md` — note that the graph now lives in
  `biscuit-terminal` and that the CLI is a consumer.

## Implementation Checklist

1. Add the `components::git_graph` module to `biscuit-terminal`.
2. Implement `GitGraph`, the typed builder, and `from_mermaid` ingestion.
3. Implement the auto builder and port the worktree algorithms.
4. Implement `Renderable`, including width policy and narrow-terminal fallback.
5. Implement `BrowserRenderable`, including CSS-variable substitution.
6. Add the `git` Cargo feature and the gated `GitGraph::from_repo` collector.
7. Add unit and integration tests for both the default and `git`-enabled builds.
8. Migrate `worktree/cli/src/commands/git_graph.rs` to a thin adapter that
   calls `GitGraph::auto`.
9. Delete the now-dead Mermaid-emission code in `worktree`.
10. Enable the `git` feature in the `biscuit-terminal` CLI and add the
    `git-graph` subcommand.
11. Update READMEs.

## Decisions Locked By This Spec

- The git graph component lives in `biscuit-terminal::components::git_graph`,
  not in `sniff` and not in `worktree`.
- `GitGraph` implements both `Renderable` and `BrowserRenderable`.
- The component accepts MermaidJS `gitGraph` strings, a typed builder, and an
  auto-configuration entry point — all three converge on one struct.
- The default build never opens a git repository. Callers supply structured
  data via `GitGraph::auto`.
- An opt-in `git` Cargo feature ships a `GitGraph::from_repo` companion that
  shells out to the system `git` binary. The feature adds no new compile-time
  dependencies and the default build pays no cost for it.
- The `biscuit-terminal` CLI enables the `git` feature and exposes a
  user-facing subcommand built on `from_repo`.
- The `worktree` CLI stays on the caller-supplied `auto` path because it
  already has its own git plumbing, error type, and visibility policy.
- Simplified two-lane rendering, commit caps, placeholder `HEAD`, and the
  sanitizer rules from the worktree implementation are all preserved.
- The previously-proposed `sniff repo git-status` integration is rescinded.
