//! Background refresher for this host's locally-owned registers.
//!
//! One task drives every local register: a pass at startup, then one
//! per [`REFRESH_INTERVAL`], each running capability detection
//! ([`crate::capability`]) and the repos scan ([`crate::repos`]).
//! Both are write-on-change, so a pass over an unchanged host touches
//! no document.

use std::path::PathBuf;
use std::time::Duration;

use crate::capability::refresh_capabilities;
use crate::register::RegisterStore;
use crate::repos::refresh_repos;

/// Ratified refresh cadence (host-capability D3: startup + hourly;
/// interval-driven repos until a post-commit event source exists).
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Spawn the register refresher. Detection is synchronous (sniff
/// probes the OS and walks scan roots), so each pass runs on the
/// blocking pool.
///
/// Shutdown discipline: each pass holds a [`RegisterStore`] clone (and
/// through it the open redb database), and an in-flight blocking task
/// cannot be aborted. The refresher therefore takes a shutdown signal
/// and — when signalled mid-pass — WAITS for the pass to finish before
/// exiting, so awaiting the returned handle guarantees the storage
/// handle is released (a daemon restarting on the same storage path
/// would otherwise hit `DatabaseAlreadyOpen`).
pub fn spawn_register_refresher(
    store: RegisterStore,
    repo_scan_roots: Vec<PathBuf>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                _ = ticker.tick() => {}
            }
            let mut pass = tokio::task::spawn_blocking({
                let store = store.clone();
                let roots = repo_scan_roots.clone();
                move || {
                    let capability = refresh_capabilities(&store);
                    let repos = refresh_repos(&store, &roots);
                    (capability, repos)
                }
            });
            let (outcome, stop) = tokio::select! {
                _ = &mut shutdown => ((&mut pass).await, true),
                outcome = &mut pass => (outcome, false),
            };
            match outcome {
                Ok((capability, repos)) => {
                    for (register, result) in [("capability", capability), ("repos", repos)] {
                        match result {
                            Ok(true) => tracing::info!(
                                target: "rendezvous_daemon::refresher",
                                register,
                                "register updated",
                            ),
                            Ok(false) => {}
                            Err(err) => tracing::warn!(
                                target: "rendezvous_daemon::refresher",
                                register,
                                %err,
                                "register refresh failed; will retry next interval",
                            ),
                        }
                    }
                }
                Err(join_err) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::refresher",
                        %join_err,
                        "register refresh task panicked; will retry next interval",
                    );
                }
            }
            if stop {
                break;
            }
        }
    })
}
