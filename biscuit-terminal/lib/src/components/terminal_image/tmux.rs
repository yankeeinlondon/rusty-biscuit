//! Emission through tmux passthrough: one envelope per graphics control
//! sequence, and the size budget tmux puts on each envelope.

use crate::discovery::detection::probe::wrap_for_tmux;

/// The largest passthrough message tmux 3.3 and later accepts, counted over
/// the whole wrapped message (envelope and doubled ESCs included).
pub(crate) const TMUX_MESSAGE_BUDGET: usize = 1 << 20;

/// `output` with every graphics control wrapped for tmux passthrough.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TmuxWrapped {
    pub(crate) text: String,
    /// The size of the largest envelope in `text`; zero when nothing was
    /// wrapped.
    pub(crate) largest_message: usize,
}

/// Wraps each APC (Kitty chunk), OSC, and DCS sequence in `output` in its own
/// tmux envelope. Everything else passes through unwrapped: tmux handles
/// cursor movement, text, and synchronized output (mode 2026) itself. A
/// sequence that is already a tmux envelope is copied, never wrapped twice;
/// an unterminated sequence is copied as it is.
pub(crate) fn wrap_graphics_for_tmux(output: &str) -> TmuxWrapped {
    let bytes = output.as_bytes();
    let mut text = Vec::with_capacity(bytes.len() + bytes.len() / 64 + 16);
    let mut largest_message = 0;
    let mut at = 0;
    while at < bytes.len() {
        let Some(rel) = bytes[at..].iter().position(|&b| b == 0x1b) else {
            text.extend_from_slice(&bytes[at..]);
            break;
        };
        let start = at + rel;
        text.extend_from_slice(&bytes[at..start]);
        match control_end(bytes, start) {
            Some(Control::Envelope(end)) => {
                text.extend_from_slice(&bytes[start..end]);
                at = end;
            }
            Some(Control::Graphics(end)) => {
                let wrapped = wrap_for_tmux(&bytes[start..end]);
                largest_message = largest_message.max(wrapped.len());
                text.extend_from_slice(&wrapped);
                at = end;
            }
            None => {
                text.push(0x1b);
                at = start + 1;
            }
        }
    }
    TmuxWrapped {
        // Only ASCII bytes are inserted, and only at sequence boundaries.
        text: String::from_utf8(text).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
        largest_message,
    }
}

/// The size of `message` once wrapped: every ESC doubled, plus the
/// `ESC P tmux;` … `ESC \` envelope.
#[cfg(test)]
fn tmux_wrapped_len(message: &[u8]) -> usize {
    const ENVELOPE: usize = b"\x1bPtmux;".len() + b"\x1b\\".len();
    message.len() + message.iter().filter(|&&b| b == 0x1b).count() + ENVELOPE
}

enum Control {
    /// An existing tmux envelope ending (exclusive) at the index.
    Envelope(usize),
    /// A graphics control ending (exclusive) at the index.
    Graphics(usize),
}

/// Classifies the escape sequence at `start` and finds its end. `None` for
/// sequences that are not wrapped (CSI and the rest) and for unterminated
/// ones.
fn control_end(bytes: &[u8], start: usize) -> Option<Control> {
    let body = start + 2;
    match bytes.get(start + 1)? {
        b'P' if bytes[body..].starts_with(b"tmux;") => {
            // Inside an envelope every ESC is doubled, so the first ESC that
            // is not followed by another ESC starts the terminator.
            let mut at = body + 5;
            while at + 1 < bytes.len() {
                match (bytes[at], bytes[at + 1]) {
                    (0x1b, 0x1b) => at += 2,
                    (0x1b, b'\\') => return Some(Control::Envelope(at + 2)),
                    _ => at += 1,
                }
            }
            None
        }
        b'_' | b'P' => find_st(bytes, body).map(Control::Graphics),
        b']' => {
            let bel = bytes[body..].iter().position(|&b| b == 0x07).map(|i| body + i + 1);
            let st = find_st(bytes, body);
            match (bel, st) {
                (Some(bel), Some(st)) => Some(Control::Graphics(bel.min(st))),
                (end, None) | (None, end) => end.map(Control::Graphics),
            }
        }
        _ => None,
    }
}

/// The index just past the first `ESC \` at or after `from`.
fn find_st(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..]
        .windows(2)
        .position(|w| w == b"\x1b\\")
        .map(|i| from + i + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(output: &str) -> String {
        wrap_graphics_for_tmux(output).text
    }

    #[test]
    fn each_kitty_chunk_gets_its_own_envelope() {
        let chunks = "\x1b_Gq=2,i=7,a=T,m=1;AAAA\x1b\\\x1b_Gm=0;BBBB\x1b\\";
        assert_eq!(
            wrap(chunks),
            "\x1bPtmux;\x1b\x1b_Gq=2,i=7,a=T,m=1;AAAA\x1b\x1b\\\x1b\\\
             \x1bPtmux;\x1b\x1b_Gm=0;BBBB\x1b\x1b\\\x1b\\"
        );
    }

    #[test]
    fn cursor_movement_and_synchronized_output_are_not_wrapped() {
        let output = "\x1b[?2026h\x1b[s\x1b[3C\x1b_Gm=0;AA\x1b\\\x1b[u\x1b[2B\r\x1b[?2026l";
        assert_eq!(
            wrap(output),
            "\x1b[?2026h\x1b[s\x1b[3C\x1bPtmux;\x1b\x1b_Gm=0;AA\x1b\x1b\\\x1b\\\x1b[u\x1b[2B\r\x1b[?2026l"
        );
    }

    #[test]
    fn an_osc_ends_at_bel_or_st_whichever_comes_first() {
        assert_eq!(
            wrap("\x1b]1337;File=inline=1:AA\x07tail"),
            "\x1bPtmux;\x1b\x1b]1337;File=inline=1:AA\x07\x1b\\tail"
        );
        assert_eq!(
            wrap("\x1b]8;;x\x1b\\a\x07"),
            "\x1bPtmux;\x1b\x1b]8;;x\x1b\x1b\\\x1b\\a\x07"
        );
    }

    #[test]
    fn an_existing_envelope_is_never_wrapped_twice() {
        let once = wrap("\x1b_Gm=0;AA\x1b\\");
        assert_eq!(wrap(&once), once);
        assert_eq!(wrap_graphics_for_tmux(&once).largest_message, 0);
    }

    #[test]
    fn empty_plain_and_unterminated_input_pass_through() {
        assert_eq!(wrap(""), "");
        assert_eq!(wrap("plain ☃ text"), "plain ☃ text");
        assert_eq!(wrap("\x1b_Gm=0;AA"), "\x1b_Gm=0;AA");
        assert_eq!(wrap("\x1b"), "\x1b");
        assert_eq!(wrap_graphics_for_tmux("").largest_message, 0);
    }

    #[test]
    fn a_large_sequence_is_one_envelope_with_its_size_reported() {
        let payload = "A".repeat(3 * TMUX_MESSAGE_BUDGET / 2);
        let message = format!("\x1b]1337;File=inline=1:{payload}\x07");
        let wrapped = wrap_graphics_for_tmux(&message);
        assert_eq!(wrapped.text.matches("\x1bPtmux;").count(), 1);
        assert_eq!(wrapped.largest_message, wrapped.text.len());
        assert_eq!(wrapped.largest_message, tmux_wrapped_len(message.as_bytes()));
    }

    #[test]
    fn the_wrapped_length_counts_doubling_and_the_envelope() {
        assert_eq!(tmux_wrapped_len(b""), 9);
        assert_eq!(tmux_wrapped_len(b"\x1b[c"), wrap_for_tmux(b"\x1b[c").len());
        let st = b"\x1b_Gm=0;AA\x1b\\";
        assert_eq!(tmux_wrapped_len(st), wrap_for_tmux(st).len());
    }
}
