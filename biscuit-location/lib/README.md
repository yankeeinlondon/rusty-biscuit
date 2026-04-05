# biscuit-location

Location services library for the biscuit ecosystem.

## Features

- **Host GPS** — one-shot GPS fix from macOS CoreLocation, Windows `Geolocator`, or Linux GeoClue2
- **IP to Location** — resolve IP addresses via local MaxMind GeoLite2 database
- **Reverse Geocoding** — coordinates to city-level place name via Nominatim
- **Distance** — geodesic distance between two points (WGS-84 ellipsoid)
- **Google Maps Link** — generate a Maps URL for any coordinates

## Usage

```rust
use biscuit_location::{LocationConfig, LocationService, Coordinates, DistanceUnit};

let svc = LocationService::new(LocationConfig::default())?;

// Distance
let la = Coordinates::new(34.0522, -118.2437)?;
let nyc = Coordinates::new(40.7128, -74.0060)?;
let km = svc.distance(la, nyc, DistanceUnit::Kilometers)?;

// Google Maps
let url = svc.google_maps_url(la)?;
```

## MaxMind Database

IP lookup requires a GeoLite2-City database file (not bundled):

1. Create a free account at [maxmind.com](https://www.maxmind.com)
2. Download `GeoLite2-City.mmdb`
3. Place it at the OS default path or set `BISCUIT_LOCATION_MAXMIND_DB`

### Path Resolution

1. Explicit `LocationConfig.maxmind_db_path`
2. `BISCUIT_LOCATION_MAXMIND_DB` environment variable
3. OS default:
   - macOS: `~/Library/Application Support/biscuit-location/GeoLite2-City.mmdb`
   - Linux: `~/.local/share/biscuit-location/GeoLite2-City.mmdb`
   - Windows: `%LOCALAPPDATA%\biscuit-location\GeoLite2-City.mmdb`
