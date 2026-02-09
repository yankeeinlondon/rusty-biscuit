use homelab_server::{build_router, state::AppState};
use std::time::Duration;
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Default server port
const DEFAULT_PORT: u16 = 3000;

/// Graceful shutdown timeout
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "homelab_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let app_state = match AppState::from_env() {
        Ok(state) => {
            tracing::info!(
                sony_configured = state.sony.is_some(),
                arcam_configured = state.arcam_host.is_some(),
                timeout_ms = state.request_timeout.as_millis() as u64,
                "Configuration loaded"
            );
            state
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to load configuration");
            std::process::exit(1);
        }
    };

    // Build router
    let app = build_router(app_state);

    // Get port from environment
    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    // Bind to address
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    tracing::info!(address = %addr, "Server listening");

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    tracing::info!("Server shutdown complete");
}

// --- Shutdown Signal ---

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Ctrl+C received, starting graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("SIGTERM received, starting graceful shutdown");
        }
    }

    // Give in-flight requests time to complete
    tracing::info!(timeout_secs = SHUTDOWN_TIMEOUT.as_secs(), "Waiting for in-flight requests");
    tokio::time::sleep(SHUTDOWN_TIMEOUT).await;
}
