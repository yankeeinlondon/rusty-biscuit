//! Library surface for the rendezvous daemon.
//!
//! The daemon's only binary entry point is its `main` binary, but the
//! gRPC service and persistence layers live in library modules so they
//! can be exercised directly from integration tests without having to
//! spawn a child process.

pub mod batcher;
pub mod capability;
pub mod discovery;
pub mod local_transport;
pub mod peers;
pub mod private_dir;
pub mod projection;
pub mod quic;
pub mod refresher;
pub mod register;
pub mod repos;
pub mod server;
pub mod service;
pub mod session_log;
pub mod storage;
pub mod sync;

pub use batcher::{BatcherConfig, BatcherHandle, BatcherWorker};
pub use discovery::{DiscoveredPeer, DiscoveryError, DiscoveryHandle};
pub use local_transport::{shared_boot_count, spawn_local_server};
pub use peers::{PeerRecord, PeerRegistry, PeerRegistryWorkers};
pub use private_dir::{PrivateDirError, default_data_dir, ensure_private_dir};
pub use projection::{Projection, ProjectionError, ProjectionRow};
pub use quic::{ALPN_PROTOCOL, InboundConnection, QuicEndpoint, QuicError};
pub use refresher::spawn_register_refresher;
pub use register::{RegisterError, RegisterStore, owner_peer_id};
pub use server::{DaemonConfig, ServerError, ServerHandle};
pub use service::RendezvousService;
pub use session_log::{AppendOutcome, ExportedUpdate, SessionLogError, SessionLogManager};
pub use storage::{PairingValue, Storage, StorageError};
pub use sync::{SYNC_PROTOCOL_VERSION, SyncChunkOutcome, SyncError, SyncOutcome, SyncService};
