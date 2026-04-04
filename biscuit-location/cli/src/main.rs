//! CLI for location services: GPS, IP lookup, reverse geocoding, and distance.
//!
//! ## Usage
//!
//! ```bash
//! where gps                              # Get location from GPS
//! where ip 8.8.8.8                       # Look up IP location
//! where reverse 34.0522 -118.2437        # Reverse geocode coordinates
//! where distance 34.05,-118.24 40.71,-74.01  # Distance between points
//! ```

mod args;
mod commands;
mod output;

use clap::Parser;

use args::Cli;

#[tokio::main]
async fn main() {
    color_eyre::install().ok();

    let cli = Cli::parse();

    if let Err(err) = commands::run(cli).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
