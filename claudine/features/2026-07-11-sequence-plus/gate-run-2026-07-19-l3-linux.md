# Gate run — 2026-07-19 — Level-3 Linux keyboard evidence, headless-container X11 (review-8 finding 1, Linux leg)

Real OS-keyboard Level-3 evidence for the sequence Ctrl+C contract on Linux:
`cli/tests/level3_linux_sequence_ctrl_c.rs::level3_linux_sequence_ctrl_c_fans_out_to_parallel_children`
executed against a real X server, a real window manager, and a real WezTerm GUI
inside a Docker Linux container. This file records measurement only. No
production code and no test assertion was changed to produce it; the one red
run below was classified and answered by correcting the *container
environment*, not the test.

**What this is and is not.** This is a headless-container X11 session (Xvfb —
a private display with no human desktop), **not** the attended native desktop
session review 8 demands. It closes the "no Linux L3 execution evidence
exists" gap with real XTEST keyboard injection, but the attended runs —
macOS native, Linux native desktop, and Windows — **remain owed**. The
attended-desktop guard (`just test-l3`'s TTY prompt / `BISCUIT_L3_TAKE_FOCUS`)
exists to protect a human's active desktop from focus theft; this container
has no desktop to steal, so the recipe's underlying nextest invocation was
replicated directly instead (details under Method). `BISCUIT_L3_TAKE_FOCUS`
was never set anywhere, and nothing L3 ran on the macOS host.

## Revision under test

```
git rev-parse HEAD
baba838446cf0e21c33dd17870c462b45c0311e6
```

The working tree is **not** clean, and that is load-bearing: the Level-3
fixtures compile only with the uncommitted biscuit-test-harness injector
modules (`win_input.rs`, `xdotool.rs`, and their `lib.rs` declarations)
present. `git status --short` at run time, verbatim:

```
 M .claudine/memory/commits.md
 M CLAUDE.md
 M biscuit-test-harness/src/lib.rs
 M claudine/features/2026-07-11-sequence-plus/review-7.md
 M claudine/features/2026-07-11-sequence-plus/spec.md
?? biscuit-test-harness/src/win_input.rs
?? biscuit-test-harness/src/xdotool.rs
?? claudine/features/2026-07-11-sequence-plus/review-8.md
?? prompts/_implement/implement-review-findings-plan.md
?? prompts/_implement/review-findings-plan.md
```

The `biscuit-test-harness` entries are the L3 injector seam the fixture
imports (`biscuit_test_harness::xdotool`); the rest are docs/prompts. Per the
review-7 Linux-gate precedent, the tree was copied with
`rsync -a --exclude=target/ --exclude=.git` (plus `--exclude=.gitnexus/`) to
`/tmp/rb-linux-src` and mounted at `/src` — the worktree `.git` file points
outside the tree and cannot be carried into a container — so the container
tested exactly this tree.

## Summary of verdicts

| Gate | Verdict | Exit code | Duration |
|---|---|---|---|
| Linux X11: `level3_linux_sequence_ctrl_c_fans_out_to_parallel_children` (XTEST keyboard → WezTerm → PTY → claudine fan-out) | **Green — 1/1, first try, no retries, under `BISCUIT_TEST_LEVEL_REQUIRED=3`** | `0` | 2.13 s |
| Attended-desktop L3 (macOS native re-run, Linux native desktop, Windows) | **Not run — still owed.** | — | — |

## Environment

**Host:** macOS (Apple Silicon), Docker Desktop VM (aarch64, ~7.75 GiB).
Nothing L3 executed on the host itself.

**Container:** `rust:latest` (Debian 13 "trixie"), run with `--init`
(tini 0.19.0 as PID 1 — load-bearing, see the classified red run) and
`--memory=7g`. Verbatim:

```
Linux d42c25acfbd7 6.12.76-linuxkit #1 SMP Thu Jun 25 13:45:40 UTC 2026 aarch64 GNU/Linux
PRETTY_NAME="Debian GNU/Linux 13 (trixie)"
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo-nextest 0.9.140 (a9fef2964 2026-07-05)
```

This is a real Linux kernel (Docker Desktop's linuxkit VM), aarch64 native —
no emulation.

**X stack:** Xvfb (X.Org, display `:99`, 1280x800x24 — a private virtual
display), Openbox 3.6.1 (EWMH-compliant WM; honors
`xdotool windowactivate --sync`), xdotool 3.20160805.1 (XTEST injection).

**Terminal:** WezTerm `20240203-110809-5046fc22`, installed from the official
`Debian12.arm64.deb` GitHub release asset (installs cleanly on trixie). GUI
started under Xvfb with `front_end = "Software"` in `wezterm.lua` (no GPU in
the container); harness reached it via
`WEZTERM_UNIX_SOCKET=/run/user/0/wezterm/gui-sock-<pid>` (on Linux the GUI
socket lives under `XDG_RUNTIME_DIR/wezterm/`, not `~/.local/share/wezterm/`).

**Injection-path preflight.** Before the test, the full XTEST chain was
probed end-to-end: `xdotool type 'echo XTESTPROBE_…'` + `xdotool key Return`
against the focused WezTerm window, then `wezterm cli get-text` showed the
probe line echoed by the pane's shell — keyboard → X server → WezTerm input
encoder → PTY → shell, confirmed before any assertion depended on it.

## Method

`just test-l3` was not used: its recipe requires a TTY confirmation prompt
(or `BISCUIT_L3_TAKE_FOCUS=1`, which must never be set by an agent). Instead
the recipe's exact nextest invocation (`just/devops.just::_test_l3`) was
replicated, from `/src/claudine`, with a **positional** filter (intersects
with the recipe's `-E` tier filter; a second `-E` would union):

```
RUN_LEVEL3=1 BISCUIT_TEST_LEVEL_REQUIRED=3 \
  cargo nextest run -p claudine-cli -j 1 -E 'test(/level3_/)' --no-tests=pass \
  level3_linux_sequence_ctrl_c
```

with `DISPLAY=:99`, `HOME=/root`, `XDG_RUNTIME_DIR=/run/user/0`, and
`WEZTERM_UNIX_SOCKET` exported. **Deliberate guard bypass, documented:** the
attended-desktop guard protects a human's desktop from focus theft; this
container's Xvfb display is private and no desktop exists to steal, so
bypassing the TTY prompt does not bypass what the guard protects.
`BISCUIT_TEST_LEVEL_REQUIRED=3` turns a missing backend into a hard failure,
so a silent skip could not masquerade as the pass below. Only this one
fixture was run — not the L3 tier, not the wider suites.

The test binary set was prebuilt in the container
(`cargo nextest run -p claudine-cli --no-run -E 'test(/level3_/)'`).
Build notes for the next round: the cached `/tmp/claudine-linux-target` was
produced on `rust:latest` — a `rust:bookworm` container cannot reuse it
(glibc 2.36 vs 2.39+; build scripts fail with `GLIBC_2.39 not found`); and
linking claudine-cli's test binaries at 7 GiB needs `CARGO_BUILD_JOBS=1`
(jobs=4 and jobs=2 both produced `ld terminated with signal 9` OOM kills;
each was resumed incrementally, no artifacts were affected).

## Result — verbatim

Nextest, from `/t/l3run2.log`:

```
 Nextest run ID 88faab28-bc0b-4fd4-9a31-85d8d12b3c97 with nextest profile: default
    Starting 1 test across 105 binaries (2324 tests skipped)
        PASS [   2.127s] (1/1) claudine-cli::level3_linux_sequence_ctrl_c level3_linux_sequence_ctrl_c_fans_out_to_parallel_children
────────────
     Summary [   2.130s] 1 test run: 1 passed, 2324 skipped
RUN_EXIT=0
```

Pane frame at completion, captured live from outside the test via a 200 ms
`wezterm cli get-text` poll (the harness kills the pane on Drop, so the frame
must be taken during the run), verbatim:

```
root@d42c25acfbd7:~# cd '/tmp/.tmp2y0jqg'
root@d42c25acfbd7:/tmp/.tmp2y0jqg# export PATH='/tmp/.tmp2y0jqg/bin':"$PATH"
root@d42c25acfbd7:/tmp/.tmp2y0jqg# NO_COLOR='1' HOME='/tmp/.tmp2y0jqg' /t/debug/
claudine sequence --goose --yolo /tmp/.tmp2y0jqg/seq.md ; echo L3LINSEQ_0rc=$?
ℹ Sequence: 2 step(s), fail_fast is set to true
ℹ Starting pre-flight checks
ℹ Preflight: shell commands approved for all 2 step(s) in the sequence
ℹ [1/2] starting blocking-group
│ ▶ child-b
│ ▶ child-c
│ ▶ child-a
^C│ child-c — interrupted (0.9s)
│ child-b — interrupted (1.0s)
│ child-a — interrupted (1.0s)
⤫ step 1/2 interrupted by Ctrl+C
- child-a — interrupted (1.0s)
- child-b — interrupted (1.0s)
- child-c — interrupted (0.9s)

⤫ Sequence finished: 0 succeeded, 1 failed
L3LINSEQ_0rc=130
root@d42c25acfbd7:/tmp/.tmp2y0jqg#
```

A concurrent window-title poll recorded exactly **one** X11 window matching
`claudine` (id `4194314`) in all 14 samples taken during the run — the
fixture's ambiguity gate (`window_id_for_title` hard-errors on ≠1 match) was
satisfied cleanly.

### The four runbook observations

1. **Pane output** — above. The `^C` echo and the three task-attributed
   `interrupted` lines are present; the injected event was a single
   `xdotool key --clearmodifiers ctrl+c` (XTEST) after
   `xdotool windowactivate --sync` on the one `claudine`-titled window.
2. **Exit code 130** — the sentinel `L3LINSEQ_0rc=130` in the frame; it is
   printed only after Claudine exits, so it doubles as proof the shell
   regained control.
3. **Later-step marker absent** — asserted by the passing test
   (`later-step-ran.txt` must not exist; the `must-not-run` step never
   launched — no `[2/2]` line in the frame).
4. **Descendant cleanup** — asserted by the passing test: all six pids
   (3 SIGINT-immune tasks + 3 SIGINT-immune backgrounded descendants) were
   dead within the 5 s reap poll. The fixture ignores SIGINT throughout, so
   dead pids can only mean Claudine's fan-out reached each task's process
   group. A post-run `ps` sweep found zero `Z`-state survivors.

## One red run, classified — environment, not product

The first execution of this fixture (same tree, same container image, but
**without** `--init`) failed; it is recorded because its classification is
load-bearing for anyone re-running this evidence:

- **Try 1** — the product contract itself held: the frame showed `^C`, all
  three children `interrupted`, no later step, and `L3LINSEQ_0rc=130`. The
  failure was the descendant-reap assertion: two descendant pids still
  answered `kill(pid, 0) == 0`. Discrimination: `ps` showed both as `Z`
  (zombie) — dead and SIGKILLed, but never reaped, because that container's
  PID 1 was `sleep infinity`, which does not `wait()` on reparented orphans.
  A zombie satisfies `kill(pid, 0)`. The fixture's own doc-comment assumption
  ("a reaped child cannot linger as a zombie") holds under any real init —
  and under `--init` (tini) the same assertion passed with zero zombies.
  Classification: **container-init artifact, not a fan-out defect.**
- **Tries 2–4** — `2 X11 windows matched "claudine"`: each retry's
  `--new-window` spawn overlapped the previous try's window teardown, so two
  WezTerm windows briefly co-existed. Pure retry cascade off the try-1
  artifact; with the root cause fixed the green run passed on try 1 and the
  single-window poll above confirms no ambiguity existed.

No assertion was loosened; the environment was corrected (`--init`) and the
fixture rerun unmodified.

## What this run does and does not certify

### Does certify

- **The full Linux X11 keyboard path, on a real kernel, end to end:** an
  XTEST-synthesized Ctrl+C key event → X server → focused WezTerm GUI
  window's input encoder → ETX on the PTY → tty SIGINT delivery →
  Claudine's signal handler → interrupt fan-out to a blocking
  `execution: parallel` group whose tasks and descendants are SIGINT-immune
  and in their own process groups → all six processes dead, later step
  suppressed, shell regains control, exit `130`.
- That the fixture runs green **without retries** under
  `BISCUIT_TEST_LEVEL_REQUIRED=3` (no silent-skip ambiguity).

### Does not certify

- **Attended-desktop conditions** — a real desktop with focus contention,
  compositor/WM variety, and a human present. Review 8's attended runs on
  macOS (re-run at current revision), native desktop Linux, and Windows are
  **still owed**; this file must not be cited as closing them.
- **The macOS Quartz injector path** (`cliclick`/System Events) — different
  injector, different encoder host; the 2026-07-18 macOS pass predates the
  process-tree rewrite and remains stale.
- **The Windows console path** (`SendKeys`/`GenerateConsoleCtrlEvent`) —
  never executed anywhere.
- **Wayland** — XTEST is X11-only; the fixture skips on Wayland by design.
- **x86_64 Linux specifically** — this kernel is aarch64; signal, process
  group, and tty semantics are architecture-independent, recorded as a
  footnote, not a hole.
