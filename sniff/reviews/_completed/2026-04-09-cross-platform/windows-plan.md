# Sniff Windows Cross-Platform Fix Implementation Plan

**Date:** 2026-04-09
**Source design:** [windows-design.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/reviews/2026-04-09-cross-platform/windows-design.md)
**Scope:** `sniff/lib` Windows implementations for timezone detection, default-route-aware primary interface selection, and Windows SCM service enumeration

**Goal:** Close the three Windows gaps identified in the cross-platform review without changing the public JSON schema or the best-effort failure model.

**Execution Order:** Implement timezone first, default-route detection second, and Windows SCM enumeration last. That keeps the early work low-risk and isolates the new Windows dependency until the end.

**Constraints:**
- Keep `Service`, `ServicesInfo`, `NetworkInfo`, and `TimeInfo` unchanged.
- Keep all Windows-specific behavior behind `#[cfg(target_os = "windows")]`.
- Preserve fallback behavior when probes fail:
  - timezone detection may still return `None`
  - primary interface selection may still fall back to the existing heuristic
  - service enumeration may still return an empty vector

---

## File Map

**Modified**

| File | Responsibility |
|------|----------------|
| `sniff/lib/Cargo.toml` | Add Windows-only `windows` crate dependency |
| `sniff/lib/src/os/time.rs` | Replace Windows `None` timezone behavior with `tzutil` probing + mapping |
| `sniff/lib/src/network/mod.rs` | Make primary-interface routing Windows-aware using `route print 0.0.0.0` |
| `sniff/lib/src/services/mod.rs` | Dispatch `InitSystem::WindowsScm` and update docs |
| `sniff/lib/tests/integration.rs` | Add Windows-only integration assertions for timezone, network, and services |

**New**

| File | Responsibility |
|------|----------------|
| `sniff/lib/src/os/windows_timezone_map.rs` | Static Windows-to-IANA timezone mapping helper |
| `sniff/lib/src/services/windows_scm.rs` | Windows SCM service enumeration via native API |

---

## Task 1: Implement Windows Timezone Detection

**Files:**
- Modify: `sniff/lib/src/os/time.rs`
- Add: `sniff/lib/src/os/windows_timezone_map.rs`

`time.rs` currently hardcodes Windows timezone detection to `None`. This task should make Windows return a best-effort timezone identifier, preferably an IANA name, while keeping the existing `TimeInfo` shape intact.

- [ ] **Step 1: Add the Windows timezone mapping module**

Create `sniff/lib/src/os/windows_timezone_map.rs` with:

```rust
pub(crate) fn map_windows_timezone_to_iana(id: &str) -> Option<String>;
```

Implementation notes:
- Use a static `match` over Windows timezone IDs.
- Prefer the CLDR `territory="001"` canonical IANA mapping.
- Cover common Windows zones at minimum:
  - `Pacific Standard Time` -> `America/Los_Angeles`
  - `Mountain Standard Time` -> `America/Denver`
  - `Central Standard Time` -> `America/Chicago`
  - `Eastern Standard Time` -> `America/New_York`
  - `UTC` -> `Etc/UTC`
  - `W. Europe Standard Time` -> `Europe/Berlin`
- Add a source comment naming the CLDR source/version used to build the table.

- [ ] **Step 2: Wire the module into `time.rs`**

At the top of `sniff/lib/src/os/time.rs`, add a Windows-only module import:

```rust
#[cfg(target_os = "windows")]
mod windows_timezone_map;
```

Then import the mapper where needed:

```rust
#[cfg(target_os = "windows")]
use self::windows_timezone_map::map_windows_timezone_to_iana;
```

- [ ] **Step 3: Add a Windows timezone ID probe helper**

In `time.rs`, add:

```rust
#[cfg(target_os = "windows")]
fn detect_windows_timezone_id() -> Option<String>;
```

Behavior:
- Run `tzutil /g`
- Require a successful exit code
- Decode stdout as UTF-8
- Trim trailing `\r\n`
- Return `None` if the trimmed result is empty

- [ ] **Step 4: Replace the Windows `detect_timezone_name()` stub**

Change the current Windows implementation from:

```rust
#[cfg(target_os = "windows")]
fn detect_timezone_name() -> Option<String> { None }
```

to:

```rust
#[cfg(target_os = "windows")]
fn detect_timezone_name() -> Option<String> {
    let windows_id = detect_windows_timezone_id()?;
    Some(map_windows_timezone_to_iana(&windows_id).unwrap_or(windows_id))
}
```

This preserves a fallback even when the mapping table is incomplete.

- [ ] **Step 5: Update docs in `time.rs`**

Update the rustdoc and field comments so they match the new behavior:
- replace the Windows note that says timezone detection returns `None`
- relax the `TimeInfo.timezone` field comment from "IANA timezone name" to "best-effort timezone identifier"
- document that Windows returns a mapped IANA name when known, otherwise the raw Windows timezone ID

- [ ] **Step 6: Add unit tests for the Windows helper path**

Add tests in `time.rs` for:
- trimming `tzutil`-style stdout
- mapping common Windows IDs to IANA
- unknown Windows IDs returning `None` from the mapper

Structure the tests so pure parsing and mapping logic runs on non-Windows too where possible.

- [ ] **Step 7: Add Windows integration coverage**

In `sniff/lib/tests/integration.rs`, add Windows-only assertions:
- `detect_timezone().timezone.is_some()`
- `test_os_timezone_without_ntp` continues to pass on Windows

Keep the assertions defensive. The important contract is that a normal Windows host should now populate `timezone`.

- [ ] **Step 8: Verify**

Run:

```bash
cargo test -p sniff os::time::tests
cargo test -p sniff test_os_timezone_without_ntp
```

- [ ] **Step 9: Commit**

```text
feat(sniff): detect Windows timezone names via tzutil
```

**Acceptance criteria:**
- Windows no longer hardcodes `TimeInfo.timezone = None`
- mapped zones produce IANA names for common cases
- unmapped zones still surface a non-empty Windows timezone ID

---

## Task 2: Implement Windows Default Route Detection

**Files:**
- Modify: `sniff/lib/src/network/mod.rs`

`find_primary_interface()` currently calls a zero-argument route detector, and the Windows branch always returns `None`. This task should let Windows derive the default-route interface by matching the route table's interface IPv4 back to an already-enumerated interface.

- [ ] **Step 1: Change the route detector signature**

Update:

```rust
fn detect_default_route_interface() -> Option<String>
```

to:

```rust
fn detect_default_route_interface(interfaces: &[NetworkInterface]) -> Option<String>
```

Then update `find_primary_interface()` to call:

```rust
select_primary_interface(
    interfaces,
    detect_default_route_interface(interfaces).as_deref(),
)
```

Keep BSD and Linux behavior returning interface names directly within the same function.

- [ ] **Step 2: Add a Windows route-output parser**

In `network/mod.rs`, add:

```rust
#[cfg(any(target_os = "windows", test))]
fn parse_windows_default_route_interface_ip(output: &str) -> Option<std::net::Ipv4Addr>;
```

Parser requirements:
- only consider rows whose first two columns are `0.0.0.0` and `0.0.0.0`
- parse column 4 as the interface IPv4
- parse the final column as the metric
- ignore malformed rows
- choose the route with the lowest metric

This should avoid depending on localized headers.

- [ ] **Step 3: Add IPv4-to-interface resolution**

Add:

```rust
fn interface_name_for_ipv4(
    interfaces: &[NetworkInterface],
    address: std::net::Ipv4Addr,
) -> Option<String>;
```

Implementation:
- search `iface.ipv4_addresses`
- return the matching `iface.name`
- return `None` when there is no match

- [ ] **Step 4: Implement the Windows route detector branch**

Inside `detect_default_route_interface(interfaces)`, add the Windows branch:

```rust
#[cfg(target_os = "windows")]
{
    command_output("route", &["print", "0.0.0.0"])
        .and_then(|output| parse_windows_default_route_interface_ip(&output))
        .and_then(|ip| interface_name_for_ipv4(interfaces, ip))
}
```

Keep the fallback behavior unchanged when any stage fails.

- [ ] **Step 5: Preserve existing BSD and Linux behavior**

Refactor the BSD/Linux code paths only as much as needed to fit the new signature. Do not change their parsing rules or fallback ordering.

- [ ] **Step 6: Add parser and resolver unit tests**

Add tests in `network/mod.rs` for:
- one default route row
- multiple default routes where the lowest metric wins
- malformed rows being ignored
- IPv4 resolving to the expected interface name
- unknown IPv4 returning `None`

Use captured Windows `route print 0.0.0.0` output in string fixtures embedded in the tests.

- [ ] **Step 7: Add Windows integration coverage**

In `sniff/lib/tests/integration.rs`, add a Windows-only assertion:
- if there is at least one eligible non-loopback IPv4 interface, `primary_interface` should usually be `Some`

Keep the test defensive so CI images with unusual routing do not fail spuriously.

- [ ] **Step 8: Verify**

Run:

```bash
cargo test -p sniff network::tests
```

- [ ] **Step 9: Commit**

```text
feat(sniff): use Windows default route for primary interface selection
```

**Acceptance criteria:**
- Windows no longer always skips the default-route signal
- `primary_interface` prefers the actual routed interface when it can be matched
- heuristic fallback still works when route parsing fails

---

## Task 3: Implement Windows SCM Service Enumeration

**Files:**
- Modify: `sniff/lib/Cargo.toml`
- Modify: `sniff/lib/src/services/mod.rs`
- Add: `sniff/lib/src/services/windows_scm.rs`

`ServiceManager::services_detailed()` already detects `InitSystem::WindowsScm` indirectly through init detection, but it never dispatches to a Windows enumerator. This task adds a native Windows SCM backend without changing the `Service` shape.

- [ ] **Step 1: Add the Windows-only dependency**

In `sniff/lib/Cargo.toml`, add:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_Services",
] }
```

Keep the dependency Windows-only.

- [ ] **Step 2: Add the Windows SCM module**

Create `sniff/lib/src/services/windows_scm.rs` with:

```rust
#[cfg(target_os = "windows")]
pub(crate) fn list_windows_scm_services() -> Vec<Service>;

#[cfg(target_os = "windows")]
fn enumerate_windows_scm_services() -> windows::core::Result<Vec<Service>>;
```

Module responsibilities:
- isolate all native SCM bindings and unsafe code
- return `Vec::new()` on failure from the public wrapper
- log failures with `warn!`

- [ ] **Step 3: Implement SCM open + enumeration flow**

Inside `enumerate_windows_scm_services()`:
- open the Service Control Manager with `OpenSCManagerW`
- call `EnumServicesStatusExW` using:
  - `SC_ENUM_PROCESS_INFO`
  - `SERVICE_WIN32`
  - `SERVICE_STATE_ALL`
- handle `ERROR_MORE_DATA` by resizing and retrying
- iterate `ENUM_SERVICE_STATUS_PROCESSW` entries

Keep the unsafe surface narrow and comment the buffer/retry logic where it is not obvious.

- [ ] **Step 4: Map native service records into `Service`**

Convert each entry into the existing `Service` struct:
- `name`: SCM service name
- `pid`: `Some(dwProcessId)` only when `dwProcessId > 0`
- `running`: `true` only for `SERVICE_RUNNING`
- `status`: raw SCM current-state code as `Some(i32)`

Do not introduce a new public status enum in this task.

- [ ] **Step 5: Dispatch Windows SCM from `services/mod.rs`**

In `ServiceManager::services_detailed()`, add the missing branch:

```rust
InitSystem::WindowsScm => list_windows_scm_services(),
```

Also add the module import behind `#[cfg(target_os = "windows")]`.

- [ ] **Step 6: Update service docs**

Update the `services_detailed()` docs in `services/mod.rs` to say:
- Windows SCM is supported
- `status` is init-system specific
- on Windows, `status` holds the raw SCM state code

Do not promise more lifecycle fidelity than the current `Service` type can express.

- [ ] **Step 7: Add focused service tests**

In `services/mod.rs` tests and `sniff/lib/tests/integration.rs`, add Windows coverage for:
- `ServiceManager::detect()` still reporting `InitSystem::WindowsScm`
- `services_detailed(ServiceState::All)` not panicking on Windows
- returned service names being non-empty
- service serialization remaining unchanged

If a non-empty assertion is used, keep it scoped to Windows integration and phrase it as a normal-host expectation rather than a hard universal guarantee.

- [ ] **Step 8: Verify**

Run:

```bash
cargo test -p sniff services::tests
```

- [ ] **Step 9: Commit**

```text
feat(sniff): enumerate Windows services via SCM
```

**Acceptance criteria:**
- `InitSystem::WindowsScm` no longer falls through to an empty vector by default
- Windows services include names and running state, with PID where available
- failures still degrade to `Vec::new()` without panicking

---

## Task 4: Cross-Cutting Verification and Cleanup

**Files:**
- Modify only if tests or docs need follow-up adjustments after Tasks 1-3

- [ ] **Step 1: Re-run focused sniff test suites**

Run:

```bash
cargo test -p sniff os::time::tests
cargo test -p sniff network::tests
cargo test -p sniff services::tests
cargo test -p sniff test_os_timezone_without_ntp
```

- [ ] **Step 2: Run a broader package test pass**

Run:

```bash
cargo test -p sniff
```

- [ ] **Step 3: Run formatting if required**

Run:

```bash
cargo fmt --package sniff
```

- [ ] **Step 4: Perform manual Windows checks on a real host**

Validate these user-visible outcomes:
1. `sniff services --json` returns a non-empty service list on a normal Windows machine.
2. `primary_interface` is populated on a normally connected Windows machine.
3. `timezone` is populated with an IANA value for common Windows zones.

- [ ] **Step 5: Optional Windows-target build check**

If a Windows toolchain or CI target is available, run:

```bash
cargo test -p sniff --target x86_64-pc-windows-msvc
```

- [ ] **Step 6: Final review against design constraints**

Confirm before merge:
- no public schema changes slipped in
- Windows code stays isolated behind `#[cfg(target_os = "windows")]`
- best-effort fallback behavior is preserved in all three areas

---

## Risks and Watchpoints

- [ ] **SCM API complexity:** `EnumServicesStatusExW` requires manual buffer sizing and careful pointer handling. Keep unsafe code local to `windows_scm.rs`.
- [ ] **Timezone mapping drift:** the static mapping table will age. Keep a CLDR source note and retain the raw Windows ID fallback.
- [ ] **Unusual routing tables:** multi-homed Windows hosts may produce several default routes. The implementation should pick the lowest metric and then fall back if no local interface matches.
- [ ] **Test environment variance:** Windows CI images may have sparse services, synthetic interfaces, or odd timezone configuration. Prefer defensive integration assertions and stronger unit coverage for parsing/mapping logic.

---

## Definition of Done

- [ ] `TimeInfo.timezone` is usually populated on Windows.
- [ ] `NetworkInfo.primary_interface` can use the Windows default route when available.
- [ ] `sniff services` returns real Windows SCM services instead of an empty list.
- [ ] Existing JSON schema remains unchanged.
- [ ] Focused tests pass locally, and Windows-specific behavior has either runtime verification or CI coverage.
