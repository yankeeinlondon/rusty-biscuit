use biscuit_location::{DistanceUnit, Location};

/// Format a location for human-readable output.
pub fn format_location(location: &Location, maps_url: Option<&str>) -> String {
    let mut lines = Vec::new();

    // Line 1: best available place string, else coordinates
    match &location.place {
        Some(place) if !place.to_string().is_empty() => {
            lines.push(place.to_string());
        }
        _ => {}
    }

    // Line 2: raw coordinates
    lines.push(format!("{}", location.coordinates));

    // Line 3: source
    let source = match &location.source {
        biscuit_location::LocationSource::Gps => "GPS".to_string(),
        biscuit_location::LocationSource::Ip { ip } => format!("IP: {ip}"),
        biscuit_location::LocationSource::ReverseGeocode => "Reverse geocode".to_string(),
        biscuit_location::LocationSource::CoordinatesLiteral => "Coordinates".to_string(),
    };
    lines.push(format!("Source: {source}"));

    // Optional accuracy
    if let Some(acc) = location.accuracy_meters {
        lines.push(format!("Accuracy: {acc:.0} m"));
    }

    // Optional maps link
    if let Some(url) = maps_url {
        lines.push(format!("Maps: {url}"));
    }

    lines.join("\n")
}

/// Format a distance value for human-readable output.
pub fn format_distance(value: f64, unit: DistanceUnit) -> String {
    let unit_str = match unit {
        DistanceUnit::Meters => "meters",
        DistanceUnit::Kilometers => "km",
        DistanceUnit::Miles => "miles",
        DistanceUnit::NauticalMiles => "nautical miles",
    };
    format!("{value:.2} {unit_str}")
}

/// Format a "no GPS fix" message.
pub fn format_no_gps() -> &'static str {
    "No GPS fix available."
}
