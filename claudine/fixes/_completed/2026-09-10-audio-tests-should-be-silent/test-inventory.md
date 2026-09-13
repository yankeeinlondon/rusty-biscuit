# Audio test inventory

## Claudine

| Tests | Tier and execution | Speech/effect and controls | Isolation and lifetime |
| --- | --- | --- | --- |
| `compose_caller_file_provenance::shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target` and `shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan` | L1, real `claudine compose` with a successful fixture Goose agent | Copy the literal production `implement-suggestions.md`. Its success lifecycle speaks “the review findings in … were implemented successfully” and plays `bong`. Child-only `PLAYA_DRY_RUN=1` now suppresses audio handoff while retaining template and lifecycle processing. | Fixture HOME/repo; explicit private spool; helper asserts the spool never exists after success or failure. These tests do not test a TTS provider or voice. |
| `default_emitter_publishes_audio_in_phase_order_without_waiting_for_playback` | L1, real default lifecycle emitter; publication only | Pinned fixture eSpeak, recognizable test speech, real doorbell asset. Lifecycle has no volume field; held worker lock prevents execution. | Private spool; shared test-local guard removes pending records under queue lock before releasing the scheduler lock, including unwind. Exact phase and sequence assertions retained. |
| `audio_actions_publish_in_order_and_return_before_worker_execution` | L1, real dispatch publication; Unix-gated fixture shell | Pinned fixture eSpeak, recognizable test speech, doorbell at explicit `0.0` volume, existing speed retained. | Private spool and the same locked cleanup; exact order and return-before-execution assertions retained. |
| `handle_human_in_the_loop_leaves_durable_doorbell_job_after_exit` | L1 CLI process, real durable handoff | TTS disabled; doorbell now explicitly `0.0`; persisted playback volume asserted. | Private CLI fixture HOME/spool, scheduler lock held across child exit and pending record inspection; jobs removed before unlock. |
| Lifecycle/dispatch handoff-warning tests | L1; real handoff fails before playback | Fixture eSpeak speech uses recognizable test wording. Effect failure retains its original nonzero setting to test error behavior. | An ordinary file supplied as spool ensures publication fails; warning-count assertions remain. |
| Other lifecycle, executor, sequence and loop unit tests | L1 recording emitters | Capture speech/effect actions; never invoke audio backend. | Inert emitter boundary; no playback queue. |
| `level2_lifecycle_control` shipped route tests | L2 existing terminal harness | Existing shipped `implement-plan` fixture strips `say`, `effect`, and shell actions; `shipped_prompt_route_drift` guards this delta. | Existing harness containment; no new real audio run required for these assertions. |
| `shipped_prompt_contract` | L1 template composition | Uses composition dry-run; lifecycle `say`/`effect` never execute. | Existing preparation-only boundary. |

The production-template source trace identifies a concrete test route with the
exact reported wording. It is strong source evidence for the likely origin,
not a claim to have reproduced the user's audible event. Production prompt text
is unchanged. The real default speech provider on this route previously came
from normal provider discovery, which explains why it need not match the voice
selected for the user's ordinary Claudine use.

## Biscuit-speaks CLI

`detached_background` is L1 despite being a Rust integration target. It pins
Kokoro and `af_heart`, exposes only copied fake `kokoro-tts` and `mpv` executables
on child PATH, and writes invalid wave bytes. Native decoding cannot play those
bytes; any host fallback is the fixture mpv. The speech text is now a clearly
identified test message. It retains cache-miss reservation, blocked synthesis,
ordered same-slot playback, and terminal journal coverage. The fixture cache
key is isolated from ordinary user messages. Its dry-run companion creates no
spool or worker. Child `TMPDIR`, `TMP`, and `TEMP` point to a private cache
instead of deleting a potentially shared host-cache entry. Normal completion
and a simulated assertion failure during synthesis both release the fake
programs, then await an empty pending/in-flight set and the scheduler lock's
release. The post-unwind test also requires the completed journal transition.
The scheduler waits for delegated playback, so this terminal boundary also
establishes the fake player's completion. No executable pending work remains
when temporary storage is removed.

Source review also confirms no double panic in either cleanup guard: I/O
failures become test failures during normal return, and destructors do not
raise a second panic while unwinding. Runtime validation is recorded separately
in `evidence.md`.


On macOS the same background harness also covers fake `say` with the explicitly
selected Samantha voice. It verifies synthesis-only WAV arguments and stdin
text, reservation-before-synthesis-return semantics, the same ready sequence,
and `--soft` reaching both the serialized `0.5` playback setting and the fake
mpv `--volume=50` argument. Both Kokoro and Say use this controlled intermediate
volume; invalid audio and fixture-only PATH make these checks silent. The
library's separate volume tests cover explicit zero. Say is excluded on other
OSes because its provider readiness contract requires macOS.

## Change history checked

The September 4 detached speech/audio changes added the publication and
background CLI cases (`49a1dbe8a`, `e372ba4e1`, `52efa64c6`), with cross-platform
fixture follow-ups (`fa4e5d6fe`, September 5 `e9b8cb3fc`). Git history places the
literal shipped `implement-suggestions.md` provenance coverage on September 2
(`3746aa2f6`, `c1c1f05e7`). This is consistent with the reported recent-test
window, without proving which historical test run emitted the announcement.

## Stronger subprocess boundary regressions

The background harness also has a test-only `audio-enqueuer` executable role.
It installs the production worker seam and invokes
`Speak::with_volume(VolumeLevel::Explicit(0.0)).play_detached()`. Captures assert
zero in the private preparation payload, the ready in-flight job, and the
actual delegated fake mpv arguments. This tests all process boundaries without
adding a production CLI flag or using audible input.

Each active case now copies its enqueuer executable into the private fixture
bin directory, so scheduler, preparation, delegate, and player processes have
unambiguous executable ownership even after reparenting. Cleanup first allows
cooperative completion. On timeout it revalidates PID/start-time and canonical
executable ownership, terminates only processes in that exact bin directory,
and polls until none remain live before removing runnable queue records under
both locks. Direct CLI children are reaped by `output()`; detached descendants
are observed through exit (zombies cannot execute and are reaped by their OS
parent). A forced-timeout regression first observes blocked synthesis and
instructs the fake program to ignore release, then verifies forced cleanup
leaves no live owned process, pending job, or in-flight job. Cleanup errors are
reported and recorded even during unwinding rather than silently discarded.

## Focused macOS verification

`just _test biscuit-speaks-cli --test detached_background -j 2 --no-fail-fast`
passed all seven cases in 1.412 seconds (nextest run
`f28652ca-6079-4a94-9895-acd2e9a9a461`). This includes exact-zero process handoff,
normal and unwind completion for Kokoro/Say, and forced-timeout termination.

The first Say fixture run exposed an additional isolation gap: the CLI resolved
requested “Samantha” to “Samantha (Enhanced)” through the host voice cache.
The fixture now sets `BISCUIT_SPEAKS_CACHE` to a private file and fake
`say -v '?'` advertises only Samantha. This preserves real voice resolution
while preventing the user's inventory from changing the test's selected voice.

Final package verification on macOS: `just test -j 2` in `biscuit-speaks/`
passed 487 tests with 19 skipped (7.924 seconds execution); `just lint` passed.
Logs: `/tmp/audio-tests-biscuit-speaks-final-test.log` and
`/tmp/audio-tests-biscuit-speaks-final-lint.log`. These are controlled L1 and
lint results; real-provider and other-OS evidence is recorded by the
orchestrator in `evidence.md`.
