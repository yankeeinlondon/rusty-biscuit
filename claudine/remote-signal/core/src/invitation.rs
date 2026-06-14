//! Self-contained peer-invitation strings for manual pairing.
//!
//! An [`Invitation`] bundles a node's 32-byte public key with the
//! socket address (`ip:port`) it advertises for QUIC traffic into a
//! single bech32-encoded string. Operators copy that string between
//! daemons out-of-band (chat, e-mail, QR code, ...) and feed it to
//! `ConnectToPeer` on the receiving side.
//!
//! ## Wire Format
//!
//! The bech32m payload (HRP `rs`) carries the following little-endian
//! byte sequence:
//!
//! ```text
//! version    : u8       (must be 1)
//! pubkey     : [u8; 32]
//! addr_kind  : u8       (4 → IPv4, 6 → IPv6)
//! ip_bytes   : 4 or 16 bytes
//! port       : u16 (big-endian)
//! ```
//!
//! The version byte is reserved for future format changes; rejecting
//! unknown versions keeps later phases free to evolve the schema
//! without breaking older nodes silently.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;

use bech32::{Bech32m, DecodeError, Hrp};

use crate::identity::{NodeIdentity, PUBLIC_KEY_LENGTH};

/// Human-readable part of an invitation bech32 string.
pub const INVITATION_HRP: &str = "rs";

/// Wire-format version currently supported by [`Invitation`].
pub const INVITATION_VERSION: u8 = 1;

const ADDR_KIND_V4: u8 = 4;
const ADDR_KIND_V6: u8 = 6;

/// Decoded contents of an invitation bech32 string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitation {
    /// 32-byte compressed Ed25519 public key of the inviting node.
    pub public_key: [u8; PUBLIC_KEY_LENGTH],
    /// Socket address the inviting node advertises for QUIC traffic.
    pub socket_addr: SocketAddr,
}

impl Invitation {
    /// Build an invitation for the local node.
    #[must_use]
    pub fn new(identity: &NodeIdentity, socket_addr: SocketAddr) -> Self {
        Self {
            public_key: identity.public_key_bytes(),
            socket_addr,
        }
    }

    /// Encode the invitation as a bech32m string with HRP `rs`.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut payload = Vec::with_capacity(1 + PUBLIC_KEY_LENGTH + 1 + 16 + 2);
        payload.push(INVITATION_VERSION);
        payload.extend_from_slice(&self.public_key);
        match self.socket_addr.ip() {
            IpAddr::V4(ip) => {
                payload.push(ADDR_KIND_V4);
                payload.extend_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                payload.push(ADDR_KIND_V6);
                payload.extend_from_slice(&ip.octets());
            }
        }
        payload.extend_from_slice(&self.socket_addr.port().to_be_bytes());
        let hrp = Hrp::parse(INVITATION_HRP).expect("static HRP is valid");
        bech32::encode::<Bech32m>(hrp, &payload).expect("payload fits in bech32m capacity")
    }

    /// Hex-encoded `node_id` derived from the embedded public key.
    #[must_use]
    pub fn node_id(&self) -> String {
        crate::identity::encode_hex(&self.public_key)
    }
}

impl FromStr for Invitation {
    type Err = InvitationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (hrp, data) =
            bech32::decode(value).map_err(|source| InvitationError::Bech32 { source })?;
        if hrp.as_str() != INVITATION_HRP {
            return Err(InvitationError::WrongHrp {
                expected: INVITATION_HRP,
                actual: hrp.as_str().to_string(),
            });
        }

        let mut cursor = data.as_slice();
        let version = take_u8(&mut cursor)?;
        if version != INVITATION_VERSION {
            return Err(InvitationError::UnsupportedVersion(version));
        }

        let public_key = take_array::<PUBLIC_KEY_LENGTH>(&mut cursor)?;
        let kind = take_u8(&mut cursor)?;
        let ip = match kind {
            ADDR_KIND_V4 => {
                let octets = take_array::<4>(&mut cursor)?;
                IpAddr::V4(Ipv4Addr::from(octets))
            }
            ADDR_KIND_V6 => {
                let octets = take_array::<16>(&mut cursor)?;
                IpAddr::V6(Ipv6Addr::from(octets))
            }
            other => return Err(InvitationError::UnknownAddrKind(other)),
        };
        let port_bytes = take_array::<2>(&mut cursor)?;
        let port = u16::from_be_bytes(port_bytes);

        if !cursor.is_empty() {
            return Err(InvitationError::TrailingBytes(cursor.len()));
        }

        Ok(Self {
            public_key,
            socket_addr: SocketAddr::new(ip, port),
        })
    }
}

/// Errors that surface when encoding/decoding an [`Invitation`].
#[derive(Debug, thiserror::Error)]
pub enum InvitationError {
    /// The bech32 envelope itself was malformed (bad checksum,
    /// unsupported encoding, ...).
    #[error("invitation is not a valid bech32 string: {source}")]
    Bech32 {
        #[source]
        source: DecodeError,
    },

    /// The HRP did not match [`INVITATION_HRP`].
    #[error("invitation HRP mismatch: expected '{expected}', got '{actual}'")]
    WrongHrp {
        expected: &'static str,
        actual: String,
    },

    /// The version byte was not [`INVITATION_VERSION`].
    #[error("unsupported invitation version: {0}")]
    UnsupportedVersion(u8),

    /// The address-kind byte was neither `4` nor `6`.
    #[error("unknown invitation address kind: {0}")]
    UnknownAddrKind(u8),

    /// The decoded payload ran out of bytes before all expected fields
    /// were consumed.
    #[error("invitation payload truncated; missing {0} bytes")]
    Truncated(usize),

    /// Extra bytes were left over after parsing all expected fields.
    #[error("invitation payload has {0} trailing bytes")]
    TrailingBytes(usize),
}

fn take_u8(cursor: &mut &[u8]) -> Result<u8, InvitationError> {
    let Some((first, rest)) = cursor.split_first() else {
        return Err(InvitationError::Truncated(1));
    };
    *cursor = rest;
    Ok(*first)
}

fn take_array<const N: usize>(cursor: &mut &[u8]) -> Result<[u8; N], InvitationError> {
    if cursor.len() < N {
        return Err(InvitationError::Truncated(N - cursor.len()));
    }
    let (head, rest) = cursor.split_at(N);
    let mut out = [0u8; N];
    out.copy_from_slice(head);
    *cursor = rest;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn fixed_identity() -> NodeIdentity {
        NodeIdentity::from_seed([0x11; 32])
    }

    #[test]
    fn round_trip_ipv4_invitation() {
        let id = fixed_identity();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)), 4242);
        let invitation = Invitation::new(&id, addr);
        let encoded = invitation.encode();
        assert!(
            encoded.starts_with("rs1"),
            "expected HRP prefix, got {encoded}"
        );
        let decoded: Invitation = encoded.parse().expect("decode");
        assert_eq!(decoded, invitation);
    }

    #[test]
    fn round_trip_ipv6_invitation() {
        let id = fixed_identity();
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)), 7777);
        let invitation = Invitation::new(&id, addr);
        let decoded: Invitation = invitation.encode().parse().expect("decode");
        assert_eq!(decoded.socket_addr, addr);
        assert_eq!(decoded.public_key, id.public_key_bytes());
    }

    #[test]
    fn node_id_matches_identity() {
        let id = fixed_identity();
        let inv = Invitation::new(&id, "127.0.0.1:1".parse().unwrap());
        assert_eq!(inv.node_id(), id.node_id());
    }

    #[test]
    fn rejects_wrong_hrp() {
        // Re-encode a payload under a different HRP.
        let id = fixed_identity();
        let inv = Invitation::new(&id, "127.0.0.1:1".parse().unwrap());
        let encoded = inv.encode();
        let mutated = encoded.replacen("rs1", "rx1", 1);
        let err = Invitation::from_str(&mutated).expect_err("should fail");
        // The checksum will fail before the HRP check fires, but
        // either way the parser must refuse.
        assert!(matches!(
            err,
            InvitationError::Bech32 { .. } | InvitationError::WrongHrp { .. }
        ));
    }

    #[test]
    fn rejects_unsupported_version() {
        let id = fixed_identity();
        let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();
        let mut payload = Vec::new();
        payload.push(99);
        payload.extend_from_slice(&id.public_key_bytes());
        payload.push(ADDR_KIND_V4);
        match addr.ip() {
            IpAddr::V4(ip) => payload.extend_from_slice(&ip.octets()),
            IpAddr::V6(_) => unreachable!(),
        }
        payload.extend_from_slice(&addr.port().to_be_bytes());
        let hrp = Hrp::parse(INVITATION_HRP).unwrap();
        let encoded = bech32::encode::<Bech32m>(hrp, &payload).unwrap();
        let err = Invitation::from_str(&encoded).expect_err("should fail");
        assert!(matches!(err, InvitationError::UnsupportedVersion(99)));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let id = fixed_identity();
        let addr: SocketAddr = "10.0.0.1:1".parse().unwrap();
        let mut payload = Vec::new();
        payload.push(INVITATION_VERSION);
        payload.extend_from_slice(&id.public_key_bytes());
        payload.push(ADDR_KIND_V4);
        match addr.ip() {
            IpAddr::V4(ip) => payload.extend_from_slice(&ip.octets()),
            IpAddr::V6(_) => unreachable!(),
        }
        payload.extend_from_slice(&addr.port().to_be_bytes());
        payload.extend_from_slice(b"junk");
        let hrp = Hrp::parse(INVITATION_HRP).unwrap();
        let encoded = bech32::encode::<Bech32m>(hrp, &payload).unwrap();
        let err = Invitation::from_str(&encoded).expect_err("should fail");
        assert!(matches!(err, InvitationError::TrailingBytes(4)));
    }
}
