# Host GPS Access

Access the host device's GPS hardware from Rust via platform-specific location services.

## Platform Matrix

| Platform | System Service | Crate | Accuracy Source |
|----------|---------------|-------|-----------------|
| macOS | CoreLocation | `objc2-core-location` | GPS, WiFi, Bluetooth, Cellular |
| Windows | Windows.Devices.Geolocation | `windows` or `irox-win-location-api` | GPS, WiFi, IP |
| Linux | GeoClue2 (D-Bus) | `geoclue-zbus` | GPS (via gpsd), WiFi, Cellular |
| Cross-platform | Wraps native APIs | `geo-loc` | Platform-dependent |

## Architecture

```
Rust App --> Native Service Wrapper
    macOS  --> CoreLocation.framework --> Internal GPS/WiFi --> Satellites
    Windows --> Windows.Devices.Geolocation --> GNSS/WiFi --> Satellites
    Linux  --> GeoClue2 (D-Bus) --> gpsd / ModemManager --> Satellites
```

## Platform Details

### macOS (`objc2-core-location`)

Uses Objective-C bindings to `CLLocationManager`. Handles GPS/WiFi positioning transitions automatically.

**Required permissions** in `Info.plist`:
- `NSLocationUsageDescription`
- `NSLocationAlwaysAndWhenInUseUsageDescription`

### Windows (`windows` crate)

Direct access to the `Geolocation` namespace via the official `windows` crate.

```rust
use windows::Devices::Geolocation::Geolocator;

async fn get_location() -> windows::core::Result<()> {
    let locator = Geolocator::new()?;
    let pos = locator.GetGeopositionAsync()?.get()?;
    let coord = pos.Coordinate()?;
    println!("Lat: {}, Long: {}", coord.Latitude()?, coord.Longitude()?);
    Ok(())
}
```

### Linux (`geoclue-zbus`)

Async D-Bus interface to the system's GeoClue service.

**System dependency**: Requires `geoclue2` installed and running (standard on GNOME/KDE).

## Hardware-Level Access (Bypass OS Services)

For direct GPS module access (USB/Serial):

| Crate | Use Case |
|-------|----------|
| `gpsd-rs` | Linux with `gpsd` daemon handling hardware multiplexing |
| `nmea-parser` | Raw NMEA sentences from serial port (`/dev/ttyUSB0`, `COM3`) |

## Security and Privacy

- **Sandboxing**: macOS and Windows (Store/UWP) require explicit "Location" capability declarations
- **User consent**: OS triggers a system-level popup on first location request
- **Background access**: CLI tools need specialized "Always" permissions or TCC overrides on macOS
