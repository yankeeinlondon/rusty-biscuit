# Windows Cross-Platform Implementation Review (Pass 3)

**Date:** 2026-04-09
**Reviewed against:** [windows-plan.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/reviews/2026-04-09-cross-platform/windows-plan.md)

## Findings

### Medium: the documented alphabetical fallback for `primary_interface` still is not implemented

The fallback contract says ties are broken alphabetically, but the implementation never sorts. It just returns the first interface encountered in each priority tier. That makes the fallback dependent on interface enumeration order rather than a stable rule, which is exactly the kind of host-dependent behavior that shows up on Windows once the default-route signal is missing or cannot be mapped back to a local interface. See [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L360), [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L441), and [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L467).

### Medium: the primary-interface fallback is still Unix-centric, so the Windows fallback path remains weak

The new default-route parsing is in place, but the fallback classifier still only recognizes Unix-style physical names (`en*`, `eth*`, `wlan*`, `wlp*`, `enp*`) and Unix/Docker virtual names. Common Windows adapter names such as `Ethernet`, `Wi-Fi`, and `vEthernet` are not modeled at all. If `route print` fails, or the selected route IP does not map cleanly to a local interface, Windows drops into a heuristic that cannot reliably distinguish a real NIC from Hyper-V, VPN, Bluetooth, or loopback-style adapters. See [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L393), [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L409), and the test coverage in [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L1105).

### Medium: the Windows timezone feature still has no direct end-to-end test for the default `detect_timezone()` path

The pure parsing and mapping helpers are covered, and the no-NTP plan-based integration test now asserts `timezone.is_some()`. But the general integration test for `detect_timezone()` still only checks offset, abbreviation, monotonic clock, and serialization. That leaves the core Windows runtime contract from the plan unverified in the normal code path: a Windows host should now populate `timezone` via `tzutil`. See [integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L285), [integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L813), and [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs#L280).

### Medium: the SCM backend is still only exercised for `ServiceState::All`, which leaves the most Windows-specific behavior unpinned

The new SCM tests prove enumeration works and service names are non-empty, but they do not verify filtered queries. `services_detailed()` still filters by `state.matches(Some(s.running))`, while the Windows backend exposes richer raw SCM status codes in `status`. Right now there is no Windows-target test that locks down what `Running`, `Stopped`, or pending states are supposed to do, so the part of the API most likely to drift is also the part with the weakest coverage. See [services/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/mod.rs#L384), [services/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/mod.rs#L394), [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L195), and [integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L913).

## Coverage Notes

- The parser-level unit coverage is good for both the Windows timezone mapper and the Windows route-table parser.
- The weakest remaining coverage is Windows-host runtime behavior: default `detect_timezone()`, fallback primary-interface selection when route mapping fails, and filtered SCM service queries.
- There is still no repo-local Windows CI/build step checking `sniff` against a real Windows target. That was the only reliable way to catch the original SCM FFI mismatch before runtime.

## Ergonomics And Performance

- Refactor `find_primary_interface_fallback()` into a single scoring function plus a stable sort or `max_by_key`. That would make the documented tie-breaker real, reduce the repeated linear scans, and make Windows-specific adapter heuristics easier to extend without duplicating logic.
- Add explicit Windows name buckets to the fallback classifier. Even a small set such as `Ethernet`, `Wi-Fi`, `vEthernet`, `Bluetooth`, and common loopback/capture adapters would make the fallback materially more useful when the route signal is unavailable.
- For the SCM path, add a small pure helper that converts raw SCM state codes into the filter semantics you want to preserve. That would let you unit-test Windows-specific lifecycle behavior without needing a Windows host for every case.

## Verification Notes

- `cargo +stable check -p sniff` passed.
- `cargo +stable test -p sniff os::time::tests` passed.
- `cargo +stable test -p sniff network::tests` passed.
- `cargo +stable test -p sniff services::tests` passed.
- `cargo +stable test -p sniff test_os_timezone_without_ntp` passed.
- `cargo +stable test -p sniff test_network_primary_interface_is_populated` passed.
- `cargo +stable test -p sniff test_services_detailed_returns_non_empty_names` passed.
- `cargo +stable check -p sniff --target x86_64-pc-windows-msvc` could not validate the Windows code path here because the environment is missing Windows SDK/toolchain headers (`windows.h`, `stdio.h`, `sys/types.h`) in transitive native dependencies.
