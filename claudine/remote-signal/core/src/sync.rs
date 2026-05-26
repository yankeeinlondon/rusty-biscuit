//! Wire helpers for the Phase-5 direct-sync protocol.
//!
//! Sync sessions run over QUIC bidirectional streams. Each frame is a
//! prost-encoded [`SyncFrame`] preceded by a big-endian `u32` length
//! prefix. The wire-level [`SignedEnvelopeWire`] mirrors the in-memory
//! [`SignedEnvelope`] so receivers can verify the Ed25519 signature on
//! the payload before applying it.

use prost::Message;

use crate::envelope::{ENVELOPE_HASH_LENGTH, EnvelopeError, SignedEnvelope};
use crate::identity::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
use crate::proto::{SignedEnvelopeWire, SyncFrame};

/// Hard cap on a single sync frame. Picked generously enough to carry a
/// full chunk snapshot (a few MiB at most) but small enough to keep an
/// adversarial peer from exhausting the daemon's memory.
pub const MAX_FRAME_LEN: u32 = 16 * 1024 * 1024;

/// Errors surfaced by the sync wire layer.
#[derive(Debug, thiserror::Error)]
pub enum SyncWireError {
    /// The peer closed the stream before sending a complete frame.
    #[error("sync stream closed unexpectedly while reading frame")]
    UnexpectedEof,

    /// The frame header indicated a payload larger than [`MAX_FRAME_LEN`].
    #[error("sync frame too large: {0} bytes (max {MAX_FRAME_LEN})")]
    FrameTooLarge(u32),

    /// The frame body did not decode as a valid protobuf message.
    #[error("sync frame decode error: {0}")]
    Decode(#[from] prost::DecodeError),

    /// I/O error reading from or writing to the underlying stream.
    #[error("sync stream I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An incoming envelope wire-message had malformed length fields.
    #[error("malformed signed envelope: {0}")]
    Envelope(String),
}

impl SyncWireError {
    /// Construct an envelope-shape error.
    pub fn envelope<S: Into<String>>(reason: S) -> Self {
        Self::Envelope(reason.into())
    }
}

impl SignedEnvelopeWire {
    /// Convert an in-memory [`SignedEnvelope`] into the wire form.
    #[must_use]
    pub fn from_envelope(envelope: &SignedEnvelope) -> Self {
        Self {
            sender: envelope.sender.to_vec(),
            message_id: envelope.message_id.to_vec(),
            content_hash: envelope.content_hash.to_vec(),
            signature: envelope.signature.to_vec(),
            payload: envelope.payload.clone(),
        }
    }

    /// Decode a wire envelope back into the strongly-typed
    /// [`SignedEnvelope`]. Verifies field lengths but does *not* verify
    /// the signature — callers should run [`SignedEnvelope::verify`]
    /// (or feed the envelope through an [`crate::EnvelopeInbox`])
    /// before trusting the payload.
    pub fn into_envelope(self) -> Result<SignedEnvelope, SyncWireError> {
        if self.sender.len() != PUBLIC_KEY_LENGTH {
            return Err(SyncWireError::envelope(format!(
                "sender length {} != {}",
                self.sender.len(),
                PUBLIC_KEY_LENGTH,
            )));
        }
        if self.message_id.len() != ENVELOPE_HASH_LENGTH {
            return Err(SyncWireError::envelope(format!(
                "message_id length {} != {}",
                self.message_id.len(),
                ENVELOPE_HASH_LENGTH,
            )));
        }
        if self.content_hash.len() != ENVELOPE_HASH_LENGTH {
            return Err(SyncWireError::envelope(format!(
                "content_hash length {} != {}",
                self.content_hash.len(),
                ENVELOPE_HASH_LENGTH,
            )));
        }
        if self.signature.len() != SIGNATURE_LENGTH {
            return Err(SyncWireError::envelope(format!(
                "signature length {} != {}",
                self.signature.len(),
                SIGNATURE_LENGTH,
            )));
        }
        let mut sender = [0u8; PUBLIC_KEY_LENGTH];
        sender.copy_from_slice(&self.sender);
        let mut message_id = [0u8; ENVELOPE_HASH_LENGTH];
        message_id.copy_from_slice(&self.message_id);
        let mut content_hash = [0u8; ENVELOPE_HASH_LENGTH];
        content_hash.copy_from_slice(&self.content_hash);
        let mut signature = [0u8; SIGNATURE_LENGTH];
        signature.copy_from_slice(&self.signature);
        Ok(SignedEnvelope {
            sender,
            message_id,
            content_hash,
            signature,
            payload: self.payload,
        })
    }
}

impl From<EnvelopeError> for SyncWireError {
    fn from(error: EnvelopeError) -> Self {
        SyncWireError::Envelope(error.to_string())
    }
}

/// Encode a [`SyncFrame`] into a length-prefixed wire packet.
#[must_use]
pub fn encode_frame(frame: &SyncFrame) -> Vec<u8> {
    let body = frame.encode_to_vec();
    let len = u32::try_from(body.len()).expect("frame fits in u32");
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&body);
    out
}

/// Decode a single length-prefixed [`SyncFrame`] from `bytes`. Returns
/// the frame plus the number of bytes consumed, or `None` if the buffer
/// does not yet contain a full frame.
pub fn try_decode_frame(bytes: &[u8]) -> Result<Option<(SyncFrame, usize)>, SyncWireError> {
    if bytes.len() < 4 {
        return Ok(None);
    }
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&bytes[..4]);
    let len = u32::from_be_bytes(len_bytes);
    if len > MAX_FRAME_LEN {
        return Err(SyncWireError::FrameTooLarge(len));
    }
    let total = 4 + len as usize;
    if bytes.len() < total {
        return Ok(None);
    }
    let frame = SyncFrame::decode(&bytes[4..total])?;
    Ok(Some((frame, total)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeIdentity;
    use crate::proto::{SyncEnd, sync_frame};

    #[test]
    fn signed_envelope_wire_round_trip_preserves_signature() {
        let identity = NodeIdentity::from_seed([13u8; 32]);
        let envelope = SignedEnvelope::seal(&identity, b"payload".to_vec());
        let wire = SignedEnvelopeWire::from_envelope(&envelope);
        let restored = wire.into_envelope().expect("decode");
        assert_eq!(restored, envelope);
        restored.verify().expect("verify");
    }

    #[test]
    fn signed_envelope_wire_rejects_bad_length() {
        let wire = SignedEnvelopeWire {
            sender: vec![0u8; 10],
            message_id: vec![0u8; ENVELOPE_HASH_LENGTH],
            content_hash: vec![0u8; ENVELOPE_HASH_LENGTH],
            signature: vec![0u8; SIGNATURE_LENGTH],
            payload: Vec::new(),
        };
        assert!(matches!(
            wire.into_envelope(),
            Err(SyncWireError::Envelope(_))
        ));
    }

    #[test]
    fn encode_then_decode_round_trips_a_frame() {
        let frame = SyncFrame {
            kind: Some(sync_frame::Kind::End(SyncEnd {})),
        };
        let bytes = encode_frame(&frame);
        let (decoded, consumed) = try_decode_frame(&bytes).expect("decode").expect("frame");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, frame);
    }

    #[test]
    fn try_decode_returns_none_when_buffer_is_short() {
        let frame = SyncFrame {
            kind: Some(sync_frame::Kind::End(SyncEnd {})),
        };
        let bytes = encode_frame(&frame);
        let partial = &bytes[..bytes.len() - 1];
        assert!(try_decode_frame(partial).expect("ok").is_none());
    }

    #[test]
    fn try_decode_rejects_oversize_header() {
        let mut bytes = (MAX_FRAME_LEN + 1).to_be_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(matches!(
            try_decode_frame(&bytes),
            Err(SyncWireError::FrameTooLarge(_))
        ));
    }
}
