---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T00:23:09-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: true
next: 2026-07-16-redundant-walk/review-2.md
implemented_by: opencode/kimi-for-coding/k3
log: darkmatter/fixes/2026-07-16-redundant-walk/log.md
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-1.md
---

# Review 1 — Redundant Walk

## Verdict

This fix is **not ready for production** under its current specification. The
implementation correctly separates trusted-fresh validation from checked
prebuilt validation, preserves the public fail-closed path, and passes every
scoped build, Level-1 test, lint, whitespace, and change-impact gate. However,
acceptance criterion 8 is explicitly unmet: the required
`multi_transclusion/build_and_validate` improvement was at least 10% and at
least 500 microseconds, while the best recorded baseline-to-candidate result was
4.4% and 461 microseconds.

The benchmark evidence is useful because it falsifies the specification's
performance premise. The removed descendant walk is real, but the same-run
decomposition estimates it at about 159 microseconds (1.5% of the full path),
not the roughly 4.15 ms attributed to it. That does not invalidate the
behavioral fix, which removes redundant I/O and restores coherent snapshot
semantics, but a mandatory acceptance threshold cannot be waived by review.

## Findings

### High — The mandatory performance acceptance threshold is not met

The specification requires `multi_transclusion/build_and_validate` to improve
by **both** at least 10% and at least 500 microseconds at the median
(`spec.md:265`). The recorded quiet-window comparison improved from 10.527 ms
to 10.066 ms: 461 microseconds, or 4.4% (`results.md:138-142`). It therefore
misses both required thresholds. `results.md` correctly records this as a
failure (`results.md:169-175`), and its candidate-only decomposition estimates
the removed walk at approximately 159 microseconds / 1.5%
(`results.md:152-167`).

This is release-blocking because acceptance criterion 8 requires both the
evidence and a satisfying result. The regression guard passes, and
`validate_prebuilt` remains materially faster than `build_and_validate`, but
those two passes do not replace the failed positive-improvement threshold.

**Suggested resolution:** amend the specification before another implementation
or review cycle. If the product goal is eliminating redundant I/O and the
hard-error race, replace the disproven 10%/500-microsecond threshold with a
mechanism-based requirement plus a benchmark guard calibrated to the measured
effect. If a large measurable speedup is itself the goal, expand the benchmark
fixture or optimize the shared validation engine in a separately scoped change;
the current fix cannot reach the existing bar without violating its non-goals.

## Implementation Assessment

No additional correctness defect was found.

- `validate` builds once and immediately calls the internal fresh seam
  (`validate.rs:334-343`).
- The checked prebuilt seam still calls `verify_graph_compatibility` before the
  shared engine and therefore before flattening (`validate.rs:357-372`).
- `validate_fresh_graph` has narrow `pub(super)` visibility and documents its
  trust preconditions (`validate.rs:375-399`).
- Both paths converge on one `validate_graph_contents` implementation
  (`validate.rs:401-416`).
- `FileTree::ensure_built` constructs the graph and routes the adjacent
  validation call through the same fresh seam with clone-stable graph options
  (`file_tree/mod.rs:232-250`).
- The changed-child mechanism test proves the fresh path validates the
  build-time snapshot while the checked path rejects the same stale graph as a
  changed dependency (`validate.rs:1503-1563`).

The design is ergonomic and difficult to misuse: separate named seams express
the trust decision more clearly than a Boolean verification flag, while the
narrow visibility prevents external callers from bypassing freshness checks.

## Requirement-to-Verification Assessment

This fix changes deterministic library/filesystem behavior. It introduces no
terminal rendering, terminal-emulator input, keybinding, paste, IME, mouse, or
OS keyboard behavior. Level 1 is therefore the appropriate verification level
for acceptance criteria 1–7 and 9; Level 2 and Level 3 are not applicable.
Criterion evidence is a performance gate rather than an L1/L2/L3 interaction
test.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One build; fresh validation skips compatibility and dependency verification | Level 1 source path + changed-child mechanism test | **Pass.** `validate` calls `validate_fresh_graph` directly. |
| 2 | `FileTree::ensure_built` uses the trusted fresh seam | Level 1 source path + file-tree integration tests | **Pass.** The just-built graph is passed directly to the internal seam. |
| 3 | Public prebuilt validation remains fail-closed before flattening | Level 1 stale/missing/unreadable/options/source/mode tests | **Pass.** Verification precedes the shared engine. |
| 4 | One shared post-verification engine | Level 1 source inspection + parity tests | **Pass.** Both seams call `validate_graph_contents`. |
| 5 | Changed child differentiates fresh and checked paths | Level 1 unit test | **Pass.** The new paired mechanism test passes. |
| 6 | Existing provenance, mismatch, parity, file-tree, and presentation behavior remains green | Level 1 unit/integration/spawned-CLI suites | **Pass.** The full area Level-1 gate passed. |
| 7 | Public signatures, errors, reports, CLI output, and graph serialization stay unchanged | Level 1 compile/API and existing CLI/serialization tests | **Pass.** No public signature/type or CLI implementation changed. |
| 8 | Same-session benchmark evidence satisfies improvement and regression thresholds | Criterion | **Fail.** Evidence exists, but the required positive-improvement threshold missed both bars. |
| 9 | Focused tests, area build/test/lint, whitespace, and change scope pass | Level 1 + tooling | **Pass.** All scoped gates are green. |

No user-observable requirement is verified below the level appropriate to its
behavior, so there is no L1/L2/L3 mismatch finding.

## Verification Performed

- `cargo nextest run -p darkmatter -E 'test(/fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph/)'`: **PASS** (1/1).
- `just build` from `darkmatter/`: **PASS** for `darkmatter`,
  `darkmatter-cli`, and `dmls`.
- `just test` from `darkmatter/`: **PASS** — 5,763 `darkmatter`, 559
  `darkmatter-cli`, and 566 `dmls` Level-1 tests (6,888 total), zero failures.
- `just lint` from `darkmatter/`: **PASS** for all three packages, with no
  warnings.
- `git diff --check`: **PASS**.
- `cargo fmt --all --check`: not runnable because this host's stable toolchain
  lacks the `rustfmt` component. No tool was installed; the canonical
  repository lint gate passed.
- `md schema validate` passes for the specification. Review-schema validation
  is blocked by existing schema infrastructure drift: the repository's
  `schemas/feature-review.yaml` is itself rejected as a standalone tagged
  schema because it combines unsupported `$schema` and `description` keys with
  `kind`/`types`. The required review frontmatter was read back directly.
- GitNexus upstream impact: `validate` is **HIGH** (17 impacted, 15 direct test
  callers); `validate_with_graph` is LOW; `FileTree::ensure_built` is MEDIUM
  (13 direct unit/integration callers). No indexed execution process crosses
  the reference subsystem. Worktree-scoped `detect_changes` reports the
  expected reference-validation/file-tree symbols and no affected process.
- `sniff` and Cargo metadata confirm the affected package area is Darkmatter
  (`darkmatter`, `darkmatter-cli`, and `dmls` in the workspace; `zed-dmls` is
  workspace-excluded). No public or cross-area effect required broader gates.

Cross-platform execution was not available in this macOS session. The change
uses only portable Rust control flow and existing filesystem abstractions; no
new platform-specific code was introduced.

## Production Readiness

**Not ready.** Resolve the specification/evidence mismatch and complete a new
review cycle before setting `ready: true`.
