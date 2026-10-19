---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-17T21:15:45-07:00
spec: 2026-10-26-task-inputs-and-providers/spec.md
feature: 2026-10-26-task-inputs-and-providers/review-1.md
next: 2026-10-26-task-inputs-and-providers/review-2.md
implemented: true
description: A **feature** review of `2026-10-26-task-inputs-and-providers/spec.md`
findings: []
observations: []
recurrence: false
human_resolved: true
human_review: true
has_blocked_findings: false
blocked: false
human_in_the_loop:
    - |-
        Decide how much editor validation to trade for faster launches. The implementation log reports that fully typing container lifecycle actions and loops costs about 3.27 seconds per release validation of a plain prompt, compared with 0.20 seconds before this work. It therefore checks most container events only for their outer shape, checks cancellation actions at the first two container levels, and fully types loops only at the first level. The specification promises typed validation through three container levels. This can let an author discover an invalid action only when running the document.

        - Accept the narrower editor checks temporarily, retaining runtime parser checks and documenting exactly where editor validation stops. This avoids the measured launch penalty.
        - Keep the specified checks and accept slower validation, or optimize schema loading before retaining those checks. This preserves earlier feedback but adds runtime cost or implementation work.

        Recommendation: accept a clearly documented temporary boundary if faster launches are the priority. Record that choice explicitly; the already agreed untyped boundary after three levels does not authorize weakening checks inside those levels. Also confirm the logged choice to leave shell, side-effect, and external-task field validation to the parser and retain numeric/boolean step shorthand.
    - |-
        Decide whether sequence documents should offer lifecycle-event completion before an event is written. The implementation gives sequence fields exclusively to the sequence schema and root lifecycle/loop fields exclusively to the general Claudine schema. The log says this removes duplicate validation work, but an otherwise plain sequence document receives no event-key completion until an event activates the general schema. For example, an author starting with only a sequence cannot discover a start event through the promised schema coverage.

        - Accept the completion limitation and retain the single declaration of each field. This keeps schema loading cheaper.
        - Restore the specified root declarations and identical shared declarations, or make completion discover them without duplicate validation. This preserves editor assistance but costs loading time or additional implementation work.

        Recommendation: accept the single declaration of each field only with an explicit decision that delayed completion is an acceptable temporary limitation.
    - |-
        Decide whether runtime rejection is sufficient for malformed values the schema accepts. The log reports that sequence: 42 activates the schema but passes validation because the schema engine converts numbers to strings; the runtime sequence parser rejects it. Root fail_fast: null also passes schema validation but is rejected by the parser. Group-key errors name the enclosing group and include its value, rather than locating the offending key individually. The specification explicitly requires malformed numeric sequence values to fail schema validation.

        - Accept these editor-validation and diagnostic limitations, documenting that runtime parsing remains authoritative. This avoids expanding this feature into schema-engine changes.
        - Require the specified schema rejection and more precise diagnostics before accepting the schema work. This may require changes in Darkmatter, the package that loads and validates these schemas.

        Recommendation: document the bounded limitations rather than add a special validation layer solely for these cases. Explicitly accept or reject the numeric-sequence departure before the implementation is judged against the specification.
human_review_items:
    - Resolve the three schema tradeoffs in human_in_the_loop, recording the chosen behavior and any required follow-up under Human rulings.
log: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/log.md
implemented_by: claude/opus
---

# Review 1: author decisions required

This review stops at the requested first-cycle decision check. Two independent
reviewers read the specification and implementation log separately. The
specification has no open questions, but the implementation log records
meaningful departures from its agreed editor-validation behavior. The three
decisions above need the author's ruling before a code review can use a stable
contract.

Production readiness has **not been assessed**. There are no code findings
because implementation comparison, reproduction, and test verification have
not started. The `ready` property is intentionally unset on this early-exit
review: assigning either readiness verdict would imply an assessment that this
review path explicitly postpones. The specification's `review_iterations` is
now 1; its completion state is unchanged.

## Basis for the requested decisions

The [schema design](spec.md:300) requires shared lifecycle types and three
typed container levels. The [implementation log's cost measurements and
departures](implementation-log.md:1165) explain the weaker validation,
separated root declarations, delayed completion, and numeric coercion. These
are reported implementation choices, not independently verified code findings.
The measurements come from a host running other work, so they establish the
agent's rationale rather than a controlled performance guarantee.

The three-level limit itself is already authorized in the specification.
Unlimited runtime nesting and later recursive schema support do not need to be
reapproved. The new decision is how much checking to retain *within* those
three levels.

## Other implementation decisions and deferred investigation

- Null and empty provider hints use the existing provider resolver: `agent:
  null` and `agent: []` mean no own provider hint, so fallback or provider
  selection applies. The log resolves a conflict with the plan in favor of the
  specification's requirement to reuse the canonical resolver. This does not
  require a separate design decision.
- Provider tests use Claude and Goose stubs instead of the named Codex and
  OpenCode stubs because the latter hung in the test helper. Whether the
  resulting tests prove each requirement remains for the code review.
- Sequence provider targets stop refreshing model catalogs during resolution.
  This keeps preflight free of provider contact, as required; the log also
  records the loss of earlier cache warming. Confirming the launch behavior
  remains part of the subsequent code review.
- Dependency-provided error names and cancellation types were reused, schema
  trigger syntax was adapted to what the loader accepts, and an amendments
  file had already been removed. These are routine integration choices.
- Task and lifecycle inputs share shape validation but retain separate
  evaluators. Static task diagnostics retain `sequence[N]` property paths and
  runtime task diagnostics retain `tasks[N]`. The log describes these as
  implementation choices preserving existing behavior; their correctness
  remains for the implementation review.
- The log [defers an unverified retry-model concern](implementation-log.md:1375):
  if a prompt removes its model between attempts, a retry might retain the old
  model through a fallback or environment value. This calls for reproduction
  during code review, not a human design ruling. The intended behavior already
  requires rebuilding launch identity.
- The same follow-up section records slower validation for sequence documents
  and pre-existing subsecond timeout text displaying whole seconds. The former
  informs the schema tradeoff; the latter is outside this feature's changes.
  The log also leaves the test-audit JavaScript suite to CI because dependencies
  were unavailable locally. These are disclosed verification limits, not new
  requests for author decisions.

## Human rulings

Ruled by Ken Snyder on 2026-10-18. The implementation is judged against these
rulings where they depart from the specification.

1. **Narrower editor checks inside the three typed levels: accepted as a
   temporary limit.** Container events are checked for shape only,
   `cancelled` is typed on levels 1–2, and the shared `loop` type is used on
   level 1 only (an object below). The cost is paid by every Claudine launch,
   not only by sequence documents (0.24 s against 3.27 s for a plain prompt,
   release `md`). The limit ends when 2026-10-30-claudine-recursive-schemas
   lands; that spec already commits to typing every container event at every
   depth. Also confirmed: `shell`, `side_effect`, and `task` stay `unknown`
   and are checked by the parser, `prompt` stays `string`, and `step` keeps its
   number/boolean shorthand arms, all matching the types `lifecycle.yaml`
   previously shipped. `docs/topics/schemas.md` (Sequence containers)
   already documents where editor checks stop.
2. **One declaration per key: accepted, with a follow-up.** `sequence.yaml`
   alone declares `sequence`/`template`/`fail_fast`, and `claudine.yaml` alone
   declares the root events and `loop`. Late completion of event keys in a
   pure sequence document is accepted. The measured release cost of
   duplicating the declarations is small (about 0.05 s), but one declaration
   per key also prevents drift. Follow-up: `_unscheduled/cross-trigger-completion`
   (completion offers keys from sibling triggers without validating against
   them). The limitation is now documented in `docs/topics/schemas.md` under
   "In the editor".
3. **Values the schema accepts but the parser rejects: accepted as a departure
   from AC9.** `sequence: 42` (converted to the string arm by Darkmatter's
   default type conversion) and root `fail_fast: null` pass the schema. The
   sequence parser rejects both before any step launches. Group-key errors
   are reported at `/…/group`, with the offending key in the message. The
   runtime parser has the final say; `docs/topics/schemas.md` already says so
   for each case. No Darkmatter change is required by this feature.
