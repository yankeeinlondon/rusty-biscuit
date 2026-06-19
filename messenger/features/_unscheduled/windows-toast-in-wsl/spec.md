# Spec: Windows Toast Notifications from WSL

## Background

When `messenger` (or any consumer of the `messenger` desktop provider) runs inside WSL, `target_os = "linux"` is true at compile time. The provider therefore selects the Linux backend (`notify-rust` D-Bus + `dunstify`/`notify-send` helpers). On a typical WSL installation without a Linux desktop environment or D-Bus daemon, this produces either:

- A transport error: "D-Bus notification failed" (no `org.freedesktop.Notifications` service)
- A silently dropped notification (D-Bus present but no display server attached)

The user experience is poor because the host Windows OS *can* display toast notifications — the binary is simply talking to the wrong subsystem.

## Goal

When the desktop provider detects it is running inside WSL, it should deliver notifications through the **Windows toast subsystem** instead of the Linux D-Bus subsystem. The WSL user sees native Windows toast banners, action center entries, and notification sounds.

## Constraints

- `winrt-notification` and other `#[cfg(target_os = "windows")]` crates **cannot** be used; WSL compiles as Linux.
- WSL-to-Windows delivery must use **WSL interop** (calling Windows executables from the Linux side).
- The feature must not break native Linux behavior when `WSL_DISTRO_NAME` is absent.

## Detection

WSL is detected at **runtime** inside `select_backend` (or a new `detect_wsl()` helper) by checking, in order:

1. Environment variable `WSL_DISTRO_NAME` is set and non-empty.
2. File `/proc/sys/fs/binfmt_misc/WSLInterop` exists.
3. `uname -r` contains `"microsoft-standard-wsl"` or `"microsoft-standard"`.

If any check passes, the host is WSL. Detection is cheap (env lookup + one `Path::exists` call) and runs once at backend construction.

## Backend Selection

`select_backend` changes from pure compile-time to compile-time + runtime:

```
#[cfg(target_os = "linux")]
if detect_wsl() {
    Arc::new(wsl::WslBackend::new(config))
} else {
    Arc::new(linux::LinuxBackend::new(config.linux.clone()))
}
```

`WslBackend` reports `DesktopPlatform::Windows` (or a new `DesktopPlatform::Wsl`) in receipts so callers know which surface handled the notification.

## Delivery Strategy

`WslBackend` tries delivery paths in priority order, mirroring the helper-election pattern used by the native Windows and Linux backends.

### 1. Windows helper binaries via WSL interop

WSL automatically resolves `.exe` files against the Windows `PATH`. The backend probes for:

- `snoretoast.exe` — if installed on Windows (e.g., via Chocolatey, Scoop, or manual install)
- A future `messenger-toast.exe` stub we could ship

Helper election uses the same `score()` + `send()` interface as `snoretoast.rs`; the only difference is the executable is invoked via `std::process::Command` from WSL rather than native Windows.

### 2. PowerShell toast fallback

If no helper scores above zero, the backend falls back to `powershell.exe` with an inline toast script. PowerShell is guaranteed on every Windows host, making this the universal floor.

The script uses `Windows.UI.Notifications` (no external modules required):

```powershell
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
$xml = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02)
$xml.SelectSingleNode("//text[@id='1']").AppendChild($xml.CreateTextNode('TITLE')) | Out-Null
$xml.SelectSingleNode("//text[@id='2']").AppendChild($xml.CreateTextNode('BODY')) | Out-Null
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('APP_NAME').Show($toast)
```

Trade-offs of the PowerShell floor:
- **No actions/buttons** — `Windows.UI.Notifications` XML template handles title + body only.
- **No replacement** — each toast is independent; no ID to update.
- **No progress** — unsupported by the basic template.
- **Reliable** — always available.

### 3. Linux D-Bus fallback

If WSL interop is disabled (`interop.enabled = false` in `wsl.conf`) or `powershell.exe` is not found, the backend degrades gracefully to the standard Linux `notify-rust` path. This preserves behavior for locked-down WSL environments.

## Scope

### In scope

- `messenger/lib/src/provider/desktop/wsl.rs` — new backend module.
- `messenger/lib/src/provider/desktop/mod.rs` — runtime WSL detection + backend routing.
- `messenger/lib/src/provider/desktop/helpers/wsl_snoretoast.rs` — WSL interop wrapper for `snoretoast.exe`.
- `messenger/lib/src/provider/desktop/helpers/wsl_powershell.rs` — PowerShell toast fallback helper.
- `DesktopPlatform` enum: add `Wsl` variant (or reuse `Windows` with metadata distinction).
- Receipt metadata: `delivery=wsl-helper` or `delivery=wsl-powershell` so callers can tell which path served the notification.
- Unit tests for `detect_wsl()` using env-var mocking.
- Cross-platform tests: the WSL backend module compiles on macOS/Linux with `#[cfg_attr(not(target_os = "linux"), allow(dead_code))]`.

### Out of scope

- Re-implementing `winrt-notification` functionality in pure Rust for Linux target.
- Shipping a custom `messenger-toast.exe` Windows binary (can be added later as a follow-up).
- Interactive actions/replies via PowerShell ( limitation of the fallback; helpers can add it later).
- WSL1 support (WSL2 only; WSL1 detection can be added if requested).
- Changing native Windows or native Linux behavior.

## Data Flow

```
User in WSL runs: messenger send "hello" --route desktop

DesktopNotificationProvider::new
  └─ select_backend
       └─ detect_wsl() → true
       └─ WslBackend::new(config)
            ├─ detect_windows_helpers() → snoretoast.exe? score?
            └─ fallback: PowerShell toast script

WslBackend::send(request)
  ├─ Try snoretoast.exe (if present + score > 0)
  ├─ Try powershell.exe toast script
  └─ Try native Linux D-Bus (if interop disabled)
```

## CLI / Library Impact

No CLI changes. A WSL user continues to run:

```bash
messenger send "Deployment complete" --route desktop
```

The difference is entirely internal: they now see a Windows toast instead of a D-Bus error.

Library callers are unaffected. `DesktopNotificationProvider::new` handles detection automatically.

## Configuration

No new configuration required. Optional future additions:

- `linux.wsl_prefer_windows_toast = false` — opt-out to force Linux D-Bus even in WSL.
- `windows.app_id` — already exists; reused by the PowerShell toast notifier name.

## Acceptance Criteria

1. `detect_wsl()` returns `true` inside WSL and `false` on native Linux/macOS.
2. `cargo check -p messenger --all-features` passes on macOS, native Linux, and native Windows.
3. `cargo test -p messenger --lib` passes; WSL backend tests use fake `Command` injection (no real WSL required in CI).
4. On a real WSL2 host with `desktop` feature enabled:
   - `messenger send "test" --route desktop` produces a visible Windows toast.
   - Receipt metadata contains `helper_used=wsl-powershell` (or `wsl-snoretoast` if helper is installed).
5. On native Linux (non-WSL), behavior is unchanged: D-Bus/`notify-rust` path is used.
6. On WSL with interop disabled (`wsl.conf` has `interop.enabled = false`), the backend falls back to Linux D-Bus without panicking.
