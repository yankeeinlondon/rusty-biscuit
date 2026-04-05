# Distance Calculation

Calculate the distance between two geographic points.

## Crate

`geo` — full algorithm suite including Haversine, Vincenty, and geodesic methods. Preferred over lightweight alternatives (`haversine-rs`, `geoutils`) for accuracy and active maintenance.

## API Surface

- Input: two locations (lat/lon pairs, or `Location` structs)
- Output: distance with configurable units (km, miles, meters, nautical miles)
- Sync — pure computation, no I/O

## CLI

```
where distance <location1> <location2>
```

Location format for CLI args TBD (lat,lon pairs, city names, IP addresses).

## Key Considerations

- `geo` uses `geo-types::Point` — need conversion from our `Location` struct
- Default to geodesic (most accurate); Haversine as fallback is unnecessary given `geo` handles both
- Bearing calculation could be added later if needed
