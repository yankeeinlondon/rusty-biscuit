use std::net::IpAddr;
use std::path::Path;

use maxminddb::{geoip2, Reader};

use crate::error::LocationError;
use crate::types::{Coordinates, Location, LocationSource, Place};

/// Wraps a MaxMind GeoLite2-City reader for IP-to-location lookups.
#[derive(Debug)]
pub struct IpLookup {
    reader: Reader<Vec<u8>>,
}

impl IpLookup {
    /// Open a MaxMind `.mmdb` file for reading.
    ///
    /// ## Errors
    ///
    /// Returns `DatabaseOpen` if the file cannot be read or parsed.
    pub fn open(path: &Path) -> crate::Result<Self> {
        let reader = Reader::open_readfile(path).map_err(|e| {
            LocationError::DatabaseOpen(format!("{path}: {e}", path = path.display()))
        })?;
        Ok(Self { reader })
    }

    /// Look up an IP address and return a `Location` with city-level place data.
    ///
    /// ## Errors
    ///
    /// Returns `IpNotFound` if the database has no record for the address.
    /// Returns `IpLookup` for other database errors.
    pub fn lookup(&self, ip: IpAddr) -> crate::Result<Location> {
        let result = self
            .reader
            .lookup(ip)
            .map_err(|e| LocationError::IpLookup(e.to_string()))?;

        if !result.has_data() {
            return Err(LocationError::IpNotFound(ip));
        }

        let city: geoip2::City = result
            .decode()
            .map_err(|e| LocationError::IpLookup(e.to_string()))?
            .ok_or_else(|| LocationError::IpNotFound(ip))?;

        city_to_location(city, ip)
    }
}

/// Map a MaxMind City record into our domain `Location`.
///
/// Tolerates missing fields because GeoLite2 entries are often sparse.
fn city_to_location(city: geoip2::City, ip: IpAddr) -> crate::Result<Location> {
    let lat = city.location.latitude;
    let lon = city.location.longitude;

    let (latitude, longitude) = match (lat, lon) {
        (Some(lat), Some(lon)) => (lat, lon),
        _ => return Err(LocationError::IpNotFound(ip)),
    };

    let coordinates = Coordinates::new(latitude, longitude)?;

    let city_name = city.city.names.english.map(|s| s.to_string());

    let subdivision = city.subdivisions.into_iter().next();

    let region = subdivision
        .as_ref()
        .and_then(|s| s.names.english)
        .map(|s| s.to_string());

    let region_code = subdivision.as_ref().and_then(|s| s.iso_code).map(|s| s.to_string());

    let country = city.country.names.english.map(|s| s.to_string());

    let country_code = city.country.iso_code.map(|s| s.to_string());

    let postal_code = city.postal.code.map(|s| s.to_string());

    let timezone = city.location.time_zone.map(|s| s.to_string());

    let place = Place {
        city: city_name,
        region,
        region_code,
        country,
        country_code,
        postal_code,
        timezone,
    };

    Ok(Location {
        coordinates,
        place: Some(place),
        source: LocationSource::Ip { ip },
        accuracy_meters: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test that requires a real MaxMind database.
    /// Set `BISCUIT_LOCATION_TEST_MMDB` to the path of a GeoLite2-City.mmdb file.
    #[test]
    fn lookup_real_db() {
        let db_path = match std::env::var("BISCUIT_LOCATION_TEST_MMDB") {
            Ok(p) => p,
            Err(_) => {
                eprintln!("Skipping: BISCUIT_LOCATION_TEST_MMDB not set");
                return;
            }
        };
        let lookup = IpLookup::open(Path::new(&db_path)).unwrap();
        let loc = lookup.lookup("8.8.8.8".parse().unwrap()).unwrap();
        assert!(loc.place.is_some());
        let place = loc.place.unwrap();
        assert_eq!(place.country_code.as_deref(), Some("US"));
    }

    #[test]
    fn open_nonexistent_db() {
        let result = IpLookup::open(Path::new("/nonexistent/path.mmdb"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LocationError::DatabaseOpen(_)));
    }
}
