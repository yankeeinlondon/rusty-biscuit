//! `claudine dashboard` — the mesh NOW view (rendezvous dashboard D1).
//!
//! A read-only CLI over the rendezvous daemon's live read RPCs
//! (`ListActiveSessions`, `ListHostCapabilities`, `ListHostRepos`) plus
//! `ListPeers` (per-peer sync freshness) and `Status` (the local node
//! id). The five responses fold into a [`MeshSnapshot`] that
//! [`DashboardReport`] renders to the terminal (and, dual-target, to
//! HTML). An absent daemon degrades to a friendly note, never an error.

use std::collections::BTreeMap;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use clap::Args;
use color_eyre::eyre::{Result, WrapErr};
use rendezvous_core::RendezvousClient;
use tonic::transport::Channel;

use crate::log;

mod model;
mod report;
#[cfg(test)]
mod tests;

use model::MeshSnapshot;
use report::DashboardReport;

/// Total wall-clock budget for the snapshot's read RPCs. A daemon that
/// accepts the IPC connection but then stalls in a handler must not hang
/// the CLI: past this we degrade to the not-responding note and exit 0.
const FETCH_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// Arguments for the `claudine dashboard` subcommand.
#[derive(Debug, Args)]
pub struct DashboardArgs {
    /// Show only this host's sessions, not the whole mesh.
    #[arg(long)]
    pub local: bool,
}

/// Render the mesh NOW view once and return.
pub async fn run(args: DashboardArgs) -> Result<()> {
    let term = log::terminal();
    let endpoint = rendezvous_core::socket::default_socket_path();

    let mut client = match rendezvous_client::connect(endpoint).await {
        Ok(client) => client,
        Err(err) => {
            tracing::debug!(target: "claudine::dashboard", %err, "daemon unreachable");
            log::message(
                &Prose::new(
                    "<dim>No rendezvous daemon is running — the dashboard needs one to read \
                     the mesh. Start it with </dim><bold>rendezvous-daemon</bold><dim>.</dim>",
                )
                .render(&term),
            );
            return Ok(());
        }
    };

    let fetch = fetch_snapshot(&mut client, unix_now_ms(), args.local);
    let snapshot = match fetch_within_deadline(FETCH_DEADLINE, fetch).await {
        Some(result) => result?,
        None => {
            tracing::debug!(
                target: "claudine::dashboard",
                "daemon accepted the connection but stalled past the fetch deadline"
            );
            log::message(
                &Prose::new(
                    "<dim>The rendezvous daemon is not responding — it accepted the connection \
                     but did not answer in time. Check </dim><bold>rendezvous-daemon</bold><dim>.\
                     </dim>",
                )
                .render(&term),
            );
            return Ok(());
        }
    };

    let scope = if args.local { "local" } else { "mesh" };
    let report = DashboardReport::new(snapshot, scope).with_inline_terminal(log::terminal());
    log::data(&report.render(&term));
    Ok(())
}

/// Query the five read RPCs and fold them into the mesh view. Split
/// from [`run`] so it can be driven end-to-end against a live daemon
/// without going through terminal rendering.
async fn fetch_snapshot(
    client: &mut RendezvousClient<Channel>,
    now_ms: i64,
    local_only: bool,
) -> Result<MeshSnapshot> {
    let status = client
        .status(rendezvous_core::StatusRequest {})
        .await
        .wrap_err("querying daemon status")?
        .into_inner();
    let peers = client
        .list_peers(rendezvous_core::ListPeersRequest {})
        .await
        .wrap_err("listing peers")?
        .into_inner()
        .peers;
    let active = client
        .list_active_sessions(rendezvous_core::ListActiveSessionsRequest {})
        .await
        .wrap_err("listing active sessions")?
        .into_inner()
        .hosts;
    let capabilities = client
        .list_host_capabilities(rendezvous_core::ListHostCapabilitiesRequest {})
        .await
        .wrap_err("listing host capabilities")?
        .into_inner()
        .hosts;
    let repos = client
        .list_host_repos(rendezvous_core::ListHostReposRequest {})
        .await
        .wrap_err("listing host repos")?
        .into_inner()
        .hosts;

    let last_synced: BTreeMap<String, i64> = peers
        .iter()
        .map(|p| (p.node_id.clone(), p.last_synced_unix_ms))
        .collect();
    let active_blobs: Vec<(&str, &str)> = active
        .iter()
        .map(|h| (h.owner_node_id.as_str(), h.sessions_json.as_str()))
        .collect();
    let capability_blobs: Vec<(&str, &str)> = capabilities
        .iter()
        .map(|h| (h.owner_node_id.as_str(), h.fields_json.as_str()))
        .collect();
    let repo_blobs: Vec<(&str, &str)> = repos
        .iter()
        .map(|h| (h.owner_node_id.as_str(), h.repos_json.as_str()))
        .collect();

    Ok(MeshSnapshot::fold(
        &status.node_id,
        now_ms,
        &last_synced,
        &active_blobs,
        &capability_blobs,
        &repo_blobs,
        local_only,
    ))
}

/// Await `fetch` but never longer than `deadline`.
///
/// ## Returns
///
/// `Some(result)` when the fetch completed in time (carrying its own
/// success or RPC error), or `None` when the deadline elapsed first —
/// the caller treats `None` as "the daemon accepted IPC but stalled" and
/// degrades gracefully rather than hanging.
async fn fetch_within_deadline(
    deadline: std::time::Duration,
    fetch: impl std::future::Future<Output = Result<MeshSnapshot>>,
) -> Option<Result<MeshSnapshot>> {
    tokio::time::timeout(deadline, fetch).await.ok()
}

fn unix_now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(now).unwrap_or(i64::MAX)
}
