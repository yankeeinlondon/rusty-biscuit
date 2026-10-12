---
kind: implementation-log
spec: claudine/features/2026-10-17-content-policy-base-schema/spec.md
plan: null
started_phase: null
completed_phase: null
implemented: false
packages:
    - darkmatter
    - content-policy
    - dmls
source_files_during_phase_1:
    - Cargo.lock
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/content_policy_disk_baseline.rs
    - darkmatter/lib/tests/content_policy_fixtures/mod.rs
docs_updated_during_phase_1:
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
human_review: false
source_files_during_phase_2:
    - content-policy/schemas/content-policy.yaml
    - content-policy/lib/tests/editor_schema.rs
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/tests/content_policy_fixtures/mod.rs
    - darkmatter/lib/tests/union_fixtures/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - darkmatter/lib/tests/l1/content_policy_embedded_baseline.rs
    - darkmatter/lib/tests/l1/schemas_union_fixture_controls.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_2:
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - content-policy/docs/topics/policy-lifecycle.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/content-policy/SKILL.md
message_to_agent: |-
    Phase 2 is done: the runtime baseline now declares `content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"`, and every library caller that attaches the Darkmatter baseline (compose's default, DMLS, `with_darkmatter_baseline_json_schema`) prepares it from embedded documents. Read "## Phase 2" in this log.

    Facts for Phase 3:

    1. Effective behavior already changed outside the CLI.
       - Default `md compose` and DMLS now validate `content_policy` against the real list type, with no `$schema`.
       - Only `md schema validate` still uses no baseline by default (`cli/src/commands/schema/validate.rs::load_api`); that is **Select CLI Baseline**.
       - The full darkmatter and claudine suites pass, so no existing fixture held an invalid policy.
    2. The shipped Content Policy schema is in its final form: `rule: "short_form(required)"` (quoted) and `policy: [short_form, long_form]` as bare names.
       - `union_fixtures::content_policy_union_schema(shipped)` now returns the shipped text unchanged.
       - `content_policy_bare_schema` is the shipped text with `rule` made optional.
       - `content_policy_fixtures::baseline_with_policy` is the identity, after an exactly-once check of the declaration line.
    3. `content_policy_editor_schema::long_form_without_rule_is_not_flagged_by_the_schema` was already flipped to `long_form_without_rule_is_rejected_by_the_schema_and_the_parser`.
       - **Verify Compose Contract** still owns replacing that file's per-entry `p{index}` fixtures with real `content_policy` lists and retiring the mixed-`Evergreen` positive fixture (`mixed_compact_and_long_form_entries_validate`).
    4. Reuse what already exists.
       - Disk/embedded equivalence, the 109 mutation cells through both sources, all 52 `DECLARATIONS` with positions through the embedded baseline, typed definitions, cache reuse, empty `dependencies()`, and trigger/document same-name isolation are all proven. See the Phase 2 requirement-to-test table.
       - CLI and DMLS tests can `#[path]`-include `darkmatter/lib/tests/content_policy_fixtures/mod.rs` and run `DECLARATIONS` through their own entry points. Add it to that package's `[package.metadata.ci.tests] source-inputs`.
    5. Public API: a new additive variant, `SchemaSourceId::Embedded(String)`.
       - It appears only in errors from preparing embedded documents. Those cannot happen at runtime: the shipped baseline panics instead, and a passive test guards it.
       - DMLS's `if let SchemaSourceId::File` sites correctly ignore it.
    6. DMLS fixture trap. A test that writes the shipped baseline into a temp workspace must also write `content-policy/schemas/content-policy.yaml` three directories above it. See `lsp_session::write_shipped_baseline_layout`. Do not put the baseline under a directory named `schemas`: DMLS then reads a malformed buffer there as a trigger-schema candidate.
    7. Windows trap, hit this phase. Never pass a raw `std::fs::canonicalize` result as a `FileReference`. The `\\?\` spelling is rejected as `InvalidSyntax`; use `biscuit_file::canonicalize_simplified`.
    8. Docs still stale, for Phase 4:
       - the "planned" base-schema wording in `content-policy/README.md` (around `:103`) and `content-policy/docs/topics/policy-lifecycle.md` (`:7`, around `:961`);
       - the "today a schema cannot type a whole list" paragraph (around `:937`);
       - the Darkmatter schema docs and skill.

       The lifecycle page's missing-`rule` sentence and the Content Policy skill's "Editor schema" section were corrected this phase.
    9. Still true from Phase 1: `CliProcessFixture` does not scrub `SCHEMAS_DIR`; `dmls/tests/l1/stdio_subprocess.rs` inherits the host environment; `just cross-check` takes nextest name filters only, and several may be listed.
---

# Implementation Log for 2026-10-17-content-policy-base-schema

Nothing has been implemented. The Phase 1 survey below was produced by an earlier run
against a plan that no longer exists; its code map and rulings remain useful
input for the next plan.

## Restructure (2026-10-10)

- The 2026-10-10 restructure replaced this spec's prerequisite
  (`2026-10-29-recursive-schema-types`) with
  `2026-10-14-union-type-constraints`, and added
  `2026-10-15-diagnostic-node-positions` for item-level diagnostic positions.
- Required `rule` in long-form entries is now normal scope (ruling R1 below
  is updated accordingly).
- `plan.md` was removed with `git rm`: its dependency gate named the wrong
  prerequisite and its Phase 6 was an optional required-`rule` phase that no
  longer exists. The pipeline re-plans from the rewritten `spec.md`.

## Phase 1 (survey; earlier plan)

The earlier Phase 1 was a survey plus a dependency gate. The gate failed: a
throwaway L1 probe that loaded the real `content-policy.yaml` through a
baseline declaring

```yaml
$schema:
  content_policy: "policy[](min(1))@./content-policy.yaml"
```

via `DarkmatterSchemas::with_baseline_from_file` failed at baseline load with

```text
Convert {
    property: "policy",
    message: "cannot apply `[]`/constraints to the union-typed named type \
              `policy@./content-policy.yaml`",
}
```

raised by `apply_import_postfix` (`darkmatter/lib/src/markdown/schemas/resolve.rs`,
rejection at `:1328`). The probe was removed so the suite stayed green; it is
the acceptance shape the next plan's first phase should re-check once
`2026-10-14-union-type-constraints` lands. Re-checks on 2026-10-05 and
2026-10-10 found the same state. No source, doc, or skill file was changed by
any run.

Paths below are relative to `darkmatter/` unless they start with another
package area. Phase numbers mentioned in the survey refer to the removed plan.

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

- **R1 — Required `rule`: IN SCOPE** (updated 2026-10-10; originally ruled
  DEFER). Decided 2026-10-10: `long_form.rule` is required as normal scope,
  written with the bare local name `short_form(required)` and delivered by
  `2026-10-14-union-type-constraints`. The shipped
  `content-policy/schemas/content-policy.yaml` gains `(required)`, switches
  same-file references to bare names, and drops its header note about the
  limitation (`:16-17`). The test
  `content_policy_editor_schema.rs::long_form_without_rule_is_not_flagged_by_the_schema`,
  which pins parser-rejects/schema-accepts, becomes a both-reject assertion.
  There is no optional phase for this any more.
- **R2 — Embedded table plugs in at the import-resolution seam**, as a
  pre-disk lookup in `ImportEngine::resolve_namespace`, keyed on the virtual
  path, consulted only when the importing namespace is embedded-origin; the
  baseline's own text enters through the resolved pipeline replacing
  `darkmatter_base_schema_ref`'s bare `parse_yaml_schema`. User
  `FileReference`s never consult the table. No disk/network fallback for
  embedded-origin misses (typed errors, reusing the resolver's cycle and
  definition rules).
- **R3 — Virtual definition identity** is the virtual path string in a
  namespace that cannot equal any resolved host path (an `embedded:`-namespaced
  internal identity — not a public file-reference scheme). Isolation test
  required: a document- or trigger-supplied schema using the same type names
  must not collide. Implementation note: `NamespaceKey` is a private enum
  (`Root | File(PathBuf)`); a third variant (or prefixed path) both work —
  pick whichever the resolver's definitions table makes natural, keep it
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
  (`merge_baseline`, `resolve.rs:1697`) and the DMLS bundle. Depends on
  `2026-10-14-union-type-constraints`: without it a union cannot be carried
  as a list item at all.
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

## Phases 2–6

Phases 2–6 never ran: they were gated on a prerequisite that the 2026-10-10
restructure replaced with 2026-10-14-union-type-constraints.

## Phase 1

Run on 2026-10-12 against the current `plan.md` (5 phases). Phase 1 changes no
production behavior. It verifies the prerequisites, refreshes the integration
map, and adds the shared fixtures and a disk-baseline test that later phases
reuse.

### Prerequisites (Verify Prerequisites)

All three prerequisites hold in the tree. The earlier gate failure, `cannot
apply [] / constraints to the union-typed named type`, no longer reproduces.

- Existing focused checks pass. In the darkmatter lib L1 suite, 166 tests pass:
  `schemas_union_*`, `content_policy_editor_schema`, `diagnostic_positions`,
  `schemas_named_type*`, `schemas_effective_presence`,
  `base_schema_end_to_end`, and `shipped_schema_corpus`. In the CLI and DMLS
  suites, 37 tests pass: `schema_union_pipeline` and `union_item_editor`.
  - `union_fixtures` (from `2026-10-14-union-type-constraints`) already
    carries the acceptance shape. `content_policy_union_schema(shipped)` is
    the shipped schema with bare local names and `rule: short_form(required)`.
  - `CONTENT_POLICY_CASES` already covers the empty-list, missing-rule, and
    null-rule rejections from a document `$schema`.
- New evidence comes from `darkmatter/lib/tests/l1/content_policy_disk_baseline.rs`.
  The temp root mirrors the repository layout:
  - `darkmatter/docs/schemas/darkmatter.yaml` is the shipped baseline plus the
    exact `content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"`
    line.
  - `content-policy/schemas/content-policy.yaml` is the target artifact.
  - Everything is loaded with `DarkmatterSchemas::with_baseline_from_file`.
  - The three `..` segments resolve, and the baseline prepares, lowers, and
    validates.
  - The authoring example is valid, and a policy-free document is valid.
  - `content_policy`'s origin is `Baseline`.
- Precedence and isolation are proven by
  `triggers_and_documents_replace_the_baseline_declaration_without_name_collisions`:
  - a matching trigger whose payload imports its own `policy`/`short_form`/
    `long_form` replaces the baseline declaration (origin `Trigger`, no
    minimum);
  - the document's `$schema` replaces both (origin `Document`);
  - a document importing the same-named types for another property leaves the
    baseline's typing of `content_policy` intact.

#### Observed diagnostic shapes (pinned in the fixture)

These outcomes satisfy the spec's contract: the offending item or field is
identified, a missing field reports its containing mapping's position, and
empty or wrong-typed lists report the value. They are not the
missing-property shape, though:

| Input | Path | Kind | Position |
| --- | --- | --- | --- |
| bad compact rule, null item, wrong-typed item | `/content_policy/N` | `invalid` (no arm of the union matched) | item start (block: column after `- `) |
| long-form missing `rule` or `action`, or an extra key | `/content_policy/N` | `invalid` (no arm matched), `property: null` | the entry mapping's start; the extra key itself is not named |
| null, wrong-typed, or empty `rule`/`action` | `/content_policy/N/rule` (`/action`) | `invalid` | the field value |
| `[]` | `/content_policy` | `invalid` ("has less than 1 item") | the value |
| string, number, object, or `{}` in place of the list | `/content_policy` | `type` | the value |

The problem's `description` carries the union's per-arm descriptions.

### Integration map (Map Integration)

This refreshes the earlier survey's line numbers and adds new findings. Paths
are relative to `darkmatter/` unless they say otherwise.

- **Source seam.**
  - `ImportEngine::resolve_namespace` is at `lib/src/markdown/schemas/resolve.rs:1571`
    and inserts dependencies at `:1592`.
  - The namespace cache is at `:1596`/`:1608`, and
    `named_type_file_path` is at `:1635`.
  - **`load_named_types` (`:1773`) does the raw `fs::read`.** This is where an
    embedded source must replace disk reads.
  - `NamespaceKey { Root, File(PathBuf) }` (`:1001`) is private.
  - **`NamespaceKey::source_id()` maps to the public
    `SchemaSourceId { InlineDocument, File(PathBuf) }`**
    (`simplified/source.rs:94`, re-exported from `schemas/mod.rs`). It appears
    in `SchemaError::UnknownNamedType { namespace }`,
    `SchemaConstraintOrigin`, and `SchemaReferenceSite`.
  - An embedded identity that must surface in typed errors may therefore need
    a public variant. Per the plan, prefer an internal representation; any
    public change must be recorded here before it is made.
- **Hidden disk read.** `resolve.rs:1842` re-reads every `SchemaSourceId::File`
  path with `fs::read_to_string` to enrich constraint-conflict spans. An
  embedded identity must not reach this path as a host file.
- **Baseline accessors.**
  - `BASE_SCHEMA`/`BASE_JSON_SCHEMA` are at `schemas/mod.rs:149-150`, and
    `darkmatter_base_schema_ref` is at `:152`. It parses bare YAML with no
    import expansion.
  - The accessors are at `:190`, `:215`, and `:229`, and
    `with_darkmatter_baseline_json_schema` is at `:409`.
  - `with_baseline_from_file` (`:373`) already expands imports through
    `resolve_yaml_schema`.
  - The JSON accessor's callers are listed in the earlier survey. They are
    unchanged, apart from new line numbers:
    - `compose/schema_validation.rs:164`
    - `compose/context/options.rs:1658`, `:2446`
    - `schemas/clean.rs:155`
    - `schemas/frontmatter_shape.rs:47`
    - `compose/context/catalog.rs:255`
  - No Claudine crate calls the base accessors directly.
- **DMLS.**
  - `overlay/schema.rs` uses the typed baseline in `effective_shape` (`:790-806`)
    and the JSON baseline in `assemble` (`:950-995`, default at `:976`).
  - `combined_baseline` (`:1021-1057`) merges extensions over
    `darkmatter_base_json_schema_ref()`.
  - `providers/frontmatter.rs` uses the typed baseline in `known_shape`
    (`:2013-2041`).
  - The DMLS cache hashes only `effective.dependencies()` plus extension
    dependencies (`overlay/mod.rs:701-715`). Embedded entries are immutable, so
    this is correct as long as nothing adds virtual paths there.
- **CLI.**
  - `SchemaTarget::Validate` is in `cli/src/args/target.rs:60-85`. It has
    `--schema` at `:71`, `--quiet`, `--format`, and `--no-trigger-schemas`. It
    has no baseline flag.
  - Dispatch is at `cli/src/commands/mod.rs:296-311`.
  - `load_api` is at `cli/src/commands/schema/validate.rs:129-143`. It reads
    `--schema`, then the snapshot's `BASELINE_SCHEMA`, and otherwise uses **no
    baseline**.
  - `no_schema` is computed at `:198-201`, and the pretty "doesn't have a
    schema definition" line is at `:262-271`.
  - JSON output is `file`/`valid`/`schema`/`problems`/`warnings`.
  - Exit precedence is 3 > 2 > 1 > 0, and usage errors exit 64.
- **Compose predicate.**
  - `env_disables_baseline_schema` is at `cli/src/commands/compose.rs:729-736`
    and reads the live process environment.
  - Its callers are `compose.rs:721` and `clean/frontmatter_repair.rs:673`.
  - It is allowlisted (count 1) in `cli/tests/l1/context_construction_guard.rs:20-26`.
- **Tests that will need attention in Phase 3.**
  - `cli/tests/l1/schema_validate.rs:95-106`
    (`schema_validate_no_schema_no_baseline_is_vacuous_success`, whose name and
    intent change).
  - `schema_validate.rs:236-293` (custom baseline cases).
  - The byte-exact snapshots of `schema_validate_baseline.rs`.
  - `schema_robustness_matrix.rs:310`, `:409-411`.
  - `schema_validate_shipped_artifacts.rs:182`.
  - `schema_triggers.rs:70-80`.
  - `entry_point_parity.rs:109`, `:718`, `:752`.
  - The root `justfile:99` (`test-prompts` runs `md schema validate --quiet`;
    it is informational).
- **Lib corpus tests that bypass import expansion (Phase 2).**
  - `base_schema_end_to_end.rs:293-326`: `parse_yaml_schema` plus
    `to_json_schema` must equal `darkmatter_base_json_schema()`. **This breaks
    once the file imports.**
  - `meta_schema_phase1.rs:235-287`.
  - `meta_schema_phase5.rs:28-60`.
  - `meta_schema_phase6.rs:12-40`.
  - `shipped_schema_corpus.rs:266` (`CORE_TYPE_USERS` for `darkmatter.yaml`)
    and `:551`.
  - `schemas/tests/mod.rs:929-952` (cache identity).
- **Fixtures and environment.**
  - `CliProcessFixture` (`cli/tests/common/fixture.rs:190`) scrubs
    `DARKMATTER_*` and `BASELINE_SCHEMA`, but **not `SCHEMAS_DIR`**, which is
    only declarable through `common/protected_env.rs:100-108`.
  - DMLS `LspFixture` runs in memory with an empty snapshot environment.
  - `dmls/tests/l1/stdio_subprocess.rs:154-170` spawns `bin_exe!("dmls")` and
    **inherits the host environment**.
  - Phase 4's standalone tests must scrub both explicitly.
  - Schema loading reads `BASELINE_SCHEMA` (validate, snapshot) and
    `DARKMATTER_NO_BASELINE_SCHEMA` (compose/clean, process). It reads
    `SCHEMAS_DIR` through `roots.rs:28` (snapshot). DMLS uses config keys only.
- **Manifests.**
  - Before this phase, no darkmatter manifest named `content-policy`.
  - Content Policy's `deps-check` (`content-policy/justfile:74-91`) checks
    normal and build edges for both default and all features.
  - `content-policy/lib/tests/editor_schema.rs:7-9` asserts that
    `EDITOR_SCHEMA` exactly equals the file.

### Fixtures (Define Fixtures)

The new shared module is `darkmatter/lib/tests/content_policy_fixtures/mod.rs`.
It is std-only and declared in lib `tests/l1/main.rs`. The CLI and DMLS can
include it by `#[path]`, as `union_fixtures` is included, and must then list
it in their `[package.metadata.ci.tests] source-inputs`.

- `MIXED_POLICY_FRONTMATTER` is the spec's authoring example, including
  `last_updated`.
- `DECLARATIONS` holds 52 cases. Each has a separately stated parser verdict
  (`None` for absent, meaning no parse call is made) and a baseline verdict,
  plus every expected problem as a path, kind, and `Anchor` (value, item, or
  field).
  - `render()` writes each list case as a block list and as a flow list with
    quoted commas, and computes each anchor's line and column from the
    authored text.
  - Accept controls:
    - every rule form and every action in one list (`every_rule_form_and_action`);
    - `Evergreen` alone in compact and long form;
    - the legal `FileChanged` paths: `notes/a(b.md`, `&Cargo.toml`,
      `^README.md`, `../x.md`, and a nonexistent file.
  - Both-reject rows cover the rule grammar, long-form entries (missing or
    null fields, every wrong type, empty values, extra field), list shape, and
    whole-value types. `" ValidFor(3mo)"` is a both-reject row that proves
    there is no trimming.
  - Limitation rows, which the parser rejects and the schema accepts:
    impossible date, duration overflow, `vault:` prefix, URL, `{{` in a path,
    `Evergreen` with another rule, `content_policy: null`, and
    `content_policy:`.
- `LOAD_FAILURES` covers duplicate `content_policy`, `rule`, and `action` keys,
  and malformed YAML carrying each field. All of them fail document loading.
- `SCHEMA_MUTATIONS` holds 109 cells over the plan's columns: baseline
  `$schema`, policy `kind`, `types`, `$schema.content_policy`,
  `types.short_form`, `types.long_form`, `types.policy`, `long_form.rule`, and
  `long_form.action`.
  - Each column starts with a control row.
  - Each cell states `PrepareFails`, `Weakened` (both controls pass),
    `RejectsValid`, or `Control`, optionally with a probe that proves a changed
    decision:
    - the minimum removed accepts `[]`;
    - `(required)` removed from `rule` or `action` accepts an entry without it;
    - `long_form: {}` accepts `[{}]`.
  - `baseline_with_policy()` inserts the declaration line into the shipped
    baseline, and asserts that the shipped baseline lacks it.

All cells and cases pass against the disk resolver as written in the plan's
tables. The only departures are in the problem kinds and positions listed
above. Specifically:

- the empty `$schema` mapping weakens enforcement;
- a null type definition rejects the valid control;
- `rule`/`action` removed from the closed `long_form` rejects the control as
  undeclared;
- an empty `types` mapping fails with an unknown requested type.

`every_mutation_column_has_a_test` guards the per-column test list. A
deliberately wrong expectation fails, so the harness is load-bearing.

**Reader-boundary audit.** Duplicate keys and malformed YAML fail at
`Markdown::try_from` for frontmatter, and at preparation for both schema
files. No reader-boundary correction is needed. A grep of the schema loaders
for `serde(default)`, `unwrap_or_default`, `filter_map(.. as_str)`, and
`.ok()` found only these sites, none of which silently drops content:

- `resolve.rs:814` (advisory heuristic);
- `resolve.rs:1842` (best-effort span enrichment);
- `resolve.rs:2945-2976` (`merge_baseline` over internally lowered JSON);
- `simplified/standalone.rs:142` (followed by a `map.len() != 2` check).

### Dependency change

`darkmatter/lib/Cargo.toml` gains the **dev-dependency**
`content-policy = { path = "../../content-policy/lib", default-features = false }`.
The lockfile gains one edge. `just deps-check` in `content-policy/` still
reports no Darkmatter crate for default or all features. Phase 2's **Wire
Baseline** promotes it to a normal dependency. The notes were added to
`darkmatter/docs/dependencies.md` and the root `docs/dependencies.md`.

### Requirement-to-test mapping

| Requirement | Test |
| --- | --- |
| Target declaration loads from a disk baseline; example valid; no-policy document valid; baseline origin | `content_policy_disk_baseline::the_authoring_example_and_a_policy_free_document_validate_against_the_baseline` |
| Empty-list, missing/null-rule rejection and item diagnostics with positions (block and flow) | `content_policy_disk_baseline::declarations_meet_their_baseline_expectations` |
| Parser column; absent never parsed | `content_policy_disk_baseline::declarations_meet_their_parser_expectations` |
| Duplicate-key and invalid-content rows at the reader boundary | `content_policy_disk_baseline::damaged_frontmatter_fails_document_loading` |
| Schema robustness tables (9 columns) | `content_policy_disk_baseline::{baseline_envelope,policy_kind,policy_types,baseline_declaration,short_form,long_form,policy_definition,long_form_rule,long_form_action}_cells`, guarded by `every_mutation_column_has_a_test` |
| Baseline/trigger/document precedence; same-name isolation | `content_policy_disk_baseline::triggers_and_documents_replace_the_baseline_declaration_without_name_collisions` |

### Gates

- `just lint` in `darkmatter/` passed.
- `just test` in `darkmatter/` passed: 9640 tests, 0 failed.
- `just test` in `claudine/` passed: 8694 tests, 0 failed.
- `just lint` in `claudine/` passed. The `ld __eh_frame` linker warning is
  pre-existing.
- `just lint` in `content-policy/` passed, including `deps-check`.
- On native Windows, `just cross-check darkmatter --os windows content_policy_disk_baseline`
  passed: 15/15, each test under 1.5 s. Linux and WSL2 were not run; CI covers
  them.
- No skipped or pre-existing failures.

## Phase 2

Run on 2026-10-12.

### Update Artifact

- `content-policy/schemas/content-policy.yaml` now declares
  `long_form.rule: "short_form(required)"` and lists `policy` as the bare
  local names `short_form` and `long_form`. The header note saying `rule`
  cannot be `(required)` is gone; the other header notes (pattern escapes,
  date shape, path limits) are unchanged.
- `content-policy/lib/tests/editor_schema.rs` gains
  `the_exported_schema_requires_rule_and_names_its_own_types_bare`. The
  existing byte-equality test already proves `EDITOR_SCHEMA` is the file.
- Shared fixtures (Phase 1 handoff item 1):
  - `union_fixtures::content_policy_union_schema` now returns the shipped
    text unchanged after asserting it requires `rule`;
    `content_policy_bare_schema` is the shipped text with `rule` made optional
    (the "without `(required)`" control).
    `schemas_union_fixture_controls::content_policy_fixture_edits_only_the_required_rule_line`
    replaces the old three-line test.
  - `content_policy_fixtures::RULE_LINE` matches the shipped, quoted
    `rule: "short_form(required)"` line.
- `content_policy_editor_schema::long_form_without_rule_is_not_flagged_by_the_schema`
  (which pinned the old limitation) is now
  `long_form_without_rule_is_rejected_by_the_schema_and_the_parser`: a
  missing `rule` (problem at the entry, `/p0`) and `rule: null` (problem at
  `/p1/rule`) are rejected by the schema and by `Policy::from_declaration`.
  Phase 3's **Verify Compose Contract** still owns the rest of that file.

### Public API decision (recorded before the change)

The embedded namespace needs an identity that cannot equal any disk path.
Internally this is a new private `NamespaceKey::Embedded(String)`. That key
reaches public types through `NamespaceKey::source_id()`, which fills
`SchemaError::UnknownNamedType { namespace, site }`, `SchemaReferenceSite`,
and `SchemaConstraintOrigin`.

- Reusing `SchemaSourceId::File(PathBuf)` for a virtual path would be wrong:
  - `locate_origin_spans` (`resolve.rs`) would `fs::read_to_string` it;
  - DMLS `overlay/mod.rs::error_dependencies` would add it to the watched
    dependencies; and
  - `dmls/src/diagnostics/named_types.rs` would treat it as a host file.
- Reusing `InlineDocument` would lose the importing virtual document that
  the spec requires typed errors to identify.
- **Change:** add the public variant `SchemaSourceId::Embedded(String)` (the
  virtual path, displayed as `embedded `<path>``).
- **Affected callers:** none break. Every match on `SchemaSourceId` outside
  `simplified/source.rs`'s own `Display` is an `if let`/`let … else` on
  `File`: `dmls/src/diagnostics/named_types.rs` (`:123`, `:236`, `:259`,
  `:286`, `:315`) and `dmls/src/overlay/mod.rs:736`. All of them correctly
  skip an embedded source (no file to read or watch). No Claudine or
  Content Policy code names the type.
- The internal preparation operation stays crate-private. Its tests are lib
  unit tests that include the shared `content_policy_fixtures` tables by
  `#[path]`, so no public test seam is added.

### Separate Sources

- New private module `darkmatter/lib/src/markdown/schemas/embedded.rs`:
  - `EmbeddedDocument { path, text }`;
  - the shipped table `SHIPPED`, holding exactly
    `darkmatter/docs/schemas/darkmatter.yaml` (an `include_str!` of the
    runtime baseline) and `content-policy/schemas/content-policy.yaml`
    (`content_policy::EDITOR_SCHEMA`);
  - `resolve_path`, which resolves a reference lexically against the
    importer's virtual directory with the repository root as the virtual
    root. It rejects empty references, URLs, `/`-absolute, drive (`:`), `\`
    and UNC spellings, `~ & ^ $ % @` prefixes, globs, empty segments, and any
    climb above the root.
- `resolve.rs`:
  - the seam is a new private `NamespaceSource` enum on `ImportEngine`:
    `Files { schema_roots, context }` (the existing request-scoped
    `FileReference` path, unchanged) or `Embedded(&[EmbeddedDocument])`;
  - `NamespaceKey::Embedded(String)` is the virtual identity, and a separate
    `embedded_cache` is keyed by virtual path;
  - an embedded namespace's imports go through `resolve_embedded_namespace`.
    It adds **no** dependency edge, never touches the disk, and returns the
    existing `SchemaError::SchemaDocument { path: <importer>, message }` for
    an invalid path or an unregistered target. An unknown type is the
    existing `UnknownNamedType`, with `namespace` and `site.source` both
    `SchemaSourceId::Embedded(..)`;
  - `load_named_types` is split so the disk read feeds the new
    `named_types_from_text`, which the embedded source shares;
  - the crate-private `prepare_embedded_schema(path, documents)` parses a
    standalone document, expands it through the shared engine, and lowers it.
    It rejects a root-union or whole-file-reference document and any
    surviving `example(...)` annotation, because that would name a disk file;
  - `rehome_examples` runs only for `File` namespaces;
  - parsing, named-type expansion, constraint merging, union postfixes,
    cycle and depth checks, and lowering are all shared, and no grammar was
    added.
- Departure from the plan's wording: the plan says to resolve the source seam
  "around namespace acquisition". That is where it is. But
  `FileResolutionContext::from_snapshot` is forbidden in production code by
  `context_construction_guard`, so the embedded path holds no request context
  at all. That absence is why the seam is an enum rather than a dummy
  context. Clippy's `large_enum_variant` required boxing the `Files` context.

### Wire Baseline

- `darkmatter/lib/Cargo.toml`: `content-policy` moved from
  `[dev-dependencies]` to `[dependencies]` (`default-features = false`, no
  `file-adapter`). The lockfile edge from Phase 1 is unchanged, and
  Content Policy's `just deps-check` still passes for default and all
  features.
- `darkmatter/docs/schemas/darkmatter.yaml` gains exactly
  `content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"`
  after `last_updated`, with no default and no description (the spec gives
  the exact line).
- `schemas/mod.rs`:
  - the two `OnceLock`s are now one `BASELINE: OnceLock<ShippedBaseline>`
    holding the typed schema and its lowered `Arc<Value>`. Both come from one
    `prepare_embedded_schema` call, so the baseline is prepared and lowered
    once per process;
  - the accessor signatures and their panic contracts are unchanged, and
    their docs now describe the embedded imports;
  - `with_darkmatter_baseline_json_schema` still shares the cached `Arc`.
- Fixture handoff item 1: `content_policy_fixtures::baseline_with_policy` is
  now the identity, after asserting that the shipped baseline carries the
  declaration line exactly once.

### Prove Resolution

- **Disk/embedded equivalence (real artifacts):**
  `base_schema_end_to_end::schema_document_transcludes_same_file_as_library_source`
  now resolves the authored `darkmatter.yaml` from the checkout with the
  normal disk resolver (`resolve_yaml_schema`). It asserts that:
  - the only import edge is `content-policy/schemas/content-policy.yaml`;
  - the typed `SimplifiedSchema` equals `darkmatter_base_schema()` exactly;
  - the JSON equals `darkmatter_base_json_schema()` exactly.

  So rule alternatives, constraints, descriptions, suggestions, and required
  fields are identical. It also pins `EDITOR_SCHEMA` to the file.
- **Disk/embedded equivalence (schema mutations):** the lib unit tests
  `schemas::tests::embedded_sources::embedded_*_cells` run all 109
  `SCHEMA_MUTATIONS` cells through `prepare_embedded_schema` with a
  test-owned table. Every cell gives the same outcome as the disk resolver's
  `content_policy_disk_baseline::*_cells`; no table change was needed. The
  fixture module is included by `#[path]` from `schemas/tests/mod.rs`.
- **Embedded-source contract:** `embedded_sources` tests cover:
  - the shipped baseline preparing with no import, example, or referenced-file
    edges;
  - relative, `./`, `../`, and bare-local references inside a table;
  - eight invalid virtual paths, each naming the importer;
  - a missing document with no disk fallback (the real
    `content-policy/schemas/content-policy.yaml` exists in the checkout but
    is not registered), and an unregistered top-level document;
  - a missing type naming both virtual documents (`SchemaSourceId::Embedded`);
  - a malformed import, a cycle, an `example(...)`, a root union, and a
    whole-file reference;
  - an embedded `schemas/types.yaml` staying distinct from a disk file with
    the same relative path.

  `embedded::tests` cover the lexical resolver directly.
- **Real baseline through the public entry point:** new
  `darkmatter/lib/tests/l1/content_policy_embedded_baseline.rs` covers:
  - the authoring example and a policy-free document, both valid, with
    origin `Baseline` and **empty** `EffectiveSchema::dependencies()`;
  - all 52 `DECLARATIONS` in block and flow presentations, meeting their
    baseline verdicts, paths, kinds, and line/column positions, with
    `content_policy` typed by the baseline (item, field, and
    containing-mapping positions as pinned in Phase 1);
  - the typed list (optional, array, one closed long-form arm with a required
    `rule` union over the same alternatives and a required
    `refresh/archive/remove` enum), the shipped arm descriptions, the seven
    shipped suggestions, and lowered `minItems: 1`;
  - one-allocation cache reuse;
  - trigger and document overrides with same-named `policy`/`short_form`/
    `long_form` types and no collisions.

  `schemas::tests::darkmatter_base_schema_is_prepared_once_with_its_json`
  adds pointer identity for the typed form, and shows the cached JSON is
  that form lowered.
- **Corpus tests:** the bare `parse_yaml_schema` plus `to_json_schema` on the
  baseline in `simplified::tests::darkmatter_baseline_schema_converts_to_json_schema`
  and `…_ctx_reflects_runtime_semantics` now read the prepared baseline.
  `darkmatter_baseline_schema_yaml_parses` still parses the authored text,
  which is legal with an unexpanded import. `meta_schema_phase1/5/6` and
  `shipped_schema_corpus` read source spans or the prepared baseline and
  pass unchanged. `schemas/tests/mod.rs`'s cache-identity tests pass
  unchanged.
- **DMLS:** `lsp_session::meta_schema_phase6_shipped_schema_activation_and_current_error`
  and `…phase7_shipped_schema_provider_path` copied the shipped baseline
  alone into a flat workspace, where its import now (correctly) fails with
  `dm.schema.prepare`. They now write it three levels deep beside
  `content-policy/schemas/content-policy.yaml`. The baseline's directory is
  `darkmatter/docs/baseline/` and not `…/schemas/`: in a directory named
  `schemas`, DMLS reads the test's deliberately malformed buffer as a
  trigger-schema candidate and reports a trigger error instead of
  `dm.schema.document_malformed`. That behavior predates this change and is
  not this test's subject.

### Docs touched early

Phase 4's **Update Behavior Records** owns docs and skills. Two statements
were made false by this phase's artifact change, so they were corrected
minimally now rather than left wrong:

- `content-policy/docs/topics/policy-lifecycle.md` no longer says the schema
  ignores a missing `rule`.
- `.claude/skills/content-policy/SKILL.md` "Editor schema" no longer says
  `rule` is optional or that the base schema does not type `content_policy`.

The "planned" wording in the Content Policy README and lifecycle page is
still there, because the `md schema validate` default is Phase 3. So is the
"today a schema cannot type a whole list" paragraph (`policy-lifecycle.md`
around `:937`), which was already stale before this phase.

### Requirement-to-test mapping (Phase 2)

| Requirement | Test |
| --- | --- |
| Shipped artifact requires `rule`, uses bare names, drops the header note | `content-policy` `editor_schema::the_exported_schema_requires_rule_and_names_its_own_types_bare`, `…the_exported_schema_is_the_schema_file` |
| Missing / null `rule` rejected by schema and parser | `content_policy_editor_schema::long_form_without_rule_is_rejected_by_the_schema_and_the_parser`; `DECLARATIONS` rows through both sources |
| Lexical virtual paths, legal `..`, above-root, host, URL, and prefix forms | `embedded::tests::*`, `embedded_sources::invalid_virtual_paths_are_rejected_with_the_importer_named` |
| Missing document or type, malformed import, cycle, examples; no disk fallback | `embedded_sources::{a_missing_document_is_an_error_without_disk_fallback, a_missing_type_names_both_virtual_documents, malformed_imports_cycles_and_examples_fail_preparation}` |
| Bare local names inside an embedded document | `embedded_sources::relative_imports_and_bare_local_names_resolve_inside_the_table`, `…the_shipped_baseline_prepares_without_dependencies` |
| Distinct embedded identity | `embedded_sources::an_embedded_namespace_is_distinct_from_a_disk_file_with_the_same_path` |
| Disk/embedded equivalence (real artifacts, typed and JSON) | `base_schema_end_to_end::schema_document_transcludes_same_file_as_library_source` |
| Disk/embedded equivalence (schema robustness tables) | `embedded_sources::embedded_*_cells` beside `content_policy_disk_baseline::*_cells`, each guarded by its column-coverage test |
| Passive prepare, lower, and compile of the shipped baseline | `embedded_sources::the_shipped_baseline_prepares_without_dependencies`, `content_policy_embedded_baseline::the_authoring_example_validates_against_the_embedded_baseline` |
| Diagnostics: baseline typing, path, kind, position | `content_policy_embedded_baseline::declarations_meet_their_baseline_expectations_through_the_embedded_baseline` |
| Typed definitions, descriptions, and suggestions survive embedding | `content_policy_embedded_baseline::the_typed_baseline_carries_the_policy_definitions` |
| Same-process cache reuse | `content_policy_embedded_baseline::the_embedded_baseline_is_prepared_once_per_process`, `schemas::tests::darkmatter_base_schema_is_prepared_once_with_its_json`, existing `darkmatter_base_json_schema_is_cached` |
| No watcher pollution | `content_policy_embedded_baseline::the_authoring_example_validates_against_the_embedded_baseline` (`dependencies()` empty), `embedded_sources::the_shipped_baseline_prepares_without_dependencies` |
| Same-name isolation with triggers and documents | `content_policy_embedded_baseline::triggers_and_documents_override_the_embedded_declaration_without_name_collisions` |

### Gates

- `just test` in `darkmatter/` (lib, CLI, DMLS): 9668 passed, 0 failed.
- `just test` in `claudine/`: 8694 passed, 0 failed.
- `just test` in `content-policy/`: 175 passed.
- `just lint` passed in `darkmatter/`, `content-policy/` (including
  `deps-check`), and `claudine/`. The `ld __eh_frame` warning predates this
  change.
- `just cross-check --os windows darkmatter content_policy embedded schema_document_transcludes darkmatter_base`:
  pass, 83 tests. The first run caught a Windows-only failure in
  `base_schema_end_to_end`: the test's raw `canonicalize()` gave a `\\?\…`
  spelling, which `FileReference` rejects. It now uses
  `biscuit_file::canonicalize_simplified` for both the reference and the
  expected import path.
- `just cross-check --os windows dmls meta_schema_phase6_shipped meta_schema_phase7_shipped`: pass.
- `just cross-check --os linux darkmatter …` (same filters): pass. WSL2 was
  not run; CI's nightly covers it.
- No skipped or pre-existing failures.
