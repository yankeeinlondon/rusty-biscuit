---
implementation_1: "2026-07-17T10:46:27-07:00"
implementation_2: "2026-07-17T11:40:50-07:00"
implementation_3: "2026-07-17T14:36:02-07:00"
implementation_4: "2026-07-17T17:09:45-07:00"
implementation_5: "2026-07-17T22:33:06-07:00"
implementation_6: "2026-07-18T05:34:55-07:00"
implementation_7: "2026-07-18T08:12:07-07:00"
implementation_8: "2026-07-18T09:46:42-07:00"
implementation_9: "2026-07-18T11:40:37-07:00"
implementation_10: "2026-07-19T20:25:49-07:00"
deferred_perf_measurement: true
---

## Implementation of Review Findings #7

> **started at:** 2026-07-18T08:12:07-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-7.md'
- this is iteration 7 of the review-to-implement cycle
- review-7 contains 2 findings (1 High, 1 Medium):
        1. High: native Linux and Windows Level-1 execution and matched work-count artifacts are still absent
        2. Medium: the claimed-unreachable Unix process escape is reachable through installer call sites
- starting the work on 'installer-reachable Unix containment claim (review-7 Medium)' at 08:14:20

> **Reconstructed record.** Everything below this line was written during review cycle 8 (2026-07-18),
> not contemporaneously. The cycle-7 session recorded only its start and the single "starting the work"
> line above before it stopped writing to this log; the source and documentation it produced
> nevertheless landed in `HEAD`. This block is reconstructed strictly from primary evidence — the two
> cycle-7 commits `882f5538b` and `98fa4d992`, [review-7.md](review-7.md), and the "Review 7 deferred
> items" section of [deferred-perf-tests.md](deferred-perf-tests.md). Claims that those sources do not
> support are marked **unrecorded** rather than reconstructed by inference.

- starting the work on 'native Linux and Windows Level-1 execution and matched work-count artifacts (review-7 High)' — **start time unrecorded**
        - `uname -a` identified the execution host as native arm64 macOS (Darwin 25.5.0, `xnu-12377.121.10`, `RELEASE_ARM64_T6041`); `rustup target list --installed` reported `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, and `x86_64-pc-windows-gnu` — no Linux target, and the Windows target cannot execute on this host
        - `git branch -r --contains c32f78e43139868cf5831905e891c388d5fa3e74` returned no remote branch, so the reviewed cycle-6 commit existed only in this local worktree and no hosted CI run for that SHA could exist — which is exactly the review's own observation that no retained run is publicly discoverable
        - Docker was **explicitly not attempted**: a Linux container on this host would exercise real `/proc` descendant discovery, but review-7 ruled Docker results inadmissible for this finding, so a cold container build of the workspace would have spent a large budget on evidence already rejected
        - deferred as a platform and execution-authority constraint, not a CPU-load deferral; `deferred_perf_measurement: true` remains set in this log's frontmatter
        - the cycle-7 entry recording the exact implementation identifier, the Unix-fixture relevance note, and the closure procedure was appended to `sniff/features/2026-07-16-performance/deferred-perf-tests.md` and committed as `882f5538b`
- work completed for 'native Linux and Windows Level-1 execution and matched work-count artifacts (review-7 High)' — **completion time unrecorded**; the containing docs commit `882f5538b` is timestamped 08:26:24
- resuming the reconstruction of 'installer-reachable Unix containment claim (review-7 Medium)' (started 08:14:20)
        - the finding rejected the cycle-6 justification that "no caller in this crate can reach the residual". The installer path routes third-party package managers (Brew, npm, pip, Cargo, Go) and downloaded shell installers through `run_command_with_timeout`, so the crate cannot assert that none of them forks and detaches during the 250 ms sampling gap
        - of the three resolutions review-7 offered, cycle 7 took the second — keep installer execution outside the stronger supervision claim and **expose its best-effort timeout semantics to callers** — plus the review's mandatory executable record
        - the resulting change is commit `98fa4d992` ("perf(sniff): surface installer timeout outcome to callers"), 4 files, +225/-8:
                - `sniff/lib/src/programs/install/options.rs` gained the public field `InstallCapturedResult::timed_out`, letting a caller distinguish a killed-at-deadline install from a clean non-zero exit
                - `sniff/lib/src/programs/install/execute.rs` sets it via `matches!(e, ProcessError::Timeout)` on the captured-error arm of every runner that goes through `run_command_with_timeout`, and `false` on every success/skip arm
                - `process` module docs and `run_with_timeout`'s rustdoc were tightened to admit the install boundary can reach the Unix residual; a `## Notes` block on `run_command_with_timeout`, `execute_install_captured`, and `execute_versioned_install_captured` records that `timed_out` is **not** a promise the install stopped
                - `sniff/docs/sniff-library-architecture.md`'s residual paragraph was corrected to agree with the source: the gap is reachable through the install boundary, not merely theoretical, and it cross-references `InstallCapturedResult::timed_out`
        - the executable record the review demanded was added to `sniff/lib/src/process.rs`: Level-1 fixture `child_detaches_between_samples`, its child `between_samples_descendant`, and the assertion `a_descendant_that_detaches_between_samples_escapes_containment` — unlike the cycle-6 fixture, this one detaches wholly between two samples instead of remaining a descendant across three intervals
        - an `EscapedDescendant` RAII helper SIGKILLs the 30-second sleeper on every exit path so the fixture cannot leak a process
        - **not addressed in cycle 7, and each became a review-8 finding:** the new public field is a source break with no recorded migration or approved contract (review-8 High #2); `execute_install`/`execute_versioned_install`, the interview, and the CLI all discard `timed_out`, so no user-facing warning was delivered (also review-8 High #2); and the new fixture could return green without running its residual assertion when timing crossed a sample boundary (review-8 Medium #3). All three were fixed in cycle 8
- work completed for 'installer-reachable Unix containment claim (review-7 Medium)' — **completion time unrecorded**; the containing source commit `98fa4d992` is timestamped 08:26:39

### Successful Completion

> **Reconstructed during cycle 8.** No contemporaneous completion record was written. The wall-clock
> duration is therefore bounded rather than measured: the cycle started at 08:12:07 and its last
> commit landed at 08:26:39, so it ran for **at most 14 minutes and 32 seconds**. The exact end time
> is unrecorded.

During this implementation both review-7 findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 was fixed and 1 was deferred (see reason below):

- **Finding 1, native Linux and Windows Level-1 execution and matched three-OS work-count artifacts** — deferred because this native arm64 macOS host has no authorized native Linux or Windows execution path, and because the reviewed tree was never published. Closing the finding requires the reviewed commit to be committed and pushed so the `sniff-cross-platform` and `sniff-performance` matrices can execute it natively; the session prohibited committing, pushing, and invoking credential helpers. Cross-compilation proves compilation only, Docker was ruled inadmissible by the review itself, and a workflow definition is not an execution record. This is a platform and execution-authority constraint, not a CPU-load deferral. Full detail and the closure procedure are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.

The files changed cover the public installer timeout disclosure (`InstallCapturedResult::timed_out`), its assignment across every captured install runner, the narrowed process-containment rustdoc on both the runner and the install entry points, the corrected residual paragraph in `sniff/docs/sniff-library-architecture.md`, and the between-samples Unix Level-1 fixture with its RAII cleanup guard.

- **verification figures are unrecorded.** Cycle 7 wrote no `just test`, `just lint`, `git diff --check`, or cross-compile result to this log, and neither cycle-7 commit message states one. No test or lint count is reconstructed here, because inventing one would be indistinguishable from a measured result.
        - the nearest corroborating evidence is external and post-dates the cycle: review 8, run against `98fa4d992` at 08:27:46, independently measured **1,671 `sniff-lib` passed** (19 skipped) and **781 `sniff-cli` passed** (3 skipped) with `just lint` and `just build` green. That is consistent with cycle 6's 1,670 plus the single fixture cycle 7 added, but it is review-8's measurement of the tree, not cycle-7's own record of its work
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command is recorded for this cycle

## Implementation of Review Findings #3

> **started at:** 2026-07-17T14:21:43-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- review-3 contains 8 findings (4 High, 4 Medium):
        1. High: the aggregate builder still reads manifests, and its purity test cannot see that path
        2. High: bare aggregate JSON performs more than one status walk in a linked worktree
        3. High: subprocess detection still bypasses the deadline and the helper is not process-tree bounded
        4. High: the canonical Level-1 suite is still red and cross-platform completion is unproven
        5. Medium: the Node/uv manifest-store fix does not cache failures or verify detection-level reuse
        6. Medium: aggregate context uses string-prefix area matching
        7. Medium: the specified Criterion workload families remain absent
        8. Medium: phase and completion records still describe superseded source
- starting the work on 'aggregate builder manifest reads + blind purity test (R2.7/R2.6)' at 14:25:30

## Implementation of Review Findings #2

> **started at:** 2026-07-17T11:40:50-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review-2 contains 6 findings (4 High, 2 Medium):
        1. High: aggregate JSON builder still performs host/filesystem observation
        2. High: per-detection manifest store not used by non-Cargo workspace discovery (Node, uv)
        3. High: subprocess execution still neither universal nor end-to-end bounded (BurntToast probe, process-tree kill)
        4. High: canonical Level-1 suite red — two snapshots depend on ambient host config
        5. Medium: specified Criterion workload families still absent
        6. Medium: phase and completion records contradict implemented source
- starting the work on 'aggregate JSON builder purity (R2.7)' at 11:43:23
        - located the residual: `build_aggregate_value` (`sniff/cli/src/output/repo_json.rs:713`) calls seven cwd-context helpers — `current_package_area_is_dirty`, `package_area_source_code_change_count` (`output/filesystem/mod.rs:1288,1365`), `render_repo_package`, `render_repo_package_root` (`output/filesystem/packages.rs:457,484`), `render_repo_package_area`, `render_repo_area`, `render_repo_package_area_root` (`output/filesystem/package_areas.rs:374,389,407`)
        - each helper calls `resolve_dir(base_dir)` (`mod.rs:1445`, falls back to `std::env::current_dir`) plus `RepoInfo::package_for_dir` / `package_area_for_dir` / `package_area_label_for_dir` / `area_for_dir`, each of which rebuilds a `PackageOwnershipIndex` and re-canonicalizes root + every package path + the query (`types.rs:274,332,314,367`); `area_for_dir` alone builds three indexes per call
        - the flow: `commands.rs:1662-1675` resolves `dir = base_dir.unwrap_or(".")`, calls `observe_repo_aggregate(dir, filesystem)` (the observation phase), then `build_aggregate_value(&result, &aggregate, &options)` with `options.base_dir`
        - plan: precompute the seven cwd-context facts inside `observe_repo_aggregate` onto a new `RepoAggregate.context` field, reusing one `PackageOwnershipIndex` and one dir canonicalization; `build_aggregate_value` becomes a verbatim projection and drops `AggregateRenderOptions.base_dir`
        - noted a second, un-named residual: `aggregate_repo_version` → `bare_aggregate_version` → `aggregate_versions` re-reads package manifests with counted probes/opens (`aggregate.rs:309,378`) when packages exist; it is the documented "approach A" trade-off, is not named by the finding, and is invisible to the work-count test (its fixture carries `packages: None`); scoping it out but will flag it in the report
        - implemented at 12:24: `RepoAggregate` gained a `context: AggregateCwdContext` field computed inside `observe_repo_aggregate` (`aggregate_view.rs`) from the detection pass's `RepoInfo`/`GitInfo` over one `PackageOwnershipIndex` and one dir canonicalization; `PackageOwnershipIndex` gained a `root()` accessor and `RepoInfo` gained `pub(crate)` `*_with_index` lookup variants (`types.rs`) that the public `package_for_dir`/`package_area_for_dir`/`package_area_label_for_dir`/`area_for_dir` now delegate to (identical semantics, one index per call instead of up to three)
        - `build_aggregate_value` is now a verbatim projection; `AggregateRenderOptions` is deleted (it carried only `base_dir`) and `commands.rs` no longer hands the builder a directory to resolve
        - the work-count test is renamed `build_aggregate_value_performs_no_observation` and asserts `report.counters.is_empty()`; `aggregate_projects_supplied_facts_verbatim` now pins that context facts project verbatim from the aggregate
        - added 7 observation-side tests in `aggregate_view.rs::tests::cwd_context` covering member-dir facts, owning-area attribution, docs-vs-source, root-area inclusion/exclusion, absent-repo defaults, and the non-monorepo `area` gate
        - targeted runs green: sniff lib 86/86 (incl. all new cwd_context tests), sniff-cli repo_json 79/79 (incl. renamed purity test)
        - full verification at 12:35: sniff lib 1,641/1,641 passed (2 skipped); sniff-cli 778 run / 777 passed / 3 skipped / 1 failed — the one failure is the pre-existing `os_json_snapshot` host drift (macOS 26.5.1 → 26.5.2, finding 4's scope), not this change; `repo_aggregate_json_snapshot` passes unchanged, proving the JSON contract is byte-identical
        - `just lint` passed; `cargo clippy -p sniff -p sniff-cli --all-targets --features sniff/remote` clean; `cargo fmt --check` shows only pre-existing rustfmt-version drift already present at HEAD (my two hand-fixed spots restored zero-new-drift in `aggregate_view.rs`)
        - end-to-end parity check: `sniff --base sniff/lib repo --json`'s context block matches the focused text commands (`repo package|package-area|area|package-root|package-area-root` and both dirty exit codes) value-for-value on the real worktree
- work completed for 'aggregate JSON builder purity (R2.7)' at 12:35:31
- starting the work on 'ManifestStore for non-Cargo workspace discovery (R5.5)' at 12:56:49
        - located the bypasses: `detect_npm_workspace`/`detect_yarn_workspace`/`detect_bun_workspace` (`npm.rs:213,251,173`) parse the root `package.json` via `parse_package_json_workspace_patterns` (`npm.rs:364`, direct open+parse); `detect_pnpm_workspace` (`npm.rs:127`) parses `pnpm-workspace.yaml` directly; `detect_uv_workspace` (`uv.rs:27`) parses the root `pyproject.toml` via `parse_uv_workspace_members` (`uv.rs:65`, direct open+parse)
        - concrete duplicates confirmed: uv's `RootMembership::Always` root seed re-parses the root `pyproject.toml` through `ManifestStore::pyproject` during name/version resolution; the nested-marker walk re-dispatches the npm detector at every member dir (they self-filter on a missing `workspaces` field but parse first), and enrichment then re-parses each member through `ManifestStore::npm`; a version-less npm member also re-parses the root `package.json` through the store's root-version fallback
        - decided to convert the three workspace parsers to pure `*_from_value` extractors and add error-preserving `required_npm` / `required_pyproject` / `required_pnpm_workspace` store accessors mirroring `required_cargo`; detectors call the accessor, then the extractor — the same shape as `detect_cargo_workspace`
        - scope call: `collect_default_workspace_patterns` (`detection.rs:932`) shares both Node parsers and is only reachable via `detect_nx`/`detect_turborepo`/`detect_lerna`, so those three also receive the store; without that, an orchestrator cohabiting with npm/pnpm (turbo+npm, nx+pnpm) still double-parses the workspace manifest, violating the finding's "each unique manifest parsed at most once per detail phase" acceptance
        - noted but left alone (config counter family, store has no config-value cache): `detect_lerna` parses `lerna.json` and then `collect_default_workspace_patterns` re-parses it — same defect shape, different counter (`REPO_CONFIG_PARSES`); will flag in the report

## Implementation of Review Findings #1

> **started at:** 2026-07-17T10:46:27-07:00

- this is a continuation of the same iteration-1 implementation section; an earlier attempt today (started 10:00:19) ran out of capacity partway through, and this retry resumes from its logged state at 2026-07-17T10:46:27-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- we are _retrying_ iteration #1 because in our last attempt the agent failed before achieving it's objectives
  - the only context we have about where we got to in this previous attempt is:
    the agent ran out of capacity; this typically is a server load issue and should succeed on a retry. There was definitely _some_ work that was done before this happened.
  - this does mean that some of the findings from the review are already likely implemented (but not all)
  - to accommodate this, as you iterate over the different findings/suggestions, always first check if the work appears to be one before attempting to fix it
  - if you believe that the previous run did fix it then be sure to log an entry to the log file mentioning this
- starting the work on 'make RepoRequest::structure() genuinely shallow' at 10:03:14
  - the previous run already implemented this finding across commits `d8bcceee5`, `429bb690c`, and `d39434b0c`; no additional source change is required in this retry
  - `RepoRequest::structure()` now requests identity and topology only, while `RepoRequest::focused(RepoDetailRequest)` selects package-manager, dependency, or test-runner enrichment independently
  - the package construction path gates all enrichment-only fields and counters on the selected request detail; its Level 1 work-count test asserts zero package enrichments, lockfile/config parses, and inventory acceptance for structure mode
  - `sniff repo dependencies`, `sniff repo package-manager`, and `sniff repo test-runner` now use focused detail requests instead of relying on accidentally enriched structure results
  - relevant request, detection, and public API documentation now describes the shallow structure contract and directs enriched callers to focused or full requests
  - GitNexus reports HIGH impact for `create_package_with_request`: 3 direct callers and 13 affected symbols across the repo, filesystem, commands, and Git modules; no new edit is being made to that high-impact path during this retry
  - focused Level 1 verification passed all 4 structure/focused-request contract tests
  - `just lint` passed for `sniff-lib` and `sniff-cli`
  - `just test` ran 1,630 tests: 1,629 passed and `filesystem::repo::area::tests::detect_area_errors_when_not_in_repo` timed out on all four attempts
    - this is the same pre-existing failure identified by the review's separate completion-evidence finding, not a regression in shallow structure detection
    - inspection found that the test calls full repository detection on the host-wide temporary directory; making that test use an isolated temporary fixture is deferred to the serial work for that review finding
- starting the work on 'make the manifest cache request-scoped' at 10:10:31
  - the previous run appears to have implemented the core finding: `ManifestStore` now lives for one repository detection and is borrowed by every package build context
  - inherited Cargo workspace-version resolution and workspace-root test-runner configuration observations already route through that shared store
  - portable Level 1 work-count fixtures already cover several inheriting Cargo members, shared `Cargo.lock` parsing, present root-scoped config reuse, absent root-scoped config reuse, and nested workspace owner-root isolation
  - this retry is auditing all detection entry points, comments, and counter assertions before deciding whether any additional source change is required
- resuming the audit of 'make the manifest cache request-scoped' at 10:53:42 (this retry)
  - audit complete: all three detection entry points construct one `ManifestStore` per detection (`detection.rs:161`, `detection.rs:475`, `detection.rs:1594`) and pass it by reference into every `PackageBuildContext`
  - `cargo_package_version_with_root` (`cargo.rs:248`) receives the root manifest value from the store; the only remaining `read_toml_at` call is the standalone-aggregation fallback, explicitly documented as such (`cargo.rs:289-292`)
  - the previous run did fix this finding; no additional source change required
- work completed for 'make the manifest cache request-scoped' at 10:53:42
- auditing 'make RepoRequest::structure() genuinely shallow' at 10:53:42
  - the previous run's verification stands: `structure_mode_performs_zero_enrichment_work` (`detection.rs:2767`) asserts zero enrichment counters in structure mode, and enrichment phases are gated on `request.structure_only` throughout `detection.rs`
  - no additional source change required
- work completed for 'make RepoRequest::structure() genuinely shallow' at 10:53:42 (verified as already implemented by the previous run)
- auditing 'Windows subprocess hardening' at 10:53:42
  - the previous run fixed all three call sites named by the review: Windows audio now uses `process::run_with_timeout` (`audio.rs:814`, no `try_wait` remains), the Windows default-route probe uses `process::run_for_stdout` with a named timeout (`network/mod.rs:983`), and `tzutil` goes through the same helper (`os/time.rs:245`)
  - the shared helper drains both pipes on dedicated threads while the parent polls the deadline (`process.rs:128-215`)
  - Level 1 coverage exists at both layers: portable helper tests manufacture output larger than a pipe buffer and inject short deadlines (`process.rs:267` `output_larger_than_a_pipe_buffer_does_not_deadlock`, `process.rs:297` `a_hung_child_is_killed_at_its_deadline`), and Windows-gated probe tests exercise the real PowerShell path (`audio.rs:1275` `windows_audio_probe_drains_large_stdout_and_stderr`)
  - the previous run did fix this finding; no additional source change required
- work completed for 'Windows subprocess hardening' at 10:53:42 (verified as already implemented by the previous run)
- auditing 'shared ownership index (R6.4)' at 10:53:42
  - the previous run implemented `PackageOwnershipIndex` (`repo/ownership.rs`) as a crate-private, request-scoped, component-aware deepest-prefix index; detection, inventory attribution, integrated document attribution (`docs.rs`), and commit-file attribution (`recent_commits.rs`) all borrow the same per-request instance
  - work-count tests assert zero canonicalizations during shared lookups (`ownership.rs:150`, `docs.rs:1396`, `recent_commits.rs:1061`, `types.rs:604`)
  - native `PathBuf` semantics are retained, including the Windows drive-prefix/casing fixture (`ownership.rs:175` `preserves_windows_drive_prefix_and_casing`)
  - `RepoInfo::package_for_dir` no longer canonicalizes per candidate: it canonicalizes the query once and delegates to the index (`types.rs:274-280`)
  - the previous run did fix this finding; no additional source change required
- work completed for 'shared ownership index (R6.4)' at 10:53:42 (verified as already implemented by the previous run)
- auditing 'focused Git ref/worktree observation reuse (R9.5/R9.6)' at 10:53:42
  - the previous run added `RefSnapshot` (`remote_refresh.rs:39`) observed once per request; branches, tracking, remotes, decorations, and commit-remote containment all project from it (`get_*_from_snapshot` family)
  - worktree enumeration reads linked-worktree administrative metadata directly via gix proxies (`get_worktrees_from_snapshot`, `remote_refresh.rs:840`), opening a checkout only when full details were explicitly requested
  - Level 1 counter tests pin both bounds: one ref walk per focused multi-facet request (`types.rs:1817-1824`) and zero linked-repository opens for metadata-only worktree listing (`types.rs:1838` `focused_worktree_metadata_opens_no_linked_repositories`)
  - the previous run did fix this finding; no additional source change required
- work completed for 'focused Git ref/worktree observation reuse (R9.5/R9.6)' at 10:53:42 (verified as already implemented by the previous run)
- starting the work on 'completion evidence: deterministic L1 suite and cross-platform artifacts' at 10:54:53
  - this is the one finding the previous run did not reach: `detect_area_errors_when_not_in_repo` (`area.rs:82`) still runs full repository detection over the host-wide temporary directory and times out on this machine
  - the review also asks for three-OS evidence and the deferred Criterion fixture families; the CI mechanism for both exists (`test.yml` `sniff-cross-platform` on macOS/Linux/Windows with `--features remote`; `sniff-performance.yml` nightly 3-OS work-count matrix with per-OS artifact upload) and benches exist under `sniff/lib/benches/`
  - dispatching a subagent to fix the red test, verify `just test`/`just lint` green in the sniff package area, and assess what of the cross-platform/benchmark asks is implementable from this macOS-only host
- starting the fix for `detect_area_errors_when_not_in_repo` at 10:56:20
  - the test ran full repository detection over the host-wide system temp dir (`std::env::temp_dir()`), which enumerates an enormous tree on this machine and timed out on every nextest attempt
  - changed `sniff/lib/src/filesystem/repo/area.rs` to point `detect_area` at an isolated, empty `tempfile::tempdir()` fixture (the crate's established test pattern; `tempfile = "3"` is already a dev-dependency)
  - verified `detect_repo` never walks upward from the given root — it gates on `has_workspace_marker(root)` (`detection.rs:129`, `detection.rs:408`) and detectors only inspect the root and below — so an isolated empty dir is deterministically `AreaError::NotInRepo` on any host; tightened the assertion from `NotInRepo | NotMonorepo` to exactly `NotInRepo` and replaced the stale comment with the why
  - scanned the rest of the module: the three sibling tests are pure display/conversion checks with no filesystem access, so no other host-environment-dependent tests exist in this file
  - the fixed test now passes in 0.017s (was a timeout on all four attempts)
  - `just test` at 11:05: sniff-lib 1,630 run / 1,630 passed / 6 skipped (previously 1,629 passed + 1 timed out); sniff-cli 778 run / 777 passed / 3 skipped / 1 failed
    - the one sniff-cli failure is `os_json_snapshot`, unrelated to this change: the host was updated macOS 26.5.1 → 26.5.2 and the checked-in `snapshots__os_json_summary.snap` pins 26.5.1 (git history shows this snapshot is routinely bumped per host OS update, e.g. `d32cdff6f`); flagged for the orchestrator, not fixed here
  - `just lint` passed clean; `cargo clippy -p sniff --all-targets --features remote` (covers test targets) also clean
  - cross-platform/benchmark assessment: `.github/workflows/test.yml` job `sniff-cross-platform` already runs `cargo check --all-targets --features remote` + `just test` on macos/ubuntu/windows, and `.github/workflows/sniff-performance.yml` job `sniff-work-counts` already collects the nightly 3-OS work-count artifacts (per-OS upload, 90-day retention); only the Linux/Windows leg execution itself and the deferred Criterion fixture families remain unactionable from this macOS-only host
- work completed for 'completion evidence: deterministic L1 suite and cross-platform artifacts' at 11:05:35

### Successful Completion

The implementation of review cycle 1 has completed successfully in approximately 22 minutes for this retry (10:46:27–11:08:36; the iteration overall spanned earlier attempts today whose work this retry audited and completed). During this implementation all 6 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 6 were fixed (5 were verified as already implemented by the previous attempts and required no further source change; 1 — the deterministic L1 suite — was implemented in this run), 0 were deferred in full, but 2 performance-evidence components of finding 6 were deferred (see reasons below):

- **Finding 6, three-OS execution legs and retained non-macOS work-count artifacts** — deferred because this host is macOS-only: the Linux/Windows legs can only execute in CI. The mechanism the review asks for is already in place and now exercises the fixed, portable test: `.github/workflows/test.yml` job `sniff-cross-platform` runs `cargo check --all-targets --features remote` plus `just test` on macos/ubuntu/windows, and `.github/workflows/sniff-performance.yml` job `sniff-work-counts` collects the 3-OS work-count artifacts nightly with per-OS uploads and 90-day retention. See `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.
- **Finding 6, running the deferred Criterion fixture families / `just bench`** — deferred because this repo's performance doctrine accepts work counters, not wall-clock timing on a loaded dev host, as evidence ("Counters, not wall time"); a local Criterion run here would produce numbers the project explicitly considers non-evidentiary, while the accepted work-count evidence is already collected nightly by the CI matrix above. The Criterion fixture families themselves exist under `sniff/lib/benches/`. See `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.

One known pre-existing issue remains, unrelated to this review's findings and post-dating the review itself: the `os_json_snapshot` sniff-cli test fails on this host because the host was updated macOS 26.5.1 → 26.5.2 while the checked-in snapshot pins 26.5.1. It is host drift that fails identically on any branch, is routinely bumped per OS update (e.g. `d32cdff6f`), and was deliberately not bumped here to avoid mixing an unrelated maintenance change into this review implementation.

## Implementation of Review Findings #3

> **started at:** 2026-07-17T14:36:02-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'aggregate builder manifest reads and purity coverage' at 14:38:39
        - audited the prior iteration-3 delta: the finding remained open; `build_aggregate_value` still reached manifest-reading `aggregate_versions`, its purity fixture used `repo: None`, and `observe_repo_aggregate` still performed standalone-package fallback detection
        - GitNexus reported CRITICAL impact for `detect_repo_context` (44 affected symbols across 10 modules; CLI `run` and filesystem benchmark `register` flows), HIGH impact for `RepoAggregate`, `Package`, and `create_package_with_request`, MEDIUM impact for `aggregate_versions`, and LOW impact for `observe_repo_aggregate` and `resolve_package_version`
        - the refined design also required the request-scoped `detect_repo_inner_with_shared_request_and_ownership` path (CRITICAL: 22 affected symbols, benchmark `register` flow, 5 modules) and `synthesize_root_package_repo_with_request` (HIGH: 11 affected symbols, CLI `run` flow, 3 modules)
        - stopped before source edits as required; the orchestrator warned the user about each blast radius and explicitly resumed this work
        - selected a narrow contract-preserving design: synthesize standalone `RepoInfo` in the original filesystem detection pass, carry the already-detected package-version collapse on `RepoAggregate`, and leave the public `Package` and focused `repo version` APIs unchanged
        - the first canonical run passed all 1,640 library tests but exposed an over-broad standalone projection through two degenerate `repo structure` snapshot failures; narrowed synthesis to the focused aggregate request so the focused/public structure contract remains unchanged
        - moved standalone-package synthesis into the focused request-scoped repository observation path so it reuses the same `ManifestStore`; a new Level 1 test proves one manifest parse and one package enrichment for the standalone projection
        - removed repository and manifest observation from aggregate projection; `RepoAggregate` now carries the uniform detected package version, and populated two-package purity coverage proves `build_aggregate_value` emits packages and version while every work counter remains zero
        - the second canonical `just test` run passed all 1,640 sniff-lib tests and 774 of 776 sniff-cli tests; the new standalone accounting, aggregate purity, single-package Cargo/Node/Python, and inherited workspace-version coverage all passed
                - the two remaining failures are the review's separate completion-evidence finding: host macOS snapshot drift from 26.5.1 to 26.5.2 and the aggregate fixture's environment-dependent default branch (`main` expected, `master` observed)
        - `just lint` passed for the directly impacted sniff package area, and `git diff --check` reported no whitespace errors
        - required GitNexus `detect_changes(compare main)` reported HIGH aggregate risk across the shared worktree delta (109 files, 1,308 symbols, 9 processes); the finding-level affected flow is the previously reviewed CLI `run` projection, while the additional reported processes originate in concurrent shared changes
- work completed for 'aggregate builder manifest reads and purity coverage' at 14:52:00
- starting the work on 'single aggregate status walk in linked worktrees' at 14:53:49
        - confirmed the duplicate path: detailed Git detection performs the first status walk, then legacy `GitRequest::full()` worktree metadata performs a second walk for the current linked worktree
        - GitNexus reported LOW impact for `select_git_request` (3 direct callers, the CLI `run` performance-output process) and LOW impact for `run`; the test helpers also reported LOW impact with no runtime process participation
        - focused bare aggregate JSON detection on detailed file changes plus Git config metadata; commits, branches, and worktrees remain supplied by `observe_repo_aggregate`, while ordinary `GitRequest::full()` selection and preset behavior remain unchanged
        - added the portable Level 1 `linked_worktree_aggregate_walks_status_and_discovers_once` fixture using libgit2's cross-platform worktree API; it asserts exactly one `git.status_walks` and one `git.repository_discoveries`, alongside the existing main-worktree bound
        - focused verification passed: 2/2 library counter tests and 3/3 CLI request-selection tests
        - canonical `just test` passed all 1,641 sniff-lib tests; sniff-cli passed 775/777, with only the review's separate finding-4 failures (`os_json_snapshot` host-version drift and `repo_aggregate_json_snapshot` default-branch drift)
        - `just lint` passed for the directly impacted sniff package area; `git diff --check` reported no whitespace errors
        - the real linked-worktree command `sniff --base sniff/lib --perf repo --json` now reports `git.status_walks: 1` and `git.repository_discoveries: 1`; no performance measurement was deferred
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the shared dirty worktree (18 files, 73 changed symbols, 2 processes); this finding's expected affected processes are the CLI `run` performance report and stderr emission paths, with no unexpected finding-level flow
- work completed for 'single aggregate status walk in linked worktrees' at 15:03:45
- starting the work on 'bounded BurntToast and subprocess-tree cleanup' at 15:05:11
        - audited R12 and the existing process dependencies before design: `libc` already supplies Unix process-group signaling, while the existing target-specific `windows` dependency only needed the Security, ToolHelp, JobObjects, and Threading feature gates enabled; no new crate was added
        - GitNexus reported HIGH impact for `run_with_timeout` (8 direct callers, 30 affected symbols across OS, network, programs, and subprocess tests; no indexed execution flows) and LOW impact for `is_burnttoast_available` (1 direct caller, no flows); the orchestrator warned the user and explicitly authorized the exact shared-helper edit
        - made Unix subprocesses members of a fresh process group and made Windows subprocesses start suspended, enter a fresh kill-on-close Job Object, and resume only after assignment; cleanup terminates the complete group/job before joining stdout/stderr drain threads on timeout, wait failure, and successful direct-child exit
        - isolated the required Windows unsafe API calls behind owned handles and documented the safety invariants; suspended startup removes the ordinary spawn-before-Job-Object-assignment race, while Unix negative-PID signaling is scoped to the dedicated child process group
        - moved the BurntToast PowerShell module query from unbounded `Command::output()` to `process::run_for_stdout` with the named 3-second `WINDOWS_BURNTTOAST` policy; spawn/timeout counters, direct execution without a shell, and the cached boolean result shape now come from the shared helper
        - added portable Level-1 same-test-executable fixtures in which the direct child exits after spawning a 30-second descendant that retains both inherited pipes; the regression test returns in approximately 27 ms instead of blocking on the descendant-held handles
        - added BurntToast Level-1 coverage for its exact `yes` marker, named timeout policy, and an injected 100 ms timeout through the shared runner; focused verification passed all 6 subprocess/BurntToast tests
        - cross-platform compile evidence: `cargo check -p sniff --all-targets --features remote --target x86_64-pc-windows-gnu` passed, compiling the suspended-start, ToolHelp resume, and Job Object path; macOS focused and canonical tests exercise the Unix process-group path, whose `CommandExt::process_group` plus `libc::kill` implementation also covers Linux
        - canonical `just test` passed all 1,645 sniff-lib tests; sniff-cli passed 775 of 777 tests, with only the review's separate finding-4 snapshot failures (`os_json_snapshot` host-version drift and `repo_aggregate_json_snapshot` default-branch drift)
        - `just lint` passed for the directly impacted sniff package area, and a finding-scoped `git diff --check` reported no whitespace errors
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the shared dirty worktree (23 files, 122 changed symbols, 2 processes); the two CLI `run` flows come from concurrent shared changes, while the finding-level pre-change analyses reported no affected execution flow
- work completed for 'bounded BurntToast and subprocess-tree cleanup' at 15:16:47
- starting the work on 'deterministic Level-1 snapshots and cross-platform evidence' at 15:19:06
        - audited the two pending snapshots: `os_json_snapshot` varied with the host's live kernel and OS versions, while `repo_aggregate_json_snapshot` inherited the host's Git default branch
        - GitNexus reported LOW impact for both snapshot tests and the OS normalizer, plus MEDIUM test-only impact for the shared Git fixture initializer (5 direct fixture callers, 9 transitive snapshot tests, no execution flows)
        - normalized the three live OS-version values to stable placeholders after asserting each remains a non-empty JSON string, preserving schema coverage without pinning the host's patch release
        - initialized all CLI snapshot Git fixtures with an explicit `main` initial branch through libgit2, independent of host Git configuration and portable across macOS, Linux, and Windows
        - focused verification passed both repaired snapshot tests; the successful Insta run removed both stale `.snap.new` artifacts, leaving the checked-in aggregate snapshot unchanged
        - canonical `just test` passed all 1,645 sniff-lib tests and all 777 sniff-cli tests (3 skipped); `just lint` passed cleanly, `git diff --check` found no whitespace errors, and the snapshot comments/docs audit found no drift
        - actual cross-target compile evidence: `cargo check -p sniff --all-targets --features remote --target x86_64-pc-windows-gnu` passed; its four warnings are pre-existing target-gated test warnings outside this finding
        - complete non-macOS evidence remains deferred: this host has no Linux Rust target and cannot execute native Linux or Windows Level-1 binaries; the Windows GNU CLI all-target check was stopped after approximately 90 seconds while still compiling dependencies under the session timeout rule, and the Windows MSVC library check was blocked in `pcre2-sys`/`aws-lc-sys` because the macOS host lacks Windows SDK C headers (`ctype.h` and `windows.h`)
        - `.github/workflows/test.yml` defines future macOS/Linux/Windows all-target and Level-1 coverage, but that workflow definition was inspected rather than executed and is not counted as passing evidence
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the shared dirty worktree (25 files, 125 changed symbols, 2 CLI `run` processes); this finding itself changes only test fixtures and snapshots with no indexed execution-flow participation
- work completed for 'deterministic Level-1 snapshots and cross-platform evidence' at 15:25:23
- starting the work on 'error-caching manifest store and detection-level reuse' at 15:27:10
        - audited the partial iteration-2 implementation: valid Node, pnpm, and uv discovery uses the request-scoped `ManifestStore`, but the required accessors cache only successful parses, so a tolerant discovery probe followed by another detector or enrichment can reopen a malformed manifest
        - GitNexus reported LOW impact for `ManifestStore` and its indexed tolerant `npm`/`pyproject` accessors (no affected symbols or processes); the newer required accessors are absent from the stale index, and the repository process catalog was reviewed
        - avoided modifying the CRITICAL `detect_repo_inner_with_shared_request_and_ownership` symbol (22 affected symbols and the benchmark `register` flow); detection-level tests can exercise the existing request boundary without changing it
        - changed `sniff/lib/src/filesystem/repo/detection.rs` so Node, pnpm-workspace, and uv manifest caches retain either the parsed value or a reconstructible I/O/parse failure under the normalized native path; tolerant accessors still return `None`, while required accessors replay the original error class and message without reopening the file
        - added eight Level-1 tests: parse- and I/O-error replay tests plus valid and malformed detection fixtures for npm, pnpm, and uv; the valid fixtures exercise discovery, version/dependency enrichment, package-enrichment counts, and one parse per unique manifest, while the malformed npm/pnpm fixtures prove a swallowed first failure is reused by a later detector
        - focused Level-1 verification passed all 8 new manifest reuse tests; the first counter run exposed adjacent detector-input opens in the malformed orchestrator fixtures, so the fixtures were narrowed and their total input bounds made explicit without weakening the per-manifest parse assertions
        - canonical `just test` passed all 1,653 sniff-lib tests and all 777 sniff-cli tests (9 and 3 skipped, respectively); `just lint` passed for the directly impacted sniff package area
        - a finding-scoped `git diff --check` passed; the workspace-wide check reports only pre-existing trailing whitespace in the concurrently edited implementation prompt
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the shared dirty worktree (25 files, 143 changed symbols, 2 CLI `run` processes); this finding's pre-change impact was LOW and its source/test changes introduce no unexpected process-level flow
        - no performance measurement or implementation work was deferred for this finding
- work completed for 'error-caching manifest store and detection-level reuse' at 15:41:46
- starting the work on 'component-aware aggregate area matching' at 15:43:10
        - confirmed review finding 6 against R6.6: `area_change_facts` converted three native Git path sources through `to_str()` and used `str::starts_with` for both named-area membership and root-area exclusion
        - GitNexus could not resolve the two newly added private helpers in its current index; analysis of their indexed containing entry point, `observe_repo_aggregate`, reported LOW impact (3 direct callers, 1 CLI `run` process family, 2 modules), and the repository process catalog was reviewed
        - retained each dirty, untracked, and detailed-change path as a native borrowed `Path`; both named-area membership and root-area exclusion now use component-aware `Path::starts_with`, preserving the existing lexical encoding, case, separator, and symlink semantics without lossy UTF-8 conversion or ad hoc case folding
        - added the portable Level-1 `area_membership_compares_whole_path_components` workspace fixture: a source change under root-level `alpha2` neither marks area `alpha` dirty nor gives it source changes, while the root area correctly claims both facts
        - focused verification passed 1/1 collision test; canonical `just test` passed all 1,654 sniff-lib tests and all 777 sniff-cli tests (9 and 3 skipped, respectively), and `just lint` passed for the directly impacted sniff package area
        - the relevant comment now states the whole-component area-path contract; a focused `git diff --check` passed and an audit confirms the membership helper contains no `to_str()` or string-prefix comparison
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent shared worktree delta (25 files, 143 changed symbols, 2 CLI `run` processes); this finding's pre-change containing-entry-point analysis was LOW and its expected affected flow is aggregate CLI context projection
        - no performance measurement or implementation work was deferred for this finding
- work completed for 'component-aware aggregate area matching' at 15:47:33
- starting the work on 'specified Criterion workload families and counter bounds' at 15:49:06
        - reading the review benchmark finding, specification matrix, performance-testing doctrine, Criterion guidance, and existing benchmark and work-counter fixtures before selecting the smallest complete workload additions
        - GitNexus reported LOW impact for the existing dirty-fixture builder (5 affected symbols, 1 direct caller, no processes), LOW impact for each benchmark registration symbol and `register_all` (no affected processes), and LOW impact for the existing fixture wrapper (4 affected symbols, 3 direct callers, no processes); the repository process catalog was reviewed before editing
        - added lazily constructed, setup-excluded Criterion families for deep/wide formatting-only detection; package-scoped Git plus inventory; standalone versus integrated nested observation; mixed-ecosystem structure detection at 100, 500, and 2,000 packages; inventory-only and inventory plus docs at 10,500 files; final assembly and package attribution at 500/2,000 documents; 100 dirty files at 1 KiB, 100 KiB, and 2 MiB; 32 divergent worktree tips; 100-tip containment; 2,000-commit sparse path history; and native case-sensitive/case-insensitive warm/cold-ish walks
        - added a real GitHub-provider Criterion workload backed by local wiremock routes; the existing deterministic provider test pins one metadata request and one root-tree request per report
        - added fixture contract tests proving zero descendant work for the deep/wide formatting case, exact requested mixed-package cardinality, and exact dirty-file byte sizes; the cardinality test caught and drove fixes for missing uv/Go membership authorities and the uv virtual root slot
        - documented every workload-to-counter-test mapping in `sniff/lib/benches/README.md`; existing Level-1 bounds cover inventory saturation, observation reuse, path-history visits, ref snapshots, ownership canonicalizations, dirty-side blob/diff work, and remote request counts without inventing wall-time gates
        - deferred only the large synthetic service-listing Criterion row: the public service API intentionally observes the host, while backend parsers and command injection are crate-private; exposing a production injection API solely for a benchmark would broaden unrelated public surface, and existing Level-1 tests already pin chunking, partial failure, timeouts, and child reaping
        - verification passed: focused fixture tests 3/3; `cargo check -p sniff --benches --features network`; `cargo check -p sniff --benches --features remote`; canonical `just test` with sniff-lib 1,657/1,657 and sniff-cli 777/777; canonical `just lint`; and focused `git diff --check`
        - no Criterion timings were collected or claimed on the loaded host; the new definitions are ready for the stable runner while work counters remain the acceptance evidence
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent shared worktree delta (29 files, 159 changed symbols, 2 CLI `run` processes); this finding's pre-change impacts were LOW and its changes are confined to benchmark fixtures, benchmark registration/documentation, and fixture contract tests
- work completed for 'specified Criterion workload families and counter bounds' at 16:03:15
- starting the work on 'authoritative phase and completion records' at 16:04:41
        - audited every performance phase and completion record against review-3 and current source; no edited Markdown file carries hash-state frontmatter, so no Darkmatter rehash was required
        - corrected Phase 2's temporary aggregate-canonicalization allowance and historical `GitRequest::full()` aggregate floor; the current record states that populated aggregate projection is counter-silent and linked-worktree aggregate status runs once
        - corrected Phase 4's lockfile-only manifest-store, blocked shallow-structure, and absent ownership-index claims; historical work-count tables remain unchanged and are explicitly labeled as phase-boundary evidence
        - corrected Phase 5's open R9.5/R9.6 claims and Phase 6's incomplete subprocess boundary; the records now describe one shared ref snapshot, metadata-only worktree projection, the BurntToast deadline, and process-group/Job-Object descendant cleanup
        - corrected Phase 8 and the umbrella plan to distinguish implemented R1–R13 source from narrower remaining evidence and benchmark limitations
                - native Linux/Windows Level-1 execution and retained non-macOS artifacts were not produced on this macOS host; workflow definitions are recorded as future coverage rather than claimed execution evidence
                - the synthetic large-service Criterion row remains deferred because exposing a production command-injection seam solely for a benchmark would broaden unrelated API surface; Level-1 work bounds already cover the behavior
        - consistency searches now return only explicitly historical phase-boundary statements or the review/log's immutable finding labels; `git diff --check` passed for all edited records
        - canonical `just test` passed all 1,657 sniff-lib tests and all 777 sniff-cli tests; canonical `just lint` passed for the directly impacted sniff package area
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent shared worktree delta (35 files, 196 changed symbols, 2 CLI `run` processes); this finding changes documentation records only and introduces no production/test behavior
- work completed for 'authoritative phase and completion records' at 16:14:27

### Successful Completion

The implementation of review cycle 3 has completed successfully in 1 hour, 40 minutes, and 37 seconds (14:36:02–16:16:39 local time). During this implementation all 8 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 8 were fixed, 0 were deferred in full, and 2 narrower evidence or workload components were deferred (see reasons below):

- **Finding 4, native Linux/Windows Level-1 execution and retained artifacts** — deferred because this implementation ran on a macOS-only host. The canonical macOS suite is green, and the installed Windows GNU target compiled `sniff` with all targets and `remote`; however, native Linux/Windows binaries cannot run here, the Linux target is not installed, and the MSVC cross-check requires Windows SDK headers unavailable on macOS. The existing three-OS workflow was inspected but is not claimed as executed evidence. See `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.
- **Finding 7, large synthetic service-listing Criterion workload** — deferred because the public service API intentionally observes the host while backend parsers and command injection are crate-private. Adding a production injection seam solely for a benchmark would broaden unrelated API surface. Existing Level-1 tests continue to pin service batching, partial failure, timeouts, and process-tree reaping. See `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.

The implemented files cover aggregate observation and rendering, focused Git request selection, cross-platform subprocess cleanup, deterministic snapshots, request-scoped manifest caching, component-aware area attribution, Criterion workload fixtures, benchmark documentation, and authoritative phase records. Final sniff-area verification passed `just test`, `just lint`, `just build`, and both network- and remote-feature benchmark compilation checks.

- final GitNexus `detect_changes(compare main)` reported HIGH risk across the complete multi-iteration dirty worktree (113 files, 1,358 indexed symbols, and 9 affected processes); all 9 traces were reviewed
        - review-cycle-3 changes participate in the expected aggregate CLI performance-output and benchmark-fixture registration flows
        - the remaining cross-package collector and request-metadata flows come from shared Sniff symbols in the broader feature delta; no unexpected review-cycle-3 execution flow was identified
- scoped `git diff --check` passed for `sniff` and the updated Sniff skill; no commit or write-mode formatting command was run

## Implementation of Review Findings #4

> **started at:** 2026-07-17T17:09:45-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- review-4 contains 6 findings (4 High, 2 Medium):
        1. High: bare aggregate execution still rediscovers and independently queries the repository
        2. High: `repo --json --perf` omits most aggregate work from its report
        3. High: Unix process-group cleanup does not guarantee a bounded process tree
        4. Medium: the documented universal subprocess boundary still has production bypasses
        5. High: native Linux and Windows Level-1 completion remains unverified
        6. Medium: the required synthetic service benchmark is still absent
- starting the work on 'aggregate repository observation reuse' at 17:13:15
        - confirmed the review finding: the bare aggregate runs the original filesystem detection and then `observe_repo_aggregate`, which performs a second repository discovery plus independent identity, branch, worktree, current-worktree, and history observations
        - GitNexus reported HIGH impact for the additive `GitInfo` evidence extension (18 indexed dependents across Git, Repo, and Commands, with no directly affected indexed process); `observe_repo_aggregate`, `select_git_request`, and the worktree helpers are LOW impact and participate only in the expected CLI `run` flow
        - proceeding with an additive, Serde-skipped aggregate evidence field produced inside the original `GitRepo::detect_with_request` call, then consumed by a repository-observation-free aggregate completion step; aggregate JSON remains unchanged
        - implemented request-scoped aggregate evidence from the original Git detection: branches share its single ref snapshot, worktrees share its discovered repository handle, and recent commits share one file-aware history observation plus the detected repository ownership index
        - changed `observe_repo_aggregate` into a pure completion step over detected Git and repository information; it no longer discovers a repository or performs status, ref, worktree, identity, or history observations
        - added a CLI command-path Level-1 regression test that exercises the real aggregate request and pins one repository discovery, one status walk, one ref walk, and zero extra worktree opens while requiring populated identity, branch, and worktree output
        - focused verification passed the new CLI regression, both linked-worktree aggregate regressions, and all 20 aggregate-view tests; `cargo check -p sniff-cli --tests` passed, `just lint` passed, and `git diff --check` passed
        - canonical `just test` completed all 1,657 sniff-lib tests successfully on two attempts, but the subsequent sniff-cli feature-set compilation exceeded the session's 60-second non-interactive command boundary; the compiled focused CLI regression passed, while the complete sniff-cli runtime suite was not rerun within that boundary
        - final GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent shared worktree delta (31 changed files and 2 affected CLI `run` processes); the affected performance-report and stderr-emission flows are the expected aggregate command paths
- work completed for 'aggregate repository observation reuse' at 17:36:46
- starting the work on 'complete aggregate performance reporting' at 17:39:12
        - confirmed the review finding: `detect_with_plan` snapshotted and detached its collector before aggregate completion and projection, while `CliPerf::emit` preferred that stale report over its end-to-end clock
        - GitNexus reported CRITICAL impact for `detect_with_plan` (33 direct and 64 total dependents across 13 modules and 4 process families), MEDIUM impact for `CliPerf::build_report`, and LOW impact for the disambiguated CLI `run` and `CliPerf` type; the repository process catalog was reviewed before editing
        - implemented one request-scoped CLI collector that is reused by performance-enabled detection and remains available for aggregate completion/projection; standalone library callers retain their existing result snapshot, while the final aggregate report uses the CLI's command-wide elapsed time and counters
        - added a named aggregate-projection stage and a spawned-CLI Level-1 regression that requires the emitted JSON and stderr reports to include that post-detection stage, elapsed time covering detection plus projection, and exact one-discovery/one-status/one-ref/zero-worktree-open bounds
        - focused verification passed the spawned-CLI regression alone and a combined 4-test remote-feature run covering collector propagation, aggregate reuse, aggregate reporting, and existing JSON performance attachment
        - canonical `just test` passed all 1,657 sniff-lib tests; its CLI phase exposed 8 stale aggregate fixtures from the preceding aggregate-evidence change, so the shared fixture was updated to request the same evidence as the real command and all 10 aggregate unit tests then passed
        - canonical `just lint` passed for the directly impacted sniff package area, and a finding-scoped `git diff --check` passed
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent shared worktree delta (16 files, 44 changed symbols, and 2 affected CLI performance-output processes); both traces were reviewed and are the expected report construction and stderr emission flows
        - no implementation or performance measurement was deferred for this finding; the full CLI suite was not repeated after the fixture correction to respect the non-interactive 60-second command boundary, with focused aggregate and reporting coverage green
- work completed for 'complete aggregate performance reporting' at 17:51:08
- starting the work on 'bounded Unix detached-descendant cleanup' at 17:53:45
        - reading the subprocess supervision implementation and Level-1 coverage before selecting a portable bounded-cleanup design
        - GitNexus reported HIGH impact for `run_with_timeout` (8 direct callers and 30 total affected symbols across OS, network, programs, and service-related modules, with no indexed execution process); the repository process catalog was reviewed and the orchestrator was warned before editing
        - proceeding with cancellable Unix pipe readers: after process-group termination and direct-child reaping, readers stop without requiring EOF from a session-detached descendant; Windows retains its existing suspended-start, kill-on-close Job Object containment and blocking EOF drain
        - the first `O_NONBLOCK` implementation passed the detached-descendant regression but made the existing 1 MiB dual-pipe fixtures time out on macOS; it was rejected and replaced with `poll(2)` readiness using a 10 ms cancellation interval, which does not alter the child pipe's blocking behavior
        - added a cfg-gated Level-1 fixture whose descendant calls `setsid()`, proves it moved to its own process group, and retains and writes both inherited pipes after the direct child times out; the regression requires bounded return and verifies the direct child is no longer waitable
        - Unix now drains bytes only after `poll(2)` reports readiness and, after cleanup cancellation, caps the final drain at 50 ms rather than waiting for EOF; this remains bounded even when an escaped descendant writes continuously, while already-available output receives a short drain grace and the descendant receives a broken pipe after readers close
        - focused verification passed the escaped-descendant regression, including its final no-sleep continuous writer, and then the corrected detached plus two 1 MiB pipe regressions 3/3; the large-output cases completed in under 0.1 seconds and the detached case returned at its injected 3-second timeout rather than its fixture's 30-second lifetime
        - canonical `just test` passed all 1,658 sniff-lib tests and all 779 sniff-cli tests (11 and 3 skipped, respectively); canonical `just lint` and a finding-scoped `git diff --check` passed
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent shared worktree delta (17 changed files, 53 changed symbols, and 2 affected CLI performance-output processes); the subprocess helper itself has no indexed process and the two reported flows come from the preceding aggregate findings
        - no implementation or performance measurement was deferred for this finding; native Linux execution remains owned by review finding 5, while the Unix design uses APIs available on both macOS and Linux and leaves Windows Job Object behavior intact
- work completed for 'bounded Unix detached-descendant cleanup' at 18:15:47
- starting the work on 'bounded installation and remote-refresh subprocess paths' at 18:17:10
        - reading the required package, Rust, and testing guidance before tracing each production bypass and its existing timeout policy
        - GitNexus reported HIGH impact for `run_with_timeout` (8 direct callers, 30 total affected symbols), `refresh_remote_tracking_refs` (4 direct, 17 total), and `fetch_single_remote` (1 direct, 18 total), plus CRITICAL impact for `execute_install_captured` (5 direct, 31 total across five modules); the orchestrator was warned before editing
        - proceeding with an additive builder-capable bounded runner and crate-private injected Level-1 seams, preserving public install signatures, result shapes, cwd/environment configuration, and direct executable invocation
        - implemented `run_command_with_timeout(&mut Command, Duration)` as the builder-capable form of the shared supervisor; `run_with_timeout` now delegates to it, and caller-configured executable, arguments, cwd, and environment survive while the boundary owns stdin, captured pipes, the deadline, termination, and reaping
        - routed ordinary install, versioned install, uv bootstrap, uv tool install, and explicit `git fetch` refresh through the shared runner; every install subprocess now uses `InstallOptions::timeout_secs`, and explicit remote refresh has the named 30-second `process::timeouts::REMOTE_REFRESH` policy
        - added injected Level-1 fixtures for ordinary, versioned, uv install, uv bootstrap, and remote refresh timeouts without invoking a real package manager, network bootstrap, or Git remote; a builder regression also executes a fixture with an injected cwd and environment value
        - the first builder regression exposed only macOS `/var` versus `/private/var` canonicalization plus libtest framing in captured stdout; the assertion was corrected to compare the canonical child-reported line, with no production change
        - focused verification passed all 6 injected timeout/builder regressions; static inspection found no remaining production `.output()`, `.status()`, or `.spawn()` bypass under `sniff/lib/src`
        - canonical `just test` passed all 1,664 sniff-lib tests and all 779 sniff-cli tests (14 and 3 skipped, respectively); canonical `just lint`, scoped `git diff --check`, and remote-feature library compilation passed
        - documentation and the Sniff skill now describe both shared runner forms, the caller-selected installation timeout, and the named remote-refresh policy; the Sniff skill's Darkmatter body hash was updated
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent review-cycle worktree (21 changed files, 79 indexed symbols, and 2 affected aggregate CLI performance-output processes); the subprocess/install/refresh changes add no indexed execution process, and the reported traces belong to the preceding aggregate findings
        - no implementation or performance measurement was deferred for this finding; native Linux/Windows execution remains owned by review finding 5
- work completed for 'bounded installation and remote-refresh subprocess paths' at 18:34:14
- starting the work on 'native Linux and Windows Level-1 evidence' at 18:38:29
        - `sniff os --json` identified the execution host as native arm64 macOS 26.5.2 (Darwin 25.5.0); Rustup has `aarch64-apple-darwin`, `x86_64-pc-windows-gnu`, and `x86_64-pc-windows-msvc` installed, but no Linux target
        - canonical native macOS Level-1 verification passed all 1,664 sniff-lib tests and all 779 sniff-cli tests (14 and 3 skipped, respectively); canonical `just lint` also passed
        - the exact current dirty tree at HEAD `407a1dbfbce1bb953ef80ce8596805c77170b424` passed `cargo check -p sniff --all-targets --features remote --target x86_64-pc-windows-gnu` in 8.23 seconds, with four existing target-gated test warnings; this is supplemental cross-compilation evidence, not native Windows Level-1 execution
        - the installed MSVC target reached native dependencies but could not compile them because this macOS host lacks the Windows SDK C headers (`ctype.h` and `windows.h`) and Visual C++ environment; this is a host toolchain limitation and supplies no native Windows evidence
        - Docker Desktop exposes a local aarch64 Linux 6.12.76 kernel and cached Rust images, but those images lack `cargo-nextest` and `just`; installing tools was not authorized, and a non-canonical container check would not satisfy native host/path/process Level-1 acceptance
        - suspended Parallels Debian 13, Ubuntu, and Windows 11 guests exist, but starting user VMs was not authorized; no guest was started and no external workflow, image pull, install, commit, or push was performed
        - native Linux and Windows Level-1 execution plus retained exact-implementation work-count artifacts for all three OSes are deferred; the workflow definitions are not claimed as execution evidence, and the precise closure requirements are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md`
- work completed for 'native Linux and Windows Level-1 evidence' at 18:44:35
- starting the work on 'synthetic large-service Criterion workload' at 18:46:55
        - reading the required package, Rust, and testing guidance before tracing the production service batching, parsing, and runner orchestration
        - GitNexus reported LOW impact for `list_systemd_services` (2 direct and 16 total dependents, with one affected CLI `run` process) and `collect_systemd_pids` (1 direct and 12 total dependents, with no indexed process); the benchmark registration symbol is not indexed
        - proceeding with an additive `bench-internals` feature that exposes only a benchmark fixture module when explicitly enabled; the default production API remains unchanged
        - selected deterministic 500- and 2,000-service workloads, which map to 5 and 17 runner calls respectively: one primary listing plus `ceil(service_count / 128)` enrichment chunks
        - implemented a doc-hidden, feature-gated synthetic systemd fixture over the production listing parser, running-service selection, chunk builder, runner dispatch, show-block parser, and PID projection; Criterion constructs fixture data and per-iteration cursor state outside the timed section
        - first verification passed benchmark compilation with `remote,bench-internals` and both focused Level-1 tests; the synthetic test covers 500 and 2,000 services, while the real shim test maps the chunk bound to the stable `process.spawns` counter
        - updated the benchmark catalog, CI benchmark ID catalog, benchmark recipes, performance workflow feature set, and the earlier service-workload deferral to record the finding as resolved
        - canonical `just test` passed all 1,665 sniff-lib tests and all 779 sniff-cli tests (14 and 3 skipped, respectively); canonical `just lint`, feature-enabled benchmark compilation, four focused Level-1 contract tests, and scoped `git diff --check` passed
        - a stricter feature-enabled all-target Clippy run reached three unrelated concurrent test-only warnings in `process.rs` and `programs/enums/metadata.rs`; the canonical package-area lint remained green and no unrelated files were changed
        - required GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent review-cycle worktree (31 changed files, 88 indexed symbols, and 2 aggregate CLI performance-output processes); this finding's indexed systemd changes had LOW pre-change impact and add no affected execution process
        - no implementation or performance measurement was deferred for this finding; benchmark compilation and deterministic work bounds are the accepted local evidence, and no unstable wall timing was collected or claimed
- work completed for 'synthetic large-service Criterion workload' at 19:02:41

### Successful Completion

The implementation of review cycle 4 has completed successfully in 1 hour, 57 minutes, and 40 seconds (17:09:45–19:07:25 local time). During this implementation all 6 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 5 were fixed, 1 was deferred (see reasons below):

- **Finding 5, native Linux/Windows Level-1 execution and retained work-count artifacts** — deferred because this macOS host cannot execute the canonical suite natively on Linux or Windows, the available Docker images lack the repository's required `just` and nextest tooling, and starting suspended user VMs or publishing the dirty implementation was not authorized. Native macOS tests/lint passed, and the exact dirty tree cross-compiled for Windows GNU, but neither cross-compilation nor workflow definitions are claimed as native execution. Full closure requirements are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.

The files changed cover aggregate Git evidence and end-to-end performance reporting, bounded cross-platform subprocess execution, installation and remote-refresh timeout enforcement, escaped Unix descendant cleanup, synthetic service benchmark fixtures and CI registration, Level-1 regression coverage, architecture/skill documentation, and review-cycle records.

- final Sniff-area verification passed all 1,665 `sniff-lib` tests and all 779 `sniff-cli` tests (14 and 3 skipped), `just lint`, `just build`, feature-enabled Criterion benchmark compilation, and scoped `git diff --check`
- the exact dirty tree passed the Windows GNU all-target, remote-feature cross-check; native Linux/Windows execution remains the single deferred evidence item
- final GitNexus `detect_changes(compare main)` reported HIGH risk across the complete multi-iteration feature delta (124 files, 1,410 indexed symbols, and 8 affected processes); all 8 traces were reviewed
        - the two aggregate performance-output traces are expected review-cycle-4 behavior
        - the three benchmark fixture registration traces are expected performance-benchmark flows
        - the three collector-propagation traces belong to the broader feature's shared performance-accounting boundary; no unexpected review-cycle-4 flow was identified
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run

## Implementation of Review Findings #5

> **started at:** 2026-07-17T22:33:06-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- review-5 contains 3 High findings:
        1. native Linux and Windows Level-1 execution remains absent
        2. a quiet session-detached Unix descendant survives timeout cleanup
        3. aggregate reuse introduces an unapproved public Rust result contract
- starting the work on 'native Linux and Windows Level-1 execution' at 22:34:46
        - `sniff os --json` identified the execution host as native arm64 macOS 26.5.2 (Darwin 25.5.0); `sniff cpu --json` identified an Apple M4 Max with 16 physical and logical cores
        - evaluated the exact review boundary: native Level-1 execution must exercise macOS, Linux, and Windows host behavior, and the work-count tables must be retained for the same implementation on all three OSes
        - cross-compilation, WSL, Docker, and workflow inspection do not supply native Windows evidence; this task also explicitly prohibits VM startup, external workflow triggers, installs, commits, and pushes, so no authorized Linux or Windows execution path exists from this macOS workspace
        - confirmed the existing `sniff-cross-platform` workflow defines native `just test` legs for macOS, Ubuntu, and Windows, while `sniff-work-counts` defines separately retained 90-day artifacts for the same three-OS matrix; neither workflow definition is claimed as execution evidence
        - deferred native Linux and Windows Level-1 execution and the retained three-OS work-count set for the dirty implementation rooted at HEAD `03b03ea5a85d6f26fa6c257f254943983e99b72c`; this is a platform and execution-authority constraint, not a CPU-load deferral
        - recorded the exact review-5 mapping, evidence boundary, and closure procedure in `sniff/features/2026-07-16-performance/deferred-perf-tests.md`; `deferred_perf_measurement: true` remains set in the implementation log frontmatter
        - native macOS package-area verification passed all 1,665 `sniff-lib` tests and all 779 `sniff-cli` tests (14 and 3 skipped, respectively); canonical `just lint` and the finding-scoped `git diff --check` passed
- work completed for 'native Linux and Windows Level-1 execution' at 22:38:58
- starting the work on 'quiet session-detached Unix descendant cleanup' at 22:39:48
        - read the required Sniff, Rust, Rust-testing, and GitNexus impact-analysis guidance and traced the timeout runner, Unix process-group cleanup, Windows Job Object containment, and current detached fixture
        - GitNexus was 13 commits stale and its prescribed refresh was stopped inside the non-interactive 60-second ceiling; the indexed `run_with_timeout` boundary is HIGH impact (8 direct callers, 30 total affected symbols across network, OS, programs, and tests), while the newer builder and process-tree symbols are absent from the stale index
        - reviewed the indexed process catalog and direct call sites; no execution flow is attributed to the subprocess boundary, so stale graph output is not being treated as authoritative process evidence
        - proceeding with Unix-only cleanup that snapshots live descendants immediately before termination, kills escaped descendants individually, and then kills the original process group; successful probes pay no process-table scan, and Windows remains on its existing kill-on-close Job Object path
        - adding a portable `cfg(unix)` Level-1 fixture that reports a quiet `setsid()` descendant PID through a unique file channel and verifies that PID no longer exists after the bounded runner returns
        - implemented descendant discovery with the existing `sysinfo` dependency on the Unix termination path; descendants are signaled deepest-first before the original process group, closing the `setsid()` escape without changing the Windows Job Object implementation or the successful subprocess hot path
        - the new fixture passes its PID-file location through the command environment, establishes a new session, writes only its PID to that separate file channel, and then remains quiet; the regression observed `ESRCH` immediately after timeout return
        - focused verification passed the new regression and all 11 subprocess tests, including large dual-pipe output, inherited-pipe cleanup, direct-child reaping, builder preservation, and shell-safety coverage
        - canonical Sniff-area targets passed independently: 1,666 `sniff-lib` tests and 779 `sniff-cli` tests passed (16 and 3 skipped, respectively), and `just lint` passed in 57.17 seconds; two combined `just test` attempts were stopped after crossing the non-interactive ceiling during recompilation, after their library legs passed, rather than waiting beyond the required bound
        - the exact dirty tree passed the Windows GNU all-target remote-feature compile guard with four existing target-gated test warnings; no Linux Rust target is installed on this macOS host, while all Unix-only imports and fixtures are `cfg(unix)`-gated and the shared APIs are supported on macOS and Linux
        - final `git diff --check` passed; GitNexus `detect_changes(all)` reported LOW risk across the concurrent tracked changes (4 files, 8 stale-index symbols, and no affected execution flows), with this finding confined to `sniff/lib/src/process.rs` plus its implementation log
        - no implementation or performance measurement was deferred for this finding
- work completed for 'quiet session-detached Unix descendant cleanup' at 22:53:39
- starting the work on 'remove unapproved public aggregate Git contract' at 22:54:40
        - read the required Sniff, Rust, Rust-testing, and GitNexus impact-analysis guidance before tracing the aggregate request, observation, and projection boundaries
        - GitNexus was 13 commits stale and its prescribed refresh was stopped by the 55-second non-interactive ceiling; the stale graph reported HIGH impact for `GitInfo` (3 direct, 18 total) and `GitRepo::detect_with_request` (13 direct), plus CRITICAL impact for `detect_filesystem_with_request` (10 direct, 69 total across four process families), so direct callers and both affected CLI performance-output traces were reviewed before editing
        - removed the public, Serde-skipped `GitInfo::aggregate` field, the public `GitAggregateEvidence` export, and the serializable `GitMetadataRequest::aggregate` flag/builder/accessor; `GitMetadataRequest::all()` now consistently enables every one of its eight approved metadata controls
        - added the dedicated `detect_repo_aggregate` library boundary, returning an opaque `RepoAggregateObservation` that splits into the stable `FilesystemInfo` plus `RepoAggregate`; crate-private aggregate evidence is collected while the original `GitRepo` handle is still available
        - preserved aggregate behavior with one repository discovery, one status walk, one ref walk, zero redundant worktree opens, one file-aware history observation, an offline path, and a pure `build_aggregate_value` projection; the aggregate snapshot and JSON schema remain unchanged
        - added compatibility coverage that pins the exact eight-key `GitMetadataRequest` serialization contract and compiles a downstream `GitInfo` literal without aggregate-only evidence, while asserting that neither wire shape contains an `aggregate` key
        - focused verification passed all 20 library aggregate-view tests and all 31 CLI aggregate tests, including schema, snapshot, stdout/stderr, offline, work-counter, and pure-projection coverage
        - canonical `just test` passed all 1,667 `sniff-lib` tests and all 780 `sniff-cli` tests (16 and 3 skipped, respectively); canonical `just lint`, focused Rust/CLI compilation, the Sniff skill Darkmatter hash update, and `git diff --check` passed
        - the optional combined Windows GNU cross-check reached both changed crates without a source diagnostic but was stopped at the 55-second ceiling before Cargo reported success, so it is not claimed as a pass; native cross-platform evidence remains owned by finding 1 and no implementation or performance measurement was deferred for this finding
        - final GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent review-cycle worktree (15 files, 36 stale-index symbols, and 2 affected CLI performance-output traces); both traces were reviewed and are covered by the green complete-command performance and clean-JSON output tests
- work completed for 'remove unapproved public aggregate Git contract' at 23:15:41

### Successful Completion

The implementation of review cycle 5 has completed successfully in 44 minutes and 1 second (22:33:06–23:17:07 local time). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was deferred (see reasons below):

- **Finding 1, native Linux and Windows Level-1 execution and retained work-count artifacts** — deferred because this native macOS host has no authorized native Linux or Windows execution path. Cross-compilation, Docker, WSL, workflow definitions, and results from another tree do not satisfy the finding. Closure requires the immutable final cycle-5 implementation to pass the canonical Level-1 suite natively on macOS, Linux, and Windows, with the matched three-OS work-count artifacts retained under one identifier; full details are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.

The files changed cover Unix process-tree cleanup and its quiet detached-descendant regression, the dedicated opaque aggregate observation boundary, removal of the unapproved public Git request/result additions, aggregate compatibility and work-count coverage, Sniff skill/spec maintenance, the deferred cross-platform evidence record, and review-cycle metadata.

- final Sniff-area verification passed all 1,667 `sniff-lib` tests and all 780 `sniff-cli` tests (16 and 3 skipped), `just lint`, focused aggregate and subprocess suites, focused Rust/CLI compilation, and `git diff --check`
- the Windows GNU all-target check for the subprocess change passed; the later optional aggregate cross-check reached the changed crates without diagnostics but was stopped before Cargo reported success, so it is not claimed as a pass
- final GitNexus `detect_changes(all)` reported MEDIUM risk across the concurrent review-cycle delta, with both affected aggregate performance-output traces reviewed and covered by green complete-command performance and valid-JSON output tests
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run

## Implementation of Review Findings #6

> **started at:** 2026-07-18T05:34:55-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-6.md'
- this is iteration 6 of the review-to-implement cycle
- review-6 contains 4 findings (3 High, 1 Medium):
        1. High: native Linux and Windows Level-1 execution remains absent
        2. High: bare aggregate JSON performs unrequested Markdown and formatting observation
        3. High: Unix descendant termination loses escaped children after direct-parent exit
        4. Medium: aggregate timing instrumentation reads clocks when collection is inactive
- starting the work on 'native Linux and Windows Level-1 execution' at 05:36:02
        - `uname -a` identified the execution host as native arm64 macOS (Darwin 25.5.0); `rustup target list --installed` shows only `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, and `x86_64-pc-windows-gnu` — no Linux target and no native Linux or Windows execution host
        - the constraint is identical to review cycle 5: cross-compilation proves compilation only, Docker is not native Windows evidence, and a workflow definition is not an execution record; this session additionally prohibits VM startup, external workflow triggers, installs, commits, and pushes
        - deferred again as a platform and execution-authority constraint, not a CPU-load deferral; `deferred_perf_measurement: true` remains set in this log's frontmatter
        - the cycle-6 entry recording the exact implementation identifier and closure procedure is appended to `sniff/features/2026-07-16-performance/deferred-perf-tests.md`
- work completed for 'native Linux and Windows Level-1 execution' at 05:36:02
- starting the work on 'bare aggregate JSON performs unrequested Markdown and formatting observation' at 05:36:30
        - confirmed the finding's premise before changing behavior: `RepoRequest::focused()` sets `structure_only: true`, so `WalkConsumers::repo_full` is false and `include_docs` was the *only* consumer forcing `WalkScope::Repository` on the aggregate boundary
        - audited `build_aggregate_value` (sniff/cli/src/output/repo_json.rs:707) and every input it reads: `identity`, `aggregate.repo`, `aggregate.commits`, `aggregate.context`, `git.file_changes`, `git.status`, `git.config`, plus `primary_language_name` (reads `fs.languages`, already empty under `.without_file_inventory()`) and `render_repo_root` (reads `fs.repo.root`)
        - `FilesystemInfo.docs` and `FilesystemInfo.formatting` are read by **no** aggregate field; `documentation_changes` is confirmed Git-derived — `aggregate_commit_family_value` filters the single `CommitDescSet` and touches no filesystem Markdown inventory
        - applied `.without_docs().without_formatting()` to the aggregate request in `detect_repo_aggregate` (sniff/lib/src/filesystem/repo/aggregate_view.rs), with a WHY comment at the request naming docs' role as a repository-wide walk consumer
        - byte-identity check on a clean fixture (2-member Cargo workspace, 11 Markdown files, `.editorconfig`, one unstaged source edit): built the pre-change and post-change binaries separately and `cmp`'d their `repo --json` output — **byte-identical**. Fixture counters: `filesystem.walk.entries_visited` 24 -> absent (0), `filesystem.docs.documents_parsed` 22 -> absent (0), `filesystem.repo.manifest_parses` 3 -> 3 (unchanged)
        - the same check against this worktree at `--base sniff/lib` diffed only in the four places my own in-progress edit to `aggregate_view.rs` appears as an unstaged change; no schema or projection difference
        - cited counters on this worktree, `sniff --base sniff/lib --perf repo --json`: `filesystem.walk.entries_visited` **10,926 -> 0 (absent)**, `filesystem.docs.documents_parsed` **6,974 -> 0 (absent)**; `filesystem.repo.manifest_parses` 79 and `filesystem.repo.nested_marker_walks` 1 unchanged. Wall time (directional only, loaded host) `detect.total` 983 ms -> 618 ms
        - `filesystem.repo.nested_marker_walks: 1` predates this change — shallow structure detection already paid it while a `WalkScope::Repository` walk was also running, so removing the shared walk does not add it
        - added Level-1 command-boundary test `aggregate_command_path_observes_no_markdown_or_formatting` (sniff/cli/src/commands/mod.rs), alongside the existing `aggregate_command_path_discovers_and_walks_status_once`. Fixture carries `README.md`, `docs/guide.md`, `docs/reference.md`, `.editorconfig`, a `Cargo.toml`, and one unstaged source change
        - the test asserts `FS_DOCS_PARSED == 0` and `FS_WALK_ENTRIES == 0`, that `FilesystemInfo.docs`/`formatting` are `None`, that the rendered aggregate object contains no `docs`/`markdown`/`formatting`/`editorconfig` key, that Git-derived `documentation_changes` still renders, and that the pre-existing Git bounds (1 discovery, 1 status walk, 1 ref walk) still hold
        - verified the guard actually bites: with `.without_docs().without_formatting()` temporarily removed the new test FAILS with `filesystem.docs.documents_parsed: 6` and `filesystem.walk.entries_visited: 8`; restored, it passes
        - `just test` in the sniff area: 1,667 `sniff-lib` tests passed (16 skipped) and 781 `sniff-cli` tests passed (3 skipped); `repo_aggregate_json_snapshot` is green, which is the independent no-schema-regression check
        - `just lint` clean (clippy + fmt check, exit 0); no write-mode formatter was run
- work completed for 'bare aggregate JSON performs unrequested Markdown and formatting observation' at 05:55:42
- starting the work on 'Unix descendant termination loses escaped children after direct-parent exit' at 05:56:20
        - evaluated both resolutions the finding offers. **(a) alone is not portably achievable**: a descendant that forks *and* calls `setsid()` between any two observations, whose parent then exits, cannot be contained on POSIX without Linux cgroups or `PR_SET_CHILD_SUBREAPER` (Linux-only, process-global — not a library's to set for its host) or a supervising process. macOS has neither. **(b) alone under-delivers**: the specific case the review names — a descendant that detaches while the parent is *live* — is closable, so narrowing to cover it would concede more than necessary
        - **decision: implement (a) as far as POSIX allows, then narrow the documented contract to exactly the residual gap.** Containment: sample the child's descendant tree on a coarse interval while the child is alive, accumulating a cumulative PID set that survives the reparenting following the child's exit; at cleanup signal that set plus the process group plus one fresh scan for late forks
        - **PID-reuse hazard identified and closed**: a cumulative PID set is only safe if identity is re-validated at kill time, or cleanup could `SIGKILL` an unrelated process that inherited a recycled PID — a strictly worse failure than missing a daemonizing helper. Each recorded PID is stored with its `sysinfo` `start_time()` and only signaled when the live process still reports the same start time. The cleanup scan the design already performs supplies the validation for free
        - **discovery — the fast path was already paying for this, and worse.** Review-5's fix placed `descendant_pids()` inside `terminate()`, which line 529 called *unconditionally* after the supervision loop. Every one of the 18 `run_with_timeout`/`run_command_with_timeout`/`run_for_stdout` call sites therefore performed a full `sysinfo` all-process refresh on **success**. Measured on this host (1,518 processes): one scan costs **16.4 / 21.4 / 32.8 ms**. Directional end-to-end per-probe `echo`: HEAD **34.4 ms** vs fix **15.2 ms** (host is loaded by concurrent agents, so treat end-to-end as directional; the isolated scan measurement is the attributable figure)
        - the sampling design therefore *removes* cost from the fast path rather than adding it. First sample is deferred a full `DESCENDANT_SAMPLE_INTERVAL` (250 ms), so any probe completing inside 250 ms — nearly all of sniff's — performs **zero** process-table scans. `terminate(force_scan)` scans only when descendants were actually recorded, or on a failure path where the deadline has already elapsed. Net: fast successful probes are strictly cheaper than HEAD; only long-running probes (`git fetch`, service-enrichment chunks) pay ~4 scans/sec, bounded
        - `terminate()` now clears the recorded set after signaling, so the timeout path's second (post-loop) call does not re-scan
        - Windows is untouched behaviorally: `sample()` is an empty body and `terminate()` ignores `force_scan`, because Job Object membership is inherited and kernel-enforced. Signature parity only
        - added Level-1 Unix fixture `child_detaches_descendant_then_exits` + test `a_detached_descendant_is_terminated_after_its_parent_exits_successfully` (sniff/lib/src/process.rs). The fixture spawns the existing `quiet_detached_descendant` (which `setsid`s and reports its PID through the `SNIFF_DETACHED_PID_FILE` channel), waits via `getpgid` until the detachment is *real*, outlives three sample intervals, then **exits 0**. The test asserts the direct child succeeded (not timed out) and that the reported PID is gone. Reaping by init is asynchronous, so existence is polled to a 3s bound rather than asserted once — a surviving fixture sleeps 30s, so the bound cannot mask a regression
        - **proved the fixture bites**: temporarily restored review-5 behavior (never record descendants; always scan at cleanup) and the new test FAILED on all 4 nextest attempts — `detached descendant 68153 survived its parent's successful exit`. Restored, it passes in 0.87s
        - **contract narrowed honestly** in three places, since the code cannot deliver unconditional tree termination on Unix: module `//!` docs (new "What tree termination guarantees" section stating Windows is total, Unix is layered — process group guaranteed, `setsid` escape best-effort — and naming the exact residual gap and why it is not portably closable), the `run_with_timeout` rustdoc `## Notes` (pointing at the module docs and warning against routing a deliberately-detaching command through the boundary), and `sniff/docs/sniff-library-architecture.md` (Subprocess Deadlines section, replacing the unqualified tree-termination claim and recording the scan-cost measurement that justifies deferred sampling)
        - no sniff probe daemonizes, so no caller in this crate reaches the residual gap; this is documented rather than enforced, as the finding's second option permits
        - `just test` in the sniff area: **1,668 `sniff-lib` passed** (17 skipped; 1,667 -> 1,668 is the new test) and **781 `sniff-cli` passed** (3 skipped). All pre-existing subprocess tests green: large dual-pipe output, both-pipes-concurrent, inherited-pipe cleanup, direct-child reaping, builder cwd/env preservation, shell-safety, and both prior descendant tests
        - focused `cargo nextest run -p sniff --features remote -E 'test(/process::tests::/) and not test(/level2_/)'`: 12 passed
        - `just lint` clean (exit 0); `cargo check -p sniff -p sniff-cli --all-targets --features sniff/remote --target x86_64-pc-windows-gnu` passed with only pre-existing target-gated warnings, **zero** from `process.rs`; `git diff --check` clean; no write-mode formatter was run
- work completed for 'Unix descendant termination loses escaped children after direct-parent exit' at 06:13:37
- starting the work on 'aggregate timing instrumentation reads clocks when collection is inactive' at 06:15:10
        - **Pattern adopted: `performance::is_collecting().then(Instant::now)`**, the crate's existing "equivalent collection guard" precedent at `filesystem/file_types/classify.rs:264`, applied to both `detect.filesystem` and `detect.total` in `detect_repo_aggregate`. One `let collecting = performance::is_collecting()` feeds both `.then(Instant::now)` calls; each `record_logged_stage` is now inside an `if let Some(..)`
        - **Why not `StageTimer::start`** (the review's first suggestion): `StageTimer::finish` calls `record_stage`, not `record_logged_stage`, so adopting it would silently drop the INFO `"performance stage complete"` tracing events these two stages emit today — a behavior change *while collection is active*, which the finding did not ask for. The guard chosen is byte-for-byte behavior-preserving whenever a collector is installed and costs one relaxed atomic load otherwise. Recorded here because a future reader will reasonably ask why the aggregate boundary does not use `StageTimer` when its sibling stages in `system_view.rs` / `classify.rs` do
        - discovery: **the disabled-path clock read is not observable through the performance collector at all.** `record_stage` already requires `is_collecting() && collector_installed()`, so with no collector the stage is dropped either way — the sibling `system_view::uncollected_walk_records_nothing_into_a_later_request` assertion (`!counts.recorded_any_stage()`) would have passed against the *unfixed* code. The one real observable is the tracing event: `record_logged_stage` emits unconditionally, so an ungated `Instant::now()` still logs 2 INFO events on the uncollected path
        - consequently the new test installs a ~30-line `tracing::Subscriber` (test-only, no new dependency) that counts INFO events targeting `sniff::performance` on the calling thread. `register_callsite` deliberately returns `Interest::sometimes()` — the default impl caches a definite answer process-wide and would suppress the callsite for every later test in the binary. Every stage nested inside aggregate detection logs at DEBUG *and* runs on spawned threads a thread-local dispatcher never sees, so the INFO count is exactly the two stages the aggregate boundary owns
        - **guard proven to bite**: temporarily restoring `let started = Some(Instant::now())` / `let filesystem_started = Some(Instant::now())` made `uncollected_aggregate_reads_no_clock_and_records_no_stage` FAIL with `left: 2, right: 0` — the two ungated stages, exactly. Fix restored from a pre-edit copy and re-verified green
        - tests added to `aggregate_view.rs`'s `work_counts` module: `uncollected_aggregate_reads_no_clock_and_records_no_stage` (disabled path) and `collected_aggregate_records_both_detection_stages` (enabled path still records both stage names). Added `WorkCounts::recorded_stage` / `WorkCounts::stage_names` to `performance::testing` to support the latter
        - **deviation from the brief**: the coverage is a *sibling* of the existing opt-in test rather than an extension of it. `system_view`'s test owns a non-git fixture and cannot call `detect_repo_aggregate`; `aggregate_view`'s `work_counts` module already has the git fixture and the `measure` import. The new test's doc comment names the sibling contract explicitly so the pair stays discoverable
        - note: both tests assert on process-global `is_collecting()`, so they require nextest's process-per-test isolation — under plain `cargo test` (shared process, threaded) a concurrent test's collector makes the precondition assert fire. Same constraint the existing `system_view` test already carries; the canonical `just test` recipe uses nextest
        - `sniff --base sniff/lib --perf repo --json` after the change still emits both stages: `detect.filesystem` (calls 1) and `detect.total` (calls 1) — `--perf` output unchanged
        - `just test` in the sniff area: **1,670 `sniff-lib` passed** (17 skipped; 1,668 -> 1,670 is the two new tests) and **781 `sniff-cli` passed** (3 skipped), exit 0
        - `just lint` clean (exit 0, zero warnings); no write-mode formatter was run
- work completed for 'aggregate timing instrumentation reads clocks when collection is inactive' at 06:31:12

### Successful Completion

The implementation of review cycle 6 has completed successfully in 1 hour and 0 minutes (05:34:55–06:35:10 local time). During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 1 was deferred (see reasons below):

- **Finding 1, native Linux and Windows Level-1 execution and retained three-OS work-count artifacts** — deferred because this native arm64 macOS host has no authorized native Linux or Windows execution path. `rustup target list --installed` carries no Linux target, Windows targets cannot execute on macOS, and this session prohibits VM startup, external workflow triggers, installs, commits, and pushes. Cross-compilation proves compilation only; Docker is not native Windows evidence; a workflow definition is not an execution record. This is a platform and execution-authority constraint, not a CPU-load deferral. Full detail and the closure procedure are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md`.

The files changed cover the aggregate request's removal of unrequested docs and formatting observation, the gated aggregate stage instrumentation, Unix cumulative descendant tracking with PID-identity revalidation, the narrowed and now-accurate process-tree termination contract in both rustdoc and the library architecture document, and the three new Level-1 regressions that pin each fix.

- final Sniff-area verification passed on a warm build with exit code 0: **1,670 `sniff-lib` tests passed** (17 skipped) and **781 `sniff-cli` tests passed** (3 skipped); `just lint` passed with zero warnings
- an earlier run of the same suite reported 13 flaky `sniff-cli` spawn tests that all passed on retry; the clean re-run reported zero flakes, so the retries were host load from concurrent agents rather than a defect
- measured outcome of finding 2 on this worktree: `filesystem.docs.documents_parsed` fell from 6,974 to zero and `filesystem.walk.entries_visited` from 10,926 to zero, with the aggregate `repo --json` output verified byte-identical by building the pre-change and post-change binaries and comparing them on a clean fixture
- measured outcome of finding 3: the review-5 design had placed a full `sysinfo` process-table scan on the **success** path of all 18 subprocess call sites, costing 16.4–32.8 ms per probe on this 1,518-process host; deferring the first sample by a full interval removes that cost entirely for probes completing quickly, so the containment fix is a net reduction rather than an addition
- each of the three fixes was proven to bite by temporarily reverting it and confirming the new regression fails, then restoring
- the Windows GNU all-target remote-feature cross-compile passed with zero warnings from the changed process code
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run

## Implementation of Review Findings #8

> **started at:** 2026-07-18T09:46:42-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-8.md'
- this is iteration 8 of the review-to-implement cycle
- review-8 contains 4 findings (2 High, 2 Medium):
        1. High: native Linux and Windows Level-1 execution and matched work-count artifacts remain absent
        2. High: the installer timeout disclosure breaks the public Rust contract but is discarded by the main callers
        3. Medium: the between-samples Unix regression can report a green test without testing the residual
        4. Medium: completion records still contradict the implemented and reviewed state
- the reviewed implementation identifier is HEAD `98fa4d992adf13a85c31995b3feb3d270a9a26d1`
- starting the work on 'native Linux and Windows Level-1 evidence' at 09:47:03
        - `uname -a` identified the execution host as native arm64 macOS (Darwin 25.5.0); `rustup target list --installed` shows only `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, and `x86_64-pc-windows-gnu` — no Linux target and no native Linux or Windows execution host
        - `git branch -r --contains 98fa4d992adf13a85c31995b3feb3d270a9a26d1` returned empty, confirming the reviewed commit exists only in this local worktree and no hosted CI run for that SHA can exist
        - the constraint is structurally identical to cycles 5, 6, and 7: cross-compilation proves compilation only, Docker was explicitly ruled inadmissible by the review, and a workflow definition is not an execution record; this session additionally prohibits commits, pushes, credential helpers, VM startup, and external workflow triggers
        - deferred again as a platform and execution-authority constraint, not a CPU-load deferral; `deferred_perf_measurement: true` remains set in this log's frontmatter
        - the cycle-8 entry recording the exact implementation identifier and closure procedure is appended to `sniff/features/2026-07-16-performance/deferred-perf-tests.md`
- work completed for 'native Linux and Windows Level-1 evidence' at 09:47:30
- starting the work on 'installer timeout contract propagated end to end' at 09:48:10
        - chose **one timeout contract expressed as a distinct variant at every layer**, rather than reusing `PackageManagerFailed` with a marker string, or reverting `InstallCapturedResult::timed_out` to close the source break
                - the marker-string option was rejected because callers would have to string-match to discriminate a deadline kill from a package-manager verdict, which is exactly the conflation the finding names
                - reverting the field was rejected because R12.5 requires a *defined result on timeout*, so the flag must exist; the fix is to propagate it, not delete it
        - the layers are `SniffInstallationError::InstallationTimedOut { pkg, manager, timeout_secs }` (legacy `Result` APIs) → `InstallInterviewEvent::TimeoutWarning { prose }` (semantic event) → `InstallInterviewOutcome::TimedOut { attempted }` (session outcome) → CLI `Prose` render plus non-zero exit
        - warning ordering is load-bearing and is emitted **after** the error `Status` and **before** the retry prompt in `handle_failure` (`interview.rs`), so the user reads the detached-descendant hazard before choosing to run a second installer
        - deliberate non-change: `RetryChoice::Quit` after a timeout still returns `AbortedByUser`, preserving existing "user chose to stop" semantics; the `TimeoutWarning` event fires regardless, so the hazard is still disclosed
        - files/symbols changed: `error.rs:167` (new variant + rustdoc stating the best-effort-kill/partial-install contract); `install/execute.rs` (extracted `install_result_from_outcome`, shared by `execute_install`/`execute_versioned_install`, timed-out arm preceding the generic failure arm, both `## Errors` sections updated); `install/command.rs:165` (`build_install_timeout_warning`); `install/interview.rs` (new event/outcome variants, private `InstallExecutor<'a>` test seam, `handle_failure` gains `timed_out: bool`); `install/mod.rs` and `programs/mod.rs` re-exports; `cli/src/install_ui.rs:87` (`Prose` render matching the existing `ConsentWarning` idiom in the same file); `cli/src/install_plan_cmd.rs:157` (non-zero exit)
        - the accepted source break is now recorded in the umbrella spec's "Intentional changes" list (`spec.md:312`), naming all four additions, the R12.5 justification, and the caller migration note — this was the finding's "no migration or separately approved contract is recorded" complaint
        - the weak helper at `execute.rs:457` now asserts `result.timed_out`, which strengthens 4 pre-existing tests
        - 7 tests added: 3 legacy injected-runner tests (including `legacy_install_still_reports_ordinary_failure_as_package_manager_failed` as the contrast case proving the new variant is not blanket), 3 interview tests (including `timeout_warning_precedes_the_retry_prompt`, which records `events.len()` at `choose_retry` and asserts the warning index is strictly lower), and 1 CLI `Prose` render test
        - **proven to bite**: temporarily reverted the timeout arm, the warning/outcome block, and all four `timed_out: matches!(e, ProcessError::Timeout)` sites; `uv_install_honors_requested_timeout` and `uv_bootstrap_honors_requested_timeout` panicked at `execute.rs:478`, and both new interview tests failed through all 4 nextest retries; restored and re-verified green
        - sniff-area verification: `just test` gave **1,677 `sniff-lib` passed** (19 skipped) and **782 `sniff-cli` passed** (3 skipped), exit 0 — up from the review's 1,671/781 baseline by exactly the 7 added tests, confirming they execute under the recipe's `--features remote`; `just lint` exit 0 with zero warnings
        - **orchestrator-caught regression outside the sniff area**: adding enum variants broke two downstream package areas the subagent never compiled, because its verification was correctly scoped to `sniff`
                - `biscuit-speaks/cli/src/install_ui.rs`, `biscuit-speaks/cli/src/main.rs`, and `playa/cli/src/install_ui.rs` all match `InstallInterviewEvent`/`InstallInterviewOutcome` exhaustively and failed with `E0004`
                - migrated all three by hand, each matching its own file's existing rendering idiom (`strip_prose_tags(..).yellow()` for biscuit-speaks, `Prose::new(..).render(&self.terminal)` for playa) rather than importing a new one; `biscuit-speaks` `main.rs` gained a `TimedOut` arm exiting non-zero, with a comment recording that the interview already emitted the hazard warning and this arm only supplies the terminal verdict
                - a workspace-wide grep confirmed `sniff`, `playa`, and `biscuit-speaks` are the only consumers of either type or of `SniffInstallationError::`
                - `cargo check -p biscuit-speaks-cli -p playa-cli --all-targets` passed, and `cargo nextest run -p biscuit-speaks-cli -p playa-cli` passed **112/112**
                - this is the concrete migration cost the finding predicted; it is now paid rather than latent
        - Level 2 and Level 3 were not attempted, as the review explicitly states they are not required absent a new real-terminal styling or OS-input requirement
        - scoped out: making `SniffInstallationError`/`InstallInterviewOutcome` `#[non_exhaustive]`, because that would itself be an unreviewed contract change and would break the in-repo exhaustive matches; the spec bullet instead records the added variants as accepted source breaks, consistent with how the existing `GitRequest`/inventory breaks are recorded
- work completed for 'installer timeout contract propagated end to end' at 10:52:00
- starting the work on 'deterministic between-samples Unix regression' at 10:53:15
        - chose the **injectable sampler** (the review's first choice); bounded retries were rejected because each retry re-runs the same unsynchronized race, so a loaded runner would just burn N attempts and then report a failure that is a host artifact rather than a containment regression
        - the entire `skew_ms` / `fork_ms / interval_ms` heuristic and its `return`-on-skip branch are **deleted**, so the no-verdict success path is structurally gone rather than merely made unlikely
        - mechanism: `sample_hook` (`process.rs:134`, gated `#[cfg(all(unix, test))]`) holds a **thread-local** `FnMut` slot — thread-local because `ProcessTree::sample` always runs on the thread that called `run_command_with_timeout`, so an installed hook cannot leak into a concurrently running test
                - `install` (`:151`) returns an RAII `HookGuard` (`:143`) that disarms on panic; `after_sample` (`:156`) takes the hook out of the slot for the call so a re-entrant hook cannot trip a `RefCell` borrow panic
                - the single production-side call is `super::sample_hook::after_sample()` at `process.rs:331`, at the very end of `sample()` under `#[cfg(test)]`
        - why it is deterministic: the test's hook writes a release marker then *blocks* until the escaped descendant publishes a reparent marker, and the descendant (`between_samples_descendant`, `:1016`) only publishes that marker after observing `getppid()` change — which cannot happen until the direct child has exited
                - therefore no sample can run while the hook blocks, and by the time it returns the child is provably exited, so the loop's next `try_wait` breaks immediately and no sample can run afterwards either
                - the fork/`setsid`/exit sequence lands strictly between two samples at any host load
                - portability: the `getppid()` comparison is against the recorded original parent rather than `ppid == 1`, which handles Linux subreapers and PID-namespace inits — this matters because CI runs this natively on Linux where descendant discovery goes through `/proc`
        - `EscapedDescendant` (`:1060`) was reworked to an `Arc<AtomicI32>` slot filled by the hook the instant the fixture publishes a PID, so the RAII cleanup guard is armed **before** any assertion runs and covers a panic anywhere after the fork
        - **no-verdict path proven gone**: 20/20 consecutive focused runs passed (`PASS=20 FAIL=0`); the skip `eprintln!` no longer exists in source, so a green run necessarily executed the `libc::kill(pid, 0)` assertion, and two independent guards (`assert!(reparent_file.exists(), ..)` and `escapee.pid().expect(..)`) would fail rather than skip if the handshake broke
        - **proven to bite, via the inverse experiment**: this test asserts the escapee *survives* (it pins the residual gap review-6 documented), so reverting sampling would make it pass more easily rather than fail
                - the condition that actually flips it was run instead: the descendant was patched to publish its marker immediately after `setsid`, and the child made to outlive a sample boundary, so a sample observes the descendant while still parented
                - result **10/10 FAIL** deterministically, with the intended message `descendant NNNNN was contained; Unix tree termination is now stronger than the module documentation claims — invert this test and correct the docs`; the patch was reverted from a byte-for-byte backup and restoration verified
        - **fast-path zero-scan behavior preserved**: the supervising loop, `DESCENDANT_SAMPLE_INTERVAL` (250 ms), and the deferred-first-sample logic are byte-identical; `git diff` on production code is exactly one doc paragraph, one `#[cfg(all(unix, test))]` module, and one `#[cfg(test)]` call, and a non-test build compiles the hook out entirely — PID-reuse revalidation via `start_time()` is untouched
        - verification: `just test` gave **1,677 `sniff-lib` passed** (19 skipped) and **782 `sniff-cli` passed** (3 skipped), matching the post-installer baseline exactly because the fixtures were rewritten in place; `just lint` exit 0; focused `cargo nextest run -p sniff --features remote -E 'test(/process::tests::/) and not test(/level2_/)'` 13/13 with the target test at ~0.39s; `ps` confirmed no orphaned 30-second sleepers after the runs
        - flagged, not fixed (pre-existing, out of scope): `cargo clippy --all-targets --features remote -- -D warnings` reports 5 `zombie_processes` plus 1 `items_after_test_module`, verified identical against `HEAD:sniff/lib/src/process.rs`; canonical `just lint` does not pass `--all-targets` and CI's `sniff-cross-platform` runs `cargo check` rather than `clippy -D warnings`, so neither gate currently sees them
        - also flagged: `cargo fmt --check` reports 235 pre-existing crate-wide diffs (10 in `process.rs`), all in untouched code and consistent with the known local-rustfmt-versus-`main` drift; no write-mode formatter was run
- work completed for 'deterministic between-samples Unix regression' at 11:12:31
- starting the work on 'reconcile contradictory completion records' at 11:13:05
        - **Part A — cycle-7 record reconstructed** strictly from primary evidence: the two cycle-7 commits `882f5538b` (docs, 08:26:24) and `98fa4d992` (source, 08:26:39, 4 files, +225/-8), `review-7.md`, and the "Review 7 deferred items" section of `deferred-perf-tests.md`
                - both the per-finding block and its `### Successful Completion` section carry an explicit blockquote marking them reconstructed during cycle 8 and naming the sources, so a reviewer can distinguish reconstructed from contemporaneous record
                - **what could not be established is marked unrecorded rather than invented**: cycle 7's own `just test` / `just lint` / `git diff --check` results (no log entry, and neither commit message states one), the per-finding start and completion timestamps beyond the single logged 08:14:20, and the cycle end time
                - the duration is therefore stated as a **bound** — at most 14m 32s (08:12:07 → last commit 08:26:39) — not as a measurement
                - the only corroborating test figures (1,671 sniff-lib / 781 sniff-cli) are cited as **review-8's** measurement of the tree at 08:27:46, explicitly not as cycle-7's own record
        - **Part B — Phase 8 versus source.** Phase 8 claimed in two places that "the large synthetic service-listing timing row remains deferred" for an API-surface reason; the source contradicts this
                - `bench-internals = []` exists in `sniff/lib/Cargo.toml`; `register_service_shapes` in `sniff/lib/benches/cases/workload_matrix.rs` is `#[cfg(feature = "bench-internals")]` and registers `workloads_service_listing/{500,2000}` over `[500usize, 2_000]`; `sniff/lib/benches/README.md:203` documents both plus their two counter-bound tests
                - per repo convention the code is correct and the doc is wrong, so both claims were rewritten to record cycle 4 as the resolution and to name the structural acceptance bound (`1 + ceil(N / 128)` runner calls, `process.spawns`), keeping the honest "no wall timing claimed" qualifier
                - Phase 8's "remaining limitations" list was rewritten so the native-platform item is stated as still open as of cycle 8 — macOS-only host, reviewed commit on no remote — with cross-compilation, Docker, WSL, and workflow definitions explicitly ruled out as substitutes; Phase 8 now makes no claim that this evidence exists
                - Phase 8's "Post-review verification" block (1,657 / 777) now leads with a note that those are historical cycle-3 phase-boundary figures, not current counts
        - **Part C — one further contradiction found.** `phases/06-remote-network-and-subprocess/spec.md:188` stated unqualified that cleanup "terminate[s] the whole group/job", which cycles 5–7 disproved on Unix
                - added a paragraph recording that the preceding text is the cycle-3 boundary and that the guarantee is asymmetric: Windows total, Unix layered with a real `setsid()` between-samples escape, reachable through the install boundary, now disclosed via `InstallCapturedResult::timed_out` and `SniffInstallationError::InstallationTimedOut`, with `process.rs` module docs as the authority
                - checked and deliberately left alone: Phase 4's "Not done at the Phase 4 boundary" and Phase 6's "Deferred, with reasons" are already past-tense and boundary-scoped, and Phase 5:308 already records its deferred reuse items as implemented
                - the umbrella `spec.md:313` already carried this cycle's installer timeout contract and its four accepted source breaks, so no further edit was needed there
        - no edited file carries `hash:` frontmatter, so no Darkmatter rehash was required; `git diff --check` passed and only Markdown files were changed by this finding
- work completed for 'reconcile contradictory completion records' at 11:16:40

### Successful Completion

The implementation of review cycle 8 has completed successfully in 1 hour, 33 minutes, and 30 seconds (09:46:42–11:20:12 local time). During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 1 was deferred (see reasons below):

- **Finding 1, native Linux and Windows Level-1 execution and matched three-OS work-count artifacts** — deferred because this native arm64 macOS host has no authorized native Linux or Windows execution path. `rustup target list --installed` carries no Linux target, Windows targets cannot execute on macOS, and `git branch -r --contains 98fa4d992` confirms the reviewed commit exists only in this local worktree, so no hosted CI run for that SHA can exist. This session is prohibited from committing, pushing, invoking credential helpers, starting VMs, and triggering external workflows. Cross-compilation proves compilation only, Docker was explicitly ruled inadmissible by the review, and a workflow definition is not an execution record. This is a platform and execution-authority constraint, not a CPU-load deferral. Full detail and the closure procedure are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md` under "Review 8 deferred items".

The files changed cover the end-to-end installer timeout contract (a distinct error variant, a semantic interview event and outcome, and an explicit pre-retry terminal warning), the downstream migration that contract forced in two other package areas, the deterministic between-samples Unix regression and its `#[cfg(all(unix, test))]` sampler hook, the umbrella spec's record of the accepted source breaks, the reconstructed cycle-7 log section, and the Phase 6 and Phase 8 current-state corrections.

- final Sniff-area verification passed with exit code 0: **1,677 `sniff-lib` tests passed** (19 skipped) and **782 `sniff-cli` tests passed** (3 skipped); `just lint` exit 0 and `git diff --check` clean
- the installer timeout work added exactly 7 tests to the review's 1,671/781 baseline, confirming they execute under the recipe's `--features remote` rather than being silently feature-gated out
- **a cross-area regression was caught by the orchestrator that the finding-2 subagent could not see**: adding enum variants broke exhaustive matches in `biscuit-speaks/cli/` and `playa/cli/`, which are outside the sniff verification scope the subagent was correctly given. All three sites were migrated by hand in each file's own rendering idiom, and `cargo check -p biscuit-speaks-cli -p playa-cli --all-targets` plus `cargo nextest run -p biscuit-speaks-cli -p playa-cli` (**112/112 passed**) now confirm the workspace is whole. This is the concrete migration cost the review predicted for the public source break; it is now paid rather than latent
- each of the two source fixes was proven to bite: the installer timeout regression by reverting the timeout arm, warning block, and all four `timed_out` sites; the between-samples regression by the inverse experiment (10/10 deterministic failures when the escape is made observable to a sample), since that test asserts the escapee *survives* and so cannot be flipped by weakening containment
- the between-samples test's silent no-verdict path is structurally gone rather than merely unlikely: the skip branch is deleted, and 20/20 consecutive focused runs produced a real verdict
- the review-6 fast-path guarantee is preserved — production process code gained only one doc paragraph and one `#[cfg(test)]` call, so a probe completing inside one 250 ms sample interval still performs zero `sysinfo` scans
- two pre-existing issues were flagged rather than fixed, both out of scope: `cargo clippy --all-targets --features remote -- -D warnings` reports 5 `zombie_processes` and 1 `items_after_test_module` in `process.rs` (verified identical against `HEAD`, and invisible to both canonical `just lint` and CI's `cargo check`-based job), and `cargo fmt --check` reports 235 crate-wide diffs consistent with the known local-rustfmt-versus-`main` drift
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run

## Implementation of Review Findings #9

> **started at:** 2026-07-18T11:40:37-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-9.md'
- this is iteration 9 of the review-to-implement cycle
- review-9 contains 3 findings (2 High, 1 Medium):
        1. High: native Linux and Windows execution and matched work-count artifacts are still absent
        2. High: Playa renders a timeout warning but discards the first-class timeout outcome
        3. Medium: the public migration is recorded only in the feature spec
- starting the work on 'Playa discards the first-class installer timeout outcome (review-9 High)' at 11:41:16
        - surveyed both existing in-repo idioms before writing anything: `sniff/cli/src/install_plan_cmd.rs:152` (`execute_install_flow` → `Result<(), Box<dyn Error>>`, exhaustive match, `TimedOut` gets its own message) and `biscuit-speaks/cli/src/main.rs:1111` (exhaustive match → `std::process::exit(1)` per arm); followed sniff's shape because it is the testable one
        - root cause confirmed: `playa/cli/src/main.rs:1009` was `if let Err(e) = run_install_interview(...)`, so every `Ok(outcome)` — `TimedOut`, `Failed`, `NotInstallable` — was dropped and the selected-install loop continued to a zero exit
        - added `install_outcome_verdict(program, &outcome, timeout_secs) -> Result<(), String>` immediately after `install_players`; exhaustive with **no wildcard arm**, so a future variant is a compile error rather than a silent success
        - verdict policy, documented at each arm:
                - `Installed` / `DryRun` → `Ok`
                - `TimedOut` → `Err` naming the deadline, kept textually distinct from `Failed`
                - `Failed` → `Err` "installation failed"
                - `NotInstallable` → `Err` "cannot be installed on this host" (unexpected, since the list is pre-filtered by `ProgramDetector::installable`)
                - `AbortedByUser` → `Ok`; the two idioms disagree here (sniff returns `Ok`, so-you-say exits 1) and sniff was followed deliberately, because Playa's boundary sits inside a multi-select loop over several players where declining one should not kill the others
        - the caller now matches `Ok(outcome)` / `Err(e)` and routes the verdict message through the existing `error_exit`, which already renders via `Prose` + `Terminal` (`playa/cli/src/main.rs:32`); no new rendering path was introduced
        - extracted `CliInstallUi::render_prose_line` in `playa/cli/src/install_ui.rs` and collapsed the three byte-identical `Announcement` / `ConsentWarning` / `TimeoutWarning` arms into one or-pattern — done because `emit` wrote straight to stdout with no seam, so the review's "assert the warning" half was otherwise untestable; behavior-preserving, net -30 lines
        - tests added (all Level 1): 5 verdict tests covering `TimedOut`, `Failed`, `NotInstallable`, `Installed`+`DryRun`, and `AbortedByUser`, plus `timeout_warning_prose_reaches_the_rendered_line` asserting the warning body survives to the emitted newline-terminated line; playa-cli 13 → 19 tests, area total 69
        - **proven to bite**: reverting the `TimedOut` arm to `Ok(())` made `just test` exit 1 with `install_verdict_timeout_fails_and_names_the_deadline` failing all 4 nextest retries at `main.rs:1832` ("a timed-out installer must not be a success"); restored, suite back to 69/69
        - verification in the **playa** area (the area changed): `just test` exit 0 (50 lib+bin, 19 cli, 0 skipped), `just lint` exit 0, `cargo check -p playa-cli --all-targets` exit 0
        - the downstream migration of the `TimedOut`/`Failed` split is now complete across all three consumers; `grep -rn InstallInterviewOutcome --include=*.rs` returns only `sniff/lib`, `sniff/cli`, `biscuit-speaks/cli`, and `playa/cli`, so there is no fourth consumer
- work completed for 'Playa discards the first-class installer timeout outcome (review-9 High)' at 11:57:19
- starting the work on 'public migration recorded only in the feature spec (review-9 Medium)' at 11:57:19
        - documentation-only; no source behavior was changed by this finding
        - `sniff/lib/CHANGELOG.md:5` — the Unreleased section gained four `**Breaking (source):**` Added entries (`InstallCapturedResult::timed_out`, `SniffInstallationError::InstallationTimedOut { pkg, manager, timeout_secs }`, `InstallInterviewEvent::TimeoutWarning { prose }`, `InstallInterviewOutcome::TimedOut { attempted }`), a Changed entry recording that `PackageManagerFailed` / `Failed` narrow to non-timeout failures, and a new `### Migration` subsection with a match-arm example; no version or date was invented
        - `sniff/lib/README.md:629` — the "program key types" list was extended past `InstallOptions` / `InstallResult` with the four timeout-contract types, plus a paragraph stating the Unix best-effort/partial-install caveat against total Windows Job Object containment
        - `sniff/cli/README.md:227` — the installation section gained two paragraphs: the timeout warning renders after the failure status and before any retry prompt, and Unix termination is best-effort so a timed-out install may leave a partial install (with advice to re-check via `sniff software <category>`), while Windows containment is kernel-enforced; sourced from `sniff/lib/src/process.rs:10-48` module docs so the guarantee is not overstated
        - **the category-count drift was worse than review-9 reported.** The review asserts "the authoritative count is nine"; nine was itself a stale intermediate value. The code says **ten**, confirmed three independent ways: 10 `ProgramsInfo` fields (`sniff/lib/src/programs/mod.rs:170`), 5 `rayon::join` pairs = 10 detectors (`mod.rs:233`), and the CLI subcommand list already exposing `notification-helpers` (`sniff/cli/src/args/mod.rs:313`)
                - per repo convention the code is authoritative, so **ten** was written rather than the number the finding asked for; this is a deliberate deviation from the review text and is flagged here for the next reviewer
                - corrected at three README sites (`sniff/lib/README.md` lines 622, 647, and 699 — the last being the `eight` the finding named) and the category table at `README.md:633` gained its missing `Notification Helpers` row, verified against the `NotificationHelper` enum at `sniff/lib/src/programs/enums/categories.rs:371` and ordered to match struct field order
                - the same stale fact in rustdoc (`sniff/lib/src/programs/mod.rs:209, 219, 220`: "all 9 categories", "instead of 9x per category" ×2) was initially left out of scope by the subagent and then **fixed on orchestrator instruction**, because the repo's drift convention requires the comment to be corrected in the same change as its detection
        - no edited file carries `hash:` frontmatter, so no Darkmatter rehash was required
- work completed for 'public migration recorded only in the feature spec (review-9 Medium)' at 12:06:51
- starting the work on 'native Linux and Windows execution and matched work-count artifacts (review-9 High)' at 12:02:11
        - re-verified the constraint rather than inheriting cycle 8's conclusion:
                - `uname -a` → native arm64 macOS (Darwin 25.5.0, `xnu-12377.121.10~1/RELEASE_ARM64_T6041`)
                - `rustup target list --installed` → `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, `x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc`; **no Linux target at all**, and neither Windows target can execute on macOS
                - `git branch -r --contains af4751810e9bc66f3e3dbe5b883c864ce76c77a0` → no remote branch, confirming review-9's own observation for the exact commit it named
        - **deferred** as a platform and execution-authority constraint, not a CPU-load deferral; structurally unchanged from cycles 5 through 8. This session is prohibited from committing, pushing, invoking credential helpers, starting VMs, and triggering external workflows, so no authorized native Linux or Windows execution path exists from this workspace
        - cross-compilation proves compilation only, Docker was ruled inadmissible for this finding by review-7, and a workflow definition is not an execution record
        - relevance note for this cycle: **no Unix or Windows process code was changed**, so the native-platform risk surface is unchanged from cycle 8 — the deterministic between-samples sampler hook and its `getppid()` portability claim have still been exercised natively on macOS only
        - **new this cycle:** the identifier to publish will be a *new* SHA, not `af4751810`, because cycle 9 changed six files across the playa and sniff areas; the three-OS evidence must be gathered against that final tree rather than any earlier reviewed commit
        - full detail and the closure procedure appended to `sniff/features/2026-07-16-performance/deferred-perf-tests.md` under "Review 9 deferred items"; `deferred_perf_measurement: true` remains set in this log's frontmatter
- work completed for 'native Linux and Windows execution and matched work-count artifacts (review-9 High)' at 12:02:21
- addendum to 'public migration recorded only in the feature spec (review-9 Medium)' — the stale-count sweep
        - the initial pass corrected only the sites review-9 named; a follow-up sweep found **eight more live instances of the same stale fact**, all corrected on orchestrator instruction: `sniff/README.md:34,47,138`, `sniff/lib/README.md:12,147`, `sniff/cli/README.md:606,855`, and `sniff/docs/sniff-library-architecture.md:88`; `sniff/cli/README.md:616` also gained the missing **Notification Helpers** bullet so the list and its count agree
        - one of those was **not** the same quantity and was fixed differently rather than bumped: `sniff/README.md:47` read "List and install programs across 9 categories", conflating listing (10 categories) with installing (8). Exactly eight `*Action::Install(` enums exist — Agent, Audio, Editor, LangPkgMgr, OsPkgMgr, TerminalApp, TtsClient, Utility — and `NotificationHelperAction` does not exist at all, with test runners a deliberate report-only leaf. The claim was split instead of renumbered, because writing "10" would have been wrong in the other direction
        - four sites were **verified and deliberately left unchanged**:
                - `sniff/cli/src/output/programs.rs:612,629` ("the eight categories" / "all 8 categories") looks like the same stale count but is not — `build_programs_json`'s `OutputFilter::Programs` arm genuinely joins exactly eight JSON builders, with notification helpers and test runners routed separately upstream in `commands/mod.rs`. Changing it would have *introduced* drift; this is the case for verifying each site rather than running a blanket find-and-replace
                - `review-9.md:58` is the review record itself, and editing it would rewrite the finding this cycle is evidence for
                - `features/_completed/2026-06-14-more-repo/test-runner-strategy.md:89` is archived, and its "the other eight categories" refers to *installable* categories and is still accurate
                - an archived `baseline-repo.json` fixture's "9 ecosystems" is an unrelated quantity
- **process defect caught and corrected during this cycle (orchestrator error).** After running the sniff gates via `cd <repo-root>/sniff`, the shell's working directory persisted into the next command, so the finding-3 log block was appended to a relative path that resolved to `<repo-root>/sniff/sniff/features/2026-07-16-performance/log.md` — a stray 10-line tree one level too deep, invisible to the real log
        - it was found by the finding-3 subagent during its sweep, which checked before acting: it confirmed the stray content was **absent** from the real log rather than a duplicate, and so declined to delete it
        - the content was merged back into the real log immediately after the finding-3 start line, and `sniff/sniff/` was removed; `git status` confirms no untracked tree remains
        - the lesson is the known relative-`cd` hazard in this worktree layout — subsequent appends now use repo-root-anchored invocations

### Successful Completion

The implementation of review cycle 9 has completed successfully in 30 minutes and 12 seconds (11:40:37–12:10:49 local time). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was deferred (see reasons below):

- **Finding 1, native Linux and Windows execution and matched work-count artifacts** — deferred because this native arm64 macOS host has no authorized native Linux or Windows execution path. The constraint was re-verified rather than inherited from cycle 8: `rustup target list --installed` carries no Linux target at all, neither installed Windows target can execute on macOS, and `git branch -r --contains af4751810e9bc66f3e3dbe5b883c864ce76c77a0` returns no remote branch for the exact commit review-9 named. This session is prohibited from committing, pushing, invoking credential helpers, starting VMs, and triggering external workflows. Cross-compilation proves compilation only, Docker was ruled inadmissible for this finding by review-7, and a workflow definition is not an execution record. This is a platform and execution-authority constraint, not a CPU-load deferral. Full detail and the closure procedure are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md` under "Review 9 deferred items".

The files changed cover the Playa command-boundary migration of the installer timeout contract (an exhaustive `InstallInterviewOutcome` match with a documented verdict per variant, plus a rendering seam that makes the timeout warning assertable), and the public documentation migration of that same contract into the library changelog, both public READMEs, the CLI installation section, the library architecture doc, and the `ProgramsInfo::detect` rustdoc.

- final verification passed with exit code 0 in both affected areas: **sniff** — 1,677 `sniff-lib` tests passed (19 skipped) and 782 `sniff-cli` tests passed (3 skipped), `just lint` exit 0, `git diff --check` clean; **playa** — 50 lib/bin and 19 cli tests passed (0 skipped), `just lint` exit 0, `cargo check -p playa-cli --all-targets` exit 0
- the finding-2 fix was **proven to bite**: reverting the `TimedOut` arm to `Ok(())` made `just test` exit 1 with `install_verdict_timeout_fails_and_names_the_deadline` failing all 4 nextest retries; restored, suite back to 69/69 green
- **the review's own stated fact was wrong and the code was followed instead.** Review-9 asserts the authoritative executable-index category count is nine; nine was itself a stale intermediate value. The code says **ten**, confirmed three independent ways (10 `ProgramsInfo` fields, 5 `rayon::join` pairs, and the CLI already exposing `notification-helpers`). Per the repo convention that code is authoritative when drift is detected, ten was written. This is a deliberate deviation from the review text and is flagged for the next reviewer
- the drift was also **wider than the finding reported**: eleven live sites in total across five files, not the one the finding named. Each site was verified individually rather than swept with find-and-replace, which is what caught `sniff/README.md:47` (listing 10 vs installing 8 — two different quantities in one sentence) and `sniff/cli/src/output/programs.rs:612` (a correct "eight" that a blanket replace would have broken)
- the downstream migration of the cycle-8 `TimedOut`/`Failed` split is now **complete across all four consumers** — `sniff/lib`, `sniff/cli`, `biscuit-speaks/cli`, and `playa/cli` — confirmed by `grep -rn InstallInterviewOutcome --include=*.rs` returning no fifth site
- one orchestrator process defect occurred and was corrected in-cycle: a persisted working directory sent the finding-3 log block to a stray path one level too deep. The subagent found it, verified the content was absent from the real log rather than duplicated, and declined to delete it; the content was merged back and the stray tree removed
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run

## Implementation of Review Findings #10

> **started at:** 2026-07-19T20:25:49-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'sniff/features/2026-07-16-performance/review-10.md'
- this is iteration 10 of the review-to-implement cycle
- review-10 contains 3 findings (1 High, 2 Medium):
        1. High: native Linux and Windows execution and matched work-count artifacts remain absent
        2. Medium: Playa's timeout regression tests stop short of the command boundary
        3. Medium: the category migration still leaves authoritative and public documentation contradictory
- starting the work on 'Playa timeout regression stops short of the command boundary (review-10 Medium)' at 20:30:04
        - audit first, per the retry convention: the finding was recorded by `1bff42855` (review cycle 10 findings), and two later commits — `6a2a00d4d test(playa-cli): strengthen install command boundary regression` and `4cf06917e docs(sniff): reconcile program category counts in docs and rustdoc` — landed on 2026-07-18 at 18:03:51, after the review; this cycle therefore verifies rather than re-implements
        - the fix is exactly the seam the review prescribed: `run_install_command` (`playa/cli/src/main.rs:1036`) is an injectable interview-runner seam returning `Result<(), InstallCommandFailure>` (message + exit code), and `install_players` (`main.rs:1009`) now routes `Ok(outcome)`/`Err(e)` through it into `error_exit`, so the selected-install loop can no longer discard the verdict
        - the composed Level-1 regression `install_command_timeout_warns_and_returns_nonzero_verdict` (`main.rs:1870`) drives the seam with a delegate that receives a `Status` error plus a `TimeoutWarning` event and returns `InstallInterviewOutcome::TimedOut`; it asserts exit code 1, a deadline-naming message distinct from plain failure, and that the warning prose survives through the delegate into the captured writer
        - the weak seams the review named are gone: `timeout_warning_prose_reaches_the_rendered_line` was deleted from `install_ui.rs`, and `CliInstallUi` is now generic over `W: Write` with a `#[cfg(test)]` `with_writer`/`into_writer` capture seam, so the event path itself is what the test exercises — removing the `TimeoutWarning` event arm would now fail the composed test
        - `install_verdict_timeout_fails_and_names_the_deadline` (`main.rs:1855`) remains as the pure-verdict helper pin, now a sibling of the composed boundary test rather than the only coverage
        - `install_verdict_timeout_fails_and_names_the_deadline` (`main.rs:1855`) remains as the pure-verdict helper pin, now a sibling of the composed boundary test rather than the only coverage
        - verification gate results recorded below under the cycle verification entries
        - a note on the review's residual-risk framing: the review worried the weak render test "would remain green if the `TimeoutWarning` event arm were removed"; in the current source that arm is one member of an exhaustive or-pattern (`install_ui.rs:72-77`), so removing it is a compile error rather than a silent green, and the composed test additionally guards the loop-discards-verdict regression the review actually named
- work completed for 'Playa timeout regression stops short of the command boundary (review-10 Medium)' at 20:37:42 (verified as already implemented by `6a2a00d4d`; no additional source change required)
- starting the work on 'category migration documentation drift (review-10 Medium)' at 20:37:58
        - this finding was also already implemented after the review was recorded, by `4cf06917e docs(sniff): reconcile program category counts in docs and rustdoc` (8 files: the Sniff skill entry and its `extending.md`/`programs.md` companions, `sniff/README.md`, `sniff/cli/README.md`, `sniff/cli/src/args/mod.rs`, `sniff/lib/README.md`, `sniff/lib/src/programs/mod.rs`)
        - verified both live surfaces the review named against the current tree:
                - `.claude/skills/sniff/SKILL.md:33` — the Programs row now reads "10 categories … Eight categories support installation with remote-bash consent; notification helpers and test runners are report-only", matching the source (10 `ProgramsInfo` fields, 8 `*Action::Install` enums)
                - `sniff/cli/README.md:229` — the installation section now leads with "Eight of the ten detectable program categories support `install` and `install-plan`. Notification helpers and test runners are report-only and expose neither action."
        - the authoritative contract the review stated — ten detectable categories, eight installable — is now what both surfaces say; no residual contradiction found
- work completed for 'category migration documentation drift (review-10 Medium)' at 20:39:20 (verified as already implemented by `4cf06917e`; no additional source change required)
- starting the work on 'native Linux and Windows execution and matched work-count artifacts (review-10 High)' at 20:39:31
        - re-verified the constraint rather than inheriting cycle 9's conclusion:
                - `uname -a` → native arm64 macOS (Darwin 25.5.0, `xnu-12377.121.10~1/RELEASE_ARM64_T6041`)
                - `rustup target list --installed` → `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, `x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc`; **no Linux target at all**, and neither Windows target can execute on macOS
                - `git branch -r --contains 77b3ea5ed0b9fffbc8a88bcca1fcd2bcd9302023` → no remote branch against a fetched `origin`, re-confirming review-10's own observation for the exact commit it named
        - **deferred** as a platform and execution-authority constraint, not a CPU-load deferral; structurally unchanged from cycles 5 through 9. This session is prohibited from committing, pushing, invoking credential helpers, starting VMs, and triggering external workflows, so no authorized native Linux or Windows execution path exists from this workspace
        - cross-compilation proves compilation only, Docker was ruled inadmissible for this finding by review-7, and a workflow definition is not an execution record
        - relevance note for this cycle: **no production source was changed at all** — the two implemented findings were verified as already landed (one test-only seam in playa-cli, one documentation commit), so the native-platform risk surface is unchanged from cycle 9
        - the identifier to publish will be a *new* SHA (current HEAD `8e4520645` plus this cycle's record edits), not `77b3ea5ed`; the three-OS evidence must be gathered against that final tree
        - full detail and the closure procedure appended to `sniff/features/2026-07-16-performance/deferred-perf-tests.md` under "Review 10 deferred items"; `deferred_perf_measurement: true` remains set in this log's frontmatter
- work completed for 'native Linux and Windows execution and matched work-count artifacts (review-10 High)' at 20:46:08
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run

### Successful Completion

The implementation of review cycle 10 has completed successfully in 21 minutes and 37 seconds (20:25:49–20:47:26 local time). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed (both verified as already implemented by commits `6a2a00d4d` and `4cf06917e`, which landed after the review was recorded; this cycle audited them against the current tree and re-ran both package areas' gates), 1 was deferred (see reasons below):

- **Finding 1, native Linux and Windows execution and matched three-OS work-count artifacts** — deferred because this native arm64 macOS host has no authorized native Linux or Windows execution path. The constraint was re-verified rather than inherited from cycle 9: `rustup target list --installed` carries no Linux target at all, neither installed Windows target can execute on macOS, and `git branch -r --contains 77b3ea5ed0b9fffbc8a88bcca1fcd2bcd9302023` returns no remote branch for the exact commit review-10 named. This session is prohibited from committing, pushing, invoking credential helpers, starting VMs, and triggering external workflows. Cross-compilation proves compilation only, Docker was ruled inadmissible for this finding by review-7, and a workflow definition is not an execution record. This is a platform and execution-authority constraint, not a CPU-load deferral. Full detail and the closure procedure are recorded in `sniff/features/2026-07-16-performance/deferred-perf-tests.md` under "Review 10 deferred items".

The files changed by this cycle are records only: this log, `sniff/features/2026-07-16-performance/deferred-perf-tests.md` (the Review 10 deferral entry), and `sniff/features/2026-07-16-performance/review-10.md` (implementation metadata). The verified-already-landed fixes themselves live in `6a2a00d4d` (`playa/cli/src/main.rs`, `playa/cli/src/install_ui.rs`) and `4cf06917e` (the Sniff skill docs, both READMEs, and two rustdoc sites).

- final verification passed with exit code 0 in both affected areas: **playa** — 19/19 `playa-cli` tests passed (0 skipped), `just lint` exit 0; **sniff** — 1,677 `sniff-lib` tests passed (19 skipped) and 782 `sniff-cli` tests passed (3 skipped), `just lint` exit 0
- both medium findings were resolved by post-review commits that predate this session; this cycle's contribution is the audit against the current tree, the composed command-boundary regression's continued green run, and the deferral record for the recurring cross-platform High finding
- no commit, push, external workflow trigger, VM startup, package installation, or write-mode formatting command was run
