# Kickoff Implementation Review

Reviewed against:

- `biscuit-location/features/2026-04-03-kickoff/kickoff.md`
- `biscuit-location/features/2026-04-03-kickoff/tech-design.md`

Validation run during review:

- `just test` in `biscuit-location/`
- `just lint` in `biscuit-location/`
- `cargo run -q -p biscuit-location-cli -- reverse 34.0522 -118.2437 --json`
- `cargo run -q -p biscuit-location-cli -- gps --json`
- `cargo run -q -p biscuit-location-cli -- distance gps 34.0522,-118.2437 --json`

## Findings

### 1. Reverse geocoding is broken against the default production endpoint

Severity: high

`biscuit-location/lib/Cargo.toml:10` enables `reqwest` with `default-features = false` and only `["json"]`. That leaves the reverse client without any TLS backend, but the default endpoint in `biscuit-location/lib/src/config.rs` is `https://nominatim.openstreetmap.org/`.

The result is that the default user-facing command fails at runtime even though all tests pass:

```text
$ cargo run -q -p biscuit-location-cli -- reverse 34.0522 -118.2437 --json
{"error":true,"message":"reverse geocoding failed: error sending request for url (https://nominatim.openstreetmap.org/reverse?lat=34.0522&lon=-118.2437&format=json&addressdetails=1)"}
```

This is a direct functionality break for one of the five core feature goals.

Suggested fix:

- enable a TLS backend for `reqwest` (`rustls-tls` is the simplest portable choice)
- add at least one integration path that exercises the default HTTPS code path, or a test that would fail if the reverse client is built without TLS support

### 2. Windows and Linux GPS support is still unimplemented

Severity: high

The kickoff/design explicitly called for host GPS support on macOS, Windows, and Linux, with Windows using the `windows` crate and Linux using `geoclue-zbus`.

Current state:

- `biscuit-location/lib/src/gps/windows.rs:5-6` is a stub that always returns `Ok(None)`
- `biscuit-location/lib/src/gps/linux.rs:5-6` is a stub that always returns `Ok(None)`
- `biscuit-location/lib/Cargo.toml:16-19` only declares macOS target dependencies; there are no Windows or Linux GPS dependencies at all

This means cross-platform GPS support is not implemented, only macOS is attempted.

Suggested fix:

- implement the Windows and Linux backends from the design
- add platform-gated normalization tests so permission denied / disabled / timeout behavior is covered even when live hardware is not available

### 3. `distance gps ...` turns a normal no-fix case into an `Internal` error

Severity: medium

The design says GPS unavailability, permission denial, and timeout are not hard errors; they should surface as `Ok(None)`. That contract is respected by `LocationService::gps()`, but `LocationService::resolve_input()` collapses `None` into `LocationError::Internal("no GPS fix available")` at `biscuit-location/lib/src/service.rs:102-108`.

That leaks into CLI behavior:

```text
$ cargo run -q -p biscuit-location-cli -- distance gps 34.0522,-118.2437 --json
{"error":true,"message":"internal error: no GPS fix available"}
```

The failure is understandable for `distance`, but the error shape is not. A missing GPS fix is an expected user-facing condition, not an internal fault.

Suggested fix:

- add a domain error such as `NoGpsFix`
- keep `Internal` reserved for genuine invariants/bugs
- decide whether `distance` should produce a dedicated user-facing message when one operand is `gps` and no fix is available

### 4. Reverse endpoint construction is brittle for configurable/self-hosted endpoints

Severity: medium

`biscuit-location/lib/src/reverse.rs:37-40` builds the request URL with raw string concatenation:

```rust
format!("{}reverse?lat=...&lon=...", self.config.endpoint, ...)
```

That only works cleanly when the configured endpoint already has the exact expected trailing slash shape. It will produce incorrect URLs for perfectly reasonable overrides such as:

- `http://host/nominatim`
- `http://host/api/`
- any endpoint carrying existing query state

The tech design explicitly called out endpoint configurability for tests and self-hosted Nominatim, so this should be robust.

Suggested fix:

- use `Url::join("reverse")`
- build query parameters with `query_pairs_mut()`
- add tests for both trailing-slash and no-trailing-slash endpoint overrides

### 5. The CLI does not expose the designed timeout override for reverse geocoding

Severity: medium

The design called for a timeout override for GPS and reverse-geocoding operations. The CLI only exposes a timeout on `where gps` (`biscuit-location/cli/src/args.rs:44-49`), and `biscuit-location/cli/src/commands.rs:23-32` only threads that timeout into `gps_timeout`.

`where reverse ...` therefore always uses the library default timeout with no CLI escape hatch.

Suggested fix:

- add either a shared `--timeout` flag or a `where reverse --timeout <SECONDS>` flag
- thread that into `LocationConfig.reverse.timeout`

## Test Coverage Gaps

### Reverse geocoding coverage is too narrow

The current reverse tests in `biscuit-location/lib/src/reverse.rs:131-257` cover happy-path mapping and an HTTP status error, but they do not cover:

- timeout behavior
- rate-limit waiting behavior
- user-agent behavior
- endpoint joining behavior
- the real default HTTPS transport path

The missing HTTPS path is what allowed the production regression above to ship while tests stayed green.

### IP lookup mapping is lightly tested

`biscuit-location/lib/src/ip.rs:104-133` only covers:

- opening a nonexistent DB
- a real-DB env-gated lookup

It does not unit-test the record mapping logic in `city_to_location()` for:

- sparse GeoLite2 records
- missing lat/lon -> `IpNotFound`
- missing city/subdivision/country fields
- IPv6 lookup mapping behavior independent of real MMDB contents

The design explicitly called for record-to-domain mapping tests separate from MMDB I/O. Those are still missing.

### CLI integration tests are strong for parsing/output shape, but light on runtime command paths

`biscuit-location/cli/tests/cli_tests.rs` is mostly focused on help, argument validation, output modes, and literal-coordinate distance flows. It does not cover:

- `where gps` no-fix behavior
- `where ip` runtime success/failure behavior with a configured DB
- `where reverse` runtime behavior
- `where distance gps ...` failure behavior
- `--maps` output for any command

Given the monorepo expectation of strong integration coverage, these gaps are material.

## Ergonomics / Performance Suggestions

### Use a file-backed MaxMind reader instead of `Reader<Vec<u8>>`

`biscuit-location/lib/src/ip.rs:11-25` stores the database as `Reader<Vec<u8>>`, which reads the full `.mmdb` into memory. The design called for a file-backed long-lived reader. Switching to a memory-mapped/file-backed reader would better match the design and reduce peak memory use for large GeoLite2 databases.

### Simplify `LocationService` construction around actual optional capabilities

`biscuit-location/lib/src/service.rs:20-21` stores `reverse_geocoder` as an `Option`, but it is always constructed as `Some(...)` when the feature is enabled. Either make reverse genuinely optional via config, or store it directly and remove the dead `None` branch.

### Reserve `Internal` for true internal faults

Several expected operational states are currently normalized into broad stringly-typed errors. Introducing more precise variants such as `NoGpsFix` and possibly `ReverseTimeout` would improve library ergonomics and CLI messaging.

### Fix docs drift while touching the feature again

`biscuit-location/cli/README.md:111-113` documents `MAXMIND_DB_PATH`, but the implementation uses `BISCUIT_LOCATION_MAXMIND_DB`. That will mislead users trying to make `where ip` work.

## Summary

The package structure, core domain types, distance calculation, maps URL generation, and CLI output work are in good shape, and the existing tests are clean. The main gaps are concentrated in runtime feature completeness:

1. reverse geocoding is currently broken against the default HTTPS endpoint
2. Windows/Linux GPS support is still stubbed out
3. some expected no-result states are surfaced as internal errors
4. the reverse configuration surface and coverage are still thinner than the design called for
