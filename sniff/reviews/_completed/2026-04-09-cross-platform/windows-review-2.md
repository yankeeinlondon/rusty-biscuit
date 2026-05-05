# Windows Cross-Platform Implementation Review (Pass 2)

**Date:** 2026-04-09
**Reviewed against:** [windows-plan.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/reviews/2026-04-09-cross-platform/windows-plan.md)

## Findings

### High: the new Windows SCM backend uses the wrong handle type and will not compile on a Windows target

`OpenSCManagerW` in the `windows` crate returns `SC_HANDLE`, and `CloseServiceHandle` also takes `SC_HANDLE`. The implementation imports `HANDLE`, stores the SCM result in a `HANDLE`, and then passes that `HANDLE` back to `CloseServiceHandle`. Those are distinct wrapper types in `windows 0.62`, so this is a hard type mismatch in the Windows-only code path. The feature looks complete on a non-Windows host because that path is never compiled here, but a real Windows-target build should fail until this is changed to `SC_HANDLE`. See [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L30), [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L38), and [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L40).

### Medium: the new SCM smoke test is wrong on Windows and will fail as soon as the implementation actually works there

The test suite now includes `test_list_windows_scm_services_returns_vec`, but it asserts that the returned list is empty. That only matches the non-Windows stub path. On a normal Windows host, the whole point of this change is that SCM enumeration should return real services, so this test becomes a false failure and does not validate the intended behavior. It should either be scoped to non-Windows, or split into separate assertions for Windows vs non-Windows behavior. See [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L188).

### Medium: the new primary-interface integration test is broader than the designed contract and can fail on valid hosts

The plan called for a defensive assertion: only expect `primary_interface` when the host has at least one eligible non-loopback IPv4 interface. The current test instead asserts whenever interface enumeration succeeds and the interface list is non-empty. That will incorrectly fail on loopback-only, IPv6-only, or otherwise minimal Windows images even if the implementation is behaving correctly. This is a test-design bug rather than a network implementation bug, but it weakens the reliability of the new coverage. See [integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L894).

## Coverage Notes

- The timezone work is in better shape than the other two areas. The mapper has direct unit coverage, the parsing helper is now pure, and [integration.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/tests/integration.rs#L813) exercises the no-NTP path end to end.
- The network route parser also has solid unit coverage for the main parsing cases in [network/mod.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/network/mod.rs#L1271).
- The Windows SCM path still has the weakest verification story. The pure `service_from_raw_status()` helper is covered, but there is no successful Windows-target compile check in this change, which is why the `SC_HANDLE` mistake escaped.

## Ergonomics And Performance

- Replace `HANDLE` with `SC_HANDLE` in [windows_scm.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/services/windows_scm.rs#L30) and keep the RAII wrapper typed to `SC_HANDLE`. That is both clearer and correct for the `windows` bindings.
- Consider removing the extra allocation in [time.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/os/time.rs#L305) by switching `String::from_utf8(stdout.to_vec())` to `std::str::from_utf8(stdout)`.
- Keep the Windows-only verification honest by adding a real Windows-target CI step such as `cargo check -p sniff --target x86_64-pc-windows-msvc`. This is the cheapest way to catch FFI binding mistakes before runtime.

## Verification Notes

- `cargo test -p sniff os::time::tests` passed.
- `cargo test -p sniff network::tests` passed.
- `cargo test -p sniff services::tests` passed.
- `cargo test -p sniff test_os_timezone_without_ntp` passed.
- `cargo test -p sniff test_network_primary_interface_is_populated` passed on this macOS host.
- `cargo test -p sniff test_services_detailed_returns_non_empty_names` passed on this macOS host.
- `cargo test -p sniff` passed on this macOS host.
- `cargo check -p sniff --target x86_64-pc-windows-msvc` could not be completed here because the Windows target is not installed in this environment.
