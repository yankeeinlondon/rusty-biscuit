---
fix: 2026-09-10-audio-tests-should-be-silent
area: claudine
areas:
        - biscuit-speaks
        - playa
        - claudine
deferred_perf_measurement: false
implementation_1: "2026-09-10T14:54:19-07:00"
---

# Log: Audio Tests Should Be Silent

## Implementation of Review Findings #1

> **started at:** 2026-09-10T14:54:19-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-10-audio-tests-should-be-silent/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review recorded three findings, all rated **High**:
        - finding 1 — publication cleanup bypasses queue ownership and can suppress failure
        - finding 2 — native mute is not verified at the output gain boundary
        - finding 3 — muted EchoGarden and gTTS playback tests are unreachable from the real-resource tier
- affected package areas taken from the specification's `areas` frontmatter: `biscuit-speaks`, `playa`, `claudine`
- findings are implemented serially, each by a dedicated subagent, with `just test` and `just lint` run in the impacted package areas
- starting the work on 'finding-1: publication cleanup bypasses queue ownership' at 14:56:26-07:00
        - correction (orchestrator): the subagent reported reading `review-0.md`; no such file exists in the fix directory and `review-1.md` is the only review present, so the findings implemented are those of `review-1.md`
        - confirmed the protocol lock ordering in `playa/lib/src/detached/mod.rs` before copying it: `enqueue_state_at` and `replace_preparing` both take `queue.lock` exclusively and only then probe/act on `worker.lock`, and `run_scheduler_with`'s final-empty path holds `worker.lock` and then takes `queue.lock`; the fixture therefore matches the scheduler's ordering (worker held for its whole life, queue acquired only inside `clear_pending`)
        - created the one shared publication fixture at `tools/test-toolkit/src/spool.rs`, re-exported as `test_toolkit::LockedAudioSpool`; it creates the spool root `0o700` on unix, blocks until it owns `worker.lock`, holds that lock for its whole lifetime, and in `clear_pending()` acquires `queue.lock` before the directory scan and releases it only after the last removal
        - `Drop` runs `clear_pending()` and propagates failure via `expect` unless `std::thread::panicking()`, so a cleanup failure is reported rather than discarded but never double-panics over the assertion that started the unwind — this closes the `let _ = self.clear_pending()` error-suppression half of the finding
        - accessors kept to what the call sites actually need: `root()`, `clear_pending()`, `open_lock()`, `queue_lock_path()`, `worker_lock_path()`; the last three exist so a test can take `queue.lock` exactly as a publisher does and probe `worker.lock` without re-deriving protocol file names
        - **decision** — added `fs4 = "0.13"` (the version every other workspace member pins; there is no `[workspace.dependencies]` table) as an *unconditional* dependency of `test-toolkit` rather than feature-gating it like the crate's binary-only `clap`/`sysinfo`
                - `fs4` resolves only to `rustix` + `windows-sys`, which every dev-dependent already builds, so the lockfile graph gains nothing
                - gating it would have forced `required-features` onto `test-toolkit`'s own test target, which no canonical recipe passes
        - adopted the fixture and deleted every divergent copy:
                - `playa/cli/tests/detached_cli.rs` — removed the local `PublicationGuard` and `worker_lock()`; added `test-toolkit` as a dev dependency of `playa-cli`
                - `biscuit-speaks/lib/tests/detached_phase4.rs` — removed `PendingJobsGuard`, the hand-rolled `0o700`/`worker.lock` setup, and the manual success-path pending-deletion loop; the success path now calls the fixture's locked `clear_pending()` and asserts `snapshot().pending.is_empty()`
                - `claudine/test-support/locked_audio_spool.rs` — deleted; its three `include!` call sites (`claudine/cli/tests/detached_audio.rs`, `claudine/lib/src/dispatch/runner/tests.rs`, `claudine/lib/src/composition/lifecycle/tests/audio_emission.rs`) now `use test_toolkit::LockedAudioSpool`. `test-toolkit` was already a dev dependency of both claudine crates
        - the `include!` removal also collapses three compiled copies of the old unwind test into one, which is why claudine's L1 count moves from 6,861 to 6,859
        - dropped the now-unused `fs4` dev dependencies from `claudine/lib`, `claudine/cli`, and `biscuit-speaks/lib` (grep confirmed the deleted guard was their only consumer); `playa-cli` keeps its own, still used by the scheduler-exit probe in `requester_exit_is_followed_by_worker_failure_journal_and_clean_exit`
        - added the synchronized unwind regression the finding asks for, in two places:
                - `tools/test-toolkit/tests/audio_spool.rs::publication_fixture_clears_a_record_committed_under_the_queue_lock`
                - `playa/cli/tests/detached_cli.rs::publication_unwind_removes_pending_before_releasing_worker_ownership` (the old inert-file body was replaced, not kept alongside)
                - **why both** — `tools/test-toolkit/Cargo.toml` declares `[package.metadata.ci] gates = false`, so CI runs none of its tests; the `playa` mirror is the copy an actual CI gate executes
        - the regression's schedule is deterministic rather than sleep-tuned: a helper thread takes `queue.lock` exactly as `enqueue_state_at` does, signals that it holds it, then waits for an `owner_finished` flag *or* a 250 ms deadline before committing its `*.pending.json` record and releasing
                - correct cleanup blocks on `queue.lock`, so `owner_finished` cannot be set while the holder waits; the holder always runs to its deadline, the record is committed, and cleanup then removes it
                - lock-free cleanup returns immediately, `catch_unwind` returns, `owner_finished` flips, and the holder commits *after* the scan — the surviving record fails the assertion every time
                - the 250 ms budget is therefore paid only by the passing path, and the broken path fails in ~10 ms
        - the test asserts `!pending.exists()` **and then** re-acquires `worker.lock`, proving no runnable record outlived worker ownership
        - **non-vacuity evidence** (hard requirement) — temporarily reverted `clear_pending()` to the pre-fix shape (`read_dir` + `remove_file` with no `queue.lock`) and re-ran both regressions:
                - `test-toolkit::audio_spool` — `publication_fixture_clears_a_record_committed_under_the_queue_lock` **FAILED** in 0.010 s at `tools/test-toolkit/tests/audio_spool.rs:109` with `a record committed under the queue lock survived fixture cleanup`
                - `playa-cli::detached_cli` — `publication_unwind_removes_pending_before_releasing_worker_ownership` **FAILED** in 0.014 s at `playa/cli/tests/detached_cli.rs:206` with the same assertion message
                - in the same broken run, the inherited inert-file test `publication_fixture_clears_jobs_before_unlocking_on_unwind` **PASSED** — which is exactly the review's point that the pre-existing regression cannot distinguish broken cleanup from protocol-correct cleanup
                - the fix was restored afterwards and both tests pass again (0.259 s / 0.267 s, i.e. the 250 ms deadline plus overhead, confirming the queue lock is genuinely being waited on)
        - comment drift fixed alongside the behavior change:
                - the stale `playa/cli/tests/detached_cli.rs:80` comment ("The file keeps worker ownership until pending work is removed, even on unwind.") went with the guard it described; the new module and fixture docs state the two-lock contract instead
                - `claudine/docs/topics/testing.md` and `.claude/skills/claudine/SKILL.md` both pointed at the deleted `claudine/test-support/locked_audio_spool.rs`; both now name `test_toolkit::LockedAudioSpool` and describe the queue-lock requirement
                - dependency docs updated for the crate moves: root `docs/dependencies.md`, `claudine/docs/dependencies.md`, `biscuit-speaks/docs/dependencies.md`, `playa/docs/dependencies.md`; `tools/test-toolkit` has no `docs/dependencies.md`, so its `README.md` gained a `LockedAudioSpool` API section instead
        - verification commands and results:
                - `cd playa && just test` → **188 passed**, 8 tier-filtered, exit 0
                - `cd playa && just lint` → exit 0
                - `cd biscuit-speaks && just test -j 2` → **487 passed** (1 slow), 19 tier-filtered, exit 0
                - `cd biscuit-speaks && just lint` → exit 0
                - `cd claudine && just test --no-fail-fast` → **6,859 passed** (6 slow), 11 tier-filtered, exit 0
                - `cd claudine && just lint` → exit 0
                - `tools/test-toolkit` has no canonical `test` recipe — its justfile defines only `verify-nextest-config`, matching its `gates = false` CI policy — so it was verified with `cargo nextest run -p test-toolkit` → **126 passed**, 2 skipped, exit 0, plus `cargo clippy -p test-toolkit --all-targets -- -D warnings` → exit 0
                - no pre-existing failures were encountered in any area; every suite above was green
        - `cargo fmt --check` reports 33 diffs inside `tools/test-toolkit`, all pre-existing: the count is identical with and without this change's edits, and none of the drifted files are the two new ones. `cargo fmt` was not run; the three formatting nits in `spool.rs` and `audio_spool.rs` were applied by hand so this change adds no drift of its own
- work completed for 'finding-1: publication cleanup bypasses queue ownership' at 15:12:43-07:00
- starting the work on 'finding-2: native mute not verified at the output gain boundary' at 15:14:46-07:00
        - only one file changed: `playa/lib/src/native_player.rs`. No other package's sources were touched, so the finding is contained to Playa's native output boundary
        - **seam design** — a gain observer (`NATIVE_GAIN_SEAM`) that stands in for the output device *and* stops the flow in the gap between `Player::set_volume` and `Player::append`
                - the review asks for proof that `0.0` is applied *before* submission, so the seam had to sit after `set_volume` and before `append`; the existing `NATIVE_BACKEND_SEAM` cannot do this because `play_native` consults it before the device path even begins, so it can only re-observe `PlaybackOptions` — the same config echo the review calls insufficient
                - the two device paths were folded into one shared `submit_to_mixer(mixer, source, options)` that both `play_source` (cached-default) and `play_source_one_shot` (channel override) now call. This is the task's "option 1" shape (extract the connect/apply step) rather than a `PlayerSink` trait: `Player::append` is generic over the source type and the drain loop already has its own `PlayerProgress` trait, so a second trait would have had to duplicate both
                - **why a device-free mixer** — the boundary is only reachable with a `rodio::mixer::Mixer` in hand, and CI's Linux and Windows runners have no audio sink. `rodio::mixer::mixer(channels, sample_rate)` is public and returns a `(Mixer, MixerSource)` pair with no device behind it, so `gain_seam_mixer` hands one back when (and only when) the observer is installed. The `Player`, the `set_volume`/`set_speed` calls, and the read-backs are all real rodio; only the device is absent. This keeps the tests L1 and hermetic on all four target OSes
                - **what is asserted is read back off the player, not echoed from config** — `AppliedGain` carries `Player::volume()`, `Player::speed()`, and `Player::len()`. `len() == 0` is the actual evidence that nothing had been submitted when the gain was observed; `volume() == 1.0` (rodio's untouched default) is the evidence that no gain was applied when none was requested
                - `AppliedGain.path` (`CachedDefault` / `ChannelOneShot`) is stamped by whichever of the two device paths asked for the mixer, so each test names the path it exercised instead of inferring it from `options.channel`
                - production behavior is unchanged: every seam symbol and both consult points are `#[cfg(test)]`, and the non-test build of `submit_to_mixer` is the same four statements in the same order as the two inlined copies it replaced. `cargo clippy --all-targets` builds the lib target without `cfg(test)` and reported no unused bindings or dead code
                - the two seams are documented as complementary and mutually exclusive: the gain seam's docs state that it must not be combined with `install_native_backend_seam_for_tests`, and it follows the same convention of requiring `native_audio::lock_native_audio_test_state`
        - **GitNexus blast radius** (MCP available, index at `d76e90e81`; `includeTests: true`):
                - `play_source` — **HIGH**, 8 impacted, 2 direct (`play_from_bytes`, `play_from_file`), 4 modules
                - `play_source_one_shot` — **LOW**, 4 impacted, 1 direct (`play_source`), 2 modules
                - `with_cached_default_mixer` — **HIGH**, 5 impacted, 2 direct, 3 modules
                - `play_native` — **HIGH**, 10 impacted, 5 direct, 4 modules (`Detached` reached indirectly)
                - every impacted symbol is inside `playa/lib/src/native_player.rs` except `Playa::attempt_native` in `playa/lib/src/playa.rs`; no signature in that set changed, so the HIGH ratings describe the fan-in of the native path rather than exposure created by this change
        - **feature gating** — none was needed on the tests themselves. `playa/lib/src/lib.rs:27` already gates the whole `native_player` module on `native-playback`, and the area's `just test` runs the library with `--features native-playback,async` (matching `[package.metadata.ci.tests] features` in `playa/lib/Cargo.toml`), so the gain path *is* built and the six new tests *do* run under the canonical recipe. Confirmed by the L1 count moving 188 → 194
                - one gap worth naming: `just lint` delegates to `_lint`, which runs `cargo clippy -p playa --all-targets` with **default** features, so it never compiles `native_player` at all. That is pre-existing and out of scope for this finding, so the recipe was left alone and the feature-enabled clippy run below was added as supplementary evidence
        - **non-vacuity evidence** (hard requirement) — two separate probes, each reverted immediately after:
                - probe 1, drop the gain: replaced `player.set_volume(vol)` with an empty body. **4 of 6 FAILED** in ~0.011 s each — both mute tests and both nonzero tests, with `left: 1.0 / right: 0.35` and `left: 1.0 / right: 0.0`. The two unity-gain tests correctly still passed, since they request no volume and therefore cannot detect a dropped `set_volume`
                - probe 2, break the ordering: moved the seam observation to *after* `player.append(source)`. **2 of 6 FAILED** with `assertion left == right failed: gain must be applied before any source is submitted / left: 1 / right: 0` — so the `queued == 0` assertion genuinely distinguishes before-append from after-append and is not a tautology
                - both probes restored; all 6 pass in 0.055 s total with no audio device involved
        - **comment drift** — `play_source_one_shot`'s existing docblock still describes the channel bypass accurately and was left as is. The new `submit_to_mixer` doc states the contract the code cannot express (gain precedes submission, and that the seam is what proves it) and deliberately avoids an intra-doc link to the `#[cfg(test)]` observer, since the docblock itself is not `cfg`-gated
        - **observed but not in scope** — `playa/lib/src/sfx_player.rs:221-224` applies volume through the same `Player::connect_new` / `set_volume` shape and has no equivalent boundary proof. Finding 2 names only `native_player.rs:407-452`, so this was left untouched and is recorded here as an adjacent gap for a future review cycle
        - verification commands and results:
                - `cd playa && just test` → **194 passed** (188 before, +6 new), 8 tier-filtered, exit 0
                - `cd playa && just lint` → exit 0
                - `cd playa && just doctest` → exit 0
                - supplementary, **outside the canonical recipe**, because `just lint` does not build the feature that contains the changed code: `cargo clippy -p playa --features native-playback,async --all-targets -- -D warnings` → exit 0
                - `cd biscuit-speaks && just test -j 2` → **487 passed**, 19 tier-filtered, exit 0
                - `cd biscuit-speaks && just lint` → exit 0
                - `cd claudine && just test --no-fail-fast` → **6,859 passed**, 11 tier-filtered, exit 0
                - `cd claudine && just lint` → exit 0
                - claudine and biscuit-speaks were run because both compile against `playa`; neither shows any change, which is the expected result for an edit whose only non-`cfg(test)` effect is the extraction of a private helper
                - `just test-real` was **not** run: it can produce audible output on this host and the finding is satisfied at L1. The existing real-resource completion tests (`biscuit-speaks/lib/tests/real_detached_phase4.rs`, `playa/lib/tests/real_playback_reports.rs`) were left untouched, as the review requires
- work completed for 'finding-2: native mute not verified at the output gain boundary' at 15:29:48-07:00
- starting the work on 'finding-3: muted EchoGarden and gTTS tests unreachable from the real tier' at 15:31:06-07:00
        - files changed: `biscuit-speaks/lib/src/test_support.rs` (new), `biscuit-speaks/lib/src/lib.rs` (one `#[cfg(test)] mod` declaration), `biscuit-speaks/lib/src/providers/host/echogarden.rs`, `biscuit-speaks/lib/src/providers/host/gtts.rs`, plus docs: `biscuit-speaks/README.md`, `.claude/skills/biscuit-speaks/SKILL.md`, `.claude/skills/rust-testing/SKILL.md`, and this fix's `evidence.md`. No `playa` or `claudine` source was touched, so only biscuit-speaks needed its gates re-run
        - **confirmed the finding's premise before acting** — `just/devops.just`'s `_test_real` runs `cargo nextest run -p <pkg> -E "$(just _tier_filter real)" --no-tests=pass` and never passes `--ignored`, and `_tier_filter real` prints exactly `test(/(^|::)real_/)`. An `#[ignore]`d `test_*` therefore ran in no canonical recipe at all: `just test` filters ignored tests out, `just test-real` does not opt into them
        - **required-resource switch design** — `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts`, a comma-separated, case-insensitive, whitespace-trimmed list, implemented once in `biscuit-speaks/lib/src/test_support.rs` and used by both providers
                - **why a named list rather than per-provider `ECHOGARDEN_REAL_REQUIRED=1` / `GTTS_REAL_REQUIRED=1`** — one variable and one parser instead of one per provider, and the repository already has this exact shape: `BISCUIT_TEST_REQUIRED_BACKENDS` names the L2 backends whose absence must be fatal while every other backend still skips. The `rust-testing` skill records *why* that beat the all-or-nothing `BISCUIT_TEST_LEVEL_REQUIRED`, and the same argument applies here — a host that can run `echogarden` may have no route to Google's TTS endpoint
                - **how it composes with the global switch** — `real_provider_required()` returns true if `PLAYA_REAL_AUDIO_REQUIRED=1` (Playa's existing all-or-nothing switch, already honored by `lib/tests/real_detached_phase4.rs` and `playa/lib/tests/real_playback_reports.rs`) **or** the provider is named in the list. The global switch keeps its current meaning; the list is the finer-grained addition
                - **typo handling copied from `Backend::parse`** — entries are matched exactly against `KNOWN_REAL_PROVIDERS`, and an unrecognized entry panics instead of being read as "not required". A silently-ignored typo would disable the very switch it was set to enable
                - `KNOWN_REAL_PROVIDERS` lists only `echogarden` and `gtts` — the providers that actually have `real_*` coverage — rather than the whole `HostTtsProvider` enum, so naming a provider with no real test is reported as the mistake it is. The module doc says to extend the list when a provider gains its first `real_*` test
                - **why `#[cfg(test)] mod test_support` in the library rather than `test-toolkit`** — the vocabulary is biscuit-speaks' own provider identifiers, and both consumers are unit-test modules inside `src/`, which see crate-private items. Putting it in `test-toolkit` would have moved TTS provider names into a crate that knows nothing about TTS. The cost is that `lib/tests/*.rs` integration tests cannot reach it; nothing needed it there today, and `real_detached_phase4.rs`'s two-line `required()` uses only the global switch, so it was left alone. Promote the module if an integration test ever needs the list
                - `dry_run_enabled()` and `DRY_RUN_VAR` are `#[cfg(feature = "playa")]`: only the playback tests consult them, and those exist only with `playa`. Without that gate the default-feature `just lint` fails on `dead_code`
        - **per-test disposition** — every test in the two files, and what was decided:
                - `echogarden::test_speak_integration` → **renamed** `real_echogarden_speaks_muted`, `#[ignore]` removed. Real synthesis and playback
                - `echogarden::test_speak_with_voice` → **renamed** `real_echogarden_speaks_muted_with_requested_voice`, `#[ignore]` removed. Real synthesis and playback
                - `echogarden::test_list_voices_integration` → **renamed** `real_echogarden_lists_installed_voices`, `#[ignore]` removed. Runs the real `echogarden` binary to enumerate voices; no audio, but it is a real resource, so it belongs in the tier
                - `gtts::test_gtts_provider_speaks` → **renamed** `real_gtts_speaks_muted`, `#[ignore]` removed. Real synthesis and playback
                - `gtts::test_list_voices_integration` → **renamed** `real_gtts_lists_installed_voices`, `#[ignore]` removed. Runs the real `gtts-cli --all`; no audio, real resource
                - `gtts::test_check_connectivity` → **renamed** `real_gtts_reports_reachable_network`, `#[ignore]` removed, and **given an assertion**. It reached the network and then asserted nothing (`println!` of the result with a comment saying it could not assert either way), so as written it could not fail. It now skips when Google is unreachable or `gtts-cli` is absent, and otherwise asserts that a ready provider reports ready and that the successful probe refreshed the cached `connectivity_ok` flag
                - `echogarden::test_is_ready_check` → **left alone**. `which::which("echogarden")` only; no audio, no external resource beyond a PATH lookup. L1 is correct
                - `echogarden::test_speak_uses_tempdir_not_namedtempfile` and every `process_vits_voices` / parsing / voice-selection test in both files → **left alone**. Pure, hermetic, L1
                - `gtts::test_is_ready_without_binary` → **left alone, but flagged**. It is L1 and produces no audio, but `GttsProvider::is_ready` performs a real 2-second-timeout TCP connect to `translate.google.com:443` whenever `gtts-cli` happens to be installed, so on such a host an L1 test touches the network. The finding names only the two playback tests, so this was not changed; recorded here as an adjacent gap
        - **richer-assertion investigation** — a playback report *is* reachable, so no test was left on `assert!(result.is_ok())`
                - `TtsExecutor::speak` returns `Result<(), TtsError>` and can prove nothing beyond success. But both providers implement `speak_with_result(&self, text, config) -> Result<SpeakResult, TtsError>`, and under `#[cfg(feature = "playa")]` both end with `.with_playback(playback)` where `playback` comes from `crate::playback::play_audio_file_with_report(...) -> Result<SpeakPlaybackReport, TtsError>` (`echogarden.rs:371`, `gtts.rs:313`). `SpeakPlaybackReport` carries `route` (`Native` / `Host(String)` / `DryRun`), `expected_millis`, `elapsed_millis`, and `verdict` (`Complete` / `Truncated { missing_millis }` / `Unverified`) — the same evidence `real_detached_phase4.rs` asserts for Kokoro
                - the three playback tests therefore switched from `speak` to `speak_with_result` and now assert: the resolved `provider` identity, `route != DryRun` (proving playback was not silently skipped), and the codec; the echogarden voice test additionally asserts `result.voice.name == "Heart"`, proving the requested voice survived resolution
                - **verdict strength differs deliberately by codec.** EchoGarden produces WAV, whose duration is always known, so both echogarden tests assert `verdict == Complete`, matching the existing `real_say_provider_speaks_muted` and Kokoro precedents. gTTS produces MP3, and `PlaybackVerdict::for_timing` returns `Unverified` whenever the source exposes no trustworthy duration (`playa/lib/src/report.rs:54`), so demanding `Complete` there would be an over-assertion that could fail on a provisioned host for a reason unrelated to mute. `real_gtts_speaks_muted` instead asserts `!matches!(verdict, Truncated { .. })` — the failure that verdict can still distinguish — with a comment saying why
                - the three playback tests are `#[cfg(feature = "playa")]`: without it `speak_with_result` returns `Err(TtsError::NoAudioPlayer)` before any playback, so there is no boundary to observe. `just test-real` passes `--features playa` for this package, so they are built where they run
        - **skip handling** — every skip branch in all six tests calls one helper, `test_support::skip_or_require(provider, reason)`, which `assert!`s that the provider is not required and only then prints `SKIP: <provider>: <reason>`. That makes it impossible to add a new skip path that bypasses the switch. The playback tests also gained the `PLAYA_DRY_RUN` branch the canonical `real_detached_phase4.rs` has, accepting the same four spellings (`1`, `true`, `TRUE`, `yes`)
        - **stale comments corrected** — `// Produces audio - run manually` (×2, echogarden) and `// Produces audio and requires internet - run manually` (gtts) were wrong after the earlier mute change and were deleted with their attributes. Each file's section banner now reads "Real-resource tests (`real_*`, selected by `just test-real`)" and states that synthesis and playback are real but explicitly muted, plus which switch turns a skip into a failure. The banners were also moved *below* `test_is_ready_check` / `test_is_ready_without_binary`, which are L1 and were sitting under the old "Integration tests" heading. `grep` confirms no `#[ignore]`, `Produces audio`, or `run manually` remains in either file
        - **`cargo nextest list` selection evidence** (the finding's central claim; list-only, nothing executed):
                - command: `cargo nextest list -p biscuit-speaks --features playa -E "$(just _tier_filter real)"`, run from the worktree root
                - result — **8 tests selected, up from the 2 the review measured**:
                        - `biscuit-speaks providers::host::echogarden::tests::real_echogarden_lists_installed_voices`
                        - `biscuit-speaks providers::host::echogarden::tests::real_echogarden_speaks_muted`
                        - `biscuit-speaks providers::host::echogarden::tests::real_echogarden_speaks_muted_with_requested_voice`
                        - `biscuit-speaks providers::host::gtts::tests::real_gtts_lists_installed_voices`
                        - `biscuit-speaks providers::host::gtts::tests::real_gtts_reports_reachable_network`
                        - `biscuit-speaks providers::host::gtts::tests::real_gtts_speaks_muted`
                        - `biscuit-speaks providers::host::say::tests::real_say_provider_speaks_muted`
                        - `biscuit-speaks::real_detached_phase4 real_kokoro_provider_reports_muted_native_complete`
                - this confirms the prompt's premise directly: `test(/(^|::)real_/)` **does** select a `real_*` test declared in an inline `#[cfg(test)] mod tests` inside `src/`, because the module path makes the marker the first segment of the final name. `real_say_provider_speaks_muted` in `say.rs` was already relying on this, so the pattern is not new — echogarden and gtts were simply never converted
                - the complementary direction was checked too: `cargo nextest list -p biscuit-speaks --features playa -E "$(just _tier_filter L1)" | grep -c real_` → **0**, so the six converted tests left L1 rather than being counted twice
        - **non-vacuity evidence** (hard requirement) — four probes, all executed, none of which can produce audio:
                - probe 1, the three non-audio real tests actually run: `PLAYA_DRY_RUN=1 BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden,gtts cargo nextest run -p biscuit-speaks --features playa -E '<the three exact names>'` → **3 passed**, and the captured stdout shows `Found 128 echogarden voices` with real voice names. So they executed rather than skipping, with the required switch armed. `PLAYA_DRY_RUN=1` was set purely as a belt-and-braces guard; none of the three consults it
                - probe 2, absent backend with the switch **unset** → clean skip: same command with `PATH=/usr/bin:/bin:$HOME/.cargo/bin` (which hides nvm's `echogarden` and `~/.local/bin/gtts-cli`) → **1 passed**, stderr `SKIP: echogarden: the echogarden binary is not installed`
                - probe 3, absent backend with the switch **set** → hard failure: same restricted PATH plus `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogarden` → **1 FAILED**, panic at `test_support.rs:83` with `echogarden: real-resource coverage is required on this host, but the echogarden binary is not installed`. Probes 2 and 3 differ only in the environment variable, so the switch — not the test body — is what turns the skip into a failure
                - probe 4, misspelled entry → hard failure: restricted PATH plus `BISCUIT_SPEAKS_REQUIRED_PROVIDERS=echogrden` → **1 FAILED** with `BISCUIT_SPEAKS_REQUIRED_PROVIDERS="echogrden" names unknown provider "echogrden"; known providers: echogarden, gtts`. Worth recording precisely: the typo guard fires when a skip branch is reached, which is exactly when the requirement matters; with every backend present nothing skips and the typo is inert. The four `test_support` unit tests cover the parser directly and unconditionally, including a `#[should_panic]` case for the typo
        - **cross-OS gating** — the change adds no `#[cfg(target_os = ...)]`, no `#[cfg(windows)]`, and no platform-specific API; `test_support.rs` uses only `std::env` and `std::fmt`. Availability is decided at runtime by `provider.is_ready()` (`which::which` plus, for gTTS, a TCP connect), so an unavailable provider produces a recorded `SKIP:` line on every OS unless the switch is set
                - macOS: `just test` + `just lint` green, and the three non-audio real tests executed (probe 1)
                - native Windows: `cargo check -p biscuit-speaks --features playa --all-targets --target x86_64-pc-windows-gnu` → **exit 0**. Per the `os` skill this is compile evidence only, never behavioral evidence
                - Linux: no evidence. `cargo check --target x86_64-unknown-linux-gnu` fails on this host in `cc-rs` (`failed to find tool "x86_64-linux-gnu-gcc"`), which is a missing cross C toolchain rather than anything about this change, and a bounded reachability probe to `$BUILD_LINUX` (`build-linux`) timed out at 30 s — the same host unavailability `evidence.md` already records for this fix
                - WSL2: not attempted; it follows Linux code paths, and `$BUILD_WSL` shares the physical drive that `evidence.md` records as exhausted. Both are deferred to CI, which is the authoritative proof anyway
        - **docs updated** (the specification's "update relevant testing documentation and skills if a new shared test convention is introduced"):
                - `.claude/skills/rust-testing/SKILL.md` — new `### Requiring a real resource individually` subsection under the L2 backend-requirement section it parallels, covering the two switches, the exact-match typo rule, the "never `#[ignore]` a real-resource test" rule with the reason, and the fact that a `real_*` name inside `#[cfg(test)] mod tests` in `src/` *is* selected. Also added two rows to the Environment Contract table and a pointer from the `Real` row of the Test Levels table. `hash:` recomputed with `md hash` to `7055b0e89017847d-717a77c3a2341172` and `last_updated` moved to 2026-09-10; re-running `md hash` confirms the value is a fixed point
                - `biscuit-speaks/README.md` — the existing "Silent audio tests" section gained the `real_*` naming/tier rules and a table of the two switches
                - `.claude/skills/biscuit-speaks/SKILL.md` — the "Volume and silent tests" paragraph now states the naming rule, the no-`#[ignore]` rule, why `test-real` enables `playa` (it is what makes the playback report assertable), and the switch. That file carries no `hash:` frontmatter, so none was recomputed
                - `claudine/fixes/2026-09-10-audio-tests-should-be-silent/evidence.md` — appended five rows to the existing Validation record table and a new `## Real backends exercised, and what is still owed` section listing all eight real-tier tests with per-test status. Existing content was not rewritten
        - **executing the real tier for EchoGarden and gTTS is deferred to a provisioned host or CI**, and the three playback tests are recorded as deferred rather than as passes
                - the reason is the specification's own rule: *"Do not use listening alone as proof of silence: muted speakers or unavailable hardware can hide a regression."* Running `just test-real` here would perform real synthesis and playback of three muted messages on the user's machine, and the only thing this host could contribute is that nobody heard them — which the specification explicitly refuses as evidence. The assertions the tests make (route, verdict, provider, voice, codec) are the evidence, and they are equally valid wherever the tier runs
                - what *was* obtained here instead: selection proof by `cargo nextest list` (which builds the test binaries without running any of them), plus real execution of the three real-tier tests that touch a real resource without producing audio. Three of the eight real-tier tests remain unexecuted in this session
                - a CI leg or provisioned host should set `BISCUIT_SPEAKS_REQUIRED_PROVIDERS` to the providers it has actually installed, so a missing backend fails the tier instead of skipping it
        - **observed but not in scope** — the same defect the finding describes exists in three more provider files, which it does not name: `providers/host/espeak.rs:829` (`test_list_voices_integration`), `providers/host/kokoro.rs:949` (`test_speak_integration` — muted and real, and it swallows its own error into an `eprintln!`, so it cannot fail), and four `#[ignore]`d ElevenLabs tests at `providers/cloud/elevenlabs.rs:1421-1440`. All are unreachable from every canonical recipe for exactly the reason given in finding 3. Left untouched for scope discipline; recorded here for a future review cycle
        - verification commands and results:
                - `cd biscuit-speaks && just test -j 2` → **491 passed** (487 before), 19 skipped, exit 0. The +4 are the `test_support` switch-parsing unit tests; the skipped count is unchanged because the six tests that left L1 as `#[ignore]`d are the same six now filtered out as `real_`
                - `cd biscuit-speaks && just lint` → exit 0 for both library and CLI
                - supplementary, since `just lint` builds only default features while the three playback tests need `playa`: `cargo clippy -p biscuit-speaks --features playa --all-targets -- -D warnings` → exit 0, and `cargo clippy -p biscuit-speaks --all-targets -- -D warnings` → exit 0
                - `playa` and `claudine` gates were **not** re-run: no source outside `biscuit-speaks` and documentation was touched, and nothing in this change is reachable from a non-test build
                - `cargo fmt -p biscuit-speaks --check` reports 47 diff hunks, **all pre-existing** (`detached.rs`, `lib.rs` module ordering, `say.rs`, `detached_phase4.rs`, `volume_control.rs`, and others) and none in the new file: the two rustfmt nits this change introduced — one in `test_support.rs`, one in `echogarden.rs` — were applied by hand, taking the count from 49 back to 47, which is the total this package already carried. `cargo fmt` was not run
- work completed for 'finding-3: muted EchoGarden and gTTS tests unreachable from the real tier' at 15:52:41-07:00
- final combined verification by the orchestrator, run after all three findings had landed, because each subagent verified at a different point in the sequence:
        - `cd playa && just test` → **194 passed**, 8 skipped, exit 0; `just lint` → exit 0
        - `cd biscuit-speaks && just test -j 2` → **491 passed**, 19 skipped, exit 0; `just lint` → exit 0
        - `cd claudine && just test --no-fail-fast` → **6,860 passed**, 11 skipped, exit 0; `just lint` → exit 0
        - `just test-real` was deliberately **not** run in this session; see the finding-3 entries for why listening on this host is not admissible evidence

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1 hour. During
this implementation all 3 review findings were evaluated to see if they could be
fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred.

No finding was deferred. One piece of *evidence* is owed rather than one piece of
work: the three EchoGarden/gTTS playback tests converted under finding 3 are now
selected by the canonical `test-real` tier, but were not executed here. Running
them on this host would synthesize and play three muted messages on the user's
machine, and the only thing this host could add is that nobody heard them — which
the specification explicitly refuses as proof of silence. Their selection was
proven instead with `cargo nextest list`, and their execution is owed by a
provisioned host or CI leg that sets `BISCUIT_SPEAKS_REQUIRED_PROVIDERS` to the
providers it actually has installed. This matches the one acceptance criterion
the specification still leaves unchecked (cross-OS and real-backend evidence).

No performance measurement was required by any finding, so
`deferred_perf_measurement` remains `false`.

The files changed by this implementation cycle are:

- **New**
        - `tools/test-toolkit/src/spool.rs` — the shared `LockedAudioSpool` publication fixture
        - `tools/test-toolkit/tests/audio_spool.rs` — the synchronized queue-boundary unwind regression
        - `biscuit-speaks/lib/src/test_support.rs` — the provider-specific required-resource switch
- **Deleted**
        - `claudine/test-support/locked_audio_spool.rs` — superseded by the shared fixture
- **Sources and tests**
        - `playa/lib/src/native_player.rs` — the native gain-application seam and its six boundary tests
        - `playa/cli/tests/detached_cli.rs`
        - `biscuit-speaks/lib/tests/detached_phase4.rs`
        - `biscuit-speaks/lib/src/lib.rs`
        - `biscuit-speaks/lib/src/providers/host/echogarden.rs`
        - `biscuit-speaks/lib/src/providers/host/gtts.rs`
        - `claudine/cli/tests/detached_audio.rs`
        - `claudine/lib/src/dispatch/runner/tests.rs`
        - `claudine/lib/src/composition/lifecycle/tests/audio_emission.rs`
- **Manifests**
        - `tools/test-toolkit/Cargo.toml` (gains `fs4`), `playa/cli/Cargo.toml` (gains `test-toolkit`),
          `claudine/lib/Cargo.toml`, `claudine/cli/Cargo.toml`, `biscuit-speaks/lib/Cargo.toml`
          (each drops a now-unused `fs4` dev dependency)
- **Documentation and skills**
        - `docs/dependencies.md`, `claudine/docs/dependencies.md`,
          `biscuit-speaks/docs/dependencies.md`, `playa/docs/dependencies.md`
        - `claudine/docs/topics/testing.md`, `tools/test-toolkit/README.md`,
          `biscuit-speaks/README.md`
        - `.claude/skills/claudine/SKILL.md`, `.claude/skills/biscuit-speaks/SKILL.md`,
          `.claude/skills/rust-testing/SKILL.md`
        - `claudine/fixes/2026-09-10-audio-tests-should-be-silent/evidence.md`

Two adjacent gaps were found but left untouched for scope discipline, and are
recorded above for a future review cycle: `playa/lib/src/sfx_player.rs` applies
volume through the same unobserved `Player::set_volume` shape that finding 2
repaired in `native_player.rs`, and the same unreachable-`#[ignore]` defect that
finding 3 repaired for EchoGarden and gTTS still exists in `espeak.rs`,
`kokoro.rs`, and `elevenlabs.rs`.

