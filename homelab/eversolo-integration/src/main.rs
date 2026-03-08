//! Eversolo DMP-A8 -- Unfolded Circle external integration driver.
//!
//! Standalone WebSocket server that speaks the UC Integration protocol,
//! exposing an Eversolo streamer as a power switch and media player entity.

mod dispatch;
mod error;
mod handler;
mod types;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use handler::EversoloIntegrationHandler;
use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleIntegrationWsHost;
use types::{DRIVER_ID, DRIVER_VERSION, DeviceConfig, MIN_CORE_API};

/// Eversolo integration driver for Unfolded Circle remotes.
#[derive(Parser, Debug)]
#[command(name = "eversolo-integration", version)]
struct Args {
    /// WebSocket listen address.
    #[arg(long, default_value = "0.0.0.0:9092")]
    listen: String,

    /// Eversolo device host (IP or hostname).
    #[arg(long)]
    host: String,

    /// Eversolo HTTP API port.
    #[arg(long, default_value_t = 9529)]
    port: u16,

    /// Device name used in entity IDs (e.g. "main" -> eversolo.main.player).
    #[arg(long, default_value = "streamer")]
    device_name: String,

    /// Wired MAC address used for Wake-on-LAN power on.
    #[arg(long)]
    mac: Option<String>,

    /// Broadcast address used for Wake-on-LAN packets.
    #[arg(long, default_value = "255.255.255.255")]
    wol_broadcast: String,

    /// UDP port used for Wake-on-LAN packets.
    #[arg(long, default_value_t = 9517)]
    wol_port: u16,

    /// Timeout in seconds for Eversolo HTTP operations.
    #[arg(long, default_value_t = 10)]
    timeout: u64,

    /// Poll interval in seconds for refreshing device state.
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

    let mut devices = HashMap::new();
    devices.insert(
        args.device_name.clone(),
        DeviceConfig {
            host: args.host.clone(),
            port: args.port,
            mac_address: args.mac.clone(),
            wol_broadcast: args.wol_broadcast.clone(),
            wol_port: args.wol_port,
        },
    );

    let hub = UnfoldedCircleIntegrationWsHost::new_event_hub();
    let handler = Arc::new(EversoloIntegrationHandler::new(
        devices,
        Duration::from_secs(args.timeout),
        hub.clone(),
    ));
    handler.refresh_all(false).await;
    handler.start_polling(Duration::from_secs(args.poll_interval));

    let _mdns = if args.mdns {
        let ws_port = args
            .listen
            .rsplit_once(':')
            .and_then(|(_, p)| p.parse::<u16>().ok())
            .expect("--listen must contain a valid port (e.g. 0.0.0.0:9092)");
        Some(unfolded_integration_helper::mdns::MdnsAdvertiser::new(
            DRIVER_ID,
            DRIVER_VERSION,
            MIN_CORE_API,
            ws_port,
        )?)
    } else {
        None
    };

    info!(
        listen = %args.listen,
        host = %args.host,
        port = args.port,
        device_name = %args.device_name,
        has_mac = args.mac.is_some(),
        wol_broadcast = %args.wol_broadcast,
        wol_port = args.wol_port,
        poll_interval = args.poll_interval,
        mdns = args.mdns,
        "starting Eversolo UC integration driver"
    );

    UnfoldedCircleIntegrationWsHost::serve_addr_with_hub(&args.listen, handler, hub).await?;

    Ok(())
}
