---
ready: true
agent: open_code
model: ""
---

# Review: Schema Validation in the Compose Pipeline (Iteration #2)

## Summary

All five findings from review #1 have been addressed:

| Review #1 Finding | Status |
|---|---|
| High: schema-preparation errors lose diagnostic in rendered block | Fixed — `blocks.rs:170-195` now renders the `summary` when `problems.is_empty()`, with unit tests + snapshots. |
| High: planner-prompt regression not verified at CLI level | Fixed — `compose_schema.rs` exercises the binary, asserts exit status, `spec`, no `dirname`, no sentinel side-effect. |
| High: styled terminal error has only in-process snapshots | Fixed — `level2_errors.rs::level2_schema_validation_block_renders_styled_link_and_bullet` captures live WezTerm pane. |
| Medium: schema-validate parity not covered | Fixed — `compose_and_schema_validate_agree_on_same_document` exercises both commands against the same fixture. |
| Medium: baseline cache safety only hash-tested | Fixed — `baseline_cache_does_not_reuse_across_distinct_baselines` is a three-run behavioral cache regression with persistent cache. |

The implementation faithfully follows the spec: validation runs after `prepare_frontmatter_for_compose`, before frontmatter interpolation, `ComposeOptions::with_baseline_schema(...)` is public, `baseline_schema` propagates to children via `options.clone()` at `mod.rs:1871`, and is included in `options_hash(...)` at `hashing.rs:195-199`.

## Findings

### Low: bullet `missing` detection relies on `problem.property.is_some()` rather than `ValidationProblem` kind

At `blocks.rs:221-237`, the bullet renderer decides between `missing` and `type`/`invalid` by checking `problem.property.is_some()`. This conflates two orthogonal dimensions: whether a property name was extracted, and what the failure category is. If the schema subsystem ever reports a type error *with* a `property` value set, the renderer would mislabel it as `missing`.

The current `ValidationProblem` shape only has `property: Option<String>`, `path: String`, `message: String`, and `arm_index: Option<usize>` — there is no explicit failure kind enum. So the renderer is doing the best it can with the available data. This is a minor fragility; it matches the spec's rendering rules correctly for the current set of problems, and the fallback to `invalid` for non-type messages provides reasonable behavior.

**Severity:** Low (correct for all current cases; will break silently if the validator starts setting `property` on type errors).

**Suggested fix:** Add a `kind: ValidationProblemKind` field to `ValidationProblem` in a future iteration, and switch on that instead of inferring from `property` presence.

### Low: `source_path` does not use `ComposeSource::display()` consistently

At `schema_validation.rs:84-96`, the `source_path` helper extracts a `PathBuf` from the compose source. For URL sources, it converts `url.as_str()` into a `PathBuf` — this produces a valid but semantically odd `PathBuf` for non-file sources. The rendered block at `blocks.rs:151` calls `path.to_string_lossy()` which will produce the URL string anyway, so the visual output is correct. The intermediate `PathBuf` is just a carrier.

**Severity:** Low (cosmetic / type purity).

### Informational: test verification levels

| Requirement | Strongest Test Level | Assessment |
|---|---|---|
| Schema validation fails fast before shell expansion | Level 1 (CLI binary via `assert_cmd` + sentinel file side-effect) | Adequate — verifies exit code, stderr content, and no shell side-effect. |
| Styled `BlockError` with OSC8 link, red label, inverse property, dim description | Level 2 (WezTerm pane capture) | Adequate — `level2_schema_validation_block_renders_styled_link_and_bullet` checks OSC8, red SGR, and inverse SGR in real terminal. |
| `md compose` / `md schema validate` parity | Level 1 (CLI binary via `assert_cmd`) | Adequate — both binaries exercised, exit codes and property names compared. |
| Baseline cache isolation | Level 1 (in-process persistent cache behavioral test) | Adequate — three-run regression proving cold/warm/cold cache hits. |
| No-op when no `$schema` and no baseline | Level 1 (unit) | Adequate — internal logic only, no terminal rendering involved. |
| Document `$schema` honored | Level 1 (unit + CLI binary) | Adequate. |
| Baseline merging / document-wins | Level 1 (unit) | Adequate. |
| Override interaction (`--set` fixes/introduces failures) | Level 1 (unit) | Adequate — override is applied before `run()` in pipeline, and unit tests verify post-override validation. |
| Recursive compose: parent `set=` satisfies child schema | Level 1 (in-process transclusion) | Adequate — tempdir fixture with real `::file` directive. |
| Schema preparation error surfaces diagnostic | Level 1 (CLI binary + unit + snapshot) | Adequate. |
| `$schema` stripped before validation | Covered by existing `DarkmatterSchemas::validate` contract | Adequate — reuse by construction. |

No user-observable requirement requires Level 3 (OS keyboard injection) because schema validation has no interactive input surface.

## Test Results

All tests pass:

- `schema_validation` unit tests: 14/14
- `schema_validation_integration`: 5/5
- `error_snapshots` (schema-related): 6/6
- `compose_schema` CLI integration: 5/5
- `level2_schema_validation_block_renders_styled_link_and_bullet`: 1/1
- `options_hash_sensitive_to_baseline_schema`: 1/1

## Conclusion

The implementation is **complete and production-ready**. All review #1 findings are resolved. The remaining low-severity observations are about future-proofing the `ValidationProblem` rendering logic, not about correctness gaps. Test coverage is strong across unit, integration, CLI binary, and Level 2 terminal capture levels.
