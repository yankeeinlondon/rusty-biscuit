---
deferred_perf_measurement: false
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - worktree/lib/src/pull_requests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - worktree/lib/src/listing.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - worktree/cli/src/commands/git_graph/tests.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - worktree/cli/src/commands/list/tests.rs
  - worktree/cli/src/commands/list/tests/pipeline.rs
  - worktree/cli/tests/list_prs.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/worktree/testing.md
source_files_during_phase_7:
  - worktree/cli/tests/perf_flag.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
  - worktree/cli/tests/list_prs.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages:
  - sniff
  - worktree
  - worktree-cli
implementation_1: "2026-10-03T14:50:53-07:00"
---

# Implementation Log: List Overlap And Keyless Notice

## Implementation of Review Findings #1

> **started at:** 2026-10-03T14:50:53-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-wt-skill/worktree/fixes/2026-10-03-list-overlap-and-keyless-notice/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- orchestration notes:
        - the log file was empty and the working tree held no source changes, so every finding needs a full implementation from `plan.md`, not a patch on an earlier attempt
        - findings run one at a time, in review order: finding 1 (plan phases 4, 5, 6), finding 2 (phases 2, 3, 8), finding 3 (phase 7 plus wiring the perf groups into the pipeline); each subagent also updates the docs its finding names (part of phase 9)
- starting the work on 'Local gathering still waits for the remote worker' at 14:58:30
        - spike S1 (verbose labels from a snapshot): `--decorate-refs-exclude=HEAD|refs/heads/|refs/remotes/` leaves `%D` with only tags and other non-branch decorations; adding a `--decorate-refs=` include pattern instead drops tags (Git then decorates only included refs), so it was rejected. Ruling for Rule 6: read `%H` plus the filtered `%D`, then rebuild `HEAD -> <current>` (at the snapshot's current-branch tip), remote-tracking names, symbolic `origin/HEAD` (beside its target's snapshot tip), and other local branches from `RefTips`, in Git's reverse-refname order. With nothing moved the spelling equals Git's own `%D` (pinned by `verbose_labels_follow_the_snapshot_and_keep_live_tags`)
        - to place `origin/HEAD`, `RefTips` now records symbolic remote HEAD targets (`remote_heads`, read with `%(symref)` in the same `for-each-ref`); it takes part in snapshot equality, which is conservative (a retargeted `origin/HEAD` triggers one regather)
        - sweep finding: the graph (`topology.rs`), comparisons (`compare_cached`), and target ancestry already passed captured SHAs to Git; the only live-ref read in a history/comparison path was verbose `%D`
        - library split: `fill_worktree_statuses` is now `load_comparison_cache` + `WorktreeList::gather_local` (`gather_dirtiness` ∥ `gather_ref_facts`, no persistent writes) + `WorktreeList::commit` (cache save, fork prune with reload and `read_at` protection, copy-record prune, all once); `refresh_dirty_status(checkout)` re-measures one entry; `listing::RefSnapshot` carries tips, read success, and time, and `matches` is false whenever either read failed. `reread_refs` and the `refs_read`/`refs_read_at` fields were removed (only `list.rs` used them)
        - CLI split: `gather_remote` became `prepare_remote` (origin lookup and `--ignore-api` write, before any speculative work) and `follow_remote` (launch, wait, spinner, PR reread); `gather_remote` survives as a `#[cfg(test)]` composition so the existing `gather` tests are unchanged. `run_pipeline` now calls `gather_listing`, which returns a `Listing` of accepted results, then renders once; tests assert through `Listing`
        - decision: with no wait and no `--ff` there is no second read, so the first gather is accepted even when the parse-step read failed (today's degraded single-read behavior); the "either read failed" rule applies only when a second read exists
        - perf (flat stages kept for finding 3): `pr gather`, `remote wait`, `list gather`, `graph gather`/`verbose gather` as before (the last three now overlap the wait), plus new conditional `regather` and `checkout status refresh`. Names were chosen not to contain an existing stage name, because `stage_from_perf` matches the first substring
        - drift left for finding 3: `worktree/README.md` and `docs/performance-testing.md` still say the `--perf` stages plus `unattributed` sum to the total; with the overlap that is no longer true (the collector clips with `saturating_sub`). That is finding 3's defect class, so the text was not changed here
        - not done here: the spec's quick release-build sample (Phase 9) was not taken; it belongs with the perf grouping (finding 3) so the sample shows the new report
        - departure from plan Phase 5: `GatherInput` is still built from the parsed `WorktreeList` (entries, default branch, forks), now with explicit tips (`from_list(list, refs)`); `build_tree` was not needed for it
        - verification: `just test` 901 passed (before the final clippy-only type-alias edit; the 16 new/changed tests were re-run after it and pass), `just lint` clean, `just test-perf` 30 passed, `just test-l2 list` 13 passed (tmux), `just check-tier-coverage worktree` no stranded tests. Mutation checks: waiting before spawning the local gathers fails `the_local_gathers_start_while_the_worker_outcome_is_held` (10 s bounded timeout); always accepting the first gather fails 5 of the 7 pipeline tests
- class sweep: a local result computed before or during the remote wait is used or persisted without first proving that the refs it was computed from are still the refs being shown; sites checked: list gather (dirtiness, caption, target, tree, comparisons), image graph gather, verbose gather (history and `%D` labels), dirty-status walks, `--ff` path, ref validation, comparison-cache save, fork-origin prune, copy-record prune, no-origin and non-image controls, failed-read pruning guard; fixed here: list gather, image graph gather, verbose gather (labels now from the snapshot), dirty-status walks (measured once during the wait, targeted refresh after an `--ff` move), `--ff` path (joins local work first), ref validation (`RefSnapshot::matches` over complete local, remote, and remote-HEAD maps with read-success tracking), cache save / fork prune / copy-record prune (one `WorktreeList::commit` after acceptance, speculative SHA-pair entries kept); clean: graph and comparison Git queries (already object-ID addressed), no-origin and non-image controls (preserved), failed-read pruning guard (preserved through both reads)
- work completed for 'Local gathering still waits for the remote worker' at 15:25:55
- starting the work on 'Successful API credential evidence and the keyless notice are missing' at 15:27:08
        - spike S2 (carrying the credential selection): the selection is made per request in `FocusedProviderClient::fetch_json` by `credential()` (provider variables for a URL-built client, the host-bound `SNIFF_{PROVIDER}_{HOST}_TOKEN` only for a discovered `ProviderAndHost` client). Minimal change: an optional per-client `SentLog` (`with_sent_log`) that `fetch_json` appends to from that same `credential()` call, installed by `blocking::run_with_deadline` and folded there. Chosen over `*_with_evidence` entry points because only two blocking signatures change (`BranchHead` gains `credentials`; `open_pull_requests{,_with}` return `OpenPullRequests { pull_requests, credentials }`) and their only workspace callers are `worktree` and sniff's own tests. No extra request, deadline, pagination, or classification change
        - sniff: new `RequestCredentials { Anonymous, Keyed { variables }, Unknown }` (plain data, `#[non_exhaustive]`, names only); fold is `Unknown` for no request, `Anonymous` only when every request was, otherwise `Keyed` with distinct names in first-use order (pages and GitLab fork lookups included)
        - decision (sniff, beyond the success path): a failure's `key` now comes from the last request actually sent instead of `credential_key()` read before the operation, and `PrUnavailable::NotFoundOrNotPermitted` gained `key`. That let `worktree` delete `SniffBranchHeads::key_in_use` entirely (its only use was the 404 `not-visible` decision, and it missed host-bound overrides); `credential_key` was removed from sniff too. Breaking source change for struct-pattern matches on that variant; recorded in sniff's CHANGELOG
        - decision (sniff, credential policy): an empty token variable is now unset everywhere sniff reads one (`credentials::token_in`, used by `provider_token` and `host_bound_provider_token`): it is never sent and no longer hides the next candidate (before, `GH_TOKEN=` sent `Bearer ` and hid `GITHUB_TOKEN`). The orchestrator asked for "empty variable = absent"; applying it to the selection rather than only to the evidence keeps the evidence truthful. This touches the spec's out-of-scope "changing credential precedence" only for the empty-string case; the author may want to confirm it. It also applies to provider discovery's host-bound retry
        - worktree evidence type: `remote_head::CredentialEvidence`, tagged `{"state": "anonymous" | "keyed" | "unknown"}` with `variables` for keyed, read through `strict_json::nested` (integer tag and repeated keys rejected). Departure from plan Rule 1's name pattern: stored names must also be upper-case (`[A-Z_][A-Z0-9_]*`), because a GitHub token (`ghp_…`, mixed case) matches the plain identifier pattern; every variable sniff reads is upper-case
        - head store format 3: `Attempt::credentials` (required, starts `unknown`), written by the new `set_credentials` the moment the API check succeeds, before the answer is published, so the fetching phase, a fetch failure or timeout, and a `source: fetch` answer all keep it. Format 2 keeps its answer and drops its attempt; format 1 migrates as before
        - PR store format 6: `credentials` is written in the same atomic write as the answer and publication id (empty answers too); format 5 is read with the same strictness as `unknown`; 4 and older and 7+ are misses. `stored_publication` now returns `StoredPublication { id, credentials }`; `OpenPrSource::fetch` returns `FetchedPrs`. The writer refuses invalid names (`PublishFailed`), so nothing token-shaped can reach disk
        - wait capture: `WaitEnd.pr_credentials` is taken from the first new publication the wait sees, in the same read as its id (a lock holder's included), and never replaced by a later one; a success known only from a receipt, and the pre-launch answer, give `unknown`. No new wait, no budget change. Head evidence is the followed attempt's own field, so an adopted worker's environment is what counts
        - rendering: `list::observed_keyless` (followed attempt `anonymous`, or `PrEnd::Published` with `pr_credentials` `anonymous`; not ignored, not origin-changed, attempt not `Unavailable`) feeds `credential_line` as the third precedence step via `CredentialCondition::AnsweredWithoutKey { higher_limits }`, through the existing dim renderer
        - decision (wording per provider): "for higher rate limits" for GitHub (github.com), GitLab (gitlab.com), and Bitbucket (bitbucket.org), whose docs give keyed requests higher limits than anonymous ones (and the blocking lookups ask only those hosts for those flavors); "to authenticate API requests" for Gitea and Forgejo (Codeberg included), which set no such default. Decided by sniff's provider display name (`keyed_limits_are_higher`), pinned by a test
        - departure: the spec's example line reads `GITHUB_TOKEN or GH_TOKEN`; the shipped line uses `credential_env`'s lookup order, `GH_TOKEN or GITHUB_TOKEN`, as the spec's "names come from credential_env" requires
        - discovery: the existing L2 credentials-warning test already had an anonymous PR answer beside the head's 404 warning, so it now also proves warning-over-notice precedence in a real pane; the new L2 test is its sibling (`level2_list_keyless_notice_is_a_dim_line_beneath_the_caption_after_the_spinner_in_tmux`) rather than a second phase inside it
        - discovery: test seeders that write PR stores at `PR_STORE_FORMAT_VERSION` had to add `"credentials": {"state": "unknown"}` (perf_support, list_remote_head, level2_list_verbose); without it every seeded answer became a miss
        - smell grep: no `#[serde(default)]` and no `Option` on the new fields; the only `.ok()` near the credentials parse is the PR store's whole-file `from_slice(..).ok()?`, which turns a malformed document into a miss (never into anonymous); `unwrap_or_default()` appears only in a test seed's PR list and finding 1's code
        - blocker worked around (host, not the change): Homebrew upgraded `libgit2` 1.9.4 → 1.9.7 during the session, and `libgit2-sys`'s cached build kept linking the removed Cellar path, so `cd sniff && just test` and `just check-tier-coverage sniff` failed to link; `cargo clean -p libgit2-sys` did not help, `LIBGIT2_NO_PKG_CONFIG=1` did. Recorded in the `os` skill's macOS page
        - verification: worktree `just test` 934 passed (final clean run; an earlier run under load average 120 timed out `git_graph::tests::gathered_graphs_lay_out_with_exact_merges_and_no_overlapping_tags` at 30 s, which passes alone in 13 s and is untouched here); `just lint` clean; `cargo clippy --all-targets -D warnings` clean for both worktree crates and sniff (with and without `remote`); `just test-l2 credential` 1 passed and `just test-l2 list` 14 passed (tmux, `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`); sniff `just test` 3127 passed (with `LIBGIT2_NO_PKG_CONFIG=1`), `just lint` clean; `just check-tier-coverage worktree` and `sniff` report no stranded tests
- class sweep: successful provider requests lose the record of which credential they were sent with before their results are published, so a reader cannot tell an observed anonymous success from a keyed or unknown one; sites checked: sniff branch-head path, sniff open-PR path (pagination and GitLab fork lookups), sniff failure `key` resolution, sniff empty-variable selection, worktree `SniffBranchHeads::key_in_use`, head attempt writer/reader, PR publication writer/reader, wait projection (own publication, lock holder, receipt-only success, later publication, adopted attempt), credentials-line selection, suppression controls (ignored, unsupported, local-path, changed origin, cached-only, unknown, post-selection), shipped CLI (PR-only, head-only, both, keyed), fallback notice coexistence; fixed here: sniff branch-head and open-PR paths (`RequestCredentials` from the sending client), sniff failure `key` (from the request sent; `NotFoundOrNotPermitted { key }`), empty variables (unset in the selection), `key_in_use` (removed), head store (format 3, evidence on the attempt), PR store (format 6, evidence with the publication), wait projection (`pr_credentials` from the accepted publication), credentials-line selection (keyless notice as third step), changed-origin and ignored suppression for success, shipped CLI (three regression tests plus keyed and coexistence); clean and preserved: warning precedence (head first, then PR), unknown/cached/local-path/unsupported suppression, closing fallback notice (unchanged, coexists), receipt format (unchanged)
- work completed for 'Successful API credential evidence and the keyless notice are missing' at 16:34:10
- starting the work on 'The performance report still adds overlapping work' at 16:35:24
        - collector API: `PerfCollector::record` (sequential top-level row) and `record_group(name, elapsed, children)` (one top-level row whose duration is the group's own measured span; children are concurrent tasks measured inside it). `build_perf_tree` is `pub(crate)` so pipeline tests assert the report shape; `recorded_stages` (test-only) flattens a group into its name then its children, so existing name readers keep working
        - decision (excess attribution): no `saturating_sub` and no `debug_assert`. Top-level rows are sequential by construction, so the rows plus `unattributed` equal the total exactly; if they ever exceed it, the report shows the excess as a visible row `over-attributed (overlapping top-level rows)` (`perf::OVER_ATTRIBUTED`) in place of `unattributed`. A `debug_assert` was rejected because it would make the over-attribution test depend on the build profile; the visible row surfaces the bug in release builds too. Pinned by `rows_beyond_the_elapsed_time_are_surfaced_not_clipped` and checked in every pipeline test through `assert_perf_reconciles`
        - departure (rendering): group children show the metrics tree's not-applicable dash `—` in the share column rather than a blank, because `MetricShare` has no blank variant and the spec asks for the existing biscuit-terminal rendering; adding a variant would change `biscuit-terminal`'s public enum (and schedule its dependents) for a cosmetic difference
        - wiring: `pr gather` is recorded right after `prepare_remote` (origin lookup only, recorded as zero when no store path exists, as before); the group span starts before the first scoped spawn and ends after both joins, so it includes the post-wait PR reread, which is always inside the window and is therefore always the child `pr reread` (Rule 5 says "when it overlaps"; it cannot run outside the group). Group name `remote wait ‖ local gather` only when the wait ran (`waited.is_some()`), else `local gather` with no `remote wait` child. `regather` is a measured group with `list regather` and `graph regather`/`verbose regather` children (names chosen so no label repeats another row's label); `fast-forward` and `checkout status refresh` stay top-level rows. `RemoteAnswers::pr_gather` became `pr_reread`; the pre-launch duration lives only in `RemotePlan`
        - parser: `perf_support::stage_from_perf` now parses rows (`perf_rows`: depth from the connector width, label = tokens before the first duration) and matches a whole label at any depth, panicking on a duplicate label instead of picking one; the old first-substring match would have read `remote wait ‖ local gather` as `remote wait`
        - discovery: `perf_support`'s own `#[cfg(test)]` tests sit under the `perf_support::` module path, which the L1 filter's `(^|::)perf_` excludes, so they run only in `just test-perf`. The new parser tests were therefore put in `cli/tests/perf_flag.rs` (L1) rather than beside the parser; recorded in the worktree skill's testing page
        - stale PR gate: `perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh` now bounds the foreground PR reads as `pr gather` + `pr reread` (the same work it bounded before the split), so splitting the stage does not weaken the guard against a reintroduced foreground request
        - tests (L1): `perf::tests::overlapping_group_children_are_excluded_from_the_top_level_sum`, `groups_keep_their_measured_span_and_sit_beside_sequential_rows`, `rows_beyond_the_elapsed_time_are_surfaced_not_clipped`, `only_top_level_rows_carry_a_share` (existing three kept); `perf_flag::the_stage_reader_picks_the_nested_child_never_the_group_containing_its_name`, `report_rows_carry_their_depth`, `list_perf_reports_a_local_only_group_that_reconciles` (shipped binary, real renderer); report-shape and exact-reconciliation assertions added to the pipeline tests (remote group with `pr reread` on unchanged tips and no `regather`; `regather` group children on every ref-change case; `fast-forward` row; timed-out case proves the group is its span, not its longest child: group ≥ `remote wait` + `list gather`) and to the three `run_pipeline` tests (local-only group with `graph gather`, `verbose gather`, or neither; no `remote wait` row). No snapshot was added
        - docs: `docs/performance-testing.md` (`pr gather`/`pr reread` meanings, nested groups with an example and a Mermaid flow, reconciliation and the over-attributed row, the full-command cost formula, the stage-reader rule, and a new dated quick sample appended; history untouched), `README.md` (the reconciliation claim drift finding 1 left), `docs/cli/list.md` (`--perf` row), `docs/git-graph.md` (graph rows), skill pages `list.md` (the "flat stages" line), `testing.md`, `git-graph.md`. Comment drift fixed on `RemoteAnswers`, `RemotePlan`, `STAGE_SLACK`, the stale gate's docs, and the `list_prs` comment that named `pr gather`
        - quick release sample (taken, not deferred): release `wt` on the macOS dev host (Mac16,5, 16 cores, load average 8–10, under the core count), `rusty-biscuit` checkout with 10 worktrees, working tree on `2147b2aa2`, unauthenticated (no `GH_TOKEN`/`GITHUB_TOKEN`), non-image stderr. `wt list` runs 2–3: total 323.9–344.1 ms, group 282.6–301.2 ms, `remote wait` 276.5–295.9 ms, `list gather` 188.5–196.2 ms hidden inside it; `-v`: group 351.9–454.1 ms with `list gather` ~187 ms and `verbose gather` 175–181 ms both inside; no regather occurred (tips never moved), so no regather cost was observed. Departure from plan Phase 9's "authenticated": no key was available on the host
        - verification: `just test` 945 passed; `just lint` clean, plus `cargo clippy -p worktree-cli --all-targets -D warnings` clean; `just test-perf` 32 passed (stale gate pr reads 8.9–11.2 ms); `just test-l2 list` 14 passed (tmux); `just check-tier-coverage worktree` no stranded tests. All with `LIBGIT2_NO_PKG_CONFIG=1`
- class sweep: timing projections add concurrently measured durations as if they were sequential and hide the resulting excess instead of measuring the concurrent span directly; sites checked: collector accounting (top-level sum, `unattributed`), metric share projection, first concurrent region (wait, PR reread, list, graph, verbose), regather region, fast-forward and checkout refresh rows, local-only pipeline, `pr gather` meaning, stage parser (`stage_from_perf`, `list_gather_from_perf`), stage readers (`perf_pr_request`, `perf_graph_stages`, `cache_cold_path`, `cache_warm_path`, `perf_flag`, `list_prs`, `list/tests.rs`, `list/tests/pipeline.rs`), justfile perf recipes, repo-wide scripts; fixed here: collector accounting (measured groups, exact reconciliation, visible over-attribution), share projection (children get none), first concurrent region (`remote wait ‖ local gather` / `local gather`), regather region (measured group with concurrent children), `pr gather` (pre-launch only; post-wait `pr reread` child), stage parser (whole-label match at any depth), stale PR gate (sums both reads); clean: fast-forward and checkout refresh rows (already sequential top-level rows), `perf_graph_stages`, cache gates, `perf_flag` and `list/tests.rs` name checks (read by exact label or flattened names), justfile recipes and scripts (no stage parsing)
- work completed for 'The performance report still adds overlapping work' at 16:47:58

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1 hour 58 minutes (14:50:53 to 16:48:40). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no finding was deferred; the spec's quick release-build sample was taken under acceptable load (load average 8–10 on 16 cores), so `deferred_perf_measurement` stays `false`
- decisions the author should confirm in review (each is logged above under its finding):
        - an empty token variable is now treated as unset in `sniff`'s own credential selection, not only in the recorded evidence (`GH_TOKEN=` previously sent `Bearer ` and hid `GITHUB_TOKEN`); this touches the spec's out-of-scope line on credential precedence
        - `sniff` failure keys now come from the request actually sent, and `PrUnavailable::NotFoundOrNotPermitted` gained a `key` field, which let `worktree`'s duplicate `key_in_use` lookup be deleted
        - stored variable names must be upper-case, which is stricter than plan Rule 1, so that a token value can never pass as a name
        - the notice says "for higher rate limits" for GitHub, GitLab, and Bitbucket, and "to authenticate API requests" for Gitea and Forgejo
        - `RefTips` records symbolic remote HEAD targets and includes them in snapshot equality, so a retargeted `origin/HEAD` costs one regather
        - group children in `--perf` show the metrics tree's `—` share instead of a blank, and attributed time beyond the total is shown as an `over-attributed` row instead of being clipped
        - the release sample is unauthenticated because no key was set on the host
- final verification:
        - `worktree/`: `just test` 945 passed, `just lint` clean, `just test-perf` 32 passed, `just test-l2 list` 14 passed (tmux), `just test-l2 credential` passed, `just check-tier-coverage worktree` no stranded tests
        - `sniff/`: `just test` 3127 passed, `just lint` clean, `just check-tier-coverage sniff` no stranded tests
        - host note: a mid-session Homebrew libgit2 upgrade required `LIBGIT2_NO_PKG_CONFIG=1` for builds; recorded in `.claude/skills/os/macos.md`
        - cross-OS evidence (Linux, native Windows, WSL2) was not gathered locally; it is left to CI

The files changed in this cycle:

- `sniff`: `lib/src/credentials.rs`, `lib/src/remote/blocking.rs`, `lib/src/remote/focused.rs`, `lib/tests/l1/branch_head.rs`, `lib/tests/l1/open_pull_requests.rs`, `lib/README.md`, `lib/CHANGELOG.md`
- `worktree` library: `lib/src/listing.rs`, `lib/src/worktree.rs`, `lib/src/pull_requests.rs`, `lib/src/remote_head.rs`, `lib/src/remote_update.rs`, `lib/src/remote_update/tests.rs`
- `worktree-cli`: `cli/src/commands/list.rs`, `cli/src/commands/list/tests.rs`, `cli/src/commands/list/tests/` (new), `cli/src/commands/list/wait.rs`, `cli/src/commands/list/wait/tests.rs`, `cli/src/commands/list_table.rs`, `cli/src/commands/git_graph.rs`, `cli/src/commands/git_graph/tests.rs`, `cli/src/commands/refresh_worker.rs`, `cli/src/perf.rs`, and the tests `level2_list_verbose.rs`, `list_prs.rs`, `list_remote_head.rs`, `list_table.rs` (with its snapshot), `perf_flag.rs`, `perf_pr_request.rs`, `perf_support/mod.rs`
- docs: `worktree/README.md`, `worktree/docs/cli/list.md`, `worktree/docs/git-graph.md`, `worktree/docs/performance-testing.md`
- skills: `.claude/skills/worktree/{list,list-remote,git-graph,testing,create-and-include}.md`, `.claude/skills/sniff/{SKILL,remote-and-repository}.md`, `.claude/skills/os/macos.md`

## Phase 1

> **phase:** rulings, spikes, and baseline (plan Phase 1), 2026-10-03

- state found: review cycle 1 (above) had already built plan phases 2–8 and most of 9, and had run spikes S1 and S2 while doing so. Phase 1 therefore wrote no code. It consolidated the rulings and spike outcomes into `plan.md` ("Implementation log" → "Checkpoint 1 (Phase 1)") and took a fresh baseline
- rulings 1–9: each was checked against the code and recorded as applied. Two places where the code departs from the plan's wording:
        - Rule 1: stored variable names are upper case only (`[A-Z_][A-Z0-9_]*`), so a mixed-case token value cannot pass as a name
        - Rule 5: the post-wait `pr reread` is always a child of the group, because it always falls inside the group's span. The regather group's children are `list regather` and `graph regather` / `verbose regather`
- S1 (appended to Rule 6): filter Git's `%D` with `--decorate-refs-exclude=HEAD|refs/heads/|refs/remotes/` so only tags and other non-branch refs come from Git, and rebuild the branch, HEAD, and remote labels from the snapshot (`RefTips.remote_heads` via `%(symref)`)
- S2: credential selection is captured in `FocusedProviderClient::fetch_json` through a per-client `SentLog`, which `blocking::run_with_deadline` folds. Only two signatures changed: `BranchHead.credentials` and `OpenPullRequests`
- stage-name consumers listed in `plan.md`: `list.rs` and `perf.rs` produce the names; `list/tests.rs`, `list/tests/pipeline.rs`, `perf_support/mod.rs`, `perf_flag.rs`, `perf_pr_request.rs`, and `perf_graph_stages.rs` read them, and `cache_cold_path.rs` / `cache_warm_path.rs` read them through `perf_support`. `list_prs.rs` no longer names a stage
- baseline (with `LIBGIT2_NO_PKG_CONFIG=1`, per the `os` skill's macOS note):
        - `worktree/`: `just test` 945 passed, 32 skipped (no failures); `just lint` clean
        - `sniff/`: `just test` 3127 passed, 32 skipped (no failures); `just lint` clean
        - no pre-existing failures to carry forward
- tests: none added; Phase 1 changes no behavior, so there is no requirement-to-test mapping. The earlier cycle's targeted tests are listed in the review-cycle section above
- frontmatter: the Phase 1 file lists are empty, because only plan, log, and spec files under the fix directory were edited. `spec.md` has `human_review: false` and a `message_to_agent` telling later phases to verify and tick the existing work, not re-implement it
- skills: no change to the `worktree` skill was needed; review cycle 1 had already updated it

## Phase 2

> **phase:** `sniff` credential metadata (plan Phase 2), 2026-10-03

- state found: review cycle 1 had already built this phase (the work is committed). The working tree had no source changes. Each task was checked against the code, no gap was found, and no code changed
- task verification:
        - evidence type: `sniff::remote::blocking::RequestCredentials { Anonymous, Keyed { variables }, Unknown }` (`sniff/lib/src/remote/blocking.rs`). It is plain data, `#[non_exhaustive]`, has no `Serialize`, and holds only variable names
        - selection from the client: `FocusedProviderClient::fetch_json` (`sniff/lib/src/remote/focused.rs`) logs `SentWith` from the same `credential()` call that builds the request's auth header. It does no second lookup and adds no request. An empty variable is unset in `credentials::token_in`, so it is neither sent nor reported
        - blocking entry points: `BranchHead.credentials` and `OpenPullRequests { pull_requests, credentials }` are folded in `run_with_deadline`. A failure is still a `PrUnavailable`, which carries no success evidence; its `key` comes from the last request sent. Deadline handling, classification, and pagination are unchanged
        - callers: `rg` for `branch_head(_with)?(` / `open_pull_requests(_with)?(` finds only `sniff` itself and `worktree/lib/src/{pull_requests,remote_update}.rs`, all compiling (`cargo check --all-targets -p worktree -p worktree-cli` clean)
- requirement-to-test mapping (all L1, `sniff-lib`, existing; none added because none was missing):
        - provider token → `l1::branch_head::a_success_reports_the_provider_token_variable_its_request_sent` (every flavor; token absent from `Debug`; exactly one request)
        - no token → `a_success_without_a_token_reports_an_anonymous_request` (one request)
        - empty variable, and an empty first candidate not hiding the next → `an_empty_variable_is_unset_for_both_the_request_and_its_credentials`
        - host-bound token (success and 404 key; global token ignored) → `a_host_bound_token_is_the_selection_reported_for_success_and_404`; empty host-bound → `an_empty_host_bound_token_leaves_a_discovered_request_anonymous`
        - pagination all anonymous / all keyed / key appears between pages / key disappears between pages, with request counts and token kept out of `Debug` → `l1::open_pull_requests::a_paginated_answer_is_anonymous_only_when_every_page_was`; an empty list still reports credentials → `an_empty_list_reports_its_requests_credentials`
        - fold rules (zero requests → unknown, repeats removed, first-use order) → unit test `remote::blocking::tests::request_credentials_are_anonymous_only_when_every_request_was`
        - token values in serialization: not applicable at this layer, because `RequestCredentials` has no serializer. The stored form is phase 3's `CredentialEvidence`
- input robustness matrix: not applicable. This phase reads no file format; the stored evidence and its matrix belong to phase 3
- OS: no OS-specific code (environment variables plus HTTP to a local wiremock), so no cross-check was run; CI's Linux leg covers the rest
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `sniff/` `just test` 3127 passed, 32 skipped; `sniff/` `just lint` exit 0; `just check-tier-coverage sniff` 0 stranded; `worktree/` `just test` 945 passed, 32 skipped; `worktree/` `just lint` exit 0. No failures, pre-existing or new
- frontmatter: the phase 2 file lists are empty because this phase edited no source, docs, or skill files. `packages` now lists `sniff`, the package this phase's scope covers, whose changes landed in review cycle 1
- skills: no `worktree` skill change needed

## Phase 3

> **phase:** stores, worker, and wait capture (plan Phase 3), 2026-10-03

- state found: review cycle 1 had already built this phase, and the working tree had no source changes. I checked each task against the code and found one test gap, which is now closed (below). No behavior changed
- task verification:
        - serialized evidence: `remote_head::CredentialEvidence` (`#[serde(tag = "state", rename_all = "kebab-case")]`) is read through the strict readers. `is_valid` requires a `keyed` state to have at least one name, and every name must match `[A-Z_][A-Z0-9_]*`. Both writers refuse anything else (`set_credentials` errors; PR `refresh` returns `PublishFailed`), so nothing shaped like a token reaches disk
        - PR store format 6: `PR_STORE_FORMAT_VERSION = 6`. `credentials` is written in the same atomic write as the answer and `publication`, including for an empty list. Format 5 is read through `UncredentialedStoreFile`, whose conversion sets `Unknown` unconditionally. Formats 4 and older, and 7 and newer, are misses. `PrSource::fetch` maps sniff's `RequestCredentials` with `CredentialEvidence::from_sniff`, and an unknown future sniff state maps to `Unknown`
        - head store format 3: `Attempt::credentials` is required and starts `Unknown`. `remote_update::Recorder::credentials` (`set_credentials`) writes it as soon as the API check succeeds, before the answer is published. `set_phase`, `finish_attempt`, and `publish_answer` (`source: fetch`) keep it. Format 2 keeps its answer and drops its attempt, and format 1 migrates as before
        - wait capture: `WaitEnd.pr_credentials` comes from the first new publication the wait sees, in the same read as its id. That includes a lock holder's publication. A later publication never replaces it. A success known only from a receipt, and the answer cached before launch, give `Unknown`. Head evidence is the followed attempt's own field. No budget changed
- matrix (plan Phase 3 table) → tests, all L1 in `worktree` (lib):
        - PR `credentials` (v6), every row (absent, null, wrong whole type, one bad element, every element bad, empty `[]`/`{}`, unknown tag, an invalid name or one shaped like a token, a duplicate key inside the object and in the envelope, trailing content, valid `unknown`/`keyed`) plus a control row → `pull_requests::tests::the_store_reader_walks_the_input_robustness_matrix`, asserted through `select_cached` and `stored_publication`
        - head `attempt.credentials` (v3), the same rows; a bad shape drops the attempt and keeps the answer. Also a duplicate key, trailing content (both halves lost), valid `unknown`/`keyed`, and a control row → `remote_head::tests::the_store_reader_walks_the_input_robustness_matrix`, asserted through `select_cached_head` and `select_attempt`
        - cross-version: PR v5 is read as `Unknown` and v7 is a miss → `a_format_5_answer_is_served_with_unknown_credentials_and_a_future_format_is_a_miss`; PR v1–v4 are misses → `earlier_formats_exactly_as_they_were_written_are_misses`; head v1 → `a_format_1_file_reads_as_a_git_answer_and_a_write_keeps_it`; head v2 → `a_format_2_file_keeps_its_answer_and_drops_its_attempt`
        - the notice side of the matrix (`unknown` and cached evidence never give the notice) → `commands::list::tests::…::unknown_cached_or_receipt_only_evidence_never_gives_the_notice` (CLI); the full notice is plan Phase 8
- other plan tests → existing tests:
        - publication before the receipt → `list::wait::tests::a_publication_before_the_receipt_carries_its_own_credentials`
        - lock contention, a real holder's anonymous `refresh` → `a_contending_holders_real_refresh_is_followed_by_its_publication_id` (ordinary and forced)
        - adoption under another worker's environment → `an_adopted_attempt_carries_the_credentials_its_own_worker_recorded`
        - a later publication is ignored → `a_later_publication_never_replaces_the_accepted_ones_credentials`; the older cached answer is never borrowed → `an_older_cached_answers_credentials_are_never_borrowed`; the publication seen at the timeout does not extend the budget → `a_publication_seen_at_the_timeout_keeps_its_credentials_without_extending_the_budget`
        - worker writes → `remote_update::tests::an_api_answer_records_its_credentials_before_the_answer_and_through_the_fetch` and `a_check_the_api_did_not_answer_has_unknown_credentials`
        - no token values in files → `remote_head::tests::credentials_survive_every_later_write_of_the_attempt_and_never_hold_a_value` and `pull_requests::tests::a_publication_records_what_its_request_was_sent_with_and_only_names`
- gap closed: no test pinned that a format-5 PR file with a stray `credentials: {"state": "anonymous"}` still reads as `unknown`. The format-5 reader ignores unknown fields, so this held only because the conversion hardcodes `Unknown`. I added that case to `a_format_5_answer_is_served_with_unknown_credentials_and_a_future_format_is_a_miss` (`worktree/lib/src/pull_requests.rs`). It passes, and a future change that read the field would fail it
- note: the wait tests' `stored_prs::store_json` helper still writes format 5. That is intentional and correct: format 5 is readable, and its evidence is `unknown`. The contention test that needs real evidence uses the real `refresh` writer
- smell grep (`serde(default)`, `Option` where absent and null must differ, `filter_map`/`unwrap_or_default`/`.ok()` on the credentials parse): clean. The only hits are the pre-existing whole-file `.ok()?` reads, which turn a malformed file into a miss and never into anonymous, plus unrelated `unix_now` and PR-summary code
- OS: no OS-specific code in this phase (JSON stores under a temp directory), so no cross-check was run
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `worktree/` `just test` 945 passed, 32 skipped (the count is unchanged because the addition is an assertion in an existing test); `just lint` exit 0. No failures, pre-existing or new
- skills: no `worktree` skill change needed; `list-remote.md` already describes the format-6 and format-3 evidence

## Phase 4

> **phase:** library gather separation (plan Phase 4), 2026-10-03

- state found: review cycle 1 had already built this phase, and the working tree had no Phase 4 source changes. I checked each task against the code and found one test gap, which is now closed (below). No behavior changed
- task verification:
        - split: `fill_worktree_statuses` (`worktree/lib/src/worktree.rs`) is now `load_comparison_cache` + `WorktreeList::gather_local` (dirtiness via `gather_dirtiness`, run concurrently with `gather_ref_facts`) + `WorktreeList::commit`. The two gathers write nothing persistent. `commit` saves the cache, prunes fork records, and prunes copy records, each once
        - compose: `list_worktrees` still calls `parse_worktree_state` and `fill_worktree_statuses`, whose composition is gather then commit on the parse step's read. The `list_worktrees_*` warm/cold subprocess-count tests pass unchanged
        - in-memory cache: one `Mutex<Cache>` from `load_comparison_cache` serves every gather of a listing. `compare_cached` adds only successful comparisons (`compare_live` returns `None` before `put`), and `commit` saves the whole cache once, so a discarded gather's entries are kept
        - targeted dirtiness: `WorktreeList::refresh_dirty_status(checkout)` measures one entry again and replaces only that result. It returns `false` when no entry has that path
        - snapshot comparison: `listing::RefSnapshot { tips, succeeded, read_at }`. `matches` compares the complete `local`, `remote`, and `remote_heads` maps, and it is false when either read failed, even two failed reads compared with each other
        - prune guard: fork pruning runs only when `refs.succeeded()`. It reloads the store from disk and protects a record no older than `read_at` with a `rev-parse --verify`. Copy-record pruning still checks for live worktrees (canonicalized entry paths)
        - docs on touched symbols: the `///` docs on `fill_worktree_statuses`, `gather_local`, `gather_ref_facts`, `commit`, `refresh_dirty_status`, `RefSnapshot::matches`, and `compare_cached` match the code; no drift found. `rg` finds no stale `reread_refs` / `refs_read` / `refs_read_at` in code or in docs outside the fix directories
- requirement-to-test mapping (all L1, `worktree` lib, in `listing::tests` / `listing::repo_tests`):
        - persistence only in the commit step; a discarded gather's cache entries survive → `gathers_persist_nothing_until_one_commit_that_keeps_every_comparison` (cache file, fork store, and copy record all checked before and after `commit`; the initial and final comparisons are both cached)
        - **added:** the shared cache seeds the final gather, and failures stay uncached → `the_shared_cache_seeds_the_final_gather_and_never_holds_a_failure`. A gather over tips naming a missing object leaves no entry in memory or on disk, and a second gather over unchanged tips issues no `rev-list`/`merge-tree`. Mutation check: bypassing the cache lookup in `compare_cached` makes the test fail
        - a `wt create` record written during the wait survives the prune → `a_fork_record_written_while_the_listing_waits_survives_its_prune`
        - a failed read never prunes → `a_commit_after_a_failed_ref_read_never_prunes_fork_records`
        - a failed read never matches; additions, deletions, and advances do not match; an empty repository read twice matches → `only_two_successful_reads_of_equal_maps_match`
        - a targeted refresh measures only the named checkout (one `git status`), and a path matching no entry returns false → `a_dirtiness_refresh_measures_only_the_named_checkout_again`
        - library-caller behavior unchanged → the existing `worktree::tests::list_worktrees_*` subprocess-count tests
- input robustness matrix: not applicable. This phase reads no file format
- OS: no OS-specific code (scoped threads and git subprocesses already used before this phase), so no cross-check was run
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `worktree/` `just lint` exit 0; `just check-tier-coverage worktree` 0 stranded; `worktree/` `just test` was run four times:
        - two full runs passed, 946 tests (945 plus the new one), 32 skipped
        - one run stopped early on a failure; the reruns showed no failure
        - one run failed `worktree-cli::list_prs a_held_pr_request_with_nothing_stored_ends_at_the_budget_with_only_the_hint` (11.8 s; it asserts the whole command takes under 5 s). Five isolated runs of that test passed (about 3.7 s each). It is a load-sensitive wall-clock bound in Phase 6's territory and is unrelated to this phase's test-only change, so it is flagged in `spec.md` `message_to_agent` and was not changed here
- skills: no `worktree` skill change needed; `list.md` already describes the gather/commit split
- re-verification run (2026-10-03, same phase): `just lint` exit 0; `just check-tier-coverage worktree` 0 stranded; `just test` run three times. The first stopped on `worktree-cli::list_prs a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` (11.2 s under full-suite load). It passed three of three isolated runs (about 3.9 s each), and the next two full runs passed, 946 tests, 32 skipped. This is a second `list_prs` test with the same load-sensitive timing as the one above, so the Phase 6 note in `spec.md` now names both

## Phase 5

> **phase:** snapshot-addressed graph and verbose gathering (plan Phase 5), 2026-10-03

- state found: review cycle 1 had already built this phase, and the working tree had no source changes. I checked each task against the code and found one test gap, which is now closed (below). No behavior changed
- task verification:
        - tips as input: `GatherInput::from_list(list, refs)` (`worktree/cli/src/commands/git_graph.rs`) takes the parse step's `WorktreeList` (entries, default branch, fork records) plus an explicit `RefTips`. It reads nothing the caption, target, or tree produce, so `run_pipeline` builds it before the local gather from `initial.tips()` and, for a regather, from `accepted_refs.tips()` (`list.rs`). The plan's "derive what it needs from `build_tree`" was not needed: the gatherer uses no tree facts
        - object IDs only: every revision the graph and verbose gatherers hand to Git comes from `input.refs` (`DefaultTips::read`, `recorded_parent`, the current tip, fork records' `base_sha`) or from Git output. `topology.rs` (`merge-base`, `rev-list`, `log --no-walk`) and `commit_details{,_since}` receive only SHAs or `SHA..SHA` ranges
        - verbose labels: per the S1 ruling, `LIVE_DECORATION_EXCLUDES` leaves Git's `%D` only tags and other non-branch refs, and `snapshot_labels` rebuilds `HEAD -> <current>`, remote-tracking names (with symbolic `origin/HEAD` from `RefTips.remote_heads`), and local branches from the snapshot in Git's order
        - preserved: `incomplete` still comes from `History::read` (shallow check, graph only), a `merge-base` gap, and placement or lane gaps. The graph and verbose conditions (`needs_graph`, `has_verbose`, detached → nothing) are unchanged; `nothing_is_gathered_when_detached_or_not_needed` and the existing git-graph suites pass
- requirement-to-test mapping (all L1, `worktree-cli`, in `commands::git_graph::tests`; each runs under both the library and `bin/wt` targets):
        - `%D`-equivalent labels match the snapshot, with tag and HEAD cases; equal to Git's own `%D` when nothing moved → `verbose_labels_follow_the_snapshot_and_keep_live_tags` (existing)
        - **added:** a ref moved after capture does not change gathered output → `refs_moved_after_the_snapshot_leave_the_gather_unchanged`. After the snapshot, both focused-view branches advance, `origin/main` is fetched forward, `feature-b` is deleted, and `late` is created. The focused graph, verbose (merge base and branch commits with their SHAs, messages, and labels), and base-view graph all equal the first gather. A recorder also asserts that no Git call names a ref (`HEAD`, a branch, `origin/main`, `origin/HEAD`, including either side of a `..` range)
        - mutation check: making verbose pass the branch name `current` instead of `current_tip` to `commit_details_since` fails the new test (an extra `a2` commit appears). The file was restored from a copy, and `git diff` shows no change to `git_graph.rs`
- input robustness matrix: not applicable. This phase reads no file format
- OS: no OS-specific code; the test uses Git subprocesses in a temp repo, like its neighbors, so no cross-check was run
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `worktree/` `just test` 948 passed (946 plus the new test under two targets), 32 skipped; `worktree/` `just lint` exit 0; `just check-tier-coverage worktree` 0 stranded. Checkpoint 5: `cargo nextest run -p worktree-cli git_graph perf_graph_stages` 104 passed. No failures, pre-existing or new; the load-sensitive `list_prs` timings noted in Phase 4 did not recur
- frontmatter: `packages` now adds `worktree-cli`, the crate this phase covers
- skills: no `worktree` skill change needed; `git-graph.md` already describes snapshot addressing and the verbose labels

## Phase 6

> **phase:** overlapped pipeline and acceptance (plan Phase 6), 2026-10-03

- state found: review cycle 1 had already built this phase (`gather_listing` in `worktree/cli/src/commands/list.rs`), and the working tree had no change to it. I checked each task against the code and closed two test gaps and one timing-bound decision handed forward from Phase 4. `list.rs` is unchanged; no behavior changed
- task verification:
        - reorder: `prepare_remote` (origin lookup, `--ignore-api` write) runs before `std::thread::scope`; inside it, scoped threads run `gather_local` (dirtiness ∥ ref facts, per-worktree parallel in the library) and `git_graph::gather` from `initial.tips()`, while the calling thread runs `follow_remote` (launch, wait, spinner, PR reread). The budget starts in `wait::wait`, before the launch
        - join, then `--ff`: both scoped handles are joined inside the scope, before `fast_forward_default`; the worker handle is never joined
        - accept or regather: reread only when `remote.waited.is_some() || ff.is_some()`; `RefSnapshot::matches` accepts only two successful equal reads; otherwise one regather (ref facts ∥ history) from the final snapshot, no loop
        - dirtiness reuse: initial results kept; `refresh_dirty_status` only for `FfResult::Moved { checkout: Some(_) }`
        - commit once: `WorktreeList::commit(accepted_refs, …)` once, then `run_pipeline` renders once with the PR answer applied in `TableFacts`
        - exit paths: the only fallible steps (`parse_worktree_state`, `prepare_remote`) run before any spawn; `gather_local`/`gather` return values, not errors, so a local failure is degraded output rather than an exit; the spinner is cleared inside `follow_remote` before it returns; scoped tasks never print
- requirement-to-test mapping (L1, `worktree-cli`, `commands::list::tests::pipeline` unless noted; each runs under the library and `bin/wt` targets):
        - overlap with the remote wait → `the_local_gathers_start_while_the_worker_outcome_is_held` (existing)
        - unchanged refs, status and comparisons once → `unchanged_tips_accept_the_first_gather_and_measure_everything_once` (existing)
        - **added:** no-origin and non-image control paths → `without_an_origin_or_an_image_the_single_gather_is_accepted`. No origin: no launch, `waited` is `None`, one ref read, group `local gather` (no `remote wait`), no regather, no caption (removing `origin` removes its tracking refs). Non-image: launch, two ref reads, group without `graph gather`, no graph, no regather. Both describe the final state
        - **added:** a preference-write error exits before speculative work → `a_failed_preference_write_starts_no_local_gather_and_launches_nothing` (`--ignore-api` with a local-path origin): `NoRepositoryIdentity`, no launch, neither local gather started (new `overlap::Installed::started`), no cache saved, nothing pruned
        - advance/rewind/addition/deletion reflect the final tips → `a_ref_change_during_the_wait_is_regathered_from_the_final_tips` (existing)
        - `--ff` moved with a holder / without / up to date / refused → `fast_forward_measures_again_only_the_checkout_it_moved` (existing)
        - failed initial or final read → `a_failed_ref_read_never_counts_as_unchanged_and_a_failed_final_read_never_prunes` (existing)
        - persistence once, speculative cache entries kept → `nothing_persists_before_the_accepted_gather_is_committed_once` (existing)
        - timeout → `a_timed_out_wait_keeps_pending_status_and_lets_a_longer_local_gather_finish` (existing); the budget bound itself → `gather::a_silent_worker_is_waited_for_only_until_the_budget` (existing)
        - ignored API: covered end to end by `list_flags::ignore_api` (existing); not repeated in-process, because it needs a `~/.wt.json` under an overridden home
- mutation checks (each applied to a copy of `list.rs`, then restored; `git diff` shows `list.rs` unchanged): always rereading refs fails the no-origin case (2 reads, expected 1); moving `prepare_remote` inside the scope, after the local spawn, fails the preference-write test
- decision handed forward from Phase 4 (`list_prs.rs` timing bounds): `a_held_live_head_check_holds_the_listing_only_until_its_deadline` and `a_held_pr_request_with_nothing_stored_ends_at_the_budget_with_only_the_hint` bounded the whole `wt list` process at < 5 s, which includes the local gather. Since the overlap, the spec says a timeout limits the wait, not local computation, so both now bound the `remote wait` row of `--perf` (which `list_with` already passed) with the same limits; `.output()` returning while the request is held still proves the worker was not joined. `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` asserts no duration, so its one slow failure under load was not a command-duration bound and was left alone. The full-command timing gates stay in `perf_pr_request.rs` (`just test-perf`). Recorded in the skill's `testing.md`
- input robustness matrix: not applicable. This phase reads no file format
- OS: no OS-specific code; the tests use Git subprocesses in temp repos and in-process seams, like their neighbors. No cross-check run
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `worktree/` `just test` 952 passed (948 plus 2 new tests × 2 targets), 32 skipped, both before and after the `list_prs.rs` edit; `just lint` exit 0; `just test-l2 list` 14 passed (the first attempt ended on a signal while blocked on Cargo's package-cache lock held by another process, before running any test; the rerun passed); `just check-tier-coverage worktree` 0 stranded; `--test list_prs` 31 passed, and the two edited tests 3 of 3 in isolation. No pre-existing failures observed
- frontmatter: `packages` unchanged (`worktree-cli` already listed)

## Phase 7

> **phase:** `--perf` nested groups (plan Phase 7), 2026-10-03

- state found: review cycle 1 had already built this phase (`worktree/cli/src/perf.rs`, wired into `gather_listing` in `list.rs`), and the working tree had no change to it. I checked each task against the code and closed one test gap. `perf.rs` and `list.rs` are unchanged; no behavior changed
- task verification:
        - group API: `PerfCollector::record_group(name, elapsed, children)` stores the group's own measured span, not a sum or maximum of its children, and keeps the children in order. `to_metric_node` gives children the metrics tree's `—` share, never a percentage. `recorded_stages` (test-only) flattens each group into its name, then its children
        - reconciliation: `reconcile` sums only top-level rows. `unattributed` is `total − rows` through `checked_sub`, with no `saturating_sub`; any excess appears as the visible `OVER_ATTRIBUTED` row
        - rendering: the biscuit-terminal `MetricsTree` inside the yellow `BlockQuote`. The group is `remote wait ‖ local gather` when the wait ran (children `remote wait`, `pr reread`, `list gather`, plus `graph gather` or `verbose gather` when gathered), and `local gather` otherwise. The `regather` group (`list regather`, then `graph regather` or `verbose regather`) is recorded only on the regather branch. `fast-forward` and `checkout status refresh` are separate top-level rows
        - callers: already wired in `gather_listing` (Phase 6). `perf_support::perf_rows` / `stage_from_perf` parse depth and match whole labels at any depth
- requirement-to-test mapping (L1, `worktree-cli`):
        - reconciliation with nested overlap → `perf::tests::overlapping_group_children_are_excluded_from_the_top_level_sum`, `rows_beyond_the_elapsed_time_are_surfaced_not_clipped`, `only_top_level_rows_carry_a_share` (existing)
        - groups beside sequential rows, and the flattening reader → `perf::tests::groups_keep_their_measured_span_and_sit_beside_sequential_rows` (existing)
        - conditional graph and verbose rows → `list::tests` local-only group assertions, plus `perf_flag::list_perf_non_image_terminal_omits_graph_stages` and `list_perf_non_image_verbose_includes_verbose_gather` (existing)
        - local-only group with no `remote wait` → `perf_flag::list_perf_reports_a_local_only_group_that_reconciles` (existing; it now uses the shared helpers below)
        - `regather` only when needed; `fast-forward` and the checkout refresh rows → the `list::tests::pipeline` cases with `assert_perf_reconciles` and `perf_group` (existing)
        - **added:** the remote group as the shipped binary actually renders it → `perf_flag::list_perf_reports_the_remote_group_from_the_real_renderer` (FakeGitea origin answering an empty PR list; no network). Before this, the parser's view of a `remote wait ‖ local gather` group was checked only against the hand-written `NESTED` string, so renderer drift could go unseen. The test asserts the group children in order (`remote wait`, `pr reread`, `list gather`); `pr gather` at top level; no `local gather`, `regather`, `fast-forward`, or `checkout status refresh` row; `remote wait` ≤ the group; and top-level reconciliation within display rounding
        - refactor in the same file: the local-only test's reconciliation and group-children code moved into `assert_top_level_reconciles` and `group_children`, which both tests use. The assertions are the same
- mutation check: recording `pr reread` as a top-level row instead of a group child (applied to a copy of `list.rs`, then restored; `git diff` shows `list.rs` unchanged) fails the new test (`["remote wait", "list gather"]` vs the expected three children)
- snapshot: none added. The double-compile rule allows one only in an integration test, and row-shape assertions on real output cover the same contract without pinning durations
- input robustness matrix: not applicable. This phase reads no file format
- OS: no OS-specific code. The test reuses the `perf_support` stand-ins (`FakeGitea`, `WorkerReaper`) that `list_prs.rs` already runs on every OS. No cross-check was run
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `worktree/` `just test` 953 passed (952 plus the new test, in a single-target integration binary), 32 skipped; `just lint` exit 0; Checkpoint 7 `just test-perf` 32 passed; `--test perf_flag` 11 passed; `just check-tier-coverage worktree` 0 stranded. No pre-existing failures observed
- frontmatter: `packages` unchanged (`worktree-cli` already listed)
- skills: no `worktree` skill change needed; `testing.md` already records where perf parser tests go and the stage-reader rule

## Phase 8

> **phase:** keyless notice (plan Phase 8), 2026-10-03

- state found: review cycle 1 had already built this phase (`observed_keyless`, `credential_line`'s third step `CredentialCondition::AnsweredWithoutKey { higher_limits }`, and `keyed_limits_are_higher` in `worktree/cli/src/commands/list.rs`; the line renders through `list_table.rs`), and the working tree had no change to it. I checked each task against the code and closed one test gap. `list.rs`, `list_table.rs`, and `wait.rs` are unchanged; no behavior changed
- task verification:
        - evidence in `RemoteAnswers`: head evidence is `followed_attempt(&waited.head)`'s `credentials` (a `Running { last }` attempt is filtered to the followed id, so an earlier run's attempt is never read); PR evidence is `WaitEnd.pr_credentials`, set only by the first publication whose id differs from the one stored before launch. Without a wait (`waited: None`), `observed_keyless` is false, so cached-only answers never count
        - one line: `credential_line` tries the attempt's confirmed condition, then this run's confirmed PR failure, then the notice. Either half alone is enough, and neither a keyed success nor a generic failure in the other half hides it
        - suppression: ignored, changed origin (`remote.origin_changed`, or the attempt's `Unavailable` outcome), `unknown` evidence, no wait, and an anonymous API failure alone give no notice. Unsupported and local-path origins have no `credential_env`, so they give no line
        - text: provider and variables come from `credential_env`; "higher rate limits" only for GitHub, GitLab, and Bitbucket. Names only; the line is the existing dim credentials line beneath the caption
        - fallback coexistence: `fallback_notice` is unchanged and computed independently
- requirement-to-test mapping (L1 unless noted):
        - head only, PR only, empty PR answer, both, mixed keyed/anonymous, generic failure in the other half → `commands::list::tests::…::keyless::either_half_alone_or_both_give_one_notice` (existing)
        - success while fetching, adopted attempt → `…::keyless::an_answer_still_fetching_or_from_an_adopted_attempt_counts` (existing)
        - unknown, receipt-only, no wait, anonymous failure alone → `…::keyless::unknown_cached_or_receipt_only_evidence_never_gives_the_notice` (existing)
        - ignored, changed origin (both forms), unsupported, local path → `…::keyless::ignored_changed_origin_unsupported_and_local_path_remotes_never_give_the_notice` (existing)
        - warning precedence → `…::keyless::a_confirmed_warning_outranks_the_notice` and `a_confirmed_head_condition_outranks_this_runs_pr_failure` (existing)
        - provider wording → `…::keyless::the_notice_promises_higher_limits_only_where_the_provider_gives_them` (existing); text snapshot for every provider and condition → `list_table::credential_lines_snapshot_every_condition_for_every_provider`; dim, directly below the caption → `list_table.rs` (existing)
        - publication before receipt, lock contention, later publication ignored → the Phase 3 `list::wait::tests` (existing)
        - shipped binary → `list_prs::an_anonymous_pr_answer_with_a_failed_head_check_shows_the_keyless_notice`, `an_anonymous_head_answer_with_a_failed_pr_request_shows_the_keyless_notice_through_the_fetch`, `two_anonymous_answers_show_exactly_one_keyless_line_and_a_key_shows_none` (position beneath the caption, no token in stores or output), `the_keyless_notice_coexists_with_the_closing_fallback_notice` (existing)
        - **added:** cached and ignored suppression through the shipped binary → `list_prs::stored_anonymous_evidence_from_an_earlier_listing_never_gives_the_notice`. A control listing gives the notice. A second listing, whose own head and PR requests fail with 500 (both re-asked, and the PR store still holds the earlier `anonymous` publication), gives none. On Unix, a third listing with `--ignore-api` asks nothing and gives none (`~/.wt.json` needs `HOME`, as in `list_flags::ignore_api`). Before this, cached suppression was proved only in-process
        - L2 → `level2_list_verbose::level2_list_keyless_notice_is_a_dim_line_beneath_the_caption_after_the_spinner_in_tmux` (existing; windowless tmux, no focus)
- mutation check: removing the "publication differs from the one before launch" condition from `Wait::published` (on a copy of `wait.rs`, then restored; `git diff` shows `wait.rs` unchanged) fails the new test at the second listing's no-notice assertion
- input robustness matrix: not applicable. This phase reads no file format; the stored-evidence matrix is Phase 3's
- OS: no OS-specific code. The new test's `--ignore-api` step is `#[cfg(unix)]` for the same `HOME` reason as `list_flags::ignore_api`; the rest runs on every OS with the `FakeGitea` stand-in. No cross-check run
- gates (with `LIBGIT2_NO_PKG_CONFIG=1`): `worktree/` `just test` 954 passed (953 plus the new single-target test), 32 skipped; `just lint` exit 0; `just test-l2 credential` 1 passed and `just test-l2 keyless` 1 passed (tmux, `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`); `just check-tier-coverage worktree` 0 stranded
- pre-existing flake: one standalone `--test list_prs` run failed `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` at 11.3 s (load average about 24). It passed alone (3.9 s) and in three more full `list_prs` runs (32 of 32 each), and inside `just test`. This is the same intermittent failure logged in Phases 4 and 6; this phase did not touch that test
- frontmatter: `packages` unchanged (`worktree-cli` already listed)
- skills: no `worktree` skill change needed
