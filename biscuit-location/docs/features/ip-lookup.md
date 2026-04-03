# IP to Location

Resolve an IP address to a geographic location using a local MaxMind GeoLite2 database.

## Crate

`maxminddb` — lazy/selective decoding, safe Rust, excellent docs.

## Database

GeoLite2-City (free tier). Not bundled — user must download from MaxMind.com (requires account). Database path resolution strategy TBD (well-known default location, env var, or explicit config).

## API Surface

- Input: IP address (v4 or v6)
- Output: `Location` (city, region, country, coordinates)
- Sync — the mmdb reader is file-backed/mmap, no network required

## CLI

```
where ip <ip-address>
```

## Key Considerations

- Database file must exist at lookup time; clear error if missing
- Atomic file renames if supporting DB updates (mmap SIGBUS risk)
- GeoLite2 has its own EULA separate from the ISC-licensed crate
