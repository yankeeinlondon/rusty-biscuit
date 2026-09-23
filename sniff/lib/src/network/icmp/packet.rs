//! ICMP and ICMPv6 echo message encoding for datagram ICMP sockets.

use std::hash::{BuildHasher, RandomState};
use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes of per-request random payload used to recognize our own reply.
pub(super) const TOKEN_LEN: usize = 16;
const HEADER_LEN: usize = 8;

const ECHO_REQUEST_V4: u8 = 8;
const ECHO_REPLY_V4: u8 = 0;
const ECHO_REQUEST_V6: u8 = 128;
const ECHO_REPLY_V6: u8 = 129;

/// Largest datagram a reply read needs: an IPv4 header with options, the echo
/// header, and the payload.
#[cfg_attr(not(unix), allow(dead_code))]
pub(super) const MAX_REPLY_LEN: usize = 60 + HEADER_LEN + TOKEN_LEN;

/// A fresh unpredictable payload.
pub(super) fn token() -> [u8; TOKEN_LEN] {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let state = RandomState::new();
    let mut token = [0_u8; TOKEN_LEN];
    token[..8].copy_from_slice(&state.hash_one((counter, 0_u8)).to_le_bytes());
    token[8..].copy_from_slice(&state.hash_one((counter, 1_u8)).to_le_bytes());
    token
}

/// An echo request. The IPv4 checksum is filled in; the kernel computes the
/// ICMPv6 checksum, which covers a pseudo-header only it knows.
pub(super) fn echo_request(
    ipv6: bool,
    identifier: u16,
    sequence: u16,
    token: &[u8; TOKEN_LEN],
) -> Vec<u8> {
    let mut packet = vec![0_u8; HEADER_LEN + TOKEN_LEN];
    packet[0] = if ipv6 { ECHO_REQUEST_V6 } else { ECHO_REQUEST_V4 };
    packet[4..6].copy_from_slice(&identifier.to_be_bytes());
    packet[6..8].copy_from_slice(&sequence.to_be_bytes());
    packet[HEADER_LEN..].copy_from_slice(token);
    if !ipv6 {
        let sum = checksum(&packet);
        packet[2..4].copy_from_slice(&sum.to_be_bytes());
    }
    packet
}

/// True when `datagram` is the echo reply to the request with `sequence` and
/// `token`.
///
/// The identifier is not compared: Linux rewrites it to the socket's port. An
/// IPv4 datagram may arrive with its IP header (macOS) or without (Linux).
pub(super) fn is_echo_reply(
    ipv6: bool,
    datagram: &[u8],
    sequence: u16,
    token: &[u8; TOKEN_LEN],
) -> bool {
    let mut message = datagram;
    if !ipv6 && let Some(first) = message.first()
        && first >> 4 == 4
    {
        let header_len = usize::from(first & 0x0f) * 4;
        match message.get(header_len..) {
            Some(rest) => message = rest,
            None => return false,
        }
    }
    let expected_type = if ipv6 { ECHO_REPLY_V6 } else { ECHO_REPLY_V4 };
    message.len() >= HEADER_LEN + TOKEN_LEN
        && message[0] == expected_type
        && message[1] == 0
        && message[6..8] == sequence.to_be_bytes()
        && message[HEADER_LEN..HEADER_LEN + TOKEN_LEN] == token[..]
}

/// RFC 1071 Internet checksum.
fn checksum(bytes: &[u8]) -> u16 {
    let mut sum: u32 = bytes
        .chunks(2)
        .map(|pair| u32::from(u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)])))
        .sum();
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply_from(request: &[u8], ipv6: bool) -> Vec<u8> {
        let mut reply = request.to_vec();
        reply[0] = if ipv6 { ECHO_REPLY_V6 } else { ECHO_REPLY_V4 };
        reply[4..6].copy_from_slice(&0xbeef_u16.to_be_bytes());
        reply
    }

    #[test]
    fn ipv4_request_has_a_valid_checksum() {
        let request = echo_request(false, 0x1234, 7, &[0xab; TOKEN_LEN]);
        assert_eq!(request.len(), HEADER_LEN + TOKEN_LEN);
        assert_eq!(request[0], ECHO_REQUEST_V4);
        // A correct checksum makes the whole message sum to zero.
        assert_eq!(checksum(&request), 0);
    }

    #[test]
    fn checksum_matches_the_rfc_1071_example() {
        // RFC 1071 section 3: 0001 f203 f4f5 f6f7 sums to ddf2, complement 220d.
        assert_eq!(checksum(&[0x00, 0x01, 0xf2, 0x03, 0xf4, 0xf5, 0xf6, 0xf7]), 0x220d);
        assert_eq!(checksum(&[0xff]), !0xff00);
    }

    #[test]
    fn ipv6_request_leaves_the_checksum_to_the_kernel() {
        let request = echo_request(true, 1, 2, &[1; TOKEN_LEN]);
        assert_eq!(request[0], ECHO_REQUEST_V6);
        assert_eq!(&request[2..4], &[0, 0]);
    }

    #[test]
    fn matches_a_reply_with_or_without_an_ipv4_header_and_any_identifier() {
        let token = token();
        let request = echo_request(false, 1, 42, &token);
        let reply = reply_from(&request, false);
        assert!(is_echo_reply(false, &reply, 42, &token));

        let mut with_header = vec![0x45_u8; 1];
        with_header.extend([0_u8; 19]);
        with_header.extend(&reply);
        assert!(is_echo_reply(false, &with_header, 42, &token));

        let v6 = reply_from(&echo_request(true, 1, 42, &token), true);
        assert!(is_echo_reply(true, &v6, 42, &token));
    }

    #[test]
    fn rejects_other_sequences_tokens_types_and_short_datagrams() {
        let token = [9_u8; TOKEN_LEN];
        let reply = reply_from(&echo_request(false, 1, 42, &token), false);

        assert!(!is_echo_reply(false, &reply, 43, &token));
        assert!(!is_echo_reply(false, &reply, 42, &[8; TOKEN_LEN]));
        assert!(!is_echo_reply(true, &reply, 42, &token));
        assert!(!is_echo_reply(false, &echo_request(false, 1, 42, &token), 42, &token));
        for cut in 0..reply.len() {
            assert!(!is_echo_reply(false, &reply[..cut], 42, &token));
        }
        // An IPv4 header length pointing past the datagram.
        assert!(!is_echo_reply(false, &[0x4f, 0, 0, 0], 42, &token));
    }

    #[test]
    fn tokens_are_unique_per_request() {
        let tokens: std::collections::HashSet<_> = (0..64).map(|_| token()).collect();
        assert_eq!(tokens.len(), 64);
    }
}
