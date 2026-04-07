# Host GPS

Query the host device's GPS hardware for current lat/lon coordinates.

## Platforms

| Platform | Service | Crate |
|----------|---------|-------|
| macOS | CoreLocation | `objc2-core-location` |
| Windows | Windows.Devices.Geolocation | `windows` |
| Linux | GeoClue2 (D-Bus) | `geoclue-zbus` |

Cross-platform wrapper `geo-loc` is an alternative if individual platform crates prove too complex.

## Behavior

- One-shot location fix (not streaming)
- Returns `Option<Location>` — `None` if no GPS hardware, permission denied, or timeout
- No fallback to IP geolocation (caller can chain if desired)

## CLI

```
where gps
```

## Key Considerations

- macOS requires `NSLocationWhenInUseUsageDescription` in Info.plist (or entitlement for CLI)
- Linux requires GeoClue2 D-Bus service running
- Windows requires location services enabled in Settings
- All platforms: permission prompt may appear on first use
