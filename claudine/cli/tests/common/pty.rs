//! Shared PTY harness for the Level 1 schema-prompt, provided-partial, and
//! sequence-overlay interactive tests.
//!
//! The schema-prompt god file was split into a schema / inline-compose binary
//! (`level1_schema_prompt_pty.rs`) and a `sequence_overlay_pty.rs` binary.
//! The draining loop (`read_for`), marker waiters (`wait_for_marker`,
//! `wait_for_raw_mode`), and the config / goose-stub stagers are shared by
//! both, so they live here **verbatim**. Gated `#[cfg(unix)]` at the
//! `mod pty;` site because `expectrl::session::OsSession` is Unix-only.
//!
//! ## No runner-visible serialization
//!
//! PTY tests carry no `#[serial]`: `Session::spawn` allocates a fresh
//! `/dev/ptmx` pair per call and each test's fixture root is unique
//! (`CliProcessFixture::named` keys on pid + nanos + counter), so there is no
//! resource two of them can contend for. The `serial(pty)` group these files
//! used to declare was also unenforceable — `serial_test` coordinates threads
//! inside one process, and nextest gives every test its own.

#![allow(dead_code)]

use super::{strip_ansi, write_executable};
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};

/// DSR cursor-position query (`ESC[6n`) that crossterm — via ratatui's
/// inline-viewport constructor — writes to stdout before it will accept
/// input. A bare expectrl PTY has no terminal emulator to answer it, so
/// the harness itself must reply or crossterm times out (~2s) and the
/// prompt aborts. See [`answer_pending_dsr`].
const DSR_QUERY: &[u8] = b"\x1b[6n";

/// Cursor-position report (`ESC[1;1R`) sent in reply to a [`DSR_QUERY`].
/// crossterm parses this into a `CursorPosition` event, unblocking the
/// inline viewport; the row/col are irrelevant for these tests.
const DSR_REPLY: &[u8] = b"\x1b[1;1R";

/// crossterm `PushKeyboardEnhancementFlags` (flags = 11) emitted by
/// biscuit-tui's `prepare_terminal` immediately after `enable_raw_mode()`.
/// It is written for BOTH inline and fullscreen prompts, so — unlike
/// [`ALT_SCREEN_ENTER`], which only fullscreen prompts emit — it is the
/// universal proof that raw mode is active. See [`wait_for_raw_mode`].
const KBD_ENHANCEMENT_PUSH: &str = "\x1b[>11u";

/// The OSC colour queries `biscuit_terminal::Terminal` construction writes to
/// `/dev/tty` — foreground (OSC 10) and background (OSC 11) — paired with the
/// replies a real emulator sends.
///
/// A bare expectrl PTY has no emulator behind it, so an unanswered query costs
/// the child biscuit-terminal's whole `DEFAULT_TIMEOUT` (1 s) *per code* before
/// it can render its first byte. That is not the child doing work: it is the
/// test forcing the child to wait out a timeout, which is exactly what a
/// readiness deadline must replace with observation. Answering is the same
/// contract [`answer_pending_dsr`] already honours for the cursor probe.
///
/// The replies are biscuit-terminal's own manufactured pair from
/// `lib/tests/level1_terminal_osc_cache.rs`: a dark background and a light
/// foreground, i.e. the ordinary developer terminal these tests' styling
/// assertions were written against.
const COLOR_QUERY_ANSWERS: [(&[u8], &[u8]); 2] = [
    (b"\x1b]10;?\x07", b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07"),
    (b"\x1b]11;?\x07", b"\x1b]11;rgb:1e1e/1e1e/1e1e\x07"),
];

/// Reply to any [`COLOR_QUERY_ANSWERS`] query present in `chunk`.
///
/// Matched against the **newly read chunk** rather than the cumulative
/// transcript, and deliberately so: every waiter below starts with its own
/// answer state, and `bg_color`/`text_color` are `OnceLock`-cached in the
/// child, so a cumulative match would re-send a reply long after the query —
/// by which point the prompt is in raw mode and the reply's leading `ESC`
/// reads as a cancel keystroke. Missing a query split across two reads costs
/// only the timeout that was there before, so this fails safe.
fn answer_color_queries(session: &mut OsSession, chunk: &[u8]) {
    let mut replied = false;
    for (query, reply) in COLOR_QUERY_ANSWERS {
        if chunk.windows(query.len()).any(|w| w == query) {
            let _ = session.write_all(reply);
            replied = true;
        }
    }
    if replied {
        let _ = session.flush();
    }
}

/// Reply to every not-yet-answered [`DSR_QUERY`] in `data`, advancing
/// `answered` so each query is answered exactly once across repeated calls
/// (e.g. a number-retry loop that spawns a second inline prompt). Counting
/// against the cumulative byte buffer keeps a query split across two reads
/// from being answered twice or missed.
fn answer_pending_dsr(session: &mut OsSession, data: &[u8], answered: &mut usize) {
    let total = if data.len() < DSR_QUERY.len() {
        0
    } else {
        data.windows(DSR_QUERY.len()).filter(|w| *w == DSR_QUERY).count()
    };
    while *answered < total {
        let _ = session.write_all(DSR_REPLY);
        *answered += 1;
    }
    if total > 0 {
        let _ = session.flush();
    }
}

/// Drain bytes from the PTY for a generous window so each `try_read`
/// call collects whatever the child has flushed so far. Returns the
/// concatenated transcript bytes as `String::from_utf8_lossy` so stray
/// ANSI fragments never panic.
pub(crate) fn read_for(session: &mut OsSession, total_deadline: Duration) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let mut dsr_answered = 0;
    let deadline = Instant::now() + total_deadline;
    session.set_expect_timeout(Some(Duration::from_millis(150)));
    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                answer_color_queries(session, &scratch[..n]);
                buf.extend_from_slice(&scratch[..n]);
                // A re-prompt (e.g. the number-retry loop) spins up a second
                // inline viewport mid-drain; answer its DSR query too.
                answer_pending_dsr(session, &buf, &mut dsr_answered);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Wait for a substring (after ANSI stripping) to appear in the cumulative
/// transcript, draining the PTY incrementally. Returns the full ANSI-bearing
/// transcript on success; panics with the accumulated transcript on
/// timeout for easier diagnosis.
pub(crate) fn wait_for_marker(session: &mut OsSession, marker: &str, deadline: Duration) -> String {
    let stop = Instant::now() + deadline;
    let mut transcript = String::new();
    let mut scratch = [0u8; 4096];
    let mut dsr_answered = 0;
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    while Instant::now() < stop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                answer_color_queries(session, &scratch[..n]);
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                // A marker that renders inside an inline viewport (e.g. the
                // number-retry validation error) only appears after the
                // viewport's DSR query is answered.
                answer_pending_dsr(session, transcript.as_bytes(), &mut dsr_answered);
                if strip_ansi(&transcript).contains(marker) {
                    return transcript;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!(
        "marker {marker:?} did not appear within {deadline:?}; transcript:\n{transcript}"
    );
}

/// Alternate-screen enter sequence crossterm emits via `EnterAlternateScreen`.
///
/// Only *fullscreen* prompts (the agent-resolution review picker) emit this;
/// inline prompts (schema property collectors) do not. Tests asserting a
/// fullscreen UI did *not* render still key off this constant.
pub(crate) const ALT_SCREEN_ENTER: &str = "\x1b[?1049h";

/// Block until the interactive widget has switched the PTY into raw mode
/// and is ready to accept a keystroke, then return the accumulated raw
/// transcript.
///
/// The prompt's property name renders in the pre-prompt status report —
/// which `run_standalone` prints to stderr *before* it calls
/// `enable_raw_mode()`. Sending a keystroke as soon as that name appears
/// races the line discipline: until raw mode disables `ICRNL`, a carriage
/// return (`\r`, the byte that means Enter) is rewritten to a line feed
/// (`\n`), which crossterm parses as `Ctrl+J` — not `Enter` — so the
/// prompt never submits and the test hangs to its deadline.
///
/// `prepare_terminal` enables raw mode and *then* emits
/// [`KBD_ENHANCEMENT_PUSH`] for both inline and fullscreen prompts, so
/// observing that sequence proves raw mode is already active. (Fullscreen
/// prompts also emit [`ALT_SCREEN_ENTER`] first, but inline prompts never
/// do, so the keyboard-enhancement push is the universal marker.)
///
/// For an *inline* prompt one more step is required before the caller may
/// type: ratatui's inline viewport blocks on a DSR cursor-position reply
/// (see [`DSR_QUERY`]) before its event loop reads input. Worse,
/// crossterm's `get_cursor_position` *consumes* any keystroke that arrives
/// before that reply. So this waiter answers the DSR query and only returns
/// once it has — guaranteeing the caller's keystroke lands in the event
/// loop, not the cursor probe. Fullscreen prompts issue no such query and
/// return as soon as the alternate screen is observed.
///
/// `seed` is the transcript already drained by the preceding
/// [`wait_for_marker`] call — the markers may have arrived in the same
/// read as the status report.
pub(crate) fn wait_for_raw_mode(session: &mut OsSession, seed: String, deadline: Duration) -> String {
    let stop = Instant::now() + deadline;
    let mut transcript = seed;
    let mut scratch = [0u8; 4096];
    let mut dsr_answered = 0;
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    // Answer any DSR query already buffered from the preceding marker wait.
    answer_pending_dsr(session, transcript.as_bytes(), &mut dsr_answered);
    loop {
        if transcript.contains(KBD_ENHANCEMENT_PUSH) {
            // Fullscreen prompts enter the alternate screen and never probe
            // the cursor, so raw mode is proven and no DSR is owed.
            if transcript.contains(ALT_SCREEN_ENTER) {
                return transcript;
            }
            // Inline prompts are ready only once their cursor probe is
            // answered — otherwise the caller's keystroke is swallowed.
            if dsr_answered >= 1 {
                return transcript;
            }
        }
        if Instant::now() >= stop {
            break;
        }
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                answer_color_queries(session, &scratch[..n]);
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                answer_pending_dsr(session, transcript.as_bytes(), &mut dsr_answered);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!(
        "raw-mode marker {KBD_ENHANCEMENT_PUSH:?} did not appear within {deadline:?}; \
         transcript:\n{transcript}"
    );
}

/// Block until the prompt re-enters raw mode for a *fresh* iteration — i.e.
/// a [`KBD_ENHANCEMENT_PUSH`] beyond the count already present in `seed` —
/// answering any inline DSR query along the way, then return the
/// accumulated transcript.
///
/// `collect_number`'s parse-and-retry loop tears the inline viewport down
/// and spawns a new `run_standalone` on rejection, so a second
/// keyboard-enhancement push is the deterministic proof that the invalid
/// value was rejected and the prompt re-rendered. It is a stronger and more
/// stable signal than scraping the re-rendered validation-error glyphs,
/// which a bare PTY (no terminal emulator answering the cursor probe with a
/// real position) cannot lay out reliably.
pub(crate) fn wait_for_raw_mode_reentry(
    session: &mut OsSession,
    seed: String,
    deadline: Duration,
) -> String {
    let base = seed.matches(KBD_ENHANCEMENT_PUSH).count();
    let stop = Instant::now() + deadline;
    let mut transcript = seed;
    let mut scratch = [0u8; 4096];
    let mut dsr_answered = 0;
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    answer_pending_dsr(session, transcript.as_bytes(), &mut dsr_answered);
    while Instant::now() < stop {
        if transcript.matches(KBD_ENHANCEMENT_PUSH).count() > base && dsr_answered >= 1 {
            return transcript;
        }
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                answer_color_queries(session, &scratch[..n]);
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                answer_pending_dsr(session, transcript.as_bytes(), &mut dsr_answered);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!(
        "prompt did not re-enter raw mode within {deadline:?}; transcript:\n{transcript}"
    );
}

/// Pre-stage a minimal claudine config at `$HOME/.claudine/config.json`.
///
/// Under a PTY, claudine detects an interactive stdin and runs the
/// first-run setup wizard when no user config exists. The wizard would
/// intercept any input we send to the schema prompt and block the test
/// indefinitely. Writing an empty JSON object satisfies the loader (every
/// `ClaudineConfig` field has a `#[serde(default)]` derivation) and
/// inherits the defaults — notably `prompt_for_missing = true`.
pub(crate) fn stage_default_config(home_dir: &std::path::Path) {
    let claudine_dir = home_dir.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();
}

/// Stage a stub `goose` binary that records that the provider was
/// launched (so the test can prove the interactive prompt successfully
/// satisfied the schema and execution proceeded).
///
/// The stub writes the marker file FIRST and then exits without reading
/// stdin. Under PTY, the wrapper's stdin remains attached to the master
/// side, so a `cat > /dev/null` stub would block on EOF that never
/// arrives. Writing the marker first lets the test observe the launch
/// even if the wrapper still has stdin open.
pub(crate) fn stage_goose_stub(bin_dir: &std::path::Path, marker_file: &std::path::Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho 'launched' > {marker}\nexit 0\n",
            marker = marker_file.display()
        ),
    );
}
