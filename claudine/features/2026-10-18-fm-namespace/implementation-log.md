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
human_review: true
human_review_items:
    - |-
      **Shell approval when each sequence task gets its own environment**

      **Why this is needed before Phase 3 (Phase 2 can go ahead without it):**
      this feature makes every sequence task read its *own* copy of the
      environment variables, captured when that task starts, instead of one
      copy taken when the whole run started. Shell commands in a sequence are
      approved once, up front, as exact text. If a task's environment differs
      from the start of the run, a command such as `deploy {{ env.TARGET }}`
      can turn into different text than the text that was approved. The spec
      assumes a later feature (`2026-10-23-shell-template-approval`, step 13
      of the pipeline; this feature is step 8) already approves commands as
      *templates* and checks the values at run time. That feature is not built
      yet, and the plan says only you can decide how to bridge the gap.

      What happens today: task shell commands run exactly as approved (never
      re-filled), and a loop iteration whose command text changed is refused
      before it runs. Prompt documents inside a sequence, however, go through
      normal approval again at run time, so new text can be approved there by
      a whitelist, a handler, or an interactive prompt.

      **Options:**

      - **A. Interim rule: exact text or refuse (recommended).** Keep today's
        exact-text approval. When a task's own environment would produce
        command text different from what was approved, refuse the task before
        anything runs, with a clear error. Record this as a deliberate
        departure from the spec until template approval lands.
        - Pros: never runs an unapproved command; no new approval machinery;
          the pipeline order is unchanged; template approval later simply
          relaxes the refusal.
        - Cons: a sequence whose tasks really do see different environment
          values fails until step 13 lands; the spec's sentence about
          template approval stays untrue for a while.
      - **B. Build template approval first.** Move
        `2026-10-23-shell-template-approval` ahead of this feature's Phase 3.
        - Pros: the spec is implemented exactly as written.
        - Cons: reorders a pipeline designed to run front to back; this
          feature stalls half-finished for a much larger piece of work.
      - **C. Split the feature.** Ship the `doc` → `fm` / `hook` rename and the
        `current_env` removal now, and move "one environment per document"
        to land after step 13.
        - Pros: no interim behavior at all.
        - Cons: the spec's acceptance criterion 5 is only half met; the
          environment work has to be re-planned separately.

      **Recommendation: A.** It is the safe default the plan itself
      suggests: it can only refuse, never run something unapproved, and it
      costs nothing to undo once template approval exists.
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
  6. The approval ruling is unresolved (see `human_review_items`); it blocks
     Phase 3 only. The environment-only seam is proven at the Darkmatter
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
- **Ruling: unresolved — author decision required before Phase 3.** Raised as
  `human_review_items`. Recommendation (per plan): accept the interim
  contract of exact-byte approval with refusal of changed commands before
  execution, recorded as a spec departure. Phase 2 (namespace rename) does not
  depend on it.

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
