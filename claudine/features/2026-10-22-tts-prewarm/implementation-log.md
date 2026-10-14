---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-22-tts-prewarm/spec.md"
plan: "claudine/features/2026-10-22-tts-prewarm/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/preparer.rs
    - claudine/lib/src/composition/lifecycle/prewarm/fakes.rs
    - claudine/lib/src/composition/lifecycle/prewarm/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
source_files_during_phase_2:
    - claudine/lib/src/config/tts.rs
    - claudine/lib/src/config/claudine_config/tests.rs
    - claudine/lib/src/events/config.rs
    - claudine/lib/src/dispatch/loader.rs
    - claudine/lib/src/dispatch/loader/tests.rs
    - claudine/lib/src/dispatch/runner/speak.rs
    - claudine/lib/src/dispatch/runner/tests.rs
    - claudine/lib/src/dispatch/tests.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/tts_settings.rs
    - claudine/cli/src/commands/config_tui/reducers.rs
    - claudine/cli/src/commands/config_tui/tabs/tts.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/prewarm/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/collect.rs
    - claudine/lib/src/composition/lifecycle/prewarm/collect/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/key.rs
    - claudine/lib/src/composition/lifecycle/prewarm/key/tests.rs
    - claudine/cli/tests/l1/composition_seams.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/lib/src/composition/lifecycle/prewarm/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify.rs
    - claudine/lib/src/composition/lifecycle/prewarm/worker.rs
    - claudine/lib/src/composition/lifecycle/prewarm/worker/tests.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/tests/l1/context_construction_guard.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/mod.rs
    - claudine/lib/src/composition/lifecycle/prewarm/worker.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify.rs
    - claudine/lib/src/composition/lifecycle/prewarm/fixture.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/prewarm/tests.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/tests/l1/context_construction_guard.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/tts_prewarm.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
human_review: false
message_to_agent: |-
    Phase 5 notes for Phase 6 (docs, skill, audit):
    - Behavior to document: one deferred worker per top-level command
      (compose, inline-compose, sequence, provider wrapper). It starts on the
      first submission that speaks, using that document's lifecycle
      settings, so the first speaking document's `tts` config and `prewarm`
      decide for the whole invocation. Submission points: the bootstrap
      before `initialize` (staged and loop), every prepared document before
      launch (stabilized reread, loop iteration, proxy target, sequence step
      or task), harness retry/resume/adoption, and sequence preflight before
      step one.
    - Limit to state plainly: a lifecycle mapping `set` in `initialize` (or
      any event) changes runtime state, not the reread's frontmatter. A
      phrase reading a `set` key is pre-warmed with its earlier text and
      misses the cache at event time (never wrong audio). A document that
      rewrites itself (`set_frontmatter`) is re-read and warmed with the new
      text.
    - In a sequence, the per-step overlay keys (state, previous, next,
      outputs, sequence_id), the caller's setters, and a prompt task's
      `params` are late. Preflight also leaves late any frontmatter value
      still holding `{{`, `$(`, or a backtick, plus `ctx`/`env` reads.
    - The CLI test seam is CLAUDINE_TEST_PREWARM=record:<path> or
      block:<path> (test-fixtures only). The SKILL.md now has a short
      pre-warm wiring paragraph; add the `tts` keys and shape table there.
    - Fixture gaps (Phase 6 task): the criterion-10 test sets
      TMPDIR/TMP/TEMP and BISCUIT_SPEAKS_CACHE on its own builder.
      CliProcessFixture still does not set them by default; decide whether to
      move them into the fixture.
---
# Implementation Log for 2026-10-22-tts-prewarm (6 phases)

## Phase 1

### Rulings (R1–R9)

All nine rulings are accepted as written, with one amendment:

- **R6 amended.** `biscuit_speaks::TtsConfig` derives `Debug, Clone, Default,
  PartialEq, Serialize, Deserialize` but not `Hash`/`Eq`. The de-duplication
  key will be (resolved text, the config's serialized form) — that covers every
  field automatically instead of a hand-picked provider/voice/speed subset, so
  a field added to `TtsConfig` later cannot silently collapse two candidates.
  The plan's "test that changes each field" still applies. Recorded in the plan.
- **R9 confirmed.** `2026-10-18-fm-namespace` and `2026-10-19-speak-prepare` are
  completed; `2026-10-20-lifecycle-grammar` is implemented (`LifecycleNode`,
  `LifecycleConditional`, `LifecyclePath` in `lib/src/composition/lifecycle/actions.rs`).
  Phase 3 is not blocked.
- **R4 note.** There is no dedicated "captured env" type. The document's
  captured binding is the prepared `ComposeContext` (`compose_context.env()`,
  with `env_overrides` inserted at `staged_boot.rs:128` / `compose/prep.rs:1143`)
  over `input_layers.file_resolution_context.env()`. `PrewarmInputs` should be
  built from those two, never from `std::env`.

### Seam map

Paths relative to `claudine/`. Data types: `PreparedComposition`
(`lib/src/composition/types.rs:649`: `effective_frontmatter`, `lifecycle:
LifecycleConfig`, `input_layers.file_resolution_context`, `compose_context`,
`document_epoch`), `BootstrapPreparation` (`lib/src/composition/prepare/bootstrap.rs:31`,
same fields without the prompt), `LifecycleConfig` (`lifecycle/mod.rs:236`).

| Hook point | Call site | Existing single snapshot function? |
| --- | --- | --- |
| Bootstrap | `cli/src/commands/wrap/composition/staged_boot.rs::bootstrap_document` (after `prepare_bootstrap`); harness proxy adoption `harness_orch/prompt.rs::bootstrap_harness_prompt` | yes: lib `prepare_bootstrap` (`lib/src/composition/prepare/service.rs:174`) serves both |
| Stabilized reread | `cli/src/commands/compose/prep.rs::run_staged_single` (result of `prepare_collecting_missing`, before `guard.set_config`); loop route iteration 1 in the same file | no: `prepare_collecting_missing` (`prep.rs:1790`) is the funnel; introduce a snapshot call there |
| Sequence preflight | `cli/src/commands/wrap/sequence/mod.rs::execute_sequence`, right after `graph.validate_prompt_lifecycle_literals()`; lib `PreflightGraph::validate_prompt_lifecycle_literals` (`lib/src/composition/sequence/preflight/mod.rs:122`) already parses each prompt's `LifecycleConfig` | no: add a per-document iterator next to it. Task `setup:`/`teardown:` are raw `Value` stacks in preflight (`preflight/shape.rs`) and must be parsed for collection |
| Proxy handoff | compose path: the target is prepared on the next pass of the handoff loop in `compose/prep.rs::run_composition_inner`, which reaches the bootstrap and reread hooks with `entry = ProxyTarget` (overlay already applied); harness path: `harness_orch/prompt.rs::materialize_harness_prompt` applies `active_overlay` before its `prepare_document` | yes: covered by bootstrap/reread (compose) and `materialize_harness_prompt` (harness) |
| Retry / resume | `harness_orch/prompt.rs::materialize_harness_prompt` (consumes `pending_retry_overlay`), reached from `loop_control.rs::materialize_attempt_prompt_phase` | yes: `materialize_harness_prompt` serves first attempt, retry, resume, proxy target, and harness loop refresh |
| Loop refresh | compose closure `compose/prep.rs::build_and_run_loop`: iterations ≥ 2 converge after `kind.prepare_without_schema`, before `resolve_shell_approvals(.., &prepared.lifecycle)` | no: match arms converge there; introduce a snapshot call |

**Worker ownership.** `CompositionPrepContext` is rebuilt per active document,
so it is the wrong owner. One guard per command belongs at:
`compose/prep.rs::run_composition_inner` (compose and inline-compose, outlives
the handoff loop), `commands/sequence.rs::run_sequence` → `wrap/sequence/mod.rs::execute_sequence`
(sequence prompt steps reuse `prepare_and_run_active_document`), and
`wrap/mod.rs::run_provider_wrapper_inner` / `runner.rs` (harness loop).
`InvocationContext` (`lib/src/invocation_context.rs`, `Arc` inner shared by
every clone across handoffs and steps) is the natural carrier for the handle.

**Settings.** `GlobalSettings` for lifecycle runs come from
`staged_boot.rs::lifecycle_settings` and `wrap/composition/pipeline.rs::construct_lifecycle_runtime`.
Both return `GlobalSettings::default()` when the lifecycle is empty (no
phrases, so harmless) **or when `config.json` fails to load** — Phase 4/5 must
decide pre-warm's state on a config load failure (default `true` per R1 would
follow the playback config, which is auto-detect in that case).

### Recorder seam

New module `lib/src/composition/lifecycle/prewarm/`:

- `preparer.rs`: crate-private `trait Preparer: Send + Sync` with
  `fn prepare(&self, text: String, config: TtsConfig) -> PrepareFuture`
  (`Pin<Box<dyn Future<Output = Result<(), PrewarmError>> + Send>>`). It is
  dyn-compatible (the worker can hold `Arc<dyn Preparer>`), and dropping the
  future is cancellation, matching biscuit-speaks' publish-after-synthesis
  cache. `SpeakPreparer` is the production impl
  (`Speak::new(text).with_config(cfg).prepare()`, builder discarded).
  `Speak::prepare`'s future is `Send`, verified by compilation.
- `PrewarmError::Prepare` carries no provider error text, so a warning cannot
  leak a phrase, secret, or raw response.
- `fakes.rs` (`#[cfg(test)]`): `FakePreparer::{recording, blocking}` plus
  `failing_on(text)`, `release(n)`, `calls()`, `texts()`, `completed()`,
  `wait_for_calls(n)`. Calls are recorded at invocation; completion only when
  the future runs to the end, so a cancelled preparation is observable.
- Module carries `#![allow(dead_code, unused_imports, reason = …)]` until
  Phase 4 wires the worker; **remove it in Phase 4**.

### Fixture check

`CliProcessFixture` (`cli/tests/common/mod.rs`) sets `PLAYA_DRY_RUN=1` and a
fixture-local `PLAYA_SPOOL_DIR` and exposes `audio_spool()`. **Gap:** it does
not set `TMPDIR`/`TMP`/`TEMP` (the audio cache is `std::env::temp_dir()`,
`biscuit-speaks/lib/src/audio_cache.rs:142`) or `BISCUIT_SPEAKS_CACHE`; the
default builder inherits the host's values. Added under Phase 6's "Fixture
gaps" task in the plan.

### Tests (requirement → test)

| Requirement | Test (`claudine` lib, L1) |
| --- | --- |
| Recorder observes text and config | `composition::lifecycle::prewarm::tests::recording_fake_records_text_and_config_and_succeeds` |
| Controllable blocking future | `…::blocking_fake_records_the_call_and_holds_completion_until_released` |
| Cancellation by drop is observable | `…::dropping_a_blocked_preparation_never_completes_it` |
| Failing fake fails only named text | `…::failing_fake_fails_only_the_named_text` |

Behavior of production code is unchanged in this phase (the seam is not yet
called), so there is no regression test to add.

## Phase 2

### What changed

- `lib/src/config/tts.rs`: `TtsConfigSettings.provider` is `Option<String>`
  and a new `prewarm: Option<bool>` is added. Both use a `present`
  deserializer so an explicit `null` is rejected instead of collapsing into
  absent (the `#[serde(default)]` on both is justified by that). Both are
  skipped on serialize when `None`, so a load/save round trip never writes a
  default into the user's file. `deny_unknown_fields` is kept.
  `TtsValue::prewarm_enabled()` is the one place that reads the switch: only an
  object with `prewarm: false` turns it off. `TtsValue` docs rewritten to say
  the value governs hook speech and lifecycle playback config, never whether
  lifecycle `say` plays.
- `lib/src/events/config.rs`: `GlobalSettings.tts_prewarm: bool` (R1). The
  derived `Default` became a manual impl so the default is `true`; that covers
  the two config-load-failure fallbacks Phase 1 flagged.
- `lib/src/dispatch/loader.rs::bridge_tts_settings`: passes an absent provider
  through as `None` (same as `true`) and sets `tts_prewarm`.
- `lib/src/dispatch/runner/speak.rs`: hook `speak` reads the optional provider;
  a provider-less object gets no `SpecificProvider` failover, exactly as `true`.
- `cli/src/commands/config_tui/`: new pure reducer
  `apply_tts_provider_selection` carries `gender` and `prewarm` across a
  provider change (voice still resets). A provider-less object shows and lists
  voices for the best available provider (what `true` uses); picking a voice
  pins that provider so the voice is not sent to a different default. Gender
  and voice edits mutate in place, so they keep `prewarm`. The `T` toggle to
  `false` still drops the object, which restores pre-warm's default. A note
  under the toggle says it controls hook speech only and that lifecycle `say`
  and its pre-warming are not affected. No new toggle.
- Other consumers (`init_wizard.rs`, `init/mod.rs`, `config/mod.rs`): no
  fallout; they only build `TtsValue::Boolean`.

### Departure: two existing assertions changed mechanically

Criterion 7 says existing `tts` tests pass "without their assertions being
edited". Two assertions compared the old `String` provider directly
(`assert_eq!(settings.provider, "say")` in `config/tts.rs` and
`config/claudine_config/tests.rs`). With `Option<String>` (R2) they cannot
compile as written; they now read `assert_eq!(..provider.as_deref(),
Some("say"))`, which asserts the same value. Every other existing test changed
only in struct-literal setup (`provider: Some(..)`, `prewarm: None`).

### Duplicate keys

`config.json` is parsed by `json_five` into a `serde_json::Value`, whose map
is last-wins. That is the loader's existing behavior for every key; the matrix
asserts it for `tts`, `provider`, and `prewarm` and records it as the format's
definition (plan Phase 2 matrix, "duplicate key" row).

### Input robustness matrix: state of every cell

Asserted in `dispatch::loader::tests::tts_config_input_robustness_matrix`
through `load_claudine_config` (a real JSON5 file) then `bridge_tts_settings`.

| Shape | `tts` | `provider` | `prewarm` |
| --- | --- | --- | --- |
| control `{ provider: "kokoro", voice: "af_heart" }` | kokoro, pre-warm on | | |
| absent | no hook settings, pre-warm on | `None`, voice kept, pre-warm on | on (control) |
| explicit null | rejected | rejected | rejected |
| wrong type | `"say"` rejected | `1` rejected | `"no"` rejected |
| one element / every element | n/a (scalars) | n/a | n/a |
| empty | `{}` loads, provider `None`, pre-warm on | `""` loads as before (runtime unknown-provider fallback) | n/a |
| duplicate key | last-wins | last-wins | last-wins |
| trailing content | rejected | n/a | n/a |
| unknown key | `enabled`, `pre_warm` rejected | | |

A mutation check (removing the `present` deserializer) made the
`provider null` cell fail, so the null cells are load-bearing.

### Requirement → test

| Requirement | Test |
| --- | --- |
| Matrix (all cells above) | `claudine dispatch::loader::tests::tts_config_input_robustness_matrix` |
| Shape table: pre-warm and bridged settings per row | `claudine dispatch::loader::tests::tts_shapes_bridge_to_the_documented_prewarm_and_playback_settings` |
| Repeated read/write/read keeps `{ prewarm: false }`, never materializes `prewarm` or `provider` | `claudine dispatch::loader::tests::tts_object_round_trips_without_materializing_defaults` |
| Hook `speak`: provider-less object behaves exactly as `true` | `claudine dispatch::runner::speak::tests::provider_less_object_drives_hook_speak_exactly_as_true` |
| Hook `speak`: `prewarm` never changes an object's playback | `…::prewarm_never_changes_hook_speak_for_an_object` |
| Hook `speak`: provider-less object keeps its voice | `…::provider_less_object_keeps_its_voice_with_the_default_provider` |
| Hook `speak`: `false`/absent stay off | `…::false_and_absent_keep_hook_speech_off` |
| Lifecycle `say` config: absent/`true`/`false` auto-detect, pre-warm on | `claudine composition::lifecycle::tests::tts_settings::every_boolean_shape_plays_auto_detect_with_prewarm_on` |
| Lifecycle `say` config: provider-less object as `true`, own `prewarm` | `…::provider_less_object_plays_as_true_and_applies_its_prewarm` |
| Lifecycle `say` config: `prewarm: false` object plays as without it | `…::prewarm_false_object_plays_as_the_same_object_without_it` |
| Config-load fallback keeps pre-warm on | `…::default_settings_keep_prewarm_on` |
| TUI provider change keeps `prewarm`/gender, resets voice | `claudine-cli commands::config_tui::reducers::tests::provider_selection_keeps_prewarm_and_gender_and_resets_voice`, `…::provider_selection_from_a_provider_less_object_keeps_prewarm`, `…::provider_selection_from_a_boolean_starts_with_prewarm_unset` |
| TUI gender edits keep `prewarm` | `claudine-cli commands::config_tui::tabs::tts::tests::gender_edits_keep_prewarm` |
| TUI voice edit keeps `prewarm` | `…::voice_selection_keeps_prewarm` |
| TUI switch to boolean restores the default | `…::switching_to_a_boolean_restores_the_prewarm_default` |
| TUI edit survives save and reload | `…::edited_prewarm_survives_save_and_reload` |
| TUI explains the toggle's scope | `…::tab_explains_the_toggle_covers_hook_speech_only` |
| Existing `tts` tests unchanged in meaning | all pre-existing `config::tts::tests::*`, `dispatch::tests::bridge_settings_*`, `dispatch::runner::speak::tests::*` pass |

The TUI provider modal's `Enter` reads the host's installed TTS providers, so
its preservation is tested through the pure reducer it calls rather than
through a key event.

### Gates

- `just test` in `claudine/`: 9086 passed, 9 skipped (pre-existing skips).
- `just lint` in `claudine/`: clean (only the existing macOS linker
  `__eh_frame` notice).
- No new dependency, CI cell, `cfg(windows)`, or path-comparison code; all new
  tests are L1, in already-declared targets (in-crate `#[cfg(test)]` modules),
  and use temp dirs, so no cross-OS rig run was needed.
- Smell grep over touched config code: the two `#[serde(default)]` are paired
  with `present` (null rejected); `unwrap_or(true)` in `prewarm_enabled` is the
  documented absent default; no `unwrap_or_default`/`.ok()` on a load-bearing
  parse.

## Phase 3

### What changed

New pure functions under `lib/src/composition/lifecycle/prewarm/`. Nothing calls
them yet; there is no I/O and no worker.

- `collect.rs`: `lifecycle_phrases(&LifecycleConfig)` walks every stack except
  `initialize` (in `LifecycleSignal::ALL` order, `loop:` loose keys or `gate:`
  included) and `stack_phrases(scope, nodes)` walks one parsed stack (a task's
  `setup:`/`teardown:`). Both reuse `actions::for_each_action`, so both
  `then`/`else` bodies are visited at every depth and no guard is evaluated.
  Root bundles, the `speak` alias, and positional/named speech arguments are
  plain `CommunicationAction`s after parsing, so they need no special case.
  Each `Phrase` carries its `LifecycleScope`, authored `LifecyclePath`, and
  `&Expr`.
- `classify.rs`: `classify(scope, &Expr, &PrewarmInputs) -> Classification`
  (`Early(text)` or `Skip(SkipReason)`).
  - The authored text is recovered (a template or literal body is a
    `StringLiteral`; a whole-value phrase is rebuilt as `{{ <Display> }}` and
    used only when it parses back to the identical `Expr`), then prepared with
    Darkmatter's `prepare_value(.., AuthoredMode::InterpolatedValue)`.
  - `$(` or a backtick anywhere in the authored text: `ShellExpansion`.
  - Any `FunctionCall` node in any branch: `FunctionCall` (R5, syntactic).
  - Roots from `static_variable_reads`, classified by the scope's lifecycle
    `BindingView` (`runtime_bindings_enclosed(scope, default, Pending)`):
    `fm` early; `ctx`/`env` early; any other reserved namespace (`current`)
    or declared global (`err`, `timing`, `loop`, `group`, `outputs`) is
    `LateRoot`; a bare document root is implicit frontmatter.
  - With sequence-injected keys present: a bare, `fm.key`, or `fm['key']` read
    of one is `SequenceInjected`; a computed `fm[...]` index or a whole-`fm`
    read (including `(fm).x`) is `TaskDependentRead`.
  - Resolution uses playback's own evaluation (below), then
    `scalar_string`; empty or whitespace-only text is `Empty`; any error is a
    silent `ResolutionFailed`.
- `key.rs`: `PrewarmKey { text, config }`, where `config` is the
  `TtsConfig`'s serialized form (R6 as amended in Phase 1).
- `executor.rs`: playback's operand evaluation was extracted into free
  functions `evaluate_operand_in`, `is_template_operand`, and the private
  `evaluate_expr_in`/`interpolate_text_in`. The executor's `eval_expr`,
  `resolve_string_value`, and `evaluate_operand` now delegate to them, so
  playback and pre-warm interpolate with one implementation. Playback
  behavior is unchanged (full suite green).

### Departures from the plan

- **R4 `PrewarmInputs` shape.** The plan names `{ fm, ctx, env,
  sequence_injected_keys }`. `ctx` and `env` both live in one Darkmatter
  `ComposeContext`, so the type is `{ frontmatter, context: DocumentContext,
  sequence_injected_keys }`, where `DocumentContext` is `Captured(ctx)` or
  `Pending(ctx)`. `Pending` is the spec's "preflight cannot establish the
  eventual snapshot" case: a phrase reading `ctx`/`env` is skipped
  (`ContextUnavailable`) and fm-only phrases still resolve. An `Option` was
  tried first and dropped: `ComposeContext` has no pure constructor, and the
  only fallback (`capture_minimal`) reads the process environment, which the
  spec forbids. The carried context in `Pending` only satisfies the
  evaluator and is never read.
- **Empty-text suppression.** Lifecycle playback has no empty-text check of
  its own (`emit_speech` receives `""`). The spec's rule "empty results are
  not submitted" is therefore implemented in pre-warm only, as empty or
  whitespace-only after `scalar_string`. Playback is unchanged.
- **Guard baseline rename.** `cli/tests/l1/composition_seams.rs`
  `SUBTREE_COMPOSE_BASELINE` named `lifecycle::executor::resolve_string_value`
  as the single `SubtreeCompose::new` owner. The call moved into
  `interpolate_text_in` (still exactly one site), so the entry and the
  guard's message now name `interpolate_text_in`, and the reason says
  pre-warm shares it. This is a rename, not a second interpolator.

### Requirement → test

All L1, in-crate `#[cfg(test)]` modules of the `claudine` lib (already
declared targets; no tier markers).

| Requirement | Test (`composition::lifecycle::prewarm::…`) |
| --- | --- |
| C1: every stack except `initialize`, event order; root bundle, `speak`, named and positional args, `gate:` | `collect::tests::every_stack_but_initialize_is_walked_in_event_order` |
| C1: loose `loop:` verbs | `collect::tests::loose_loop_verbs_are_walked` |
| C1: `then`, `else`, depth-3 nesting, guarded bundle; guards not evaluated | `collect::tests::both_branches_are_walked_at_every_depth_without_evaluating_guards` |
| C1: only `say`/`speak` collected | `collect::tests::only_speech_is_collected` |
| Spec: never scan arbitrary frontmatter for `say` | `collect::tests::frontmatter_keys_named_say_are_not_phrases` |
| Task `setup:`/`teardown:` walked in their scope | `collect::tests::task_setup_and_teardown_stacks_are_walked_in_their_own_scope` |
| C2: `fm`, bare, bracket, `fm.err`, `ctx`, `env` early with resolved text | `classify::tests::literal_and_frontmatter_ctx_env_phrases_are_early_with_playback_text` |
| C3: operators, indexing, fallback, ternary over early values | `classify::tests::operators_indexing_fallback_and_ternary_over_early_values_are_early` |
| C2: `err`, `timing`, `current`, `loop`, `group` late, incl. untaken branch / fallback / index | `classify::tests::late_roots_are_skipped_even_in_untaken_branches` |
| C3: any call, zero-arg `date()`, shell-probing `has_alias`, untaken branch | `classify::tests::function_calls_are_ineligible_wherever_they_appear` |
| C3: authored `$( … )`/backtick ineligible; no command runs (marker file absent) | `classify::tests::authored_shell_expansion_is_ineligible_and_never_runs` |
| C3: data returned by an interpolation is not re-evaluated | `classify::tests::data_returned_by_an_interpolation_is_never_evaluated_again` |
| C3: empty results not submitted | `classify::tests::empty_results_are_not_submitted` |
| Speculative resolution failure is a silent skip | `classify::tests::a_failed_speculative_resolution_is_a_silent_skip` |
| Unrecoverable whole-value text is skipped, not misread | `classify::tests::a_whole_value_phrase_whose_text_cannot_be_recovered_is_skipped` |
| C2/C12: sequence-injected keys late via bare, dotted, bracket; computed index / whole `fm` skipped | `classify::tests::sequence_injected_keys_are_late_through_bare_dotted_and_bracket_reads` |
| R4: `ctx`/`env` wait for the document's capture | `classify::tests::ctx_and_env_wait_for_the_documents_own_capture` |
| Task scopes classify against their own binding view | `classify::tests::task_stacks_classify_against_their_own_scope` |
| C12 drift guard: every reserved namespace and declared global has a decided timing | `classify::tests::every_reserved_namespace_and_declared_global_has_a_decided_timing` |
| R6: equal inputs give equal keys | `key::tests::equal_text_and_config_give_equal_keys` |
| R6: different text separates | `key::tests::different_text_gives_a_different_key` |
| R6: each `TtsConfig` field separates (destructure fails to compile on a new field) | `key::tests::changing_any_config_field_gives_a_different_key` |

The Input Robustness Matrix does not apply: this phase reads no file
format or configuration.

### Gates

- `just test` in `claudine/`: 9108 passed, 9 skipped (the same pre-existing
  skips). The first run failed only `claudine-cli::l1
  composition_seams::subtree_compose_baseline_holds_the_line` (the rename
  above); green after the baseline update.
- `just lint` in `claudine/`: clean (only the existing macOS linker
  `__eh_frame` notice).
- No `cfg(windows)`, path comparison, new dependency, or CI cell; the one
  temp path in a test is never executed or compared, so no cross-OS rig run
  was needed.

## Phase 4

### What changed

- New `lib/src/composition/lifecycle/prewarm/worker.rs`:
  - `PrewarmWorker` is the invocation-owned guard. `start(&GlobalSettings,
    dry_run)` checks the gates, builds the candidate config with
    `tts_config_from_settings(settings.tts)` (the exact config lifecycle
    `say` plays with, R1), and starts one worker with `SpeakPreparer`.
    `start_gated` is the same with the preparer injected; `start_with` skips
    the gates (tests); `disabled()` is the zero-cost no-op.
  - `disabled_by(dry_run, prewarm_setting, playa_dry_run)` is the pure gate:
    `--dry-run`, then `prewarm: false`, then `PLAYA_DRY_RUN` in exactly
    Playa's spellings (`1`, `true`, `TRUE`, `yes`).
  - `PrewarmHandle` (cloneable) `submit_phrases(phrases, &PrewarmInputs)`
    returns at once: it gates before classifying, classifies on the caller's
    thread (no I/O), records each `PrewarmKey` when queued, and sends the
    resolved text over an unbounded channel. A key is never queued twice in
    one invocation, whether it is in progress, done, or failed.
  - The worker is a dedicated `std::thread` running a current-thread tokio
    runtime. `Speak::prepare` does provider discovery synchronously before
    its first await, so polling it on the launch runtime would occupy one of
    its threads; a separate thread keeps every TTS operation off it, and
    works whether or not the caller has a runtime. It inherits the caller's
    tracing dispatcher and span.
  - Serial: one `prepare` future at a time. The first failure logs one
    `warn` with no phrase, secret, or provider text; later failures are
    silent; later candidates continue. No fallback provider.
  - Drop sets a `watch` cancel flag; the worker drops its in-flight future
    (cancellation) and exits without draining the queue. Drop waits at most
    250 ms for that acknowledgement so the dependency's child/tempfile
    cleanup runs before process exit, but never for synthesis. Submissions
    after cancellation are no-ops.
- `prewarm/mod.rs`: registers `worker`; the module allow stays
  (`dead_code`, `unused_imports`) with a new reason, because nothing in
  production starts the worker until Phase 5.
- `prewarm/classify.rs`: removed an unused `is_template_operand` import that
  the module allow had been hiding since Phase 3 (drift found and fixed).
- `lib/tests/l1/context_construction_guard.rs`: one reasoned allowance for
  the `PLAYA_DRY_RUN` read in `worker.rs`.

### Departures from the plan

- **Thread, not `spawn_blocking`.** The plan says "a single serial task on the
  tokio runtime; blocking provider work goes through
  `spawn_blocking`/`std::thread::scope`". `Speak::prepare` is one future
  whose blocking discovery and async synthesis cannot be split from outside,
  so the whole worker runs on its own thread with its own runtime (the same
  idea as `enqueue_speech`'s thread, but long-lived). Same guarantee, one
  owner.
- **Classification is on the submitting side.** The plan's
  "records the dedup key at queue time" needs the resolved text, so
  `submit_phrases` classifies before queuing. Classification is pure and
  cheap; Phase 5 should still call it after the launch-critical work where
  convenient.
- **Bounded drop.** Shutdown does not drain or await synthesis, but it does
  wait up to 250 ms for the in-flight future's drop, so cancellation cleanup
  runs before the process exits.

### Requirement → test

All L1, in-crate `#[cfg(test)]` modules of the `claudine` lib (declared
targets, no tier markers).

| Requirement | Test |
| --- | --- |
| Gates, every accepted `PLAYA_DRY_RUN` spelling, unaccepted spellings, precedence | `composition::lifecycle::prewarm::worker::tests::every_gate_and_every_accepted_dry_run_spelling_disables_prewarm` |
| C6: `--dry-run` and `prewarm: false` prepare nothing, resolve nothing | `…::dry_run_and_prewarm_false_start_nothing_and_resolve_nothing` |
| C10 (recorder half): each accepted `PLAYA_DRY_RUN` value read from the environment starts nothing; control `0` runs | `…::every_accepted_playa_dry_run_spelling_starts_nothing` |
| Same config as playback (`tts_config_from_settings`) | `…::candidates_use_the_lifecycle_playback_configuration` |
| C9 (lib): submit returns with a blocked `prepare`; serial; late `err` phrase not queued | `…::submission_returns_while_preparation_is_blocked_and_runs_serially` |
| C12 (worker half): dedup at queue time, across handle clones, while in progress, after completion; changed text is new | `…::a_candidate_is_queued_once_per_invocation` |
| C8: one sanitized `warn`, later candidates run, failed not retried | `…::failures_warn_once_without_the_phrase_and_later_candidates_still_run` (mutation: removing the warn-once guard makes it fail) |
| C13: no-op success counts as done | `…::a_successful_or_no_op_preparation_counts_as_done` |
| C13: drop cancels in flight, does not drain, completed work survives, later submits ignored | `…::dropping_the_worker_cancels_without_draining_and_keeps_completed_work` |
| Disabled worker is a no-op | `…::a_disabled_worker_is_a_no_op` |
| C13: an event during a blocked prepare publishes normally; no join, no cancel | `composition::lifecycle::tests::audio_emission::speech_fired_during_a_blocked_prewarm_publishes_without_joining_it` |

No elapsed-time thresholds are asserted; blocking is controlled by the fake's
gate. The CLI halves of criteria 6, 9, and 10 belong to Phase 5's wiring
tests. The Input Robustness Matrix does not apply (no file format or
configuration is read in this phase).

### Gates

- `just test` in `claudine/`: 9119 passed, 9 skipped (the same pre-existing
  skips).
- `just lint` in `claudine/`: clean (only the existing macOS linker
  `__eh_frame` notice).
- Cross-OS: no paths, `cfg`, process spawning, or filesystem in the new
  code; one named `std::thread` and a current-thread runtime behave the
  same on every OS. No rig run was needed.


## Phase 5

### What changed

Every invocation flow now submits early phrases to one command-owned worker.

- **Deferred worker** (`prewarm/worker.rs`). `PrewarmWorker::deferred(dry_run)`
  is the production constructor. It starts no thread and reads no setting
  until the first submission that carries phrases. That submission's
  `GlobalSettings` (the lifecycle settings the document plays speech with)
  decide the gates and the candidate configuration, so the first speaking
  document wins for the whole invocation. `--dry-run` turns it off at
  construction. `PrewarmWorker`, `PrewarmHandle`, and `PrewarmInputs` (with
  `PrewarmInputs::captured`) are now `pub`, and `lifecycle::prewarm` is a
  public module. `start_gated`, `start_with`, and `submit_phrases` are
  test-only. Phase 4's module-level `allow(dead_code, unused_imports)` is
  removed, along with the re-exports nothing used.
- **Scoped injected keys.** `PrewarmHandle::with_injected_keys` returns a
  clone that unions extra sequence-injected keys into every submission. Its
  queue and de-duplication are shared, so differently scoped tasks still
  dedup invocation-wide.
- **Entry points** (`prewarm/mod.rs`, `prewarm/sequence.rs`).
  - `collect_and_submit(handle, settings, lifecycle, || inputs)` is the one
    entry for a prepared document, and the future `prep` directive's. Inputs
    are built lazily, only when the handle is live and the lifecycle speaks.
  - `submit_sequence_preflight(handle, settings, graph, sequence, caller_keys,
    context)` walks the preflight graph:
    - each task's `setup:`/`teardown:` (parsed with `parse_task_action_stack`),
      against the sequence document's frontmatter;
    - each prompt document a task reaches, judged per task, so the task's
      `params` keys are late for that task only;
    - the sequence document's own lifecycle, when a step runs its body.
  - Late during preflight: `ROOT_OVERLAY_KEYS`, the caller's `--set` keys,
    every top-level frontmatter value still holding `{{`, `$(`, or a backtick
    (resolving it would mean evaluating it), and `ctx`/`env` (`Pending`).
  - Nothing is composed, fetched, or run.
- **Carrier** (`invocation_context.rs`). `InvocationContext` has a `prewarm`
  handle (default: disabled). It is set with `with_prewarm`, narrowed with
  `with_prewarm_injected_keys`, and copied by `enter_document`, so every
  document view shares it. `DocumentEpoch::prewarm()` reaches it from an
  epoch.
- **Ownership.** `compose/prep.rs::run_composition_inner` (compose and
  inline-compose), `commands/sequence.rs::run_sequence_inner`, and
  `wrap/mod.rs::run_provider_wrapper_inner` each create one deferred worker
  and install its handle on the invocation. Dropping it on return cancels
  unfinished preparation (R7).
- **Submission sites:**
  - `wrap/composition/pipeline.rs::construct_lifecycle_runtime`: every
    prepared document before its launch (eager read, staged stabilized
    reread, each loop iteration, a compose-path `proxy` target, each sequence
    step and prompt task). Inputs are the effective frontmatter and the exact
    lifecycle context, with env overrides applied.
  - `staged_boot.rs::StagedLifecycleRuntime::new`: the staged bootstrap
    walk, before `initialize`.
  - `compose/prep.rs::build_and_run_loop`: the loop's bootstrap walk over
    `initialize_frontmatter`, before the engine fires `initialize`.
  - `harness_orch/loop_control.rs::prewarm_materialized`: every fresh
    harness read (retry, resume, adopted target's stabilized reread, harness
    loop refresh) and an adopted target's bootstrap before its `initialize`.
    It uses the guard's settings and the materialized frontmatter, context,
    and env overrides.
  - `wrap/sequence/mod.rs::submit_preflight_prewarm`: called right after
    `validate_prompt_lifecycle_literals`, before any step. It loads settings
    only when the handle is live.
  - `wrap/sequence/iterate.rs` scopes each step's view with
    `ROOT_OVERLAY_KEYS` plus the step's caller-layer keys.
    `wrap/sequence/task_run.rs` scopes a prompt task's view with its `params`
    keys.
- **Test seam** (`prewarm/fixture.rs`, `test-fixtures` only).
  `CLAUDINE_TEST_PREWARM=record:<path>` swaps the preparer for one that
  appends each text to `<path>`; `block:<path>` appends and never finishes.
  Gates still apply. It has a `context_construction_guard` allowance and is
  gated for `test_seam_gate_guard`.

### Departures and findings

- **Lazy start instead of start-at-entry.** The plan said "create the worker
  once per top-level invocation". It is created there, but it starts on first
  use. The settings that decide the gates and the candidate configuration are
  the lifecycle settings each flow already loads, and those only exist once a
  document is read. A command whose documents never speak starts no thread
  and reads no extra config.
- **Sequence-injected keys come from the handle, not the call site.** A
  site does not know whether it runs inside a sequence step, a prompt task,
  or a proxy inside either. The sequence scopes the invocation view instead,
  and every site inherits it.
- **Lifecycle `set` in `initialize` is not a new snapshot.** A mapping `set`
  writes runtime state that event-time playback reads, not the stabilized
  reread's effective frontmatter. A phrase reading such a key is pre-warmed
  with its pre-`set` text and misses the cache at event time: the spec's
  documented `set` limit, never wrong audio. The CLI reread test uses
  `set_frontmatter`, which rewrites the document. Phase 6's docs should
  state this limit plainly.
- **`outputs` outside a sequence is an ordinary absent property.** It is
  not a declared global in a plain document's event scope. Its read resolves
  as missing, the same as playback. Inside a sequence it is a root overlay
  key and therefore late.
- **The proxy target is covered twice.** On the compose path an
  `initialize` proxy target is prepared by both the pipeline site and the
  harness adoption site. A mutation that turned off either one alone left the
  proxy test green; de-duplication makes the second submission a no-op.

### Requirement → test

L1 throughout. The lib tests are in-crate `#[cfg(test)]` modules. The CLI
tests are `cli/tests/l1/tts_prewarm.rs`, declared in `l1/main.rs` under
`#[cfg(all(unix, feature = "test-fixtures"))]` (Unix-only like
`launch_recorder`; `test-fixtures` is a `local-features` and CI feature).

| Requirement | Test |
| --- | --- |
| Deferred worker starts on the first speaking submission; silent documents build no inputs | `claudine composition::lifecycle::prewarm::tests::entry::a_deferred_worker_starts_with_the_first_submission_that_speaks` |
| C6 (lib): `prewarm: false` from the first speaking document prepares nothing, and cannot be re-enabled | `…::entry::the_first_speaking_submission_decides_the_gates` |
| C6 (lib): `--dry-run` off before any submission, resolves nothing | `…::entry::dry_run_turns_a_deferred_worker_off_before_any_submission` |
| C12: bootstrap and reread share one worker; retry / loop refresh / proxy resubmission dedup | `…::entry::bootstrap_and_reread_share_one_worker_without_repeating_a_candidate` |
| C2/C12: sequence-injected keys late via bare, dotted, bracket; computed index skipped; `loop`/`err` late; unscoped control | `…::entry::sequence_injected_keys_are_late_in_every_document_their_handle_submits` |
| C12: parallel tasks with different scopes share dedup | `…::entry::differently_scoped_tasks_share_one_de_duplication` |
| C4 (lib): three prompts, setup/teardown (untaken branch), body-step sequence lifecycle queued in one non-blocking call | `claudine composition::lifecycle::prewarm::sequence::tests::every_early_phrase_of_three_prompts_is_queued_before_any_step_runs` |
| C4/C12: per-task `params` late only for that task; unresolved template/shell values, `env`, `ctx`, overlay `state` left late; no shell runs | `…::sequence::tests::task_inputs_unresolved_values_and_captures_are_left_late` |
| Disabled handle walks nothing | `…::sequence::tests::a_disabled_handle_walks_nothing` |
| C1/C2 (CLI): compose prepares early phrases once with resolved text, skips `err`, walks an untaken `then` | `claudine-cli::l1 tts_prewarm::compose_prepares_every_early_phrase_once_with_its_resolved_text` |
| C9/C8: blocked `prepare` does not delay launch, exit success, nothing on the terminal | `…::a_blocked_preparation_does_not_delay_the_provider_launch` |
| C6 (CLI): `--dry-run` prepares nothing | `…::dry_run_prepares_nothing` |
| C6 (CLI): `config.json` `tts: { prewarm: false }` prepares nothing and still launches | `…::prewarm_false_in_config_prepares_nothing_and_still_launches` |
| C10: frozen shipped `clarify.md` (`start` says) under the fixture's `PLAYA_DRY_RUN=1` leaves no `biscuit-speaks-*` file in fixture `TMPDIR`/`TMP`/`TEMP`, no `BISCUIT_SPEAKS_CACHE` file, no spool | `…::playa_dry_run_prepares_nothing_and_leaves_no_audio_artifacts` |
| C10: each accepted spelling (`1`, `true`, `TRUE`, `yes`) keeps the recorder untouched | `…::every_accepted_playa_dry_run_spelling_prepares_nothing` |
| C4 (CLI): three-prompt sequence; with a blocking recorder the only started preparation is step one's preflight-only `setup:` phrase, and all three steps launch; with a recording one every phrase is prepared | `…::a_three_prompt_sequence_submits_every_document_before_step_one` |
| C5: `proxy` target prepared with its `with` overlay, never the authored value | `…::a_proxy_target_is_prepared_with_its_overlay` |
| C12: retry with `retry.with` prepares the new text; unchanged phrase not repeated | `…::a_retry_prepares_its_new_snapshot_without_repeating_a_candidate` |
| C12: `initialize` rewrites the document; reread finds the new text, unchanged phrase not repeated | `…::initialize_changes_are_discovered_by_the_reread_without_repeating_a_candidate` |

**Mutation checks.** Each wiring site was disabled in turn and the CLI suite
was run, then the site was restored and the restore confirmed by diff.

| Site disabled | Tests that failed |
| --- | --- |
| Pipeline | compose, blocked, staged, retry |
| Harness | retry |
| Sequence preflight | sequence |
| Staged bootstrap | staged |

The Input Robustness Matrix does not apply: this phase reads no new file
format or configuration field. `config.json` is read through the Phase 2
loader unchanged.

### Gates

- `just test` in `claudine/`: 9138 passed, 9 skipped (the same pre-existing
  skips). The first run failed only `context_construction_guard` on the new
  seam's `std::env::var`; it is green after the reasoned allowance.
- `just lint` in `claudine/`: clean apart from the existing macOS linker
  `__eh_frame` notice.
- `just test-l2` not run: the detached-playback boundary was not touched
  (criterion 14).
- Cross-OS: no new `cfg(windows)`, path comparison, or process spawning in
  production code. The seam's `mode:path` split is on the first `:`, so a
  Windows drive path survives. The lib tests use temp dirs and are portable.
  The CLI tests are Unix-only because their stub providers are shell scripts.
  The behavior they cover is also asserted portably at lib level, so no rig
  run was needed.
- New L1 test durations: CLI 0.6–2.4 s each (the slowest is the four-spelling
  loop), lib under 0.1 s.
