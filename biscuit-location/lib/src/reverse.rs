use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::Instant;
use url::Url;

use crate::config::ReverseGeocodeConfig;
use crate::error::LocationError;
use crate::types::{Coordinates, Location, LocationSource, Place};

/// Build the Nominatim reverse-geocoding URL for the given endpoint and coordinates.
///
/// Joins a `reverse` path segment onto the configured endpoint and appends the
/// required query parameters using percent-encoding. Tolerates endpoints with
/// or without a trailing slash, extra path segments, or pre-existing query
/// state.
///
/// ## Errors
///
/// Returns [`LocationError::ReverseGeocode`] when the endpoint cannot accept
/// relative joins (e.g. an endpoint that lacks a path or uses an incompatible
/// scheme).
fn build_reverse_url(endpoint: &Url, coords: &Coordinates) -> crate::Result<Url> {
    // Ensure the endpoint has a trailing slash so the `reverse` segment is
    // joined as a child path, not a sibling. `Url::join` treats an endpoint
    // like `http://host/api` as having a filename `api`, and joining `reverse`
    // onto that replaces `api` instead of appending to it.
    let base = if endpoint.path().ends_with('/') {
        endpoint.clone()
    } else {
        let mut with_slash = endpoint.clone();
        with_slash.set_path(&format!("{}/", endpoint.path()));
        with_slash
    };

    let mut url = base
        .join("reverse")
        .map_err(|e| LocationError::ReverseGeocode(format!("invalid endpoint URL: {e}")))?;

    url.query_pairs_mut()
        .append_pair("lat", &coords.latitude.to_string())
        .append_pair("lon", &coords.longitude.to_string())
        .append_pair("format", "json")
        .append_pair("addressdetails", "1");

    Ok(url)
}

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

        let url = build_reverse_url(&self.config.endpoint, coords)?;

        let resp = self
            .client
            .get(url)
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

    // ---- URL building ----

    #[test]
    fn build_url_trailing_slash_endpoint() {
        let endpoint = Url::parse("https://example.com/").unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let url = build_reverse_url(&endpoint, &coords).unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
        assert_eq!(url.path(), "/reverse");
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query.get("lat").map(String::as_str), Some("34.0522"));
        assert_eq!(query.get("lon").map(String::as_str), Some("-118.2437"));
        assert_eq!(query.get("format").map(String::as_str), Some("json"));
        assert_eq!(query.get("addressdetails").map(String::as_str), Some("1"));
    }

    #[test]
    fn build_url_no_trailing_slash_endpoint() {
        let endpoint = Url::parse("https://example.com/nominatim").unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let url = build_reverse_url(&endpoint, &coords).unwrap();
        assert_eq!(url.path(), "/nominatim/reverse");
    }

    #[test]
    fn build_url_subpath_with_trailing_slash_endpoint() {
        let endpoint = Url::parse("https://example.com/api/").unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let url = build_reverse_url(&endpoint, &coords).unwrap();
        assert_eq!(url.path(), "/api/reverse");
    }

    #[test]
    fn build_url_escapes_special_characters() {
        // Coordinates that produce an uncommon but valid numeric string.
        let endpoint = Url::parse("https://example.com/").unwrap();
        let coords = Coordinates::new(-0.5, 180.0).unwrap();
        let url = build_reverse_url(&endpoint, &coords).unwrap();
        let query = url.query().unwrap();
        assert!(query.contains("lat=-0.5"));
        assert!(query.contains("lon=180"));
    }

    #[tokio::test]
    async fn reverse_with_endpoint_missing_trailing_slash_works() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(NOMINATIM_LA_RESPONSE))
            .mount(&mock_server)
            .await;

        // Endpoint with a non-slash-terminated subpath:
        let endpoint = Url::parse(&format!("{}/api", mock_server.uri())).unwrap();
        let geocoder = ReverseGeocoder::new(test_config(endpoint)).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let location = geocoder.reverse(&coords).await.unwrap();
        assert_eq!(location.source, LocationSource::ReverseGeocode);
    }

    #[tokio::test]
    async fn reverse_respects_min_interval_between_requests() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(NOMINATIM_LA_RESPONSE))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let mut cfg = test_config(endpoint);
        cfg.min_interval = Duration::from_millis(200);
        let geocoder = ReverseGeocoder::new(cfg).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();

        let start = std::time::Instant::now();
        geocoder.reverse(&coords).await.unwrap();
        geocoder.reverse(&coords).await.unwrap();
        let elapsed = start.elapsed();

        // Two requests with a 200ms min_interval must take at least ~200ms.
        assert!(
            elapsed >= Duration::from_millis(180),
            "rate limiter did not delay second request, elapsed: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn reverse_sends_configured_user_agent() {
        use wiremock::matchers::header;

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .and(header("user-agent", "where-test/9.9.9"))
            .respond_with(ResponseTemplate::new(200).set_body_string(NOMINATIM_LA_RESPONSE))
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let mut cfg = test_config(endpoint);
        cfg.user_agent = "where-test/9.9.9".to_string();
        let geocoder = ReverseGeocoder::new(cfg).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();

        // Mock only matches when user-agent header is exactly the configured value.
        let result = geocoder.reverse(&coords).await;
        assert!(
            result.is_ok(),
            "request did not include the configured user-agent: {result:?}"
        );
    }

    #[tokio::test]
    async fn reverse_respects_configured_timeout() {
        use std::time::Duration as StdDuration;

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/reverse"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(NOMINATIM_LA_RESPONSE)
                    .set_delay(StdDuration::from_millis(500)),
            )
            .mount(&mock_server)
            .await;

        let endpoint = Url::parse(&format!("{}/", mock_server.uri())).unwrap();
        let mut cfg = test_config(endpoint);
        cfg.timeout = Duration::from_millis(100);
        let geocoder = ReverseGeocoder::new(cfg).unwrap();
        let coords = Coordinates::new(0.0, 0.0).unwrap();

        let result = geocoder.reverse(&coords).await;
        assert!(result.is_err(), "expected timeout error, got: {result:?}");
        assert!(matches!(
            result.unwrap_err(),
            LocationError::ReverseGeocode(_)
        ));
    }

    /// Live integration test confirming the default HTTPS endpoint can be reached.
    ///
    /// Guarded by `BISCUIT_LOCATION_RUN_LIVE_TESTS` so it is opt-in and never
    /// runs in CI by default. Its purpose is to catch misconfigurations where
    /// `reqwest` is built without a TLS backend — the build itself succeeds,
    /// but any HTTPS request at runtime fails with a transport error.
    #[tokio::test]
    async fn reverse_live_default_endpoint_accepts_https() {
        if std::env::var("BISCUIT_LOCATION_RUN_LIVE_TESTS").is_err() {
            eprintln!("Skipping: BISCUIT_LOCATION_RUN_LIVE_TESTS not set");
            return;
        }
        let config = ReverseGeocodeConfig::default();
        let geocoder = ReverseGeocoder::new(config).unwrap();
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let result = geocoder.reverse(&coords).await;
        assert!(
            result.is_ok(),
            "live HTTPS reverse geocode failed: {result:?}"
        );
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
