---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-07T15:53:22-07:00
spec: 2026-09-07-required-vs-eager/spec.md
implemented: false
description: A **fix** review of `2026-09-07-required-vs-eager/spec.md`
fix: 2026-09-07-required-vs-eager/review-4.md
previous: 2026-09-07-required-vs-eager/review-3.md
---

# Review 4: Required vs Eager

## Verdict

The fix is **ready for production**. Review 3's blocker is closed at the shared
validation seam: the workspace now uses `jsonschema` 0.55, which enforces the
two-name `required` shape that 0.42 silently dropped, and permanent Level-1
regressions exercise both Darkmatter's effective-schema path and a real DMLS
initialize/open/publish-diagnostics session. The required/eager behavior from
the specification remains unchanged and all scoped gates pass.

One low-severity documentation inconsistency remains in dependency catalogs.
It does not affect the public required/eager contract or production behavior.

## Findings

### 1. Low: Two package dependency pages still name the retired 0.42 workspace pin

`claudine/contract/docs/dependencies.md` and
`unchained-ai/contract/docs/dependencies.md` each identify the dependency as
`jsonschema` 0.55 and then, two lines later, say it is pinned to the
workspace-wide 0.42 used by Darkmatter and Schematic. The manifests, lockfile,
root dependency catalog, and `cargo tree` evidence all establish 0.55 as the
current single version.

Change those trailing `0.42` references to `0.55`, or remove the exact version
from the prose and retain only the shared-version invariant. This is a
documentation-only cleanup and does not block release.

## Review 3 Closure

The prior high-severity finding is closed.

- `jsonschema` was upgraded from 0.42.2 to 0.55.0 in all five workspace
  manifests that select it. `fancy-regex` was aligned to 0.19 so Darkmatter
  continues to share the transitive copy.
- `schemas_required_count_matrix.rs` exercises one through four omitted
  required properties through `DarkmatterSchemas`, including one supplied from
  a required pair and the affected `additionalProperties` schema-object shape.
  It asserts the exact missing property names rather than only diagnostic
  counts.
- `strict_mode_reports_every_absent_required_property_at_any_required_count`
  drives six strict-mode documents through the normal DMLS LSP session. It
  covers two required-only properties, two `required; eager` properties, and
  one supplied plus one omitted, while asserting diagnostic code, unique
  property name, severity, source, and exact range.
- The dependency API adaptation is limited to the lifetime parameter added to
  `jsonschema::Keyword`; no validation policy or diagnostic mapping was
  changed.

The regression evidence is non-vacuous: the implementation log records all
four new tests failing on the affected two-required cases under 0.42.2 and
passing under 0.55.0.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — independent syntax | Level 1 parser tests and real DMLS LSP sessions | Appropriate and present. |
| AC2 — strict eager-only absence | Level 1 real DMLS session with a missing `required; eager` control | Appropriate and present; the count regression now also prevents false-clean two-required schemas. |
| AC3 — supplied eager validation | Level 1 real DMLS session with typed diagnostics and value ranges | Appropriate and present. |
| AC4 — hover distinction | Level 1 real DMLS hover requests on definition and instance surfaces | Appropriate and present. |
| AC5 — completion detail | Level 1 real DMLS completion request | Appropriate and present. |
| AC6 — one wording authority | Level 1 catalog, provider, CLI, and LSP assertions | Appropriate and present. |
| AC7 — no phase fork | Source inspection and Level 1 DMLS coverage | Appropriate; DMLS still uses passive shared validation. |
| AC8 — semantic matrix and array ownership | Level 1 Darkmatter phase tests, shared-validator regressions, and real DMLS sessions | Appropriate and present, including null, invalid values, both array placements, and the required-count matrix. |
| AC9 — documentation parity | Level 1 shipped-document assertions and source inspection | The required/eager semantic surfaces agree. Finding 1 concerns dependency-version prose only. |
| AC10 — passive editor behavior | Level 1 no-side-effects and temporary-workspace DMLS tests | Appropriate and present. |

Levels 2 and 3 are not applicable. The requirements concern schema
compilation, validation, and structured LSP payloads; none depends on terminal
rendering, a terminal input encoder, or OS keyboard injection.

## Verification Performed

- `darkmatter/ just test`: **7,709 passed; 51 skipped**.
- `darkmatter/ just lint`: **clean**, including `zed-dmls` for
  `wasm32-wasip2`.
- Root `just test claudine`: **7,082 passed; 13 skipped**. The existing macOS
  `__eh_frame` compact-unwind linker warning was emitted; the gate passed.
- Root `just test biscuit-file schematic unchained-ai`: **2,879 passed; 6
  skipped**.

## Production Readiness

Every user-observable requirement has the appropriate Level-1 evidence, Review
3's false-clean diagnostic defect is fixed at its upstream cause, and all
directly affected package areas pass their scoped tests. The low-severity
dependency-document wording should be cleaned up, but it does not prevent this
fix from shipping.
