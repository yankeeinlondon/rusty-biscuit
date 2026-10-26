//! Hybrid OSC color query (actual + heuristic) plus terminal default fallbacks.

use std::time::Duration;

use crate::discovery::detection::{TerminalApp, get_terminal_app, is_tty};
use crate::discovery::os_detection::is_ci;

use super::parse::parse_osc_color_response;
use super::parse::{ansi_index_to_rgb, parse_colorfgbg};
use super::types::DEFAULT_TIMEOUT;
use super::types::{ColorSource, OscQueryError, RgbValue};

/// Tracing target marking one *actual* tty round-trip attempt for an OSC code.
///
/// Round-trip attempts are the quantity the per-process colour caches exist to
/// suppress, and they cannot be counted from outside the process when the
/// terminal is a real emulator: WezTerm/Kitty consume the query and answer on
/// the wire, so there is no PTY master to tally request bytes on. Emitting the
/// attempt as its own tracing event lets an in-repo observer — currently
/// `examples/discovery_probe.rs`, which feeds the Level-2 cache proof in
/// `tests/level2/level2_terminal_osc_wezterm.rs` — count attempts with a local layer
/// instead of the library exporting a counter as public API.
///
/// The event carries a `code` field; a dedicated target keeps it distinguishable
/// from the outcome/fallback events this module also logs at `debug`.
pub(crate) const OSC_QUERY_ATTEMPT_TARGET: &str = "biscuit_terminal::osc_query_attempt";

/// Human-readable name for an OSC color query code.
fn osc_color_name(code: u8) -> &'static str {
    match code {
        10 => "foreground color",
        11 => "background color",
        12 => "cursor color",
        _ => "unknown color",
    }
}

/// Query terminal color using a hybrid approach.
///
/// This function tries multiple detection methods in order:
/// 1. Actual OSC query (if supported and not in CI/multiplexer)
/// 2. `COLORFGBG` environment variable
/// 3. Terminal application defaults
///
/// The fallback chain ensures we always return a reasonable result
/// when possible, while preferring actual terminal queries for accuracy.
///
/// The live step for OSC 10 and 11 is one batched round trip shared by both
/// codes and cached per process, so building a `Terminal` (which wants both)
/// costs the terminal one exchange.
///
/// The color is [`ColorSource::Measured`] only when the live query answered.
pub(super) fn query_osc_color(code: u8) -> Option<(RgbValue, ColorSource)> {
    resolve_color(code, true, || live_color_cached(code))
}

/// Query terminal color with a custom timeout (one uncached live attempt).
pub(super) fn query_osc_color_with_timeout(code: u8, timeout: Duration) -> Option<RgbValue> {
    resolve_color(code, true, || {
        live_colors(&[code], timeout).into_iter().next().flatten()
    })
    .map(|(color, _)| color)
}

/// The live OSC 10/11 answers come from the once-per-process startup
/// exchange, which the graphics probe joins.
fn live_color_cached(code: u8) -> Option<RgbValue> {
    match code {
        10 | 11 => {
            let [fg, bg] = crate::discovery::startup_exchange::live_fg_bg();
            if code == 10 { fg } else { bg }
        }
        _ => live_colors(&[code], DEFAULT_TIMEOUT)
            .into_iter()
            .next()
            .flatten(),
    }
}

/// One round trip asking for every code in `codes`; one answer per code.
fn live_colors(codes: &[u8], timeout: Duration) -> Vec<Option<RgbValue>> {
    for &code in codes {
        tracing::debug!(
            target: OSC_QUERY_ATTEMPT_TARGET,
            code,
            "OSC{} actual query attempted",
            code
        );
    }
    match query_osc_batch(codes, timeout) {
        Ok(response) => codes
            .iter()
            .map(|&code| {
                let color = find_osc_color(&response, code);
                if color.is_none() {
                    tracing::debug!(code, "OSC{} ({}) not answered", code, osc_color_name(code));
                }
                color
            })
            .collect(),
        Err(e) => {
            tracing::debug!(
                codes = ?codes,
                error = %e,
                "OSC color query failed, falling back to heuristics"
            );
            vec![None; codes.len()]
        }
    }
}

/// Whether a live color query may be sent: a terminal on stdout, not CI,
/// not a Windows SSH server session, a terminal known to answer, no
/// multiplexer, and probing not turned off by `BISCUIT_TERMINAL_PROBE` (any
/// value other than `auto` turns it off).
pub(crate) fn color_query_wanted() -> bool {
    let tty = is_tty();
    let ci = is_ci();
    if !tty {
        tracing::debug!("color query skipped: not a TTY");
    }
    if ci {
        tracing::debug!("color query skipped: CI environment");
    }
    let probe_allowed = std::env::var_os(crate::discovery::detection::PROBE_ENV)
        .is_none_or(|value| value == "auto");
    // The Windows OpenSSH server's console host sits between this program
    // and the client's terminal; it is never queried.
    let windows_ssh_server = cfg!(windows) && std::env::var_os("SSH_CONNECTION").is_some();
    tty && !ci
        && probe_allowed
        && !windows_ssh_server
        && super::support::answers_color_queries(&get_terminal_app())
        && detect_multiplexer().is_none()
}

/// The fallback chain without a live query, for a terminal constructed with
/// probing off. Always an estimate.
pub(crate) fn heuristic_color(code: u8) -> Option<RgbValue> {
    resolve_color(code, false, || None).map(|(color, _)| color)
}

/// The gate and fallback chain around a live color lookup.
fn resolve_color(
    code: u8,
    live_allowed: bool,
    live: impl FnOnce() -> Option<RgbValue>,
) -> Option<(RgbValue, ColorSource)> {
    let answer = if live_allowed && color_query_wanted() { live() } else { None };
    let colorfgbg = std::env::var("COLORFGBG").ok();
    sourced_color(code, answer, colorfgbg.as_deref(), &get_terminal_app())
}

/// The terminal's `answer` to the live query when there is one, else the
/// `COLORFGBG` and terminal-app estimates.
fn sourced_color(
    code: u8,
    answer: Option<RgbValue>,
    colorfgbg: Option<&str>,
    term_app: &TerminalApp,
) -> Option<(RgbValue, ColorSource)> {
    if let Some(color) = answer {
        tracing::debug!(
            code,
            r = color.r,
            g = color.g,
            b = color.b,
            source = "actual_query",
            "OSC{} color detected via actual query",
            code
        );
        return Some((color, ColorSource::Measured));
    }
    if let Some(colorfgbg) = colorfgbg
        && let Some(color) = parse_colorfgbg(colorfgbg, code)
    {
        tracing::debug!(
            code,
            colorfgbg = %colorfgbg,
            r = color.r,
            g = color.g,
            b = color.b,
            source = "COLORFGBG",
            "OSC{} color detected via COLORFGBG env var",
            code
        );
        // A palette index mapped to the standard ANSI colors, and inherited
        // through multiplexers and SSH: an estimate, not the terminal's color.
        return Some((color, ColorSource::Estimated));
    }

    if let Some(color) = get_terminal_default_color(term_app, code) {
        tracing::debug!(
            code,
            terminal = ?term_app,
            r = color.r,
            g = color.g,
            b = color.b,
            source = "terminal_defaults",
            "OSC{} color detected via terminal defaults",
            code
        );
        return Some((color, ColorSource::Estimated));
    }

    // Suppress unused warnings on non-unix
    let _ = ansi_index_to_rgb;

    tracing::debug!(
        code,
        query = osc_color_name(code),
        "OSC{} ({}) detection failed, no source available",
        code,
        osc_color_name(code)
    );
    None
}

/// Get default colors for known terminal applications.
fn get_terminal_default_color(app: &TerminalApp, code: u8) -> Option<RgbValue> {
    match app {
        // No guess: it is never queried, and its profiles range from the
        // white "Basic" (which follows the system appearance) to dark ones,
        // so a fixed white default once read a dark profile as light.
        // `color_mode` falls back to the system appearance instead.
        TerminalApp::AppleTerminal => None,

        // Most modern terminals default to dark themes
        TerminalApp::Kitty
        | TerminalApp::Alacritty
        | TerminalApp::Wezterm
        | TerminalApp::ITerm2
        | TerminalApp::Ghostty
        | TerminalApp::Warp
        | TerminalApp::Foot
        | TerminalApp::Contour
        | TerminalApp::GnomeTerminal
        | TerminalApp::Konsole
        | TerminalApp::VsCode
        | TerminalApp::WindowsTerminal
        | TerminalApp::Wast => match code {
            10 | 12 => Some(RgbValue::new(229, 229, 229)),
            11 => Some(RgbValue::new(30, 30, 30)),
            _ => None,
        },

        TerminalApp::Other(_) => match code {
            10 | 12 => Some(RgbValue::new(229, 229, 229)),
            11 => Some(RgbValue::new(30, 30, 30)),
            _ => None,
        },
    }
}

/// Check if running inside a terminal multiplexer.
pub(crate) fn detect_multiplexer() -> Option<&'static str> {
    if std::env::var("TMUX").is_ok() {
        Some("tmux")
    } else if std::env::var("ZELLIJ").is_ok() {
        Some("zellij")
    } else if std::env::var("STY").is_ok() {
        Some("screen")
    } else {
        None
    }
}

/// Perform an actual OSC query to the terminal.
///
/// Sends the OSC query followed by a DA1 sentinel and reads the reply with a
/// deadline. A terminal that answers DA1 but not the query fails fast with
/// [`OscQueryError::Unsupported`]; one that answers nothing fails within the
/// silence budget, and every later query in the process then fails at once.
///
/// ## Errors
///
/// [`OscQueryError::Timeout`] when the terminal answered nothing,
/// [`OscQueryError::Unsupported`] when it answered DA1 but not the query.
///
pub fn query_osc_actual(code: u8, timeout: Duration) -> Result<RgbValue, OscQueryError> {
    let response = query_osc_batch(&[code], timeout)?;
    find_osc_color(&response, code)
        .ok_or_else(|| OscQueryError::ParseError("invalid response format".into()))
}

/// Send one OSC `?` query per code in a single write and return the raw reply.
fn query_osc_batch(codes: &[u8], timeout: Duration) -> Result<Vec<u8>, OscQueryError> {
    use crate::discovery::tty_query::{TtyQueryError, round_trip};

    // Pre-flight checks
    if !is_tty() {
        return Err(OscQueryError::NotTty);
    }
    if is_ci() {
        return Err(OscQueryError::CiEnvironment);
    }
    if let Some(mux) = detect_multiplexer() {
        return Err(OscQueryError::Multiplexer(mux.to_string()));
    }

    let request: Vec<u8> = codes
        .iter()
        .flat_map(|code| format!("\x1b]{code};?\x07").into_bytes())
        .collect();
    round_trip(&request, timeout).map_err(|e| match e {
        TtyQueryError::Silent => OscQueryError::Timeout(timeout),
        TtyQueryError::Unanswered => OscQueryError::Unsupported(codes[0]),
        TtyQueryError::InputPending => OscQueryError::IoError(e.to_string()),
        TtyQueryError::Io(e) => OscQueryError::IoError(e),
    })
}

/// Find and parse the reply for `code` among the OSC replies in `response`.
pub(crate) fn find_osc_color(response: &[u8], code: u8) -> Option<RgbValue> {
    let mut from = 0;
    while let Some(rel) = response[from..].windows(2).position(|w| w == b"\x1b]") {
        let start = from + rel;
        if let Some(color) = parse_osc_color_response(&response[start..], code) {
            return Some(color);
        }
        from = start + 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_terminal_has_no_default_color() {
        let app = TerminalApp::AppleTerminal;
        for code in [10, 11, 12] {
            assert_eq!(get_terminal_default_color(&app, code), None, "OSC{code}");
        }
    }


    #[test]
    fn test_get_terminal_default_color_modern_terminals() {
        let terminals = [
            TerminalApp::Kitty,
            TerminalApp::Alacritty,
            TerminalApp::Wezterm,
            TerminalApp::ITerm2,
            TerminalApp::Ghostty,
        ];

        for app in terminals {
            let bg = get_terminal_default_color(&app, 11);
            assert!(bg.is_some(), "{:?} should have default bg", app);
            assert!(bg.unwrap().is_dark(), "{:?} should default to dark bg", app);

            let fg = get_terminal_default_color(&app, 10);
            assert!(fg.is_some(), "{:?} should have default fg", app);
            assert!(
                fg.unwrap().is_light(),
                "{:?} should default to light fg",
                app
            );
        }
    }

    #[test]
    fn test_get_terminal_default_color_unknown() {
        let app = TerminalApp::Other("unknown".to_string());

        let bg = get_terminal_default_color(&app, 11);
        assert!(bg.is_some());
        assert!(bg.unwrap().is_dark());
    }

    #[test]
    fn only_a_live_answer_is_measured() {
        let kitty = TerminalApp::Kitty;
        let answer = RgbValue::new(40, 44, 52);
        assert_eq!(
            sourced_color(11, Some(answer), Some("15;0"), &kitty),
            Some((answer, ColorSource::Measured))
        );
        assert_eq!(
            sourced_color(11, None, None, &kitty),
            Some((RgbValue::new(30, 30, 30), ColorSource::Estimated)),
            "a per-app default is an estimate"
        );
        assert!(
            matches!(sourced_color(11, None, Some("15;0"), &kitty), Some((_, ColorSource::Estimated))),
            "a COLORFGBG index is an estimate"
        );
        assert_eq!(sourced_color(11, None, None, &TerminalApp::AppleTerminal), None);
    }

    #[test]
    fn find_osc_color_picks_each_code_from_a_batched_reply() {
        let reply = b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07\x1b]11;rgb:1e1e/1e1e/1e1e\x1b\\";
        assert_eq!(find_osc_color(reply, 10), Some(RgbValue::new(229, 229, 229)));
        assert_eq!(find_osc_color(reply, 11), Some(RgbValue::new(30, 30, 30)));
        assert_eq!(find_osc_color(reply, 12), None);
    }

    #[test]
    fn test_get_terminal_default_color_invalid_code() {
        let app = TerminalApp::Kitty;
        assert!(get_terminal_default_color(&app, 99).is_none());
    }

    #[test]
    fn test_detect_multiplexer_none() {
        if std::env::var("TMUX").is_err()
            && std::env::var("ZELLIJ").is_err()
            && std::env::var("STY").is_err()
        {
            assert!(detect_multiplexer().is_none());
        }
    }

    #[test]
    fn test_query_osc_actual_not_tty_in_tests() {
        let result = query_osc_actual(11, Duration::from_millis(50));

        match result {
            Err(OscQueryError::NotTty) | Err(OscQueryError::CiEnvironment) => {}
            Err(OscQueryError::Multiplexer(_)) => {}
            Ok(_) => {}
            Err(OscQueryError::Timeout(_)) => {}
            Err(OscQueryError::ParseError(_)) => {}
            Err(e) => {
                panic!("Unexpected error variant: {:?}", e);
            }
        }
    }
}
