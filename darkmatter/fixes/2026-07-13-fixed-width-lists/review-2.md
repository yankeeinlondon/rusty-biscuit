---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T12:57:50-07:00
spec: 2026-07-13-fixed-width-lists/spec.md
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
implemented: true
implemented_by: codex/default
description: "A **fix** review of `2026-07-13-fixed-width-lists/spec.md`"
fix: 2026-07-13-fixed-width-lists/review-2.md
previous: 2026-07-13-fixed-width-lists/review-1.md
next: 2026-07-13-fixed-width-lists/review-3.md
---

# Review 2 — Fixed-Width Lists

## Verdict

This fix is **not ready for production**. Review 1's reference-definition corruption and
ten-digit-marker defects are fixed, the structural fingerprint now records ordered-list starts and
task state, and the focused Level-1 suites pass. However, the required configured eight-space
nesting path still destroys valid nested-list structure, the mandatory performance budgets remain
unmeasured, and preserve-mode idempotence still has no durable regression test.

## Findings

### High — Configured eight-space indentation flattens nested lists

The specification requires configured 2-, 4-, and 8-space nesting coverage (`spec.md:572`) and
requires nested-list boundaries and semantic structure to survive cleanup (AC 7, 9, 10, and 13).
The implementation does not satisfy the eight-space case. `fix_list_indentation` infers nesting as
`current_indent / 2` from the serialized line's absolute column
(`cleanup/lists.rs:278-295`), even though that column includes marker-width effects and is not a
semantic nesting depth.

A freshly built CLI reproduces the user-visible corruption:

```text
input:
- Parent alpha beta gamma delta.
  - Child alpha beta gamma delta epsilon.

md clean --indent 8 --fixed-width 24 - output:
- Parent alpha beta
  gamma delta. - Child
  alpha beta gamma delta
  epsilon.
```

The child marker is absorbed into the parent's prose. The non-fixed-width path first emits the
child at eight columns, but cleaning that output again also collapses the child into the parent, so
default cleanup is not idempotent for this accepted CLI mode. A wide ordered parent exposes the
same depth-calculation defect with `--indent 4`: the first cleanup changes a six-column child indent
to twelve columns, and the next cleanup flattens it.

The nominal configured-indentation test contains only 2 and 4
(`cleanup/tests/reflow.rs:736-756`). The added eight-column test explicitly bypasses
`--indent 8`, uses `cleanup_content_with_indent(..., 2)` under a six-digit parent, and tests only
that `reflow_to_width` consumes indentation already present
(`cleanup/tests/reflow.rs:954-1002`). That is useful reflow coverage, but it is not the configured
eight-space requirement and cannot detect the CLI failure above.

There is also a contract conflict to resolve: eight spaces under a narrow unordered marker are not
a portable CommonMark representation of a nested child. Silently accepting `--indent 8` while
changing the parse tree is not an acceptable resolution.

**Suggested resolution:** decide the public contract first. Either remove/reject the unrepresentable
eight-space mode for affected marker shapes, or redesign indentation normalization around parsed
list depth and a CommonMark-valid serialization strategy. Then add exact-output and structural-
fingerprint Level-1 tests through `cleanup_content_with_indent`, `md clean --indent 8`, fixed-width
cleanup, and a second cleanup pass. Do not substitute an already-eight-column source fixture for
the configured-mode test.

### High — Mandatory timing budgets still have no verdict

The specification requires same-host pre-fix/candidate Criterion evidence: default cleanup within
10%, fixed-width list cleanup within 15%, and fixed-width cleanup below 2x full cleanup
(`spec.md:632-644`, AC 15). The remediation added a well-targeted `clean_list_budgets` benchmark
and deterministic parse-count tests, but `deferred-performance-tests.md` explicitly records the
timing measurement as deferred. The current host remains inadmissible: review-time load averages
were `72.74 58.53 77.91` with eight users, far above the artifact's own load ceiling of 2.0.

The parse-count evidence is valuable and passes: default cleanup parses once, standalone strip and
reflow parse once each, and both fixed-width orchestration paths parse twice. It proves the
structural half of AC 15; it does not prove any of the three normative timing budgets.

**Suggested resolution:** on a quiet host, run the documented baseline → candidate → baseline
bracket with the same Criterion profile and shared target data. Record pre/post load, both baseline
drift, every per-fixture median, and explicit pass/fail arithmetic for B1, B2, and B3. Until every
required case passes, keep `ready: false`.

### Medium — Preserve-mode idempotence is not retained as a regression

AC 13 requires default, preserve, and fixed-width cleanup to be idempotent (`spec.md:707`). The
suite has a fixed-width fixed-point test and DMLS has a generic canonical-document idempotence test,
while CLI/compose preserve tests assert only one invocation. A repository search finds no dedicated
test that runs list-prose cleanup twice in preserve mode. The implementation log records a manual
repeated invocation, but that is not a durable guard.

**Suggested resolution:** add a list fixture with authored soft breaks and run the preserve library
sequence twice, asserting exact byte equality after the first pass. Add the equivalent spawned-CLI
test, ideally including `--save`, so future serializer or indentation changes cannot regress AC 13
without failing Level 1.

## Requirement-to-Verification Assessment

Fixed-width list cleanup is deterministic Markdown source transformation. Level 1 is the correct
behavioral level for AC 1–15; no requirement depends on terminal-emulator rendering or OS input.
AC 16 separately requires the package area's Level-2 gate, and the retained implementation run is
green. No Level-3 test is applicable.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | Collapse eligible list soft breaks at all nesting/quote depths | Level 1 exact library and spawned-CLI tests | Pass for represented structures. |
| 2 | Remove continuation layout and use the Unicode join policy | Level 1 exact ASCII and Unicode matrix | Pass. |
| 3 | Strip mode emits no hanging whitespace | Level 1 exact strings and a fixed-point assertion | Pass. |
| 4 | Preserve mode performs no collapse or synthesis | Level 1 library, compose, and spawned-CLI single-pass tests | Pass behavior; fixed-point coverage is missing. |
| 5 | Fixed width unwraps the complete logical list paragraph | Level 1 exact library/CLI/DMLS tests | Pass. |
| 6 | Created lines use complete aligned container prefixes | Level 1 exact nested/blockquote/task output plus width checks | Pass for valid covered inputs. |
| 7 | Per-item marker/task/nesting/quote prefixes | Level 1 exact matrix | **Fail:** configured eight-space nesting is bypassed and corrupts CLI output. |
| 8 | Total display width obeys documented overflow exceptions | Level 1 Unicode-width and indivisible hard-break-suffix tests | Pass. |
| 9 | Paragraph/item/child-block structure remains intact | Level 1 exact output and fingerprint helper | **Fail:** `--indent 8` absorbs a nested child into parent prose. |
| 10 | Fingerprint preserves required list semantics | Level 1 fingerprint with ordinal and task state | Partial: helper is improved, but it is not applied to the failing configured-eight-space path. |
| 11 | Normal/compact/loose spacing remains compatible | Level 1 mode matrix | Pass for covered list forms. |
| 12 | Equivalent library, compose, CLI stdout/save, and DMLS sequences agree | Level 1 cross-surface parity tests | Pass for representative fixtures; parity does not repair the configured-indent corruption. |
| 13 | Default, preserve, and fixed-width cleanup are idempotent | Level 1 fixed-width and DMLS tests plus manual preserve evidence | **Fail:** configured indentation is not idempotent; durable preserve coverage is absent. |
| 14 | Public API, CLI schema, dependencies, and platform behavior remain stable | Source/API inspection plus Level 1 CLI tests | Pass for the review-1 remediation. |
| 15 | Parse and timing budgets are satisfied | Level 1 exact parse counts; Criterion harness smoke test | **Fail:** required timing comparisons were deferred. |
| 16 | Area build/L1/L2/lint and bounded impact gates pass | Retained `just` gates plus GitNexus | Gate evidence passes, but AC 7/9/13/15 remain open. |

## Verification Performed

- GitNexus upstream impact remains broad: `strip_incidental_newlines` is **CRITICAL** (39 direct,
  64 total indexed dependents); `reflow_to_width` is **CRITICAL** (20 direct, 51 total, reaching
  CLI and compose processes); `cleanup_parser` is **CRITICAL** (2 direct, 194 total); and
  `apply_strip_edits` is **HIGH** (1 direct, 65 total).
- `sniff` identifies the affected packages as `darkmatter`, `darkmatter-cli`, and `dmls` in the
  `darkmatter` / `darkmatter/dmls` package areas. It also identifies the expected workspace
  consumers of the public `darkmatter` crate.
- Fresh focused Level 1: **PASS**, 85/85 cleanup/reflow/parse-count tests.
- Fresh spawned-CLI Level 1: **PASS**, 19/19 `clean` integration tests.
- Fresh DMLS Level 1: **PASS**, 7/7 formatting tests.
- Retained implementation evidence at the reviewed HEAD records `just build`, full `just test`
  (5,884 darkmatter, 615 darkmatter-cli, 568 dmls), `just lint`, and `just test-l2` (91/91) all
  passing. The fresh review did not repeat the real-terminal gate because this feature has no
  terminal-rendering requirement and the same implementation already has a complete green area
  gate.
- The fresh focused Level-1 reparse test confirms that used link-reference definitions still
  resolve, and fresh CLI checks confirm that ten-digit numeric prose receives no hanging indent.
- Fresh CLI checks reproduce nested-list flattening through `--indent 8 --fixed-width 24` and
  non-idempotent indentation normalization under wide ordered markers.
- The Criterion harness exists and its smoke test is recorded as passing, but no admissible timing
  vector exists. Current host load is too high to create one honestly.
- Required frontmatter round-trips through `biscuit-file`. `md schema validate` remains blocked by
  existing schema-infrastructure drift: `schemas/feature-review.yaml` combines tagged-schema
  `kind`/`types` with unsupported `$schema` and `description` keys. `git diff --check` passes.

This review ran on macOS. The implementation is pure Rust and the generated whitespace/newline
policy is platform-neutral by inspection, but Windows and Linux were not freshly executed.

## Production Readiness

**Not ready.** Resolve the configured eight-space indentation contract without changing the parse
tree, add durable preserve-mode fixed-point coverage, and produce passing Criterion evidence for
all required budgets before setting `ready: true`.
