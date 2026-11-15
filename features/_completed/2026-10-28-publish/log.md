---
feature: 2026-10-28-publish
implementation_1: "2026-11-09T13:02:58-08:00"
---

# Publish — review fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-09T13:02:58-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/features/2026-10-28-publish/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` flag confirmed `false` before repairs began
- no local OS testing rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); testing runs on the macOS host only

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Rollout records installation success without proving the required channels | fixed | All required children (F1.1–F1.3) fixed; sibling routes F1.4–F1.8 verified clean; `just distribution-test` 447 OK, `just test test-toolkit` 524 passed |
| R1-F1.1 | Homebrew workflow + candidate generation: `brew: pass` only after successful isolated Homebrew installation checks on supported targets | fixed | New `brew-install` job (4 targets, read-only, no secrets) runs `tap.py install-check`; `rollout_candidate` yields `pass` only with tap success + `formula_current` + `brew-install` success, otherwise `unchanged`/`disabled` or an error. Tests: `RolloutCandidateTests.test_channel_results_follow_the_installation_jobs` (replaces the test that expected the unsupported pass), `test_a_run_that_did_not_prove_every_enabled_route_proposes_nothing`, `test_tap.py` `FormulaCurrentTest`, `InstallCheckTest`, `InstallCommandLineTest`; Rust contract `distribution_records_brew_pass_only_after_homebrew_installs_on_every_target` (mutation-checked) |
| R1-F1.2 | First-chain candidate + widening gate: first bh entry with disabled channels must not unlock wider rollout | fixed | `check_rollout` requires an all-`pass` bh entry; tests `test_only_a_run_that_installed_every_channel_proposes_the_first_chain`, `RolloutPreflightTests.test_an_incomplete_bh_chain_refuses_the_second_package_before_any_run_lookup` |
| R1-F1.3 | Ledger parser + widening gate: incomplete channel evidence cannot qualify as the first complete chain | fixed | `RolloutEntry.channels` retained; incomplete entries remain valid records but never qualify; distinct "proves the complete chain" error. Tests `test_an_entry_with_any_channel_not_installed_is_a_record_but_never_the_first_chain` (also through `check_policy_rollout`), `test_a_newer_incomplete_entry_does_not_hide_an_older_complete_chain`, `test_the_candidate_command_reads_the_tap_output_strictly` |
| R1-F1.4 | Archive smoke route executes archived command | clean | `smoke` job runs `build.sh smoke`/`smoke-container` (lines 301–335): extracts the archive, runs `--version`/`--help`, fails on nonzero/empty output, `ldd` check on Linux; upload needs smoke, install needs upload, candidate needs install |
| R1-F1.5 | Shell installer route executes installer in isolated dir | clean | `install_check.sh` downloads the Release installer, requires byte equality with the built one, installs into a scratch HOME with system-only PATH, runs `--version`/`--help` |
| R1-F1.6 | Binstall route runs real binstall, no compile fallback | clean | `verify_binstall.py manifest` uses `--strategies crate-meta-data`, requires one installed file, checks `--version` and the Release download URL; candidate requires `success` when enabled and `skipped` when disabled (both directions tested) |
| R1-F1.7 | Local runtime verifier reports pending without live evidence | clean | `verify_runtime.py` marks enabled channels `pending(...)`; `Check` rejects `pass` for pending/disabled — `test_channels_disabled_by_policy_and_enabled_ones_pending`, `test_check_rejects_pass_for_pending_or_disabled` |
| R1-F1.8 | Prerelease formula decision records unchanged, never manufactures proof | clean | Prerelease → `formula_update: false` → tap and brew-install skipped → `unchanged`; candidate refuses either job having run; `tap.py publish` returns `skip` with `formula_current: false`; `install-check` refuses a prerelease |
| R1-F1.9 | (sweep discovery) Tap `noop` status conflated "already holds this version" with "holds a newer version" | fixed | `publish` report gains a strict `formula_current` field validated per status by `formula_current()`; control and invalid-shape tests in `FormulaCurrentTest` |

### Progress

- starting the work on 'R1-F1: Rollout records installation success without proving the required channels' at 13:04:09
        - defect class: a channel's installation result is inferred from a different operation (publication-job success) or from a disabled channel, and that inferred result is then consumed as proof that authorizes a wider decision
        - sweep sites identified by the orchestrator before delegation: `rollout_candidate` channel mapping (archive, shell-installer, brew, binstall), `_rollout_entry` / `ROLLOUT_CHANNEL_RESULTS` (ledger parser), `check_rollout` / `check_policy_rollout` / `gate_rollout` (widening consumers), the `homebrew-tap` → `rollout-candidate` job wiring in `distribute-binaries.yml`, the tap report statuses (`created`/`updated`/`noop`/`skip`), `verify_runtime.py` pending reporting, Rust workflow contracts in `tools/test-toolkit/tests/ci_workflow_contracts.rs`, and `docs/distribution.md` §7
        - delegated to one foreground subagent (single finding; no parallel agents)
        - subagent: `rollout_candidate` now takes `brew_install` and `formula_current`; `brew: pass` requires the tap job's success, a tap report saying the formula names this version, and the new `brew-install` job's success; every other combination of job results is a `DistributionError` (`scripts/ci/distribution.py`)
        - subagent: `RolloutEntry` keeps its channel results; `check_rollout` accepts as the first chain only the newest bh entry whose four channels are all `pass`, and raises a distinct "proves the complete chain" error naming each bh entry's non-pass channels; `disabled`/`unchanged` entries remain valid ledger records
        - subagent: `tap.py publish` reports `formula_current` (created/updated → true; noop → true only when the tap already holds this exact version; skip and dry-run → false); new `tap.py formula-current` reads it strictly against the selection; new `tap.py install-check` runs `brew tap`/`install`/`list --versions`/the installed command's `--version`/`uninstall` through the `Runner` seam with tokens stripped and the host checked against the target
        - subagent: Homebrew pinned at Homebrew/brew 7.0.12 (commit bf464f57275cae0be106b43d190c98564ac59b0e, published 2026-11-02; the host's own brew is 7.0.12) for runners without Homebrew
        - subagent: `just distribution-test` green after the Python changes (447 tests, OK)
        - subagent: workflow: `homebrew-tap` exposes `formula_current` (read by `tap.py formula-current`); new `brew-install` matrix job (all four build targets, `contents: read`, no secret, `fail-fast: false`, 45 min) needs `[preflight, homebrew-tap]`, runs only when the tap succeeded and holds this version, installs pinned Homebrew/brew only where the runner has none, runs `tap.py install-check`, uploads `distribution-brew-install-<target>`; `rollout-candidate` needs it and passes `--brew-install` and `--formula-current`
        - subagent: discovery: Homebrew 7's tap-trust gate is satisfied by `brew install <user>/<tap>/<formula>` (`cmd/install.rb` calls `Trust.trust_fully_qualified_items!`), so the install check needs no `brew trust` step, matching what a user types
        - subagent: Rust contract `distribution_records_brew_pass_only_after_homebrew_installs_on_every_target` added; `brew-install` and `rollout-candidate` added to the credential/existence and production-event contracts; mutation check (dropping `brew-install` from the candidate's `needs`) fails the new contract
        - subagent: docs/distribution.md (flow diagram, job table, preflight list, §7 channel-result table and complete-chain rule, §9 "Installing from the tap", §10, verify-run list), the workflow header, the rust-devops skill's distribution contract, `distribution.py`'s module docstring, and the `rollout.json` pending reasons (`approval`, `first-chain`) updated to the all-`pass` rule
        - subagent: verification: `just distribution-test` 447 tests OK; `cargo nextest run -p test-toolkit --test ci_workflow_contracts` 219/219 passed; actionlint 1.7.12 (with shellcheck 0.11.0) clean; shellcheck clean on the extracted Homebrew bootstrap; pinned tag clone resolves to the pinned commit
        - orchestrator reconciliation: inspected the combined diff of `distribution.py`, `tap.py`, and the workflow; channel mapping, ledger gate, and job wiring match the design and the tests named in the ledger exist and pass
        - decisions recorded for author review (also added to the review's `human_review_items`):
                - Homebrew install proof is a `tap.py install-check` subcommand behind the existing `Runner` seam (alternative: standalone shell script) for unit testability
                - "formula current" is an explicit strict report field (alternatives: parse the reason text, compare versions in workflow shell)
                - a tap that already holds this exact version counts as current and can reach `pass` (alternative: every no-op is `unchanged`)
                - the widening gate needs all four channels `pass`, so the bh release that opens rollout must enable both `brew` and `binstall` (alternative: allow `disabled` channels bh does not ship)
                - the gate uses the newest qualifying bh entry, not the newest bh entry overall
                - pinned Homebrew/brew 7.0.12 (commit `bf464f5…`) is installed only on runners without Homebrew; runners that ship Homebrew use their own version; the pin lives in the job env (single consumer) rather than `build-images.json`
- work completed for 'R1-F1: Rollout records installation success without proving the required channels' at 13:19:30

### Completion Report

- **outcome: complete** — elapsed about 17 minutes (13:02:58 → 13:20)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 9 (F1.1–F1.3 and the sweep discovery F1.9 fixed; F1.4–F1.8 clean with stated invariants)
- verified repairs:
        - `brew: pass` now requires a real `brew install` from the public tap on every target via the new `brew-install` job; publication-job success alone never yields `pass`
        - the rollout widening gate requires a bh ledger entry whose archive, shell-installer, brew, and binstall channels are all `pass`; incomplete entries stay valid records but unlock nothing
- open finding/child IDs: none
- live-evidence notes (spec-sanctioned pending states, not open rows): `brew-install` has never run in real CI because the tap repository, its credentials, and an approved bh release do not exist yet; Homebrew on `ubuntu-22.04-arm` and runner Homebrew detection are proven only by the fake-runner tests and actionlint
- changed files:
        - `scripts/ci/distribution.py`, `scripts/ci/test_distribution.py` — candidate mapping, ledger channel retention, complete-chain gate (F1.1–F1.3)
        - `scripts/distribution/tap.py`, `scripts/distribution/test_tap.py` — `formula_current` report field, `formula-current` and `install-check` subcommands (F1.1, F1.9)
        - `.github/workflows/distribute-binaries.yml` — `homebrew-tap` output, new `brew-install` job, candidate wiring (F1.1)
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` — workflow contracts for `brew-install` and candidate wiring (F1.1)
        - `.github/distribution/rollout.json` — pending reasons restate the all-`pass` rule (F1.2)
        - `docs/distribution.md`, `.claude/skills/rust-devops/SKILL.md` — flow, job table, §7 channel results and complete-chain rule, §9 tap installation (F1.1–F1.3)
- final verification:
        - `just distribution-test` — 447 tests OK, none skipped (F1.1–F1.9)
        - `just test test-toolkit` — 524 passed, 4 skipped (pre-existing skips), 3 slow (F1.1 contracts)
                - the 3 slow tests (`no_archive_executed_target_bakes_in_a_producer_path`, `the_shipped_planner_emits_plans_the_guards_reader_accepts`, `report_resolves_the_worktree_base_by_the_wt_contract`) are untouched by this cycle and are also 5.4–6.7 s when rerun alone, so they are slow regardless of load, not load-sensitive; they predate this change and are unrelated to R1-F1
        - `cargo clippy -p test-toolkit --all-targets -- -D warnings` — clean (`tools/test-toolkit` has no `lint` recipe)
        - `actionlint` 1.7.12 on `distribute-binaries.yml` — clean (the older 1.6.23 on PATH rejects the pre-existing `macos-15` label only)
        - `.github/distribution/rollout.json` parses; the repository ledger is also loaded by the distribution suite
- no local OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, and `BUILD_MACOS` are unset, so only macOS evidence was produced
