//! `rendezvous-daemon` binary entry point.
//!
//! Phase 1 booted the gRPC server on a Unix Domain Socket. Phase 2 also
//! brings up the persistence stack: a redb file as the source of truth
//! for Loro snapshots and a DuckDB analytical projection driven by a
//! flume-based micro-batching pipeline. Networking, pairing, and sync
//! subsystems arrive in later phases.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use rendezvous_core::socket::default_socket_path;
use rendezvous_daemon::server::{DaemonConfig, NetworkConfig, ServerError, spawn_uds_server};
use tracing_subscriber::EnvFilter;

/// CLI arguments for the daemon.
#[derive(Debug, Parser)]
#[command(
    name = "rendezvous-daemon",
    about = "Rendezvous companion daemon (Phase 2: session-log persistence)"
)]
struct Cli {
    /// Override the Unix Domain Socket path. Defaults to the value
    /// resolved by `rendezvous_core::socket::default_socket_path`
    /// (also honouring `$RENDEZVOUS_SOCKET`).
    #[arg(long = "socket", env = "RENDEZVOUS_SOCKET")]
    socket: Option<PathBuf>,

    /// Directory used for the daemon's persistent state (redb file and
    /// DuckDB projection). Defaults to `$RENDEZVOUS_DATA_DIR` if
    /// set, otherwise `<tempdir>/rendezvous-data`.
    #[arg(long = "data-dir", env = "RENDEZVOUS_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Keep the DuckDB projection in memory instead of persisting it.
    /// Handy for short-lived debugging sessions where rebuilding the
    /// projection from redb on every restart is acceptable.
    /// Directory to scan (bounded depth) for git checkouts feeding the
    /// repos/{node_id} register. Repeatable; also settable as a
    /// colon-separated list via the environment.
    #[arg(long = "repo-root", env = "RENDEZVOUS_REPO_ROOTS", value_delimiter = ':')]
    repo_roots: Vec<PathBuf>,

    #[arg(long = "in-memory-projection")]
    in_memory_projection: bool,

    /// UDP socket address the Phase-4 QUIC endpoint binds to. A `0`
    /// port lets the OS pick. Defaults to `0.0.0.0:0`.
    #[arg(long = "quic-bind", env = "RENDEZVOUS_QUIC_BIND")]
    quic_bind: Option<SocketAddr>,

    /// Disable mDNS advertising and browsing. The daemon still
    /// accepts manual invitations.
    #[arg(long = "no-mdns")]
    no_mdns: bool,

    /// Disable the entire Phase-4 networking stack (QUIC + mDNS).
    /// Phase 1–3 functionality remains available over the UDS gRPC
    /// surface.
    #[arg(long = "no-networking", conflicts_with_all = ["quic_bind", "no_mdns"])]
    no_networking: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        eprintln!("rendezvous-daemon: {error}");
        let mut source = std::error::Error::source(&error);
        while let Some(inner) = source {
            eprintln!("  caused by: {inner}");
            source = inner.source();
        }
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn run() -> Result<(), ServerError> {
    init_tracing();

    let cli = Cli::parse();
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);
    let data_dir = cli
        .data_dir
        .unwrap_or_else(|| std::env::temp_dir().join("rendezvous-data"));
    let mut config = DaemonConfig::with_data_dir(&data_dir);
    if !cli.repo_roots.is_empty() {
        config = config.with_repo_scan_roots(cli.repo_roots.clone());
    }
    if cli.in_memory_projection {
        config = config.with_in_memory_projection();
    }
    if cli.no_networking {
        config = config.without_networking();
    } else {
        let mut networking = NetworkConfig::default();
        if let Some(bind) = cli.quic_bind {
            networking.quic_bind = bind;
        }
        if cli.no_mdns {
            networking.mdns_enabled = false;
        }
        config = config.with_networking(networking);
    }
    let handle = spawn_uds_server(socket_path, config)?;

    tracing::info!(
        socket = %handle.socket_path().display(),
        data_dir = %data_dir.display(),
        quic_addr = ?handle.quic_local_addr(),
        "rendezvous-daemon listening"
    );

    wait_for_shutdown_signal().await;
    tracing::info!("shutdown signal received, stopping rendezvous-daemon");

    handle.shutdown().await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,rendezvous_daemon=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(error) => {
            tracing::warn!(%error, "failed to install SIGTERM handler; falling back to Ctrl-C only");
            let _ = tokio::signal::ctrl_c().await;
            return;
        }
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = sigterm.recv() => {},
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
