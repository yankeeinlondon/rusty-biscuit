---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T15:47:24-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-5.md
previous: 2026-07-13-proxy-with/review-4.md
next: 2026-07-13-proxy-with/review-6.md
---

# Review 5: Proxy With

## Verdict

The feature is **not ready for production**. The current `claudine-cli` target
does not compile after the typed surfaced-handoff change. Beyond that immediate
release blocker, the new command-owned handoff path is not wired into composition
requests and its returned value is discarded, so terminal-event proxies still
use the harness-local coordinator and retain the incomplete launch plan identified
in review 4.

The initialize route has improved: live initialize proxies now return to the
command coordinator, receive fresh outer preparation, and can acquire a target
loop. The `proxy.with` parser, typed overlay evaluation, precedence, target
bootstrap, redaction, and the existing initialize-route fixtures remain
substantial implemented work. They do not close the command-wide equivalence
contract while the other proxy producers, retry/resume refresh, and sequence
containment retain separate orchestration paths.

## Findings

### 1. Critical: `claudine-cli` does not compile

`run_harness_loop` now takes a shared-ledger argument and returns a four-element
tuple carrying `SurfacedHandoff`, but the direct-wrapper call still supplies the
old argument list and destructures the old three-element tuple
(`cli/src/commands/wrap/wrapper_stages.rs:489-523`). A targeted
`just test-cli composition_seams` fails with `E0061` and `E0308` at that call.

The composition caller compiles far enough to expose a related warning:
`surfaced_handoff` is bound but unused (`composition/runner.rs:469`). This is not
an isolated test issue; the production binary target fails to build, so no CLI
or Level 2 release gate can run on the current tree.

Update every caller and tuple consumer together, then add a compile-time/corpus
guard covering every `run_harness_loop` call site. The direct wrapper should pass
`None` for a command ledger and explicitly reject an impossible surfaced
handoff, rather than silently ignoring it.

### 2. Critical: the shared terminal-handoff channel is both unreachable and dropped

The command creates the intended invocation-wide `SharedRunLedger`
(`compose/prep.rs:221-225`), but `build_execution_request` still hard-codes
`handoff_ledger: None` and even documents that it is “not yet threaded here”
(`compose/prep.rs:805-811`). Repository-wide search finds no production
construction with `handoff_ledger: Some(...)`. Consequently the `Some` arm in
`surface_or_adopt_terminal_proxy` is unreachable; terminal-event proxies always
take the `None` arm and call the harness-local coordinator's `adopt`
(`loop_control.rs:809-840`).

Even if the ledger were passed, `run_composition_body` receives the returned
`surfaced_handoff` and then sets `SingleCompositionOutcome::initialize_handoff`
to `None` (`composition/runner.rs:469-521`). The command coordinator would never
see the committed handoff. This directly contradicts the comments immediately
above both paths and leaves AC1-4 and the invocation-wide portion of AC16
unimplemented for terminal recovery and loop-terminal proxies.

The behavioral consequence remains review 4's launch/loop defect. In-harness
adoption can rebuild only `AGENT`/`MODEL`/`YOLO`; its own module states that
provider/profile/binary, argv, MCP injection, system prompt, and child CWD are
not rebuilt (`target_launch.rs:10-26`). A failure/success/finalize proxy to a
looping or differently configured target therefore remains observably different
from direct invocation. The sole terminal-proxy L2 fixture deliberately uses a
non-looping target (`level2_lifecycle_control.rs:882-973`), so it cannot catch
this.

Thread the single ledger through `execute_loop_or_single` and
`build_execution_request`, propagate the returned `SurfacedHandoff` into the
outcome, and add Level 2 terminal-event rows that exercise a looping target and
target-owned provider/MCP/system-prompt/argv/CWD changes. A structural test
should also require at least one production `Some(shared_ledger)` construction;
the current endpoint census proves types exist, not that the live branch is
reachable.

### 3. Critical: sequence proxies intentionally retain the reduced harness path

The new handoff helper explicitly keeps sequence proxies on the harness-local
coordinator (`loop_control.rs:775-840`), and sequence constructs its request with
`handoff_ledger: None` (`sequence/iterate.rs:174`). That satisfies containment
only in the narrow sense that a proxy does not advance or restart the sequence
step. It does not satisfy R1, R3, R6, or R7: the target is still another harness
attempt, cannot rebuild the complete launch bundle, and cannot acquire its own
document loop through the command coordinator.

The Level 2 sequence test uses a non-looping target under an explicitly pinned
`--goose` provider and asserts only that the target runs once per step
(`level2_lifecycle_control.rs:2829-2860`). It cannot verify target loop ownership,
provider/profile/binary selection, MCP, system prompt, argv, environment, child
CWD, structured mode, or dispatch configuration.

Model a sequence step as the command-owned coordinator scope required by R1:
handoffs remain inside the current step, while document identity and launch
planning still return above the provider harness. Add Level 2 direct/proxy
equivalence for a looping sequence target and at least one target-owned launch
facet.

### 4. High: initialize-handoff commit failures bypass required catch/finalize routing

Live initialize proxies now leave the pipeline as an uncommitted
`SurfacedHandoff::Request`. The source lifecycle guard is dropped, and the outer
command later calls `commit_proxy(...)?` (`compose/prep.rs:439-451`). A missing
target, cycle, or hop-limit rejection therefore returns directly from the command
instead of routing through the still-legal source `blocked`/`finalize` behavior
required by the atomic-handoff contract and AC29.

The Level 2 initialize-cycle test checks only that the target initialized and a
cycle/hop message appeared (`level2_lifecycle_control.rs:1440-1473`). It does not
give the active source `blocked` or `finalize` markers, so it cannot detect the
lost lifecycle routing. The Level 1 handoff-failure tests exercise the old
harness-local `route_handoff_failure` path, not this new outer commit point.

Commit while the source lifecycle routing capability is still available, or
return a typed commit outcome to a command-level error router that retains that
capability. Add Level 2 missing-target, cycle, and hop-limit cases that assert
the exact event order, `err.*` availability, no duplicate finalize, and no
target activation.

### 5. High: resume compatibility is derived from frozen and heuristic launch state

Retry/resume re-entry calls the canonical composition service, but complete
launch planning still lives outside that refresh. `session_compat_key` therefore
reads the frozen provider/profile/binary/CWD/argv/base environment plus the
small per-attempt environment overlay (`session_key.rs:35-76`), not a typed
fresh target launch plan. Document changes to MCP, system-prompt delivery,
provider/profile, argv, child CWD, structured mode, or provider-specific resume
identity cannot reliably enter the comparison because those facets were not
canonically rebuilt in the first place.

The fallback extraction is also lossy: MCP identity is explicitly “best-effort”
and the provider-specific `extra` map is always empty
(`session_key.rs:73-75,123-157`). System-prompt extraction attempts to read every
flag value as a file, including inline prompt values (`session_key.rs:98-115`),
so an inline value that happens to name an existing file is hashed as the file's
contents rather than as the literal delivered to the provider. The helper uses
`DefaultHasher` instead of the repository's `biscuit-hash` authority
(`session_key.rs:160-164`).

All incompatibility evidence is Level 1 projection/extraction testing. Level 2
has a compatible resume happy path, but no refresh mutates a non-renegotiable
facet and proves the provider is not resumed and the typed, facet-naming retry
diagnostic renders. This is a wrong-level gap for AC15.

Build the compatibility key directly from the final typed launch bundle used to
spawn the provider. Then add Level 2 refusal cases for each adapter's
non-renegotiable facets and assert both non-launch and the rendered diagnostic.

### 6. High: several user-observable requirements remain at the wrong test level

The current Level 2 matrix has useful initialize-route and redaction coverage,
but the following required behaviors still have only Level 1 evidence or no
representative test:

- AC6 inline proxy closure ownership: only an in-process ownership test; no real
  run verifies that only the final target file is rewritten.
- AC7/10 terminal-event and sequence proxy equivalence: no looping target or
  complete launch-facet comparison on either route.
- AC10 target launch equivalence: the matrix covers model environment, file
  resolution, and output channels, but not provider/profile/binary, MCP,
  system-prompt delivery, argv, effective child environment/CWD, interactivity,
  structured mode, or dispatch configuration.
- AC15 resume incompatibility: Level 1 only.
- AC17 approved bytes equal executed bytes: Level 1 only; Level 2 proves target
  audit/denial, not equality through execution.
- AC26 overlay retention across retry, resume, and loop refresh: Level 1 only;
  Level 2 covers only multi-hop forwarding and omission.
- AC29 initialize handoff-refusal routing: the Level 2 cycle test checks the
  diagnostic but not `blocked`/`finalize` behavior.

These are observable process selection, file mutation, lifecycle ordering,
shell execution, and iteration behaviors, so Level 1 state tests are not
sufficient. Level 3 is not required because none depends on the terminal
emulator's keyboard encoder.

There is also still no continuous Level 2 gate. The Claudine workflow runs only
the Level 1 area recipes on Ubuntu (`.github/workflows/claudine-tests.yml:35-113`)
and does not invoke the reusable Linux `test-l2` job. A prior local pass cannot
protect a typed-channel regression such as this iteration's unreachable and
dropped handoff.

### 7. Medium: sign-off artifacts and documentation contradict the code

`notes/acceptance-map.md` declares all 30 criteria mapped and AC3, AC10, AC15,
AC17, AC26, and AC29 complete, despite the unreachable handoff branch, incomplete
launch rebuild, and wrong-level evidence above. The `target_launch` module itself
correctly documents the remaining structural work, while the acceptance map
calls that work resolved. The active L2 source still contains “Why this is
`#[ignore]`d” and “both `#[ignore]`d” prose for enabled tests
(`level2_lifecycle_control.rs:2315-2322,2412-2443`).

The Claudine skill's composition topic also says the launch rebuild and resume
compatibility key are not implemented (`.claude/skills/claudine/composition.md:630-662`),
while the acceptance map claims both complete. The topic is closer to the real
limitations but stale about the initialize route that now works. Reconcile the
plan, acceptance map, topic docs, skill, and test comments only after the live
routes and required test levels are established.

## Verification-level audit

| User-observable requirement | Strongest present | Assessment |
|---|---:|---|
| `proxy.with` parsing, recursive interpolation, typed values, merge/null semantics | Level 1 | Appropriate for pure data semantics |
| Initialize proxy bootstrap, target initialize/reread, model projection, authored loop | Level 2 tests exist | Appropriate scenarios, but current CLI does not compile |
| Terminal-event proxy target launch and loop ownership | Level 2 non-looping smoke test | **Mismatch/incomplete:** no full launch or loop equivalence |
| Sequence-step containment | Level 2 non-looping smoke test | **Mismatch/incomplete:** containment only; no target launch/loop equivalence |
| Initialize handoff refusal and event-aware catch/finalize | Level 2 diagnostic-only | **Mismatch:** lifecycle order and `err.*` are unverified |
| Inline final-target closure | Level 1 | **Mismatch:** requires Level 2 file-output evidence |
| Resume compatible follow-up delivery | Level 2 | Appropriate for the happy path |
| Resume incompatibility and facet-naming diagnostic | Level 1 | **Mismatch:** requires Level 2 runtime/non-launch evidence |
| Approved bytes equal executed bytes | Level 1 | **Mismatch:** requires Level 2 end-to-end evidence |
| Overlay survival through retry/resume/loop | Level 1 | **Mismatch:** requires Level 2 lifecycle/provider observation |
| Overlay non-disclosure in rendered status | Level 2 pane capture | Appropriate |
| OS keyboard/input encoding | None | Not applicable; no Level 3 requirement |

## Validation performed

- `just test-cli composition_seams` — **failed to compile** `claudine-cli` with
  `E0061` and `E0308` at `wrapper_stages.rs:489`; it also reported the unused
  `surfaced_handoff` in `composition/runner.rs:469`.
- `just test` — catalog types passed 21 tests and the library passed 3,527 tests;
  the longer area run was stopped at the non-interactive duration guard while
  compiling the contract, before CLI/generator completion. The later targeted
  CLI command established the release-blocking compile failure.
- `just test-l2` and `just lint` were not run because the current CLI target
  cannot compile; neither can produce a meaningful green gate until finding 1
  is fixed.

The green library results validate the provider-neutral state and overlay
semantics they exercise. They do not validate the broken CLI wiring or the
missing real-terminal routes above.
