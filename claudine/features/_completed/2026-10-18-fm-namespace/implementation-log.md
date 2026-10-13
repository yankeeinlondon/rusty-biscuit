---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-18-fm-namespace/spec.md
plan: claudine/features/2026-10-18-fm-namespace/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/request_context_builder.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
human_review: false
message_to_agent: |-
  Phase 1 inventory, spike, and approval evidence are in this log under
  "## Phase 1". Points that change the plan's assumptions for Phase 2:

  1. Three Claudine production sites special-case the *frontmatter* `doc`
     root and are not named in the plan's task ownership. They must become
     `fm` as part of the rename (assign them to the Document Roots work, or
     run them alongside Hook Adapter, but do not leave them):
     - `claudine/lib/src/composition/sequence/expr.rs:113`
       (`SourceExpressionLookup::get`) and its tests at `:242-250`;
     - `claudine/lib/src/composition/looping/expression.rs:128, 222-243`
       (`resolve_doc`) — its dotted read clones the entire frontmatter
       object; switch it to the shared Darkmatter `fm` helper (borrow and
       clone only the leaf) to honor the cheap-dotted-lookup rule;
     - `claudine/lib/src/composition/reserved.rs:13`
       `EXPRESSION_RESERVED_ROOTS = ["true","false","doc","env"]`
       (consumer `looping/config.rs:249`).
  2. No separate "harness validation" caller of the hook adapter exists; the
     phrase is stale doc text in `claudine/lib/src/dispatch/expression.rs`
     (module docs and `:139`). Template rendering (`dispatch/template.rs`,
     used by bash/speak/report runners and `messaging/send.rs`) is the
     message surface; cover it and fix the comment.
  3. Many Darkmatter sites hardcode `"doc"` instead of reading the catalog
     (`collect_variable_roots`, `static_truthiness`, `blocks.rs`
     `collect_frontmatter_roots`, preflight `collect_expression_roots`,
     evaluator `presentation_path_value`, shell-expansion body
     classification). The bracket-dependency gap (`doc['a']` records no
     dependency) is shared by `frontmatter_interpolation.rs`, `blocks.rs:525`,
     and `preflight/collect.rs:1494`.
  4. The DMLS dasherized quick-fix emits `doc['key']`
     (`dmls/src/overlay/expressions.rs:1118`, `references_key` at `:1139`);
     it must emit `fm['key']`.
  5. `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:109-128, 267` names
     `darkmatter/schemas/partials/doc.yaml` by path; update it with the
     `git mv` to `partials/fm.yaml`.
  6. The approval ruling is decided (2026-10-13, option A; see "### Approval
     ruling"): exact-text approval is retained and a later task's command
     whose text changes under its own captured map is refused before
     execution. Phase 3 is unblocked. The environment-only seam is proven at the Darkmatter
     level by `request_context_builder.rs::an_environment_only_replacement_enters_a_document_under_another_map`;
     Claudine's `InvocationContext` still lacks an environment-only
     derivation, and `composition/select.rs` / `looping/config.rs` read the
     live process environment (Phase 3 concern).
---

# Implementation Log for 2026-10-18-fm-namespace (5 phases)

## Phase 1

- Started 2026-10-12 on macOS. Phase 1 is discovery: surface inventory, approval
  ruling, and the environment spike.

### Wave 1 — Surface Inventory

Baseline taken 2026-10-12 before any change. `rg -l current_env --type rust`
lists 42 files (same count as the spec's 2026-10-10 figure).

#### Darkmatter production sites (`darkmatter/lib/src/markdown/…`)

| Site | What it special-cases |
| --- | --- |
| `compose/expression/catalog/roots.rs:63-98` | `RESERVED_ROOTS` = `["doc", "ctx", "env", "current", "current_env"]`; order asserted at `roots.rs:114`, mirror test at `:120` |
| `compose/expression/binding.rs:84` `is_reserved_namespace` | catalog search; used by `ResolvedBinding::classify`, `classify_root`, `names_document_property`, global/runtime-value registration refusal |
| `compose/expression/absence.rs:242` `is_reserved_root` | catalog + `null`; feeds `observe_missing` (`expression/mod.rs:485`) and `directive_targets.rs:244` |
| `compose/expression/doc_namespace.rs` | `is_doc_namespace`, `resolve_doc_namespace`, `resolve_doc_namespace_in_map` (dotted reads clone only the leaf; bare clones the map) |
| `compose/conditions.rs:369` | `doc` before ordinary lookup |
| `compose/frontmatter_interpolation.rs:133` | `FrontmatterSeedState::get` |
| `compose/context/effective_state.rs:227, 266` | `get` / `get_checked` |
| `compose/frontmatter_shell_expansion.rs:843` | single-token `doc.*` body is a value, never a command |
| `compose/frontmatter_interpolation.rs:1454` `static_truthiness`, `:1526-1563` `collect_variable_roots` | dependency collection; reused by `collect_deferred_key_references` (`:1588`) and the shell-blocking walker (`:1326`) |
| `compose/interpolation/evaluator.rs:369` `presentation_path_value` | strips `doc.` |
| `compose/directive_targets.rs:238` `expression_path` | `doc.<single>` → path |
| `compose/preflight/collect.rs:1496` `collect_expression_roots`; `:1329-1340` `lazy_root_read` | strips `doc.`; `current`/`current_env` lazy reads |
| `markdown/errors/blocks.rs:527-530` `collect_frontmatter_roots` | skips bare `doc`, strips `doc.` |
| `compose/context/current.rs:58, 468, 509-531` | `CURRENT_ENV_ROOT`, `resolve_current_env` (live `std::env::var`) |
| `compose/context/capture/capabilities.rs:31,48,68,79,85` | `current_env_keys` refresh capability |
| `compose/preflight/mod.rs:257-264` | `deferred_context` docs |
| `compose/expression/error.rs:407-423` | `ReservedRootPathUnknown` message names `current_env` |
| `compose/subtree.rs:13, 538` | doc + test loop over the five root names |

Sites that hardcode `doc` rather than consulting the catalog:
`collect_variable_roots`, `static_truthiness`, `blocks.rs`
`collect_frontmatter_roots`, preflight `collect_expression_roots` and
`lazy_root_read`, evaluator `presentation_path_value`, capabilities
`scan_root`, shell-expansion body classification.

**Bracket dependency gap confirmed:** `doc['a']` parses as
`Index{Variable("doc"), StringLiteral("a")}`; the base is skipped as bare
`doc` and a string index contributes nothing, so no dependency is recorded.
`blocks.rs:525` and `preflight/collect.rs:1494` share the gap.

DMLS (`darkmatter/dmls/src/overlay/expressions.rs`): `document_root()` (`:588`)
looks the root up by `RootMembers::DocumentFrontmatter` (no hardcoded name);
the dasherized quick-fix builds `doc['key']` (`:1118`) and `references_key`
matches `Index{Variable(doc), …}` (`:1139`); `is_undeclared_property`,
`validate_expression`, `static_variable_reads` go through the catalog. DMLS
has no completion/hover for `current_env`. `darkmatter/cli/src` has no sites.

Passive consumers that already classify through the catalog:
`static_variable_reads` (`absence.rs:164`), `validate_expression`
(`prepared.rs:284`), `BindingView::classify_root`/`names_document_property`,
`observe_missing` → `unknown_identifiers::reconcile`, `directive_targets`.

#### Claudine production sites

| Site | What it special-cases | Owner |
| --- | --- | --- |
| `claudine/lib/src/dispatch/expression.rs:87-96, 178-184, 250-287` | hook `doc` root: bare `doc` = flat normalized event fields + `extra` + `os` + `hardware` + optional `git`/`project`; `env` excluded; missing `doc.x` returns `None` (no fallback) | Hook Adapter |
| `claudine/lib/src/composition/sequence/expr.rs:113` (`SourceExpressionLookup::get`) | frontmatter `doc` root in sequence expressions | **not named in plan** — frontmatter → `fm` |
| `claudine/lib/src/composition/looping/expression.rs:128, 222-243` (`resolve_doc`) | frontmatter `doc` root in loop expressions; dotted read clones the whole frontmatter (violates cheap-dotted rule) | **not named in plan** — frontmatter → `fm` |
| `claudine/lib/src/composition/reserved.rs:13` `EXPRESSION_RESERVED_ROOTS` = `["true","false","doc","env"]`, consumed by `looping/config.rs:249` | **not named in plan** — frontmatter → `fm` |
| `claudine/lib/src/composition/lifecycle/bindings.rs:48` `LATE_NAMESPACES = ["current", "current_env"]`; only consumer `approval_diagnostics` (`:232`) → `composition/preflight.rs:470` and `sequence/preflight/mod.rs:907` | Lifecycle Bindings |
| `claudine/lib/src/composition/error/mod.rs:1185` (`LifecycleShellResolution` doc), `:1866` (`SequenceShellLateBinding.root` doc) | Lifecycle Bindings |

Comment-only `current_env` mentions: `composition/prepare.rs:832`,
`sequence/expr.rs:38`, `lifecycle/context.rs:8`, `lifecycle/executor.rs:459,997,1107`,
`looping/engine.rs:738`, `invocation_context.rs:1672`,
`cli/src/commands/wrap/composition/pipeline.rs:1885`, `staged_boot.rs:183`.

Hook adapter callers: templates (`dispatch/template.rs:452`, used by
`runner/{bash,mod,speak,report}.rs` and `messaging/send.rs:139,143`), matchers
(`dispatch/matcher.rs:159`), `when` (`runner/mod.rs:86,156`). **No separate
"harness validation" caller exists**: the phrase appears only in
`dispatch/expression.rs`'s module/type docs. Report/blocking messages render
through `dispatch/template.rs`, so template coverage is the harness-message
coverage; the Hook Adapter task should correct that comment drift.

#### Tests touching the old roots

- Darkmatter `doc` root: `lib/tests/l1/{binding_contract, dasherized_identifier_compose, absent_property_contract, expression_regression, nested_composition, compose_expression_failure_contract, directive_target_analysis}.rs`, `lib/tests/prompts/dasherized_identifier_corpus.rs`, `cli/tests/l1/{compose_transclusion, compose_schema, compose_dasherized_identifiers}.rs`, `dmls/tests/l1/{lsp_session, undeclared_property}.rs`, `dmls/tests/fixtures/mapping_only_corpus/validate.md`, `lib/src/markdown/compose/tests/lookup_parity.rs`, plus unit tests in `doc_namespace.rs`, `conditions.rs`, `effective_state.rs`, `frontmatter_interpolation.rs`, `preflight/collect.rs`, `subtree.rs`, `lexer.rs`, `parser.rs`, `current.rs`, `dmls/src/overlay/expressions.rs`.
- Darkmatter `current_env`: `lib/tests/l1/{current_root_documentation_contract, current_root_migration_guard, context_construction_guard, binding_contract, directive_target_analysis}.rs`, `lib/src/markdown/compose/tests/lazy_roots.rs`, `cli/tests/l1/{compose_lazy_roots, compose_remote_caching}.rs`, `dmls/tests/l1/no_side_effects.rs`, unit tests in `capabilities.rs`, `current.rs`.
- Claudine hook `doc`: `lib/src/dispatch/expression/tests.rs:562-625` (4 tests).
- Claudine frontmatter `doc`: `lib/src/composition/{interpolation_conformance, sequence/task/tests, prepare/tests, lifecycle/executor/tests/binding_contract, lifecycle/context/tests, lifecycle/bindings/tests, lifecycle/tests/diagnostics}.rs`, `sequence/expr.rs` and `looping/expression.rs` unit tests, `cli/tests/l1/{sequence_jit, sequence_groups, agent_text_is_data, compose_schema_cli}.rs`, `cli/tests/level2/{level2_lifecycle_action_forms, level2_auto_complete_chooser}.rs`, `cli/tests/level3/level3_auto_complete_chooser.rs`.
- Claudine `current_env`: `cli/tests/level2/level2_ac28_lifecycle_lazy_roots.rs`, `lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs`, `cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs`, `lib/src/composition/{lifecycle/context/tests, prepare/tests, sequence/preflight/tests, lifecycle/bindings/tests}.rs`, `lib/tests/l1/context_construction_guard.rs`.
- No existing test uses an ordinary property literally named `current_env`; schema-property fixtures with an ordinary `doc` key exist in `darkmatter/lib/src/markdown/schemas/{validate,completion}.rs`.

#### Authored artifacts

- Prompts: `prompts/clarify.md:27,28,38,41,198` (`doc.doc`) and its frozen copy `claudine/lib/tests/fixtures/frozen_prompts/prompts/clarify.md`. No shipped prompt uses `current_env` or the hook `doc` root; `darkmatter/prompts/` and `claudine/prompts/` have no hits.
- Schemas: `darkmatter/schemas/darkmatter.yaml:4` (`doc@./partials/doc.yaml`), `darkmatter/schemas/partials/doc.yaml` (only includer is `darkmatter.yaml`); referenced by path in `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:109-128, 267`. The embedded `darkmatter/docs/schemas/darkmatter.yaml` has no `doc`/`current_env`. Claudine: `claudine/docs/schemas/partials/{mutations.yaml:12, side-effect.yaml:6}`.
- Docs (`current_env`): `darkmatter/docs/topics/{darkmatter-expressions, compose-requests}.md`, `darkmatter/docs/inline/{preflight-checks, interpolation}.md`, `darkmatter/dmls/docs/diagnostics.md`, `claudine/docs/topics/{composition, pre-flight-checks}.md`, `claudine/docs/topics/state-management/context-variables.md`, `claudine/docs/topics/flow-control/{flow-control-reference, lifecycle}.md`. `claudine/docs/rollout-strategy.md` is a planning snapshot, not migrated.
- Docs (`doc` namespace): above plus `darkmatter/docs/inline/fm-interpolation.md`, `darkmatter/docs/topics/state-management/index.md`, `darkmatter/docs/topics/schemas/parsing/lexing.md`, `claudine/docs/topics/{schemas, frontmatter-properties}.md`, `claudine/docs/topics/flow-control/sequences.md`, `claudine/docs/topics/state-management/{side-effects, expression-engine}.md`.
- Skills: `.claude/skills/darkmatter/{compose, schema, library-surfaces, testing}.md`, `.claude/skills/claudine/{cli-commands, cli-reference}.md`, `.claude/skills/claudine/timeline.md` (append only).
- Excluded historical records: `**/features/**`, `**/fixes/**`, `**/reviews/**`, `**/research/**`, existing `timeline.md` entries.

#### Acceptance criterion → owning task

| AC | Owner |
| --- | --- |
| 1 Rename complete | Document Roots, Hook Adapter (+ the three unplanned Claudine frontmatter sites); audited by Migration Audit |
| 2 `fm` behaves as `doc` did | Document Roots (Darkmatter), Hook Adapter (Claudine) |
| 3 `doc` is userland | Document Roots, Passive Tooling |
| 4 Reserved roots shadow frontmatter | Document Roots (`md compose`), Lifecycle Bindings (Claudine) |
| 5 Live environment root removed / per-document env | Document Roots (removal), Document Snapshot + Entry Routing + Lifecycle Bindings (capture) |
| 6 Editor support | Passive Tooling |
| 7 Docs and skills | Documentation Migration, Authored Catalogs |
| 8 Gates | Required Gates |

#### Input robustness matrix

Not applicable. The inventory found no manifest/config/schema-loader
deserialization change: the work alters expression lookup, in-memory request
inputs, and authored schema descriptions/keys (the partial rename changes a
type key and a path, read by the existing loader unchanged).

### Wave 1 — Environment Spike

Run once on macOS with a supplied-map fixture; no process environment was
mutated and no provider was launched.

**Retained regression test:**
`darkmatter/lib/tests/l1/request_context_builder.rs::an_environment_only_replacement_enters_a_document_under_another_map`
(passes, 0.13 s). In one repository fixture with a fixture home:

- document `first.md` prepared from a snapshot carrying map A renders
  `env.NAME` and `markdown_title("{{NAME}}/target.md")` from map A, and its
  `::file ./child.md` transclusion renders map A too (transclusion shares the
  including document's map);
- `first.resolution_context().clone().with_env(B)` passed to
  `ComposeRequest::with_context` composes `second.md` with map B for both
  `env.NAME` and the `{{NAME}}` file reference, and aligns `ctx.env`
  (`options().context().env()`) to B;
- restoring map A on that context yields a context equal (`PartialEq`) to the
  first one: home, repository root, launch `@` scope, source/tree anchors are
  untouched by an environment-only replacement; the first request still
  reads map A.

**Minimum API seam (Darkmatter):** `FileResolutionContext::with_env` +
`ComposeRequest::with_context` already exist and suffice; no new Darkmatter
API is needed for the document entry itself. One caveat:
`FileResolutionContext::with_env` calls `reselect_tree()` because a `VAULT`
entry participates in tree-root selection when no repository or explicit base
dir exists. That is consistent with "every consumer reads the document map"
(the vault tree root is an environment consumer), but Phase 3 must state it in
the request docs.

**Actual capture sites today (one map per process, not per request):**

- `darkmatter/cli/src/main.rs:156` `RequestSnapshot::from_process()`.
- `claudine/cli/src/request.rs:31` `initialize()` stores
  `from_process().at_request_dir(launch_dir)` in a `OnceLock`;
  `InvocationContext::capture_with_observation`
  (`claudine/lib/src/invocation_context.rs:999-1003`) copies it into
  `inner.environment`, feeding `capture_launch_context` and `derive_source`.
- Sequence task / serial group leg: `cli/src/commands/wrap/sequence/task_run.rs:209-235`
  `prepare_task_context` derives from the one invocation and inserts
  `env_overrides` into `task_context.env_mut()` **only** — the task's
  `FileResolutionContext` keeps the launch map, so the two can disagree today.
  Parallel legs share one `open_group_run` (`task_run.rs:449-464`).
- Proxy: `lib/src/composition/lifecycle/control.rs:288` and
  `coordinator/commit.rs:83` use the invocation's `request_context`.

**Live process reads that bypass the snapshot (Phase 3 must route these
through the document map or justify them):** `composition/select.rs`
(`std::env::var` ×6, provider/model selection — i.e. provider preparation
reads the live env), `looping/config.rs:19-48` (×4),
`invocation_context.rs` `refresh_evidence` (`std::env::vars()`, serving lazy
`current.agent`/`current.model`) and `EnvBaseline::capture` (child env
baseline). All are listed in `claudine/lib/tests/l1/context_construction_guard.rs`.

**Missing Claudine seam:** `InvocationContext` has no environment-only
derivation; `capture*` take a fresh repository observation and
`derive_source`/`derive_composition_source` keep the snapshot env. Phase 3
needs one `InvocationContext` method that replaces `inner.environment` and the
derived `FileResolutionContext` map together while keeping the repository
observation, home, and source anchors (the Darkmatter half is proven above).

**Tests that encode request-wide reuse (to be rewritten in Phase 3):**
`claudine/lib/src/composition/sequence/tests.rs::external_sequence_reuses_request_snapshot_after_environment_mutation`
and `lifecycle/control/tests.rs::lifecycle_proxy_reuses_request_snapshot_after_environment_mutation`
(both pass today).

### Approval ruling

- **Prerequisite status:** `2026-10-23-shell-template-approval` is not
  implemented (status `finalized-spec`; no `TemplateApproval`/template
  approval code exists). It is pipeline step 13; this feature is step 8.
- **Current behavior, with evidence:**
  - Task `shell:`/`setup:`/`teardown:` bytes are resolved once at sequence
    preflight (`sequence/preflight/mod.rs:850-890`) and executed verbatim;
    a site with no record fails closed (`SequenceShellResolution`,
    `task/mod.rs:894-903`). Tests: `the_preflight_approved_bytes_are_executed_verbatim`,
    `approved_bytes_are_never_re_interpolated` (pass).
  - Exact-byte enforcement in Darkmatter: `shell_expansion/mod.rs:244-262`
    returns `NotPreApproved` before execution (`pre_approved_rejects_unknown_commands`).
  - Changed bytes in a later loop iteration are refused before they run:
    `claudine/cli/tests/l1/preflight_execution_parity.rs`
    `a_loop_body_command_is_refused_when_only_the_first_iteration_is_approved`,
    `a_loop_lifecycle_shell_is_refused_when_only_the_first_iteration_is_approved`,
    `a_lifecycle_no_error_shell_does_not_absorb_an_unapproved_command` (all pass).
  - **Not strictly fail-closed:** JIT prompt/step documents
    (`cli/.../sequence/jit.rs:192-232`) call `approve_document_shell` again at
    run time with the shared approval cache, which goes through
    whitelist/handler/cache (and an interactive prompt on a TTY); without any
    of those it fails with `ShellApprovalUnavailable`. New bytes are never run
    unapproved, but they can be newly approved mid-sequence.
  - No test today covers a later sequence task whose environment change
    produces new command bytes.
- **Consequence for per-document capture:** once each task captures its own
  map, a task's preflight-approved bytes were resolved against the launch map,
  not the task's map. Without template approval, the task must re-resolve its
  bytes against its own map and refuse when they differ from the approved
  bytes, before execution.
- **Ruling (author, 2026-10-13): option A — retain exact-text approval and
  refuse changed commands.** This is a recorded, temporary departure from
  spec §4's template-approval sentence; the spec is left as decided.
  - Phase 3 implements per-document capture in full (task, serial/parallel
    group leg, and `proxy` target; selectors, loop settings, lazy evidence,
    file references, schema inputs).
  - At each later task, preflight-approved shell text is re-resolved against
    that task's captured map. Identical text runs as approved; different
    text is refused before execution through the existing enforcement
    (`NotPreApproved` / the sequence shell-resolution refusal), with an error
    naming the command and the change. Stale approvals never authorize new
    text, and no template approval is introduced here.
  - JIT prompt/step documents keep their existing run-time approval of the
    exact text they are about to run; the refusal applies only to text
    approved at sequence preflight.
  - Tests use supplied maps: an unchanged map runs the approved text, a
    changed map that alters command text is refused before execution, and a
    changed map that leaves command text identical runs.
  - `2026-10-23-shell-template-approval` (pipeline step 13) replaces the
    refusal for later tasks when it lands.

### Phase 1 tests and gates

Phase 1 changes no production behavior; its one code change is the retained
spike regression test.

| Requirement | Test / evidence |
| --- | --- |
| Environment-only replacement preserves home, repository, launch `@` scope, source/tree anchors | `request_context_builder::an_environment_only_replacement_enters_a_document_under_another_map` (new) |
| Transclusion reads the including document's map; a separately entered document reads its own | same test (map A in `first.md` + `child.md`; map B in `second.md`; `env.*` and `{{VAR}}` file reference agree) |
| Current approval refuses changed bytes before execution | existing `preflight_execution_parity` refusal tests (3) and `shell_tasks` verbatim-bytes tests (2), re-run and passing |
| Current request-wide reuse contract (to be replaced in Phase 3) | existing `external_sequence_reuses_request_snapshot_after_environment_mutation`, `lifecycle_proxy_reuses_request_snapshot_after_environment_mutation`, passing |

Gates (macOS, 2026-10-12):

- `cargo nextest run -p darkmatter --test l1 an_environment_only_replacement` — 1 passed.
- `claudine/`: `just test` — 8699 passed, 9 skipped; `just lint` — clean.
- `darkmatter/`: `just lint` — clean (includes DMLS and the zed-dmls WASM check).
- Not run: Darkmatter's full `just test` (only one Darkmatter test file
  changed; the targeted test passed), and no cross-OS run. The new test reuses
  the existing portable patterns of `request_context_builder.rs`
  (`canonicalize_simplified`, `to_portable_string`, temp-dir anchored request,
  no process environment mutation), so it carries no new OS risk; CI's
  event-driven schedule covers Linux on the PR.
- Input robustness matrix: not applicable (see the inventory).
- No pre-existing failures observed.

### Phase 1 re-verification (2026-10-12)

A second Phase 1 run found both Wave 1 tasks already complete and checked
off (committed in `dda140c7c`). No code, docs, or skills changed in this run.
Gates re-run on macOS:

- `cargo nextest run -p darkmatter --test l1 an_environment_only_replacement` — 1 passed.
- `claudine/`: `just test` — 8699 passed, 9 skipped; `just lint` — exit 0
  (one unrelated macOS linker note about `__eh_frame` size).

The approval ruling was unresolved at this point; it was decided on
2026-10-13 (option A, see "### Approval ruling").

## Review cycle 1 migration audit

Scope: review-1 finding 1's artifact rows (shipped prompts, frozen fixtures,
topic pages, READMEs, skills) and acceptance criterion 1's discovery searches,
run on the working tree on 2026-10-12 after the migration edits. Historical
records excluded by path: `**/features/**`, `**/fixes/**`, `**/reviews/**`,
`**/research/**`, and `.claude/skills/claudine/timeline.md` entries dated
before 2026-10-12. `current_env` hits are out of scope here (the live-root
removal is a separate work unit) and are not classified.

### What was migrated

| Class | Sites | Change |
| --- | --- | --- |
| Shipped prompt | `prompts/clarify.md` (5 spans) | `doc.doc` → `doc`: the prompt reads its own ordinary `doc` property |
| Frozen prompt fixtures | `claudine/lib/tests/fixtures/frozen_prompts/prompts/clarify.md`; new copy `claudine/cli/tests/fixtures/frozen_prompts/prompts/clarify.md` | byte-identical to the shipped prompt; the CLI copy feeds the new end-to-end test |
| Darkmatter docs | `topics/darkmatter-expressions.md` (availability invariant, value sources, reserved list, Mermaid root diagram, variable access, Namespaces table, new "The `fm` namespace" section with shadowing example and `doc` → `fm` migration table, token resolution, condition shortcut order, Host Bindings, bracket-access note), `inline/fm-interpolation.md`, `inline/interpolation.md`, `topics/state-management/index.md`, `topics/schemas/authoring-schemas.md`, `topics/schemas/parsing/lexing.md` | `doc` root → `fm`; reserved-root shadowing and ordinary `doc` stated |
| Claudine docs | `topics/flow-control/lifecycle.md` (early-binding list, unavailable-global note, lifecycle context, `fm.err` escape hatch now covering `timing` and the fixed roots), `flow-control/sequences.md`, `flow-control/looping.md`, `frontmatter-properties.md`, `composition.md`, `state-management/expression-engine.md` (plus a shadowing paragraph) | `doc` root → `fm` |
| Claudine hook docs | `topics/configuring-actions.md` (`hook` row in Available Paths, new "The `hook` root" section with hook `doc` → `hook` migration table, `{{hook}}` template row, expression-mode matchers), `topics/unified-events.md` §3.7, `cli/README.md`, `lib/README.md` | hook-only `hook` root documented |
| Skills | darkmatter `compose.md`, `library-surfaces.md`, `dmls.md`; claudine `hook-actions.md`, `architecture.md` (`EventMetaExpressionLookup` resolution list); claudine `timeline.md` new 2026-10-12 entry | `doc` → `fm`; `hook` added |
| Rust tests with stale spellings | `claudine/lib/src/composition/lifecycle/tests/validation.rs` (`doc.err` escape-hatch test → `fm.err`, renamed), `claudine/cli/tests/level2/level2_lifecycle_action_forms.rs` (literal `LITERAL=doc.title` → `fm.title`, matching its module doc), `darkmatter/dmls/src/overlay/expressions.rs` (two kebab-path unit tests), `darkmatter/lib/src/markdown/compose/expression/parser.rs` (object-key list half-migrated), `darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs` (assertion message) | spelling only; assertions unchanged |

Clean (no old-root use found): `darkmatter/docs/topics/compose-requests.md`,
`darkmatter/dmls/docs/diagnostics.md`, `hover.md`, `autocomplete.md` (already
migrated by the passive-tooling unit), all Darkmatter READMEs,
`claudine/docs/topics/state-management/context-variables.md`,
`topics/pre-flight-checks.md`, darkmatter skill `schema.md` and `testing.md`,
claudine skill `cli-commands.md` and `cli-reference.md`, `darkmatter/prompts/`,
`claudine/prompts/`, and the darkmatter lib/cli and claudine cli/lib frozen
prompt trees other than `clarify.md`.

### Classification of remaining hits

Search 1 (`{{ … doc … }}`):

| Hit | Class |
| --- | --- |
| `prompts/clarify.md` and its two frozen copies (`{{doc}}`, `parent_dir(doc)`, `basename(doc)`) | ordinary property (the prompt declares `doc: "{{spec \|\| design}}"`) |
| `darkmatter/lib/tests/l1/binding_contract.rs:179`, `fm_namespace_contract.rs:198-216`, `darkmatter/dmls/tests/l1/no_side_effects.rs:336`, `undeclared_property.rs:226,313`, `claudine/cli/tests/l1/compose_schema_cli.rs:1806` | ordinary property tests (`doc` declared, or asserted undeclared) |
| `claudine/cli/tests/l1/compose_schema_cli.rs:1268-1305` | ordinary property test (new frozen `clarify.md` test) |
| `claudine/lib/src/dispatch/template/tests.rs:380-381`, `dispatch/runner/report.rs:222-227` | explicit migration test: the old hook spelling stays literal |
| `darkmatter/docs/topics/darkmatter-expressions.md:366,406-408`, `state-management/index.md:22`, `claudine/docs/topics/state-management/expression-engine.md:84`, `darkmatter/dmls/docs/hover.md:147` | ordinary-property documentation or explicit migration table |
| `claudine/docs/topics/configuring-actions.md:317`, `unified-events.md:941`, `claudine/cli/README.md:95` | explicit hook migration guidance |
| `darkmatter/dmls/tests/fixtures/mapping_only_corpus/validate.md:6` (`{{doc.doc}}`) | unrelated literal: a frozen corpus of prompts as of commit `3df43cc67` whose source prompt no longer exists; its byte-identical diagnostics baseline is the test's subject, and the document also declares `doc`, so it is an ordinary-property path under the new contract |

Search 2 (`when` with `doc.`): `claudine/lib/src/dispatch/runner/tests.rs:472`
(explicit migration test: the old condition skips the action) and
`claudine/docs/topics/configuring-actions.md:316` (migration table).

Search 3 (quoted `doc.` paths in Rust): `fm_namespace.rs:150,159,196` and
`claudine/lib/src/dispatch/expression/tests.rs:602-603` (explicit migration
tests: `doc.` is not the root), `darkmatter/dmls/src/overlay/expressions.rs:1341-1342`,
`claudine/lib/src/composition/looping/expression.rs:421`, and
`darkmatter/dmls/tests/l1/undeclared_property.rs:297` (ordinary property tests).

Additional forms the regexes miss: a multiline `{{ … }}` search and a `$( … )`
token search over the three prompt folders, both `docs/` trees, and the two
skills found only `doc.md` file paths. Bracket forms (`doc['…']`/`doc["…"]`) in
Rust are all `serde_json`/`toml_edit` locals named `doc`. A production-source
search for `"doc"`, `"doc."`, `"doc["`, and `starts_with("doc` outside test
modules found only the `err.detail.doc` payload key
(`claudine/lib/src/diagnostics/registry.rs`, `composition/error/render/mod.rs`),
JSON-schema property names in validator tests, and the `doc` file extension:
no production code special-cases a `doc` root.

Defects found and fixed in this audit: the five stale Rust test spellings in
the table above, and the availability invariant on the main expressions page,
which claimed `fm.*` resolves in Claudine hook conditions (hooks have no
frontmatter; they read `hook`).

Coverage of the three evaluation families: document composition is exercised
by `fm_namespace_contract.rs` and the new
`compose_schema_cli::compose_frozen_clarify_reads_its_ordinary_doc_property`
(the frozen shipped prompt through `claudine compose --dry-run`); hook
dispatch (templates, `when`, matchers, report messages) by the hook-adapter
unit's tests named in the log; harness validation has no separate message that
names the payload root (report/speak/message templates share `interpolate`).

Not a `docs/` defect, recorded for the author: `claudine/docs/rollout-strategy.md`
names this feature by slug. It is a roadmap of scheduled features rather than a
behavior topic page, and every row there names a slug, so it was left as is.

## Review cycle 5: per-document environment

Scope: review-5's one finding ("Per-document environment capture and its
acceptance tests are not implemented"), unblocked by the author's ruling of
2026-10-13 (option A, recorded under "### Approval ruling" above). Work units
1, 2, and 2b of "## Implementation of Review Findings #5" in the
[repair log](log.md) carry the full detail; this section is the inventory.

### The seam

- `InvocationContext` is now a view of one document.
  `enter_document(DocumentEnvironment) -> InvocationContext` replaces only the
  environment-dependent state, together: the request snapshot map, the launch
  `FileResolutionContext` map and the `VAULT`-selected tree root, and the
  child-process baseline. Home, launch CWD, repository and host evidence and
  caches, the launch `@` scope, source anchors, and work accounting stay
  shared.
- `capture_document()` enters a map from the installed `EnvironmentSource`
  (`with_environment_source(..)`); with none installed it re-enters the
  launch map, so the library never reads the process.
  `document_environment()` and `DocumentEpoch::environment()` expose the map.
  New types `DocumentEnvironment` and `EnvironmentSource` live in
  `lib/src/invocation_context/document_environment.rs`.
- CLI boundary: the launch document is the one
  `RequestSnapshot::from_process()` in `cli/src/request.rs`;
  `request::document_environment_source()` (`biscuit_file::capture_env`, with
  a reasoned allowance in the CLI context-construction guard) is installed on
  the compose and sequence invocations. Production captures pin `PWD` to the
  launch value, because Claudine rewrites `PWD` on every child-CWD switch
  (`switch_process_cwd`).
- Child processes start from the launch raw baseline with only the variables
  the document's map changes, adds, or removes rewritten; home variables are
  never rewritten (`HomeBaseline` is request-owned).
- Observable capture count: `InvocationWorkSnapshot::document_environment_captures`
  is 1 for the launch document plus 1 per entered document; epochs,
  re-preparation, extension, and transclusion add none.

### Routes

| Document | Capture site |
| --- | --- |
| Sequence step (its composition, `initialize`, task, prompt document, body request, loop iterations, retry/resume, handlers, `finalize`) | `iterate::run_one_step` → `capture_document()`, then `CompositionPrepContext::for_document` and `SequenceRunContext::for_document` |
| Serial group leg | `WrapperPromptRunner::open_member_run` |
| Parallel group leg | new `PromptTaskRunner::open_leg_run`, every leg in declaration order before any leg starts, joining the group's shared `RunEvidence` |
| Prompt task | composes and launches under `PromptTaskRequest.document` (the leg's or step's view) |
| `proxy` target | the owning coordinator (`compose/prep.rs` command loop, `iterate::run_step_proxy_loop`), after the commit resolves the target in the source's file context and before the target is prepared |

Consumers rerouted to the view: lazy `current.*` (`refresh_evidence`), `ctx`
evidence, `derive_source`, event environment contexts, library selectors
(`ambient_env_lookup` removed; `prepared_environment(prepared)`, hint
selectors take `&DocumentEnvironment`), loop settings
(`resolve_*_from_env(&DocumentEnvironment)`), CLI model resolution
(`resolve_document_model`, including the retry/resume rebuild via
`LaunchRebuildIntent.environment`), and the provider model stage
(`exec_prep::resolve_model_and_validate`, `LaunchPlanInputs.environment`).

### Approval refusal (implemented spec departure)

Implemented under the 2026-10-13 ruling, as a temporary departure from spec
§4's template-approval sentence; the spec is left as decided.

- Preflight records a `PreflightApproval` (authored text, stage, step
  frontmatter, both contexts, file-resolution context) for every
  interpolating `shell:`/`setup:`/`teardown:` command.
- `TaskExecution` (new `document` field) re-resolves each, in stage order and
  before any stage runs, under its document's map through the same
  `compose_shell_text` preflight uses (`sequence/preflight/reapproval.rs`).
- Identical text runs. Changed text fails the task with
  `CompositionError::SequenceShellApprovalChanged`, naming the command, the
  approved and resolved text, and the changed variables. The refused command
  never runs.
- Prompt and step documents keep run-time exact-text approval
  (`jit::compose_step` unchanged).
- `2026-10-23-shell-template-approval` replaces the refusal when it lands;
  the `verify_preflight_approvals` comment and the `reapproval.rs` module doc
  mark the interim contract.

### Model re-selection (design choice)

Preflight keeps planning every step's target under the launch map, because
the review screen, the forwarded-switch check, and `--dry-run` need it before
any step runs. At step entry (`run_one_step`) and when a group leg or a
step's prompt task runs (`WrapperPromptRunner::document_target`),
`sequence::resolve::StepSelection::for_document` re-selects the model under
that document's map, and that model runs: it becomes the request's
`resolved_target`, the `MODEL` override, and the rebuild intent's
`fallback_model`. "Use" was chosen over "refuse" because model selection is
not an approval: a `SequenceReview` or `ExplicitCli` model stands, and every
other reason is a reading of the environment and is re-read. The provider is
never re-selected (`classify_agent_resolution` and the picker read no
environment variable). `proxy` targets already select under their own epoch.

### Shell child environment

- Darkmatter: `ComposeOptions::with_shell_child_environment(ShellChildEnvironment)`
  (new public type in `shell_expansion::types`). When set, `run_action` clears
  the inherited environment, applies the supplied raw `OsString` pairs, looks
  the executable up on the supplied `PATH` (`which::which_in`), and sets `PWD`
  to the command directory; `NO_COLOR` stripping still applies. Encoded in the
  options' graph identity, left out of the compose-cache fingerprint. Reaches
  `run_action` through `execute_directive_outcome` (`::shell`,
  `::shell-block`, frontmatter `$( … )`).
- Claudine: `canonical_compose_options` sets it from the epoch's (else the
  view's) `env_baseline()`; `system_prompt/prepare.rs` and `wrap/overlay.rs`
  set it from their view. `ShellRunner`/`TaskShellRunner::for_child_environment`
  (default `None`); `SystemShellRunner` returns `DocumentShellRunner`,
  `SystemTaskShell` gained `in_environment`; `TaskExecution::{run_shell,
  run_stack}` ask for the task's `document` environment. Each view holds its
  `DocumentShellRunner` (`shell_runner()`, also on `DocumentEpoch`), handed to
  the stacks in `task_run.rs`, `compose/prep.rs`, and
  `harness_orch/loop_control/lifecycle_events.rs`.

### Tests

All use supplied maps; none mutates the process environment.

- Unit 1, lib `invocation_context/document_tests.rs`:
  `every_consumer_of_the_launch_document_reads_its_captured_map`,
  `a_second_document_reads_its_own_map_and_the_first_keeps_its_own`,
  `re_preparing_a_document_captures_no_new_environment`,
  `a_captured_document_reads_the_installed_source_or_else_the_launch_map`,
  `a_child_process_starts_from_the_document_map_with_the_launch_home`,
  `a_preparation_without_a_snapshot_reads_its_resolution_context_map`.
- Unit 2, lib: `composition::sequence::tests::an_external_sequence_reference_resolves_under_each_documents_own_map`
  and `composition::lifecycle::control::tests::a_proxy_target_reference_resolves_under_the_map_of_the_document_that_authors_it`
  (rewritten from the two request-snapshot-reuse tests);
  `composition::sequence::task::tests::document_environment::{an_unchanged_map_runs_the_approved_text,
  a_changed_map_that_leaves_the_text_identical_runs_it,
  a_changed_map_that_changes_the_text_is_refused_before_anything_runs,
  a_task_with_no_document_view_runs_the_approved_text}`.
- Unit 2, CLI `cli/tests/l1/document_environment_routes.rs` (through
  `CliProcessFixture` and the `test-fixtures` seam
  `CLAUDINE_TEST_DOCUMENT_ENVIRONMENTS`):
  `successive_sequence_steps_each_read_the_map_they_captured`,
  `serial_group_legs_each_capture_their_own_map`,
  `parallel_group_legs_each_capture_their_own_map`,
  `a_proxy_target_captures_its_own_map_before_its_initialize`,
  `re_entering_a_document_reads_its_map_without_capturing_again`,
  `a_failure_handler_and_its_retry_read_the_documents_map` (Unix),
  `a_later_step_runs_approved_text_only_while_its_map_leaves_it_unchanged`,
  `a_prompt_task_and_its_transclusion_read_their_legs_map` (Unix).
- Unit 2b, CLI (same file): `task_shells_start_from_the_map_their_step_captured`,
  `serial_group_leg_shells_start_from_their_legs_map`,
  `parallel_group_leg_shells_start_from_their_legs_map`,
  `a_later_step_selects_its_model_and_starts_its_shells_from_its_own_map`
  (Unix), `each_prompt_leg_selects_its_model_from_its_own_map` (Unix).
  Darkmatter `lib/tests/l1/shell_child_environment.rs` (Unix):
  `a_body_directive_and_a_frontmatter_command_read_the_supplied_environment`,
  `the_supplied_environment_replaces_the_inherited_one_and_pwd_names_the_working_directory`.
- Mutation checks (from the log): reverting `refresh_evidence` and the
  resolution-context replacement fails three unit-1 tests; reverting the
  step/leg/target captures fails all seven unit-2 CLI route tests; disabling
  only the refusal fails the option-A test; disabling the runners'
  `for_child_environment`, `DocumentShellRunner`'s environment, and the
  compose child environment fails all four CLI shell tests; disabling only
  re-selection fails both model tests.

### Gate evidence

| Unit | Gates |
| --- | --- |
| 1 | claudine `just lint` passed; claudine `just test` 8719 passed, 9 skipped, 0 failed; `just check-tier-coverage claudine` 0 stranded |
| 2 | claudine `just lint` passed; claudine `just test` 8731 passed, 9 skipped, 0 failed; `just check-tier-coverage claudine` 0 stranded |
| 2b | claudine `just lint` passed; claudine `just test` 8736 passed, 9 skipped, 0 failed; darkmatter `just lint` passed; darkmatter `just test` 9778 passed, 11 skipped, 0 failed; `just check-tier-coverage` claudine and darkmatter 0 stranded |

All on macOS. The `$DOC_MARK`-through-`printenv` tests are Unix-only; the
task and leg shell tests are cross-platform through `%DOC_MARK%` but had only
run on macOS when this section was written.

### Plan checkboxes

- **Document Snapshot** is checked: the entry API exists (`enter_document`,
  `capture_document`), the unit-1 consumer agreement test covers `env.*`,
  `ctx.agent`, lazy `current`, an env-expanded reference, a transclusion,
  approved and executed `::shell` text, the `SCHEMAS_DIR` schema root, the
  model selector, and the loop settings, and source anchors and
  request-owned state are shared, not replaced.
- **Entry Routing** is checked: every document boundary (step, serial leg,
  parallel leg, `proxy` target) has changed-map evidence, and loop
  iterations, `success` retry, failure handlers and their retry, `finalize`,
  and transclusion have stable-map evidence. No test named in the log
  exercises `resume` specifically; it shares the retry rebuild path
  (`LaunchRebuildIntent.environment`).
- **Lifecycle Bindings** is checked on review-cycle-1 evidence:
  `LATE_NAMESPACES = ["current"]`, retired `current_env` errors removed,
  `binding_contract::{lifecycle_globals_shadow_frontmatter_keys_that_fm_still_reads,
  reserved_roots_shadow_same_named_frontmatter_keys_in_a_lifecycle_event,
  a_set_is_read_by_bare_and_qualified_fm_and_an_earlier_snapshot_is_unchanged}`,
  and `prepare::tests::current_env_in_a_lifecycle_shell_command_reads_the_frontmatter`.
  The task's bullet on sequence-owned frontmatter properties (`fm.state`
  reading the same value as `state`) has no named test in the log.

### Documentation

Topic pages updated to the per-document contract: Darkmatter
`docs/topics/compose-requests.md` ("One environment per document", with an
anchor kept for the old `#one-environment-per-request` link, plus the shell
child environment subsection); Claudine `docs/topics/composition.md` (new
"One Environment per Document" section with a Mermaid diagram, approval and
model sub-sections, and corrections to "One owner", the Git-state bullet,
target identity, Request-Owned Context, and the sequence summary),
`flow-control/sequences.md`, `flow-control/lifecycle.md`,
`flow-control/flow-control-reference.md`,
`state-management/context-variables.md`, and `pre-flight-checks.md`. Skills:
claudine `architecture.md`, `cli-commands.md`, `cli-reference.md`, and a
2026-10-13 `timeline.md` entry; darkmatter `requests.md` and `compose.md`.
