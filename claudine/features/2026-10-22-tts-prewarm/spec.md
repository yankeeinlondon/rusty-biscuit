---
area: claudine
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
pipeline_step: 12
depends-on:
    - 2026-10-19-speak-prepare
    - 2026-10-20-lifecycle-grammar
related:
    - 2026-09-30-configuration
---
# TTS Pre-warm

A lifecycle `say` that fires at the end of a long agent run should speak
immediately, not after a second or three of synthesis. Claudine already knows
most of those phrases when it prepares the document. This feature synthesizes
them up front, through biscuit-speaks' `Speak::prepare`, so that when an event
fires its audio is already in the cache.

```yaml
success:
    - effect: "chime"
    - say: "{{ fm.title }} is done"      # known at prepare: pre-warmed
failure:
    - say: "failed: {{ err.msg }}"       # needs err: synthesized at event time, as today
```

This material was split out of the lifecycle-ergonomics feature (now
`2026-10-20-lifecycle-grammar`) during the 2026-10-10 restructure. The owner's
framing, unchanged: a warm cache adds nothing for `initialize` and little for
`start` or `blocked`, which fire moments after preparation; the value is real
for `success`, `failure`, `finalize`, and the loop gate, whose phrases are
known long before they are needed, and across a sequence it extends to every
event of every step. Audible first-utterance latency today is roughly
0.2–0.8 s for macOS `say`, 0.5–3 s for Kokoro on CPU, and 1–3 s for gTTS,
Echogarden, and cloud providers.

## Decided

### The walk

- **When.** As soon as a document's lifecycle stacks have been parsed during
  preparation, before `initialize` fires.
- **What.** Every `say`/`speak` phrase in every stack **except `initialize`**
  (it fires before a prepare could complete): `start`, `blocked`, `success`,
  `failure`, `finalize`, the `loop:` block's loose keys and `gate:`, and a
  sequence task's `setup:`/`teardown:` while those exist. The walk is total: it
  enters both `then` and `else` bodies at every nesting level, whatever the
  guard says. The guard decides whether a phrase plays, not what it says.
- **Sequences.** During sequence preflight the same walk runs over every prompt
  document in the preflight graph as it is built today, so a phrase in step five
  is warm before step one starts.
- **Targets of `proxy` and `prep`.** A target's phrases are pre-warmed when that
  document is itself prepared, after its `with` overlay has been applied, never
  by the caller.
- **De-duplication.** One `prepare` per distinct (resolved text, TTS config) per
  run.

### Early and late phrases

Classification uses Darkmatter's current binding model: the phrase is prepared
as an expression value, `static_variable_reads` lists the variables it reads,
and each read's root decides.

- A phrase is **early** when it is static text, or every root it reads is
  frontmatter (`fm`, or a bare name through the implicit namespace), `ctx`, or
  `env`. Early phrases are resolved against the launch frontmatter and context
  and prepared.
- Every other root is **late**: `err`, `timing`, `current`, `loop`, `group`,
  the sequence overlay values, and any root added later. A phrase that reads a
  late root is never pre-warmed; it is synthesized at event time exactly as
  today.
- A phrase containing a `$( … )` shell expansion, or calling a Darkmatter
  side-effect verb, is never pre-warmed: pre-warm runs no shell and causes no
  side effect.
- A phrase whose resolution fails is skipped silently; the event resolves it
  again when it fires.

### Running it

- **Fire-and-forget.** Pre-warm runs on a background task, one phrase at a
  time, and never delays `initialize`, preflight, or the provider launch. It
  never fails the run and never changes the exit code; a failed `prepare` is
  recorded as one tracing `warn` record per run, not a terminal line. If the
  process exits first, unfinished work is abandoned; biscuit-speaks' atomic
  cache writes make that safe.
- **Same config as playback.** Each phrase is prepared with the TTS
  configuration the lifecycle executor will play it with (the one
  `tts_config_from_settings` builds from the bridged `tts` setting), so the
  cache key matches at play time (`2026-10-19-speak-prepare` resolves default
  voice and rate to concrete values).
- **Skipped** under `--dry-run`, and when the setting below turns it off.
- **Silent in tests.** Under `PLAYA_DRY_RUN=1`, `Speak::prepare` synthesizes
  nothing, writes no cache entry, and spawns nothing, so the existing
  `CliProcessFixture` default (child-local `PLAYA_DRY_RUN=1` and a private
  `PLAYA_SPOOL_DIR`) keeps every test silent. No new switch is added.

### The setting: `tts` in `~/.claudine/config.json`

Decided 2026-10-10: the switch lives in Claudine's user config file,
`~/.claudine/config.json` (JSON5 accepted), as part of `tts`. It is never a
frontmatter key. There is no `claudine.toml`.

`tts` keeps its two shapes, a boolean and an object, with today's meaning.
Decided 2026-10-11: `say` behavior does not change, `"tts": false` keeps
today's behavior, and the object form gains exactly one key, the optional
`prewarm` switch. Nothing else about `tts` changes in this spec:

```json5
{ tts: true }
{ tts: false }
{ tts: { provider: "kokoro", voice: "af_heart" } }
{ tts: { provider: "kokoro", voice: "af_heart", prewarm: false } }
```

| Key | Type | Default | Meaning |
|---|---|---|---|
| `provider` | string | required | as today |
| `voice` | string or `{ male, female }` | absent | as today |
| `gender` | `male` \| `female` | as today | as today |
| `prewarm` | boolean | `true` | **new**: `false` turns pre-warm off |

Every shape's effect:

| Value | Hook `speak` actions | Lifecycle `say` config | Pre-warm |
|---|---|---|---|
| `tts` absent | off (as today: absent is `false`) | auto-detect (as today) | **on** |
| `true` | on, auto-detect (as today) | auto-detect (as today) | **on** |
| `false` | off (as today) | auto-detect (as today) | **on** |
| `{ provider, … }` without `prewarm` | on, with the object's provider/voice (as today) | the object's provider/voice (as today) | **on** |
| `{ provider, …, prewarm: false }` | as the same object without `prewarm` | as the same object without `prewarm` | **off** |

Pre-warm is on unless `prewarm: false` is written, because it follows the
audio lifecycle `say` will actually play, and lifecycle `say` plays under every
shape today (`bridge_tts_settings` maps `false` to no settings, and the
lifecycle executor then uses an auto-detected configuration). This feature does
not change whether, or with which configuration, lifecycle `say` or hook
`speak` plays.

Compatibility: every existing `config.json` keeps loading with the same
meaning. `true`, `false`, and objects with `provider` (and optional `voice`,
`gender`) deserialize and behave as before; `deny_unknown_fields` still rejects
misspelled keys, including any key other than `prewarm` that is not already
accepted. The config TUI's TTS tab round-trips the object form without
dropping `prewarm`.

**Coordination.** Claudine's configuration is reworked after this pipeline
(`2026-09-30-configuration`, which is not part of this pipeline). That rework
adapts to this `tts` shape (decided 2026-10-10); its inventory entry for
`tts` is refreshed when that spec is next touched.

### Limits

- A phrase naming a late root is never pre-warmed.
- An early phrase whose frontmatter is changed by `set` before its event
  resolves to different text than was pre-warmed: a graceful cache miss — the
  correct text is synthesized at event time — never wrong audio. The same holds
  when `ctx` or `env` changed between preparation and the event.
- Phrases for events that never fire are synthesized anyway. For a cloud
  provider that is a billed call of a few dozen characters per unfired phrase.
- `proxy … with` cannot switch the TTS provider or voice; neither is
  frontmatter.
- Providers that produce no file (eSpeak, SAPI) gain nothing; their `prepare` is
  a no-op.
- Cache entries still have no eviction or TTL.
- `prewarm` exists only in the object form, whose `provider` stays required.
  A user with `tts: true`, `tts: false`, or no `tts` who wants pre-warm off
  writes the object form, which also selects that provider for hook `speak`
  actions; the configuration rework after this pipeline revisits the shape.

## Out of scope

- `Speak::prepare`, default voice/rate resolution, and the cache-consult fix:
  `2026-10-19-speak-prepare`.
- Any change to `say` or to what `tts: false` means: none (decided
  2026-10-11).
- The wider configuration rework, which follows this pipeline:
  `2026-09-30-configuration`.

## Acceptance criteria

All L1 unless marked; `just test` / `just lint` in `claudine/`; portable to
macOS, Linux, native Windows, and WSL2; no audio device, network, or model
download needed; no window focus; no new CI cell. Library tests observe
`prepare` calls through an injected recorder; CLI tests use `CliProcessFixture`
and assert `fixture.audio_spool()` stays absent.

1. A document with `say` phrases in `start`, `success`, `failure`, `finalize`,
   `gate:`, a `then` body, an `else` body, and a depth-3 nested body prepares
   exactly the early phrases, once each, and none from `initialize`.
2. Phrases reading `err`, `timing`, `current`, `loop`, `group`, or a sequence
   overlay value are not prepared; phrases reading only `fm`/bare names, `ctx`,
   or `env` are, with the resolved text.
3. A phrase with `$( … )` or a side-effect verb call is not prepared, and no
   shell or side effect runs during preparation.
4. A sequence of three prompts prepares every early phrase of all three during
   preflight, before step one launches.
5. A `proxy` target's phrases are prepared with its `with` overlay applied, and
   only when the target is prepared.
6. `prewarm: false` in `config.json` and `--dry-run` each prepare nothing.
7. Configuration: each row of the shape table deserializes to the stated hook,
   lifecycle, and pre-warm behavior; existing `true`, `false`, and
   `{ provider, voice, gender }` configs load unchanged and the existing
   `tts` configuration tests pass without their assertions being edited; an
   unknown key in the object (for example `enabled`) is rejected; an object
   without `provider` is rejected as today; the config TUI round-trips
   `prewarm`.
8. A `prepare` that errors does not change the run's outcome or exit code and
   prints nothing to the terminal.
9. Pre-warm does not delay launch: with a recorder whose `prepare` blocks, the
   provider launch still happens (asserted through a recording stub provider).
10. Under the fixture's `PLAYA_DRY_RUN=1`, composing a shipped prompt with
    lifecycle `say` creates no audio cache file and no spool entry, on all four
    environments.
11. Docs: `claudine/docs/topics/flow-control/lifecycle.md` (or the audio page it
    links) describes pre-warm, early/late phrases, and the limits;
    the configuration docs and `.claude/skills/claudine/` describe the `tts`
    object, its keys, and the shape table.
12. `just test`, `just test-l2`, and `just lint` are green in `claudine/`.

## Open questions

None.
