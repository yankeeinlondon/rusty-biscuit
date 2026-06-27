---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-27
start_phase: 1
yolo: true
packages:
    - darkmatter
    - claudine
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/loop_expression.rs
    - claudine/lib/src/composition/loop_engine.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/schema_validation.rs
    - claudine/lib/src/composition/error.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/completion/schema_completion.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/compose/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/schema_validation.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-06-27-path-resolution/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
    - darkmatter
    - claudine
source_code:
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - claudine/lib/src/composition/lifecycle_executor.rs
    - claudine/lib/src/composition/loop_expression.rs
    - claudine/lib/src/composition/loop_engine.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/schema_validation.rs
    - claudine/lib/src/composition/error.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/completion/schema_completion.rs
documentation:
    - claudine/fixes/2026-06-27-path-resolution/plan.md
---

# Execution Plan: Lifecycle File-Reference Resolution Ignores the Launch Area

Reference spec: [`spec.md`](spec.md)

## Goal

Unify caller-supplied file-reference resolution on a single explicit,
stable fallback anchor — the captured launch area — so lifecycle event-time
resolution no longer depends on the mutated ambient process CWD. Preserve
the existing document-first contract for references authored inside the
prompt document.

## Root-Cause Summary

`switch_process_cwd(child_cwd)` repositions the process to the repo root
before lifecycle events interpolate. `file_exists`/`frontmatter` expression
functions fall back to ambient-CWD resolution (now wrong), and the launch
area — already captured as `LaunchWorkspaceContext.launch_cwd` and threaded
as `ctx_base_dir` for `ctx.*` capture — is never consulted for file
references. Schema `file` validation only "works" today by the luck of
running before the `chdir`.

## Key Design Decisions (locked by spec)

1. **Named fallback anchor.** Add a third, explicitly named field
   (`file_ref_fallback_dir: Option<PathBuf>`) to `ResolutionContext`. Do
   **not** overload `base_dir` (document-dir contract) or `ctx_base_dir`
   (`ctx.*` capture contract).
2. **Resolution order for local filesystem arguments:**
   1. absolute paths returned as-is by `FileReference`;
   2. document-relative via `resolve_from(base_dir)`;
   3. launch-area fallback via `resolve_from(file_ref_fallback_dir)` when present;
   4. **no ambient-CWD fallback** in production composition/lifecycle/schema paths.
3. **Schema validator shares the same resolver.** `jsonschema` 0.42's
   `with_format` accepts `F: Fn(&str) -> bool + Send + Sync + 'static`, so a
   closure capturing an immutable fallback dir is viable — **no thread-local
   state required** (resolves the spec's open question favorably).
4. **DRR threading in Claudine.** `StackExecutionContext::resolution_context()`
   derives the fallback from the already-present `ctx_base_dir` (the launch
   area) — no new field on `StackExecutionContext`.

---

## Phase 1 — Darkmatter: Canonical Resolver + `ResolutionContext` Fallback

**Goal:** establish the single resolution order and the shared helper every
other phase consumes. Foundational — all later phases depend on this.

**Packages:** `darkmatter/lib`

### Tasks

- [x] Add `pub file_ref_fallback_dir: Option<PathBuf>` to `ResolutionContext` in `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`. Update `ResolutionContext::new` (defaults to `None`), the `Default` impl, and every struct-literal construction site in the file/tests so they compile (notably the two `fetch_remote_text` test literals at `resolve_ctx.rs:216` and `:242`).
- [x] Add a `#[must_use] pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self` builder on `ResolutionContext`.
- [x] Extract a single canonical resolver helper (e.g. `pub(crate) fn resolve_file_ref_with_fallback(raw, base_dir: &Path, fallback: Option<&Path>) -> Result<Option<PathBuf>, FileReferenceError>`) encoding the four-step order above. Place it alongside `resolve_arg` in `functions.rs` (or a small sibling module) so both the expression path and (Phase 2B) the schema validator call it.
- [x] Rewrite `resolve_arg` (`functions.rs:931`) to call the shared helper with `ctx.base_dir` and `ctx.file_ref_fallback_dir.as_deref()`, **removing** the ambient-CWD `file_ref.resolve()` fallback branch. Keep the `magic_paths` injection step before resolution.
- [x] Replace the drifted doc comment at `functions.rs:915-924` (the "agreement with the ambient-CWD schema validator" rationale) with the new explicit launch-area-fallback contract. State the resolution order. Per repo drift rules: code is correct, the old comment is now wrong.
- [x] Update `expression_resolution_context` and `frontmatter_resolution_context` in `darkmatter/lib/src/markdown/compose/context/options.rs` to set `file_ref_fallback_dir: None` for now (Phase 2A threads the real value). This keeps the struct-literal exhaustive-check happy and is a no-op behavior change at this phase.

### Tests (co-located, L1)

- [x] `resolve_ctx` unit test: document-relative hit wins over a same-named file in the fallback dir (conflicting-filename precedence → verification goal #9).
- [x] `resolve_ctx` unit test: a path missing under `base_dir` but present under `file_ref_fallback_dir` resolves via the fallback.
- [x] `resolve_arg`/`file_exists_fn` unit test: with a fallback set and the process CWD mutated to an unrelated dir, resolution still succeeds and does **not** consult ambient CWD (no `std::env::set_current_dir` dependence).
- [x] Unit test: `ResolutionContext::new(base_dir)` (no fallback) preserves today's behavior for existing small unit tests.

### Validation Checkpoint

- [x] `just test` (darkmatter) green; `just lint` clean. No claudine-side changes yet, so claudine still compiles against the new `ResolutionContext` field (defaults keep behavior).

---

## Phase 2 — Darkmatter: Thread Fallback Through Production Builders

**Goal:** make the darkmatter-side production surfaces (`ComposeOptions` and
`DarkmatterSchemas`) carry and use the explicit fallback. Two **parallelizable
tracks** — both depend only on Phase 1.

**Packages:** `darkmatter/lib`

### Track A — `ComposeOptions` builders (parallel with Track B)

- [x] Add `pub(crate) file_ref_fallback_dir: Option<PathBuf>` to `ComposeOptions` in `context/options.rs`; default `None` in `new_with_context`; add to the `Debug` impl field list.
- [x] Add a `#[must_use] pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self` builder.
- [x] Wire `self.file_ref_fallback_dir` into `expression_resolution_context` and `frontmatter_resolution_context` so both `ResolutionContext`s carry it.
- [x] In the compose-stage `schema_validation::run` (`darkmatter/lib/src/markdown/compose/schema_validation.rs`), pass `options.file_ref_fallback_dir` into the `DarkmatterSchemas` builder it constructs (depends on Track B's `DarkmatterSchemas` API — coordinate the method name, or sequence this sub-task after Track B lands).

### Track B — `DarkmatterSchemas` + `darkmatter-file` validator (parallel with Track A)

- [x] Add `file_ref_fallback_dir: Option<PathBuf>` to `DarkmatterSchemas` (`schemas/mod.rs`) with a `#[must_use] pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self` builder. Update `Default`.
- [x] Generalize `register_darkmatter_formats` (`schemas/format.rs:60`) to accept the fallback dir and build the `darkmatter-file` format callback as a **closure capturing `Option<PathBuf>`** (viable because jsonschema 0.42 `with_format` takes `F: Fn(&str) -> bool + Send + Sync + 'static`). The closure calls the Phase 1 shared helper with the captured fallback.
- [x] Generalize `build_validator` (`schemas/validate.rs:164`) to receive the fallback and forward it to `register_darkmatter_formats`. Since `ValidatorCache` is owned by a single `DarkmatterSchemas` instance (one fallback), cache keying on schema JSON alone stays correct — document this invariant in a `//` note at the cache.
- [x] Ensure `$schema` **reference** resolution (`schemas::resolve`) is untouched — it stays document-relative. Only the `file`-typed **value** validator changes.

### Tests (co-located, L1)

- [x] Track A: `ComposeOptions` with a fallback produces a `ResolutionContext` carrying it (assert on the built context).
- [x] Track B: a `darkmatter-file` property value resolves via the captured fallback even when the process CWD is an unrelated dir (`#[serial_test::serial("darkmatter-file-cwd")]` per existing convention in `format.rs`).
- [x] Track B: `$schema: ./schema.yaml` and a root-union `$schema` string arm still resolve relative to the document dir, not the fallback → verification goal #6.

### Validation Checkpoint

- [x] `just test` + `just lint` (darkmatter) green after both tracks merge. No claudine caller yet sets the new builders, so behavior is unchanged for end users (fallback defaults to `None`).

---

## Phase 3 — Claudine: Thread Launch Area Into Lifecycle / Loop / Schema Contexts

**Goal:** make every lifecycle event, loop condition, and schema validation in
claudine resolve caller-supplied file references against the captured launch
area, independent of the post-`chdir` ambient CWD.

**Packages:** `claudine/lib`, `claudine/cli`

### Tasks

- [x] **Core DRY change.** Update `StackExecutionContext::resolution_context()` (`claudine/lib/src/composition/lifecycle_executor.rs:366`) to attach `self.ctx_base_dir` as the `file_ref_fallback_dir` when present, while keeping `base_dir` (document dir) as the primary anchor. Because `with_signal`/`with_error` copy `ctx_base_dir` already, no new field is needed on the struct.
- [x] Audit every `StackExecutionContext` construction site and confirm production paths set `ctx_base_dir` to the launch area:
  - `claudine/cli/src/commands/wrap/composition/mod.rs` — preflight `blocked` helper (~`:190`) and `init_ctx` (~`:1663`) already pass `Some(launch_workspace.launch_cwd.as_path())`; verify still correct after the change.
  - `claudine/lib/src/composition/loop_engine.rs` `build_loop_context` (`:1164`) sets `ctx_base_dir: lifecycle_ctx.launch_area`; verify.
  - `claudine/lib/src/composition/lifecycle_executor.rs` test helpers (`:1278`, `:1318`) intentionally use `None` — leave as-is (unit-test ergonomics).
- [x] **Loop conditions.** Add an optional `file_ref_fallback_dir` to `LoopExpressionLookup` (`claudine/lib/src/composition/loop_expression.rs`) with a `with_file_ref_fallback_dir` builder; have its `resolution_context()` propagate it. Update all `LoopExpressionLookup::new(...).with_base_dir(...)` call sites in `loop_engine.rs` (lines ~445, ~545, ~818, ~1140, ~1353, ~1366, ~1387) to also pass `lifecycle_ctx.launch_area` as the fallback so `file_exists` inside `loop.while`/`loop.until` resolves against the launch area.
- [x] **Claudine `ComposeOptions`.** Where claudine builds `ComposeOptions` for composition preparation (prepare layer / `schema_validation.rs` / wrap composition), set `.with_file_ref_fallback_dir(launch_workspace.launch_cwd.clone())` so prepare-time body interpolation and schema validation use the explicit anchor instead of the (currently-luckily-correct) ambient CWD.
- [x] **Claudine schema wrapper.** In `claudine/lib/src/composition/schema_validation.rs` (`:427`), pass the launch-area fallback into `DarkmatterSchemas::new().with_file_ref_fallback_dir(...)` (Phase 2B API). Source the fallback from the prepared launch workspace / prep context.
- [x] Note (out of scope, no change unless trivial): `claudine/cli/src/completion/schema_completion.rs:51,717` also call `DarkmatterSchemas::new()`; completions lack a launch-area context and the spec does not list them. Leave ambient behavior, add a one-line `//` note if helpful.

### Tests (co-located, L1)

- [x] Post-`chdir` independence: construct a `StackExecutionContext` with `base_dir` = a prompt dir, `ctx_base_dir` = a launch dir containing `spec.md`; `std::env::set_current_dir(repo_root)`; assert `resolve_string_value("{{file_exists(spec)}}", ...)` is `true` → verification goals #1, #3.
- [x] Prepare-time vs event-time agreement: assert the same `file_exists(spec)` value from both the `ComposeOptions` path (Phase 2A) and the `StackExecutionContext` path → verification goal #2.
- [x] `iteration` derivation: a focused unit test mirroring `review-feature.md`'s logic where `spec` carries `review_iterations` — assert `iteration` increments once the spec resolves (was stuck at `1` before the fix) → verification goal #4.

### Validation Checkpoint

- [x] `just test` + `just lint` (claudine) green. Manually verify the end-to-end repro from the spec symptom (`spec=fixes/.../spec.md` from the `claudine/` area renders `file_exists(spec)` as `true` in the `initialize` event).

---

## Phase 4 — Regression Suite + Full Validation

**Goal:** close every remaining verification goal with explicit regression
tests and run the full quality gates across both packages.

**Packages:** `darkmatter/lib`, `claudine/lib`

### Tasks

- [x] L1 regression (darkmatter or claudine): a path that exists **only** under the launch area (not under the prompt dir, not under the repo root) resolves — proves the new fallback is the source of the hit → verification goal #8. (`regression_path_only_under_launch_area_resolves` in `lifecycle_executor.rs`; three-way isolation across prompt dir / repo-root ambient CWD / launch area.)
- [x] L1 regression: an intentionally conflicting filename present in **both** the prompt dir and the launch area — the prompt-dir file wins (document-first) → verification goal #9 (re-affirmed end-to-end). (`regression_conflicting_filename_prompt_dir_wins` in `lifecycle_executor.rs`, plus `document_relative_hit_wins_over_fallback_conflict` at the resolver unit level.)
- [x] L1 regression: a document-relative body reference (`::file _senior-reviewer.md`) still resolves next to the prompt document after the fallback change → verification goal #5. (`body_file_transclusion_stays_document_relative_with_fallback` in `compose/tests.rs`.)
- [x] L1 regression: `$schema` reference + root-union `$schema` string arm remain document-relative (re-affirm Phase 2B test at the claudine integration level) → verification goal #6. (`schema_reference_stays_document_relative_through_claudine_load` + `root_union_schema_string_arm_stays_document_relative_through_claudine_load` in `schema_validation.rs`.)
- [x] L1 regression: a `file`-typed schema property and `{{file_exists(spec)}}` agree for the same `spec` value across prepare-time body interpolation, lifecycle event interpolation, and post-`chdir` schema validation → verification goal #7. (`file_property_and_file_exists_agree_across_schema_and_body` covers schema + body; `prepare_time_and_event_time_agree_on_file_reference` + `file_exists_resolves_against_launch_area_after_chdir` cover the lifecycle event-time dimension.)
- [x] Confirm `prompts/review-feature.md`'s `{{ctx.area}}/{{spec}}` prefixing is **unchanged** (agent-legibility, not a workaround) → spec scope note. (Verified: `{{ctx.area}}/{{spec}}` and `{{ctx.area}}/{{design}}` prefixing untouched by this work.)

### Validation Checkpoint (final gate)

- [x] `just test` green for **darkmatter** (lib 4757 + cli 528) and **claudine** (lib 2994 + contract 47 + cli 1761). `just test-l2`: the 93 non-interactive L2 tests pass; the 11 interactive **biscuit-tui schema-prompt PTY** tests (`level2_schema_prompt_pty`) fail uniformly on this host because the prompt never enters raw mode / the alternate screen (`\u{1b}[?1049h` marker times out) — an environmental backend limitation, not a regression: they fail identically across `string`/`boolean`/`enum`/`number` properties, none of which exercise the `file`-typed resolution this work changes, and the only sibling that does **not** drive the interactive widget passes.
- [x] `just lint` clean for both package areas.
- [x] `just doctest` clean for both package areas.
- [x] `cargo fmt --check` (read-only) reports no drift **introduced by this work** — the flagged diffs are a repo-wide local-rustfmt-vs-`main` drift across 160+ files (the documented "`main` is the formatting authority" artifact), landing on pre-existing committed lines this work never edited (e.g. `schema_completion.rs:47`/`:208`). Write-mode `cargo fmt` was **not** run, per repo policy.

---

## Parallelism Map

| Phase | Parallelizable? | Notes |
|-------|-----------------|-------|
| 1     | No              | Foundation; everything depends on the shared resolver + new field. |
| 2     | **Yes**         | Track A (`ComposeOptions`) and Track B (`DarkmatterSchemas`) are independent once Phase 1 lands. Merge the `schema_validation::run` sub-task after Track B's API is fixed. |
| 3     | Partial         | The core `resolution_context()` one-liner is sequential; the `LoopExpressionLookup`, `ComposeOptions`, and `schema_validation.rs` wiring can proceed in parallel once that lands. |
| 4     | Yes             | Independent regression tests; can be authored in parallel. |

## Risk Notes

- **jsonschema format callback:** confirmed `F: Fn(&str) -> bool + Send + Sync + 'static` in 0.42.2 (`options.rs:395`), so the closure-capture approach in Phase 2B is viable without thread-locals. If a future jsonschema bump narrows this, fall back to a post-validation `file`-property walk driven by the simplified schema's property metadata.
- **`ValidatorCache` invariant:** a single `DarkmatterSchemas` instance has one fallback; cache keying on schema JSON alone is correct. Document at the cache.
- **No ambient-CWD removal in tests:** `ResolutionContext::new(base_dir)` (no fallback) preserves existing unit-test ergonomics; only production constructors pass the fallback.
