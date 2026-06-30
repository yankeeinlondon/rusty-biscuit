---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-23
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/cli/tests/schema_validate.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
  - darkmatter-cli
source_code:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/cli/tests/schema_validate.rs
documentation:
  - darkmatter/docs/topics/schema-definition.md
packages:
  - darkmatter
  - darkmatter-cli
---

# Execution Plan — Surface Property Descriptions in Schema Validation Errors

Threads the declared property `description` (from `-> {description}` arrow
syntax, inline-object per-property descriptions, or a `description` keyword in a
referenced JSON Schema file) onto every `ValidationProblem` and renders it across
the three surfaces that present validation failures.

Source of truth: `darkmatter/features/2026-06-23-description/spec.md`.

## Touchpoint Map

| File | Change |
|------|--------|
| `darkmatter/lib/src/markdown/schemas/mod.rs` | Add `ValidationProblem.description`; enrich in `EffectiveSchema::validate_with_positions` |
| `darkmatter/lib/src/markdown/schemas/validate.rs` | Add `resolve_problem_description` + `type_label`; set `description: None` in `build_problem` |
| `darkmatter/lib/src/markdown/errors/blocks.rs` | Per-problem dimmed-italic description sub-line in `schema_validation_failed_block` |
| `darkmatter/cli/src/commands/schema/validate.rs` | Pretty sub-line (`emit_problem_bullet`) + JSON field (`emit_json`) |
| `darkmatter/lib/tests/error_snapshots/markdown_error.rs` | Add `description` to 7 `ValidationProblem { ... }` literals |
| `darkmatter/docs/topics/schema-definition.md` | Document the new sub-line / JSON field |

## Build & Test Commands (package area)

- Unit tests: `just test`
- Integration (L2) tests: `just test-l2`
- Linter: `just lint`
- Doctests: `just doctest`
- Never run `cargo fmt` (write mode). Match surrounding style by hand.

---

## Phase 1 — Data Model & Compile Restoration

**Goal:** add the `description` field to `ValidationProblem` and restore
compilation with the field set to `None` everywhere. No behavior change yet —
this phase is a clean, observable checkpoint (builds green, all existing tests
pass, field is dead/`None`).

**Why first:** every downstream phase depends on the field existing. Splitting
the mechanical field-add + literal-fix from the resolver keeps the compile
breakage isolated and reviewable.

- [x] Add `pub description: Option<String>` field to `ValidationProblem` in `darkmatter/lib/src/markdown/schemas/mod.rs` (after `arm_index`), with the rustdoc from the spec (declared description resolved from the compiled JSON Schema; `None` when the property declares none).
- [x] Set `description: None` in the `build_problem` constructor in `darkmatter/lib/src/markdown/schemas/validate.rs:280` so the production validator path constructs problems with no description (enrichment is a later post-pass).
- [x] Fix every existing `ValidationProblem { ... }` struct literal in the repo to add `description: None` (or a meaningful value where a snapshot intends to show one). Known literals:
    - [x] `darkmatter/lib/tests/error_snapshots/markdown_error.rs` — 7 literals at lines ~133, ~154, ~182, ~206, ~269, ~291, ~300.
- [x] **Validation checkpoint:** `cargo build -p darkmatter -p darkmatter-cli` compiles; `just test` passes with no behavior change (every `description` is `None`).

---

## Phase 2 — Description Resolver & Enrichment

**Goal:** implement the JSON-Schema walk that resolves the failing property's
description and wire it as a post-pass in `validate_with_positions`. After this
phase, `ValidationProblem.description` is populated end-to-end for every
caller that reads a `ValidationReport`.

**Dependency:** Phase 1 (the field must exist).

### Resolver helpers

- [x] Add `pub(super) fn resolve_problem_description(root: &serde_json::Value, problem: &ValidationProblem) -> Option<String>` in `darkmatter/lib/src/markdown/schemas/validate.rs` implementing the spec algorithm:
    - [x] Pick the base schema: `root["anyOf"][arm_index]` when `problem.arm_index` is `Some(i)`, else `root`.
    - [x] Split the instance `path` into JSON-pointer segments, decoding `~1` → `/` and `~0` → `~` (reuse the existing `unescape_pointer_segment` helper at validate.rs:380).
    - [x] For a `Missing` problem, append `problem.property` as a final segment (Decision #3).
    - [x] Walk segments: before each descent, unwrap a nullable `anyOf` wrapper into its non-null arm (Decision #5); numeric segment → descend `node["items"]` (Decision #4, stop `None` if absent); named segment → descend `node["properties"][segment]` (stop `None` if absent).
    - [x] Read the description at the final node: direct `node["description"]` string; else apply the property-level-union articulation (Decision #6) over non-null arms `D`; else `None`.
- [x] Add a private `type_label(arm: &Value) -> Option<&'static str>` helper mapping a compiled arm schema to `string` / `number` / `integer` / `boolean` / `object` / `array` / `enum` (display aid only; never affects validation; `None` when no label is derivable).
- [x] Implement Decision #6 union articulation:
    - [x] `|D| == 1` → return that lone description verbatim (no wrapper).
    - [x] `|D| >= 2` → synthesize `a union type of: {A} | {B}`, each `{X}` the arm's `description` or `type_label(arm)`.
    - [x] `|D| == 0` → synthesize `a union type of: {labels}` from `type_label`; `None` when no label is derivable.
    - [x] Always exclude the `{ "type": "null" }` sentinel from the `D` count and the articulation.
- [x] Keep the walk defensive: any missing/typeless node returns `None` rather than panicking (Risk mitigation).

### Enrichment wiring

- [x] In `EffectiveSchema::validate_with_positions` (`darkmatter/lib/src/markdown/schemas/mod.rs:275`), after `collect_problems` / `collect_root_union_problems` produce the problem list, enrich each problem:
    - [x] `problem.description = resolve_problem_description(&self.json_schema, problem)`.
    - [x] Filter out whitespace-only descriptions (Decision #8).
    - [x] Filter out descriptions byte-for-byte equal to `problem.message` (Decision #9).
- [x] Confirm schema-preparation failures remain outside this path (they have no `ValidationProblem` list and reach `schema_validation_failed_block` with empty `problems` — unchanged).

### Resolution unit tests (`validate.rs` `#[cfg(test)]`)

- [x] Top-level `Type` problem resolves the property's `description`.
- [x] Top-level `Missing` problem resolves through `properties[property]` (Decision #3).
- [x] JSON Pointer escaping decoded before walking — schema keys with literal `/` or `~` resolve through `~1` / `~0`.
- [x] Nested inline-object path `/config/name` resolves through `properties.config` → unwrap nullable `anyOf` → `properties.name`.
- [x] Array path `/authors/0/name` descends through `items` (Decision #4).
- [x] Nullable optional property (`anyOf: [null, typed]`) reads the wrapper-level description (Decision #5).
- [x] Property-level union, exactly one arm describes → that lone description verbatim, no `a union type of:` wrapper (Decision #6, `|D| == 1`).
- [x] Property-level union, ≥2 arms describe → `a union type of: {A} | {B}`, mixing descriptions and type labels for description-less arms (Decision #6, `|D| >= 2`).
- [x] Property-level union, no arm describes → `a union type of: number | string` from type labels (Decision #6, `|D| == 0`).
- [x] Union articulation and `D` count both exclude the `{ "type": "null" }` sentinel.
- [x] Property with no description → `None`.
- [x] Unknown-property / `additionalProperties` failure → `None`; resolver must not reuse the parent object's description (Decision #10).
- [x] Unresolvable / exotic path → `None`, no panic.
- [x] Root union: description resolves against `anyOf[arm_index]` for the winning arm (e.g. `arm_index: Some(1)` reads arm 1).
- [x] Description equal to the message is suppressed (Decision #9).
- [x] Whitespace-only description is suppressed (Decision #8).
- [x] **Validation checkpoint:** `just test` — resolver unit tests and end-to-end `DarkmatterSchemas::validate` populate descriptions.

---

## Phase 3 — Render Surfaces

**Goal:** render the resolved description across the three failure surfaces.
The three tracks are **independent and parallelizable** — each consumes
`problem.description` and emits its own styling.

**Dependency:** Phase 2 (`description` must be populated before rendering).

> **Parallelizable** — Tracks A, B, and C can proceed concurrently.

### Track A — `md schema validate` pretty output

- [x] In `emit_problem_bullet` (`darkmatter/cli/src/commands/schema/validate.rs:255`), after printing the problem bullet, emit a trailing dimmed sub-line (one indent level beneath the bullet) when `problem.description` is `Some`:
    ```
        - title expected string (at line 2 of frontmatter)
            The headline shown in listing pages
    ```
- [x] Preserve the existing CLI location wording (`(at line N of frontmatter)`) and the full stripped JSON Pointer prefix for nested paths (Decision #11 — no diagnostic-surface redesign).
- [x] Omit the sub-line entirely when `description` is `None` (no stray blank lines).

### Track B — `md schema validate` JSON output

- [x] In `emit_json` (`darkmatter/cli/src/commands/schema/validate.rs:320`), add `"description"` to each per-problem `serde_json::json!` object: the string value when present, `null` when absent.

### Track C — `MarkdownError::SchemaValidationFailed` status block

- [x] In `schema_validation_failed_block` (`darkmatter/lib/src/markdown/errors/blocks.rs:255`), push a per-problem dimmed-italic sub-line after each problem bullet, reusing the existing `<i><dim>{desc}</dim></i>` markup the block already emits for the document-level `description:` line.
- [x] Confirm the document-level description line and the per-problem description lines coexist without collision.
- [x] Do **not** add a description line to schema-preparation failures (empty `problems` list) — continue rendering the preparation summary unchanged.

### Render tests

- [x] **Track A:** pretty output emits the dimmed description sub-line beneath the bullet when present; omits it when absent; existing location wording and nested JSON Pointer prefixes unchanged.
- [x] **Track B:** JSON output carries `"description"` with the string value, and `null` when absent.
- [x] **Track C:** `schema_validation_failed_block` includes the description sub-line per problem; document-level `description:` and per-problem descriptions coexist; preparation-failure rendering unchanged when `problems` is empty.
- [x] **Validation checkpoint:** `just test` and `just test-l2` — render tests pass; snapshot tests in `error_snapshots/markdown_error.rs` updated where literals gained a `description`.

---

## Phase 4 — Documentation, Backward-Compatibility & Full Validation

**Goal:** update public docs, prove no regressions, and run the full
verification suite.

**Dependency:** Phase 3 (all surfaces finalized).

> **Parallelizable** — documentation and backward-compat tests can proceed concurrently.

### Documentation

- [x] Update `darkmatter/docs/topics/schema-definition.md`:
    - [x] **Pretty Output** section (line ~561): show the dimmed description sub-line beneath each problem bullet.
    - [x] **JSON Output** section (line ~582): show the new `"description"` field (string value + `null` case).
    - [x] **Error Rendering** section (line ~872): note the per-problem description sub-line reuses the dimmed-italic treatment, and that the `->` description now appears in `md schema validate` and compose schema-failure blocks.
    - [x] Note (near the `->` arrow / `description` authoring docs) that the description surfaces at the point of failure.

### Backward-compatibility tests

- [x] All existing repository `ValidationProblem` literals (incl. error snapshot tests) compile with the new field (covered by Phase 1; re-confirm here).
- [x] A schema with **no descriptions** produces byte-for-byte identical output to pre-feature behavior — no stray blank lines in pretty, JSON, or the status block (Decision #8).

### Full validation

- [x] `just lint` clean across the package area.
- [x] `just test` — all unit tests pass.
- [x] `just test-l2` — all integration tests pass.
- [x] `just doctest` — doctests pass.
- [x] **Final checkpoint:** end-to-end manual smoke — a missing-required and wrong-type failure on a schema with `->` descriptions renders the description sub-line in `md schema validate` (pretty + JSON) and in a compose schema-failure block; a description-less schema renders identically to before.
