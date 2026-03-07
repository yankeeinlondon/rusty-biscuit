//! Sony Receiver -- Unfolded Circle External Integration Driver.
//!
//! Standalone WebSocket server that speaks the UC Integration protocol,
//! exposing Sony ES receivers (STR-AZ7000ES and similar) as power switch
//! and media player entities with volume, mute, and input source control.

mod dispatch;
mod error;
mod handler;
mod responses;
mod types;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use handler::SonyIntegrationHandler;
use schematic_schema::unfolded_circle_integration_ws::UnfoldedCircleIntegrationWsHost;

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

    let handler = Arc::new(SonyIntegrationHandler::new(
        devices,
        Duration::from_secs(args.timeout),
    ));

    info!(
        listen = %args.listen,
        host = %args.host,
        port = args.port,
        device_name = %args.device_name,
        "starting Sony receiver UC integration driver"
    );

    UnfoldedCircleIntegrationWsHost::serve_addr(&args.listen, handler).await?;

    Ok(())
}
