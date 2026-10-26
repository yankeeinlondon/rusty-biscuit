//! One bounded round trip to the controlling terminal, shared by every live
//! query (OSC 10/11/12 colors, the graphics probe, CSI 14 t window pixels,
//! DSR cursor position).
//!
//! Each request is followed by a DA1 (`CSI c`) sentinel. Terminals process
//! input in order and virtually all of them answer DA1, so the DA1 reply marks
//! the end of whatever the terminal was going to say about the request: a
//! terminal that answers DA1 but not the request is known not to support it
//! without waiting out a timeout. The caller chooses the sentinel's bytes, so
//! a request wrapped for tmux passthrough is ordered by a sentinel wrapped the
//! same way.
//!
//! Two process-wide memos keep a non-answering terminal cheap:
//!
//! - **Silent terminal.** When no escape-introduced byte arrives within the
//!   silence budget, the terminal is recorded as silent and every later query
//!   in the process fails immediately without writing anything. A terminal that
//!   answers nothing therefore costs one bounded wait per process, not one per
//!   query.
//! - **Unanswered request.** When DA1 arrives with no reply before it, that
//!   exact request is never written again in this process ([`round_trip`]
//!   only).
//!
//! The terminal itself is reached through a platform [`port`](QueryPort):
//! `/dev/tty` in raw mode on Unix, the console input and output handles on
//! native Windows.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::discovery::raw_mode::TERMINAL_QUERY_MUTEX;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix::open_port;
#[cfg(windows)]
use windows::open_port;

/// The DA1 (primary device attributes) request appended to every query.
pub(crate) const DA1_REQUEST: &[u8] = b"\x1b[c";

/// How long a local terminal may stay completely silent before it is treated
/// as one that answers nothing; this bounds the whole-process cost of a pty
/// nobody reads. WezTerm (20260716 build) was measured answering every query
/// kind, DA1 included, in 170-200 ms from a background pane (2026-10-05), so a
/// budget near that misreads a real terminal as silent and leaks its late
/// reply onto the shell prompt.
pub(crate) const LOCAL_SILENCE_BUDGET: Duration = Duration::from_millis(500);

/// Upper bound on the bytes one exchange collects; terminal replies are a few
/// dozen bytes.
const MAX_RESPONSE_BYTES: usize = 4096;

static TERMINAL_SILENT: AtomicBool = AtomicBool::new(false);
static UNANSWERED: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Why a round trip produced no reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TtyQueryError {
    /// The terminal answered nothing within the silence budget, now or
    /// earlier in this process.
    Silent,
    /// The terminal answered DA1 but not the request, now or earlier.
    Unanswered,
    /// Input was already waiting, and the caller asked not to consume it.
    InputPending,
    /// The controlling terminal could not be opened, configured, or read.
    Io(String),
}

impl std::fmt::Display for TtyQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Silent => f.write_str("terminal does not answer queries"),
            Self::Unanswered => f.write_str("terminal does not answer this query"),
            Self::InputPending => f.write_str("input was pending; query not sent"),
            Self::Io(e) => f.write_str(e),
        }
    }
}

/// One read from a [`QueryPort`].
pub(crate) enum ReadOutcome {
    /// This many bytes were read into the buffer.
    Data(usize),
    /// Nothing arrived within the timeout.
    TimedOut,
    /// The terminal hung up; waiting cannot help.
    Hangup,
}

/// The controlling terminal, opened and configured for one exchange. Dropping
/// it restores the terminal's modes.
pub(crate) trait QueryPort {
    /// Writes and flushes `bytes`.
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()>;
    /// Waits at most `timeout` for input and reads what is available.
    fn read_within(&mut self, buffer: &mut [u8], timeout: Duration) -> std::io::Result<ReadOutcome>;
    /// Whether input is already waiting, without consuming it.
    fn input_pending(&mut self) -> std::io::Result<bool>;
    /// Called once with everything the exchange read; the ranges in
    /// `typeahead` are input that was not part of a terminal reply. A port
    /// that can put input back does so; Unix discards it.
    fn return_typeahead(&mut self, response: &[u8], typeahead: &[std::ops::Range<usize>]);
}

/// How one [`exchange`] behaves.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExchangeOptions {
    /// Record a silent terminal process-wide (see the module docs). An
    /// exchange whose sentinel some terminals ignore (a tmux-wrapped one)
    /// turns this off so its silence says nothing about the terminal.
    pub record_silence: bool,
    /// Send nothing when input is already waiting, so typed keys are not
    /// consumed.
    pub skip_if_input_pending: bool,
}

impl Default for ExchangeOptions {
    fn default() -> Self {
        Self {
            record_silence: true,
            skip_if_input_pending: false,
        }
    }
}

/// What one [`exchange`] read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exchange {
    /// Every byte read, up to and including the DA1 reply when one arrived.
    pub bytes: Vec<u8>,
    /// Where the DA1 reply starts in `bytes`; `None` when the deadline came
    /// first.
    pub da1_at: Option<usize>,
}

/// Whether an earlier round trip in this process found the terminal silent.
pub(crate) fn terminal_known_silent() -> bool {
    TERMINAL_SILENT.load(Ordering::Relaxed)
}

/// Write `request` plus the DA1 sentinel to the controlling terminal and
/// return the bytes the terminal sent before its DA1 reply.
///
/// `timeout` bounds the whole exchange. The wait for the *first* escape byte
/// is further capped by [`silence_budget`], so a silent terminal costs at
/// most that budget. Callers own their preconditions (TTY, CI, multiplexer).
///
/// When the deadline passes after some reply bytes but before DA1 (a terminal
/// that answers the request but not DA1), the collected bytes are returned.
pub(crate) fn round_trip(request: &[u8], timeout: Duration) -> Result<Vec<u8>, TtyQueryError> {
    if is_memoized_unanswered(request) {
        return Err(TtyQueryError::Unanswered);
    }
    let Exchange { mut bytes, da1_at } =
        exchange(request, DA1_REQUEST, timeout, ExchangeOptions::default())?;
    if let Some(at) = da1_at {
        bytes.truncate(at);
        if !bytes.contains(&0x1b) {
            memoize_unanswered(request);
            return Err(TtyQueryError::Unanswered);
        }
    }
    Ok(bytes)
}

/// Write `request` then `sentinel` and read until a DA1 reply or the
/// deadline, returning everything read.
///
/// Unlike [`round_trip`], a DA1 reply with nothing before it is a result, not
/// an error: the graphics probe reads "answered DA1, not the Kitty query" as
/// an answer.
///
/// ## Errors
///
/// [`TtyQueryError::Silent`] when nothing escape-introduced arrived within the
/// silence budget (or an earlier exchange recorded the terminal silent),
/// [`TtyQueryError::InputPending`] when `options` asked to skip busy input,
/// and [`TtyQueryError::Io`] when the terminal could not be used.
pub(crate) fn exchange(
    request: &[u8],
    sentinel: &[u8],
    timeout: Duration,
    options: ExchangeOptions,
) -> Result<Exchange, TtyQueryError> {
    if terminal_known_silent() {
        return Err(TtyQueryError::Silent);
    }

    let _lock = TERMINAL_QUERY_MUTEX
        .lock()
        .map_err(|_| TtyQueryError::Io("terminal query mutex poisoned".into()))?;
    // Re-check under the lock: a concurrent query may have just found the
    // terminal silent while this one waited.
    if terminal_known_silent() {
        return Err(TtyQueryError::Silent);
    }
    let mut port = open_port()?;

    if options.skip_if_input_pending
        && port
            .input_pending()
            .map_err(|e| TtyQueryError::Io(e.to_string()))?
    {
        return Err(TtyQueryError::InputPending);
    }

    let mut wire = Vec::with_capacity(request.len() + sentinel.len());
    wire.extend_from_slice(request);
    wire.extend_from_slice(sentinel);
    port.write_all(&wire)
        .map_err(|e| TtyQueryError::Io(e.to_string()))?;

    let start = Instant::now();
    let deadline = start + timeout;
    let silence_deadline = start + silence_budget(timeout);
    let mut response: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 256];
    let mut da1_at = None;

    loop {
        if let Some(at) = find_da1_reply(&response) {
            let end = da1_end(&response, at);
            da1_at = Some(at);
            port.return_typeahead(&response[..end], &typeahead_ranges(&response[..end]));
            response.truncate(end);
            break;
        }
        // Stray non-escape bytes (typeahead, a `^D` some pty hosts inject on
        // EOF) are not evidence that the terminal is listening.
        let heard = response.contains(&0x1b);
        let limit = if heard {
            deadline
        } else {
            silence_deadline.min(deadline)
        };
        let now = Instant::now();
        if now >= limit || response.len() >= MAX_RESPONSE_BYTES {
            port.return_typeahead(&response, &typeahead_ranges(&response));
            break;
        }
        match port.read_within(&mut buffer, limit - now) {
            Ok(ReadOutcome::Data(n)) => response.extend_from_slice(&buffer[..n]),
            Ok(ReadOutcome::TimedOut) => {}
            Ok(ReadOutcome::Hangup) => {
                port.return_typeahead(&response, &typeahead_ranges(&response));
                break;
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(e) => return Err(TtyQueryError::Io(e.to_string())),
        }
    }

    if da1_at.is_some() || response.contains(&0x1b) {
        return Ok(Exchange {
            bytes: response,
            da1_at,
        });
    }
    if options.record_silence {
        TERMINAL_SILENT.store(true, Ordering::Relaxed);
        tracing::debug!(
            waited_ms = start.elapsed().as_millis() as u64,
            "terminal answered nothing; skipping further terminal queries this process"
        );
    }
    Err(TtyQueryError::Silent)
}

/// Environment variable overriding [`LOCAL_SILENCE_BUDGET`], in milliseconds.
pub(crate) const SILENCE_BUDGET_ENV: &str = "BISCUIT_TERMINAL_QUERY_SILENCE_MS";

/// The longest a query waits for the terminal's first escape byte, never more
/// than `timeout`.
///
/// [`SILENCE_BUDGET_ENV`] wins when it parses. Otherwise remote sessions keep
/// the caller's full timeout, since a network round trip can legitimately
/// exceed the local budget.
fn silence_budget(timeout: Duration) -> Duration {
    use crate::discovery::detection::{Connection, detect_connection};
    if let Some(ms) = std::env::var(SILENCE_BUDGET_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
    {
        return timeout.min(Duration::from_millis(ms));
    }
    match detect_connection() {
        Connection::Local => timeout.min(LOCAL_SILENCE_BUDGET),
        _ => timeout,
    }
}

fn is_memoized_unanswered(request: &[u8]) -> bool {
    UNANSWERED
        .lock()
        .map(|seen| seen.iter().any(|r| r == request))
        .unwrap_or(false)
}

fn memoize_unanswered(request: &[u8]) {
    if let Ok(mut seen) = UNANSWERED.lock()
        && !seen.iter().any(|r| r == request)
    {
        seen.push(request.to_vec());
    }
}

/// Index of the first complete DA1 reply (`ESC [ ? <digits;> c`), if any.
pub(crate) fn find_da1_reply(bytes: &[u8]) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = bytes[from..].windows(3).position(|w| w == b"\x1b[?") {
        let start = from + rel;
        let params = &bytes[start + 3..];
        let len = params
            .iter()
            .take_while(|b| b.is_ascii_digit() || **b == b';')
            .count();
        if params.get(len) == Some(&b'c') {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

/// One past the final `c` of the DA1 reply starting at `at`.
fn da1_end(bytes: &[u8], at: usize) -> usize {
    bytes[at..]
        .iter()
        .position(|&b| b == b'c')
        .map_or(bytes.len(), |rel| at + rel + 1)
}

/// The ranges of `bytes` that are not part of an escape sequence: input the
/// user typed while the exchange was reading.
///
/// Recognized sequences are CSI (`ESC [` … final byte), OSC (`ESC ]` … BEL
/// or ST), and APC/DCS/PM/SOS (`ESC _`, `ESC P`, `ESC ^`, `ESC X` … ST). An
/// unterminated sequence at the end is treated as reply bytes, not typeahead.
pub(crate) fn typeahead_ranges(bytes: &[u8]) -> Vec<std::ops::Range<usize>> {
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut push = |range: std::ops::Range<usize>| {
        if range.is_empty() {
            return;
        }
        match ranges.last_mut() {
            Some(last) if last.end == range.start => last.end = range.end,
            _ => ranges.push(range),
        }
    };
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != 0x1b {
            push(i..i + 1);
            i += 1;
            continue;
        }
        let Some(&kind) = bytes.get(i + 1) else {
            break;
        };
        i = match kind {
            b'[' => {
                let body = i + 2;
                bytes[body..]
                    .iter()
                    .position(|b| (0x40..=0x7e).contains(b))
                    .map_or(bytes.len(), |rel| body + rel + 1)
            }
            b']' | b'_' | b'P' | b'^' | b'X' => {
                let body = i + 2;
                let mut end = bytes.len();
                let mut j = body;
                while j < bytes.len() {
                    if kind == b']' && bytes[j] == 0x07 {
                        end = j + 1;
                        break;
                    }
                    if bytes[j] == 0x1b && bytes.get(j + 1) == Some(&b'\\') {
                        end = j + 2;
                        break;
                    }
                    j += 1;
                }
                end
            }
            // Alt-modified keys arrive as ESC plus the key; both are input.
            _ => {
                push(i..i + 2);
                i + 2
            }
        };
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn da1_reply_is_found_after_a_query_reply() {
        let bytes = b"\x1b]11;rgb:0000/0000/0000\x07\x1b[?62;22c";
        assert_eq!(find_da1_reply(bytes), Some(24));
    }

    #[test]
    fn da1_reply_alone_is_found_at_start() {
        assert_eq!(find_da1_reply(b"\x1b[?1;2c"), Some(0));
    }

    #[test]
    fn incomplete_or_foreign_csi_is_not_da1() {
        assert_eq!(find_da1_reply(b"\x1b[?62;22"), None);
        assert_eq!(find_da1_reply(b"\x1b[12;34R"), None);
        assert_eq!(find_da1_reply(b"\x1b[?2027;1$y"), None);
    }

    #[test]
    fn unanswered_memo_matches_exact_request_only() {
        let request = b"\x1b[test-memo-unique t";
        assert!(!is_memoized_unanswered(request));
        memoize_unanswered(request);
        assert!(is_memoized_unanswered(request));
        assert!(!is_memoized_unanswered(b"\x1b[other t"));
    }

    #[test]
    fn da1_end_is_one_past_its_final_byte() {
        let bytes = b"ab\x1b[?62;4cxy";
        assert_eq!(da1_end(bytes, 2), 10);
    }

    #[test]
    fn typeahead_is_everything_outside_replies() {
        let bytes = b"ab\x1b]11;rgb:0/0/0\x07c\x1b_Gi=1;OK\x1b\\d\x1b[?62;4c";
        let ranges = typeahead_ranges(bytes);
        let typed: Vec<u8> = ranges.iter().flat_map(|r| bytes[r.clone()].to_vec()).collect();
        assert_eq!(typed, b"abcd");
    }

    #[test]
    fn typeahead_keeps_alt_keys_and_drops_an_unterminated_reply() {
        assert_eq!(typeahead_ranges(b"\x1bx"), vec![0..2]);
        assert_eq!(typeahead_ranges(b"q\x1b_Gi=1;O"), vec![0..1]);
    }
}
