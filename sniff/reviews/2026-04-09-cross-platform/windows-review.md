# Windows Cross-Platform Implementation Review

**Date:** 2026-04-09
**Reviewed against:** [windows-design.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/reviews/2026-04-09-cross-platform/windows-design.md)

## Findings

### High: the new Windows SCM implementation does not compile as written

The new SCM code constructs the guard with `ScopeGuard(...)` instead of `ScopeGuard::new(...)`, but the tuple struct stores `Option<F>`, not `F`, so this call is a type mismatch. That means the primary feature in this change set, Windows SCM enumeration, is not buildable in its current form. See [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L31) and [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L33).

### Medium: the timezone table claims canonical CLDR mappings but returns stale aliases for several zones

The mapper documents itself as using CLDR `territory="001"` canonical names, but it returns older aliases such as `America/Godthab`, `Europe/Kiev`, `Asia/Calcutta`, and `Asia/Katmandu`. That misses the design requirement to normalize Windows IDs to canonical IANA identifiers and creates avoidable cross-platform inconsistency for downstream consumers. See [windows_timezone_map.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/windows_timezone_map.rs#L3), [windows_timezone_map.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/windows_timezone_map.rs#L37), [windows_timezone_map.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/windows_timezone_map.rs#L46), [windows_timezone_map.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/windows_timezone_map.rs#L58), and [windows_timezone_map.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/windows_timezone_map.rs#L60).

### Medium: the designed Windows integration coverage for network and services was not implemented

The design called for Windows integration assertions that `primary_interface` is usually populated on eligible hosts and that `services_detailed(ServiceState::All)` is usually non-empty with non-empty service names. What landed is parser-level route testing in [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L557) and a Windows SCM smoke test that only checks “does not panic” in [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L136). There is no end-to-end Windows assertion in [integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs) comparable to the plan, so the two user-visible behaviors this work was meant to fix remain unvalidated at integration level.

### Low: the timezone trimming test does not exercise the implemented helper

`test_detect_windows_timezone_id_trims_output` only trims a hard-coded local string; it never calls a parser/helper that belongs to production code. As a result, the `tzutil` output handling path in [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs#L289) is still effectively untested. See the current test at [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs#L588).

### Low: the SCM record-to-`Service` mapping has no focused unit coverage

The Windows service mapper derives three important fields from native SCM data: `running`, `pid`, and raw `status`. That conversion is implemented inline in [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L95), but there is no pure helper and no unit test for state-code-to-`running` behavior, despite that being called out in the design. This leaves the most platform-specific logic fragile and only testable on a real Windows host.

## Ergonomics And Performance

- `map_windows_timezone_to_iana()` should return `Option<&'static str>` and convert to `String` only at the call site in [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs#L280). That removes one allocation per lookup and makes the mapping table cheaper and clearer.
- `interface_name_for_ipv4()` is dead on non-Windows builds, and `windows_timezone_map` is only needed for Windows or tests. Gating both with `#[cfg(any(target_os = "windows", test))]` would remove dead-code warnings and tighten platform scoping.
- The SCM mapper should use the named `SERVICE_RUNNING` constant instead of the literal `4` in [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L97). That is a readability improvement with no downside.
- The Windows timezone and SCM code would both benefit from small pure helpers:
  - `parse_windows_timezone_id_output(&str) -> Option<String>`
  - `service_from_scm_status(...) -> Service`
  
  That would make the tests meaningful on non-Windows hosts and reduce duplication in the platform-specific paths.

## Verification Notes

- `cargo test -p sniff network::tests` passed.
- `cargo test -p sniff os::time::tests` passed.
- `cargo test -p sniff services::tests` passed.
- `cargo check -p sniff --target x86_64-pc-windows-gnu` could not complete in this environment because the cross toolchain is missing `x86_64-w64-mingw32-gcc`, so I could not mechanically verify the Windows-only code path end-to-end here.
