use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};

/// Location services: GPS, IP lookup, reverse geocoding, and distance.
#[derive(Parser)]
#[command(name = "where", version, about, long_about = None)]
#[command(after_help = AFTER_HELP)]
pub struct Cli {
    /// Override MaxMind database path
    #[arg(long, global = true, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub db_path: Option<PathBuf>,

    /// Include a Google Maps link in location output
    #[arg(long, global = true)]
    pub maps: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Get current location from host GPS
    Gps {
        /// GPS fix timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,
    },

    /// Look up geographic location of an IP address
    Ip {
        /// IPv4 or IPv6 address to look up
        address: IpAddr,
    },

    /// Reverse geocode coordinates to a place name
    Reverse {
        /// Latitude (-90 to 90)
        lat: f64,
        /// Longitude (-180 to 180)
        lon: f64,
    },

    /// Calculate distance between two locations
    Distance {
        /// First location: gps, ip:<addr>, or lat,lon
        from: String,
        /// Second location: gps, ip:<addr>, or lat,lon
        to: String,
        /// Distance unit
        #[arg(long, default_value = "kilometers", value_parser = parse_unit)]
        unit: biscuit_location::DistanceUnit,
    },
}

fn parse_unit(s: &str) -> Result<biscuit_location::DistanceUnit, String> {
    match s.to_lowercase().as_str() {
        "m" | "meters" => Ok(biscuit_location::DistanceUnit::Meters),
        "km" | "kilometers" => Ok(biscuit_location::DistanceUnit::Kilometers),
        "mi" | "miles" => Ok(biscuit_location::DistanceUnit::Miles),
        "nm" | "nautical" | "nauticalmiles" => Ok(biscuit_location::DistanceUnit::NauticalMiles),
        _ => Err(format!(
            "unknown unit '{s}': expected meters, kilometers, miles, or nauticalmiles"
        )),
    }
}

const AFTER_HELP: &str = "\
EXAMPLES:
  where gps                              Get location from GPS
  where ip 8.8.8.8                       Look up Google DNS location
  where reverse 34.0522 -118.2437        Reverse geocode LA coordinates
  where distance 34.05,-118.24 40.71,-74.01   LA to NYC distance
  where distance gps ip:8.8.8.8         GPS to IP-based location
  where ip 8.8.8.8 --maps               Include Google Maps link
";
