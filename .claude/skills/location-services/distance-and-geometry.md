# Distance Calculation and Geometry

Lightweight crates for point-to-point distance without the full GeoRust stack.

## `haversine-rs`

[docs.rs](https://docs.rs/haversine-rs/0.3.0/haversine_rs/) | Spherical Earth model

### API

| Function | Description |
|----------|-------------|
| `distance(start, end, unit)` | Great-circle distance between two points |
| `distance_vec(points, unit)` | Cumulative distance along a path |
| `bearing(start, end)` | Initial bearing (forward azimuth) in degrees |
| `point_at_distance_and_bearing(start, dist, bearing, unit)` | Project a point from origin |

**Units**: `Unit::Kilometers`, `Miles`, `NauticalMiles`, `Yards`, `Feet`, `Inches`

### Point-to-Point Distance

```rust
use haversine_rs::{point::Point, units::Unit, distance};

let nyc = Point::new(40.7128, -74.0060);
let london = Point::new(51.5074, -0.1278);
let miles = distance(nyc, london, Unit::Miles);
```

### Route Length

```rust
use haversine_rs::{point::Point, units::Unit, distance_vec};

let route = vec![
    Point::new(34.0522, -118.2437), // LA
    Point::new(37.7749, -122.4194), // SF
    Point::new(47.6062, -122.3321), // Seattle
];
let total_km = distance_vec(route, Unit::Kilometers);
```

### Bearing and Projection

```rust
use haversine_rs::{point::Point, units::Unit, bearing, point_at_distance_and_bearing};

let start = Point::new(45.0, -75.0);
let heading = bearing(start, Point::new(50.0, -70.0));
let dest = point_at_distance_and_bearing(start, 500.0, 90.0, Unit::Kilometers);
```

### Gotchas

| Issue | Detail |
|-------|--------|
| ~0.5% error | Spherical model; use Vincenty or `geo` for sub-meter precision |
| Coordinate order | `Point::new(lat, lon)` -- GeoJSON uses `[lon, lat]` |
| No input validation | `lat=200.0` silently produces wrong results |
| Antipodal precision | Floating point issues at ~20,000 km distances |

## `geoutils`

[Repo](https://github.com/srishanbhattarai/geoutils) | [docs.rs](https://docs.rs/geoutils/0.5.1/) | v0.5.1 (last release: Aug 2022)

Lightweight geodesic helper with Vincenty (WGS-84) distance, Haversine, radius checks, and center calculation. **Maintenance is inactive** -- maintainer noted potential archival in Sep 2023.

### API

| Function | Method |
|----------|--------|
| Precise distance (WGS-84) | `Location::distance_to(&other)` -- Vincenty, returns `Result` |
| Fast approximate distance | `Location::haversine_distance_to(&other)` |
| Radius check (geofencing) | `Location::is_in_circle(&center, Distance)` |
| Center of points | `Location::center(&[&locations])` |
| Serialization | `serde` feature flag |

### Vincenty Distance

```rust
use geoutils::Location;

let berlin = Location::new(52.518611, 13.408056);
let moscow = Location::new(55.751667, 37.617778);
let distance = berlin.distance_to(&moscow)?;
println!("{:.3} m", distance.meters());
```

### Geofencing

```rust
use geoutils::{Distance, Location};

let store = Location::new(40.7580, -73.9855);
let courier = Location::new(40.7615, -73.9777);
let in_range = courier.is_in_circle(&store, Distance::from_meters(1_000.0))?;
```

### Center of Points

```rust
use geoutils::Location;

let points = [
    Location::new(34.0522, -118.2437),
    Location::new(36.1699, -115.1398),
    Location::new(32.7157, -117.1611),
];
let center = Location::center(&points.iter().collect::<Vec<_>>());
```

### Gotchas

| Issue | Detail |
|-------|--------|
| Vincenty convergence | `distance_to` fails after 100 iterations for some point pairs; handle `Result` |
| Internal rounding | Distances rounded to 3 decimal places, centers to 6 |
| `center` panics on empty | Divides by `coords.len()` without guard; validate input |
| No coordinate validation | `Location::new` accepts any `f64` values |
| `is_in_circle` exclusive | Uses `<` not `<=` at boundary; use `distance_to` + manual `<=` if needed |
| Inactive maintenance | Last release Aug 2022; open PRs unmerged; consider fork or alternative |

## Choosing Between Them

| Need | Use |
|------|-----|
| Simple distance with multiple unit support | `haversine-rs` |
| Bearing and destination projection | `haversine-rs` |
| WGS-84 ellipsoidal accuracy | `geoutils` (or `geo` for active maintenance) |
| Radius geofencing | `geoutils` |
| Long-term maintenance | `haversine-rs` or `geo` (avoid `geoutils` for new projects) |
| Full spatial algorithms | `geo` crate from GeoRust ecosystem |
