---
prompt: "To aid in sending desktop notifications, the **messenger** library will evaluate the host to see if any the known\n\"helper utilities\" for the host's OS are available to be used. Your task is to do deep research into the \ncapabilities of each of the utilities listed below.\n\nUtilities to research:\n\n- macOS\n    - [alerter](https://github.com/vjeantet/alerter)\n    - [terminal-notifier](https://github.com/julienXX/terminal-notifier)\n- Windows\n    - BurntToast\n    - SnoreToast\n- Linux\n    - notify-send\n    - dunstify\n    - kdialog\n\nFor each utility:\n\n- provide key URL's for documentation\n- mention the installation methods which are typically used to install\n- give an overview of it's functionality\n    - Plain text messages? Markdown? Hyperlinks?\n    - Image support?\n    - Reply/Action buttons?\n    - Sound Effect\n- give an example of how it would be called\n- discuss how well it fits into the goals of the **Messenger** library\n\nFor each OS:\n\n- which of the OS's utilities we researched who you use first when multiple are installed on the host? why?\n\nUse `messenger` skill while doing this research."
last_updated: 2026-04-26
---
Researching each utility now. I'll compose comprehensive findings using my knowledge plus targeted verification.Composing the research document now with verified facts.

# Desktop Notification Helpers — Research

The `messenger` library's desktop provider already speaks the platform-native APIs (D-Bus on Linux, AppleScript / `NSUserNotification` on macOS, WinRT on Windows). However, several feature gaps remain when going through native APIs alone — most notably **action buttons**, **inline reply text**, **rich icons**, and **sound selection**. A small set of well-known third-party CLIs fill those gaps. When detected on the host, `messenger` should opportunistically use them to expand the desktop provider's capability set.

This document catalogs each candidate utility with its capabilities, install path, an invocation example, and a verdict on its fit with the `messenger` API.

---

## macOS

### terminal-notifier

A non-blocking notification dispatcher that targets macOS Notification Center. Originally the de-facto standard, but its interactive features (reply field, action buttons) were rolled back in the **2.0.0** release because they relied on private Apple APIs that began failing on signing/notarization. As of recent versions, it is best treated as a **fire-and-forget** poster.

**Documentation**

- Project: <https://github.com/julienXX/terminal-notifier>
- Releases: <https://github.com/julienXX/terminal-notifier/releases>
- Ruby gem wrapper: <https://rubygems.org/gems/terminal-notifier>

**Installation**

| Method   | Command                                                                                |
|----------|----------------------------------------------------------------------------------------|
| Homebrew | `brew install terminal-notifier`                                                       |
| MacPorts | `sudo port install terminal-notifier`                                                  |
| RubyGems | `gem install terminal-notifier`                                                        |
| Manual   | Download `.tar.bz2` from Releases and place `terminal-notifier.app` in `/Applications` |

**Functionality**

| Feature                | Support                                                                                                                            |
|------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| Plain text body        | ✅ via `-message`                                                                                                                  |
| Markdown / HTML        | ❌                                                                                                                                 |
| Hyperlinks (clickable) | ⚠️ Indirect — `-open URL` makes the **whole** notification clickable                                                               |
| Inline image           | ✅ `-contentImage PATH` (right-side image)                                                                                         |
| App icon override      | ✅ `-appIcon PATH` (private API; sometimes ignored on signed builds)                                                               |
| Action buttons         | ❌ Removed in 2.0.0                                                                                                                |
| Reply text input       | ❌ Removed in 2.0.0                                                                                                                |
| Sound                  | ✅ `-sound NAME` — any file from `/System/Library/Sounds` (e.g. `Basso`, `Glass`, `Ping`, `Pop`, `Submarine`, `Tink`) or `default` |
| Group / replace        | ✅ `-group ID` plus `-remove ID`                                                                                                   |
| Click action           | ✅ `-execute COMMAND`, `-open URL`, or `-activate BUNDLE_ID`                                                                       |
| Bypass DND             | ✅ `-ignoreDnD`                                                                                                                    |

**Example**

```bash
terminal-notifier \
    -title "Deploy succeeded" \
    -subtitle "api-service" \
    -message "All checks green. Tap to open the build." \
    -sound Glass \
    -contentImage ~/icons/check.png \
    -open "https://ci.example.com/build/4192" \
    -group "deploy.api-service"
```

**Fit with messenger**

Strong fit for the **send** path. It cleanly extends the desktop provider with: sound selection, content-image, click-to-open URL, and stable `group_id` semantics (the existing `Dispatch.group_id` field maps directly to `-group`). It does **not** support `actions` or reply input, so it cannot satisfy the "interactive" desktop dreams listed in the parent spec — those belong to `alerter`.

---

### alerter

A Swift rewrite (originally a fork of terminal-notifier) that stays in the foreground until the user dismisses, replies, or picks an action — making it the right tool when `messenger` actually needs a **response** rather than just a notice. It is **blocking** by design, which is the opposite contract of `terminal-notifier`.

**Documentation**

- Project: <https://github.com/vjeantet/alerter>
- Homebrew tap: <https://github.com/vjeantet/homebrew-tap>

**Installation**

| Method         | Command                                                     |
|----------------|-------------------------------------------------------------|
| Homebrew (tap) | `brew install vjeantet/tap/alerter`                         |
| MacPorts       | `sudo port install alerter`                                 |
| Manual         | Download release `.zip`, drop the binary anywhere on `PATH` |

Requires macOS 13 (Ventura) or newer. Modern releases use **double-dash** flags (`--reply`, `--actions`).

**Functionality**

| Feature            | Support                                                                    |
|--------------------|----------------------------------------------------------------------------|
| Plain text body    | ✅ `--message`                                                             |
| Markdown / HTML    | ❌                                                                         |
| Hyperlinks         | ❌ — but the alert body click is observable as `@CONTENTCLICKED`           |
| Inline image       | ✅ `--contentImage`                                                        |
| App icon override  | ✅ `--appIcon`                                                             |
| Action buttons     | ✅ — single dropdown via `--actions "Yes,No,Maybe"` plus `--dropdownLabel` |
| Reply text input   | ✅ `--reply "Type your reply"` (returns the typed text on stdout)          |
| Sound              | ✅ `--sound NAME` (same name set as terminal-notifier)                     |
| Close button label | ✅ `--closeLabel`                                                          |
| Output format      | Plain text by default, structured event object with `--json`               |
| Cancel handling    | ✅ Closes notification gracefully on SIGINT / SIGTERM                      |
| Timeout            | ❌ (caller must enforce)                                                   |
| Schedule / delay   | ❌                                                                         |
| Bypass DND         | ❌                                                                         |

**Stdout return values** (plain mode)

| Output            | Meaning                                           |
|-------------------|---------------------------------------------------|
| `@CLOSED`         | User dismissed without acting                     |
| `@CONTENTCLICKED` | User clicked the body of the alert                |
| `@TIMEOUT`        | Banner expired                                    |
| `@ACTIONCLICKED`  | User picked the dropdown action                   |
| _label text_      | The picked action label, when `--actions` is used |
| _typed text_      | The typed reply string, when `--reply` is used    |

**Example**

```bash
reply=$(alerter \
    --title "Incoming message" \
    --subtitle "from @ken" \
    --message "Heading out — back at 7?" \
    --reply "Reply…" \
    --closeLabel "Ignore" \
    --sound Tink \
    --json)
echo "$reply"
# {"activationType":"replied","activationValue":"Yes, see you then."}
```

**Fit with messenger**

The right complement to `terminal-notifier`: it gives the desktop provider real **action button** and **reply** semantics, which `Dispatch.actions` already models. Because it is blocking, it should be invoked from a worker task and the result attached to the `SendReceipt` (e.g. via `metadata["activation"]`). The `--json` mode is the preferred parsing target — easier than scraping the `@`-prefixed sentinel values. Useful only on the **send** path; `replace`/`dismiss` are not in alerter's vocabulary.

### macOS priority

When both are installed, `messenger` should choose **`terminal-notifier` by default**, falling back to `alerter` only when the dispatch actually needs interactivity. Reasoning:

1. The vast majority of desktop sends are notice-only. `terminal-notifier` is non-blocking and fire-and-forget — it matches the asynchronous send contract everywhere else in `messenger`.
2. `alerter` blocks the calling process until the user reacts. If `messenger` defaulted to it, every desktop send would freeze the caller until dismissed.
3. Use `alerter` when, and only when, the `Dispatch` has `actions`, `reply_to`, or an explicit "interactive" hint.

The detection logic should therefore look like:

```text
if dispatch.needs_interactivity() && alerter present:
    use alerter
elif terminal-notifier present:
    use terminal-notifier
elif alerter present:
    use alerter (alerter can do plain notices too)
else:
    fall back to native AppleScript / NSUserNotification path
```

---

## Windows

### BurntToast

A first-class PowerShell module for Windows 10/11 toast notifications. By far the **richest** capability surface of any helper in this study, but the dependency on PowerShell makes invocation from Rust slightly heavier (`powershell.exe -Command …` cold start ≈ 200–400 ms).

**Documentation**

- Project: <https://github.com/Windos/BurntToast>
- Wiki: <https://github.com/Windos/BurntToast/wiki>
- PSGallery: <https://www.powershellgallery.com/packages/BurntToast>

**Installation**

| Method        | Command                                              |
|---------------|------------------------------------------------------|
| PowerShellGet | `Install-Module -Name BurntToast -Scope CurrentUser` |
| Chocolatey    | `choco install burnttoast-psmodule`                  |
| Manual        | `git clone` + `Import-Module ./BurntToast.psd1`      |

Requires Windows 10 / Server 2019 or newer.

**Functionality**

| Feature                  | Support                                                             |
|--------------------------|---------------------------------------------------------------------|
| Plain text body          | ✅ `-Text` (multiple lines via array)                               |
| Markdown                 | ❌ — but XAML toast templates allow rich layout                     |
| Hyperlinks               | ⚠️ Only via `-Button` with a `-Arguments` URI, not inline           |
| Inline image (logo)      | ✅ `-AppLogo PATH`                                                  |
| Hero image (banner)      | ✅ `-HeroImage PATH`                                                |
| Action buttons           | ✅ `-Button (New-BTButton -Content "Reply" -Arguments "reply://…")` |
| Reply text input         | ✅ `New-BTInput -Id 'reply' -Type Text -Title 'Reply'`              |
| Sound                    | ✅ `-Sound NAME` (Windows toast sounds) or `-Silent`                |
| Progress bar             | ✅ via `-ProgressBar` and `Update-BTNotification`                   |
| Snooze / dismiss buttons | ✅ pre-built helpers                                                |
| Scenarios                | ✅ `Alarm`, `Reminder`, `IncomingCall`, `Urgent` (1.1+)             |
| Schedule                 | ✅ `New-BurntToastNotification -Trigger`                            |
| Activation result        | Delivered via `OnActivated` event handler — **not** stdout          |

**Example**

```powershell
$reply  = New-BTInput   -Id 'reply' -Type Text -PlaceHolderContent 'Reply…'
$send   = New-BTButton  -Content 'Send'    -Arguments 'send'    -Id 'reply'
$ignore = New-BTButton  -Content 'Ignore'  -Arguments 'ignore'

New-BurntToastNotification `
    -Text 'Incoming message','from @ken','Heading out — back at 7?' `
    -AppLogo 'C:\icons\ken.png' `
    -Sound 'IM' `
    -TextBox $reply `
    -Button $send,$ignore
```

**Fit with messenger**

Best-in-class feature set on Windows, mapping cleanly to `Dispatch.actions`, `attachments` (image), `subtitle`, `sound`, and `progress`. The catches:

1. **Activation results** require an event handler in the same PowerShell session — capturing the user's reply from a one-shot CLI invocation needs a small wrapper script that pumps the message loop and writes the result to stdout, or uses Windows protocol activation for cross-process delivery.
2. **Cold start** of `powershell.exe` is non-trivial; the Rust desktop adapter should reuse a long-lived `pwsh` child where possible.
3. Distribution is per-user (`-Scope CurrentUser`); the messenger CLI should *detect* but never *auto-install*.

### SnoreToast

A standalone single-binary toast tool maintained by KDE (used by Krita and KDE-on-Windows). Far lighter than BurntToast and easier to invoke from Rust because it is a plain `.exe` with deterministic exit codes.

**Documentation**

- Project: <https://invent.kde.org/libraries/snoretoast>
- Mirror: <https://github.com/KDE/snoretoast>
- Tarballs: <https://download.kde.org/stable/snoretoast/>

**Installation**

| Method      | Command                                                        |
|-------------|----------------------------------------------------------------|
| vcpkg       | `vcpkg install snoretoast`                                     |
| KDE tarball | Download `.tar.xz` from `download.kde.org`, unpack, run        |
| Source      | CMake build against the WinRT/UWP toast APIs                   |
| Bundled     | Many KDE Windows apps ship `snoretoast.exe` next to the binary |

Note: SnoreToast is **not** in the official Chocolatey or Scoop main buckets. The cleanest end-user path is to bundle it with the messenger CLI.

**Functionality**

| Feature            | Support                                              |
|--------------------|------------------------------------------------------|
| Plain text body    | ✅ `-m TEXT`                                         |
| Markdown           | ❌                                                   |
| Hyperlinks         | ❌                                                   |
| Inline image       | ✅ `-p PATH` (PNG only, ≤ 1024 × 1024, ≤ 200 KB)     |
| Action buttons     | ✅ `-b "Yes;No;Maybe"` (semicolon-separated)         |
| Reply text input   | ✅ `-tb` (mutually exclusive with `-b`)              |
| Sound              | ✅ `-s NAME` (Windows toast sound name) or `-silent` |
| Persistence        | ✅ `-d short\|long`                                   |
| Group / replace    | ✅ `-id ID`                                          |
| AppID branding     | ✅ `-appID ID` (run `--install` first to register)   |
| Pipe communication | ✅ `-pipeName \\.\pipe\…` for async delivery         |

**Exit codes** — the entire reason SnoreToast is great for scripting:

| Code | Meaning                                |
|------|----------------------------------------|
| `-1` | Failure                                |
| `0`  | Success / activated                    |
| `1`  | Hidden                                 |
| `2`  | Dismissed                              |
| `3`  | Timed out                              |
| `4`  | Button pressed (which button → stdout) |
| `5`  | Text entered (typed text → stdout)     |

**Example**

```bat
snoretoast.exe -appID com.ken.messenger ^
               -t "Incoming message" ^
               -m "Heading out — back at 7?" ^
               -p "%USERPROFILE%\icons\ken.png" ^
               -s "Notification.IM" ^
               -b "Reply;Ignore" ^
               -id "msg-4192"
```

**Fit with messenger**

Excellent fit. Single executable, deterministic exit codes, and no PowerShell host. The exit-code-driven activation model is **directly consumable** from the Rust desktop adapter (just inspect `ExitStatus.code()` and read stdout). The PNG-only / 200 KB image limit is restrictive but acceptable since the desktop provider already documents "images only" attachments.

### Windows priority

When both are installed, `messenger` should prefer **SnoreToast** by default. Reasoning:

1. **Process model**: SnoreToast is a one-shot `.exe` with stable exit codes; BurntToast requires hosting `powershell.exe` and pumping a message loop to capture activations. From a library, the SnoreToast contract is far closer to "send a notification, parse the result".
2. **Cold start**: SnoreToast launches in tens of milliseconds; PowerShell host is hundreds.
3. **Bundling**: SnoreToast is a single binary that can be shipped alongside the messenger CLI; BurntToast must be installed per-user into a PowerShell module path.
4. Fall through to **BurntToast** when SnoreToast is unavailable AND the dispatch needs hero images, scenarios (Alarm/IncomingCall), or progress bars — features SnoreToast lacks.

```text
if snoretoast present:
    use snoretoast
elif burnttoast available (Get-Module -ListAvailable BurntToast):
    use burnttoast
else:
    fall back to native WinRT path
```

---

## Linux

### notify-send

The reference CLI for the freedesktop **org.freedesktop.Notifications** D-Bus spec, shipped with `libnotify`. Available on essentially every desktop Linux distribution. Capability is whatever the running notification daemon implements.

**Documentation**

- libnotify: <https://gitlab.gnome.org/GNOME/libnotify>
- Spec: <https://specifications.freedesktop.org/notification-spec/latest/>
- Man page: <https://man.archlinux.org/man/notify-send.1>

**Installation**

| Distro          | Package                     |
|-----------------|-----------------------------|
| Arch            | `pacman -S libnotify`       |
| Debian / Ubuntu | `apt install libnotify-bin` |
| Fedora / RHEL   | `dnf install libnotify`     |
| Alpine          | `apk add libnotify`         |

Action support (`-A`) requires libnotify ≥ **0.7.8** (2021) and a daemon that advertises the `actions` capability.

**Functionality**

| Feature                     | Support                                                                                 |
|-----------------------------|-----------------------------------------------------------------------------------------|
| Plain text body             | ✅                                                                                      |
| Pango markup (limited HTML) | ✅ — `<b>`, `<i>`, `<u>`, `<a href="…">` (daemon-dependent)                             |
| Hyperlinks                  | ✅ via `<a href="…">` (GNOME, KDE Plasma, dunst); stripped by some others               |
| Inline image                | ✅ `-i ICON` (theme name or absolute path); large `image-data` hint via `-h`            |
| App icon                    | ✅ `-n / --app-icon`                                                                    |
| Action buttons              | ✅ `-A NAME=Label` (libnotify 0.7.8+, daemon must support)                              |
| Reply text input            | ❌ (no spec support)                                                                    |
| Sound                       | ✅ `-h string:sound-name:NAME` (XDG sound theme) or `-h string:sound-file:PATH`         |
| Suppress sound              | ✅ `-h boolean:suppress-sound:true`                                                     |
| Urgency                     | ✅ `-u low\|normal\|critical`                                                             |
| Timeout                     | ✅ `-t MS` (some daemons ignore: GNOME Shell, NotifyOSD; Plasma ignores for `critical`) |
| Category                    | ✅ `-c CATEGORY`                                                                        |
| Replace                     | ✅ `-r ID` (capture original ID with `-p`)                                              |
| Transient                   | ✅ `-e`                                                                                 |
| Selected action capture     | ✅ stdout, or write to fd with `--selected-action-fd`                                   |

**Example**

```bash
id=$(notify-send \
    --print-id \
    --app-name "Messenger" \
    --icon ~/icons/ken.png \
    --urgency normal \
    --expire-time 8000 \
    --hint string:sound-name:message-new-instant \
    --action "open=Open" \
    --action "ignore=Ignore" \
    "Incoming message" \
    "From <b>@ken</b>: heading out — back at 7? \
     <a href='https://example.com/m/4192'>Open thread</a>")
echo "Notification id: $id"
```

**Fit with messenger**

Already the foundation of the existing Linux desktop adapter. Adding the `-A` action support, `-r` replace, and sound hint plugs the remaining capability gaps for the `notify-send` shell-out path (or, equivalently, the existing D-Bus call path). Pango markup means the **Plain** text dialect is sufficient for safe rendering, with **Markdown** lowered through a small subset (bold/italic/links).

### dunstify

The companion CLI for the **dunst** notification daemon. It is essentially `notify-send` plus first-class action results, an explicit `--close ID`, and `--wait`. It only does anything useful on hosts where dunst is the active D-Bus notification daemon.

**Documentation**

- Project: <https://github.com/dunst-project/dunst>
- Man page: <https://man.archlinux.org/man/dunstify.1>
- Wiki: <https://github.com/dunst-project/dunst/wiki>

**Installation**

| Distro          | Package                                                     |
|-----------------|-------------------------------------------------------------|
| Arch            | `pacman -S dunst`                                           |
| Debian / Ubuntu | `apt install dunst`                                         |
| Fedora          | `dnf install dunst`                                         |
| From source     | `make` against the dunst tree (dunstify is built alongside) |

`dunstify` is shipped with the dunst package. It will succeed even when another daemon is active, but actions and replace semantics only work end-to-end when **dunst itself owns** `org.freedesktop.Notifications`.

**Functionality**

| Feature            | Support                                                                                                      |
|--------------------|--------------------------------------------------------------------------------------------------------------|
| Plain text body    | ✅                                                                                                           |
| Pango markup       | ✅ — same subset as notify-send (configurable in dunst)                                                      |
| Hyperlinks         | ✅ when dunst's `markup` is set to `full`                                                                    |
| Inline image       | ✅ `-i ICON`, `-I PATH` (raw image), `-h` for `image-data` hint                                              |
| Action buttons     | ✅ `-A "key,Label"` — chosen `key` printed to stdout                                                         |
| Reply text input   | ❌                                                                                                           |
| Sound              | ✅ via `-h string:sound-name:NAME` (delegated to dunst rules)                                                |
| Urgency            | ✅ `-u low\|normal\|critical`                                                                                  |
| Timeout            | ✅ `-t MS`                                                                                                   |
| Replace            | ✅ `-r ID` (or `--replace`)                                                                                  |
| Explicit close     | ✅ `-C ID` (or `--close=ID`)                                                                                 |
| Stack tag (group)  | ✅ `--stack-tag`                                                                                             |
| Wait for dismissal | ✅ `-w / --wait` (also prints close reason: `1` dismissed, `2` timed out, `3` closed by call, `4` undefined) |
| Capability probe   | ✅ `--capabilities`, `--serverinfo`                                                                          |

**Example**

```bash
key=$(dunstify \
    --appname "Messenger" \
    --icon ~/icons/ken.png \
    --urgency normal \
    --timeout 8000 \
    --hint string:sound-name:message-new-instant \
    --stack-tag "thread-4192" \
    --action "open,Open" \
    --action "ignore,Ignore" \
    --action "default,Default" \
    "Incoming message" \
    "<b>@ken</b>: heading out — back at 7?")
case "$key" in
    open)   xdg-open "https://example.com/m/4192" ;;
    ignore) ;;
esac
```

**Fit with messenger**

The single best Linux helper for **action-driven** notifications. The `key` returned on stdout maps cleanly to `Dispatch.actions[i].id`, and `--close ID` plus `--stack-tag` map directly to the existing `replace_id` and `group_id` overrides. The cost is the strict requirement that dunst is the running daemon — `messenger` must probe `org.freedesktop.Notifications`'s `GetServerInformation` (name == `dunst`) before electing dunstify as the helper.

### kdialog

KDE's general-purpose dialog utility, included with KDE Frameworks. It is a **dialog** tool, not really a notification client — its `--passivepopup` mode is a non-grabbing transient popup, but it offers no action buttons, no reply input, and no return value to capture. Better for Yes/No prompts than for desktop notifications.

**Documentation**

- Project: <https://invent.kde.org/utilities/kdialog>
- Man page: <https://man.archlinux.org/man/kdialog.1>
- KDE docs: <https://docs.kde.org/stable5/en/kdialog/kdialog/>

**Installation**

| Distro          | Package                         |
|-----------------|---------------------------------|
| Arch            | `pacman -S kdialog`             |
| Debian / Ubuntu | `apt install kdialog`           |
| Fedora          | `dnf install kdialog`           |
| Bundled         | Ships with full Plasma installs |

**Functionality**

| Feature             | Support                                                                                                                                   |
|---------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| Plain text body     | ✅ via `--passivepopup TEXT TIMEOUT`                                                                                                      |
| Pango / HTML markup | ❌                                                                                                                                        |
| Hyperlinks          | ❌                                                                                                                                        |
| Inline image        | ⚠️ `--icon ICON` only (theme name / path)                                                                                                 |
| Action buttons      | ❌ in `--passivepopup`. Available in `--yesno`, `--menu`, `--radiolist`, `--checklist`, but those are blocking dialogs, not notifications |
| Reply text input    | ⚠️ Only via `--inputbox` / `--password` (modal dialogs, not notifications)                                                                |
| Sound               | ❌                                                                                                                                        |
| Title               | ✅ `--title`                                                                                                                              |
| Replace / dismiss   | ❌                                                                                                                                        |
| Stdout return       | ❌ for `--passivepopup`; ✅ for the modal dialog modes                                                                                    |

**Example**

```bash
# Notification (no return value):
kdialog --title "Messenger" \
        --icon dialog-information \
        --passivepopup "Heading out — back at 7?" 8

# Modal Yes/No (returns 0=yes, 1=no, 2=cancel):
if kdialog --title "Messenger" \
           --yesno "Reply to @ken?"; then
    reply=$(kdialog --title "Reply" --inputbox "Type your reply")
    echo "$reply"
fi
```

**Fit with messenger**

Limited fit. `--passivepopup` is strictly less capable than `notify-send` — and on a Plasma host, `notify-send` already routes through KNotifications, so kdialog adds nothing on the notification path. The interesting use is the **separate** `messenger dialog` command described in the parent spec: a blocking, modal "send" prompt that pairs nicely with `--inputbox` or `--yesno` and reads the result from stdout. That is a different command shape from the desktop provider's `send` and should stay distinct.

### Linux priority

The host environment dictates the order, not user preference alone. Always probe `org.freedesktop.Notifications` first to see *which daemon owns the bus*.

```text
1. Daemon == "dunst" AND dunstify present  -> dunstify
2. notify-send present                     -> notify-send
3. (optional) Plasma session AND kdialog   -> kdialog --passivepopup  (notice-only)
4. Fall back to direct D-Bus call
```

Reasoning:

- **dunstify wins under dunst** because it gives back the activated action key on stdout, which `notify-send` only does in libnotify ≥ 0.7.8 with a daemon that advertises the `actions` capability — dunst always does.
- **notify-send is the universal default**. It is part of `libnotify`, is available on every distribution, and routes to whatever daemon is running (GNOME Shell, KNotifications, mako, xfce4-notifyd, dunst). Its capability set will be a function of that daemon, but the *invocation* is always the same.
- **kdialog is a fallback / specialty tool**. It should never displace `notify-send` for desktop notifications, but it remains the right tool for the separate `messenger dialog` interactive prompt described in the parent spec, especially on Plasma.

---

## Cross-OS Capability Matrix

| Capability               | terminal-notifier | alerter     | BurntToast | SnoreToast     | notify-send | dunstify    | kdialog    |
|--------------------------|:-----------------:|:-----------:|:----------:|:--------------:|:-----------:|:-----------:|:----------:|
| Plain body               | ✅                | ✅          | ✅         | ✅             | ✅          | ✅          | ✅         |
| Markup (HTML/XAML/Pango) | ❌                | ❌          | ⚠️ XAML    | ❌             | ✅ Pango    | ✅ Pango    | ❌         |
| Hyperlinks               | ⚠️ click-only     | ❌          | ⚠️ button  | ❌             | ✅          | ✅          | ❌         |
| Image                    | ✅                | ✅          | ✅         | ✅ PNG         | ✅          | ✅          | ⚠️ icon    |
| Action buttons           | ❌                | ✅ dropdown | ✅         | ✅             | ✅ 0.7.8+   | ✅          | ❌         |
| Reply input              | ❌                | ✅          | ✅         | ✅             | ❌          | ❌          | ❌         |
| Sound                    | ✅                | ✅          | ✅         | ✅             | ✅          | ✅          | ❌         |
| Replace / group          | ✅                | ❌          | ✅         | ✅             | ✅          | ✅          | ❌         |
| Blocking                 | ❌                | ✅          | ⚠️ async   | ✅             | ❌          | ⚠️ `--wait` | ✅ dialogs |
| Result on stdout         | ⚠️                | ✅          | ❌ event   | ✅ exit+stdout | ✅ action   | ✅ key      | ✅ dialogs |

---

## Recommended Detection & Election Logic

The desktop provider should:

1. **At startup**, probe each helper *once* (via `which` / `where` / D-Bus daemon name) and cache the result in a `HelperCatalog` struct alongside the existing capability flags.
2. **At send time**, score helpers against the `Dispatch`:

    - If `actions.is_some()` or `reply_to.is_some()` → require `actions`/`reply` capable helper.
    - Else → prefer the cheapest, non-blocking helper.

3. **Allow override** via a `MessengerConfig.desktop.prefer_helpers: Vec<HelperName>` field, honored in user-listed order. This satisfies the spec's "prefers" requirement and lets library callers override the defaults.
4. **Sniff integration**: detection should live in the `sniff` library (matching the parent spec's `sniff notification-apps` proposal), so both `messenger` and other consumers can reuse the catalog.

### Default election order

| OS      | Default order                                                                                   | Override key                     |
|---------|-------------------------------------------------------------------------------------------------|----------------------------------|
| macOS   | `terminal-notifier` → `alerter` (forced when interactive) → native                              | `desktop.prefer_helpers.macos`   |
| Windows | `snoretoast` → `burnttoast` → native                                                            | `desktop.prefer_helpers.windows` |
| Linux   | `dunstify` (only under dunst) → `notify-send` → native D-Bus → `kdialog` (notice-only fallback) | `desktop.prefer_helpers.linux`   |
