---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: >-
    OSC 99 is kitty's structured desktop-notification protocol, carrying separate
    titles and bodies, metadata, chunking, and optional interaction reports.
    Terminals and operating systems determine the native notification's appearance
    and which optional features they can honor.
apps_supporting:
    - kitty
    - Foot
    - Contour
prompt: |-
    The OSC 99 informal standard is Kitty's desktop notification protocol, which supports titles, bodies, identifiers, urgency, actions, and close or activation reports back to the program.

    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - What visual representation does it provide? How does that vary by terminal app and operating system?
        - How does it compare with the simpler OSC 9 and OSC 777 notification sequences?
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

    - set `apps_supporting` as a list of those apps which are known to support OSC 99
    - set `description` as a one to two sentence description of this standard
hash: 460156bf8d888291-a3e6b0554a71a3a2
last_updated: 2026-09-28
---
# OSC 99: Kitty desktop notifications

OSC 99 is an informal, kitty-originated protocol for asking the terminal emulator to show a desktop notification. Unlike the older notification escape sequences, it represents a notification as structured, potentially multi-part data and can return user interactions to the program. The protocol is not a request for an in-terminal popup: the terminal hands the content to the host operating system's notification service, which controls presentation and policy.

## How it works

A message has the form `ESC ] 99 ; metadata ; payload ESC \\` (OSC 99 terminated by ST). Both semicolons are required, including for a plain body-only notification. Metadata consists of single-letter `key=value` fields separated by colons. The `p` field selects the payload kind; the default is a title, while `p=body` supplies the body. A program can send title and body in separate chunks under the same `i` identifier; `d=0` marks a chunk as unfinished and the final chunk completes it. Reusing an identifier can update/replace the notification. Payloads can be base64 encoded (`e=1`) to safely carry arbitrary UTF-8 bytes.

Other metadata covers urgency (`u`), notification occasions (`o`), application name (`n`), action buttons (`p=buttons`), and response behavior (`a`). With `a=report`, activation and button clicks are reported as OSC 99 replies carrying the identifier (and button index); `a=focus` requests focusing the originating window. `c=1` asks for a close report. Protocol extensions are designed to be ignorable when unknown. Consult the [kitty protocol reference](https://sw.kovidgoyal.net/kitty/desktop-notifications/) for the complete key definitions and exact escaping rules.

Applications can probe support by sending `OSC 99 ; i=<id> : p=? ; ST`; a supporting terminal replies with its supported action, occasion, and close-event capabilities. This is a real round-trip query, so it requires reading terminal input and should be done through a parser that can safely handle asynchronous terminal replies. A timeout or absent reply means the feature is unavailable; do not block startup indefinitely. Multiplexers must pass the query and route its reply correctly. For basic smoke testing, kitty documents `printf '\x1b]99;;Hello world\x1b\\'`; use `kitten notify` for richer kitty-specific cases.

## Appearance and platform behavior

The protocol does not define a visual design. The terminal submits title, body, urgency and any supported metadata to the platform notification API; the operating system and the user's settings choose the banner/toast style, icon, placement, duration, grouping, sounds, buttons, and whether notifications are suppressed. Linux desktops commonly show a notification-daemon or desktop-shell banner; macOS uses Notification Center; Windows uses its notification system. Focus modes, per-app permissions, notification settings, and desktop environment differences can hide or alter notifications. Applications should treat these as native desktop notifications, not as stable pixels or a reliable delivery channel.

Optional semantics vary too. Some operating-system APIs do not expose dismissal events (kitty notes this for macOS), so a terminal may return an `untracked` close report instead of an observed dismissal. Button appearance and availability are platform-dependent. Kitty's protocol intentionally does not standardize custom icons; a Foot-specific symbolic-icon extension is not portable. A working OSC 99 implementation therefore does not imply every metadata field or response is implemented on every OS.

## Comparison with OSC 9 and OSC 777

OSC 9 (popularized by iTerm2) and OSC 777 (the urxvt-style `notify;title;body` form) are simpler notification requests. OSC 9 generally carries a message/body, with implementation-specific variants; OSC 777 can carry a title and body but has no common structured mechanism for IDs, chunk assembly, urgency, capability discovery, or interaction callbacks. OSC 99 provides those richer operations and an explicit query, at the cost of more involved encoding and sparse implementation coverage. Escape sequences are not reliably autodetected from environment-variable names, and a terminal may silently ignore an unsupported sequence. For broad compatibility, choose a known-supported protocol based on a reliable capability probe or terminal-specific policy, and send a simpler fallback only when appropriate; blindly sending multiple protocols can create duplicate notifications.

## Origin, adoption, and support

Kitty documents the design as partly based on discussion in the now-defunct `terminal-wg`; the protocol is an informal terminal extension rather than a standards-track OSC specification. Kitty's current reference documents the protocol and its query mechanism, and notes that close-event reports were added in kitty 0.36.0. The protocol has continued to evolve through implementation and documentation work, but adoption remains limited relative to OSC 9 and OSC 777. Newer terminal-aware applications and libraries increasingly expose OSC 99 as a selectable notification protocol; that ecosystem activity is evidence of interest, not evidence that a particular terminal supports it.

### Terminal application support

“Supports” below means there is direct project or current implementation evidence for OSC 99, not merely some desktop-notification feature. Protocol support may vary by release and platform. For a terminal not confirmed here, use the OSC 99 capability query instead of assuming support.

| Terminal                               | OSC 99 status                                                                       | Notes                                                                                                                                                                                                                                                                                                                    |
|----------------------------------------|-------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| kitty                                  | Yes; reference implementation                                                       | Full structured protocol; native presentation and available fields depend on OS. Close reports may be `untracked` where the OS cannot report dismissal. Kitty also accepts legacy OSC 9.                                                                                                                                 |
| Foot                                   | Yes                                                                                 | Foot added OSC 99 implementation; its developer proposed a separate symbolic-icon extension, which is not portable. Check the installed Foot release's documentation for exact optional-field coverage.                                                                                                                  |
| Contour                                | Yes in current development/release source                                           | Implements structured metadata, chunking, base64, urgency, occasions, close/activation reports, and capability/alive responses. Its current source describes a configurable inferred close timeout where dismissal cannot be observed; the response is marked `untracked`. Flatpak uses the desktop notification portal. |
| WezTerm                                | No known OSC 99 support; OSC 777 notifications                                      | Use its documented OSC 777 path; do not infer OSC 99 from general notification support.                                                                                                                                                                                                                                  |
| Ghostty                                | No known OSC 99 support; OSC 777 notifications                                      | A request for OSC 99 was discussed in the project; current notification guidance identifies OSC 777.                                                                                                                                                                                                                     |
| iTerm2                                 | No known OSC 99 support; OSC 9 notifications                                        | Use OSC 9 for its supported notification path.                                                                                                                                                                                                                                                                           |
| cmux                                   | No known OSC 99 support                                                             | cmux is Ghostty-based and its notification integration recognizes OSC 9/99/777, but recognition/interception is not evidence that the app implements kitty's OSC 99 semantics. Its own `cmux notify` interface is an alternative.                                                                                        |
| Konsole                                | No known OSC 99 support                                                             | No project-maintained evidence of OSC 99 support found; do not confuse generic desktop notifications with this protocol.                                                                                                                                                                                                 |
| Apple Terminal (Terminal.app)          | No known OSC 99 support                                                             | No evidence of OSC 99 support; OSC 9 compatibility is also not established by the sources reviewed here.                                                                                                                                                                                                                 |
| Alacritty                              | No known OSC 99 support                                                             | No known native desktop-notification protocol support.                                                                                                                                                                                                                                                                   |
| GNOME Terminal / VTE                   | No known OSC 99 support; OSC 777 notifications                                      | VTE-family terminals, including GNOME Terminal, commonly use OSC 777.                                                                                                                                                                                                                                                    |
| Tilix, Terminator, Xfce Terminal, rxvt | No known OSC 99 support; OSC 777 reported                                           | These are commonly listed as OSC 777 implementations, but per-version behavior can differ.                                                                                                                                                                                                                               |
| Windows Terminal                       | No known OSC 99 support; notification support is associated with OSC 9/777 variants | Use its documented notification protocol rather than assuming OSC 99.                                                                                                                                                                                                                                                    |
| Warp                                   | No known OSC 99 support; OSC 777 reported                                           | Third-party compatibility references identify OSC 777.                                                                                                                                                                                                                                                                   |

“No known support” is deliberately distinct from a claim that support is impossible: terminal implementations change, forks may differ, and multiplexers can pass through or intercept sequences. The most reliable application behavior is to query OSC 99 and retain a tested fallback. Multiplexer passthrough settings may be necessary, and a multiplexer that consumes OSC 99 but does not forward terminal replies can break capability detection and callbacks.

### Evidence and further reading

- [Kitty desktop notification protocol](https://sw.kovidgoyal.net/kitty/desktop-notifications/) — normative-in-practice protocol description, examples, queries, and platform notes.
- [Contour current release metadata](https://github.com/contour-terminal/contour/blob/master/metainfo.xml) — current OSC 99 implementation and notification behavior notes.
- [Kitty discussion on notification icons](https://github.com/kovidgoyal/kitty/issues/7657) — explains why portable custom icons were left out and discusses Foot's proposed extension.
- [Ghostty discussion about OSC 99](https://github.com/ghostty-org/ghostty/discussions/4405) — feature request context; current ecosystem guidance lists Ghostty with OSC 777.
- [OpenTUI notification protocol documentation](https://opentui.com/docs/core-concepts/notifications/) — current cross-terminal protocol selection guidance, including OSC 9/777 alternatives.
- [Silvery terminal matrix](https://silvery.dev/reference/terminal-matrix) — a compatibility cross-check listing kitty as OSC 99 and several common alternatives by their notification protocol.
