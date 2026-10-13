---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-20-lifecycle-grammar/spec.md
plan: claudine/features/2026-10-20-lifecycle-grammar/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - claudine/cli/tests/l1/compose_schema_cli.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
human_review: false
message_to_agent: |-
  Phase 1 froze the contracts you must build against. Read the
  implementation log's "Frozen contracts" and "Chosen diagnostic names"
  sections before editing; they are binding unless you record a departure.

  Key points for Phase 2 (Wave 2, shared action model):
  - Node model: `LifecycleNode { Action, Conditional }`, with
    `LifecycleConditional { when, then, otherwise, path }` and a
    `path: LifecyclePath` on every `LifecycleAction`. `LifecycleStackItem`
    and `LifecycleNotification` are deleted; stacks are `Vec<LifecycleNode>`
    (empty means absent).
  - `LifecyclePath` is the promoted `source_map.rs::LifecycleSurfacePath`
    (Field / MapKey / Index segments), rendered like
    `failure[0].else[1].shell.command`. It fills the existing `property`
    field of errors; delete the post-hoc "{event}.stack[i]" annotator in
    parse.rs.
  - Effect check: parse.rs gets `LifecycleParseOptions { is_known_effect,
    authored }`; the public `parse_lifecycle_config(fm, path)` moves to
    lifecycle/mod.rs and supplies a playa-backed predicate from audio.rs.
    Today `parse.rs:297` is the only AC9 violation (plus the doc comment at
    `parse.rs:68`).
  - Errors: add `RemovedGrammar { source_path, path, removed, replacement }`;
    rename `LifecycleProxyWith*` to `LifecycleWith*` (EvaluationFailed gains
    `verb`), and `LifecycleProxyOnlyParameter` to `LifecycleWithUnsupported`
    (`param` dropped; it was always `with`). Rename `ProxyWith` ->
    `WithOverlay`, `ProxyWithValue` -> `WithValue`, `ProxyWithError` ->
    `WithOverlayError`.
  - The old `{action: verb, ...}` discriminator long form is still live in
    parse.rs (`parse_long_form_action_object`) and must become a
    `RemovedGrammar` error; the regex inventory does not find it.
  - `_loop_*` seeding lives in `looping/types.rs::insert_ambient_overrides`,
    not in `reserved.rs` (which only lists the names).

  Phase 1 also fixed one pre-existing L1 failure (a trailing-space match in
  `compose_schema_cli.rs`); `just test` and `just lint` in claudine/ were
  green at the end of Phase 1 (8740 passed, 9 skipped).
---
# Implementation Log for 2026-10-20-lifecycle-grammar (5 phases)

## Phase 1

Phase 1 is readiness, inventory, and contract freezing. No grammar code, doc,
or skill changed. The one source edit fixes a pre-existing test failure (see
"Phase 1 verification").

### Prerequisite: `2026-10-18-fm-namespace`

Ready. Evidence, from code rather than the spec alone:

- The spec is closed (`_completed/`, implementation log through review cycle 6).
- `fm` is a Darkmatter fixed root: `darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs`
  `RESERVED_ROOTS` = `fm`, `ctx`, `env`, `current`.
- Claudine's reserved list is `["true", "false", "fm", "env"]`
  (`claudine/lib/src/composition/reserved.rs:13`); `doc` is gone.
- `current_env` survives only in tests that assert it is now ordinary
  frontmatter data (`lifecycle/bindings/tests.rs:236`,
  `prepare/tests.rs::current_env_in_a_lifecycle_shell_command_reads_the_frontmatter`,
  DMLS `overlay/expressions.rs:1343-1377`). `LATE_NAMESPACES = ["current"]`.
- Host-declared binding mechanism exists and is what `loop` will use:
  `darkmatter::markdown::compose::expression::{BindingView, BindingViewBuilder::declare,
  GlobalDeclaration, Availability::{Available, Unavailable, ExecutionDependent},
  UnavailabilityReason, RuntimeBinding}` (`expression/binding.rs:186-340`), with
  Claudine's per-scope catalog in `claudine/lib/src/composition/lifecycle/bindings.rs`
  (`err`/`timing`/`group`/`outputs`). Root classification order is reserved
  root → declared global → document property, so a declared `loop` wins over
  the `loop:` configuration block, which stays readable as `fm.loop`.

### Baseline inventory (AC16 search, 2026-10-13)

Pattern `^\s+stack:|say_first|^\s+- action:|_loop_(count|is_first|is_last|last_output|last_exit_code)`
over `prompts/ claudine/docs claudine/lib claudine/cli darkmatter/docs
darkmatter/dmls darkmatter/lib darkmatter/prompts claudine/prompts
.claude/skills/claudine`: **161 files, 1180 hits (73 Rust, 88 non-Rust)**.
`darkmatter/prompts/` and `claudine/prompts/` have no hits.

Known false positives (not grammar; leave as-is):

- Rust struct fields named `stack:` — `darkmatter/lib/src/markdown/render_tree/fold.rs`,
  `darkmatter/lib/src/markdown/schemas/resolve.rs`,
  `darkmatter/lib/src/markdown/compose/transclusion/types.rs`,
  `claudine/cli/src/commands/wrap/sequence/task_run.rs:164`.
- `claudine/lib/src/reporting/queries/common.rs:189` — SQL alias
  `human_in_loop_count` matches `_loop_count`.
- `darkmatter/dmls/docs/diagnostics.md:200` — `- action: archive` is
  `content_policy` data.
- `claudine/docs/schemas/partials/stack.yaml` — `stack:` is a type name in the
  draft schema set owned by `2026-10-21-claudine-schemas`; out of scope.
- `claudine/lib/src/harness/report.rs:202` — quoted literal data in a report test.
- `claudine/docs/research/hooks/*.md` other than `_fleet.md` — provider-hook
  tables; excluded by the spec.
- `darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.md` — a dated
  snapshot used as Darkmatter compose input; historical data.

Buckets to migrate (owners below):

- **Root prompts (24):** `_add/add-context-variables`, `_add/add-expressions`,
  `_clarify/findings`, `_implement/implement-plan`, `_implement/implement-review`,
  `_implement/implement-suggestions`, `_pr/{diagnose,dirty,fix,open,push,triage}`,
  `_prompt`, `_reviews/{cross-platform,dry,feature-review}`, `clarify`, `commit`,
  `format`, `implement`, `merge-conflicts`, `plan`, `pr`, `review`. Matches the
  spec's list. `_prompt.md` and `_implement/implement-plan.md` also read `_loop_*`.
- **Research Markdown:** `claudine/docs/research/_TEMPLATE.md` and all 22
  `claudine/docs/research/*/_fleet.md` (acp, agent-cli, agent-errors,
  agent-logging, agent-models, agent-permissions, hooks, local_runners, mcp,
  memory, model-config, non-interactive-sessions, plugins, reasoning-level,
  resume, signals, skills, slash-commands, steering, subagents, system-prompt,
  usage); every one holds `stack:`. `hooks/_fleet.md` is a fleet and is
  migrated; only the other `hooks/*.md` provider tables are excluded.
- **Claudine docs:** `getting-started/index.md`, `topics/composition.md`,
  `topics/flow-control/{lifecycle,flow-control,flow-control-reference,looping,sequences}.md`,
  `topics/frontmatter-properties.md`, `topics/pre-flight-checks.md`,
  `topics/signal-handling.md`, `topics/state-management/side-effects.md`.
  `composition.md`, `pre-flight-checks.md`, and `signal-handling.md` are not in
  the spec's AC13 list but carry old grammar and must be swept too.
- **Skills:** `.claude/skills/claudine/SKILL.md:10` (`say_first`). Other skill
  pages need a structural sweep for `stack:` examples in Phase 4.
- **Frozen/fixture Markdown:** `claudine/cli/tests/fixtures/frozen_prompts/prompts/{_implement/implement-plan,clarify,format,implement,plan,review}.md`,
  `claudine/cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md`,
  `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md` (hash-pinned),
  `claudine/cli/tests/fixtures/nested_span_regression/commit.md`,
  `claudine/lib/tests/fixtures/frozen_prompts/prompts/{_reviews/cross-platform,clarify,review}.md`,
  `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md`,
  `darkmatter/lib/tests/fixtures/frozen_prompts/prompts/_reviews/feature-review.md`
  (Darkmatter compose input; migrate so the frozen copy matches its source).
- **Darkmatter schema extension:** `darkmatter/docs/schemas/claudine.yaml` (5),
  `darkmatter/docs/schemas/claudine-types.yaml` (6).
- **Claudine Rust source:** `lifecycle/{parse,mod,executor,audio}.rs`,
  `error/mod.rs`, `error/render/mod.rs`, `reserved.rs`,
  `looping/{expression,types,engine,config}.rs`, `prepare/bootstrap/tests.rs`,
  `prepare/tests.rs`, `sequence/task/{group,tests}.rs`,
  `cli/src/commands/compose/loop_run.rs` (comment),
  `cli/src/commands/wrap/harness_orch/prompt/tests.rs` (`- action:` fixture).
  Lifecycle unit tests: `lifecycle/tests/{audio_emission,diagnostics,nested_span,parse_config}.rs`;
  loop tests: `looping/engine/tests/{gate_ambient,iteration_actions,post_checked_timing,seed_state}.rs`,
  `looping/{actions,config}/tests.rs`.
- **Claudine CLI tests:** 24 `cli/tests/l1` files (largest:
  `compose_initialize_acceptance` 39, `characterization_error_routes` 37,
  `lifecycle_literal_escapes` 19, `compose_initialize_staged_boot` 16,
  `handoff_owners` 11), `cli/tests/l1/error_guards/source_scan.rs`,
  `cli/tests/common/{incomplete_subagents,terminal_interrupt}.rs`, and 9
  `cli/tests/level2` files (`level2_lifecycle_control` alone has 339 hits;
  `level2_lifecycle_loop` 36, `level2_typed_error_render_capture` 18,
  `level2_lifecycle_action_forms` 17).
- **Darkmatter Rust:** `dmls/src/diagnostics/nested_span.rs:40` (`say_first`
  in the structural field list), `lib/src/markdown/compose/expression/parser.rs:2141`,
  `lib/tests/l1/dasherized_identifier_compose.rs:77-78`,
  `lib/tests/prompts/dasherized_identifier_corpus.rs:435`. These use
  `_loop_count` as a dasherized-identifier sample; rename the sample (for
  example `_iter_count`) rather than teach Darkmatter a `loop` root.

Not found by the regex but in scope (structural search in Phase 4):

- The old **structured discriminator long form** `{action: verb, …}` is live in
  `lifecycle/parse.rs:1038` (`parse_stack_item_action_object` →
  `parse_long_form_action_object`) and the variadic error text in
  `error/mod.rs` names it. It is removed grammar and gets the removed-grammar
  diagnostic with a `verb: { … }` replacement hint.
- Communication long-form aliases `message`/`sound` and the dead `route`
  parameter: `lifecycle/action_shape.rs:421-431`.
- Root-dictionary notification fields (`start: {message: …}`) stay legal: they
  are exactly the new root-dictionary bundle. Only `say_first` and `stack`
  inside them are removed.

### Consumer trace

| Concern | Production sites |
| --- | --- |
| `LifecycleStackItem` | `lifecycle/{actions,action_shape,parse,executor,mod}.rs`, `composition/mod.rs` (re-export), `sequence/task/mod.rs` |
| `LifecycleNotification` / notification fields / `loop_concerns` | `lifecycle/{mod,parse,action_shape,audio,executor}.rs`, `looping/config.rs` (`LIFECYCLE_CONCERN_KEYS` filtering, lines 270-281); tests `lifecycle/tests/{nested_span,audio_emission,parse_config,action_shape_control}.rs` |
| `audio_phases` / `say_first` | `lifecycle/{audio,executor,mod}.rs`, `lifecycle/tests/audio_emission.rs` |
| Expression surfaces and source map | `lifecycle/source_map.rs` (`LifecycleSurfacePath`, `LifecycleSourceMap`), `lifecycle/validate.rs` (`iter_stack_expression_surfaces`, nested spans, `err` availability, shell collection) |
| Shell approval | `lifecycle/validate.rs::collect_lifecycle_shell_commands{,_for}` ← `composition/preflight.rs`; `parse.rs::runs_a_shell` (initialize prohibition) |
| `StackControl` | defined `lifecycle/executor.rs:96`; consumed by `lifecycle/{control,runtime}.rs`, `looping/engine.rs`, CLI `wrap/harness_orch/loop_control{,/control_dispatch,/lifecycle_events,/proxy}.rs`, `wrap/composition/{mod,pipeline,preflight}.rs` |
| `ControlDispatch` | `lifecycle/{control,runtime}.rs`, CLI `loop_control{,/control_dispatch}.rs`; tests `lib/tests/l1/{lifecycle_control_flow_spike,agent_errors_fleet}.rs` |
| Proxy overlay transport | `StackControl::Proxy.overlay` → `control_dispatch.rs:195-215` (`EvaluatedProxyRequest`) → `HarnessPromptState.overlay` → `harness_orch/prompt.rs::load_overlaid_source`/`load_overlaid_document` (fresh read + `merge_frontmatter_overlay`) and `prompt.rs:221` `proxy_overlay`; compose path `cli/src/commands/compose/prep.rs` (`proxy_overlay` threaded through prep) → `lib/src/composition/prepare.rs:94,382` → `runtime_state.rs::LayeredOverrides::proxy_overlay`; `wrap/composition/staged_boot.rs:325` |
| Loop values | seeded by `looping/types.rs::insert_ambient_overrides` (called from `looping/types.rs:86` and `looping/engine.rs:603`), resolved by `looping/expression.rs:247-251`, named in `reserved.rs::AMBIENT_VARIABLE_NAMES` and `composition/types.rs:172-175` (`NAMES`), excluded in `looping/config.rs:256`; comments in `looping/engine.rs:50,279,598`, `cli/.../loop_run.rs:103`, `cli/.../prep.rs:1732`, `error/render/mod.rs:214` |
| Removed-key scan precedent | `parse.rs::scan_removed_validation_keys` ← `composition/prepare.rs`; CLI boundary test `cli/tests/l1/compose_removed_validation_keys.rs` |
| Errors to rename/add | `error/mod.rs` (`LifecycleProxyWith{NotMapping,WholeMapping,DynamicKey,EvaluationFailed}`, `LifecycleProxyOnlyParameter`, `LifecycleSayConflict`), `error/render/{mod,lifecycle}.rs`, `error/tests.rs`, CLI `loop_control/error_routing.rs` and its tests `unowned_handoff.rs`, `recovery_identity.rs`; `lib/tests/l1/diagnostic_detail_conformance.rs` |
| AC9 forbidden reference today | `parse.rs:297` `playa::SoundEffect::from_name` and the doc comment at `parse.rs:68` |

### Frozen contracts

These are binding for Phases 2–5. Later phases change them only by recording
the departure here.

1. **Node model** (`lifecycle/actions.rs`, lift-ready):
   `pub enum LifecycleNode { Action(LifecycleAction), Conditional(LifecycleConditional) }`;
   `pub struct LifecycleConditional { pub when: Expr, pub then: Vec<LifecycleNode>, pub otherwise: Vec<LifecycleNode>, pub path: LifecyclePath }`
   (`otherwise` empty means no `else`; the parser has already rejected an
   explicit empty `else`). `LifecycleAction` gains `pub path: LifecyclePath`
   (the authored path of the verb key). A stack is `Vec<LifecycleNode>`.
   `LifecycleStackItem` and `LifecycleNotification` are deleted;
   `LifecycleConfig` keeps only `stacks: LifecycleStacks`, whose fields become
   `Vec<LifecycleNode>` (empty = omitted/null/`[]`/`{}`; no `Option`).
   Task `setup`/`teardown` use the same `Vec<LifecycleNode>`.
2. **Dictionary canonicalization** (`parse.rs`): a bundle expands to leaf
   actions in this fixed order: `set`; `stdout`, `stderr`, `info`, `warn`,
   `success`, `message`, `notify`, `effect`, `say`/`speak`; `shell`; side-effect
   and expression-function verbs by ascending ASCII canonical name; the one
   control directive. A guarded list item becomes one `Conditional` whose
   `then` is the expansion. Canonical order replaces authored mapping order, so
   lifecycle parsing stops consulting `AuthoredOrder`/`mapping_orders` for
   bundle order (other `AuthoredOrder` users in `sequence/` are untouched).
3. **Depth accounting:** a `when` in a stack root list is depth 1; a `when`
   inside a `then`/`else` body is parent depth + 1; siblings share depth.
   Depth 6 → `LifecycleStackInvalidShape` with the authored path of the sixth
   `when`. Bundles containing `when` count the same as `then` items.
4. **Effect-catalog injection:** `parse.rs` exposes
   `parse_lifecycle_config_with(frontmatter, source_file, &LifecycleParseOptions)`
   where `LifecycleParseOptions { is_known_effect: &dyn Fn(&str) -> bool, authored: Option<&Frontmatter> }`
   (task stacks take the same options). The public
   `parse_lifecycle_config(frontmatter, source_file)` moves to
   `lifecycle/mod.rs` (not lift-ready) and supplies a predicate defined in
   `lifecycle/audio.rs`: `playa::SoundEffect::from_name(name).is_some()`. Call
   sites and `LifecycleUnknownEffect` timing (parse time, literal names only;
   `{{ }}` names deferred to event time) are unchanged.
5. **Authored path:** promote `source_map.rs::LifecycleSurfacePath` to the
   shared `pub struct LifecyclePath` (segments `Field`, `MapKey`, `Index`;
   lift-ready). `Display` renders `failure[0].else[1].shell.command`;
   `MapKey` segments that are not identifier-safe render bracket-quoted
   (`with["a.b"]`), matching the existing safe-segment rule of
   `LifecycleProxyWithDynamicKey`. Expansion records the authored key
   (`speak` stays `speak`), never a synthetic execution index. Errors keep the
   existing `property: String` field and fill it with the rendered path of the
   offending node; the post-hoc "stack annotator" (`parse.rs` around 700-760)
   that rewrote `property` to `{event}.stack[i]` is deleted.
6. **Walker API:** one recursive walker in `validate.rs`,
   `walk_stack(&[LifecycleNode], &mut impl FnMut(&LifecyclePath, Surface<'_>))`,
   visits every expression surface in both branches, `when` guards, `set`
   values, and `retry.with`/`proxy.with` values. Nested-span checks,
   `err`-availability checks, binding validation, `_loop_*` detection, and
   shell collection all use it. DMLS mirrors the same path spelling.
7. **Host binding `loop`:** declared in `lifecycle/bindings.rs` like `group`
   (lexical). Inside a loop (the loop `gate` scope and document events of a
   loop iteration) it is `Available`; known absence is
   `Unavailable("claudine.outside-loop")` (new constant `OUTSIDE_LOOP`); unknown
   membership during passive checks is `ExecutionDependent`. The runtime value
   is the object `{count, is_first, is_last, last_output, last_exit_code}`
   supplied as a `RuntimeBinding`; `looping/expression.rs` resolves `loop`,
   `loop.<member>`, and `loop['<member>']` for loop conditions. A missing member
   reads `null` from the object and never falls back to frontmatter.
   Composition of a loop iteration declares the same global for body
   templates. Plain `md compose` gains nothing. `insert_ambient_overrides`,
   `AMBIENT_VARIABLE_NAMES`, and `AmbientVariables::NAMES` are deleted.
8. **Retry overlay transport:** rename the overlay model for sharing —
   `ProxyWith` → `WithOverlay`, `ProxyWithValue` → `WithValue`,
   `ProxyWithError` → `WithOverlayError`. `LifecycleControlAction::Retry` gains
   `with: Option<WithOverlay>`; `StackControl::Retry` gains
   `overlay: IndexMap<String, Value>` (resolved data, empty when omitted),
   evaluated once by the executor with the same code path as `proxy.with`.
   `ControlDispatch::Retry` is unchanged; the wrapper reads the overlay from
   the `StackControl` (as `control_dispatch.rs` already does for proxy) and
   stores it as `HarnessPromptState.pending_retry_overlay: Option<IndexMap<…>>`.
   `load_overlaid_source` merges `state.overlay` (retained proxy overlay) first
   and then `pending_retry_overlay.take()` key-wise on top, so retry wins and is
   consumed by exactly one fresh read. The compose (pre-launch re-entry) path
   passes the same merged map through the existing `proxy_overlay` option.
   `resume` never sets it. An overlay evaluation failure raises before any
   `ControlDispatch`, so no transition and no budget is spent.

### Chosen diagnostic names

- **Removed grammar:** `CompositionError::RemovedGrammar { source_path: PathBuf, path: String, removed: String, replacement: String }`.
  One variant for every removed spelling: `stack`, `- action:`/`action` beside
  `when`, the `{action: verb}` discriminator map, `say_first`, communication
  `message`/`sound`/`route` parameters, and reads of the five `_loop_*` names
  (frontmatter expression or body span). `path` is the authored path (for a
  body span, `body` plus the span's line). It is not `Lifecycle*`-prefixed
  because `_loop_*` reads in a document body are not lifecycle grammar.
- **Shared `with` family:** `LifecycleWithNotMapping`,
  `LifecycleWithWholeMapping`, `LifecycleWithDynamicKey`,
  `LifecycleWithEvaluationFailed` (adds `verb: String`; `target` stays and names
  the document the overlay is built for — the proxy target, or the current
  document for `retry`), and `LifecycleWithUnsupported { source_path, property, verb }`
  replacing `LifecycleProxyOnlyParameter` (`with` is the only parameter it ever
  carried).
- **Loop-block shapes:** mixing loose verbs with `gate:` and `when` at the loop
  root reuse `LifecycleStackInvalidShape` (no new name).
- `LifecycleSayConflict` keeps its name; its meaning becomes `say` and `speak`
  in one dictionary.
- `LifecycleUnknownEffect`, `LifecycleActionOrder` (now list-local),
  `LifecycleMultipleLifecycleActions` keep their names.

### File ownership

| Owner | Files |
| --- | --- |
| Integration (Wave 2 + merges) | `lifecycle/{actions,action_shape,signatures,mod}.rs`, `composition/error/**`, `composition/mod.rs` re-exports, consolidated test registrations (`lib/tests/l1/main.rs`, `cli/tests/l1/main.rs`, `cli/tests/prompts/main.rs`), this log |
| Parser/source-map/validator (Wave 3) | `lifecycle/{parse,source_map,validate}.rs`, `lifecycle/tests/**`, `looping/config.rs` (concern extraction), `sequence/task/**` |
| Binding/loop/expression (Wave 3) | `lifecycle/bindings.rs`, `lifecycle/context*`, `looping/{expression,engine,types,seed}.rs` and `looping/engine/tests/**`, `reserved.rs`, `composition/types.rs`, `cli/src/commands/compose/{loop_run,prep}.rs` loop parts, Darkmatter `_loop_count` identifier samples |
| Executor/audio (Wave 4) | `lifecycle/{executor,audio,runtime}.rs`, `lifecycle/executor/tests/**` |
| Darkmatter schema/DMLS (Wave 4) | `darkmatter/docs/schemas/claudine{,-types}.yaml`, `darkmatter/dmls/src/diagnostics/nested_span{.rs,/**}`, `darkmatter/dmls/src/overlay/frontmatter.rs`, `darkmatter/dmls/tests/fixtures/sequence_descent/**` |
| Retry overlay (Wave 5) | `lifecycle/control.rs`, CLI `wrap/harness_orch/**`, `wrap/composition/**`, `compose/prep.rs` overlay parts |
| Prompts / fixtures / docs (Wave 6) | as split in the plan |

### Planned AC-to-test locations

| AC | Where the evidence lands |
| --- | --- |
| AC1, AC2, AC3, AC4, AC17 (parser) | `lifecycle/tests/parse_config.rs`, `lifecycle/tests/action_shape_control.rs`, new `lifecycle/tests/grammar.rs`; task stacks in `sequence/task/tests.rs` |
| AC4, AC17 (executor) | `lifecycle/executor/tests/{conditions_control,action_dispatch,mutation_visibility}.rs` |
| AC5 | parse: `lifecycle/tests/action_shape_control.rs`; evaluation: `executor/tests/proxy_with_evaluation.rs`; transitions: CLI `loop_control/tests/{retry_resume,proxy,active_state_wiring}.rs` |
| AC6 | `lifecycle/tests/validation.rs`; CLI boundary: new `cli/tests/l1/compose_removed_grammar.rs` modeled on `compose_removed_validation_keys.rs` (`compose`, `inline-compose`, `sequence`) |
| AC7 | `looping/engine/tests/gate_ambient.rs` (migrated), `looping/expression.rs` tests, `lifecycle/bindings/tests.rs`, `cli/tests/l1/loop_gate_ambient.rs` |
| AC8 | `lifecycle/tests/audio_emission.rs` (fake publisher, both orders, survival across transition); spool boundary via `test_toolkit::LockedAudioSpool` as in `lib/tests/l1/tts_phase5_contract.rs` |
| AC9 | new `claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs` reading the six files via `include_str!` |
| AC10 | opt-in `cli/tests/prompts/shipped_prompts.rs`; default `cli/tests/l1/shipped_prompts.rs` over frozen fixtures |
| AC11 | `cli/tests/prompts/shipped_prompt_route_drift.rs` |
| AC12 | `darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs` (holds `lifecycle_inventory_matches_the_authored_claudine_schema`), `dmls/src/overlay/frontmatter.rs` tests, `dmls/tests/l1/mapping_only_corpus.rs` |
| AC13–AC16 | docs facet lint, skill grep, final inventory in Phase 5 |

### Phase 1 verification

- **Pre-existing failure found and fixed.** The first baseline `just test`
  failed one test unrelated to this feature:
  `claudine-cli::l1 compose_schema_cli::compose_frozen_clarify_reads_its_ordinary_doc_property`.
  The frozen `clarify.md` (a verbatim copy of `prompts/clarify.md`, added in
  `5c5d243be`) ends its specification line with `{{doc}}. ` — a trailing
  space — and the test, added in `6e9e4c4f0`, trimmed only the start of each
  line before `ends_with("demo-spec.md.")`. It failed deterministically on
  this branch before any Phase 1 edit. Fix: the test's line matcher now uses
  `str::trim` (`claudine/cli/tests/l1/compose_schema_cli.rs:1289-1296`). The
  fixture is left byte-identical to its source prompt.
- `just test` in `claudine/` (macOS): **8740 run, 8740 passed, 9 skipped**
  (the recipe's standing skips; none added here).
- `just lint` in `claudine/`: passed.
- No other OS run: the only code edit is a string-trim in one L1 test, with no
  platform-dependent behavior.

### Checkpoint

Prerequisite available; inventory, ownership, diagnostic names, and contracts
recorded above; no design ruling blocks Wave 2.
