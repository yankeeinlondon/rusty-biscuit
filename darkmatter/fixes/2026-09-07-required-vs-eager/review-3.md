---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-07T14:29:55-07:00
spec: 2026-09-07-required-vs-eager/spec.md
implemented: false
description: A **fix** review of `2026-09-07-required-vs-eager/spec.md`
fix: 2026-09-07-required-vs-eager/review-3.md
previous: 2026-09-07-required-vs-eager/review-2.md
---

# Review 3: Required vs Eager

## Verdict

The fix is **not ready for production**. Both findings from Review 2 are
implemented correctly: Claudine now tells inline agents that eager-only
properties remain optional at completion, and DMLS's strict absence fixture
now covers both array placements. During that implementation, however, a
Level-1 real LSP regression probe exposed a separate failure in the same
required-property diagnostic contract: a strict-mode document with exactly two
omitted required properties receives no `dm.schema.missing_required`
diagnostics. The probe was removed instead of becoming a permanent regression
test, and the defect remains unfixed.

## Findings

### 1. High: DMLS drops every strict missing-required diagnostic when exactly two properties are required

The implementation log records two independent reproductions through the
normal DMLS initialize/open/publish-diagnostics path. With strict schema mode
enabled, one omitted required property produces one diagnostic, two produce
zero, three produce three, and four produce four
(`darkmatter/fixes/2026-09-07-required-vs-eager/log.md:33-41`). The exactly-two
case also remains silent when one of those two properties is supplied. The
compiled JSON Schema still contains both names in `required`, but the
`EffectiveSchema::validate_with_options` report passed to DMLS is incorrectly
valid and empty; `md schema validate` reports the omissions for an equivalent
document. DMLS's production diagnostic path consumes that report directly
(`darkmatter/dmls/src/diagnostics/frontmatter.rs:60-73`).

Calling the defect unrelated or pre-existing does not make it compatible with
this specification. The required semantic matrix says required properties must
be present at completion, DMLS behavior says existing missing-required behavior
must be retained, and AC2 relies on a missing `required; eager` control. The
current AC2 fixture has deliberately only one required property
(`darkmatter/dmls/tests/lsp_session.rs:3694-3762`), so it passes while the
two-property failure remains invisible. A real author commonly has two required
fields and would receive a false-clean editor verdict.

Keep the reproduced 1/2/3/4 matrix as a permanent Level-1 LSP regression,
identify why DMLS's effective-schema validation differs from `md schema
validate`, and fix the shared validation/preparation seam rather than adding a
DMLS-local count workaround. Include controls for two required-only properties,
two `required; eager` properties, and one supplied plus one omitted. Assert the
diagnostic codes, property names, severities, and stable mapping ranges.

## Review 2 Closure

Both prior findings are closed.

- `def_is_required_at_completion` now considers only `Constraint::Required`
  across item constraints, postfix array constraints, and union arms
  (`claudine/lib/src/composition/inline_prompt.rs:78-95`). Its matrix covers all
  four required/eager combinations and both array placements, and the normal
  `prepare_inline` path pins the exact table delivered to the agent
  (`claudine/lib/src/composition/prepare/tests.rs:263-302`).
- `STRICT_EAGER_ABSENCE_DOC` now includes omitted `file(eager)[]` and
  `file[](eager)` properties and asserts that neither is named by the sole
  required-control diagnostic (`darkmatter/dmls/tests/lsp_session.rs:3694-3762`).

The relevant behavior-changing documentation was updated with the code; no
remaining active Darkmatter or Claudine surface was found that asserts eager
alone controls presence.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — independent syntax | Level 1 parser tests and real DMLS LSP sessions | Appropriate and present. |
| AC2 — strict-mode eager-only absence | Level 1 real DMLS initialize/open/publish-diagnostics session | The one-control fixture passes, but strict missing-required reporting fails for exactly two required properties. Finding 1 blocks readiness. |
| AC3 — supplied eager validation and value ranges | Level 1 real DMLS session | Appropriate and present. |
| AC4 — definition and instance hover distinction | Level 1 real DMLS hover requests plus provider tests | Appropriate and present. |
| AC5 — accurate completion detail | Level 1 real DMLS completion request | Appropriate and present. |
| AC6 — one wording authority | Level 1 catalog, provider, CLI, and LSP assertions | Appropriate and present; DMLS derives eager prose from `schema_constraint_descriptors()`. |
| AC7 — no phase fork | Source inspection plus Level 1 DMLS coverage | Appropriate; no DMLS runtime-phase projection was introduced. |
| AC8 — matrix, null, invalid values, and array ownership | Level 1 Darkmatter phase tests and real DMLS sessions | Required/eager and array-placement coverage is present. The missing 1/2/3/4 required-count regression is a material diagnostic gap under Finding 1. |
| AC9 — documentation parity | Level 1 shipped-document assertions plus source inspection | The named active documentation, descriptor, plan, skill, comments, DMLS output, and Claudine prompt now agree. |
| AC10 — passive editor behavior | Level 1 DMLS no-side-effects and temporary-workspace LSP tests | Appropriate and present; no shell, remote, terminal, browser, or host-input behavior is required. |

Levels 2 and 3 are not applicable. These requirements concern schema
projection, structured LSP payloads, and generated Markdown prompt text; they do
not depend on real-terminal rendering or terminal/OS input encoding.

## Verification Performed

- `darkmatter/ just test`: **7,704 passed; 51 skipped**.
- Targeted DMLS strict eager/required absence regression: **1 passed**.
- Root targeted Claudine prompt regressions: **3 passed; 7,092 skipped**.
- The Claudine build emitted the existing macOS linker warning that the
  `__eh_frame` section is too large for compact unwind offsets; the test command
  still exited successfully.

The green suite does not exercise the reproduced exactly-two-required case.
That missing Level-1 regression and its associated false-clean diagnostic
behavior are the production blocker.

## Production Readiness

The required/eager independence work and Review 2 corrections are otherwise
complete and appropriately verified at Level 1. Production readiness remains
blocked until strict DMLS validation reliably reports required-property
omissions for ordinary multi-property schemas and the reproduced matrix is
kept as a regression test.
