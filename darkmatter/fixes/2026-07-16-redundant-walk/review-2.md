---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-18T01:11:12-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: true
next: 2026-07-16-redundant-walk/review-3.md
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-2.md
previous: 2026-07-16-redundant-walk/review-1.md
---

# Review 2 — Redundant Walk

## Verdict

This fix is **ready for production** under the amended specification. Review 1's
sole High finding was a disproven performance threshold, not an implementation
defect. The specification now makes the observable mechanism the primary guard
and calibrates the benchmark floor to the measured cost of the removed walk.
The existing same-session evidence satisfies every amended performance guard.

The implementation keeps the trust decision explicit: internally just-built
graphs use a narrowly visible fresh seam, caller-supplied graphs retain the full
fail-closed compatibility check, and both paths converge on one validation
engine. No functionality, testing, ergonomics, or performance gap remains in
the reviewed scope.

## Findings

None.

## Review 1 Closure

Review 1 found that the original requirement of at least 10% and 500
microseconds improvement could not be met because it attributed the entire
`validate_prebuilt` floor to descendant re-verification. The same-session
decomposition instead measured that walk at approximately 159 microseconds,
or 1.5% of `build_and_validate`.

The specification preserves that correction in its audit trail and now uses:

- the named fresh/checked/shared call structure and changed-child test as the
  primary mechanism guard;
- a 100-microsecond median improvement guard, satisfied by the recorded
  461-microsecond quiet-window delta;
- the existing regression guard, satisfied by all three fixtures; and
- the prebuilt-gap guard, satisfied by measured gaps of 2.4× to approximately
  15×.

This resolves the previous finding without weakening the public checked-graph
freshness contract or expanding the fix beyond its stated non-goals.

## Implementation Assessment

- `validate` builds once and immediately routes the graph through
  `validate_fresh_graph` (`validate.rs:334-343`).
- `validate_with_graph` still invokes `verify_graph_compatibility` before the
  shared engine and therefore before flattening (`validate.rs:357-372`).
- `validate_fresh_graph` has narrow `pub(super)` visibility and documents the
  conditions under which verification may be skipped (`validate.rs:375-399`).
- `validate_graph_contents` remains the single owner of flattening, fragment
  preparation, local and remote checks, fail-fast behavior, and report
  construction (`validate.rs:401-416`).
- `FileTree::ensure_built` keeps construction, clone-stable option setup, and
  fresh validation adjacent, with no caller handoff (`file_tree/mod.rs:232-250`).
- The paired changed-child test proves that the fresh seam validates the graph's
  build-time reference snapshot while the checked seam rejects that same stale
  graph as a changed dependency (`validate.rs:1503-1563`).

The named seams make the trust boundary harder to misuse than a Boolean switch,
and the shared engine avoids behavior drift or duplicated validation work.

## Requirement-to-Verification Assessment

This fix changes deterministic library and filesystem behavior. It introduces
no terminal rendering, terminal-emulator input, keybinding, paste, IME, mouse,
browser, or OS keyboard behavior. Level 1 is therefore the appropriate test
level for acceptance criteria 1–7 and 9. Criterion is the appropriate evidence
for criterion 8; Level 2 and Level 3 are not applicable.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One graph build; ordinary validation skips compatibility and dependency-manifest verification | Level 1 source path plus changed-child mechanism test | **Pass.** `validate` calls the fresh seam directly. |
| 2 | `FileTree::ensure_built` uses the same trusted fresh seam | Level 1 source path plus file-tree unit/integration tests | **Pass.** The just-built graph is validated directly with clone-stable options. |
| 3 | Public prebuilt validation remains fully fail-closed before flattening | Level 1 edited/missing/unreadable/cache-stale/document/source/mode/options tests | **Pass.** The compatibility check remains first. |
| 4 | Both paths share one post-verification engine | Level 1 source inspection plus parity tests | **Pass.** Both seams call `validate_graph_contents`. |
| 5 | Changed child differentiates fresh and checked paths | Level 1 paired unit test | **Pass.** Fresh validation uses the stored reference snapshot; checked validation reports `Changed`. |
| 6 | Existing provenance, mismatch, parity, file-tree, and presentation behavior remains green | Level 1 unit/integration/spawned-CLI suites | **Pass.** The complete area Level-1 gate passed. |
| 7 | Public signatures, errors, reports, CLI output, and graph views remain unchanged | Level 1 compile/API, CLI baseline, parity, and serialization coverage | **Pass.** No public type, signature, or CLI implementation changed. |
| 8 | Same-session evidence satisfies the amended mechanism, improvement, regression, and prebuilt-gap guards | Criterion plus mechanism inspection | **Pass.** 461 microseconds exceeds the 100-microsecond guard; all other guards pass. |
| 9 | Focused tests, area build/test/lint, whitespace, and change-scope gates pass | Level 1 plus repository tooling | **Pass.** All required gates are green. |

No user-observable requirement is verified below its appropriate level, so
there is no Level-1/Level-2/Level-3 mismatch finding.

## Verification Performed

- `just build` from `darkmatter/`: **PASS** for `darkmatter`,
  `darkmatter-cli`, and `dmls`.
- `just test` from `darkmatter/`: **PASS** — 5,763 `darkmatter`, 559
  `darkmatter-cli`, and 566 `dmls` Level-1 tests (6,888 total), zero failures.
- `just lint` from `darkmatter/`: **PASS** for all three packages, with no
  warnings.
- `git diff --check`: **PASS**.
- `md get` read back every required review, previous-review, and specification
  frontmatter value exactly. `md schema validate` accepts the specification.
  Review-schema validation remains blocked by existing schema-infrastructure
  drift: `schemas/feature-review.yaml` is itself rejected as a standalone
  tagged schema because it combines unsupported `$schema` and `description`
  keys with `kind`/`types`.
- GitNexus upstream impact: `validate` is **HIGH** (17 impacted, 15 direct test
  callers); `validate_with_graph` is LOW; `FileTree::ensure_built` is MEDIUM
  (13 direct unit/integration callers). No indexed execution process crosses
  the reference subsystem. Worktree-scoped `detect_changes` reports the
  expected reference-validation/file-tree symbols, zero affected processes,
  and overall low change risk. The index predates the two newly extracted seam
  symbols, so current-source inspection supplies their call relationships.
- `sniff repo packages` confirms the affected package area is Darkmatter
  (`darkmatter`, `darkmatter-cli`, and `dmls`; `zed-dmls` is workspace-excluded).
  No public or cross-area change requires broader gates.

Cross-platform execution was not available in this macOS session. The source
change uses portable Rust control flow and existing cross-platform filesystem
abstractions; no OS-specific code was added.

## Production Readiness

**Ready.** The prior finding is resolved, all amended acceptance criteria pass,
and no new finding blocks release.
