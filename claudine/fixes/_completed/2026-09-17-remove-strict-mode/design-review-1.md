# Initial technical design review

Reviewed 2026-09-18. Scope: the initial design containing confirmed decisions D1
and D2, their supporting investigations, the complete functional specification,
and the overlapping live-context requirements. This is an interim review, not
approval for implementation planning. No implementation or specification was
changed.

## Result

D1 and D2 faithfully record the two human selections and are compatible with
this fix's binding and passive-validation requirements. They do not secretly
select the schema format, generator representation, activation syntax, error
types, or prepared-expression API. Pending sections are appropriately labeled;
their incompleteness is not a defect in this initial checkpoint.

Two clarifications should accompany the next design revision:

1. **Define “evaluation” in the cache-lifetime sentence.** The established
   lifetime is one existing lookup operation, not necessarily one expression.
   `SubtreeCompose::compose` constructs one `LayeredLookup` at
   `darkmatter/lib/src/markdown/compose/subtree.rs:386`; its recursive subtree
   processing reuses that lookup. Several expression-bearing leaves can
   therefore share one lazy cache. D2's explicit preservation language is
   correct, but “each evaluation” could otherwise be read as authorizing a
   narrower cache. Record this concrete boundary and leave any change to it
   subject to a separate ruling.
2. **Update graph evidence status without overstating recovery.** The initial
   refresh-timeout account is now historical. `list_repos` during this review
   mapped this worktree to `rusty`, indexed at
   `488b8b2e9cf4eebc02625ba33bc01afcf392fa1a`, and reported two commits behind
   HEAD. The former `better-static-analysis` identifier was rejected. A
   repository-bound context query returned the expected 17 `EvaluationLookup`
   implementors, but `LifecycleCurrent` also had a malformed community-ID
   candidate. This is not a verified clean impact assessment. Retain the
   unresolved-impact qualification and coordinate refresh with the orchestrator.

## Live-context scope question

This discrepancy is consequential and requires explicit human agreement before
designing the corresponding descriptors and providers.

Current code at `claudine/lib/src/composition/lifecycle/context.rs:487-595`
captures `ctx` and process environment before evaluation, exposes
`current.ctx.*` and `current.env.*`, and injects a lazy closure that serializes
an already-captured object. It does not inject `current_env`.

The separate `darkmatter/features/2026-09-09-more-context/spec.md:462-575`
ratifies a different eventual contract: `current.*` and `current_env.*` are
Darkmatter built-in roots across all expression surfaces; observation is lazy
per key; fresh context uses invocation-owned evidence; old nested spellings
are removed without aliases. Its Q2 at line 1032 explicitly leaves memoization
scope unresolved. D2 here must not be represented as resolving that Q2: it
preserves today's generic injected-global cache and does not authorize the
live-context migration.

The current fix names `current_env` among Claudine's lifecycle globals while
also requiring preservation of capture timing. Accordingly, present the next
question as **which feature owns the coherent live-global migration**, not as
an unqualified choice of spelling or a proposed reversal of the ratified
more-context outcome.

- **Retain existing snapshot behavior in this fix; more-context owns the
  complete migration.** Recommended for scope control and timing preservation.
  Requires an explicit clarification to this fix's specification so its
  catalog requirements do not silently promise a newly implemented
  `current_env`. Record that future built-ins will supersede the lifecycle
  injection without changing their ratified destination or inventing an alias.
- **Bring the coherent live-global subset into this fix.** Reasonable if the
  human wants it delivered together. Requires an explicit scope amendment,
  corresponding cross-feature ownership clarification, resolution of Q2, and
  concrete per-key provider/evidence/capture contracts. It is materially more
  than adding another lifecycle global and must cover every expression
  surface, reserved-root collisions, and shell-preflight treatment.

Avoid an interim lifecycle-only `current_env` or direct-mirror alias merely to
make the existing catalog wording convenient. That would create new behavior
without settling timing, built-in ownership, and cross-consumer consistency.
Do not silently edit either specification before the human chooses.

## Subsequent contract checks

These are necessary follow-up design details, not grounds to reopen the agreed
separation:

- The ordinary resolver default must classify reserved namespaces before
  treating a miss as document data. Missing tails may still evaluate to null.
- Checked association must state how execution-dependent declarations map to
  available or unavailable runtime entries, and reject incomplete registration
  without invoking providers.
- Shared passive classification must be structurally independent of runtime
  value lookup, including schema-provided types for absent properties.
- The migration inventory must inspect forwarding wrappers and string helpers
  so a richer failure cannot be flattened back into `Option` or empty text.
- Provider error and cache semantics remain to be selected explicitly;
  declarations/session separation alone does not settle failure caching or
  recursive provider behavior.

No prototype is needed to establish the current-versus-intended distinction;
the relevant behavior and unresolved memoization question are explicit in
source and the related specification.
