---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-11-schema-loader-yaml-spellings/spec.md"
plan: "claudine/features/2026-10-11-schema-loader-yaml-spellings/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.continued.yaml
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/README.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/suggestions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/cli/tests/l1/schema_validate.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/topics/schemas/dmls-schema-support.md
  - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/schema.md
  - .claude/skills/darkmatter/compose.md
source_code:
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
  - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.continued.yaml
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/suggestions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/l1/lsp_session.rs
  - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/cli/tests/l1/schema_validate.rs
documentation:
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/README.md
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/topics/schemas/dmls-schema-support.md
  - darkmatter/dmls/docs/diagnostics.md
  - .claude/skills/darkmatter/schema.md
  - .claude/skills/darkmatter/compose.md
completed_phase: 4
implemented: true
packages:
  - darkmatter
  - dmls
  - darkmatter-cli
---

# Implementation Log for 2026-10-11-schema-loader-yaml-spellings (4 phases)

## Phase 1

### Wave 1 — Contract inventory (Map Consumers)

Owning surface and validation path for every acceptance criterion (AC):

| AC | Owning surface | Validation path |
| --- | --- | --- |
| 1 load both spellings | `source.rs` `BlockLocator` (strict = `locate_yaml_value`, `multi_line_scalars: false`) reached from `standalone.rs::parse_standalone_schema_document` → `parse_standalone_schema_payload_with_source`; `resolve.rs` loads referenced files through the same parser | lib L1 (`resolve_schema_with_roots`, `DarkmatterSchemas::effective_for`), CLI L1 `cli/tests/l1/schema_validate.rs` (`CliProcessFixture`) |
| 2 envelopes, unions, compact items, siblings | same locator: `value()` empty-value branch (indentless only when `multi_line_scalars`), `value()` anchored branch (`child_indent` requires `> indent`, so `key: &a` + same-indent `- x` is unsupported even tolerant), `sequence()` compact-item indent (`indent + 2` strict vs `indent + relative` tolerant) | lib L1 matrix + Phase 2 regressions |
| 3 folded plain scalars | `inline_value()` → `yaml_scalar::decode_scalar_node` (tolerant only today) | lib L1 vs `serde_yaml_ng` |
| 4 structural + suggestion ranges | structural: `project_*_source` via located `DecodedScalar`; suggestion: `project_suggestion_spans` → `scan_value_scalars` (line-oriented, one physical line per value, uses `raw_offset` on both endpoints, not `DecodedScalar::project`) | lib L1 `schemas_source_projection.rs`, `suggest_constraint_phase*.rs` |
| 5 DMLS | `dmls/src/diagnostics/frontmatter.rs::standalone_type_definition_diagnostic` ranges a malformed definition at the tolerant `FrontmatterAst` entry `value_span` (whole value) — the Phase 3 seam; grammar error carries an expression-relative `span` that must project through the offending `DecodedScalar`. `standalone_payload_node` uses strict `locate_schema_value`. Cursor: `cursor.rs` `locate_type_definition_cursor` / `locate_schema_declaration_cursor` decode via `decode_partial_scalar_at(prefix, 0)` (single-scalar prefix) | DMLS L1 `dmls/tests/l1/suggest_constraint_phase1.rs`, `lsp_session.rs` |
| 6 exclusions unchanged | strict gates today all hang off one flag `multi_line_scalars`: (a) tags in `node_properties_end`, (b) tagged empty value, (c) indentless after empty value, (d) empty value = null scalar, (e) block/quoted/plain continuation in `inline_value`, (f) compact item width | existing `meta_schema_phase6.rs::source_aware_v1_presentation_boundary_is_closed` (block scalar only); Phase 2 adds the rest |
| 7 corpus / normal invocation | `meta_schema_repo_schemas.rs` (repo `schemas/`), `meta_schema_phase6.rs::shipped_base_schema_exposes_structural_spans_across_anchors_and_aliases`, `meta_schema_phase4` corpus (`darkmatter/docs/schemas`), CLI `schema_validate.rs` | retained unchanged |

Minimum shared seam (for Phase 2): split `multi_line_scalars` into two
permissions — *shared* (indentless sequences after an empty or anchored
mapping value, plain-scalar continuation through `decode_scalar_node`) and
*tolerant-only* (tags, block scalars, quoted continuations, empty values,
compact-item width). No new accessor or locator is needed; `scan_value_scalars`
must consume whole continued plain scalars (via `decode_scalar_node`) and the
suggestion projector should use `DecodedScalar::project`.

Classifier facts confirmed by reading `standalone.rs`: invalid YAML (including
duplicate keys, which `serde_yaml_ng` rejects, and multi-document input) →
`Ok(None)`; tagged needs `kind` to be the string `schema`, then exactly two
keys and a mapping `types`; pure needs `$schema` as the sole key. For
`Ok(None)` the resolver falls back to raw JSON Schema (`simplified: None`) for
valid YAML and returns `AmbiguousReferenced` for invalid YAML.

### Wave 2 — Fixture contract (Build Matrix)

Added `darkmatter/lib/tests/l1/schema_yaml_spellings.rs` (declared in
`tests/l1/main.rs`) and the fixture set
`darkmatter/lib/tests/fixtures/schema_yaml_spellings/` (provenance in its
`README.md`). No production code changed in this phase.

**Robustness matrix** — one table-driven test per seed, each walking every
cell from an unedited seed with exactly one edit:

| Test | Seed | Columns covered |
| --- | --- | --- |
| `tagged_envelope_robustness_matrix` | `tagged.conventional.yaml` | `kind`, `types`, named definition `title`, property-union `status` (scalar and compact mapping arms), tagged extra root keys (string and non-string) |
| `pure_mapping_robustness_matrix` | `nested.conventional.yaml` | `$schema`, nested definition `meta.level`, pure extra root key |
| `pure_root_union_robustness_matrix` | `root-union.conventional.yaml` | root-union members (compact mapping arms, reference arm) |

Each cell asserts the public result three ways: classification
(`parse_standalone_schema_document`), normal loading
(`resolve_schema_with_roots` of a real schema file in a temp dir — raw JSON
Schema with `simplified: None` for a declined mapping, `AmbiguousReferenced`
for declined non-mapping/invalid YAML, `SchemaDocument` for E), and, for A
cells, validation of instances through `DarkmatterSchemas::effective_for` on a
document whose `$schema` references that file. Control rows prove every
declaration is load-bearing (`title: ab` fails `min(3)`, `level: 9` fails
`max(5)`, `status: word` fails the union, `{}` fails the root union, etc.).
Edits address keys and items without their indentation so Phase 2 can run the
same cells against the `serialized` (indentless) seeds. Every cell gets its own
schema file name so a path-keyed cache cannot serve stale content.

Outcomes observed match the plan's table in every cell. Facts worth knowing:

- A removed **nested** definition is not "undeclared but allowed": nested
  mapping shapes are closed, so the key becomes an unknown key
  (`additionalProperties`); a removed **top-level** definition is accepted as
  undeclared. The `level absent` cell asserts the former.
- Removing the sole `$schema` key leaves an empty document; normal loading
  rejects it (`AmbiguousReferenced`). A separate cell renames the key beside
  other content and gets the raw-JSON-Schema reading.
- A repeated scalar (or mapping) union arm is accepted — it is not a
  duplicate key. A duplicate key inside a mapping arm is invalid YAML (D).
- `standalone.rs` builds its unsupported-key message with
  `filter_map(YamlValue::as_str)`, but the `map.len() != 2` check still
  rejects a non-string extra key; a cell now pins that.

**Paired fixtures** (`conventional` vs new presentation), each with a sibling
after the list or continuation: tagged / nested / root-union `serialized`
(byte-exact `serde_yaml_ng` output, guarded by
`serializer_captures_are_serde_yaml_ng_output`), mapping-value anchors with an
alias (`anchors.indentless.yaml`), both spec examples verbatim, continuations
in the type expression, a constraint list, a description and a sequence item
with a trailing comment (`continued.continued.yaml`), and a blank line plus
no final newline (`blank-line.continued.yaml`, paired with a single-line
double-quoted `\n`). `presentation_pairs_share_semantics_and_record_the_strict_baseline`
proves every pair resolves to structurally equal JSON Schema through the
source-free parser and that the conventional spelling loads source-aware.

**Baseline captured (same test):** six new-presentation fixtures fail at the
projection boundary (`SchemaDocument` … "could not project SimplifiedSchema
expression spans through YAML source"). The two fixtures whose continuation
lies wholly inside a **description** (`spec-continued`, `blank-line`) do *not*
fail: the strict locator reads only the first physical line, never consumes
the continuation lines, and returns success with a truncated `Definition`
span and **no span at all for the sibling `next`**. This is a pre-existing
false success — the spec's claim that this example fails with the projection
error is inaccurate for description-only folds — and Phase 2 must make the
locator consume or reject every line (and check decoded text against the
semantic value) rather than stop early. The test asserts this baseline so the
locator change cannot land without replacing it.

Requirement → test mapping for this phase:

| Requirement | Test |
| --- | --- |
| Robustness table, every cell, control row | the three `*_robustness_matrix` tests |
| Serializer-produced fixture with provenance | `serializer_captures_are_serde_yaml_ng_output` + fixture `README.md` |
| Paired fixtures mean the same schema; baseline outcomes | `presentation_pairs_share_semantics_and_record_the_strict_baseline` |

Gates: `cargo nextest … --test l1 schema_yaml_spellings` (5 tests, ≤0.1 s
each); `just test` in `darkmatter/` — 9120 passed, 11 skipped, 0 failed;
`just lint` in `darkmatter/` clean; `cargo clippy -p darkmatter --test l1 -- -D
warnings` clean. No cross-OS run: the tests are in-memory plus a temp dir, the
fixtures are LF on every checkout (`* text=auto eol=lf`), and no OS-specific
code was touched. Claudine's `just test`/`just lint` were not run: no Claudine
source changed (only this feature's plan/log/spec metadata). No pre-existing
failures; nothing skipped beyond the recipe's standing 11.

Phase 1 status: complete.

## Phase 2

### Wave 3 — Strict Locator (Separate Permissions)

`darkmatter/lib/src/markdown/schemas/simplified/source.rs`:

- The single `multi_line_scalars` flag is now a private `LocatorGrammar
  { Strict, Tolerant }` (stored on `BlockLocator` as `tolerant: bool`).
  **Shared** by both grammars: an indentless block sequence after an empty
  mapping value *and* after a mapping-value anchor (new `block_value` helper
  used by both branches of `value()`), and a plain scalar continued onto
  more-indented lines (`inline_value` → `yaml_scalar::decode_scalar_node`).
  **Tolerant-only**, unchanged: tags (`node_properties_end`), tagged empty
  values, empty values as null, block scalars, quoted continuations, the
  compact-item width `indent + relative`. Strict keeps `- ` (`indent + 2`).
- Strict plain continuation is taken only when the first line is a plain
  scalar (not quoted, flow, tag, anchor, alias, or block header). A tagged or
  quoted first line stays single-line, so its continuation lines go unconsumed.
- **Full consumption (fixes the Phase 1 false success):** strict
  `locate_yaml_value_with` fails unless every non-comment source line belongs
  to the located node. A description-only continuation therefore can no
  longer load with its tail unread and its sibling missing.
- **Semantic agreement:** new `verify_located(&LocatedValue, &YamlValue)` runs
  before any structural projection in `parse_property_definition_with_source`,
  `parse_schema_declaration_with_source`, and the standalone payload path. It
  requires the same mapping keys in order, equal sequence lengths, and every
  string scalar's decoded text equal to `serde_yaml_ng`'s; an alias (`*x`) is
  not compared, and non-string scalars carry no expression. Any mismatch is
  the existing projection error.
- A strict scalar that opens its own line (`key:` then an indented scalar, or
  a DMLS value slice) also goes through `inline_value`, so its continuation is
  read; tolerant keeps reading such a scalar as its first line only (no
  tolerant behavior change).

- Error precedence is unchanged: each source-aware entry point still runs the
  semantic parse before locating and verifying. A grammar error therefore still
  outranks a projection error. For example, `bogus(2)` inside a block scalar
  reports "unknown constraint", not the projection message.

### Wave 4 — Scalar Coordinates (Project Suggestions)

- `scan_value_scalars` now records each value's holder column (mapping key,
  `-`, or a compact item's key) and decodes plain values through
  `decode_scalar_node`. It skips the physical lines a continued scalar
  consumed, and trailing comments are no longer part of the decoded value.
  Flow sequences and quoted values keep the old single-line behavior.
- `project_atom_suggestions` uses `DecodedScalar::project` instead of
  `raw_offset` on both endpoints, so a candidate's start never lands in
  skipped indentation.
- **Occurrence identity bug found and fixed for the standalone loader.** A probe
  showed that `project_suggestion_spans`, which matches by source order,
  *swaps* spans between a pattern key and an identical property authored after
  it. Properties are visited before pattern keys, so the property took the
  pattern key's line. The standalone loader now projects suggestion spans per
  path through the same located scalars the structural map uses
  (`project_located_schema_suggestions` and friends, called from
  `parse_standalone_schema_payload_with_source`). `standalone.rs` no longer
  calls `project_suggestion_spans`. An alias carrying suggestions stays a
  projection error, as it effectively was before (the scanner found no later
  scalar for it). The frontmatter path (`parse_yaml_schema_with_source`, used by
  DMLS's overlay) still scans by order, and its doc comment records this
  pattern-key limitation.
- `cursor.rs` needed no change: `decode_partial_scalar_at` maps plain text
  1:1, so a token on a continuation line projects to its authored bytes. The
  new tests pin this for LF and CRLF.

### Departure: DMLS slices must start at a line boundary

Following continuations by indentation exposed a hidden coupling. DMLS's
`FmEntry::value_span` for a block mapping or sequence starts at the first
key or `-`, mid-line. A slice taken from it reads that first line at column
0 while its siblings keep their columns, so the first value appeared to
continue onto its sibling. This turned the existing DMLS test
`suggest_phase5_schema_remains_active_alongside_warnings` red. Before this
phase the same slices were silently misread too: the strict locator returned
only the first key or item.
Resolution (smallest correct seam, no library API change):

- library docs (`locate_schema_value`, `parse_property_definition_with_source`,
  `parse_yaml_schema_with_source`) now state the contract: a source-aware
  slice starts at a line boundary, with its indentation included, or is
  dedented;
- new `dmls::overlay::frontmatter::line_aligned_span` widens a value span back
  to its line start when only spaces precede it. It is applied in
  `overlay/suggestions.rs` (suggestion lint slice) and
  `diagnostics/frontmatter.rs::smallest_invalid_definition_span`.
  DMLS hover (`providers/frontmatter.rs`) already dedents and is unchanged.

This touches `dmls` production code in Phase 2, ahead of Phase 3's DMLS work.
That was unavoidable to keep the area green.

### Requirement → test mapping (Phase 2)

All tests are L1. Library tests are in `darkmatter/lib/tests/l1/schema_yaml_spellings.rs`; the DMLS test is in `darkmatter/dmls/tests/l1/lsp_session.rs`.

| Requirement | Test |
| --- | --- |
| Equivalent spellings load with structurally equal JSON Schema; sibling `next` keeps its span (replaces the Phase 1 baseline) | `presentation_pairs_load_with_equal_semantics_and_keep_their_siblings` |
| Robustness matrix unchanged under the indentless serializer seeds | `tagged_envelope_robustness_matrix_indentless`, `pure_mapping_robustness_matrix_indentless`, `pure_root_union_robustness_matrix_indentless` (+ the three Phase 1 conventional runs) |
| Continuation tokens and cross-fold expressions get exact structural spans; trailing comment excluded; LF and CRLF | `continued_plain_scalars_project_structural_spans_onto_authored_lines` |
| Blank-line folding matches `serde_yaml_ng`; no final newline | `a_blank_line_in_a_continued_scalar_decodes_like_the_yaml_parser` |
| Both envelopes, nested arm, compact-mapping root union, anchors and aliases, siblings | `indentless_sequences_project_each_arm_and_stop_at_the_sibling` |
| Non-ASCII byte ranges; caller offset applied once | `continued_scalars_project_byte_ranges_through_non_ascii_text`, `a_continued_definition_projects_through_a_caller_offset` |
| Strict exclusions stay closed (block scalars, tags incl. before a continuation and on an indentless arm, multi-line quoted, complex key, explicit flow key, directive), wrapped in `SchemaDocument` | `strict_source_aware_parsing_keeps_every_other_presentation_closed` |
| Suggestion candidates on continuation lines / across folds; advisory lint on a continuation line; structural `Argument` spans equal candidate spans; CRLF | `suggestion_candidates_on_continuation_lines_project_exactly` |
| Repeated identical definitions (folded and single-line, with non-suggesting definitions between) keep their own spans | `repeated_identical_definitions_keep_their_own_suggestion_spans` |
| Pattern key before an identical property keeps its own spans (failed before the fix) | `a_pattern_key_before_an_identical_property_keeps_its_own_suggestion_spans` |
| Frontmatter projector: offset after the fold, non-ASCII, CRLF | `folded_suggestions_project_through_a_caller_offset_and_non_ascii_text` |
| Multiple continuations, trailing whitespace and comment, blank lines in a sequence item, no final newline: decoded = `serde_yaml_ng`, semantics = single-line spelling | `folded_values_decode_like_the_yaml_parser_and_match_single_line_semantics` |
| Genuine projection mismatch never yields a range (structural and suggestion) | `a_source_that_disagrees_with_the_value_is_a_projection_error` |
| Cursor on continuation text: same token/role as single-line, exact span, argument in a fold | `a_cursor_on_a_continuation_line_reads_the_authored_token` |
| DMLS line-aligned slice (failed without `line_aligned_span`; verified by temporarily reverting) | `a_malformed_arm_after_the_first_block_sequence_item_is_ranged_exactly`; the overlay path is pinned by the pre-existing `suggest_phase5_schema_remains_active_alongside_warnings` (red without the alignment) |

### Gates

- `just test` in `darkmatter/`: 9137 passed, 11 skipped, 0 failed (one SLOW
  pre-existing unrelated test:
  `compose::transclusion::…::relevel_output_matches_the_oracle_across_shipped_fixtures`).
- `just lint` in `darkmatter/`: clean (one `collapsible_if` fixed).
- `just test` in `claudine/` (depends on the darkmatter lib): 8673 passed, 9
  skipped; `just lint` in `claudine/`: clean.
- Shipped corpus, tolerant-frontmatter, anchor/alias, and normal-invocation
  suites ran inside those gates unchanged and green.
- Smell review of touched code: no `#[serde(default)]`, no absent/null
  `Option` conflation, no `filter_map(as_str)`, no `unwrap_or_default` on a
  load-bearing value. `locate_schema_value`'s `.ok()` is its documented
  `None` fallback. `verify_located` skips only non-string keys and scalars,
  which carry no expression text.
- No cross-OS run. All changed code is in-memory string processing, CRLF is
  covered by explicit strings, fixtures are LF via `.gitattributes`, and no
  path or `cfg` code changed. CI covers the other environments.

Phase 2 status: complete.

## Phase 3

### Wave 5 — Recover Diagnostics (DMLS)

Before any production change, the new `darkmatter/dmls/tests/l1/schema_yaml_spellings.rs`
failed in four ways. The `bogus` diagnostic was ranged over the whole folded
value, from line 3 col 5 to line 5 col 0. The same happened under UTF-16.
Completion on a continuation line of a partly typed document offered nothing.
Completion in an indentless pure root union (`$schema:` then `- count: num` at
column 0) also offered nothing. Hover already selected the right node in every
probe, so hover needed no production change.

Production changes (`dmls` only; no library change in this phase):

- `diagnostics/frontmatter.rs`: a new `grammar_token_span` is used by
  `standalone_type_definition_diagnostic` before its existing fallbacks. It
  locates the payload with the strict locator (`standalone_payload_node`). It
  pairs each located string scalar under the failing definition with its
  `serde_yaml_ng` value (`collect_string_scalars`; a sequence item inherits its
  property, and a structural disagreement contributes nothing). It then picks
  the scalar whose own re-parse reproduces the same `Grammar` message and span.
  That scalar is decoded with `decode_scalar_node` at its holder column, and
  the decoded text must equal the semantic value. The grammar span is then
  projected with `DecodedScalar::project`; an empty result is rejected.
  Otherwise the existing nested-property and whole-value fallbacks apply
  unchanged. Code and severity are unchanged.
- `providers/frontmatter.rs`:
  - `block_cursor`'s line-based reading is now `line_cursor`. When a line is
    neither `key:` nor `- `, `continued_scalar_holder` finds the line holding
    the plain scalar it continues, and the cursor takes that scalar's value
    start. The holder is the nearest less-indented line with an indicator;
    blank lines fold, and a comment line ends the scalar. A quoted, block,
    flow, tagged, anchored, or empty holder value is not continued this way.
  - `enclosing_path` no longer treats a column-0 `- ` line as top level, and
    `enclosing_path_by_indent` accepts a same-column `key:` line as the parent
    of an indentless item, skipping sibling items (`is_sequence_item_line`).
    This also gives frontmatter indentless items their parent key, where they
    previously had no ancestors.

**Behavior note (departure worth reviewing):** token-level ranging is not
limited to folded scalars. A single-line standalone grammar error such as
`t: string(min(1); bogus(2))` is now also ranged at `bogus`, not the whole
value; the single-line case in the test pins this. No existing test asserted
the old whole-value range for a grammar error whose span the parser narrows.
The frontmatter inline `$schema` path (`smallest_invalid_definition_span`) was
left unchanged: the plan scopes this work to standalone diagnostics.

### Wave 5 — Prove Validation (CLI)

Six L1 tests were added to `darkmatter/cli/tests/l1/schema_validate.rs`
(already declared in `tests/l1/main.rs`), all through `CliProcessFixture`, with
explicit LF strings (plus one CRLF variant), and with real schema files
referenced as `$schema: ./schema.yaml`. A mutation check (`max(8)` → `max(9)`)
turned both the equality test and the tagged CLI test red.

### Requirement → test mapping (Phase 3)

DMLS tests are in `darkmatter/dmls/tests/l1/schema_yaml_spellings.rs`; CLI
tests are in `darkmatter/cli/tests/l1/schema_validate.rs`.

| Requirement | Test |
| --- | --- |
| Spec `bogus` example: physical line 5, columns 5–9 (LSP 3:4..3:9), UTF-16 client | `the_specified_bogus_example_underlines_line_five_columns_five_to_nine` |
| Grammar error token range, code `dm.schema.invalid_type_definition`, severity error: spec example LF and CRLF, single-line spelling, folded description plus sibling, pure nested mapping property, indentless union arm, identical text earlier in a comment | `a_grammar_error_on_a_continuation_line_ranges_the_offending_token` |
| UTF-8 vs UTF-16 conversion with non-BMP text (`😀é`) before the token on its continuation line | `a_continuation_line_grammar_error_converts_through_the_negotiated_encoding` |
| A genuinely unprojectable document (block-scalar sibling, which the strict locator rejects) keeps the whole-value fallback; never an empty or token range | `an_unprojectable_grammar_error_keeps_the_whole_value_fallback` |
| Hover on continuation text, indentless items, the sibling, and a repeated identical-shape definition equals the conventional spelling's hover; LF and CRLF; UTF-16 | `hover_on_continuation_text_and_indentless_items_selects_the_conventional_node` |
| Completion: continuation line (partial and complete), indentless item (partial and complete), indentless pure root-union compact item (partial); edit range starts at the authored token; UTF-8 and UTF-16, LF and CRLF | `completion_on_continuation_text_and_indentless_items_replaces_the_authored_token` |
| Lint-invalid suggestion on a continuation line stays a warning, exactly ranged, LF and CRLF; the schema still loads (the sibling hovers) | `a_lint_invalid_suggestion_on_a_continuation_line_stays_advisory_and_exact` |
| `md schema validate`, tagged envelope with an indentless union, a continued constraint list and description, and a sibling: valid and invalid instances, each failure naming its path and constraint | `schema_validate_applies_tagged_indentless_union_and_continued_scalar` |
| Same, CRLF | `schema_validate_applies_tagged_presented_spelling_with_crlf` |
| Folded description reaches the resolved schema | `schema_validate_reports_folded_description_of_continued_scalar` |
| Indentless pure `$schema` root union of compact mappings, a nested union, and a sibling | `schema_validate_applies_indentless_pure_root_union` |
| `types` sequence stays rejected (exit 2, "must be a mapping") | `schema_validate_rejects_indentless_types_sequence` |
| Resolved JSON Schema structurally equals the conventional spelling (tagged LF/CRLF, root union) | `schema_yaml_spellings_resolve_to_conventional_json_schema` |

### Gates

- `just test` in `darkmatter/`: 9150 passed, 11 skipped (the recipe's standing
  skips), 0 failed. This includes the shipped corpus, anchor/alias,
  tolerant-frontmatter, Phase 1 matrix, and Phase 2 range suites.
- `just lint` in `darkmatter/`: clean after one `map_or_identity` fix.
- `cargo nextest run -p dmls` after the lint fix: 811 passed, 6 skipped.
- `just test` in `claudine/`: 8673 passed, 9 skipped; `just lint` in
  `claudine/`: clean.
- `just check-tier-coverage darkmatter`: no stranded tests. All new tests are
  L1; the DMLS ones run in the in-memory LSP harness and open no windows.
- Smell review of the new code: no `#[serde(default)]`, `unwrap_or_default`,
  or `filter_map(as_str)` on load-bearing input. The `.err()`/`?` uses in
  `grammar_token_span` only choose between the exact range and the existing
  fallback.
- No cross-OS run. The changes are in-memory string processing; CRLF is
  covered by explicit strings, the DMLS fixtures use temp-dir paths through
  `url::Url::from_file_path` as the existing suite does, and no `cfg` code
  changed. CI covers the other environments.

Phase 3 status: complete.

## Phase 4

### Wave 6 — Behavior Documentation (Update Boundary)

Docs (current-behavior record, no feature references):

- `darkmatter/docs/topics/schemas/definition.md`, "Source-Aware Presentation
  Boundary". This section now lists both new spellings in the accepted
  presentations. It adds compact examples for an indentless union with a
  sibling, a continued type expression, constraint list, and description, and
  blank-line folding (shown equal to the quoted `\n` spelling). It explains
  where a continued value ends, that folding never makes an invalid
  expression valid, and how byte ranges work: original buffer, LF/CRLF kept,
  continuation tokens ranged on their own line, cross-fold expressions given
  one enclosing range, and DMLS converting ranges to the negotiated editor
  encoding. It restates the unchanged exclusions with the actual failure
  message, and notes that frontmatter's tolerance does not extend to schema
  files.
- `darkmatter/dmls/docs/diagnostics.md` ("Ranging"): a standalone
  `invalid_type_definition` grammar error is ranged at the offending token
  (the spec's `bogus` example). It falls back to the whole value when the
  token cannot be placed exactly, and is never given an empty range.
- `darkmatter/docs/topics/schemas/dmls-schema-support.md` (Diagnostics): the
  same token ranging, plus hover and completion on continuation lines and
  indentless items, and position-encoding conversion.

Every doc example was checked against the real `md` binary before being
written down. The first draft of the continued-constraint example used `,`
between constraints, and `md schema validate` rejected it ("expected `;` or
`)` between constraints"). It now uses `;`. With `;`, a valid instance passes,
and an over-length one fails on `max(8)` with the folded description intact.
The blank-line example validates. A `!!str` sibling fails with "could not
project SimplifiedSchema expression spans through YAML source", which is the
message the doc quotes.

Code comments:

- `source.rs` module `//!` "V1 presentation boundary" still described the old
  boundary. It now names continued plain scalars and parent-aligned block
  sequences, and lists the excluded tags, block scalars, and quoted
  continuations; "may return a projection error" became "return".
  The per-function locator, tolerant-locator, and suggestion-scanner comments
  were already corrected in Phase 2, and `yaml_scalar.rs` too. No comment
  still claims that strict parsing rejects all multi-line scalars.
- `cursor.rs` `//!`: names continuation-line text among the presentations that
  project to authored ranges (behavior pinned in Phase 2, comment lagged).
- `dmls/src/providers/frontmatter.rs::enclosing_path`: the doc comment said
  "Empty for a top-level line". That drifted in Phase 3, because a column-0
  `- ` item now belongs to the key above it. Corrected.

Skills:

- `.claude/skills/darkmatter/compose.md` claimed that schema documents "keep
  the closed v1 grammar that rejects" block *and multi-line* scalars. That
  drift is now fixed: the strict and tolerant grammars share indentless
  sequences and continued plain scalars, and only tolerant follows the rest.
- `.claude/skills/darkmatter/schema.md` gained a short "source spans" note
  for agents. It covers the one locator with two grammars, the
  full-consumption and `serde_yaml_ng` agreement checks, and the hidden
  coupling found in Phase 2: a source-aware slice must start at a line
  boundary, and DMLS uses `line_aligned_span`. It also says to never add a
  third locator.
- READMEs (`darkmatter/lib`, `dmls`): no public claim conflicts; unchanged.
  No dependency change, so no `dependencies.md` change.
- The `claudine` skill needed no change. This work does not touch Claudine
  behavior.

### Wave 7 — Final Verification (Run Gates)

- `just test` in `darkmatter/`: **9150 passed, 11 skipped, 0 failed** (two
  runs; the 11 are the recipe's standing skips). The log confirms that the
  required suites ran and passed. That includes all 28 lib/DMLS
  `schema_yaml_spellings::` tests, the six robustness-matrix runs (eight
  matching names), `strict_source_aware_parsing_keeps_every_other_presentation_closed`,
  `schemas_source_projection`, the CLI `schema_validate` spelling tests, and
  the shipped-corpus, anchor/alias, and tolerant-frontmatter tests. No test
  was reported SLOW.
- `just lint` in `darkmatter/`: clean (0 warnings or errors).
- `just test` in `claudine/`: 8673 passed, 9 skipped. `just lint` in
  `claudine/`: exit 0; the only output line is the pre-existing macOS linker
  note `__eh_frame section too large`, which is not a clippy finding.
- `just check-tier-coverage darkmatter`: 0 stranded tests.
- Portability review: the fixtures are `eol: lf` (`git check-attr`). CRLF
  cases are built as explicit strings. CLI tests use `CliProcessFixture`
  (isolated home and CWD) with runtime temp paths. DMLS tests use the
  in-memory LSP harness with `Url::from_file_path` on temp dirs. No `cfg`
  code, shell setup, or window focus is involved. Local evidence is
  **macOS only**. No cross-OS run was made: every change is in-memory string
  processing or a comment, so there is no OS-specific risk to probe. The tests
  remain eligible for the existing Linux/Windows/WSL2 CI coverage, and no CI
  cells were added.
- Diff review against acceptance criteria 1–7, over the feature commits
  `3010cee8b..HEAD` plus this phase:
  1. Library structural equality and CLI valid/invalid validation:
     `presentation_pairs_load_with_equal_semantics_and_keep_their_siblings`,
     `schema_yaml_spellings_resolve_to_conventional_json_schema`, and
     `schema_validate_applies_*`.
  2. Both envelopes, the indentless pure root union, the nested arm, compact
     mappings, siblings, and `types` sequence rejection:
     `indentless_sequences_project_each_arm_and_stop_at_the_sibling`,
     `schema_validate_applies_indentless_pure_root_union`, and
     `schema_validate_rejects_indentless_types_sequence`.
  3. Folds, blank lines, comments, and no final newline, decoded the same as
     `serde_yaml_ng`: `folded_values_decode_like_the_yaml_parser_and_match_single_line_semantics`
     and `a_blank_line_in_a_continued_scalar_decodes_like_the_yaml_parser`.
  4. Exact structural and suggestion ranges, the advisory lint, repeated
     definitions, CRLF, non-ASCII text, and offsets: the Phase 2 range tests
     and `a_lint_invalid_suggestion_on_a_continuation_line_stays_advisory_and_exact`.
  5. DMLS `bogus` ranging, hover, completion, and UTF-16: the Phase 3 DMLS
     tests.
  6. Strict exclusions stay closed and wrapped in `SchemaDocument`; the
     tolerant and anchor tests still pass.
  7. The corpus, the gates, and L1 portability, as recorded above.
- Envelope recognition, raw JSON Schema interpretation, and the public
  `SchemaError::SchemaDocument` shape are unchanged. `standalone.rs` only
  stopped calling `project_suggestion_spans`, because the payload path now
  projects per path (Phase 2).
- Smell grep over added source lines: one `unwrap_or_default`, in DMLS
  `is_sequence_item_line`. It is an editor-only lexical check where a missing
  line means "not a list item", so it reads no load-bearing input. There is
  no `#[serde(default)]`, `filter_map(as_str)`, or new load-bearing `.ok()`.

### Departures from the spec (recorded here, spec left as decided)

- Phase 1 found that the spec's description-only fold example loaded with a
  truncated span; it did not fail with the projection error as the spec
  claimed. The docs describe current behavior, which now loads correctly.
- Phase 3 extended token-level grammar-error ranging to *single-line*
  standalone definitions as well (`t: string(min(1); bogus(2))` underlines
  `bogus`). The docs state this generally ("ranged at the offending token").

Phase 4 status: **implementation complete, ready for review.** The author
owns moving the feature to `_completed`.
