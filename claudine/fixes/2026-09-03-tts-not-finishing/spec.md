---
status: draft
created: 2026-09-03
updated: 2026-09-03
reviewed: true
implemented: true
reviewed_by: codex/default
reviewed_on: 2026-09-03
area: claudine
packages:
    - playa
    - biscuit-speaks
    - claudine
review_iterations: 2
phase_1_protocols_ratified: true
---

# Spoken status messages stop before the sentence ends

## Summary

Claudine's spoken status messages (lifecycle `say:` in the repo prompts, and
the `so-you-say --background` call in `just commit`) start playing and then
stop a second or two before the sentence finishes. The tail of every sentence
is lost.

Two things are wrong, and the second is what let the first hurt.

1. **The TTS binaries never use playa's native playback.** playa's design is
   native-first: decode in-process and play through CoreAudio / WASAPI /
   ALSA, falling back to a host player subprocess only when native is
   unavailable. That is true for the standalone `playa` CLI and for nothing
   else. The native path is gated behind the `native-playback` feature, which
   `biscuit-speaks` and `claudine` do not enable, and the playback function
   `biscuit-speaks` calls has no native branch at all. Every spoken sentence
   therefore goes to a host player.
2. **The host player playa ranks first on macOS clips mono audio.** `mpv`
   on Ken's host ends playback of **mono** files early while exiting with
   status 0. Every file-producing TTS provider (Kokoro, the one Ken's config
   selects) emits mono WAV, so every sentence is clipped, while playa's
   bundled effects (all stereo) play in full. Nothing in the stack checks
   that playback ran for as long as the audio, so the clipping was invisible.

Neither is caused by a commit in this repository; the host's mpv was rebuilt
by Homebrew on 2026-07-10 and the default output device is a USB DAC that
refuses a mono CoreAudio layout. The repository's job is to stop depending
on that player for speech and to notice when any player misbehaves.

The investigation also confirmed a third defect Ken has asked to fix in the
same change: Claudine's fire-and-forget audio cannot survive the process it
lives in. The hook handler spawns audio as a Tokio task and then calls
`process::exit`, which drops the sound before it starts; the lifecycle path
avoids that only by blocking the composition thread for up to 30 seconds per
signal. Ken's direction: Claudine hands audio off and continues immediately,
and several pending clips play one after another without Claudine waiting.

The fix has four parts, each in the package that owns the behavior:

- playa makes native playback the real default for every playback entry
  point, and every consumer that speaks compiles it in (D1);
- playa verifies playback completion against the audio's known duration so
  a misbehaving player is visible (D2), and hardens mpv for probed mono input
  without changing multichannel playback (D3);
- playa and biscuit-speaks gain a detached, serialized playback path that
  outlives the caller (D4, D5);
- Claudine adopts that path everywhere it makes sound (D6).

> **Reader's note (review 2026-09-03):** The original spool proposal used a
> non-blocking worker-lock probe to avoid spawning a worker when one appeared
> to be active. That protocol can strand a job if it is committed after the
> worker's final empty scan but before the worker releases its lock. The
> reviewed design coordinates publication and the worker's empty-queue handoff
> under a separate short-lived queue lock, so a committed job either belongs
> to the current worker or causes a successor to be spawned. The review also
> makes worker-entrypoint registration and per-user spool security explicit.
> These are correctness requirements, not optional hardening.

## Diagnosis

### Which code path actually runs

| Consumer | playa features enabled | Native playback compiled in |
|---|---|---|
| `playa` CLI | `sfx-native`, `sfx-native-audio`, `native-playback` (default) | yes |
| `biscuit-speaks` lib (so-you-say) | `async` only | no |
| `claudine` lib | `sound-effects`, `async` | no |
| `claudine-cli` | `sound-effects` | no |

Even with the feature on, native-first logic exists only in the `Playa`
builder (`Playa::play` / `play_async` in `playa/lib/src/playa.rs`).
biscuit-speaks calls `playa_explicit_with_options_async` in
`biscuit-speaks/lib/src/playback.rs`, which goes straight to `select_player`
and a host subprocess. Claudine's sound effects do use the builder, but
without the feature it compiles down to the same host path.

The `playa/README.md` states "native playback is off by default; opt in per
feature" as a library tenet. The consumers that speak never opted in, so the
skill's "native-first" description has never applied to speech.

### Reproduction of the clipping

Measured on Ken's macOS host on 2026-09-03 (mpv 0.41.0_6, Homebrew bottle
rebuilt 2026-07-10 against ffmpeg 8.1.2; default output device AudioQuest
DragonFly USB DAC, 2 output channels, 44.1 kHz).

The Kokoro provider synthesized a 21-word sentence to a 24 kHz mono 16-bit
WAV of 5.97 seconds. Wall-clock time to play that file:

| Player / flags                                  | Wall time | Verdict          |
|-------------------------------------------------|-----------|------------------|
| `afplay`                                        | 7.21 s    | complete         |
| `ffplay -nodisp -autoexit`                      | 6.37 s    | complete         |
| `mpv --no-video --no-terminal --really-quiet`   | 4.76 s    | **clipped**      |
| same, `--no-config`                             | 4.78 s    | clipped          |
| same, `--ao=avfoundation`                       | 4.96 s    | clipped          |
| same, `--audio-samplerate=48000`                | 4.73 s    | clipped          |
| same, `--gapless-audio=no`                      | 4.74 s    | clipped          |
| same, `--audio-buffer=1`                        | 4.74 s    | clipped          |
| same, **`--audio-channels=stereo`**             | 6.33 s    | **complete**     |
| same file resampled to 48 kHz mono              | 4.77 s    | clipped          |
| same file resampled to 44.1 kHz mono            | 4.72 s    | clipped          |
| same file upmixed to 24 kHz **stereo**          | 6.30 s    | complete         |
| bundled effect `cartoon-accent9.wav` (48 kHz stereo, 2.73 s) | 3.12 s | complete |
| 3.0 s **silent** 24 kHz mono WAV                | 1.71 s    | clipped          |
| 3.0 s silent mono, `--audio-channels=stereo`    | 3.31 s    | complete         |

The channel count is the trigger. Sample rate, content, and mpv config are
irrelevant. Silence reproduces it, which matters for testing: the regression
test can be inaudible.

mpv's verbose log shows the CoreAudio output failing to negotiate a mono
layout on this device, then falling back to the AVFoundation output, which
reports end of stream early:

```text
[ao/coreaudio] requested format: 24000 Hz, mono channels, s16
[ao/coreaudio] selected audio output device: AudioQuest DragonFly (174)
[ao/coreaudio] unable to set the input channel layout on the audio unit ([206][255][255][255]/-50)
AO: [avfoundation] 24000Hz mono 1ch s16
[cplayer] audio EOF reached          <- at 2.6 s of a 5.97 s file
[cplayer] Exiting... (End of file)
```

mpv exits with status 0, so `playa_with_player_and_options_async` returns
`Ok(())` and biscuit-speaks reports a successful `speak`.

### Why it is not a repository regression

- `playa/lib/src/playback.rs` and `player.rs`: last change 2026-06-29
  (byte-playback cache hardening); mpv argument construction is unchanged
  since 2026-03.
- `biscuit-speaks/lib/src/providers/host/kokoro.rs` and `playback.rs`: last
  change 2026-04-01.
- `claudine/lib/src/composition/lifecycle/audio.rs`: last change 2026-07-11
  (module move); `say_blocking` semantics unchanged.
- The feature wiring above has been this way since native playback was
  introduced; speech has always used a host player.

What changed is the host: Homebrew rebuilt mpv on 2026-07-10, and the
DragonFly DAC is the default output. The repository cannot tell those apart
and should not have to.

### Where the sound comes from

Ken's `~/.claudine/config.json` selects `tts.provider: kokoro` and configures
one `sound_effect` action (`doorbell-2` on `human_in_the_loop`). No `speak`
actions are configured, so the spoken status Ken hears comes from:

- **Lifecycle `say:`** in the repo prompts (`prompts/sentrux.md`,
  `prompts/clarify.md`, `prompts/_implement/implement-plan.md`, and others)
  via `DefaultLifecycleEmitter::emit_speech` → `say_blocking` →
  `biscuit_speaks::Speak::play` → Kokoro → playa host path → mpv.
- **`just _speak`** (`just/notify.just`) via `so-you-say … --background`,
  which re-executes itself detached and then follows the same path.

### Adjacent defect confirmed on the hook path

`claudine handle` fires the `sound_effect` action through
`tokio::task::spawn_blocking` and then reaches `std::process::exit` in
`claudine/cli/src/commands/handle.rs` about 130 ms later. Probing the
`human_in_the_loop` binding on this host: the handle exits in 0.13 s and no
player process exists afterward, so the 6.8 s doorbell never plays. The
`speak` action (`execute_speak_from_claudine`, a plain `tokio::spawn`) has
the same exposure.

Native playback makes this worse, not better: an in-process CoreAudio stream
dies with the process. Fire-and-forget audio therefore needs a process of
its own (D4).

## Established contracts

These hold today and must still hold after the fix.

- playa is native-first with host fallback. Native failures that are not
  format-related (device open timeout, stream errors) trip a process-local
  circuit breaker and route subsequent playback to host players
  (`native_player::should_fallback_to_host`, `native_audio` breaker).
- Host players are ranked by capability (speed, volume, streaming) and the
  best installed one that satisfies the requested options is used. This
  ranking is not changed.
- biscuit-speaks providers own synthesis; playa owns playback of any file or
  byte buffer a provider produces. Streaming providers (`say` on macOS, SAPI
  on Windows, eSpeak without a file) speak directly and never touch playa.
- `PLAYA_DRY_RUN=1` / `.dry_run()` skips all device, subprocess, and file
  work and returns `Ok(())`.
- `playa --background` preserves the foreground command's playback options,
  including force-host selection, output channel, and ducking policy.
- Claudine's lifecycle emission order is deterministic: `say_first` +
  `effect` plays speech then effect; `say` + `effect` plays effect then
  speech. `LifecycleEmitter` is the injection seam tests use.
- The Claudine hook handler must return to the calling agent promptly
  (15 s deadline, exit discipline in `handle.rs`).
- L1 tests never require an audio device. Tests that do live in the `real_`
  tier and skip cleanly when no sink is present. `#[ignore = "requires real
  audio device"]` is the pattern to retire, not extend.
- The playa *library* keeps native playback opt-in (README tenet). Consumers
  that make sound opt in explicitly; see D1.

## Decisions

### D1. Native playback is the real default for speech

Two changes make the documented design true for every consumer:

**a. One playback pipeline.** The free functions in `playa/lib/src/playback.rs`
(`playa`, `playa_explicit`, `playa_explicit_with_options`, and their
`_async` twins) become thin wrappers over the `Playa` builder, so the
native-first / host-fallback / circuit-breaker logic lives in exactly one
place. Only `playa_with_player*` (an explicit player was requested) stays
host-only. biscuit-speaks' `play_audio_file` / `play_audio_bytes` then get
native playback with no code change of their own; they are nonetheless
rewritten to use the builder directly so the dependency is visible.

**b. Consumers compile it in.** `biscuit-speaks`' `playa` feature enables
`playa/native-playback` in addition to `async`; `claudine` (lib and cli)
enables `native-playback` on their playa dependency. `sfx-native-audio`
(OS-specific SFX routing) is *not* enabled for these consumers; plain
`native-playback` through the default device is what speech needs.

Consequences that must be handled in the same change:

- Linux builds of biscuit-speaks and claudine now link ALSA. Their
  `[package.metadata.ci.native]` gains `libasound2-dev` (biscuit-speaks
  already declares `espeak-ng`; playa already declares `libasound2-dev` and
  `libpulse-dev`). `just init` / `_ensure-native-libs` already provisions
  these for the playa area; confirm it covers the workspace.
- Windows uses WASAPI through cpal; no extra libraries.
- The native device-open deadline and stall window (`native_player.rs`,
  5 s stall window, 300 s playback cap) now apply to speech. The stall
  breaker measures progress, so a real sentence never trips it; document
  this in the biscuit-speaks skill.
- rodio decodes WAV/MP3/OGG via symphonia and converts mono to the device's
  channel count itself. Phase 1 must record evidence that the 24 kHz mono
  Kokoro WAV plays to completion through the native path on Ken's host
  (this is the acceptance test for the whole fix).

### D2. playa verifies playback completion against the audio's known duration

Trusting a player's exit status (or a native sink reporting empty) is what
made this regression invisible. playa already derives effect durations from
container headers at build time (`effect_durations.rs`). The same header
probe becomes a runtime helper, `probe_audio_metadata(&AudioData) ->
Option<ProbedAudioMetadata>`, whose result includes duration and channel
count. It covers at least PCM WAV and the formats the effects already cover;
formats it cannot parse yield `None` and are exempt. A URL is never fetched a
second time merely to probe it: URL sources are unverified unless metadata is
already available from the playback pipeline.

Every playback, native or host, records wall time from start to completion
and compares it with the probed duration adjusted for the effective speed the
selected route actually applies (including any existing player-specific
clamp), not merely the originally requested value:

```rust
pub struct PlaybackReport {
    pub route: PlaybackRoute,         // Native | Host(AudioPlayer) | DryRun
    pub expected: Option<Duration>,   // None when the format was not probed
    pub elapsed: Duration,
    pub verdict: PlaybackVerdict,     // Complete | Truncated { missing: Duration } | Unverified
}
```

The report and its enums derive `Serialize` and `Deserialize` because spool
journals persist them. Serialized enum names are snake_case, and newly added
fields require Serde defaults so an upgraded diagnostic reader can consume the
immediately previous journal schema.

`Playa::play_with_report` / `play_async_with_report` return it; the existing
`play` / `play_async` and the free functions keep returning
`Result<(), PlaybackError>` on top of it. Report-returning counterparts also
cover the explicit-player functions so diagnostics and per-player real tests
do not need to bypass the public API. Dry-run short-circuits before metadata
probing and returns an unverified dry-run report without reading the source. A
`Truncated` verdict:

- logs one `tracing::warn!` naming the route, expected and elapsed seconds
  (the line that would have surfaced this bug on day one);
- does **not** return an error and does **not** retry. Replaying from the
  start repeats what the listener already heard. The verdict is information
  for callers, the spool journal (D4), and tests.

Truncation threshold: elapsed < expected × 0.9 − 250 ms. Startup adds time
and never removes it, so a false `Truncated` needs the player to finish
faster than the audio, which only happens when it stopped early. The
constant lives in one place with a comment explaining the asymmetry. Durations
are divided by a validated, positive playback-speed multiplier; invalid speed
values remain rejected at the existing options boundary rather than producing
a meaningless report.

### D3. The mpv fallback upmixes probed mono input

Host players remain the fallback for hosts without native support and for
the breaker-tripped case, and mpv stays ranked first among them. Both
`build_player_command` and `build_player_args` add `--audio-channels=stereo`
only when D2 positively identifies a one-channel source. mpv upmixes mono
internally; this side-steps the failed mono negotiation on CoreAudio and the
early EOF on AVFoundation. Unknown, stereo, and multichannel input is left
untouched: forcing every invocation to stereo would silently downmix
legitimate surround audio and is not an acceptable library-wide side effect.
The comment at the conditional must say why (criterion B in `AGENTS.md`) so
it is not cleaned up later. No other player gets a channel flag; `afplay`,
`ffplay`, `paplay`, `aplay`, and `sox` played mono correctly or have no safe
equivalent.

> **Reader's note:** the first draft forced stereo for every mpv invocation.
> Review narrowed the workaround to positively identified mono sources so the
> TTS fix does not alter Playa's general multichannel contract.

### D4. playa owns a detached, serialized playback path

Fire-and-forget audio must outlive the process that requested it, and
clips requested in quick succession must play in order. With native playback
the audio *is* the process, so the only way to survive `process::exit` is a
worker process that owns the device. Both properties belong in playa, next
to the playback pipeline they depend on, so every caller gets them from one
implementation.

Design: a **private, versioned spool** plus a **worker** process.

- `playa::detached::enqueue(job: SpoolJob) -> Result<JobId, PlaybackError>`
  atomically publishes one versioned job envelope and returns immediately.
  The default root is `<temp_dir>/playa-spool-<user-hash>/v1/`, where
  `user-hash` is the `biscuit_hash::xx_hash` fingerprint of the stable OS-user
  ID from `sniff::os::current_user_id()`; raw SIDs/user IDs are not used as
  path components. Linux must not use a shared, unqualified
  `/tmp/playa-spool`. `PLAYA_SPOOL_DIR` overrides the root for tests. Unix
  creates the root with mode `0700`; every platform
  rejects a symlink/reparse-point root or job file and uses create-new plus
  atomic rename so another local user cannot turn the worker into an
  arbitrary-command launcher.
- A job envelope contains `schema_version`, `job_id`, `enqueued_at`, and one
  payload. A job is one of:
  - `PlayFile { path, playback, delete_after }`, where `playback` preserves
    `PlaybackOptions`, auto-versus-force-host routing, output channel, and
    optional ducking configuration. Paths are made absolute at enqueue time;
    CLI-authored path references are resolved through
    `biscuit_file::FileReference` at the CLI boundary before the job is built.
  - `Command { program, args }` for implemented streaming TTS providers that
    speak directly (macOS `say`, SAPI, and eSpeak). The program is resolved to
    an absolute executable with the existing `sniff` discovery result, is
    launched directly rather than through a shell, and the worker waits for
    it. Arguments and paths use a lossless OS-string encoding (Unix bytes or
    Windows wide units); JSON's Unicode strings are not assumed to round-trip
    every valid path.
  Byte buffers are not a job type: the enqueuing side writes them to the
  existing audio cache and enqueues a `PlayFile`. That cache switches its
  current `DefaultHasher` fingerprint to `biscuit_hash::xx_hash`, the repo's
  authority for non-cryptographic content hashing. `delete_after` is true
  only for a newly materialized, spool-owned file; a shared content-addressed
  cache entry is never deleted after one job.
- A short-lived `<spool>/queue.lock` serializes publication and allocates a
  durable increasing sequence number; wall-clock nanoseconds are timestamps,
  not monotonic tickets. `<spool>/worker.lock` separately serializes playback.
  The enqueuer holds `queue.lock` while publishing and testing `worker.lock`.
  If the worker lock is held, the active worker owns the published job; if the
  probe acquires it, the enqueuer releases the probe and spawns a successor
  before releasing `queue.lock`.
  `enqueue` returns `Ok(JobId)` only after one of those conditions is true. If
  spawning fails, it marks the just-published job failed while still holding
  `queue.lock` and returns the error, preventing a caller's fallback playback
  from later being duplicated. A requester crash in the commit-to-spawn window
  can leave a pending job, which the next enqueue recovers or the stale policy
  eventually discards; detached delivery is explicitly best-effort, not
  durable messaging.
  A worker that observes an empty queue acquires `queue.lock`, rechecks, and
  releases `worker.lock` while still holding `queue.lock`. This lock ordering
  closes the empty-queue handoff race without a timing assumption. A 200 ms
  sleep is not a correctness mechanism.
- The **worker** is the current executable re-executed with the environment
  marker `PLAYA_SPOOL_WORKER=<spool dir>`; no argv is added, so host
  binaries' clap surfaces are untouched. It runs detached: `process_group(0)`
  on Unix; `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW`
  on Windows; stdio null. It takes the lock and loops: read the lowest-named
  pending job, atomically rename it to an in-flight name, dispatch by payload,
  append the outcome, clean up, and repeat. `PlayFile` uses
  `Playa::play_with_report` (native-first, D1); `Command` waits for the direct
  process and records its exit status as an unverified command outcome rather
  than fabricating a `PlaybackReport`. An in-flight file left by a crashed
  worker is quarantined and journaled rather than replayed; detached audio is
  best-effort and at-most-once once playback starts. A pending job older than
  10 minutes is discarded with a journaled warning rather than played: a stale
  spool must never speak yesterday's status.
- Host binaries opt in with one call at the top of `main`, before any
  argument parsing:

  ```rust
  if let Some(code) = playa::detached::run_if_worker() {
      std::process::exit(code);
  }
  ```

  A normal-process call records that the executable installed the seam;
  `enqueue` returns `PlaybackError::NoDetachedWorker` if no seam was installed.
  Merely spawning the executable cannot otherwise prove that its `main`
  honors the marker. `claudine`, `so-you-say`, and `playa` add the call in
  this fix.
- `PLAYA_DRY_RUN=1` and `.dry_run()` retain their established no-file,
  no-subprocess behavior. Detached enqueue under dry-run returns a dry-run
  `JobId` without creating a spool or worker. Queue tests use an injected fake
  playback executor and call the worker drain seam directly; process tests use
  a purpose-built fixture executable. Dry-run is not repurposed as a spool
  test mode.
- The journal records IDs, timestamps, a redacted source kind, and a typed
  outcome: either a Playa report or a direct-command exit status. It never
  records speech text, command arguments, credentials, or full paths. It
  rotates at a documented bounded size and keeps one prior file. `playa spool`
  renders the journal and pending jobs with `TerminalRenderable` components;
  path text uses
  `biscuit_file::try_portable_string`. Pending `Command` details are redacted.

#### Ratified v1 worker protocols

The owner ratified both recommended option 3 designs for Phase 1. Protocol
version `1` therefore has one per-user scheduler queue, reserves a sequence
before cache-miss synthesis, and delegates playback to the executable that
accepted the job. These are one contract: a scheduler must not synthesize TTS
or silently substitute its own compiled Playa capabilities.

All persisted enum discriminants use snake_case. `schema_version`, identity,
sequence, state, executable identity, and capability version are required and
never receive Serde defaults. A newly added optional field must use
`#[serde(default)]`; a newly added collection may default only when absence and
an empty collection have identical semantics. Readers accept version 1 records
with unknown additive fields. Any unsupported version is atomically moved to
quarantine and journaled from its redacted header; it is never executed or
rewritten as the current version.

| Projection | Required v1 content | Private content allowed | Compatibility and terminal behavior |
|---|---|---|---|
| Job envelope | `schema_version`, `job_id`, durable `sequence`, `enqueued_at`, lossless absolute enqueuer executable, `capability_version`, and payload state | Paths and command arguments, because the entire spool is private | Unknown additive fields ignored; missing required fields or unsupported versions quarantine the job |
| Preparation record | Envelope identity plus `state: preparing`, `preparation_version`, deadline, speech text, and non-secret `TtsConfig` | Speech text and non-secret synthesis configuration | Only the matching helper may replace it with the same sequence as `ready` or `failed`; it may not allocate another job |
| Delegated-play request | Envelope identity, `delegated_play_version`, complete playback/command projection, capability version, and a create-new report-sidecar path inside the spool | Losslessly encoded path/argument data needed to execute the job | The delegate validates identity, version, capability, and sidecar ownership before playback; mismatch fails without fallback |
| Delegated report | `schema_version`, `job_id`, sequence, delegate identity, and exactly one typed Playa report or command-exit/failure outcome | No speech text; no command arguments or credentials | Written create-new and atomically published; missing, malformed, duplicate, or mismatched reports fail the in-flight job |
| Journal projection | IDs, sequence, timestamps, redacted source kind, state transition, and typed redacted outcome | None: no speech text, command arguments, credentials, or full paths | Versioned independently, bounded and rotated with one prior file; diagnostic readers tolerate additive defaulted fields |

The scheduler marker is `PLAYA_SPOOL_WORKER`; the playback delegate marker is
`PLAYA_DELEGATED_PLAY_WORKER`; the biscuit-speaks preparation marker is
`BISCUIT_SPEAKS_PREPARATION_WORKER`. Each marker contains only the path of its
private request record, never serialized JSON, speech text, command arguments,
or credentials. A worker entry seam validates that the referenced record is a
regular, non-linked file owned by the spool before reading it. The two Playa
entry seams run before argument parsing in every participating executable; the
biscuit-speaks seam runs before argument parsing in `so-you-say` and in any
binary that advertises preparation capability.

The lock order is always `queue.lock` before a worker-lock probe or worker-lock
release; no code waits for playback while holding `queue.lock`. Publication,
the enqueue-side ownership decision, and the worker's final empty recheck use
that order. A `Preparing` head slot has a deadline exactly ten minutes after
`enqueued_at`. The scheduler may wait for its atomic state replacement but may
not run a later sequence first. At the deadline it atomically marks the slot
failed, journals `preparation_timed_out`, removes private preparation material,
and advances. An explicit helper failure follows the same cleanup and advance
path with `preparation_failed`.

Ready jobs atomically move to in-flight before delegation. From that point they
are at-most-once: a scheduler or delegate crash quarantines the abandoned
in-flight request and report sidecar rather than replaying it. A missing,
replaced, non-absolute, incompatible, or unlaunchable enqueuer executable; a
delegate exit without a valid report; and report identity/version mismatch all
produce one failed journal outcome and cleanup, then advance the queue. None
may degrade playback options or fall back to the scheduler executable.

Ordering guarantee: all successfully published jobs for one OS user play in
the durable sequence allocated under `queue.lock`, including jobs from
different processes. Two clips never overlap because only the
`worker.lock` holder plays. A crash may lose the in-flight clip but never
causes an automatic duplicate replay.

### D5. biscuit-speaks exposes provider-agnostic fire-and-forget

`Speak` gains `async fn play_detached(self) -> Result<JobId, TtsError>`. The
executor trait gains one method with a default that returns
`TtsError::DetachedUnsupported`:

```rust
fn detached_job(&self, text: &str, config: &TtsConfig)
    -> impl Future<Output = Result<playa::SpoolJob, TtsError>> + Send;
```

- Implemented file-producing providers (Kokoro, EchoGarden, gTTS, and
  ElevenLabs) return a ready cached `PlayFile { delete_after: false }` on a
  cache hit. On a cache miss, `play_detached` first reserves the ordered
  `Preparing` slot and re-executes a detached biscuit-speaks helper. The helper
  inherits the requester's credentials and model environment without writing
  either to the spool, synthesizes to spool-owned temporary content or the
  shared cache, then atomically publishes `Ready` or `Failed` into the same
  sequence. The ten-minute D4 preparation deadline bounds abandonment.
- Implemented streaming providers (`say`, SAPI, and eSpeak) return
  `Command { … }` built from the same argument logic `speak` uses; text is
  passed by argument or temp file, never by a pipe the caller would have to
  keep open. Enum-only providers without a concrete `TtsExecutor` are not
  brought into scope by this fix.
- `play_detached` walks the failover strategy exactly like `play`, picks the
  first ready provider, asks it for the job, and enqueues it. Without the
  `playa` feature it returns `TtsError::NoAudioPlayer`, as `play` does for
  file producers today.
- `play_with_result` gains
  `SpeakResult::playback: Option<SpeakPlaybackReport>`. The always-compiled,
  serde-compatible biscuit-speaks DTO losslessly projects Playa's route,
  timing, and verdict without making the optional `playa` dependency part of
  `SpeakResult`'s unconditional public type. It is `Some` for file playback
  routed through Playa and `None` for direct streaming providers, whose audio
  duration is not known. Deserialization defaults the new field for stored
  older results.

`so-you-say --background` becomes `play_detached` and the playback self re-exec in
`biscuit-speaks/cli/src/main.rs` is removed; `playa --background` becomes
`enqueue` and `spawn_background_process` is removed. `playa --background`
keeps its meaning under D4. `so-you-say --background` returns after a ready job
is durably published or a preparing slot and detached helper are durably owned,
including cache misses.

### D6. Claudine uses the detached path everywhere it makes sound

- `DefaultLifecycleEmitter::emit_speech` and `emit_effect` enqueue instead of
  blocking. `say_blocking`, `play_effect_blocking`, and
  `run_blocking_with_timeout` are deleted with the 30 s / 15 s budgets they
  existed for; the deprecated `emit_lifecycle_signal` follows them. The
  composition thread continues as soon as a playable job or `Preparing` slot
  is durably published. Cache-miss synthesis happens afterward in the detached
  biscuit-speaks helper.
- `audio_phases` ordering is preserved by enqueuing the phases in order; the
  spool serializes them.
- `execute_speak_from_claudine` and `execute_sound_effect` in
  `dispatch/runner` enqueue instead of spawning Tokio tasks. `claudine
  handle` may then `process::exit` freely; the worker is a separate process
  group. This fixes the confirmed doorbell loss.
- `claudine` calls `playa::detached::run_if_worker()` first thing in `main`,
  before `completion::maybe_complete()`, because a spool worker must never
  parse argv or touch the launch-directory snapshot.
- If `enqueue` fails (no worker seam, unwritable temp dir), Claudine logs a
  warning and drops the best-effort audio on both lifecycle and hook paths.
  Falling back to a potentially 300-second blocking playback after deleting
  the lifecycle timeout would violate the prompt-return contract this change
  is intended to establish.
- A successfully reserved speech slot is followed by its configured effect in
  durable sequence even while synthesis is still running. A preparation
  timeout or failure is journaled and advances to the effect; it never lets the
  effect overtake the reserved speech slot.

### D7. Non-decisions

- The host-player ranking stays. Preferring `afplay` for mono would hide the
  problem on one OS; D1 removes speech from the host path on every OS with
  a device, and D2 makes any future misbehavior visible.
- The playa library default stays opt-in; consumers enable
  `native-playback` explicitly. Pulling rodio/ALSA into every Playa consumer
  is not necessary to fix the known callers.
- The rendezvous daemon is not the audio worker. It is optional and
  best-effort, and its lifetime is unrelated to audio.
- Lifecycle audio remains fire-and-forget only. A new blocking mode would
  expand the lifecycle DSL and is not required by the reported defect.
- Real-audio tests remain in `just test-real`; ordinary `just all` must stay
  deterministic on headless hosts.

## Requirements

### playa

- R1. The free playback functions route through the `Playa` builder's
  native-first pipeline; `playa_with_player*` remain host-only (D1a).
- R2. `probe_audio_metadata` returns the correct duration and channel count
  for every WAV in `playa/effects/` and for a 24 kHz mono 16-bit WAV; returns
  `None` for unrecognized bytes and does not fetch URL sources.
- R3. `PlaybackReport`, `PlaybackRoute`, and `PlaybackVerdict` are public;
  `Playa::play_with_report`, `play_async_with_report`, and report-returning
  explicit-player counterparts exist; existing signatures are unchanged and
  implemented on top of the report. The journal has an explicit versioned
  serialization projection rather than relying on unstable `Debug` output.
- R4. A `Truncated` verdict emits exactly one `warn!` and no error, on both
  the native and host routes.
- R5. `--audio-channels=stereo` appears in mpv arguments from both builders
  for positively identified mono input only, with the criterion-B comment;
  unknown, stereo, and multichannel sources do not receive it (D3).
- R6. `playa::detached` implements D4: `enqueue`, `run_if_worker`,
  `SpoolJob`, `JobId`, versioned envelopes, private per-user storage, the
  queue/worker lock protocol, bounded redacted journal, stale/in-flight job
  handling, and the Unix/Windows detach flags. `PLAYA_SPOOL_DIR` provides test
  isolation without weakening production path validation.
- R7. `playa spool` renders the journal and pending jobs with
  `TerminalRenderable` components; `playa --background` uses `enqueue`
  without losing playback options, and `playa` calls `run_if_worker`.
- R8. `playa/README.md`, `playa/docs/`, and the `playa` skill document the
  single pipeline, the report, conditional mpv workaround, delivery semantics,
  privacy model, and detached path. `docs/dependencies.md` gains `fs4`,
  `biscuit-file`, and `biscuit-hash` if they are new to playa.

### biscuit-speaks

- R9. The `playa` feature enables `playa/native-playback`; `play_audio_file`
  and `play_audio_bytes` use the `Playa` builder (D1).
- R10. `[package.metadata.ci.native]` adds `libasound2-dev` for
  `ubuntu-latest`.
- R11. `Speak::play_detached`, `TtsExecutor::detached_job`, and
  `SpeakResult::playback` per D5, with an implementation for every concrete
  executor in `providers/host` and `providers/cloud`. Enum-only providers
  remain explicitly unsupported.
- R12. `so-you-say --background` uses `play_detached`; the self re-exec is
  removed; `so-you-say` calls `run_if_worker` first in `main`.
- R13. The `biscuit-speaks` and `so-you-say` skills and READMEs document
  native playback, `play_detached`, and the serialization guarantee.

### claudine

- R14. `claudine` lib and cli enable `native-playback` on playa; CI native
  metadata adds `libasound2-dev` for Linux legs of both crates.
- R15. Lifecycle and dispatch audio per D6; blocking helpers and their
  timeouts removed; `main` installs the worker seam before completion
  handling.
- R16. `claudine/docs` lifecycle and hook-actions pages, plus the `claudine`
  skill (`lifecycle.md`, `hook-actions.md`), state that `say`, `say_first`,
  `effect`, `speak`, and `sound_effect` are fire-and-forget, serialized, and
  played natively when the host allows.
- R17. `just/notify.just` `_speak` and `_play_background` keep working
  unchanged.

## Testing

The regression must be caught by tests going forward. Two layers.

### L1 (no device, runs everywhere)

playa:

- `free_functions_take_native_route_when_available`: with a fake native
  backend seam (added at the playback boundary; dry-run cannot be used because
  it must short-circuit all playback work),
  assert `playa_explicit_with_options_async` reports route `Native` before
  any host player is consulted, and route `Host(..)` once the breaker is
  tripped. Guard for D1a; written red first.
- `mpv_args_upmix_only_mono` (sync and async builders): mono receives the
  flag; unknown, stereo, and six-channel fixtures do not. Guard for D3;
  written red first (`feedback_prove_new_tests_non_vacuous`).
- `probe_audio_metadata_*`: every bundled effect matches the build-time table;
  a hand-built 24 kHz mono header yields the computed duration and one channel;
  multichannel and garbage fixtures prove the positive and `None` cases.
- `verdict_*`: table-driven over (expected, elapsed, speed) including the
  boundary at 0.9× − 250 ms and `Unverified` when expected is `None`.
- `truncated_verdict_warns_once_and_returns_ok`: a fake host player (the
  test binary re-executed via `current_exe`, per
  `feedback_l2_probe_no_production_bin`) exits after 100 ms while the probed
  duration is 2 s; the report is `Truncated`, the function returns `Ok`, and
  a `tracing` test subscriber captured exactly one warning.
- `spool_*` with an injected fake playback executor and isolated
  `PLAYA_SPOOL_DIR`: jobs from one and multiple processes journal in allocated
  sequence order; concurrent publication produces unique IDs; enqueue during
  the worker's final empty check cannot strand a job; only one worker plays;
  a stale job is discarded; a crash-left in-flight job is quarantined rather
  than replayed; unsupported schema versions are quarantined; journal content
  is redacted and bounded; and the worker exits when empty. Security tests
  reject a symlinked root/job and assert Unix `0700` permissions. Every test
  keys its spool dir on `{pid}-{nanos}-{atomic}`.
- `dry_run_detached_has_no_side_effects`: no spool, journal, audio cache file,
  or child process is created, while enqueue returns a dry-run job ID.
- `run_if_worker_*`: `None` without the marker, records the installed seam in
  a normal process, and `Some(0)` in the purpose-built worker fixture.
- `detach_flags_*`: `#[cfg(unix)]` asserts the worker's process group
  differs from the parent's; `#[cfg(windows)]` asserts it has no console.
  Ordinary L1 tests, not tier-prefixed.

biscuit-speaks:

- `playa_feature_enables_native_playback`: a compile-time check that
  `playa::detached` and the native route are reachable with the crate's
  `playa` feature (a `cfg(feature = "playa")` test referencing
  `PlaybackRoute::Native`).
- `detached_job_*` per provider: with the provider's executable stubbed on
  `PATH` (pattern from `9a2fc8426`), each concrete executor produces
  `PlayFile` for file producers or `Command` with the same arguments `speak`
  builds for streaming providers; enum-only providers return
  `DetachedUnsupported`.
- `play_detached_walks_failover_like_play`: first ready provider wins; none
  ready yields the same error `play` yields.
- `speak_result_playback_serde`: old serialized results without `playback`
  deserialize to `None`; a Playa report round-trips through
  `SpeakPlaybackReport`; direct streaming speech leaves it `None`.
- CLI: `so-you-say --background` with stubbed synthesis and the purpose-built
  spool worker returns before a deliberately blocked playback is released;
  the journal then shows one job. This does not weaken `PLAYA_DRY_RUN`.

claudine:

- Lifecycle executor tests keep their `Recorder` emitters; add one test with
  `DefaultLifecycleEmitter`, an isolated spool, and the fake worker proving
  `say` + `effect` journals effect then speech, `say_first` + `effect` the
  reverse, and that emission returns while playback is deliberately blocked.
- Dispatch runner: `speak` and `sound_effect` actions enqueue (journal has
  the job) and `execute_actions` returns before the job plays.
- `handle` process test: run the built `claudine` with a
  `human_in_the_loop` payload, fake worker, and isolated spool; after the
  process exits, the journal contains the doorbell job. Guard for the
  confirmed hook-path loss.

### `real_` tier (needs an audio sink, skips clean without one)

playa gains a `real_` module (retiring `#[ignore = "requires real audio
device"]` for these cases); `just test-real` already selects the tier:

- `real_native_plays_mono_wav_to_the_end`: generate a 2 s **silent**
  24 kHz mono WAV in a temp dir; `Playa::from_path(..).play_with_report()`
  reports route `Native` and verdict `Complete`. This is the acceptance
  test for D1 on Ken's host.
- `real_every_installed_host_player_plays_mono_wav_to_the_end`: the same
  file through a report-returning explicit-player entry point for each player
  `match_available_players` returns; assert `Complete`. `force_host()` alone
  is insufficient because it still chooses only the highest-ranked player.
  On this host today the test fails for mpv without D3 and passes with it.
  This is the test that would have caught the regression.
- `real_stereo_effect_plays_to_the_end`: control case with a bundled effect
  at zero volume where the route supports it.

biscuit-speaks `real_` test: with the OS default provider stack,
`Speak::new(<short sentence>).play_with_result()` reports `Complete` on
route `Native`. Skips when no provider is ready.

Gating is route-specific: the native test skips when
`native_audio_available()` is false; each host-player case skips only when
that player is absent; the biscuit-speaks case skips when no concrete provider
is ready. All skip when `PLAYA_DRY_RUN` is set, and hard-fail when
`PLAYA_REAL_AUDIO_REQUIRED=1` so a CI leg with a sink cannot go green by
skipping. Names use the `real_` prefix so `just test-real` selects them and
`just test` does not.

### Verification for this fix

- `just lint`, `just test`, `just test-real` in `playa`, `biscuit-speaks`,
  and `claudine`; `just ci-local --lint-only` first (the no-features clippy
  build is where feature gating breaks; D1b adds new gates).
- On Ken's host, before D1/D3: the host-player `real_` test fails for mpv
  with a `Truncated` verdict naming about 1.3 s missing. After D1: a
  so-you-say built from the branch plays the 5.97 s Kokoro WAV on route
  `Native` with verdict `Complete`. After D3: the host-player test passes for
  mpv too. Record all three runs in `evidence.md` next to this spec.
- Manual: `just _speak "the fix is in"` and a lifecycle `say:` prompt both
  finish the sentence; two back-to-back `so-you-say --background` calls play
  sequentially without overlap; `playa spool` shows both with `Native` /
  `Complete`.

## Cross-platform notes

- Native: cpal targets CoreAudio, WASAPI, and ALSA. The device-open
  deadline and circuit breaker already exist and now protect speech; a host
  with no device (CI runner, headless server) trips to host players or
  fails cleanly with `NoCompatiblePlayer`, exactly as the `playa` CLI does
  today.
- Detach: Unix uses `CommandExt::process_group(0)`; Windows uses
  `CommandExt::creation_flags`. Both are in `std`; no new dependency.
- Locks: `fs4` advisory locks behave the same on all three OSes for the two
  single-writer critical sections used here. A lock held by a crashed worker
  is released by the OS. A stale pending job is discarded; an in-flight job
  is quarantined and never replayed automatically.
- Windows has no `mpv` by default, but users who install it get D3 too.
  SAPI is a `Command` job (PowerShell `SpeechSynthesizer`), so
  `play_detached` works there without playa's players.
- Spool paths are per-user and losslessly encoded. Build them with
  `Path::join`, never string concatenation, never compare them textually, and
  use biscuit-file's portable-path renderer only for redacted display
  (`reference_windows_path_spelling_traps`).

## Ratified Phase 1 decisions

### 1. Cache-miss synthesis runs in a detached helper

**Ratified: option 3.** The reviewed specification and the owner's explicit
instruction to execute Phase 1 approve the recommended reserved-slot design.
The alternatives below remain as decision history; they are not implementation
options.

The requested contract says Claudine hands audio off and continues
immediately. D5 as currently scoped only hands off *playback*: Kokoro,
EchoGarden, gTTS, and ElevenLabs still complete readiness checks and synthesis
before `play_detached` can enqueue a file. A cache miss can therefore block the
composition path for seconds, and the synchronous `LifecycleEmitter` cannot
simply await the proposed async method. This is a major behavior/API decision
and must be resolved before implementing D5/D6.

1. **Keep synthesis in the caller.**

   - Pros: smallest change; credentials and model environment never enter a
     persistent job; existing provider methods and failure reporting stay
     intact.
   - Cons: violates immediate return on cache misses; does not fit the
     synchronous lifecycle emitter without another blocking runtime bridge;
     slow cloud or local synthesis can still delay prompt completion.

2. **Spawn one detached synthesis helper per cache miss, then enqueue when
   ready.**

   - Pros: caller returns quickly; helper naturally inherits the requesting
     process's credentials and model environment; Playa remains unaware of
     TTS.
   - Cons: completion order can differ from request order, so speech/effect
     ordering is lost unless the spool first reserves a sequence slot; helper
     crashes need cleanup and timeout semantics.

3. **Reserve a spool slot, then prepare it in a detached biscuit-speaks
   helper.** The pending job begins as `Preparing`; the helper inherits the
   requester's environment, synthesizes to a spool-owned file, and atomically
   marks that same sequence slot `Ready` or `Failed`. The Playa worker waits
   for the head slot with a bounded preparation deadline before advancing.

   - Pros: satisfies immediate return, preserves total audio order, keeps
     credentials out of the job file, and avoids a dependency cycle between
     Playa and biscuit-speaks.
   - Cons: largest implementation; requires a biscuit-speaks worker entry seam,
     preparation timeout/failure journal states, and cleanup for abandoned
     `Preparing` jobs.

**Selected: option 3.** It is the only option that satisfies both of
Ken's explicit requirements—immediate handoff and serialized ordering—without
persisting credentials or making Playa depend on biscuit-speaks. Use a typed,
versioned preparation record containing text and non-secret `TtsConfig`; keep
the spool private as required by D4, redact text from diagnostics, set a
bounded preparation deadline, and continue to the next job after journaling a
failure. If this complexity is rejected, option 1 is acceptable only if the
spec explicitly weakens "continues immediately" to "continues immediately
after synthesis" and retains a bounded lifecycle wait.

### 2. The enqueuer executable owns delegated playback

**Ratified: option 3.** One scheduler preserves global ordering and delegates
each job to its losslessly recorded absolute enqueuer executable. The
alternatives below remain as decision history; they are not implementation
options.

D4 shares one per-user spool across `playa`, `so-you-say`, `claudine`, and
future library consumers, but re-executes whichever binary first wins the
worker lock. Those binaries need not have identical Playa features. For
example, a custom `playa` build may enqueue ducking while an already-running
Claudine worker has no ducking backend. Schema compatibility alone does not
prove execution capability.

1. **Require every spool host to compile one fixed worker feature set.**

   - Pros: worker remains simple and plays in-process; one queue preserves
     ordering.
   - Cons: forces optional native/ducking dependencies onto every consumer;
     third-party binaries can violate the assumption; startup must still
     detect old or incompatible binaries.

2. **Partition the spool by a capability fingerprint.**

   - Pros: each worker handles only jobs it can execute; no delegation
     protocol is needed.
   - Cons: separate queues can play concurrently and reorder audio, directly
     violating the user-level serialization contract.

3. **Use the lock holder only as the scheduler and delegate each job to the
   absolute executable recorded by its enqueuer.** The scheduler waits for
   that child and receives its versioned report through a private sidecar.

   - Pros: preserves one global order and the exact feature set/options the
     enqueuer accepted; supports future consumers without expanding Playa's
     default features.
   - Cons: adds a second internal worker mode and report handoff; a replaced or
     deleted executable makes the job fail and must be journaled; process
     launch overhead occurs per clip.

**Selected: option 3.** Correct ordering and option fidelity are core
requirements, while optional-feature uniformity is not an enforceable library
contract. The recorded executable path must use D4's lossless encoding and
private-spool validation, the child must be launched directly with null stdio,
and an unavailable/incompatible executable must fail that job rather than let
another binary silently degrade its options.

## Suggested phases

0. Resolve both open questions and update D4–D6 plus their acceptance tests
   before implementation begins.
1. playa: D1a single pipeline, D2 metadata probe and report, D3 conditional
   mpv mono upmix,
   the `real_` tests, and evidence runs on Ken's host (red, then green for
   native, then green for mpv).
2. playa: D4 detached spool, worker seam, CLI `spool` and `--background`.
3. biscuit-speaks: D1b feature, D5 `play_detached`, `so-you-say
   --background`, CI native metadata, tests.
4. claudine: D1b feature, D6, delete blocking helpers, tests, docs and
   skill updates, CI native metadata.
5. Documentation and dependency-doc drift across all three areas.
