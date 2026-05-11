use std::net::IpAddr;
use std::path::Path;

use maxminddb::{Mmap, Reader, geoip2};

use crate::error::LocationError;
use crate::types::{Coordinates, Location, LocationSource, Place};

/// Wraps a MaxMind GeoLite2-City reader for IP-to-location lookups.
///
/// Uses a memory-mapped reader so lookups avoid copying the full database
/// into process memory. The database file is assumed to be immutable while
/// the process is running; atomic replacement (rename) is safe, in-place
/// modification is not.
#[derive(Debug)]
pub struct IpLookup {
    reader: Reader<Mmap>,
}

impl IpLookup {
    /// Open a MaxMind `.mmdb` file for reading using a memory map.
    ///
    /// ## Errors
    ///
    /// Returns `DatabaseOpen` if the file cannot be opened or parsed.
    ///
    /// ## Safety
    ///
    /// The underlying `Reader::open_mmap` call is `unsafe` because memory
    /// mapping relies on the file being immutable for the lifetime of the
    /// mapping. This library assumes the `.mmdb` file is not truncated or
    /// modified in place while the reader is alive; callers that rotate
    /// databases must use atomic file replacement (e.g. `rename`).
    pub fn open(path: &Path) -> crate::Result<Self> {
        // SAFETY: The MaxMind DB is read-only during the process lifetime.
        // See the type-level doc comment for the required caller contract.
        let reader = unsafe { Reader::open_mmap(path) }.map_err(|e| {
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
            .ok_or(LocationError::IpNotFound(ip))?;

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

    let region_code = subdivision
        .as_ref()
        .and_then(|s| s.iso_code)
        .map(|s| s.to_string());

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
    use maxminddb::geoip2::{self, city};

    fn localhost_v4() -> IpAddr {
        "127.0.0.1".parse().unwrap()
    }

    fn localhost_v6() -> IpAddr {
        "::1".parse().unwrap()
    }

    /// Build a `geoip2::City` record with the given lat/lon and no other data.
    fn city_with_coords(lat: Option<f64>, lon: Option<f64>) -> geoip2::City<'static> {
        geoip2::City {
            location: city::Location {
                latitude: lat,
                longitude: lon,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn mapping_returns_ip_not_found_when_coordinates_missing() {
        let ip = localhost_v4();
        let record = city_with_coords(None, None);
        let err = city_to_location(record, ip).unwrap_err();
        assert!(matches!(err, LocationError::IpNotFound(_)));
    }

    #[test]
    fn mapping_returns_ip_not_found_when_only_latitude_present() {
        let ip = localhost_v4();
        let record = city_with_coords(Some(34.0), None);
        assert!(matches!(
            city_to_location(record, ip).unwrap_err(),
            LocationError::IpNotFound(_)
        ));
    }

    #[test]
    fn mapping_returns_ip_not_found_when_only_longitude_present() {
        let ip = localhost_v4();
        let record = city_with_coords(None, Some(-118.0));
        assert!(matches!(
            city_to_location(record, ip).unwrap_err(),
            LocationError::IpNotFound(_)
        ));
    }

    #[test]
    fn mapping_tolerates_sparse_record_with_only_coordinates() {
        let ip = localhost_v4();
        let record = city_with_coords(Some(34.0522), Some(-118.2437));
        let location = city_to_location(record, ip).unwrap();
        assert_eq!(location.coordinates.latitude, 34.0522);
        assert_eq!(location.coordinates.longitude, -118.2437);
        assert_eq!(location.source, LocationSource::Ip { ip });
        let place = location.place.expect("place should always be populated");
        assert!(place.city.is_none());
        assert!(place.region.is_none());
        assert!(place.region_code.is_none());
        assert!(place.country.is_none());
        assert!(place.country_code.is_none());
        assert!(place.postal_code.is_none());
        assert!(place.timezone.is_none());
    }

    #[test]
    fn mapping_populates_all_fields_when_present() {
        let ip = localhost_v4();
        let record = geoip2::City {
            city: city::City {
                names: geoip2::Names {
                    english: Some("Los Angeles"),
                    ..Default::default()
                },
                ..Default::default()
            },
            country: city::Country {
                iso_code: Some("US"),
                names: geoip2::Names {
                    english: Some("United States"),
                    ..Default::default()
                },
                ..Default::default()
            },
            subdivisions: vec![city::Subdivision {
                iso_code: Some("CA"),
                names: geoip2::Names {
                    english: Some("California"),
                    ..Default::default()
                },
                ..Default::default()
            }],
            location: city::Location {
                latitude: Some(34.0522),
                longitude: Some(-118.2437),
                time_zone: Some("America/Los_Angeles"),
                ..Default::default()
            },
            postal: city::Postal {
                code: Some("90012"),
            },
            ..Default::default()
        };

        let location = city_to_location(record, ip).unwrap();
        let place = location.place.unwrap();
        assert_eq!(place.city.as_deref(), Some("Los Angeles"));
        assert_eq!(place.region.as_deref(), Some("California"));
        assert_eq!(place.region_code.as_deref(), Some("CA"));
        assert_eq!(place.country.as_deref(), Some("United States"));
        assert_eq!(place.country_code.as_deref(), Some("US"));
        assert_eq!(place.postal_code.as_deref(), Some("90012"));
        assert_eq!(place.timezone.as_deref(), Some("America/Los_Angeles"));
    }

    #[test]
    fn mapping_prefers_first_subdivision() {
        // Oxford, UK has England first, Oxfordshire second. We should pick
        // England (the largest subdivision).
        let ip = localhost_v4();
        let record = geoip2::City {
            subdivisions: vec![
                city::Subdivision {
                    iso_code: Some("ENG"),
                    names: geoip2::Names {
                        english: Some("England"),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                city::Subdivision {
                    iso_code: Some("OXF"),
                    names: geoip2::Names {
                        english: Some("Oxfordshire"),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            location: city::Location {
                latitude: Some(51.75),
                longitude: Some(-1.26),
                ..Default::default()
            },
            ..Default::default()
        };
        let place = city_to_location(record, ip).unwrap().place.unwrap();
        assert_eq!(place.region.as_deref(), Some("England"));
        assert_eq!(place.region_code.as_deref(), Some("ENG"));
    }

    #[test]
    fn mapping_preserves_ipv6_in_source() {
        let ip = localhost_v6();
        let record = city_with_coords(Some(0.0), Some(0.0));
        let location = city_to_location(record, ip).unwrap();
        assert_eq!(location.source, LocationSource::Ip { ip });
    }

    #[test]
    fn mapping_rejects_out_of_range_coordinates() {
        // Guard: if a database ever contains invalid coords, we must surface it.
        let ip = localhost_v4();
        let record = city_with_coords(Some(500.0), Some(0.0));
        assert!(matches!(
            city_to_location(record, ip).unwrap_err(),
            LocationError::InvalidCoordinates { .. }
        ));
    }

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
