use get_if_addrs::get_if_addrs;
use homelab_server::{build_router, config::HomeyConfig, state::AppState};
use std::time::Duration;
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Default server port
const DEFAULT_PORT: u16 = 3000;

/// Graceful shutdown timeout for SIGTERM
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
/// Graceful shutdown timeout for Ctrl+C (shorter, user-initiated)
const CTRL_C_TIMEOUT: Duration = Duration::from_secs(3);

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

    // Load configuration: try config file first, fall back to ENV vars
    let app_state = match load_config() {
        Ok(state) => state,
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

    // Log server startup with styled output - show actual IPs, not 0.0.0.0
    let local_ips = get_local_ip_addresses();
    if local_ips.is_empty() {
        println!(
            "\x1b[1mStarting Homelab\x1b[0m server at localhost:{}\n",
            port
        );
    } else {
        println!("\x1b[1mStarting Homelab\x1b[0m server at:");
        for ip in local_ips {
            println!("  http://{}:{}", ip, port);
        }
        println!();
    }

    // Bind to address
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await.unwrap();
    tracing::info!(address = %addr, "Server listening");

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    tracing::info!("Server shutdown complete");
}

/// Loads configuration from file or environment.
///
/// Priority:
/// 1. Load from ~/homey.json (creates if missing)
/// 2. If config is empty, migrate from ENV vars (SONY_RECEIVER, ARCAM_AMP)
/// 3. Save migrated config back to file
fn load_config() -> Result<AppState, Box<dyn std::error::Error>> {
    let config_path = HomeyConfig::default_path();

    // Load or create config file
    let mut config = match &config_path {
        Some(path) => {
            tracing::info!(path = %path.display(), "Loading configuration");
            HomeyConfig::load_from(path)?
        }
        None => {
            tracing::warn!("Home directory not found, using empty config");
            HomeyConfig::new()
        }
    };

    // Migrate from ENV if config is empty
    if config.migrate_from_env() {
        tracing::info!("Migrated devices from environment variables");

        // Log what was migrated
        for (name, service) in &config.sony_receivers {
            tracing::info!(
                name = %name,
                host = %service.host,
                port = service.port,
                "Migrated Sony receiver from SONY_RECEIVER env"
            );
        }
        for (name, service) in &config.arcam_amps {
            tracing::info!(
                name = %name,
                host = %service.host,
                port = service.port,
                "Migrated Arcam amplifier from ARCAM_AMP env"
            );
        }

        // Save migrated config
        if let Some(path) = &config_path {
            config.save_to(path)?;
            tracing::info!(path = %path.display(), "Saved migrated configuration");
        }
    }

    // Log device counts
    tracing::info!(
        sony_receivers = config.sony_receivers.len(),
        arcam_amps = config.arcam_amps.len(),
        "Configuration loaded"
    );

    // Warn about legacy route deprecation if ENV vars are set
    if std::env::var("SONY_RECEIVER").is_ok() {
        tracing::warn!(
            "SONY_RECEIVER env var detected. Legacy /sony/* routes are deprecated. \
             Use /sony_receiver/{{name}}/* instead."
        );
    }
    if std::env::var("ARCAM_AMP").is_ok() {
        tracing::warn!(
            "ARCAM_AMP env var detected. Legacy /arcam/* routes are deprecated. \
             Use /arcam_amp/{{name}}/* instead."
        );
    }

    Ok(AppState::from_config(config, config_path))
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

    let timeout = tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Ctrl+C received, starting graceful shutdown");
            CTRL_C_TIMEOUT
        }
        _ = terminate => {
            tracing::info!("SIGTERM received, starting graceful shutdown");
            SHUTDOWN_TIMEOUT
        }
    };

    // Give in-flight requests time to complete
    tracing::info!(
        timeout_secs = timeout.as_secs(),
        "Waiting for in-flight requests"
    );
    tokio::time::sleep(timeout).await;
}

/// Get local IP addresses (excluding loopback) for display purposes.
fn get_local_ip_addresses() -> Vec<String> {
    get_if_addrs()
        .map(|ifaces| {
            ifaces
                .into_iter()
                .filter(|iface| !iface.is_loopback())
                .map(|iface| format!("{}", iface.ip()))
                .collect()
        })
        .unwrap_or_default()
}
