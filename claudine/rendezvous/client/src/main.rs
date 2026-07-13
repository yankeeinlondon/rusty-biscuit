//! `rendezvous-test-client` binary.
//!
//! A minimal command-line client that opens a gRPC channel over the
//! daemon's Unix Domain Socket and exercises either the Phase 1
//! liveness RPCs (`ping` / `status`) or the Phase 2 session-log RPCs
//! (`append-entry`, `list-chunks`, `list-entries`, `query-projection`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rendezvous_client::connect;
use rendezvous_core::socket::default_socket_path;
use rendezvous_core::{
    AppendEntryRequest, ApprovePeerRequest, ConnectToPeerRequest, CreateInvitationRequest,
    ListChunkEntriesRequest, ListPairingsRequest, ListPeersRequest, ListSessionChunksRequest,
    PingRequest, QueryProjectionRequest, RevokePeerRequest, StatusRequest, SyncWithPeerRequest,
};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "rendezvous-test-client",
    about = "Phase 1/2 test client for rendezvous-daemon"
)]
struct Cli {
    /// Override the Unix Domain Socket path. Defaults to the value
    /// resolved by `rendezvous_core::socket::default_socket_path`
    /// (also honouring `$RENDEZVOUS_SOCKET`).
    #[arg(long = "socket", env = "RENDEZVOUS_SOCKET", global = true)]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Round-trip a nonce against the daemon.
    Ping {
        /// Nonce echoed back by the daemon.
        #[arg(long, default_value = "ping")]
        nonce: String,
    },
    /// Request the daemon's uptime and version snapshot.
    Status,
    /// Append a single entry to a session log.
    AppendEntry {
        #[arg(long, default_value = "local-node")]
        owner_node_id: String,
        #[arg(long)]
        session_id: String,
        #[arg(long, default_value = "test-client")]
        source: String,
        #[arg(long, default_value = "info")]
        level: String,
        #[arg(long)]
        message: String,
        /// JSON object literal attached to the entry. Empty to omit.
        #[arg(long, default_value = "")]
        metadata_json: String,
    },
    /// List chunk IDs known for a session.
    ListChunks {
        #[arg(long, default_value = "local-node")]
        owner_node_id: String,
        #[arg(long)]
        session_id: String,
    },
    /// List entries currently materialised in a chunk's Loro doc.
    ListEntries {
        /// Full chunk path (`session/{node}/{session}/part/{index}`).
        #[arg(long)]
        chunk_id: String,
    },
    /// Query the DuckDB analytical projection for a session.
    QueryProjection {
        #[arg(long, default_value = "local-node")]
        owner_node_id: String,
        #[arg(long)]
        session_id: String,
    },
    /// Encode the daemon's connection info as a bech32 invitation
    /// string suitable for sharing with another node.
    CreateInvitation {
        /// Optional override of the advertised socket address. Empty
        /// uses the daemon's bound QUIC address (substituting
        /// `127.0.0.1` for the wildcard).
        #[arg(long, default_value = "")]
        advertise_addr: String,
    },
    /// Decode an invitation string and initiate a QUIC handshake with
    /// the peer it describes.
    ConnectToPeer {
        /// Bech32 invitation produced by `create-invitation`.
        #[arg(long)]
        invitation: String,
    },
    /// List all peers the daemon currently knows about.
    ListPeers,
    /// Approve a peer for direct sync. The pairing is persisted in
    /// redb so it survives daemon restarts.
    ApprovePeer {
        /// 64-character lowercase hex `node_id` of the peer to
        /// approve.
        #[arg(long)]
        node_id: String,
        /// Free-form note saved alongside the pairing.
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Revoke a previously approved pairing.
    RevokePeer {
        #[arg(long)]
        node_id: String,
    },
    /// List all currently approved pairings.
    ListPairings,
    /// Run the direct-sync protocol against a paired peer. The peer
    /// must already have an active QUIC connection (call
    /// `connect-to-peer` first).
    SyncWithPeer {
        #[arg(long)]
        node_id: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        eprintln!("rendezvous-test-client: {error}");
        let mut source = std::error::Error::source(&error);
        while let Some(inner) = source {
            eprintln!("  caused by: {inner}");
            source = inner.source();
        }
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[derive(Debug, thiserror::Error)]
enum ClientError {
    #[error(transparent)]
    Connect(#[from] rendezvous_client::ConnectError),
    #[error("gRPC call failed: {0}")]
    Rpc(#[from] tonic::Status),
}

async fn run() -> Result<(), ClientError> {
    init_tracing();

    let cli = Cli::parse();
    let socket = cli.socket.unwrap_or_else(default_socket_path);
    tracing::debug!(socket = %socket.display(), "connecting to rendezvous-daemon");

    let mut client = connect(socket).await?;

    match cli.command {
        Command::Ping { nonce } => {
            let response = client
                .ping(PingRequest {
                    nonce: nonce.clone(),
                })
                .await?
                .into_inner();
            println!(
                "pong nonce={} daemon_version={} timestamp_unix_ms={}",
                response.nonce, response.daemon_version, response.timestamp_unix_ms,
            );
        }
        Command::Status => {
            let response = client.status(StatusRequest {}).await?.into_inner();
            println!(
                "status daemon_version={} uptime_seconds={} started_at_unix_ms={} node_id={}",
                response.daemon_version,
                response.uptime_seconds,
                response.started_at_unix_ms,
                response.node_id,
            );
        }
        Command::AppendEntry {
            owner_node_id,
            session_id,
            source,
            level,
            message,
            metadata_json,
        } => {
            let response = client
                .append_entry(AppendEntryRequest {
                    owner_node_id,
                    session_id,
                    source,
                    level,
                    message,
                    metadata_json,
                })
                .await?
                .into_inner();
            println!(
                "appended chunk_id={} chunk_index={} sequence={} rotated={} timestamp_unix_ms={}",
                response.chunk_id,
                response.chunk_index,
                response.sequence,
                response.rotated_chunk,
                response.created_at_unix_ms,
            );
        }
        Command::ListChunks {
            owner_node_id,
            session_id,
        } => {
            let response = client
                .list_session_chunks(ListSessionChunksRequest {
                    owner_node_id,
                    session_id,
                })
                .await?
                .into_inner();
            for chunk in response.chunk_ids {
                println!("{chunk}");
            }
        }
        Command::ListEntries { chunk_id } => {
            let response = client
                .list_chunk_entries(ListChunkEntriesRequest { chunk_id })
                .await?
                .into_inner();
            for entry in response.entries {
                println!(
                    "seq={} source={} level={} message={}",
                    entry.sequence, entry.source, entry.level, entry.message,
                );
            }
        }
        Command::QueryProjection {
            owner_node_id,
            session_id,
        } => {
            let response = client
                .query_projection(QueryProjectionRequest {
                    owner_node_id,
                    session_id,
                })
                .await?
                .into_inner();
            for row in response.rows {
                println!(
                    "chunk={} chunk_index={} sequence={} source={} level={} message={}",
                    row.chunk_id,
                    row.chunk_index,
                    row.sequence,
                    row.source,
                    row.level,
                    row.message,
                );
            }
        }
        Command::CreateInvitation { advertise_addr } => {
            let response = client
                .create_invitation(CreateInvitationRequest { advertise_addr })
                .await?
                .into_inner();
            println!(
                "invitation={} node_id={} socket_addr={}",
                response.invitation, response.node_id, response.socket_addr,
            );
        }
        Command::ConnectToPeer { invitation } => {
            let response = client
                .connect_to_peer(ConnectToPeerRequest { invitation })
                .await?
                .into_inner();
            match response.peer {
                Some(peer) => println!(
                    "connected node_id={} socket_addr={} state={} source={} last_error={}",
                    peer.node_id,
                    peer.socket_addr,
                    peer.state,
                    peer.source,
                    peer.last_error,
                ),
                None => println!("connected (no peer info returned)"),
            }
        }
        Command::ListPeers => {
            let response = client
                .list_peers(ListPeersRequest {})
                .await?
                .into_inner();
            for peer in response.peers {
                println!(
                    "node_id={} socket_addr={} state={} source={} last_seen_unix_ms={} last_synced_unix_ms={} last_error={}",
                    peer.node_id,
                    peer.socket_addr,
                    peer.state,
                    peer.source,
                    peer.last_seen_unix_ms,
                    peer.last_synced_unix_ms,
                    peer.last_error,
                );
            }
        }
        Command::ApprovePeer { node_id, note } => {
            let response = client
                .approve_peer(ApprovePeerRequest { node_id, note })
                .await?
                .into_inner();
            match response.pairing {
                Some(pairing) => println!(
                    "approved node_id={} paired_at_unix_ms={} note={}",
                    pairing.node_id, pairing.paired_at_unix_ms, pairing.note,
                ),
                None => println!("approved (no pairing info returned)"),
            }
        }
        Command::RevokePeer { node_id } => {
            let response = client
                .revoke_peer(RevokePeerRequest { node_id })
                .await?
                .into_inner();
            println!("removed={}", response.removed);
        }
        Command::ListPairings => {
            let response = client
                .list_pairings(ListPairingsRequest {})
                .await?
                .into_inner();
            for pairing in response.pairings {
                println!(
                    "node_id={} paired_at_unix_ms={} note={}",
                    pairing.node_id, pairing.paired_at_unix_ms, pairing.note,
                );
            }
        }
        Command::SyncWithPeer { node_id } => {
            let response = client
                .sync_with_peer(SyncWithPeerRequest { node_id })
                .await?
                .into_inner();
            println!(
                "synced node_id={} received_bytes={} sent_bytes={} chunks={}",
                response.node_id,
                response.received_bytes,
                response.sent_bytes,
                response.chunks.len(),
            );
            for chunk in response.chunks {
                println!(
                    "  chunk={} received_bytes={} sent_bytes={} advanced={}",
                    chunk.chunk_id, chunk.received_bytes, chunk.sent_bytes, chunk.advanced,
                );
            }
        }
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,rendezvous_client=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
