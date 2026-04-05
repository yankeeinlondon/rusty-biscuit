use url::Url;

use crate::types::Coordinates;

/// Generate a Google Maps URL that shows the given coordinates.
///
/// Uses the Maps search URL format which requires no API key:
/// `https://www.google.com/maps/search/?api=1&query={lat},{lon}`
pub fn google_maps_url(coords: &Coordinates) -> crate::Result<Url> {
    let mut url = Url::parse("https://www.google.com/maps/search/")
        .map_err(|e| crate::LocationError::GoogleMapsUrl(e.to_string()))?;
    url.query_pairs_mut()
        .append_pair("api", "1")
        .append_pair("query", &format!("{},{}", coords.latitude, coords.longitude));
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_url() {
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let url = google_maps_url(&coords).unwrap();
        assert_eq!(
            url.as_str(),
            "https://www.google.com/maps/search/?api=1&query=34.0522%2C-118.2437"
        );
    }

    #[test]
    fn url_contains_coordinates() {
        let coords = Coordinates::new(51.5074, -0.1278).unwrap();
        let url = google_maps_url(&coords).unwrap();
        let url_str = url.as_str();
        assert!(url_str.contains("51.5074"));
        assert!(url_str.contains("-0.1278"));
    }

    #[test]
    fn url_starts_with_google_maps() {
        let coords = Coordinates::new(0.0, 0.0).unwrap();
        let url = google_maps_url(&coords).unwrap();
        assert!(url.as_str().starts_with("https://www.google.com/maps/"));
    }
}
