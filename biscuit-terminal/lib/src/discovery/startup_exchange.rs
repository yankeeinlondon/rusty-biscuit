//! The once-per-process exchange `Terminal::new()` makes: the OSC 10/11
//! color queries, the OSC 4 queries for ANSI palette slots 0–15, on native
//! Windows the cell-size query (`CSI 16 t`), and,
//! when the environment cannot decide the graphics route, the Kitty graphics
//! query, written together and ended by one DA1 sentinel.
//! Inside a visible tmux the graphics part is the DA1 reply alone: tmux
//! answers it for itself, and the Kitty query is left out.
//!
//! Whichever caller needs the exchange first (the color lookup, the palette
//! lookup, or the graphics route) sends everything the process needs, so
//! graphics probing adds no extra round trip and no new moment at which input
//! is read.
//! Results are cached per process and per output route: the direct exchange
//! here, the tmux-wrapped one separately. A probe skipped because input was
//! waiting is not cached, so a later construction may still probe.
//!
//! The developer guide is `biscuit-terminal/docs/discovery/terminal-queries.md`.

use std::sync::Mutex;

use crate::discovery::detection::probe::{
    Da1Reply, KittyReply, ProbeReply, ProbeResult, ProbeSkip, kitty_query, next_kitty_id, wrap_for_tmux,
};
use crate::discovery::detection::{RouteInputs, probe_plan};
use crate::discovery::fonts::CellSize;
use crate::discovery::fonts::window_size::{CELL_SIZE_REQUEST, find_cell_size_reply};
use crate::discovery::osc_queries::RgbValue;
use crate::discovery::osc_queries::parse::parse_osc4_palette;
use crate::discovery::osc_queries::query::{
    OSC_QUERY_ATTEMPT_TARGET, color_query_wanted, find_osc_color, palette_query_wanted,
};
use crate::discovery::osc_queries::types::DEFAULT_TIMEOUT;
use crate::discovery::tty_query::{DA1_REQUEST, ExchangeOptions, TtyQueryError, exchange};

#[derive(Default)]
struct DirectState {
    colors: Option<[Option<RgbValue>; 2]>,
    /// ANSI slots 0–15; `Some` once asked, every slot `None` when unanswered.
    palette: Option<[Option<RgbValue>; 16]>,
    graphics: Option<ProbeResult>,
    /// `Some(None)` once asked and not usefully answered.
    cell: Option<Option<CellSize>>,
}

static DIRECT: Mutex<DirectState> = Mutex::new(DirectState {
    colors: None,
    palette: None,
    graphics: None,
    cell: None,
});

/// Whether the exchange asks for the cell size. Unix reads it from the
/// `TIOCGWINSZ` ioctl on every render instead (`fonts::cell_size`); native
/// Windows has no pixel ioctl, so only the terminal can say.
const ASKS_CELL_SIZE: bool = cfg!(windows);
static TUNNELED: Mutex<Option<ProbeResult>> = Mutex::new(None);

/// The live OSC 10 and 11 answers, `[foreground, background]`.
///
/// Callers have already applied the color-query gate. When the graphics
/// route also wants a direct probe, it rides along, and so does the palette.
pub(crate) fn live_fg_bg() -> [Option<RgbValue>; 2] {
    let Ok(mut state) = DIRECT.lock() else {
        return [None, None];
    };
    if let Some(colors) = state.colors {
        return colors;
    }
    let plan = probe_plan(&RouteInputs::from_process());
    let graphics = (state.graphics.is_none() && plan.direct).then_some(!plan.direct_da1_only);
    let palette = state.palette.is_none();
    let cell = ASKS_CELL_SIZE && state.cell.is_none();
    run_direct(&mut state, true, palette, graphics, cell);
    state.colors.unwrap_or([None, None])
}

/// The terminal's ANSI palette, slots 0–15, sending the exchange if no
/// earlier caller did (the colors and graphics probe ride along). Every slot
/// is `None` when the palette query gate refuses (no TTY, CI, a multiplexer,
/// probing off, a terminal not known to answer) or the
/// terminal did not answer; a slot is `None` alone when its reply was
/// missing or malformed.
///
/// Asked at most once per process: an unanswered query is not sent again.
pub(crate) fn live_palette() -> [Option<RgbValue>; 16] {
    if !palette_query_wanted() {
        return [None; 16];
    }
    let Ok(mut state) = DIRECT.lock() else {
        return [None; 16];
    };
    if let Some(palette) = state.palette {
        return palette;
    }
    let colors = state.colors.is_none();
    let plan = probe_plan(&RouteInputs::from_process());
    let graphics = (state.graphics.is_none() && plan.direct).then_some(!plan.direct_da1_only);
    let cell = ASKS_CELL_SIZE && state.cell.is_none();
    run_direct(&mut state, colors, true, graphics, cell);
    state.palette.unwrap_or([None; 16])
}

/// The terminal's `CSI 16 t` answer, sending the exchange if no earlier
/// caller did (the colors ride along). `None` when the terminal is not one
/// the startup exchange may query (the color-query gate: no TTY, CI, probing
/// off, the Windows SSH server, a legacy console), or when the terminal gave
/// no usable answer.
///
/// Asked once per process: a cell's pixel size follows the font, not the
/// window, so a resize does not change it, and asking again at render time
/// would read console input while the program may be reading it too.
#[cfg(windows)]
pub(crate) fn live_cell_size() -> Option<CellSize> {
    let Ok(mut state) = DIRECT.lock() else {
        return None;
    };
    if let Some(cell) = state.cell {
        return cell;
    }
    if !color_query_wanted() {
        return None;
    }
    let colors = state.colors.is_none();
    let palette = state.palette.is_none();
    run_direct(&mut state, colors, palette, None, true);
    state.cell.flatten()
}

/// The `CSI 16 t` answer an earlier exchange already received, without
/// sending one. `Terminal::cell_size` reads this so a terminal built with
/// `ProbeMode::Off` never starts an exchange at render time; with
/// `ProbeMode::Auto` construction already asked in the same exchange.
#[cfg(windows)]
pub(crate) fn received_cell_size() -> Option<CellSize> {
    DIRECT.lock().ok().and_then(|state| state.cell.flatten())
}

/// The direct graphics probe's result, sending the exchange if no earlier
/// caller did. The colors ride along when they are wanted and not yet known.
/// `kitty` adds the Kitty query to the DA1 request; without it the result
/// only carries the DA1 reply.
pub(crate) fn direct_probe(kitty: bool) -> ProbeResult {
    let Ok(mut state) = DIRECT.lock() else {
        return ProbeResult::Skipped(ProbeSkip::Unavailable("probe state poisoned".into()));
    };
    if let Some(result) = &state.graphics {
        return result.clone();
    }
    let colors = state.colors.is_none() && color_query_wanted();
    let palette = state.palette.is_none() && palette_query_wanted();
    let cell = ASKS_CELL_SIZE && state.cell.is_none();
    run_direct(&mut state, colors, palette, Some(kitty), cell)
        .unwrap_or_else(|| ProbeResult::Skipped(ProbeSkip::Unavailable("probe not sent".into())))
}

/// The tmux-wrapped graphics probe's result: the Kitty query and its DA1
/// sentinel, each wrapped once for tmux passthrough. Never answered by tmux
/// itself, so its silence records nothing about the terminal.
pub(crate) fn tunneled_probe() -> ProbeResult {
    let Ok(mut cached) = TUNNELED.lock() else {
        return ProbeResult::Skipped(ProbeSkip::Unavailable("probe state poisoned".into()));
    };
    if let Some(result) = &*cached {
        return result.clone();
    }
    let id = next_kitty_id();
    let options = ExchangeOptions {
        record_silence: false,
        skip_if_input_pending: true,
    };
    let result = match exchange(
        &wrap_for_tmux(&kitty_query(id)),
        &wrap_for_tmux(DA1_REQUEST),
        DEFAULT_TIMEOUT,
        options,
    ) {
        Ok(reply) => ProbeResult::Replied(ProbeReply::parse(&reply.bytes, id)),
        Err(error) => probe_error(error),
    };
    if result != ProbeResult::Skipped(ProbeSkip::BusyInput) {
        *cached = Some(result.clone());
    }
    tracing::debug!(?result, "tunneled graphics probe");
    result
}

/// Sends one direct exchange and records what it answered. `graphics` is
/// `None` when no graphics probe is wanted, otherwise whether the Kitty query
/// joins the DA1 request; `palette` adds the OSC 4 queries and `cell` the
/// cell-size query. Returns the graphics result when it was part of the
/// exchange.
fn run_direct(
    state: &mut DirectState,
    colors: bool,
    palette: bool,
    graphics: Option<bool>,
    cell: bool,
) -> Option<ProbeResult> {
    if !colors && !palette && graphics.is_none() && !cell {
        return None;
    }
    let id = (graphics == Some(true)).then(next_kitty_id);
    if colors {
        for code in [10u8, 11] {
            tracing::debug!(
                target: OSC_QUERY_ATTEMPT_TARGET,
                code,
                "OSC{} actual query attempted",
                code
            );
        }
    }
    if palette {
        tracing::debug!(target: OSC_QUERY_ATTEMPT_TARGET, code = 4u8, "OSC4 actual query attempted");
    }
    let request = direct_request(colors, palette, cell, id);
    let options = ExchangeOptions {
        record_silence: true,
        // Colors have always been read regardless of waiting input; only the
        // graphics query holds back.
        skip_if_input_pending: graphics.is_some(),
    };

    match exchange(&request, DA1_REQUEST, DEFAULT_TIMEOUT, options) {
        Ok(reply) => {
            if colors {
                state.colors = Some([
                    find_osc_color(&reply.bytes, 10),
                    find_osc_color(&reply.bytes, 11),
                ]);
            }
            if palette {
                let found = parse_osc4_palette(&reply.bytes);
                tracing::debug!(
                    answered = found.iter().filter(|slot| slot.is_some()).count(),
                    "ANSI palette from OSC 4"
                );
                state.palette = Some(found);
            }
            if cell {
                let measured = find_cell_size_reply(&reply.bytes);
                tracing::debug!(?measured, "cell size from CSI 16 t");
                state.cell = Some(measured);
            }
            let result = graphics.map(|_| {
                ProbeResult::Replied(match id {
                    Some(id) => ProbeReply::parse(&reply.bytes, id),
                    None => ProbeReply {
                        kitty: KittyReply::NoReply,
                        da1: Da1Reply::find(&reply.bytes),
                    },
                })
            });
            if let Some(result) = &result {
                tracing::debug!(?result, "direct graphics probe");
                state.graphics = Some(result.clone());
            }
            result
        }
        Err(TtyQueryError::InputPending) => {
            tracing::debug!("graphics probe skipped: input was waiting");
            if colors || palette || cell {
                run_direct(state, colors, palette, None, cell);
            }
            Some(ProbeResult::Skipped(ProbeSkip::BusyInput))
        }
        Err(error) => {
            tracing::debug!(%error, "startup exchange failed");
            if colors {
                state.colors = Some([None, None]);
            }
            if palette {
                state.palette = Some([None; 16]);
            }
            if cell {
                state.cell = Some(None);
            }
            let result = graphics.map(|_| probe_error(error));
            if let Some(result) = &result {
                state.graphics = Some(result.clone());
            }
            result
        }
    }
}

/// The direct exchange's request, before its DA1 sentinel: the colors, the
/// palette, the cell-size query, then the Kitty query with image ID `kitty`.
fn direct_request(colors: bool, palette: bool, cell: bool, kitty: Option<u32>) -> Vec<u8> {
    let mut request = Vec::new();
    if colors {
        for code in [10u8, 11] {
            request.extend_from_slice(format!("\x1b]{code};?\x07").as_bytes());
        }
    }
    if palette {
        // One query per slot: multi-pair OSC 4 requests have had
        // compatibility bugs in terminals that answer single pairs.
        for index in 0..16u8 {
            request.extend_from_slice(format!("\x1b]4;{index};?\x07").as_bytes());
        }
    }
    if cell {
        request.extend_from_slice(CELL_SIZE_REQUEST);
    }
    if let Some(id) = kitty {
        request.extend_from_slice(&kitty_query(id));
    }
    request
}

/// A probe that ended without an exchange: silence is a reply with nothing
/// in it (unknown, never a negative), anything else is unavailability.
fn probe_error(error: TtyQueryError) -> ProbeResult {
    match error {
        TtyQueryError::Silent => ProbeResult::Replied(ProbeReply {
            kitty: KittyReply::NoReply,
            da1: None,
        }),
        TtyQueryError::InputPending => ProbeResult::Skipped(ProbeSkip::BusyInput),
        TtyQueryError::Unanswered | TtyQueryError::Io(_) => {
            ProbeResult::Skipped(ProbeSkip::Unavailable(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::tty_query::{find_da1_reply, typeahead_ranges};

    fn position(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    #[test]
    fn only_native_windows_asks_for_the_cell_size() {
        assert_eq!(ASKS_CELL_SIZE, cfg!(windows));
    }

    #[test]
    fn the_cell_size_query_rides_in_the_request_ahead_of_the_da1_sentinel() {
        let request = direct_request(true, false, true, Some(7));
        let mut wire = request.clone();
        wire.extend_from_slice(DA1_REQUEST);

        let cell_at = position(&wire, CELL_SIZE_REQUEST).expect("CSI 16 t is in the request");
        let da1_at = position(&wire, DA1_REQUEST).expect("DA1 sentinel");
        assert!(cell_at < da1_at, "CSI 16 t must precede the sentinel: {wire:?}");
        assert_eq!(da1_at, request.len(), "the sentinel is written once, last");
        assert!(position(&request, b"\x1b]11;?\x07").is_some());
        assert!(position(&request, &kitty_query(7)).is_some());
    }

    #[test]
    fn the_cell_size_query_is_left_out_unless_asked() {
        let request = direct_request(true, false, false, Some(7));
        assert_eq!(position(&request, CELL_SIZE_REQUEST), None);
        assert_eq!(direct_request(false, false, true, None), CELL_SIZE_REQUEST);
    }

    #[test]
    fn the_palette_queries_ride_in_the_color_request_ahead_of_the_sentinel() {
        let request = direct_request(true, true, false, None);
        let osc11_at = position(&request, b"\x1b]11;?\x07").expect("OSC 11 is in the request");
        for index in 0..16u8 {
            let query = format!("\x1b]4;{index};?\x07");
            let at = position(&request, query.as_bytes())
                .unwrap_or_else(|| panic!("OSC 4 slot {index} is in the request"));
            assert!(at > osc11_at, "slot {index} follows the colors: {request:?}");
        }
        assert_eq!(position(&request, b"\x1b]4;16;"), None, "only slots 0-15");
        assert_eq!(position(&request, DA1_REQUEST), None, "the sentinel is appended by the exchange");
        let sequences: Vec<&[u8]> = request
            .split(|&b| b == 0x07)
            .filter(|sequence| !sequence.is_empty())
            .collect();
        assert_eq!(sequences.len(), 18, "OSC 10, OSC 11, and 16 OSC 4 queries");
        assert!(
            sequences.iter().all(|sequence| sequence.ends_with(b";?")),
            "only `?` queries: the palette is never set: {request:?}"
        );
    }

    #[test]
    fn the_palette_queries_are_left_out_unless_asked() {
        let request = direct_request(true, false, true, Some(7));
        assert_eq!(position(&request, b"\x1b]4;"), None);
    }

    #[test]
    fn colors_and_palette_are_read_from_one_reply() {
        let reply = b"\x1b]10;rgb:cccc/dddd/eeee\x07\x1b]11;rgb:1111/2222/3333\x07\
            \x1b]4;0;rgb:0000/0000/0000\x07\x1b]4;1;rgb:cdcd/3131/3131\x07\x1b[?62;22c";
        assert_eq!(find_osc_color(reply, 11), Some(RgbValue { r: 17, g: 34, b: 51 }));
        let palette = parse_osc4_palette(reply);
        assert_eq!(palette[0], Some(RgbValue::new(0, 0, 0)));
        assert_eq!(palette[1], Some(RgbValue::new(205, 49, 49)));
        assert!(palette[2..].iter().all(Option::is_none));
    }

    #[test]
    fn every_part_is_read_from_one_interleaved_reply() {
        // Replies in request order, as a terminal sends them, ended by DA1.
        let reply = b"\x1b]10;rgb:cccc/dddd/eeee\x1b\\\x1b]11;rgb:1111/2222/3333\x1b\\\
            \x1b[6;20;10t\x1b_Gi=7;OK\x1b\\\x1b[?61;4;6c";

        assert_eq!(
            find_cell_size_reply(reply),
            Some(CellSize {
                width: 10,
                height: 20
            })
        );
        assert_eq!(find_osc_color(reply, 11), Some(RgbValue { r: 17, g: 34, b: 51 }));
        assert!(find_osc_color(reply, 10).is_some());
        let probe = ProbeReply::parse(reply, 7);
        assert_eq!(probe.kitty, KittyReply::Accepted);
        assert!(probe.da1.is_some_and(|da1| da1.has_sixel()));
        assert!(find_da1_reply(reply).is_some());
    }

    #[test]
    fn a_cell_size_reply_is_never_returned_as_typeahead() {
        // Windows writes typeahead back to the console; a reply must not be.
        let bytes = b"a\x1b[6;20;10tb\x1b[?61c";
        let typed: Vec<u8> = typeahead_ranges(bytes)
            .iter()
            .flat_map(|range| bytes[range.clone()].to_vec())
            .collect();
        assert_eq!(typed, b"ab");
    }
}
