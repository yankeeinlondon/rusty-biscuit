mod api;
mod daemon;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use biscuit_clipboard::backend::SystemClipboard;
use biscuit_clipboard::config::WatcherRestartPolicy;
use biscuit_clipboard::watcher::{WatcherEvent, spawn_system_watcher};
use biscuit_clipboard::{History, Storage, Supervisor, SupervisorAction, SupervisorStatus};
use clap::Parser;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use api::AppState;

#[derive(Parser, Debug)]
#[command(name = "clipper", about = "Biscuit clipboard background service")]
struct Args {
    #[arg(long, default_value = "0")]
    port: u16,

    #[arg(long)]
    pid_file: Option<String>,

    #[arg(long)]
    port_file: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let daemon_files = create_daemon_files(&args);

    if daemon_files.is_already_running() {
        error!(
            pid = ?daemon_files.read_pid(),
            "clipper is already running"
        );
        std::process::exit(1);
    }

    let _lock = match daemon_files.acquire_lock() {
        Ok(lock) => lock,
        Err(e) => {
            error!(error = %e, "failed to acquire lock");
            std::process::exit(1);
        }
    };

    if let Err(e) = daemon_files.write_pid() {
        error!(error = %e, "failed to write PID file");
        std::process::exit(1);
    }

    let storage = match Storage::new() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to initialize storage");
            std::process::exit(1);
        }
    };

    // Live clipboard backend for `/current` reads. The watcher
    // supervisor still constructs its own `SystemClipboard` per restart
    // (see `run_watcher_supervisor`); the two are independent contexts.
    let live_backend: Arc<dyn biscuit_clipboard::ClipboardBackend + Send + Sync> =
        match SystemClipboard::new() {
            Ok(b) => Arc::new(b),
            Err(e) => {
                error!(error = %e, "failed to construct live clipboard backend");
                std::process::exit(1);
            }
        };

    let watcher_status = Arc::new(RwLock::new(SupervisorStatus::Running));
    let events_tx = AppState::new_events_channel();

    let state = Arc::new(AppState {
        history: Mutex::new(History::new()),
        storage,
        clipboard: live_backend,
        watcher_status: watcher_status.clone(),
        events_tx,
    });

    // Wire the watcher → history pipeline. The history Mutex inside
    // `state` is the single source of truth; the watcher task locks it
    // per-event.
    let state_for_watcher = state.clone();
    let watcher_status_for_supervisor = watcher_status.clone();
    let restart_policy = WatcherRestartPolicy::from_env();

    tokio::spawn(async move {
        run_watcher_supervisor(
            state_for_watcher,
            watcher_status_for_supervisor,
            restart_policy,
        )
        .await;
    });

    let app = api::router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(%addr, error = %e, "failed to bind");
            daemon_files.cleanup().ok();
            std::process::exit(1);
        }
    };

    let actual_port = listener.local_addr().unwrap().port();
    if let Err(e) = daemon_files.write_port(actual_port) {
        error!(error = %e, "failed to write port file");
    }

    info!(port = actual_port, "clipper listening on 127.0.0.1");

    let df_sigint = daemon_files.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("received SIGINT, shutting down clipper");
        df_sigint.cleanup().ok();
        std::process::exit(0);
    });

    #[cfg(unix)]
    {
        let df_sigterm = daemon_files.clone();
        tokio::spawn(async move {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .unwrap()
                .recv()
                .await;
            info!("received SIGTERM, shutting down");
            df_sigterm.cleanup().ok();
            std::process::exit(0);
        });
    }

    if let Err(e) = axum::serve(listener, app).await {
        error!(error = %e, "server error");
    }

    daemon_files.cleanup().ok();
}

/// Spawn the system watcher and pipe its events into history. On
/// watcher death (panic or `Died` event), use the supervisor to retry
/// with exponential backoff up to the configured retry budget. When the
/// budget is exhausted, mark the shared `watcher_status` as `Stopped`
/// and exit; `/health` will return 503 from that point on.
async fn run_watcher_supervisor(
    state: Arc<AppState>,
    status: Arc<RwLock<SupervisorStatus>>,
    policy: WatcherRestartPolicy,
) {
    let mut supervisor = Supervisor::new()
        .with_max_retries(policy.max_retries)
        .with_base_delay(policy.base_delay)
        .with_max_delay(policy.max_delay);

    loop {
        let backend = match SystemClipboard::new() {
            Ok(b) => Arc::new(b),
            Err(e) => {
                error!(error = %e, "failed to construct SystemClipboard");
                handle_failure(&mut supervisor, &status).await;
                if matches!(*supervisor.status(), SupervisorStatus::Stopped) {
                    return;
                }
                continue;
            }
        };

        let (handle, mut rx) = match spawn_system_watcher(backend) {
            Ok(pair) => pair,
            Err(e) => {
                error!(error = %e, "failed to spawn system watcher");
                handle_failure(&mut supervisor, &status).await;
                if matches!(*supervisor.status(), SupervisorStatus::Stopped) {
                    return;
                }
                continue;
            }
        };

        info!("clipboard watcher running");
        supervisor.record_success();
        *status.write().await = SupervisorStatus::Running;

        let mut watcher_died = false;
        while let Some(event) = rx.recv().await {
            if !apply_watcher_event(&state, event, &mut watcher_died).await {
                break;
            }
        }

        // Drop the handle to join the watcher thread cleanly.
        drop(handle);

        if !watcher_died {
            warn!("watcher channel closed without explicit death; restarting");
        }

        handle_failure(&mut supervisor, &status).await;
        if matches!(*supervisor.status(), SupervisorStatus::Stopped) {
            error!("watcher supervisor exhausted retry budget; service is degraded");
            return;
        }
    }
}

/// Apply a single [`WatcherEvent`] to the daemon's history. Returns
/// `true` to continue draining the channel, `false` to break out (the
/// watcher died and the supervisor should re-spawn).
async fn apply_watcher_event(
    state: &Arc<AppState>,
    event: WatcherEvent,
    watcher_died: &mut bool,
) -> bool {
    match event {
        WatcherEvent::Changed { formats, concealed } => {
            if concealed {
                debug!("watcher: dropping concealed clipboard event");
                return true;
            }
            if formats.is_empty() {
                return true;
            }
            let mut history = state.history.lock().await;
            let summary = history
                .insert(formats)
                .map(biscuit_clipboard::api_types::EntrySummary::from);
            let history_len = history.len();
            drop(history);

            if let Some(summary) = summary {
                debug!(entries = history_len, "history: inserted new entry");
                // Fan out to SSE subscribers; failure means no subscribers
                // are listening, which is fine.
                let _ = state.events_tx.send(summary);
            }
            true
        }
        WatcherEvent::Died { reason } => {
            error!(reason = %reason, "watcher died, will restart");
            *watcher_died = true;
            false
        }
    }
}

async fn handle_failure(supervisor: &mut Supervisor, status: &Arc<RwLock<SupervisorStatus>>) {
    let action = supervisor.record_failure();
    *status.write().await = supervisor.status().clone();
    match action {
        SupervisorAction::Retry { delay } => {
            warn!(?delay, "watcher will be retried");
            tokio::time::sleep(delay).await;
        }
        SupervisorAction::GiveUp => {
            tokio::time::sleep(Duration::from_millis(0)).await;
        }
    }
}

fn create_daemon_files(args: &Args) -> daemon::DaemonFiles {
    if let Some(pid_path) = &args.pid_file {
        let runtime_dir = std::path::PathBuf::from(pid_path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        daemon::DaemonFiles::with_runtime_dir(runtime_dir)
    } else {
        daemon::DaemonFiles::new().unwrap_or_else(|e| {
            error!(error = %e, "failed to initialize daemon files");
            std::process::exit(1);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_clipboard::content::ClipboardFormat;

    fn make_state() -> Arc<AppState> {
        use biscuit_clipboard::backend::MockClipboard;
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new()
            .unwrap()
            .with_cache_dir(dir.path().to_path_buf());
        // leak the tempdir for the test lifetime — these are unit tests
        // and the OS will reap on exit.
        std::mem::forget(dir);
        Arc::new(AppState {
            history: Mutex::new(History::new()),
            storage,
            clipboard: Arc::new(MockClipboard::default()),
            watcher_status: Arc::new(RwLock::new(SupervisorStatus::Running)),
            events_tx: AppState::new_events_channel(),
        })
    }

    #[tokio::test]
    async fn apply_event_drops_concealed_entries_before_history() {
        let state = make_state();
        let mut watcher_died = false;

        let event = WatcherEvent::Changed {
            formats: vec![ClipboardFormat::Text("super-secret".to_string())],
            concealed: true,
        };

        let cont = apply_watcher_event(&state, event, &mut watcher_died).await;
        assert!(cont, "concealed events must not break the loop");
        assert!(!watcher_died);
        assert_eq!(
            state.history.lock().await.len(),
            0,
            "concealed clipboard events must never reach history"
        );
    }

    #[tokio::test]
    async fn apply_event_inserts_non_concealed_entries() {
        let state = make_state();
        let mut watcher_died = false;

        let event = WatcherEvent::Changed {
            formats: vec![ClipboardFormat::Text("hello".to_string())],
            concealed: false,
        };

        let cont = apply_watcher_event(&state, event, &mut watcher_died).await;
        assert!(cont);
        assert_eq!(state.history.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn apply_event_skips_empty_format_lists() {
        let state = make_state();
        let mut watcher_died = false;

        let event = WatcherEvent::Changed {
            formats: vec![],
            concealed: false,
        };

        let cont = apply_watcher_event(&state, event, &mut watcher_died).await;
        assert!(cont);
        assert_eq!(state.history.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn apply_event_died_breaks_loop_and_marks_dead() {
        let state = make_state();
        let mut watcher_died = false;

        let event = WatcherEvent::Died {
            reason: "panic".to_string(),
        };

        let cont = apply_watcher_event(&state, event, &mut watcher_died).await;
        assert!(!cont, "Died event must break out of the drain loop");
        assert!(watcher_died);
    }

    #[tokio::test]
    async fn handle_failure_marks_status_degraded_then_stopped() {
        let status = Arc::new(RwLock::new(SupervisorStatus::Running));
        let mut supervisor = Supervisor::new()
            .with_max_retries(2)
            .with_base_delay(Duration::from_millis(0))
            .with_max_delay(Duration::from_millis(0));

        handle_failure(&mut supervisor, &status).await;
        assert!(matches!(
            *status.read().await,
            SupervisorStatus::Degraded { .. }
        ));

        handle_failure(&mut supervisor, &status).await;
        assert_eq!(*status.read().await, SupervisorStatus::Stopped);
    }
}
