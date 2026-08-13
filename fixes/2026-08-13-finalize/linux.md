# Ubuntu / Linux CI Failures — Run 31651014023

Investigation of the Ubuntu-runner failures in CI run [31651014023](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31651014023) (branch `fix/ctx-launch-anchor`, 2026-08-13). Eight failing cells: four the merge gate classified BLOCKING (sniff test, sniff lint, sniff-cli lint, dmls test-l2) and four baseline-covered (claudine-cli test, claudine-cli test-l2, biscuit-terminal-cli test-l2, darkmatter-cli test-l2). Drift claims below are grounded against the last completed `ci` run on main, [31588186544](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31588186544) (2026-08-12): every cell except `claudine-cli / test` fails there with the byte-identical test identities. The `claudine-cli / test` failures are this PR's own — they are the Linux face of the launch-context failures that block on macOS and Windows.

Blocking cells first.

## sniff: filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing

**Cell:** sniff / test (ubuntu-latest), job 94295875124 — **BLOCKING** (also fails on macOS and Windows).

**Error:**

```
thread '...resolve_acting_binary_returns_none_when_binary_missing' panicked at
sniff/lib/src/filesystem/repo/standard.rs:1921:9:
assertion failed: resolved.is_none()
```

**Analysis: main drift; the test was never hermetic.** The test (standard.rs:1912-1923) builds `ExecutableIndex::for_test(HashMap::new())` and expects `resolve_acting_binary_with_version(MonorepoStandard::GradleMultiProject, ...)` to return `None`. But `ExecutableIndex::find_with_source` (sniff/lib/src/executable_index.rs:208-216) treats the eager map as layer 1 only and falls through to a live `which(program)` when the map misses — so the "empty index" still resolves `gradle` from the real PATH. GitHub-hosted runners (all three OSes) preinstall Gradle; developer machines without it pass. The test arrived on main in `963ddb58b` (monorepo standard types) and the identical failure is present on main baseline run 31588186544. Not PR-caused. Fix hypothesis: make the test seam authoritative — have `for_test` (or a flag on it) suppress the `which` fallback, or point the test at a binary name that cannot exist on any host.

## sniff: lint producer (clippy, `-D warnings`)

**Cell:** sniff / lint, job 94295875023 — **BLOCKING**. Producer failed during `cargo clippy`, so no test identities were recorded.

**Errors:** 9 errors in `sniff` (lib test) plus 1 in the `merge_conflict_prediction` integration test:

- `unused import: super::*` — `sniff/lib/src/services/launchd.rs:40`
- `unused variable: helpers` — `sniff/lib/src/programs/notification_helpers.rs:169`
- `clippy::permissions_set_readonly_false` — `sniff/lib/tests/merge_conflict_prediction.rs:480` (`permissions.set_readonly(false)`; "on Unix platforms this results in the file being world writable")
- `clippy::items_after_test_module` — `sniff/lib/src/hardware/storage.rs:218` and `sniff/lib/src/programs/enums/metadata.rs:3324`
- `clippy::zombie_processes` (5×) — `sniff/lib/src/process.rs:845, 863, 907, 937, 989` ("spawned process is never `wait()`ed on")

**Analysis: main drift, mostly Linux-only visibility.** None of these files differ from the merge-base (`git diff d05bdbf36...HEAD` is empty for all of them), and the toolchain is pinned at 1.97.1 in `rust-toolchain.toml` since 2026-07-24 on both sides — so nothing in this PR changed the inputs. The same job is red on main baseline run 31588186544. Several of these only fire on a Linux lint host, which is why local (macOS) clippy never caught them: `launchd.rs`'s only test is `#[cfg(target_os = "macos")]`, leaving `use super::*;` dead elsewhere; `notification_helpers.rs:169`'s `helpers` is consumed only under `#[cfg(not(target_os = "linux"))]`; `storage.rs`'s post-test-module items are the `#[cfg(target_os = "linux")]` storage implementation, which simply doesn't exist in a macOS compilation. The `zombie_processes` hits are intentional leaked-child fixtures for the process-tree tests (added around `eb0dac5ea`, main, 2026-07-17) and likely want explicit `#[allow(clippy::zombie_processes)]` with a rationale rather than a `.wait()`. This is main's cleanup to do (or a small lint-fix commit on any branch), not a regression from this PR.

## sniff-cli: lint producer (clippy, `-D warnings`)

**Cell:** sniff-cli / lint, job 94295874915 — **BLOCKING**. Same shape: producer failed, no identities.

**Error:** one error fails the `snapshots` test target:

```
error: this `if` statement can be collapsed
  --> sniff/cli/tests/snapshots.rs:672:17
    = note: `-D clippy::collapsible-if` implied by `-D warnings`
```

Clippy suggests collapsing the nested `if let Some(binary) = entry.get_mut("binary") { if binary.get("path")... }` into a let-chain (`&&`).

**Analysis: main drift.** `sniff/cli/tests/snapshots.rs` is untouched by this PR; the offending code came from `dd3f84608` ("fix: improve cross-platform reliability", main, 2026-08-10), and the job is red on main baseline run 31588186544. The let-chain-aware `collapsible_if` is a 1.97-era lint, so this landed after the last time sniff-cli lint was green. Mechanical one-line fix on main.

## dmls: level2_editor_neovim::level2_neovim_decodes_semantic_token_families_and_positions

**Cell:** dmls / test-l2 (ubuntu-latest), job 94308943474 — **BLOCKING**.

**Error:**

```
panicked at darkmatter/dmls/tests/level2_editor_neovim.rs:171:28:
probe emitted no JSON
E5113: Error while calling lua chunk: .../fixtures/editor_neovim/probe.lua:39:
attempt to call field 'get_clients' (a nil value)
```

**Analysis: main drift — Ubuntu never had a new-enough Neovim.** The probe (`darkmatter/dmls/tests/fixtures/editor_neovim/probe.lua:38-44`) polls `vim.lsp.get_clients(...)`, an API introduced in Neovim 0.10. The workflow's provisioning step (`.github/workflows/_package-ci.yml:620`) does a plain `sudo apt-get install -y neovim` on Ubuntu, and this job's log confirms `NVIM v0.9.5` — ubuntu-latest's archive version, which only has the deprecated `vim.lsp.get_active_clients`. macOS provisions via Homebrew and gets a current Neovim, which is why only the Ubuntu leg dies. The identical failure is on main baseline run 31588186544; the test has plausibly never passed on an apt-provisioned runner since it landed in `4cf0ccd1d`. Fix hypothesis: either compat-shim the probe (`local get_clients = vim.lsp.get_clients or vim.lsp.get_active_clients`) or provision Neovim ≥ 0.10 on Ubuntu (PPA or release tarball) — the probe shim is the cheaper, self-contained fix.

## claudine-cli: shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture

**Cell:** claudine-cli / test (ubuntu-latest), job 94295876522 — baseline-covered on Ubuntu, but the same identity blocks on macOS/Windows. Reproduced locally on this worktree.

**Error:**

```
assertion `left == right` failed: a shipped prompt in the `implement` route changed.
  left:  {"prompts/_implement/implement-plan.md": "62d70fb16a02592c-652e691e678f8b5a", ...}
  right: {"prompts/_implement/implement-plan.md": "7f268c6d7aa0dd12-652e691e678f8b5a", ...}
```

Only the first (frontmatter) hash segment differs; the body segment matches the pin.

**Analysis: stale pin after the main merge — a two-sided edit race.** Timeline: branch commit `398222663` edited `prompts/_implement/implement-plan.md` (the +3-line `> **Plan:** @{{area}}/{{plan}}` body block) and refreshed `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json` in the same commit — consistent at that point. Then main-side commit `69a15f6c0` ("planning(claudine): start ctx-launch-anchor at phase 2", 2026-08-12 19:06 UTC) edited the same prompt's *frontmatter* (`phase:` gains a `frontmatter(plan, 'phase')` fallback) **without** refreshing any pin. Merge `e1dea6847` combined both edits; the pin still records the pre-merge frontmatter hash, so the frontmatter segment drifts (62d70fb1 computed vs 7f268c6d pinned) while the body segment agrees. Note main's own pin (`65a5bf30...`) is also stale against `69a15f6c0`, so main's tip will go red on its next completed run — the breaking edit is main-side, but this branch must still absorb it. Fix: per the test's own instructions, re-derive the L2 fixture copy from the merged bytes (the fixture copy at `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md` was last synced by `398222663` and needs the frontmatter edit too), then run `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli --test shipped_prompt_route_drift`.

## claudine-cli: commands::wrap::harness_orch::prompt::tests::stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity

**Cell:** claudine-cli / test (ubuntu-latest), job 94295876522 — baseline-covered on Ubuntu; same identity blocks on macOS/Windows. Reproduced locally.

**Error:**

```
panicked at claudine/cli/src/commands/wrap/harness_orch/prompt.rs:443:9:
assertion `left == right` failed
  left:  Some("unknown")
  right: Some("codex")
```

**Analysis: PR-caused — the seeded AGENT env never reaches the raw `agent` facet.** The test (added by branch commit `7c6773a8e`, "route wrap pipeline through launch-context seam") captures a launch context, then inserts `AGENT=codex` into `context.env_mut()` and into `CallerInputLayers.env_overrides`, and asserts `prepared_context.get("agent") == Some("codex")`. Two mechanisms conspire:

1. `ComposeContext::get()` returns raw `inner.values` (darkmatter `compose/context/runtime.rs:313-315`); env-derived AGENT/MODEL overrides are only projected by `values_with_env_agent_overrides()` (runtime.rs:368-376), i.e., via `get_effective`/`as_object`. The raw `values["agent"]` was fixed at `capture_launch_context` time from the real process env — AGENT unset on CI and locally, hence `"unknown"`.
2. In `harness_prepare_options` (prompt.rs:169-185), the *fresh-capture* arm copies `input_layers.env_overrides` into the context env, but the *extend* arm — the one this test exercises, since it pre-seeds `epoch_context` — calls `extend_launch_context` which early-returns when nothing is missing, and never re-projects identity facets.

So the "capture-once, extend-don't-reanchor" contract holds for topology (`area` asserts pass) but the agent identity facet is anchored to the process env at capture and is unreachable through the seeded overrides on this path. Deterministic on every OS (fails on this machine too), so this is the PR's own red test — either the harness should project caller env overrides into the epoch context on the extend arm (and/or the prepared context should expose effective values), or the test's expectation about `get()` vs effective lookup is wrong. This matches the known 08-12 regression note on the compose guard path for `ctx.*` capture-once.

## claudine-cli: shipped_prompts::shipped_implement_prompt_runs_real_router_target — TIMEOUT

**Cell:** claudine-cli / test (ubuntu-latest), job 94295876522 — baseline-covered on Ubuntu; overlaps the blocking macOS/Windows set.

**Error:**

```
TERMINATING [> 90.000s] claudine-cli::shipped_prompts shipped_implement_prompt_runs_real_router_target
TIMEOUT [ 90.011s]
```

No assertion output — the test simply never finished (nextest 90s termination).

**Analysis: PR-correlated latency/hang in real composition.** This test drives the real `implement` router target through the CLI. The job was green on main baseline run 31588186544, and the wrap pipeline was rerouted through the launch-context seam by branch commits `7c6773a8e`/`6c6fdf63e`/`8a11bd9c4` — every real-composition test that timed out in this job (this one plus the two `wrap_perf` parity tests below) is downstream of that seam. See the next section for the shared latency hypothesis.

## claudine-cli: wrap_perf::compose_perf_stdout_matches_non_perf and wrap_perf::inline_compose_perf_stdout_matches_non_perf — TIMEOUT

**Cell:** claudine-cli / test (ubuntu-latest), job 94295876522 — baseline-covered on Ubuntu; overlaps the blocking macOS/Windows set.

**Error:** both hit nextest's 90s termination with no output beyond `(test timed out)`.

**Analysis: PR-correlated — compose got slower (or intermittently wedges) under the launch-context seam.** Each parity test runs the real `claudine compose --goose` binary twice (with/without `--perf`) against a tempdir HOME with a stub `goose` (`claudine/cli/tests/wrap_perf.rs:288-316`). Notably, **both tests pass locally on this exact tree in ~23s combined** — so this is not a hard, deterministic hang. But the job was green on main, three real-composition tests all blew the same 90s limit on this run, and the same family fails on the macOS/Windows legs, which rules out a pure runner-load artifact. Working hypothesis: the new launch-context capture (repo/topology evidence gathering at the launch directory — here a non-repo tempdir) adds enough per-invocation cost that a loaded 2-core runner crosses 90s, echoing the previously-fixed "compose hang from non-repo CWD" shape. Worth profiling `InvocationContext::capture_at`/`capture_launch_context` from a non-repository CWD before assuming load. If the macOS/Windows logs show these as hard failures rather than timeouts, trust those diagnostics over this leg's.

## claudine-cli: level2_context_capture width-cap tests (4)

**Cell:** claudine-cli / test-l2 (ubuntu-latest), job 94314527710 — baseline-covered. Failing identities:

- `level2_context_default_at_140_fills_cap_in_tmux`
- `level2_context_default_caps_at_140_in_wide_tmux`
- `level2_context_default_narrow_preserves_type_and_wraps_in_tmux`
- `level2_context_default_preserves_columns_at_min_width_in_tmux`

**Error:** three fail at `level2_context_capture.rs:180` ("expected box-drawing glyphs from the real table renderer"), one at `:421` ("narrow report must keep the `Property` column"). The captured frame shows why — `claudine context` never printed a table:

```
$ env FORCE_COLOR=1 COLUMNS=140 .../target/debug/claudine context
  Welcome to Claudine!
  Let's get you set up. This will only take a moment.
  Text-to-Speech
```

**Analysis: main drift — the first-run init wizard intercepts `context` on an unseeded CI HOME.** The tests drive the binary in a real tmux shell using the runner's actual HOME (the test file sets no HOME/XDG isolation), and with no Claudine config present the CLI opens the onboarding wizard instead of rendering the context report. Main baseline run 31588186544 fails the same four identities with the same "Welcome to Claudine!" frames, so this is not the PR and not the old 140-cap fill-contract issue — it is an environment/test-isolation gap that predates this branch. Fix hypothesis: seed a minimal config (or set the wizard-suppressing env/HOME) in the L2 harness before invoking `claudine context`, mirroring what the L1 `wrap_perf` tests already do with `seed_minimal_config`.

## biscuit-terminal-cli: level2_diagrams::level2_diagram_fallback_when_no_image_protocol

**Cell:** biscuit-terminal-cli / test-l2 (ubuntu-latest), job 94312897730 — baseline-covered. (Note: the failing identity is the tmux no-image-protocol fallback test, not an Apple Terminal test — the Apple Terminal "doublem" variant exists only on the macOS leg.)

**Error:**

```
panicked at biscuit-terminal/cli/tests/level2_diagrams.rs:570:5:
expected fallback fenced code block in tmux (no image protocol). plain:
$ bt flowchart "A --> B"
bt: command not found
```

**Analysis: main drift — bare `bt` is not on the tmux session's PATH.** `send_bt_command` (`biscuit-terminal/cli/tests/common/mod.rs:29-33`) types the literal string `bt ...` into the shared tmux shell, so the test depends entirely on the session shell having the built binary on PATH — which the Ubuntu runner's login shell does not. Identical failure on main baseline run 31588186544. This is the known bare-command L2 hazard (same family as the stale-host-binary gotcha): the fix is to send the absolute path to the freshly built `bt` (or a shim that resolves it via `cargo_bin`/`current_exe`) rather than relying on shell PATH state.

## darkmatter-cli: level2_schema_about::level2_schema_about_light_terminal_uses_dark_code_theme

**Cell:** darkmatter-cli / test-l2 (ubuntu-latest), job 94298083827 — baseline-covered.

**Error:**

```
panicked at darkmatter/cli/tests/level2_schema_about.rs:242:5:
schema-about YAML examples should use the exact OneHalf Dark background RGB(40,44,52).
line:
" \u{1b}[48;2;250;250;250m \u{1b}[38;2;56;58;66m    ...bar...: ...string[]"
```

The run simulates a light terminal (`COLORFGBG='0;15' THEME='one-half' CODE_THEME='one-half' md schema about`) and asserts code blocks keep the dark code theme; the captured row instead carries the OneHalf **Light** background (`48;2;250;250;250`).

**Analysis: main drift — the light-terminal/dark-code-block contract does not hold on the Ubuntu tmux path.** The repo's rendering contract is that code blocks stay on the dark variant even in light terminals (the ThemePair carries both variants and the terminal is the color-mode source). On this leg the renderer resolved the code theme to the light variant, i.e., either `COLORFGBG` propagation through the tmux harness or the CLI's light/dark detection diverges on Linux. The identical identity fails on main baseline run 31588186544 (and on the macOS L2 leg of that run), so it predates this PR entirely. Diagnosis should start with what `md` detects as terminal background inside the CI tmux pane versus what the test believes `COLORFGBG='0;15'` forces.
