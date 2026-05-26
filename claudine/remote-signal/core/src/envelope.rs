//! Authenticated envelope for network-bound payloads.
//!
//! Every byte that leaves the daemon over the mesh is wrapped in a
//! [`SignedEnvelope`]. The envelope binds together:
//!
//! - the **payload** (an opaque byte blob, e.g. an exported Loro
//!   delta or a control message),
//! - the sender's **public key** (so receivers can look up the
//!   verifying key without trusting transport metadata),
//! - a BLAKE3 **content hash** over the payload (cheap structural
//!   integrity check; the signature ultimately covers it too),
//! - a deterministic **message ID** derived from the content hash and
//!   the sender, so the [`EnvelopeInbox`] can deduplicate replays,
//! - an Ed25519 **signature** over the canonical `(sender || message_id
//!   || content_hash || payload)` byte string.
//!
//! Receivers run [`SignedEnvelope::verify`] before doing anything else
//! with the payload. The companion [`EnvelopeInbox`] adds in-process
//! replay protection: an envelope whose message ID has been seen
//! before is rejected even if it would otherwise verify.

use std::collections::HashSet;

use biscuit_hash::blake3_hash_bytes;

use crate::identity::{
    NodeIdentity, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, encode_hex, verify_signature,
};

/// Length, in bytes, of the BLAKE3 content hash and message ID.
pub const ENVELOPE_HASH_LENGTH: usize = 32;

/// Errors returned when an envelope fails verification or when the
/// inbox rejects a replay.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The recomputed BLAKE3 hash did not match the value carried in
    /// the envelope.
    #[error("envelope content hash mismatch")]
    ContentHashMismatch,

    /// The recomputed message ID did not match the value carried in
    /// the envelope.
    #[error("envelope message id mismatch")]
    MessageIdMismatch,

    /// The Ed25519 signature did not validate against the sender's
    /// public key.
    #[error("envelope signature is invalid")]
    InvalidSignature,

    /// The envelope's sender public key was not a valid Ed25519 point.
    #[error("envelope sender public key is malformed")]
    MalformedPublicKey,

    /// The inbox has already accepted an envelope with this message
    /// ID.
    #[error("duplicate message id {0}")]
    DuplicateMessageId(String),
}

/// Authenticated wrapper around an outgoing payload. All fields are
/// fixed-size byte arrays so the type round-trips cleanly through
/// protobuf, a custom binary wire, or any encoder that can carry
/// byte arrays without losing length information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedEnvelope {
    /// 32-byte compressed Ed25519 public key of the sender.
    pub sender: [u8; PUBLIC_KEY_LENGTH],
    /// Deterministic message identifier (32 bytes, BLAKE3 of
    /// `content_hash || sender`). Used by [`EnvelopeInbox`] for replay
    /// deduplication.
    pub message_id: [u8; ENVELOPE_HASH_LENGTH],
    /// BLAKE3 hash of `payload`.
    pub content_hash: [u8; ENVELOPE_HASH_LENGTH],
    /// 64-byte Ed25519 signature over the canonical
    /// `(sender || message_id || content_hash || payload)` byte
    /// string.
    pub signature: [u8; SIGNATURE_LENGTH],
    /// Opaque payload bytes.
    pub payload: Vec<u8>,
}

impl SignedEnvelope {
    /// Sign `payload` with `identity` and return a ready-to-send
    /// envelope.
    #[must_use]
    pub fn seal(identity: &NodeIdentity, payload: Vec<u8>) -> Self {
        let sender = identity.public_key_bytes();
        let content_hash = blake3_hash_bytes(&payload);
        let message_id = derive_message_id(&content_hash, &sender);
        let mut signing_input = signing_input(&sender, &message_id, &content_hash, &payload);
        let signature = identity.sign(&signing_input);
        signing_input.clear();
        Self {
            sender,
            message_id,
            content_hash,
            signature,
            payload,
        }
    }

    /// Hex-encoded message ID, useful for logging and tests.
    #[must_use]
    pub fn message_id_hex(&self) -> String {
        encode_hex(&self.message_id)
    }

    /// Hex-encoded sender public key (the wire-level `node_id`).
    #[must_use]
    pub fn sender_node_id(&self) -> String {
        encode_hex(&self.sender)
    }

    /// Re-derive the BLAKE3 hash, recompute the deterministic message
    /// ID, then verify the Ed25519 signature. Returns the verified
    /// payload on success.
    pub fn verify(&self) -> Result<&[u8], EnvelopeError> {
        let recomputed_hash = blake3_hash_bytes(&self.payload);
        if recomputed_hash != self.content_hash {
            return Err(EnvelopeError::ContentHashMismatch);
        }
        let recomputed_id = derive_message_id(&self.content_hash, &self.sender);
        if recomputed_id != self.message_id {
            return Err(EnvelopeError::MessageIdMismatch);
        }
        let signing_input = signing_input(
            &self.sender,
            &self.message_id,
            &self.content_hash,
            &self.payload,
        );
        if !verify_signature(&self.sender, &signing_input, &self.signature) {
            return Err(EnvelopeError::InvalidSignature);
        }
        Ok(&self.payload)
    }
}

/// In-process replay protection for [`SignedEnvelope`] traffic.
///
/// The inbox stores the set of message IDs it has accepted so far and
/// rejects any subsequent envelope whose ID has been seen before. Use
/// one inbox per logical inbound stream (e.g., per peer); two peers
/// must not share an inbox or legitimate duplicate-broadcast traffic
/// will be silently dropped.
#[derive(Debug, Default)]
pub struct EnvelopeInbox {
    seen: HashSet<[u8; ENVELOPE_HASH_LENGTH]>,
}

impl EnvelopeInbox {
    /// Build a new empty inbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify `envelope` and record its message ID so subsequent
    /// duplicates are rejected. Returns the verified payload bytes on
    /// success.
    pub fn accept<'a>(
        &mut self,
        envelope: &'a SignedEnvelope,
    ) -> Result<&'a [u8], EnvelopeError> {
        if self.seen.contains(&envelope.message_id) {
            return Err(EnvelopeError::DuplicateMessageId(envelope.message_id_hex()));
        }
        let payload = envelope.verify()?;
        self.seen.insert(envelope.message_id);
        Ok(payload)
    }

    /// Number of distinct envelopes the inbox has accepted so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Returns `true` when the inbox has accepted nothing yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

fn derive_message_id(
    content_hash: &[u8; ENVELOPE_HASH_LENGTH],
    sender: &[u8; PUBLIC_KEY_LENGTH],
) -> [u8; ENVELOPE_HASH_LENGTH] {
    let mut buf = Vec::with_capacity(ENVELOPE_HASH_LENGTH + PUBLIC_KEY_LENGTH);
    buf.extend_from_slice(content_hash);
    buf.extend_from_slice(sender);
    blake3_hash_bytes(&buf)
}

fn signing_input(
    sender: &[u8; PUBLIC_KEY_LENGTH],
    message_id: &[u8; ENVELOPE_HASH_LENGTH],
    content_hash: &[u8; ENVELOPE_HASH_LENGTH],
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        PUBLIC_KEY_LENGTH + ENVELOPE_HASH_LENGTH + ENVELOPE_HASH_LENGTH + payload.len(),
    );
    buf.extend_from_slice(sender);
    buf.extend_from_slice(message_id);
    buf.extend_from_slice(content_hash);
    buf.extend_from_slice(payload);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_identity(seed_byte: u8) -> NodeIdentity {
        let seed = [seed_byte; 32];
        NodeIdentity::from_seed(seed)
    }

    #[test]
    fn seal_and_verify_round_trip() {
        let identity = fixed_identity(7);
        let envelope = SignedEnvelope::seal(&identity, b"hello".to_vec());
        let payload = envelope.verify().expect("verify");
        assert_eq!(payload, b"hello");
        assert_eq!(envelope.sender, identity.public_key_bytes());
        assert_eq!(envelope.sender_node_id(), identity.node_id());
    }

    #[test]
    fn verify_rejects_tampered_payload() {
        let identity = fixed_identity(1);
        let mut envelope = SignedEnvelope::seal(&identity, b"original".to_vec());
        envelope.payload = b"tampered".to_vec();
        assert_eq!(envelope.verify(), Err(EnvelopeError::ContentHashMismatch));
    }

    #[test]
    fn verify_rejects_tampered_signature() {
        let identity = fixed_identity(2);
        let mut envelope = SignedEnvelope::seal(&identity, b"payload".to_vec());
        envelope.signature[0] ^= 0xFF;
        assert_eq!(envelope.verify(), Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn verify_rejects_spoofed_sender() {
        let alice = fixed_identity(3);
        let mallory = fixed_identity(4);
        let mut envelope = SignedEnvelope::seal(&alice, b"hi".to_vec());
        // Mallory pretends to be the sender. The recomputed message id
        // will not match, so the envelope is rejected.
        envelope.sender = mallory.public_key_bytes();
        assert_eq!(envelope.verify(), Err(EnvelopeError::MessageIdMismatch));
    }

    #[test]
    fn message_id_is_deterministic_per_sender_and_payload() {
        let identity = fixed_identity(5);
        let first = SignedEnvelope::seal(&identity, b"payload".to_vec());
        let second = SignedEnvelope::seal(&identity, b"payload".to_vec());
        assert_eq!(first.message_id, second.message_id);
    }

    #[test]
    fn different_payloads_have_different_message_ids() {
        let identity = fixed_identity(6);
        let a = SignedEnvelope::seal(&identity, b"a".to_vec());
        let b = SignedEnvelope::seal(&identity, b"b".to_vec());
        assert_ne!(a.message_id, b.message_id);
    }

    #[test]
    fn inbox_accepts_first_occurrence_and_rejects_replay() {
        let identity = fixed_identity(8);
        let envelope = SignedEnvelope::seal(&identity, b"once".to_vec());
        let mut inbox = EnvelopeInbox::new();
        let bytes = inbox.accept(&envelope).expect("accept first");
        assert_eq!(bytes, b"once");
        let err = inbox.accept(&envelope).expect_err("reject replay");
        assert!(matches!(err, EnvelopeError::DuplicateMessageId(_)));
        assert_eq!(inbox.len(), 1);
    }

    #[test]
    fn inbox_rejects_invalid_envelope_without_storing_id() {
        let identity = fixed_identity(9);
        let mut envelope = SignedEnvelope::seal(&identity, b"bad".to_vec());
        envelope.signature[0] ^= 0xFF;
        let mut inbox = EnvelopeInbox::new();
        let err = inbox.accept(&envelope).expect_err("invalid");
        assert_eq!(err, EnvelopeError::InvalidSignature);
        assert!(inbox.is_empty());
    }

    #[test]
    fn inbox_accepts_distinct_envelopes_from_same_sender() {
        let identity = fixed_identity(10);
        let first = SignedEnvelope::seal(&identity, b"alpha".to_vec());
        let second = SignedEnvelope::seal(&identity, b"beta".to_vec());
        let mut inbox = EnvelopeInbox::new();
        inbox.accept(&first).expect("alpha");
        inbox.accept(&second).expect("beta");
        assert_eq!(inbox.len(), 2);
    }
}
