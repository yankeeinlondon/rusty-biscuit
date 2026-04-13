# Cross-Platform Review: Sniff Library

**Date:** 2026-04-09
**Reviewer:** Claude (automated cross-platform audit)
**Scope:** All OS-specific code in `sniff/lib/src/`
**Host:** macOS (review targets Windows and Linux implementations)

---

## Summary

The Sniff library has solid cross-platform scaffolding with `#[cfg]` gates throughout, but several Windows and Linux code paths have functional gaps, logic bugs, or silent no-ops that would produce incorrect or empty results on those platforms. This review identifies **14 issues** across 8 files, organized by severity.

---

## Critical Issues

### C1. Windows service listing returns empty vector

**File:** `services/mod.rs:381-387`
**Platform:** Windows

The `services_detailed()` method dispatches on `InitSystem` but `WindowsScm` falls through to the `_ => Vec::new()` catch-all:

```rust
let all_services = match self.init_system {
    InitSystem::Launchd => list_launchd_services(),
    InitSystem::Systemd => list_systemd_services(),
    InitSystem::OpenRc => list_openrc_services(),
    InitSystem::Runit => list_runit_services(),
    _ => Vec::new(),  // <-- WindowsScm lands here
};
```

Windows is correctly detected as `WindowsScm` (line 184), but no service enumeration is ever performed.

**Impact:** `sniff services` on Windows always returns an empty list.

**Suggested fix:** Add a `list_windows_services()` function that uses `sc query type=service state=all` or the Windows Service Control Manager API via the `windows` crate. Minimal implementation:

```rust
InitSystem::WindowsScm => list_windows_scm_services(),
```

```rust
fn list_windows_scm_services() -> Vec<Service> {
    let output = match Command::new("sc")
        .args(["query", "type=service", "state=all"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse SERVICE_NAME / STATE lines from sc query output
    parse_sc_query_output(&stdout)
}
```

---

### C2. Windows default route detection returns `None`

**File:** `network/mod.rs:491-501`
**Platform:** Windows

The `detect_default_route_interface()` function has implementations for macOS/BSD and Linux, but falls through to `None` on Windows:

```rust
#[cfg(not(any(
    target_os = "macos",
    target_os = "freebsd",
    ...
    target_os = "linux"
)))]
{
    None
}
```

**Impact:** `primary_interface` is always `None` on Windows, degrading the quality of network detection — the primary interface selection heuristic loses its best signal.

**Suggested fix:** Use `route print 0.0.0.0` or `Get-NetRoute -DestinationPrefix '0.0.0.0/0'` via PowerShell:

```rust
#[cfg(target_os = "windows")]
{
    command_output("route", &["print", "0.0.0.0"])
        .and_then(|output| parse_windows_default_route_interface(&output))
}
```

Parse the "Interface List" and "Active Routes" table from `route print` output.

---

### C3. Windows timezone detection is unimplemented

**File:** `os/time.rs:269-273`
**Platform:** Windows

```rust
#[cfg(target_os = "windows")]
fn detect_timezone_name() -> Option<String> {
    // Windows timezone detection requires registry queries
    None
}
```

**Impact:** `timezone` field in `TimeInfo` is always `None` on Windows, and `timezone_abbr` falls back to the numeric offset string from chrono.

**Suggested fix:** Read the registry key or use `tzutil /g`:

```rust
#[cfg(target_os = "windows")]
fn detect_timezone_name() -> Option<String> {
    let output = Command::new("tzutil")
        .arg("/g")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tz = String::from_utf8(output.stdout).ok()?.trim().to_string();
    // Windows uses its own timezone IDs (e.g., "Pacific Standard Time").
    // Optionally map to IANA via a lookup table, or return the Windows ID.
    if tz.is_empty() { None } else { Some(tz) }
}
```

Note: Windows timezone names don't match IANA names. A mapping table (e.g., `windowsZones.xml` from Unicode CLDR) would be needed for full IANA compatibility, but returning the Windows name is still better than `None`.

---

## High Severity Issues

### H1. `has_in_path()` on Unix doesn't check executable permission

**File:** `services/mod.rs:509-512`
**Platform:** Linux / macOS

```rust
#[cfg(not(windows))]
{
    env::split_paths(&path_var).any(|dir| dir.join(exe_name).is_file())
}
```

This only checks if the file exists, not if it's executable. Compare with the careful implementation in `os/package_manager.rs:404-421` which checks `mode & 0o111 != 0`.

**Impact:** Could return `true` for a file named `rc-status` that is not executable (e.g., a data file or a script without the execute bit), causing misdetection of the init system.

**Suggested fix:**

```rust
#[cfg(not(windows))]
{
    use std::os::unix::fs::PermissionsExt;
    env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(exe_name);
        candidate.is_file()
            && candidate
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    })
}
```

---

### H2. Linux `detect_storage_impl` doesn't handle escaped mount paths

**File:** `hardware/storage.rs:150-158`
**Platform:** Linux

`/proc/mounts` uses octal escapes for spaces and special characters in mount paths (e.g., `\040` for space, `\011` for tab). The current code does a raw `split_whitespace()` which will:

1. **Split a mount point containing a space into two tokens**, misaligning all subsequent fields.
2. **Pass the escaped path to `statvfs`**, which will fail because the path doesn't exist.

```rust
let parts: Vec<&str> = line.split_whitespace().collect();
// parts[1] may be truncated: "/mnt/My" instead of "/mnt/My Files"
```

**Impact:** Mount points with spaces (e.g., `/media/user/My Drive`) will be either silently skipped or produce incorrect results.

**Suggested fix:** Add an unescape function:

```rust
fn unescape_mount_path(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Read up to 3 octal digits
            let mut octal = String::new();
            for _ in 0..3 {
                // peek isn't available on Chars, so use a different approach
                // or parse the bytes directly
            }
            // ... parse octal escape
        } else {
            result.push(c);
        }
    }
    result
}
```

A simpler approach: use `mountinfo` (`/proc/self/mountinfo`) instead of `/proc/mounts` — it uses a well-defined field format with an explicit separator (`-`) and properly escapes special characters.

---

### H3. `parse_linux_default_route_interface` only compiled on Linux

**File:** `network/mod.rs:531-540`
**Platform:** Linux (compile-time gating issue)

The function `parse_linux_default_route_interface` is gated with `#[cfg(target_os = "linux")]`, but the unit test at line 1205 is also gated with `#[cfg(target_os = "linux")]`. This means:

1. The function's parsing logic is **never tested on CI** unless Linux CI is set up.
2. Compare with `parse_bsd_default_route_interface` (line 513) which includes `test` in its cfg: `#[cfg(any(target_os = "macos", ..., test))]` — this allows the function to compile and be tested on any platform.

**Impact:** Parsing bugs in the Linux network route code would go undetected.

**Suggested fix:** Add `test` to the cfg gate for the function, matching the BSD pattern:

```rust
#[cfg(any(target_os = "linux", test))]
fn parse_linux_default_route_interface(output: &str) -> Option<String> {
```

And remove the `#[cfg(target_os = "linux")]` from the test:

```rust
#[test]
fn test_parse_linux_default_route_interface() {
    // ...
}
```

---

## Medium Severity Issues

### M1. GPU detection returns empty on Windows and Linux

**File:** `hardware/gpu.rs:265-268`
**Platform:** Windows, Linux

```rust
#[cfg(not(target_os = "macos"))]
pub fn detect_gpus() -> Vec<GpuInfo> {
    Vec::new()
}
```

**Impact:** No GPU information is ever reported on Windows or Linux. While this is documented as intentional ("future: add Vulkan/D3D12 support"), it silently degrades the hardware detection output.

**Suggested fix (Linux, low effort):** Parse `/sys/class/drm/card*/device/` for PCI vendor/device IDs, or parse `lspci -v` output for VGA/3D controllers:

```rust
#[cfg(target_os = "linux")]
pub fn detect_gpus() -> Vec<GpuInfo> {
    detect_gpus_lspci().unwrap_or_default()
}
```

**Suggested fix (Windows, low effort):** Use `wmic path win32_VideoController get Name,AdapterRAM,DriverVersion`:

```rust
#[cfg(target_os = "windows")]
pub fn detect_gpus() -> Vec<GpuInfo> {
    detect_gpus_wmic().unwrap_or_default()
}
```

---

### M2. Audio detection returns empty on Windows and Linux

**File:** `hardware/audio.rs:474-477`
**Platform:** Windows, Linux

Same pattern as GPU — returns empty vector.

**Impact:** No audio device information on Windows or Linux.

**Suggested fix (Linux):** Parse `/proc/asound/cards` or use `aplay -l`:

```rust
#[cfg(target_os = "linux")]
pub fn detect_audio_devices() -> Vec<AudioDeviceInfo> {
    detect_audio_alsa().unwrap_or_default()
}
```

**Suggested fix (Windows):** Use `wmic path Win32_SoundDevice get Name,Status` or PowerShell `Get-AudioDevice`.

---

### M3. Windows NTP status detection is unimplemented

**File:** `os/time.rs:325-329`
**Platform:** Windows

```rust
#[cfg(target_os = "windows")]
pub fn detect_ntp_status() -> NtpStatus {
    NtpStatus::Unknown
}
```

**Impact:** NTP status is never reported on Windows.

**Suggested fix:** Use `w32tm /query /status`:

```rust
#[cfg(target_os = "windows")]
pub fn detect_ntp_status() -> NtpStatus {
    let output = run_command_with_timeout("w32tm", &["/query", "/status"], 5);
    match output {
        Some(text) if text.contains("Leap Indicator: 0") => NtpStatus::Synchronized,
        Some(text) if text.contains("Leap Indicator:") => NtpStatus::Unsynchronized,
        _ => NtpStatus::Unknown,
    }
}
```

---

### M4. macOS NTP status always returns `Unknown`

**File:** `os/time.rs:318-323`
**Platform:** macOS

```rust
#[cfg(target_os = "macos")]
pub fn detect_ntp_status() -> NtpStatus {
    NtpStatus::Unknown
}
```

**Impact:** NTP status is never reported on macOS, even though it's the primary development platform.

**Suggested fix:** Use `systemsetup -getusingnetworktime` (requires admin on newer macOS) or `sntp -d time.apple.com`:

```rust
#[cfg(target_os = "macos")]
pub fn detect_ntp_status() -> NtpStatus {
    // systemsetup may require elevated privileges on macOS 13+
    let output = run_command_with_timeout(
        "systemsetup", &["-getusingnetworktime"], 5
    );
    match output.as_deref().map(str::trim) {
        Some(s) if s.contains("On") => NtpStatus::Synchronized,
        Some(s) if s.contains("Off") => NtpStatus::Inactive,
        _ => NtpStatus::Unknown,
    }
}
```

---

### M5. Windows `detect_windows_package_managers` may misidentify MSYS2 pacman

**File:** `os/package_manager.rs:1194`
**Platform:** Windows

The MSYS2 pacman detection hardcodes `C:\msys64\usr\bin\pacman.exe`:

```rust
let msys2_pacman_path = PathBuf::from(r"C:\msys64\usr\bin\pacman.exe");
```

**Impact:** MSYS2 installed to a non-default location (e.g., `D:\msys64` or `C:\msys32` for 32-bit) will not be detected. More importantly, if native `pacman` is on PATH (e.g., from an Arch WSL integration), `command_exists_in_path("pacman", &path_dirs)` isn't called at all — the only pacman check is the hardcoded path.

**Suggested fix:** Also check PATH for pacman and distinguish by inspecting the path:

```rust
// Check MSYS2 at known locations
let msys2_paths = [
    PathBuf::from(r"C:\msys64\usr\bin\pacman.exe"),
    PathBuf::from(r"C:\msys32\usr\bin\pacman.exe"),
];
let msys2_found = msys2_paths.iter().find(|p| p.is_file());

// Also check PATH
if let Some(path) = msys2_found.or_else(|| command_exists_in_path("pacman", &path_dirs).as_ref()) {
    // ...
}
```

---

## Low Severity Issues

### L1. Locale detection uses only Unix-style environment variables

**File:** `os/locale.rs:82-86`
**Platform:** Windows

The `detect_locale()` function reads `LANG`, `LC_ALL`, `LC_CTYPE`, etc. These environment variables are standard on Unix but are **not typically set on Windows**. Windows uses the system locale configured via the Control Panel / Settings app.

**Impact:** On a default Windows installation, all locale fields will be `None`. Only MSYS2/Cygwin/WSL environments set these variables.

**Suggested fix:** On Windows, fall back to reading the system locale:

```rust
#[cfg(target_os = "windows")]
fn detect_windows_locale() -> Option<(String, String)> {
    // Use `Get-Culture` via powershell or the Win32 API
    // GetUserDefaultLocaleName / GetSystemDefaultLocaleName
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", "(Get-Culture).Name"])
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
    // "en-US" format — convert to POSIX-like "en_US"
    Some((name.replace('-', "_"), "UTF-8".to_string()))
}
```

---

### L2. `StorageKind` never populated on macOS or Linux

**File:** `hardware/storage.rs:103,191`
**Platform:** macOS, Linux

Both the macOS and Linux `detect_storage_impl` functions always set `kind: StorageKind::Unknown`. Only the `sysinfo` fallback (used on Windows and other platforms) populates `StorageKind::Ssd` / `StorageKind::Hdd`.

```rust
// macOS (line 103)
kind: StorageKind::Unknown,

// Linux (line 191)
kind: StorageKind::Unknown,
```

**Impact:** Storage type (SSD vs HDD) is never reported on macOS or Linux, despite the platform-specific implementations being used specifically to avoid `sysinfo` overhead.

**Suggested fix (Linux):** Check `/sys/block/{dev}/queue/rotational` — `0` = SSD, `1` = HDD:

```rust
fn detect_storage_kind_linux(device: &str) -> StorageKind {
    let dev_name = Path::new(device)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let rotational_path = format!("/sys/block/{dev_name}/queue/rotational");
    match std::fs::read_to_string(&rotational_path) {
        Ok(s) if s.trim() == "0" => StorageKind::Ssd,
        Ok(s) if s.trim() == "1" => StorageKind::Hdd,
        _ => StorageKind::Unknown,
    }
}
```

**Suggested fix (macOS):** Use `diskutil info` or IOKit to query the `SolidState` property.

---

### L3. Git system config path only supplemented on macOS

**File:** `filesystem/git/detection.rs:526-534`
**Platform:** Windows, Linux

```rust
#[cfg(target_os = "macos")]
{
    let macos_system = std::path::Path::new(
        "/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig",
    );
    if macos_system.exists() {
        let _ = config.add_file(macos_system, git2::ConfigLevel::ProgramData, false);
    }
}
```

On Windows, Git for Windows installs a system-level gitconfig at `C:\Program Files\Git\etc\gitconfig` which libgit2 may not find automatically via its `ProgramData` search.

**Impact:** On Windows, `credential.helper` and other system-level git settings may not be detected.

**Suggested fix:**

```rust
#[cfg(target_os = "windows")]
{
    // Git for Windows system gitconfig
    let git_for_windows = std::path::Path::new(
        r"C:\Program Files\Git\etc\gitconfig"
    );
    if git_for_windows.exists() {
        let _ = config.add_file(git_for_windows, git2::ConfigLevel::ProgramData, false);
    }
}
```

---

## Summary Table

| ID | Severity | Platform | File | Description |
|----|----------|----------|------|-------------|
| C1 | Critical | Windows | `services/mod.rs` | Windows service listing always empty |
| C2 | Critical | Windows | `network/mod.rs` | Default route detection unimplemented |
| C3 | Critical | Windows | `os/time.rs` | Timezone detection unimplemented |
| H1 | High | Linux/macOS | `services/mod.rs` | `has_in_path` doesn't check executable bit |
| H2 | High | Linux | `hardware/storage.rs` | Escaped mount paths break parsing |
| H3 | High | Linux | `network/mod.rs` | Linux route parser not testable on macOS CI |
| M1 | Medium | Win/Linux | `hardware/gpu.rs` | GPU detection returns empty |
| M2 | Medium | Win/Linux | `hardware/audio.rs` | Audio detection returns empty |
| M3 | Medium | Windows | `os/time.rs` | NTP status unimplemented |
| M4 | Medium | macOS | `os/time.rs` | NTP status unimplemented |
| M5 | Medium | Windows | `os/package_manager.rs` | Hardcoded MSYS2 path |
| L1 | Low | Windows | `os/locale.rs` | Locale detection Unix-only |
| L2 | Low | macOS/Linux | `hardware/storage.rs` | StorageKind never populated |
| L3 | Low | Windows | `filesystem/git/detection.rs` | Windows system gitconfig not supplemented |

---

## Recommendations

### Quick wins (can fix now, no new dependencies)

1. **H1** — Add executable-bit check to `has_in_path()` (5-line change)
2. **H3** — Add `test` to `parse_linux_default_route_interface` cfg gate (1-line change)
3. **L2** — Add `/sys/block/*/queue/rotational` check for Linux storage kind

### Medium effort (new code, existing tools)

4. **C1** — Implement `list_windows_scm_services()` via `sc query`
5. **C2** — Implement Windows default route via `route print`
6. **C3** — Implement `detect_timezone_name()` on Windows via `tzutil /g`
7. **M3** — Implement Windows NTP via `w32tm /query /status`
8. **M4** — Implement macOS NTP via `systemsetup -getusingnetworktime`

### Higher effort (new dependencies or complex parsing)

9. **M1/M2** — GPU/Audio detection on Linux (`lspci` / `/proc/asound`) and Windows (`wmic` / PowerShell)
10. **H2** — Switch Linux storage to `/proc/self/mountinfo` or add octal unescape
11. **L1** — Windows locale via PowerShell `Get-Culture` or Win32 API

### Testing recommendations

- Add a CI matrix that includes Windows and Linux (GitHub Actions supports all three)
- For unit tests with platform-specific parsing, use the `test` cfg gate pattern (like `parse_bsd_default_route_interface`) so parsers are always compiled and tested
- Consider adding mock/fixture-based tests for all platform command parsers to avoid requiring actual platform presence
