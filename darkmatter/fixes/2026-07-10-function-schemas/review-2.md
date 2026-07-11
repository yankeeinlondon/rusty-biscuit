---
$schema: "@.claudine/schemas/review.yaml"
ready: false
agent: codex/default
created: 2026-07-11T14:15:32
implemented: true
---

# Review: Authored Expression-Function Schemas (Iteration 2)

## Verdict

Not ready for production.

The three findings from review iteration 1 are implemented: production parsing now validates the authored SimplifiedSchema, valid fixture parsing remains owned, and pure/context handlers are gated by catalog-derived arity. The full affected Level 1 suites and lint pass. One runtime authority gap remains for lazy functions, and one new regression test materially degrades the ordinary Level 1 suite.

## Findings

### High: lazy-function dispatch still bypasses catalog-derived arity

Requirement 7 applies to pure, context-aware, and lazy dispatch paths. Pure and context bindings now call `accepts_arity` before their handlers (`functions/mod.rs:2489-2546`), but `evaluate_function` handles `and` and `or` directly before consulting the joined binding or any descriptor (`expression/mod.rs:639-665`). The implementation itself confirms that the registry gate is “structurally unreachable” for those functions (`functions/mod.rs:5000-5005`).

The current YAML describes both functions as a single variadic parameter, so their existing behavior happens to accept every arity. That coincidence does not establish catalog authority: changing a lazy function's authored parameter shape would update its descriptor, DMLS, and documentation while runtime eligibility would remain unchanged. The lazy tests only prove that the names resolve at one manufactured arity; they do not prove that the evaluator derives eligibility from the authored shape.

Move arity eligibility behind a shared joined-registry query usable before lazy evaluation, or expose a registry operation that resolves the binding and eligible overload without eagerly evaluating arguments. Add Level 1 fixtures/unit tests that exercise lazy eligibility from parameter shapes independently of today's all-arity catalog entries, plus end-to-end evaluator checks at the minimum and representative variadic arities. This preserves short-circuit evaluation while making the catalog the actual signature authority.

### Medium: the owned-fixture leak regression makes the normal Level 1 suite unreasonably slow

`try_parse_catalog_returns_owned_catalog_without_leaking` reparses and recompiles the self-declared JSON Schema 1,000 times. In the canonical `just test` run it took approximately 24 seconds by itself. It is not named `slow_*`, so it also remains in `sanity`, whose repository contract is a fast confidence run of no more than roughly 15 seconds.

The loop cannot directly prove absence of a leak; it proves repeatable success while imposing substantial cost. Reduce the iteration count to a small regression value and rely on the owned return type/code boundary for the lifetime guarantee, or move a deliberately high-volume check to an appropriately named slow test or memory-analysis workflow.

## Requirement Verification and Test Rigor

- Requirements 1-6 and 8-10 are verified at Level 1 through embedded-catalog parsing, structural and semantic malformed fixtures, descriptor projections, registry parity, generated-document comparison, CLI assertions, and DMLS completion/hover assertions.
- Requirement 7 is verified at Level 1 for pure and context functions, including fixed, optional, overloaded, and out-of-range arities. Its lazy-dispatch branch is not adequately verified and does not use catalog-derived eligibility; this is the high-severity gap above.
- `md schema about` has Level 1 content assertions and Level 2 tmux presentation coverage. No keyboard encoder, paste, IME, mouse, or other input-device behavior is part of this fix, so Level 3 is not applicable.
- Generated Markdown and DMLS metadata are semantic outputs; Level 1 comparison/assertion is the appropriate verification level.

## Validation Performed

```text
just test
  darkmatter: 5,475 passed
  darkmatter-cli: 552 passed
  dmls: 412 passed

just lint
  passed for darkmatter, darkmatter-cli, and dmls
```

Level 2 was not rerun in this iteration because the remaining blocker is an in-process evaluator authority issue, not terminal rendering. Existing Level 2 schema-about coverage is appropriate for the unchanged terminal presentation requirement.
