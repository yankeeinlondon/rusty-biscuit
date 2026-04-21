# Desktop Notifications Platform Guide

The `desktop` provider delivers a local OS notification to the current host's notification center. It is a single library provider (`DesktopNotificationProvider`) that picks its backend at construction time based on the compile target.

| Host | Backend crate | Delivery path |
|------|---------------|---------------|
| Linux | `notify-rust` | D-Bus (freedesktop.org Notifications spec) |
| macOS | `objc2-user-notifications` (opt-in) or `osascript` | `UserNotifications.framework` or AppleScript |
| Windows | `winrt-notification` | WinRT toast |

Desktop is the only provider that does not need credentials or a destination identifier — the target is the host OS itself. The CLI therefore accepts `--provider desktop` without `--channel`.

## Capability Summary

- `supports_markdown_rendering`: `false` — rendered to plain text
- `supports_reply`: `false`
- `supported_attachment_kinds`: `{ Image }` — non-image attachments drop in best-effort and error in strict
- `supports_location`: `false` — location drops in best-effort and errors in strict
- `supports_silent_delivery`: `true`
- `supports_link_preview_control`: `false`

A title-only message (`Message::title(...)` with no body) is valid only when the resolved provider is `desktop`. Every other provider still rejects an empty body.

## Enabling

Library:

```toml
[dependencies]
messenger = { version = "0.1", features = ["desktop"] }
```

CLI: `messenger-cli` enables `desktop` by default.

Platform-specific dependencies pulled in by the `desktop` feature:

| Target | Crates |
|--------|--------|
| `target_os = "linux"` | `notify-rust` |
| `target_os = "macos"` | `objc2-foundation`, `objc2-user-notifications` |
| `target_os = "windows"` | `winrt-notification` |
| shared | `uuid` |

## Linux (D-Bus)

Delivery goes through the freedesktop.org Notifications interface on the session bus. Any compliant daemon works — GNOME, KDE Plasma, XFCE, MATE, Cinnamon, Dunst, and similar.

Portable fields map onto D-Bus as follows:

| Portable | D-Bus |
|----------|-------|
| `title` | `summary` |
| `body` | `body` |
| `app_name` | `app_name` |
| `icon` (`Named` or `Path`) | icon name or absolute path |
| `urgency` (`Low`/`Normal`/`Critical`) | `urgency` hint |
| `category` | `category` hint |
| `desktop_entry` (config) | `desktop-entry` hint |
| first image attachment | `image-path` hint |
| `silent` | `suppress-sound` hint |
| `timeout_ms` | `expire_timeout` |

The daemon returns a numeric notification ID, which becomes `MessageRef::Desktop { notification_id }`. Errors talking to D-Bus surface as `MessengerError::Transport`.

No permission prompts or setup steps are required on Linux.

## macOS (AppleScript or native)

macOS uses one of two explicit strategies, driven by `MacOsDesktopConfig::strategy`:

| Strategy | Backend | Authorization | Notes |
|----------|---------|---------------|-------|
| `Auto` (default) | AppleScript via `osascript` | None | v1 maps `Auto` to AppleScript unconditionally. No notification center authorization prompt is ever triggered. |
| `AppleScript` | AppleScript via `osascript` | None | Explicit alias for the default. |
| `NativeUserNotifications` | `UserNotifications.framework` | Required | Needs a bundled, signed app identity to succeed. Unbundled CLI runs typically see their notifications silently dropped. |

Why the default is AppleScript:

- A `cargo install`ed CLI binary has no bundle identity and no persistent authorization story.
- Heuristic "am I bundled?" detection misfires when the binary is launched from a terminal inside an IDE.
- An implicit authorization prompt on first `send` is bad UX in scripted or CI contexts.

Receipt metadata includes a `delivery` key — `applescript` or `native` — so callers can tell which path was taken.

Portable fields on the AppleScript path collapse to title/body only; `subtitle`, `icon`, and images are best-effort and may be ignored. The native path maps `title` → `title`, `subtitle` → `subtitle`, `body` → `body`, first image → attachment, `category` → `categoryIdentifier`, and may later map `urgency` → `interruptionLevel`.

Both paths return a UUID as `notification_id`; AppleScript does not expose a system handle, and the native submit path is fire-and-forget. `MessageRef::Desktop` on macOS therefore preserves the UUID so future versions can add replacement/dismissal without changing the type.

## Windows (WinRT toast)

Windows toasts ship through `winrt-notification`. Unpackaged Win32 binaries need two things to render a toast:

1. An App User Model ID (AUMID) — defaults to `RustyBiscuit.Messenger`.
2. A Start Menu shortcut whose filename matches `<app_id>.lnk`, pointing at the `messenger` executable.

Both are created by `messenger setup desktop`. The shortcut is written under `%APPDATA%\Microsoft\Windows\Start Menu\Programs\<app_id>.lnk`. Setup prints the absolute shortcut path on success.

`send` never writes outside `~/.messenger/`. When the prerequisites are missing, the Windows backend fails eagerly:

```text
MissingConfiguration {
    provider: Desktop,
    field: "Windows desktop notifications require `messenger setup desktop` to register the Start Menu shortcut and App User Model ID",
}
```

Mapping notes:

| Portable | WinRT toast |
|----------|-------------|
| `title` | toast title |
| `body` first line | `text1` |
| remaining body lines | `text2` |
| first image attachment | toast image |
| `silent` | `sound(None)` / `sound(Some(Sound::Default))` |

Categories, replacement IDs, and millisecond-accurate timeouts are optional in v1 and may be dropped.

On success the backend generates a UUID for `notification_id` and records `metadata["delivery"] = "winrt"`.

## iOS and Android

Intentionally out of scope for the `desktop` provider. Mobile push will land as separate `apns` and `fcm` providers — the auth model, targeting model, and delivery guarantees differ enough that sharing the desktop adapter would be misleading. See the spec's Rollout Plan (Phase 4) for context.

## Troubleshooting

- **Linux:** nothing appears. Check that a notification daemon is running (`busctl --user call org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications GetServerInformation`). Errors from D-Bus surface as `MessengerError::Transport`.
- **macOS:** notification appears briefly and disappears without landing in the notification center. This is normal AppleScript behavior on macOS 11+ for unbundled senders; use `strategy: native_user_notifications` with a bundled, signed app if persistence is required.
- **Windows:** `send` returns `MissingConfiguration`. Run `messenger setup desktop` to recreate the Start Menu shortcut. Confirm the shortcut exists at `%APPDATA%\Microsoft\Windows\Start Menu\Programs\<app_id>.lnk` and that its target points at the `messenger` executable.

## Related Documents

- [User Guide](../user-guide.md) — platform setup, CLI configuration, library usage.
- [messenger README](../../README.md) — high-level package overview.
- [messenger-cli README](../../cli/README.md) — CLI flags, route shapes, setup flow.
