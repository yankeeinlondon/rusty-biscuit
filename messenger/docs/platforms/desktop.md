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

## Quick Test

Send a test notification to verify the provider is working:

```bash
messenger send --provider desktop --title "Test Notification" "Hello from messenger"
```

Platform-specific notes:

- **Linux / macOS** — no prior setup is required; the command works immediately.
- **Windows** — run `messenger setup desktop` first to create the required Start Menu shortcut and App User Model ID.

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

### The "Show" Button

macOS notifications include a default **Show** button that appears on hover. What it does depends on the delivery strategy:

| Strategy | "Show" Behavior | Controllable? |
|----------|-----------------|---------------|
| **AppleScript** | Opens **Script Editor** (the host process of `osascript`) | **No** — AppleScript offers no API to override the default action. The notification is associated with the `osascript` process, and the system routes the click to its parent application. |
| **NativeUserNotifications** | Does nothing by default (fire-and-forget) | **Partially** — You can register `UNNotificationAction` buttons via `UNNotificationCategory`, but the default "Show" behavior for a plain notification with no category still has no effect. |

If you need actionable notifications on macOS, you must use the **NativeUserNotifications** strategy, bundle the binary into a signed `.app`, and register a `UNNotificationCategory` with explicit actions. See [Moving to Native UserNotifications](#moving-to-native-usernotifications-on-macos) below.

### Moving to Native UserNotifications on macOS

The AppleScript strategy works out of the box but has limited features and the unintuitive "Show → Script Editor" behavior. To unlock the full macOS notification API (action buttons, replacement, dismissal, interruption levels), you need to switch to the native `UserNotifications.framework` backend. This requires more than a config change — it requires a **bundled, signed application identity**.

**Prerequisites**

- macOS 10.14+ (UserNotifications framework requirement)
- Apple Developer ID or personal signing certificate
- The `messenger` binary must be inside a `.app` bundle (see step 2)

**Step-by-step migration**

1. **Switch strategy in config**
   Update your desktop route to use the native strategy:
   ```json
   {
     "routes": {
       "desktop.local": {
         "provider": "desktop",
         "macos": {
           "strategy": "native_user_notifications",
           "bundle_id": "com.yourorg.messenger"
         }
       }
     }
   }
   ```

2. **Bundle the binary**
   The native framework rejects notifications from unpackaged processes. Create a minimal app bundle:
   ```bash
   APP="Messenger.app"
   mkdir -p "$APP/Contents/MacOS"
   cp $(which messenger) "$APP/Contents/MacOS/"
   cat > "$APP/Contents/Info.plist" <<'EOF'
   <?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
   <plist version="1.0">
   <dict>
     <key>CFBundleIdentifier</key>
     <string>com.yourorg.messenger</string>
     <key>CFBundleName</key>
     <string>Messenger</string>
     <key>CFBundleExecutable</key>
     <string>messenger</string>
     <key>CFBundleVersion</key>
     <string>1.0</string>
     <key>LSUIElement</key>
     <true/>
   </dict>
   </plist>
   EOF
   ```
   The `LSUIElement` key prevents a dock icon from appearing.

3. **Code-sign the bundle**
   Sign with your Developer ID or ad-hoc certificate:
   ```bash
   codesign --force --deep --sign "Developer ID Application: Your Name" Messenger.app
   ```
   For local testing, you can use ad-hoc signing:
   ```bash
   codesign --force --deep --sign - Messenger.app
   ```

4. **Request notification authorization**
   On first launch, the system will prompt the user to allow notifications from your app. The current native implementation is fire-and-forget and does not explicitly request authorization — the prompt appears automatically on first `send`. For production apps, you should call `requestAuthorizationWithOptions:completionHandler:` in a proper app delegate before sending.

5. **Run from the bundle**
   Always invoke the bundled binary, not the raw `cargo install`ed executable:
   ```bash
   ./Messenger.app/Contents/MacOS/messenger send --provider desktop --title "Hello" "Native notification"
   ```

**Known limitations of the current native implementation**

- **Actions are not wired**: The `actions` field on `DesktopConfig` / `DesktopOverrides` is accepted but not passed through to `UNNotificationCategory`. Adding action buttons requires extending `send_native_with_id` to create a `UNNotificationCategory`, register it with `setNotificationCategories:`, and attach it to the `UNMutableNotificationContent`.
- **No completion handler**: Delivery is fire-and-forget. The receipt's `metadata["delivery_confirmed"]` is always `"false"`.
- **Replace/dismiss are available**: Unlike AppleScript, the native backend supports `replace` and `dismiss` via `UNUserNotificationCenter`.
- **Urgency is not mapped**: `NotificationUrgency` is not yet translated to `interruptionLevel`.

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
