---
kind: implementation-log
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/darkmatter/features/2026-10-04-content-policy-base-schema/spec.md
plan: darkmatter/features/2026-10-04-content-policy-base-schema/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: 1
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
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages: []
---

# Implementation Log for 2026-10-04-content-policy-base-schema (6 phases)

## Phase 1

**Outcome: the dependency gate FAILED. Phases 2–6 are stopped and must not be
started until `2026-09-28-recursive-schema-types` is implemented.** Phase 1's
survey and rulings below are complete, so the next implementation run can begin
at Phase 2 as soon as the prerequisite lands. `human_review: true` is set on
the spec for the sequencing decision this forces.

### Dependency gate result (STOPPED)

`2026-09-28-recursive-schema-types` is **not implemented**:

- its spec status is `draft-spec` and its directory contains only `spec.md`
  (no plan, no implementation log);
- `TypeExpr` (`lib/src/markdown/schemas/simplified/types.rs:183-201`) has no
  `Ref` variant — only `Primitive`, `InlineObject`, `Imported`;
- lowering (`simplified/convert.rs`) emits no `$defs` and no `$ref`;
- `apply_import_postfix` (`lib/src/markdown/schemas/resolve.rs:1308`, rejection
  at `:1326-1330`) still rejects `[]`/constraints over a union-typed named
  type, the exact rejection the prerequisite's item 3 lifts;
- the shipped content-policy editor schema documents the consequence in its
  header (`content-policy/schemas/content-policy.yaml:16-17`), and
  `lib/tests/l1/content_policy_editor_schema.rs` works around it by typing each
  policy entry as its own `policy@./content-policy.yaml` property.

First-hand probe evidence (per the plan's Wave 1 task, a throwaway L1 test was
added, run, and removed; it is reproduced by re-adding
`lib/tests/l1/recursive_types_gate_probe.rs` with three tests). The probe wrote
the real `content-policy/schemas/content-policy.yaml` and a baseline

```yaml
$schema:
  content_policy: "policy[](min(1))@./content-policy.yaml"
```

into a temp dir and loaded it through `DarkmatterSchemas::with_baseline_from_file`.
All three probes failed at baseline load with the identical typed error:

```text
Convert {
    property: "policy",
    message: "cannot apply `[]`/constraints to the union-typed named type \
              `policy@./content-policy.yaml`",
}
```

(wrapped in `Baseline { message: "could not load baseline from .../baseline.yaml",
source: ReferencedSchema { ... } }`). This is precisely the failure mode the
plan predicted for adding the `content_policy` reference before the
prerequisite lands. The probe was removed so `just test` stays green; keeping a
known-failing test in the tree would break the area's suite.

### Baseline accessor / caller map

Accessors (`lib/src/markdown/schemas/mod.rs`): `darkmatter_base_schema()`
(`:185`, typed `SimplifiedSchema`), `darkmatter_base_json_schema()` (`:210`,
owned `Value`), `darkmatter_base_json_schema_ref()` (`:224`, `&'static Value`),
both private: `darkmatter_base_schema_ref()` (`:148`) and
`darkmatter_base_json_schema_arc()` (`:158`). Initialisation parses
`include_str!("../../../../docs/schemas/darkmatter.yaml")` with
`parse_yaml_schema` **without import expansion**, then `to_json_schema`, both
behind `OnceLock` (`BASE_SCHEMA` / `BASE_JSON_SCHEMA`, `:146-147`), both
`expect(..)`-panicking on failure. Re-exported from `lib/src/lib.rs:93`.

| Caller | Form used | Needs resolved? | Panics today? |
| --- | --- | --- | --- |
| `dmls/src/overlay/schema.rs:791` (`effective_shape`) | typed | **yes** — unions/definitions must be visible for completion/hover (R5) | inherits accessor panic |
| `dmls/src/overlay/schema.rs:1035` (`load_extension_schema` merge floor) | JSON ref | yes — `merge_baseline(lower, extension)` floor | inherits |
| `dmls/src/providers/frontmatter.rs:1527` (`known_shape`) | typed | **yes** — same shape-overlay pattern | inherits |
| `dmls/src/providers/frontmatter.rs:4003` (test) | typed | test only | n/a |
| `lib/src/markdown/compose/context/catalog.rs:255` (`project_descriptors`) | typed | yes — reads the `ctx` block of the baseline; any import-bearing property must be resolvable | `panic!` if not single shape / missing `ctx` |
| `lib/src/markdown/compose/context/options.rs:1648` (`with_darkmatter_baseline_schema`) | typed | yes — the compose fast path (`baseline_is_darkmatter_default` reuses the cached JSON form at `:2436`) | inherits |
| `lib/src/markdown/schemas/frontmatter_shape.rs:47` | typed ref (crate-private) | yes — discriminant narrowing over baseline properties | inherits |
| `lib/benches/effective_schema_ownership.rs:138,155,209` | JSON | bench; merge/clone cost only | inherits |
| lib tests `base_schema_end_to_end.rs` (typed + JSON, incl. `:325` disk/file equivalence), `meta_schema_phase1.rs:255`, `meta_schema_phase5.rs:59` | both | corpus tests that parse/convert the authored baseline **without import expansion** — Phase 5 triage list | n/a |
| `cli/src/commands/compose.rs:725` (`with_darkmatter_baseline_schema`) | via options | yes | via options |
| `cli/src/commands/schema/validate.rs:138-141` (`load_api`) | none today — `DarkmatterSchemas::new` stays **unconfigured** when no `--schema`/`BASELINE_SCHEMA` | Phase 3 changes this to the embedded baseline | n/a |
| `cli/src/commands/schema/assignment.rs:430`, `cli/src/commands/schema/triggers.rs:32` | `DarkmatterSchemas::new` (deliberately unconfigured) | no baseline attached by design | n/a |
| `cli/src/commands/clean/*` (`clean.rs:153-156`, `frontmatter_repair.rs:673-676`) | `with_darkmatter_baseline_json_schema` / file | yes | via builder |
| Downstream Claudine (`claudine/lib/src/composition/schema/mod.rs:473`, `supplied.rs:398`, `claudine/cli/src/completion/schema_completion/mod.rs:56`) | `DarkmatterSchemas::new` + `effective_for` | consumes typed `EffectiveSchema` — public-signature stability matters (R4) | n/a |

Note: the plan listed `binding_contract` among known callers; it has **no**
baseline-schema coupling — its `BindingView::baseline()` is the expression
host-binding view, unrelated to `darkmatter_base_schema`. One fewer site to
migrate.

### Import resolver seam map (for R2/R3)

Resolution path for a schema document (`lib/src/markdown/schemas/resolve.rs`):

1. `resolve_yaml_schema[_with_roots]` (`:133`/`:143`) — entry point; a string
   value is a file reference, resolved via `biscuit_file::FileReference`
   through `resolve_reference_in_context` (`:421`); bytes are read from disk.
2. `parse_yaml_referenced_file` (`:564`) — UTF-8 decode, then
   `parse_standalone_schema_document` recognizes a pure root-`$schema`
   document or a `kind: schema` document.
3. `resolve_standalone_schema` (`:714`) — assigns the document's identity
   `NamespaceKey::File(canonical_path(path))` and calls
   `expand_document_imports`.
4. `expand_document_imports` (`:932`) — builds an `ImportEngine` whose current
   namespace holds the document's own named types (`@this`).
5. `ImportEngine::expand_import` (`:1078`) — per `Name@ref`:
   `resolve_namespace` (`:1141`) resolves the right-hand `@` reference via
   `FileReference` + `resolve_file_reference_in_context` to a host path,
   canonicalizes it (`canonical_path`, `:1246`), inserts it into
   `dependencies` (watched files / `referenced_files`), memoizes the parsed
   namespace in `namespace_cache: HashMap<PathBuf, CachedNamespace>`, then the
   cycle stack `(NamespaceKey, String)` guards recursion and
   `apply_import_postfix` (`:1308`) applies `[]`/constraints — rejecting a
   union-typed base (`:1326`).

Injection conclusions:

- **Baseline entry seam** — `darkmatter_base_schema_ref` (`mod.rs:148`) never
  runs import expansion at all; Phase 2's "resolve + lower once" routes the
  embedded baseline text through the same resolved pipeline as disk documents,
  with an embedded-origin first namespace.
- **Import target seam** — `resolve_namespace` is the single point where an
  `@file` reference becomes a disk read; the embedded table lookup slots in
  front of `resolve_file_reference_in_context`, keyed on the *virtual path*,
  and only when the *importing* namespace is embedded-origin, so user
  `FileReference`s never consult it (R2).
- **Identity** — `NamespaceKey::File(PathBuf)` keyed by canonical host path is
  the definition identity; a virtual namespace needs a key that cannot equal
  any host path (R3). `dependencies` insertion (`:1196`) must be skipped for
  embedded namespaces so virtual names never enter watchers/`referenced_files`.

### Content Policy constraints (confirmed)

- `content-policy/lib/Cargo.toml`: `default = []`, single opt-in feature
  `file-adapter = ["biscuit-file/file-reference"]` — adding with
  `default-features = false` pulls only `biscuit-file/yaml`, `biscuit-hash`,
  `chrono`, `serde`, `serde_json`.
- `cargo tree -p content-policy -e normal --all-features`: **zero** darkmatter
  crates; `cargo tree -p content-policy -i darkmatter` errors with "did not
  match any packages". No reverse dependency exists, including all features.
- Guard rail: `content-policy/justfile` `deps-check` (run by its lint) fails if
  a PDF crate or `darkmatter` ever enters Content Policy's normal+build graph,
  for default and `--all-features` — the new darkmatter→content-policy
  direction keeps it green; CI asserts it (`[package.metadata.ci.tests]
  all-features = true`).
- Catalogs to update when the dependency lands (Phase 2/5): root
  `docs/dependencies.md` (`:339`, `:418`) and
  `darkmatter/docs/dependencies.md` (currently has **no** content-policy
  entry — add one).
- `content_policy::EDITOR_SCHEMA` is `include_str!` of
  `content-policy/schemas/content-policy.yaml` (`lib/src/lib.rs:67`) and has an
  equivalence test (`lib/tests/editor_schema.rs`) — the embedded table's second
  entry should reference the constant, never a production `include_str!` of the
  path.

### Rulings (R1–R8)

- **R1 — Required `rule`: DEFER** (option 3). The shipped schema documents the
  limitation (`content-policy.yaml:16-17`) and
  `content_policy_editor_schema.rs::long_form_without_rule_is_not_flagged_by_the_schema`
  already pins parser-rejects/schema-accepts. Phases 1–5 stay independent of
  the decision; Phase 6 runs only on explicit author confirmation. Not an
  acceptance criterion until then.
- **R2 — Embedded table plugs in at the import-resolution seam**, as a
  pre-disk lookup in `ImportEngine::resolve_namespace`, keyed on the virtual
  path, consulted only when the importing namespace is embedded-origin; the
  baseline's own text enters through the resolved pipeline replacing
  `darkmatter_base_schema_ref`'s bare `parse_yaml_schema`. User
  `FileReference`s never consult the table. No disk/network fallback for
  embedded-origin misses (typed errors, reusing the prerequisite's cycle and
  definition rules).
- **R3 — Virtual definition identity** is the virtual path string in a
  namespace that cannot equal any resolved host path (an `embedded:`-namespaced
  internal identity — not a public file-reference scheme). Isolation test
  required: a document- or trigger-supplied schema using the same type names
  must not collide. Implementation note: `NamespaceKey` is a private enum
  (`Root | File(PathBuf)`); a third variant (or prefixed path) both work —
  pick whichever the prerequisite's definitions table makes natural, keep it
  private.
- **R4 — Accessor signatures UNCHANGED.** All stay infallible; resolution
  failure of a *shipped* embedded document remains a panic with a clear
  message (library bug), guarded by a passive corpus test so it fails in CI,
  not at runtime. Departures: none recorded. DMLS, compose options, catalog,
  and Claudine all consume today's signatures.
- **R5 — The typed view from `darkmatter_base_schema()` carries referenced
  definitions inline** (including unions) so DMLS needs no second resolution
  pass. Confirmed against `dmls/src/overlay/schema.rs:791` (`effective_shape`)
  and `dmls/src/providers/frontmatter.rs:1527` (`known_shape`): both start
  from the typed baseline and overlay extension/document shapes by property
  name — union/definition visibility must survive baseline merging
  (`merge_baseline`, `resolve.rs:1697`) and the DMLS bundle. Blocked on the
  prerequisite: without it a union cannot be carried as a list item at all.
- **R6 — Env access: validate reads `request.snapshot().env()`**; one shared
  parse of the disable values (`1|true|TRUE|yes|YES`) is extracted and used by
  both validate and compose. Today `compose.rs:729`
  `env_disables_baseline_schema()` reads `std::env::var` directly (also used
  by `clean/frontmatter_repair.rs:673`) and is allowlisted in
  `cli/tests/l1/context_construction_guard.rs` (`commands/compose.rs`,
  `std::env::var`, count 1). If the shared helper lets compose read the
  snapshot too, that allowlist entry must be removed in the same change;
  otherwise the follow-up is logged here and the entry stays. `validate.rs:132`
  already demonstrates the snapshot pattern for `BASELINE_SCHEMA`.
- **R7 — Duplicate frontmatter key: assert whatever the parser does today.**
  No parser change. The robustness-matrix test (Phase 4) documents the
  observed outcome as the expected outcome.
- **R8 — No wider measurement.** Cache-reuse is proven by a second-load test
  (`std::ptr::eq` on `darkmatter_base_json_schema_ref()` results, pattern of
  `schemas/tests/mod.rs::darkmatter_base_json_schema_is_cached`). Installed-
  binary timing / multi-OS measurement would be an author decision, not
  scheduled.

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no behavior; its one probe test was throwaway by design and
was removed after producing the gate evidence above (a failing test cannot
ship). No new permanent tests, no docs, no skills, no package sources changed
(this log and the plan/spec frontmatter are the only edits).

### Verification

- `just lint` in `darkmatter/`: **green** (clippy + wasm32-wasip2 zed-dmls
  check).
- `just test` in `darkmatter/`: 7898 passed, **2 pre-existing failures**, both
  in `darkmatter-cli::l1 hash_kind_save_diff`
  (`test_hash_save_keeps_the_comment_after_a_quote_inside_a_plain_date`,
  `test_hash_save_reads_a_content_colon_as_part_of_a_plain_key`). Both are
  date-boundary flakes: the CLI stamps `last_updated` with the host clock's
  `2026-10-06` while the fixtures expect `2026-10-05` (the session's calendar
  date), i.e. the host has crossed midnight relative to the tests' date
  expectation. Zero darkmatter source files were modified in this phase
  (`git status` confirms), so these are unrelated to Phase 1; re-run after the
  local date matches or fix the fixtures separately.

### Unfinished / blocked

- Phases 2–6: **blocked** by the dependency gate. Do not start them until
  `2026-09-28-recursive-schema-types` is implemented (status `implemented` or
  in `_completed`), then re-run its Phase 1 verification idea: the removed
  probe (`name[](min(1))` loads, validates, survives `merge_baseline`, and is
  visible in the typed view) is the acceptance shape.

## Phase 2

**Outcome: NOT STARTED — the dependency gate still fails.** The phase-2
implementation run was requested on 2026-10-05. Per the plan's Dependency Gate
("This feature cannot start implementation until that prerequisite lands") and
Phase 1's recorded stop, no Phase 2 task was started, no plan checkbox was
checked, and no source file was touched. This section records the
re-verification that justified the stop.

### Gate re-verification (2026-10-05)

- `2026-09-28-recursive-schema-types` spec status is still `draft-spec`; its
  directory contains only `spec.md` (no plan, no implementation log; last
  modified Sep 28).
- `TypeExpr` (`lib/src/markdown/schemas/simplified/types.rs:183-201`) still
  has only `Primitive`, `InlineObject`, `Imported` — no `Ref` variant.
- `apply_import_postfix` (`lib/src/markdown/schemas/resolve.rs:1322-1331`)
  still returns the `SchemaError::Convert` "cannot apply `[]`/constraints to
  the union-typed named type `{name}@{reference}`" — the exact error the
  Phase 1 probe observed for `policy[](min(1))@./content-policy.yaml`.
- No `$defs`/`$ref` lowering exists: a search over `lib/src/markdown/schemas`
  finds `$defs` only in a reject-list at `resolve.rs:633`.
- `git log -- darkmatter/lib/src/markdown/schemas/` shows no recursive-types
  work since Phase 1 (latest commit touching the tree:
  `b9ce7c893 perf(darkmatter): compile the baseline validator once per
  context`).

### Why no partial implementation was done

Wave 1 (dependency add, embedded table, virtual path normalizer) is
technically independent of the prerequisite, but starting it was rejected:

1. The plan's gate is feature-level, not wave-level, and both Phase 1's
   recorded outcome and the spec's `message_to_agent` say do not start
   Phases 2–6 until the prerequisite lands.
2. Phase 2 Wave 2 is specified to "reuse the prerequisite's cycle/definition
   rules" — those rules do not exist, so the resolver hook cannot be written
   as specified; a Wave-1-only landing would be dead code (an unused embedded
   table and an unused `content-policy` dependency) behind `allow(dead_code)`.
3. Wave 3 (the `content_policy` baseline entry) is proven fatal by the Phase 1
   probe: it panics at baseline load and would break `darkmatter_base_schema()`
   for DMLS, compose, and Claudine, turning the area's suite red.
4. Checking off Wave 1 tasks while Waves 2–3 are blocked would misrepresent
   plan state and invite Phase 3 (which requires "the embedded baseline must
   be loadable") to run against a half-landed Phase 2.

### Requirement-to-test mapping (Phase 2)

None — no behavior changed; no test was added, removed, or run (the working
tree has zero darkmatter source changes this phase, so Phase 1's green
`just lint` / `just test` evidence stands).

### Unfinished / blocked

All Phase 2 tasks remain unchecked and untouched: dependency add, embedded
table, virtual path normalizer, resolver hook, resolve + lower once, origin
and positions, runtime baseline update, and validation checkpoint 2. The
blocker is unchanged from Phase 1: implement
`2026-09-28-recursive-schema-types` (or choose another option in the spec's
`human_review_items`), then re-run Phase 2. Phase 1's survey (accessor/caller
table, resolver seam map, rulings R1–R8) remains valid for that run.

## Phase 3

**Outcome: NOT STARTED — the dependency gate still fails, and Phase 2 (this
phase's direct prerequisite) was never started.** The phase-3 implementation
run was requested on 2026-10-05. Per the plan's Phase 3 heading ("Depends on
Phase 2 (the embedded baseline must be loadable)"), the Dependency Gate
(feature-level, not phase-level), and the spec's standing `message_to_agent`
instruction ("Do NOT start Phases 2-6 until the prerequisite lands"), no
Phase 3 task was started, no plan checkbox was checked, and no source file
was touched. This section records the re-verification that justified the
stop.

### Gate re-verification (2026-10-05)

- `2026-09-28-recursive-schema-types` spec status is still `draft-spec`; its
  directory still contains only `spec.md` (no plan, no implementation log).
- `TypeExpr` (`lib/src/markdown/schemas/simplified/types.rs:184-201`) still
  has only `Primitive`, `InlineObject`, `Imported` — no `Ref` variant.
- `apply_import_postfix` (`lib/src/markdown/schemas/resolve.rs:1328`) still
  returns the `SchemaError::Convert` "cannot apply `[]`/constraints to the
  union-typed named type" rejection that the Phase 1 probe hit for the exact
  `policy[](min(1))@…` shape this feature needs in its baseline.
- Phase 2 is verifiably absent from the tree: `darkmatter/lib/Cargo.toml`
  has no `content-policy` dependency, `darkmatter/docs/schemas/darkmatter.yaml`
  has no `content_policy` property, and `git status` shows zero modified
  files under `darkmatter/` or `content-policy/` beyond this feature's own
  markdown.

### Why no partial implementation was done

Phase 3's substance is the five-step baseline selection order in `load_api`
(`cli/src/commands/schema/validate.rs`), whose terminal step is "embedded
baseline via `DarkmatterSchemas::new(ctx).with_baseline(darkmatter_base_schema())`".
That step, and validation checkpoint 3's triage ("`last_updated: not-a-date`
fails by default, passes with opt-out"), only change behavior once Phase 2
has landed the resolved embedded baseline and the `content_policy` baseline
entry. Landing Phase 3 without Phase 2 was rejected for the same reasons the
phase-2 run rejected a Wave-1-only landing:

1. The `--no-baseline-schema` flag and selection rewrite would be built
   against a baseline that does not yet validate `content_policy`; the
   checkpoint's required default-behavior change cannot be tested, so the
   phase's own acceptance evidence cannot exist.
2. Checking off Phase 3 tasks while Phase 2 is blocked would misrepresent
   plan state and invite Phase 4 (whose corpus tests validate "through the
   default baseline") to run against a nonexistent baseline.
3. The plan's gate is feature-level; the spec's `human_review_items` already
   put the sequencing decision in front of the author, and a third silent
   partial landing would not change that decision, only muddy the record.

### Requirement-to-test mapping (Phase 3)

None — no behavior changed; no test was added, removed, or run. The working
tree has zero darkmatter source changes this phase, so Phase 1's green
`just lint` / `just test` evidence stands (same reasoning as the Phase 2
run's).

### Unfinished / blocked

All Phase 3 tasks remain unchecked and untouched: the `--no-baseline-schema`
argument, the five-step selection function, the output/exit-code rules, the
`--no-trigger-schemas` separation, and validation checkpoint 3. The blocker
is unchanged from Phases 1–2: implement
`2026-09-28-recursive-schema-types` (or choose another option in the spec's
`human_review_items`), then run Phase 2, then Phase 3. Phase 1's survey
(accessor/caller table, resolver seam map, rulings R1–R8) remains valid;
ruling R6 (env access via `request.snapshot().env()` plus one shared parse
of the disable values, shared with compose's
`env_disables_baseline_schema`) is the Phase 3 design note to carry forward.

## Phase 4

**Outcome: NOT STARTED — the dependency gate still fails, and Phases 2–3
(this phase's prerequisites) were never started.** The phase-4 implementation
run was requested on 2026-10-05. Phase 4 is the test phase: every Wave 1 and
Wave 2 task asserts behavior that Phase 2 (embedded baseline with the
`content_policy` entry) and Phase 3 (CLI selection rules) are specified to
introduce, and its validation checkpoint requires `just test` green in
`darkmatter/`. Per the plan's Dependency Gate (feature-level), Phase 4's own
heading ("Use the repository Test Toolkit … no L2/L3 tier" against the
default baseline), and the spec's standing `message_to_agent` ("Do NOT start
Phases 2-6 until the prerequisite lands"), no Phase 4 task was started, no
plan checkbox was checked, and no source or test file was touched. This
section records the re-verification that justified the stop.

### Gate re-verification (2026-10-05)

- `2026-09-28-recursive-schema-types` spec status is still `draft-spec`; its
  directory still contains only `spec.md` (no plan, no implementation log).
- `TypeExpr` (`lib/src/markdown/schemas/simplified/types.rs:184-201`) still
  has only `Primitive`, `InlineObject`, `Imported` — no `Ref` variant; the
  doc comment above it still describes only those three arms.
- `apply_import_postfix` (`lib/src/markdown/schemas/resolve.rs:1325-1331`)
  still returns the `SchemaError::Convert` "cannot apply `[]`/constraints to
  the union-typed named type `{name}@{reference}`" rejection that the Phase 1
  probe observed for the exact `policy[](min(1))@…` baseline shape.
- Phase 2 is verifiably absent from the tree: `darkmatter/lib/Cargo.toml`
  has no `content-policy` dependency, `darkmatter/docs/schemas/darkmatter.yaml`
  has no `content_policy` property, and
  `darkmatter/lib/src/markdown/schemas/embedded.rs` does not exist.
- Phase 3 is verifiably absent from the tree:
  `cli/src/commands/schema/validate.rs` contains zero occurrences of
  `no-baseline-schema` / `no_baseline_schema`.
- `git log -- darkmatter/lib/src/markdown/schemas/` is unchanged since
  Phase 1 (latest commit still `b9ce7c893 perf(darkmatter): compile the
  baseline validator once per context`); no recursive-types work has landed.
- The file Phase 4 Wave 1 would extend (`lib/tests/l1/content_policy_editor_schema.rs`)
  exists and currently pins the R1 workaround state (per-rule
  `policy@./content-policy.yaml` properties), i.e. the corpus extension task
  presumes a baseline that does not exist yet.

### Why no partial implementation was done

Phase 4 has no implementation content of its own — it is tests all the way
down, and each test targets gated behavior:

1. **Declaration agreement corpus** validates declarations "through the
   default baseline" — the baseline only gains `content_policy` in Phase 2
   Wave 3, and only reaches `md schema validate`'s default path via Phase 3's
   selection rewrite. Today the default path attaches no baseline at all
   (`load_api` leaves `DarkmatterSchemas::new` unconfigured), so every
   both-reject row would fail and every item-level `/content_policy/N`
   diagnostic assertion would fail.
2. **Robustness matrix test** asserts compiled-schema outcomes per edit cell
   (reject at `/content_policy`, `/content_policy/1`, min-1, exit 3 vs 1) —
   none of those problems can be produced without the Phase 2+3 pipeline.
3. **Embedded resolution tests** exercise the embedded table, virtual path
   normalizer, resolver hook, and cache reuse — all Phase 2 deliverables
   (`embedded.rs` does not exist).
4. **CLI precedence tests** drive `--no-baseline-schema`, the flag/`--schema`
   conflict, truthy env opt-out, and exit-2 custom-baseline behavior — all
   Phase 3 deliverables (the flag does not exist).
5. **Wave 2 subprocess tests** run built `md`/`dmls` binaries outside the
   repo and assert validation, compose pending-value behavior, and DMLS
   completion of rule forms/actions — the binaries do not contain the
   behavior until Phases 2–3 land.
6. Adding these tests now would produce a large red suite, directly
   violating the phase's own completion requirement ("all tests are passing
   using `just test`") and the plan-level rule that a known-failing test
   cannot ship (the same reason the Phase 1 probe was removed). The test
   design requirement "fails for the reported behavior and succeeds only
   after the fix" presumes the fix lands in the same plan sequence — here
   the fixing phases are gated off and out of this run's scope.
7. Authoring test *files* against the current workaround state (per-property
   `policy@./content-policy.yaml`) would be discarded wholesale when Phases
   2–3 land, and checking off any Phase 4 task would misrepresent plan state
   and invite Phase 5 (existing-test triage) to run against a nonexistent
   corpus.

### Requirement-to-test mapping (Phase 4)

None — no behavior changed and no test was added, removed, or run. The
working tree has zero darkmatter source changes this phase, so Phase 1's
green `just lint` / `just test` evidence stands (same reasoning as the
Phase 2 and Phase 3 runs; the two `hash_kind_save_diff` date-boundary flakes
noted there remain unrelated pre-existing failures).

### Unfinished / blocked

All Phase 4 tasks remain unchecked and untouched: the declaration agreement
corpus, the robustness matrix test, the embedded resolution tests, the CLI
precedence tests, the installed-binary subprocess tests, the passivity
assertions, the compose/DMLS entry-point tests, and validation checkpoint 4.
The blocker is unchanged from Phases 1–3: implement
`2026-09-28-recursive-schema-types` (or choose another option in the spec's
`human_review_items`), then run Phase 2, then Phase 3, then Phase 4. The
Phase 4 brief's own guidance (Test Toolkit, `CliProcessFixture`,
`application_input`/`application_input_removed`, tier naming, declared
test targets under `autotests = false`, and `include_str!`/manifest-dir
repository-read spelling) remains the authoring checklist for that run.

## Phase 5

**Outcome: NOT STARTED — the dependency gate still fails, and Phases 2–4
(the behavior Phase 5 triages tests and updates docs for) were never
started.** The phase-5 implementation run was requested on 2026-10-05.
Phase 5 is "Existing-Test Triage and Documentation Drift": every Wave 1
task updates tests, docs, READMEs, catalogs, skills, or code comments to
match behavior that Phases 2 (embedded baseline with the `content_policy`
entry) and 3 (CLI selection rules) are specified to introduce — and the
tree verifiably contains none of it. Per the plan's Dependency Gate
(feature-level, not phase-level) and the spec's standing `message_to_agent`
instruction ("Do NOT start Phases 2-6 until the prerequisite lands"), no
Phase 5 task was started, no plan checkbox was checked, and no source, test,
doc, README, catalog, or skill file was touched. This section records the
re-verification that justified the stop.

### Gate re-verification (2026-10-05, fifth check)

- `2026-09-28-recursive-schema-types` spec status is still `draft-spec`; its
  directory still contains only `spec.md` (no plan, no implementation log).
- `TypeExpr` (`lib/src/markdown/schemas/simplified/types.rs:183-201`) still
  has only `Primitive`, `InlineObject`, `Imported` — no `Ref` variant.
- `apply_import_postfix` (`lib/src/markdown/schemas/resolve.rs:1322-1331`)
  still returns the `SchemaError::Convert` "cannot apply `[]`/constraints to
  the union-typed named type `{name}@{reference}`" rejection that the Phase 1
  probe observed for the exact `policy[](min(1))@…` baseline shape.
- Phase 2 is verifiably absent: `lib/src/markdown/schemas/embedded.rs` does
  not exist, `darkmatter/lib/Cargo.toml` has no `content-policy` dependency,
  and `darkmatter/docs/schemas/darkmatter.yaml` has no `content_policy`
  property.
- Phase 3 is verifiably absent: `cli/src/commands/schema/validate.rs`
  contains zero occurrences of `no-baseline-schema`/`no_baseline_schema`.
- Phase 4 is verifiably absent: `lib/tests/l1/content_policy_editor_schema.rs`
  still pins the R1 workaround state (each policy entry typed as its own
  `policy@./content-policy.yaml` property, header comment at line 7), and no
  robustness-matrix, embedded-resolution, CLI-precedence, or subprocess test
  files from Phase 4's waves exist.
- `git log -- darkmatter/lib/src/markdown/schemas/` is unchanged since
  Phase 1 (latest commit still `b9ce7c893 perf(darkmatter): compile the
  baseline validator once per context`); `git status` shows zero modified
  files beyond this feature's own plan/spec/log markdown.

### Why no partial implementation was done

Each Phase 5 Wave 1 task, assessed against the current tree:

1. **Baseline corpus tests** — the task updates
   `base_schema_end_to_end.rs`, `meta_schema_phase1/5`,
   `effective_schema_ownership`, and the DMLS frontmatter test that
   "parse/convert the authored baseline without import expansion". That
   triage is only meaningful once Phase 2's "resolve + lower once" changes
   how the baseline loads. Today those tests correctly describe the current
   baseline-loading behavior (bare `parse_yaml_schema`, no import
   expansion, per the Phase 1 accessor table); "updating" them would either
   be a no-op or would rewrite them against a pipeline that does not exist,
   breaking the green suite the checkpoint requires.
2. **Docs** — the task writes the baseline contract page, CLI docs for the
   `--no-baseline-schema` flag, env precedence, opt-out, the Mermaid
   selection-order diagram, and per-rule examples. Every one of those
   describes Phase 2+3 behavior that does not exist. Per the repo's drift
   rules, `docs/` is the current record of how a package behaves — a page
   describing non-existent behavior is a defect of the change that writes
   it. (Marking pages **planned** is the mechanism for decided-but-unbuilt
   behavior, but Phases 2–3 never ran, so even the design details those
   pages would mark planned are unsettled; and writing them now would
   preempt the phase that owns those decisions.)
3. **READMEs and catalogs** — the dependency-catalog entries document a
   `darkmatter → content-policy` dependency that has not been added, and
   README sections would describe the embedded baseline and validate flag
   that do not exist.
4. **Skills** — `.claude/skills/darkmatter/` (schema.md, cli.md) and
   `.claude/skills/content-policy/` updates, plus removing Content Policy's
   "planned" base-schema wording, all describe Phase 2+3 behavior; removing
   "planned" wording now would be actively wrong (the base-schema support
   is still not built).
5. **Comment drift** — the `///` docs on
   `darkmatter_base_schema`/`darkmatter_base_json_schema` ("Loads … via
   include_str!", "Panics …") still describe exactly what the code does
   today; there is no drift because the code never changed. Same for
   `catalog.rs` module docs and the `rule` note in
   `content-policy/schemas/content-policy.yaml` (R1 stays deferred).

Validation checkpoint 5 (`just test`/`just lint` green in `darkmatter/` and
`content-policy/` plus GitNexus/Sniff downstream checks) presumes the
implementation landed; running the suites now would only re-measure the
Phase 1 state.

### Requirement-to-test mapping (Phase 5)

None — no behavior changed and no test was added, removed, or run. The
working tree has zero darkmatter or content-policy source changes this
phase, so Phase 1's green `just lint` / `just test` evidence stands (same
reasoning as the Phase 2, 3, and 4 runs; the two `hash_kind_save_diff`
date-boundary flakes noted there remain unrelated pre-existing failures).

### Unfinished / blocked

All Phase 5 tasks remain unchecked and untouched: baseline corpus test
triage, docs, READMEs and catalogs, skills, comment drift, and validation
checkpoint 5. The blocker is unchanged from Phases 1–4: implement
`2026-09-28-recursive-schema-types` (or choose another option in the spec's
`human_review_items`), then run Phase 2, then Phase 3, then Phase 4, then
Phase 5. Phase 5's own brief (which tests to keep as the passive
shipped-artifact test, which doc pages to touch, the
never-reference-this-feature-directory rule for `docs/`, and the Mermaid
selection-order diagram) remains the checklist for that run.
