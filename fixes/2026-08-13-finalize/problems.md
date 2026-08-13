# Problems — fix/ctx-launch-anchor finalize (CI run 31651014023)

Consolidated problem structure distilled from the four per-OS investigation
files (windows.md, linux.md, macos.md, wsl.md), integrated one file at a time.
Drift claims are grounded against main's last completed run 31588186544
(2026-08-12): "reproduces on main" below means byte-identical test identities
on that run.

Problems are grouped by root cause, not by cell, because most failing cells
share a small number of causes. P1–P4 are this branch's responsibility; P5–P7
are main drift or runner-environment exposure that this branch merely
scheduled.

---

## P1 — PR defect: prepared context never sees injected epoch env (`ctx.agent` leaks ambient host value)

**Status: Critical. PR-introduced (branch commit 7c6773a8e); never worked as
authored. Deterministic on every OS — blocking on macOS/Windows claudine-cli,
red under baseline on ubuntu and wsl2. Reproduces locally on macOS.**

`harness_orch::prompt::tests::stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity`
fails at `claudine/cli/src/commands/wrap/harness_orch/prompt.rs:443`. The test
injects `AGENT=codex` via `context.env_mut()` and
`CallerInputLayers::env_overrides`, yet `prepared_context.get("agent")`
returns whatever `AGENT` was in the *process environment* at capture:
`"unknown"` on CI (no `AGENT`), `"claude"` locally under an agent session, and
it passes only when the host env carries `AGENT=codex` (proved locally). Two
mechanisms conspire:

1. `ComposeContext::get()` returns raw `inner.values` (darkmatter
   `compose/context/runtime.rs:313-315`); the AGENT/MODEL env-override
   projection (`values_with_env_agent_overrides`, runtime.rs:368-376) is
   applied only by `as_object()`/`get_effective()`. Raw `values["agent"]` was
   baked at `capture_launch_context` time from the real process env.
2. In `harness_prepare_options` (prompt.rs:169-185), the *fresh-capture* arm
   copies `input_layers.env_overrides` into the context env, but the *extend*
   arm — the one this test exercises, since it pre-seeds `epoch_context` —
   calls `extend_launch_context`, which captures only *missing* groups and
   never re-projects identity facets.

So capture-once holds for topology (the `area` asserts pass) but the agent
identity facet is anchored to ambient env and unreachable through seeded
overrides on the extend path. The test is internally inconsistent: the second
prepare's assertions correctly use `as_object()` (effective view); only line
443 uses raw `.get()`. This matches the known 08-12 regression note on the
compose guard path for `ctx.*` capture-once.

- Fix is a contract decision:
  - *Test fix (minimal):* assert line 443 via the effective view, accepting
    that raw `values.agent` carries launch-time ambient and overrides live in
    the projection.
  - *Behavior fix (matches the PR's stated anchoring intent):* fold
    caller/epoch env overrides into the evidence environment *before*
    `capture_launch_context` (or re-run agent/model population into `values`
    on the extend arm), covering both the test seam and
    `harness_prepare_options`'s production fallback branch, which has the same
    after-capture insertion pattern.
- Either way, the test must neutralize ambient `AGENT`/`MODEL` so its outcome
  stops varying with the host session.

## P2 — PR defect (merge artifact): shipped-prompt hash pin stale after the main merge

**Status: PR-side to absorb, though the breaking edit is main-side.
Cross-platform (macOS, Windows blocking; ubuntu, wsl2 baseline-covered).
Verified locally: `md hash` on the shipped file yields CI's computed hash.**

`shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`:
a two-sided edit race on `prompts/_implement/implement-plan.md`. Branch commit
398222663 edited the prompt body (+3-line `> **Plan:** …` block) and refreshed
the pin in the same commit — consistent at that point. Main commit 69a15f6c0
independently edited the same prompt's *frontmatter* (`phase:` gains a
`frontmatter(plan, 'phase')` fallback, `{{iteration}}` → `{{phase}}`) without
refreshing any pin. Merge e1dea6847 combined both, so the frontmatter hash
segment drifts (`62d70fb1…` computed vs `7f268c6d…` pinned) while the body
segment matches. The test is an explicit review gate working as designed.
Note: **main's own pin is also stale against 69a15f6c0** — main's tip will go
red on its next completed run regardless of this branch.

- Fix: re-derive the fixture copy
  (`claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`,
  last synced by 398222663 — it needs the merged frontmatter edit too), review
  the delta per the test's module docs (the `{{iteration}}` → `{{phase}}`
  lines live in the `success.stack` block the fixture drops, so no structural
  change; the companion structural test passes), then refresh with
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli --test shipped_prompt_route_drift`.

## P3 — Windows path handling in the PR's subject area (claudine lib + CLI blocking cells)

**Status: PR-exposed. The 12 remaining Windows blocking tests (3 claudine lib
+ 9 claudine-cli after removing the cross-OS P1/P2 pair) decompose into four
mechanisms — two product bugs, one test-harness bug, one expectation
mismatch. All tests are new in this PR (3773a92b3 added them; 812df339a opted
them into native Windows), so nothing here is a regression of
previously-green Windows behavior — but P3a/P3b are real product defects on
Windows.**

### P3a — Escape-eaten `\.` in textual `ctx.*` interpolation (product bug, darkmatter compose)

When a `ctx.*` value containing a Windows path is interpolated into markdown
text *before* the CommonMark parse, `\.` (backslash + punctuation) is consumed
as a CommonMark escape: `…\Temp\.tmpXYZ\…` composes as `…\Temp.tmpXYZ\…`.
Backslashes before letters survive (not valid escapes); Unix paths have no
backslashes, so the bug is invisible off-Windows. On Windows a composed prompt
shipped to an agent would carry a corrupted path.

- Explains 5 tests: claudine lib `appendix_relocation…` +
  `primary_prompt_relocation…`, claudine-cli `sequence_ctx_launch_anchor` ×2,
  and `proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs` — the
  last is a pure unit test with no CLI spawn, pinning the bug to darkmatter
  compose interpolation itself. In the sequence tests the anchoring invariant
  under test actually *held* (root identical and stable across tasks); only
  the rendering of the interpolated path is corrupt.
- Fix direction: escape backslashes (or all CommonMark-escapable punctuation)
  when interpolating values textually, or interpolate into the parsed tree
  instead of the raw source. One fix covers all five tests.

### P3b — `\\?\` verbatim prefix leaking into projected context paths (product bug, claudine)

`std::fs::canonicalize` on Windows returns `\\?\C:\…` verbatim paths.
`invocation_context.rs:1317-1319` documents that a verbatim key "must never
reach a projection or a comparison against an authored path", but
`canonical_or_self` (`system_prompt/context.rs:86-89`) projects canonicalized
`cwd`/`repo_root`/`package_*` directly, so discovered files inherit the
prefix.

- Explains: claudine lib `normal_session_composes_the_shipped_root_system_prompt_from_launch_context`
  (`prepared.source` = `\\?\D:\a\…`). Secondary hazard: several
  `ctx_launch_anchor_baseline` tests canonicalize their fixtures and pin
  verbatim `expected_launch_repo`, so they may trip this next once P3c is
  fixed.
- Fix direction: strip the verbatim prefix when projecting (dunce-style
  simplification); keep canonicalization confined to cache keys as
  `canonical_key` already does.

### P3c — Windows `.cmd` test stubs are broken (PR test-harness bug)

Two distinct stub defects in the new Windows provider/printf stubs:

1. `printf.cmd` (`stage_printf` in `ctx_launch_anchor_baseline.rs`) uses
   `<nul set /p` which hits EOF → ERRORLEVEL=1, with no trailing
   `exit /b 0` — so every `$(printf …)` frontmatter expansion "fails" despite
   correct stdout. Explains 3 tests (`cli_loop_reuses…`,
   `cli_uses_launch_context_across…`, `inline_cli_uses…`).
2. `.cmd` provider stubs cannot receive multi-line composed prompts as argv:
   Rust's std BatBadBut hardening (CVE-2024-24576) rejects batch-file
   arguments containing quotes/newlines ("batch file arguments are invalid").
   Explains 2 tests (`cli_keeps_eager_schema_files…`,
   `cli_keeps_launch_repository_facts_absent…`) — in the latter the
   launch-context behavior under test visibly held before the spawn died.
- Fix direction: `exit /b 0` (or ERRORLEVEL-clearing form) for printf; replace
  `.cmd` stubs with real executables — the established repo pattern is
  re-exec'ing the test binary via `current_exe` (as L2 probes do) — or route
  prompts via stdin/temp file instead of argv.

### P3d — Shell-directive quoting mismatch in jit preflight (test-vs-product contract)

`sequence::jit::tests::template_preflight_combines_launch_facts_with_the_selected_target`:
the approved command quotes the interpolated path
(`echo "C:\…\launch" claude …`) because it contains shell-special `\`; the
test builds its expectation as the raw unquoted string. Not the escape-eating
bug — `\.` survives here. Fix direction: derive the expectation through the
same quoting rule the product applies (or make the approval matcher normalize
quoting); confirm on native Windows which layer inserts the quotes before
choosing test fix vs product fix.

## P4 — PR-correlated: real-composition tests time out on CI (router target + wrap_perf parity)

**Status: PR-correlated latency, not a deterministic hang. The
claudine-cli/ubuntu-latest cell was GREEN on main's baseline run — these three
timeouts are new with this branch. Baseline-covered on ubuntu but the same
family straddles the wsl2 timeout cluster.**

`shipped_prompts::shipped_implement_prompt_runs_real_router_target` and both
`wrap_perf` stdout-parity tests hit nextest's 90s termination with no output.
Both parity tests **pass locally on this exact tree in ~23s combined**, so
this is not a hard hang — but three real-composition tests blowing the same
limit on a job that was green on main, with the wrap pipeline rerouted
through the launch-context seam (7c6773a8e, 6c6fdf63e), points at
per-invocation launch-context capture cost (repo/topology evidence gathering
from a non-repo tempdir HOME) crossing the budget on loaded 2-core runners —
echoing the previously-fixed "compose hang from non-repo CWD" shape.

- Fix direction: profile `InvocationContext::capture_at` /
  `capture_launch_context` from a non-repository CWD before assuming runner
  load; ensure the seam does not re-gather evidence per invocation where the
  epoch already carries it. The WSL2 pass independently suggested splitting
  the new multi-case matrix tests so each case fits a budget — same lever.

## P5 — WSL2 archive-only guest: environment assumptions, all main drift

**Status: main drift, none PR-caused. Five tests block only because their
package/environment cells have no `ci-baseline.toml` entry; all five
reproduce byte-identically on main's WSL2 leg.**

The WSL2 guest runs nextest archives only — no rustup, no toolchain, no
sibling-package binaries, no usable desktop D-Bus, and heavy process-spawn/IO
latency. Every WSL2 failure in this run reduces to one of four assumptions:

1. **A Rust toolchain on PATH** (`rustc`/`cargo` invoked at runtime):
   - BLOCKING: claudine `shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic`,
     `direct_composition_runs_shell_in_configured_working_directory`,
     `non_repository_session_runs_shell_in_launch_cwd`.
   - Baseline-covered: darkmatter ×6 (`$(rustc)` §2-ladder parse fixtures,
     rustc-stderr diagnostics), claudine-cli `contextual_errors`, model_id
     `ui::ui` (trybuild needs cargo), sniff-cli `repo_aggregate_json_snapshot`
     (cargo binary resolves to null).
   - Fix direction: gate on toolchain discoverability or swap fixtures to a
     guaranteed-present command (`echo` / PATH-injected stub); the cwd-probe
     tests already honor `$RUSTC`, so skip when neither `$RUSTC` nor `rustc`
     resolves, or pre-build the probe into the archive.
2. **Native-host timing budgets**:
   - BLOCKING: sniff `test_detect_completes_in_reasonable_time` (29.5s vs 20s
     cap here; 40.1s on main — WSL PATH scanning crosses /mnt/c drvfs).
   - Baseline-covered: the claudine-cli 90s-timeout family (~13 TIMEOUTs incl.
     the PR's three multi-case matrix tests, which fail with the OLD signature
     — siblings in the same new files pass), parallel-overlap and
     exit-code-timing tests, darkmatter `every_catalog_variable_survives_ambient_options`.
   - Fix direction: environment-gate budgets (e.g. `BISCUIT_CI_ENVIRONMENT`),
     exclude `/mnt/*` from sniff's program scan, split matrix tests per case.
3. **Desktop D-Bus**:
   - BLOCKING: messenger `native_fallback_delivers_when_no_helpers_installed` —
     bus connection fails with `Permission denied (os error 13)`, a shape the
     test's environment-skip arm (which matches only "Notifications"/"service")
     doesn't recognize. Fix direction: widen the skip arm to accept
     connection-level D-Bus transport failures.
4. **`md` sibling binary in the archive** (claudine-cli specific):
   - Baseline-covered: `inline_compose_writes_hash_that_passes_md_diff`.

Because the blocking five reproduce identically on main, the choice is:
fix the tests' environment assumptions on main, or add baseline entries for
claudine/wsl2-ubuntu, sniff/wsl2-ubuntu, and messenger/wsl2-ubuntu until the
guest carries a toolchain.

## P6 — Main-drift blocking cells in unrelated packages (non-hermetic tests, lints, provisioning)

**Status: main drift. Each package is byte-identical to main on this branch
(or the offending code came from main); they block only because path-scoped
CI rarely schedules these cells and they were never baselined. All verified
red-on-main except where noted.**

- `sniff::…::resolve_acting_binary_returns_none_when_binary_missing` — fails
  on **all three** GitHub OSes, passes locally. Not hermetic:
  `ExecutableIndex::find_with_source` (executable_index.rs:208-216) treats the
  eager test map as layer 1 only and falls through to a live `which(program)`
  — contradicting `for_test`'s own doc — and runner images preinstall
  `gradle`. Latent since June (963ddb58b). Fix direction: when
  `eager_path.is_some()`, the eager map is authoritative — no `which()`
  fallthrough (the sibling fallback test survives this).
- `sniff::hardware::gpu::tests::test_detect_gpus_on_macos` — passes on real
  Apple silicon, fails on Apple-Virtualization CI VMs whose paravirtual
  display registers no `IOAccelerator` service; likely never passable on
  virtualized runners (the pre-FFI `ioreg` implementation queried the same
  class). Fix direction: skip/downgrade when no IOAccelerator service exists
  or a VM is detected (sniff already detects virtualization).
- **sniff lint producer** — 10 clippy errors, none in files this PR touches,
  most only visible on a Linux lint host due to cfg gating (macOS-local clippy
  can't see them): `unused_imports` (launchd.rs:40, its only test is
  macOS-gated), `unused_variables` (notification_helpers.rs:169),
  `permissions_set_readonly_false` (merge_conflict_prediction.rs:480), 2×
  `items_after_test_module` (storage.rs:218, metadata.rs:3324), 5×
  `zombie_processes` (process.rs — intentional leaked-child fixtures wanting
  explicit `#[allow]` with rationale).
- **sniff-cli lint producer** — one `clippy::collapsible_if` at
  `sniff/cli/tests/snapshots.rs:672`, from main commit dd3f84608; the
  let-chain-aware lint is 1.97-era, landing after sniff-cli lint was last
  green. Mechanical one-line fix.
- **dmls `level2_neovim_decodes_semantic_token_families_and_positions`** —
  probe calls `vim.lsp.get_clients` (Neovim ≥ 0.10); Ubuntu provisioning does
  plain `apt-get install neovim` → 0.9.5 (confirmed in log). macOS gets a
  current Neovim via Homebrew, which is why only Ubuntu dies. Plausibly never
  passed on an apt-provisioned runner. Fix direction: compat-shim the probe
  (`vim.lsp.get_clients or vim.lsp.get_active_clients`) — cheaper and
  self-contained vs provisioning Neovim ≥ 0.10.
- **rendezvous-daemon Windows DACL tests ×2** — the descriptors are
  **correct**; the asserts substring-match the literal SID against SDDL text,
  but Windows renders well-known SIDs as aliases: the runner account
  (built-in Administrator, RID 500) renders as `LA` →
  `O:LAD:P(A;;GA;;;LA)`. Passes for any normal dev account, fails on
  GH-hosted runners. Fix direction: compare parsed ACE SIDs (round-trip
  through `ConvertStringSidToSid`) instead of substring-matching SDDL.
- **unchained-ai Windows pty_runner tests ×2** — both time out at the 5s
  deadline of the `child.try_wait()` polling loop introduced by 30fb0d603
  (PR #55 lineage), which defers dropping the ConPTY master until exit is
  observed; on Windows the exit is evidently never observable until the
  master/pseudo-console closes — a deadlock by ordering. PR #55's own Windows
  CI run was cancelled at merge, so the loop was never Windows-validated.
  Fix direction: on Windows keep the reader-EOF-driven wait or close the
  master before polling; needs native Windows evidence.

## P7 — Baseline-covered cells (context only, none PR-caused)

- **claudine-cli L2 `level2_context_*` (macOS ×3, ubuntu ×4):** NOT the
  140-cap margin contract — the pane never rendered a table. `claudine
  context` hit the **first-run onboarding wizard** (runner HOME unseeded; tmux
  stdin is a TTY, so `ensure_config_exists` → interactive init blocked at
  "Press Enter to continue" until capture timeout). Identical frames on
  main's baseline run. Fix direction: seed a minimal config in the L2 harness
  before spawning (mirroring the L1 `wrap_perf` tests' `seed_minimal_config`).
- **darkmatter-cli L2 (macOS `level2_code_block_clears_inherited_dim…` luma 44;
  ubuntu `level2_schema_about_light_terminal_uses_dark_code_theme` got OneHalf
  Light 250,250,250):** two faces of the same contract — code blocks must keep
  the dark variant regardless of terminal color mode — not holding on CI tmux.
  Branch's darkmatter diff is compose-context-only → main-side color-mode
  detection (`COLORFGBG`/OSC-11 propagation through the harness) issue.
- **biscuit-terminal-cli L2 (macOS `level2_apple_terminal_double_underline…`):**
  harness setup failure — Apple Terminal spawn/attach timed out on the
  headless runner (Automation/TCC hazard); `available()` passed but should
  skip clean; the timeout also cancelled 75 sibling tests.
- **biscuit-terminal-cli L2 (ubuntu `level2_diagram_fallback_when_no_image_protocol`
  — the actual ubuntu identity; the "doublem" test is macOS-only):**
  `bt: command not found` — `send_bt_command` types bare `bt` into the tmux
  shell, depending on login-shell PATH. The known bare-command L2 hazard; fix
  is sending the absolute path to the freshly built binary.
- **darkmatter-cli Windows `schema_validate_legacy_pretty_output_is_byte_identical`:**
  8.3 short-name mismatch — CLI emits the `file://` URL from the `%TEMP%`
  short path (`RUNNER~1`) while the test rebuilds it from `fs::canonicalize`
  (`runneradmin`). The test's comment claims the CLI canonicalizes; on Windows
  it doesn't — comment/expectation drift. Fix direction: canonicalize the
  emitted URL in the CLI or normalize both sides as the passing JSON variant
  does.
- **biscuit-tui-cli Windows `bash_script_passes_syntax_check`:** availability
  guard accepts the System32 WSL bash shim (spawn succeeds ≠ usable bash);
  the shim fails with UTF-16LE stderr that renders empty. Fix direction:
  require `bash --version` exit 0 + plausible stdout, prefer Git Bash on
  Windows, or gate to non-Windows.
- **sniff-cli Windows worktree-verbose ×2:** worktree paths surface in `\\?\`
  verbatim form, then rendering mangles them further (`\?\C:\…` display,
  `file://D:\?\C:\…` links) — same verbatim-leak + backslash-hostile-markdown
  mechanisms as P3a/P3b, living in sniff's worktree listing on main. Fix
  direction: dunce-style simplification before display + proper path-to-URL
  conversion.
- **sniff-cli Windows `repo_aggregate_json_snapshot`:** `[BASE]` redaction is
  a string replacement that misses because the test's base spelling
  (short-form/verbatim) diverges from sniff's normalized long-form output,
  plus `/` vs `\` separators. Fix direction: canonicalize/simplify the base
  the same way sniff does before building the redaction; make it
  separator-insensitive.
- **WSL2 baseline-covered families** — see P5 (toolchain, timing, `md`
  sibling binary).

---

## Cross-cutting observations

- The merge gate's "blocking" set mixes two very different things: genuine
  PR-subject work (P1–P4) and never-baselined environmental/nonhermetic cells
  (P5, P6). Only P1–P4 require changes on this branch; every P5/P6 blocking
  identity was verified red-on-main.
- **Windows path hygiene is a monorepo-wide theme, not a claudine quirk:**
  verbatim `\\?\` leakage, backslash-hostile markdown/URL rendering, and 8.3
  short-name divergence each appear in at least two packages (claudine,
  sniff-cli, darkmatter-cli). P3a/P3b fix claudine's instances; the sniff-cli
  and darkmatter-cli instances are main-side siblings of the same defect
  family.
- The PR's new WSL2-visible tests fail with pre-existing harness signatures,
  not new logic signatures — no WSL2-specific changes needed to the anchoring
  work.
- Recurring test-hygiene themes: (a) sensitivity to ambient host state (env
  vars in P1, PATH contents in P6 and the bare-`bt` L2, HOME/config in P7,
  host SID in P6); (b) fixtures that assume a runtime toolchain (P5); (c)
  stubs/guards that check "spawnable" instead of "usable" (P3c,
  biscuit-tui-cli); (d) Linux-only clippy visibility means macOS-local `just
  lint` cannot certify the workspace.
- Main's 2026-08-12 runs (31588186544 for ubuntu/macOS/Windows cells, its
  WSL2 leg for guest cells) are the drift references used throughout.
