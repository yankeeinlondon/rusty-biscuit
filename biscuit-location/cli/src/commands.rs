use std::time::Duration;

use biscuit_location::{Coordinates, LocationConfig, LocationService};
use tracing::{debug, info_span, Instrument};

use crate::args::{Cli, Commands};
use crate::output::{self, OutputMode};

/// Execute the CLI command and write output to stdout.
///
/// Completions are handled by the caller in `main.rs` before this runs —
/// they must execute without constructing a `LocationService` because they
/// do not depend on any runtime configuration.
pub async fn run(cli: Cli, mode: OutputMode) -> color_eyre::Result<()> {
    debug!(
        ?cli.command,
        quiet = cli.quiet,
        verbose = cli.verbose,
        ?mode,
        "dispatching command"
    );

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
            let span = info_span!("gps");
            let result = svc.gps().instrument(span).await?;
            match result {
                Some(location) => emit_location(&svc, &location, cli.maps, cli.quiet, mode)?,
                None => emit(&output::render_no_gps(mode), cli.quiet),
            }
        }

        Commands::Ip { address } => {
            let span = info_span!("ip", address = %address);
            let location = span.in_scope(|| svc.ip(address))?;
            emit_location(&svc, &location, cli.maps, cli.quiet, mode)?;
        }

        Commands::Reverse { lat, lon } => {
            let coords = Coordinates::new(lat, lon)?;
            let span = info_span!("reverse", %lat, %lon);
            let location = svc.reverse(coords).instrument(span).await?;
            emit_location(&svc, &location, cli.maps, cli.quiet, mode)?;
        }

        Commands::Distance { from, to, unit } => {
            let span = info_span!("distance", ?from, ?to);
            let (from_loc, to_loc) = async {
                let from_loc = svc.resolve_input(from).await?;
                let to_loc = svc.resolve_input(to).await?;
                color_eyre::Result::<_>::Ok((from_loc, to_loc))
            }
            .instrument(span)
            .await?;

            let unit = biscuit_location::DistanceUnit::from(unit);
            let value = svc.distance(from_loc.coordinates, to_loc.coordinates, unit)?;
            emit(&output::render_distance(value, unit, mode), cli.quiet);
        }

        Commands::Completions { .. } => {
            // Handled in main.rs before this function runs.
            unreachable!("completions are handled in main before dispatch");
        }
    }

    Ok(())
}

fn emit_location(
    svc: &LocationService,
    location: &biscuit_location::Location,
    include_maps: bool,
    quiet: bool,
    mode: OutputMode,
) -> color_eyre::Result<()> {
    let maps_url = if include_maps {
        Some(svc.google_maps_url(location.coordinates)?.to_string())
    } else {
        None
    };
    emit(&output::render_location(location, maps_url, mode), quiet);
    Ok(())
}

fn emit(text: &str, quiet: bool) {
    if quiet {
        // `--quiet` suppresses data output; keep stderr clean too.
        return;
    }
    println!("{text}");
}
