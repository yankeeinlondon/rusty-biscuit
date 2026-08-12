---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T23:39:09-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-4.md
previous: 2026-07-13-file-resolution/review-3.md
next: 2026-07-13-file-resolution/review-5.md
---

# Review 4: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. Review 3's completion, recursive
interpolation, diagnostic-detail, package-context, and request-snapshot gaps
have been substantially addressed. The real Claudine completion producer now
round-trips through shared `FileReference` resolution, recursive interpolation
uses effective anchoring, and the primary composition/Darkmatter seams receive
an explicit request snapshot.

Production closure is nevertheless blocked by two executable regressions and
an incomplete migration. A Level 1 process test proves that launching from one
repository while targeting a document in another still gives nested magic
references the launch repository's roots. A Level 2 tmux test proves a
target-initialize A→B→A proxy cycle no longer surfaces its cycle/hop-limit
error. The public Darkmatter reference-analysis APIs named by the feature's own
D12 inventory also retain ambient `resolve_relative` calls, suppressed errors,
and manual path-join fallbacks. In addition, the required Claudine Level 1 and
Level 2 area gates are red.

## Findings

### 1. High — The request snapshot remains anchored to the launch repository when the source document belongs to another repository

`capture_file_resolution_context` discovers the Git root, package area,
package, and Claudine magic roots from process CWD
(`claudine/lib/src/composition/resolve.rs:51-92`). After the top-level source is
resolved, child contexts change the source/base but intentionally retain those
launch-derived roots. That is correct for nested documents reached through an
explicit configured trust root, but not for a top-level document selected from
a different repository: D2 requires the nearest trusted worktree containing
the source, and D10 forbids process-CWD discovery from deciding a document's
repository.

The Level 1 process test
`sequence_magic_reference_uses_source_doc_location_not_cwd` fails after all
four nextest attempts (`claudine/cli/tests/sequence_magic_reference.rs:208-226`).
It launches Claudine in an unrelated repository containing a three-step
`fixtures/steps.yaml`, targets a document in a second repository whose fixture
has two steps, and observes three provider invocations instead of two. The
nested `@fixtures/steps.yaml` therefore resolves from the launch repository,
not the authoring document's repository. This violates D2, D10, and Acceptance
Criteria 12 and 14.

**Required change:** distinguish the provisional context used to resolve a
top-level CLI argument from the definitive document execution snapshot. Once
the source is known, establish the source-containing repository/package roots
once and derive nested contexts from that snapshot without further ambient
reads. Keep an explicit API for deliberately trusted external home/magic/vault
documents rather than treating every cross-repository top-level source as a
nested external document. Add the same outside-launch-repository fixture for
compose and inline-compose, plus a nested proxy/transclusion case.

### 2. High — A real-terminal target-initialize proxy cycle no longer reports the cycle

`level2_lifecycle_initialize_proxy_cycle_guarded` fails in the canonical tmux
Level 2 tier after all four attempts
(`claudine/cli/tests/level2_lifecycle_control.rs:1378-1402`). The pane records
the hand-off to `target.md` and the back-proxy to `doc.md`, and the target's
initialize action runs, but the terminal never contains the required proxy
cycle or hop-limit error.

The guard still compares `PathBuf` values directly in
`proxy_handoff_allowed` (`claudine/lib/src/composition/lifecycle/control.rs:206-222`),
while the request-context route now supplies resolved paths through a different
handoff path (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs:537-575`).
Whatever path-identity mismatch or error-routing loss underlies the regression,
the user-observable D6 behavior is broken. Its strongest appropriate test is
Level 2 because the requirement is that the typed error surfaces in a real
terminal, and that Level 2 test is red. This violates D6 and Acceptance
Criteria 5 and 10.

**Required change:** normalize one stable lexical path identity before adding
or comparing proxy-chain entries, preserve that identity across
re-materialization, and route a rejected target-initialize handoff through the
same typed terminal error path as the other proxy routes. Add a Level 1
A→B→A identity test using the request-scoped resolver in addition to restoring
the existing Level 2 assertion.

### 3. High — The D12 audit still has unmigrated Darkmatter reference-analysis resolvers

The feature inventory explicitly marks Darkmatter's three `reference/*`
callers as requiring migration to the detailed request context
(`claudine/features/2026-07-13-file-resolution/inventory.md:64-84`). They remain
on the pre-feature implementation:

- `markdown/reference/mod.rs:32-52` calls ambient `resolve_relative`, suppresses
  parser/resolver errors, then falls back to `base_dir.join(raw_target)`.
- `markdown/reference/graph.rs:840-866` repeats the same ambient resolution,
  error suppression, join fallback, and opportunistic canonicalization.
- `markdown/reference/validate.rs:499-529,636-672` repeats both patterns for
  local targets and cross-document fragments.

These public analysis/validation surfaces can therefore disagree with the
request-scoped composition pipeline after CWD/environment mutation, bypass the
shared repository-first candidate plan, and turn invalid, missing-context, or
I/O failures into a manually joined path. Their current Level 1 tests exercise
the legacy behavior; there is no request-snapshot parity test for these APIs.
This violates D1, D2, D5, D8, D12, and Acceptance Criteria 12, 13, and 15.

**Required change:** migrate these functions to an explicit
`FileResolutionContext`/detailed outcome, remove the join/canonicalize fallback
from reference classification, and choose an explicit public compatibility
boundary if context cannot be added without an API transition. Add collision,
ambient-mutation, invalid-reference, and permission/I/O fixtures shared across
transclusion enumeration, graph construction, and validation. Complete the
same audit decision for `markdown/schemas/detect.rs`, which the inventory also
marks as needing an explicit policy.

### 4. Medium — `FileResolutionContext` derivation can bypass its repository-containment invariant

`for_source` and `for_base` set `derived_authoring_base = true`
(`biscuit-file/lib/src/file_reference/context.rs:165-195`). `validate` then
skips repository containment for every derived context
(`context.rs:301-327`) on the assumption that the request context was already
validated. The type does not encode that assumption, and neither derivation
method validates or returns a `Result`. A caller can derive from an invalid
unvalidated request context and resolution will accept a repository root that
the initial base never trusted.

This is not a filesystem sandbox escape—the documentation correctly says the
check is not a sandbox—but it weakens D2's semantic trust-boundary guarantee
and contributed to making launch-root retention difficult to distinguish from
a valid external-document derivation.

**Required change:** make the validated request snapshot an enforceable API
state, validate before derivation, or keep validation active unless a clearly
named trusted-external derivation method is selected. Add a test proving an
invalid initial root cannot become valid merely by calling `for_source` first.

### 5. Medium — Claudine's required test-placement gate is red

The Level 1 area run fails
`claudine-cli::test_placement repository_test_placement` because
`claudine/lib/src/harness/resolve.rs` contains 207 production lines and 432
inline test lines, exceeding the enforced 300-line inline-test threshold. The
Claudine architecture rules require substantial tests to live in a sibling
`tests.rs` or `tests/` module. This is a required repository gate, not a test
count preference, and makes Acceptance Criterion 10 false.

**Required change:** move the inline resolver tests to
`claudine/lib/src/harness/resolve/tests.rs` (or a focused `tests/` directory)
without changing their behavior, leaving `#[cfg(test)] mod tests;` at the
production module boundary.

### 6. Medium — Documentation and code comments still ratify the removed document-first/launch-fallback contract

The specification explicitly requires old document-first/launch-fallback
claims to be updated. Material drift remains, including
`darkmatter/docs/inline/schema-validation.md:104`,
`darkmatter/lib/src/markdown/schemas/validate.rs:68,149,323,358`,
`darkmatter/lib/src/markdown/schemas/mod.rs:428-430`,
`claudine/cli/src/commands/compose/prep.rs:285`, and
`claudine/cli/src/commands/wrap/sequence/jit.rs:231-232`. Several tests and
comments still call document-first behavior the contract even where the
implementation now uses repository-first resolution and treats launch area as
diagnostic-only.

This drift makes the public behavior and maintenance guidance contradictory,
violating Acceptance Criterion 7 and the specification's Documentation and
Migration section.

**Required change:** update the public Darkmatter schema documentation and
then remove or rewrite stale rustdoc, inline comments, test names, and assertion
messages across the touched schema/expression/sequence surfaces. Preserve
mentions of launch context only where it really is the top-level reference
base or diagnostic metadata.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
|---|---|---|
| Shared parsing, effective interpolation kind, explicit/implicit precedence, candidate provenance, fallible probing, home, and recursive behavior | Level 1 `biscuit-file` unit/integration tests | Appropriate and green. Review 3's recursive interpolation and I/O-detail gaps are closed. |
| A value emitted by real Claudine completion executes unchanged | Level 1 subprocess integration in `completion_resolution_round_trip` | Appropriate and green: 2/2 collision/package-only round trips passed. |
| Request snapshot survives ambient mutation while nested documents adopt their own source | Level 1 unit/process tests | Appropriate level, but incomplete and red: the outside-launch-repository sequence fixture fails (Finding 1), and public reference-analysis APIs lack parity coverage (Finding 3). |
| Proxy routes share resolution and cycle/hop-limit behavior | Level 1 route tests plus Level 2 tmux lifecycle test | Resolution/no-match coverage is appropriate; the user-visible target-initialize cycle test is red (Finding 2). |
| Bare motivating reference succeeds, paired explicit source-relative reference fails, and ordered no-match candidates render in a real terminal | Level 2 tmux tests in `level2_file_resolution_capture` | Appropriate and green: all three dedicated file-resolution captures passed. |
| Exact rendered candidate order and typed error presentation | Level 2 tmux capture | Appropriate and green for the dedicated no-match fixtures. No Level 3 input-encoder behavior is claimed. |
| macOS/Linux/Windows path classification and home semantics | Host-independent/target-gated Level 1 tests; macOS runtime execution | Appropriate for deterministic path semantics. Native Windows execution was not available on this host. |
| Required package gates | Area `just test`, `just lint`, and Claudine `just test-l2` | Not satisfied: Claudine L1 has two failures and L2 has one failure; all lint gates are green. |

Level 3 is not applicable. The feature does not claim behavior driven by real
OS keyboard/mouse events, paste, IME, hotkeys, or terminal input encoding.

## Verification Performed

- Read the specification, Review 3, implementation inventory, the shared
  resolver/context/completion code, Claudine orchestration and completion
  adapters, Darkmatter context/reference surfaces, and feature-focused tests.
- Used GitNexus execution-flow search to confirm the main request-scoped
  `FileResolutionContext` path through composition, schema, expression,
  transclusion, link, and completion surfaces. The remaining public
  `markdown/reference/*` callers are outside that migrated seam.
- `biscuit-file/just test` passed: 372 library/integration tests passed with 4
  skipped; 61 CLI tests passed.
- `biscuit-file/just lint` passed.
- `darkmatter/just test` passed: 5,645 library tests, 555 CLI tests, and 566
  DMLS tests passed; 209 tests were skipped by their configured requirements.
- `darkmatter/just lint` passed.
- `claudine/just test` failed. The library tests completed without a failure,
  but the CLI tier failed after retries in
  `sequence_magic_reference_uses_source_doc_location_not_cwd` and
  `repository_test_placement`; fail-fast left later tests unrun.
- Targeted `claudine-cli --test completion_resolution_round_trip` passed 2/2.
- `claudine/just lint` passed, including all 18 error-guard checks and the
  package clippy/format checks.
- `claudine/just test-l2` failed: 91 passed, 1 failed, and 56 were not run after
  fail-fast. The failure was
  `level2_lifecycle_initialize_proxy_cycle_guarded`; the three dedicated
  file-resolution capture tests passed.
- Biscuit-file parsing confirmed every requested review, previous-review, and
  specification frontmatter value. `md schema validate` could not validate the
  review because the repository's `schemas/feature-review.yaml` is itself
  rejected as a standalone SimplifiedSchema: tagged schemas currently permit
  only `kind` and `types`, but that file also declares `description` and
  `$schema`. This is schema-tooling drift, not a review-frontmatter parse error.
- No formatting or Git commit was performed. Existing unrelated worktree
  changes were preserved.

## Production Readiness Closure

Production readiness requires all three high findings to close, the context
invariant and documentation/test-placement findings to be corrected, and the
full required Claudine Level 1 and Level 2 gates to pass. Closure evidence must
include the cross-repository top-level source fixtures, a green real-terminal
proxy-cycle assertion, and explicit-context tests for the remaining Darkmatter
reference-analysis APIs.
