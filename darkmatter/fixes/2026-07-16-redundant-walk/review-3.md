---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T01:28:32-07:00
spec: 2026-07-16-redundant-walk/spec.md
implemented: true
implemented_by: opencode/kimi-for-coding/k3
log: darkmatter/fixes/2026-07-16-redundant-walk/log.md
description: "A **fix** review of `2026-07-16-redundant-walk/spec.md`"
fix: 2026-07-16-redundant-walk/review-3.md
previous: 2026-07-16-redundant-walk/review-2.md
---

# Review 3 — Redundant Walk

## Verdict

This fix is **not ready for production**. The fresh/checked seam split is
implemented correctly for reference extraction, and the checked public path
still rejects stale descendants before flattening. However, the claimed fresh
snapshot is incomplete when fragment validation is enabled: the shared engine
reopens transcluded children from disk to collect their headings. A concurrent
heading edit can therefore change the one-step report after graph construction,
even though the specification requires validation to use the just-built child
snapshot.

## Findings

### High — Fragment validation rereads transcluded children instead of using the fresh graph snapshot

The snapshot contract says that fresh validation uses the child contents held
by the newly constructed graph and that an edit after a child is loaded does not
affect the one-step operation (`spec.md:175-182`). That is not true when
`ReferenceValidationOptions::validate_fragments` is enabled, including through
the user-facing `md validate refs --fragments` path
(`cli/src/commands/validate.rs:42-52`).

`validate_graph_contents` calls `collect_composed_heading_slugs` whenever
fragment validation is enabled (`validate.rs:428-444`). That helper iterates the
already-built graph's child nodes but then reloads each child with
`Markdown::try_from` (`validate.rs:820-833`). Consequently, a child heading
changed after graph construction supplies the heading set used by the fresh
path. The report is a mixture of build-time reference records and post-build
heading content rather than the coherent snapshot promised by the specification.

The new mechanism test does not cover this path. It uses default options, where
fragment validation is false, and changes only the child's links while leaving
its heading unchanged (`validate.rs:1507-1545`). It proves that post-edit
references are absent from the flattened graph, but it does not prove the
broader snapshot claim made by its name, documentation, and acceptance
criterion 5.

This is a Level-1 correctness and verification gap. Level 1 is the appropriate
level because the behavior is deterministic in-process filesystem state; Level
2 and Level 3 would add no relevant evidence. It is High severity because the
user-observable `--fragments` behavior is verified below its required scenario,
and the review rules require such a mismatch to block production readiness.

**Suggested resolution:** retain build-time prepared heading slugs for graph
children in a private, non-serialized snapshot owned by `ReferenceGraph` (or an
equivalent private artifact) and make composed-heading collection use that
snapshot on the fresh path. Keep cross-document fragment targets that were not
loaded as graph descendants on their existing validation path. Extend the
paired mechanism test with `validate_fragments: true`, a root fragment link to
a transcluded child's original heading, and a post-build heading rename. Fresh
validation must keep the original heading result, while checked validation must
still return a changed-dependency `ReferenceGraphMismatch` before flattening.

## Implementation Assessment

Outside the finding above, the implementation matches the design:

- `validate` builds once and immediately calls `validate_fresh_graph`.
- `FileTree::ensure_built` uses the same narrowly visible fresh seam with
  clone-stable graph options and no caller handoff.
- `validate_with_graph` runs `verify_graph_compatibility` before the shared
  engine, preserving fail-closed prebuilt reuse.
- `validate_graph_contents` remains the single post-freshness validation and
  report engine.
- Public signatures, error variants, serialized graph views, and CLI wiring are
  unchanged.
- The amended Criterion evidence still satisfies the mechanism, improvement,
  regression, and prebuilt-gap guards for the redundant compatibility walk.

The named seams are ergonomic and safer than a Boolean verification switch.
The remaining problem is the data available to the fresh seam, not its API
shape.

## Requirement-to-Verification Assessment

This fix changes library and filesystem behavior only. Level 1 is appropriate
for every behavioral requirement; Level 2 and Level 3 are not applicable.
Criterion remains the appropriate performance evidence for acceptance
criterion 8.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | One graph build; ordinary validation skips compatibility and dependency-manifest verification | Level 1 source path plus changed-child mechanism test | **Pass.** The fresh seam does not call compatibility verification. |
| 2 | `FileTree::ensure_built` uses the trusted fresh seam | Level 1 source path plus file-tree tests | **Pass.** Construction and fresh validation remain adjacent. |
| 3 | Public prebuilt validation remains fail-closed before flattening | Level 1 provenance and stale-descendant tests | **Pass.** Compatibility verification remains first. |
| 4 | Fresh and checked paths share one post-verification engine | Level 1 source inspection plus parity tests | **Pass.** Both paths converge on `validate_graph_contents`. |
| 5 | Changed-child test proves fresh snapshot behavior while checked validation rejects staleness | Level 1 mechanism test | **Partial / fail.** It covers changed references with fragment validation disabled, but not transcluded-child headings reread by `--fragments`. |
| 6 | Existing provenance, mismatch, parity, file-tree, and presentation behavior stays green | Level 1 focused and prior full-area suites | **Pass for covered behavior.** The missing heading-edit scenario has no test. |
| 7 | Public signatures, errors, reports, CLI output, and graph views remain unchanged | Level 1 compile/API and existing CLI/serialization tests | **Pass.** No public surface changed. |
| 8 | Same-session evidence satisfies amended performance guards | Criterion plus mechanism inspection | **Pass.** The recorded 461-microsecond median improvement exceeds the 100-microsecond guard, with the other guards also satisfied. |
| 9 | Focused tests, area build/test/lint, whitespace, and change-scope gates pass | Level 1 plus repository tooling | **Pass on recorded closure evidence; current focused/build/lint/whitespace gates also pass.** |

## Verification Performed

- Focused reference/validation/file-tree selection: **PASS**, 453/453 tests.
- `just build` from `darkmatter/`: **PASS** for `darkmatter`,
  `darkmatter-cli`, and `dmls`.
- `just lint` from `darkmatter/`: **PASS** for all three packages.
- `git diff --check`: **PASS**.
- `md get` read back every required review, previous-review, and specification
  frontmatter value exactly. `md schema validate` accepts the specification.
  Review-schema validation remains blocked by existing schema-infrastructure
  drift: `schemas/feature-review.yaml` is rejected as a standalone tagged
  schema because it combines `$schema` and `description` with `kind`/`types`.
- A fresh `just test` attempt reached 2,210/5,763 passing `darkmatter` tests
  with no failures before being interrupted at the non-interactive session's
  command-duration ceiling. Review 2 records the completed same-source area
  run: 6,888 Level-1 tests across `darkmatter`, `darkmatter-cli`, and `dmls`,
  all passing.
- GitNexus impact: `validate` is **HIGH** due to 15 direct test dependents;
  `FileTree::ensure_built` is MEDIUM; checked-prebuilt helpers are LOW. No
  indexed execution process crosses the reference subsystem. Worktree change
  detection reports low overall risk and the expected reference/file-tree
  symbols.
- `sniff` identifies the affected area as Darkmatter: `darkmatter`,
  `darkmatter-cli`, and `dmls`; `zed-dmls` is workspace-excluded.

Cross-platform execution was unavailable in this macOS session. The changed
control flow and filesystem APIs are portable Rust, but the High finding is
platform-independent and reproducible at Level 1 on macOS, Linux, and Windows.

## Production Readiness

**Not ready.** The fresh path must stop rereading transcluded-child headings for
fragment validation, and a Level-1 heading-mutation mechanism test must lock in
that snapshot behavior.
