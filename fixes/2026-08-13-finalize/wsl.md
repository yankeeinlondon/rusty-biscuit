# WSL2 (wsl2-ubuntu) CI failures — run 31651014023

Analysis of every failing `wsl2-ubuntu` test cell in CI run 31651014023 (branch
`fix/ctx-launch-anchor`, 2026-08-13). Evidence: CI logs pulled per job via the
GitHub API, test sources on this branch, and the same cells on main's most
recent full run (31588186544, commit e03bafee9, 2026-08-12) for drift
comparison. Blocking cells (no `ci-baseline.toml` entry for the
package/environment pair) come first; the baseline-covered cells follow, led by
a cluster analysis of the claudine-cli known-red set.

**Headline:** every blocking WSL2 failure (claudine ×3, sniff ×1, messenger ×1)
reproduces byte-for-byte on main's 2026-08-12 run — all five are main drift,
not PR-caused. They block only because their package cells were never
baselined. Within the baseline-covered claudine-cli cluster, two tests are
genuine PR-area failures — but both also fail on plain ubuntu-latest in this
run, so neither is a WSL2 problem. The PR's new `ctx_launch_anchor_baseline`
tests that appear in the WSL2 cluster fail with the *old* harness signature
(90 s slowness timeouts), not a new one: the PR needs no WSL2-specific changes.

**The WSL2 harness environment assumptions, confirmed still current.** The
`_wsl-ci.yml` workflow is explicit (lines 36–42): the guest is **archive-only —
`cargo nextest archive` is built on ubuntu-latest and the guest only *runs* the
binaries. No rustup, no toolchain.** Every failure in this run reduces to one
of three environment assumptions baked into tests:

1. **A Rust toolchain on PATH.** Tests that spawn `rustc` (cwd probes,
   `::shell rustc …` fixtures, darkmatter's `$(rustc)` parse fixtures which
   classify a bare token as a shell command *only if it is found on PATH* — the
   §2 ladder in `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`),
   or `cargo` (trybuild's `model_id::ui`, sniff's monorepo-standard binary
   probe). Nothing to find: the guest has neither binary.
2. **Native-host timing budgets.** Process spawn + 9p-adjacent I/O in the guest
   is slow enough to blow 90 s per-test caps, 20–30 s wall-clock assertions,
   and parallel-overlap timing windows.
3. **Desktop-session services.** The guest's D-Bus is present but unusable
   (`Permission denied`), in a failure shape the messenger test's
   environment-skip arm does not recognize.

A fourth, claudine-cli-specific assumption rides along: the **`md` (darkmatter-cli)
sibling binary** is expected next to the test binary or on PATH, and a nextest
archive does not carry cross-package binaries (see
`project_claudine_test_needs_md_binary`).

---

# Blocking cells

## claudine: composition::error::tests::shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic

**Cell:** claudine / wsl2 test (job 94314765777) — **BLOCKING** (no baseline
entry for claudine/wsl2-ubuntu/L1).

**Error** (`claudine/lib/src/composition/error/tests.rs:1002`):

```
expected captured rustc stderr text in diagnostic: ⤫ ShellExpansionError: command not found
┃ Command: rustc
┃ > 7 │ ::shell rustc --edition=invalid
┃ Install the binary or update $PATH so it is discoverable.
```

**Insights.** The fixture runs `::shell rustc --edition=invalid` and asserts the
diagnostic preserves rustc's own stderr. In the archive-only guest `rustc` does
not exist, so the expansion takes the *command-not-found* path instead of the
*command-failed-with-stderr* path — harness assumption #1. The test dates to
the 2026-06-18 shell-error-diagnostics feature; this PR does not touch
`composition/error/tests.rs`. **Identical failure on main's 2026-08-12 WSL2
run**, and it was already named as WSL2 burn-down territory in
`fixes/2026-08-06-cicd/burn-down.md` ("WSL2 L1 … claudine: 4 tests"). Main
drift, never-worked-on-WSL2. Fix hypothesis: gate on `rustc` discoverability
(skip or substitute a guaranteed-present failing binary), or add a
claudine/wsl2 baseline entry until the guest carries a toolchain.

## claudine: composition::prepare::tests::direct_composition_runs_shell_in_configured_working_directory

**Cell:** claudine / wsl2 test — **BLOCKING**.

**Error** (`claudine/lib/src/composition/prepare/tests.rs:828`):

```
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

**Insights.** Line 828 is `Command::new(rustc).…output().unwrap()` — the test
compiles a `cwd_probe.rs` at runtime to prove `::shell` runs in
`shell_working_directory`. The spawn of `rustc` itself is what returns
`NotFound` (assumption #1). Test predates the PR (the honoring `RUSTC` env-var
variant landed on main 2026-08-02 in 224911717, "close post-acceptance native
Windows test regressions"); the PR adds only an unrelated test to this file.
Identical on main's WSL2 run; listed in the 2026-08-06 burn-down. Main drift.
Fix hypothesis: same as above — the probe already honors `$RUSTC`, so either
skip when neither `$RUSTC` nor `rustc` resolves, or pre-build the probe into
the archive as a fixture binary.

## claudine: system_prompt::prepare::tests::non_repository_session_runs_shell_in_launch_cwd

**Cell:** claudine / wsl2 test — **BLOCKING**.

**Error** (`claudine/lib/src/system_prompt/prepare/tests.rs:94`):

```
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

**Insights.** Line 94 is the `.output().unwrap()` inside the shared
`compile_cwd_probe` helper (same rustc-at-runtime pattern as the previous
test). Same introduction history (224911717, on main), same identical failure
on main's WSL2 run, same burn-down listing. Main drift; fix travels with the
`compile_cwd_probe` fix above. Note the burn-down's fourth listed test
(`validate_permissions_readonly_file`) now passes — the claudine/wsl2 set has
shrunk from 4 to these 3, all rustc-dependent.

## sniff: integration::test_detect_completes_in_reasonable_time

**Cell:** sniff / wsl2 test (job 94307875618) — **BLOCKING** (no baseline entry
for sniff/wsl2-ubuntu; the existing sniff Windows entry is keyed to
sniff-cli/windows-latest).

**Error** (`sniff/lib/tests/integrations.rs:80`, quoted from log):

```
Detection took too long: 29.466629982s
```

**Insights.** The test caps a full `detect()` at 20 s ("Allow slack for CI
environments, package manager detection (PATH scanning), and boundary-aware
mixed-workspace package discovery"). WSL2 blows it: 29.5 s here, **40.1 s on
main's 2026-08-12 run** — main drift, harness assumption #2. The PR does not
touch sniff. Likely dominated by PATH scanning: a WSL guest PATH appends the
Windows host's directories (`/mnt/c/...`), each probe crossing the 9p/drvfs
boundary. Fix hypothesis: raise or environment-gate the budget when
`BISCUIT_CI_ENVIRONMENT=wsl2-ubuntu` (or detect WSL via `sniff`'s own runtime
markers), or exclude `/mnt/*` PATH entries from the program scan.

## messenger: provider::desktop::linux::tests::native_fallback_delivers_when_no_helpers_installed

**Cell:** messenger / wsl2 test (job 94310212217) — **BLOCKING** (no baseline
entry for messenger on any environment).

**Error** (`messenger/lib/src/provider/desktop/linux.rs:713`):

```
expected native receipt or notification-service skip, got Transport { provider: Desktop,
message: "D-Bus notification failed: I/O error: Permission denied (os error 13)" }
```

**Insights.** The test already anticipates headless runners: it accepts a
Transport error as an environment skip, but only when the message names
`org.freedesktop.Notifications` or contains "service". The WSL2 guest fails
one step earlier — connecting to the bus socket at all (`Permission denied`,
os error 13) — so the message matches neither pattern and falls through to the
panic arm. Harness assumption #3, in a failure shape the skip arm was not
written for. Identical on main's WSL2 run; the PR does not touch messenger.
Main drift. Fix hypothesis: widen the skip arm to accept connection-level
D-Bus transport failures (e.g. any `D-Bus notification failed:` I/O or
permission error), keeping the panic for genuinely unexpected shapes.

---

# Baseline-covered cells

## claudine-cli WSL2 cluster analysis (job 94317065858, baseline-covered)

The claudine-cli/wsl2-ubuntu/L1 cell is baselined ("red on main at the time of
sync; ~33 failing tests", source run 30791562395, expiry 2026-11-30). This
run's cell shows **25** red tests — 12 FAIL + 13 TIMEOUT (2,388 run, 2,363
passed, 242 skipped) — versus **26** on main's 2026-08-12 run (11 FAIL + 15
TIMEOUT). The overlap is 20 tests; the churn is entirely inside the flaky
90 s-timeout family (six loop/sequence tests red on main but green here, five
red here but green there — three of those five being tests this PR adds).

Verdict on the earlier "3 harness assumptions" theory: **still explains the
cluster**, with the concrete forms being (1) no `rustc` in the archive-only
guest, (2) no `md` sibling binary in the nextest archive, and (3) native-host
timing budgets (90 s per-test cap, parallel-overlap windows, watchdog/exit-code
timing). Every one of the 25 fits one of those three; none shows a new logic
signature.

**The PR-relevant question — do the new `ctx_launch_anchor_baseline` /
`sequence_ctx_launch_anchor` tests fail with a new signature? No.** All three
red `ctx_launch_anchor_baseline` tests are pure 90 s TIMEOUTs with no output
("(test timed out)"), i.e. the existing assumption-#2 signature. Their sibling
tests in the same new files **pass on WSL2** — slowly:
`cli_keeps_eager_schema_files_source_relative_and_ctx_launch_relative` in
22.1 s, `cli_keeps_launch_repository_facts_absent_when_source_is_in_repo` in
64.8 s (flagged SLOW >60 s), and all three `sequence_ctx_launch_anchor` tests
in 2.9–13.9 s. The three that time out are exactly the multi-case matrix tests
that spawn the real CLI once per case, so they sit furthest over the cap. On
ubuntu-latest in this same run all of them pass. No anchoring logic is
implicated; if anything is worth doing, it is splitting the matrix tests into
per-case tests so each fits the WSL2 budget.

**Two entries are NOT harness noise** — but they are also not WSL2 problems:
`shipped_prompt_route_drift` and the `stabilized_reread…` unit test fail
identically on ubuntu-latest (and the claudine-cli macOS/Windows cells also
failed in this run). They are cross-platform PR-area failures that the WSL2
cell merely also displays; details in their entries below. They need fixing on
this branch, but not for WSL2 reasons.

### The 25 red tests

## claudine-cli: ctx_launch_anchor_baseline::cli_loop_reuses_launch_context_for_root_and_package_prompt_copies

TIMEOUT 90.015s, no output ("(test timed out)"). **New in this PR.** Multi-copy
loop test spawning the real CLI repeatedly; passes on ubuntu-latest. Cluster
assumption #2 (timing), same signature as the pre-existing timeout family — see
cluster analysis.

## claudine-cli: ctx_launch_anchor_baseline::cli_uses_launch_context_across_launch_source_matrix

TIMEOUT 90.014s, no output. **New in this PR.** Launch-source matrix (multiple
CLI invocations per test); passes on ubuntu-latest. Assumption #2; see cluster
analysis.

## claudine-cli: ctx_launch_anchor_baseline::inline_cli_uses_launch_context_across_launch_source_matrix

TIMEOUT 90.005s, no output. **New in this PR.** Inline-compose variant of the
matrix test; passes on ubuntu-latest. Assumption #2; see cluster analysis.

## claudine-cli: shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture

FAIL 0.029s (`claudine/cli/tests/shipped_prompt_route_drift.rs:137`):

```
a shipped prompt in the `implement` route changed.
left:  {"prompts/_implement/implement-plan.md": "62d70fb16a02592c-652e691e678f8b5a", …}
right: {"prompts/_implement/implement-plan.md": "7f268c6d7aa0dd12-652e691e678f8b5a", …}
```

**Not a WSL2 issue — fails identically on ubuntu-latest.** PR-caused: commit
398222663 ("implement-plan adds phase fallback and plan link") edited
`prompts/_implement/implement-plan.md` (+3 lines) and bumped
`shipped-hashes.json`, but the pinned hash does not match the shipped bytes now
on the branch (first hash segment differs; `prompts/implement.md` matches).
Fix: re-derive the fixture from the current shipped bytes and refresh the pin
with `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli
--test shipped_prompt_route_drift`, after reviewing the fixture-vs-shipped
delta as the test's module docs require.

## claudine-cli: bin/claudine commands::wrap::harness_orch::prompt::tests::stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity

FAIL 0.342s (`claudine/cli/src/commands/wrap/harness_orch/prompt.rs:443`):

```
assertion `left == right` failed
  left: Some("unknown")
 right: Some("codex")
```

**Not a WSL2 issue — fails identically on ubuntu-latest.** New unit test in
this PR. The very first `harness_prepare_options` call returns
`ctx.agent = "unknown"` even though the test injects `AGENT=codex` into both
the epoch context env snapshot and `CallerInputLayers::env_overrides` — the
prepared context is not seeing the injected env, falling back to the (empty)
process environment. This is squarely in the PR's capture-once/launch-epoch
subject area and needs a cross-platform fix on the branch (likely the prepare
path re-capturing ambient env instead of honoring the provided epoch context).
Severity: Critical for this PR, platform-independent.

## claudine-cli: contextual_errors::compose_shell_execution_failure_renders_rich_block

FAIL 51.8s (`claudine/cli/tests/contextual_errors.rs:133`): "expected captured
rustc stderr text in diagnostic; got: ⤫ ShellExpansionError: command not
found". Assumption #1 (no rustc in guest) — the CLI-level twin of the blocking
claudine library test. Red on main's WSL2 run too.

## claudine-cli: inline_compose_hash::inline_compose_writes_hash_that_passes_md_diff

FAIL 20.0s (`claudine/cli/tests/inline_compose_hash.rs:49`): "md binary not
found at \"/tmp/nextest-archive-…/target/…/debug/md\" and not on PATH; run
`cargo build -p darkmatter-cli --bin md` or `just init`". Assumption — the
nextest archive carries no cross-package binaries and the guest cannot build
one. Red on main too.

## claudine-cli: handle_blocking_output::handle_flushes_blocking_payload_before_nonzero_exit

FAIL 53.0s: "Unexpected return code, failed var == 2" (assert_cmd closure).
Exit-code observed differs under guest timing/process handling. Red on main
too; assumption #2 family.

## claudine-cli: loop_cli::compose_loop_rate_limit_pause_waits_then_continues

FAIL 27.1s: "Unexpected failure. ┃ API Error …" — the rate-limit
pause/continue choreography breaks under guest timing. Red on main too
(36.1 s there); assumption #2.

## claudine-cli: loop_cli::compose_fail_fast_deprecated_env_emits_warning

TIMEOUT 90.007s, no output. Red on main too; assumption #2.

## claudine-cli: sequence_groups::a_parallel_group_overlaps_its_members

FAIL 7.6s (`claudine/cli/tests/sequence_groups.rs:465`): "three 1s tasks took
7.567537234s; serial execution would need ~3s". Parallel-overlap wall-clock
assertion; guest spawn latency destroys the overlap window. Red on main too;
assumption #2.

## claudine-cli: sequence_groups::max_parallel_bounds_the_overlap

FAIL 9.3s (`sequence_groups.rs:517`): "four 1s tasks capped at 2 took
9.268370599s; nothing overlapped". Same as above.

## claudine-cli: sequence_overlay_pty::pty_sequence_prompt_dedupes_and_launches_all_steps

FAIL 21.4s (`sequence_overlay_pty.rs:188`): "expected both sequence steps to
launch after the deduped prompt was satisfied; counter content: \"1\"" — the
second PTY step never launched within the window. Red on main too;
assumption #2 (PTY + spawn latency).

## claudine-cli: sequence_overlay_pty::pty_sequence_step_overlay_satisfies_required_property

FAIL 21.5s, same file/family as above. Red on main too; assumption #2.

## claudine-cli: sequence_schema::sequence_per_step_step_timeout_override

FAIL 10.0s (`sequence_schema.rs:337`): "per-step step_timeout override should
fire quickly; run took 10.016814176s". A should-fire-fast assertion blown by
guest latency. Red on main too; assumption #2.

## claudine-cli: wrap_opencode::opencode_stderr_stream_error_cap_1_17_8_forces_early_termination

FAIL 30.1s (`wrap_opencode.rs:697`): "1.17.8 stream-error cap must map to
exit_code=1; got -1, stderr=" — exit code −1 means the child died to a signal
(watchdog kill under guest timing) before the mapped exit could be observed.
Red on main too; assumption #2.

## claudine-cli: shipped_prompts::shipped_implement_prompt_runs_real_router_target

TIMEOUT 90.008s, no output. Runs the real shipped router end-to-end — the
heaviest compose path. Red on main too (also times out on ubuntu-latest in
this run, so it straddles environments); assumption #2.

## claudine-cli: wrap_compose_validation::compose_dry_run_quiet_and_silent_are_no_op

TIMEOUT 90.008s, no output. Red on main too; assumption #2.

## claudine-cli: wrap_compose_validation::compose_initialize_error_with_failure_raise_surfaces_failure_evaluation_error

TIMEOUT 90.007s, no output. Red on main too; assumption #2.

## claudine-cli: wrap_compose_validation::compose_initialize_when_evaluation_error_exits_non_zero

TIMEOUT 90.008s, no output. Red on main too; assumption #2.

## claudine-cli: wrap_inline_compose::inline_compose_dry_run_quiet_and_silent_are_no_op

TIMEOUT 90.016s, no output. Red on main too; assumption #2.

## claudine-cli: wrap_perf::compose_perf_stdout_matches_non_perf

TIMEOUT 90.008s, no output. Red on main too (and on ubuntu-latest in this
run); assumption #2.

## claudine-cli: wrap_perf::inline_compose_perf_stdout_matches_non_perf

TIMEOUT 90.005s, no output. Red on main too (and on ubuntu-latest in this
run); assumption #2.

## claudine-cli: command_routing::agents_and_commands_route_to_empty_state_messages

TIMEOUT 90.006s, no output. Red on main too; assumption #2.

## claudine-cli: context_command::context_reports_preserve_all_columns_at_minimum_supported_width

TIMEOUT 90.007s, no output. Red on main too; assumption #2.

---

## darkmatter WSL2 (job 94308828241, baseline-covered)

Cell baselined (darkmatter/wsl2-ubuntu/L1, expiry 2026-11-30). 7 red tests
(6 FAIL + 1 TIMEOUT of 6,245 run) — **the identical set fails on main's
2026-08-12 WSL2 run.** The PR's darkmatter changes (51 insertions in the
context-capture path) introduced no new red. All seven are assumption #1
(rustc) or #2 (timing).

## darkmatter: markdown::compose::frontmatter_shell_expansion::tests::detects_no_cache_suffix

FAIL (`…/frontmatter_shell_expansion/tests/tests.rs:146`):

```
Err … "Frontmatter shell expression `$( rustc )` contains no shell command in
executed position … Did you mean to use `{{ rustc }}` interpolation instead?"
```

The §2 token ladder classifies a bare name as a shell command only when it is
**found on PATH** (rule 6); with no rustc in the guest, `$(rustc)::no-cache`
parses as a property reference and is rejected. Assumption #1; main-identical.
Fix hypothesis: these three parser tests need a fixture command guaranteed
present (e.g. `echo`) or a PATH-injected stub — they are testing suffix
parsing, not rustc.

## darkmatter: markdown::compose::frontmatter_shell_expansion::tests::no_cache_defaults_false_without_suffix

FAIL (`tests.rs:156`), same "`$( rustc )` contains no shell command in executed
position" parse error. Assumption #1; main-identical.

## darkmatter: markdown::compose::frontmatter_shell_expansion::tests::no_cache_combines_with_timeout_either_order

FAIL (`tests.rs:164`), same parse error for
`$(rustc)::no-cache::timeout:5`. Assumption #1; main-identical.

## darkmatter: interpolation_literal_pipeline::frontmatter_literal_survives_shell_bracketed_interpolation_passes

FAIL (`darkmatter/lib/tests/interpolation_literal_pipeline.rs:14`): "rustc
should be available while running Rust tests: Os { code: 2, kind: NotFound …}"
— the test's own expect message states the (broken) assumption. It runs
`rustc --print sysroot` to predict shell output. Assumption #1;
main-identical.

## darkmatter: shell_expansion_coordinates::shell_block_execution_failed_renders_inner_diagnostic

FAIL (`darkmatter/lib/tests/shell_expansion_coordinates.rs:169`): "rendered
diagnostic should contain source excerpt centered on file line 7; got: ⤫
ShellBlockError: command failed …" — the `failing_command()` fixture is
`rustc --edition=invalid`; with rustc absent the block takes the spawn-failure
path, whose rendered diagnostic omits the `> 7 │` source excerpt the test
asserts. Assumption #1; main-identical.

## darkmatter: shell_expansion_coordinates::shell_block_origin_counts_lines_not_bytes_with_crlf

FAIL (`shell_expansion_coordinates.rs:142`): "CRLF fixture should highlight
file line 7; got: ⤫ ShellBlockError: command failed …" — same rustc fixture,
same degraded diagnostic. Assumption #1; main-identical.

## darkmatter: ambient_ctx_capture::every_catalog_variable_survives_ambient_options

TIMEOUT 30.026s, no output ("(test timed out)"). Sweeps the full ctx catalog,
which probes git/host facts per variable — over the guest's slow I/O it blows
the 30 s cap. Also times out on main's WSL2 run. Assumption #2. This is the
one PR-adjacent darkmatter test name (ambient capture), but the main-identical
timeout predates the branch.

---

## model_id WSL2 (job 94314094253, baseline-covered)

## model_id: ui::ui

**Cell:** baselined (model_id/wsl2-ubuntu/L1: "1 failing test, e.g.
model_id::ui::ui").

**Error** (trybuild, `trybuild-1.0.116/src/run.rs:62`):

```
ERROR: failed to execute cargo: No such file or directory (os error 2)
…
tests failed
```

**Insights.** trybuild UI tests compile fixture crates by invoking `cargo` at
runtime — impossible in the archive-only guest (assumption #1, cargo flavor).
Main-identical; never worked on WSL2; the PR does not touch model_id. Fix
hypothesis: gate the trybuild test on `cargo` discoverability, or exclude it
from the WSL2 leg via a nextest filterset (it is a compiler-behavior test;
running it once per host OS is arguably enough).

---

## sniff-cli WSL2 (job 94308180688, baseline-covered)

One red test in this cell, not several: the WSL2 log shows exactly **1 failed
(789 run, 2 skipped)**. The worktree-verbose failures the task brief mentions
(`test_repo_worktree_verbose_includes_path`,
`test_repo_worktrees_verbose_output`) failed in the **windows-latest** sniff-cli
cell of this run, not the WSL2 cell.

## sniff-cli: snapshots::repo_aggregate_json_snapshot

**Cell:** baselined (sniff-cli/wsl2-ubuntu/L1: "14 failing tests" at sync — now
down to 1).

**Error** (insta, `sniff/cli/tests/snapshots.rs:810`):

```
 "monorepo_standards": [
-  "binary": { "name": "cargo", "path": "<normalized>", … },
+  "binary": null,
```

**Insights.** The aggregate JSON snapshot pins that the cargo monorepo standard
resolves its `binary` — but the guest has no `cargo` on PATH, so detection
reports `null` (assumption #1, cargo flavor). Never worked on WSL2 under the
archive model; the PR does not touch sniff-cli. Fix hypothesis: normalize the
`binary` object of monorepo standards in the snapshot (it is host-dependent by
nature, like the already-`<normalized>` path), or gate the snapshot on cargo
presence.
