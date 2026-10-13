---
total_phases: 6
created: 2026-10-12
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/providers/host/say/system_defaults.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/prepare.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-speaks/SKILL.md
source_files_during_phase_2:
    - biscuit-speaks/lib/src/prepared.rs
    - biscuit-speaks/lib/src/lib.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/traits.rs
    - biscuit-speaks/lib/src/test_support.rs
    - biscuit-speaks/lib/src/providers/host/kokoro.rs
    - biscuit-speaks/lib/src/providers/host/gtts.rs
    - biscuit-speaks/lib/src/providers/host/echogarden.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/prepare.rs
docs_updated_during_phase_2:
    - biscuit-speaks/docs/tts-caching.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-speaks/api-reference.md
packages:
    - biscuit-speaks
source_files_during_phase_3:
    - biscuit-speaks/lib/src/cache.rs
    - biscuit-speaks/lib/src/prepared.rs
    - biscuit-speaks/lib/src/speak.rs
    - biscuit-speaks/lib/src/providers/host/say.rs
    - biscuit-speaks/lib/src/providers/host/say/system_defaults.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs
    - biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs
    - biscuit-speaks/lib/tests/common/mod.rs
    - biscuit-speaks/lib/tests/prepare.rs
    - biscuit-speaks/lib/tests/volume_control.rs
    - biscuit-speaks/lib/tests/detached_phase4.rs
docs_updated_during_phase_3:
    - biscuit-speaks/docs/tts-caching.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-speaks/SKILL.md
    - .claude/skills/biscuit-speaks/configuration.md
    - .claude/skills/os/windows.md
---
# Speak Prepare — Implementation Plan

Spec: `2026-10-19-speak-prepare` (area: `biscuit-speaks/`).

## Summary and Definition of Done

`Speak::prepare` is a stub that returns `Ok(self)`. This work makes it real:
the first available provider synthesizes the phrase into the shared audio
cache (or, for eSpeak/SAPI and unresolvable external defaults, succeeds as a
no-op), and the returned `Speak` privately retains the resolved synthesis
settings so later playback reuses the file. A *fresh* `Speak` must also hit the
same file when it resolves the same provider and effective settings. This is
what lets Claudine's pre-warm discard the returned builder.

The work has four hard parts, in order of risk:

1. **Default resolution without changing audible output.** Say needs the
   system voice and WPM rate read from a *verified* OS lookup; ElevenLabs needs
   a local, non-secret, 24-hour resolution snapshot so a fresh detached call
   can find prepared default speech with no network round trip.
2. **One key path per provider** shared by `prepare`, `speak`,
   `speak_with_result`, `detached_job`, and `cached_detached_job`, resolved
   once per operation.
3. **Retained settings** on `Speak` that survive clone, `with_volume`, provider
   failover, and the detached helper handoff (`PreparationRecord`).
4. **Dry run and no-op contracts** checked before any discovery.

Done means all of these hold:

- [ ] Every acceptance criterion 1–12 in the spec has a passing L1 test (map in "Validation Checkpoints").
- [ ] `just test` and `just lint` pass in `biscuit-speaks/` with and without `playa`; `just test` passes in `claudine/`.
- [ ] Docs, README, and `.claude/skills/biscuit-speaks/` describe the shipped behavior; the stub TODO and stale `is_prepared` doc are gone.
- [ ] No new CI matrix cell or gate (verified with `just ci-local --plan`).

Out of scope (do not build): tts-prewarm, cache eviction, a CLI flag,
background execution, benchmarks.

## Phase 1: Rulings, Spikes, and Test Scaffolding

### Necessary Rules

The spec leaves these unstated. Each ruling below is the planner's proposed
default; the author may overrule before Phase 2 starts, and implementers must
treat them as binding otherwise.

1. **Retained-settings shape.** A crate-private, `Serialize + Deserialize`
   enum `ResolvedSynthesis` with one variant per file-producing provider
   (`Say{voice, rate_wpm}`, `Kokoro{voice}`, `Gtts{language}`,
   `Echogarden{engine, voice, speed}`, `ElevenLabs{voice, model, speed}`),
   stored on `Speak` as `Option<(TtsProvider, ResolvedSynthesis)>`. It must be
   serializable because it rides in `PreparationRecord` through the helper
   handoff (spec §Cache and playback behavior). It is never merged into
   `TtsConfig`. Rationale: Say's absolute WPM cannot be expressed as
   `SpeedLevel`, and the spec forbids passing a provider-specific voice to
   another provider.
2. **Trait method returns `()`, internals return settings.** The spec fixes
   `TtsExecutor::prepare -> Result<(), TtsError>`. Each provider gets a
   crate-private `resolve(&self, text, config) -> Result<Resolution, TtsError>`
   (`Resolved(ResolvedSynthesis)` | `Unresolved`) and a
   `synthesize_resolved`/`cache_path_for(&ResolvedSynthesis)` pair. The trait
   `prepare` calls these and discards the settings; `Speak::prepare` calls them
   directly to retain the result.
3. **Remove the in-memory `audio`/`audio_format` fields of `Speak`.** Nothing
   fills them; the spec says no bytes are retained. This also removes the
   `play_with_result` "not supported for prepared audio" error path. `is_prepared`
   reads the new retained state.
4. **`Resolution::Unresolved` rule.** Unresolved voice *or* rate (Say), an
   unreachable/untrusted ElevenLabs resolution where the spec says no-op, or an
   Echogarden default that cannot be pinned → `prepare` returns `Ok`, retains
   nothing, `is_prepared()` stays false, and playback synthesizes normally
   **without** reading or publishing a cache entry under an unresolved identity
   (so the legacy `system-default` Say key is neither read nor written).
5. **ElevenLabs account context.** The spec requires a non-secret identifier
   that a *network-free* fresh call can compute, and forbids hashing the API
   key into the snapshot. Proposed: account context = (API base URL, name of
   the credential's source, e.g. the environment variable) plus an optional
   user-set `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` label. Known limit: two keys
   under the same variable name share a context; the 24-hour expiry and the
   every-`prepare`-refreshes rule bound the damage. **Author decision
   requested** if a stronger discriminator is wanted; it cannot be derived from
   the key without violating the spec.
6. **Snapshot key and file layout.** One file per (selection-inputs, account
   context) tuple under the `BISCUIT_SPEAKS_CACHE` directory, named by
   `biscuit_hash::xx_hash` of the canonical selection-input string (not of any
   credential), JSON, atomic temp-file + rename. `written_at` is Unix seconds
   from an injectable `Clock` trait. Expiry is `now - written_at >= 24h`; a
   `written_at` in the future is a miss.
7. **Snapshot-as-cache-key inputs.** The ElevenLabs cache key keeps today's
   spelling `"{voice}-{model}"` + `with_speed` only when speed is not Normal
   (existing behavior), so existing files stay valid.
8. **Say rate semantics.** Resolved rate = the integer WPM `say` would use
   (from the OS lookup) when `SpeedLevel::Normal`; for an explicit speed it is
   the existing `resolve_rate` mapping. If the spike shows the explicit-speed
   mapping depends on the (unknowable) system base, that case is `Unresolved`
   unless the lookup succeeded. Never reuse the 175-WPM assumption as the
   "normal" rate.
9. **Say with no `playa`.** Ordinary `play`/`play_with_result` consult the
   cache and copy synthesized output into it independent of `playa`
   (spec criterion 3); cache plumbing therefore lives outside `#[cfg(feature =
   "playa")]` blocks.
10. **Enum-only providers.** `Speak::prepare` returns the same
    `ProviderFailed{message:"Provider not yet implemented"}` error that
    `execute_host_provider` returns today; it does not skip them.
11. **Wider measurement.** The spec forbids a timing spike. No benchmark is
    planned; if the author wants cold-vs-warm numbers they must request them.

### Spikes

Each runs once, before the work it informs.

- [x] **Spike A — Say effective-settings lookup (macOS only).** Informs Phase 3.
    - On the macOS host, determine how to read the voice and WPM rate that
      `say` actually uses when invoked without `-v`/`-r` (candidates:
      `defaults read com.apple.speech.voice.prefs`, `com.apple.speech.synthesis.general.prefs`,
      `say -v '?'` default marker). Method: change the system voice/rate, run
      `say -o` with no flags, compare output duration/voice to what each
      candidate lookup reports.
    - Deliverable: a short note in `implementation-log.md` naming the one
      lookup (command, key, output grammar), the conditions it is trusted
      under, and what it returns when unset. If no lookup is reliably
      authoritative, record that and Phase 3 implements Unresolved-always for
      "system default" (spec-conforming); explicit voice+rate still prepare.
    - Other OSes cannot produce this evidence; the resolver is injectable and
      its tests are portable. (Load the `os` skill before claiming otherwise.)
- Dropped (spec already answers): no ElevenLabs latency spike, no cache-hit
  timing spike.

### Test scaffolding (Wave 1, concurrent)

- [x] **Wave 1 / Task: Recording executable harness.** In
  `biscuit-speaks/lib/tests/`, add a shared helper (modeled on
  `volume_control.rs`) that installs fake `say`, `kokoro`/`echogarden`/`gtts`
  executables on a fixture `PATH`, counts invocations, and can fail or hang on
  demand. Uses the monorepo test toolkit env guard; `TMPDIR`/`TMP`/`TEMP` owned
  by the fixture; `BISCUIT_SPEAKS_CACHE` pointed at a fixture dir.
- [x] **Wave 1 / Task: ElevenLabs fake transport + clock.** Design the
  injection seams (`trait ElevenLabsTransport`, `trait Clock`) in
  `elevenlabs.rs` with the production implementations unchanged in behavior.
  Ship one fake with request counting and scripted voice lists.
- [x] **Wave 1 / Task: Injectable Say resolver seam.** Define
  `trait SystemSpeechDefaults { fn voice(&self) -> Option<String>; fn rate_wpm(&self) -> Option<u32>; }` with a fake; production impl is a placeholder returning `None` until Phase 3.
- [x] **Wave 1 / Task: Locked spool fixture wiring.** Confirm
  `test_toolkit::LockedAudioSpool` usage for detached publication tests and a
  fake playback/publication boundary for `play`/`play_with_result`.

**Checkpoint 1:** scaffolding compiles in both feature configs; Spike A note
written; author has had the chance to overrule rulings 1–8.

## Phase 2: Core Contract, Builder State, and Simple Providers

Dependencies: Phase 1. Highest-fan-out phase; do Wave 2 first, then Wave 3.

### Wave 2 (sequential, one owner — shared types)

- [x] **Task: `ResolvedSynthesis` and `Resolution`.** Define in a new
  crate-private module (e.g. `prepared.rs`) per ruling 1; serde round-trip
  tests.
- [x] **Task: Trait default.** Add `TtsExecutor::prepare(&self, &str, &TtsConfig)
  -> impl Future<Output=Result<(),TtsError>> + Send` with `Ok(())` default, not
  gated on `playa`. Dry-run short-circuit lives in a shared helper
  (`dry_run_enabled()`), accepting `1`, `true`, `TRUE`, `yes` exactly as
  `play_detached` does, and **callable without `playa`**. Refactor
  `play_detached` to use the same helper (single source of truth).
- [x] **Task: `Speak` state.** Replace `audio`/`audio_format` with the retained
  `Option<(TtsProvider, ResolvedSynthesis)>`. `is_prepared()` = retained state
  present. `with_config/voice/gender/language/failover` clear it;
  `with_volume` keeps it; `Clone` copies it. Rewrite docs for the struct,
  `is_prepared`, `prepare`, `play`, and delete the stub TODO.

### Wave 3 (concurrent, one subagent per provider; disjoint files)

Each provider task: implement `resolve` + shared `cache_path_for`, route
`speak`, `speak_with_result`, `detached_job`, `cached_detached_job`, and
`prepare` through them (resolve once, pass to key builder and synthesizer),
consult the cache before synthesizing, publish via `write_atomic`, clean temp
files/children on failure or cancellation.

- [x] **Kokoro** (`providers/host/kokoro.rs`): concrete voice; speed applied at playback, not in key.
- [x] **gTTS** (`providers/host/gtts.rs`): language identity only (no manufactured voice); speed at playback.
- [x] **Echogarden** (`providers/host/echogarden.rs`): key = engine + voice + synthesis speed; the existing `voice.unwrap_or("default")` literal is replaced — a mutable engine-selected default that cannot be pinned is `Unresolved`; do not re-apply speed at playback.
- [x] **eSpeak/SAPI**: confirm they keep the default no-op; add the "no subprocess, no file" test.

### Wave 4 (after Waves 2–3): `Speak::prepare`

- [x] **Task: selection and retention.** Order: dry run (return `Ok(self)`
  unchanged, before any provider lookup) → `get_providers_for_strategy` → empty
  ⇒ `NoProvidersAvailable` → take the first candidate only → resolve + prepare →
  on `Resolved` store the pair; on `Unresolved`/eSpeak/SAPI return success
  with state unchanged → provider error propagates with no further provider
  tried. Enum-only providers return the ruling-10 error. No cloud added to
  automatic host selection; no readiness probes.
- [x] **Task: tests** for acceptance 1, 2, 7, 8 (builder + direct), and
  Kokoro/gTTS/Echogarden halves of 3, 5, 9.

**Checkpoint 2:** `just test` + `just lint` in `biscuit-speaks/`, with and
without `playa`; two sequential prepares → one synthesis per provider; nothing
plays; spool and journal absent.

## Phase 3: Say and ElevenLabs Default Resolution

Waves 5 and 6 are independent and run concurrently (different files:
`say.rs` vs `elevenlabs.rs` + a new snapshot module). Both depend on Phase 2.

### Wave 5 — Say (macOS-gated provider; portable resolver/key tests)

- [x] **Task: production resolver.** Implement `SystemSpeechDefaults` from the
  Spike A lookup. Absent, malformed, or unrecognized output ⇒ `None`
  ⇒ `Unresolved`. Record how the real lookup was verified (spec requirement)
  in the implementation log and the provider doc comment's *why*, not a
  command narration.
- [x] **Task: shared key and resolution.** Voice from explicit request, else
  gender (Alex/Samantha, per existing `resolve_voice`), else system default
  from the resolver; rate per ruling 8. Key `("say", "{voice};rate={wpm}")`
  — same spelling as today for concrete inputs. Never read or write
  `system-default`. Remove the legacy path-based cache key in `detached_job`'s
  unresolved branch, or confirm it never reads the shared cache.
- [x] **Task: ordinary playback caching.** `speak`/`speak_with_result` look up
  and publish through the cache with and without `playa`; volume applied at
  playback only; rate already in WAV.
- [x] **Task: `cached_detached_job`.** Ready job when the resolved key's file
  exists, even with no requested voice and Normal speed; `Unresolved` ⇒ `Ok(None)`.
- [x] **Task: tests** (acceptance 3-Say, 4-Say, 5): gender choices preserved;
  changing the stubbed default makes a fresh object miss while the prepared
  object keeps its settings; unresolved voice or rate ⇒ no file, no cache lookup.

### Wave 6 — ElevenLabs resolution snapshot

- [x] **Task: selection refactor.** Extract today's `resolve_voice_id` into a
  function returning `(voice, model, inputs)`; behavior (gender match →
  first voice → configured fallback) is untouched and covered by a
  characterization test written *first*.
- [x] **Task: snapshot module.** Serialize/deserialize `Snapshot{ voice, model,
  inputs(gender, language, model, fallback), account_context, written_at }`;
  atomic publish under the `BISCUIT_SPEAKS_CACHE` directory; no credential
  field exists on the type (compile-time guarantee) and a test greps the file.
- [x] **Task: write/refresh rule.** Every `prepare` re-resolves via the API
  and rewrites the snapshot; normal synthesis also records it. A changed
  account voice list updates the snapshot.
- [x] **Task: read rule in `cached_detached_job`.** With no requested voice,
  compute inputs + account context locally, load a *trusted* snapshot
  (matching, `< 24h`), build the key, publish a ready job with **zero** API
  requests. Any other outcome ⇒ cache miss ⇒ today's miss path.
- [x] **Task: tests** — see Input Robustness Matrix below plus acceptance 11.

#### Input Robustness Matrix — ElevenLabs snapshot (reader of a local file)

Load-bearing fields: `voice`, `model`, `inputs` (gender, language, model,
fallback), `account_context`, `written_at`. The unedited fixture is the
control row (fresh, matching ⇒ ready job, 0 API calls). Every edited row must
yield a **cache miss resolved via the API**, asserted through the public
result (`play_detached` outcome + fake-transport request count), one test
walking the table from a real written snapshot with one edit per cell.

| Shape | voice | model | inputs | account_context | written_at |
| --- | --- | --- | --- | --- | --- |
| absent | miss | miss | miss | miss | miss |
| explicit null | miss | miss | miss | miss | miss |
| wrong type, whole field | miss | miss | miss | miss | miss |
| wrong type, one element (inputs map) | n/a | n/a | miss | n/a | n/a |
| empty (`""`, `{}`) | miss | miss | miss | miss | miss |
| duplicate key | miss | miss | miss | miss | miss |
| trailing/invalid content | whole file ⇒ miss | | | | |
| stale (>24h) / future | | | | | miss |
| mismatched selection / account | | | miss | miss | |

Use `#[serde(deny_unknown_fields)]`, no `#[serde(default)]` on these fields, and
a duplicate-key-rejecting parse; grep for `unwrap_or_default`, `.ok()`,
`filter_map(.. as_str())` on this path before declaring the matrix done. The
Say OS-lookup output parser gets the same treatment with outcome "absent /
malformed / unrecognized ⇒ `None` ⇒ no-op".

**Checkpoint 3:** acceptance 4, 5, 11 pass; macOS run of the real Say lookup
recorded; a characterization test proves ElevenLabs selection is unchanged.

## Phase 4: Playback With Retained Settings and Helper Handoff

Depends on Phases 2–3. Single owner for `speak.rs`; the handoff piece can run
as a concurrent task in Wave 7.

### Wave 7 (concurrent)

- [ ] **Task: `play` / `play_with_result` with retained settings.** Try the
  prepared provider with the retained settings first, built from its
  `ResolvedSynthesis`; metadata (`SpeakResult`) names the actual provider and
  voice. On failure continue the existing failover using the **original**
  `TtsConfig` for other providers; `SpecificProvider` never switches. A missing
  cache file is re-synthesized through the correct path.
- [ ] **Task: `play_detached` with retained settings.** Cache hit ⇒ ready job;
  miss ⇒ today's ordered reservation + helper, unchanged. Extend
  `PreparationRecord` with the optional `(provider, ResolvedSynthesis)`
  (backward-compatible: absent decodes as none) so the helper synthesizes with
  the retained settings if the file vanished. Synthesis must not move into the
  requesting process.
- [ ] **Task: volume/speed rules.** Volume never in keys; prepared file plays at
  playback-time volume; baked-speed providers (Echogarden, ElevenLabs) do not
  re-apply speed; Kokoro/gTTS apply requested speed at playback (table in
  spec §Cache and playback behavior gets one test row per provider).

### Wave 8 (after Wave 7): behavior tests

- [ ] Acceptance 3 (returned vs fresh object across `play`,
  `play_with_result`, `play_detached`), 6 (failure propagation, fallback with
  original config, missing file, handoff), 9 (injected failure/cancellation
  leaves no partial entry or child; concurrent publication leaves a complete
  file; duplicates permitted, not asserted away).
- [ ] Detached tests use `test_toolkit::LockedAudioSpool`; no terminal/browser
  window gains focus.

**Checkpoint 4:** all acceptance criteria except docs (10) and final gates (12)
pass in both feature configurations.

## Phase 5: Documentation and Skill

Concurrent tasks (disjoint files), Wave 9. Audience: a developer new to the
repo; lead with what the reader can do; mark nothing "planned" (code landed).

- [ ] `biscuit-speaks/README.md`: preparation, returned-vs-fresh example.
- [ ] `biscuit-speaks/docs/tts-caching.md` and the provider/API topic pages:
  retained settings, no-op cases, cache reuse, dry run, the ElevenLabs snapshot
  and its 24-hour trust window, Say lookup; include one Mermaid diagram of the
  prepare → retain → fresh-object-hit flow. Docs must not link to or name the
  feature directory.
- [ ] `.claude/skills/biscuit-speaks/` (`SKILL.md`, `api-reference.md`,
  `providers.md`, `configuration.md`): same facts, kept compact.
- [ ] Source-doc pass over every symbol whose behavior changed: `Speak`,
  `is_prepared`, `prepare`, `play`, `audio_cache.rs` module table, trait docs,
  provider module docs (drift rule: code wins; report drift found).
- [ ] Add the comment-quality check: no HOW-narration or format-string quoting.

## Phase 6: Final Validation

- [ ] `just test`, `just test-l2` (if the area defines L2 for speaks), and `just lint` in `biscuit-speaks/`.
- [ ] Compile the library with `--no-default-features` / without `playa`; run its tests.
- [ ] `just test` in `claudine/`.
- [ ] `just ci-local --plan`: confirm no new cell or gate; platform-gated
  tests (`Say` → macOS, `Sapi` → Windows) compile on all targets (load the
  `os` skill before any `#[cfg(windows)]` or path-comparison work).
- [ ] Grep the smell list: `#[serde(default)]`, `unwrap_or_default`, `.ok()` on
  the snapshot and Say-lookup parse paths.
- [ ] Implementation log written (departures from spec, Spike A result, ruling
  outcomes). Terminal state: "implementation complete, ready for review"; do
  not move the feature to `_completed`.

## Validation Checkpoints (acceptance map)

| Criterion | Where proven |
| --- | --- |
| 1 trait default, compile both configs | Wave 2, Phase 6 |
| 2 single synthesis, no spool/journal | Wave 4 |
| 3 returned vs fresh reuse | Wave 8 |
| 4 Say/ElevenLabs default lookup | Wave 5, 6 |
| 5 default changes, key sensitivity, speed once | Waves 5, 7, 8 |
| 6 error propagation, fallback, handoff | Wave 8 |
| 7 `is_prepared` semantics | Wave 4 |
| 8 dry run | Wave 4 |
| 9 failure/cancellation/concurrency | Wave 8 |
| 10 docs | Phase 5 |
| 11 snapshot | Wave 6 |
| 12 gates | Phase 6 |

## Risks

- **Say lookup has no authoritative source** → `Unresolved` for system defaults
  (spec-conforming, weaker pre-warm hit rate). Mitigated by Spike A first.
- **Account context is weaker than a key-derived one** (ruling 5) → bounded by
  24h expiry and refresh-on-prepare; author decision flagged.
- **Key spelling drift** orphans old cache files → acceptable per spec; keep
  existing spellings for concrete inputs and test them as golden values.
- **`speak.rs` merge pressure**: Waves 2, 4, 7 all touch it; keep one owner.
