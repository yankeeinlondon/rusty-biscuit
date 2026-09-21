---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-20T11:58:22-07:00"
spec: 2026-09-19-less-brittle/spec.md
implemented: false
description: "A **fix** review of `2026-09-19-less-brittle/spec.md`"
fix: 2026-09-19-less-brittle/review-6.md
previous: 2026-09-19-less-brittle/review-5.md
---

# Review 6

**Production-ready.** Review 5's only finding is implemented. The two
cross-language schema documents are now exact entries in `SUITE_OWNER_PATHS`;
there is no schema-directory prefix that can select `test-toolkit` for
unrelated files. Production-planner tests prove both sides of the boundary:
the two shared inputs select `repo-deps` and `test-toolkit`, while the schema
README and a hypothetical future schema select only `repo-deps`.

Review 5 contains one unblocked finding and no blocked-findings section. No
previously blocked finding became actionable before this implementation.

## Findings

None.

## Previous-review disposition

- **The schema-directory ownership rule schedules unrelated files:**
  implemented. `.github/ci/schemas/archive_guard_cases.json` and
  `.github/ci/schemas/contract.json` are listed individually with both owners.
  `.github/ci/schemas/README.md` and an unrelated future schema retain only the
  existing `.github/ci/` ownership by `repo-deps`.
- **Blocked findings:** review 5 contains no blocked-findings section. Reviews
  2 through 4 also recorded none, so there is no deferred finding to carry
  forward.

## Requirement-to-verification map

This fix has no terminal rendering, keyboard, mouse, paste, IME, or other
terminal-emulator input requirement. L2 and L3 would not verify its behavior;
L1 logic, temporary-filesystem, subprocess, real-Git, and workflow-contract
tests are the appropriate levels.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Twelve path migrations and runtime remapping | L1 package and relocation tests plus the canonical full-tree guard | Appropriate level; the shared resolver remains used at every migrated site. |
| Token-aware matching and expression-local exemptions | L1 lexer/matcher fixtures plus the canonical full-tree guard | Appropriate level; comments, strings, multiline forms, unsafe second occurrences, and hosted-root literals are covered. |
| Deletion, rename, missing-path, and symlink handling | L1 real-Git and temporary-filesystem fixtures | Appropriate level; the planner carries deletion identity and the reader fails closed on unexpected absence. |
| Canonical plan scope and cross-language validation | L1 Python validator, Rust reader, frozen field-set contract, and shared accept/reject corpus | Appropriate level; both readers enforce the same closed shape. |
| Guard-only scheduling, reporting, merge blocking, and evidence rules | L1 planner, workflow-source, local-runner, and rollup contracts | Appropriate level; one Linux lint-cell companion runs without unrelated suites or reusable partial-scan evidence. |
| Narrow ownership of cross-language schema inputs | L1 production-planner ownership tests with positive and negative paths | Appropriate level; review 5's over-selection gap is closed. |
| Standalone full-tree operation | L1 canonical guard recipe over 3,475 eligible Rust files | Appropriate level; five existing allowlisted forms were found and no new violation was reported. |

## Verification performed

- `python3 scripts/ci/test_affected_scope.py`: **298 passed**.
- `python3 scripts/ci/test_schema.py`: **134 passed**.
- `just test repo-deps`: **420 tests run, 420 passed, 1 skipped**.
- `just test test-toolkit`: **312 tests run, 312 passed, 2 skipped**.
- `just _lint repo-deps` and `just _lint test-toolkit`: passed with zero
  warnings.
- `cd tools/test-toolkit && just archive-path-guard`: **2 passed**; full-tree
  mode checked 3,475 files and reported only the five existing allowlisted
  forms.
- `git diff --check`: passed.

The refreshed GitNexus working-tree analysis reports 25 changed symbols across
11 files, no affected execution process, and low aggregate risk. Its direct
impact lookup for the module-level `SUITE_OWNER_PATHS` constant is `UNKNOWN`
because bare constant reads are not linked; text inspection confirms the only
production consumer is `suite_owner_paths`, which checks the exact-path table
before falling back to the remaining prefixes. No high- or critical-risk
impact was reported.

No terminal or browser window was launched. Cross-OS execution evidence is
left to CI and does not affect readiness under the requested closure rules.
No human-only test or unresolved design decision remains, so human review is
not required by this review.

The requested previous-review path under `prompts/_reviews/fixes/` does not
exist in this checkout. The authoritative previous review is
`fixes/2026-09-19-less-brittle/review-5.md`; it is marked implemented and now
points to this review. The specification records `review_iterations: 6` and is
marked completed.
