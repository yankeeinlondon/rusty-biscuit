//! One-line stderr hints that explain why a graphics route has no pixels.
//!
//! The library never prints; it carries a [`RouteReason`] whose `fix` names
//! the change that would give graphics. The CLI turns that into at most one
//! stderr line per distinct reason per process, and only when stderr is a
//! terminal, so piped and `--json` output stay clean (hints never go to
//! stdout).

use std::collections::HashSet;
use std::io::{IsTerminal, Write};
use std::sync::{LazyLock, Mutex};

use biscuit_terminal::discovery::detection::{RouteReason, RouteReasonKind};
use biscuit_terminal::terminal::Terminal;

static EMITTED: LazyLock<Mutex<HashSet<RouteReason>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// The hint line for `reason`, or `None` when the reason carries no fix.
fn hint_line(reason: &RouteReason) -> Option<String> {
    let fix = reason.fix.as_deref()?;
    let label = match reason.kind {
        RouteReasonKind::InvalidInput => "configuration error",
        _ => "no graphics",
    };
    Some(format!("bt: {label}: {} ({fix})", reason.message))
}

/// Writes the hint for `reason` to `out` when it should print.
///
/// A hint prints only when `stderr_is_terminal`, the reason has a fix, and
/// `emitted` has not seen this reason before; a printed reason is recorded
/// in `emitted`. Returns whether a line was written.
fn emit_hint_to(
    reason: &RouteReason,
    stderr_is_terminal: bool,
    emitted: &mut HashSet<RouteReason>,
    out: &mut impl Write,
) -> bool {
    if !stderr_is_terminal {
        return false;
    }
    let Some(line) = hint_line(reason) else {
        return false;
    };
    if !emitted.insert(reason.clone()) {
        return false;
    }
    writeln!(out, "{line}").is_ok()
}

/// Prints the hint for `reason` to stderr, once per distinct reason per
/// process, when stderr is a terminal and the reason has a fix.
pub fn emit_route_hint(reason: &RouteReason) {
    let mut emitted = EMITTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    emit_hint_to(
        reason,
        std::io::stderr().is_terminal(),
        &mut emitted,
        &mut std::io::stderr().lock(),
    );
}

/// Prints the route hint for `terminal` when its graphics route has no
/// pixels.
pub fn emit_terminal_route_hint(terminal: &Terminal) {
    if !terminal.graphics_route.has_pixels() {
        emit_route_hint(&terminal.graphics_route.reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(kind: RouteReasonKind, message: &str, fix: Option<&str>) -> RouteReason {
        RouteReason {
            kind,
            message: message.to_string(),
            fix: fix.map(str::to_string),
        }
    }

    fn emit(reason: &RouteReason, tty: bool, emitted: &mut HashSet<RouteReason>) -> String {
        let mut out = Vec::new();
        emit_hint_to(reason, tty, emitted, &mut out);
        String::from_utf8(out).expect("hint is UTF-8")
    }

    #[test]
    fn hint_prints_for_reason_with_fix_on_terminal_stderr() {
        let blocked = reason(
            RouteReasonKind::BlockedTransport,
            "tmux drops graphics",
            Some("set allow-passthrough on"),
        );

        let out = emit(&blocked, true, &mut HashSet::new());

        assert_eq!(out, "bt: no graphics: tmux drops graphics (set allow-passthrough on)\n");
    }

    #[test]
    fn hint_is_silent_when_stderr_is_not_a_terminal() {
        let blocked = reason(RouteReasonKind::BlockedTransport, "blocked", Some("fix it"));
        let mut emitted = HashSet::new();

        let out = emit(&blocked, false, &mut emitted);

        assert!(out.is_empty());
        assert!(emitted.is_empty(), "a suppressed hint must not count as emitted");
    }

    #[test]
    fn hint_is_silent_for_reason_without_fix() {
        let unknown = reason(RouteReasonKind::Unknown, "nothing established graphics", None);

        let out = emit(&unknown, true, &mut HashSet::new());

        assert!(out.is_empty());
    }

    #[test]
    fn hint_prints_each_distinct_reason_once() {
        let first = reason(RouteReasonKind::BlockedTransport, "first", Some("fix one"));
        let second = reason(RouteReasonKind::NoSupport, "second", Some("fix two"));
        let mut emitted = HashSet::new();

        let outputs = [
            emit(&first, true, &mut emitted),
            emit(&first, true, &mut emitted),
            emit(&second, true, &mut emitted),
            emit(&second, true, &mut emitted),
        ];

        let printed: Vec<&String> = outputs.iter().filter(|o| !o.is_empty()).collect();
        assert_eq!(printed.len(), 2, "{outputs:?}");
        assert!(outputs[0].contains("first") && outputs[2].contains("second"));
    }

    #[test]
    fn hint_labels_invalid_input_as_configuration_error() {
        let invalid = reason(
            RouteReasonKind::InvalidInput,
            "BISCUIT_GRAPHICS=bogus is not recognized",
            Some("use one of auto, kitty, iterm2, sixel, none"),
        );

        let out = emit(&invalid, true, &mut HashSet::new());

        assert!(out.starts_with("bt: configuration error: "), "{out:?}");
        assert!(out.contains("(use one of auto, kitty, iterm2, sixel, none)"), "{out:?}");
    }

    /// The library checks for a terminal before it validates
    /// `BISCUIT_GRAPHICS`, so a piped `bt` cannot show this reason; resolve
    /// it here as a terminal would see it.
    #[test]
    fn hint_for_resolved_invalid_override_is_a_configuration_error() {
        use biscuit_terminal::discovery::detection::{
            GRAPHICS_ENV, RouteInputs, resolve_graphics_route,
        };
        let route =
            resolve_graphics_route(&RouteInputs::new(true).with_var(GRAPHICS_ENV, "bogus"));

        let out = emit(&route.reason, true, &mut HashSet::new());

        assert_eq!(route.reason.kind, RouteReasonKind::InvalidInput);
        assert!(out.starts_with("bt: configuration error: BISCUIT_GRAPHICS="), "{out:?}");
        assert!(out.contains("auto, kitty, iterm2, sixel, none"), "{out:?}");
    }
}
