---
phase: 1
created: 2026-07-18
artifact: acceptance-matrix
---

# Phase 1 Acceptance Matrix

Maps every in-scope opportunity, safety invariant, CLI mode, OS newline form,
and performance case to a named test or benchmark location. Locations are
binding targets for Phases 2–7; a row is green only when the named artifact
exists and passes. Test tiers follow `rust-testing` (L1 = unit/in-process,
L2 = integration/real-resource); all rows are L1 unless marked otherwise.

## A. In-scope opportunities (auto-applied)

| ID | Opportunity (decisions ref) | Test location | Cases |
|----|------------------------------|---------------|-------|
| A-1 | Source normalization: BOM, CRLF/CR→LF, trailing whitespace, final newline (D2-S1) | `biscuit-file/lib/src/yaml/analyze/tests/normalization.rs` (table-driven) | BOM at start; CRLF; lone CR; mixed endings; trailing spaces outside scalars; missing final newline; each candidate proven against Value equality |
| A-2 | Parse-equivalent whitespace cleanup (D2-S1) | `biscuit-file/lib/src/yaml/analyze/tests/whitespace.rs` (table-driven) | `[ 80,443 ]` → `[80, 443]`; spaces around `:` and `-`; rejection when values differ (`host:localhost` untouched) |
| A-3 | No-schema reserved-indicator quoting (D2-S3, D3) | `biscuit-file/lib/src/yaml/analyze/tests/reserved_indicator.rs` | `@daily-report` (flagship, original failing input); `` ` `` `%` `&` `*` `!` leading indicators; mapping and sequence contexts; nested mappings; lexeme byte-equality; report-only boundaries (` #` present, multi-line, flow context, `host:localhost`, `-80`, comments, URLs, Windows paths) |
| A-4 | Schema-proven scalar quoting (D2-S2) | `darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs` | `release: 1.20` → `"1.20"` (original failing input, native and quoted variants); sole-string-mismatch proof; full-schema pass; unrelated-regression rejection; unconstrained key silently skipped (D8) |
| A-5 | Combined edit application (D2-S4) | `biscuit-file/lib/src/yaml/analyze/tests/edit_set.rs` | Non-overlap validation; end-to-begin application; UTF-8 boundary rejection; out-of-range rejection; audit record of applied/rejected; 8-round S3 iteration cap; no partial chain on exhaustion |

## B. Report-only diagnostics (detected, never mutated)

| ID | Diagnostic | Test location | Cases |
|----|-----------|---------------|-------|
| B-1 | Duplicate mapping keys | `biscuit-file/lib/src/yaml/analyze/tests/duplicate_keys.rs` | Both conflicting spans; every nesting level; nested duplicates; candidate info only |
| B-2 | Anchor/alias conditions | `biscuit-file/lib/src/yaml/analyze/tests/anchors.rs` | Undeclared, forward, misspelled, duplicate, unused; graph-sensitive source preserved |
| B-3 | Multiple documents | `biscuit-file/lib/src/yaml/analyze/tests/multi_document.rs` | Two+ documents reported; no selection/split/rewrite |
| B-4 | Schema-free lints (ambiguous scalars, suspicious empty values, block-scalar smells, comment-truncation, style/indentation, similar keys) | `biscuit-file/lib/src/yaml/analyze/tests/lints.rs` | Positive, negative, nested, comment-preservation, ordering per detector; suppression/confidence boundaries quiet intentional YAML; every heuristic threshold justified in-test |
| B-5 | Schema-guided key correction + shape/type repair | `darkmatter/lib/src/markdown/schemas/tests/clean_suggestions.rs` | `timeuot` → `timeout` suggestion; enum/shape suggestions; report-only shape shared with biscuit-file; no `default` insertion |
| B-6 | Report-only never mutates | `biscuit-file/lib/src/yaml/analyze/tests/classification_gate.rs` | Exhaustive enum test: neither report-only classification reaches edit application, even with candidate repairs attached (single auto-apply filter keyed on classification) |

## C. Safety invariants

| ID | Invariant (decisions ref) | Test location | Cases |
|----|---------------------------|---------------|-------|
| C-1 | Value equality for S1 (D2) | `biscuit-file/lib/tests/yaml_safety.rs` (property) | Accepted S1 edit ⇒ reparsed Value identical; schema-associated ⇒ schema results identical |
| C-2 | Parse-recovery proof for S3 (D2) | `biscuit-file/lib/tests/yaml_safety.rs` | Quoted lexeme byte-equals original; context match (same key, same nesting); candidate parses; schema pass when associated |
| C-3 | Schema proof for S2 (D2) | `darkmatter/lib/tests/schema_quoting_safety.rs` | Exactly one plain-scalar string mismatch; full-schema pass; problem set strictly shrinks |
| C-4 | Untouched-byte preservation | `biscuit-file/lib/tests/yaml_safety.rs` + `darkmatter/cli/tests/clean.rs` | Output reconstructed from accepted spans; every byte outside accepted edits and body cleanup is identical (multibyte, CRLF, comments, anchors, block scalars) |
| C-5 | Structured error location preserved | `biscuit-file/lib/src/yaml/tests/location.rs` | Parser byte/line/column projection; multibyte spans; CRLF offsets; `YamlError` display snapshots byte-identical to pre-change |
| C-6 | Retained path source / no TOCTOU reread (D5) | `biscuit-file/lib/src/yaml/tests/retained_source.rs` | Path-backed diagnostics read the retained copy even after the file changes on disk; `from_value` returns no span diagnostics; `from_str`/`from_bytes` retain |
| C-7 | Shared `SourceSpan` vocabulary, no crate drift | `biscuit-file/lib/tests/span_compat.rs` (compile-time/public-API) | biscuit-file span type ↔ darkmatter re-export lossless; public import paths stable |
| C-8 | Clean input parses once; candidate-free input reparses zero times | `biscuit-file/lib/tests/parse_count.rs` (instrumented counters) | Parse-count assertions for clean, candidate-bearing, and candidate-free inputs |

## D. CLI modes (`md clean`)

| ID | Mode (decisions ref) | Test location | Cases |
|----|----------------------|---------------|-------|
| D-1 | stdout default with auto-repair (D6) | `darkmatter/cli/tests/clean.rs` (L1) | `title: @daily-report` input repaired on stdout; body cleanup still applied; untouched frontmatter bytes preserved (I-F4 regression: `release: 1.20` no longer corrupts) |
| D-2 | stdin (`-` and implicit) | `darkmatter/cli/tests/clean.rs` | Same results as file input; trigger discovery inert (D7) |
| D-3 | `--save` in-place | `darkmatter/cli/tests/clean.rs` | Repaired file written; delta report on stdout; `--save`+stdin exit 1 (baseline F8a) |
| D-4 | `--save` verbose, originally invalid frontmatter (D6) | `darkmatter/cli/tests/clean.rs` | Raw text-level summary instead of `Markdown::delta`; no baseline-`Markdown` pretense |
| D-5 | `--json` envelope (D6, D8) | `darkmatter/cli/tests/clean_json.rs` + golden fixtures | Sole stdout payload; every field, enum spelling, span, empty `repairs[]`, multi-diagnostic ordering pinned; `--save --json` writes file and prints envelope; human rendering suppressed |
| D-6 | STDERR suggestions (D6) | `darkmatter/cli/tests/clean.rs` | Report-only findings on stderr in human mode; exit code unchanged (0); rendered via `TerminalRenderable` components |
| D-7 | Unrepairable frontmatter (D4 step 4) | `darkmatter/cli/tests/clean.rs` | Exit 1, miette error preserved, file untouched (baselines F3/F9), diagnostics on stderr |
| D-8 | No-frontmatter and empty-frontmatter bypass (D4 step 2) | `darkmatter/cli/tests/clean.rs` | Zero YAML/schema/trigger work (counter-instrumented); output identical to pre-change baseline F1 |
| D-9 | Byte-level idempotency (D8) | `darkmatter/cli/tests/clean.rs` | clean(clean(x)) == clean(x) byte-for-byte for every fixture class |
| D-10 | Schema flags (D7) | `darkmatter/cli/tests/clean_schema.rs` | Default baseline keys; inline `$schema`; referenced schema; root union; `--baseline-schema`; `--no-baseline-schema`; matching/nonmatching triggers; `--no-trigger-schemas`; `--schema` replaces document `$schema`; stdin with schema flags; top-level `--save` shorthand defaults |
| D-11 | Sentinel fenced-YAML body block | `darkmatter/cli/tests/clean.rs` | Intentionally broken YAML inside a body ```yaml fence is byte-untouched; ordinary body cleanup behavior retained |

## E. OS newline / encoding forms

| ID | Form | Test location | Cases |
|----|------|---------------|-------|
| E-1 | LF | Covered in every table above | Baseline form |
| E-2 | CRLF | `biscuit-file/lib/src/yaml/analyze/tests/line_endings.rs` + `darkmatter/cli/tests/clean.rs` | Span accuracy with CRLF; normalization candidate; body CRLF untouched by YAML engine |
| E-3 | Lone CR | same | Normalized as S1; spans correct |
| E-4 | UTF-8 BOM | same + normalization table | Removal proven parse-equivalent |
| E-5 | Non-ASCII keys/values, Windows paths | `reserved_indicator.rs` / `lints.rs` tables | Multibyte span integrity; `C:\Users\Ken` not misquoted; URLs not misquoted |
| E-6 | Final-newline variants | normalization table | Add/remove final newline only when Value-equal |

## F. Corpus and performance

| ID | Case | Location | Cases |
|----|------|----------|-------|
| F-1 | YAML Test Suite subset (vendored, SHA-pinned, MIT) | `biscuit-file/lib/tests/yaml_corpus.rs` + `biscuit-file/lib/tests/corpus/` | Valid, expected-failure, duplicate-key, anchor/alias, flow, scalar, BOM, multi-document cases with upstream IDs; no network |
| F-2 | Mutation fixtures from real monorepo frontmatter | `biscuit-file/lib/tests/yaml_mutation.rs` | Every v1 repair/finding class injected; exact spans, deterministic ordering, classification, candidate edits, accepted output, untouched bytes |
| F-3 | Suite-wide invariants | `biscuit-file/lib/tests/yaml_safety.rs` (property) | C-1…C-4 across corpus + mutation inputs; idempotency of the repair engine |
| F-4 | No-frontmatter zero-work proof | `darkmatter/lib/tests/clean_counters.rs` (instrumentation) | Counter assertions: zero YAML analysis, zero schema resolution, zero trigger discovery |
| F-5 | Clean-frontmatter parse-once proof | same + `biscuit-file/lib/tests/parse_count.rs` | Exactly one parse, zero candidate reparses |
| F-6 | Performance regression comparison | `darkmatter/lib/benches/clean_hot_paths.rs` baseline `phase1-before` | Phase 7 re-run: `--baseline phase1-before`; no measurable regression in the two common cases; results recorded in a feature artifact |
| F-7 | Cross-platform compile/test | CI matrix (macOS/Windows/Linux) | Both affected packages build and test green |

## G. Baseline fixtures

The four functional-baseline inputs live in `baselines/` (`no-fm.md`,
`clean-fm.md`, `invalid-reserved.md`, `coercible.md`) and are reused by
D-1…D-9 rows so pre/post behavior is comparable verbatim.
