use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::config::ReverseGeocodeConfig;
use crate::error::LocationError;
use crate::types::{Coordinates, Location, LocationSource, Place};

/// Reverse geocoder that calls a Nominatim-compatible API.
pub struct ReverseGeocoder {
    client: reqwest::Client,
    config: ReverseGeocodeConfig,
    last_request: Mutex<Option<Instant>>,
}

impl ReverseGeocoder {
    /// Create a new reverse geocoder with the given configuration.
    pub fn new(config: ReverseGeocodeConfig) -> crate::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.timeout)
            .build()
            .map_err(|e| LocationError::Internal(e.to_string()))?;
        Ok(Self {
            client,
            config,
            last_request: Mutex::new(None),
        })
    }

    /// Reverse geocode coordinates into a `Location` with place metadata.
    ///
    /// Enforces a minimum interval between requests to respect Nominatim rate limits.
    pub async fn reverse(&self, coords: &Coordinates) -> crate::Result<Location> {
        self.enforce_rate_limit().await;

        let url = format!(
            "{}reverse?lat={}&lon={}&format=json&addressdetails=1",
            self.config.endpoint, coords.latitude, coords.longitude
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LocationError::ReverseGeocode(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LocationError::ReverseGeocode(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let body: NominatimResponse = resp
            .json()
            .await
            .map_err(|e| LocationError::ReverseGeocode(e.to_string()))?;

        Ok(body.into_location(coords))
    }

    /// Sleep if the previous request was less than `min_interval` ago.
    async fn enforce_rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(prev) = *last {
            let elapsed = prev.elapsed();
            if elapsed < self.config.min_interval {
                tokio::time::sleep(self.config.min_interval - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }
}

/// Raw Nominatim JSON response.
#[derive(Debug, Deserialize)]
struct NominatimResponse {
    #[allow(dead_code)]
    display_name: Option<String>,
    address: Option<NominatimAddress>,
}

#[derive(Debug, Deserialize)]
struct NominatimAddress {
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    state: Option<String>,
    #[serde(rename = "ISO3166-2-lvl4")]
    state_code: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    postcode: Option<String>,
}

impl NominatimResponse {
    fn into_location(self, coords: &Coordinates) -> Location {
        let place = self.address.map(|addr| {
            // Nominatim uses city/town/village for different settlement sizes
            let city = addr.city.or(addr.town).or(addr.village);

            // Extract region code from ISO3166-2 format (e.g., "US-CA" -> "CA")
            let region_code = addr
                .state_code
                .as_deref()
                .and_then(|s| s.split('-').nth(1))
                .map(|s| s.to_string());

            Place {
                city,
                region: addr.state,
                region_code,
                country: addr.country,
                country_code: addr.country_code.map(|c| c.to_uppercase()),
                postal_code: addr.postcode,
                timezone: None, // Nominatim does not include timezone
            }
        });

        Location {
            coordinates: *coords,
            place,
            source: LocationSource::ReverseGeocode,
            accuracy_meters: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use url::Url;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(endpoint: Url) -> ReverseGeocodeConfig {
        ReverseGeocodeConfig {
            endpoint,
            user_agent: "test/0.1.0".to_string(),
            timeout: Duration::from_secs(5),
            min_interval: Duration::from_millis(0), // No rate limit in tests
        }
    }

    const NOMINATIM_LA_RESPONSE: &str = r#"{
        "place_id": 282305978,
        "display_name": "Los Angeles, Los Angeles County, California, United States",
        "address": {
            "city": "Los Angeles",
            "county": "Los Angeles County",
            "state": "California",
            "ISO3166-2-lvl4": "US-CA",
            "country": "United States",
            "country_code": "us",
            "postcode": "90012"
        }
    }"#;

    #[tokio::test]
    async fn reverse_maps_nominatim_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .and(query_param("format", "json"))
            .and(query_param("addressdetails", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(NOMINATIM_LA_RESPONSE))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();

        assert_eq!(location.source, LocationSource::ReverseGeocode);
        let place = location.place.unwrap();
        assert_eq!(place.city.as_deref(), Some("Los Angeles"));
        assert_eq!(place.region.as_deref(), Some("California"));
        assert_eq!(place.region_code.as_deref(), Some("CA"));
        assert_eq!(place.country.as_deref(), Some("United States"));
        assert_eq!(place.country_code.as_deref(), Some("US"));
        assert_eq!(place.postal_code.as_deref(), Some("90012"));
    }

    #[tokio::test]
    async fn reverse_handles_town_instead_of_city() {
        let mock_server = MockServer::start().await;

        let response = r#"{
            "display_name": "Smallville, Kansas, United States",
            "address": {
                "town": "Smallville",
                "state": "Kansas",
                "country": "United States",
                "country_code": "us"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(39.0, -97.0).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();

        let place = location.place.unwrap();
        assert_eq!(place.city.as_deref(), Some("Smallville"));
    }

    #[tokio::test]
    async fn reverse_handles_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let result = geocoder.reverse(&coords).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LocationError::ReverseGeocode(_)));
    }

    #[tokio::test]
    async fn reverse_handles_empty_address() {
        let mock_server = MockServer::start().await;

        let response = r#"{"display_name": "Ocean"}"#;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();

        assert!(location.place.is_none());
    }
}
