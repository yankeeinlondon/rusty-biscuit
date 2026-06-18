# Performance Review: `messenger info` Cold-Start Bottleneck

## Observed Behavior

The installed `messenger info` binary can take **6+ seconds** on a cold cache
(first run after install, after cache expiry, or on a new machine). Warm runs
complete in ~100ms because the host-capabilities cache (`~/.sniff-programs.json`)
short-circuits the expensive detection paths.

## Execution Pipeline (call-by-call)

```
main()
 └─ info::run()
     ├─ Config::load()                              ~1ms (file read + JSON parse)
     ├─ config_helpers_for_host()
     │   └─ detect_os_type()                         ← call #1
     └─ build_report()
         ├─ detect_os_type()                         ← call #2 (duplicate)
         ├─ InstalledNotificationHelpers::new()
         │   └─ CategoryDetector::new()
         │       └─ find_programs_with_source_parallel()
         │           └─ rayon::par_iter → which() per helper (6 helpers)
         │               + find_macos_app_bundle() per helper  ← SLOW
         ├─ helper_record() × 6 helpers:
         │   ├─ info.version(helper)  [for installed helpers]
         │   │   ├─ ProgramEnum::path()
         │   │   │   └─ find_program() → which()       ← re-scans PATH
         │   │   └─ run_command_with_timeout()          ← spawns subprocess (3s timeout)
         │   └─ best_install_hint()
         │       └─ HostCapabilities::load_or_detect() ← called 6 times!
         │           ├─ [cache miss] HostCapabilities::detect()
         │           │   ├─ detect_os_type()           ← call #3 (duplicate)
         │           │   ├─ detect_has_bash()          → which("bash")
         │           │   ├─ InstalledOsPackageManagers::new()
         │           │   │   └─ CategoryDetector::new() → which() × 9 OS PMs
         │           │   ├─ InstalledLanguagePackageManagers::new()
         │           │   │   └─ CategoryDetector::new() → which() × 17 lang PMs
         │           │   └─ detect_can_sudo()
         │           │       ├─ Command::new("id")     ← subprocess
         │           │       └─ Command::new("sudo")   ← subprocess (2s probe timeout)
         │           └─ [cache hit] read + JSON parse
         └─ compute_election_order()                   ~0ms
```

## Identified Bottlenecks

### 1. `HostCapabilities::load_or_detect()` called once per helper (6x)

**Location:** `info.rs:149` inside `best_install_hint()`

Each call to `best_install_hint` independently calls
`HostCapabilities::load_or_detect()`. On a warm cache this is just 6 file reads +
JSON deserializations. On a cold cache, **all six calls trigger the full detection
pipeline** (26 `which` scans, 2 subprocess probes) because the first call writes
the cache but subsequent calls still enter `load_or_detect()` before the cache
file exists on disk.

**Fix:** Call `HostCapabilities::load_or_detect()` once in `build_report()` and
pass the reference into `best_install_hint()` / `helper_record()`.

**Estimated impact:** Eliminates 5 redundant cache reads (warm) or 5 redundant
full detections (cold, ~1-2s each on a slow machine).

### 2. `CategoryDetector::new()` uses `which` per binary instead of `ExecutableIndex`

**Location:** `types.rs:467` — `CategoryDetector::new()`

`CategoryDetector::new()` calls `find_programs_with_source_parallel()`, which
runs `which::which()` + `find_macos_app_bundle()` for each binary in parallel
via Rayon. Each `which` call scans the entire PATH directory by directory.
`find_macos_app_bundle()` does additional filesystem checks for app bundles.

The `ExecutableIndex` (in `find_program.rs`) already provides a solution: scan
all PATH directories once, then do O(1) HashMap lookups. The index exists and is
used by `sniff repo` and `sniff software`, but `CategoryDetector::new()` still
uses the per-binary `which` approach.

**Fix:** Refactor `CategoryDetector::new()` to build an `ExecutableIndex` once
and delegate to `new_with_index()`, or build the index at a higher level and
thread it through.

**Estimated impact:** Reduces notification-helper PATH scanning from 6 independent
`which` traversals (~10-20 directories each) to a single directory scan. Saves
~50-200ms on cold runs.

### 3. `ProgramEnum::version()` re-scans PATH instead of using detected path

**Location:** `schema.rs:288-289` — `ProgramEnum::path()` always calls
`find_program()` (another `which` scan), and `schema.rs:300` — `version()` calls
`self.path()` to find the binary.

The `CategoryDetector` already found the binary path during `new()`, but the
`version()` method on `CategoryDetector` delegates to `ProgramEnum::version()`,
which re-runs `which` from scratch. The detected path is thrown away.

**Fix:** Add a `version_from_path(path: &Path)` method to `ProgramEnum`, or pass
the detected path through `CategoryDetector::version()` so it can be used
directly instead of re-scanning.

**Estimated impact:** Eliminates 2 unnecessary `which` scans for the 2 installed
helpers (terminal-notifier, alerter) on this machine.

### 4. `detect_os_type()` called 3 times

**Location:**
- `info.rs:329` — `config_helpers_for_host()` calls `detect_os_type()`
- `info.rs:71` — `build_report()` calls `detect_os_type()`
- `host_capability.rs:64` — `HostCapabilities::detect()` calls `detect_os_type()`

The OS type doesn't change during a single CLI invocation. Each call is cheap
but still involves sysctl / file reads.

**Fix:** Compute `detect_os_type()` once in `info::run()` and pass it through.

**Estimated impact:** Minor (sub-millisecond), but improves code clarity.

### 5. Version detection spawns subprocesses with 3-second timeout

**Location:** `schema.rs:339-382` — `run_command_with_timeout()`

Each installed helper's version is detected by spawning a subprocess
(e.g., `terminal-notifier --version`) with a 3-second timeout. On this host,
2 helpers are installed, so 2 subprocesses spawn sequentially. If a helper is
slow to start (e.g., Cold macOS app bundle launch, slow disk), each can take
up to 3 seconds before timing out.

**Fix:** Run version detection in parallel using `rayon` or `tokio::spawn`,
or cache version strings alongside the `CategoryDetector` results.

**Estimated impact:** Reduces total version-detection wall time from
`sum(helper startup time)` to `max(helper startup time)`. On a slow machine
with 2 installed helpers, this could save 100-500ms.

### 6. `detect_can_sudo()` runs `sudo -n true` on every cold detection

**Location:** `host_capability.rs:485-492`

This spawns `sudo -n true`, which can hang for 1-2 seconds on some
configurations (LDAP, PAM modules). The result is cached in the host
capabilities cache file, so it only triggers on a cold cache, but it adds to
the cold-start penalty.

**Fix:** This is already mitigated by the cache. Consider increasing
`CACHE_TTL` or making the sudo probe optional for `messenger info` (it's only
needed for the install-plan hint, not for the info display).

**Estimated impact:** Saves 1-2s on cold cache when sudo is misconfigured or
slow.

### 7. Full package-manager detection on cold cache is expensive

**Location:** `host_capability.rs:73-74` — `detect()` creates
`InstalledOsPackageManagers::new()` (9 PMs) and
`InstalledLanguagePackageManagers::new()` (17 PMs).

When the host capabilities cache is cold, `detect()` scans for 26 package
managers via `CategoryDetector::new()` + `find_programs_with_source_parallel()`.
This is 26 `which` calls + 26 macOS bundle checks in parallel.

This detection is only needed for `best_install_hint()`, which just picks the
best install command to display. For `messenger info`, a simpler heuristic would
suffice.

**Fix:** For `messenger info`, consider a lightweight hint path that checks only
the OS-default package manager (e.g., `which brew` on macOS) instead of the
full 26-PM scan.

**Estimated impact:** Eliminates ~200ms of `which` scanning on cold cache.

## Priority Matrix

| # | Fix | Effort | Impact (cold) | Impact (warm) |
|---|-----|--------|---------------|----------------|
| 1 | Single `HostCapabilities` call | Small | **High** (~5-10s) | Low (~5 cache reads) |
| 5 | Parallel version detection | Medium | **Medium** (~0.5-1s) | None |
| 6 | Skip sudo probe for info | Small | **Medium** (~1-2s) | None |
| 2 | Use `ExecutableIndex` in `CategoryDetector::new()` | Medium | Medium (~0.2s) | None |
| 7 | Lightweight PM detection for info | Small | Medium (~0.2s) | None |
| 3 | Reuse detected path in version() | Small | Low (~0.1s) | None |
| 4 | Single `detect_os_type()` call | Trivial | Negligible | Negligible |

## Recommended Implementation Order

1. **Fix #1** (single `HostCapabilities` call) — highest impact, simplest change
2. **Fix #6** (skip sudo probe for info) — remove unnecessary slow path
3. **Fix #5** (parallel version detection) — meaningful improvement
4. **Fix #2** (`ExecutableIndex` adoption) — architectural improvement
5. **Fix #7** (lightweight PM for info) — further cold-start reduction
6. **Fix #3** (reuse detected path) — cleanup
7. **Fix #4** (single OS detection) — trivial cleanup
