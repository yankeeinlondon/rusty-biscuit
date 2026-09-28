# Renderer feasibility experiment

Date: 2026-09-27. Scope: clarification evidence for `2026-09-27-graph-merged-branch`, not production implementation or acceptance evidence for the completed fix.

## Result

The installed `mermaid-rs-renderer` 0.3.1 can render the tested topology with full labels on their exact commits and without tag collisions, using its existing public graph and layout interfaces. No dependency upgrade or source patch was needed. The current application integration does not yet expose this solution.

Two changes were tested in an isolated harness:

1. Measure the widest tag polygon in an initial layout, then set the existing `gitgraph.commit_step` configuration to at least that width plus 16 SVG units and render again. The 16-unit margin is an experimental choice, not a specified product constant.
2. Supply the verified second parent of the labeled merge in the public parsed graph before layout. The harness already knows that parent from its fixture; it does not discover Git ancestry or implement a general parser repair.

The dependency parser preserves merge IDs and tags but uses the entire merge statement suffix as its source-branch lookup. Thus `merge feature id: "MERGE" tag: "origin/main"` loses the `feature` parent. Providing the known second parent repairs that specific deficiency without moving labels to a following commit or changing the merge identity.

## Executed cases

| Case | Result |
|---|---|
| Two adjacent commits with long full reference labels, defaults | One overlapping pair of tag bounding boxes; commit step 34 |
| Same input with measured spacing | Zero overlapping pairs; commit step 246.66406; both labels remain on their original commit objects |
| Labeled merge using unmodified parser output | Merge ID and both tags survive, but parents contain only `PARENT`; this is a structural failure even though rendering succeeds |
| Same merge with explicit known second parent and measured spacing | Parents are exactly `PARENT`, `TIP`; both `origin/main` and `main-at-merge` remain on `MERGE`; following commit still points to `MERGE`; zero tag overlaps |
| Old fork and merge with compressed gaps | Eight nodes, including three omission markers totaling 8,997 omitted commits; fork and merge identities retained; correct two-parent merge; zero tag overlaps |

[Exact results](renderer-spike/results.txt) include parent lists, tags, dimensions, and tag bounds. SVG output was generated for all five cases. The three successful candidate PNGs were opened through the image inspection tool and visually checked: long labels are separate, the branch visibly joins at the labeled merge, and compressed history keeps both endpoints.

- [Adjacent labels](renderer-spike/adjacent-spaced.png)
- [Labeled merge](renderer-spike/merge-repaired.png)
- [Compressed history](renderer-spike/old-compressed-repaired.png)

The harness asserts tag membership on the corresponding commit object before checking layout bounds. This proves attachment in the tested graph model; the rendered images provide a separate visual check. Same-commit tags stack without overlapping in these fixtures.

## Integration boundary and recommendation

Prefer a narrow existing-dependency integration over an immediate renderer fork. `biscuit-visualized` already calls the public parse, layout, and SVG functions independently in `src/src/mermaid/render.rs`, but it currently uses default layout configuration and the unmodified parsed graph. Its existing parse/layout boundary is one feasible integration point for backend-specific adjustments; this experiment does not determine the best production ownership boundary. `biscuit-terminal` should remain responsible for lane semantics, merge destinations, preserved connection anchors, compression, and terminal fitting. `worktree-cli` should supply verified Git ancestry.

A concrete implementation plan must choose how typed, verified merge information reaches the visualization backend, and ensure measurement and rendering use exactly the same adjusted graph and spacing. The hard-coded fixture repair is not a production API proposal. The initial experimental recommendation was to opt in through a narrow GitGraph path. The subsequent human ruling supersedes that restriction: design for greatest reuse, and place shared improvements in `biscuit-terminal` or `biscuit-visualized` according to the actual integration seams. Reusable label layout and merge correctness should serve applicable callers broadly, with regression coverage for existing consumers; the ruling does not assign every rendering correction to `biscuit-visualized`. This is ownership guidance, not permission to expand the supported Git histories. The shared component needs a way to represent merges and protect their endpoints during compression; names and exact public contract remain planning details.

One candidate considered by the experiment is a typed GitGraph rendering request carrying the existing diagram plus verified merge records (destination commit ID and expected parent IDs). This is not a confirmed API or a requirement for caller-specific opt-in. The component maps full Git object IDs to its unique emitted IDs; the backend locates those existing nodes, validates the metadata, and supplies the exact expected parents before layout. This candidate avoids a second Mermaid parser. Under the final reuse-first ruling, planning should choose the parser integration or typed graph input that best serves shared callers; broadly useful rendering corrections belong at the appropriate shared component or backend boundary. Invalid metadata must fail explicitly rather than silently fabricate an edge. The same request and layout policy must feed both measurement and rendering, and artifact cache identity must include the additional inputs. A direct typed graph input is an alternative worth considering during planning if it fits the existing backend more simply. Neither contract was implemented by this spike; the harness supplies one fixture-known parent directly. The final ownership ruling does not change the measured results or establish that this candidate is the best production design.

Global spacing is simple and worked here, but increased the adjacent-label natural width from about 279 to 632 SVG units. Existing terminal shrinking therefore matters. The compressed successful example was about 1,391 units wide. More compact collision-aware placement could improve density but is not required by this experiment or established as necessary.

## Limits

- Synthetic rendering fixtures only; no real-Git ancestry discovery, shallow-history fallback, or actual component trimming was implemented. The compressed fixture supplies omission markers directly. It proves renderability of preserved anchors, not correctness of the future gathering/compression algorithm.
- Bounding checks cover tag polygons against other tag polygons. The fixtures include tags on two lanes, but do not establish general cross-lane collision freedom or every possible collision involving lane labels, commit captions, arbitrary fonts, or many stacked labels. Earlier partial/repeated merge reconstruction remains outside the agreed scope.
- Default Mermaid light theme on this macOS host only. No cross-OS, alternate-theme, terminal-protocol, narrow final-raster, clipping-after-fitting, or production performance evidence was produced.
- No GUI window was opened or focused. No production sources, repository lockfile, dependencies, or installed registry sources were changed.
- Initial harness iterations corrected an inappropriate assertion that the asymmetric tag polygon center equals its commit coordinate and corrected expected tags after adding the multiple-label case. The final run succeeded; these were harness corrections, not production fixes.
- The biscuit-visualized skill still describes renderer v0.2; actual installed/source dependency evidence for this experiment is 0.3.1. This experiment follows the code, not that stale version description.

## Reproduction

The retained [harness](renderer-spike/spike.rs) embeds all input fixtures. The retained lockfile fixes the exploratory project's resolved dependencies; it does not modify the monorepo lockfile. Run from the repository root:

```sh
spike_dir=$(mktemp -d /tmp/wt-graph-spike.XXXXXX)
mkdir "$spike_dir/src"
cp worktree/fixes/2026-09-27-graph-merged-branch/renderer-spike/spike.rs "$spike_dir/src/main.rs"
cp worktree/fixes/2026-09-27-graph-merged-branch/renderer-spike/Cargo.lock "$spike_dir/Cargo.lock"
cat > "$spike_dir/Cargo.toml" <<'TOML'
[package]
name = "wt-graph-renderer-spike"
version = "0.0.0"
edition = "2024"
[dependencies]
mermaid-rs-renderer = { version = "=0.3.1", default-features = false, features = ["png"] }
[workspace]
TOML
cargo run --offline --locked --manifest-path "$spike_dir/Cargo.toml" -- "$spike_dir/output"
```

Executed final command during this experiment: `cargo run --offline --manifest-path /tmp/wt-graph-spike.X4PSGW/Cargo.toml -- /tmp/wt-graph-spike.X4PSGW/output` (exit 0). This runs a standalone assertion harness; no package test suite was claimed or required for this preparatory experiment.
