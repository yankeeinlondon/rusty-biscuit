//! Location services for the biscuit ecosystem.
//!
//! Provides host GPS lookup, IP-to-location resolution, reverse geocoding,
//! distance calculation, and Google Maps link generation.

mod config;
mod error;
mod types;

pub use config::{LocationConfig, ReverseGeocodeConfig, resolve_maxmind_path};
pub use error::{LocationError, Result};
pub use types::*;
