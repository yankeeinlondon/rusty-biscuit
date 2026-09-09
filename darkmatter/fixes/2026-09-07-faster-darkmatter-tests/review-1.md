---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T21:11:06-07:00
spec: 2026-09-07-faster-darkmatter-tests/spec.md
implemented: false
description: A **fix** review of `2026-09-07-faster-darkmatter-tests/spec.md`
fix: 2026-09-07-faster-darkmatter-tests/review-1.md
---

# Review 1: Faster Darkmatter Tests

## Verdict

Not ready for production.

The deterministic CLI fixture, test-population reconciliation, passive-path
instrumentation, HTTP ownership, and L1/L2/browser routing are substantial and
the focused fixture/resource tests pass. Production readiness is nevertheless
blocked by missing candidate CI/budget evidence, unexecuted Level-3 behavior,
an incomplete DMLS server-thread cleanup contract, and the absence of an
isolated candidate change set.

## Findings

### High — Required CI comparison and ratified budgets do not exist

The specification requires three consecutive candidate runs for every
configured CI package/environment leg and requires `results.md` to report
ratified family budgets and comparable evidence (`spec.md:167-173`,
`spec.md:193-196`). The implementation records only one baseline sample per
leg, no candidate CI source or candidate result, no matched-identity
comparison, and no ratified budget (`results.md:20`, `results.md:30-44`,
`results.md:68-81`). The plan correspondingly leaves the comparison,
reconciliation, budget, and AC7 tasks unchecked (`plan.md:1536-1544`,
`plan.md:1611-1613`).

This is correctly disclosed as pending, but pending mandatory evidence cannot
support a production-ready verdict. Produce a stable candidate revision,
collect the required consecutive baseline and candidate samples on Ubuntu,
macOS, Windows, and WSL2, reconcile every configured cell, ratify the budgets,
and record the matched-identity results without changing the limits after the
candidate is observed.

**DECISION:** this bar was set to high and should not be considered a blocker being production ready

### High — Level-3 keyboard, pointer, and image-paint behavior was not executed

The implementation changes the synchronization used by the Level-3 popover
keyboard/pointer tests and the real-terminal image-paint test. Those tests make
user-observable claims about OS Tab/Enter delivery, pointer hover, and painted
pixels. Their appropriate verification is Level 3 because they depend on OS
input injection or OS screen capture. The only candidate evidence is a compile
and clippy check; `just test-l3` refused before exercising any behavior
(`results.md:143-144`, `log.md:758-763`, `log.md:799-801`).

Under the review's rigor rule, compile reachability is not Level-3 behavioral
evidence. Run the three popover interactions and image-paint test with the
explicit Level-3 focus authorization on a suitable macOS host, record skips as
unavailable rather than passes, and retain a genuine pass for each claimed
behavior before production readiness.

**CRITICAL:** There needs to be a CLEAR explanation why the synchronization was changed. It may be for good reasons but it needs to be explained!

### High — DMLS server-thread cleanup is neither complete nor bounded

The specification requires DMLS sessions to own bounded cleanup on success,
failure, and cancellation (`spec.md:142-145`). Only the large
`lsp_session.rs` fixture retains a `JoinHandle`; its `Drop` implementation calls
`join()` without a deadline and panics if the server thread panicked
(`dmls/tests/lsp_session.rs:259-271`). If an assertion is already unwinding,
that second panic can abort the test process, and a stuck server makes cleanup
unbounded.

Two other in-memory fixture copies still discard the handle returned by
`thread::spawn` entirely (`dmls/tests/suggest_constraint_phase1.rs:20-42`,
`dmls/tests/no_side_effects.rs:18-42`). Their normal shutdown waits for an
outcome, but an assertion failure drops a detached thread without joining it.
The added unwind regression covers the native `ChildGuard` subprocess, not any
of these in-memory server fixtures. Consequently the claims that server
threads are reaped on unwind/cancellation and that AC4 is verified
(`results.md:93`, `results.md:173`) overstate the evidence.

Use one shared owned in-memory fixture for all three targets, make teardown
non-panicking during unwind, impose a documented completion bound, and add a
failure-path regression that proves the server worker has finished before the
fixture workspace is released.

### High — The reviewed tree is not an isolated candidate

The closeout says this implementation is confined to tests, test-only
instrumentation, fixtures, tier routes, and audit tooling
(`results.md:22-24`). The current worktree instead has 1,306 modified tracked
files plus 264 untracked files; its tracked diff spans 1,310 files and roughly
60,000 insertions / 50,000 deletions. GitNexus change detection reports 8,067
changed symbols, 119 affected execution flows, 1,306 changed files, and
CRITICAL aggregate risk. The changes include production Darkmatter, Sniff,
rendering, schema, and unrelated package code.

Some of that state may belong to concurrent or sibling work, but there is no
staged or committed candidate that identifies which exact bytes this fix owns.
That prevents the local evidence from being tied to a reproducible production
artifact and prevents a reviewer from validating the stated scope. Isolate the
fix into a reviewable candidate, run change detection on that candidate, and
reconcile any difference from the source state used for the recorded gates.

### Medium — The isolation guard permits most protected environment defaults to be undone

The fixture protects home, config, cache, Git plumbing, rendering, and
application variables, but the structural isolation detector only flags
post-build `current_dir`, `PATH`, and `env_clear` calls
(`cli/tests/spawn_site_guard.rs:319-350`). Its negative tests explicitly assert
that overriding `HOME` or `COLUMNS`, and removing `HOMEDRIVE`, are ignored
(`cli/tests/spawn_site_guard.rs:986-988`). An accidental
`.env("HOME", host_home)` or `.env("GIT_DIR", checkout)` can therefore restore
the contamination that the fixture is meant to prevent while both structural
gates remain green.

Retain intentional test-specific environment claims, but route them through a
named builder override or reconcile protected-key overrides with a narrow,
reasoned allowlist. Add negative tests for home/cache, Git plumbing, rendering,
and Darkmatter application namespaces.

## Requirement Verification and Test Rigor

| Requirement family | Strongest verification present | Assessment |
|---|---:|---|
| Complete inventory and runner reconciliation | Level 1/static: source scan, eleven Nextest captures, family reconciler, and malformed-input gates | Appropriate for inventory behavior; local reconciliation is recorded. |
| Deterministic `md` and `zed-dmls` launch isolation | Level 1: real child processes under hostile inherited CWD/home/cache/Git/PATH/rendering inputs; focused fixture/guard run passed 35/35 | Appropriate boundary, subject to the guard gap above. |
| Passive schema, trigger, completion, hover, and no-effect behavior | Level 1: in-process output assertions plus effect/network counters and sentinels | Appropriate; no terminal or OS input is involved. |
| HTTP ownership, caching, consent, refresh, and fallback | Level 1: loopback server with exact request/output assertions and bounded missing-request teardown | Appropriate for the network/process contract. |
| DMLS process/protocol ownership | Level 1: real stdio child and in-memory protocol tests; focused run passed 23/23 | Normal-path semantics pass, but failure/cancellation ownership is incomplete as found above. |
| Terminal glyph/style/layout behavior | Level 2: recorded real-terminal runs for 18 library, 69 CLI, and 3 DMLS tests | Appropriate; terminal input encoding is not part of these claims. |
| Browser computed layout/style | Browser tier: 43 recorded headless-browser tests | Appropriate; protocol-driven browser behavior does not require Level 3. |
| OS keyboard/pointer popover behavior and real image painting | Compile/clippy only; Level-3 runtime absent | Insufficient. These user-observable claims require Level 3 and are a high-severity gap. |
| Cross-platform performance acceptance | One CI baseline per leg and local alternating runs; no candidate CI comparison | Incomplete; the required hosted evidence and budgets remain pending. |

## Validation Performed

```text
cargo nextest run -p darkmatter-cli --test md_process_fixture --test spawn_site_guard
  35 passed, 0 skipped

cargo nextest run -p dmls --features effects-instrumentation \
  --test stdio_subprocess --test no_side_effects --test suggest_constraint_phase1
  23 passed, 0 skipped

sniff repo package-areas --json
sniff repo packages --json
sniff repo package-dependencies --json

GitNexus detect_changes(scope=all)
  CRITICAL: 8,067 changed symbols, 119 affected flows, 1,306 changed files
```

These focused passes validate the current normal paths; they do not close the
failure-path, Level-3, CI-performance, or candidate-isolation findings.
