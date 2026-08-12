---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T00:16:31-07:00
spec: 2026-07-11-provider-errors-as-data/spec.md
implemented: true
description: A **feature** review of `2026-07-11-provider-errors-as-data/spec.md`
feature: 2026-07-11-provider-errors-as-data/review-1.md
next: /
previous: /
---

# Review: Provider Errors as Data (Iteration 1)

## Verdict

Not ready for production. Phase A's facts-backed migration is implemented and
the focused Level 1 suites are green, but the feature's defining Phase C outcome
has not landed: research is not the runtime source, the accepted vocabulary has
not been graduated, and the motivating Codex overload classification remains
absent. The completed research tooling also has two fail-open paths that can
certify a behavior-changing or invalid document as clean, and the required Kilo
discrimination fixture is missing.

## Findings

### Critical: The feature stops before research graduation and does not deliver its primary behavior

The specification requires the `agent-errors/` research frontmatter to become
the source of generated runtime vocabulary in Phase C, with facts deleted and
accepted deltas covered by parser tests. The current implementation is still
explicitly Phase-A facts-backed:

- `claudine/gen/src/vocabulary.rs:70` sets `DECLARED_SOURCE` to `Facts`.
- `claudine/gen/src/vocabulary.rs:207-212` loads no research topics while
  building the generated artifact.
- All nine parser-backed facts files still carry `error_vocabulary`.
- `claudine/docs/research/agent-errors/_delta-report.md` leaves the only delta,
  Codex `overloaded`, pending human adjudication.
- `claudine/features/_completed/2026-07-11-provider-errors-as-data/plan.md:513-615` marks
  source projection, facts deletion, delta tests, regeneration, documentation,
  and final acceptance incomplete.

This means G4/G6/G7 and Phase C are not implemented, and the motivating
"selected model is overloaded/at capacity" class still does not match the Codex
runtime vocabulary. The mandatory B3 and C1 human checkpoints are also still
pending, so this is not merely missing cleanup after a complete implementation.

Recommended fix: complete the checkpoint dispositions, implement the
provenance-object-to-runtime projection, load `agent-errors` during vocabulary
generation, prove the facts/research collision, delete graduated facts keys,
land only accepted deltas with positive and collision fixtures, regenerate
twice, and update the Claudine skill/onboarding documentation before closing the
feature.

**Verification level:** the current facts-backed classifier behavior has Level
1 parser and generator coverage. No test can verify the required research-backed
runtime behavior because that behavior is not implemented. Level 1 generator +
parser integration is the appropriate minimum; this feature does not require a
real terminal or OS keyboard injection.

### High: A gate execution error is treated as a clean research result

The fleet success stack runs the checker with `no_error: true` at
`claudine/docs/research/agent-errors/_fleet.md:37-40`, resumes only when the
findings file exists at `:46-51`, and declares success when that file is absent
at `:53-56`. However, `check_provider` can return an error before writing a
findings file, including schema-validation, input-read, directory-creation, and
rename failures (`claudine/gen/src/agent_errors_check.rs:480-531`). The existing
schema-invalid integration test explicitly expects this error path without a
findings artifact (`claudine/gen/tests/agent_errors_check.rs:161-187`).

Because the shell error is suppressed, an invalid document or checker IO failure
leaves no findings file, skips `resume`, and satisfies the "clean" condition.
Removing a stale findings file before writing its replacement makes this worse:
an error after removal converts a previously known failure into apparent
success. This violates D10's requirement that exhaustion or validation failure
remain machine-visible and never turn a known-bad document into a successful
fleet result.

Recommended fix: represent all checker outcomes explicitly. For example, write
an atomic report with `status: clean | findings | gate_error` and have lifecycle
conditions branch on that status; never infer clean from absence. Preserve the
last valid failure report until its replacement is durably ready. Add a Level 1
process/lifecycle test that forces schema and findings-write failures and proves
neither path emits the clean-success actions.

**Verification level:** Level 1 IO tests cover clean/findings files, and a Level
1 test only parses the fleet lifecycle shape. There is no end-to-end lifecycle
test for checker failure, resume, correction, or exhausted failure. Level 1 is
appropriate, but the required observable workflow is both unverified and broken.

### High: Seed preservation ignores semantic kind, bucket position, and order

Bucket order is explicitly the behavior contract, and D8 requires re-kinds and
reorderings to be surfaced for adjudication. The deterministic gate instead
flattens all needle text within each branch at
`claudine/gen/src/agent_errors_check.rs:212-245` and
`:315-347`. A seeded needle moved from `ApiRemote` to `Configuration`, moved
between repeated buckets, or reordered within a bucket still counts as present;
numeric codes likewise compare only the code and ignore their semantic kind.
An `evidence: seed` claim on the moved entry also passes because provenance uses
the same flattened sets.

The gate can therefore report zero findings for research that silently changes
classification or first-match precedence, undermining both the validate-and-
resume pilot and the delta report's mechanical completeness claim.

Recommended fix: compare an order-preserving identity for every seed row,
including branch, bucket index or stable bucket identity, semantic kind, item
position, and needle/code. Emit distinct findings for removal, re-kind, and
reorder so C1 can apply R1/R3/R4 deliberately. Add table-driven tests for a
cross-kind move, repeated-kind bucket move, intra-bucket reorder, bucket reorder,
and numeric-code re-kind.

**Verification level:** Level 1 unit/IO tests cover dropped rows only. Level 1
is the correct level for this data invariant, but the behavior-changing cases
are absent and currently pass incorrectly.

### High: Kilo's shared-parser proof does not discriminate its vocabulary from OpenCode's

D4 requires an end-to-end Kilo fixture whose winning classification differs
from OpenCode, specifically to prove the shared parser consults runtime provider
identity instead of a hard-coded OpenCode table. The current tests at
`claudine/lib/src/stream/providers/opencode.rs:1343-1392` feed the same `rate
limit` message and assert `ApiRemote` for both providers. Pointer inequality
between two identical statics proves separate storage, not that classification
uses the Kilo table.

The constructor also does not follow D4's rejection contract. Invalid identities
are silently coerced to OpenCode at `opencode.rs:573-588`, and the test at
`:1395-1403` locks that fallback in. A future wiring bug can therefore produce
plausible OpenCode classifications rather than fail at the construction boundary.

Recommended fix: make construction reject providers other than OpenCode/Kilo
with a typed error (or prevent invalid construction through a narrower internal
identity type). Add a Level 1 parser fixture with deliberately different Kilo
and OpenCode winning kinds; if production research remains identical, inject
test vocabulary through the classifier seam rather than using pointer identity
as a proxy.

**Verification level:** Level 1 parser tests are the appropriate level. Current
Level 1 coverage verifies provider stamps and separate statics, but not the
specified classification discrimination or rejection behavior.

## Requirement Coverage

| Requirement | Strongest verification | Status |
|---|---:|---|
| Phase A facts migration preserves all existing tables and Kimi codes | Level 1 generator/parser tests | Ready |
| Generated standalone module is drift-checked | Level 1 generator integration | Ready while facts-backed |
| Parsers consume generated vocabulary | Level 1 parser tests | Ready |
| Kimi exact code precedence and fallback | Level 1 parser/fixture tests | Ready |
| Kilo selects Kilo vocabulary through the shared parser | Level 1 identity/static tests | Gap: no discriminating winner fixture |
| Fleet documents validate and carry provenance | Level 1 schema/checker tests | Partial: gate can fail open |
| Seeds retain kind and ordering through research | Level 1 dropped-row tests | Broken: re-kind/reorder passes |
| Research becomes the sole runtime source | None | Not implemented |
| Accepted deltas have positive and collision tests | None | Not implemented; adjudication pending |
| Goose remains explicit and empty without a parser | Level 1 generated-table test | Ready |
| Provider-onboarding and skill docs describe the final contract | Code review | Not implemented |

No requirement in this feature needs Level 2 real-terminal capture or Level 3
OS input injection. Its user-observable outcome is semantic classification data,
for which Level 1 protocol/parser and generation tests are sufficient when they
cross the actual seams.

## Verification

- `cargo nextest run --color=never -p claudine-gen -p claudine` from
  `claudine/`: **3,538 passed, 0 failed, 0 skipped**.
- Generated vocabulary drift test passed as part of the focused suite.
- Static review traced facts/research loading, the deterministic findings-file
  lifecycle, seed comparison, Kilo/OpenCode construction, and the Phase C plan.

The green Level 1 baseline confirms the implemented Phase A behavior; it does
not resolve the critical incomplete scope or the three high-severity gaps.
