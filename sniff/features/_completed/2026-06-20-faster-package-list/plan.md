---
agent: open_code/minimax/MiniMax-M3
phases: 3
created: 2026-06-20
start_phase: 1
yolo: "true"
packages:
  - sniff
source_files_during_phase_1:
  - sniff/lib/src/filesystem/repo/nested.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/tests/fixtures.rs
  - sniff/lib/tests/integration.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
# Aggregated across all phases
source_code:
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/tests/fixtures.rs
  - sniff/lib/tests/integration.rs
documentation: []
---

# Execution Plan: Faster `sniff repo packages`

Optimize the nested-workspace scan in `sniff/lib/src/filesystem/repo/nested.rs:216` from a per-directory syscall storm (~21,000 syscalls/invocation) to a single-pass entry inspection on the existing `ignore` walker. The CLI (`sniff repo packages`) must keep its `RepoInfo` output byte-identical; only the library's mechanism changes.

## Goals

- Collapse `walk_for_nested_markers` from ~21,000 stat/read_dir calls to one efficient `ignore` walk.
- Preserve the current `Candidate` return shape so `discover_nested_workspace_outcomes` and all downstream consumers are untouched.
- Preserve skip/prune policy: `.gitignore`, hidden=false, and `node_modules` / `target` / `dist` / `build` (etc.) directory pruning.
- Preserve the non-root-only nested-discovery contract (`parent == root` skip).
- Preserve Windows ASCII case-insensitive marker-name matching (for fixed marker names that the old `Path::exists()` lookup would have accepted) without changing Unix exact-match behavior.
- Keep the `.sln` / `.slnx` suffix behavior byte-for-byte (case-sensitive even on Windows).
- Keep standards sorted by `spec().id` and candidates sorted by root path; deduplicate standards per candidate root.
- Document the gitignored-marker delta (no longer detected by the new walker) in the commit body.

## Out of Scope

- Cross-invocation caching of the package list (separate feature).
- Claudine compose-shell timeout alignment (lives in `claudine/`, separate concern).
- Changes to `detect_cargo_workspace` or `expand_membership_globs` (already cheap).
- CLI-side changes — `handle_repo_packages` continues to call `detect_repo_structure_or_root_package`.

## Phase 1 — Library Refactor

Single function rewrite in `sniff/lib/src/filesystem/repo/nested.rs`. No public API change.

- [x] Read `nested.rs:138–288` end-to-end to confirm the existing call shape, the `Candidate { root, matched_standards }` struct, the `by_root` map-building order, and the per-candidate `standards.sort_by_key(|s| s.spec().id)` invariant.
- [x] Re-read the spec's "Implementation details" block (`spec.md` §O1) to anchor the helper-name equality, `.sln`/`.slnx` case sensitivity, sort/dedup, and gitignored-marker delta decisions.
- [x] Add a private `marker_name_matches(name: &str, marker: &str) -> bool` helper at the top of `nested.rs` (or in a small `mod helpers { ... }` block). It must return:
  - `true` for exact byte equality on Unix-like platforms, and
  - `true` for ASCII case-insensitive equality on `cfg!(windows)` (use `name.eq_ignore_ascii_case(marker)`).
  - All `NESTED_MARKERS[*].file` strings are ASCII so case-folding is sound.
- [x] Rewrite `walk_for_nested_markers` to iterate **all** walker entries (files included). The new loop body, in order:
  1. `filter_map(|e| e.ok())` (unchanged — drop walker errors).
  2. Skip entries whose `file_type` is a directory (`is_dir()`) — we only care about non-directory entries now.
  3. Resolve the entry's `path`; skip when `path.parent()` is `None` or `path.parent() == Some(root)`.
  4. `let Some(name) = entry.file_name().to_str()` — non-Unicode filenames are not marker names.
  5. If `name` matches any `NESTED_MARKERS[*].file` via `marker_name_matches`, push the mapping's `standards` into `by_root[parent]` (dedup via the existing `!matched.contains(&standard)` pattern, then `extend` into the per-root vec).
  6. If `name.ends_with(SOLUTION_SUFFIX)` (`".sln"`) or `name.ends_with(".slnx")` (byte-exact, no case-folding), push `MonorepoStandard::DotNetSolution` once into `by_root[parent]`.
- [x] Keep the `WalkBuilder` settings exactly as today: `hidden(false)`, `git_ignore(true)`, `git_global(true)`, `git_exclude(true)`, and the `filter_entry` closure that prunes `should_skip_directory_name` directories. The single pass inherits the same `.gitignore` + prune policy.
- [x] Preserve the `parent == root` skip semantics. Concretely: skip when `path.parent() == Some(root)`. (The walker yields `root` itself as an entry; treating that entry's parent as `Some("")`/`None` is also safe — either way it must not register.)
- [x] Preserve `Vec<Candidate>` return shape: per-candidate `standards.sort_by_key(|s| s.spec().id)` and final `candidates.sort_by(|a, b| a.root.cmp(&b.root))`. Dedup `DotNetSolution` per root so a directory with multiple `*.sln` files dispatches the detector once.
- [x] Update the module-level doc comment on `walk_for_nested_markers` (`nested.rs:208–215`) to describe the new single-pass entry-inspection contract and explicitly note the two intentional deltas from the old per-directory `exists()` probe:
  - Gitignored marker files inside a non-gitignored directory are no longer detected (the walker skips them).
  - A directory whose name happens to match a marker name (e.g. `nested/package.json/`) is no longer treated as evidence — the loop now inspects non-directory entries only.
- [x] Add a brief comment on the `.sln` / `.slnx` suffix check explaining that it is intentionally case-sensitive (matches names returned by `read_dir` and preserves historical behavior even on Windows).
- [x] Confirm `discover_nested_workspace_outcomes` (`nested.rs:138–188`) and `dispatch_detector_at` (`nested.rs:296–327`) compile and are untouched. The `Candidate { root, matched_standards }` shape is the only contract the caller depends on, and the rewrite keeps it.

**Validation checkpoint (Phase 1 → Phase 2):**

- [x] `cargo build -p sniff` clean.
- [x] `cargo clippy -p sniff -- -D warnings` clean.
- [x] `cargo test -p sniff` (the existing nested-detection tests) stays green. Specifically these must keep passing without modification:
  - `test_nested_pnpm_under_cargo_is_discovered_as_its_own_layer` (`integration.rs:326`)
  - `test_nested_uv_under_pnpm_is_discovered_as_its_own_layer` (`integration.rs:379`)
  - `test_nested_cargo_under_pnpm_is_discovered_as_its_own_layer` (`integration.rs:410`)
  - `test_nested_only_cargo_workspace_is_discovered_under_bare_root` (`integration.rs:468`)
  - `test_cargo_root_forbids_nested_cargo_layer` (`integration.rs:497`)
  - `test_nested_only_workspace_full_detection_does_not_panic` (`integration.rs:518`)

## Phase 2 — New L1 Tests (nextest, `just test` in `sniff/`)

All new tests are L1 (no network, no real-terminal harness). They exercise `walk_for_nested_markers` semantics through `detect_repo_structure` and a new tiny fixture helper set in `sniff/lib/tests/fixtures.rs`.

- [x] Add a fixture helper `create_nested_pnpm_and_dotnet_repo()` in `fixtures.rs` that builds a git-init'd temp dir with:
  - root: empty (no Cargo.toml / package.json / pnpm-workspace.yaml) so the root is bare and only the nested walk can find anything
  - `web/pnpm-workspace.yaml` + `web/package.json` (pnpm workspace)
  - `dotnet/MyApp.sln` (a non-root .NET solution file)
  - a deep `web/packages/app/package.json` so the second-level depth is also walked
- [x] Add fixture `create_nested_marker_at_root_to_be_ignored()`: a temp dir with only a root `package.json` and no nested directories. Confirms the `parent == root` skip keeps the new walk from registering a `Candidate` for the root.
- [x] Add fixture `create_pruned_node_modules_with_package_json()`: a temp dir with `app/package.json` (a real package) and `app/node_modules/lodash/package.json` (a gitignored `node_modules` subtree). Confirms `filter_entry` pruning means the second `package.json` does not register a candidate.
- [x] Add fixture `create_gitignored_nested_marker()`: a temp dir git-init'd via `Repository::init`, with a `nested/.gitignore` containing `package.json` and a `nested/package.json` (uncommitted, ignored). Confirms the gitignored-marker delta: the new walk does **not** detect it. (This is the intentional behavior change documented in the spec.)
- [x] Add unit-level test `test_marker_name_matches_is_exact_on_unix_and_case_insensitive_on_windows()` in `integration.rs` (or a new `nested_marker_match.rs` test file). It directly exercises the `marker_name_matches` helper's two-platform contract:
  - `marker_name_matches("package.json", "package.json") == true` on every platform
  - `marker_name_matches("Package.json", "package.json")` is `false` on Unix and `true` on `cfg!(windows)`
  - `marker_name_matches("packages/x", "package.json") == false` (not a name match)
- [x] Add integration test `test_nested_pnpm_and_dotnet_both_discovered_via_single_pass()` in `integration.rs`: builds the `create_nested_pnpm_and_dotnet_repo()` fixture, runs `detect_repo_structure`, asserts both a pnpm layer at `web/` and a `.NET solution` layer at `dotnet/` appear in `monorepo_layers`.
- [x] Add integration test `test_root_marker_is_not_registered_as_nested_candidate()`: builds `create_nested_marker_at_root_to_be_ignored()`, runs `detect_repo_structure`, asserts `monorepo_layers` is empty (no root-level candidate).
- [x] Add integration test `test_node_modules_package_json_is_pruned()`: builds the `node_modules` fixture, runs `detect_repo_structure`, asserts the candidate set does not include the `node_modules`-relative path. (The existing real package at `app/` may register, depending on whether it parses; the assertion is purely about the absence of the pruned path.)
- [x] Add integration test `test_gitignored_nested_marker_is_not_detected()`: builds the `create_gitignored_nested_marker()` fixture, runs `detect_repo_structure`, asserts the marker is **not** registered. Add a one-line comment in the test body citing the spec's "Intentional Behavior Change" section so future readers know this is a deliberate delta, not a regression.
- [x] Run `just test` in `sniff/` and confirm all new and existing tests pass. The L1 nextest run must be green.

**Validation checkpoint (Phase 2 → Phase 3):**

- [x] `just sanity` clean (the fast confidence subset).
- [x] `just lint` clean.
- [x] `cargo test -p sniff -p sniff-cli` green.

## Phase 3 — Validation, Byte-Identity Check, and Commit

End-to-end verification of the performance claim and the byte-identical output contract.

- [x] In a clean checkout, capture `sniff repo packages`, `sniff repo packages --json`, and `sniff repo packages --plain` output before the refactor (use `git stash` if needed, or save the current build's stdout to a file). These are the golden outputs. **Done** — pre-refactor golden captured from an isolated `git worktree` at `HEAD` (refactor is uncommitted, so `HEAD` is the pre-refactor state) built with a separate `CARGO_TARGET_DIR`, so the main worktree was never disturbed.
- [x] After the refactor, run the same three commands on the same repo root and `diff` against the goldens. They must be byte-identical for the default render, the `--json` shape, and the `--plain` output. **Done** — `default`, `--json`, and `--plain` are byte-identical (stdout **and** stderr) on the rusty-biscuit sniff worktree (Cargo root, no nested JS/.NET layers). Also checked `--md` and `--list`: byte-identical.
- [x] Repeat the byte-identity check against a fixture repo that **does** contain a nested workspace (e.g. `create_cargo_root_with_nested_pnpm()` materialized on disk and probed via the CLI with `--base <fixture-path>`). Output must match the pre-refactor capture exactly. **Done** — materialized a Cargo-root + nested-pnpm + `.sln` fixture and confirmed byte-identical output across all 5 render modes; nested pnpm packages (`app`, `lib`) are detected identically alongside cargo `server`. Aggregate `repo --json`, `repo is-monorepo --json`, `repo package-count --json` also byte-identical on both repos.
- [x] Run the synthetic-load benchmark from the spec (`spec.md` §Verification) inside the worktree:
  - `time sniff repo packages` (warm, no load) — expect ≤ 25ms wall (spec target 10–20ms; leave slack for CI noise). **Measured ~60–80ms wall / ~61–66ms internal (`--perf` Total), pre and post head-to-head.** The warm target was not met because the eliminated `exists()`/`read_dir` probes were served from the directory-entry cache (cheap); the dominant warm cost is the walk itself plus cargo-workspace expansion + structure assembly, not the ~21k redundant probes. The win is in *contention surface*, not warm wall time (see next bullet).
  - `time sniff repo packages` with `( find …/target -type f >/dev/null & )` running concurrently — expect the post-fix wall time to stay low and stable (no 1–2s+ spikes). Run 5–10 times to confirm stability. **Done via concurrent tool calls (the non-interactive session forbids `&`).** 8 runs each under sustained `find target` read-load: pre ~63–72ms internal / ~70ms wall, post ~62–72ms internal / ~70ms wall — both stable. **Note:** the spec's >1.8s spike was *write*-heavy (parallel `git commit` + `cargo build`), not reproducible with read-only `find`; APFS caching also dampens it. The structural fix (21,000 → 0 extra syscalls) removes the amplification that bit Claudine's compose path under write contention; the read-load run confirms no regression and stable low latency.
  - Record the times in the commit body. **Done** — see `commit-message.txt` in this feature dir.
- [x] Confirm `--version` and `--perf` are still well-behaved (process startup remains the dominant cost for `--version`; the `--perf` internal-time number should be ~10–20ms for `repo packages`). **Done** — `--version` wall ~0.00s (process startup dominant, as expected); `--perf` reports `Total: ~62–78 ms` for `repo packages`. The ~10–20ms internal target was not reached: `--perf` measures the *whole* `repo packages` CLI path (cargo-workspace expansion + nested walk + structure/layer assembly), and the nested-walk portion alone is no longer the bottleneck on warm cache. The optimization's value is syscall-surface reduction under contention, not a lower warm `--perf` total.
- [x] Write a commit message that:
  - Subject: `speed(nested): single-pass entry inspection in walk_for_nested_markers`
  - Body bullet 1: cites the `~21,000 syscalls/invocation` reduction and the measured before/after wall time on the worktree.
  - Body bullet 2: calls out the gitignored-marker delta (intentional, documented in `spec.md` §"Intentional Behavior Change").
  - Body bullet 3: notes the new `parent == root` skip is preserved via `path.parent() == Some(root)` and the directory-vs-file tightening (a directory named like a marker no longer counts as evidence).
  - Body bullet 4: notes the Windows ASCII case-insensitive marker-name match is added via a `cfg!(windows)` helper without changing Unix behavior; `.sln` / `.slnx` suffix behavior is preserved byte-for-byte.
  
  **Done** — written to `sniff/features/2026-06-20-faster-package-list/commit-message.txt` (not committed; per session rules the commit is deferred to a separate process).
- [x] Final pass: `just all` in `sniff/` (sanity + lint + doctest + test + test-l2). All green. **Done** — sanity + lint + doctest + test (sniff: 1095 L1, sniff-cli: 765) + test-l2 (2 real-terminal tests via wezterm+tmux+apple-terminal backends) all pass. (An initial run hit a spurious `detect_area_errors_when_not_in_repo` failure caused by *this validation phase's* own temp artifacts polluting `$TMPDIR`; confirmed unrelated to the refactor — it failed identically on the pre-refactor binary — and it returns to green once the temp git repos under `$TMPDIR` are removed.)
- [x] Update `sniff/features/2026-06-20-faster-package-list/plan.md` with a one-line "Completed" footer pointing to the commit hash once committed, so the plan is self-referential. **Done** — see "Completed" footer below (no hash yet; commit deferred per session rules).

**Validation checkpoint (Phase 3 complete):**

- [x] PR description summarizes: (a) syscall count reduction, (b) before/after wall times, (c) byte-identical CLI output, (d) the two intentional behavior deltas, (e) tests added. **Covered in `commit-message.txt`** (which doubles as the PR description skeleton): (a) ~21,000 → 0 extra syscalls; (b) pre/post head-to-head warm + under-load wall/internal times; (c) byte-identical across 5 render modes on both a Cargo-root repo and a nested-workspace fixture; (d) gitignored-marker delta + directory-vs-file tightening; (e) 5 new L1 tests enumerated.

---

## Completed

All three phases implemented and verified on the `sniff` worktree (2026-06-20). `just all` in `sniff/` is green (sanity + lint + doctest + test + test-l2). CLI output (`repo packages` in default/`--json`/`--plain`/`--md`/`--list`, plus aggregate `repo --json`/`is-monorepo`/`package-count`) is byte-identical pre- vs post-refactor on both the rusty-biscuit Cargo-root worktree and a materialized nested-workspace fixture. Commit deferred to the separate commit process per session rules; the prepared message lives at `sniff/features/2026-06-20-faster-package-list/commit-message.txt`.

## Risks & Mitigations

- **Risk:** The `ignore` walker's `filter_entry` is called on each entry, not each directory. If pruning changes shape between the per-directory and per-entry models, the new walk might explore a different set.
  - **Mitigation:** Keep `WalkBuilder` settings byte-for-byte identical. Confirm via the existing `test_cargo_root_forbids_nested_cargo_layer` and `test_nested_only_*` regression tests.
- **Risk:** The gitignored-marker delta could be considered a regression by downstream users.
  - **Mitigation:** Document explicitly in commit body and spec. The spec already calls this out and judges the risk negligible (markers are conventionally committed). If a downstream report contradicts that judgment, revert the `git_ignore` filter on this walker only — not on the whole detection pipeline.
- **Risk:** Windows case-insensitive comparison is needed for fixed marker names but not for `.sln` / `.slnx` suffix.
  - **Mitigation:** `marker_name_matches` helper exists for fixed names only; the suffix check stays a plain `ends_with`. Helper-level test pins both contracts.
- **Risk:** Path with non-Unicode filenames could match a marker name byte-for-byte but `to_str()` returns `None`.
  - **Mitigation:** The spec accepts this (all marker names are ASCII). The skip-on-`to_str()`-is-`None` preserves the old behavior of `Path::exists()` (which would have returned `false` for a non-UTF-8 path that does not literally exist on disk).
- **Risk:** The `parent == root` skip must handle the walker yielding the root entry itself.
  - **Mitigation:** Skip when `path.parent() == Some(root)`; the root entry's parent is either `Some("")`/`None` or `Some(parent_of_root)`, both of which fail the equality check, so the root entry is naturally excluded.
