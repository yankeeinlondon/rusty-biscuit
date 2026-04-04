use std::time::Duration;

use biscuit_location::{Coordinates, LocationConfig, LocationInput, LocationService};

use crate::args::{Cli, Commands};
use crate::output;

/// Execute the CLI command and print output.
pub async fn run(cli: Cli) -> color_eyre::Result<()> {
    let gps_timeout = match &cli.command {
        Commands::Gps { timeout } => Duration::from_secs(*timeout),
        _ => Duration::from_secs(10),
    };

    let config = LocationConfig {
        maxmind_db_path: cli.db_path,
        gps_timeout,
        ..LocationConfig::default()
    };

    let svc = LocationService::new(config)?;

    match cli.command {
        Commands::Gps { .. } => {
            match svc.gps().await? {
                Some(location) => {
                    let maps_url = if cli.maps {
                        Some(svc.google_maps_url(location.coordinates)?.to_string())
                    } else {
                        None
                    };
                    println!(
                        "{}",
                        output::format_location(&location, maps_url.as_deref())
                    );
                }
                None => {
                    println!("{}", output::format_no_gps());
                }
            }
        }

        Commands::Ip { address } => {
            let location = svc.ip(address)?;
            let maps_url = if cli.maps {
                Some(svc.google_maps_url(location.coordinates)?.to_string())
            } else {
                None
            };
            println!(
                "{}",
                output::format_location(&location, maps_url.as_deref())
            );
        }

        Commands::Reverse { lat, lon } => {
            let coords = Coordinates::new(lat, lon)?;
            let location = svc.reverse(coords).await?;
            let maps_url = if cli.maps {
                Some(svc.google_maps_url(location.coordinates)?.to_string())
            } else {
                None
            };
            println!(
                "{}",
                output::format_location(&location, maps_url.as_deref())
            );
        }

        Commands::Distance { from, to, unit } => {
            let from_input: LocationInput = from.parse()?;
            let to_input: LocationInput = to.parse()?;
            let from_loc = svc.resolve_input(from_input).await?;
            let to_loc = svc.resolve_input(to_input).await?;
            let value = svc.distance(from_loc.coordinates, to_loc.coordinates, unit)?;
            println!("{}", output::format_distance(value, unit));
        }
    }

    Ok(())
}
