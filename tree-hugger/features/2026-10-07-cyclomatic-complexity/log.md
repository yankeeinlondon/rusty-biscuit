---
feature: 2026-10-07-cyclomatic-complexity
implementation_1: "2026-10-05T21:33:11-07:00"
---

# Cyclomatic Complexity — Log

## Implementation of Review Findings #1

> **started at:** 2026-10-05T21:33:11-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-07-cyclomatic-complexity/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Type syntax inside callable bodies inflates complexity' at 21:33:44
        - defect class: the complexity traversal descended into type-level syntax (type annotations, type arguments, cast/assertion/satisfies targets, type queries) embedded in a callable body and counted value-shaped decisions there
        - enumerated type positions from the pinned grammars' `node-types.json` (Cargo.lock resolves tree-sitter-rust 0.24.2, not the 0.24.0 named in `Cargo.toml` and `rust.rs`; its `_type` supertype is identical): Rust `_type` subtypes plus `type_arguments` and `bracketed_type`; TypeScript/TSX `type` and `primary_type` subtypes plus every `*_annotation` kind and `type_arguments`
        - discovery: kind pruning alone is not enough for TypeScript `as`/`satisfies`, whose target may be a value-shaped kind (`{} as import(a ? 'a' : 'b')` parses its target as `call_expression`); added `Rules::type_sides`, which prunes the children after the `as`/`satisfies` keyword
        - kinds shared with values are deliberately not pruned by kind: Rust `macro_invocation`/`metavariable` (a macro's tokens are already opaque), TypeScript `call_expression`/`member_expression`/`const` (reached only under an annotation, type arguments or a type side)
        - type regions are pruned but error-checked (like `opaque`): an error inside an in-body type still makes the total `null`, as before
        - new unit test `complexity::tests::every_grammar_type_kind_is_pruned_or_a_shared_value_kind` reads each grammar's `NODE_TYPES` (the TS 0.23.2 grammar is ABI 14 and reports no supertypes at runtime) so a grammar bump that adds a type kind fails
        - every sweep-table row is now a `symbols_metrics` row (TS rows run through `.ts` and `.tsx`; angle-bracket assertions in `TS_ONLY_CYCLOMATIC`, `.ts` only), with value controls beside each type position (cast operand, turbofish call arguments, array repeat length, inline `const` block = 2, object values, `as` operand, `<T>` operand); confirmed the new rows fail with the pruning disabled
        - `symbols_query::COMPLEXITY_SOURCES` gained `types.rs`/`types.ts`/`types.tsx`, so selective vs complete queries and the cache's widen-then-warm-`Hit` test cover type syntax
        - `EXTRACTOR_REVISION` 28 → 29; `cache_upgrade::entry_from_extractor_revision_twenty_eight_is_not_served_after_the_type_syntax_repair` (Rust and TS); `symbols.json`/`symbols.jsonl` goldens regenerated (only `rev.28` → `rev.29` in provenance); `docs/cache-design.md` and the tree-hugger skill revision list updated
        - docs: `docs/cyclomatic-complexity.md` gained "Type syntax never adds" with Rust and TS examples, plus Rust/TS table rows and a flowchart edge; rule-table module docs updated
        - class sweep: type syntax inside callable bodies; sites checked: Rust let annotation (array, reference, tuple, pointer, fn-pointer), generic type arguments, call and value turbofish, cast target, qualified path `<T as Tr>::f`, nested alias/struct, header types, nested const item, inline const block, array-repeat value; TS/TSX object/array/tuple annotations, `as` target (object and `import(…)`), `satisfies` target, `<T>x` (.ts), call type arguments, `typeof import(…)` query, header types, local alias/interface, conditional type, nested class, object value; JavaScript (no type grammar; anonymous class row added); other 13 languages (no rules); fixed here: Rust annotations, type arguments, turbofish, cast target, bracketed path type; TS/TSX annotations, type queries, type arguments, `as`/`satisfies`/angle-bracket targets; clean: header types, named nested types, nested const item, inline const (2 by convention), array-repeat value, conditional type, nested class, object value, JavaScript, other languages
        - `just test`: 1101 passed, 0 failed; `just lint`: clean (exit 0); `just check-tier-coverage tree-hugger`: 0 stranded
- work completed for 'Type syntax inside callable bodies inflates complexity' at 21:43:46
- starting the work on 'Default groups change complexity when their labels are reordered' at 21:44:26
        - discovery: `Counts::Arm` decided a `switch_case` from that case alone (its `body` field, or being the last label), so a `switch_default` earlier in the same bodyless run was never seen: `default: case 1: break;` reported 2
        - `Counts::Arm { body, default }` now counts a group once, at the label that ends it (has a body, or is the last label), and only when no label of the group (walking back until a label with a body) is the `default` kind; comments and other extras (JS `html_comment` included, via `Node::is_extra`) are skipped in both directions
        - rule-table comments (`Counts::Arm`, `javascript.rs` module doc and `switch_case` entry) describe the group-based rule; `docs/cyclomatic-complexity.md` states that a `default` anywhere in a group makes it add nothing, with a `groups` example pinned as a matrix row; the tree-hugger skill names the rule and the new table
        - new `symbols_metrics::SWITCH_GROUP_CYCLOMATIC` (run as `.js`, `.ts`, `.tsx`, LF/CRLF, both query paths, clean-parse asserted) holds every switch row of the review's sweep table plus a two-group control; Rust rows `grouped_pattern` (`1 | 2 => 1, _ => 2` = 2) and `wildcard_first` (guarded `_` before an or-pattern = 4) join the existing `two_arms` (= 2); confirmed the new rows and the cache test fail with the default check disabled
        - `symbols_query::COMPLEXITY_SOURCES` gained `switches.js` (`default: case 1: break; case 2: case 3: break;` = 2), so selective/complete queries and the cache's widen-then-warm-`Hit` test cover label groups
        - `EXTRACTOR_REVISION` stays 29 (unreleased, shared with the type-syntax repair); the revision-28 cache-upgrade test was renamed `entry_from_extractor_revision_twenty_eight_is_not_served_after_the_complexity_repairs` and gained a JavaScript `default: case 1:` source (stale 2 → recomputed 1); its probe function is now `measured` in all three sources; goldens needed no change
        - defect class: a grouped-branch rule decided an arm's contribution from one member of the group (its position or its last label) instead of from the whole group's membership
        - class sweep: grouped-branch counting decided from one group member; sites checked: `Counts::Arm` (JS `switch_case`, shared by TypeScript/TSX through `javascript::DECISIONS`), `Counts::UnlessLast` (Rust `match_arm`), Rust or-patterns (one `match_arm` node), Rust guards (`Counts::WithField` on `match_pattern`), the Rust last-arm default convention, `Counts::Always`/`Operator`/`Tokens`/`WithField`/`UnlessFieldKind` (single-node rules, no grouping), the other 13 languages (no rule table); fixed here: JS/TS/TSX switch label groups with a `default` before the last label (including a final bodyless group and comment-separated labels); clean: Rust match arms (no label grouping: an or-pattern is one node, the last-arm convention is positional by documented design), Rust guards, single-node rules, other languages
        - `just test`: 1102 passed, 0 failed; `just lint`: clean (exit 0, after replacing a redundant closure clippy flagged); `just check-tier-coverage tree-hugger`: 0 stranded
- work completed for 'Default groups change complexity when their labels are reordered' at 21:50:16

### Successful Completion

The implementation of review cycle 1 has completed successfully in 19 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- orchestrator verification at 21:51:50: `just test` in `tree-hugger/` passed 1102 of 1102 with 0 skipped; `just lint` passed
- both repairs share `EXTRACTOR_REVISION` 29, because revision 29 was never committed or released; a stale revision-28 cache entry is recomputed for both repairs
- the review's human-review item, author acceptance of the counting convention and the initial language set, is still open; it is the author's decision, not a technical finding
