# Windows Backend Implementation Review

## Validation Performed

- Ran `cargo test -p playa --lib`
- Ran `cargo test -p playa --lib --features audio-ducking`
- Attempted `cargo check -p playa --target x86_64-pc-windows-gnu --features audio-ducking-windows,sfx-native-windows`

The Windows-target `cargo check` could not complete in this environment because the MinGW cross toolchain is not installed (`x86_64-w64-mingw32-gcc` missing). The findings below therefore combine local verification with static review of the Windows-only code paths.

## Findings

1. High: the new Windows backend still does not compile on its target platform because the session-instance string extraction is written against the wrong `PWSTR` API shape.

`playa/lib/src/ducking/windows.rs:269`, `playa/lib/src/ducking/windows.rs:325`

The implementation treats `instance_id_pwstr.to_string()` as a safe, infallible conversion returning `String`. In `windows-strings`, `PWSTR::to_string()` is `unsafe` and returns `Result<String, FromUtf16Error>`. As written, both `enumerate_sessions()` and `find_simple_volume_by_key()` immediately use the result as if it were a `String` (`is_empty()`, `== target_key`), so the Windows backend cannot build successfully once the Windows-only code is actually compiled.

This is also the core path for snapshot and restore keying, so it is not a cosmetic issue. The backend’s primary identifier path is currently broken.

Recommended fix:
- Add one shared helper that:
  - safely converts `PWSTR` to `String`
  - always frees the COM allocation
  - returns `Option<String>` or `Result<Option<String>, DuckingError>`
- Use that helper from both session enumeration and lookup.

2. High: `fade_to_floor()` and `fade_restore()` keep COM apartment state and COM interfaces alive across `.await`, which is unsafe for this backend design.

`playa/lib/src/ducking/windows.rs:108`, `playa/lib/src/ducking/windows.rs:111`, `playa/lib/src/ducking/windows.rs:123`, `playa/lib/src/ducking/windows.rs:127`, `playa/lib/src/ducking/windows.rs:146`, `playa/lib/src/ducking/windows.rs:149`, `playa/lib/src/ducking/windows.rs:159`, `playa/lib/src/ducking/windows.rs:166`, `playa/lib/src/ducking/backend.rs:11`

Both async ducking methods create `ComGuard`, obtain `IAudioSessionManager2`, then hold those values and `ISimpleAudioVolume` handles while awaiting `tokio::time::sleep(...)` inside the fade loop. That is a bad fit for the crate’s contract: `DuckResult` is explicitly a `Send` future. On a multithreaded Tokio runtime, the future may resume on a different worker thread after an `.await`.

That creates two problems:

- `CoUninitialize()` must run on the same thread that called `CoInitializeEx()`, but `ComGuard` can now be dropped on a different thread.
- The COM interfaces were acquired on one apartment/thread and may be used again after resumption on another.

This is exactly the class of problem the design was trying to avoid by keeping the backend stateless and reacquiring Windows objects per operation.

Recommended fix:
- Make each fade phase run entirely on one blocking thread, or
- reacquire COM state between suspension points and do not hold `ComGuard` or COM interfaces across `.await`

The first option is cleaner here: run the whole fade loop synchronously inside `spawn_blocking` or a dedicated blocking helper so the COM lifecycle stays pinned to one thread.

3. Medium: the added Windows test coverage is too weak to catch the real implementation risks.

`playa/lib/src/ducking/tests.rs:147`, `playa/lib/src/ducking/windows.rs:339`, `playa/lib/src/sfx_player.rs:740`, `playa/lib/src/windows_com.rs`

The new non-Windows tests in `ducking/tests.rs` are only value-level policy checks over manually constructed `VolumeSnapshot` data. They do not exercise:

- session enumeration
- `PWSTR` conversion and freeing
- active/inactive filtering in the real backend
- restore-time session lookup
- mute restoration behavior
- `ComGuard` behavior for `RPC_E_CHANGED_MODE`
- `ComGuard` drop balancing

The Windows-only tests that were added are all ignored smoke tests and do not validate the failure-prone parts of the implementation. There are also no unit tests at all for `playa/lib/src/windows_com.rs`.

Because of that, both high-severity issues above can exist without any test failing, which is exactly what happened here.

Recommended fix:
- Add unit tests around a shared `PWSTR` conversion helper
- Add unit tests around `ComGuard` policy with injectable COM init outcomes
- Add ignored Windows integration tests for:
  - snapshot on a live default render endpoint
  - fade/restore round-trip for a live session
  - SFX setup when COM is already initialized on the thread

## Functionality Gaps Against The Design

- The planned shared helper for session-key extraction and memory cleanup was not implemented. That omission directly contributed to the broken `PWSTR` handling.
- The design intent to avoid carrying Windows handles across async boundaries was not met in the fade paths.
- The planned COM regression coverage was not implemented.

## Ergonomics And Performance Opportunities

- `find_simple_volume_by_key()` re-enumerates all sessions once per snapshot entry, so fade and restore are currently O(n^2) in session count. Once the COM/threading model is corrected, building a per-phase key lookup would reduce repeated COM calls and simplify the code.
- `ComGuard::new()` should return a typed Windows error instead of `String`. Keeping the original error or HRESULT would make logging, testing, and diagnostics much sharper.
- The special `Err(HRESULT(1))` branch in `playa/lib/src/windows_com.rs:42` is a sign that the helper is modeling `CoInitializeEx`’s result shape indirectly. It would be better to encode the actual success states explicitly and test them.

## What Looks Good

- Feature wiring for `audio-ducking-windows` is present in both the library and CLI.
- The Windows SFX path now goes through a shared COM guard instead of discarding the `CoInitializeEx` result.
- `duck-info` and the audio ducking docs were updated to describe the Windows WASAPI strategy.
