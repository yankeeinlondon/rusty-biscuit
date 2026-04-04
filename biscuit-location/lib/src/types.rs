use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

use crate::error::LocationError;

/// A validated latitude/longitude pair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinates {
    /// Create validated coordinates.
    ///
    /// ## Errors
    ///
    /// Returns `InvalidCoordinates` if latitude is outside `[-90, 90]`
    /// or longitude is outside `[-180, 180]`.
    pub fn new(latitude: f64, longitude: f64) -> crate::Result<Self> {
        if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
            return Err(LocationError::InvalidCoordinates {
                latitude,
                longitude,
            });
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }
}

impl fmt::Display for Coordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.latitude, self.longitude)
    }
}

/// City-level place metadata from geocoding or IP lookup.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub city: Option<String>,
    pub region: Option<String>,
    pub region_code: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub postal_code: Option<String>,
    pub timezone: Option<String>,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<&str> = [
            self.city.as_deref(),
            self.region.as_deref(),
            self.country.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        write!(f, "{}", parts.join(", "))
    }
}

/// How a location was obtained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationSource {
    Gps,
    Ip { ip: IpAddr },
    ReverseGeocode,
    CoordinatesLiteral,
}

/// A resolved geographic location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub coordinates: Coordinates,
    pub place: Option<Place>,
    pub source: LocationSource,
    pub accuracy_meters: Option<f64>,
}

/// A distance stored canonically in meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Distance {
    pub meters: f64,
}

/// Units for displaying distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceUnit {
    Meters,
    Kilometers,
    Miles,
    NauticalMiles,
}

impl Distance {
    /// Convert the canonical meter value to the requested unit.
    pub fn as_unit(&self, unit: DistanceUnit) -> f64 {
        match unit {
            DistanceUnit::Meters => self.meters,
            DistanceUnit::Kilometers => self.meters / 1_000.0,
            DistanceUnit::Miles => self.meters / 1_609.344,
            DistanceUnit::NauticalMiles => self.meters / 1_852.0,
        }
    }
}

/// A user-supplied location reference (CLI input grammar).
///
/// Parses from strings:
/// - `"gps"` — host GPS fix
/// - `"ip:1.2.3.4"` — IP lookup
/// - `"37.77,-122.41"` — coordinate literal
#[derive(Debug, Clone, PartialEq)]
pub enum LocationInput {
    Gps,
    Ip(IpAddr),
    Coordinates(Coordinates),
}

impl FromStr for LocationInput {
    type Err = LocationError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("gps") {
            return Ok(Self::Gps);
        }
        if let Some(ip_str) = s.strip_prefix("ip:") {
            let ip: IpAddr = ip_str
                .parse()
                .map_err(|_| LocationError::InvalidLocationInput(s.to_string()))?;
            return Ok(Self::Ip(ip));
        }
        // Try lat,lon
        if let Some((lat_str, lon_str)) = s.split_once(',') {
            let lat: f64 = lat_str
                .trim()
                .parse()
                .map_err(|_| LocationError::InvalidLocationInput(s.to_string()))?;
            let lon: f64 = lon_str
                .trim()
                .parse()
                .map_err(|_| LocationError::InvalidLocationInput(s.to_string()))?;
            let coords = Coordinates::new(lat, lon)?;
            return Ok(Self::Coordinates(coords));
        }
        Err(LocationError::InvalidLocationInput(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_coordinates() {
        let c = Coordinates::new(37.7749, -122.4194).unwrap();
        assert_eq!(c.latitude, 37.7749);
        assert_eq!(c.longitude, -122.4194);
    }

    #[test]
    fn boundary_coordinates() {
        assert!(Coordinates::new(90.0, 180.0).is_ok());
        assert!(Coordinates::new(-90.0, -180.0).is_ok());
        assert!(Coordinates::new(0.0, 0.0).is_ok());
    }

    #[test]
    fn invalid_latitude() {
        assert!(Coordinates::new(91.0, 0.0).is_err());
        assert!(Coordinates::new(-91.0, 0.0).is_err());
    }

    #[test]
    fn invalid_longitude() {
        assert!(Coordinates::new(0.0, 181.0).is_err());
        assert!(Coordinates::new(0.0, -181.0).is_err());
    }

    #[test]
    fn distance_unit_conversion() {
        let d = Distance { meters: 1_609.344 };
        assert!((d.as_unit(DistanceUnit::Miles) - 1.0).abs() < 1e-9);
        assert!((d.as_unit(DistanceUnit::Kilometers) - 1.609344).abs() < 1e-6);
        assert!((d.as_unit(DistanceUnit::Meters) - 1_609.344).abs() < 1e-9);
    }

    #[test]
    fn distance_nautical_miles() {
        let d = Distance { meters: 1_852.0 };
        assert!((d.as_unit(DistanceUnit::NauticalMiles) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn location_input_gps() {
        assert_eq!("gps".parse::<LocationInput>().unwrap(), LocationInput::Gps);
        assert_eq!("GPS".parse::<LocationInput>().unwrap(), LocationInput::Gps);
    }

    #[test]
    fn location_input_ip() {
        let input: LocationInput = "ip:8.8.8.8".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Ip("8.8.8.8".parse().unwrap())
        );
    }

    #[test]
    fn location_input_ip_v6() {
        let input: LocationInput = "ip:2001:4860:4860::8888".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Ip("2001:4860:4860::8888".parse().unwrap())
        );
    }

    #[test]
    fn location_input_coordinates() {
        let input: LocationInput = "37.7749,-122.4194".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Coordinates(Coordinates::new(37.7749, -122.4194).unwrap())
        );
    }

    #[test]
    fn location_input_coordinates_with_spaces() {
        let input: LocationInput = "37.7749, -122.4194".parse().unwrap();
        assert_eq!(
            input,
            LocationInput::Coordinates(Coordinates::new(37.7749, -122.4194).unwrap())
        );
    }

    #[test]
    fn location_input_invalid() {
        assert!("not-a-location".parse::<LocationInput>().is_err());
        assert!("ip:not-an-ip".parse::<LocationInput>().is_err());
        assert!("999,0".parse::<LocationInput>().is_err()); // invalid latitude
    }

    #[test]
    fn place_display() {
        let place = Place {
            city: Some("Los Angeles".into()),
            region: Some("California".into()),
            country: Some("United States".into()),
            ..Default::default()
        };
        assert_eq!(place.to_string(), "Los Angeles, California, United States");
    }

    #[test]
    fn place_display_partial() {
        let place = Place {
            country: Some("Japan".into()),
            ..Default::default()
        };
        assert_eq!(place.to_string(), "Japan");
    }

    #[test]
    fn coordinates_display() {
        let c = Coordinates::new(34.0522, -118.2437).unwrap();
        assert_eq!(c.to_string(), "34.0522, -118.2437");
    }
}
