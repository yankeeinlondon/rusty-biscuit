//! The graphics probe's wire format: the Kitty query it sends, the parsers
//! for what comes back, and the tmux passthrough envelope.
//!
//! Everything here is pure. The live exchange that writes these bytes lives
//! in `discovery::startup_exchange`; when a probe is sent is decided by
//! [`probe_plan`](super::probe_plan). The developer guide is
//! `biscuit-terminal/docs/discovery/graphics-routes.md`.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};

/// The Kitty graphics query for image `id`: one RGB pixel sent as direct
/// data (`a=q,t=d,f=24,s=1,v=1`), whose three zero bytes are `AAAA` in
/// base64. A query is never displayed and leaves no image behind.
#[must_use]
pub fn kitty_query(id: u32) -> Vec<u8> {
    format!("\x1b_Gi={id},s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\").into_bytes()
}

/// The largest Kitty image ID this crate allocates: 24 bits, so a Unicode
/// placeholder cell can carry the ID in a truecolor foreground.
pub const MAX_KITTY_ID: u32 = 0xFF_FFFF;

/// A nonzero Kitty image ID, at most [`MAX_KITTY_ID`], not used recently in
/// this process by a probe or by an image upload. After
/// [`MAX_KITTY_ID`] IDs the sequence wraps back to 1.
///
/// The sequence starts at a per-process offset, so two programs sharing a
/// terminal are unlikely to correlate each other's replies or overwrite each
/// other's images.
#[must_use]
pub fn next_kitty_id() -> u32 {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let _ = NEXT.compare_exchange(
        0,
        (std::process::id().wrapping_mul(2_654_435_761) & MAX_KITTY_ID) | 1,
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
    loop {
        if let Some(id) = kitty_id_from_counter(NEXT.fetch_add(1, Ordering::Relaxed)) {
            return id;
        }
    }
}

/// The image ID for counter value `counter`, or `None` for the zero ID the
/// sequence skips.
fn kitty_id_from_counter(counter: u32) -> Option<u32> {
    let id = counter & MAX_KITTY_ID;
    (id != 0).then_some(id)
}

/// What a terminal said about one Kitty query.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KittyReply {
    /// A complete `OK` for the query's image ID.
    Accepted,
    /// A complete error reply for the query's ID (for example
    /// `ENOTSUPPORTED:…`); the terminal understood the protocol and refused.
    Rejected(String),
    /// More than one complete reply for the ID, and they disagree.
    Conflicting,
    /// No complete reply carried the ID.
    NoReply,
}

/// Finds the reply to the Kitty query for `id` in `bytes`.
///
/// Only complete `ESC _ G … ESC \` replies whose control data carries
/// `i=<id>` count; fragments, replies for other IDs, and anything between
/// replies (typed input, other terminal replies) are ignored.
#[must_use]
pub fn parse_kitty_reply(bytes: &[u8], id: u32) -> KittyReply {
    let mut found: Option<KittyReply> = None;
    let mut from = 0;
    while let Some(rel) = find(&bytes[from..], b"\x1b_G") {
        let start = from + rel + 3;
        let Some(len) = find(&bytes[start..], b"\x1b\\") else {
            break;
        };
        let body = &bytes[start..start + len];
        from = start + len + 2;

        let (control, message) = match body.iter().position(|&b| b == b';') {
            Some(semi) => (&body[..semi], &body[semi + 1..]),
            None => (body, &[][..]),
        };
        let carries_id = control
            .split(|&b| b == b',')
            .any(|pair| pair.strip_prefix(b"i=") == Some(id.to_string().as_bytes()));
        if !carries_id {
            continue;
        }
        let reply = if message == b"OK" {
            KittyReply::Accepted
        } else {
            KittyReply::Rejected(String::from_utf8_lossy(message).into_owned())
        };
        found = match found {
            None => Some(reply),
            Some(previous) if previous == reply => Some(previous),
            Some(_) => Some(KittyReply::Conflicting),
        };
    }
    found.unwrap_or(KittyReply::NoReply)
}

/// A primary device attributes (DA1) reply: `ESC [ ? class ; attributes c`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Da1Reply {
    /// The first parameter: the terminal's conformance class (for example 62
    /// or 64).
    pub class: u32,
    /// Every later parameter: the features the terminal reports.
    pub attributes: Vec<u32>,
}

impl Da1Reply {
    /// The first complete DA1 reply in `bytes`.
    #[must_use]
    pub fn find(bytes: &[u8]) -> Option<Self> {
        let mut from = 0;
        while let Some(rel) = find(&bytes[from..], b"\x1b[?") {
            let start = from + rel + 3;
            let len = bytes[start..]
                .iter()
                .take_while(|b| b.is_ascii_digit() || **b == b';')
                .count();
            if bytes.get(start + len) == Some(&b'c') {
                let params = &bytes[start..start + len];
                let mut numbers = params
                    .split(|&b| b == b';')
                    .map(|p| std::str::from_utf8(p).ok().and_then(|s| s.parse::<u32>().ok()));
                if let Some(Some(class)) = numbers.next() {
                    let attributes: Option<Vec<u32>> = numbers.collect();
                    if let Some(attributes) = attributes {
                        return Some(Self { class, attributes });
                    }
                }
            }
            from = start;
        }
        None
    }

    /// Whether the reply reports Sixel: attribute `4`, never the class and
    /// never a `4` inside another number.
    #[must_use]
    pub fn has_sixel(&self) -> bool {
        self.attributes.contains(&4)
    }
}

/// What one probe exchange produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbeReply {
    pub kitty: KittyReply,
    /// The DA1 reply that ended the exchange; `None` when the deadline came
    /// first.
    pub da1: Option<Da1Reply>,
}

impl ProbeReply {
    /// Parses an exchange's bytes for the query with image `id`.
    #[must_use]
    pub fn parse(bytes: &[u8], id: u32) -> Self {
        Self {
            kitty: parse_kitty_reply(bytes, id),
            da1: Da1Reply::find(bytes),
        }
    }
}

/// Why a probe was not sent or produced nothing to read.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProbeSkip {
    /// Input was already waiting, so the exchange would have consumed typed
    /// keys. Not a capability answer: a later construction may probe.
    BusyInput,
    /// The controlling terminal could not be opened or read.
    Unavailable(String),
}

/// The result of one probe exchange.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProbeResult {
    Replied(ProbeReply),
    Skipped(ProbeSkip),
}

/// The graphics probe results resolution reads. `None` means not asked.
///
/// `direct` is the query written as-is, answered by whatever the program
/// talks to first (the terminal, or a multiplexer). `tunneled` is the query
/// wrapped for tmux passthrough, answered only by the terminal outside a tmux
/// that has passthrough on. The two are never merged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphicsProbe {
    pub direct: Option<ProbeResult>,
    pub tunneled: Option<ProbeResult>,
}

/// Which probe exchanges would help resolution decide.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbePlan {
    pub direct: bool,
    pub tunneled: bool,
    /// The direct exchange sends the DA1 request without the Kitty query.
    /// A visible tmux answers DA1 itself (its own Sixel support) and would
    /// take an unwrapped Kitty query as a new pane title.
    pub direct_da1_only: bool,
}

impl ProbePlan {
    pub const NONE: Self = Self {
        direct: false,
        tunneled: false,
        direct_da1_only: false,
    };
    pub const DIRECT: Self = Self {
        direct: true,
        tunneled: false,
        direct_da1_only: false,
    };

    /// Whether any exchange is planned.
    #[must_use]
    pub fn any(&self) -> bool {
        self.direct || self.tunneled
    }
}

/// Wraps one complete escape sequence for tmux passthrough:
/// `ESC P tmux; <sequence with every ESC doubled> ESC \`.
#[must_use]
pub fn wrap_for_tmux(sequence: &[u8]) -> Vec<u8> {
    let mut wrapped = Vec::with_capacity(sequence.len() + 16);
    wrapped.extend_from_slice(b"\x1bPtmux;");
    for &byte in sequence {
        if byte == 0x1b {
            wrapped.push(0x1b);
        }
        wrapped.push(byte);
    }
    wrapped.extend_from_slice(b"\x1b\\");
    wrapped
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_is_a_one_pixel_direct_transmission() {
        assert_eq!(
            kitty_query(7),
            b"\x1b_Gi=7,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\".to_vec()
        );
    }

    #[test]
    fn probe_ids_are_fresh_and_nonzero() {
        let first = next_kitty_id();
        let second = next_kitty_id();
        assert_ne!(first, 0);
        assert_ne!(second, 0);
        assert_ne!(first, second);
        assert!(first <= MAX_KITTY_ID && second <= MAX_KITTY_ID);
    }

    #[test]
    fn ids_stay_in_24_bits_and_wrap_past_zero() {
        assert_eq!(kitty_id_from_counter(1), Some(1));
        assert_eq!(kitty_id_from_counter(MAX_KITTY_ID), Some(MAX_KITTY_ID));
        assert_eq!(kitty_id_from_counter(MAX_KITTY_ID + 1), None);
        assert_eq!(kitty_id_from_counter(MAX_KITTY_ID + 2), Some(1));
        assert_eq!(kitty_id_from_counter(u32::MAX), Some(MAX_KITTY_ID));
        assert_eq!(kitty_id_from_counter(0), None);
    }

    #[test]
    fn wrapping_doubles_every_escape_exactly_once() {
        assert_eq!(wrap_for_tmux(b"\x1b[c"), b"\x1bPtmux;\x1b\x1b[c\x1b\\".to_vec());
        assert_eq!(
            wrap_for_tmux(&kitty_query(3)),
            b"\x1bPtmux;\x1b\x1b_Gi=3,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\x1b\\\x1b\\".to_vec()
        );
    }
}
