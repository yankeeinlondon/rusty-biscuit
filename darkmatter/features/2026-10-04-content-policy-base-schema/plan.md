---
area: darkmatter
kind: plan
total_phases: 6
created: 2026-10-05
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages: []
---

# Plan: Compile Content Policy's Schema into the Base Schema

## Summary and Definition of Done

Darkmatter's runtime baseline (`darkmatter/docs/schemas/darkmatter.yaml`, loaded
by `include_str!` in `lib/src/markdown/schemas/mod.rs`) gains a typed,
non-empty `content_policy` list whose schema is Content Policy's exported
`EDITOR_SCHEMA`. Today the baseline is parsed *without* import expansion
(`darkmatter_base_schema_ref` / `darkmatter_base_json_schema_arc`, both
`expect(..)` on failure), so the new `policy[](min(1))@../../../content-policy/...`
reference would panic. The work therefore has three coupled parts:

1. **Embedded resolution** — an internal table of virtual-path → embedded text
   (`darkmatter/docs/schemas/darkmatter.yaml`, `content-policy/schemas/content-policy.yaml`)
   that the normal import resolver consults, so both the typed schema (DMLS
   completion/hover) and compiled schema (validation) are fully resolved in an
   installed binary with no checkout.
2. **Default entry points** — `md schema validate` (`load_api` in
   `cli/src/commands/schema/validate.rs`) starts using the embedded baseline,
   with `--no-baseline-schema` / `DARKMATTER_NO_BASELINE_SCHEMA` / `--schema` /
   `BASELINE_SCHEMA` precedence read from the captured request snapshot.
   `DarkmatterSchemas::new` stays unconfigured.
3. **Proof and docs** — parser/validator agreement tests over shared complete
   declarations, subprocess tests outside the repo, and drift updates.

**Done when** every acceptance criterion (AC 1–7) in the spec is met:
schema-free example passes library/CLI/compose/DMLS; unknown rule, unknown
action, empty list and wrong shape are rejected with item-level diagnostics,
typed problem codes, baseline origin and source positions; DMLS completes and
hovers inside entries; installed-binary tests pass with no schema files on disk;
disk and embedded copies behave identically; CLI precedence is tested; shared
declaration cases pin agreement and each documented limitation; `just test` and
`just lint` pass in `darkmatter/` and `content-policy/`; docs/skills/READMEs/
dependency catalogs are updated. Terminal end state is "implementation complete,
ready for review" (no `just complete`, no move to `_completed`).

## Dependency Gate

`2026-09-28-recursive-schema-types` is still `draft-spec` with no plan in its
directory. **This feature cannot start implementation until that prerequisite
lands** (union named type as list item, list-level `min`, definitions surviving
merge). Phase 1 verifies this and stops the plan if it is not true.

## Input Robustness Matrix

The only load-bearing field read from a document is `content_policy` (frontmatter
validated against the compiled schema), plus the env var
`DARKMATTER_NO_BASELINE_SCHEMA`. Outcomes are asserted through the public
validation result (`md schema validate` JSON / `ValidationReport`), not a parser
return. One test walks every row from one real fixture with one edit per row; a
control row proves the unedited fixture is valid.

| Shape | `content_policy` edit | Outcome |
| --- | --- | --- |
| control | example from the spec | valid |
| absent | key omitted | valid; no policy or `null` inserted on compose |
| explicit null | `content_policy:` / `null` | valid (Darkmatter optional-property convention); parser rejects — documented difference |
| wrong type, whole field | `content_policy: 123`, `content_policy: ValidFor(3mo)` (bare string) | reject, problem at `/content_policy` |
| wrong type, one element | `[ValidFor(3mo), 123]` | reject, problem at `/content_policy/1` |
| wrong type, every element | `[123]` | reject |
| empty | `[]` | reject (min 1) |
| duplicate key | `content_policy` twice in frontmatter | outcome per Darkmatter's existing duplicate-key rule (Phase 1 ruling R7) |
| trailing/invalid content | frontmatter with garbage after valid YAML | exit code 3 (parse failure), not 1 |

| Env value | Outcome |
| --- | --- |
| unset | embedded baseline active |
| `1`, `true`, `TRUE`, `yes`, `YES` | baseline disabled |
| empty, `0`, `false`, `no`, other | baseline stays active (matches compose) |

Smell greps before closing: `unwrap_or_default()`, `filter_map(.. as_str())`,
`.ok()` on parse of an embedded document; `std::env::var` in CLI code (compose.rs
`apply_compose_baseline_schema` currently reads the process env — see R6).

## Phase 1: Gate, Rulings, and Baseline Survey

Wave 1 (parallel, read-only except the log):

- [x] **Verify prerequisite** — confirm `2026-09-28-recursive-schema-types` is implemented (status `implemented`/completed); if not, stop and report. Add a throwaway L1 test (kept if useful) that a named union type works as `name[](min(1))`, survives `merge_baseline`, and appears in `DmlsTypedSchema`/typed view.
    - **NOT IMPLEMENTED — gate failed, plan stopped.** Spec status is `draft-spec` (no plan in its directory); `TypeExpr` has no `Ref` variant; lowering emits no `$defs`; `resolve.rs:1326` still rejects `[]` over a union-typed named type. The throwaway L1 probe was added, failed with exactly that error, and was removed (evidence in the implementation log). Phases 2–6 must not start until the prerequisite lands.
- [x] **Map baseline accessors** — list every caller of `darkmatter_base_schema`, `darkmatter_base_schema_ref`, `darkmatter_base_json_schema(_ref)`, `DarkmatterSchemas::new`, `with_baseline*` (known: `dmls/src/providers/frontmatter.rs`, `dmls/src/overlay/schema.rs`, `lib/src/markdown/compose/context/catalog.rs`, `lib/benches/effective_schema_ownership.rs`, lib tests `base_schema_end_to_end`, `meta_schema_phase1/5`, `binding_contract`, `cli/src/commands/compose.rs`). Record which need fully resolved typed vs JSON form and which currently panic on failure.
    - Table written to the implementation log. `binding_contract` has no baseline-schema coupling (its `BindingView::baseline()` is the expression binding view).
- [x] **Map import resolver seams** — find where `resolve.rs` reads import files and assigns definition identities, to choose the injection point for the embedded table (ruling R2).
    - Seam map in the implementation log: baseline entry (`mod.rs:148`, no import expansion today), `ImportEngine::resolve_namespace` (`resolve.rs:1141`, the `@file`→disk seam), `NamespaceKey::File(canonical_path)` identity, `dependencies` insertion to skip for embedded namespaces.
- [x] **Confirm `content-policy` constraints** — verify `content-policy/lib/Cargo.toml` features (`file-adapter`) and that Content Policy has no Darkmatter dependency with all features on (`cargo tree -p content-policy -e normal --all-features`); check `darkmatter/docs/dependencies.md` and the repo dependency catalog for the add.
    - Confirmed: features `default = []` + `file-adapter`; `cargo tree --all-features` has zero darkmatter crates; `deps-check` in `content-policy/justfile` guards the direction; `darkmatter/docs/dependencies.md` has no content-policy entry yet (add in Phase 2); root `docs/dependencies.md:339,418` lists it.

Wave 2 (sequential, author-facing):

- [x] **Record rulings** — resolve R1–R8 below in the implementation log before Phase 2.
    - All eight resolved in the implementation log's "Rulings (R1–R8)" section. R1 defers the editor `rule` check (Phase 6 only on author confirmation); R2/R3 fix the embedded-table injection and virtual identity; R4 keeps accessor signatures; R5 confirmed against `dmls/src/overlay/schema.rs:791`; R6 keeps compose's allowlisted process-env read unless the shared helper allows the snapshot (validate reads the snapshot either way); R7/R8 as planned.

### Necessary Rules

- **R1 — Required `rule` (spec Open Question).** The spec recommends scoped `(required)` on a union reference but says the author must confirm. *Plan default:* **defer** the editor check (option 3) and keep Phases 1–5 independent of it; Phase 6 (optional) implements the recommended option only if the author confirms. Until confirmed, missing-rule rejection is not an acceptance criterion; the test records parser-rejects/schema-accepts for it.
- **R2 — Where the embedded table plugs in.** Ruling: inject at the import-resolution seam as a pre-disk lookup keyed on the *virtual path* only for imports originating from an embedded document; user `FileReference`s never consult it.
- **R3 — Virtual definition identity.** Ruling: identity is the virtual path string prefixed so it cannot equal any resolved host path (e.g. `embedded:` namespace in the internal identity only, not a public scheme); asserted by an isolation test.
- **R4 — Accessor signatures.** `darkmatter_base_schema() -> SimplifiedSchema` and `..._json_schema()` are infallible and panicking today. Ruling: keep signatures; resolution failure of a *shipped* embedded document stays a panic with a clear message (library bug), guarded by a passive corpus test so it fails in CI not at runtime. Record any departure in the log.
- **R5 — "Resolved" typed schema for DMLS.** Ruling: the typed view returned by `darkmatter_base_schema()` carries referenced definitions inline (including unions) so DMLS needs no second resolution pass; confirm against `dmls/src/overlay/schema.rs:791`.
- **R6 — Env access.** `cli/src/commands/compose.rs:731` reads `std::env::var("DARKMATTER_NO_BASELINE_SCHEMA")` directly while the spec requires the captured request snapshot. Ruling: validate reads `request.snapshot().env()`; extract one shared parse of the disable values used by both validate and compose; moving compose to the snapshot is in scope only if the shared helper requires it (otherwise log as a follow-up and update `context_construction_guard.rs`/`spawn_site_guard.rs` allowlists accordingly).
- **R7 — Duplicate frontmatter key.** Ruling: assert whatever Darkmatter's existing frontmatter parser does; do not change it. Document in the matrix test.
- **R8 — Wider measurement.** Spec says no performance spike. A cache-reuse test (second load returns the same `Arc`) suffices. If the author wants installed-binary timing or multi-OS measurements, that is an author decision, not scheduled here.

### Spikes

None. The spec's own prototype (~110 lines, installed-binary operation) already answers the feasibility question; recorded as a ruling, no spike scheduled.

### Validation checkpoint 1

- [x] Prerequisite verified; rulings logged; baseline accessor/caller table written to the implementation log.
    - Prerequisite verified **not implemented** — the dependency gate failed and the plan is stopped before Phase 2. See the implementation log for the probe evidence, the accessor/caller table, the resolver seam map, the content-policy constraint confirmation, and rulings R1–R8.

## Phase 2: Embedded Schema Table and Resolution

Wave 1:

- [ ] **Add dependency** — add `content-policy` (`default-features = false`, no `file-adapter`) to `darkmatter/lib/Cargo.toml`; update `darkmatter/docs/dependencies.md` and the root dependency catalog; assert no reverse dependency from Content Policy.
- [ ] **Embedded table** — new internal module (e.g. `schemas/embedded.rs`): static table of two entries (`darkmatter/docs/schemas/darkmatter.yaml` via `include_str!`, `content-policy/schemas/content-policy.yaml` = `content_policy::EDITOR_SCHEMA`); no production `include_str!` of the content-policy path.
- [ ] **Virtual path normalizer** — lexical `/`-only resolution of `./`, `../`; reject escape above the virtual root, absolute paths, backslashes, drive letters, empty segments; no `canonicalize`, no CWD. Unit-test on all three OS path styles.

Wave 2 (depends on Wave 1):

- [ ] **Resolver hook** — teach import expansion to resolve imports from an embedded origin against the table (R2); missing document, missing type, malformed import are typed errors with no disk/network fallback; reuse the prerequisite's cycle/definition rules.
- [ ] **Resolve + lower once** — replace `BASE_SCHEMA`/`BASE_JSON_SCHEMA` initialisation with the resolved pipeline, keeping `OnceLock` caches; carry definitions through baseline merging and the typed view (R3, R5).
- [ ] **Origin and positions** — problems keep `SchemaOrigin::baseline()`, instance path, and document source positions; embedded names are not added to dependency watchers or `referenced_files`.

Wave 3:

- [ ] **Update runtime baseline** — add `content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"` to `darkmatter/docs/schemas/darkmatter.yaml`; optional, no default.

### Validation checkpoint 2

- [ ] `just test` in `darkmatter/` compiles and existing baseline tests are triaged: those that parse/convert the authored baseline without import expansion are updated (Phase 5 finishes them). `cargo tree` shows no `darkmatter` under `content-policy`.

## Phase 3: CLI Selection Rules

Depends on Phase 2 (the embedded baseline must be loadable). Can run concurrently with Phase 4 test authoring that does not touch the CLI.

- [ ] **Argument** — add `--no-baseline-schema` to the `schema validate` args, with a clap `conflicts_with = "schema"` usage error.
- [ ] **Selection function** — rewrite `load_api` per the five-step order: opt-out flag; `--schema`; truthy env opt-out (ignore `BASELINE_SCHEMA`); `BASELINE_SCHEMA`; embedded baseline via `DarkmatterSchemas::new(ctx).with_baseline(darkmatter_base_schema())`. Env from `request.snapshot().env()` (R6); custom paths resolve in launch context; a failing custom baseline stays exit 2, never falls back.
- [ ] **Output** — JSON fields and exit codes (0/1/2/3) unchanged; `schema` stays null when only the built-in baseline applied; pretty output must not say "no effective schema" when the built-in baseline validated. Opt-out with no document/trigger schema prints the "no effective schema" form.
- [ ] **Preserve separate meaning** of `--no-trigger-schemas`; document/trigger schemas still validate after opt-out.

### Validation checkpoint 3

- [ ] Existing `cli/tests/l1` schema-validate tests triaged: assertions relying on schema-free validation either pass `--no-baseline-schema`/env or are rewritten; one test explicitly keeps the old behavior via opt-out (`last_updated: not-a-date` fails by default, passes with opt-out).

## Phase 4: Tests (Wave-parallel)

Use the repository Test Toolkit, `CliProcessFixture`, and `application_input`/`application_input_removed`; no windows gain focus; no L2/L3 tier.

Wave 1 (independent test files, parallel subagents):

- [ ] **Declaration agreement corpus** — extend `lib/tests/l1/content_policy_editor_schema.rs`: one shared collection of *complete* real-YAML declarations (block lists; quoted flow-list rules containing commas), validated through the default baseline and through `Policy::from_declaration` (no clock/evidence). Rows exactly as in the spec table: both-accept (each rule, actions, mixed list, `Evergreen` alone), both-reject (`Duration(3mo)`, bad units/case, zero/leading-zero durations, nested property refs, missing action, unknown action, non-list non-null, empty list), parser-only-reject (impossible dates, overflow, `vault:`/URL paths, `{{` in path), editor-limit cases (`Evergreen` + other, extra long-form fields, explicit null), and missing `rule` (schema outcome per R1). Each documented limitation asserts both outcomes so drift fails.
- [ ] **Robustness matrix test** — the table above, one test per format (frontmatter YAML), one edit per cell, control row first.
- [ ] **Embedded resolution tests** — nested same-file imports, missing embedded document, missing type, invalid virtual paths (`..` escape, absolute, backslash), definition isolation with document- and trigger-supplied schemas of the same type names, disk-resolved authored baseline vs embedded equivalence (validation behavior and type info), second load reuses cached `Arc`.
- [ ] **CLI precedence tests** (`cli/tests/l1`) — each of the five steps, flag/`--schema` conflict, broken custom baseline (exit 2, no fallback), env truthy/non-truthy values, document schema replacing `content_policy`, document/trigger validation after opt-out, JSON fields and quiet behavior unchanged.

Wave 2 (depends on Wave 1 fixtures):

- [ ] **Installed-binary subprocess tests** — run built `md` and `dmls` with the working directory and `HOME`/schema env pointing at a temp dir containing only the test documents (`SCHEMAS_DIR` unset); assert validation, compose pending-value behavior, and DMLS completion of rule forms/actions plus hover inside list entries. Portable to macOS/Linux/Windows/WSL2; no new CI cells.
- [ ] **Passivity assertions** — validation leaves document bytes/frontmatter unchanged, performs no file reads of watched paths (use a `FileChanged(...)` rule pointing at a missing file), and never stamps `last_updated`.
- [ ] **Compose and DMLS entry points** — schema-free example passes `md compose` and DMLS diagnostics; ordinary no-policy document gains no `content_policy` key.

### Validation checkpoint 4

- [ ] `just test` and `just lint` in `darkmatter/` green; new corpus test fails if any documented limitation changes outcome (verify by temporarily editing one pattern locally).

## Phase 5: Existing-Test Triage and Documentation Drift

Wave 1 (parallel):

- [ ] **Baseline corpus tests** — update `lib/tests/l1/base_schema_end_to_end.rs`, `meta_schema_phase1/5`, `binding_contract`, `effective_schema_ownership` bench, and DMLS tests (`dmls/src/providers/frontmatter.rs` ~4003) that parse/convert the authored baseline without import expansion; keep one passive shipped-artifact test plus normal-entry-point tests.
- [ ] **Docs** — update `darkmatter/docs` schema definition, baseline contract page (`docs/topics/schemas/`), CLI docs for `md schema validate` (flag, env, precedence, opt-out, compatibility note), DMLS schema-support page. Pages describe behavior in their own words, never reference this feature directory; use a Mermaid diagram for the baseline selection order; add a compact example per rule.
- [ ] **READMEs and catalogs** — Darkmatter and Content Policy READMEs, `darkmatter/docs/dependencies.md`, the repository dependency catalog.
- [ ] **Skills** — `.claude/skills/darkmatter/` (schema.md, cli.md) and `.claude/skills/content-policy/`; remove base-schema **planned** wording from Content Policy's README and policy-lifecycle page, keeping remaining parser/schema limitations (including R1 outcome).
- [ ] **Comment drift** — fix `///` on `darkmatter_base_schema`/`darkmatter_base_json_schema` ("Loads ... via include_str!", "Panics ..."), `catalog.rs` module docs, and the `rule` note in `content-policy/schemas/content-policy.yaml` only if R1 changes it.

### Validation checkpoint 5

- [ ] `just test` and `just lint` in `darkmatter/` and `content-policy/` green. GitNexus `detect_changes`/Sniff downstream check for public-type impact (Claudine consumes Darkmatter); note result in the implementation log. No `cargo fmt`; no commits unless the author requests.

## Phase 6: Optional — Scoped Required for Union References (only if R1 is confirmed)

Skip entirely if the author keeps R1 deferred. Do not start without explicit confirmation.

- [ ] **Resolver/lowering** — honor `(required)` on a property whose type is a union reference (`long_form.rule`), without changing inheritance of required markers inside reusable types; reject both missing and null `rule`.
- [ ] **Shipped schema** — in `content-policy/schemas/content-policy.yaml` add `(required)` to `rule`, update its header comment, and its tests (Content Policy owns the pattern; ships through `EDITOR_SCHEMA`).
- [ ] **Tests** — through named imports, list items, validation, and DMLS; update the corpus row for missing `rule` to both-reject; verify reused-type requiredness behavior is unchanged.
- [ ] **Docs** — update schema docs and the Content Policy limitation lists.

### Validation checkpoint 6

- [ ] `just test`/`just lint` in both areas green; implementation log records the scope addition.

## Closing Checklist

- [ ] All AC 1–7 mapped to a passing test (table in the implementation log).
- [ ] Implementation log records API departures (R4), env-access decision (R6), and R1 outcome.
- [ ] State: implementation complete, ready for review. The author moves the spec to `_completed`.
