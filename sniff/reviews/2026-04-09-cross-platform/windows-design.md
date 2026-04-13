# Windows Cross-Platform Fix Design

**Date:** 2026-04-09
**Scope:** `sniff/lib` Windows implementations for services, network primary-interface routing, and timezone detection
**Source review:** [review.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/reviews/2026-04-09-cross-platform/review.md)

## Summary

This design fixes the three Windows gaps called out in the 2026-04-09 cross-platform review:

1. `sniff services` returns an empty list because `InitSystem::WindowsScm` has no enumerator.
2. `primary_interface` is never informed by the default route on Windows.
3. `TimeInfo.timezone` is always `None` on Windows.

The implementation should preserve the current public API and best-effort failure model:

- service detection still returns `Vec<Service>` and degrades to an empty vector on failure
- network detection still falls back to the existing heuristic when the route signal cannot be resolved
- time detection still returns a populated `TimeInfo` even when timezone probing fails

The main design choices are:

- use the Windows Service Control Manager API for service enumeration
- use `route print 0.0.0.0` plus IPv4-to-interface matching for default-route detection
- use `tzutil /g` plus a static Windows-to-IANA mapping table for timezone names

## Goals

- Make Windows service enumeration return real services with running state and PID where available.
- Improve `primary_interface` selection on Windows by using the actual default route.
- Populate `TimeInfo.timezone` on Windows with a stable identifier instead of `None`.
- Keep the existing serialized shapes and public function signatures stable where possible.
- Keep Windows-specific code isolated behind `#[cfg(target_os = "windows")]`.

## Non-Goals

- Redesigning the `Service` model to represent the full Windows SCM state machine.
- Implementing Windows NTP detection.
- Adding IPv6 default-route selection.
- Building an automated CLDR ingestion pipeline for timezone mappings.

## File Map

**Modified**

- `sniff/lib/Cargo.toml`
- `sniff/lib/src/services/mod.rs`
- `sniff/lib/src/network/mod.rs`
- `sniff/lib/src/os/time.rs`
- `sniff/lib/tests/integration.rs`

**New**

- `sniff/lib/src/services/windows_scm.rs`
- `sniff/lib/src/os/windows_timezone_map.rs`

## C1. Windows Service Enumeration

### Current Problem

`ServiceManager::services_detailed()` only dispatches to launchd, systemd, OpenRC, and runit. On Windows, `detect_init_with_evidence()` correctly returns `InitSystem::WindowsScm`, but the match falls through to `_ => Vec::new()`.

### Design Choice

Use the Windows SCM API through the `windows` crate instead of parsing `sc.exe` output.

This is the better long-term fit for `sniff` because it avoids:

- localized or version-dependent command output
- `sc.exe` argument quirks such as `type=` / `state=` spacing
- partial output and resume-index handling for large service sets
- the loss of structured fields like PID and current state

### Dependency Change

Add a Windows-only dependency in [Cargo.toml](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/Cargo.toml):

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_Services",
] }
```

This keeps the dependency off non-Windows builds.

### Module Structure

Add a private Windows-specific helper module:

- [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs)

`services/mod.rs` becomes the dispatch surface only:

```rust
match self.init_system {
    InitSystem::Launchd => list_launchd_services(),
    InitSystem::Systemd => list_systemd_services(),
    InitSystem::OpenRc => list_openrc_services(),
    InitSystem::Runit => list_runit_services(),
    InitSystem::WindowsScm => list_windows_scm_services(),
    _ => Vec::new(),
}
```

### API Shape

The Windows helper should expose:

```rust
#[cfg(target_os = "windows")]
pub(crate) fn list_windows_scm_services() -> Vec<Service>;
```

Internally it should have a fallible helper:

```rust
#[cfg(target_os = "windows")]
fn enumerate_windows_scm_services() -> windows::core::Result<Vec<Service>>;
```

`list_windows_scm_services()` should translate errors into a warning plus `Vec::new()`, matching the rest of the module.

### Implementation Plan

1. Open the Service Control Manager with `OpenSCManagerW`.
2. Call `EnumServicesStatusExW` with:
   - `SC_ENUM_PROCESS_INFO`
   - `SERVICE_WIN32`
   - `SERVICE_STATE_ALL`
3. Handle `ERROR_MORE_DATA` by resizing the buffer and retrying until enumeration succeeds.
4. Iterate the returned `ENUM_SERVICE_STATUS_PROCESSW` entries.
5. Convert `lpServiceName` into UTF-8 `String`.
6. Map `SERVICE_STATUS_PROCESS` into `Service`:
   - `name`: service name from SCM
   - `pid`: `Some(dwProcessId)` only when `dwProcessId > 0`
   - `running`: `true` only for `SERVICE_RUNNING`
   - `status`: raw SCM current-state code as `Some(i32)`
7. Return the collected vector.

### Why `status` Should Hold the SCM State Code

The current `Service` model has only:

- `running: bool`
- `status: Option<i32>`

It does not have a cross-platform explicit state enum. For Windows, the most useful structured value is the raw SCM state code:

- `1` = stopped
- `2` = start pending
- `3` = stop pending
- `4` = running
- `5` = continue pending
- `6` = pause pending
- `7` = paused

This keeps the fix non-breaking while preserving more detail than a last-exit-code-only field would.

### Known Limitation

`ServiceState::Initializing` still cannot be represented correctly by the current filter path because `services_detailed()` filters via `state.matches(Some(s.running))`. That is existing behavior, not a Windows-specific regression.

This design does not change that API. A follow-up could introduce an explicit service lifecycle enum and teach all platforms to populate it.

### Documentation Updates

Update service docs in [services/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/mod.rs):

- add Windows SCM to the `services_detailed()` behavior list
- clarify that `status` is init-system specific and stores the raw SCM state code on Windows

### Test Plan

Add Windows-only integration coverage:

- `ServiceManager::detect()` on Windows should still report `InitSystem::WindowsScm`
- `services_detailed(ServiceState::All)` should usually be non-empty on Windows
- returned service names should be non-empty

Add pure helper tests where practical:

- mapping from raw SCM state code to `running`
- serialization of `ServicesInfo` remains unchanged

## C2. Windows Default Route Detection

### Current Problem

`detect_default_route_interface()` has BSD and Linux implementations only. On Windows it returns `None`, so `find_primary_interface()` loses its strongest signal and relies entirely on heuristics.

### Design Choice

Keep this fix command-based and avoid a new networking dependency.

Use `route print 0.0.0.0`, but do not parse the localized header tables. Instead:

- parse only the numeric rows for the IPv4 default route
- extract the interface IPv4 address from the route row
- map that IPv4 address back to a `NetworkInterface` by matching against `ipv4_addresses`

This is more robust than matching interface aliases because:

- `route` may expose different labels than `getifaddrs`
- IP address matching is already available from local interface enumeration
- the numeric route row is stable even when headers are localized

### Signature Change

Change the route detector so it can resolve against the already-enumerated interfaces:

```rust
fn detect_default_route_interface(interfaces: &[NetworkInterface]) -> Option<String>
```

That changes the call flow from:

```rust
select_primary_interface(interfaces, detect_default_route_interface().as_deref())
```

to:

```rust
select_primary_interface(
    interfaces,
    detect_default_route_interface(interfaces).as_deref(),
)
```

BSD and Linux implementations can keep returning interface names directly inside the same function body.

### Windows Implementation Plan

1. Run:

```rust
command_output("route", &["print", "0.0.0.0"])
```

2. Parse only lines whose first two whitespace-separated columns are:
   - `0.0.0.0`
   - `0.0.0.0`

3. For each matching row, parse:
   - column 4 as the route's interface IPv4 address
   - final column as the metric

4. Choose the route with the lowest metric.

5. Resolve the selected IPv4 address back to an interface name by checking:

```rust
iface.ipv4_addresses.contains(&selected_ip)
```

6. Return that interface name.

7. If any step fails, return `None` and let the current heuristic continue unchanged.

### Helper Functions

Add Windows-testable helpers in [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs):

```rust
#[cfg(any(target_os = "windows", test))]
fn parse_windows_default_route_interface_ip(output: &str) -> Option<std::net::Ipv4Addr>;

fn interface_name_for_ipv4(
    interfaces: &[NetworkInterface],
    address: std::net::Ipv4Addr,
) -> Option<String>;
```

### Why Not PowerShell Here

`Get-NetRoute` is attractive, but it creates a name-matching problem:

- PowerShell exposes `InterfaceAlias` or `IfIndex`
- `getifaddrs` currently gives `NetworkInterface.name`
- there is no existing interface-index field in `NetworkInterface`

Matching by IPv4 address avoids that mismatch without changing the serialized network model.

### Test Plan

Add parser tests with captured Windows output:

- single default route
- multiple default routes where the lowest metric wins
- malformed rows are ignored

Add resolver tests:

- selected IPv4 maps back to the expected interface name
- unknown IPv4 returns `None`

Add a Windows-only integration assertion:

- if there is at least one eligible non-loopback IPv4 interface, `primary_interface` should usually be `Some`

The integration assertion should stay defensive because some CI images may have unusual network setups.

### Known Limitation

This design only improves IPv4 default-route selection. IPv6 default-route support remains a separate follow-up.

## C3. Windows Timezone Detection

### Current Problem

[time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs) explicitly returns `None` for Windows timezone detection. That leaves:

- `TimeInfo.timezone = None`
- `timezone_abbr` falling back to chrono's numeric offset on many systems
- the cross-platform contract weaker than Linux and macOS

### Design Choice

Use `tzutil /g` as the OS probe, then normalize the Windows timezone ID to an IANA timezone name with a static mapping table.

If a Windows ID is not in the mapping table, return the raw Windows ID instead of `None`.

This gives the best compatibility without requiring registry access or another runtime dependency.

### Probe Flow

In [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs):

```rust
#[cfg(target_os = "windows")]
fn detect_timezone_name() -> Option<String> {
    let windows_id = detect_windows_timezone_id()?;
    Some(map_windows_timezone_to_iana(&windows_id).unwrap_or(windows_id))
}
```

Supporting helpers:

```rust
#[cfg(target_os = "windows")]
fn detect_windows_timezone_id() -> Option<String>;
```

`detect_windows_timezone_id()` should:

1. run `tzutil /g`
2. reject non-zero exit status
3. decode UTF-8 or UTF-8-compatible output
4. trim trailing `\r\n`
5. return `None` when the result is empty

### Mapping Table

Add a private mapping module:

- [windows_timezone_map.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/windows_timezone_map.rs)

Expose:

```rust
pub(crate) fn map_windows_timezone_to_iana(id: &str) -> Option<String>;
```

Implementation should use a static `match` or sorted table derived from Unicode CLDR `windowsZones.xml`, using the canonical territory `001` mapping where one Windows zone maps to multiple IANA zones.

Examples:

- `Pacific Standard Time` -> `America/Los_Angeles`
- `Mountain Standard Time` -> `America/Denver`
- `Central Standard Time` -> `America/Chicago`
- `Eastern Standard Time` -> `America/New_York`
- `UTC` -> `Etc/UTC`
- `W. Europe Standard Time` -> `Europe/Berlin`

### Why Mapping Matters

Without mapping, returning raw Windows IDs would technically fix the `None` problem, but it would still weaken behavior:

- `iana_to_abbreviation()` would not recognize most Windows IDs
- docs currently describe the field as an IANA timezone name
- downstream consumers would receive mixed identifier formats across platforms

Mapping first preserves the current cross-platform intent for the common case, while raw Windows IDs remain a safe fallback for unmapped zones.

### Documentation Updates

Update docs in [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs):

- replace "Windows: returns `None`" with the real behavior
- relax the field comment for `TimeInfo.timezone` to "best-effort timezone identifier"
- mention that Windows returns mapped IANA names when known, otherwise the Windows timezone ID

### Test Plan

Add unit tests for:

- `detect_windows_timezone_id()`-style output trimming using sample `tzutil` stdout
- mapping common Windows IDs to IANA names
- unknown Windows IDs returning `None` from the mapper

Keep existing tests and add Windows coverage:

- `detect_timezone().timezone.is_some()` on Windows
- the existing integration test `test_os_timezone_without_ntp` should pass on Windows after this change

## Cross-Cutting Implementation Notes

### Error Handling

All three fixes should preserve the current "best effort, no panic" behavior:

- SCM enumeration failure: `warn!` and return `Vec::new()`
- route parsing failure: return `None`, allowing heuristic fallback
- timezone probe failure: return `None`, leaving offset-based time info intact

### Conditional Compilation

Keep Windows-only logic tightly scoped:

- Windows SCM code stays in `services/windows_scm.rs`
- Windows timezone mapping stays in `os/windows_timezone_map.rs`
- parser helpers that can be unit-tested on non-Windows should use `#[cfg(any(target_os = "windows", test))]`

### Backward Compatibility

This design intentionally avoids public schema changes. The following existing types remain unchanged:

- `Service`
- `ServicesInfo`
- `NetworkInfo`
- `TimeInfo`

That keeps JSON output stable while fixing missing Windows data.

## Verification Plan

### Focused Tests

Run:

```bash
cargo test -p sniff services::tests
cargo test -p sniff network::tests
cargo test -p sniff os::time::tests
cargo test -p sniff test_os_timezone_without_ntp
```

### Windows Manual Checks

On a Windows host:

1. Run `sniff services --json` and confirm the service list is non-empty.
2. Run `sniff` or the equivalent network path and confirm `primary_interface` is populated on a normal connected host.
3. Run the OS/time detection path and confirm `timezone` is populated with a mapped IANA name for common zones.

### Build Check

If Windows CI is available, add or run:

```bash
cargo test -p sniff --target x86_64-pc-windows-msvc
```

## Risks and Follow-Ups

### Risk 1: Service Lifecycle Fidelity

Windows has richer service states than the current `Service` struct can represent. This fix restores enumeration but does not fully express `start pending`, `paused`, or `stop pending` in the public API.

Follow-up:

- introduce a cross-platform `ServiceStatus` enum
- make `ServiceState::Initializing` meaningful across providers

### Risk 2: CLDR Mapping Drift

Windows-to-IANA mappings can change over time.

Mitigation:

- add a source comment in `windows_timezone_map.rs` naming the CLDR version used
- keep the raw Windows ID fallback so detection still works when a mapping is missing

### Risk 3: Unusual Routing Tables

Multi-homed hosts may have several default routes.

Mitigation:

- select the lowest-metric route
- fall back to the existing heuristic if the route's interface IPv4 cannot be matched to local interfaces

## Recommended Implementation Order

1. Implement C3 first.
   This is the smallest change and immediately fixes a failing Windows expectation in `TimeInfo`.
2. Implement C2 second.
   It is self-contained and improves network quality without changing output schema.
3. Implement C1 last.
   It is the largest change because it adds a Windows-only dependency and native API code.

That order delivers quick user-visible improvements while reserving the most invasive change for the end.
