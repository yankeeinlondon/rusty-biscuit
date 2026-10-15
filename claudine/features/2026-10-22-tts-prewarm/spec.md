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
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 3
completed: true
human_review: false
pipeline_step: 12
depends-on:
    - 2026-10-19-speak-prepare
    - 2026-10-20-lifecycle-grammar
related:
    - 2026-09-30-configuration
implemented: true
---
# TTS Pre-warm

A lifecycle `say` that fires at the end of a long agent run should avoid
unnecessary synthesis delay. Claudine already knows many of those phrases
when it prepares the document. This feature submits them for background
preparation through biscuit-speaks'
[Speak::prepare](../../../biscuit-speaks/lib/src/speak.rs), the speech builder's
cache preparation API. If preparation finishes and playback resolves the same
settings, the event reuses cached audio. This is best effort: a busy Playa
playback queue can still delay audible speech, and an event never waits for
pre-warm to finish.

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

An **invocation** is one top-level compose, inline-compose, sequence, or
provider-wrapper command. Its **bootstrap** is the initial shell-free read of
frontmatter used to run `initialize`; the **stabilized reread** is preparation
of the document after initialization. Pre-warm follows both reads, while one
worker belongs to the whole command.

- **When.** Submit candidates when the shell-free bootstrap has parsed the
  lifecycle, before `initialize` fires. Walk again after the stabilized reread
  and whenever preparation produces a new effective lifecycle or frontmatter
  snapshot. Initialization can change both; warming only the bootstrap would
  miss the phrases that will actually execute. Neither walk composes the body
  or runs initialization itself.
- **What.** Every `say`/`speak` phrase in every stack **except `initialize`**:
  `start`, `blocked`, `success`, `failure`, `finalize`, the `loop:` block's
  loose keys and `gate:`, and a sequence task's `setup:`/`teardown:` while
  those exist. Walk parsed lifecycle actions, including root dictionary
  bundles, positional and named speech arguments, and both `then` and `else`
  bodies at every supported nesting level. Do not evaluate guards or walk
  arbitrary frontmatter looking for keys named `say`. The guard decides
  whether a phrase plays, not what it says. The grammar dependency removes
  `say_first`; this feature does not restore it or add hook-event pre-warm.
- **Sequences.** During sequence preflight, collect candidates from every
  statically reached prompt document and every task's `setup:`/`teardown:`.
  Reuse the existing graph; do not prepare prompt bodies early, resolve new
  references, fetch remote content, or run task setup. Submit all eligible
  candidates before step one launches, without waiting for synthesis.
  References unavailable to preflight are covered when their document is
  actually prepared. Repeated prompt paths may have different task inputs:
  evaluate eligibility against the applicable inputs, not path alone.
- **Targets and re-entry.** A `proxy` target's phrases are collected when
  that document is itself prepared, after its `with` overlay has been applied,
  never by the caller. Retry/resume preparation and loop refresh use the same
  collection entry point when they obtain a new snapshot. A future `prep` directive
  uses this entry point when implemented; implementing `prep` is not part of this
  feature. An old candidate may finish after a handoff, but it can only write
  its own cache entry, never publish playback or mutate the active document.
- **De-duplication.** One preparation attempt per distinct (resolved text,
  playback TTS config) per top-level invocation, shared across sequence tasks,
  loop iterations, retries, and proxy handoffs. Record candidates when queued,
  so rediscovery while preparation is in progress does not enqueue a duplicate.
  A failed attempt is not retried by pre-warm in that invocation; event-time
  playback retains its normal behavior. Different text or configuration is a
  new candidate. The existing cache remains responsible for reuse across runs.

### Early and late phrases

Use Darkmatter's authored-value preparation and binding classification rather
than scanning strings for variable names. Its
[static_variable_reads](../../../darkmatter/lib/src/markdown/compose/expression/absence.rs)
walk reports variable reads in every expression branch, including bracket
indexes; it does not report function calls or prove an expression is safe to
execute. Inspect those separately before evaluating any candidate.

- A phrase is **early** when it is literal text, or every root it reads is
  frontmatter (`fm`, or a bare name classified as an implicit frontmatter
  property), `ctx`, or `env`. Resolve against an owned snapshot of the
  effective document inputs and the context used by preparation. Use the same
  interpolation, typed-value handling, text conversion, and empty-text
  suppression as lifecycle playback. Produced strings remain data: never
  evaluate braces or shell syntax returned by an expression a second time.
- Every declared global other than those early roots is **late**: `err`,
  `timing`, `current`, `loop`, `group`, and any root added later. Consult the
  binding view (the scope's declarations of reserved roots), so a new global
  cannot accidentally become an implicit frontmatter property. A phrase reading a late root is synthesized
  at event time. `fm.err` still reads a document property and may be early.
- Sequence-injected values are late even when exposed as frontmatter rather
  than a separate root. Check the resolved frontmatter dependency, including
  literal bracket access and bare names. If a computed index or a whole-`fm`
  read could include a task-dependent value, conservatively skip it. During
  preflight also skip inputs not yet available as resolved data; do not
  execute frontmatter interpolation or shell expansion to obtain them.
  Use only `ctx`/`env` snapshots that are established for that document; if
  preflight cannot establish the eventual snapshot, defer the dependent phrase
  to ordinary document preparation. Reconsider unresolved phrases there, but
  sequence-injected values remain late under this feature's classification rule.
- Any authored shell expansion or **any function call** makes the phrase
  ineligible, even in an expression branch that would not execute. Operators,
  indexing, fallback expressions, and ternaries over early values remain
  eligible. No guards, functions, shell runners, or side-effect dispatchers
  are invoked to decide eligibility.
- A phrase whose speculative resolution fails is skipped silently, without
  changing ordinary validation or suppressing the event's own error. The event
  resolves it again when it fires. Missing values follow existing lifecycle
  semantics; do not invent a pre-warm-specific error or speech spelling.

**Reader's note.** Excluding only side-effect verbs was insufficient: some
Darkmatter read-only functions probe a login shell, filesystem, or remote
service, and a call with no variable arguments passes a root-only check.
Skipping all function calls is the conservative first implementation. It
keeps preparation free of speculative command execution and network reads;
function-based phrases still play normally. A later extension can permit
functions with an established pure-evaluation contract.

`ctx` and `env` use the document's captured binding values; pre-warm does not
recapture host facts or read the live process environment. The environment
capture is defined by `2026-10-18-fm-namespace`, the frontmatter namespace
feature: `env` is captured once at the start of each document's lifecycle
(each sequence task, group leg, and `proxy` target captures its own; loop
iterations, retries, and resumes reuse it). Pre-warm uses that same captured
map as the document's other consumers.

### Running it

- **Background ownership.** One invocation-owned worker processes candidates
  serially, shared by all sequence tasks and re-entries. Collection and
  submission never await synthesis and never delay `initialize`, preflight,
  or provider launch on a TTS operation. Provider discovery, default lookup,
  synthesis, and cache access happen in the worker. Blocking provider work
  must not occupy the launch task's executor thread. There is no worker per
  document and no detached pre-warm daemon.
- **Failure and shutdown.** Pre-warm never changes the outcome or exit code.
  A failed `prepare` produces at most one tracing `warn` record per invocation,
  with no terminal line and no resolved phrase text, secrets, or raw provider
  response. Continue with later candidates. Stop accepting work and cancel
  unfinished preparation when the invocation finishes or is interrupted;
  shutdown does not drain this queue or await synthesis. Use the dependency's
  cancellation cleanup for synthesis children and temporary files. Completed
  cache files remain reusable. This queue is separate from outbound message
  delivery, whose existing shutdown drain remains required.
- **Same config as playback.** Build each candidate's configuration through
  Claudine's
  [tts_config_from_settings](../../lib/src/composition/lifecycle/audio.rs),
  which supplies lifecycle playback settings from the bridged `tts` setting.
  Carry the pre-warm enablement separately: the existing bridge's `None` for
  `tts: false` does not mean warming is disabled. Discard the builder returned
  by `prepare`; the later speech action resolves and publishes normally.
  Cross-object reuse depends on the same provider, effective synthesis
  settings, and cache directory. The speech-preparation dependency preserves
  existing default selection and permits a successful no-op when an external
  default cannot be resolved safely; it does not guarantee every default voice
  or rate becomes concrete. Preparation errors do not select a fallback
  provider, while playback retains its existing fallback rules.
- **No playback changes.** Preparation reserves no Playa queue slot, starts
  no scheduler, and publishes no playback job. Event-time speech preserves
  its existing queue ordering and survives the requesting process as today.
  If speech fires before warming finishes, it follows its ordinary cache-miss
  path; it does not join or cancel the preparation. Concurrent preparation
  and playback may duplicate synthesis or a billed call. Atomic cache writes
  prevent partial reusable output; cross-process duplicate suppression is
  outside this feature.
- **Skipped** under `--dry-run`, when `prewarm: false`, and under the existing
  `PLAYA_DRY_RUN` values (`1`, `true`, `TRUE`, `yes`). Check these before
  speculative resolution or starting the worker. The dependency also checks
  dry run before provider selection, cache access, or subprocess creation.
  The existing `CliProcessFixture` default remains sufficient; no test switch
  is added.

### The setting: `tts` in `~/.claudine/config.json`

Decided 2026-10-10: the switch lives in Claudine's user config file,
`~/.claudine/config.json` (JSON5 accepted), as part of `tts`. It is never a
frontmatter key. There is no `claudine.toml`.

`tts` keeps its two shapes, a boolean and an object, with today's meaning.
Decided 2026-10-11: `say` behavior does not change, `"tts": false` keeps
today's behavior, and the object form gains the optional `prewarm` switch.
Decided 2026-10-11: `"tts": true` means the default provider, and the
object's `provider` becomes optional and defaults to the default provider, so
`prewarm` can be set without choosing a provider. Nothing else about `tts`
changes in this spec:

```json5
{ tts: true }
{ tts: false }
{ tts: { provider: "kokoro", voice: "af_heart" } }
{ tts: { provider: "kokoro", voice: "af_heart", prewarm: false } }
{ tts: { prewarm: false } }
```

| Key | Type | Default | Meaning |
|---|---|---|---|
| `provider` | string | the default provider | as today when present; absent selects the default provider, as `true` does |
| `voice` | string or `{ male, female }` | absent | as today |
| `gender` | `male` \| `female` | `female` | selects the gendered voice alias, as today |
| `prewarm` | boolean | `true` | **new**: `false` turns pre-warm off |

Every shape's effect:

| Value | Hook `speak` actions | Lifecycle `say` config | Pre-warm |
|---|---|---|---|
| `tts` absent | off (as today: absent is `false`) | auto-detect (as today) | **on** |
| `true` | on, auto-detect (as today) | auto-detect (as today) | **on** |
| `false` | off (as today) | auto-detect (as today) | **on** |
| `{ provider, … }` without `prewarm` | on, with the object's provider/voice (as today) | the object's provider/voice (as today) | **on** |
| `{ provider, …, prewarm: false }` | as the same object without `prewarm` | as the same object without `prewarm` | **off** |
| object without `provider` (e.g. `{ prewarm: false }`) | on, with the default provider (as `true`) | as `true` | per its `prewarm` (default **on**) |

Pre-warm is on unless `prewarm: false` is written, because it follows the
audio lifecycle `say` will actually play, and lifecycle `say` plays under every
shape today (`bridge_tts_settings` maps `false` to no settings, and the
lifecycle executor then uses an auto-detected configuration). This feature does
not change whether, or with which configuration, lifecycle `say` or hook
`speak` plays.

Compatibility: every existing `config.json` keeps loading with the same
meaning. `true`, `false`, and objects with `provider` (and optional `voice`,
`gender`) deserialize and behave as before; an object without `provider`,
previously rejected, now loads and uses the default provider; `deny_unknown_fields` still rejects
misspelled keys, including any key other than `prewarm` that is not already
accepted. The terminal configuration editor's TTS tab preserves `prewarm` on save and
when changing an object's provider, voice, or gender. All object reconstruction paths carry
it forward. A deliberate switch to a boolean drops the object and therefore
restores pre-warm's default of on; the TTS tab explains that the hook-speech
toggle does not disable lifecycle speech or warming. No new TUI toggle is
required by this feature.

**Coordination.** Claudine's configuration is reworked after this pipeline
(`2026-09-30-configuration`, which is not part of this pipeline). That rework
adapts to this `tts` shape (decided 2026-10-10); its inventory entry for
`tts` is refreshed when that spec is next touched.

### Limits

- A phrase naming a late root is never pre-warmed.
- An early phrase whose frontmatter is changed by `set` before its event
  resolves to different text than was pre-warmed: a graceful cache miss — the
  correct text is synthesized at event time — never wrong audio. `ctx` and
  `env` remain captured within that document lifecycle; a newly prepared
  document uses its own supplied bindings and may produce different text.
- Phrases for events that never fire are synthesized anyway. For a cloud
  provider this may be a billed synthesis request for each unfired phrase.
- `proxy … with` cannot switch the TTS provider or voice; neither is
  frontmatter.
- Providers that produce no file (eSpeak, SAPI), or whose external defaults
  cannot safely be resolved, gain nothing; their `prepare` is a no-op.
- Synthesis may consume CPU or make network requests even for an unfired
  branch, and may duplicate event-time work. Serial preparation limits this
  invocation to one active pre-warm synthesis, but does not guarantee that
  speculative work has no effect on provider execution time.
- Cache entries still have no eviction or TTL.
- `prewarm` exists only in the object form. A user with `tts: true` who wants
  pre-warm off writes `{ prewarm: false }`, which keeps the default provider.
  A user with `tts: false` or no `tts` who wants pre-warm off writes the
  object form, which also enables hook `speak` actions with the default
  provider; the configuration rework after this pipeline revisits the shape.

## Out of scope

- `Speak::prepare`, safe default voice/rate resolution, and the cache-consult fix:
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
   exactly the eligible early phrases, once each, and none from `initialize`.
   Cover `speak` aliases, root bundles, and named speech arguments too.
2. Phrases reading `err`, `timing`, `current`, `loop`, `group`, or a sequence
   overlay value are not prepared; phrases reading only `fm`/bare names, `ctx`,
   or `env` are, with the resolved text, when their snapshots are established
   and they satisfy the remaining eligibility rules.
3. A phrase with an authored `$( … )` or any function call is not prepared,
   including a shell-probing read-only function, a zero-argument call, and a
   call in an untaken branch. No shell, function, guard, or side effect runs.
   Literal text and data returned by a single interpolation retain playback's
   spelling and are not re-evaluated. Empty results are not submitted.
4. A sequence of three prompts submits every eligible early phrase from all
   three during preflight, before step one launches. A blocked preparation
   does not block that launch. Unresolved inputs are skipped without composing
   bodies or fetching content; ordinary preparation later discovers newly
   resolved early phrases.
5. A `proxy` target's phrases are prepared with its `with` overlay applied, and
   only when the target is prepared.
6. `prewarm: false` in `config.json` and `--dry-run` each prepare nothing.
7. Configuration: each row of the shape table deserializes to the stated hook,
   lifecycle, and pre-warm behavior; existing `true`, `false`, and
   `{ provider, voice, gender }` configs load unchanged and the existing
   `tts` configuration tests pass without their assertions being edited; an
   unknown key in the object (for example `enabled`) is rejected; an object
   without `provider` (for example `{ prewarm: false }`) loads, uses the
   default provider for hook `speak` exactly as `true` does, and applies its
   `prewarm` value; the config TUI round-trips
   `prewarm` across save and provider/voice/gender edits; deliberately changing
   to a boolean restores the documented default.
8. Preparation errors do not change the outcome or exit code, print nothing
   to the terminal, and produce at most one sanitized tracing warning per
   invocation. Later candidates still run; failed candidates are not retried
   by pre-warm. Event-time synthesis remains available.
9. Pre-warm does not delay launch: with a recorder whose `prepare` blocks, the
   provider launch still happens (asserted through a recording stub provider).
10. Under the fixture's `PLAYA_DRY_RUN=1`, composing a shipped prompt with
    lifecycle `say` creates no audio cache file, voice-capability cache, spool
    entry, or synthesis subprocess. Isolate cache observations with fixture-owned
    `TMPDIR`/`TMP`/`TEMP` and `BISCUIT_SPEAKS_CACHE`; do not inspect the user's
    caches. Test accepted dry-run values and the disabled setting before
    discovery using injected recorders, on the existing environment schedule.
11. Docs: `claudine/docs/topics/flow-control/lifecycle.md` (or the audio page it
    links) describes pre-warm, early/late phrases, and the limits;
    the configuration docs and `.claude/skills/claudine/` describe the `tts`
    object, its keys, and the shape table.
12. The bootstrap and stabilized reread share one worker: initialization
    changes to a phrase discover the new resolved text without repeating an
    existing candidate. Retry, loop refresh, proxy handoff, repeated prompt
    paths, and parallel sequence tasks share invocation-wide de-duplication.
    Cover bare and bracket frontmatter reads, new declared globals, and
    sequence-dependent values accessed through `fm`.
13. An event before preparation finishes uses normal speech publication
    without waiting. A successful prepare never publishes audio, and a no-op
    provider/default is treated as success. Shutdown and interruption request
    cancellation without draining the queue; completed cache entries survive.
    Use controllable fake futures and publishers, not elapsed-time thresholds
    or real speech. Provider-child cleanup is covered by the dependency's tests.
14. `just test` and `just lint` are green in `claudine/`; run `just test-l2`
    if implementation changes the existing detached-playback boundary. No new
    performance spike is needed: call recordings prove reuse and the existing
    latency figures already explain why ahead-of-time synthesis helps.

## Open questions

None.
