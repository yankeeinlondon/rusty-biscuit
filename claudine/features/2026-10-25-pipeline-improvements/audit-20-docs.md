---
kind: implementation-log
---

# Documentation audit: 2026-10-30-claudine-recursive-schemas

Updated **2 existing canonical documents**; created **0 documents**. Nine
canonical references are recorded in the completed specification. Every
referenced page declares `kind: canonical-documentation`, and the snapshot
body is byte-for-byte unchanged from HEAD.

| Canonical document | Functionality covered | Change in this audit |
| --- | --- | --- |
| `claudine/docs/topics/schemas.md` | Recursive ordinary/initialization/cancellation families and containers, generated argument forms, deferred values, parser-only depth, embedding, ownership and regeneration, launch/editor parity | Added a maintainer guide to the deterministic 2 MB embedded-schema budget and stage-based timing workflow; corrected a remaining claim that literal schema rejection precedes all execution, since shell-free initialization can precede the verdict. |
| `claudine/docs/topics/composition.md` | Strict control-key launch verdict, deferred values, and validation phases | Reviewed current integrated behavior; no edit needed. |
| `claudine/docs/topics/flow-control/lifecycle.md` | Recursive action grammar, conditional counting, initialization restrictions, event-time binding and dispatch | Reviewed current examples and runtime boundary; no edit needed. |
| `claudine/docs/topics/state-management/side-effects.md` | Authored lifecycle effects, evaluated argument binding and typed metadata ownership | Reviewed current behavior; already canonical. |
| `darkmatter/docs/topics/schemas/definition.md` | Recursive resolution/lowering, generated shared verb definitions, signature projection, conversions and deferred operands | Reviewed current detailed reference and regeneration commands; no edit needed. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Matching trigger schemas, recursive completion/hover, editor/CLI parity and passive analysis | Reviewed current recursive and generated-verb coverage; no edit needed. |
| `darkmatter/docs/inline/schema-validation.md` | Passive validation APIs, stage reports, cold/warm measurements and cache behavior | Reviewed timing command and benchmark workflow used by the new maintainer subsection; no edit needed. |
| `darkmatter/docs/topics/side-effects.md` | Shared effect engine, typed event-time binding, ownership and passive validation boundary | Added canonical kind; replaced a blanket pure-composition claim with the precise effects-engine boundary and a link to separately enabled shell behavior. |
| `darkmatter/dmls/docs/hover.md` | Generated long-form parameters, overload-aware hover and recursive cursor-directed lookup | Reviewed current behavior; already canonical. |

The schema topic is the obvious entry point for an unfamiliar developer. It
already includes compact YAML examples, separate schema/parser verdicts,
recursive container diagrams, and a schema-to-runtime flow. The existing
shared reference owns action projection and conversions, so no duplicate
canonical page was needed.

## Validation

Compared the documented families with `claudine/schemas/lifecycle.yaml` and
`sequence-types.yaml`; checked generated ownership against Darkmatter's
catalog projection and the `regen-lifecycle-schema` recipe. The size guide
matches `embedded_trigger_payloads_resolve_within_two_megabytes`: compact
JSON bytes, `$defs` and built-in baseline included, all three embedded trigger
combinations, without disk or user schema input. No historical byte count is
presented as the current size. The 200 ms validation target is explicitly a
development goal, with no new timing threshold or test gate.

All nine implicit repository-relative references exist under package-area
`docs/` folders and carry the requested kind. Checked relative Markdown
targets outside code fences in the two edited documents; all exist. No
feature/fix names or links were added to canonical documentation.
`git diff --check` passes. The Claudine and Darkmatter skills already describe
the final recursive families, generated signatures, parser limit, and shared
ownership accurately, so no skill edit was needed. Test execution belongs to
the concurrent test agent; this documentation slice ran no test suites.
