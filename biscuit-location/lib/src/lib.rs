//! Location services for the biscuit ecosystem.
//!
//! Provides host GPS lookup, IP-to-location resolution, reverse geocoding,
//! distance calculation, and Google Maps link generation.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use biscuit_location::{LocationConfig, LocationService, Coordinates, DistanceUnit};
//!
//! # #[tokio::main]
//! # async fn main() -> biscuit_location::Result<()> {
//! let svc = LocationService::new(LocationConfig::default()).await?;
//!
//! // Distance between two points
//! let la = Coordinates::new(34.0522, -118.2437)?;
//! let nyc = Coordinates::new(40.7128, -74.0060)?;
//! let km = svc.distance(la, nyc, DistanceUnit::Kilometers)?;
//! println!("{km:.1} km");
//!
//! // Google Maps link
//! let url = svc.google_maps_url(la)?;
//! println!("{url}");
//! # Ok(())
//! # }
//! ```

mod config;
mod distance;
mod error;
mod gps;
mod ip;
mod maps;
mod maxmind;
#[cfg(feature = "reverse")]
mod reverse;
mod service;
mod types;

pub use config::{LocationConfig, ReverseGeocodeConfig, resolve_maxmind_path};
pub use distance::distance;
pub use error::{LocationError, Result};
pub use maps::google_maps_url;
pub use maxmind::{
    can_auto_download, download_database, download_database_to, resolve_credentials,
};
pub use service::LocationService;
pub use types::{
    Coordinates, Distance, DistanceUnit, Location, LocationInput, LocationSource, Place,
};
