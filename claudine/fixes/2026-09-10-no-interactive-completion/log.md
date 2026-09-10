---
fix: 2026-09-10-no-interactive-completion
area: claudine
deferred_perf_measurement: false
---

# Log: Restore Interactive Completion Before Initialize Consumes Caller File Inputs

## Implementation of Review Findings #1

> **started at:** 2026-09-10T14:16:59-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-10-no-interactive-completion/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review contains 8 findings; findings 1 and 2 are the two blockers listed under `## Required before ready: true`
- the impacted package area is `claudine` (packages `claudine` and `claudine-cli`), so `just test`, `just test-l2`, and `just lint` are run from `claudine/` only
- planned disposition:
        - findings 1, 2, 3, 5, 6, 7 are in scope for this cycle
        - finding 4 is a documented judgment call by the reviewer (PTY vs. shared harness) and is not a code change
        - finding 8 is explicitly recorded by the reviewer as outside this fix's scope (shipped prompt content, not code)
- starting the work on 'finding-1-launch-area-anchoring' at 14:20:26-07:00
        - scope confirmed as a **test-only** change; the production scope derivation in `claudine/cli/src/commands/schema_interactive/supplied.rs` is already correct and was left untouched
        - read path traced end to end: the closure builds `ScopeContext { cwd: origin.base_dir(), .. }`, `file_candidate_paths` walks from `scopes::property_value_root(ctx)` (which is `ctx.cwd`), and `path_matches_query` filters the resulting absolute paths — so a decoy that only differs by living outside the launch area is the right discriminator
        - changes made:
                - `claudine/cli/tests/level2_provided_partial_file_pty.rs` — `review_router_fixture` now also seeds `<repo-root>/fixes/2026-09-10-local-decoy/spec.md`, which matches the same `fixes/2026-09-10-local` substring but sits outside the `packages/example` launch area
                - same file — `review_router_session` was split so `review_router_session_in(fixture, router, launch_dir)` can launch the identical fixture from a different directory
                - same file — the single-match test now asserts the chooser did not render (no keyboard-enhancement push) and that the provider prompt does not contain `local-decoy`
                - same file — new test `level2_review_router_partial_repo_root_launch_widens_candidates_to_the_chooser` launches the same fixture from the repository root, waits for the chooser, and selects the decoy (`SELECTED=decoy`)
                - `claudine/cli/tests/common/pty.rs` — `KBD_ENHANCEMENT_PUSH` raised from private to `pub(crate)` so the confirmation-vs-chooser assertion can reuse it
        - non-vacuity evidence — perturbation 1 (production): scope `cwd` changed from `origin.base_dir()` to `origin.repository_root()`
                - `level2_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy` **FAILED** — the transcript shows the chooser listing both `fixes/2026-09-10-local-decoy/spec.m` and `packages/example/fixes/2026-09-10-l…` instead of the confirmation
                - `level2_review_router_partial_chooser_keeps_selected_identity_in_proxy` and `level2_review_router_partial_decline_and_cancel_stop_before_initialize` also **FAILED**
                - `level2_review_router_partial_repo_root_launch_widens_candidates_to_the_chooser` **PASSED**, which is correct: its launch directory already is the repository root
        - non-vacuity evidence — perturbation 2 (the reviewer's exact claim): production still root-anchored **and** the decoy seed removed
                - the three pre-existing router tests went **green again** (3 passed), reproducing the vacuity finding 1 reported; only the new repo-root test **FAILED** (no chooser to wait for)
                - this is the proof in both directions: the decoy is what makes root anchoring observable, and the new repo-root test genuinely observes the out-of-area candidate rather than passing by construction
        - non-vacuity evidence — perturbation 3 (fixture, isolates the `local-decoy` assertion): decoy re-seeded *inside* the launch area as the sole candidate with `marker: alpha`, `local-a` withheld, and the helper's directory argument relaxed to `spec.md`
                - `SELECTED=alpha`, the directory check, `TOKEN=retained`, the single-dialog count, and the no-lifecycle-error check all still passed; `assert!(!prompt.contains("local-decoy"))` was the **sole failure**, with the panic showing `SPEC=…/packages/example/fixes/2026-09-10-local-decoy/spec.md`
        - honest caveat on one assertion: `!transcript.contains(KBD_ENHANCEMENT_PUSH)` cannot fail independently, because `wait_for_marker("Use this file")` panics first on every path that renders the chooser; it is kept as an explicit restatement of "confirmation, not chooser" and is commented as such
        - all three perturbations were reverted; the production file is byte-identical to its pre-perturbation state and the fixture is back to seeding `local-a`, the optional `local-b`, and the out-of-area decoy
        - verification (run from `claudine/`):
                - `just test-l2 review_router` — 4 tests run, **4 passed**
                - `just test-l2 --no-fail-fast` (full L2 suite) — 245 tests run, **243 passed, 2 failed**
                - both failures are `level2_dry_run_document_cell_renders_osc8_link_in_wezterm` and `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`; both panic on a captured frame containing `Atuin AI is not yet configured. / Enable Atuin AI`, i.e. the recorded Atuin-wedge host trap in the WezTerm capture panes — unrelated binaries, untouched sources, no PTY or schema involvement
                - `just lint` — clean; no clippy warnings, no errors, and the `claudine-cli::error_guards` lint suite reported 18 tests run, 18 passed
                - no terminal or browser window was brought into focus; the new test is an `expectrl` PTY session and opens no window
- work completed for 'finding-1-launch-area-anchoring' at 14:34:53-07:00
- starting the work on 'finding-2-vacuous-assertion' at 14:39:23-07:00
