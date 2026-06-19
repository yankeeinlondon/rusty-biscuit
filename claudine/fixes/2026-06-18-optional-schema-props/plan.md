---
agent: open_code/zai-coding-plan/glm-5.2
phases: 5
created: 2026-06-18
start_phase: 1
yolo: true
spec: claudine/fixes/2026-06-18-optional-schema-props/spec.md
source_files_during_phase_1:
- darkmatter/lib/src/markdown/schemas/simplified/convert.rs
source_files_during_phase_2:
- darkmatter/lib/src/markdown/schemas/coerce.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
source_files_during_phase_3:
- claudine/lib/src/composition/schema_validation.rs
docs_updated_during_phase_1: []
docs_updated_during_phase_2: []
docs_updated_during_phase_3: []
docs_created_during_phase_1: []
docs_created_during_phase_2: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_1: []
skills_files_updated_during_phase_2: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
- darkmatter/features/_completed/2026-05-11-schemas/spec.md
- darkmatter/docs/topics/schema-definition.md
- claudine/docs/topics/composition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .opencode/skill/darkmatter/SKILL.md
- .claude/skills/darkmatter/SKILL.md
- .opencode/skill/claudine/SKILL.md
- .claude/skills/claudine/SKILL.md
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
- darkmatter/lib/src/markdown/schemas/simplified/convert.rs
- darkmatter/lib/src/markdown/schemas/coerce.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- claudine/lib/src/composition/schema_validation.rs
documentation:
- darkmatter/features/_completed/2026-05-11-schemas/spec.md
- darkmatter/docs/topics/schema-definition.md
- claudine/docs/topics/composition.md
packages:
- darkmatter
- claudine
hash: a85aedfb66d03640-149cf21bc95588be
last_updated: 2026-06-19
---

# Plan — Optional Schema Properties Accept `null` as "Absent"

## Context

A composition document with an optional `$schema` property whose frontmatter value resolves to `null` (a common Darkmatter ternary result) is wrongly rejected by schema validation. Root cause: `darkmatter/lib/src/markdown/schemas/simplified/convert.rs::atom_to_schema` only null-tolerates the optional `file` + empty-string sentinel (Decision A). Every other optional typed atom emits a bare typed fragment that rejects `null` under Draft 2020-12 semantics.

**The fix is a single semantic rule:** a typed property is nullable if and only if it is not `required`. The change lives almost entirely in `darkmatter` (converter + coercion recognizer). Claudine needs **no** logic changes — only an end-to-end regression test.

> **Hard coupling warning:** the converter change (`convert.rs`) and the coercion recognizer change (`coerce.rs`) **must land together**. Emitting nullable `anyOf` wrappers without updating `coercion_target` regresses existing optional-string/number/boolean coercion, because `coercion_target` returns `None` for unrecognized `anyOf` shapes. See Phase 2 checkpoint.

### Files in scope

| File | Change |
|------|--------|
| `darkmatter/lib/src/markdown/schemas/simplified/convert.rs` | Generalize Decision A into a two-stage nullable wrap; lift union null-wrap to property level |
| `darkmatter/lib/src/markdown/schemas/coerce.rs` | Teach `coercion_target` to recurse through Darkmatter's nullable wrappers |
| `claudine/lib/src/composition/schema_validation.rs` | End-to-end regression test (`#[cfg(test)] mod tests`) |
| `darkmatter/features/_completed/2026-05-11-schemas/spec.md` | "Optional by default" callout |
| `darkmatter/docs/topics/schema-definition.md` | Public docs + `any(required)` note |
| `claudine/docs/topics/composition.md` | "Required vs Optional" table row for present-`null` |
| `.opencode/skill/darkmatter/SKILL.md` (and/or `.claude/skills/darkmatter/`) | One-liner: optional = nullable (conditional) |

### Out of scope (per spec)

No new `nullable`/`non-null` authoring keyword. No change to claudine's `categorize_problems` / `is_required` / `interactive_shape_for_atom` (Acceptance M). No change to YAML null serialization, root-level unions, or drop-invalid-optionals logic.

---

## Phase 1 — Schema Converter: Nullable Wrapper (`darkmatter convert.rs`)

**Goal:** make `atom_to_schema` emit `{"anyOf":[{"type":"null"},<typed>]}` for every non-required atom, preserve the optional-`file` empty-string arm, keep required atoms byte-for-byte unchanged, and emit clean single-level `anyOf` for optional property-level unions.

**Dependency:** None — this is the foundation. Sequential within the phase (all tasks touch one function cluster).

- [x] **1.1** Add two private helper fns next to `atom_to_schema`:
  - `wrap_optional_null(inner: Value) -> Value` → `{"anyOf":[{"type":"null"}, inner]}` (string `"null"`, per Draft 2020-12 — never `{"type": null}`).
  - `wrap_optional_null_with_empty_file(inner: Value) -> Value` → `{"anyOf":[{"type":"null"},{"const":""}, inner]}` (generalizes the existing Decision A `json!({ "anyOf": [ { "const": "" }, inner ] })`).
- [x] **1.2** Introduce a "union-arm mode" so per-arm wrapping can be suppressed. Extract the typed-fragment + array-build logic into an inner helper (e.g. `atom_fragment_without_null_wrap(name, atom, required) -> (Value, bool)`) that returns the **bare** fragment (array constraints applied, default/description attached), and have `atom_to_schema` call it. `union_property_to_schema` will call the inner helper directly so the general null-wrap is not applied per-arm. (Equivalently: add a `suppress_null_wrap: bool` parameter — choose whichever matches surrounding style.)
- [x] **1.3** Rewrite the wrap-selection block in `atom_to_schema` (convert.rs:199-218) to the two-stage form from spec §Proposed Implementation:
  - optional scalar `file` → `wrap_optional_null_with_empty_file(inner)` (preserves `""` tolerance, adds `null`).
  - optional scalar non-file (string/number/boolean/boolish/numberlike/date family/url/email/enum/object/inline-object/`any`) → `wrap_optional_null(inner)`.
  - optional array → `wrap_optional_null(finished_array_fragment)` (wrap the *finished* array, after `apply_array_constraints` — constraints stay inside).
  - required anything → bare `inner` (byte-for-byte unchanged).
- [x] **1.4** Keep the existing `default`/`description` attachment block (convert.rs:222-229) attaching annotations to the **outer** property schema (the wrapper), exactly as it does today for the optional-`file` wrapper. Do **not** move annotations onto the typed arm.
- [x] **1.5** Update `union_property_to_schema` (convert.rs:134-176): call the inner helper from 1.2 per arm (no per-arm null wrap). After building the arm `anyOf`, wrap the whole union once in `{"anyOf":[{"type":"null"}, <union anyOf>]}` when `!required`. Preserve the optional-`file` empty-string arm **inside** the file arm fragment (it comes from the inner helper naturally). Required unions get neither the property-level null arm nor the empty-string tolerance.
- [x] **1.6** Split `v1_scalar_atoms_are_byte_for_byte_unchanged` (convert.rs:1092-1129) into two tests:
  - `required_scalar_atoms_are_byte_for_byte_unchanged` — identical expected keys to today (Acceptance J).
  - `optional_scalar_atoms_emit_null_wrap` — documents the new `anyOf` shape per optional scalar type.
- [x] **1.7** Add converter unit tests covering Acceptance A–I (use `convert(...)` + `build_validator(...).is_valid(...)` pattern from `optional_file_accepts_empty_string_as_absent`, convert.rs:704-712):
  - [x] **A** every optional primitive validates `{}`, `{"opt": null}`, and a well-typed non-null value; required variant rejects `{"opt": null}`.
  - [x] **B** optional `file` validates `{}`, `{"opt": null}`, `{"opt": ""}`, `{"opt":"@x.md"}`; existing `optional_file_accepts_empty_string_as_absent` still passes.
  - [x] **C** optional `object` and inline `{ foo: string }` each validate `{}` and `{"opt": null}`.
  - [x] **D** optional `string[]` validates `{}`, `{"opt": null}`, `{"opt": []}`, `{"opt":["a"]}`.
  - [x] **E** optional `[string, number]` validates `{}`, `{"opt": null}`, `{"opt":"x"}`, `{"opt":3}`.
  - [x] **F** optional `string(not-empty; min(5))` validates `{"opt": null}` but still rejects `{"opt":""}` and `{"opt":"ab"}`.
  - [x] **G** required `string(required)` rejects `{"req": null}` with a Type problem.
  - [x] **H** optional `any` validates `{}`, `{"opt": null}`, any non-null; `any(required)` rejects `{}` but accepts `{"req": null}` (presence-only). Add a doc-comment in the test stating this is intentional compatibility preservation.
  - [x] **I** optional `string(default(hello)) -> A greeting` carries `default` and `description` on the **outer** wrapper object.
- [x] **1.8** Add a test asserting optional property-level union output is a **single-level** `anyOf` plus the null arm (no nested per-arm `anyOf`), e.g. `opt: [string, number]` → `anyOf` length 3: `[{"type":"null"}, {"type":"string"}, {"type":"number"}]` (or the equivalent single-level shape your lift produces). Document the exact emitted shape.

**Checkpoint (Phase 1):** `cargo test -p darkmatter --lib markdown::schemas::simplified::convert` is green. Manually eyeball one optional `string` fragment — it must be `{"anyOf":[{"type":"null"},{"type":"string"}]}` and the required fragment must be `{"type":"string"}`.

---

## Phase 2 — Coercion: Nullable Wrapper Recognition (`darkmatter coerce.rs`)

**Goal:** make `coercion_target` transparent to nullable wrappers for non-null values, while leaving `Value::Null` untouched. **Must land with Phase 1** — without this, optional `string`/`number`/`boolean`/array/inline-object coercion silently regresses the moment Phase 1 ships.

**Dependency:** Phase 1 (recognizes the shapes Phase 1 emits). Sequential.

- [x] **2.1** In `coercion_target` (coerce.rs:76), before the existing `anyOf`/`type` dispatch, add a recognizer for Darkmatter's nullable wrapper: an `anyOf` whose arms are exactly `[{"type":"null"}, <typed>]` (2 arms) or `[{"type":"null"}, {"const":""}, <typed>]` (3 arms, optional-file form). When recognized and **the wrapper is Darkmatter's**, recurse into the typed arm via `coercion_target`. Use a dedicated helper (e.g. `unwrap_nullable_arm(schema) -> Option<&Value>`) so the exact-shape boundary stays explicit.
- [x] **2.2** Generalize the optional-`file` wrapper recognizer: the 3-arm `null`/`const:""`/file-typed form recurses into the file-typed arm (which is itself `ToString`).
- [x] **2.3** For property-level nullable unions, recognize the outer `anyOf: [{"type":"null"}, <inner anyOf>]` shape and recurse into the inner union `anyOf` so the existing `coerce_property_union` path (coerce.rs:341) handles non-null values unchanged. Ensure `coerce_object`'s step 2 (the `prop_schema.get("anyOf")` branch at coerce.rs:314) still receives the **inner** arm list after unwrapping the null arm.
- [x] **2.4** Leave `Value::Null` untouched end-to-end: coercion never converts null (the existing scalar coercers already return `None` for non-string/non-number — add an explicit regression assertion that an optional `string` property holding `null` yields `changed: false` and the value is preserved).
- [x] **2.5** Keep boolish/numberlike exact-shape matching strict: the nullable recognizer must unwrap to the **existing** exact boolish/numberlike shapes, not loosen `target_from_any_of`. Verify `unrelated_boolean_enum_union_is_none`, `boolish_subset_enum_union_is_none`, `unrelated_number_string_pattern_union_is_none` (coerce.rs:591-629) still hold after unwrapping.
- [x] **2.6** Add coercion regression tests (Acceptance L):
  - [x] optional `string`/`number`/`boolean` holding a coercible scalar (`"42"`, `"true"`, `7`) still coerces through the null wrapper; `null` is untouched.
  - [x] optional `boolish`/`numberlike` exact shapes still coerce when wrapped (nullable wrapper → inner boolish/numberlike anyOf → target).
  - [x] optional `string[]` (nullable array wrapper) still coerces element-wise for a non-null array; `null` untouched.
  - [x] optional inline object `{ enabled: boolean }` (nullable wrapper) still coerces inner properties for a non-null object; `null` untouched.
  - [x] optional property-level union `[string, number]` (nullable wrapper) still runs per-arm coercion for non-null values.
- [x] **2.7** Re-run the existing `coercion_is_idempotent` (coerce.rs:982) and `null_array_object_untouched_against_scalar_targets` (coerce.rs:1003) tests; both must remain green.

**Checkpoint (Phase 2):** `cargo test -p darkmatter --lib markdown::schemas::coerce` is green. **Critical cross-check:** a quick standalone assertion that `coercion_target(&json!({"anyOf":[{"type":"null"},{"type":"string"}]})) == Some(ToString)` — if this returns `None`, Phase 1 cannot ship.

---

## Phase 3 — End-to-End Claudine Validation (`claudine schema_validation.rs`)

**Goal:** lock the incident closed with an end-to-end test and prove claudine needs no logic changes (Acceptance K + M).

**Dependency:** Phases 1 + 2 merged. Sequential.

> **Note:** `claudine/prompts/review-feature.md` does **not** exist in-repo today. Reproduce the *pattern* (optional `string` schema property whose Darkmatter template resolves to `null`), not the literal file.

- [x] **3.1** Add a test in `claudine/lib/src/composition/schema_validation.rs::tests` (after `make_source` helper, schema_validation.rs:1289) that builds a composition source with:
  - `$schema: { design: string }` (optional `string`), and
  - a `design:` frontmatter value driven by a Darkmatter ternary that resolves to `null` (mirror the incident: `design: "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}"` with no `design.md` present in the temp dir).
- [x] **3.2** Assert `prepare_direct_with_schema(&source, PrepareOptions::default())` succeeds (`Ok`) and that `prepared.effective_frontmatter["design"]` is `Value::Null` — i.e. the resolved null is **retained**, not silently dropped as an invalid optional (Acceptance K).
- [x] **3.3** Add a sibling test for the **inline** path: `prepare_inline_with_schema` succeeds on an equivalent inline-composition source whose optional `string` resolves to `null`.
- [x] **3.4** Add a guard test that a **required** `string` whose template resolves to `null` still fails with `CompositionError::SchemaValidation` (proves `is_required` / `categorize_problems` correctly classify the required case without modification — Acceptance M). Assert the categorization reads `Constraint::Required` from the `PropertyAtom`, not the JSON Schema.
- [x] **3.5** Confirm **no** edits are needed in `categorize_problems` (schema_validation.rs:558), `is_required` (schema_validation.rs:630), or `interactive_shape_for_atom` (schema_validation.rs:658). If implementation reveals any of these need touching, **stop and revise the spec** (spec §M) before proceeding.

**Checkpoint (Phase 3):** `cargo test -p claudine --lib composition::schema_validation` is green, including the new null-resolution tests.

---

## Phase 4 — Documentation Updates

**Goal:** make "optional = nullable" the documented contract everywhere users read it.

**Dependency:** None — **fully parallelizable with Phases 1–3.** All four doc tasks are independent of each other and may be done concurrently.

- [x] **4.1** `darkmatter/features/_completed/2026-05-11-schemas/spec.md` — under "Optional by default" (~line 63), add: *"An optional property accepts `null` as a sentinel for absent. A frontmatter slot whose value resolves to `null` (e.g. from a Darkmatter ternary `{{ x ? y : null }}`) validates the same way as a missing key."* Include a short worked example.
- [x] **4.2** `darkmatter/docs/topics/schema-definition.md` — make the same update under the public "Optional by default" section, and add a note in the type table that `any(required)` remains **presence-only** because `any` already includes `null`.
- [x] **4.3** `claudine/docs/topics/composition.md` — in the "Required vs Optional" table (~line 322), add a row/note clarifying that a *present* `null` is in the "Present and valid" column for optional properties (today the table only covers Missing / Present-valid / Present-invalid).
- [x] **4.4** `.opencode/skill/darkmatter/SKILL.md` and/or `.claude/skills/darkmatter/SKILL.md` — **conditional:** only if the SimplifiedSchema section mentions type validity. Add a one-liner: optional = nullable. If no such section exists, skip (do not invent one).
- [x] **4.5** Manual check (no code change expected): run the darkmatter CLI `schema explain` (or equivalent) against a doc with an optional `string` and confirm the type info still reads cleanly through the new `anyOf` wrapper. If the explainer breaks, **fix the reader** to look through Darkmatter's nullable wrapper — do **not** relocate property annotations off the wrapper (spec §Reader's note).

**Checkpoint (Phase 4):** Docs render cleanly; `md hash` updated on any Markdown docs whose body changed (per repo hashing convention, use Darkmatter's `md hash`).

---

## Phase 5 — Final Validation & Acceptance Sign-off

**Goal:** prove the whole Acceptance Criteria matrix (A–M) holds across the workspace and the original incident is resolved.

**Dependency:** Phases 1–4 complete. Sequential.

- [x] **5.1** Run darkmatter full lib test suite: `cargo test -p darkmatter --lib markdown::schemas` (covers convert + coerce + validate integration).
- [x] **5.2** Run claudine full lib test suite: `cargo test -p claudine --lib composition` (covers schema_validation end-to-end).
- [x] **5.3** Run doctests for the two changed modules: `cargo test --doc -p darkmatter markdown::schemas`.
- [x] **5.4** Walk the Acceptance Criteria checklist explicitly (spec §Acceptance Criteria A–M) and confirm each is covered by a passing test. Tabulate results:
  - [x] A — optional primitives accept `null`
  - [x] B — optional `file` accepts `null` and `""`
  - [x] C — optional `object` / inline objects accept `null`
  - [x] D — optional arrays accept `null`
  - [x] E — optional property-level unions accept `null`
  - [x] F — constraints bypassed by `null`
  - [x] G — required typed atoms reject `null`
  - [x] H — `any` behavior explicit (optional accepts null; `any(required)` presence-only)
  - [x] I — `default`/`description` survive the wrap
  - [x] J — required-atom snapshot byte-for-byte stable
  - [x] K — end-to-end claudine test (review-feature pattern)
  - [x] L — coercion regression (null untouched, non-null coerces)
  - [x] M — no claudine code changes required
- [x] **5.5** Manual smoke test (the incident reproduction): `claudine compose` a prompt whose `$schema` declares an optional `string` and whose frontmatter ternary resolves to `null` against a folder missing the looked-up file. Confirm compose succeeds where it previously aborted with `CompositionError: schema validation failed`.
- [x] **5.6** Final `cargo fmt --check` (read-only, per repo policy — never write-mode) and clippy on the two changed crates.

**Checkpoint (Phase 5):** All green. The plan is complete when 5.5 passes — that is the literal incident closed.

---

## Parallelism Map

| Work | Can run in parallel with |
|------|--------------------------|
| Phase 1 (convert.rs) | Phase 4 (docs) |
| Phase 2 (coerce.rs) | Phase 4 (docs) — but **not** independent of Phase 1 (must follow it) |
| Phase 3 (claudine test) | Phase 4 (docs) — but **not** independent of Phases 1+2 |
| Phase 4 tasks 4.1–4.4 | Each other (all four doc edits are independent) |

**Recommended critical path:** 1 → 2 → 3 → 5, with Phase 4 dispatched in parallel at the start.

## Risk Register (from spec §Gotchas)

1. **Coercion coupling** — Phase 1 without Phase 2 regresses optional coercion. The Phase 2 checkpoint cross-check is mandatory before merge.
2. **Annotation placement** — `default`/`description` must stay on the outer wrapper, not the typed arm.
3. **Array constraints** — `apply_array_constraints` output stays *inside* the array arm; the null wrap wraps the finished array.
4. **Union output shape** — lift the null wrap to the property level; avoid nested per-arm `anyOf`.
5. **`type: "null"` spelling** — string form, never `{"type": null}`.
6. **Snapshot drift** — split required vs optional snapshot tests; required must be byte-identical.
7. **`any(required)` carve-out** — deliberately preserved as presence-only; document as intentional.