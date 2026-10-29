---
area: biscuit-speaks
canonical_docs:
    - biscuit-speaks/docs/tts-caching.md
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 2
completed: true
human_review: false
pipeline_step: 9
related:
    - 2026-10-22-tts-prewarm
    - 2026-10-20-lifecycle-grammar
implemented: true
---
# Speak Prepare

biscuit-speaks gains one uniform way to synthesize a phrase ahead of time:
`Speak::prepare`. File-producing providers populate the shared audio cache;
eSpeak and Windows SAPI return successfully without preparing a file. Preparation
never plays audio or publishes a playback job. Later playback can reuse the
file rather than synthesize again, including when the voice was not explicitly
requested and its effective default can be resolved safely.

The consumer is Claudine's TTS pre-warm (`2026-10-22-tts-prewarm`), but the API
is general: any caller that knows a phrase ahead of time can use it.

```rust
use biscuit_speaks::Speak;

// Long before the phrase is needed; config is the caller's TtsConfig:
let prepared = Speak::new("the build is green")
    .with_config(config.clone())
    .prepare().await?;

// Later, reuse the chosen provider and synthesis settings:
prepared.play_detached().await?;

// A fresh object can also reuse the file if it resolves to the same settings:
Speak::new("the build is green")
    .with_config(config)
    .play_detached().await?;
```

Detached playback requires the `playa` feature. A cache hit avoids synthesis;
it does not promise immediate audible playback, since Playa serializes queued
audio and a previous job may still be playing. Preparation itself is awaited
and can make a billed cloud synthesis request. Claudine owns the decision to
run it in the background.

This material was split out of `2026-10-20-lifecycle-grammar` during the
2026-10-10 restructure.

## What exists today

All source links in this section refer to the biscuit-speaks package.

- The [audio cache](../../../biscuit-speaks/lib/src/audio_cache.rs) hashes
  provider, provider-specific voice identity, text, format, and optional speed
  with `biscuit_hash::xx_hash`. It writes files under the OS temporary directory
  and has no eviction or expiration policy. Some providers include model,
  engine, or language in the voice identity.
- Kokoro, gTTS, Echogarden, and ElevenLabs reuse files in their synthesis paths.
  [SayProvider](../../../biscuit-speaks/lib/src/providers/host/say.rs), which
  implements macOS speech, synthesizes a temporary WAV for ordinary playback
  each time; only its detached path currently copies eligible output into the
  shared cache.
- eSpeak and SAPI use direct speech commands rather than this file cache.
  Their preparation will be a no-op; this makes no promise about their latency.
- [Speak::prepare](../../../biscuit-speaks/lib/src/speak.rs) is a stub returning
  `Ok(self)`. The same builder's `is_prepared` currently checks an optional
  in-memory audio buffer, which the stub never fills.
- Say's detached cache lookup requires both a concrete voice and an explicit
  rate. Normal speed leaves the system rate unspecified, so even an explicit
  voice does not suffice. Gender preferences already select Alex or Samantha;
  `Gender::Any` leaves the system voice unspecified.
- [ElevenLabsProvider](../../../biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs),
  which implements cloud speech, rejects detached cache lookup when no voice
  is requested. Its normal synthesis instead fetches available voices, uses a
  gender match or the first voice, and falls back to its configured default
  only when that resolution fails or finds nothing. Ordinary playback already
  consults its cache after this resolution.

**Reader's note.** The review preserves those audible selection rules. Replacing
ElevenLabs' selection with its configured fallback, or replacing macOS system
settings with a library-chosen voice and rate, would change what users hear.
Neither change is authorized merely to improve cache reuse. The macOS lookup
and the ElevenLabs default lookup are both decided below.

## Decided

### Provider preparation contract

[TtsExecutor](../../../biscuit-speaks/lib/src/traits.rs), biscuit-speaks' provider
execution trait, gains:

```rust
fn prepare(
    &self,
    text: &str,
    config: &TtsConfig,
) -> impl std::future::Future<Output = Result<(), TtsError>> + Send;
```

Its default implementation returns `Ok(())` without work. Match the existing
trait's `Send` future contract; preparation is available without `playa`.
Say, Kokoro, gTTS, Echogarden, and ElevenLabs implement it using their shared
synthesis-to-cache path. They consult the cache before synthesizing and return
success only after a complete file is published, except when an unresolved
external default requires the no-op fallback below. Do not call a provider's
`speak` or enqueue a detached job to implement preparation.

Direct calls to this trait method and calls through `Speak` obey the same dry
run and no-playback rules. eSpeak and SAPI retain the default. Enum-only
providers have no executor implementation to inherit it: `Speak::prepare` must
report their existing unsupported-provider error rather than pretend they
prepared audio.

### Selecting a provider and retaining the result

[Speak](../../../biscuit-speaks/lib/src/speak.rs), biscuit-speaks' speech builder,
keeps `pub async fn prepare(self) -> Result<Self, TtsError>`.

- Use the existing provider ordering and availability rules for the requested
  failover strategy. Do not introduce cloud providers into automatic host
  selection, or add network readiness probes solely for preparation. No
  candidates yields `NoProvidersAvailable`.
- Prepare the first available candidate. An available eSpeak or
  SAPI candidate is a successful no-op; do not skip it to warm a different
  provider. A selected provider's construction, resolution, synthesis, or
  cache-write error is returned. Preparation does not try another provider
  after that failure. This is narrower than playback's existing failover.
- Keep the caller's original `TtsConfig` and playback failover policy. Store
  resolved synthesis settings privately on the returned object, associated
  with their selected provider. Do not write a provider-specific voice into
  the general configuration and later pass it to a different provider.
- Playback on that object tries the prepared provider with the retained
  settings first. If that attempt fails, its existing playback failover policy
  remains available, with the original caller configuration for other
  providers. `SpecificProvider` still forbids switching providers. A fallback
  provider need not have prepared audio.
- A fresh `Speak` resolves current defaults independently. Claudine's pre-warm
  discards the returned builder, so cross-object cache reuse is a required
  behavior, conditional on the same provider, effective settings, and cache
  directory. Availability or default changes can legitimately cause a miss.

No audio bytes need to be retained in memory. `is_prepared()` means that this
object completed file preparation successfully; it stays false when a fresh
object performs a no-op, dry run, or unresolved-default fallback. It is an observation of completed work,
not a guarantee that a temporary file still exists. Cloning retains this state.
`with_config`, `with_voice`, `with_gender`, `with_language`, and `with_failover`
clear the retained preparation settings and state; `with_volume` keeps them,
because volume affects playback rather than synthesis. A subsequent `prepare`
resolves again and consults the cache.

### Resolving defaults without changing the audio

Use one provider-specific resolution path for preparation, ordinary playback,
and detached cache lookup. Resolve once per operation and pass the resulting
values to both the key builder and synthesizer. Respect explicit voice/model
choices, gender and language preferences, and provider engine settings.

- A returned prepared object keeps the chosen settings even if defaults change
  later. A fresh object observes the changed defaults and must not reuse a
  file for the previous settings.
- For Say, retain the actual voice and integer words-per-minute rate. Do not
  convert an OS rate into `SpeedLevel::Explicit`: that type is a clamped
  multiplier, not an absolute rate. Do not assume the library's 175-WPM base
  is the system's current normal rate.
- For ElevenLabs, retain the voice chosen by its current resolution algorithm
  and the effective model identifier. Its configured default voice is only
  the existing fallback, not a new selection rule.
- gTTS identifies synthesis by language rather than an enumerated voice;
  do not manufacture a concrete voice for it. Echogarden's engine and any
  engine-selected default must also be accounted for. A mutable external
  default must not be represented only by the literal `default` in a key.
- When an external default cannot be resolved reliably, preparation succeeds
  as a no-op and playback synthesizes normally without consulting or publishing
  reusable output under that unresolved identity. Apply this to an unresolved
  rate as well as voice. Explicit, fully resolved configurations still prepare.
- For Say's system voice and normal rate, read the effective settings `say`
  actually uses through a verified OS lookup, behind an injectable resolver so
  tests can pin the result. Use only a lookup established to describe `say`'s
  effective settings; never read an undocumented preference and assume it is
  authoritative. When the lookup is absent, malformed, or unrecognized,
  `prepare` does nothing (the existing safe behavior) and playback synthesizes
  normally. A mocked resolver alone does not prove real macOS defaults are
  supported; the implementation records how the real lookup was verified.

### ElevenLabs default resolution snapshot

Current ElevenLabs voice selection fetches the account's voice list. Running
that lookup in `cached_detached_job` would put network latency and failure
handling into a path that today checks an explicit voice locally and returns
before detached synthesis. Because Claudine discards `Speak::prepare`'s
returned object, a fresh detached call must be able to find prepared default
speech without that network round trip. ElevenLabs therefore keeps a local
resolution snapshot:

- Preparation and normal synthesis record the effective voice and model
  together with the selection inputs that produced them (requested gender,
  language, model, and configured fallback) and the account context needed to
  tell callers apart. The account context is a non-secret identifier; raw API
  keys and other credentials are never persisted, hashed into the snapshot, or
  logged.
- The snapshot is a local file in biscuit-speaks' existing cache directory,
  the same location `BISCUIT_SPEAKS_CACHE` overrides for the voice-capability
  cache. It is published atomically.
- A fresh detached call with no requested voice looks up a snapshot whose
  selection inputs and account context match its own. A matching, trusted
  snapshot supplies the voice and model for the cache key, so a cache hit
  publishes a ready job without any network request.
- A missing, unreadable, mismatched, or expired snapshot, or any other
  uncertainty, is a cache miss. The call then follows today's miss path and
  the existing helper resolves the voice through the API.
- Refresh rule: every `prepare` re-checks the selection through the API and
  rewrites the snapshot; otherwise a snapshot is trusted for 24 hours from
  when it was written, after which it counts as expired.
- The snapshot never changes which voice is selected; it only records the
  result of the existing selection algorithm. Changing selection to the
  configured fallback is not a substitute: that would change the voice rather
  than solve the lookup cost.

### Cache and playback behavior

Reuse the existing `CacheKey` and `biscuit-hash`; do not introduce a second
cache or hash implementation. Share each provider's key construction across
all entry points. Keep existing key spellings for unchanged concrete inputs
where possible; changed keys may leave old temporary files unused, which is
acceptable under the existing no-expiration policy. Never read legacy Say
`system-default` entries as though they represented resolved settings.

| Provider | Synthesis identity beyond text/format | Playback handling |
|---|---|---|
| Say | Concrete voice and integer rate | Apply volume; rate is already in the WAV |
| Kokoro | Concrete voice | Apply volume and requested speed at playback |
| gTTS | Language | Apply volume and requested speed at playback |
| Echogarden | Engine, voice, and synthesis speed | Apply volume; avoid applying speed again |
| ElevenLabs | Voice, model, and synthesis speed | Apply volume; avoid applying speed again |

Retain current distinctions between normal and explicit speed in existing
keys unless all paths are changed together. Volume stays out of synthesis keys.
A prepared file must play at the volume requested at playback time.

Both ordinary playback methods consult the resolved cache before synthesis.
Detached cache hits publish a ready file job. Cache misses retain today's
ordered preparation reservation, helper process, and ready/failed transition;
this feature must not move synthesis into the requesting detached-playback
process. Any settings retained by a prepared object must survive helper handoff
if its file disappeared before playback.

Preparation does not reserve a spool slot, start a scheduler, or require the
executable's detached-worker initialization. Cancellation and synthesis failure
must not publish a partial cache entry; temporary files and children are cleaned
up. Concurrent calls may duplicate synthesis, including billed cloud work;
atomic publication must leave a complete usable file. One-synthesis assertions
apply to sequential calls, not simultaneous callers. No new cross-process lock
or duplicate-request suppression is introduced.

### Dry run

Use the same accepted `PLAYA_DRY_RUN` values as detached playback (`1`, `true`,
`TRUE`, and `yes`), including when `playa` is disabled. Check before provider
selection, construction, voice enumeration, default lookup, or cache access.
Return the builder unchanged: no new snapshot or prepared state is produced,
and any existing preparation state is retained.
Direct provider preparation also returns without work. There are no network
calls, model loads, subprocesses, cache writes, or spool/journal changes. This
resolves the original contradiction between promising a snapshot and forbidding
subprocesses that might be needed to discover it.

## Out of scope

- Walking documents, choosing phrases, and the `tts.prewarm` setting:
  `2026-10-22-tts-prewarm`.
- Cache eviction, expiration, model-version tracking, and a cache storage
  redesign. Existing caches are assumed to contain complete files produced by
  trusted local callers; broad cache-integrity hardening is separate work.
- A `so-you-say` CLI flag for preparing phrases.
- Preparing direct-streaming or unimplemented providers, changing ordinary
  playback failover policy, or making preparation run in the background.
- Performance benchmarks. Recording synthesis and lookup calls answers whether
  cache reuse works; this change does not require a timing spike.

## Acceptance criteria

These are L1 tests: hermetic unit or subprocess tests with no audio device,
model download, external network, or terminal/browser window. Cover macOS,
Linux, native Windows, and WSL2 through the existing environment policy;
platform-specific providers may have target-gated tests, with portable resolver
and cache tests covering shared behavior.

Use the monorepo test toolkit. Recording executables can follow
`biscuit-speaks/lib/tests/volume_control.rs`; ElevenLabs uses an injected fake
API transport or fixture responses rather than a recording executable. Pin
provider inventories, default lookups, and environment settings. Isolate audio
cache writes through fixture-owned `TMPDIR`/`TMP`/`TEMP`, and voice-capability
cache writes through `BISCUIT_SPEAKS_CACHE`. Tests with process-wide environment
changes use the toolkit's environment guard. Detached publication tests use
`test_toolkit::LockedAudioSpool` so a worker cannot play published fixtures and
pending work is cleaned up even on failure.

1. The provider trait has a `Send` no-op default; direct eSpeak/SAPI preparation
   produces no subprocess or file. Both feature configurations compile.
2. Each file-producing provider, with fully resolved settings, synthesizes
   exactly once for two sequential prepares and produces a reusable cache file.
   Preparation itself plays nothing and leaves the spool and journal absent.
3. A returned prepared object and a fresh object with matching effective
   settings reuse that file. Cover `play`, `play_with_result`, and
   `play_detached` through fake playback/publication boundaries; metadata names
   the actual provider and voice. Cover Say ordinary playback with and without
   `playa`, since both currently synthesize afresh.
4. With no requested voice and normal speed, Say and ElevenLabs detached
   lookup return ready jobs when their defaults are resolvable. Stubbed
   resolution preserves Say's gender choices and ElevenLabs' gender/first-voice
   selection and configured fallback. An unresolved external voice or rate
   yields no preparation file and no reusable-cache lookup.
5. Changing a default between preparation and playback makes a fresh object
   miss the old entry, while the returned prepared object retains its original
   settings. Changing provider-specific synthesis settings changes the key;
   changing volume does not. Verify baked speed is not applied twice.
6. Selected-provider preparation errors propagate without preparing another
   provider. Later playback failover uses the original configuration for its
   fallback. Missing files are synthesized again through the correct path;
   a prepared detached object's settings survive the helper handoff.
7. `is_prepared` follows the semantics above, including no-op and dry run;
   changing synthesis-related builder inputs clears preparation state, and
   changing volume retains it.
8. Dry run short-circuits both builder and direct provider preparation before
   any discovery or network work, even with unavailable providers and with
   `playa` disabled. Assert no cache, spool, journal, or subprocess effects.
9. Injected synthesis failure or cancellation leaves no partial cache entry
   or leaked child. Concurrent publication leaves complete output; duplicate
   synthesis is permitted rather than asserted away.
10. Update `biscuit-speaks/README.md`, the area's caching/provider/API topic
    pages, and `.claude/skills/biscuit-speaks/` with preparation, retained
    settings, no-op behavior, cache reuse, and the ElevenLabs resolution
    snapshot and its 24-hour trust window. Review the affected source docs
    and comments, including `is_prepared` and the stub's TODO. Provide a compact
    example that distinguishes the returned object from a fresh object.
11. ElevenLabs resolution snapshot, through the injected fake API transport
    and an injectable clock: after a default-voice `prepare`, a fresh detached
    call with matching selection inputs and account context returns a ready
    job and makes no API request. A snapshot older than 24 hours, a snapshot
    for different selection inputs or a different account context, a missing
    snapshot, and a corrupt snapshot each produce a cache miss that the helper
    resolves through the API. A second `prepare` re-checks the selection
    through the API even when the snapshot is fresh, and a changed account
    voice list updates the snapshot. The snapshot file contains no API key or
    other credential.
12. `just test` and `just lint` pass in `biscuit-speaks/`; compile the library
    without `playa` too. `just test` passes in `claudine/`, which depends on
    biscuit-speaks. Use the existing CI planner; no new matrix cell or gate is
    introduced by this specification.

## Open questions

None.
