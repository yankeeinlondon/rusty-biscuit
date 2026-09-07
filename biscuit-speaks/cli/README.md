# so-you-say CLI

`so-you-say` speaks command-line arguments or up to 10,000 characters from
stdin using biscuit-speaks' OS-aware provider selection and failover.

```bash
so-you-say "Hello"
so-you-say --provider kokoro --voice af_heart "Hello"
so-you-say --meta "Report the foreground provider and playback"
so-you-say --background "Queue this and return immediately"
```

Foreground file-producing providers use Playa's native-first playback pipeline
and report route, expected/elapsed duration, and completion verdict through
`SpeakResult::playback`. Direct streaming providers such as macOS `say`, Windows
SAPI, and eSpeak have no duration report.

`--background` publishes speech into Playa's private per-user spool. Cache hits
are ready immediately; cache misses reserve their sequence before an internal
helper synthesizes. Playa serializes all participating processes without
overlap, and playback survives this CLI exiting. Preparation failure or the
ten-minute deadline fails that slot and advances the queue. Delivery remains
best-effort after handoff.

Use `playa spool` for redacted queue and journal status. Speech text can exist in
private preparation records but is never printed there. `PLAYA_DRY_RUN=1`
returns without synthesis, playback, cache, spool, journal, or subprocess side
effects.

Supported detached providers are Kokoro, EchoGarden, gTTS, ElevenLabs, macOS
`say`, Windows SAPI, and eSpeak. Deferred enum-only providers report detached
playback as unsupported instead of silently falling back.
