# Spike: cost of the per-remote containment engine for commit linking

Answers the pending measurement in Decision 9 (spec.md): the linking design reuses
`populate_recent_commit_remotes_from_snapshot` (sniff/lib
`filesystem/git/remote_refresh.rs:716`), which walks ancestry from every
remote-tracking tip and stops each walk when all target commits are seen. When
candidate commits are unpushed (or simply not on a tip's line), that tip's walk runs
to exhaustion. Decision rule: the unpushed worst case must be comfortably under
~100 ms to ship without a cap.

## Verdict

**Needs a cap / bounded walk. Decision 9's engine reuse stands, but the uncapped
worst case is not shippable.**

- Small/medium repos (12k commits, ≤ 50 tips): engine adds 34–75 ms — near the rule
  in the *normal* state, because stale remote tips already exhaust every walk.
  Unpushed commits add almost nothing on top (exhaustion already dominates).
- A real-world large clone (vscode: 165k commits, 5,197 remote-tracking tips):
  uncapped containment took **175–212 seconds** (726M commit visits), with or
  without a commit-graph file. The unpushed state adds +0.02% visits over the
  all-pushed state — the "worst case" is not unpushed; it is *many stale tips*,
  which a plain clone of any big OSS repo has.
- deep()'s existing cap of 50 tips bounds vscode to ~1.9 s (still 20x the rule) and
  **silently breaks containment**: alphabetical tip truncation dropped every tip
  containing main's recent 10, so 0/10 commits linked. Any cap must be
  priority-ordered (preferred remotes first), not positional.

## Harness

The engine is `pub(crate)`, so the spike measured the observable production path
(`GitRepo::discover` + `detect_with_request`) with a minimal request:
`commit_count = 10`, `refresh_remote_tracking = true`,
`include_commit_remote_containment = true`,
`metadata = GitMetadataRequest::none().commits(true)`, `max_remote_branches = None`.
An otherwise-identical request with containment off performs no ref snapshot, so
(wall time and counter deltas between the two variants) isolates exactly
`RefSnapshot::observe(remote branches)` + the engine walk. Counters
(`git.commit_visits`, `git.ref_walks`) read via `sniff::performance::
PerformanceCollector` + `with_current_collector`. Both variants run the same
`git fetch --prune` per remote (local-path remotes; the fetch cost cancels in the
delta but is included in absolute walls). `deep()` was also timed for context.
sniff lib built by path with default features (the engine needs no
`remote`/`network` feature; only `remote_observation` is feature-gated).

Crate: throwaway bin at `/var/folders/.../T/opencode/link-spike` (harness +
clones preserved there). Median of 3 (5 for drift-sensitive re-runs); 10-minute
cells marked as single runs.

## Environment

- Apple M4 Max (Mac16,5), 128 GB RAM, macOS 27.0; git 2.55.0, rustc 1.97.1
  (release profile), sniff lib @ this worktree (gix 0.84.0).
- A/C checkouts: local full clones of this monorepo (12,089 commits, 22 walkable
  origin tips incl. several stale feature branches; no commit-graph — matching the
  monorepo itself). Unpushed states fabricated with 5 `--allow-empty` commits on a
  local branch.
- B checkout: `git clone --filter=blob:none --no-checkout` of microsoft/vscode +
  empty cone sparse checkout (keeps the status walk cheap); 165,439 commits,
  5,197 remote-tracking tips (vscode keeps thousands of merged user branches);
  origin repointed to a local `--mirror` clone so fetches stay offline and
  prune-stable. Commit-graph toggled with `git commit-graph write --reachable` /
  file deletion.
- C checkout: monorepo clone + 3 extra remotes (local bare clones) + 22 branches
  at `HEAD~1..~22` pushed across them (~50 walkable tips).

## Results (median wall ms; engine visits = containment-minus-baseline counter delta)

| Scenario | Checkout | Baseline | +Containment | Engine delta | Engine visits | Contained |
| --- | --- | --- | --- | --- | --- | --- |
| A1 all-pushed | monorepo, 22 tips | 89.5 | 123.5 | **+34 ms** | 231,578 | 10/10 |
| A2 5 unpushed | monorepo, 22 tips | 86.4 | 125.3 | **+39 ms** | 243,657 | 5/10 |
| A2 + graph file | monorepo | 89.0 | 125.2 | **+36 ms** | 243,657 | 5/10 |
| A3 old window (~500 back) | monorepo | 92.7 | 103.5 | **+11 ms** | 55,555 | 10/10 |
| C1 all-pushed, ~50 tips | fork-style | 190 | 266 | **+76 ms** | 472,689 | 10/10 |
| C2 5 unpushed, ~50 tips | fork-style | 196 | 262 | **+66 ms** | 521,005 | 5/10 |
| C3 = C2 + graph | fork-style | 203 | 387* | *+120–180 ms (noisy) | 521,005 | 5/10 |
| B1 all-pushed, uncapped | vscode, 5,197 tips | 104 | **212,639** (1 run) | ≈ +212.5 s | 726,619,704 | 10/10 |
| B2 5 unpushed, uncapped | vscode | 104 | **175,389** (1 run) | ≈ +175.3 s | 726,785,133 | 5/10 |
| B3 = B2 + graph, uncapped | vscode | — | **187,806** (1 run) | ≈ +187.7 s | 726,785,133 | 5/10 |
| B2 cap 50 | vscode | 104 | 1,952 | ≈ +1.85 s | 7,644,852 | **0/10** |
| B3 cap 50 + graph | vscode | — | 1,993 | ≈ +1.89 s | 7,644,852 | 0/10 |
| A1 via `deep()` | monorepo | — | 157 | (context: full deep request) | — | 10/10 |

Derived per-visit cost: ~0.15 µs (12k-commit repo, object cache resident),
~0.25–0.5 µs (vscode), ~6 µs in the degraded state described below.

## Findings that refine Decision 9's assumptions

1. **The worst case is not "unpushed" — it is stale tips.** Any remote-tracking
   branch that does not descend from the target set walks to exhaustion in the
   *normal, all-pushed* state (monorepo A1: 231k visits vs A2's 243k; vscode B1 vs
   B2 differ by 0.02%). Repos with many merged-but-live remote branches (vscode,
   kubernetes, flutter) pay tips × full-history visits on every invocation.
2. **Commit-graph is performance-neutral here.** Identical visit counts; wall time
   within noise with vs without (A2: 36 vs 39 ms; B2 cap-50: 1.99 s vs 1.95 s; B
   uncapped: 188 s vs 175–212 s). gix 0.84's `ByCommitTime` walk speed is dominated
   by object access either way. Decision 9's "commit-graph accelerated" clause
   should not be relied on as a mitigation.
3. **A positional cap breaks correctness.** deep()'s `max_remote_branches = 50`
   truncates tips alphabetically after sorting by remote name; on vscode the 50
   walked tips contained none of main's recent 10, so containment (and therefore
   `remote: true` / `commit_url` under Decision 9) would be silently wrong. Today's
   deep tier already returns no "Synced to:" for such repos. Any bound must order
   tips by the preferred-remote policy (origin first) before truncating, or bound
   per-tip work instead of the tip list.
4. **Per-visit cost is not stable.** One fork-style clone degraded mid-spike to a
   persistent ~13x per-visit cost (451,830 visits: 266 ms → ~2,900 ms) with
   identical counters; it survived `git repack -ad` and MIDX removal and was not
   root-caused (suspected gix 0.84 object-access pathology tied to pack layout
   after gc/repack churn — fresh identical clones stayed fast). A budget sized off
   best-case µs/visit needs ~10x headroom.
5. **Old-but-contained windows are cheap** (A3: +11 ms): most tips contain the old
   targets and stop early. Depth of the target window is not a cost driver;
   tip-count × unshared-history is.
6. Containment is currently reachable only through `refresh_remote_tracking = true`
   (the fetch). Decision 9's no-network linking must invoke the engine with its own
   local snapshot, as the spec already implies; this spike confirms the engine
   itself needs no network.

## Caveats / not measured

- B uncapped cells are single runs (3–6 min each; the medians-of-3 protocol was
  dropped for time). Their magnitude (minutes) is far beyond any run-to-run noise.
- Absolute walls include a local `git fetch --prune` per remote (~40–90 ms) and the
  status walk; deltas between paired variants are the engine-only figures.
- Fetch-triggered `gc --auto` wrote split commit-graph chains and MIDX files into
  measurement repos mid-run; pairs measured back-to-back after quiescence. The A2
  "graph" row reflects an auto-gc chain (manual `--reachable` write with a broken
  flag failed silently); B/C graph rows used verified graph files.
- The vscode clone is blobless with an empty sparse cone; full checkouts may differ
  in page-cache pressure. kubernetes/flutter not tried (one large repo sufficed).
- Windows/Linux not measured; single host, warm-cache regime, release build.
- The degraded-clone anomaly (finding 4) was observed, not reproduced under
  control; treat as a variance risk, not a characterized cost.
