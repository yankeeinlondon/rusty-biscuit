# Faster `sniff repo packages` Specification

Optimize the `sniff repo packages` execution path so it is fast **and consistent under disk contention**. The command is normally ~100ms warm, but it issues ~21,000 filesystem syscalls per invocation, so under concurrent disk load it can balloon past 1–2s. This was surfaced by Claudine: the `prompts/commit.md` composition runs `::shell sniff repo packages` while subagents commit **in parallel** (plus any background `cargo build`), and the resulting contention pushed the command past the 10s compose-shell timeout, producing an intermittent and baffling `ShellExpansionError: command timed out`.

## Current Performance

Measured in a linked git worktree of the rusty-biscuit monorepo (~70 Cargo members, ~1,521 walked directories):

| Condition | Wall time |
|---|---|
| Main checkout, warm cache | ~0.15s |
| Linked worktree, warm cache | ~0.10–0.13s |
| Linked worktree, back-to-back (cache churn) | 0.4–1.8s, highly variable |
| Under concurrent disk load (e.g. `find target/ &`) | spikes well past 1.8s |

`sniff --version` is ~0.00s, and `--perf` reports ~100ms of internal work — so process startup is **not** a factor. The cost is I/O, and it is hypersensitive to contention.

## Current Hot Path

```
cli/src/commands/repo.rs                 sniff/lib/src/filesystem/repo/...
────────────────────────                 ─────────────────────────────────
handle_repo_packages()             ──►  detect_repo_structure_or_root_package()
  (structure_only = true)                 └── detect_repo_inner_with_shared()
                                              ├── detect_cargo_workspace()         ← CHEAP, glob-based
                                              │     reads root Cargo.toml members
                                              │     (explicit paths, no wildcards)
                                              └── discover_nested_workspace_outcomes()
                                                    └── walk_for_nested_markers()   ← DOMINANT COST
```

`detect_cargo_workspace` (`cargo.rs:15`) is already cheap: it reads the root `Cargo.toml`, takes `workspace.members` (explicit paths in this repo — no glob traversal), and expands them. The waste is entirely in the **nested-workspace** scan that runs afterward.

## Identified Bottleneck

### B1: Per-directory syscall storm in `walk_for_nested_markers`

`walk_for_nested_markers` (`sniff/lib/src/filesystem/repo/nested.rs:216`) walks the **entire repo tree** (~1,521 directories here, honoring `.gitignore` and skipping `node_modules`/`target`/`dist`/`build`). For **every directory** it performs:

- ~13 × `path.join(marker.file).exists()` stat calls — one per `NESTED_MARKERS` entry (`nested.rs:244–252`); **and**
- one full `std::fs::read_dir(path)` to look for `.sln`/`.slnx` solution files (`nested.rs:255–266`).

That is **~21,000 syscalls per invocation** (~1,521 × (13 + 1)). Warm cache it totals ~100ms; under the parallel-`git commit` disk contention the `commit` operation itself generates, each syscall can stall and the aggregate spikes past the 10s compose-shell ceiling.

For this repo the scan finds **nothing**: the root is a `ForbidsNested` Cargo workspace with no nested JS/.NET sub-workspaces. The entire walk is pure overhead on every call.

**Impact:** the dominant cost (~100ms warm; the source of the multi-second contention spikes).

## Proposed Optimization

### O1: Single-pass entry inspection

Rewrite `walk_for_nested_markers` to detect markers from the **filenames the walker already yields**, instead of re-probing the filesystem per directory.

Current consume loop (`nested.rs:233–267`) filters the walker to *directories only*, then probes each directory. The new loop:

1. Iterate **all** walker entries (files included). Keep the existing `WalkBuilder` settings unchanged: `hidden(false)`, `git_ignore(true)`, `git_global(true)`, `git_exclude(true)`, and the `filter_entry` directory pruning (`node_modules`/`target`/`dist`/`build` via `should_skip_directory_name`).
2. For each **file** entry:
   - Resolve `parent`; skip when `parent == root` (nested discovery is non-root only — preserves the current `path == root` skip).
   - If `file_name` matches a `NESTED_MARKERS[*].file`, insert that mapping's standards into `by_root[parent]`.
   - If `file_name` ends with `SOLUTION_SUFFIX` (`.sln`) or `.slnx`, insert `MonorepoStandard::DotNetSolution` into `by_root[parent]`.
3. Build the returned `Vec<Candidate>` from `by_root` exactly as today.

This collapses ~21,000 redundant syscalls into the one efficient `ignore`-crate walk (batched `readdir`, no extra per-directory `stat`/`read_dir`), with marker matching done in memory on already-yielded entries.

## Target Performance

| Condition | Current | Target | Optimization |
|---|---|---|---|
| Worktree, warm cache | ~100ms | ~10–20ms | O1 |
| Worktree, back-to-back | 0.4–1.8s | stable, low | O1 |
| Under concurrent disk load | spikes >1.8s | resilient (single walk) | O1 |

## Scope / Blast Radius

- One function in `nested.rs`. The signature and the `Candidate` return shape are unchanged, so **no callers change** (`discover_nested_workspace_outcomes` and below are untouched).
- `detect_cargo_workspace` / `expand_membership_globs` are already cheap and are **not** modified.
- Cross-platform safe: only the *mechanism* of inspection changes (filename matching of yielded entries). The `.sln`/`.slnx` suffix match is preserved.

## Intentional Behavior Change (review-worthy)

Today the per-directory `exists()` probe finds a marker file **even if it is gitignored**, because `exists()` bypasses the walker's `git_ignore` filter. The single-pass version only sees walker-yielded files, so a *gitignored* marker (e.g. a gitignored `package.json`) inside a non-gitignored directory would no longer register a nested candidate.

Marker files (`package.json`, `pnpm-workspace.yaml`, `pom.xml`, `*.sln`, …) are conventionally committed, so the risk is negligible. Fully preserving the old semantics would require disabling `git_ignore`, which re-explodes the walk into `node_modules` and defeats the optimization. The change should be noted in the commit body.

## Tests (L1, nextest — `just test` in `sniff/`)

- **Keep green:** existing nested-detection tests (a nested pnpm/npm/.NET workspace under a Cargo root still produces its layer).
- **Add:** fixture with a deep non-root `package.json` **and** a non-root `*.sln` → assert both candidates/standards are detected via the new path.
- **Add:** a marker placed directly in `root` is ignored (the `parent == root` skip).
- **Add (prune guard):** a `node_modules/` directory containing a `package.json` is **not** detected (confirms `filter_entry` pruning still applies).
- `just lint` clean.

## Verification

Before/after in a worktree under synthetic load:

```
( find /…/target -type f >/dev/null & ); time sniff repo packages
```

Expect the post-fix wall time to stay low and stable where today it spikes to 1–2s+. Also confirm `sniff repo packages` output is byte-identical before and after on this repo (and on a fixture repo that *does* contain a nested workspace).

## Constraints

- **Public API stability:** `detect_repo_structure`, `detect_repo_structure_or_root_package`, and the `RepoInfo`/package output shape are unchanged.
- **Correctness:** the detected nested-workspace forest must be identical to today's for any repo whose markers are committed (the only delta is the gitignored-marker edge case above).
- **Cross-platform:** must work on macOS, Linux, and Windows.
- **No new dependencies:** use the existing `ignore` crate walk.

## Out of Scope

- **Cross-invocation caching** of the package list (a persisted, root-`Cargo.toml`-keyed cache). Would make repeats near-instant and fully contention-immune, but adds invalidation complexity — a separate feature.
- **Claudine compose-shell timeout inconsistency.** The compose path uses a 10s shell-expansion timeout (`claudine prepare.rs`, via Darkmatter's `ComposeOptions` default) while the harness path uses 30s (`claudine harness/shell.rs`). Worth aligning to 30s as defense-in-depth, but it lives in the `claudine` package area and only raises the ceiling — it does not address this root cause. Track separately.
