# Sniff Performance Review — 2026-07-13

Deep performance review of `sniff/lib` (plus the CLI consumption paths that drive
library cost). Five parallel review passes covered: the git subsystem, repo/package
discovery + file inventory, programs/services/package, os/hardware/network/remote,
and top-level orchestration. Every High finding was independently re-verified
against the working-tree source before inclusion.

**Status: findings recorded, unfixed.**

Baseline for severity: this monorepo (~50 packages, tens of thousands of files),
default plan (`OsRequest::full()` + `HardwareRequest::full()` + `NetworkRequest::full()`
+ full filesystem). Documented-and-gated costs (audio ~1.5s, WAN IP cold lookup,
`deep()` remote refresh) are not findings; only unrequested, duplicated, or
unbounded work is.

## Executive Summary

The shared-work architecture holds up: the shared filesystem walk, staged
detection, scoped-thread domain fan-out, and `ExecutableIndex` all behave as
documented, and no "runs despite skip" gating bugs were found. The avoidable cost
concentrates in:

1. The `sniff repo --json` aggregate re-running the working-tree status walk **8×**
   and re-opening the repo ~7× after detection already produced the data (H1).
2. Repo detection performing a redundant serial full-tree walk for nested markers
   (H2), and the standalone `detect_repo` path walking the tree 3–4× (H3).
3. Two git paths that walk unbounded history (H4, H5).
4. Remote reports fetching identical API payloads 2–3× (H6).
5. A default-on, uncached NTP network probe (H7).
6. N+1 serial subprocess spawns in systemd/runit service listing, with no
   timeouts anywhere in the services subsystem (H8, M11).

Fix-order recommendation is at the bottom. H1–H3 compound: bare `sniff repo --json`
triggers all three today.

---

## High Findings

### H1 — `sniff repo --json` re-walks the tree 8× and re-opens the repo ~7×

**Where:** `sniff/cli/src/output/repo_json.rs:805-808` (`build_aggregate_value`),
`repo_json.rs:977-1013` (`scope_bucket` / `changed_paths`),
`sniff/lib/src/filesystem/blast_radius.rs:68-82` (`collect_changed_paths`).
Dispatch: `sniff/cli/src/commands/mod.rs:1657-1666`.

**What happens:** The aggregate first runs full detection (`GitRequest::full()` →
`result.filesystem.git` already holds `file_changes`, `worktrees`, `branches`
with ahead/behind, and 10 commits). `build_aggregate_value` then ignores it:

- Four `scope_bucket` calls (Dirty/Staged/Unstaged/Untracked), each calling
  `changed_paths` twice (AllFiles + SourceCode) = **8 × `collect_changed_paths`**,
  each doing a fresh `GitRepo::discover` **plus a full status walk**. All 8 walks
  yield the same underlying change set; scope and kind are pure post-filters.
- `list_worktrees(dir)` (`repo_json.rs:744`) re-opens and duplicates
  `result…git.worktrees`.
- `branches_at(dir, false)` (`repo_json.rs:757`) re-opens and re-runs the
  per-branch ahead/behind revwalks (see H-adjacent M1), duplicating
  `result…git.branches`.
- `repo_root(dir)` ×2, `merge_conflicts_at`, `get_current_worktree_name` — each
  an independent `trusted_discover`.

**Cost:** the status walk is the single dominant cost in the library; the
aggregate pays it 9× (1 detection + 8 aggregate) plus ~4 extra discoveries and a
duplicated branch-divergence pass. `--json` is the machine path scripts/agents
invoke repeatedly.

**Fix:** thread `result.filesystem.git` into the aggregate builders:
- Classify the 4 scopes and the AllFiles/SourceCode/documentation splits from
  the already-populated `file_changes` (paths are already repo-root-relative;
  `changed_path_package_context` already takes `result`). 8 walks → 0.
- Populate `worktrees`/`branches` from the detection result.
- Reuse `result…git.repo_root` for the discovery-based helpers.
- Only `merge_conflicts_at` and the commit-family set (already shared via one
  `commit_set`, which is correct) are genuinely new work.

### H2 — `walk_for_nested_markers`: redundant serial full-tree walk every repo detection

**Where:** `sniff/lib/src/filesystem/repo/nested.rs:230-245`
(`walk_for_nested_markers`), invoked unconditionally from
`discover_nested_workspace_outcomes` (`nested.rs:145`), called at
`repo/detection.rs:300`.

**What happens:** a single-threaded `WalkBuilder::build()` traversal of the
entire tree runs on every `detect_repo_inner_with_shared` — including the
integrated path where `build_filesystem_system_view` has already walked the tree
in parallel and built the `ManifestIndex`. Three of the 12 markers
(`Cargo.toml`, `package.json`, `pyproject.toml`) are already in the index; the
shared walk sees every file and could match the remaining nine
(`pnpm-workspace.yaml`, `go.work`, `settings.gradle[.kts]`, `pom.xml`,
`rush.json`, `nx.json`, `turbo.json`, `lerna.json`, `*.sln`) in the same pass.

**Cost:** a full extra tree traversal (tens of thousands of dirents),
single-threaded, per detection. The largest avoidable filesystem cost in repo
detection.

**Fix:** extend `system_view::process_file_entry` (`system_view.rs:146`) to
collect nested-marker candidate dirs into the accumulator and pass them into
`discover_nested_workspace_outcomes`. For the standalone path, fold marker
matching into `ManifestIndex::build`'s walk (identical `WalkBuilder` config).
Minimum viable: switch to `build_parallel()`.

### H3 — Standalone `detect_repo` performs 3–4 independent full-tree walks

**Where:** `repo/types.rs:435` → `detect_repo_inner_with_shared(root, false, None, None)`;
consumers include `blast_radius.rs:96` and standalone doc detection.

**What happens:** with no shared view, one run does (1) `ManifestIndex::build`
parallel walk (`manifest_index.rs:123`), (2) `walk_for_nested_markers` serial
walk (H2), (3) `scan_file_inventory` parallel walk (`classify.rs:96` via
`detection.rs:368`), plus the Cargo membership glob walk (M4).

**Fix:** route standalone `detect_repo` through
`system_view::build_filesystem_system_view` (manifest index + inventory in one
walk), exactly as `detect_filesystem_with_request` already does; H2's fix
collapses the marker walk.

### H4 — `get_commits_for_path` diffs the entire history with unpruned full-tree diffs

**Where:** `sniff/lib/src/filesystem/git/discovery.rs:615-679` (walk),
`discovery.rs:569-579` (`commit_touches_path`). Backs
`api::commits_for_path_at` / `get_commits_for_path`.

**What happens:** the revwalk has no history bound; for every commit it computes
the complete tree-vs-parent diff (`get_commit_files_with_cache_fallible`
enumerates the whole change set), then discards everything except a
`starts_with` prefix test via `.any()`. For a prefix matching fewer than `count`
commits (the normal case), the loop runs to root history.

**Cost:** O(history × tree-size) object decoding to answer "recent commits under
`sniff/lib`".

**Fix:** (a) prune the tree diff to the prefix (pathspec-style filter) and
short-circuit on first match instead of collecting all files; (b) bound the
outer walk (max commits scanned).

### H5 — `deep()` commit containment walks full ancestry of up to 50 remote tips

**Where:** `sniff/lib/src/filesystem/git/remote_refresh.rs:535-597`
(`populate_recent_commit_remotes`).

**What happens:** for each remote tip (up to `max_remote_branches = 50` under
`deep()`), the full ancestry is walked and **every commit in history** is
inserted into `HashMap<ObjectId, Vec<String>>` with a `remote_name.clone()` per
commit per remote — to test containment of ~10 recent commits. The time-based
early stop is correctly ruled out (gix's ByCommitTime frontier is not globally
monotonic; see the skewed-timestamp test), but a **target-set** stop is valid.

**Fix:** build a `HashSet<ObjectId>` of the requested SHAs; per tip, remove
targets as they are seen and `break` when the set is empty — bounding each walk
by the depth of the oldest requested commit. Drop the per-commit `String` clone
(push an index / `&str`, resolve at the end).

### H6 — `remote::fetch_report` fetches repo metadata and the recursive tree 2–3× per report

**Where:** `sniff/lib/src/remote/provider.rs:157-208`; verified for GitHub at
`github.rs:255,304,312,500,508`. Same shape: Gitea
(`gitea.rs:313,370,379,607,616`), Bitbucket (`bitbucket.rs:276,337,428,711`),
GitLab (`gitlab.rs:280,330,541` — 3× `ListRepositoryTree`).

**What happens:** `fetch_report` awaits the required `get_repo_metadata`
(fetches `GetRepository`), then `tokio::join!`s `list_documents` and
`detect_cicd`, each of which re-fetches `GetRepository` to rediscover the
default branch and then fetches the **full recursive tree**. Worst case per
report: 3× `GetRepository` + 2× recursive tree (the largest payload).

**Fix:** resolve `RepositoryInfo`/default branch once and thread it into
`list_documents`/`detect_cicd` (private `*_with_branch` helpers); fetch the
recursive tree once and share it. Bonus (M10): Bitbucket's three directory
listings inside `list_documents` are serial awaits — join them.

### H7 — NTP status: default-on, uncached network round-trip

**Where:** `OsRequest::full()` sets `include_ntp_status: true`
(`request.rs:163`); default plan uses `full()`. macOS probe at
`sniff/lib/src/os/time.rs:365-386`.

**What happens:** every `detect()` reads `/etc/ntp.conf`, then spawns
`sntp <server>` (default `time.apple.com`) — a live NTP network round-trip,
blocking the OS-detection thread up to `NTP_TIMEOUT_SECS = 3`, never cached.
Same class of issue as the darkmatter compose sntp probe (2026-07-12 review).

**Doc drift (fix in same change):** `time.rs:326` claims a "5-second timeout"
and `time.rs:408` claims "up to 10 seconds on Linux"; the real bound is 3s and
Linux uses a fast local `timedatectl` D-Bus call. The architecture doc's cost
table repeats the 10s claim. Code is correct; comments/docs are stale.

**Fix:** gate NTP off by default (it is not core OS identity), and/or TTL-cache
`NtpStatus` like the WAN-IP cache. Correct the stale docs.

### H8 — systemd/runit service listing: N+1 serial subprocess spawns

**Where:** `sniff/lib/src/services/systemd.rs:40-78`
(`get_systemd_service_pid` — one `systemctl show` per running unit, serial);
`services/runit.rs:32,45-72` (one `sv status` per service dir, serial).

**Cost:** 80–150 sequential spawns at ~5–15ms each on a typical systemd host →
0.5–2s+ of pure fork/exec latency dominating service enumeration.

**Fix:** batch — `systemctl show --property=Id,MainPID <unit1> <unit2> …`
collapses N+1 spawns to 2 (or 1 with `--all --type=service`); `sv status`
accepts multiple names in one invocation. See also M11 (no timeouts).

---

## Medium Findings

### M1 — Plain `full()` pays 2 revwalks per local branch for ahead/behind

`remote_refresh.rs:252-306` via `ahead_behind` → `count_reachable_excluding`:
`full()` has `commit_count: 10` so `wants_repo_metadata()` is true, and every
non-current local branch costs two revwalks of its divergence vs HEAD.
Commit-graph softens per-commit cost but it remains O(branches × divergence).
Fix: gate divergence counts behind `deep()` or a dedicated
`include_branch_divergence` flag; or compute lazily for rendered branches only.

### M2 — Per-dirty-file status diff work is doubled

`status.rs:215-246` loop; `staged_diff_stats` (`:362-409`) and
`staged_diff_patch` (`:451-518`) each re-fetch `head_tree_id_or_empty` +
`find_tree` + `index_or_empty` (root tree decoded up to 2× per dirty file), and
with `include_diffs` a modified file's blobs are loaded and histogram-diffed
twice (`diff_bytes` for counts, `text_hunks` for the patch). Fix: hoist
head-tree/index out of the loop; compute the unified diff once and derive
added/removed from it.

### M3 — Ref-decoration cache defeated by full-map clone per commit query

`discovery.rs:260` (`get_recent_commits_fallible`), `:421-424`
(`get_commit_by_sha_fallible`), `:638-641` (`get_commits_for_path_fallible`):
`ref_decorations.cloned()` deep-clones the entire
`HashMap<ObjectId, Vec<RefDecoration>>` the cache exists to avoid recomputing.
Fix: borrow in the `Some` branch (Cow or split owned/borrowed paths); clone only
the per-commit `Vec<RefDecoration>` into `CommitInfo`.

### M4 — Cargo workspace glob expansion ignores the ManifestIndex

`repo/cargo.rs:65` → `glob.rs:174` (`walk_manifest_dirs`): per glob member,
another `ignore` walk of the prefix subtree with `dir_has_manifest`
(`glob.rs:247-249`) doing up to 4 `exists()` per directory — run twice (members
+ excludes). For a root workspace spanning most top-level dirs this re-walks
nearly the whole tree. Fix: thread `Option<&ManifestIndex>` into
`detect_cargo_workspace`/`expand_membership_globs` and match globs against
index entries (same optimization Nx/Turbo/Lerna nested discovery already has).

### M5 — `ManifestEntry.kinds` is dead; `create_package` re-probes with ~35 `exists()` per package

`manifest_index.rs:81-82` computes `kinds: HashSet<ManifestKind>` per entry,
then only tests use it (`#[allow(dead_code)]`). Meanwhile every `create_package`
(`detection.rs:1319`) re-derives manifest presence by stat-probing:
`detect_package_ecosystem` (4), `detect_package_managers` (up to 7),
`resolve_package_name` (up to 4), `resolve_package_version` (up to 4),
`create_package` body (6), `detect_test_runners`/`ecosystems_present` (~12 incl.
`read_dir`). ≈ 1,500–2,000 stats per full run on this repo. Fix: pass the owning
dir's `&HashSet<ManifestKind>` (or a `ManifestPresence` struct) into
`create_package`; fall back to `exists()` only for non-indexed manifests.

### M6 — Root `Cargo.lock` parsed ≥3× per run

`CargoLockVersions::parse` (full `read_to_string` + TOML parse of a
hundreds-of-KB lockfile) at `cargo.rs:48`, `detection.rs:348`,
`detection.rs:583` (once per Cargo layer), `detection.rs:66` (single-package
path). Fix: parse once at the top of `detect_repo_inner_with_shared`; thread
`Option<&CargoLockVersions>` through.

### M7 — `aggregate_versions` re-parses the root `Cargo.toml` per inheriting member

`repo/aggregate.rs:377-442` (`resolve_version_source`): uncached
`read_toml`/`read_json` per package; `version.workspace = true` members each
trigger `cargo_package_version_with_source` → `read_toml(root_manifest)`
(`cargo.rs:241-247`). O(P) re-parses of the same file for `sniff repo version`
in this workspace-inheritance-heavy repo. Fix: parse the root manifest once and
share; or store `VersionSource` on `Package` during detection (the version
itself is already resolved at `detection.rs:1337`).

### M8 — Per-package test-runner config probing multiplies syscalls

`test_runner_usage.rs:124-153`: per package × per ecosystem spec, literal globs
→ `exists()`, wildcard globs → `glob_walk` recursive `read_dir` (depth ≤ 16);
nextest's root-scoped config is re-probed at `repo_root` **once per package**
(`:132-136`). Fix: resolve root-scoped configs once per run; derive per-package
config presence from the already-built `FileInventory` instead of fresh walks.

### M9 — Shared walk always rooted at the git root: package-scoped queries over-scan

`filesystem/mod.rs:106-109` + `system_view.rs:67-107`: when git detection runs,
the shared walk is rooted at the repo root even when `base_dir` is a single
package; `filter_inventory` (`mod.rs:207-244`) narrows only after the walk. Up
to ~50× walk amplification for a per-package inventory/language query. Fix:
root the shared walk at `root` when neither full repo detection nor docs need
repo-wide scope.

### M10 — Remote misc: Bitbucket serial listings; WAN-IP client/endpoints

- Bitbucket `list_documents` (`bitbucket.rs:331-419`): repo fetch + root +
  `docs/` + `doc/` directory listings are serial awaits — join the three
  listings; thread the branch in per H6.
- WAN IP (`network/mod.rs:24,368-396`): single default endpoint
  (`api64.ipify.org`) so the retry loop has no fallback, and a fresh
  `reqwest::blocking::Client` is built per call (plus a dedicated OS thread per
  fetch inside a tokio runtime). Fix: ≥2 default endpoints; reuse a `OnceLock`
  client.

### M11 — Subprocess probes without timeouts / with pipe-deadlock hazard

- **Services (all backends):** `systemd.rs:15,65`, `launchd.rs:7`,
  `openrc.rs:9`, `runit.rs:48` — plain `.output()` with no timeout; one wedged
  `systemctl`/`sv`/`rc-status`/`launchctl` hangs all of `services_detailed()`.
  Reuse the `try_wait` + timeout pattern from `programs/schema.rs` /
  `host_capability.rs` (extract a shared helper).
- **`diskutil` (macOS storage):** `hardware/storage.rs:128` — the only hardware
  probe with no timeout; `diskutil` is known to stall on flaky external
  volumes. Bound it like the other probes.
- **Pipe-buffer deadlock:** `programs/schema.rs:327-365` and
  `host_capability.rs:101-132` read piped output only **after** child exit; a
  child writing > ~64KB (`npm ls -g --json`, `cargo install --list`) blocks on
  write, always burns the full timeout, and is reported missing/unverified.
  Fix: drain pipes concurrently (reader threads or `wait_with_output` under a
  timeout thread).

### M12 — `HostCapabilities::detect()` uses the lazy index: ~104 per-name PATH walks

`programs/host_capability.rs:65` calls `ExecutableIndex::build_path_only()`, so
`find_programs_with_source` takes the per-name `which()` branch
(`executable_index.rs:257-264`) for 37 OS + 67 language package-manager names —
mostly misses, each walking the full PATH — and bypasses the warm
`EAGER_PATH_CACHE` that `ProgramsInfo::detect()` may already have populated.
Fix (one line): use `build_eager_path()`.

### M13 — `list_worktrees` opens each linked worktree as a full repo, serially

`git/worktree.rs:180-202`: per worktree, `trusted_open` + canonicalize just to
read HEAD, in a serial loop — while `remote_refresh::get_worktrees` already
rayon-parallelizes the same shape. Fix: parallelize, or read the per-worktree
`HEAD` file directly for a name/branch listing.

---

## Low Findings

- **L1** `recent_commits.rs:429-437` (+ duplicates at `:522-530`, `:621-629`):
  per-commit file→package attribution is O(files × packages) `starts_with`, and
  the block is copy-pasted across the three `collect_commits_*` fns. Sort
  packages by descending depth and break on first match; factor into one helper.
- **L2** `recent_commits.rs:388-404`: date-window queries walk full history
  (deliberate `continue`-not-`break` skew guard). If profiled hot: stop after a
  run of commits older than `since` by a max-skew bound (24–48h).
- **L3** `remote_refresh.rs:365-398`: `push_relevant_ahead` re-globs and peels
  all `refs/remotes/<remote>/*` per tracking query; cache the hidden-tip set or
  reuse `remote_tracking_tips()`.
- **L4** `recent_commits.rs:381-383` (+2 dupes): `diff_resource_cache` never
  cleared across long walks — the exact hazard `discovery.rs:472-475` warns
  about. Periodic `clear_resource_cache()` on unbounded walks.
- **L5** `git/api.rs:45-47` path-based helpers re-discover/open the repo per
  call; fine for one-shot CLI, offer a batched handle entry point for chained
  callers.
- **L6** Regexes recompiled per call: `repo/standard.rs:608` (~50×/run during
  version stamping), `programs/schema.rs:404,422`. Hoist into
  `LazyLock`/`OnceLock` per the existing pattern (`git/types.rs:94-95`).
- **L7** Remote providers allocate `body.to_lowercase()` for every 403 to
  substring-match "rate limit" (`github.rs:138`, `gitlab.rs:174`,
  `gitea.rs:212`, `bitbucket.rs:149`); eager `request.clone()` on rarely-taken
  anonymous-retry paths.
- **L8** `network/mod.rs:241,323`: interface cache deep-clones the full result
  under the mutex on every hit/store, with a 1s TTL that rarely helps within a
  single detect; clone cost may exceed savings.
- **L9** `programs/inventory.rs:140-179` `Program::from_binary_name` and
  `install/command.rs:189-195` `method_available` do linear enum scans; a
  static name→variant map makes them O(1).
- **L10** `manifest_index.rs:236-256` `package_dirs_in_tree` is O(P × M) with
  two `normalize_path` allocations per call, called once per workspace package
  (`detection.rs:351-360`). Skip leaf members or pre-group entries by prefix.
- **L11** `locale.rs:120` `detect_windows_locale` spawns PowerShell (no
  timeout, ~200–700ms cold start) whenever `LANG`/`LC_*` are unset on Windows.
- **L12** `detection.rs:1121-1130` `merge_path_lists` is O(n²) via
  `contains` per insert; `docs.rs:1021` `assign_packages` is O(docs × packages);
  per-entry `PathBuf`/`to_string_lossy` allocations across the walks
  (`manifest_index.rs:151`, `detection.rs:1211-1215`). All small at current
  scale; listed for completeness.
- **L13** `executable_index.rs:110-128`: `build_with_bundles(true)` bypasses
  `BUNDLE_INDEX_CACHE` (which `build_eager_path` uses), re-scanning ~25 app
  bundles on repeated `build()` calls. Route through the cache.
- **L14** `programs/test_runner.rs:324-326` + `local_bin.rs:196-210`: test
  runners resolve serially and re-stat `node_modules/.bin` ancestors per Node
  runner; memoize the ancestor set or rayon-ize if it ever profiles hot.
- **L15** Legacy `SniffConfig`/`detect()` cannot express anything between "skip
  domain" and "full", so the default pulls NTP + WAN-IP + audio; the concurrent
  fan-out makes total ≈ max(domain), but NTP/WAN become the long pole. A
  default-cost policy choice, not a bug (H7 addresses the worst component).
- **L16** macOS audio (~1.5s CoreAudio IPC) is correctly gated and parallel but
  uncached across repeated `detect()` calls; `get_channel_count` does a raw
  alloc/dealloc per call. Optional short-TTL cache only if repeated detection
  becomes a use case.

---

## Verified Healthy (no action)

- Repo discovery de-duplicated across git/repo stages (`filesystem/mod.rs:97-109`);
  single `GitRepo::discover`, handle threaded through.
- Docs and inventory genuinely reuse the shared walk (zero-copy `Arc` in the
  no-filter case); `detect_formatting` does no tree walk.
- Domain fan-out is genuinely concurrent (`lib.rs:281-354`): all handles spawn
  before any join; total ≈ max(domain).
- Git request gating honored: `identity()` provably skips the status walk;
  `minimal()`/`summary()` short-circuit to a dirty flag; the four status layers
  share one gix handle. `refresh_remote_tracking_refs` is deep-only, uses
  `GIT_TERMINAL_PROMPT=0`, bounded parallelism (1–3).
- `get_worktrees` shares one `into_sync()` base handle across rayon workers and
  skips ahead/behind + status for non-current worktrees unless `full_details`.
- `sysinfo` refresh is targeted (`RefreshKind::nothing().with_cpu().with_memory()`),
  not `refresh_all`; macOS GPU uses the ~200µs IOKit path; `diskutil` and macOS
  bundle scans are batched; storage pre-filters virtual filesystems.
- `ProgramsInfo::detect()` builds one eager, `Arc`-shared, `OnceLock`-cached
  index; categories fan out via `rayon::join`; version probing is strictly
  on-demand; install-plan building is spawn-free.
- The 110+ package-manager registry does no eager probing (`is_available()` is
  a PATH check, on demand only); init-system identity never triggers service
  listing; Windows SCM uses the native `EnumServicesStatusExW` API.
- The focused change commands (`dirty-packages`, `staged-source-code`, …)
  consume `result…git.file_changes` — only the `--json` aggregate re-walks (H1).
- `sniff repo structure` legitimately needs `RepoRequest::full()` (renders
  per-package languages/frameworks); the `git-status` plan is already trimmed.
- All lazy statics are cheap at load; no eager global init.
- Prior fixes held under review: nested-marker syscall storm (walk itself is
  the remaining issue, H2), worktree fan-out, git-status language scan.

---

## Suggested Fix Order

| Phase | Findings | Rationale |
|-------|----------|-----------|
| 1 | H1, H2, H3 | Biggest everyday-CLI wins; compound on `sniff repo --json` |
| 2 | H7, H8, M11 | Latency cliffs + hang risks (network probe, N+1 spawns, missing timeouts) |
| 3 | H4, H5, M1, M2, M3 | Pathological git scaling on history/branch-heavy repos |
| 4 | H6, M10 | Remote report round-trips halved |
| 5 | M4–M9, M12, M13 | Syscall/parse fan-out cleanups; mostly plumbing existing data through |
| 6 | Low findings | Opportunistic, alongside adjacent work |

Doc-drift corrections (H7: `time.rs:326`, `time.rs:408`, architecture-doc cost
table) should land with whichever phase touches them, per the authoring
discipline in `CLAUDE.md`.

## Method

Five parallel read-only review passes (git subsystem; repo discovery +
inventory; programs/services/package; os/hardware/network/remote; orchestration
+ CLI consumption), each quoting confirmed source, followed by independent
re-verification of every High finding against the working tree at review time
(branch `sniff`, including uncommitted edits to the git module and CLI repo
output).
