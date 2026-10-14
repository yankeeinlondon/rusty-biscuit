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
message_to_agent: |-
    Phase 1 notes for the next phase:
    - The Preparer seam lives in claudine/lib/src/composition/lifecycle/prewarm/
      (preparer.rs, fakes.rs). Tests use FakePreparer::{recording, blocking}
      with failing_on/release/wait_for_calls/completed. The module has a
      temporary #![allow(dead_code, unused_imports, reason = ...)]; remove it
      once the worker (Phase 4) uses the seam.
    - R6 is amended in the plan: TtsConfig derives PartialEq + Serialize but not
      Hash/Eq, so key on (resolved text, serialized TtsConfig).
    - staged_boot::lifecycle_settings and pipeline::construct_lifecycle_runtime
      fall back to GlobalSettings::default() when config.json fails to load; the
      new tts_prewarm field (R1) needs a deliberate default there (true, matching
      the auto-detect playback config used in that case).
    - CliProcessFixture does not isolate TMPDIR/TMP/TEMP or BISCUIT_SPEAKS_CACHE
      (recorded under Phase 6 "Fixture gaps").
    - The seam map (all six hook points, worker ownership sites) is in the
      implementation log under Phase 1.
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
