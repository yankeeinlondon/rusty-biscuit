use get_if_addrs::get_if_addrs;
use homelab::arcam::Arcam;
use homelab_server::{build_router, config::{self, HomeyConfig}, state::AppState};
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
                .unwrap_or_else(|_| "homelab_server=info,tower_http=info".into()),
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

    // Spawn Arcam heartbeat keepalive task
    spawn_arcam_keepalive(app_state.clone());

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
    if config::migrate_from_env(&mut config) {
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

// --- Arcam Keepalive ---

/// Heartbeat interval for Arcam amplifiers.
///
/// Sends a heartbeat to each configured Arcam amp every 2 minutes to keep
/// the network interface active during standby. The PA series powers down
/// its network port after an extended idle period; periodic heartbeats
/// prevent this.
const ARCAM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2 * 60);

/// Query and cache the Arcam model, logging results.
async fn query_and_cache_model(state: &AppState, host: &str, device_name: &str) {
    let arcam = Arcam::from(host);
    match arcam.get_system_model().await {
        Ok(model) => {
            tracing::info!(device = %device_name, model = %model, "Arcam model identified");
            *state.arcam_model.write().await = Some(model);
            state
                .arcam_heartbeat_alive
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Err(e) => {
            tracing::warn!(device = %device_name, error = %e, "Failed to query Arcam system model");
        }
    }
}

/// Spawns a background task that sends periodic heartbeats to all configured
/// Arcam amplifiers.
///
/// On startup, queries the system model from ALL configured Arcam amps
/// (both legacy and named devices) and caches the first successful result
/// in `state.arcam_model`. Each heartbeat cycle updates
/// `state.arcam_heartbeat_alive` based on success/failure.
fn spawn_arcam_keepalive(state: AppState) {
    tokio::spawn(async move {
        // Query model for ALL configured devices at startup
        // First try named devices from config
        {
            let hosts = state.arcam_hosts.read().await;
            for (name, service) in hosts.iter() {
                query_and_cache_model(&state, &service.host, name).await;
            }
        }

        // Also try legacy device if not already done
        if state.arcam_hosts.read().await.is_empty()
            && let Some(host) = &state.arcam_host
        {
            query_and_cache_model(&state, host, "legacy").await;
        }

        loop {
            tokio::time::sleep(ARCAM_HEARTBEAT_INTERVAL).await;

            let hosts = state.arcam_hosts.read().await;
            let mut any_alive = false;

            // Heartbeat to named devices
            for (name, service) in hosts.iter() {
                let arcam = Arcam::from(service.host.as_str());
                match arcam.heartbeat().await {
                    Ok(true) => {
                        tracing::debug!(device = %name, "Arcam heartbeat: alive");
                        any_alive = true;
                    }
                    Ok(false) => {
                        tracing::warn!(device = %name, "Arcam heartbeat: unexpected response");
                    }
                    Err(e) => {
                        tracing::warn!(device = %name, error = %e, "Arcam heartbeat failed");
                    }
                }
            }

            // Also heartbeat to legacy device if no named devices configured
            if hosts.is_empty() && let Some(host) = &state.arcam_host {
                let arcam = Arcam::from(host.as_str());
                match arcam.heartbeat().await {
                    Ok(true) => {
                        tracing::debug!(device = "legacy", "Arcam heartbeat: alive");
                        any_alive = true;
                    }
                    Ok(false) => {
                        tracing::warn!(device = "legacy", "Arcam heartbeat: unexpected response");
                    }
                    Err(e) => {
                        tracing::warn!(device = "legacy", error = %e, "Arcam heartbeat failed");
                    }
                }
            }

            state
                .arcam_heartbeat_alive
                .store(any_alive, std::sync::atomic::Ordering::Relaxed);
        }
    });
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
