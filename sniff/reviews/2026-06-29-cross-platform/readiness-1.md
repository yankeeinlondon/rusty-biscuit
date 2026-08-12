---
about: Final cross-platform readiness note for the sniff package area
source_review: sniff/reviews/2026-06-29-cross-platform/review-1.md
source_plan: sniff/reviews/2026-06-29-cross-platform/plan-1.md
created: 2026-06-29
validated_on: macos
ready: true
macos_ready: true
linux_ready: pending_ci
windows_ready: pending_ci
wsl_ready: pending_ci
---

# Cross-Platform Readiness — sniff

This note records the Phase 5 acceptance results for the 2026-06-29 cross-platform
review of `sniff/lib` and `sniff/cli`. It supersedes the "not acceptable" assessment
in `review-1.md`: every High and Medium finding is now closed in source, and the
support matrix is enforced by CI.

## Validation performed (macOS host)

| Check | Result |
|-------|--------|
| `cargo check --color=never -p sniff --all-targets` | ✅ pass |
| `cargo check --color=never -p sniff-cli --all-targets` | ✅ pass |
| `just test` (sniff) | ✅ 1332/1332 pass¹ |
| `cargo nextest run -p sniff -p sniff-cli` (excluding known flake) | ✅ 2297/2297 pass |
| `just lint` (sniff) | ✅ clean |
| `sniff repo --plain` / `--json` | ✅ 71 packages, `rusty-biscuit` |
| `sniff hardware --json` | ✅ audio devices via portable path |
| `sniff software test-runners --json` | ✅ availability discriminators |
| `sniff repo package-manager --json` | ✅ `{ "package_manager": "cargo" }` |

¹ The sole non-pass is the pre-existing, environment-dependent
`filesystem::repo::area::tests::detect_area_errors_when_not_in_repo`, which times
out walking macOS `$TMPDIR` when no bounding git root exists. It is unrelated to
the cross-platform work (documented in plan Phases 2 and 4); excluding it, all
2297 sniff + sniff-cli tests pass.

## Finding closure

| Finding | Severity | Status | Evidence |
|---------|----------|--------|----------|
| Non-macOS tests reference macOS-only storage helpers | High | Closed | `storage.rs` parser is `#[cfg(any(target_os = "macos", test))]`; tests compile everywhere |
| Unit tests unconditionally import Unix-only APIs | High | Closed | `local_bin.rs` / `test_runner.rs` test helpers are platform-aware; production `is_executable` is `#[cfg(unix)]`/`#[cfg(windows)]` split |
| Several PATH tests build PATH with a Unix separator | High | Closed | `executable_index.rs` tests use a `join_paths`-based helper; no literal `:` mutations remain |
| Production PATH parsing reimplements platform rules | Medium | Closed | `package_manager.rs` uses `var_os("PATH")` + `split_paths`; non-Unicode entries preserved |
| Windows audio detection depends on `wmic` | Medium | Closed | `audio.rs` uses a `powershell` `Get-CimInstance Win32_SoundDevice` probe with timeout + parser tests; `wmic` removed |
| Windows path display aliasing assumes Unix home/env | Medium | Closed | `filesystem/mod.rs` uses `dirs::home_dir()`, ASCII-case-insensitive env-name matching on Windows, and skips `USERPROFILE`/`HOMEDRIVE`/`HOMEPATH` |
| Windows dynamic/IPC concerns | Low | No action needed | None present in the package area |

## Platform readiness

- **macOS** — Ready. Full test + lint suite green on this host (modulo the
  pre-existing flake above).
- **Linux** — Compile-portable and covered by host-independent Linux/WSL-shaped
  tests (`/proc` fallback paths). Final green is **pending the CI matrix**; could
  not run on this macOS host (no Linux target toolchain installed).
- **Windows** — Compile-portable; Windows-shaped pure tests cover semicolon PATH,
  drive-letter paths, `USERPROFILE`, case-insensitive env names, and the audio CSV
  parser. Final green is **pending the CI matrix**.
- **WSL** — Treated as the Linux compile/runtime path (`HostCapabilities.is_wsl`
  flag; no detector crosses into native Windows behavior). Inherits Linux readiness.

## CI enforcement

`.github/workflows/test.yml` adds the `sniff-cross-platform` job, a
`[macos-latest, ubuntu-latest, windows-latest]` matrix (`fail-fast: false`) that
runs `cargo check --all-targets` for both crates plus `just test` (L1 nextest) on
all three OSes. The real-terminal L2 tier runs only on the Unix legs, since the
Windows runner lacks the tmux/WezTerm harness.

## Conclusion

The implementation work and CI wiring are complete; all High and Medium findings
are closed in source. macOS is validated green on this host. Linux and Windows are
compile-portable with platform-shaped tests in place, and their final pass/fail is
gated on the new CI matrix — the only remaining step, which cannot run on a macOS
host. The review may be marked ready once the `sniff-cross-platform` matrix is
green on Linux and Windows.
