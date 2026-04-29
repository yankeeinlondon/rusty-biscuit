use std::net::IpAddr;
use std::path::PathBuf;

use biscuit_location::LocationInput;
use clap::{Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::Shell;

/// Location services: GPS, IP lookup, reverse geocoding, and distance.
#[derive(Parser, Debug)]
#[command(name = "geo", version, about, long_about = None)]
#[command(after_help = AFTER_HELP)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// Override MaxMind database path
    #[arg(long, global = true, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub db_path: Option<PathBuf>,

    /// Include a Google Maps link in location output
    #[arg(long, global = true)]
    pub maps: bool,

    /// Emit machine-readable JSON on stdout
    #[arg(long, global = true, conflicts_with = "plain")]
    pub json: bool,

    /// Strip terminal escape codes from text output
    #[arg(long, global = true)]
    pub plain: bool,

    /// Suppress data output; errors still go to stderr
    #[arg(short, long, global = true, conflicts_with_all = ["verbose", "json"])]
    pub quiet: bool,

    /// Increase diagnostic verbosity on stderr (stackable: -v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Get current location from host GPS
    Gps {
        /// GPS fix timeout in seconds
        #[arg(long, default_value_t = 10, value_name = "SECONDS")]
        timeout: u64,
    },

    /// Look up geographic location of an IP address
    Ip {
        /// IPv4 or IPv6 address to look up
        #[arg(value_name = "ADDRESS")]
        address: IpAddr,
    },

    /// Reverse geocode coordinates to a place name
    Reverse {
        /// Latitude (-90 to 90)
        #[arg(value_name = "LAT", allow_hyphen_values = true)]
        lat: f64,
        /// Longitude (-180 to 180)
        #[arg(value_name = "LON", allow_hyphen_values = true)]
        lon: f64,
        /// HTTP request timeout in seconds
        #[arg(long, default_value_t = 10, value_name = "SECONDS")]
        timeout: u64,
    },

    /// Calculate distance between two locations
    Distance {
        /// First location: gps, ip:<addr>, or lat,lon
        #[arg(value_name = "LOCATION", allow_hyphen_values = true)]
        from: LocationInput,
        /// Second location: gps, ip:<addr>, or lat,lon
        #[arg(value_name = "LOCATION", allow_hyphen_values = true)]
        to: LocationInput,
        /// Distance unit
        #[arg(long, value_enum, default_value_t = DistanceUnitArg::Kilometers, value_name = "UNIT")]
        unit: DistanceUnitArg,
    },

    /// Download or update the MaxMind GeoLite2-City database
    FetchDb {
        /// Re-download even if the database already exists
        #[arg(long)]
        force: bool,
    },

    /// Generate shell completions
    #[command(hide = true)]
    Completions {
        /// Target shell
        #[arg(value_name = "SHELL")]
        shell: Shell,
    },
}

/// CLI-facing distance unit enum, mapped to `biscuit_location::DistanceUnit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DistanceUnitArg {
    /// Meters (m)
    #[value(alias = "m")]
    Meters,
    /// Kilometers (km)
    #[value(alias = "km")]
    Kilometers,
    /// Miles (mi)
    #[value(alias = "mi")]
    Miles,
    /// Nautical miles (nm)
    #[value(name = "nautical-miles", aliases = ["nm", "nautical", "nauticalmiles"])]
    NauticalMiles,
}

impl From<DistanceUnitArg> for biscuit_location::DistanceUnit {
    fn from(value: DistanceUnitArg) -> Self {
        match value {
            DistanceUnitArg::Meters => Self::Meters,
            DistanceUnitArg::Kilometers => Self::Kilometers,
            DistanceUnitArg::Miles => Self::Miles,
            DistanceUnitArg::NauticalMiles => Self::NauticalMiles,
        }
    }
}

const AFTER_HELP: &str = "\
Examples:
  geo gps                                   Get location from GPS
  geo ip 8.8.8.8                            Look up Google DNS location
  geo reverse 34.0522 -118.2437             Reverse geocode LA coordinates
  geo distance 34.05,-118.24 40.71,-74.01   LA to NYC distance
  geo distance gps ip:8.8.8.8               GPS to IP-based location
  geo ip 8.8.8.8 --maps                     Include Google Maps link
  geo ip 8.8.8.8 --json                     Machine-readable JSON output
  geo fetch-db                              Download MaxMind database

Environment variables:
  MAXMIND_LICENSE_KEY  Required for fetch-db and auto-download (free signup at maxmind.com)
  MAXMIND_ACCOUNT_ID   Optional account ID (defaults to '1' for free tier)

Shell completions:
  geo completions bash   > ~/.local/share/bash-completion/completions/geo
  geo completions zsh    > ~/.zfunc/_geo
  geo completions fish   > ~/.config/fish/completions/geo.fish

Exit codes:
  0   success
  1   runtime error (lookup failed, network issue, etc.)
  2   usage error (invalid arguments)
 ";

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("geo").chain(args.iter().copied()))
    }

    #[test]
    fn parses_gps_with_default_timeout() {
        let cli = parse(&["gps"]).unwrap();
        assert!(matches!(cli.command, Commands::Gps { timeout: 10 }));
    }

    #[test]
    fn parses_gps_custom_timeout() {
        let cli = parse(&["gps", "--timeout", "30"]).unwrap();
        assert!(matches!(cli.command, Commands::Gps { timeout: 30 }));
    }

    #[test]
    fn parses_ip_v4() {
        let cli = parse(&["ip", "8.8.8.8"]).unwrap();
        match cli.command {
            Commands::Ip { address } => assert_eq!(address.to_string(), "8.8.8.8"),
            _ => panic!("expected ip command"),
        }
    }

    #[test]
    fn parses_ip_v6() {
        let cli = parse(&["ip", "2001:4860:4860::8888"]).unwrap();
        assert!(matches!(cli.command, Commands::Ip { .. }));
    }

    #[test]
    fn rejects_invalid_ip() {
        assert!(parse(&["ip", "not-an-ip"]).is_err());
    }

    #[test]
    fn parses_reverse_with_negative_longitude() {
        let cli = parse(&["reverse", "34.0522", "-118.2437"]).unwrap();
        match cli.command {
            Commands::Reverse { lat, lon, timeout } => {
                assert!((lat - 34.0522).abs() < 1e-6);
                assert!((lon - (-118.2437)).abs() < 1e-6);
                assert_eq!(timeout, 10);
            }
            _ => panic!("expected reverse command"),
        }
    }

    #[test]
    fn parses_reverse_with_custom_timeout() {
        let cli = parse(&["reverse", "34.0522", "-118.2437", "--timeout", "30"]).unwrap();
        match cli.command {
            Commands::Reverse { timeout, .. } => assert_eq!(timeout, 30),
            _ => panic!("expected reverse command"),
        }
    }

    #[test]
    fn parses_distance_default_unit() {
        let cli = parse(&["distance", "gps", "ip:8.8.8.8"]).unwrap();
        match cli.command {
            Commands::Distance { unit, .. } => assert_eq!(unit, DistanceUnitArg::Kilometers),
            _ => panic!("expected distance command"),
        }
    }

    #[test]
    fn parses_distance_unit_alias() {
        for (alias, expected) in [
            ("m", DistanceUnitArg::Meters),
            ("km", DistanceUnitArg::Kilometers),
            ("miles", DistanceUnitArg::Miles),
            ("mi", DistanceUnitArg::Miles),
            ("nautical-miles", DistanceUnitArg::NauticalMiles),
            ("nm", DistanceUnitArg::NauticalMiles),
        ] {
            let cli = parse(&["distance", "gps", "gps", "--unit", alias]).unwrap();
            match cli.command {
                Commands::Distance { unit, .. } => assert_eq!(unit, expected, "alias {alias}"),
                _ => panic!("expected distance command"),
            }
        }
    }

    #[test]
    fn rejects_unknown_distance_unit() {
        assert!(parse(&["distance", "gps", "gps", "--unit", "furlongs"]).is_err());
    }

    #[test]
    fn parses_distance_coordinate_input() {
        let cli = parse(&["distance", "34.05,-118.24", "40.71,-74.01"]).unwrap();
        assert!(matches!(cli.command, Commands::Distance { .. }));
    }

    #[test]
    fn rejects_invalid_location_input_early() {
        assert!(parse(&["distance", "not-a-location", "gps"]).is_err());
    }

    #[test]
    fn global_flags_conflict_quiet_verbose() {
        assert!(parse(&["--quiet", "--verbose", "gps"]).is_err());
    }

    #[test]
    fn global_flags_conflict_json_plain() {
        assert!(parse(&["--json", "--plain", "gps"]).is_err());
    }

    #[test]
    fn global_flags_conflict_quiet_json() {
        assert!(parse(&["--quiet", "--json", "gps"]).is_err());
    }

    #[test]
    fn verbose_stacks() {
        let cli = parse(&["-vvv", "gps"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn completions_subcommand_parses() {
        let cli = parse(&["completions", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Completions { shell: Shell::Bash }
        ));
    }

    #[test]
    fn distance_unit_conversion_to_lib() {
        assert_eq!(
            biscuit_location::DistanceUnit::from(DistanceUnitArg::Meters),
            biscuit_location::DistanceUnit::Meters
        );
        assert_eq!(
            biscuit_location::DistanceUnit::from(DistanceUnitArg::Kilometers),
            biscuit_location::DistanceUnit::Kilometers
        );
        assert_eq!(
            biscuit_location::DistanceUnit::from(DistanceUnitArg::Miles),
            biscuit_location::DistanceUnit::Miles
        );
        assert_eq!(
            biscuit_location::DistanceUnit::from(DistanceUnitArg::NauticalMiles),
            biscuit_location::DistanceUnit::NauticalMiles
        );
    }
}
