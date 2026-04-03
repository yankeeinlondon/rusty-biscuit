# GeoRust Ecosystem

The [GeoRust](https://georust.org/) ecosystem is the center of gravity for Rust geospatial work. Most crates integrate with or depend on `geo-types` and `geo`.

## Crate Reference

| Crate | Category | Description | docs.rs |
|-------|----------|-------------|---------|
| `geo-types` | Core geometry | Shared primitive types: `Point`, `LineString`, `Polygon`, `Multi*`, `Rect` | [docs.rs](https://docs.rs/geo-types/) |
| `geo` | Spatial algorithms | Topology, boolean ops, buffering, transforms, clustering, distance | [docs.rs](https://docs.rs/geo/) |
| `geojson` | Interchange | GeoJSON parse/serialize for `Geometry`, `Feature`, `FeatureCollection` | [docs.rs](https://docs.rs/geojson/) |
| `wkt` | Interchange | OGC Well-Known Text geometry format | [docs.rs](https://docs.rs/wkt/) |
| `proj` | Projection | PROJ bindings for CRS transforms, EPSG reprojection, grid-aware transforms | [docs.rs](https://docs.rs/proj/) |
| `gdal` | GIS data access | GDAL bindings for raster/vector I/O, metadata, format translation | [docs.rs](https://docs.rs/gdal/) |
| `geozero` | GIS data access | Zero-copy reading/writing across GeoJSON, WKT, WKB, MVT, PostGIS, FlatGeobuf | [docs.rs](https://docs.rs/geozero/) |
| `rstar` | Spatial indexing | R*-tree for nearest-neighbor, bounding-box queries, bulk loading | [docs.rs](https://docs.rs/rstar/) |
| `geographiclib-rs` | Algorithms | Precise ellipsoidal geodesic calculations, polygon area/perimeter | [docs.rs](https://docs.rs/geographiclib-rs/) |
| `geohash` | Discrete indexing | Geohash encode/decode, neighborhood ops, prefix-based proximity | [docs.rs](https://docs.rs/geohash/) |
| `h3o` | Discrete indexing | Pure Rust Uber H3 hexagonal grid, cell traversal, analytics/tiling | [docs.rs](https://docs.rs/h3o/) |
| `maxminddb` | IP geolocation | MaxMind DB reader for GeoIP2/GeoLite2 lookups | [docs.rs](https://docs.rs/maxminddb/) |

## Dependency Graph

```
geo-types --> geo --> rstar
                  --> geojson
                  --> wkt
                  --> geozero
                  --> proj
          gdal --> proj
       geohash --> geo
           h3o --> geo
     maxminddb    (independent: IP geolocation)
```

## Selection Guide

| Need | Start With |
|------|-----------|
| General geospatial primitives | `geo-types` + `geo` |
| Web API payloads | `geojson` |
| Database interchange | `wkt` or `geozero` |
| CRS / reprojection | `proj` |
| Raster data or broad format support | `gdal` |
| Nearest-neighbor queries | `rstar` |
| Grid bucketing / tiling | `geohash` (simple) or `h3o` (hexagonal) |
| IP-to-location | `maxminddb` |
| Precise ellipsoidal geodesics | `geographiclib-rs` |
| Multi-format zero-copy | `geozero` |

## Adjacent Crates (Less Central)

| Crate | Purpose |
|-------|---------|
| `geocoding` | Forward/reverse geocoding (less active) |
| `shapefile` | Shapefile reading |
| `osmpbfreader` | OpenStreetMap PBF format |
| `postgis` | PostGIS integration |

## Sources

Data checked against [crates.io](https://crates.io/) and [docs.rs](https://docs.rs/) as of 2026-04-01.
