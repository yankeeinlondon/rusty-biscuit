---
total_phases: 5
created: 2026-09-26
phase: 1
agent: codex/default
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - sniff/lib/src/process.rs
    - sniff/lib/src/lib.rs
    - sniff/lib/Cargo.toml
    - playa/lib/src/detached/mod.rs
    - playa/lib/src/detached/tests.rs
    - playa/lib/Cargo.toml
    - biscuit-speaks/lib/src/detached.rs
    - worktree/lib/Cargo.toml
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remove/safety.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - Cargo.lock
docs_updated_during_phase_2:
    - docs/dependencies.md
    - sniff/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/os/windows.md
    - .claude/skills/os/build-hosts.md
source_files_during_phase_3:
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/pr_refresh.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/remove.rs
    - worktree/lib/src/remove/handoff.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
packages:
    - sniff
    - playa
    - biscuit-speaks
    - worktree
    - worktree-cli
---

# List and remove performance implementation plan

## Scope and success criteria

Implement `2026-09-25-list-remove-performance`: serve origin-bound PR cache entries immediately, refresh stale entries in a detached process, and restrict the removal handoff's network work to evidence its approved action still needs. Move the existing detached-child configuration from `playa` into `sniff::process`. Preserve full content fingerprints, remote deletion leases, first-run removal reporting, and live dirty status.

Completion means:

- A matching stale PR answer renders immediately with its age while a bounded worker refreshes it. Cache misses retain the foreground 300 ms request deadline. Changed/missing origins, future timestamps, and old formats never expose old badges.
- Concurrent refresh workers cannot fetch simultaneously; successful refreshes become visible on the next list, failures preserve the store, and worker crashes release the lock.
- Capturing the parent's output returns while its refresh is blocked, including on native Windows; workers use the main checkout as their working directory.
- Handoffs keeping a branch or explicitly deleting it make no second-run network request without remote deletion. Automatic deletion uses fresh local proof first and otherwise revalidates network evidence. Changed local content or remote deletion approvals refuse before mutation.
- Deterministic L1 tests and the full-command stale-cache performance gate pass, with recorded execution evidence for macOS, Linux, native Windows, and WSL2. Existing warm/cold performance bounds and relevant removal regressions remain intact.
- Documentation reflects the new contracts. Implementation is complete and ready for review; the author retains ownership of moving the fix to `_completed`.

No watcher, daemon, metadata-only fingerprint, persistent refresh service, change to Git status flags, or reuse of cached listing PRs for removal safety is in scope.

Phases are sequential checkpoints. Within each numbered wave, tasks marked **parallel** can be assigned to separate subagents with the stated file ownership. Waves have unique indices across the plan; a later wave starts after the preceding wave's checkpoint. Shared manifests, lockfiles, and final documentation edits have one integration owner. Every delegated brief must preserve the non-interactive session constraint; no prompts, credential setup, commits, or focus-stealing tests.

## Phase 1 — Resolve contracts and risks

### Necessary Rules

The revised specification is authoritative for this implementation. It is still marked `draft-spec`; do not silently change its status or reopen its settled decisions. Apply the following explicit interpretations and record the interface decisions before implementation:

1. **Local proof excludes all remote-tracking refs.** The current `safety::classify` accepts `origin/<default>` without a live check. That cannot be the handoff's local shortcut. In the second run, remote-only proof, including the remote default, must be verified live. Keep first-run reporting behavior outside this narrowly scoped change unless shared code requires adjustment and regression coverage.
2. **Remote absence is a changed head.** `run_handoff` currently compares SHAs only when the new SHA is present. Under this specification, an approved present head becoming absent must also refuse before local removal; absent-to-present and changed present heads likewise refuse. Verified absent-to-absent can remain a no-op remote deletion. An unavailable answer never equals verified absence.
3. **Lock the stable sidecar, not the replaced JSON file.** Keep a persistent sidecar file and hold its nonblocking exclusive lock through recheck, request, and atomic publication. Never unlink the sidecar to release ownership. Lock contention exits without fetching; lock/open errors leave the store untouched.
4. **Deduplication is concurrent refresh exclusion.** After a successful refresh, a later worker skips on freshness. After failure, a later invocation may retry because failures are not cached. Cold foreground requests retain existing behavior; this plan does not add blocking coordination or a failure cooldown to them.
5. **Identity is the exact Git-returned origin value.** Reuse that value to construct the request source; persist only its `biscuit-hash` digest, never the raw URL. Use repository-explicit Git calls for parent and worker. Missing/unreadable origins mean no reusable answer. Recheck identity before publishing foreground as well as background results so neither writer misbinds its response.
6. **Freshness boundary is explicit.** Preserve the existing convention that an elapsed age of 60 seconds is stale. Reject future fetch times. Determine age-line visibility and minutes at rendering, including when a fresh selection crosses the boundary before rendering.
7. **No fabricated safety result.** Skipping assessment for an approved action is distinct from proving the branch safe. Adjust downstream execution/reporting to accept unneeded facts explicitly; do not invent a `Safe` tier or reuse first-run network answers.

### Wave 1 — Contract and process investigation

- [x] **Map interfaces** — integration owner; parallel with the process spike.
  - Inspect `worktree/lib/src/pull_requests.rs`, `remove/{safety,handoff,remote,inventory}.rs`, and `worktree/cli/src/commands/{list.rs,list_table.rs,remove/mod.rs}` with existing tests.
  - Specify the smallest cache result/refresh-needed interface, repository-explicit origin source, refresh entry point, and handoff local-proof result. Preserve injected clock/provider/remote-head seams for deterministic tests.
  - Write the approved-action matrix: `Keep`, `Delete`, `DeleteIfSafe`, and no attached branch, each with/without remote approval. Identify which facts `execute`, reports, and lost-commit messaging actually require.
  - Record the two implementation/spec gaps above and their regression cases. Confirm no handoff record version bump is needed unless its serialized contract actually changes.

- [x] **Probe process lifecycle** — parallel; owns a bounded spike note and disposable test prototype, not production edits.
  - Load the `os`, `sniff`, and `playa` skills. Inspect `sniff/lib/src/process.rs`, `playa/lib/src/detached/{mod.rs,tests.rs}`, and `.claude/skills/os/windows.md`.
  - Prove a captured parent can finish while a synchronized detached child stays alive, with null standard streams and the existing Windows handle-inheritance fix. Exercise native Windows, not just cross-compilation. Establish cleanup on success, failure, and forced worker termination.
  - Prototype two-process `fs4` lock contention and crash release using readiness acknowledgments and bounded waits, not fixed sleeps. Check atomic store replacement can occur while the sidecar is locked on Windows.
  - Decide how the real worker tests inject a blocking local source without introducing a user-facing test flag or contacting an external provider. Record the public `sniff::process` API boundary and Windows dependency features needed to move the helper unchanged.

- [x] **Confirm baseline** — after both parallel tasks; integration owner.
  - Record current test names and existing evidence; run focused baseline tests/performance measurements where needed, without running a workspace-wide suite.
  - Fix the new stale full-command gate's method before implementation: existing mixed fixture, warm comparison cache, seeded matching stale PR store for every sample, images disabled, best of five, generous 1-second full-command ceiling plus deterministic proof of no wait. Compare fresh and stale measurements on the same host; 80 ms is an observation, not a portable threshold.
  - Checkpoint: interface notes, action matrix, executable process/lock test approach, and baseline are recorded. Any remaining uncertainty has a conservative rule and a concrete test; do not proceed with an unresolved destructive-action policy.

## Phase 2 — Build independent library foundations

### Wave 2 — Three parallel implementation tracks

- [x] **Move detachment helper** — parallel; owns `sniff` and `playa` source/tests/manifests.
  - Expose `sniff::process::configure_detached_child`; retain existing process-group, creation-flag, and standard-handle behavior. Keep unrelated process internals private.
  - Use the existing Windows binding strategy or add the narrowly required target dependency/features. Preserve the caller obligation to set standard streams explicitly; do not run the intentionally detached worker through sniff's bounded-child runner.
  - Migrate all callers and relevant tests, including consumers outside `playa` found by reference search. Remove the old implementation rather than maintaining two copies.
  - Update touched API/module docs, especially sniff's existing claim that every spawned process uses the bounded runner. Verify playback detachment behavior with existing hermetic tests.

- [x] **Implement PR storage** — parallel; owns `worktree/lib/src/pull_requests.rs` and its tests; requests manifest edits from integration owner.
  - Load `biscuit-hash`; bump the PR format version and add the exact-origin digest. Treat corrupt/old/future-dated/mismatched entries as misses. Preserve valid cached empty results as answers.
  - Separate immediate cache selection from refresh execution. Matching fresh entries do not fetch or launch; matching stale entries return badges plus refresh intent; misses fetch under the existing deadline when a source exists.
  - Implement sidecar locking with `fs4` 0.13, reread origin and store under the lock, skip a now-fresh matching result, and atomically publish only a successful response whose origin still matches. Timestamp successful fetches consistently with the settled interface.
  - Preserve stored bytes after provider/authentication failure, identity changes, and failed publication; never turn failure into a successful empty result. Retain source-repository plus branch badge matching.
  - Add L1 clock/source-count/store tests covering the freshness boundary, origin changes even during a fresh cache hit, missing origin, empty answers, corrupt/old stores, future timestamps, failed requests, and changes during fetch.

- [x] **Implement local proof** — parallel; owns `worktree/lib/src/remove/safety.rs` and its tests.
  - Add the narrow second-run assessment path agreed in Phase 1: fresh local default, another local branch, or tag containing the tip can prove deletion without constructing/calling network sources.
  - Exclude the branch being deleted, `origin/HEAD`, and the remote deletion destination from admissible protection. A Git failure produces unknown evidence, never safety.
  - When local proof is absent, retain live PR validation and verify remote-tracking protection, including `origin/<default>`, against current remote heads. Preserve the exact-tip/source-repository PR rules and `--force-remote` exclusions.
  - Use counting `PrSource` and `RemoteHeads` stubs to prove zero calls for local proof and required calls/refusal for missing, moved, or unavailable remote-only protection. Keep first-run reporting tests passing.

- [x] **Integrate foundations** — after the three parallel tasks; integration owner.
  - Add `worktree -> fs4` in `worktree/lib/Cargo.toml`; consolidate dependency/feature changes and `Cargo.lock` once. Review the public helper's callers and the agreed cache/safety signatures.
  - Checkpoint: focused foundation tests pass, all consumers compile, and no new network access occurs in local-proof/cache-hit tests. The helper preserves playback behavior and only its intended API becomes public.

## Phase 3 — Wire list and handoff flows

### Wave 3 — Parallel CLI integration

- [x] **Wire PR refresh** — parallel; owns list rendering, hidden command dispatch, and PR CLI tests.
  - Add a hidden internal subcommand in `args.rs`/`main.rs` that receives the main checkout path explicitly and runs the bounded refresh operation. Register any shared module in both CLI targets as required.
  - Spawn the current executable with that explicit path and working directory, null stdin/stdout/stderr, and the shared detachment helper. Do not wait, join, or capture the worker; suppress normal output for the internal command. Spawn failure leaves stale results usable.
  - Change `commands/list.rs` to select cached results immediately and launch refresh only for stale matching entries. Preserve parallel graph/status gathering and foreground miss behavior. Keep `--perf` measuring the parent's work rather than worker completion.
  - Update `list_table.rs` to derive stale age at render time using the existing biscuit-terminal components. Cover successful cached empty results, boundary crossing, refresh failure, and unavailable first-run answers without fabricated badges.
  - Update `cli/tests/list_prs.rs`, table snapshots, and injected pipeline tests for the origin lookup on every cache hit. Check hidden-command help/completion behavior and absence of wrapper protocol output.

- [x] **Minimize handoff facts** — parallel; owns `commands/remove/` and removal CLI tests.
  - Split local fact collection from optional safety/network assessment. Keep first-run reporting complete; select second-run facts using `Approvals.branch` and remote approval.
  - Consume the one-use token, retain 60-second expiry and exit semantics, verify the caller has left, and re-read the entry, tip/branch, effective include rules, copy baseline, and full fingerprint before any mutation. Release the target working directory as before.
  - For `Keep`, explicit `Delete`, or no local branch action, skip local-branch safety lookup. For `DeleteIfSafe`, use the fresh local proof and then the live fallback only when needed; refuse if protection cannot be established.
  - Independently honor remote approval: recompute destination/push endpoint, reject reinterpretation and multiple endpoints, query the live head, and compare both presence and SHA to the approval before local removal. Preserve the original approved lease and endpoint recheck immediately before push.
  - Preserve partial-result reporting for a post-preflight lease failure and existing refusal exit codes. Update behavior-related comments and avoid misleading output when safety facts were intentionally omitted.

### Wave 4 — Integration checkpoint

- [x] **Verify command contracts** — integration owner.
  - Run focused PR, removal policy/handoff, and shell-wrapper L1 tests through the package recipes. Confirm the new hidden command cannot recurse into list or open terminal windows.
  - Verify each action-matrix row's network count, remote refusal before filesystem/branch mutation, and unchanged first-run behavior. Inspect the final diff for accidental fingerprint, included-file observation, or Git status changes.
  - Checkpoint: both command paths are wired, compile together, and pass their focused behavioral tests; performance evidence and destructive race coverage follow in Phase 4.

## Phase 4 — Prove concurrency, safety, and latency

### Wave 5 — Parallel regression suites

- [ ] **Prove refresh lifecycle** — parallel; owns PR subprocess tests and dedicated fixture support.
  - Use real subprocesses and a blocking local source with readiness/release synchronization. Assert the stale parent renders and exits with captured output before the worker is released; separately assert the worker was started and eventually publishes an answer visible to the next list.
  - Start concurrent list/worker processes; while one request is blocked, prove competitors exit without requests. After success, prove the under-lock freshness recheck skips another request. Kill the lock holder, then prove a later worker refreshes.
  - Verify the worker's working directory is the main checkout, including invocation from a linked checkout. Run the inherited-pipe regression on native Windows. Test spawn failure, failed/authentication refresh, origin changes during fetch, and preservation of stored bytes.
  - Reuse fixture command builders and relocatable binary lookup from the Test Toolkit conventions. Isolate repository/cache state, including native Windows' cache location. Own and terminate/reap workers and local servers even when assertions fail; no nextest leaks or live network calls.

- [ ] **Prove handoff safety** — parallel; owns removal regression tests.
  - Count PR/live-head calls after moving out for keep, explicit delete, local default proof, other local branch proof, tag proof, remote-only proof, and PR-only proof; cross relevant cases with remote approval. Only remote deletion preflight may add network work to the otherwise zero-call cases.
  - Use local bare remotes to remove/move the last remote protection, including remote-default-only proof. Verify refusal leaves worktree files, registration, and branch intact.
  - Cover remote destination/endpoint changes, reinterpretation, live-head changes/absence/unavailability, and a concurrent push after preflight that fails the approved lease with the expected partial result.
  - Add end-to-end handoff regressions for a dirty file larger than 1 MiB and a file inside an untracked nested repository: change bytes while preserving size and restoring modification time, then assert refusal and retained contents.
  - Prove unreadable dirty content refuses deletion using a reliable injected read failure where permissions are insufficient on a platform. Retain index/mode/symlink/rules/baseline mutation, expiry, replay, and caller-still-inside coverage. Do not alter the fingerprint or included-file observation contracts to make tests cheaper.

- [ ] **Prove stale latency** — parallel; owns `cli/tests/perf_pr_request.rs` and performance-only fixture changes.
  - Add the agreed full non-image stale-cache gate using `MixedFixture` and the local blocked refresh. Reseed/verify stale matching state for each sample so warm-up or worker success cannot turn it into a fresh-cache test.
  - Assert cached badges and age output as well as the generous full-command bound. Pair timing with the deterministic blocked-worker test; a 1-second ceiling alone would not detect a reintroduced 300 ms wait.
  - Retain foreground-miss deadline, offline, warm/cold `list gather`, and full-command gates. Update seeded stores and subprocess-count expectations for the deliberate origin lookup without relaxing unrelated bounds.
  - Measure fresh versus stale full-command time on the same fixture/host/profile and record `pr gather` plus total duration. Run timing gates serially with `just -d worktree test-perf`, after other test workloads finish.

## Phase 5 — Validate and document delivery

### Wave 6 — Final evidence and documentation

- [ ] **Run platform gates** — parallel with documentation; one test coordinator avoids conflicting builds and timing contention.
  - Run `just -d worktree test`, then `just -d worktree test-perf`; run affected `sniff` and `playa` L1 suites through their area recipes. Include other direct consumers actually changed by the helper move, not the entire workspace.
  - Run area lint recipes after checking their expansion against the prohibition on running `cargo fmt`; if a recipe invokes it, run its Clippy/check components directly and record that limitation. Do not format or commit without explicit authorization.
  - Use the OS skill's supported cross-check workflow for Linux, native Windows, and WSL2 in addition to macOS, with non-interactive commands. Confirm executed tests and durations, not merely successful skips; WSL2 must exercise relocatable archive paths. Record exact package, environment, command/filter, result, and unresolved gaps.
  - Reuse qualifying evidence; obtain missing evidence for required cases. No speculative CI matrix changes or full-scope CI run. If wrapper/terminal behavior changed, run relevant existing `just -d worktree test-l2` coverage through its harness without raising focus; all new acceptance tests above remain hermetic L1.
  - Fix failures in their owning track, rerun affected checks, and retain no leaked workers. Do not declare cross-platform verification complete while evidence is missing.

- [ ] **Update behavior docs** — parallel; owns documentation and skill edits after interfaces settle.
  - Update `worktree/README.md` for immediate stale badges, foreground misses, age display, and the conditional handoff speedup. Avoid implying every removal becomes network-free.
  - Update `worktree/docs/performance-testing.md` with the stale full-command gate, deterministic no-wait proof, measured fresh/stale results, the added origin Git call, and the investigated Git status CPU/no-watcher findings.
  - Update root `docs/dependencies.md` for `worktree -> fs4` and any actual Windows dependency changes; update existing per-area dependency docs where affected.
  - Update the worktree skill's PR format/freshness/refresh and handoff contracts; update relevant sniff/playa API documentation and skills, plus the old helper reference in `.claude/skills/os/windows.md`. Record any new OS-specific discovery there.
  - Audit touched symbol comments for drift, especially the old zero-Git fresh-cache claim and the assumption that all PR gathering waits for a request. Reference related specs by directory identifier alone.

### Wave 7 — Review handoff

- [ ] **Audit acceptance** — integration owner; depends on all prior work.
  - Map every specification acceptance bullet to passing test names and platform evidence, including the two Phase 1 safety gaps, native Windows pipe handling, worker crash recovery, and same-size/same-time dirty-file edits.
  - Review dependency/API changes, hidden-command behavior, documentation, and timing evidence together. Confirm no raw origin URL was added to the PR store or worker arguments, no cached list answer influences removal safety, and no unapproved metadata shortcut or daemon was introduced.
  - Check off only verified tasks and report any remaining evidence limitation explicitly. End at “implementation complete, ready for review”; leave the fix in place and do not run `just complete`.
