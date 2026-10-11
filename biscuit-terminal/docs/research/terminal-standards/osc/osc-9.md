---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: OSC 9 requests that a terminal display a text notification. Adoption is informal, and delivery behavior depends on the emulator and its notification settings.
apps_supporting:
    - iTerm2
    - WezTerm
    - Ghostty
    - kitty
    - Contour
    - Foot
    - Apple Terminal
    - VS Code integrated terminal
    - Warp
    - Zed
prompt: |-
    The OSC 9 informal standard (`OSC 9 ; <text> ST`), introduced by iTerm2, posts a desktop notification with the given text.

    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - What visual representation does it provide? How does that vary by terminal app (OS notification, in-terminal toast, bell)?
        - How does it conflict with ConEmu's `OSC 9 ; <n> ; ...` sub-commands (e.g., 9;4 progress)? What does a terminal that only knows the iTerm2 meaning do when it receives a ConEmu sub-command, and how can a program avoid that?
        - Where and when it was introduced? How well it is supported? Is there continuing adoption?
    - which terminal apps support it currently?
        - Are there any known quirks for any of the apps that a developer should be aware of? How can these quirks be worked around?
        - Note: be sure to include _at least_ the following terminal apps:
            - Wezterm
            - Ghostty
            - iTerm2
            - cmux
            - kitty
            - konsole
            - AppleTerminal
            - Alacritty
            - Contour
            - Foot
            - GNOME Terminal
            - more is better so add others which have readily available info

    Once you've completed your research and have written the prose content to the body of this document, you will need to update the Frontmatter properties:

    - set `apps_supporting` as a list of those apps which are known to support OSC 9
    - set `description` as a one to two sentence description of this standard
hash: f9a029e7dbef9a00-8d8ee824a2fcb084
last_updated: 2026-09-28
---
# OSC 9 desktop notifications

## Overview

OSC 9 is an informal terminal-emulator extension introduced by iTerm2 for asking the terminal to display a notification containing one text string. Its basic form is `OSC 9 ; message ST`, commonly emitted as `ESC ] 9 ; message ESC \\` (or terminated by BEL, `\x07`). For example:

```sh
printf '\033]9;Build finished\033\\'
```

OSC is an in-band control sequence: a program writes it to the terminal's output stream, and the emulator recognizes the command and routes the payload to its notification UI. This does not define a portable notification title, urgency, action, or acknowledgement. Each emulator applies its own policy; notifications may be disabled, suppressed while the pane is focused, or delegated to the OS notification service. OSC 9 has no standardized capability query or acknowledgement, so an application cannot reliably test support by sending it. An application can use known terminal identity/environment variables as a heuristic, but these are not proof (multiplexers, SSH, and nested terminals complicate detection). If notification delivery matters, offer an explicit user-configurable backend or OS-specific fallback.

The visible result is implementation-specific. iTerm2 posts a macOS notification and only does so when the session is not the active session in the frontmost window; its notification behavior is configurable. WezTerm documents a toast. Ghostty's `desktop-notifications` setting enables requests; on GTK/Linux notifications are described as desktop notifications, while in-app toast notifications are a separate mechanism for selected app events. Kitty uses its desktop-notification support for legacy OSC 9, subject to notification settings and platform services. Other emulators may surface native OS notifications, an in-app banner/toast, or no visible result. OSC 9 itself does not mean “ring the bell,” and a bell is not a dependable fallback.

## The ConEmu OSC 9 collision

ConEmu independently assigned subcommands beneath OSC 9. Its `OSC 9 ; 4 ; state ; progress ST` reports a taskbar/title progress indicator (states 0–4); other numeric subcommands have ConEmu-specific meanings. Since both conventions share the same OSC number, a notification-only terminal that does not recognize ConEmu's subcommands can interpret the rest of the sequence as notification text. For example, `OSC 9;4;1;50 ST` may produce an unwanted notification reading `4;1;50` (or similar), rather than a progress bar. This is an interoperability defect, not a valid way to request an iTerm2 notification.

Avoid emitting ConEmu subcommands to a terminal that only implements iTerm2's message form. Detect known implementations or allow the feature to be configured; do not try to infer support from the sequence itself. Where supported, `OSC 777 ; notify ; title ; body ST` is an alternative notification convention that avoids the OSC 9 namespace collision (Ghostty, WezTerm, and Foot document/implement it). For portable progress reporting, likewise gate OSC 9;4 on known terminal support. A terminal that explicitly implements both conventions must parse the leading `4` as progress before treating ordinary OSC 9 payload as a notification. iTerm2 documents both notification and ConEmu-style progress forms; Ghostty chose to support notifications and progress while preventing ConEmu subcommands from triggering false notifications; Contour's current implementation likewise distinguishes the exact progress prefix.

## Origin, status, and testing

OSC 9 is not an ECMA-48 or xterm standardized notification protocol. It originated as an iTerm2 extension, and later emulator adoption has made it a useful de facto convention, but support is fragmented and behavior varies. Adoption continues: current emulator documentation and implementations still add OSC 9 and/or the related progress convention. This should be treated as a best-effort enhancement, not a universal terminal capability.

There is no safe, standard “query OSC 9 support” request. A practical manual check is to emit a harmless message in a terminal with its notification preferences enabled, then repeat while the session is unfocused; lack of a popup is inconclusive because OS notification permissions, focus policy, remote/multiplexer layers, and profile settings can suppress it. Parser-only tests can confirm that a terminal implementation consumes the sequence, but cannot prove that the desktop notification reaches a person. Avoid automated tests that unexpectedly display notifications or steal focus.

## Terminal support

The table records support for the iTerm2-style message notification, not merely some use of OSC 9. “Unverified” means the reviewed project documentation does not establish support; it is not a claim that every version definitely lacks it. Confirm versions and settings before relying on any entry.

| Terminal                      | OSC 9 notification support               | Behavior and developer notes                                                                                                                                                                                                                                                                                    |
|-------------------------------|------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| iTerm2                        | Yes                                      | Native macOS notification; by default only when the session is not active in the frontmost window. User notification settings can disable or change delivery. BEL and ST are accepted terminators.                                                                                                              |
| WezTerm                       | Yes                                      | Documented as “toast” notification. WezTerm also accepts BEL or ST. It additionally supports OSC 777 notifications.                                                                                                                                                                                             |
| Ghostty                       | Yes                                      | Enabled by default through `desktop-notifications`; users can disable it. Supports OSC 777 as an alternative and ConEmu OSC 9;4 progress.                                                                                                                                                                       |
| cmux                          | Yes (Ghostty-based terminal surface)     | cmux documents its own notification API and notification behavior; its terminal surface is Ghostty-based. OSC-specific behavior follows the embedded terminal implementation and may be subject to cmux notification policy. Prefer cmux's notification API when targeting cmux-specific notification features. |
| kitty                         | Yes                                      | Kitty calls OSC 9 a legacy desktop-notification protocol and recommends its richer OSC 99 notification protocol for structured messages, display conditions, urgency, and activation behavior.                                                                                                                  |
| Contour                       | Yes                                      | Current Contour documentation says plain OSC 9 notifications remain supported alongside its ConEmu-style progress handler. It documents a past bug where bare `OSC 9;4` incorrectly notified with text “4”; use a current release.                                                                              |
| Foot                          | Yes                                      | Foot supports OSC 9/777 notification integration. Notification delivery depends on its desktop notification backend; Foot documents action/activation behavior and notes that its `notify-send` path cannot report the XDG activation token.                                                                    |
| Apple Terminal (Terminal.app) | Reported                                 | Current terminal compatibility survey reports support, but Apple does not publish a detailed OSC 9 contract. Test on the target macOS version and notification settings.                                                                                                                                        |
| Alacritty                     | Unverified / partial in parser libraries | The upstream Alacritty terminal parser API has historically lacked an application notification callback; an embedding app can add its own handling. Do not assume standalone Alacritty will display OSC 9 notifications without verifying the exact build.                                                      |
| Konsole                       | Unverified for notification form         | Available references establish Konsole support for ConEmu OSC 9;4 progress, not the iTerm2 text notification. Do not put it in an OSC 9 notification allowlist based on progress support alone.                                                                                                                 |
| GNOME Terminal                | Unverified                               | The reviewed GNOME Terminal/VTE references did not document OSC 9 notifications. VTE-based terminals should be checked individually; shared VTE support for other OSC features does not imply notification support.                                                                                             |
| Windows Terminal              | Unverified for notification form         | Current references establish ConEmu OSC 9;4 progress support, but not that the iTerm2-style text form raises a notification. Do not infer notification support from progress support.                                                                                                                           |
| VS Code integrated terminal   | Yes (reported)                           | Recent compatibility surveys list support; behavior is mediated by the host editor and its notification preferences. Verify on the supported VS Code version.                                                                                                                                                   |
| Warp                          | Yes (reported)                           | Recent compatibility surveys list support; check the app's notification policy.                                                                                                                                                                                                                                 |
| Zed terminal                  | Yes in current implementation            | Added a terminal notification event and UI after OSC 9 support was requested; notification is an in-app floating notification rather than the sequence defining an OS-level display.                                                                                                                            |

## Sources

- [iTerm2 escape-code reference](https://iterm2.com/documentation-escape-codes.html) — notification syntax, terminators, progress subcommand, and iTerm2 behavior.
- [WezTerm escape-sequence reference](https://wezterm.org/escape-sequences.html) — OSC 9 toast and OSC 777 notification examples.
- [Ghostty VT reference](https://ghostty.org/docs/vt/reference) and [configuration source](https://github.com/ghostty-org/ghostty/blob/main/src/config/Config.zig) — sequence support and notification/progress settings.
- [kitty desktop notifications](https://sw.kovidgoyal.net/kitty/desktop-notifications/) — legacy OSC 9 support and the richer OSC 99 protocol.
- [ConEmu ANSI escape codes](https://conemu.github.io/en/AnsiEscapeCodes.html) — original ConEmu OSC 9 subcommands, including progress and security-sensitive process/macro commands.
- [Contour OSC 9;4 reference](https://contour-terminal.org/vt-extensions/osc-9-4-progress/) and [release notes](https://contour-terminal.org/release-notes/) — distinction between plain notification and progress, plus implementation history.
- [Foot configuration manual](https://manpages.debian.org/foot/foot.ini.5.en.html) — notification activation and backend notes.
- [cmux notifications documentation](https://cmux.com/docs/notifications) — cmux notification policy and APIs.
- [Terminal Support survey](https://terminfo.dev/extensions/notifications-osc-9) — cross-emulator support snapshot; secondary source, useful as a lead rather than an authoritative guarantee.
- [Zed implementation issue](https://github.com/zed-industries/zed/issues/58014) — OSC 9 notification integration and in-app presentation.
