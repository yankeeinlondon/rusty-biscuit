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
human_review: false
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
message_to_agent: |-
    Phase 3 notes for the next phase:
    - Entry points (all crate-private, re-exported from
      composition::lifecycle::prewarm): lifecycle_phrases(&LifecycleConfig)
      and stack_phrases(scope, &[LifecycleNode]) return Vec<Phrase>
      { scope, path, text: &Expr }; classify(scope, &Expr, &PrewarmInputs)
      returns Classification::Early(resolved_text) or Skip(reason);
      PrewarmKey::new(&text, &TtsConfig) is the dedup key.
    - PrewarmInputs is { frontmatter: Map, context: DocumentContext,
      sequence_injected_keys: BTreeSet<String> }. DocumentContext::Captured(cc)
      is the document's own ComposeContext (prepared.compose_context with
      env_overrides applied); use Pending(cc) in sequence preflight, where any
      ctx/env phrase is skipped. Never build one with ComposeContext::capture*
      in production; it reads the process environment.
    - classify does no I/O and runs no function or shell, but it does build an
      EffectiveState and evaluate. It is cheap, but call it on the worker side
      or after the launch-critical path, per the spec's "never delay launch".
    - Playback and pre-warm share lifecycle::executor::evaluate_operand_in.
      The single SubtreeCompose owner is now interpolate_text_in (the guard
      baseline in cli/tests/l1/composition_seams.rs names it).
    - The prewarm module still carries #![allow(dead_code, unused_imports)];
      remove it once the worker uses these functions (Phase 4).
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
