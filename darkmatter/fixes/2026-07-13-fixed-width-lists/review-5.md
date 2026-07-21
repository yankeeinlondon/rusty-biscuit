---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T21:51:43-07:00
spec: 2026-07-13-fixed-width-lists/spec.md
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
implemented: true
implemented_by: codex/default
description: "A **fix** review of `2026-07-13-fixed-width-lists/spec.md`"
fix: 2026-07-13-fixed-width-lists/review-5.md
previous: 2026-07-13-fixed-width-lists/review-4.md
---

# Review 5 — Fixed-Width Lists

## Verdict

This fix is **not ready for production**. Review 4's quoted-code, quoted-additional-paragraph,
raw-HTML marker-theft, configured-eight-column, and ten-digit-marker reproducers are fixed, and
focused Level-1 verification is green across the library, compose, CLI, and DMLS. Two blocking
requirements remain. Full cleanup still parses marker-looking content inside an opaque
`::shell-block` as Markdown, changing literal markers and block layout; the new protected-body
tests use only the serializer's canonical `*` marker and therefore lock in the wrong output. The
mandatory Criterion B1/B2/B3 timing evidence also remains explicitly deferred.

## Findings

### High — Opaque shell-block bodies are still rewritten as Markdown lists

The new source-range overlay prevents a marker-looking shell line from consuming the authored
marker belonging to a later real list item, but it does not keep the opaque body out of the generic
pulldown-cmark serialization path. `cleanup_content_internal` uses the opaque ranges only while
extracting marker and item side channels (`cleanup/mod.rs:312-318`); the original parser events,
including list events created from shell payload bytes, still flow through cmark serialization
(`cleanup/mod.rs:329-355`).

A fresh spawned-CLI reproduction is:

```markdown
::shell-block
- first literal
+ second literal
::end-block

* Actual.
```

`md clean -` emits:

```markdown
::shell-block

* first literal

* second literal
  ::end-block
* Actual.
```

Both literal shell markers are normalized, blank lines are synthesized inside the opaque body,
the closer is indented as list content, and the separator before the real list disappears. The
same defect applies to a task-looking shell line: `- [ ] literal task` becomes
`* [ ] literal task`.

The new library oracle explicitly expects a corrupted shell layout
(`cleanup/tests/reflow.rs:1480-1482`), and the CLI oracle repeats it
(`cli/tests/clean.rs:285-286`). Using `* literal shell` as the fixture makes marker preservation
vacuous because `*` is already the serializer's canonical unordered marker. Compose and DMLS then
compare against the same incorrect direct-library output, so cross-surface byte parity does not
verify the specification. The pulldown-cmark structural fingerprint also has no Darkmatter
directive-body ownership state and cannot detect this rewrite.

This violates the opaque-body contract in Design Decision 1 and AC 9, 10, 12, and 14. It also
leaves the protected-child portion of AC 5 incomplete.

**Suggested resolution:** protect complete `::shell-block` body ranges across event serialization,
not only marker restoration. A source placeholder/splice or equivalent event-level representation
must preserve payload bytes and prevent payload markers from becoming Markdown items while keeping
the opener and closer structural. Add exact-output and Darkmatter-aware structural Level-1 tests
for `-`, `+`, `*`, ordered, and task-looking payload lines; cover quoted and unquoted shell blocks,
multiple payload lines, a following differently marked real list, fixed width, compose, CLI
stdout/save, DMLS, and a second cleanup pass.

### High — Required performance timing evidence remains deferred

The deterministic half of AC 15 passes: the focused parse-count selection confirms one parse for
default cleanup and each indent/spacing variant, one parse for standalone reflow, and exactly two
for the full cleanup-plus-fixed-width sequence. The Criterion harness also exists. There are still
no same-host baseline/candidate medians, no baseline-drift calculation, and no B1, B2, or B3
verdicts. `deferred-performance-tests.md` continues to state that the normative measurement was
not run.

The host remains inadmissible for an honest measurement. At review time it had eight logged-in
users, another active agent session, and load averages of `7.14 15.62 32.81`, above the documented
one-minute ceiling of 2.0. No timing sample was taken. A green benchmark smoke run would prove only
harness execution, not any of the specification's 10%, 15%, or 2x budgets.

**Suggested resolution:** on a quiet admissible host, run the documented baseline → candidate →
baseline bracket with a fresh shared target directory. Record every median, enforce the 3%
baseline-drift guard, and report each B1/B2/B3 case independently. AC 15 remains failed until the
complete vector passes.

### Medium — The complete Level-2 gate is not recorded for the reviewed HEAD

The implementation evidence for the Review 4 remediation runs exhaustive Level 1 and lint but
declares Level 2 unnecessary for each deterministic cleanup finding. That is correct for proving
the cleanup behavior, but AC 16 separately requires the complete package-area Level-2 gate. The
91/91 retained green result in Review 4 predates the two reviewed remediation commits.

A fresh `just test-l2` run passed all 19 library tests and the first 30 CLI tests before the
non-interactive 60-second command ceiling required an interrupt; 39 CLI tests and the three DMLS
tests were not reached in that invocation. This is not a product failure, but it is incomplete
current-HEAD acceptance evidence.

**Suggested resolution:** run the sanctioned broker-owning Level-2 recipe on the reviewed HEAD,
partitioning by integration binary if necessary to stay within execution limits, and record one
gap-free 19/69/3 result.

## Requirement-to-Verification Assessment

The fixed-width-list behavior is deterministic Markdown source transformation. Level 1 is the
appropriate behavioral tier for AC 1–15; no requirement involves a terminal emulator's rendering
or input encoder, so Level 3 is not applicable. AC 16 independently mandates the area Level-2 gate.

| AC | Requirement | Strongest verification | Assessment |
| --- | --- | --- | --- |
| 1 | Collapse list-prose soft breaks at all nesting and quote depths | Level 1 exact library, compose, spawned-CLI, and DMLS tests | Pass for the required list-prose matrix. |
| 2 | Remove continuation layout and retain only the Unicode join separator | Level 1 exact ASCII/Unicode tests | Pass. |
| 3 | Strip mode emits no synthesized hanging whitespace | Level 1 exact output and fixed-point tests | Pass. |
| 4 | Preserve mode performs no list collapse or synthesis | Level 1 library/CLI/compose and second-pass tests | Pass. |
| 5 | Fixed width unwraps logical prose while protecting child blocks | Level 1 exact output and fingerprints | **Fail:** opaque shell payloads are serialized as lists before reflow. |
| 6 | Created lines carry complete aligned container prefixes | Level 1 exact nested/quote/task output and width checks | Pass. |
| 7 | Per-item digit/task/configured-indent/quote prefixes | Level 1 exact indent-2/4/8 matrix and parser fingerprints | Pass, including exact eight-column narrow-marker nesting. |
| 8 | Total display width respects only documented overflow exceptions | Level 1 display-width assertions | Pass for every represented non-opaque prose fixture. |
| 9 | Paragraph, item, child-block, and protected-block structure remains intact | Level 1 exact output and structural fingerprints | **Fail:** shell-block payload layout and marker spelling change. |
| 10 | Structural fingerprints preserve list and protected ownership | Level 1 pulldown-cmark fingerprint | **Fail:** it does not model Darkmatter directive ownership and accepts the shell rewrite. |
| 11 | Normal, compact, and loose spacing retain compatibility | Level 1 exact mode matrix | Pass, including the nine/ten-digit boundary. |
| 12 | Equivalent library, compose, CLI stdout/save, and DMLS sequences agree | Level 1 cross-surface tests | **Fail:** protected-body surfaces agree with the same incorrect library oracle. |
| 13 | Default, preserve, and fixed-width cleanup are idempotent | Level 1 second-pass tests | Pass for the represented canonical outputs. |
| 14 | Public API, CLI schema, marker rules, dependencies, and platform behavior remain stable | Source inspection plus Level 1 CLI/API tests | **Fail:** literal unordered markers inside opaque shell payloads are normalized. |
| 15 | Parse and timing budgets pass | Level 1 parse counters; Criterion timing absent | **Fail:** parse counts pass, but B1/B2/B3 have no measurements. |
| 16 | Build/L1/L2/lint and bounded impact gates pass | Exhaustive retained L1/lint, fresh focused L1, partial fresh L2, GitNexus | **Incomplete:** the full current-HEAD L2 selection is not recorded. |

## Verification Performed

- Read the complete specification, Reviews 1–4, implementation log, deferred-performance record,
  remediation commits, affected production code, and added tests.
- `sniff` identifies the affected scope as `darkmatter`, `darkmatter-cli`, and `dmls` in the
  `darkmatter` / `darkmatter/dmls` package areas.
- GitNexus rates `fix_list_indentation` and `extract_indented_code_markers` **CRITICAL** at 154
  upstream dependents each, and `cleanup_content_internal` **CRITICAL** at 171 upstream dependents.
  Review-cycle change detection reports the expected compose inline-post cleanup flows; unrelated
  pre-existing documentation edits in the dirty worktree were left untouched.
- Fresh focused Level 1 passed: 20/20 library/compose tests, 26/26 spawned `clean` tests, and 12/12
  DMLS formatting tests. One unrelated CLI file test passed on its configured second attempt after
  Nextest reported a leaked handle on the first attempt.
- Fresh CLI reproductions confirmed the opaque-shell marker and layout rewrite for unordered and
  task-looking payload lines.
- Fresh `just test-l2` passed 19/19 library and 30/30 started CLI tests before the command-duration
  ceiling; the incomplete aggregate was interrupted cleanly. Review 4 retains a prior 91/91 area
  result, but it is not current-HEAD evidence for AC 16.
- The implementation record reports exhaustive Level 1 and lint passing for all three affected
  packages. No write-mode formatter was run.
- No Criterion timing was run because the host failed the documented admissibility conditions.

This review ran on macOS. The cleanup implementation is pure Rust and has no new platform branch,
but Windows and Linux were not executed during this review.

## Production Readiness

**Not ready.** Preserve opaque `::shell-block` payload bytes through full cleanup, replace the
vacuous protected-body oracles with specification-level tests, produce a complete passing
Criterion timing vector, and record the full current-HEAD Level-2 gate before setting `ready: true`.
