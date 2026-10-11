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
packages:
  - darkmatter
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
