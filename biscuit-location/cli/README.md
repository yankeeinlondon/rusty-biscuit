# biscuit-location-cli

CLI for location services. Installed as the `where` binary.

## Commands

```bash
where gps                                    # GPS fix from host
where ip 8.8.8.8                             # IP to location
where reverse 34.0522 -118.2437              # Coordinates to place
where distance 34.05,-118.24 40.71,-74.01    # Distance between points
```

## Global Flags

- `--json` — emit machine-readable JSON on stdout (mutually exclusive with `--plain`)
- `--plain` — strip ANSI escape codes from text output
- `-q`, `--quiet` — suppress data output on stdout (errors still go to stderr)
- `-v`, `--verbose` — increase diagnostic verbosity on stderr (stackable: `-v`, `-vv`, `-vvv`)
- `--maps` — include a Google Maps link in location output
- `--db-path <PATH>` — override MaxMind database path

## Per-Command Flags

- `where gps --timeout <SECONDS>` — GPS fix timeout (default: 10)
- `where reverse --timeout <SECONDS>` — HTTP request timeout for reverse geocoding (default: 10)
- `where distance --unit <UNIT>` — distance unit: `meters`/`m`, `kilometers`/`km` (default), `miles`/`mi`, `nautical-miles`/`nm`

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

## Output Modes

The CLI emits three output shapes on stdout:

| Mode      | Activation                          | Description                          |
|-----------|-------------------------------------|--------------------------------------|
| Styled    | default (TTY, `FORCE_COLOR`)        | ANSI-styled human-readable text      |
| Plain     | `--plain`, `NO_COLOR`, or piped     | Unstyled text, safe for pipelines    |
| JSON      | `--json`                            | Machine-readable JSON                |

The `NO_COLOR=1` environment variable and a non-TTY stdout both force plain
mode. Set `FORCE_COLOR=1` to keep styled output even when piping (useful for CI).

### JSON schema

**Location** (`where gps`, `where ip`, `where reverse`):

```json
{
  "coordinates": { "latitude": 34.0522, "longitude": -118.2437 },
  "place": { "city": "Los Angeles", "region": "California", "country": "United States", ... },
  "source": { "kind": "ip", "address": "8.8.8.8" },
  "accuracy_meters": 1000.0,
  "maps_url": "https://..."
}
```

`place`, `accuracy_meters`, and `maps_url` are omitted when absent.

**Distance** (`where distance`):

```json
{ "value": 3944.42, "unit": "kilometers" }
```

**No GPS fix** (`where gps` with no fix available):

```json
{ "ok": false, "reason": "no_gps_fix" }
```

**Error envelope** (stderr, `--json` mode only):

```json
{ "error": true, "message": "MaxMind database not found at /path/to/db.mmdb" }
```

## STDOUT vs STDERR

- **STDOUT**: command results only. Safe to pipe.
- **STDERR**: error messages, usage errors, and diagnostic traces.
- In `--json` mode, stdout is always valid JSON; errors go to stderr as a
  single-line JSON envelope.
- `--quiet` suppresses stdout entirely; errors still appear on stderr.

## Exit Codes

| Code | Meaning                                                            |
|------|--------------------------------------------------------------------|
| `0`  | success                                                            |
| `1`  | runtime error (lookup failed, missing database, network error, …) |
| `2`  | usage error (invalid arguments, conflicting flags)                |

## Prerequisites

### `where ip`

Requires a MaxMind GeoLite2-City database at one of:

- the path given by `--db-path`
- the path from the `BISCUIT_LOCATION_MAXMIND_DB` environment variable
- the default location used by `biscuit-location` (see the library README)

Download the free GeoLite2-City database from
<https://www.maxmind.com/en/geolite2> (account required).

### `where reverse`

Performs a reverse-geocoding request over the network (OpenStreetMap Nominatim
by default). Requires outbound HTTPS access. Respects the public Nominatim
rate limits — avoid high-volume scripted use.

### `where gps`

Requires a host GPS source. Behavior is platform-specific; see the
`biscuit-location` library README for supported backends.

## Verbosity & Diagnostics

`--verbose` is for user-facing diagnostics. It initializes a `tracing`
subscriber whose filter scales with the flag count:

| Flag   | Effective filter                              |
|--------|-----------------------------------------------|
| (none) | disabled (zero overhead)                      |
| `-v`   | warn globally, info for `biscuit_location`, `r#where` |
| `-vv`  | info globally, debug for `biscuit_location`, `r#where` |
| `-vvv` | debug globally, trace for `biscuit_location`, `r#where` |

Setting `RUST_LOG` overrides all `-v` flags and accepts any valid
`tracing_subscriber::EnvFilter` directive.

```bash
RUST_LOG=debug where distance gps ip:8.8.8.8
```

Diagnostic output is always emitted to stderr so it cannot corrupt JSON on
stdout.

## Shell Completions

Generate completion scripts per shell:

```bash
# Bash (system-wide)
where completions bash > ~/.local/share/bash-completion/completions/where

# Zsh (add to fpath)
where completions zsh > ~/.zfunc/_where

# Fish
where completions fish > ~/.config/fish/completions/where.fish

# PowerShell
where completions powershell > $PROFILE.CurrentUserAllHosts.Completions/where.ps1

# Elvish
where completions elvish > ~/.elvish/completions/where.elv
```

The `completions` subcommand is hidden from `--help` by design, but is
documented here and in `where --help`'s "Shell completions" section.

## Examples

```bash
# Human-readable defaults
where ip 8.8.8.8

# Include a Google Maps link
where ip 8.8.8.8 --maps

# JSON for scripting
where ip 8.8.8.8 --json | jq '.coordinates.latitude'

# Distance in miles between LA and NYC
where distance 34.05,-118.24 40.71,-74.01 --unit mi

# Verbose diagnostics to stderr, clean data on stdout
where distance gps ip:8.8.8.8 -vv 2> debug.log
```
