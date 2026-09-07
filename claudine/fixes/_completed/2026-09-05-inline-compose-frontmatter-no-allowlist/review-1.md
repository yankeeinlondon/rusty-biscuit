---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-07T13:54:51-07:00
spec: 2026-09-05-inline-compose-frontmatter-no-allowlist/spec.md
log: claudine/fixes/2026-09-05-inline-compose-frontmatter-no-allowlist/log.md
implemented: true
implemented_by: codex/default
next: 2026-09-05-inline-compose-frontmatter-no-allowlist/review-2.md
description: A **fix** review of `2026-09-05-inline-compose-frontmatter-no-allowlist/spec.md`
fix: 2026-09-05-inline-compose-frontmatter-no-allowlist/review-1.md
---

# Review 1: Inline Compose Frontmatter Without an Allowlist

## Verdict

The fix is **not ready for production** at its implementation snapshot
(`6ac33e7eb`). The allowlist was removed, closure-owned properties are handled
as specified, shipped guardrails migrate correctly, and the failure paths
remain intact. However, the implementation does not preserve response order
for newly returned frontmatter properties, directly violating AC1.

The response-block channel reviewed here was subsequently and intentionally
superseded by `2026-09-05-inline-flow-and-validations`, which retained the
no-allowlist ruling but restored direct agent edits to the source file. That
later design change is not treated as a regression in this review.

## Findings

### Medium — Parsed response properties are sorted instead of kept in response order

`extract_replacement_parts` parses YAML into `serde_json::Value` and then
collects `map.iter()` into an `IndexMap` (`closure.rs:73-92` at
`6ac33e7eb`). The resolved Claudine feature graph does not enable
`serde_json/preserve_order`, so `serde_json::Map` is backed by `BTreeMap` and
iterates keys in lexical order. `apply_inline_closure` and
`rewrite_harvested_frontmatter` faithfully retain that already-sorted order
(`closure.rs:168-189, 393-403`), not the order in the provider response.

For example, a response that returns `zeta` before `alpha` writes `alpha`
before `zeta`. AC1 explicitly requires new keys to be inserted in response
order before `last_updated`, so the written artifact and the corresponding
status sequence are wrong even though all values survive.

The order test does not cross the parser boundary: it manually constructs an
`IndexMap` in the desired order before calling `rewrite_inline_document`
(`closure/tests.rs:153-165`). The closure and CLI fixtures also happen to use
keys already in lexical order (`generated_by` before `title`, and
`access_points` before `generated_by`), so they cannot reveal the defect.

Preserve order at the YAML parse boundary. Prefer deriving the property order
from the already source-ordered `top_level_key_locations` result, or deserialize
to an order-preserving YAML mapping, rather than enabling a workspace-wide
Serde JSON feature solely for this path. Add a parser-to-write regression with
reverse-lexical keys and assert both file order and status order.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: replace an existing key, insert two new keys in response order, preserve `prompt`, and ignore returned `hash`/`last_updated` | Level 1 closure tests | **Gap.** Merge and closure-owned behavior are covered, but the order assertion bypasses response parsing and therefore misses the defect above. |
| AC2: refresh a previously inserted key in place without duplication | Level 1 closure test and Level 1 CLI subprocess test | Appropriate and present. |
| AC3: migrate the byte-equal 2026-09-01 guardrails and preserve customized guardrails | Level 1 real-filesystem unit tests, including an injected atomic-write failure | Appropriate and present. |
| AC4: provider-stub first/second run without an allowlist, prompt-byte preservation, property insertion/refresh, and delivered guardrail instruction | Level 1 Unix CLI subprocess test | Appropriate for the behavior and present on macOS/Linux. The core parser and rewrite tests are platform-independent; no terminal emulator or input encoder is involved. |
| AC5: malformed YAML, duplicate keys, non-mapping roots, missing bodies, and unchanged bodies fail without writing | Level 1 parser and real-filesystem closure tests | Appropriate and present. |

Levels 2 and 3 are not applicable. The requirements concern parsed response
data, filesystem mutation, captured status text, and prompt bytes. They do not
claim terminal glyph geometry, styling fidelity, scrolling, or physical input
encoding.

## Additional Review Notes

No additional correctness, ergonomics, or performance defect was found in the
target snapshot. Removing the allowlist also removed its plan state,
prepare-time validation, warning machinery, and typed error rather than leaving
dead compatibility plumbing. `CLOSURE_OWNED_PROPERTIES` centralizes the three
exceptions, and the textual rewrite continues to preserve authored nodes while
stamping a consistent Darkmatter `Simple` hash.

## Verification Performed

- Historical Claudine Level 1 run at `6ac33e7eb`: **4,390 passed, 1 unrelated
  existing test timed out, 11 skipped, and 2,330 were canceled after the
  timeout**. All target closure tests completed successfully before that
  timeout.
- Focused historical CLI test
  `inline_compose_harvests_and_refreshes_response_frontmatter`: **passed**.
- Resolved Cargo feature audit: Claudine does **not** enable
  `serde_json/preserve_order`; the default map implementation is a `BTreeMap`.

## Production Readiness

AC1 is not satisfied end to end. The implementation snapshot is therefore not
ready for production, even though AC2-AC5 have verification at the appropriate
Level 1 tier.
