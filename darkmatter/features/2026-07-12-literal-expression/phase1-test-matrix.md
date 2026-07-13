# Phase 1 — Acceptance-Criterion → Test-Location Matrix

Maps each spec acceptance criterion (spec.md §Acceptance Criteria) to the focused
test surface that will prove it. The Phase-1 failing scaffolds live in
`darkmatter/lib/tests/schemas_literal_expression.rs` (added this phase, marked
`#[ignore]` with the implementing phase, so `just test` stays green until the
behavior lands and the ignore is lifted). Each row also notes where the
*permanent* coverage settles.

| AC | Requirement | Test surface (permanent home) | Phase-1 scaffold |
|----|-------------|-------------------------------|------------------|
| 1 | `literal(...)` parse/serialize/reparse/`const`; `literal()` & `literal(a,b)` errors | `schemas_grammar_proptest.rs` (round-trip), `schemas_convert_snapshots.rs` (`const`), grammar unit tests in `simplified/grammar.rs` | `literal_*` grammar/serialize tests |
| 2 | non-`required` accepts missing/`null`; `required` enforces; `default` must equal value | `schemas_validate_table.rs` fixtures, `simplified/lint.rs` unit tests | `literal_required_*`, `literal_optional_*`, `literal_default_*` |
| 3 | Coercion writes back typed scalars (`"2"`→`2`), pending excluded, validate-gated | `schemas/coerce.rs` unit tests + compose-level test | `literal_coerce_*` |
| 4 | Property-union mixing `literal` with atoms (`[literal(auto), number]`) | `schemas_validate_table.rs` fixtures | `literal_property_union_*` |
| 5 | `expression` accepts either-dialect (condition-mode parse; superset regression); rejects garbage w/ format-name `constraint` problem; never executes | `schemas_validate_table.rs`, expression corpus regression (Phase 3), `dmls/tests/no_side_effects.rs` | `expression_accepts_*`, `expression_rejects_*`, `expression_superset_*` |
| 6 | Native bool/number coerce to string; mappings/sequences are mismatches | `schemas/coerce.rs` unit tests + compose-level test | `expression_coerce_*`, `expression_mapping_sequence_mismatch` |
| 7 | `literal` as trigger constraint; `expression` behaves like yaml/json in triggers | `triggers/matcher.rs` unit tests, `triggers/lint.rs` | `literal_trigger_*`, `expression_trigger_*` (matcher unit — Phase 3) |
| 8 | `md schema about` lists both; existing `md schema validate` byte-identical | `cli/tests/schema_validate_baseline.rs` (executable pretty+JSON snapshots) + `about_lists_literal_expression` about parity | baseline captured; `about_lists_literal_expression` (Phase 2) |
| 9 | DMLS: D1 expr completion/hover/`dm.expression.*`; D2 literal exact-value/hover/required-key; D3 union key-completion narrowing; ranges for plain/quoted/escaped/multibyte | `dmls/tests/lsp_session.rs`, `dmls/tests/no_side_effects.rs`, DMLS provider unit tests | Phases 5–6 (DMLS crate); scaffolds land with those phases |
| 10 | Discriminant narrowing type-sensitive, single unambiguous arm; absent/unknown/duplicate/conflicting preserve union behavior | `schemas_validate_table.rs` fixtures + shared selector unit tests; `dmls` provider tests | `union_narrow_*`, `union_no_narrow_*` (Phase 4) |
| 11 | All L1/L2 green via area `just test` / `just test-l2` | area CI gate | n/a (release gate, Phase 7) |

## Notes on scaffold entry points (compile-safe against current public API)

- **Grammar / serialize** — `darkmatter::markdown::schemas::simplified::grammar::parse_type_expr`
  and `serialize_property_atom`. `literal`/`expression` currently lex as unknown
  type keywords → `Err`, so a scaffold asserting `Ok` fails for the intended
  missing behavior (not a harness error).
- **Validation / coercion / pending** — `DarkmatterSchemas::validate` on an
  in-memory `Markdown` with inline `$schema`, and the compose pipeline
  (`Markdown::compose_with(ComposeOptions::new())`) for write-back observation.
- **DMLS (AC 9)** — deferred to Phases 5–6 because the provider scaffolds
  require the in-crate LSP session fixture; documented here so nothing is
  dropped.

## Baseline artifacts (this phase)

- `phase1-baseline.txt` — original `md schema validate` pretty + JSON capture for
  representative union/enum/required fixtures. The union-failure diagnostics here
  MUST stay byte-identical for schemas without literal discriminants (AC #8/#10).
  This capture is now an **executable regression**: the seven cases and their
  expected pretty/JSON output are checked in under
  `darkmatter/cli/tests/fixtures/schema_validate_baseline/` and asserted
  byte-for-byte by `darkmatter/cli/tests/schema_validate_baseline.rs` (run by
  `just test`), so the recorded output can no longer drift undetected.
- `phase1-baseline-about.txt` — `md schema about` full output. Post-change output
  must be a pure superset (both new types appended in deterministic order).
