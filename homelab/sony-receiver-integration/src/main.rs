//! Sony Receiver -- Unfolded Circle External Integration Driver.
//!
//! Standalone WebSocket server that speaks the UC Integration protocol,
//! exposing Sony ES receivers (STR-AZ7000ES and similar) as power switch
//! and media player entities with volume, mute, and input source control.

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

use handler::SonyIntegrationHandler;
use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleIntegrationWsHost;
use types::{DRIVER_ID, DRIVER_VERSION, MIN_CORE_API};

/// Sony receiver integration driver for Unfolded Circle remotes.
#[derive(Parser, Debug)]
#[command(name = "sony-receiver-integration", version)]
struct Args {
    /// WebSocket listen address
    #[arg(long, default_value = "0.0.0.0:9091")]
    listen: String,

    /// Sony receiver host (IP or hostname)
    #[arg(long)]
    host: String,

    /// Sony receiver JSON-RPC port
    #[arg(long, default_value_t = 10000)]
    port: u16,

    /// Device name used in entity IDs (e.g., "living" -> sony.living.power)
    #[arg(long, default_value = "receiver")]
    device_name: String,

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
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let mut devices = HashMap::new();
    devices.insert(args.device_name.clone(), (args.host.clone(), args.port));

    let hub = UnfoldedCircleIntegrationWsHost::new_event_hub();
    let handler = Arc::new(SonyIntegrationHandler::new(
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
            .expect("--listen must contain a valid port (e.g. 0.0.0.0:9091)");
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
        poll_interval = args.poll_interval,
        mdns = args.mdns,
        "starting Sony receiver UC integration driver"
    );

    UnfoldedCircleIntegrationWsHost::serve_addr_with_hub(&args.listen, handler, hub).await?;

    Ok(())
}
