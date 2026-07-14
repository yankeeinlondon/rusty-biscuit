//! Shared protobuf schema, generated gRPC stubs, and IPC helpers for the
//! rendezvous daemon and its clients.
//!
//! Phase 1 wired the `Ping` / `Status` round-trip over a Unix Domain
//! Socket. Phase 2 adds the session-log document model
//! ([`session_log`]) plus the append/read RPC surface that exercises
//! the daemon's redb-backed Loro persistence and the DuckDB analytical
//! projection. Later phases will extend the `Rendezvous` service with
//! pairing and sync operations.

pub mod document;
pub mod envelope;
pub mod identity;
pub mod invitation;
pub mod repo_identity;
pub mod session_log;
pub mod session_status;
pub mod socket;
pub mod sync;

/// Generated protobuf types and gRPC stubs for the `rendezvous` package.
///
/// Re-exported from the build-script output so consumers can refer to a
/// stable module path (`rendezvous_core::proto`) instead of the raw
/// `tonic::include_proto!` macro call site.
pub mod proto {
    tonic::include_proto!("rendezvous");
}

pub use proto::rendezvous_client::RendezvousClient;
pub use proto::rendezvous_server::{Rendezvous, RendezvousServer};
pub use proto::{
    AppendEntryRequest, AppendEntryResponse, ApprovePeerRequest, ApprovePeerResponse,
    ConnectToPeerRequest, ConnectToPeerResponse, CreateInvitationRequest,
    CreateInvitationResponse, HostActiveSessions, HostCapability, HostRepos,
    ListActiveSessionsRequest, ListActiveSessionsResponse, ListChunkEntriesRequest,
    ListChunkEntriesResponse, ListHostCapabilitiesRequest, ListHostCapabilitiesResponse,
    ListHostReposRequest, ListHostReposResponse, ReportSessionEventRequest,
    ReportSessionEventResponse, SessionEventKind, SessionStatusBasis, SessionStatusState,
    StatusProducer,
    ListPairingsRequest, ListPairingsResponse, ListPeersRequest, ListPeersResponse,
    ListSessionChunksRequest, ListSessionChunksResponse, PairingInfo, PeerConnectionState,
    PeerInfo, PeerSource, PingRequest, PingResponse, ProjectionRow, QueryProjectionRequest,
    QueryProjectionResponse, RevokePeerRequest, RevokePeerResponse, SessionEntry,
    SignedEnvelopeWire, StatusRequest, StatusResponse, SyncAdvertiseEnd, SyncChunkAdvertise,
    SyncChunkOutcome, SyncDelta, SyncEnd, SyncError, SyncFrame, SyncHello, SyncWithPeerRequest,
    SyncWithPeerResponse, sync_frame,
};

pub use document::{
    CAPABILITY_DOMAIN, DocumentId, DocumentIdParseError, REPOS_DOMAIN, SESSION_DOMAIN,
    SESSIONS_ACTIVE_DOMAIN,
};
pub use repo_identity::canonical_repo_id;
pub use envelope::{
    ENVELOPE_HASH_LENGTH, EnvelopeError, EnvelopeInbox, EnvelopeSealer, PayloadKind,
    SignedEnvelope,
};
pub use identity::{
    NodeIdentity, NodeIdentityError, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, verify_signature,
};
pub use invitation::{INVITATION_HRP, INVITATION_VERSION, Invitation, InvitationError};

pub use session_log::{
    ChunkConfig, ChunkId, ChunkIdParseError, ChunkMetadata, DEFAULT_MAX_BYTES_PER_CHUNK,
    DEFAULT_MAX_ENTRIES_PER_CHUNK, Entry,
};

/// Semver string of the `rendezvous-core` crate, surfaced so the daemon
/// and clients can report a single shared version in protocol responses.
pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
