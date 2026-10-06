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
