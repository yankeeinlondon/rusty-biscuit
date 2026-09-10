---
created: 2026-09-10
status: proposed
spec: ./spec.md
---

# Implementation Plan: Silent Audio Tests

Implement [the spec](./spec.md) across Claudine, biscuit-speaks, and Playa.
Success means that automated tests retain their synthesis, playback, and queue
coverage without audible output, misleading speech, or delayed audio escaping
fixture cleanup. This plan does not implement the fix.

## Design and verified constraints

Use native Playa playback and effective first-class TTS volume controls,
alongside test configuration and fixture isolation. Keep default configuration
values and production lifecycle announcements unchanged. Repairing provider
paths that ignore requested volume is explicitly in scope. Tests may
select an engine or voice different from the user's Claudine configuration;
make that selection explicit instead of inheriting personal settings.

Source inspection establishes several constraints on the implementation:

- `biscuit-speaks/lib/src/playa_bridge.rs` carries `config.volume` into
  `PlaybackOptions` and detached file jobs. Use `VolumeLevel::Explicit(0.0)`
  for real speech playback, and verify its passage to the actual backend.
- Claudine's library and CLI already enable Playa `native-playback`, and
  biscuit-speaks' `playa` feature enables it transitively. Its CLI enables
  `playa`. Verify the actual test/build feature graph retains this wiring.
- Direct speech is different: the current `say` path has no volume flag,
  and both foreground and detached eSpeak command construction omit volume.
  Assigning zero in `TtsConfig` alone does not mute those commands. Repair these
  gaps in the existing public volume contract, using controlled executables to
  verify command construction before exercising real providers.
- `playa/lib/src/playback.rs` ignores volume for some host players, including
  mpg123 and ogg123. Native-first playback can fall back to host programs.
  A zero-volume request is therefore insufficient without controlling the
  reachable fallback routes or using intrinsically silent source audio.
- `enqueue_state_at` in `playa/lib/src/detached/mod.rs` returns without spawning
  a scheduler when `worker.lock` is already held. The spec's suspected teardown
  race is not established merely by Rust drop order: trace existing workers,
  helpers, and subsequent publishers before deciding what needs repair.
- The background CLI fixture already limits `PATH` to fake Kokoro/mpv programs
  and produces invalid audio bytes. Preserve those properties while checking
  cleanup; do not replace fake bytes with an audible file.

Paths below are relative to the repository root.

## Phase 1: Finish the inventory and establish safe boundaries

- [ ] Inspect test call sites, inline test modules, process fixtures, and tier
  recipes in all three areas. Use GitNexus query/context to trace speech,
  effect, composition, and detached-worker flows; use targeted source searches
  for fixtures and text literals. Consult the September 4 audio changes.
- [ ] Record each affected test's tier, provider/player, source text or audio,
  effective volume, fallback routes, spool/cache isolation, child lifetime,
  and whether it performs real playback or only constructs/inspects requests.
  Keep this inventory and subsequent evidence in this fix directory.
- [ ] Trace whether any test loads or executes
  `prompts/_implement/implement-suggestions.md` or related real prompt templates.
  If found, isolate its lifecycle emission in the test fixture. Do not edit
  the production announcement. If no caller is found, retain the exact
  reported phrase as unresolved rather than attributing it to another test.
- [ ] Classify paths as real playback needing mute, already controlled
  construction/execution, or incomplete containment needing repair. Expand
  outside the three areas only for demonstrated callers of the same defect.
- [ ] Before modifying any existing symbol, run upstream GitNexus impact and
  report direct callers, affected processes, and risk. Surface HIGH/CRITICAL
  results before edits. Refresh a stale index when necessary.

**Exit criterion:** Every identified execution path has an explicit silence
strategy. Do not run currently audible tests to establish a baseline; use source
inspection and controlled recording fixtures first.

## Phase 2: Honor TTS volume and mute real playback

- [ ] Audit `Speak::with_volume`, `TtsConfig::with_volume`, and `VolumeLevel`
  through each supported provider's foreground, cache, and detached paths.
  Retain this existing first-class API and define zero as mute, with the
  existing normalized range and clamping semantics. Verify representative
  intermediate levels as well as zero; muting alone is not volume support.
- [ ] Verify native volume is applied before submitting audio and remains
  effective for normal audio and sound-effect routing. Preserve native feature
  wiring in affected consumers and test recipes. Inspect optional non-Playa
  builds too: an unsupported volume request must not be silently ignored.
- [ ] Make eSpeak honor configured volume in foreground and detached command
  construction. Verify the provider's authoritative amplitude semantics before
  selecting the conversion from normalized volume; keep both paths consistent.
- [ ] For `say`, investigate synthesis to a file followed by native Playa
  playback using the selected voice/rate and requested volume. Prefer the
  existing file/preparation pipeline over a separate playback implementation.
  Preserve detached ordering, cleanup, and return-after-publication semantics
  if direct command jobs become prepared file jobs. Verify platform behavior
  before fixing the exact synthesis format or flags in code.
- [ ] Audit SAPI and other concrete providers for effective volume support,
  including detached serialization. Use an existing provider-native control
  where it honors the contract; otherwise use synthesis plus controlled Playa
  playback. Do not switch the user's provider/voice just to obtain volume.
- [ ] Make fallback preserve explicit volume. If a host player cannot honor
  it, exclude that player from that request or return an explicit unsupported
  error when no valid route exists. Do not silently play at default volume.
  Apply upstream impact analysis before changing shared selection behavior,
  and document any intentional change to unsupported-request handling.

- [ ] In `biscuit-speaks/lib/tests/real_detached_phase4.rs`, change the spoken
  text to “This is a test message.” and set explicit zero volume. Make the
  test's name describe its pinned Kokoro provider and native completion
  contract. Use GitNexus rename for the existing symbol and update references.
- [ ] Keep Kokoro selection deterministic and retain native-route and complete
  verdict assertions. Prevent the real test from reaching an uncontrolled
  fallback player: use the existing discovery/fixture facilities to expose
  only verified volume-capable players and required synthesis dependencies.
  Use `sniff` for executable discovery. Prove that excluded players cannot be
  reached, including after native failure; do not rely on a preflight
  availability check alone.
- [ ] If existing fixture controls cannot constrain a real route safely,
  separate synthesis-to-file from playback and exercise completion with
  verified zero samples, while retaining explicit coverage of the speech
  configuration and handoff. Record any resulting coverage change. Do not
  quietly replace the entire real test with dry-run or a skip.
  This testing fallback does not satisfy the separate requirement to repair
  a production provider that ignores the first-class volume setting.
- [ ] Replace work-status text in the five files listed in the spec:
  `biscuit-speaks/lib/tests/detached_phase4.rs`,
  `biscuit-speaks/cli/tests/detached_background.rs`,
  `playa/lib/src/detached/tests.rs`,
  `claudine/lib/src/composition/lifecycle/tests/audio_emission.rs`, and
  `claudine/lib/src/dispatch/runner/tests.rs`.
  Preserve the test-message phrase in Unicode/quoting fixtures as well as
  their existing special characters and exact argument assertions.
- [ ] Update direct-command expectations where effective volume adds arguments
  or moves speech to file preparation. Replace the obsolete command-shape
  assertion with equivalent lossless-text, voice/rate, volume, and ordered
  handoff coverage; do not freeze the volume bug into expected arguments.
- [ ] Update cache keys and expected payloads consistently with fixture text.
  Preserve serialization compatibility corpora and nonzero-volume assertions
  when they are safely isolated and test that specific contract.
- [ ] Apply zero volume to other confirmed real speech/effect tests from the
  inventory. Retain Playa's existing zero-filled PCM completion tests.

**Exit criterion:** Real tests have effective mute through every reachable
route; speech fixtures are recognizable as tests, with their original
behavioral coverage intact.

## Phase 3: Contain detached jobs through completion and cleanup

- [ ] Set explicit effect volume `0.0` in the configuration assembled by
  `claudine/cli/tests/detached_audio.rs`. Assert the persisted job retains it
  alongside the existing durable state and sequence assertions.
- [ ] In Claudine dispatch publication coverage, use zero effect volume unless
  a nonzero value is essential to the assertion. Keep speech pinned to its
  fixture executable; zero in a speech config is not a replacement for that
  executable boundary.
- [ ] For lifecycle publication coverage, retain the real default emitter and
  queue inspection. There is no lifecycle effect-volume field in the inspected
  path. Establish non-execution through the private spool and held worker lock,
  then remove executable pending work before releasing ownership. Do not add
  a public lifecycle volume setting solely to accommodate this test.
- [ ] Apply cleanup consistently to publication tests that leave ready or
  preparing jobs. Use a small test-local RAII fixture where repeated ownership
  ordering requires it. Remove pending payloads while protected, close handles
  before deleting their directory on Windows, and preserve queue lock order.
  Avoid a workspace-wide abstraction unless the inventory demonstrates a need.
- [ ] For tests that intentionally start helpers or delegates, retain process
  ownership or observable completion signals. On success and unwinding,
  release blocked fixture programs, wait with a bounded deadline for exit,
  and terminate/reap only fixture-owned children if needed before removing
  their spool. Merely touching a release marker does not prove completion.
- [ ] Keep invalid-spool failure tests failing before playback and preserve
  their warning assertions. Prevent host provider/player fallback even on
  fixture failure. Do not clean or cancel the user's real queue or workers.

**Exit criterion:** Publication tests cannot execute their queued content, and
worker-execution tests leave no runnable work after success or failure.

## Phase 4: Add focused regression evidence

Extend existing fixtures and tests instead of duplicating implementation details.

| Boundary | Required evidence |
|----------|-------------------|
| Speech config → file job | Exact zero survives ready cache jobs, preparation configuration, and the resulting ready job |
| Public TTS volume → provider/native output | Zero and intermediate levels reach the effective output control in foreground and detached paths; voice selection is preserved |
| Parent → detached helper/delegate | Record child arguments or job options and verify mute survives process boundaries |
| Native → supported host fallback | Controlled native failure selects a fixture player whose captured command requests zero volume |
| Unsupported-volume route | A controlled backend proves explicit volume is honored by rerouting or rejected; silent fixtures keep negative tests safe |
| Direct streaming speech | Controlled programs prove effective volume arguments or the synthesis-to-Playa handoff, including on error |
| Publication teardown | No executable pending work remains when worker ownership is released |
| Active helper teardown | Completion and simulated assertion-failure/unwind paths leave no live fixture worker or playable queued job |

- [ ] Use deterministic markers, recorded commands, and job-state assertions
  rather than sleeps as the primary proof. Retain bounded timeouts for failure.
- [ ] Demonstrate that regression assertions detect omitted mute or incomplete
  cleanup using controlled fixtures only; never remove mute from a real
  playback run to prove a test fails.
- [ ] Recheck original timing, order, durable-publication, completion-report,
  and handoff-warning assertions. Do not loosen them to make cleanup pass.

**Exit criterion:** Evidence covers effective silence and process lifetime,
not merely a builder's numeric setting or the absence of audible sound.

## Phase 5: Document and validate across platforms

- [ ] Update relevant testing documentation with the zero-volume rule, explicit
  test-message wording, and intentional provider/voice selection. Document
  pinned Kokoro coverage so it cannot be mistaken for a check of the user's
  normal Claudine voice. Update area skills if fixture workflows change.
- [ ] Review comments on every changed symbol; correct drift encountered in
  those symbols without unrelated comment cleanup. Update READMEs if public
  behavior changes and dependency documentation only if dependencies change.
- [ ] Run focused safe tests first, then `just test` and `just lint` in each
  affected area. Use the area's argument forwarding and nextest filters as
  defined by its recipes; check feature gates so the edited targets actually run.
- [ ] Run applicable Claudine `just test-l2` coverage using the headless harness
  without focusing terminal/browser windows. Playa and biscuit-speaks currently
  declare L2 inapplicable; their Rust integration-test file layout alone does
  not make those tests L2.
- [ ] After effective mute is verified, run the relevant Playa and biscuit-speaks
  `just test-real` coverage. Record selected provider/player, route, completion
  verdict, and skips. A skipped real test is not playback evidence.
- [ ] Follow `.claude/skills/os/SKILL.md` and its build-host guidance. Discover
  available `BUILD_*` hosts at execution time and use `just cross-check <pkg>
  --os linux`, `--os windows`, and `--os wsl` for affected packages as supported
  by the recipe. Verify selected tiers/features rather than assuming the
  cross-check command runs every real-resource target.
- [ ] Record separate macOS, Linux, native Windows, and WSL2 results. Exercise
  WSL2 archive behavior and Windows executable/handle cleanup. Distinguish
  compile checks, controlled behavioral checks, and real audio-backend evidence.
  Record unavailable provisioning or audio backends as unmet evidence, not passes.
- [ ] Review the final diff and map evidence to every spec acceptance criterion.
  Keep exact-phrase attribution unresolved if still unproven. Mark implementation
  complete and move to `_completed` only after required work and verification
  are satisfied. Do not run `cargo fmt` or commit without explicit instruction;
  if a commit is later requested, run `detect_changes()` before committing.

## Completion record

To be filled during implementation: changed tests and containment decisions,
commands and results, platform/backend matrix, remaining limitations, and the
acceptance-criterion checklist. No implementation or runtime validation has been
performed as part of authoring this plan.
