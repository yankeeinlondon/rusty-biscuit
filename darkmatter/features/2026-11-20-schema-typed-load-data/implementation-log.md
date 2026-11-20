---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-20-schema-typed-load-data/spec.md"
plan: "darkmatter/features/2026-11-20-schema-typed-load-data/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - "simplified-schema/lib/src/expression/ast.rs"
    - "simplified-schema/lib/src/expression/lexer.rs"
    - "simplified-schema/lib/src/expression/mod.rs"
    - "simplified-schema/lib/src/expression/parser.rs"
    - "simplified-schema/lib/src/simplified/grammar.rs"
    - "simplified-schema/lib/tests/l1/expression_syntax.rs"
    - "darkmatter/lib/src/markdown/compose/cache/remote_cache.rs"
    - "darkmatter/lib/src/markdown/compose/remote_fetch.rs"
    - "darkmatter/lib/src/markdown/compose/expression/absence.rs"
    - "darkmatter/lib/src/markdown/compose/expression/call_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/lint.rs"
    - "darkmatter/lib/src/markdown/compose/expression/member_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/prepared.rs"
    - "darkmatter/lib/src/markdown/compose/preflight/collect.rs"
    - "darkmatter/lib/src/markdown/compose/remote.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/admission.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/evaluator.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs"
    - "darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs"
    - "darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs"
    - "darkmatter/lib/tests/l1/semantic_results_never_persist.rs"
    - "darkmatter/dmls/src/overlay/expressions.rs"
    - "claudine/lib/src/composition/lifecycle/prewarm/classify.rs"
    - "claudine/lib/src/dispatch/matcher.rs"
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - ".claude/skills/darkmatter/remote-cache.md"
packages:
    - "simplified-schema"
    - "darkmatter"
    - "darkmatter-lsp"
    - "claudine"
decisions_for_review:
    - |-
        Phase 1 changes the shared expression tree, which breaks every Darkmatter, DMLS, and Claudine reader of object literals; the plan schedules their spread semantics for Phase 2 but Phase 1 must still compile and pass. I adapted every reader now: pure walkers visit spread operands (what Phase 2 asks for), the evaluator already merges entries in order with a null spread adding nothing, and passive typing treats any literal containing a spread as "some object" (no member or option findings). A non-object spread operand fails at evaluation with the generic catch-all error ("...(): object spread requires an object or null, found number") instead of the planned typed error and code. Alternatives: leave the build broken until Phase 2 (fails this phase's gates), or reject spread at evaluation until Phase 2 (simpler, but a parsed spread would fail every document). Phase 2 replaces the placeholder error with `InvalidSpreadOperand` / `dm.expression.invalid_spread_operand` and the "unknown" typing with ordered spread inference; nothing else needs redoing.
    - |-
        A cache-only remote runtime registers nothing when asked to fetch: `register_and_fetch` returns without creating a slot, so `get_content` reports the URL as never registered, and editor reads must go through `cached_text(url, now)`. The plan only said this path must never register a fetch task. The alternative was to fill the slot synchronously from the cache (hit becomes a ready body, miss a failure), which would let existing `get_content` callers work unchanged but would turn "pending" into a read failure. If the reviewer prefers that, only `register_and_fetch`'s cache-only branch and its tests change.
message_to_agent: |-
    Phase 1 already adapted every object-literal reader to the new `ObjectEntry` / `SpannedObjectEntry` enums so the workspace compiles; Phase 2 "Integrate spread evaluation" replaces these interim pieces rather than starting from tuple walkers:
    (1) `darkmatter/lib/src/markdown/compose/expression/mod.rs` evaluator already merges in written order (last key wins, null adds nothing) but reports a non-object operand as `ExpressionError::Other { function: "...", .. }` - replace it with the typed `InvalidSpreadOperand` + `dm.expression.invalid_spread_operand` and the authored-span projection (test: `lib/tests/l1/more_is_more_literals_and_indexes.rs::object_spread_merges_in_written_order_and_rejects_a_non_object` asserts the current message; update it).
    (2) `call_check.rs`: `expression_type` returns `ScopeShape::Unknown { non_null: true }` for any literal with a spread, and `partial_object` / `literal_value` return `None` for one - replace with Rule 2's ordered inference and literal folding.
    (3) Pure walkers (absence, lint, member_check, prepared, preflight/collect, remote.rs, slots admission/evaluator/grants targets, DMLS overlay/expressions.rs x3, Claudine prewarm/classify.rs x2 and dispatch/matcher.rs) visit `entry.value()` already; Phase 2 still owes their tests (cursor scope / hover inside operands, Claudine classification) and DMLS field-key spans.
    `ObjectEntry` and `SpannedObjectEntry` are re-exported from `darkmatter::markdown::compose::expression`.
    Phase 4: the cache-only API (`RemoteFetchRuntime::cache_only`, `cached_text`, `CacheOnlyPending`, `remote_cache::{lookup_fresh, FreshCachedBody, CacheOnlyMiss}`) is marked `#[cfg_attr(not(test), expect(dead_code, ...))]`; remove those attributes when the editor consumes them or the build fails with an unfulfilled expectation. A cache-only runtime's `register_and_fetch` registers nothing.
---

# Implementation Log for 2026-11-20-schema-typed-load-data (5 phases)

## Phase 1

### Add spread syntax (Simplified Schema)

- `expression/lexer.rs`: new `Token::Ellipsis` for three adjacent dots (one
  token, three-byte span). A lone `.` is still `Token::Dot`.
- `expression/ast.rs`: `Expr::ObjectLiteral(Vec<ObjectEntry>)` and
  `SpannedExprKind::ObjectLiteral(Vec<SpannedObjectEntry>)`, each enum with
  `Field { key, value }` / `Spread { operand }` and a borrowed `value()`
  accessor; `SpannedObjectEntry::erase`, and `Display` prints `...operand`.
  Both enums are re-exported from `simplified_schema::expression` and from
  `darkmatter::markdown::compose::expression` (its public `Expr` names them).
- `expression/parser.rs`: `parse_object_literal` accepts `... expression` only
  at the start of an entry. The explicit-key `HashSet` survives across spreads,
  so `{ a, ...rest, a }` is still `duplicate object key "a"` at the second key.
  `[...x]` is rejected with "array spread is not supported"; `...` anywhere
  else is the ordinary "expected expression" error. Grammar doc updated.
- `simplified/grammar.rs`: the literal-example extractor rejects a spread at
  any depth ("`...base` is not a literal; a spread is not allowed in a literal
  example").
- `expression::scan` is unchanged and still asks the parser; a test pins
  `{{ {...base, nested: {k: 1}}}}` closing at the right `}}`.

### Interim consumer adaptation (needed for Phase 1 to compile)

The AST change breaks every tuple-destructuring consumer, and Phase 2 owns
their real spread semantics. To keep Darkmatter, DMLS, and Claudine compiling
and green in this phase:

- pure walkers (absence, call-node visits, prepared, member_check pending
  lists, preflight `visit_calls_in_scope`, `remote.rs`, slot admission /
  evaluator / grant targets, DMLS `overlay/expressions.rs` x3, Claudine
  `prewarm/classify.rs` x2 and `dispatch/matcher.rs`) now visit
  `entry.value()`, i.e. spread operands too — the behavior Phase 2 asks for;
- `lint.rs` flags only written keys and substitutes inside spread operands;
- `call_check.rs`: `expression_type` returns `Unknown { non_null: true }` for a
  literal containing a spread, `partial_object` and `literal_value` return
  `None` for one (no passive option inspection, never a literal);
- the evaluator (`expression/mod.rs`) merges entries in written order, last
  key wins, `null` spread contributes nothing; any other operand fails with
  `ExpressionError::Other { function: "...", message: "object spread requires
  an object or null, found <type>" }` as a placeholder for Phase 2's typed
  `InvalidSpreadOperand` / `dm.expression.invalid_spread_operand`.

### Add cache lookup (Darkmatter)

- `cache/remote_cache.rs`: `read_cached` is split into a side-effect-free
  `decode_cached` (Absent / NoStore / Unusable / Usable) shared by both paths;
  the fetching path still purges a `no-store` entry exactly as before.
  New `lookup_fresh(store, url, now) -> Result<FreshCachedBody, CacheOnlyMiss>`
  serves only a current-version, decodable, non-`no-store`, non-`no-cache`
  entry whose `expires_at` is after `now`, and returns body plus expiry.
- `remote_fetch.rs`: `RemoteFetchRuntime::cache_only(policy, network, store)`
  builds a runtime whose `register_and_fetch` / `register_nested` register
  nothing (no task, no slot); `cached_text(url, now)` checks scheme
  (HTTP(S)), then the `network` configuration error, then the host policy
  (`check_allowed`), and only then opens the store, mapping every miss to
  `CacheOnlyPending::{NotHttp, Refused, NoStore, Miss(..)}`.
  `with_other_policy` keeps a cache-only runtime cache-only. The store root is
  whatever the caller passes; Phase 4 opens
  `FileStore::resolve_cache_root(context.base_dir(), None)`.
- The new items carry `#[cfg_attr(not(test), expect(dead_code, …))]` until
  Phase 4 consumes them; the `expect` will fail the build once they are used,
  so it must be removed then.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| spread order, shorthand, operand is a full expression | `parser::tests::object_spread::{entries_keep_written_order, spread_operand_is_a_whole_expression}`; L1 `expression_syntax::object_spread_keeps_entry_order_and_operand_spans` |
| explicit duplicates across a spread (incl. shorthand / quoted) rejected at the second key | `object_spread::written_duplicates_separated_by_a_spread_are_rejected`; L1 `object_spread_rejects_written_duplicates_and_array_spread` |
| a spread never conflicts with a written key at parse time | `object_spread::spread_may_repeat_a_written_key` |
| nesting | `object_spread::nested_spreads_parse_inside_operands_and_values` |
| interpolation closer with adjacent `}}}}` | L1 `a_spread_with_nested_objects_closes_at_the_interpolation_closer` |
| `...` rejected outside an object entry, array spread, malformed entries | `object_spread::{spread_is_rejected_outside_an_object_entry, malformed_spread_entries_are_rejected}` |
| spans (operand, key, value) and erasure parity | `object_spread::spread_spans_cover_the_operand_and_keys_keep_theirs`; `spanned::erasure_matches_parse` corpus (+2 spread rows) |
| catalog example literal rejects spread at any depth | L1 `a_catalog_example_result_rejects_spread_at_any_depth` (control + 4 shapes) |
| interim evaluator merge / null / wrong type | darkmatter L1 `more_is_more_literals_and_indexes::object_spread_merges_in_written_order_and_rejects_a_non_object` |
| cache-only: fresh hit with expiry | `remote_fetch::cache_only_tests::a_fresh_entry_returns_its_body_and_expiry` |
| stale (== now, < now, no lifetime) | `…::an_entry_expiring_now_or_earlier_or_without_a_lifetime_is_expired` |
| miss / no store | `…::a_missing_entry_or_store_is_pending` (also: no store directory created) |
| denied host before cache | `…::a_denied_host_is_refused_before_the_cache_is_read` |
| invalid permission block outranks an allowed host | `…::an_invalid_network_permission_outranks_an_allowed_host` |
| no-store (current + legacy version), no-cache | `…::no_store_and_no_cache_entries_are_never_served_or_purged` |
| corrupt body / missing blob / other version / bad manifest JSON | `…::corrupt_or_foreign_entries_are_unusable` |
| non-HTTP URL | `…::a_non_http_url_never_reaches_the_cache` |
| slot-derived runtime stays cache-only | `…::a_derived_slot_runtime_stays_cache_only` |
| zero requests, zero writes, no registration | every cache-only test goes through `observe`: wiremock `.expect(0)` + `verify`, byte+mtime snapshot of the store before/after, `registered_urls()` empty, `get_content` = `Ok(None)` |
| normal path still purges `no-store` | existing `remote_cache` / `persistent_cache_tests` (all pass) |

The deterministic clock is an injected `now` argument (`UNIX_EPOCH + 1e6 s`).
The input-robustness matrix does not apply to this phase: no file-format or
configuration reader changed (the cache manifest decode was refactored, not
changed, and its shapes are covered above).

### Other changes found by the gates

- `darkmatter/lib/tests/l1/semantic_results_never_persist.rs`: its exact
  `FileStore` allowlist counts rose with the new transport-cache code
  (`remote_cache.rs` 6 → 8: shared decode + `lookup_fresh`; `remote_fetch.rs`
  5 → 6: `cache_only`). Every new mention is transport-cache code, so no
  semantic result gains a path to disk. The comments now name the new uses.
- clippy `large_enum_variant`: `StoredEntry::Usable` holds a `Box<CachedEntry>`.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-lsp, simplified-schema, test-toolkit, darkmatter-cli]`,
  `change_class: package`, so the gates ran.
- `simplified-schema`: `just test` 2017/2017 passed; `just lint` clean;
  `just check-tier-coverage simplified-schema` 0 stranded.
- `darkmatter`: `just test` 17274/17274 passed, none at or above 5 s;
  `just lint` clean (including the zed-dmls wasm check);
  `just check-tier-coverage darkmatter` 0 stranded. The opt-in `prompts`
  binary (`--features prompt-tests`) compiles.
- `claudine` (edited directly): `just test-library` 6932/6932 passed, none
  slow; `just lint` clean.
- No other workspace crate names `ObjectLiteral` (repo-wide search), and the
  cache API is `pub(crate)`, so no further dependent can be affected.

### Problems that existed before this phase

- `horizontal_rule_integration::tests::test_custom_weight_thick_differs_from_thin`
  fails when the test process runs with `LANG=C`, as this agent shell did
  (`TERM_PROGRAM=rex`). The terminal rule falls back to ASCII, and thin and
  thick then render the same `-`. It passes alone and in the full suite under
  `LANG=C.UTF-8`, which is how the recorded runs above were made. It is not
  caused by this phase: the test reads the ambient locale, so it depends on the
  host it runs on.
- `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
  showed as modified during this session. This phase did not edit it, and it
  is left alone.
