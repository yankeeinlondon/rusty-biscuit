---
total_phases: 6
created: 2026-10-12
phase: 1
agent: claude/sonnet
yolo: true
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
---
# Plan: TTS Pre-warm

Source: [spec](./spec.md). Depends on `2026-10-19-speak-prepare`,
`2026-10-20-lifecycle-grammar`, and `2026-10-18-fm-namespace` (all upstream in
the pipeline).

## Summary and Definition of Done

Claudine submits the `say`/`speak` phrases it can resolve at prepare time to
`biscuit_speaks::Speak::prepare` on one invocation-owned background worker, so a
later lifecycle `say` finds a warm audio cache. The work has five parts:

1. **Config shape.** `tts` in `~/.claudine/config.json` gains an optional
   `prewarm` key and an optional `provider` (object form), carried through the
   bridge to the runtime without changing today's hook/lifecycle behavior.
2. **Candidate collection.** A walk over *parsed lifecycle actions* (not
   arbitrary frontmatter) that yields every `say`/`speak` phrase outside
   `initialize`, including `loop:`, `gate:`, `setup:`/`teardown:`, root bundles,
   named arguments, and `then`/`else` bodies at any depth.
3. **Early/late classification.** Using Darkmatter's `static_variable_reads` plus
   a separate function-call/shell inspection, decide whether each phrase is
   eligible, then resolve it speculatively with playback's own interpolation.
4. **Worker.** One serial, cancellable, de-duplicating worker per invocation,
   fed from the bootstrap, stabilized reread, sequence preflight, proxy/retry/
   loop re-entry. Never awaited by launch, never changes outcome or exit code.
5. **Docs and tests** for all of the above.

Done when all 14 acceptance criteria in the spec hold, `just test` and
`just lint` in `claudine/` are green (plus `just test-l2` only if the detached
playback boundary is touched), the `docs/` tree and the `claudine` skill
describe pre-warm, and no test touches an audio device, the network, a window,
or the user's real caches.

## Phase 1: Rulings, Seams, and Test Scaffolding

Pin down the open design seams, then build the injection points every later
phase needs.

### Necessary Rules

The spec says "Open questions: None", but these are places where it is silent
or two readings are possible. Each ruling below is the plan's default; the
author may overturn one before Phase 2 starts.

- [x] **R1. Where `prewarm` lives at runtime.** `bridge_tts_settings` maps
  `tts: false` to `GlobalSettings.tts = None`, so the enable bit cannot ride on
  `TtsSettings` (it does not exist for `false`/absent). *Ruling:* add a separate
  field (`GlobalSettings.tts_prewarm: bool`, default `true`) next to `tts`.
  `TtsSettings` and `tts_config_from_settings` are not changed, so playback
  config stays byte-identical. The pre-warm candidate config is built by calling
  the same `tts_config_from_settings` on the same bridged value.
- [x] **R2. Optional `provider` on `TtsConfigSettings`.** Make `provider:
  Option<String>`; `bridge_tts_settings` maps `None` to `TtsSettings.provider =
  None` (same as `true`). Hook `speak` (`dispatch/runner/speak.rs`) must treat a
  provider-less object exactly as `true`. Serializing must omit `provider` when
  `None` (round-trip stays minimal). `prewarm` is `Option<bool>` in the
  struct so a TUI round-trip does not materialize a default into the user's file;
  the effective default is `true`.
- [x] **R3. Untagged enum error quality.** `TtsValue` is `#[serde(untagged)]`,
  so an object with an unknown key (`enabled`) produces serde's generic "did not
  match any variant" message. *Ruling:* keep the shape; acceptance only requires
  rejection. Improving the message is out of scope unless a one-line custom
  deserialize is cheaper than the test churn.
- [x] **R4. The `fm` snapshot for speculative resolution.** The spec says
  "owned snapshot of the effective document inputs." *Ruling:* the collector
  takes an explicit `PrewarmInputs { fm, ctx, env, sequence_injected_keys }`
  value built by the caller; it never reaches into live runtime state. `env` and
  `ctx` come from the document's captured binding (per `2026-10-18-fm-namespace`);
  if that capture is not yet available at a call site, that site is skipped (not
  approximated from the live process environment).
- [x] **R5. Function-call detection is syntactic.** Walk the parsed expression
  AST for any call node (including in untaken branches) and the raw authored
  string for `$(`/backtick shell expansion. *Ruling:* a phrase with either is
  ineligible; no catalog lookup of "pure" functions (spec: first implementation
  skips all calls).
- [x] **R6. De-duplication key.** (resolved text, playback `TtsConfig`).
  `TtsConfig` may not implement `Hash`/`Eq`. *Ruling:* key on resolved text plus
  a canonical string of the config's provider/voice/speed fields, derived in one
  function next to the key type, covered by a test that changes each field.
  *Amended in Phase 1:* `biscuit_speaks::TtsConfig` derives `PartialEq` and
  `Serialize` (not `Hash`/`Eq`), so the canonical string is the config's
  serialized form, which covers every field (failover, gender, language,
  volume, …), not only provider/voice/speed. The per-field test still applies.
- [x] **R7. Shutdown trigger.** "Invocation finishes or is interrupted" maps to
  the existing top-level command scope and signal path
  (`docs/topics/signal-handling.md`). *Ruling:* the worker is owned by a guard
  that cancels on `Drop`, and the interrupt path already drops the command scope;
  no new signal handler.
- [x] **R8. Wider measurement (author decision, not scheduled).** The spec
  rules out a performance spike. If the author wants a measured hit-rate or
  cold-start figure for Kokoro, that is a separate request; this plan does not
  include it.
- [x] **R9. Dependency state.** Phases 2+ assume `Speak::prepare` (already present
  in `biscuit-speaks/lib/src/speak.rs`), the grammar feature's parsed lifecycle
  action types, and the `fm` namespace capture have landed. If the grammar types
  are not yet on this branch, Phase 3 blocks; Phases 1-2 do not.

### Spikes

None. The spec's own figures (0.2-3 s first-utterance latency) answer the only
performance question, and call recordings prove reuse. Reading
`static_variable_reads` and the lifecycle parse types is ordinary Phase 1 reading,
not a spike.

### Tasks

**Wave 1.1 (parallel)**

- [x] **Map seams.**
  - Read `lifecycle/audio.rs`, `executor.rs` (lines near the two
    `tts_config_from_settings` calls), `lifecycle/mod.rs`, `dispatch/loader.rs`
    `bridge_tts_settings`, `dispatch/runner/speak.rs`, and the call sites in
    `cli/src/commands/wrap/composition/{staged_boot,pipeline}.rs`.
  - Record in `implementation-log.md` the concrete hook points for: bootstrap,
    stabilized reread, sequence preflight, proxy handoff, retry/resume, loop
    refresh. Confirm each has (or can get) a single "new snapshot" function.
- [x] **Recorder seam.**
  - Define a crate-private `trait Preparer` (`async fn prepare(text, TtsConfig)
    -> Result<(), PrewarmError>`) with a production impl calling
    `Speak::new(text).with_config(cfg).prepare()` and discarding the builder.
  - Tests inject a recording / blocking / failing fake. Satisfies the "injected
    recorder" and "controllable fake futures" acceptance wording.
- [x] **Fixture check.** Confirm `CliProcessFixture` sets `PLAYA_DRY_RUN=1`,
  isolates `TMPDIR`/`TMP`/`TEMP` and `BISCUIT_SPEAKS_CACHE`, and exposes
  `audio_spool()`. List any gap as a task in Phase 6.

**Validation checkpoint 1:** the seam map exists in the implementation log, the
rulings are accepted or amended, and `just test` in `claudine/` is unchanged and
green.

## Phase 2: Configuration (`tts.prewarm`, optional `provider`)

Independent of the walk; lands first because later phases read the bit.

### Input Robustness Matrix

`config.json` (JSON5) is the only format. Load-bearing fields: `provider` (now
optional), `prewarm`, and the `tts` value itself. One test walks the matrix from
a real-shape fixture with one edit per cell, asserting through the public result
(`bridge_tts_settings` plus effective pre-warm), not the serde return value.

| Shape | `tts` | `provider` | `prewarm` |
| --- | --- | --- | --- |
| absent | off for hooks, pre-warm **on** | default provider | **on** |
| explicit null | rejected (not a bool or object) | rejected, not read as absent | rejected, not read as absent or `true` |
| wrong type, whole field | `"tts": "say"` rejected | `provider: 1` rejected | `prewarm: "no"` rejected |
| wrong type, one element | n/a (scalar) | n/a | n/a |
| wrong type, every element | n/a | n/a | n/a |
| empty | `{}` loads as default-provider object, pre-warm on | `""` rejected or unknown-provider warn as today (assert current behavior unchanged) | n/a (boolean) |
| duplicate key | JSON5 duplicate `prewarm`: assert the loader's existing duplicate-key behavior; if last-wins, record it as the format's definition | same | same |
| trailing/invalid content | garbage after document rejected as today | n/a | n/a |
| control row | unedited `{ provider: "kokoro", voice: "af_heart" }` gives provider kokoro, pre-warm on | | |

`Option<bool>` for `prewarm` with `#[serde(default)]` is the one grep hit to
justify: null must differ from absent, so use a deserializer that rejects `null`
rather than collapsing it. Grep `#[serde(default)]`, `unwrap_or_default`, `.ok()`
in the touched files before declaring done.

### Tasks

**Wave 2.1 (sequential: type first)**

- [ ] **Type change.** In `lib/src/config/tts.rs`, make `provider` optional and
  add `prewarm`, keep `deny_unknown_fields`, update docs on `TtsValue`.
- [ ] **Bridge.** In `dispatch/loader.rs`, add `tts_prewarm` to `GlobalSettings`
  per R1 and compute it from every shape per the spec's shape table; map absent
  provider to `None`.

**Wave 2.2 (parallel after 2.1)**

- [ ] **Hook speak.** Update `dispatch/runner/speak.rs` and
  `dispatch/runner/tests.rs`/`dispatch/tests.rs` so a provider-less object
  behaves as `true` (no assertion edits to existing tests).
- [ ] **Other consumers.** Fix compile fallout in `cli/src/commands/init_wizard.rs`,
  `cli/src/commands/init/mod.rs`, `runaway/config.rs`, `config/mod.rs`.
- [ ] **TUI tab.** `cli/src/commands/config_tui/tabs/tts.rs`: preserve `prewarm`
  in every object reconstruction path (provider, voice, gender edits); switching
  to a boolean drops the object; add the explanatory text that the hook toggle
  does not disable lifecycle speech or warming. No new toggle.
- [ ] **Config tests.** Add the matrix test and one test per shape-table row in
  `config/claudine_config/tests.rs` / `tts.rs`. Existing tts tests must pass
  unedited. Reject `{ enabled: true }`.

**Validation checkpoint 2:** acceptance criteria 6 (config half) and 7 pass;
`just test` and `just lint` green.

## Phase 3: Candidate Collection and Classification

Pure functions, no I/O, no worker yet. Fully unit-testable with the recorder
absent.

### Tasks

**Wave 3.1 (parallel)**

- [ ] **Action walker.** New module `composition/lifecycle/prewarm/collect.rs`:
  walk the parsed lifecycle (all stacks except `initialize`, `loop:` loose keys
  and `gate:`, sequence `setup:`/`teardown:`) yielding raw phrase expressions
  with their origin. Cover `speak` aliases, root dictionary bundles, positional
  and named speech arguments, and `then`/`else` to arbitrary depth. Never
  evaluate guards, never scan arbitrary frontmatter for `say`.
- [ ] **Classifier.** New `prewarm/classify.rs`: given a phrase and
  `PrewarmInputs` (R4), return `Early(resolved_text)` or `Skip(reason)`.
  - roots from `static_variable_reads`; early = `fm`/implicit frontmatter bare
    name, `ctx`, `env`; any other declared global is late, decided from the
    binding view so a new global cannot become an implicit property.
  - `fm.err` stays early.
  - sequence-injected keys late via literal bracket and bare name; computed
    index or whole-`fm` read skipped conservatively.
  - function call or shell expansion (R5) ineligible.
  - resolve with playback's interpolation, typed-value handling, text
    conversion, and empty suppression; produced strings are never re-evaluated;
    a resolution failure is a silent skip.
- [ ] **Dedup key.** `prewarm/key.rs` per R6.

**Wave 3.2 (after 3.1)**

- [ ] **Unit tests (criteria 1, 2, 3).** Table-driven: every eligibility rule,
  including zero-arg call, shell-probing function, call in an untaken branch,
  operators/indexing/fallback/ternary over early values, empty results not
  submitted, and brace text returned by an interpolation not re-evaluated.
  Also a drift guard: a test listing every declared reserved root and asserting
  its classification, so adding a global without deciding fails.

**Validation checkpoint 3:** criteria 1-3 and the classification half of 12
pass at unit level.

## Phase 4: The Worker

### Tasks

**Wave 4.1 (sequential)**

- [ ] **Worker type.** `prewarm/worker.rs`: `PrewarmHandle` with non-blocking
  `submit(candidates)` (records the dedup key at queue time) and a cancel-on-drop
  guard (R7). A single serial task on the tokio runtime; blocking provider work
  goes through `spawn_blocking`/the existing `std::thread::scope` pattern so
  the launch task's executor thread is never occupied.
- [ ] **Gates.** Before starting the worker or resolving anything, check
  `--dry-run`, `prewarm == false`, and `PLAYA_DRY_RUN` (`1`, `true`, `TRUE`,
  `yes`); a disabled handle is a zero-cost no-op.
- [ ] **Failure policy.** At most one tracing `warn` per invocation, no phrase
  text, secrets, or raw response; no terminal output; continue with later
  candidates; failed key is not retried; no fallback provider.
- [ ] **Candidate config.** Build via `tts_config_from_settings` on the bridged
  settings (R1); discard the builder returned by `prepare`.

**Wave 4.2 (after 4.1)**

- [ ] **Worker tests (criteria 8, 9, 13).** Controllable fake futures: submit
  returns immediately with a blocked `prepare`; a no-op success counts as done;
  a failure warns once and later candidates run; cancellation on drop does not
  drain; an event firing before completion uses the normal publish path
  (assert no join, no cancel). No elapsed-time thresholds.

**Validation checkpoint 4:** worker tests green; `just lint` clean.

## Phase 5: Wiring Into Invocation Flows

The riskiest phase: many call sites, one worker. Do the shared entry point
first, then fan out.

### Tasks

**Wave 5.1 (sequential)**

- [ ] **Single entry point.** `prewarm::collect_and_submit(snapshot, handle)`,
  the one function every site calls (also the future `prep` entry point; do not
  implement `prep`).
- [ ] **Ownership.** Create the worker once per top-level invocation in the
  compose / inline-compose / sequence / provider-wrapper entry
  (`cli/.../staged_boot.rs`, `pipeline.rs`) and thread the handle through; one
  worker per command, not per document.

**Wave 5.2 (parallel after 5.1; independent call sites)**

- [ ] **Bootstrap walk** before `initialize` fires, using the shell-free
  bootstrap parse.
- [ ] **Stabilized reread walk** after initialization, and on any new effective
  lifecycle/frontmatter snapshot; dedup against already-queued keys.
- [ ] **Sequence preflight** (`composition/sequence/preflight`): collect from
  every statically reached prompt document and each task's `setup:`/`teardown:`
  using the existing graph; submit all before step one launches; skip
  unresolved inputs; evaluate eligibility against the applicable task inputs,
  not path alone; no body composition, reference resolution, remote fetch, or
  setup execution.
- [ ] **Re-entry:** `proxy` (collected when the target is itself prepared, after
  its `with` overlay), retry/resume preparation, and loop refresh
  (`composition/looping`). A late-finishing candidate from a prior snapshot may
  only write its cache entry.

**Wave 5.3 (after 5.2)**

- [ ] **Wiring tests (criteria 4, 5, 12).** Three-prompt sequence submits all
  eligible phrases before the stub provider launches, even with a blocking
  `prepare`; unresolved input skipped then discovered later; proxy overlay
  applied; bootstrap vs. reread shares one worker with no repeat; retry, loop
  refresh, repeated prompt path, and parallel tasks dedup invocation-wide;
  bare and bracket `fm` reads, a new declared global, and sequence-dependent
  `fm` values.
- [ ] **CLI tests with `CliProcessFixture` (criteria 6, 10).** Assert
  `--dry-run` and `prewarm: false` prepare nothing; under `PLAYA_DRY_RUN=1`
  composing a shipped prompt with lifecycle `say` creates no audio cache, voice
  capability cache, spool entry, or subprocess; `audio_spool()` stays absent;
  accepted dry-run spellings tested with recorders. No window focus, no audio
  device.

**Validation checkpoint 5:** `just test`, `just lint`; run `just test-l2` only
if the detached-playback boundary was touched (criterion 14).

## Phase 6: Docs, Skill, and Closing Audit

### Tasks

**Wave 6.1 (parallel)**

- [ ] **Lifecycle docs.** Update `docs/topics/flow-control/lifecycle.md` (or the
  audio page it links) with pre-warm, early vs. late phrases (small example each,
  Mermaid flow of bootstrap -> reread -> worker -> cache), and the Limits list.
  Audience is a developer new to this repo. Do not name any feature or fix
  directory; state behavior in its own words.
- [ ] **Config docs.** Document the `tts` object, its keys, and the shape table
  in the configuration docs; mark nothing "planned" that has landed.
- [ ] **Skill.** Update `.claude/skills/claudine/` (SKILL.md and the relevant
  topic file; `topics` is a symlink to `claudine/docs/topics`) with the `tts`
  keys and shape table.
- [ ] **Comment drift pass.** Review `///`/`//` comments on every touched symbol
  (`TtsValue`, `bridge_tts_settings`, `tts_config_from_settings`, the TUI tab)
  and fix any that drifted.
- [ ] **Fixture gaps** noted in Task "Fixture check" (Phase 1), if any.
  - `CliProcessFixture` does not isolate `TMPDIR`/`TMP`/`TEMP` (the audio
    cache directory is `std::env::temp_dir()`) or `BISCUIT_SPEAKS_CACHE` (the
    voice-capability cache file); the child inherits the host's values. Add
    fixture-owned values for all four before the criterion-10 CLI test, or set
    them on that test's builder.

**Wave 6.2 (after 6.1)**

- [ ] **Audit.** Walk acceptance criteria 1-14 against tests by name; record
  the map in `implementation-log.md`. Grep `#[serde(default)]`,
  `unwrap_or_default`, `.ok()` over touched config code. Confirm no new CI cell,
  no new dependency (else update `docs/dependencies.md`), and Windows/WSL2
  portability (no path-comparison or `cfg(windows)` assumptions; load the `os`
  skill if any are touched).
- [ ] **Final run.** `just test` and `just lint` in `claudine/`. Terminal state
  is "implementation complete, ready for review"; do not move the feature to
  `_completed`.

**Validation checkpoint 6:** all 14 criteria mapped to passing tests; docs and
skill updated in the same change.
