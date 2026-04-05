use std::net::IpAddr;

/// Alias for `Result<T, LocationError>`.
pub type Result<T> = std::result::Result<T, LocationError>;

/// Errors produced by biscuit-location operations.
#[derive(Debug, thiserror::Error)]
pub enum LocationError {
    #[error("invalid coordinates: latitude must be in [-90, 90] and longitude in [-180, 180], got ({latitude}, {longitude})")]
    InvalidCoordinates { latitude: f64, longitude: f64 },

    #[error("invalid location input: {0}")]
    InvalidLocationInput(String),

    #[error("MaxMind database not found at {0}")]
    DatabasePathNotFound(String),

    #[error("failed to open MaxMind database: {0}")]
    DatabaseOpen(String),

    #[error("IP lookup failed: {0}")]
    IpLookup(String),

    #[error("no location data found for IP {0}")]
    IpNotFound(IpAddr),

    #[error("reverse geocoding failed: {0}")]
    ReverseGeocode(String),

    #[error("failed to build Google Maps URL: {0}")]
    GoogleMapsUrl(String),

    #[error("GPS not supported on this platform")]
    UnsupportedPlatform,

    #[error("no GPS fix available")]
    NoGpsFix,

    #[error("internal error: {0}")]
    Internal(String),
}
