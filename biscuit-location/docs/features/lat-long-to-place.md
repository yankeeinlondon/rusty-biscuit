# Reverse Geocoding

Convert bare lat/lon coordinates into a recognizable place (city, town, state, country).

## Crate

`geocoding` with a free/open-source provider — Nominatim (OpenStreetMap) is the primary candidate. No API key required, but Nominatim has usage policies (1 req/sec, user-agent required).

## Accuracy

City level.

## API Surface

- Input: latitude, longitude
- Output: `Location` (city, region, country, etc.)
- Async — requires network call to geocoding provider

## CLI

```
where reverse <lat> <long>
```

## Key Considerations

- Network dependency — this is the only feature requiring internet access at query time
- Nominatim rate limiting (1 request/second for the public instance)
- Could self-host Nominatim for higher throughput, but out of scope for v1
- Graceful handling when offline or provider unreachable
