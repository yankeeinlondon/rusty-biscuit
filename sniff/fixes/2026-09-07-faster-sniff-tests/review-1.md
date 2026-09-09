---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-09T13:38:56-07:00
spec: 2026-09-07-faster-sniff-tests/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/fixes/2026-09-07-faster-sniff-tests/log.md
description: "A **fix** review of `2026-09-07-faster-sniff-tests/spec.md`"
fix: 2026-09-07-faster-sniff-tests/review-1.md
---

# Review 1 — Faster Sniff Tests

## Verdict

The fix is **not ready for production**. The implementation makes useful local
improvements: deterministic CLI processes now use a shared isolation policy,
the generic raw-spawn allowlist is empty, requested-work counters cover the
seeded Git path, and the affected terminal tests synchronize on complete Level
2 frames. The focused fixture and spawn-guard suites also pass in this review.

The required release evidence is incomplete, however. No CI run contains the
candidate, native Windows has not compiled or run its candidate-only paths, and
the saved before/after timings cannot demonstrate that the fix made the suite
faster. The fixture's fluent CWD escape also falls short of the specified
ownership-by-construction contract.

## Findings

### High — No candidate CI or native-Windows verification exists

The acceptance contract requires candidate evidence for the configured macOS,
Ubuntu, native-Windows, and WSL2 environments, including three consecutive
candidate runs and native-Windows compile/runtime coverage. The results file
instead records that the branch containing the implementation was unpushed and
that the newest CI run did not contain the candidate
([`results.md:162`](./results.md#L162)). AC6 and AC8 consequently remain
explicitly pending ([`results.md:178`](./results.md#L178)).

This is especially material for the new process fixture. Its Windows command
adapter, case-insensitive environment behavior, `cmd.exe /D /C set` recorder,
`SystemRoot`/`COMSPEC`/`PATHEXT` restoration, and Windows-only software lookup
paths cannot be exercised by macOS or WSL. A green baseline run from before the
implementation is not regression evidence for those paths.

Run the committed candidate through all four declared environments, retain
three compatible consecutive candidate runs, and make the native-Windows L1
job authoritative for the Windows fixture branch. Production readiness cannot
be inferred until those required runs pass.

Strongest verification present: Level 1 on macOS for the candidate and
pre-candidate Level 1 CI on the other environments. The required candidate
Level 1 native-Windows/CI evidence is absent, so this is a verification gap.

DECISION: this will not be used to determine production readiness; we can 
run tests locally to ensure no breakage where we feel there is OS risk:

- {{ env.BUILD_LINUX }} - is the host for testing on Linux
- {{ env.BUILD_WIN }} - is the host for testing on Windows
- {{ env.BUILD_WSL }} - is the host for testing on WSL



### High — The recorded comparison does not prove the faster-tests outcome

The only five-run alternating comparison reports the candidate full L1 suite
at 44.80 seconds versus 21.53 seconds for baseline, and candidate sanity at
17.03 seconds versus 7.94 seconds. Every paired ratio regressed, by roughly
2.0–2.3 times. The report correctly refuses a causal conclusion because the
candidate ran under extreme load and against a heavily dirty shared worktree
([`results.md:80`](./results.md#L80)). Its provenance also records the candidate
with the same base SHA as baseline plus a very large set of uncommitted changes
([`provenance.json:24`](./measurement/local-phase8/provenance.json#L24)).

Later one-off warm runs show `sanity` within 15 seconds and L1 at 23.855
seconds, but they are not the specified alternating, compatible before/after
sample. The changed CLI cohort's apparent 18.1% reduction is inside measured
drift, while per-family CI budgets remain pending. Work counters prove that
particular incidental Git work was removed; they do not establish the full
suite timing outcome.

Repeat the alternating measurement from clean, pinned baseline and candidate
trees under compatible load, then complete the matched CI samples and ratify
the per-family timing budgets. Until that evidence exists, the primary outcome
of this fix has not been demonstrated.

Strongest verification present: Level 1 work-counter tests prove selected work
elimination, but the required performance verification is inconclusive and the
only controlled timing table points in the wrong direction.

### Medium — The fluent CWD escape does not enforce fixture ownership or justification

`SniffCommandBuilder::ambient_context` correctly requires a directory inside
its `SniffCliFixture`, but the commonly used fluent
`OwnedSniffCommand::ambient_context` and `DisposableAmbientContext`
implementations only reject paths inside the checkout
([`common/mod.rs:223`](../../cli/tests/common/mod.rs#L223)). Any existing path
elsewhere—including a developer-owned repository or home subdirectory—is
accepted. Dozens of call sites then use this escape without the required
comment naming the repository/tool or proof, for example
[`cli.rs:170`](../../cli/tests/cli.rs#L170) and
[`cli.rs:273`](../../cli/tests/cli.rs#L273).

That means deterministic input ownership is a convention rather than the
specified structural guarantee. The fixture tests prove that the builder form
rejects an external temporary directory, but there is no corresponding
negative test for the fluent form; the live spawn guard checks binary
construction only and does not validate CWD ownership or escape reasons.

Keep repository directories under the same owned fixture, or let the owned
command retain explicit temporary-directory ownership. Add a negative test for
an arbitrary outside path and extend the structural guard to require a
non-empty, call-site-specific reason for each PATH/context escape.

Strongest verification present: Level 1 containment tests cover checkout and
builder-workspace boundaries, but not the fluent escape that most repository
CLI tests use.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| CLI output, JSON shape, exit status, repository projection, and deterministic environment policy | Level 1 real-binary integration tests | Appropriate level and passing locally; candidate Windows coverage is missing. |
| Requested work, observation reuse, and worker counter propagation | Level 1 in-process counter tests | Appropriate level; proves selected work removal, not end-to-end speed. |
| PTY process wiring and bounded EOF for `sniff os` | Level 1 manufactured PTY | Appropriate because no terminal-emulator rendering or input-encoder behavior is promised. |
| CI/CD and Git status glyphs, SGR styles, links, and layout | Level 2 tmux pane capture | Appropriate and passing through the canonical `just test-l2` route. |
| Keyboard, hotkey, paste, IME, or mouse behavior | No such requirement | Level 3 is not applicable. |
| macOS/Linux/Windows/WSL native detector and fixture behavior | Candidate Level 1 only on macOS | Gap: candidate CI and native-Windows execution are required. |
| Faster full L1 and sanity cohorts within ratified budgets | Level 1 timing artifacts | Gap: the compatible comparison is confounded/regressed and CI budgets are pending. |

## Validation Performed

- `cargo test --color=never -p sniff-cli --test cli_process_fixture --test spawn_site_guard`
  passed: 13 tests, 0 failed.
- Reviewed the six-selection inventory/reconciliation artifacts, requirement
  mapping, local and CI measurement records, work-count comparison, and final
  gate ledger.
- Existing local evidence records passing `just sanity`, `just test`, `just
  check`, `just lint`, `just doctest`, required-tmux `just test-l2`, and the
  Sniff leak sweep; those results do not replace the missing candidate CI or
  clean matched performance comparison.

## Production Readiness

Not ready. The implementation is locally functional, and its terminal
requirements use the correct verification levels, but the cross-platform
candidate evidence and the core faster-tests performance proof remain open.
