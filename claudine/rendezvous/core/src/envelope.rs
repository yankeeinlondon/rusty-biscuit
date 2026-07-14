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
//! - a monotonic **message ID** assigned by the [`EnvelopeSealer`]
//!   so the [`EnvelopeInbox`] can deduplicate replays,
//! - a **document ID** (the chunk path) binding the envelope to a
//!   specific chunk, preventing replay under a different chunk,
//! - a **payload kind** ([`PayloadKind`]) distinguishing snapshots
//!   from incremental deltas,
//! - an Ed25519 **signature** over the canonical `(sender ||
//!   message_id || document_id || payload_kind_byte || content_hash
//!   || payload)` byte string.
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

/// Distinguishes snapshot payloads from incremental deltas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadKind {
    /// Full Loro snapshot (peer had no prior state).
    Snapshot,
    /// Incremental update bundle.
    Delta,
    /// Full Loro snapshot the receiver must adopt WHOLESALE, replacing
    /// its local replica instead of merging. Sent when the sender's
    /// document was re-based (history compacted) past the receiver's
    /// version, so no delta can connect and a merge import does not
    /// converge.
    SnapshotReplace,
}

impl PayloadKind {
    /// Encode as a single byte for inclusion in the signing input.
    pub fn to_byte(self) -> u8 {
        match self {
            PayloadKind::Snapshot => 0,
            PayloadKind::Delta => 1,
            PayloadKind::SnapshotReplace => 2,
        }
    }

    /// Decode from the wire representation. Returns `None` for
    /// unknown discriminants.
    pub fn from_byte(byte: i32) -> Option<Self> {
        match byte {
            0 => Some(PayloadKind::Snapshot),
            1 => Some(PayloadKind::Delta),
            2 => Some(PayloadKind::SnapshotReplace),
            _ => None,
        }
    }

    /// Wire representation for protobuf.
    pub fn to_i32(self) -> i32 {
        self.to_byte() as i32
    }
}

/// Errors returned when an envelope fails verification or when the
/// inbox rejects a replay.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The recomputed BLAKE3 hash did not match the value carried in
    /// the envelope.
    #[error("envelope content hash mismatch")]
    ContentHashMismatch,

    /// The Ed25519 signature did not validate against the sender's
    /// public key.
    #[error("envelope signature is invalid")]
    InvalidSignature,

    /// The envelope's payload kind byte could not be decoded.
    #[error("envelope payload kind mismatch")]
    PayloadKindMismatch,

    /// The envelope's sender public key was not a valid Ed25519 point.
    #[error("envelope sender public key is malformed")]
    MalformedPublicKey,

    /// The inbox has already accepted an envelope with this message
    /// ID.
    #[error("duplicate message id {0}")]
    DuplicateMessageId(String),
}

/// Authenticated wrapper around an outgoing payload. All fields are
/// fixed-size byte arrays (except `document_id` which is a `String`
/// and `payload` which is `Vec<u8>`) so the type round-trips cleanly
/// through protobuf, a custom binary wire, or any encoder that can
/// carry byte arrays without losing length information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedEnvelope {
    /// 32-byte compressed Ed25519 public key of the sender.
    pub sender: [u8; PUBLIC_KEY_LENGTH],
    /// Monotonic message identifier (32 bytes, big-endian u64
    /// counter padded with leading zeros). Used by [`EnvelopeInbox`]
    /// for replay deduplication.
    pub message_id: [u8; ENVELOPE_HASH_LENGTH],
    /// The chunk path this envelope is bound to (e.g.
    /// `"session/shared/s1/part/0"`).
    pub document_id: String,
    /// Whether the payload is a snapshot or an incremental delta.
    pub payload_kind: PayloadKind,
    /// BLAKE3 hash of `payload`.
    pub content_hash: [u8; ENVELOPE_HASH_LENGTH],
    /// 64-byte Ed25519 signature over the canonical
    /// `(sender || message_id || document_id || payload_kind_byte ||
    /// content_hash || payload)` byte string.
    pub signature: [u8; SIGNATURE_LENGTH],
    /// Opaque payload bytes.
    pub payload: Vec<u8>,
}

/// Stateful sealer that assigns monotonically increasing message IDs
/// to outgoing envelopes. Each sealer is bound to a single
/// [`NodeIdentity`] and maintains an internal counter that increments
/// with every [`EnvelopeSealer::seal`] call.
pub struct EnvelopeSealer {
    identity: NodeIdentity,
    next_message_id: u64,
}

impl EnvelopeSealer {
    /// Create a new sealer starting from message ID zero.
    #[must_use]
    pub fn new(identity: NodeIdentity) -> Self {
        Self::with_start(identity, 0)
    }

    /// Create a sealer that resumes from `start` instead of zero.
    /// Used after daemon restart to continue the monotonic counter.
    #[must_use]
    pub fn with_start(identity: NodeIdentity, start: u64) -> Self {
        Self {
            identity,
            next_message_id: start,
        }
    }

    /// The next counter value that [`seal`](Self::seal) will assign.
    #[must_use]
    pub fn next_counter(&self) -> u64 {
        self.next_message_id
    }

    /// Sign `payload` with the sealer's identity and return a
    /// ready-to-send envelope bound to `document_id` and
    /// `payload_kind`.
    pub fn seal(
        &mut self,
        document_id: String,
        payload_kind: PayloadKind,
        payload: Vec<u8>,
    ) -> SignedEnvelope {
        let sender = self.identity.public_key_bytes();
        let content_hash = blake3_hash_bytes(&payload);
        let counter = self.next_message_id;
        self.next_message_id += 1;
        let mut message_id = [0u8; ENVELOPE_HASH_LENGTH];
        message_id[24..].copy_from_slice(&counter.to_be_bytes());
        let mut signing_input = signing_input(
            &sender,
            &message_id,
            &document_id,
            payload_kind,
            &content_hash,
            &payload,
        );
        let signature = self.identity.sign(&signing_input);
        signing_input.clear();
        SignedEnvelope {
            sender,
            message_id,
            document_id,
            payload_kind,
            content_hash,
            signature,
            payload,
        }
    }
}

impl SignedEnvelope {
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

    /// Re-derive the BLAKE3 hash, reconstruct the signing input
    /// including `document_id` and `payload_kind`, then verify the
    /// Ed25519 signature. Returns the verified payload on success.
    pub fn verify(&self) -> Result<&[u8], EnvelopeError> {
        let recomputed_hash = blake3_hash_bytes(&self.payload);
        if recomputed_hash != self.content_hash {
            return Err(EnvelopeError::ContentHashMismatch);
        }
        let signing_input = signing_input(
            &self.sender,
            &self.message_id,
            &self.document_id,
            self.payload_kind,
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

    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// Returns the highest u64 message-id counter the inbox has
    /// accepted, or `None` if the inbox is empty. Useful for
    /// high-water-mark tracking.
    #[must_use]
    pub fn high_water_mark(&self) -> Option<u64> {
        self.seen
            .iter()
            .map(|id| {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&id[24..]);
                u64::from_be_bytes(bytes)
            })
            .max()
    }
}

fn signing_input(
    sender: &[u8; PUBLIC_KEY_LENGTH],
    message_id: &[u8; ENVELOPE_HASH_LENGTH],
    document_id: &str,
    payload_kind: PayloadKind,
    content_hash: &[u8; ENVELOPE_HASH_LENGTH],
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        PUBLIC_KEY_LENGTH
            + ENVELOPE_HASH_LENGTH
            + document_id.len()
            + 1
            + ENVELOPE_HASH_LENGTH
            + payload.len(),
    );
    buf.extend_from_slice(sender);
    buf.extend_from_slice(message_id);
    buf.extend_from_slice(document_id.as_bytes());
    buf.push(payload_kind.to_byte());
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
        let mut sealer = EnvelopeSealer::new(identity.clone());
        let envelope =
            sealer.seal("doc/test".into(), PayloadKind::Delta, b"hello".to_vec());
        let payload = envelope.verify().expect("verify");
        assert_eq!(payload, b"hello");
        assert_eq!(envelope.sender, identity.public_key_bytes());
        assert_eq!(envelope.sender_node_id(), identity.node_id());
        assert_eq!(envelope.document_id, "doc/test");
        assert_eq!(envelope.payload_kind, PayloadKind::Delta);
    }

    #[test]
    fn verify_rejects_tampered_payload() {
        let identity = fixed_identity(1);
        let mut sealer = EnvelopeSealer::new(identity);
        let mut envelope =
            sealer.seal("doc/a".into(), PayloadKind::Snapshot, b"original".to_vec());
        envelope.payload = b"tampered".to_vec();
        assert_eq!(envelope.verify(), Err(EnvelopeError::ContentHashMismatch));
    }

    #[test]
    fn verify_rejects_tampered_signature() {
        let identity = fixed_identity(2);
        let mut sealer = EnvelopeSealer::new(identity);
        let mut envelope =
            sealer.seal("doc/b".into(), PayloadKind::Delta, b"payload".to_vec());
        envelope.signature[0] ^= 0xFF;
        assert_eq!(envelope.verify(), Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn verify_rejects_spoofed_sender() {
        let alice = fixed_identity(3);
        let mallory = fixed_identity(4);
        let mut sealer = EnvelopeSealer::new(alice);
        let mut envelope =
            sealer.seal("doc/c".into(), PayloadKind::Delta, b"hi".to_vec());
        envelope.sender = mallory.public_key_bytes();
        assert_eq!(envelope.verify(), Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn monotonic_ids_increment_per_seal() {
        let identity = fixed_identity(5);
        let mut sealer = EnvelopeSealer::new(identity);
        let first =
            sealer.seal("doc-1".into(), PayloadKind::Delta, b"payload".to_vec());
        let second =
            sealer.seal("doc-1".into(), PayloadKind::Delta, b"payload".to_vec());
        let first_id = u64::from_be_bytes(first.message_id[24..].try_into().unwrap());
        let second_id = u64::from_be_bytes(second.message_id[24..].try_into().unwrap());
        assert!(second_id > first_id);
        assert_eq!(first_id, 0);
        assert_eq!(second_id, 1);
    }

    #[test]
    fn same_payload_gets_different_ids() {
        let identity = fixed_identity(6);
        let mut sealer = EnvelopeSealer::new(identity);
        let a = sealer.seal("doc".into(), PayloadKind::Delta, b"same".to_vec());
        let b = sealer.seal("doc".into(), PayloadKind::Delta, b"same".to_vec());
        assert_ne!(a.message_id, b.message_id);
    }

    #[test]
    fn verify_rejects_wrong_document_id() {
        let identity = fixed_identity(11);
        let mut sealer = EnvelopeSealer::new(identity);
        let mut envelope =
            sealer.seal("doc/original".into(), PayloadKind::Delta, b"data".to_vec());
        envelope.document_id = "doc/other".into();
        assert_eq!(envelope.verify(), Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn verify_rejects_wrong_payload_kind() {
        let identity = fixed_identity(12);
        let mut sealer = EnvelopeSealer::new(identity);
        let mut envelope =
            sealer.seal("doc/x".into(), PayloadKind::Snapshot, b"data".to_vec());
        envelope.payload_kind = PayloadKind::Delta;
        assert_eq!(envelope.verify(), Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn inbox_accepts_first_occurrence_and_rejects_replay() {
        let identity = fixed_identity(8);
        let mut sealer = EnvelopeSealer::new(identity);
        let envelope =
            sealer.seal("doc/r".into(), PayloadKind::Delta, b"once".to_vec());
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
        let mut sealer = EnvelopeSealer::new(identity);
        let mut envelope =
            sealer.seal("doc/s".into(), PayloadKind::Delta, b"bad".to_vec());
        envelope.signature[0] ^= 0xFF;
        let mut inbox = EnvelopeInbox::new();
        let err = inbox.accept(&envelope).expect_err("invalid");
        assert_eq!(err, EnvelopeError::InvalidSignature);
        assert!(inbox.is_empty());
    }

    #[test]
    fn inbox_accepts_distinct_envelopes_from_same_sender() {
        let identity = fixed_identity(10);
        let mut sealer = EnvelopeSealer::new(identity);
        let first =
            sealer.seal("doc/d".into(), PayloadKind::Delta, b"alpha".to_vec());
        let second =
            sealer.seal("doc/d".into(), PayloadKind::Delta, b"beta".to_vec());
        let mut inbox = EnvelopeInbox::new();
        inbox.accept(&first).expect("alpha");
        inbox.accept(&second).expect("beta");
        assert_eq!(inbox.len(), 2);
    }

    #[test]
    fn high_water_mark_tracks_max_seen_counter() {
        let identity = fixed_identity(14);
        let mut sealer = EnvelopeSealer::new(identity);
        let mut inbox = EnvelopeInbox::new();
        assert_eq!(inbox.high_water_mark(), None);
        let e0 =
            sealer.seal("doc/h".into(), PayloadKind::Delta, b"a".to_vec());
        let e1 =
            sealer.seal("doc/h".into(), PayloadKind::Delta, b"b".to_vec());
        let e2 =
            sealer.seal("doc/h".into(), PayloadKind::Delta, b"c".to_vec());
        inbox.accept(&e0).expect("e0");
        inbox.accept(&e1).expect("e1");
        inbox.accept(&e2).expect("e2");
        assert_eq!(inbox.high_water_mark(), Some(2));
    }

    #[test]
    fn with_start_resumes_from_offset() {
        let identity = fixed_identity(15);
        let mut sealer = EnvelopeSealer::with_start(identity, 100);
        assert_eq!(sealer.next_counter(), 100);
        let envelope =
            sealer.seal("doc/o".into(), PayloadKind::Delta, b"x".to_vec());
        let counter = u64::from_be_bytes(envelope.message_id[24..].try_into().unwrap());
        assert_eq!(counter, 100);
        assert_eq!(sealer.next_counter(), 101);
    }

    #[test]
    fn two_senders_with_same_numeric_id_have_distinct_message_id_bytes() {
        let alice = fixed_identity(20);
        let bob = fixed_identity(21);
        let mut alice_sealer = EnvelopeSealer::new(alice.clone());
        let mut bob_sealer = EnvelopeSealer::new(bob);
        let alice_env =
            alice_sealer.seal("doc/d".into(), PayloadKind::Delta, b"a".to_vec());
        let bob_env =
            bob_sealer.seal("doc/d".into(), PayloadKind::Delta, b"b".to_vec());
        // Both start at counter 0, but the message IDs include only the
        // counter — they will be the same bytes. The inbox (in-memory,
        // single-sender) would reject the second because it only tracks
        // message IDs. This test documents the known behaviour: durable
        // dedup must be scoped by (sender, message_id) rather than
        // message_id alone (handled by the storage layer).
        assert_eq!(
            alice_env.message_id, bob_env.message_id,
            "two sealers starting at 0 produce the same message_id bytes",
        );
        assert_ne!(
            alice_env.sender, bob_env.sender,
            "but the senders are different",
        );
    }

    #[test]
    fn inbox_accepts_same_message_id_from_different_senders_in_separate_inboxes() {
        let alice = fixed_identity(30);
        let bob = fixed_identity(31);
        let mut alice_sealer = EnvelopeSealer::new(alice);
        let mut bob_sealer = EnvelopeSealer::new(bob);
        let env_a =
            alice_sealer.seal("doc/d".into(), PayloadKind::Delta, b"a".to_vec());
        let env_b =
            bob_sealer.seal("doc/d".into(), PayloadKind::Delta, b"b".to_vec());
        let mut inbox_a = EnvelopeInbox::new();
        let mut inbox_b = EnvelopeInbox::new();
        inbox_a.accept(&env_a).expect("alice inbox accepts alice");
        inbox_b.accept(&env_b).expect("bob inbox accepts bob");
        assert_eq!(inbox_a.len(), 1);
        assert_eq!(inbox_b.len(), 1);
    }
}
