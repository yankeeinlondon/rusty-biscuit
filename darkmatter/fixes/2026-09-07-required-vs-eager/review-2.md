---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-07T12:23:09-07:00
spec: 2026-09-07-required-vs-eager/spec.md
implemented: false
description: A **fix** review of `2026-09-07-required-vs-eager/spec.md`
fix: 2026-09-07-required-vs-eager/review-2.md
previous: 2026-09-07-required-vs-eager/review-1.md
---

# Review 2: Required vs Eager

## Verdict

The fix is **not ready for production**. The Darkmatter and DMLS work requested
by the specification is implemented and well verified at Level 1, but a
downstream user-facing Claudine prompt still presents the retired contract to
every inline agent: it labels an eager-only property as required at completion.
That directly conflicts with Darkmatter's actual completion verdict and can
cause an agent to populate a property the schema intentionally leaves optional.

## Findings

### 1. High: Claudine's inline prompt still says eager-only properties are required at completion

`def_is_required_at_completion` returns true for either `Constraint::Required`
or `Constraint::Eager`, and its documentation explicitly says eager implies
required-at-completion
(`claudine/lib/src/composition/inline_prompt.rs:78-90`). The generated property
table is placed in the prompt sent to every inline agent, under the column “At
completion” (`inline_prompt.rs:45-64`). This is user-observable behavior, not a
historical comment.

The implementation's downstream test does not catch the drift. It pins it:
`eager_without_required_is_still_required_at_completion` expects
`string(eager)` to render as `required` (`inline_prompt.rs:139-145`). The Phase
5 evidence then lists this passing test as proof that runtime verdicts are
unchanged (`darkmatter/fixes/2026-09-07-required-vs-eager/plan.md:315-318`). In
contrast, Darkmatter's completion validator allows eager-only absence, and
Claudine's current public contract says the same
(`claudine/docs/topics/composition.md:681-695`).

The strongest verification is Level 1, which is the correct level for generated
prompt text, but it verifies the wrong outcome. Change the helper to treat only
`required` as a completion presence rule across item constraints, postfix array
constraints, and union arms. Rename and invert the eager-only regression, retain
`required`, `required; eager`, array-level, and union controls, and add an
assertion through the normal inline preparation path so the exact property table
delivered to the agent is pinned. GitNexus reports a CRITICAL transitive radius
for this helper: one direct caller and 21 affected symbols across inline
preparation, looping, service, and compose paths.

### 2. Low: the strict-mode absence fixture claims both array placements but exercises only one

The comment above `STRICT_EAGER_ABSENCE_DOC` says `spec` and `items` cover eager
“at both array placements,” but `spec` is `string(eager)` and `items` is only
`file(eager)[]`; there is no omitted `file[](eager)` property
(`darkmatter/dmls/tests/lsp_session.rs:3694-3712`). Postfix eager absence is
covered by Darkmatter's phase test, and both placements are covered by DMLS
hover tests, so this is not a separate production blocker. Correct the comment,
or add an omitted `file[](eager)` sibling if this fixture is intended to prove
strict-mode diagnostic parity for both placements.

## Review 1 Closure

The prior review's implementation concerns represented in the revised
specification are closed: the authoritative descriptor now separates timing
from presence; DMLS inspects item constraints, postfix array constraints, and
all union arms; both hover surfaces use the descriptor catalog; strict-mode
diagnostics distinguish eager-only from `required; eager`; supplied invalid
values retain typed value ranges; and the named public documentation, active
plan, skill, and behavior-adjacent comments were updated.

The requested previous-review file could not be resolved. `FileReference`
resolution of
`@prompts/_reviews/darkmatter/fixes/2026-09-07-required-vs-eager/review-1.md`
returned no match, and no `review-1.md` exists in this fix directory. Its
`next` and `implemented` properties therefore could not be updated without
fabricating a historical review.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — independent syntax | Level 1 Darkmatter parser/phase tests and a real DMLS LSP session | Appropriate and present. |
| AC2 — strict-mode eager-only absence | Level 1 real DMLS initialize/open/publish-diagnostics session with a `required; eager` control | Appropriate and present. |
| AC3 — supplied eager validation and exact value range | Level 1 real DMLS session with scalar and file-shaped invalid values | Appropriate and present. |
| AC4 — definition and instance hover distinction | Level 1 real DMLS hover requests plus provider unit tests | Appropriate and present. |
| AC5 — catalog-derived completion detail | Level 1 real DMLS completion request | Appropriate and present. |
| AC6 — one wording authority | Level 1 CLI, provider, catalog, and LSP assertions against `schema_constraint_descriptors()` | Appropriate and present. GitNexus reports a CRITICAL radius for the catalog function (10 direct consumers and two CLI flows), all of which compile and pass. |
| AC7 — no DMLS phase fork | Source inspection plus the full Level 1 DMLS suite | Appropriate; no `SchemaPhase`, `validate_for_phase`, or `project_atom` use exists in DMLS production code. |
| AC8 — four-cell matrix, null, invalid values, and array ownership | Level 1 Darkmatter phase tests and real DMLS hover sessions | Appropriate and present. The strict diagnostic fixture's postfix-array comment is inaccurate as described in Finding 2. |
| AC9 — documentation parity | Level 1 shipped-document assertions plus direct inspection | The named Darkmatter/DMLS surfaces agree, but the downstream Claudine prompt and its test still publish the retired completion rule; Finding 1 remains open. |
| AC10 — passive editor behavior | Level 1 DMLS no-side-effects process/socket test and temporary-workspace LSP fixtures | Appropriate and present. No terminal, browser, or OS input behavior is involved. |

Levels 2 and 3 are not applicable. These requirements concern schema
projection, generated Markdown/LSP payloads, diagnostics, and hover/completion
metadata; none depends on terminal-emulator rendering or physical input
encoding.

## Verification Performed

- `darkmatter/ just test`: **7,704 passed; 51 skipped**.
- `darkmatter/ just lint`: **passed**, including `zed-dmls` for
  `wasm32-wasip2`.
- Root `just test claudine`: **7,080 passed; 13 skipped**. This includes the
  stale eager-only prompt-header assertion described in Finding 1.
- `git diff --check` and `git diff --cached --check`: **passed** after the
  review metadata and specification iteration were updated.

## Production Readiness

The DMLS-specific acceptance paths have appropriate Level 1 evidence, and no
Level 2 or Level 3 gap exists. Production readiness is blocked by the prompt
contract mismatch in Finding 1: users can receive a schema verdict that says
eager-only is optional while the agent editing their document is told that the
same property is required.
