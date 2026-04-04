# biscuit-location-cli

CLI for location services. Installed as the `where` binary.

## Commands

```bash
where gps                                    # GPS fix from host
where ip 8.8.8.8                             # IP to location
where reverse 34.0522 -118.2437              # Coordinates to place
where distance 34.05,-118.24 40.71,-74.01    # Distance between points
```

## Flags

- `--maps` — include a Google Maps link in output
- `--db-path <PATH>` — override MaxMind database path
- `--unit <UNIT>` — distance unit: meters, kilometers (default), miles, nauticalmiles

## Distance Inputs

The `distance` command accepts three input forms:

- `gps` — resolve via host GPS
- `ip:<address>` — resolve via IP lookup
- `<lat>,<lon>` — literal coordinates

```bash
where distance gps 37.7749,-122.4194
where distance ip:8.8.8.8 ip:1.1.1.1
where distance 51.5074,-0.1278 48.8566,2.3522
```
