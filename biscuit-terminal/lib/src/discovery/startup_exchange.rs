//! The once-per-process exchange `Terminal::new()` makes: the OSC 10/11
//! color queries and, when the environment cannot decide the graphics route,
//! the Kitty graphics query, written together and ended by one DA1 sentinel.
//! Inside a visible tmux the graphics part is the DA1 reply alone: tmux
//! answers it for itself, and the Kitty query is left out.
//!
//! Whichever caller needs the exchange first (the color lookup or the
//! graphics route) sends everything the process needs, so graphics probing
//! adds no extra round trip and no new moment at which input is read.
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
use crate::discovery::osc_queries::RgbValue;
use crate::discovery::osc_queries::query::{OSC_QUERY_ATTEMPT_TARGET, color_query_wanted, find_osc_color};
use crate::discovery::osc_queries::types::DEFAULT_TIMEOUT;
use crate::discovery::tty_query::{DA1_REQUEST, ExchangeOptions, TtyQueryError, exchange};

#[derive(Default)]
struct DirectState {
    colors: Option<[Option<RgbValue>; 2]>,
    graphics: Option<ProbeResult>,
}

static DIRECT: Mutex<DirectState> = Mutex::new(DirectState {
    colors: None,
    graphics: None,
});
static TUNNELED: Mutex<Option<ProbeResult>> = Mutex::new(None);

/// The live OSC 10 and 11 answers, `[foreground, background]`.
///
/// Callers have already applied the color-query gate. When the graphics
/// route also wants a direct probe, it rides along.
pub(crate) fn live_fg_bg() -> [Option<RgbValue>; 2] {
    let Ok(mut state) = DIRECT.lock() else {
        return [None, None];
    };
    if let Some(colors) = state.colors {
        return colors;
    }
    let plan = probe_plan(&RouteInputs::from_process());
    let graphics = (state.graphics.is_none() && plan.direct).then_some(!plan.direct_da1_only);
    run_direct(&mut state, true, graphics);
    state.colors.unwrap_or([None, None])
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
    run_direct(&mut state, colors, Some(kitty))
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
/// joins the DA1 request. Returns the graphics result when it was part of
/// the exchange.
fn run_direct(state: &mut DirectState, colors: bool, graphics: Option<bool>) -> Option<ProbeResult> {
    if !colors && graphics.is_none() {
        return None;
    }
    let id = (graphics == Some(true)).then(next_kitty_id);
    let mut request = Vec::new();
    if colors {
        for code in [10u8, 11] {
            tracing::debug!(
                target: OSC_QUERY_ATTEMPT_TARGET,
                code,
                "OSC{} actual query attempted",
                code
            );
            request.extend_from_slice(format!("\x1b]{code};?\x07").as_bytes());
        }
    }
    if let Some(id) = id {
        request.extend_from_slice(&kitty_query(id));
    }
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
            if colors {
                run_direct(state, true, None);
            }
            Some(ProbeResult::Skipped(ProbeSkip::BusyInput))
        }
        Err(error) => {
            tracing::debug!(%error, "startup exchange failed");
            if colors {
                state.colors = Some([None, None]);
            }
            let result = graphics.map(|_| probe_error(error));
            if let Some(result) = &result {
                state.graphics = Some(result.clone());
            }
            result
        }
    }
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
