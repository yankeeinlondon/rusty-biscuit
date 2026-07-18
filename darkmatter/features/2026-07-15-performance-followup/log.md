---
implementation_5: "2026-07-17T14:19:23-07:00"
implementation_6: "2026-07-17T17:33:52-07:00"
deferred_perf_measurement: true
---

# Implementation Log — Performance Follow-up

## Implementation of Review Findings #5

> **started at:** 2026-07-17T14:19:23-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-15-performance-followup/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- starting the work on 'verify-hash-orchestration-seam-removal' at 14:21:17
        - verified `darkmatter/lib/src/internal.rs` does not exist (removed)
        - verified no `internal-hash-orchestration` feature in `darkmatter/lib/Cargo.toml` (only `default` and `effects-instrumentation` remain) or `darkmatter/cli/Cargo.toml` (no features section)
        - verified `darkmatter/lib/src/lib.rs` has no `pub mod internal` — public modules are only `catalog`, `diff`, `editor`, `effects`, `layout`, `markdown`, `mermaid`, `prelude`, `render`, `style`, `terminal`, `testing`
        - verified `plan_hash_save_explained` does not exist anywhere in the source tree
        - verified `Markdown::diff_hash` at `darkmatter/lib/src/markdown/hash/explain.rs:471` is private (no `pub` keyword) with a rustdoc note explaining the pairing must stay private per compatibility invariant 2
        - verified `Markdown::plan_hash_save` at `darkmatter/lib/src/markdown/hash/save.rs:63` is `pub fn` (the public two-call path)
        - verified CLI at `darkmatter/cli/src/commands/hash.rs:154-155` uses the public two-call path for `--diff` (`compare_hash` + `explain_hash_diff`) and at `:177/:183` for `--save` (`plan_hash_save` + `explain_hash_diff`), with a comment documenting that the artifact is computed twice
        - next: run the focused hash library tests and spawned-CLI behavior tests to confirm output/persisted values/exit statuses remain compatible
        - ran `cargo nextest run -p darkmatter -E 'test(/hash/)'`: 148/148 hash-related library tests pass
        - ran `cargo nextest run -p darkmatter-cli -E 'test(/hash/)'`: 41/41 CLI hash tests pass (includes `hash_kind_save_diff` and `hash_directory` spawned-CLI suites which freeze exit statuses, persisted values, and output)
        - no replacement seam found: the public hash API is `hash`, `hash_frontmatter`, `hash_body`, `compare_hash`, `explain_hash_diff`, `plan_hash_save`, `compute_hash`, `apply_hash_save` — the same surface that existed before the review-4 remediation; the shared `diff_hash` helper is now a private method with a rustdoc note recording that publishing the pairing would breach compatibility invariant 2
- work completed for 'verify-hash-orchestration-seam-removal' at 14:28:00
- starting the work on 'verify-parked-waiter-test' at 14:24:32
        - verified the `parked_waiters: HashMap<String, usize>` field at `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:1001` is `#[cfg(test)]`-gated, so production builds carry neither the field nor its bookkeeping
        - verified the counter is incremented under the shared mutex immediately before `wait_timeout_while`'s atomic park (types.rs:1145-1151) and decremented immediately after the wait returns (types.rs:1160-1167)
        - verified `parked_waiters_for_test` (types.rs:1215) is `#[cfg(test)]`-gated and reads the count under the same mutex, so a positive reading proves the waiter is enqueued on `reservation_done`
        - verified the test `handler_error_notifies_a_waiter_blocked_on_the_same_command` (mod.rs:3086) blocks the approver's handler on a spin-loop over `parked_waiters_for_test("echo shared")` with a 5s `WAITER_PARK_DEADLINE` that fails loudly if the waiter never parks
        - verified the waiter thread's outcome is received via `outcome_rx.recv_timeout(NOTIFICATION_BUDGET)` (10s), well under the 30s `RESERVATION_WAIT_TIMEOUT`, so a missed `notify_all` produces a bounded deterministic panic rather than a nextest-timeout
        - next: mutation-check by temporarily removing `notify_all` in `complete_allow_once`, run the test expecting a bounded deterministic failure, then restore immediately
        - ran the test with the implementation intact: PASS in 0.834s
        - mutation-check performed: commented out `self.reservation_done.notify_all();` at `types.rs:1187` and re-ran the test
            - result: FAIL in 10.847s with the exact designed panic at `mod.rs:3195`: "the waiter was still parked after 10s: the failed flow released its reservation without waking it, so the waiter is sitting out the full RESERVATION_WAIT_TIMEOUT"
            - the failure is bounded (10.8s ≪ nextest's terminate-after ceiling) and deterministic (every retry failed identically, TRY 4 FAIL)
        - restored `notify_all` immediately after the mutation run and re-ran the test: PASS in 0.932s; `git diff` confirms no residual source change from the mutation
        - confirmed the test-only hook does not alter production behavior: both `parked_waiters` (field) and `parked_waiters_for_test` / `pending_allow_once_for_test` (accessors) are `#[cfg(test)]`-gated, and the increment/decrement bookkeeping inside `reserve_allow_once` is wrapped in `#[cfg(test)]` blocks — a production build carries neither the state nor the writes
- work completed for 'verify-parked-waiter-test' at 14:28:00

## Implementation of Review Findings #6

> **started at:** 2026-07-17T17:33:52-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-15-performance-followup/review-6.md'
- this is iteration 6 of the review-to-implement cycle
- the review contains one actionable finding plus one tracked production-readiness blocker:
        - **Finding (Low):** the terminal-detection test documentation names the wrong evidence file and tier
        - **Tracked blocker (not a new finding):** the integrated compose-regression threshold remains unestablished — the review explicitly states this must stay deferred in `performance-compliance.md` rather than be carried forward as a finding
- starting the work on 'fix-terminal-detection-test-doc-drift' at 17:35:05
        - verified the named evidence file does not exist and enumerated the real files under `biscuit-terminal/lib/tests/`: `level2_terminal_osc_wezterm.rs` (Level-2 interactive PTY WezTerm OSC evidence) and `level1_terminal_osc_cache.rs` (Level-1 OSC-cache query-count test); `level2_terminal_osc_cache.rs` is not present
        - edited the module `//!` doc in `darkmatter/cli/tests/compose_terminal_detection.rs` (comment-only; no test code, assertions, or classification touched)
            - old: `The interactive (PTY) OSC evidence lives in biscuit-terminal's` `level2_terminal_osc_cache.rs` `.`
            - new: `The interactive (PTY) OSC evidence lives in biscuit-terminal's Level-2` `level2_terminal_osc_wezterm.rs` `; the Level-1 OSC-cache query-count test is` `level1_terminal_osc_cache.rs` `.`
            - fix points readers to the real Level-2 interactive evidence and, for completeness, separately names the Level-1 cache test at its correct tier
        - ran `cargo clippy -p darkmatter-cli --tests` as the compile+lint gate: finished clean, no warnings or errors introduced (confirms the test file still compiles)
- work completed for 'fix-terminal-detection-test-doc-drift' at 17:38:08-0700

### Successful Completion

The implementation of review cycle 6 has completed successfully in approximately 5 minutes 16 seconds. During this implementation all 2 review items were evaluated to see if they could be fixed as a part of this implementation cycle — 1 actionable finding and 1 tracked production-readiness blocker: 1 was fixed, 1 was deferred (see reasons below):

- **Deferred — tracked production-readiness blocker: the integrated compose-regression threshold.** This is a deferred *performance measurement*, not a code finding, and review-6.md explicitly directs that it stay deferred in `performance-compliance.md` rather than be carried forward as a review finding. Establishing a pass/fail against the feature's 5% integrated compose threshold requires an admissible quiet-host benchmark run: the predeclared contract in `performance-compliance.md` requires the 1-minute load average to stay below 2.0 for the full capture. At capture time the host reported a 1-minute load average of **30.56** (5-minute 49.57, 15-minute 42.72) on a 16-core machine — more than fifteen times the admissibility ceiling — so no legitimate measurement could take place. The attempt is recorded under *Attempt and Result Log → 2026-07-17 — Review 6 implementation cycle* in `performance-compliance.md`, and it maps back to the *Tracked Production-Readiness Blocker* section of `review-6.md`. The three required commit SHAs remain committed objects, so the measurement can be completed unchanged once a quiet host is available; no owner ruling is outstanding.

The files touched by this cycle are:

- `darkmatter/cli/tests/compose_terminal_detection.rs` — the comment-only fix for the one actionable finding (module `//!` doc now points to the real Level-2 evidence `level2_terminal_osc_wezterm.rs` and separately names the Level-1 cache test `level1_terminal_osc_cache.rs`).
- `darkmatter/features/2026-07-15-performance-followup/performance-compliance.md` — appended the review-6 inadmissible-host attempt record for the deferred compose-regression measurement.
- `darkmatter/features/2026-07-15-performance-followup/log.md` — this implementation log.
