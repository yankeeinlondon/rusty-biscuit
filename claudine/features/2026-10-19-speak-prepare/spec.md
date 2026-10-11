---
area: biscuit-speaks
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 9
related:
    - 2026-10-22-tts-prewarm
    - 2026-10-20-lifecycle-grammar
---
# Speak Prepare

biscuit-speaks gains one uniform way to say "synthesize this phrase now, so that
playing it later is instant": `Speak::prepare`. Every provider accepts the call.
Providers that write audio files synthesize into the shared audio cache;
providers that synthesize live (eSpeak, Windows SAPI) do nothing. Playback then
finds the prepared file, including under default voice and rate settings, where
today the cache is never consulted for macOS `say` and ElevenLabs.

The consumer is Claudine's TTS pre-warm (`2026-10-22-tts-prewarm`), but the API
is general: any caller that knows a phrase ahead of time can use it.

```rust
use biscuit_speaks::Speak;

// Long before the phrase is needed:
let prepared = Speak::new("the build is green").with_config(config).prepare().await?;

// Later, in this process or another one with the same config:
Speak::new("the build is green").with_config(config).play_detached().await?;
// -> cache hit: a ready job is published, no synthesis
```

This material was split out of the lifecycle-ergonomics feature (now
`2026-10-20-lifecycle-grammar`) during the 2026-10-10 restructure. The decisions
below were made there and are unchanged.

## What exists today

- A shared file cache, `biscuit-speaks/lib/src/audio_cache.rs`: the key is
  `provider:voice_id:text:format[:speed]`, hashed with `biscuit_hash::xx_hash`;
  entries are written atomically to `{OS temp}/biscuit-speaks-{hash}.{ext}`; no
  eviction, no TTL.
- File-producing providers that use it: Kokoro, gTTS, Echogarden, ElevenLabs,
  and macOS `say` (`providers/host/say.rs` runs `say -o <tmp>.wav` and copies
  the result into the cache).
- Providers that produce no file: eSpeak (`providers/host/espeak.rs`) and SAPI
  (`providers/host/sapi.rs`). They synthesize with near-zero latency, so there
  is nothing to prepare.
- `Speak::prepare` (`speak.rs`, the `pub async fn prepare(self) -> Result<Self, TtsError>`
  stub) returns `Ok(self)` without doing anything.
- **Defect.** The cache is consulted only when voice (and for `say`, rate) is
  explicit. `SayProvider::cached_detached_job` returns `None` unless
  `resolve_voice` and `resolve_rate` both return `Some` (it bails because
  "system voice and rate defaults can change independently of this config"),
  and ElevenLabs' `cached_detached_job` returns `None` when
  `config.requested_voice` is absent. Under default settings these providers
  never hit the cache.

## Decided

1. **A `prepare` method on the provider trait.** `TtsExecutor`
   (`biscuit-speaks/lib/src/traits.rs`) gains
   `prepare(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError>`
   with a default implementation that returns `Ok(())` and does nothing. It is
   not gated on the `playa` feature; the audio cache exists without Playa.
2. **File-producing providers implement it** by synthesizing into the shared
   cache when the entry for `(text, config)` is absent, and returning `Ok(())`
   without work when it is present. They are Say, Kokoro, gTTS, Echogarden, and
   ElevenLabs. eSpeak and SAPI keep the no-op default. Deferred enum-only
   providers keep the default too.
3. **`Speak::prepare` is implemented** with its existing signature. It selects
   the provider exactly as playback would select it first (same failover
   strategy and availability checks), snapshots the configuration (rule 4),
   calls that provider's `prepare`, and returns `Self` carrying the snapshotted
   configuration, so `prepared.play()` and `prepared.play_detached()` use the
   same key. It does not fail over: a failure of the selected provider's
   `prepare` is returned as the error, because audio prepared by a different
   provider would not be the audio playback looks up.
4. **Default voice and rate are resolved to concrete values, once.** When the
   config leaves voice or rate to a default, the provider resolves the default
   to the concrete value it will use (a voice name or identifier, a rate) and
   uses that value both in the cache key and in the synthesis command. The key
   computed at prepare time is therefore the key computed at play time, and a
   cached file always matches its key. A different process with the same
   configuration and the same OS defaults computes the same key.
    - This must not change which voice or rate is audible: the resolved value
      is the one the provider would have used anyway.
    - When a provider cannot name an OS default it depends on (for example,
      macOS `say`'s system voice cannot be read on the host), that call keeps
      today's behavior: no cache consult at playback, and `prepare` returns
      `Ok(())` without synthesizing. Correct audio wins over a cache hit.
5. **The cache-consult defect is fixed.** Playback (`play`, `play_with_result`,
   `play_detached` and its `cached_detached_job` path) checks the cache first
   whether or not a voice was requested explicitly, using the key from rule 4.
   For ElevenLabs the default voice is the provider's configured default voice
   id.
6. **Dry run.** Under `PLAYA_DRY_RUN=1`, `prepare` performs no synthesis, writes
   no cache entry, and spawns no subprocess; it returns `Ok` with the
   snapshotted `Self`. This is the switch Claudine's tests rely on.

## Out of scope

- Walking documents, choosing phrases, and the `tts.prewarm` setting:
  `2026-10-22-tts-prewarm`.
- Cache eviction or TTL: none is added.
- A `so-you-say` CLI flag for preparing phrases: not added.

## Acceptance criteria

All L1 unless marked. Tests need no audio device, no model download, and no
network on any of macOS, Linux, native Windows, and WSL2 (the WSL2 nightly leg
has no audio device; L1 has no network anywhere). Recording executables and
fake providers stand in for real synthesizers, as `lib/tests/volume_control.rs`
already does; the synthesized-audio cache is pointed at a private directory
through the fixture's `TMPDIR`/`TMP`/`TEMP`, and the voice-capability cache at
a private `BISCUIT_SPEAKS_CACHE`.

1. `TtsExecutor::prepare` exists with a no-op default; eSpeak and SAPI use the
   default, and a test proves calling it on them spawns nothing and writes no
   cache entry.
2. For each file-producing provider, `prepare` on a cold cache invokes its
   synthesizer once (observed through a recording stub) and leaves a cache
   entry; a second `prepare` with the same text and config invokes nothing.
3. `Speak::prepare` returns a `Speak` whose configuration carries the resolved
   voice and rate; `play_detached` on it, and on a fresh `Speak` with the
   original configuration, both find the prepared entry and publish a ready job
   without synthesis (asserted through the publication-only spool fixture).
4. Regression for the defect: with no requested voice, default rate, and a
   warm cache, `SayProvider::cached_detached_job` and ElevenLabs'
   `cached_detached_job` return a ready job. A Say test whose stubbed system
   default voice cannot be read asserts rule 4's fallback (no cache consult,
   `prepare` synthesizes nothing).
5. Changing the stubbed OS default voice between `prepare` and playback yields
   a cache miss, never the stale file.
6. Under `PLAYA_DRY_RUN=1`, `Speak::prepare` spawns nothing and creates no cache
   file.
7. `biscuit-speaks/README.md`, the `biscuit-speaks/docs/` pages that describe
   caching and the provider trait, and `.claude/skills/biscuit-speaks/` describe
   `prepare`, rule 4's resolution, and the fixed cache consult; the stub's
   `TODO` doc example is replaced by a working one.
8. `just test` and `just lint` are green in `biscuit-speaks/`; `just test` is
   green in `claudine/` (it depends on biscuit-speaks). No CI cell is added.

## Open questions

None.
