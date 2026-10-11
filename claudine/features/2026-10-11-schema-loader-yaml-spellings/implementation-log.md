---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-11-schema-loader-yaml-spellings/spec.md"
plan: "claudine/features/2026-10-11-schema-loader-yaml-spellings/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
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

