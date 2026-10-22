//! PTY test helpers for biscuit-terminal PTY tests.
//!
//! These helpers spawn the `discovery_probe` example binary inside a
//! pseudoterminal (via [`expectrl`]) so that `is_tty()` returns `true`
//! and OSC / DSR sequences can be manufactured by the test.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use expectrl::session::OsSession;

/// The OSC 10 (foreground color) query the library writes to `/dev/tty`,
/// `ESC ] 10 ; ? BEL`. The `;?` form is distinct from any manufactured
/// `;rgb:` reply, so counting it in the master stream is unambiguous.
pub const OSC10_QUERY: &[u8] = b"\x1b]10;?\x07";

/// The OSC 11 (background color) query, `ESC ] 11 ; ? BEL`.
pub const OSC11_QUERY: &[u8] = b"\x1b]11;?\x07";

/// The DA1 sentinel the library appends to every terminal query, `CSI c`.
pub const DA1_QUERY: &[u8] = b"\x1b[c";

/// A DA1 reply as a VT220-class terminal sends it. Every live query waits for
/// this (or its timeout), so a test that manufactures a query reply should
/// answer DA1 too, listed after the query's own answer.
pub const DA1_REPLY: &[u8] = b"\x1b[?62;22c";

/// Count non-overlapping occurrences of `needle` in `haystack`.
pub fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    let mut count = 0;
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

/// Locate the `discovery_probe` example binary.
///
/// Derives the path from the current test executable location so it
/// works in both debug and release profiles without relying on
/// `CARGO_BIN_EXE_` env vars (which cargo only sets for binaries, not
/// examples).
fn discovery_probe_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe unavailable");
    // test exe is at  <target_dir>/<profile>/deps/<name>
    // we want        <target_dir>/<profile>/examples/discovery_probe
    let mut dir = exe.parent().unwrap().to_path_buf(); // deps/
    dir = dir.parent().unwrap().to_path_buf(); // <profile>/
    dir.join("examples")
        .join(format!("discovery_probe{}", std::env::consts::EXE_SUFFIX))
}

/// Silence budget for probes whose replies this harness manufactures.
///
/// The library's local default also has to cover the driver's poll interval
/// and the probe's startup, which a contended host can stretch; one second
/// restores the pre-budget per-query allowance. A test of the default itself passes this variable
/// with an empty value, which the library ignores.
pub const SILENCE_BUDGET_FOR_TESTS: (&str, &str) = ("BISCUIT_TERMINAL_QUERY_SILENCE_MS", "1000");

/// Base environment variables that prevent hangs and unwanted side-effects
/// in the spawned probe.
///
/// * `BISCUIT_TERMINAL_QUERY_SILENCE_MS=1000` on non-Windows hosts; see
///   [`SILENCE_BUDGET_FOR_TESTS`].
/// * `NO_COLOR=1` on non-Windows hosts disables color output so assertions need not
///   cope with unrelated SGR sequences.
/// * `PROBE_FORCE_TTY=true` on Windows because ConPTY child handles are not
///   recognized by Rust's `IsTerminal` implementation. `NO_COLOR` is omitted
///   there because the prose probes exercise non-color SGR capabilities.
pub fn anti_hang_env() -> Vec<(&'static str, &'static str)> {
    #[cfg(not(windows))]
    let env = vec![("NO_COLOR", "1"), SILENCE_BUDGET_FOR_TESTS];
    #[cfg(windows)]
    let env = vec![("PROBE_FORCE_TTY", "true")];
    env
}

/// Spawn the `discovery_probe` example binary in a PTY with the given
/// extra environment variables.
///
/// Caller-supplied variables override the base anti-hang set.
///
/// ## Panics
///
/// Panics if the binary cannot be found or the PTY spawn fails.
pub fn spawn_with_env(envs: &[(&str, &str)]) -> OsSession {
    spawn_removing(envs, &[])
}

/// Variables that identify a terminal, multiplexer, or graphics policy to
/// the library, beyond the ones [`spawn_with_env`] always removes.
///
/// A test of an *unidentified* terminal has to drop the host terminal's own
/// identity too, or a run from iTerm2 (`LC_TERMINAL`), WezTerm
/// (`TERM_PROGRAM`), or tmux (`TMUX`) resolves the route from the
/// environment and never probes.
pub const IDENTITY_VARS: &[&str] = &[
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "LC_TERMINAL",
    "LC_TERMINAL_VERSION",
    "TMUX",
    "TMUX_PANE",
    "STY",
    "ZELLIJ",
    "ZELLIJ_SESSION_NAME",
    "MSYSTEM",
    "COLORFGBG",
    "BISCUIT_GRAPHICS",
    "BISCUIT_GRAPHICS_TRANSPORT",
    "BISCUIT_TERMINAL_PROBE",
];

/// Like [`spawn_with_env`], but the child also loses every variable in
/// [`IDENTITY_VARS`], so only `envs` (and `PROBE_TERM`/`PROBE_TERM_PROGRAM`
/// overrides) say what terminal it runs in.
pub fn spawn_unidentified_with_env(envs: &[(&str, &str)]) -> OsSession {
    spawn_removing(envs, IDENTITY_VARS)
}

/// The marker the shell around a [`spawn_redirected_with_env`] probe prints
/// to the pty once the probe has exited.
#[cfg(unix)]
pub const REDIRECTED_PROBE_EXITED: &str = "redirected_probe_exited";

/// Like [`spawn_with_env`], but the probe's stdout and stderr both go to
/// `output`, so neither standard stream is a terminal while `/dev/tty` is
/// still the pty: the library must not query, and anything it wrote to
/// `/dev/tty` would still show on the master side. The child also loses
/// [`IDENTITY_VARS`], as with [`spawn_unidentified_with_env`]. A shell around
/// the probe prints [`REDIRECTED_PROBE_EXITED`] to the pty when the probe
/// exits.
#[cfg(unix)]
pub fn spawn_redirected_with_env(envs: &[(&str, &str)], output: &std::path::Path) -> OsSession {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
        .arg(format!("\"$0\" >\"$1\" 2>&1; echo {REDIRECTED_PROBE_EXITED}"))
        .arg(existing_probe())
        .arg(output);
    configure_and_spawn(cmd, envs, IDENTITY_VARS)
}

fn existing_probe() -> PathBuf {
    let bin = discovery_probe_path();
    if !bin.exists() {
        panic!(
            "discovery_probe example not found at {}. \
             Run `cargo build -p biscuit-terminal --example discovery_probe` first.",
            bin.display()
        );
    }
    bin
}

fn spawn_removing(envs: &[(&str, &str)], extra_removals: &[&str]) -> OsSession {
    let bin = existing_probe();
    #[cfg(windows)]
    let cmd = {
        let shell =
            std::env::var_os("ComSpec").unwrap_or_else(|| r"C:\Windows\System32\cmd.exe".into());
        let mut cmd = Command::new(shell);
        cmd.args(["/d", "/s", "/c"]);
        cmd.arg(format!("\"\"{}\" > CONOUT$ 2>&1\"", bin.display()));
        cmd
    };
    #[cfg(not(windows))]
    let cmd = Command::new(&bin);
    configure_and_spawn(cmd, envs, extra_removals)
}

fn configure_and_spawn(
    mut cmd: Command,
    envs: &[(&str, &str)],
    extra_removals: &[&str],
) -> OsSession {
    // Remove terminal-specific env vars that would override TERM_PROGRAM
    // detection, so tests can manufacture a terminal identity cleanly. The
    // SSH/Mosh markers go too: a run over SSH (as `just cross-check` does)
    // would otherwise give every query the remote silence budget.
    for var in [
        "SSH_CLIENT",
        "SSH_CONNECTION",
        "SSH_TTY",
        "MOSH_CONNECTION",
        "WEZTERM_UNIX_SOCKET",
        "WEZTERM_PANE",
        "KITTY_WINDOW_ID",
        "KITTY_PID",
        "ITERM_SESSION_ID",
        "ITERM_PROFILE",
        "GHOSTTY_RESOURCES_DIR",
        "ALACRITTY_WINDOW_ID",
        "ALACRITTY_SOCKET",
        "ALACRITTY_LOG",
        "WT_SESSION",
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "TRAVIS",
        "CIRCLECI",
    ] {
        cmd.env_remove(var);
    }
    for var in extra_removals {
        cmd.env_remove(var);
    }

    for (k, v) in anti_hang_env() {
        cmd.env(k, v);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }

    // `mut` is only consumed by the Windows-only ConPTY resize below.
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut session =
        expectrl::Session::spawn(cmd).expect("failed to spawn discovery_probe in PTY");
    #[cfg(windows)]
    session
        .get_process_mut()
        .resize(160, 40)
        .expect("failed to size discovery_probe ConPTY");
    session
}

/// Convenience: spawn with only the anti-hang environment.
pub fn spawn_probe() -> OsSession {
    spawn_with_env(&[])
}

/// Read everything currently available from `session` without blocking.
///
/// Uses a short timeout so that slow output is still captured, but the
/// function returns promptly when the pipe is dry.
pub fn try_read_available(session: &mut OsSession) -> String {
    let mut buf = String::new();
    let mut scratch = [0u8; 4096];

    // Poll a few times with a tiny sleep — enough for the probe to finish
    // a single query/response cycle.
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(10));
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                buf.push_str(&String::from_utf8_lossy(&scratch[..n]));
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
        }
    }
    buf
}

/// A manufactured terminal answer: when `query` appears in the master stream,
/// `reply` is written back into the PTY — once for [`ProbeAnswer::new`], or
/// once per occurrence for [`ProbeAnswer::every`].
///
/// The type is protocol-neutral. `query` is whatever byte sequence the probe
/// writes to `/dev/tty` — an OSC colour query (`ESC ] 11 ; ? BEL`), the DSR
/// cursor-position query (`ESC [ 6 n`), or the DA1 sentinel — and `reply` is
/// the response a real terminal would send.
///
/// Answering on observation rather than after a fixed delay is what makes a
/// probe test independent of process-start latency: `Session::spawn` returns
/// well before the child has entered raw mode and emitted anything, so a timed
/// reply spends most of the probe's response budget on startup.
pub struct ProbeAnswer {
    pub query: &'static [u8],
    pub reply: &'static [u8],
    answered: usize,
    repeat: bool,
    kitty: Option<KittyAnswer>,
}

/// A Kitty graphics reply whose image ID is read off the query it answers.
#[derive(Clone, Copy)]
struct KittyAnswer {
    wrapped: bool,
    message: &'static str,
}

impl ProbeAnswer {
    pub fn new(query: &'static [u8], reply: &'static [u8]) -> Self {
        Self {
            query,
            reply,
            answered: 0,
            repeat: false,
            kitty: None,
        }
    }

    /// Answer every plain Kitty graphics query with
    /// `ESC _ G i=<id> ; <message> ESC \`, echoing the query's own ID.
    ///
    /// The library picks a fresh ID per query, so the reply cannot be a
    /// constant. A tmux-wrapped query is not answered; see
    /// [`ProbeAnswer::kitty_wrapped`].
    pub fn kitty(message: &'static str) -> Self {
        Self {
            repeat: true,
            kitty: Some(KittyAnswer {
                wrapped: false,
                message,
            }),
            ..Self::new(b"", b"")
        }
    }

    /// Answer every tmux-wrapped Kitty graphics query the way a tmux with
    /// passthrough on delivers the terminal's answer: unwrapped,
    /// `ESC _ G i=<id> ; <message> ESC \`.
    pub fn kitty_wrapped(message: &'static str) -> Self {
        Self {
            repeat: true,
            kitty: Some(KittyAnswer {
                wrapped: true,
                message,
            }),
            ..Self::new(b"", b"")
        }
    }

    /// Answer every occurrence of `query`, as a terminal answers DA1.
    pub fn every(query: &'static [u8], reply: &'static [u8]) -> Self {
        Self {
            repeat: true,
            ..Self::new(query, reply)
        }
    }
}

/// Drive a probe session: read the master stream, answer each pending
/// [`ProbeAnswer`] when its query appears, and stop when the ASCII
/// `until` marker is seen in the collected bytes or `deadline` elapses.
///
/// `until` should be a marker the probe prints *after* the exchange completes,
/// so the child has run to completion before the session is dropped. Observing
/// the query alone only proves the probe asked, not that it finished.
///
/// Returns the raw master bytes collected. Callers can both byte-count OSC
/// query sequences (interleaved from `/dev/tty`) and parse the probe's
/// `key=value` stdout lines from the same buffer, since both share the PTY.
///
/// ## Panics
///
/// Panics if `until` is never seen before `deadline`.
pub fn drive_probe(
    session: &mut OsSession,
    answers: &mut [ProbeAnswer],
    until: &str,
    deadline: Duration,
) -> Vec<u8> {
    let start = Instant::now();
    let mut collected: Vec<u8> = Vec::new();
    let mut scratch = [0u8; 4096];

    loop {
        match session.try_read(&mut scratch) {
            Ok(0) => {}
            Ok(n) => collected.extend_from_slice(&scratch[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }

        for answer in answers.iter_mut() {
            if let Some(kitty) = answer.kitty {
                let ids = kitty_query_ids(&collected, kitty.wrapped);
                while answer.answered < ids.len() {
                    let id = ids[answer.answered];
                    let reply = format!("\x1b_Gi={id};{}\x1b\\", kitty.message);
                    let _ = session.write_all(reply.as_bytes());
                    let _ = session.flush();
                    answer.answered += 1;
                }
                continue;
            }
            let seen = count_occurrences(&collected, answer.query);
            let owed = if answer.repeat { seen } else { seen.min(1) };
            while answer.answered < owed {
                let _ = session.write_all(answer.reply);
                let _ = session.flush();
                answer.answered += 1;
            }
        }

        if find_subslice(&collected, until.as_bytes()) {
            return collected;
        }
        if start.elapsed() > deadline {
            panic!(
                "drive_probe: marker {until:?} not seen within {deadline:?}; collected {} bytes: {:?}",
                collected.len(),
                String::from_utf8_lossy(&collected)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    collected
}

/// The tail every Kitty graphics query the library sends carries after its
/// image ID: one RGB pixel, queried (`a=q`), never displayed.
const KITTY_QUERY_TAIL: &[u8] = b",s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\";

/// The tail of a tmux-wrapped Kitty query: every ESC inside the envelope is
/// doubled once, then the envelope's own `ESC \` closes it.
const KITTY_WRAPPED_TAIL: &[u8] = b",s=1,v=1,a=q,t=d,f=24;AAAA\x1b\x1b\\\x1b\\";

/// The image IDs of every complete Kitty graphics query in `bytes`, in order.
///
/// With `wrapped`, only tmux-wrapped queries
/// (`ESC P tmux; ESC ESC _ G i=<id>,… ESC ESC \ ESC \`) count; without it,
/// only plain ones. A wrapped query contains the plain introducer one byte
/// in, so the two are told apart by the byte before `ESC _ G`.
pub fn kitty_query_ids(bytes: &[u8], wrapped: bool) -> Vec<u32> {
    const INTRO: &[u8] = b"\x1b_Gi=";
    const ENVELOPE: &[u8] = b"\x1bPtmux;\x1b";
    let tail = if wrapped {
        KITTY_WRAPPED_TAIL
    } else {
        KITTY_QUERY_TAIL
    };
    let mut ids = Vec::new();
    let mut i = 0;
    while i + INTRO.len() <= bytes.len() {
        if &bytes[i..i + INTRO.len()] != INTRO {
            i += 1;
            continue;
        }
        let in_envelope = i >= ENVELOPE.len() && &bytes[i - ENVELOPE.len()..i] == ENVELOPE;
        let escaped = i > 0 && bytes[i - 1] == 0x1b;
        let start = i + INTRO.len();
        let digits = bytes[start..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        let after = start + digits;
        let complete = digits > 0 && bytes[after..].starts_with(tail);
        let kind_matches = if wrapped { in_envelope } else { !escaped };
        if complete && kind_matches {
            let id = std::str::from_utf8(&bytes[start..after])
                .ok()
                .and_then(|text| text.parse().ok());
            if let Some(id) = id {
                ids.push(id);
            }
        }
        i = start;
    }
    ids
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    count_occurrences(haystack, needle) > 0
}
