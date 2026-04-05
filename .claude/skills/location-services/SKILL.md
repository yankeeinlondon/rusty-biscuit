---
name: location-services
description: Expert knowledge for Rust geolocation, IP-to-location (MaxMind), distance calculation (Haversine/Vincenty), host GPS access, and the GeoRust crate ecosystem. Use when working in biscuit-location/, implementing geolocation features, choosing between geo crates, or adding location-based services.
---

# Location Services

Rust crate ecosystem and patterns for geolocation, distance calculation, IP-based lookup, and host GPS access.

## Crate Categories

| Category | Crates | Use For |
|----------|--------|---------|
| Core geometry | `geo-types`, `geo` | Shared point/polygon types, spatial algorithms, Vincenty/Haversine |
| IP geolocation | `maxminddb`, `geoip2` | IP-to-city/country/ASN via MaxMind `.mmdb` files |
| Distance (lightweight) | `haversine-rs`, `geoutils` | Simple point-to-point distance without full GIS stack |
| Interchange formats | `geojson`, `wkt` | GeoJSON/WKT serialization |
| Projection | `proj` | CRS transforms, EPSG reprojection |
| GIS data access | `gdal`, `geozero` | Raster/vector I/O, zero-copy format conversion |
| Spatial indexing | `rstar` | R*-tree nearest-neighbor, bounding-box queries |
| Discrete indexing | `geohash`, `h3o` | Cell-based bucketing, tiling, geo partitioning |
| Host GPS | Platform APIs | Native lat/lon from device hardware |

## Crate Selection

### IP Geolocation

| | `maxminddb` | `geoip2` (IncSW) |
|---|---|---|
| Decoding | Lazy/selective via `decode_path` | Eager codegen structs |
| Safety | Safe Rust (mmap is only unsafe) | More internal unsafe |
| Custom schemas | Excellent | Standard MaxMind only |
| Documentation | Excellent | Minimal |
| Best for | Most production apps | Extreme throughput, full-record decode |

Database is **not bundled** -- download GeoLite2 (free, requires account) or GeoIP2 (commercial) from [MaxMind.com](https://www.maxmind.com). Crate is ISC licensed; database has its own EULA.

See [ip-geolocation.md](ip-geolocation.md) for API examples, gotchas, and detailed comparison.

### Distance Calculation

| | `haversine-rs` | `geoutils` | `geo` |
|---|---|---|---|
| Model | Spherical (Haversine) | Vincenty (WGS-84) + Haversine | Full algorithms suite |
| Accuracy | ~0.5% error | High (Vincenty can fail to converge) | Highest |
| Extras | Bearing, projection, route distance | Geofencing, center-of-points, serde | Polygons, projections, indexing |
| Maintenance | Active | Inactive since 2022 | Active (GeoRust) |
| Weight | Minimal | Minimal | Heavier |

See [distance-and-geometry.md](distance-and-geometry.md) for API examples and gotchas.

### Host GPS Access

| Platform | Service | Crate |
|----------|---------|-------|
| macOS | CoreLocation | `objc2-core-location` |
| Windows | Windows.Devices.Geolocation | `windows` |
| Linux | GeoClue2 (D-Bus) | `geoclue-zbus` |
| Cross-platform | Wraps native APIs | `geo-loc` |
| Hardware direct | Serial GPS/NMEA | `gpsd-rs`, `nmea-parser` |

See [host-gps.md](host-gps.md) for platform details and permission requirements.

## GeoRust Ecosystem

The [GeoRust](https://georust.org/) ecosystem centers on `geo-types` + `geo`. Most Rust geography crates integrate with or depend on them.

```
geo-types --> geo --> geojson, wkt, rstar, proj, geozero
                  gdal --> proj
          geohash --> geo
              h3o --> geo
        maxminddb    (separate: IP geolocation)
```

Key observations:
- `geo-types` + `geo` are the foundation; start here for general geospatial work
- `proj` and `gdal` bridge to the wider GIS world (CRS, raster, format support)
- `rstar` for in-memory spatial indexing; `geohash`/`h3o` for discrete cell-based indexing
- `maxminddb` is a different category entirely (IP lookup, not geometry)

See [georust-ecosystem.md](georust-ecosystem.md) for full crate table with repo links and features.

## Common Patterns

### MaxMind IP Lookup

```rust
use maxminddb::{geoip2, Reader};
let reader = Reader::open_readfile("GeoLite2-City.mmdb")?;
let city: geoip2::City = reader.lookup("1.1.1.1".parse()?)?;
```

### Haversine Distance

```rust
use haversine_rs::{point::Point, units::Unit, distance};
let d = distance(Point::new(40.71, -74.00), Point::new(51.51, -0.13), Unit::Kilometers);
```

### Vincenty Distance (geoutils)

```rust
use geoutils::Location;
let d = Location::new(52.52, 13.41).distance_to(&Location::new(55.75, 37.62))?.meters();
```

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Swapping lat/lon order | `haversine-rs` and `geoutils` use `(lat, lon)`; GeoJSON uses `[lon, lat]` |
| Using `geoutils` for new projects | Inactive since 2022; prefer `haversine-rs` or `geo` |
| Not handling Vincenty convergence failure | `geoutils::distance_to` returns `Result`; fall back to Haversine |
| Assuming MaxMind DB is bundled | Must download separately; requires MaxMind account |
| Using `maxminddb` mmap without atomic file updates | File modification during read causes SIGBUS; use atomic renames |
| Expecting sub-meter accuracy from Haversine | Spherical model has ~0.5% error; use Vincenty or `geo` for precision |
| Calling `geoutils::Location::center` with empty slice | Panics (divides by zero); validate input first |
