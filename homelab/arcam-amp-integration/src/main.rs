//! Arcam Amplifier — Unfolded Circle External Integration Driver.
//!
//! Standalone WebSocket server that speaks the UC Integration protocol,
//! exposing Arcam PA-series amplifiers as switch entities (power, mute).

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

use handler::ArcamIntegrationHandler;
use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleIntegrationWsHost;

/// Arcam amplifier integration driver for Unfolded Circle remotes.
#[derive(Parser, Debug)]
#[command(name = "arcam-amp-integration", version)]
struct Args {
    /// WebSocket listen address
    #[arg(long, default_value = "0.0.0.0:9090")]
    listen: String,

    /// Arcam device host (IP or hostname)
    #[arg(long)]
    host: String,

    /// Arcam device TCP port
    #[arg(long, default_value_t = 50000)]
    port: u16,

    /// Device name used in entity IDs (e.g., "office" -> arcam.office.power)
    #[arg(long, default_value = "amp")]
    device_name: String,

    /// Timeout in seconds for Arcam TCP operations
    #[arg(long, default_value_t = 5)]
    timeout: u64,

    /// Poll interval in seconds for refreshing amplifier state.
    #[arg(long, default_value_t = 5)]
    poll_interval: u64,
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
    let handler = Arc::new(ArcamIntegrationHandler::new(
        devices,
        Duration::from_secs(args.timeout),
        hub.clone(),
    ));
    handler.refresh_all(false).await;
    handler.start_polling(Duration::from_secs(args.poll_interval));

    info!(
        listen = %args.listen,
        host = %args.host,
        port = args.port,
        device_name = %args.device_name,
        poll_interval = args.poll_interval,
        "starting Arcam UC integration driver"
    );

    UnfoldedCircleIntegrationWsHost::serve_addr_with_hub(&args.listen, handler, hub).await?;

    Ok(())
}
