use std::net::IpAddr;

use url::Url;

use crate::config::{LocationConfig, resolve_maxmind_path};
use crate::distance;
use crate::error::LocationError;
use crate::gps;
use crate::ip::IpLookup;
use crate::maps;
use crate::types::{Coordinates, DistanceUnit, Location, LocationInput, LocationSource};

/// Configured facade for all location services.
///
/// Holds the MaxMind reader, reverse geocoder configuration, and GPS settings.
/// Construct via `LocationService::new()` and use the methods to perform lookups.
pub struct LocationService {
    ip_lookup: Option<IpLookup>,
    config: LocationConfig,
    #[cfg(feature = "reverse")]
    reverse_geocoder: Option<crate::reverse::ReverseGeocoder>,
}

impl LocationService {
    /// Create a new location service from configuration.
    ///
    /// If a MaxMind database path resolves successfully, the IP lookup reader
    /// is opened eagerly. If the path does not exist, IP lookups will return
    /// `DatabasePathNotFound` at call time.
    pub fn new(config: LocationConfig) -> crate::Result<Self> {
        let db_path = resolve_maxmind_path(config.maxmind_db_path.as_ref());
        let ip_lookup = match db_path {
            Some(ref p) if p.exists() => Some(IpLookup::open(p)?),
            _ => None,
        };

        #[cfg(feature = "reverse")]
        let reverse_geocoder = Some(crate::reverse::ReverseGeocoder::new(
            config.reverse.clone(),
        )?);

        Ok(Self {
            ip_lookup,
            config,
            #[cfg(feature = "reverse")]
            reverse_geocoder,
        })
    }

    /// Request a one-shot GPS fix from the host device.
    pub async fn gps(&self) -> crate::Result<Option<Location>> {
        gps::current_fix(self.config.gps_timeout).await
    }

    /// Look up the geographic location of an IP address.
    ///
    /// ## Errors
    ///
    /// Returns `DatabasePathNotFound` if no MaxMind database was configured or found.
    pub fn ip(&self, ip: IpAddr) -> crate::Result<Location> {
        match &self.ip_lookup {
            Some(lookup) => lookup.lookup(ip),
            None => Err(LocationError::DatabasePathNotFound(
                resolve_maxmind_path(self.config.maxmind_db_path.as_ref())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "no path configured".to_string()),
            )),
        }
    }

    /// Reverse geocode coordinates into a location with place metadata.
    #[cfg(feature = "reverse")]
    pub async fn reverse(&self, coordinates: Coordinates) -> crate::Result<Location> {
        match &self.reverse_geocoder {
            Some(geocoder) => geocoder.reverse(&coordinates).await,
            None => Err(LocationError::Internal(
                "reverse geocoder not configured".to_string(),
            )),
        }
    }

    /// Calculate the distance between two coordinates.
    pub fn distance(
        &self,
        from: Coordinates,
        to: Coordinates,
        unit: DistanceUnit,
    ) -> crate::Result<f64> {
        let d = distance::distance(&from, &to)?;
        Ok(d.as_unit(unit))
    }

    /// Generate a Google Maps URL for the given coordinates.
    pub fn google_maps_url(&self, coordinates: Coordinates) -> crate::Result<Url> {
        maps::google_maps_url(&coordinates)
    }

    /// Resolve a `LocationInput` into a `Location`.
    ///
    /// This dispatches to GPS, IP lookup, or returns a literal coordinate location
    /// depending on the input variant.
    pub async fn resolve_input(&self, input: LocationInput) -> crate::Result<Location> {
        match input {
            LocationInput::Gps => self
                .gps()
                .await?
                .ok_or_else(|| LocationError::Internal("no GPS fix available".to_string())),
            LocationInput::Ip(ip) => self.ip(ip),
            LocationInput::Coordinates(coords) => Ok(Location {
                coordinates: coords,
                place: None,
                source: LocationSource::CoordinatesLiteral,
                accuracy_meters: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_without_maxmind_db() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let result = svc.ip("8.8.8.8".parse().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn service_distance() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let la = Coordinates::new(34.0522, -118.2437).unwrap();
        let nyc = Coordinates::new(40.7128, -74.0060).unwrap();
        let km = svc.distance(la, nyc, DistanceUnit::Kilometers).unwrap();
        assert!(km > 3900.0 && km < 4000.0);
    }

    #[test]
    fn service_google_maps_url() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let url = svc.google_maps_url(coords).unwrap();
        assert!(url.as_str().contains("google.com/maps"));
    }

    #[tokio::test]
    async fn resolve_coordinate_literal() {
        let config = LocationConfig::default();
        let svc = LocationService::new(config).unwrap();
        let coords = Coordinates::new(51.5074, -0.1278).unwrap();
        let loc = svc
            .resolve_input(LocationInput::Coordinates(coords))
            .await
            .unwrap();
        assert_eq!(loc.source, LocationSource::CoordinatesLiteral);
        assert_eq!(loc.coordinates.latitude, 51.5074);
    }
}
