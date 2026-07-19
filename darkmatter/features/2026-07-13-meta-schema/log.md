---
feature: 2026-07-13-meta-schema
description: "Implementation log for the meta-schema feature's review-to-implement cycles"
deferred_perf_measurement: false
implementation_2: "2026-07-18T09:03:32-07:00"
---

# Meta Schema — Implementation Log

## Implementation of Review Findings #2

> **started at:** 2026-07-18T09:03:32-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-meta-schema/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review contains **4** findings:
        - **High** — DMLS reconstructs parser state and ranges from text instead of consuming the required sidecar
        - **High** — a semantic arm in a valid mixed union produces false diagnostics and suppresses sibling completions
        - **High** — invalid standalone outer declarations use the wrong code and whole-document range
        - **Medium** — trimmed schema references are syntax-checked and resolved with different strings
- impacted package area (per the spec's implementation surface map): `darkmatter`, covering the `darkmatter` lib, `darkmatter-cli`, and `dmls` crates — so `just test` / `just lint` from that area is the verification scope
- findings are implemented serially, foundation-first, so that later DMLS work can build on the shared parser authority:
        1. Medium — trimmed schema references (library, isolated)
        2. High — sidecar-backed parser-state authority for DMLS
        3. High — mixed-union gating for diagnostics and completion
        4. High — standalone outer-declaration diagnostic contract

### Finding 4 (Medium) — Trimmed schema references

- starting the work on 'trimmed-schema-references' at 09:04:31-07:00
        - confirmed the defect: `classify_schema_reference` computed `trimmed` for the HTTP(S) and bare-name decisions but built the `FileReference` from the untrimmed `reference`, so `" ./schemas/post.yaml "` was classified on one string and resolved on another
        - a whitespace-only value was non-empty to `FileReference::new`, so it fell through as `PathQualified` rather than being rejected
        - discovered the duplicated policy the finding warns about: `resolve_reference` in `resolve.rs` was calling `reference.trim()` a **second** time, independently of the classifier
        - fixed `darkmatter/lib/src/markdown/schemas/reference.rs` so classification, `FileReference::new`, and the `reference` field on both `RemoteUnsupported` and `Unresolved` all derive from the single `trimmed` value
        - added an explicit whitespace-only guard returning `SchemaError::Unresolved { reference: "", source: InvalidSyntax("empty reference string") }` before `FileReference::new` is reached
        - fixed `darkmatter/lib/src/markdown/schemas/resolve.rs` so `resolve_reference` takes the canonical string back off the classifier via `classified.file_reference().raw()` — syntax check, bare-name root lookup, sibling suggestion, path resolution, and error reporting now use one string by construction
        - aligned two sibling call sites in `resolve.rs` carrying the identical defect: `resolve_namespace` (`Name@file` imports) and `resolve_one_example` (`example(...)` artifacts) both used `trimmed` for the remote/bare-name checks but passed the untrimmed string to `FileReference::new`
        - verified nothing downstream depends on the untrimmed error text — no references to `RemoteUnsupported` or the `$schema` resolution message anywhere in `claudine/`
- work completed for 'trimmed-schema-references' at 09:20:04-07:00
        - added `padded_schema_references_are_classified_and_resolved_as_one_trimmed_string` to `darkmatter/lib/tests/schemas_source_projection.rs`, beside the existing `schema_declaration_parser_classifies_syntax_without_io` sentinel
        - the test covers leading, trailing, both-sided, and tab/newline padding for path-qualified and bare-name references; asserts kind and canonical `raw()` on the classifier; asserts the same product from `parse_schema_declaration` on a quoted padded YAML scalar
        - it asserts `""`, `" "`, `"   "`, `"\t"`, `"\n"`, and `" \t\n "` all reject as an empty reference with the structured error
        - resolver parity is proven by resolving every padded form (plus padded bare names against a schema root) through `resolve_schema_with_roots` and asserting an identical `json_schema` to the unpadded reference
        - the passive-operation proof is retained: classifying `"  ./no/such/dir/missing.yaml  "` and `" missing.yaml "` succeeds, which can only hold if classification performs no existence check or file I/O
        - deliberate non-change: `parse_schema_declaration_with_source` still maps `SchemaSpanKind::FileReference` onto the **authored** text including padding — spans must point at what the author actually typed so an LSP squiggle lands on real characters, and that is independent of the value used for resolution
        - gates green: `just test` → darkmatter 5816 passed / 140 skipped, darkmatter-cli 561 passed / 71 skipped, dmls 584 passed / 3 skipped, zero failures; `just lint` clean with zero warnings

### Finding 1 (High) — Sidecar-backed parser-state authority for DMLS

- starting the work on 'dmls-sidecar-parser-state' at 09:20:04-07:00
        - discovered the root cause: the library already had a **strict** source-aware sidecar (`parse_property_definition_with_source` / `parse_schema_declaration_with_source` in `simplified/source.rs`) that can only answer questions about text which already parses
        - editor completion asks the opposite question — about text that by definition does not parse yet — which is exactly why DMLS had grown its own text heuristics; the missing piece was a **tolerant** entry point, not a second AST
        - new library module `darkmatter/lib/src/markdown/schemas/simplified/cursor.rs` (private module, re-exported from `markdown::schemas`) is the new authority:
                - `SchemaCursor { path: SchemaSourcePath, role: SchemaCursorRole, token: String, token_span: Range<usize> }` — reuses the existing `SchemaSourcePath` sidecar path type, so no new path model and no second spanned AST (spec ruling Q6)
                - `SchemaCursorRole::{ Type, Constraint { subject, array_level }, Argument { subject, constraint, array_level }, InlineObjectKey, ImportReference { name } }`
                - entry points `locate_type_definition_cursor(value_source, value_offset, cursor)` and `locate_schema_declaration_cursor(...)`
                - the implementation drives **`grammar::Lexer`** — the same lexer the real parser uses — over the authored prefix, tracking an explicit frame stack matching the grammar's EBNF (`InlineObject` / `ConstraintList` / `ArgList`), and reproduces the parser's own special cases (`enum`/`literal` item lists lex in argument mode; `[]` before `(` selects the array-level list)
                - no `rfind`, `split`, or indentation heuristic anywhere in it
        - projection goes through the `yaml_scalar` seam via a new **`decode_partial_scalar_at`**, the tolerant sibling of `decode_scalar_at` (an unterminated quote decodes to what exists so far; plain scalars keep trailing whitespace) — no line-ending normalization, so CRLF and multibyte offsets stay exact
        - added a structural locator `locate_schema_value` + `SchemaValueNode` / `SchemaValueKind` / `SchemaValueEntry` to `simplified/source.rs`, a public projection of the private `locate_yaml_value` — this is what diagnostics needs to range something *inside* a value the semantic parser rejects
        - taught `locate_inline` to accept empty flow collections (`[]`, `{}`), which it previously errored on
        - resolved the array-surface question from the existing contract rather than inventing one: added `SchemaConstraintDescriptor::accepts_array_level()` in `about.rs`, derived from the catalog's existing `target_types` prose rather than a new hand-maintained list
- work completed for 'dmls-sidecar-parser-state' at 10:18:31-07:00
        - rewired `darkmatter/dmls/src/providers/frontmatter.rs`:
                - `meta_schema_completion` now uses `SchemaAuthoringState` only for *activation*; everything downstream comes from the cursor API
                - added `semantic_value_start`, which prefers the frontmatter AST's `value_span` and only falls back to the line for a buffer whose YAML is not yet parseable
                - `type_definition_completions` dispatches on `SchemaCursorRole`; new `constraint_completions` handles item-level vs array-level lists, plus `accepts_constraint` comparing catalog entries as whole keywords instead of substrings
        - rewired `darkmatter/dmls/src/diagnostics/frontmatter.rs`: `smallest_invalid_definition_span` now walks `locate_schema_value`'s structural tree deepest-first instead of the frontmatter AST's descendants — this also closes a real gap, since the AST does not enumerate the interior of a sequence item
        - heuristics deleted: `semantic_value_token` (the `,`/`[` splitter with quote-stripping), the `partial.rfind('(')` + `trim_end_matches("[]")` constraint branch, the `accepted_constraints.contains(...)` substring test, and `empty_flow_sequence_span` (the raw-byte `[]` scanner)
        - tests added:
                - library L1: 18 unit tests in `cursor.rs` covering all three named failure states, nested inline objects, flow union arms, arguments, imports, fresh positions, hard-parse-failure tolerance, quoted/multibyte/escaped projection, document offsets, and mid-value cursors
                - library L1: `array_level_constraint_set_is_published_and_parseable` pins the array surface as exactly `required, default, generated, min, max, unique`, and proves `suggest` is rejected
                - DMLS L1: `meta_schema_completion_reads_parser_state_for_partially_authored_values` in `dmls/tests/lsp_session.rs` — 6 cases × LF/CRLF = 12 in-memory `lsp_session` runs (second constraint after nested paren, postfix array constraints, single-quoted inline object, block-sequence union arm with constraint, double-quoted, multibyte-in-value), each also asserting the `textEdit` range replaces the authored token rather than appending
        - **no existing public signature changed** — everything is additive; the only visibility change widens `grammar::Lexer` and five of its methods from private to `pub(super)` so the sibling module can use the one grammar authority, leaving the CRITICAL blast radius on `parse_property_definition` / `parse_schema_declaration` untouched
        - two intentional behavior deltas: `ImportReference` now returns no completions (previously the type-keyword catalog was reachable after `@`, where it matched nothing — a no-op in practice, now correct by construction), and `Argument` / `InlineObjectKey` positions deliberately offer nothing, as they are author-supplied values and names
        - gates green: `just test` → darkmatter 5835 passed / 140 skipped, darkmatter-cli 561 passed / 71 skipped (1 flaky, retried green — the pre-existing `schema_about` leaked-handles flake the review already recorded), dmls 585 passed / 3 skipped; `just lint` clean, exit 0; `no_side_effects.rs` sentinels pass unchanged
        - orchestrator independently re-verified with `cargo check -p dmls --all-targets` after the IDE surfaced stale mid-edit syntax diagnostics: compiles clean, the diagnostics were a language-server snapshot artifact

### Finding 2 (High) — Mixed-union gating for diagnostics and completion

- starting the work on 'mixed-union-gating' at 10:18:31-07:00
        - confirmed the mechanism: `semantic_authored_diagnostics` iterated the overlay's `SchemaAuthoringState::Frontmatter` values and emitted the specialized code whenever the recorded semantic kinds rejected the authored value, consulting the `ValidationReport` only for `relatedInformation` — never to ask whether the *union as a whole* failed
        - activation is intentionally broad (any arm carrying `type-definition`/`schema` records the kind), so a valid mixed union produced a false `dm.schema.invalid_type_definition`
        - **extracted a shared gating helper**: the expression path's policy was inline and expression-specific (a `HashSet<&str>` of `report.problems` paths built at the top of `expression_diagnostics`), now lifted to `union_rejected_paths(report: &ValidationReport) -> HashSet<&str>`
                - its rustdoc `## Notes` records why activation being broader than rejection is exactly what makes the gate necessary
                - both `semantic_authored_diagnostics` and `expression_diagnostics` call it; the expression docblock now points at the shared authority instead of restating the policy
        - gated the specialized diagnostic on `union_rejected.contains(value.pointer)` **before** any parsing work, so a valid document does no redundant AST/YAML/`parse_property_definition` work
- work completed for 'mixed-union-gating' at 10:36:28-07:00
        - completion no longer short-circuits: `completion` in `darkmatter/dmls/src/providers/frontmatter.rs` split its ordinary body into a new `schema_completion` helper, and is now `meta_schema_completion(...).unwrap_or_default()` extended with `schema_completion(...)`, passed through the existing `dedup_completions`
        - semantic items lead, so the pure-`type-definition` ordering and the dedup/first-seen preselection policy are unchanged; sibling union arms merge in behind them
        - the Finding 1 cursor/sidecar API is untouched — no text heuristics were reintroduced
        - tests added to `darkmatter/dmls/tests/lsp_session.rs` (Level-1, in-memory `lsp_session` fixture) with helpers `mixed_semantic_union_doc` (parameterized on arm order) and `codes`:
                - `mixed_semantic_union_gates_the_specialized_diagnostic_on_whole_union_failure` — `type-definition`/`string` and `schema`/`string`, both arm orders, each with a value the `string` arm accepts (specialized code must be **absent**) and a value no arm accepts (must be **present**); 8 documents total
                - `single_arm_semantic_type_keeps_its_specialized_diagnostic` — pins the non-union common path so the fix does not regress it
                - `mixed_semantic_union_completion_merges_sibling_arm_candidates` — `[type-definition, enum(foo, bar)]` in both arm orders must offer `foo`, `bar` **and** the semantic `string` candidate
        - both fixes were verified load-bearing by temporarily disabling each in turn: without the gate the diagnostics test reproduces the review's exact false positive; without the merge the completion test loses `foo`/`bar`
        - discovered a real validator property worth recording: `string` is **not** a discriminating arm for scalar values — `value: 42` against `[type-definition, string]` produced no diagnostics at all because the `string` arm coerces a YAML number rather than type-mismatching, so the whole-union-failure case had to use a block mapping
        - checked that merging adds no noise for `$schema` authoring: values under `$schema:`, the `$schema` scalar itself, and block-sequence arms all resolve through `def_at_path_ctx` against the effective shape, which has no `$schema` property, so the ordinary provider returns empty there
        - one deliberate contract delta flagged: a semantic cursor role that intentionally offers nothing (`Argument`, `InlineObjectKey`, `ImportReference`) previously returned `Some(vec![])` and suppressed the ordinary provider entirely; the ordinary provider now runs there too, which traces to empty for semantic-typed properties in every case but is a genuine contract change rather than a pure merge
        - gates green: `just test` → darkmatter 5835 passed / 140 skipped, darkmatter-cli 561 passed / 71 skipped (no `schema_about` flake this run), dmls 588 passed / 3 skipped; `just lint` clean, exit 0; `no_side_effects.rs` sentinels pass unchanged

### Finding 3 (High) — Standalone outer-declaration diagnostic contract

- starting the work on 'standalone-outer-declaration-diagnostics' at 10:36:28-07:00
        - discovered the standalone fallback had exactly two outcomes: `standalone_type_definition_diagnostic` (inner, mapping-payload properties only) and an unconditional whole-buffer `dm.schema.document_malformed` — everything else (empty root unions, illegal union arms, invalid whole-file references) hit the generic code with a whole-document range
        - discovered the outer/inner boundary is **structural**, not incidental: an outer declaration is a *sequence* payload (root union) or a *scalar* payload (whole-file reference), whereas a *mapping* payload's failure always belongs to one of its property definitions
                - restricting the new path to sequence/scalar payloads is what mechanically preserves the `invalid_type_definition` vs `invalid_schema_shape` split, rather than relying on ordering luck
        - discovered `parse_yaml_schema` does not classify union-arm references (only `parse_schema_declaration` does), so a whitespace-only reference *arm* (`$schema: ["   "]`) parses clean today and never reaches the error path; the finding's reachable reference case is the scalar form `$schema: "   "`, which the Finding 4 trimmed-reference fix now rejects with `Unresolved { reference: "" }`
- work completed for 'standalone-outer-declaration-diagnostics' at 11:01:17-07:00
        - changed `darkmatter/dmls/src/diagnostics/frontmatter.rs`: added `standalone_declaration_diagnostic` between the inner and generic paths, emitting `dm.schema.invalid_schema_shape` from `parse_schema_declaration`'s error with a range from the library's structural locator (`locate_schema_value`)
                - supporting helpers `standalone_payload_node`, `invalid_union_arm_span`, and `arm_is_rejected` (which mirrors `parse_schema_declaration`'s arm handling: mapping → inline shape, string → `classify_schema_reference`, anything else → illegal)
                - extracted `standalone_payload_key` so both standalone helpers derive the envelope's payload key from one place
                - no raw-byte scanning and no decoded-text range reconstruction — the Finding 1 sidecar authority is consumed throughout
        - changed `darkmatter/dmls/docs/diagnostics.md`: the `invalid_schema_shape` row records the standalone outer-declaration cases and their ranging; the `document_malformed` row now states it applies only when no more precise declaration or definition diagnostic claims the failure
                - `dm.schema.invalid_type_definition` is absent from that table entirely — pre-existing drift from an earlier phase, left alone under CLAUDE.md Rule 3 (surgical changes)
        - added `standalone_outer_declaration_errors_are_shape_coded_and_precisely_ranged` to `darkmatter/dmls/tests/lsp_session.rs`, covering 7 documents: pure empty root union, **tagged** empty root union, pure invalid scalar arm, pure invalid local reference, malformed YAML (tab indent), pure malformed inner definition, tagged malformed inner definition
                - each case asserts the exact LSP **range** *and* that the set of `dm.schema.*` document codes is exactly the one expected, so a whole-document range or a code collapse fails the test
                - helpers `range_of` / `whole_document_range` derive expectations from the fixture text rather than hard-coded numbers
        - verified load-bearing: temporarily disabling the new branch reproduces the review's exact defect — `pure-empty-union.yaml` reverts to `dm.schema.document_malformed` over `0:0–1:0`
        - gates green: `just test` → darkmatter 5835 / darkmatter-cli 561 / dmls 589 passed; `just lint` exit 0; `no_side_effects.rs` sentinels pass unchanged, since reference classification is passive and no filesystem access was added

### Final Verification

- the orchestrator re-ran both gates independently from the `darkmatter` package area **after all four findings had landed together**, to confirm the composed result rather than trusting per-finding reports:
        - `just lint` → exit 0, zero warnings and zero errors across `darkmatter`, `darkmatter-cli`, and `dmls`
        - `just test` → exit 0; darkmatter **5835 passed / 140 skipped**, darkmatter-cli **561 passed / 71 skipped**, dmls **589 passed / 3 skipped**; zero failures and no flaky retries on this run
- net test growth across the cycle: darkmatter 5816 → 5835 (+19), dmls 584 → 589 (+5), darkmatter-cli unchanged at 561
- during Finding 1 the IDE surfaced syntax diagnostics in `dmls/src/providers/frontmatter.rs` that contradicted the subagent's green report; `cargo check -p dmls --all-targets` confirmed a clean compile, so those were stale mid-edit language-server snapshots rather than real breakage

### Successful Completion

The implementation of review cycle 2 has completed successfully in 1 hour and 58 minutes. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 0 were deferred.

No finding required a performance measurement, so `deferred_perf_measurement` remains `false`.

Two follow-up items were surfaced during the work. Neither is a deferred finding — both fall outside the scope of what the review asked for — but both are recorded here so they are not rediscovered:

- **Invalid reference *arms* inside an otherwise-valid root union** (`$schema: ["   "]`) still produce no diagnostic. `parse_yaml_schema` accepts them as `SchemaArm::FileRef` without classification, so `parse_standalone_schema_document` returns `Ok` and the error path is never entered. Catching this would require running `parse_schema_declaration` on *every* standalone document rather than only on already-failing ones — a behavior expansion past Finding 3, which is scoped to the fallback path. This warrants a design decision rather than an unilateral widening.
- **A pure envelope with a valid scalar reference payload** (`$schema: ./other.yaml`) still reports `document_malformed` with the envelope's own "`$schema` must be a mapping or sequence" message. The declaration parser accepts that string, so there is no declaration-shape error to report; the complaint is genuinely envelope-level. This behavior is unchanged by this cycle.

The files changed during this cycle are:

- `darkmatter/lib/src/markdown/schemas/reference.rs` — single-trim canonical reference construction and the whitespace-only guard
- `darkmatter/lib/src/markdown/schemas/resolve.rs` — resolver parity via `classified.file_reference().raw()`, applied to `resolve_reference`, `resolve_namespace`, and `resolve_one_example`
- `darkmatter/lib/src/markdown/schemas/simplified/cursor.rs` — **new** tolerant parser-state authority (`SchemaCursor` / `SchemaCursorRole`)
- `darkmatter/lib/src/markdown/schemas/simplified/source.rs` — structural locator `locate_schema_value` and empty-flow-collection support in `locate_inline`
- `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs` — `Lexer` widened to `pub(super)` so the cursor module reuses the one grammar authority
- `darkmatter/lib/src/markdown/schemas/about.rs` — `SchemaConstraintDescriptor::accepts_array_level()`
- `darkmatter/dmls/src/providers/frontmatter.rs` — cursor-driven completion, `schema_completion` split, union-merged candidates
- `darkmatter/dmls/src/diagnostics/frontmatter.rs` — `union_rejected_paths` gate, structural-locator ranging, `standalone_declaration_diagnostic`
- `darkmatter/dmls/docs/diagnostics.md` — diagnostic-code table updated for the standalone outer-declaration contract
- `darkmatter/lib/tests/schemas_source_projection.rs` — padded/whitespace-only reference parity tests
- `darkmatter/dmls/tests/lsp_session.rs` — parser-state, mixed-union, and standalone-declaration Level-1 LSP regressions
