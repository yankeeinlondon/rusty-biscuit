# biscuit-location

## Goals

Build a new package area called `biscuit-location` (lib + CLI) providing location-based services:

1. **Host GPS** — query the host device's GPS hardware for current coordinates
2. **IP to Location** — resolve an IP address to a geographic location using a local MaxMind database
3. **Reverse Geocoding** — convert bare lat/lon coordinates into a recognizable place (city, town, state, etc.)
4. **Distance** — calculate the distance between two geographic points
5. **Google Maps Link** — generate a link to a static Google Map for a location

## Clarified Requirements

### 1. Host GPS

- **Platforms:** macOS, Windows, Linux
- **Fallback:** return `None` if GPS hardware is unavailable or permission denied
- **Mode:** one-shot location fix

### 2. IP to Location

- **Database:** GeoLite2 (free tier, requires MaxMind account)
- **Crate:** `maxminddb`
- **Database path:** TBD (well-known default or user-configured)

### 3. Reverse Geocoding

- **Goal:** lat/lon to city-level place name
- **IP lookup:** MaxMind via `maxminddb`
- **Coordinate lookup:** `geocoding` crate with a free/open-source provider (e.g., Nominatim/OpenStreetMap)
- **Accuracy:** city level

### 4. Distance

- **Crate:** `geo` (full algorithm suite) unless computational cost is a real concern
- **Units:** configurable
- **Bearing:** TBD

### 5. Google Maps Link

- **Type:** regular Google Maps URL (no API key required), not static image API

### 6. Architecture

- **Pattern:** lib + CLI (follows monorepo convention)
- **CLI binary name:** `where`
- **Common type:** `Location` struct (or enum if variance requires)
- **CLI subcommands:**
  - `where gps` -> `Option<Location>`
  - `where ip <ip>` -> `Location`
  - `where distance <L1> <L2>`
  - `where reverse <lat> <long>`
- **Error handling:** `thiserror` (monorepo convention)
- **Async:** GPS and reverse geocoding are inherently async; IP lookup and distance can be sync
