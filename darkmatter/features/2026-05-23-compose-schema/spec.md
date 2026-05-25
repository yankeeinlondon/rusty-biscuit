---
created: 2026-05-23
reviewed: true
status: ready for planning and implementation
---

# Schema Validation in the Compose Pipeline

Darkmatter's `compose` pipeline currently runs frontmatter interpolation and frontmatter shell expansion without ever consulting the schemas subsystem. As a result, a document that declares a required property via `$schema` can still produce cryptic downstream failures when that property is missing or invalid — e.g. `dirname ''` exits 1 because `{{ spec }}` interpolated to an empty string.

This feature adds an always-on **Schema Validation** stage to the compose pipeline. It runs after `--set` / `--state` overrides are applied to the frontmatter, but **before** any interpolation or shell expansion. When the effective frontmatter violates the resolved schema, compose aborts with a styled `BlockError` that names the offending property — instead of a downstream tool's exit code.

## Goals & Non-Goals

**Goals**

- Fail fast in `md compose` when the effective frontmatter does not satisfy a declared `$schema`.
- Reuse `DarkmatterSchemas::validate` so `md compose` and `md schema validate` agree by construction.
- Allow callers (notably the future claudine schema integration) to inject a workspace-wide baseline schema through `ComposeOptions`.
- Produce a rich, styled error that names the failing property, includes the validator's expected/actual detail when available, and shows the document's `description:` for context.

**Non-Goals (v1)**

- **Authoring `$schema` on existing prompt files** (including `prompts/plan.md`, the document that triggered this report). That is a content change, separate from this feature. With no `$schema` and no baseline, this feature is a no-op — the cryptic `dirname ''` error on `prompts/plan.md` remains until somebody writes a schema for it.
- **Claudine integration.** The companion claudine feature is tracked at `claudine/features/2026-05-15-schemas/`. Claudine will consume `ComposeOptions::with_baseline_schema(...)` once landed; no claudine work is part of this spec.
- **Soft-mode bypass.** No `--allow-schema-violations` flag. The stage is always on and always strict. A soft-mode flag may follow if a need emerges, mirroring the existing `--allow-invalid-frontmatter-assignment` pattern.
- **Cache reuse across compose invocations.** Each `run_compose` builds a fresh `DarkmatterSchemas`. If profiling later shows this is a hot path, callers can pass an `Arc<DarkmatterSchemas>` — out of scope here.
- **Toggling the stage off.** The stage is not part of the `ComposeOperation` enum. `ComposeOptions::only(...)` cannot exclude it.

## Foundational Decisions

- **Decision #1** — Schema Validation runs as a new always-on pre-effective-state stage, positioned **after** `prepare_frontmatter_for_compose(...)` applies external-state defaults and `--set` overrides, but **before** Frontmatter Interpolation.
- **Decision #2** — The stage validates the **effective** (post-override) frontmatter. `--set` and `--state` can fulfill required properties; a document with `spec: ""` plus `--set spec=design.md` validates successfully.
- **Decision #3** — The stage reuses `DarkmatterSchemas::validate` directly and treats `ValidationReport { valid: false, problems }` as the compose failure path. No parallel validator and no duplicate problem shape.
- **Decision #4** — When a document declares no `$schema` and no baseline is configured, the stage is a no-op. Compose proceeds unchanged.
- **Decision #5** — Schema violations are hard errors. The stage returns before any interpolation or shell expansion runs.
- **Decision #6** — Recursive compose runs validate every Markdown document they compose. Parent `::file set=` overlays are applied to a child before the child's schema stage, but those overlays still do not propagate to grandchildren.
- **Decision #7** — A configured baseline schema is part of the compose options that affect output/failure behavior, so it must participate in transclusion cache keys and persistent cache option hashing.

## Pipeline Placement

```
Load markdown
  └─ Apply --set / --state overrides
      └─ [NEW] Schema Validation  ──► fails fast on SchemaError
          └─ Frontmatter Interpolation   ({{ var }})
              └─ Frontmatter Shell Expansion ($(cmd))
                  └─ Text Replacement
                      └─ Page Blocks
                          └─ Interpolation
                              └─ Shell Expansion
                                  └─ Shell Blocks
                                      └─ Link Resolve (abs)
                                          └─ Transclusion
                                              └─ Inline Post
                                                  └─ Finalization
```

The stage runs against the document's frontmatter only — body content is not yet validated by the schemas subsystem and is unchanged by this work.

## API Surface

### `ComposeOptions`

A new field and builder method are added. The field stores a parsed `SimplifiedSchema`, not a filesystem path; file-backed baseline loading already lives on `DarkmatterSchemas::with_baseline_from_file(...)` and is intentionally not exposed through compose options in this version.

```rust
use darkmatter::markdown::schemas::SimplifiedSchema;

impl ComposeOptions {
    /// Attach a baseline `SimplifiedSchema` that is merged with any
    /// `$schema` declared in the document before validation runs.
    ///
    /// Callers (e.g. claudine) can register a workspace-wide schema
    /// without editing every prompt file. When both baseline and
    /// document `$schema` declare the same property, the document
    /// side wins — matching the existing `schemas::resolve::merge`
    /// rule.
    pub fn with_baseline_schema(mut self, schema: SimplifiedSchema) -> Self {
        self.baseline_schema = Some(schema);
        self
    }
}
```

A single new field `baseline_schema: Option<SimplifiedSchema>` lives alongside the other compose options. Add it to `ComposeOptions::Debug` as `Some(..)` / `None`; do not dump full schema contents in normal debug output.

No CLI flag exposes it in this version — `md compose` itself only honors document-level `$schema`.

### Validation entry point

The stage instantiates `DarkmatterSchemas::new()`, optionally chains `.with_baseline(schema)?`, and calls `.validate(&md)` where `md` is the markdown document with post-default/post-override frontmatter. The validator's existing resolution rules apply:

- Document `$schema` is resolved (inline / file reference / root union) via `schemas::resolve`.
- Baseline is merged using the existing baseline-merge semantics.
- `$schema` is stripped from the instance before validation by `DarkmatterSchemas::validate`, so baselines using `additionalProperties: false` do not reject every authored document.
- Neither present → no-op.

`SchemaError` values from schema parsing, resolution, conversion, baseline merge, or validator construction should propagate through the same compose error path as validation failures, but the rendered block should distinguish "schema could not be prepared" from "frontmatter did not satisfy the prepared schema".

### Error type

A new variant is added to `MarkdownError` unless implementation finds an existing compose sub-error type that is already used for pre-effective-state stages. `MarkdownError` is the preferred location because the current compose pipeline returns `MarkdownResult<T>`, and `MarkdownError::status_block` already delegates to richer sub-errors where needed.

```rust
// Sketch — final placement determined by existing error layering.
#[error("Schema validation failed for {path:?}: {summary}")]
SchemaValidationFailed {
    path: PathBuf,                 // source file or "<stdin>"
    problems: Vec<ValidationProblem>,
    summary: String,               // short one-liner for the top-level message
}
```

The variant implements `biscuit_terminal::errors::BlockError`. The block body shows:

- **Header**: `Schema validation failed` plus OSC8 link to the source file.
- **Description line**: document `description:` (if present) rendered as `<i><dim>...</dim></i>`. Matches the convention from the claudine schemas spec.
- **One bullet per problem**:
  - Missing required: `<red>missing</red> <inverse>{property}</inverse>: required but not provided`.
  - Wrong type: `<red>type</red> <inverse>{property_or_path}</inverse>: {message}`.
  - Format / constraint failure (for example `darkmatter-file`): `<red>invalid</red> <inverse>{property_or_path}</inverse>: {message}`.
- Each line carries the YAML source `line:col` from `ValidationProblem::{line,column}` when available.
- Root-union failures include `arm_index` when present, for example `schema arm 2`, so authors know which union arm was considered the closest match.

The CLI top-level handler already converts `Result` errors into a styled print + non-zero exit. No additional CLI plumbing is required.

## Module Layout & Touchpoints

**New file:** `darkmatter/lib/src/markdown/compose/schema_validation.rs`. Exports a `pub(crate) fn run(...)` that:

1. Checks whether either document `$schema` or `ComposeOptions::baseline_schema` is present; if neither exists, returns `Ok(())` without constructing a validator.
2. Builds `DarkmatterSchemas::new()` plus `.with_baseline(...)` when `ComposeOptions::baseline_schema` is set.
3. Calls `.validate(&md)`.
4. Converts schema-preparation `SchemaError` into the chosen compose error path while preserving it as the source.
5. Converts `ValidationReport { valid: false, problems }` into the new validation-failed error variant. On success, returns `Ok(())`.

**`compose::mod.rs`** — invoke the new stage immediately after `prepare_frontmatter_for_compose(...)` and before `frontmatter_interpolation::interpolate_frontmatter(...)`. Update the long doc comment that enumerates Inline Pre stages to list "0. Schema Validation" as the first stage. Because `run_compose_pipeline_internal(...)` is used for child documents too, this placement validates transcluded Markdown children after their scoped `set=` overlay is applied.

**`compose::types.rs`** — add the `baseline_schema` field to `ComposeOptions`, plus the `with_baseline_schema(...)` builder method. Add `ComposeStage::SchemaValidation` so perf output can report this stage in execution order, and update the fixed-size perf array and stage display text accordingly. Add the new error variant on the relevant compose error enum (preferred: `MarkdownError`).

**`compose::perf.rs`** — add `PerfMetricKind::SchemaValidation` before `FrontmatterInterpolation`; keep the public/private stage ordering and fixed array size in sync.

**`compose::cache::hashing`** — include `baseline_schema` in `options_hash(...)` using canonical JSON from `schemas::to_json_schema(...)`. This prevents cached child compose results from being reused across different baseline schemas. Add a unit test proving two otherwise-identical `ComposeOptions` values with different baselines produce different option hashes.

**CLI (`darkmatter/cli/src/commands.rs`)** — no changes for this feature. The new error propagates through the existing top-level error handler. Document-level `$schema` validation becomes active for `md compose`; baseline injection remains library-only.

**Skill file (`.claude/skills/darkmatter/SKILL.md`)** — update the "Compose Pipeline" → "Inline Pre" list to include Schema Validation as the first stage. Regenerate the `hash:` frontmatter via `md hash` after edits.

## Testing Strategy

### Unit tests (alongside `schema_validation.rs`)

- **No-op cases**:
  - Document with no `$schema` and no baseline → returns `Ok(())`; downstream stages still run.
- **Document `$schema` is honored**:
  - Inline schema declaring `spec: "file(required)"`, frontmatter has `spec: ""` → error.
  - Same schema, frontmatter has `spec: "design.md"` → ok.
  - Wrong-type required (`spec: 42`) → error.
- **Baseline merging via `ComposeOptions::with_baseline_schema(...)`**:
  - Baseline declares `spec` required; document has no `$schema` → baseline applies; missing `spec` errors.
  - Both declare `spec` with different types → document wins (existing `resolve::merge` rule).
- **Override interaction**:
  - Document has `spec: ""` (would fail) + `--set spec=design.md` → ok (validates post-overrides).
  - Document has `spec: "design.md"` + `--set spec=""` → fails.
- **Recursive compose interaction**:
  - Child document has `$schema` requiring `child_input`; parent transcludes it with `set.child_input=ok` → child validates.
  - Same child without the parent `set=` overlay → child validation fails before child frontmatter interpolation or shell expansion.
- **Cache safety**:
  - Distinct `baseline_schema` values produce distinct `cache::hashing::options_hash(...)` values.

### Integration tests

Located under `darkmatter/lib/src/markdown/compose/mod.rs` `#[cfg(test)]` or `darkmatter/cli/tests/`.

- The **planner-prompt regression**: synthesize a fixture matching the shape of the failing prompt (`spec: ""` + `dir: "$(dirname '{{ spec }}')"`) plus an inline `$schema` requiring `spec` as a `file`. Assert:
  - `md compose` exits non-zero with the styled `BlockError`.
  - The error mentions `spec`, **not** `dirname` (i.e. shell expansion never ran).
  - Verified either via a sentinel command in `$(...)` that would leave a detectable side-effect, or simpler — `compose_with` report shows zero frontmatter shell replacements.
- **`md schema validate` parity**: assert that schema-validation outcomes for the same document match between `md compose` (new stage) and `md schema validate` (existing command). Guards against drift even though they share the underlying validator.
- **Baseline cache regression**: compose a transcluded child with a baseline requiring one property, then compose the same child with a different baseline requiring another property. Assert the second run does not reuse the first cached success/failure.

### Snapshot tests

`insta` snapshot of the rendered `BlockError` for the missing-required-property case, with terminal width pinned (consistent with other compose error snapshots in `darkmatter/lib/src/`).

## Risks

- **Error type location.** The new variant may land on `MarkdownError`, on a `ComposeError` sub-type, or as a wrapper around `SchemaError`. The choice depends on existing error layering in compose stages. Not blocking the design; resolved during implementation.
- **Schema resolution side effects.** `schemas::resolve` may touch the filesystem (loading `$schema: ./foo.yaml`). The new stage runs that I/O on every `md compose` invocation that declares a `$schema`. Acceptable cost; matches what `md schema validate` already does.
- **File-reference validation is CWD-sensitive.** The existing `file` SimplifiedSchema type resolves property values from the current working directory at validation time. This spec intentionally does not change that contract. Tests that validate `file` properties should set the process CWD or use references that are valid from the test's known working directory.
- **Cache-key omissions.** A baseline schema changes whether a compose succeeds and may change downstream composed output by stopping execution earlier. Omitting it from `options_hash(...)` would allow stale child compose results. The implementation must include baseline schemas in option hashing before enabling the feature.
- **No-schema documents stay broken.** This feature does not close the originating bug report on `prompts/plan.md`. Authoring a `$schema` for that file is a separate change, and `md compose` documents that already lack a schema continue to behave exactly as today.

## Related Work

- `darkmatter/features/2026-05-11-schemas/spec.md` — the schemas subsystem this feature consumes.
- `claudine/features/2026-05-15-schemas/spec.md` — claudine's integration plan, which will adopt `ComposeOptions::with_baseline_schema(...)` once landed.
