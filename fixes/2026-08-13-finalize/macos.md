# macOS CI Failures — Run 31651014023

Analysis of the macOS-side failures in CI run 31651014023 (branch `fix/ctx-launch-anchor`, merge commit 829567375, run date 2026-08-12/13; analyzed 2026-08-13). Five macOS jobs failed: two BLOCKING cells (claudine-cli L1, sniff L1) and three baseline-covered L2 cells (claudine-cli, darkmatter-cli, biscuit-terminal-cli). Both blocking claudine-cli failures and both sniff tests were run locally on this macOS host; the two claudine-cli failures reproduce exactly, and both sniff tests pass locally — which itself is diagnostic (see those sections). Blocking cells first.

## claudine-cli: shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture

**Cell:** claudine-cli / test (macos-latest), job 94295876445 — **BLOCKING**. Reproduced locally (fails identically).

**Error:**

```
assertion `left == right` failed: a shipped prompt in the `implement` route changed.
  left:  {"prompts/_implement/implement-plan.md": "62d70fb16a02592c-652e691e678f8b5a", ...}
  right: {"prompts/_implement/implement-plan.md": "7f268c6d7aa0dd12-652e691e678f8b5a", ...}
```

`left` is the hash of the shipped file as it exists now; `right` is the committed pin in `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json`. Only the frontmatter half of the Simple hash differs (`62d7…` vs `7f26…`); the body half (`652e691e678f8b5a`) matches, so this is a frontmatter-only drift. `prompts/implement.md` is untouched.

**Insights:**

- **Regression introduced by the merge from main, mechanically PR-side.** Branch commit 398222663 (`fix(prompts): implement-plan adds phase fallback and plan link`, Aug 12 20:30) edited the shipped `prompts/_implement/implement-plan.md`, re-derived the fixture, and refreshed the pin — correctly. Independently, main commit 69a15f6c0 (`planning(claudine): start ctx-launch-anchor at phase 2`, Aug 12 20:06) made an overlapping edit to the same shipped file: the same `phase:` fallback expression **plus** `{{iteration}}` → `{{phase}}` in the two `success.stack` `message:`/`warn:` lines. The merge `e1dea6847` (Merge branch 'main' into fix/ctx-launch-anchor) combined both, so the merged file carries the `{{iteration}}` → `{{phase}}` change the branch's pin refresh never saw. Net drift since the pin (verified with `git diff 398222663 HEAD -- prompts/_implement/implement-plan.md`) is exactly those two lines.
- The test is working as designed — it is an explicit review gate, not a correctness assertion. It cannot fail on main because main's pin (`65a5bf30…-56ca8ed9…`) matches main's copy of the file; only this branch holds the merged byte combination.
- **Fix:** review that the `{{iteration}}` → `{{phase}}` change needs no fixture-side mirror (it does not — those lines live inside the `success.stack` block that the fixture drops as part of the documented `shell:`-bearing removals), then refresh the pin:
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli --test shipped_prompt_route_drift`
  The companion structural test (`fixture_preserves_the_shipped_schema_and_loop_semantics`) passes, confirming the load-bearing keys are unaffected.

## claudine-cli: commands::wrap::harness_orch::prompt::tests::stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity

**Cell:** claudine-cli / test (macos-latest), job 94295876445 — **BLOCKING**. Reproduced locally.

**Error (CI):**

```
panicked at claudine/cli/src/commands/wrap/harness_orch/prompt.rs:443:9:
assertion `left == right` failed
  left: Some("unknown")
 right: Some("codex")
```

Line 443 asserts `first_context.get("agent") == Some("codex")` after injecting `AGENT=codex` into the launch epoch context via `context.env_mut()`.

**Insights — the failing value is the host environment leaking through, and the test is host-env-sensitive:**

- **Local reproduction produced a different wrong value than CI:** locally the assertion failed with `left: Some("claude")` — not `Some("unknown")`. That is the tell: `prepared_context.get("agent")` returns whatever `AGENT` was in the *process environment* when `InvocationContext::capture_at` snapshotted it (this session runs under a Claude agent, so `AGENT=claude`; CI has no `AGENT`, so darkmatter's `populate_agent` defaults to `"unknown"`). It never returns the injected override.
- **Proof:** running the test with `AGENT=codex` in the host environment makes it pass. So the assertion is satisfied only by ambient env, never by the test's own injection.
- **Mechanism.** Two facts combine:
  1. `darkmatter::…::ComposeContext::get()` reads the raw `values` map; only `as_object()` / `get_effective()` apply the AGENT/MODEL env-override projection (`values_with_env_agent_overrides`, main commits 0629742b8 + 117817cc4 — all ancestors of the test's authoring commit, so this is not new behavior).
  2. `capture_launch_context` bakes `values.agent` at capture time from the invocation's environment snapshot (`ContextCaptureEvidence::new(self.inner.environment.clone())` in `claudine/lib/src/invocation_context.rs`). The test's `env_mut().insert("AGENT", "codex")` happens *after* capture and only feeds the projection, never the raw values. The extension path (`extend_launch_context`) captures only *missing* groups, so the already-captured `agent` value is never re-anchored.
- **Classification: PR-introduced; never worked as authored.** The test was added by branch-only commit 7c6773a8e (`route wrap pipeline through launch-context seam`, Aug 12 20:30). The final merge from main brought zero changes to `prompt.rs`, `invocation_context.rs`, or darkmatter's compose-context code (`git diff 6c6fdf63e HEAD` on those paths is empty), so this is not merge drift. The test can only ever have passed in a session whose ambient env carried `AGENT=codex` — plausibly the authoring agent session was a codex-wrapped run, which would have masked the leak at authoring time.
- Internal inconsistency in the test corroborates this: the *second* prepare's agent/model assertions (lines 457–458) go through `second_context.as_object()` (the effective view) and are correct; only the first prepare's line 443 uses raw `.get()`.
- **Fix hypothesis, two options — a contract decision:**
  - *Test fix (minimal):* assert via the effective view (`as_object()` / `get_effective`) at line 443, matching lines 457–458. This accepts that raw `values.agent` carries the launch-time ambient value and the override lives in the projection.
  - *Behavior fix (if the PR's anchoring contract requires it):* since this PR is about anchoring prepared `ctx.*` to launch evidence, one could argue `get("agent")` returning the ambient host agent instead of the epoch's declared identity is exactly the bug the test name describes. Then either `env_mut()` (or the epoch-context construction in `harness_prepare_options`) should re-run agent/model population into `values`, or capture should receive the layered env (input-layer `env_overrides` folded into the evidence environment *before* `capture_launch_context`). Note `harness_prepare_options`'s own fallback branch has the same after-capture insertion pattern, so a behavior fix should cover both the test seam and the production seam.
- Secondary hazard worth fixing regardless: the test's outcome varies with the host's `AGENT` env var (fails one way on CI, another way under an agent session, passes under codex). Whatever fix is chosen should neutralize ambient `AGENT`/`MODEL` for this test.

## sniff: filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing

**Cell:** sniff / test (macos-latest), job 94295875009 — **BLOCKING** (also failing on ubuntu-latest and windows-latest in this run).

**Error:**

```
panicked at sniff/lib/src/filesystem/repo/standard.rs:1921:9:
assertion failed: resolved.is_none()
```

**Insights:**

- **Passes locally, fails on all three CI OSes — because the test is not hermetic and the outcome depends on whether `gradle` is on `PATH`.** The test builds `ExecutableIndex::for_test(HashMap::new())` (an empty synthetic eager PATH index) and expects `GradleMultiProject` resolution to return `None`. But `ExecutableIndex::find_with_source` (sniff/lib/src/executable_index.rs:208) falls through an eager-index *miss* to a live `which(program)` lookup. GitHub-hosted runner images ship `gradle` preinstalled on all three OSes, so `which("gradle")` succeeds there and resolution returns `Some(… BinarySource::Path)`. This host has no `gradle` (`which gradle` → not found), so it passes here.
- **Classification: main drift / environment exposure, not PR-caused.** `sniff/` is byte-identical to `origin/main` on this branch (empty `git diff origin/main -- sniff/`). The test was introduced by 963ddb58b (June 15, on main); the `which` fallthrough predates it (bd847dca2 / 19dcd7a1b). Main's recent green runs did not include sniff jobs at all (path-scoped CI), so this has likely been latent since the test landed and only surfaces when a branch actually schedules the sniff cell on GitHub runners.
- **Fix hypothesis:** make a present eager index authoritative — when `eager_path.is_some()`, do not fall through to `which()` (in both `find_with_source` and `find`), or at minimum guarantee that `for_test` indices are hermetic. The sibling test `resolve_acting_binary_falls_back_to_path_when_wrapper_missing` relies on the eager map *hit*, not the `which` fallthrough, so it survives that change.

## sniff: hardware::gpu::tests::test_detect_gpus_on_macos

**Cell:** sniff / test (macos-latest), job 94295875009 — **BLOCKING** (macOS-only test).

**Error:**

```
panicked at sniff/lib/src/hardware/gpu.rs:378:9:
Expected at least one GPU on macOS
```

**Insights:**

- **Passes locally (real Apple-silicon Mac), fails on the CI runner — a virtualization gap, not a code regression.** `detect_gpus()` on macOS queries IOKit for `IOAccelerator` service entries (6b5dbc00c, March, on main: "replace ioreg subprocess with direct IOKit FFI"). GitHub's macos-latest runners are Apple-Virtualization VMs whose paravirtual display device does not register an `IOAccelerator` service, so the iterator comes back empty and `detect_gpus()` returns `[]`. Note the previous implementation shelled out to `ioreg -rd1 -c IOAccelerator` — the *same* registry class — so this test has most likely never been able to pass on a virtualized macOS runner in either implementation; the path-scoped CI simply rarely runs the sniff cell.
- **Classification: main drift / runner-environment exposure, not PR-caused** (sniff is unchanged vs main on this branch).
- **Fix hypothesis:** either (a) make the test tolerate virtualized hosts — skip (or downgrade to "returns a vec") when no `IOAccelerator` service exists / when running in a VM (sniff already has virtualization detection in its host observations), or (b) give `detect_gpus` a fallback source that works in VMs, e.g. `system_profiler SPDisplaysDataType`, which does report the "Apple Paravirtual device" on runners. (a) is the surgical choice; (b) changes production behavior and cost (subprocess spawn) for a test's benefit.

## claudine-cli (L2): level2_context_capture width-cap tests

**Cell:** claudine-cli / test-l2 (macos-latest), job 94314527718 — **baseline-covered (did not block)**. Not run locally (L2; focus-risk not worth it — CI evidence is unambiguous).

Exact failing tests (all three `default`-report rows that ran; each failed identically after a ~17.5s wait timeout):

- `level2_context_default_at_140_fills_cap_in_tmux`
- `level2_context_default_caps_at_140_in_wide_tmux`
- `level2_context_default_narrow_preserves_type_and_wraps_in_tmux`

**Error:** `expected box-drawing glyphs from the real table renderer. plain: …` — and the captured pane text shows what actually happened: instead of the context table, the pane contains the **first-run onboarding wizard**:

```
  Welcome to Claudine!
  Let's get you set up. This will only take a moment.
  …
?   Press Enter to continue (Y/n)
```

**Insights:**

- **This is NOT the known 140-cap fill / 1-char right-margin contract issue.** The renderer never ran. `claudine context` requires config (`ensure_config_exists` in `claudine/cli/src/main.rs` runs for every command except Handle/Completions/Complete); the runner's HOME has no Claudine config; stdin inside tmux **is** a TTY, so `run_initialization` took the interactive path and blocked at "Press Enter to continue" until the capture wait timed out. The L2 test file does not isolate or seed HOME/config.
- The wizard gating is long-standing main code (ccb3490d2 / 21b553e4c, April, on main) and this branch does not touch it — consistent with the merge gate finding these failures baseline-covered (present on main's baseline too).
- **Fix hypothesis:** have the L2 context-capture harness seed a config before spawning (e.g. run one non-TTY `claudine` invocation first — headless init writes the config — or point the config path at a pre-created file), so first-run onboarding can never interpose. That would also remove an ordering/race sensitivity: any L2 row that reaches the runner before something has materialized a config will hit the wizard.

## darkmatter-cli (L2): level2_code_block_styling::level2_code_block_clears_inherited_dim_before_theme_colors

**Cell:** darkmatter-cli / test-l2 (macos-latest), job 94298083788 — **baseline-covered (did not block)**. (The ubuntu-latest test-l2 job also failed in this run, consistent with a non-macOS-specific cause.)

**Error:**

```
panicked at darkmatter/cli/tests/level2_code_block_styling.rs:248:5:
code block must clear inherited dim before applying the page-inverted background, got luma 44.
```

The test pre-emits `\x1b[2m` (dim), renders a Rust code block with `COLORFGBG=15;0` (dark terminal) in tmux, and requires the code panel's background luma > 175 (the light page-inverted panel). It measured luma 44 — a dark background, i.e. the page-inversion for code blocks did not take effect in the captured frame (the plain capture shows the code block itself rendered fine).

**Insights:**

- This branch's darkmatter diff vs main is only the additive compose-context helpers (`extend_with_evidence`, requirement helpers in `context/runtime.rs` + `capture/groups.rs`) — nothing in rendering, theming, or terminal color-mode detection. Combined with the baseline-covered classification, this is **main-side, not PR-caused**.
- Luma 44 is characteristic of the dark-theme code background being applied *without* inversion, which points at terminal color-mode resolution inside the CI tmux (OSC-11 unanswered; `COLORFGBG` honored or not) or the code-block contrast/inversion feature not engaging on the runner — the same territory as the known code-block color-mode and ThemePair work. A CI-tmux-specific detection difference is the most likely trigger; needs a main-side look rather than anything in this PR.

## biscuit-terminal-cli (L2): level2_apple_terminal_prose::level2_apple_terminal_double_underline_plain_text_visible

**Cell:** biscuit-terminal-cli / test-l2 (macos-latest), job 94312897737 — failed in CI but was **not in the blocking list**; classification: baseline-covered / environmental (biscuit-terminal has **zero diff vs main** on this branch, so it cannot be PR-caused).

**Error:**

```
panicked at biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:246:10:
attach/spawn Apple Terminal: Custom { kind: TimedOut, error: "command timed out after 10s" }
```

**Insights:**

- The failure is in harness setup, not the assertion: `AppleTerminalHarness::shared_or_else(… spawn_shell …)` timed out spawning/attaching Apple Terminal (50s total including retries). `available()` passed its gate, but actually scripting Terminal.app on a GitHub macOS runner is at the mercy of the VM's GUI session and Automation/TCC permissions — a classic headless-CI Apple Terminal hazard.
- This is an infrastructure/backend-availability failure on the runner. If the backend genuinely cannot spawn there, the `available()` check for the Apple Terminal backend should be tightened so the row skips clean on such hosts (per the skip-clean discipline) instead of timing out and cancelling the rest of the file (75 tests were skipped after the cancel).
