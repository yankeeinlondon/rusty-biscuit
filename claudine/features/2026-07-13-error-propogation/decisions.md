---
created: 2026-07-16
phase: 1
status: ratified
---

# Decisions — End-to-End Typed Error Propagation

Rulings taken during execution of [`plan.md`](./plan.md). Each records what was
decided, the authority for the decision, and the condition that would reverse it.

---

## D-1 — Cross-crate diagnostic identity lives in Claudine (Option A)

**Decision.** Claudine owns semantic adapters. Every lower-layer error
(Darkmatter, biscuit-file, `std::io`) that crosses into Claudine is retained as
`#[source]` by a Claudine diagnostic wrapper. That wrapper either owns its
facets and `BlockError` rendering (a *semantic* boundary that deliberately
reclassifies the operation) or transparently delegates both to its cause (a
*transparent* wrapper). Effective-diagnostic selection consults the Claudine
registry before falling back to Darkmatter's `as_block_error` registry for
lower-layer causes.

**Authority.** [`spec.md`](./spec.md) §"Open Questions" → "Where should
cross-crate diagnostic identity live?". The spec names Option A as its own
recommendation, states plainly that "D2–D10 assume Option A", and records that
Option B "should be ratified as a separate cross-package contract before
implementation". Option C is explicitly excluded by the spec ("Option C should
not be used").

**Consequences accepted.**

- Every lower-layer ingress into Claudine must be wrapped. This is the cost that
  buys the dependency direction.
- Direct Darkmatter callers *outside* Claudine do not gain Claudine facets. This
  is acceptable: they already render through Darkmatter's own `BlockError`.
- Careless adapters could duplicate detail across the wrapper and its cause. The
  catalog-parity test (Phase 6) is what prevents this, not reviewer diligence.

**Reversal condition (Option B exit).** If maintainers prefer Option B — moving a
provider-neutral diagnostic facet trait down beside `BlockError` in
`biscuit-terminal` and implementing it directly in Darkmatter and biscuit-file —
then this plan **stops at Phase 1**. Option B invalidates D2's crate-local
registry, turns a Claudine transport fix into a cross-area public API change in
`biscuit-terminal`, and forces a ruling on who owns the category/code catalogs.
That is a separate specification, not a phase of this one. Nothing in Phases 2–8
should be started while Option B is under consideration, because Phase 2's
registry is precisely the artifact Option B deletes.

---

## D-2 — The two proxy routes do not share a resolver (discovered, not decided)

This is recorded here because it materially changes what Phase 7's cross-route
parity criterion can assert, and it was not visible from the spec.

**Finding.** Spec Acceptance Criterion 5 requires the same proxy failure to have
"identical diagnostic identity, headline, hint, and available typed resolution
detail regardless of which supported lifecycle route initiated the handoff".
Characterization against `HEAD` shows the two routes do not currently fail in the
same *place*, let alone with the same identity:

| Route | Resolver | Where it fails on a missing target |
|---|---|---|
| `initialize` proxy | `claudine::composition::resolve_proxy_target` | At resolution — the resolver probes existence and returns `HarnessError::PathResolutionFailed` |
| terminal/recovery proxy | `claudine::harness::resolve_harness_path` | **Not at resolution.** The path resolves successfully; the run announces "flow control redirected to …" and then fails later in pre-flight with an `io::Error` read failure |

Their observed `HEAD` surfaces are therefore unrelated:

```text
# initialize route
Error: lifecycle initialize proxy: path resolution failed for "…": proxy target does not exist: …

# failure route
Error: pre-flight shell approval failed: proxy target pre-flight: failed to read '…': No such file or directory (os error 2)
```

**Implication for Phase 7.** Typing the errors is necessary but *not sufficient*
for AC5. Two routes that fail at different stages against different resolvers
cannot be made to agree on code/headline/hint by wrapping alone — the terminal
route would still be reporting a pre-flight read, not a resolution miss.

**Ruling.** Converging the two routes onto one resolver is a **routing change**,
and D10 requires that any routing change discovered mid-audit is split into a
separate spec rather than fixed here. Phase 4/5 must therefore preserve both
routes' current failure *stages*. If Phase 7 finds AC5 unsatisfiable without
converging the resolvers, that is the trigger to raise the separate spec — not a
license to change routing inside this feature.

**Note.** The downstream [`2026-07-13-file-resolution`](../2026-07-13-file-resolution/spec.md)
feature unifies file-reference resolution and is the natural home for this
convergence. Sequencing it after this feature keeps the dependency edge acyclic,
exactly as this spec's reader's note requires.
