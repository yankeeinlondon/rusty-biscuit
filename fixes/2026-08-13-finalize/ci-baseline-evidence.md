# F8 — CI baseline identity evidence and drafted entries

Closes the **analysis half** of the review-1 CRITICAL finding ("the ratified CI
baseline and identity-diff closure is not implemented"). This document collects
the per-cell identity evidence that Ratified Decision 2 requires *before* an
entry may be written, and carries the reviewed draft TOML.

**Nothing here has been applied.** `.github/ci/ci-baseline.toml` is unchanged.
Phase 5's entry gate — "blocked until all branch-owned P1-P4 identities are
green" — has not been reached, because the P1-P4 fixes are still uncommitted and
no CI run exists for them.

## Source runs

| role | run | workflow | branch | head commit | created |
|---|---|---|---|---|---|
| source (main) | `31588186544` | `ci` | `main` | `e03bafee9` | 2026-08-12T10:36:44Z |
| branch | `31651014023` | `ci` | `fix/ctx-launch-anchor` | `cbe84893a` | 2026-08-12T23:28:25Z |

### The WSL2 run identifier is not separate

`wsl.md` cites main's WSL2 comparison as "main's most recent full run
(31588186544, commit e03bafee9, 2026-08-12)". That is correct and complete:
`.github/workflows/_wsl-ci.yml` is a **reusable** workflow invoked from `ci.yml`,
so the `wsl2-ubuntu` legs are jobs *inside* run `31588186544` (e.g. job
`94137358112`, `claudine / wsl2 (claudine) / test (claudine on wsl2-ubuntu)`),
and that run's `ci-results` artifact carries 85 `wsl2-ubuntu` cells directly.
`gh run list` shows no standalone `wsl`-named workflow run on either day.

**There is no second run id.** Every drafted entry, including the three
`wsl2-ubuntu` cells, uses `source_run = "31588186544"`. `problems.md`'s phrase
"its WSL2 leg" (line 366) should be replaced with the run id for the reason the
spec gives; it is not evidence of a distinct run.

## Method

1. `gh run download <id> -n ci-results -D <dir>` for both runs.
2. Failing identities read from `.records[].failed_tests` (the same field
   `ci-rollup compare` consumes), keyed by `{package, environment, tier}`.
3. `lint` cells emit no JUnit — they are producer-status cells with
   `counts.total == 0`. Their identity evidence was taken out of band, from the
   failing lint jobs' logs (`gh api .../actions/jobs/<id>/logs`), as the exact
   set of clippy diagnostics with `file:line:col`.
4. Cross-checked with a local `ci-rollup compare --base <main> --head <branch>`
   (exit 2, REGRESSED — see "Branch-superset finding").

---

## Per-cell identity table

`=` means the two runs produced the identical failing-identity set.

### 1. `claudine/wsl2-ubuntu/L1` — **=** (3 / 3) — P5.1 (no Rust toolchain)

Both runs (`4030` tests on main, `4042` on branch; 3 failed on each):

- `claudine::composition::error::tests::shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic`
- `claudine::composition::prepare::tests::direct_composition_runs_shell_in_configured_working_directory`
- `claudine::system_prompt::prepare::tests::non_repository_session_runs_shell_in_launch_cwd`

The branch adds 12 tests to this cell and none of them fail. Main handoff:
`problems.md` P5 item 1 (gate on `$RUSTC`/`rustc` discoverability, or pre-build
the cwd probe into the archive).

### 2. `sniff/wsl2-ubuntu/L1` — **=** (1 / 1) — P5.2 (timing budget)

- `sniff::integration::test_detect_completes_in_reasonable_time`

29.5 s against a 20 s cap on the branch; 40.1 s on main. Handoff: `problems.md`
P5 item 2 (environment-gate the budget, or exclude `/mnt/*` from the program
scan).

### 3. `messenger/wsl2-ubuntu/L1` — **=** (1 / 1) — P5.3 (desktop D-Bus)

- `messenger::provider::desktop::linux::tests::native_fallback_delivers_when_no_helpers_installed`

Handoff: `problems.md` P5 item 3 (widen the skip arm to connection-level D-Bus
transport failures).

### 4. `sniff/macos-latest/L1` — **=** (2 / 2) — P6 (non-hermetic index + VM GPU)

- `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`
- `sniff::hardware::gpu::tests::test_detect_gpus_on_macos`

Handoff: `problems.md` P6 bullets 1 and 2.

### 5. `sniff/ubuntu-latest/L1` — **=** (1 / 1) — P6 (non-hermetic index)

- `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`

Handoff: `problems.md` P6 bullet 1 (`eager_path.is_some()` must be
authoritative — no `which()` fallthrough).

### 6. `sniff/windows-latest/L1` — **=** (1 / 1) — P6 (non-hermetic index)

- `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing`

Same handoff as cell 5.

### 7. `sniff/ubuntu-latest/lint` — **=** (10 / 10 diagnostics) — P6 (clippy debt)

No JUnit identities exist for a `lint` cell. Log-derived diagnostic set, taken
from job `94087827510` (main) and job `94295875023` (branch) — byte-identical:

| lint | location |
|---|---|
| `unused_imports` (`use super::*`) | `sniff/lib/src/services/launchd.rs:40:9` |
| `unused_variables` (`helpers`) | `sniff/lib/src/programs/notification_helpers.rs:169:13` |
| `permissions_set_readonly_false` | `sniff/lib/tests/merge_conflict_prediction.rs:480:5` |
| `items_after_test_module` | `sniff/lib/src/hardware/storage.rs:218:1` |
| `items_after_test_module` | `sniff/lib/src/programs/enums/metadata.rs:3324:1` |
| `zombie_processes` ×5 | `sniff/lib/src/process.rs:845:9`, `:863:26`, `:907:9`, `:937:26`, `:989:26` |

Handoff: `problems.md` P6 "sniff lint producer" bullet.

### 8. `sniff-cli/ubuntu-latest/lint` — **=** (1 / 1 diagnostic) — P6 (clippy debt)

Job `94087827818` (main) and `94295874915` (branch), identical:

| lint | location |
|---|---|
| `clippy::collapsible_if` | `sniff/cli/tests/snapshots.rs:672:17` |

From main commit `dd3f84608`. Handoff: `problems.md` P6 "sniff-cli lint
producer" bullet — a mechanical one-line fix.

### 9. `dmls/ubuntu-latest/L2` — **=** (1 / 1) — P6 (Neovim provisioning)

- `dmls::level2_editor_neovim::level2_neovim_decodes_semantic_token_families_and_positions`

Handoff: `problems.md` P6 dmls bullet (compat-shim
`vim.lsp.get_clients or vim.lsp.get_active_clients`).

### 10. `rendezvous-daemon/windows-latest/L1` — **=** (2 / 2) — P6 (SDDL alias)

- `rendezvous-daemon::local_transport::windows::tests::the_pipe_dacl_names_this_user_and_nobody_else`
- `rendezvous-daemon::private_dir::tests::the_current_user_descriptor_names_this_account_and_nobody_else`

Handoff: `problems.md` P6 rendezvous-daemon bullet (compare parsed ACE SIDs
instead of substring-matching SDDL text).

### 11. `unchained-ai/windows-latest/L1` — **=** (2 / 2) — P6 (ConPTY ordering)

- `unchained-ai::primitives::services::pty_runner::tests::test_ansi_stripping`
- `unchained-ai::primitives::services::pty_runner::tests::test_run_echo_command`

Handoff: `problems.md` P6 unchained-ai bullet. Note that bullet's own caveat —
"needs native Windows evidence" — so the handoff is a *diagnosis*, not yet a
prescribed fix.

---

## Accept / reject verdict

| # | cell | identical? | superset? | verdict |
|---|---|---|---|---|
| 1 | `claudine/wsl2-ubuntu/L1` | yes (3) | no | **ACCEPT** |
| 2 | `sniff/wsl2-ubuntu/L1` | yes (1) | no | **ACCEPT** |
| 3 | `messenger/wsl2-ubuntu/L1` | yes (1) | no | **ACCEPT** |
| 4 | `sniff/macos-latest/L1` | yes (2) | no | **ACCEPT** |
| 5 | `sniff/ubuntu-latest/L1` | yes (1) | no | **ACCEPT** |
| 6 | `sniff/windows-latest/L1` | yes (1) | no | **ACCEPT** |
| 7 | `sniff/ubuntu-latest/lint` | yes (10 diagnostics, log-derived) | no | **ACCEPT — caveated** |
| 8 | `sniff-cli/ubuntu-latest/lint` | yes (1 diagnostic, log-derived) | no | **ACCEPT — caveated** |
| 9 | `dmls/ubuntu-latest/L2` | yes (1) | no | **ACCEPT** |
| 10 | `rendezvous-daemon/windows-latest/L1` | yes (2) | no | **ACCEPT** |
| 11 | `unchained-ai/windows-latest/L1` | yes (2) | no | **ACCEPT** |

No candidate cell is rejected: all eleven produced complete identity evidence on
both sides and none is a branch superset.

### The caveat on the two `lint` cells (needs Ken's call)

A `lint` cell carries **zero test identities**. `ci-rollup compare` — the engine
behind `just ci-diff` — diffs `failed_tests`, which is empty for both lint
cells, so once an entry exists the automated gate can never notice a *new*
clippy error appearing in that package. Spec F8's acceptance criterion
("`just ci-diff` shows no new failing test identity hidden inside an accepted
cell") is structurally unsatisfiable here: it will report *nothing*, which reads
as clean but proves nothing.

The identity evidence itself is complete — the diagnostic sets above are
byte-identical between the two runs — so the ratified rule's letter is met. Two
things are worth weighing before applying entries 7 and 8:

- Option B is unusually cheap for these two. `sniff-cli` is one
  `clippy::collapsible_if`; `sniff` is ten diagnostics of which five want an
  `#[allow(clippy::zombie_processes)]` with a rationale on intentional leaked-child
  fixtures. Both are already enumerated as main-side follow-ups in the spec's
  out-of-scope list.
- If entries 7 and 8 are applied anyway, the diagnostic tables above are the
  only re-verification anchor at expiry, which is why they are spelled out in
  full inside each `reason` string.

### Source files re-checked at branch tip

None of the eleven cells' offending sources changed after run `31588186544`
(latest touch: `sniff/cli/tests/snapshots.rs` at `dd3f84608`, 2026-08-10;
`unchained-ai/.../pty_runner.rs` at `30fb0d603`, 2026-08-09). Every entry is
therefore still live and should reproduce on the Phase 5 verdict run.

---

## Branch-superset finding — where new red hides inside a baseline

No **proposed** cell is a superset. But the identity diff surfaces 26 new failing
identities on the branch, and **five of them land inside cells that are already
baselined today** — exactly the masking hazard F8 warns about, already active
before any new entry is written:

`claudine-cli/wsl2-ubuntu/L1` (baselined, `source_run = 30791562395`,
`expiry = 2026-11-30`) newly carries:

- `claudine-cli::bin/claudine::commands::wrap::harness_orch::prompt::tests::stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity` (P1)
- `claudine-cli::shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture` (P2)
- `claudine-cli::ctx_launch_anchor_baseline::cli_loop_reuses_launch_context_for_root_and_package_prompt_copies` (P4/P5.2)
- `claudine-cli::ctx_launch_anchor_baseline::cli_uses_launch_context_across_launch_source_matrix` (P4/P5.2)
- `claudine-cli::ctx_launch_anchor_baseline::inline_cli_uses_launch_context_across_launch_source_matrix` (P4/P5.2)

`claudine-cli/ubuntu-latest/L1` is likewise already baselined and its five new
identities — the same P1/P2 pair plus the three P4 latency timeouts
(`shipped_prompts::shipped_implement_prompt_runs_real_router_target`,
`wrap_perf::compose_perf_stdout_matches_non_perf`,
`wrap_perf::inline_compose_perf_stdout_matches_non_perf`) — are accepted
cell-wide by the existing entry. The branch's own F7 acceptance therefore cannot
be read off the merge verdict; it has to be read off `ci-diff` or the raw cell.

The remaining new identities are in cells with no baseline entry and block
normally: `claudine/windows-latest/L1` (3, P3a/P3b),
`claudine-cli/macos-latest/L1` (2, P1/P2), `claudine-cli/windows-latest/L1`
(11, P3a-P3d).

`ci-rollup compare` also reports 11 identities that stopped failing (2 on
`biscuit-speaks-cli/wsl2-ubuntu/L1`, 2 on `claudine/windows-latest/L1`, 6 on
`claudine-cli/wsl2-ubuntu/L1`, 1 on `darkmatter/windows-latest/L1`), consistent
with the base being older than the branch rather than its merge-base.

## One blocking cell F8 does not disposition — and does not need to

`biscuit-terminal/ubuntu-latest/lint` is `FAIL` on **both** runs with the
byte-identical diagnostic (`unused_mut` at
`biscuit-terminal/lib/tests/common/pty.rs:131:9`), has no baseline entry, and is
absent from F8's candidate list and from `linux.md` (which counted four blocking
Ubuntu cells; there were five).

It needs **no** entry: commit `d2681101a` ("chore(biscuit-terminal): silence
unused-mut warning in PTY test helper", 2026-08-13) landed after both runs and is
present on this branch and on `main`. The cell should go green on the Phase 5
verdict run. Recorded here so the gap is not rediscovered as a surprise block.

(`tabby/ubuntu-latest/lint` failed on main and already passes on the branch —
same shape, no action.)

---

## Verdict dry run

Draft applied to a scratch copy of the baseline and evaluated against the branch
run's rollup:

```
ci-rollup verdict --results <branch ci-results.json> \
                  --baseline <draft> --today 2026-08-13
```

- All 11 drafted entries report `baseline-accepted`. None reports
  `baseline-no-result` or `baseline-now-passing`, and none is expired.
- `tier = "lint"` parses as `Tier::Other("lint")` on both the baseline and the
  cell side, so the two lint entries match their cells correctly.
- No drafted entry duplicates an existing one (the current file has 14
  `[[failure]]` entries; none shares a `{package, environment, tier}` key with
  the draft).
- Residual blocks against run `31651014023`: `biscuit-terminal/ubuntu-latest/lint`
  (already fixed at tip, above), `claudine/windows-latest/L1`,
  `claudine-cli/macos-latest/L1`, `claudine-cli/windows-latest/L1` — all three
  claudine cells being exactly the P1-P4 work this branch owns.

This is the expected shape: with the draft applied, the merge gate's remaining
blocks are *only* the branch's own subject-area failures.

---

## Drafted TOML

Append after the `claudine-cli` / ubuntu / L1 entry, before the
"Approved skip budget" section.

```toml
# ---------------------------------------------------------------------------
# Main-drift cells scheduled by fix/ctx-launch-anchor (fixes/2026-08-13-finalize,
# F8 / Ratified Decision 2). Each entry's failing test identities were diffed
# between source run 31588186544 (main, 2026-08-12, commit e03bafee9 — the same
# run carries the wsl2-ubuntu leg) and branch run 31651014023 and are IDENTICAL.
# Short expiry on purpose: these are main-side handoffs, not accepted debt.
# ---------------------------------------------------------------------------

[[failure]]
package = "claudine"
environment = "wsl2-ubuntu"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P5 toolchain assumption: the archive-only WSL2 guest carries no rustc, so three tests that spawn the compiler at runtime take the command-not-found path. Identical 3-test identity set on run 31588186544, e.g. claudine::system_prompt::prepare::tests::non_repository_session_runs_shell_in_launch_cwd"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "sniff"
environment = "wsl2-ubuntu"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P5 timing budget: detect() exceeds its 20s cap in the WSL2 guest (29.5s here, 40.1s on main) because the guest PATH crosses /mnt/c drvfs. Identical 1-test identity set on run 31588186544: sniff::integration::test_detect_completes_in_reasonable_time"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "messenger"
environment = "wsl2-ubuntu"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P5 desktop-service assumption: the guest D-Bus socket refuses connection with Permission denied, a shape the test's environment-skip arm does not match. Identical 1-test identity set on run 31588186544: messenger::provider::desktop::linux::tests::native_fallback_delivers_when_no_helpers_installed"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "sniff"
environment = "macos-latest"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P6 non-hermetic ExecutableIndex plus a virtualized-runner GPU probe. Identical 2-test identity set on run 31588186544: sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing and sniff::hardware::gpu::tests::test_detect_gpus_on_macos"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "sniff"
environment = "ubuntu-latest"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P6 non-hermetic ExecutableIndex: find_with_source falls through the eager test map to a live which(), and the runner image preinstalls gradle. Identical 1-test identity set on run 31588186544: sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "sniff"
environment = "windows-latest"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P6 non-hermetic ExecutableIndex, same mechanism as the ubuntu and macOS legs. Identical 1-test identity set on run 31588186544: sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing"
source_run = "31588186544"
expiry = "2026-09-30"

# A lint cell emits no JUnit, so `ci-rollup compare` cannot police it. The exact
# clippy diagnostic set is recorded here instead; it is the only re-verification
# anchor this entry has at expiry.
[[failure]]
package = "sniff"
environment = "ubuntu-latest"
tier = "lint"
owner = "@yankeeinlondon"
reason = "P6 clippy debt visible only on a Linux lint host. Identical 10-diagnostic set on run 31588186544: unused_imports services/launchd.rs:40, unused_variables programs/notification_helpers.rs:169, permissions_set_readonly_false tests/merge_conflict_prediction.rs:480, items_after_test_module hardware/storage.rs:218 and programs/enums/metadata.rs:3324, and 5x zombie_processes in process.rs (845, 863, 907, 937, 989)"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "sniff-cli"
environment = "ubuntu-latest"
tier = "lint"
owner = "@yankeeinlondon"
reason = "P6 clippy debt from main commit dd3f84608. Identical single-diagnostic set on run 31588186544: clippy::collapsible_if at sniff/cli/tests/snapshots.rs:672"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "dmls"
environment = "ubuntu-latest"
tier = "L2"
owner = "@yankeeinlondon"
reason = "P6 provisioning gap: apt-get installs Neovim 0.9.5 while the probe calls vim.lsp.get_clients (Neovim >= 0.10). Identical 1-test identity set on run 31588186544: dmls::level2_editor_neovim::level2_neovim_decodes_semantic_token_families_and_positions"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "rendezvous-daemon"
environment = "windows-latest"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P6 SDDL alias brittleness: the descriptors are correct, but the assertions substring-match a literal SID while Windows renders the runner's built-in Administrator account as the LA alias. Identical 2-test identity set on run 31588186544: rendezvous-daemon::local_transport::windows::tests::the_pipe_dacl_names_this_user_and_nobody_else and rendezvous-daemon::private_dir::tests::the_current_user_descriptor_names_this_account_and_nobody_else"
source_run = "31588186544"
expiry = "2026-09-30"

[[failure]]
package = "unchained-ai"
environment = "windows-latest"
tier = "L1"
owner = "@yankeeinlondon"
reason = "P6 ConPTY shutdown ordering: the try_wait polling loop added by 30fb0d603 defers dropping the ConPTY master until exit is observed, which Windows never reports, so both tests hit the 5s deadline. Identical 2-test identity set on run 31588186544: unchained-ai::primitives::services::pty_runner::tests::test_ansi_stripping and unchained-ai::primitives::services::pty_runner::tests::test_run_echo_command"
source_run = "31588186544"
expiry = "2026-09-30"
```

## Remaining Phase 5 work (not done here)

1. Land P1-P4 and get a CI run in which every branch-owned identity is green.
   This is the entry gate; the draft must not be applied before it.
2. Re-run the identity diff against that run — the drafted `reason` strings pin
   identity sets from `31588186544`, and any entry whose set has moved must be
   re-justified rather than copied forward.
3. Apply the draft, run `ci-verdict` for real, and run `just ci-diff`.
4. Confirm `biscuit-terminal/ubuntu-latest/lint` is green (it should be, via
   `d2681101a`); if it is not, it needs its own disposition.
5. Decide the entries 7/8 caveat: baseline the two lint cells, or take the
   cheap Option B main-side clippy fix instead.
