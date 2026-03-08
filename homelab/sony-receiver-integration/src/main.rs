//! Sony Receiver -- Unfolded Circle External Integration Driver.
//!
//! Standalone WebSocket server that speaks the UC Integration protocol,
//! exposing Sony ES receivers (STR-AZ7000ES and similar) as power switch
//! and media player entities with volume, mute, and input source control.

#[allow(dead_code)]
mod dispatch;
mod discovery;
mod driver;
#[allow(dead_code)]
mod error;
mod handler;
#[allow(dead_code)]
mod types;

use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use handler::SonyIntegrationHandler;
use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleIntegrationWsHost;
use types::{DRIVER_ID, DRIVER_VERSION, MIN_CORE_API};
use unfolded_integration_helper::{DeviceManager, PersistentRegistry, SubscriptionRegistry};

/// Sony receiver integration driver for Unfolded Circle remotes.
#[derive(Parser, Debug)]
#[command(name = "sony-receiver-integration", version)]
struct Args {
    /// WebSocket listen address
    #[arg(long, default_value = "0.0.0.0:9091")]
    listen: String,

    /// Sony receiver host (IP or hostname). Optional seed hint.
    #[arg(long)]
    host: Option<String>,

    /// Sony receiver JSON-RPC port
    #[arg(long, default_value_t = 10000)]
    port: u16,

    /// Device name used in entity IDs (e.g., "living" -> sony.living.power)
    #[arg(long, default_value = "receiver")]
    device_name: String,

    /// Override default persistence directory
    #[arg(long)]
    data_dir: Option<String>,

    /// Timeout in seconds for Sony HTTP operations
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Poll interval in seconds for refreshing receiver state.
    #[arg(long, default_value_t = 5)]
    poll_interval: u64,

    /// Advertise this integration via mDNS/DNS-SD for auto-discovery.
    #[arg(long)]
    mdns: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,mdns_sd::service_daemon=off")),
        )
        .init();

    let args = Args::parse();

    // Load or create registry
    let registry_path = args
        .data_dir
        .as_ref()
        .map(|d| std::path::PathBuf::from(d).join("registry.json"))
        .unwrap_or_else(|| PersistentRegistry::default_path(DRIVER_ID));

    let registry = PersistentRegistry::load(&registry_path).await?;

    let hub = UnfoldedCircleIntegrationWsHost::new_event_hub();
    let subscriptions = SubscriptionRegistry::new(hub.clone());
    let manager = DeviceManager::new(
        registry.clone(),
        subscriptions,
        Duration::from_secs(args.timeout),
        Duration::from_secs(args.poll_interval),
    );

    // Seed from --host if provided
    if let Some(ref host) = args.host {
        let _ = registry
            .seed_from_cli_hint(host, args.port, &args.device_name)
            .await;
        registry.save().await?;
    }

    let handler = Arc::new(SonyIntegrationHandler::new(manager));
    let metadata = types::driver_metadata();
    let published_name = metadata
        .name
        .get("en")
        .cloned()
        .unwrap_or_else(|| DRIVER_ID.to_string());
    let developer_name = metadata
        .developer
        .as_ref()
        .map(|developer| developer.name.clone())
        .unwrap_or_else(|| "Unknown developer".to_string());

    let _mdns = if args.mdns {
        let ws_port = args
            .listen
            .rsplit_once(':')
            .and_then(|(_, p)| p.parse::<u16>().ok())
            .expect("--listen must contain a valid port (e.g. 0.0.0.0:9091)");
        Some(unfolded_integration_helper::mdns::MdnsAdvertiser::new(
            DRIVER_ID,
            &published_name,
            &developer_name,
            DRIVER_VERSION,
            MIN_CORE_API,
            ws_port,
        )?)
    } else {
        None
    };

    info!(
        listen = %args.listen,
        host = ?args.host,
        port = args.port,
        device_name = %args.device_name,
        poll_interval = args.poll_interval,
        mdns = args.mdns,
        "starting Sony receiver UC integration driver"
    );

    UnfoldedCircleIntegrationWsHost::serve_addr_with_hub(&args.listen, handler, hub).await?;

    Ok(())
}
