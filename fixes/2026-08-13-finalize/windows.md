# Windows CI Failures — run 31651014023

Analysis of the Windows (`windows-latest`) test failures in CI run 31651014023 (created 2026-08-12 23:28 UTC, i.e. the night of 2026-08-12/13) for PR branch `fix/ctx-launch-anchor` at head `cbe84893a`. Eight Windows jobs failed; the merge gate classified five as BLOCKING (claudine, claudine-cli, rendezvous-daemon, unchained-ai, sniff) and three as baseline-covered (darkmatter-cli, biscuit-tui-cli, sniff-cli). Evidence is CI logs plus source/history reading; Windows cannot be reproduced locally on this macOS host.

Three recurring Windows path mechanisms explain most of the blocking claudine failures:

1. **Escape-eaten `\.`** — when a `ctx.*` value containing a Windows path is textually interpolated into markdown *before* the CommonMark parse, the `\.` sequence (backslash + punctuation) is consumed as a backslash escape, so `...\Temp\.tmpXYZ\...` composes as `...\Temp.tmpXYZ\...`. Backslashes before letters (`\Users`, `\AppData`) survive because they are not valid escapes. Unix paths never contain `\`, so this is invisible off-Windows.
2. **`\\?\` verbatim prefix leakage** — `std::fs::canonicalize` on Windows returns `\\?\C:\...` verbatim paths. `claudine/lib/src/invocation_context.rs` explicitly documents that a verbatim key "must never reach a projection or a comparison against an authored path", but `claudine/lib/src/system_prompt/context.rs:86-89` (`canonical_or_self`) projects canonicalized `cwd`/`repo_root`/`package_*` directly, and several new tests also canonicalize their fixtures.
3. **Batch-file test stubs** — the new Windows provider stubs are `.cmd` files; Rust's `std::process` batch-file spawning rejects arguments containing quotes/newlines ("batch file arguments are invalid", the BatBadBut/CVE-2024-24576 hardening), and the `printf.cmd` shim has a classic `<nul set /p` ERRORLEVEL=1 bug.

## claudine: system_prompt::prepare::tests::appendix_relocation_keeps_launch_context_and_source_local_files

**Cell:** claudine / test (claudine on windows-latest), job 94295876560 — **BLOCKING** (PR subject area).

**Error** (`claudine/lib/src/system_prompt/prepare/tests.rs:343`):

```
appendix relocation changed ctx.repo_root for C:\Users\RUNNER~1\...\.tmpXYCznj\launch-repository\appendix-root.md:
appendix.area=alpha-lib appendix.repo=C:\Users\RUNNER~1\AppData\Local\Temp.tmpXYCznj\launch-repository
```

The composed markdown contains `Temp.tmpXYCznj` — the backslash between `Temp` and `.tmpXYCznj` is gone — so the `contains(launch_repo.to_string_lossy())` assertion fails.

**Insights:** Test is new in this PR (commit 3773a92b3, "anchor prepared ctx.* to invocation launch evidence"); never ran on Windows before, so this is not a regression of previously-green behavior — but it exposes a real product defect: `{{ ctx.repo_root }}` is substituted into raw markdown text before parsing, and CommonMark escape processing eats `\.` (mechanism 1 above). Only the `\.` positions collapse; `\U`/`\A`/`\L` survive, matching the observed corruption exactly. On Windows the composed prompt shipped to an agent would contain a corrupted path. Fix hypothesis: escape backslashes (or all CommonMark-escapable punctuation pairs) when interpolating values textually, or interpolate into the parsed tree instead of the raw source. PR-caused in the sense that the PR introduced the anchored-ctx interpolation paths and their tests; the escape-eating itself is a latent darkmatter compose behavior.

## claudine: system_prompt::prepare::tests::primary_prompt_relocation_keeps_launch_context_and_source_local_files

**Cell:** claudine / test (claudine on windows-latest), job 94295876560 — **BLOCKING** (PR subject area).

**Error** (`tests.rs:268`):

```
source relocation changed ctx.repo_root for ...\launch-repository\system-root.md:
launch.area=alpha-lib launch.repo=C:\Users\RUNNER~1\AppData\Local\Temp.tmpryANNc\launch-repository
```

**Insights:** Identical mechanism to the appendix test above — `\.` eaten from the interpolated `ctx.repo_root` (mechanism 1). Same fix. New test in this PR; never-worked-on-Windows, one shared underlying product bug for both relocation tests plus the claudine-cli sequence/harness_orch failures below.

## claudine: system_prompt::prepare::tests::normal_session_composes_the_shipped_root_system_prompt_from_launch_context

**Cell:** claudine / test (claudine on windows-latest), job 94295876560 — **BLOCKING** (PR subject area).

**Error** (`tests.rs:212`):

```
assertion `left == right` failed
  left: Some("\\\\?\\D:\\a\\rusty-biscuit\\rusty-biscuit\\system-prompt.md")
 right: Some("D:\\a\\rusty-biscuit\\rusty-biscuit\\system-prompt.md")
```

**Insights:** `prepared.source` carries a `\\?\` verbatim path (mechanism 2). `SystemPromptContext` canonicalizes `cwd`/`repo_root`/`package_area_root`/`package_root` via `canonical_or_self` (`system_prompt/context.rs:86-89`), so files discovered under those roots inherit the verbatim prefix, violating the invariant documented at `invocation_context.rs:1317-1319`. On macOS/Linux `canonicalize` is benign (symlink resolution only), so this is Windows-only. New test in this PR; never-worked-on-Windows. Fix hypothesis: strip the verbatim prefix when projecting (dunce-style simplification) or keep canonicalization confined to cache keys as `canonical_key` already does — do not canonicalize the projected context paths.

## claudine-cli: ctx_launch_anchor_baseline::cli_loop_reuses_launch_context_for_root_and_package_prompt_copies

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error:** the spawned `claudine compose --codex .../loop.md` exits 1 with:

```
⤫ ShellExpansionError: execution failed
  Command: printf 'alpha-lib'
  Exit code: 1
  stdout:
  alpha-lib
```

**Insights:** The command produced the correct stdout yet exited 1. The test's own Windows shim is the culprit: `stage_printf` in `claudine/cli/tests/ctx_launch_anchor_baseline.rs` writes

```
@echo off
<nul set /p "=%~1"
```

`set /p` reading from `nul` hits EOF and sets ERRORLEVEL=1, and the script has no trailing `exit /b 0`, so `printf.cmd` always exits 1 — darkmatter's frontmatter shell expansion correctly treats that as failure. Test file is new in this PR (812df339a "enable direct/inline/loop Level 1 matrix on native Windows" explicitly opted these tests into Windows), so this is a PR-side test-harness bug, not product behavior and not main drift. Fix: append `exit /b 0` to `printf.cmd` (or use `set /p` in a way that clears ERRORLEVEL). Secondary note: the test also passes the *canonicalized* (`\\?\`-prefixed) prompt path to the CLI and pins `expected_launch_repo` to the verbatim form; after the printf fix, the frontmatter `when:` comparison of `ctx.repo_root == expected_launch_repo` may surface a verbatim-vs-plain mismatch next (mechanism 2).

## claudine-cli: ctx_launch_anchor_baseline::cli_keeps_eager_schema_files_source_relative_and_ctx_launch_relative

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error:** composition succeeds (banner, preflight, Agent Prompt all rendered) and then the CLI fails spawning the provider:

```
Error: batch file arguments are invalid
```

**Insights:** The Windows codex stub is `codex.cmd` (a batch trampoline into a PowerShell fixture, staged by `stage_windows_provider`). Rust's std hardening for batch-file spawning (CVE-2024-24576, "BatBadBut") rejects arguments containing quotes/newlines, and claudine passes the multi-line composed prompt as an argument — so *any* prompt handed to a `.cmd` stub via argv is unspawnable. PR-side test-harness limitation (stub design), new in this PR; never-worked-on-Windows. Fix hypothesis: replace the `.cmd` stubs with real executables — the established repo pattern is re-exec'ing the test binary via `current_exe` (the same pattern used for L2 probes) — or route the prompt via stdin/temp file instead of argv.

## claudine-cli: ctx_launch_anchor_baseline::cli_uses_launch_context_across_launch_source_matrix

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error:** same `ShellExpansionError` as the loop test: `Command: printf 'alpha-lib'`, exit 1, correct stdout.

**Insights:** Same `printf.cmd` ERRORLEVEL bug; same fix; same follow-on caution about the verbatim `expected_launch_repo` pin. PR-side test-harness bug.

## claudine-cli: ctx_launch_anchor_baseline::inline_cli_uses_launch_context_across_launch_source_matrix

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error:** inline (Goose) variant of the matrix test; same `ShellExpansionError` from `preflight_area: "$(printf '{{ ctx.area }}')"`, exit 1 with correct stdout.

**Insights:** Same `printf.cmd` bug, same fix.

## claudine-cli: ctx_launch_anchor_baseline::cli_keeps_launch_repository_facts_absent_when_source_is_in_repo

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error:** composition renders the expected empty-facts Agent Prompt (`baseline.body.area=[] baseline.body.repo=[] ...`) — i.e. the behavior under test actually held — then dies at provider spawn with `Error: batch file arguments are invalid`.

**Insights:** Same `.cmd`-stub/BatBadBut limitation as the eager-schema test. The launch-context behavior being asserted looks correct on Windows; only the stub transport fails. Same fix (exe stub / stdin delivery).

## claudine-cli: sequence_ctx_launch_anchor::external_sequence_uses_launch_facts_and_source_relative_schema_and_file_reads

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error** (`claudine/cli/tests/sequence_ctx_launch_anchor.rs:141`): the sequence ran successfully, but stdout shows

```
Step area=[alpha-lib] root=[C:\Users\runneradmin\AppData\Local\Temp.tmpQP6rk1\launch-repository] ... file=[source-marker].
```

and the assertion `stdout.contains("root=[{launch_repo}]")` fails because the real path is `Temp\.tmpQP6rk1`.

**Insights:** Escape-eaten `\.` (mechanism 1) in the composed `{{ ctx.repo_root }}` — everything else about launch-fact anchoring (area, agent, model, source-relative file read) worked. New test in this PR; same underlying interpolation bug as the claudine lib relocation tests. One fix covers all of them.

## claudine-cli: sequence_ctx_launch_anchor::serial_and_parallel_prompt_tasks_keep_target_identity_and_launch_facts

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error** (`sequence_ctx_launch_anchor.rs:243`):

```
assertion `left == right` failed: launch root changed in a task; ...
  left: 0
 right: 4
```

All four task lines printed `root=[...Temp.tmpcIf0uV\launch-repository]` (backslash eaten), so the expected `root=[...Temp\.tmpcIf0uV...]` matched 0 of 4 times.

**Insights:** Same escape-eaten `\.` (mechanism 1). Note the launch root was *identical and stable* across all four tasks — the PR's actual invariant held; only the Windows rendering of the interpolated path is corrupt.

## claudine-cli: shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING**, but **cross-OS** (also fails macOS; reproduced locally).

**Error** (`claudine/cli/tests/shipped_prompt_route_drift.rs:137`):

```
a shipped prompt in the `implement` route changed.
  left:  {"prompts/_implement/implement-plan.md": "62d70fb16a02592c-652e691e678f8b5a", ...}
 right:  {"prompts/_implement/implement-plan.md": "7f268c6d7aa0dd12-652e691e678f8b5a", ...}
```

**Insights:** Nothing Windows-specific — the body-hash segment differs identically on every OS (I verified locally: `md hash prompts/_implement/implement-plan.md` on this worktree yields `62d70fb16a02592c-652e691e678f8b5a`, matching CI's computed "left"). Timeline: the branch edited the shipped prompt and re-pinned the fixture hash in 398222663 ("implement-plan adds phase fallback and plan link"), then merge e1dea6847 ("Merge branch 'main' into fix/ctx-launch-anchor", 2026-08-13 00:20 +0100) brought main's newer `prompts/_implement/implement-plan.md` body without refreshing the pin. So: main drift merged into the branch, plus a stale branch-side pin. Fix: review the merged shipped change, re-derive the side-effect-free fixture copy, then re-pin with `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli --test shipped_prompt_route_drift` (the test's own message prescribes exactly this).

## claudine-cli: harness_orch::prompt::tests::proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error** (`claudine/cli/src/commands/wrap/harness_orch/prompt.rs:516`):

```
wrong target-adjusted launch snapshot for ProxyTarget
  left: "codex/gpt-reentry/codex/gpt-reentry/C:\\Users\\RUNNER~1\\AppData\\Local\\Temp.tmpCj82fo\\launch"
 right: "codex/gpt-reentry/codex/gpt-reentry/C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpCj82fo\\launch"
```

**Insights:** The materialized prompt (`{{ ctx.repo_root }}` in the document body) lost the `\` before `.tmpCj82fo` — escape-eaten `\.` again (mechanism 1), this time in a pure unit test with no CLI spawn, which pins the bug to darkmatter compose interpolation itself rather than any harness/stub layer. New test in this PR; never-worked-on-Windows; same single product fix as the relocation and sequence tests.

## claudine-cli: harness_orch::prompt::tests::stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING**, but **cross-OS** (also fails macOS; reproduced locally).

**Error** (`prompt.rs:443`):

```
assertion `left == right` failed
  left: Some("unknown")
 right: Some("codex")
```

**Insights:** Nothing Windows-specific in the failure itself — `ctx.agent` in the first prepared epoch context resolves to the `"unknown"` fallback instead of the `AGENT=codex` the test injected into the epoch context env (`context.env_mut().insert("AGENT", "codex")`) and into `CallerInputLayers::env_overrides`. This is a genuine logic defect in the PR's launch-epoch preparation path (`harness_prepare_options` not projecting the epoch context's env overlay into `ctx.agent`), new test, fails everywhere. It should be diagnosed and fixed on macOS where it reproduces locally; Windows adds nothing.

## claudine-cli: sequence::jit::tests::template_preflight_combines_launch_facts_with_the_selected_target

**Cell:** claudine-cli / test (windows-latest), job 94295876499 — **BLOCKING** (PR subject area).

**Error** (`claudine/cli/src/commands/wrap/sequence/jit/tests.rs:236`):

```
target-adjusted command was not approved:
{"echo \"C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpMjhAMb\\launch\" claude claude-sonnet-4-6 claude claude-sonnet-4-6"}
```

**Insights:** Windows-only expectation mismatch, and notably *not* the escape-eating bug — here the approved command retains `\.` intact but the interpolated `ctx.repo_root` was wrapped in double quotes (`echo "C:\...\launch" ...`), presumably because the shell-directive interpolation quotes values containing shell-special characters (`\`). The test builds its expectation as the raw unquoted `echo {launch_root} claude ...`, which matches on Unix (no backslashes) but not on Windows. New test in this PR; never-worked-on-Windows. Fix hypothesis: derive the expected string through the same quoting rule the product applies (or make the approval matcher normalize quoting), rather than string-formatting the raw path. Worth confirming on a native Windows host which layer inserts the quotes before choosing between test fix and product fix.

## rendezvous-daemon: local_transport::windows::tests::the_pipe_dacl_names_this_user_and_nobody_else

**Cell:** rendezvous-daemon / test (windows-latest), job 94295875986 — **BLOCKING**, likely main drift (not PR subject).

**Error** (`claudine/rendezvous/daemon/src/local_transport/windows/tests.rs:110`):

```
the current user must own the descriptor; got: O:LAD:P(A;;GA;;;LA)
```

**Insights:** The descriptor is actually correct — it names exactly one principal, protected DACL, owner == that principal. The assertion fails because it string-matches the raw SID (`S-1-5-21-...-500`) against SDDL text, and `ConvertSecurityDescriptorToStringSecurityDescriptorW` renders well-known SIDs as two-letter aliases: the GitHub runner account (`runneradmin`) is the built-in Administrator (RID 500), rendered as `LA`. So this is an environment-conditional string-brittleness: it passes wherever the user's SID has no SDDL alias (any normal dev account) and fails on GH-hosted Windows runners. This PR does not touch rendezvous; the tests came from main (dd3f84608 "fix: improve cross-platform reliability"), and PR-independent — main drift / never-green-on-GH-runners. Fix: compare parsed ACE SIDs for equality (round-trip the expected SID through `ConvertStringSidToSid`/SDDL rendering) instead of substring-matching the literal SID text.

## rendezvous-daemon: private_dir::tests::the_current_user_descriptor_names_this_account_and_nobody_else

**Cell:** rendezvous-daemon / test (windows-latest), job 94295875986 — **BLOCKING**, likely main drift.

**Error** (`claudine/rendezvous/daemon/src/private_dir/tests.rs:237`): `got: O:LAD:P(A;;GA;;;LA)` — same shape as above; the test fetches the SID via `sniff::os::current_user_id()` and substring-matches it against the SDDL.

**Insights:** Identical SDDL-alias mechanism (`LA` vs literal `S-1-5-...-500`) and identical fix. Main drift; unrelated to launch-context work.

## unchained-ai: pty_runner::tests::test_ansi_stripping

**Cell:** unchained-ai / test (windows-latest), job 94295876848 — **BLOCKING**, likely main drift.

**Error** (`unchained-ai/lib/src/primitives/services/pty_runner.rs:286`):

```
ANSI-producing command should succeed: TimeoutError("Command timed out after 5s")
```

**Insights:** The test runs `cmd.exe /D /S /C echo ...` inside a portable-pty ConPTY and times out at exactly the 5 s deadline of the polling loop. That loop is new: commit 30fb0d603 "fix(unchained-ai): make PTY shutdown deterministic" (2026-08-09, part of the fix/pty-non-windows line that merged as PR #55) replaced the old reader-EOF wait with a `child.try_wait()` poll and defers `drop(pair.master)` until after exit is observed. On Windows the child's exit is evidently never observed within 5 s — consistent with ConPTY keeping the child (or its observable exit) pinned until the pseudo console / master side is closed, which the new ordering never does before the deadline. This PR does not touch unchained-ai (`git diff e03bafee9..cbe84893a -- unchained-ai/` is empty), so it is main drift from the PTY-shutdown rewrite. Critically, PR #55's own full ci run was cancelled at merge time (run 31703837068), and main's green runs since (e.g. e03bafee9) do not include the Windows matrix — so the new loop was never validated on native Windows. Fix hypothesis: on Windows, keep the old reader-EOF-driven wait (or drop/close the master and pseudo console before polling for exit), per the "fix non-Windows without breaking Windows" intent of PR #55's title. Needs native Windows evidence to confirm the exact ConPTY ordering.

## unchained-ai: pty_runner::tests::test_run_echo_command

**Cell:** unchained-ai / test (windows-latest), job 94295876848 — **BLOCKING**, likely main drift.

**Error** (`pty_runner.rs:278`): `echo command should succeed: TimeoutError("Command timed out after 5s")`.

**Insights:** Same ConPTY exit-observation stall as `test_ansi_stripping`; both tests exercise the same `run_pty_blocking` loop rewritten in 30fb0d603. Same fix.

## sniff: filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing

**Cell:** sniff / test (windows-latest), job 94295875123 — **BLOCKING**, main drift (also fails macOS and Ubuntu in the same run).

**Error** (`sniff/lib/src/filesystem/repo/standard.rs:1921`): `assertion failed: resolved.is_none()`.

**Insights:** The test builds `ExecutableIndex::for_test(HashMap::new())` (empty synthetic PATH map) and expects `resolve_acting_binary_with_version(GradleMultiProject, ...)` to find nothing. But `ExecutableIndex::find_with_source` (`sniff/lib/src/executable_index.rs:208`) falls back to a real `which(program)` lookup even when an eager test map is present — and all three GitHub-hosted images preinstall Gradle, so `which("gradle")` succeeds and the test fails on every CI OS while passing on hosts without Gradle. This directly contradicts `for_test`'s own doc ("without requiring real monorepo binaries to be installed on the host"). The fallback exists identically on origin/main, so: main drift / hermeticity defect, not PR-caused. Fix: when `eager_path` is `Some`, treat the eager map as authoritative and skip the `which` fallback (and the OS-specific layers).

## darkmatter-cli: schema_validate_baseline::schema_validate_legacy_pretty_output_is_byte_identical

**Cell:** darkmatter-cli / test (windows-latest), job 94295873928 — **baseline-covered** (did not block).

**Error** (`darkmatter/cli/tests/schema_validate_baseline.rs:108`):

```
pretty output drifted for legacy case `property_union_invalid`
  left:  "- ✗ the document [doc.md](file://C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmp7oaLS1\\doc.md) ..."
 right:  "- ✗ the document [doc.md](file://C:\\Users\\runneradmin\\AppData\\Local\\Temp\\.tmp7oaLS1\\doc.md) ..."
```

**Insights:** 8.3 short-name mismatch: the CLI emits the document URL from the path it was given (the tempdir arrives via `%TEMP%`, which on GH runners is the short form `RUNNER~1`), while the test rebuilds the expected URL from `fs::canonicalize` (long form `runneradmin`, `\\?\` stripped). The test's comment claims the CLI links "by its canonical (symlink-resolved) absolute path" — on Windows the CLI evidently does not canonicalize, so comment/expectation and behavior have drifted. Environment-conditional (only bites when `%TEMP%` is a short path and the username exceeds 8 chars); baseline-covered, so main fails the same way — main drift, unrelated to this PR. Fix: canonicalize (short-name-expand) the emitted URL in the CLI, or normalize both sides in the test the way the JSON variant of this baseline already passes.

## biscuit-tui-cli: completions::tests::bash_script_passes_syntax_check

**Cell:** biscuit-tui-cli / test (windows-latest), job 94295876119 — **baseline-covered**.

**Error** (`biscuit-tui/cli/src/completions.rs:471`):

```
bash -n rejected the generated script:

```

(empty stderr after the colon).

**Insights:** The availability guard only checks that spawning `bash --version` does not *error* (`.output().is_err()`), not that it exits 0 — on Windows runners `bash` can resolve to the System32 WSL shim, which spawns fine but fails without an installed distro and cannot open a `C:\Users\...` path argument anyway; its UTF-16LE error output renders as effectively empty through `from_utf8_lossy`, matching the blank stderr. The generated script itself parses cleanly on Linux, so this is a test-portability hole, not a completions regression. Main drift / never-worked-on-Windows; unrelated to this PR. Fix: strengthen the guard (require `bash --version` exit 0 and plausible stdout), prefer Git Bash explicitly on Windows, or gate the test to non-Windows.

## sniff-cli: cli::test_repo_worktree_verbose_includes_path (and test_repo_worktrees_verbose_output)

**Cell:** sniff-cli / test (windows-latest), job 94295875012 — **baseline-covered**. Two failing tests: `test_repo_worktree_verbose_includes_path` (`sniff/cli/tests/cli.rs:7938`) and `test_repo_worktrees_verbose_output` (`cli.rs:8105`).

**Error** (from the second, which prints the output):

```
verbose should contain worktree path: .tmplu5Q5k (on master branch, located at
[\?\C:\Users\runneradmin\AppData\Local\Temp\.tmplu5Q5k](file://D:\?\C:\Users\runneradmin\...))
```

**Insights:** The worktree paths surface in `\\?\` verbatim form (canonicalized somewhere in the git/worktree scan) and are then further mangled by rendering: the display text shows `\?\C:\...` (one backslash eaten) and the `file://` link is the nonsensical `file://D:\?\C:\...`. The tests assert `stdout.contains(worktree_path)` with the plain tempdir path, which the verbatim/mangled output never contains. Same two Windows mechanisms as the claudine failures (verbatim leakage plus backslash-hostile markdown rendering), but living in sniff's worktree listing — baseline-covered, so present on main; not PR-caused. Fix: simplify verbatim paths before display (dunce-style) and build `file://` URLs via a proper path-to-URL conversion instead of string concatenation.

## sniff-cli: snapshots::repo_aggregate_json_snapshot

**Cell:** sniff-cli / test (windows-latest), job 94295875012 — **baseline-covered**.

**Error** (insta snapshot `repo_aggregate_json`, asserted at `sniff/cli/tests/snapshots.rs:810`):

```
-        "root": "[BASE]",
+        "root": "C:\\Users\\runneradmin\\AppData\\Local\\Temp\\.tmpg7e9UN",
...
-          "[BASE]/Cargo.toml"
+          "C:\\Users\\runneradmin\\AppData\\Local\\Temp\\.tmpg7e9UN\\Cargo.toml"
```

**Insights:** The snapshot's `[BASE]` redaction is a string replacement of the fixture base path, and on Windows the emitted values do not string-match the base the test used for redaction — a spelling divergence between the path the test holds (likely `%TEMP%` short form `RUNNER~1`, or verbatim) and the long-form path sniff reports after its own normalization (`runneradmin`), plus `/` vs `\` separator differences visible in the expected `[BASE]/Cargo.toml`. Environment-conditional redaction fragility; baseline-covered → main drift, unrelated to this PR. Fix: canonicalize/simplify the base path the same way sniff does before building the redaction, and make the redaction separator-insensitive.
