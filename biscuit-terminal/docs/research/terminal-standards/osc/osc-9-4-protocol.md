---
$schema:
    apps_supporting: string[](required)
apps_supporting:
    - WezTerm
    - Ghostty
    - iTerm2
    - cmux
    - kitty
    - Konsole
    - Contour
    - GNOME Terminal
    - Ptyxis
    - Windows Terminal
    - ConEmu
    - mintty
    - WebSSH
prompt: |-
    The OSC (Operating System Command) 9;4 Protocol -- sometimes called "ConEmu Progress Bar" -- provides a way to show progress in a running program. 

    Your task is to research this standard and:

    - provide a detailed overview of the standard. 
        - How it works. 
        - Whether it can be tested for. 
        - What visual representation does it provide? How does that vary by terminal app?
        - Where and when it was introduced? How well it is supported? Is there continuing adoption?
    - which terminal apps support it currently? 
        - How does each terminal app represent the progress?
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

    - set `apps_supporting` as a list of those apps which are known to support OSC 9;4
hash: 16b7e96269155d22-abcadbfeeb7e834b
last_updated: 2026-09-28
---
## Overview

OSC 9;4 is a terminal control sequence for reporting the state of a long-running operation to the terminal emulator. It is a ConEmu extension to the Operating System Command (OSC) family, sometimes called the ConEmu progress-bar protocol. It does not draw a bar in the command's text output. Instead, the terminal receives the report and may show it in its own interface: a tab, window title/status area, dock or taskbar button, or session list.

The sequence has this form:

```text
ESC ] 9 ; 4 ; state [ ; percentage ] ST
```

`ESC ]` begins an OSC. `ST` is either BEL (`\x07`) or String Terminator (`ESC \`). The ConEmu grammar accepts these five states:

| State | Meaning                 | Percentage behavior                                                                                         |
|-------|-------------------------|-------------------------------------------------------------------------------------------------------------|
| `0`   | Clear/withdraw progress | Clear the current indicator.                                                                                |
| `1`   | Normal progress         | Set the percentage, an integer from 0 through 100.                                                          |
| `2`   | Error                   | Mark the operation failed; percentage is optional and otherwise usually retained or treated as unspecified. |
| `3`   | Indeterminate/busy      | Show activity without a known percentage.                                                                   |
| `4`   | Paused/warning          | Mark paused; percentage is optional and commonly retained.                                                  |

For example, `ESC ] 9 ; 4 ; 1 ; 60 BEL` reports 60% completion, `ESC ] 9 ; 4 ; 3 BEL` reports indeterminate activity, and `ESC ] 9 ; 4 ; 0 BEL` clears it. ConEmu's specification permits omitted percentage for states 2 and 4 and documents state 3 as indeterminate. Implementations differ in edge handling, so emit a percentage for state 1, use decimal digits in the 0–100 range, and use state 0 at completion or cancellation. [ConEmu sequence reference](https://conemu.github.io/en/AnsiEscapeCodes.html#ConEmu_specific_OSC)

### Testing and capability detection

There is no standard query/response handshake that reliably asks a terminal whether it displays OSC 9;4. A program can send a test sequence, but a quiet terminal may mean either support with no visible UI in the current configuration or an ignored sequence. Do not use a destructive or visible probe as a startup capability check. The practical test is to run a short, user-invoked demo that sends 25%, indeterminate, error, paused, and clear states, and observe the terminal chrome. For automated testing, capture the emitted bytes and verify the sequence formatting; use real-terminal integration tests to verify GUI presentation. Parser acceptance alone does not prove that a terminal has any UI surface wired to the state.

A feature/version allowlist is more dependable than guessing from `TERM`, which primarily identifies the terminal's text capabilities. If the feature is optional, make emission configurable or enable it only for known supporting terminals. When forwarding through tmux, screen, SSH, or another multiplexer, test that exact path: the multiplexer may pass, consume, or fail to propagate the OSC, and the outer terminal is the one that ultimately renders it.

### Visual presentation and lifecycle

The sequence specifies state, not a required graphic. On Windows, ConEmu and Windows Terminal can put progress on the taskbar; Windows Terminal also shows a ring in the tab header. Ghostty draws a bar at the top of the terminal surface, while kitty draws a bar at the top of its window. WezTerm exposes parsed progress to Lua/tab-title formatting (and its `pane:get_progress()` API is documented for nightly builds), rather than promising one fixed built-in bar. iTerm2 presents a native progress bar and lets profiles configure/disable it. VTE-based terminals such as GNOME Terminal and Ptyxis can surface progress in their window/tab integration. Contour provides a thin per-tab bar and an optional `{Progress}` status-line placeholder, and can aggregate sessions to the OS taskbar where available. mintty has a configurable progress bar. The exact color, animation, placement, aggregation, and whether a taskbar/dock surface exists are emulator- and platform-specific.

Progress state can become stale if a process dies before clearing it. Send state 0 on success, failure handling, cancellation, and other normal exits. Some terminals expire or reset stale state (Ghostty has a roughly 15-second timeout and recommends regular updates); others retain it until cleared or reset. A program with long quiet phases should periodically refresh state if targeting timeout-based implementations. Do not send updates excessively: normal task progress reporting does not need a frame-rate stream.

## Support by terminal application

“Supported” below means there is evidence that the terminal parses OSC 9;4 and connects it to a visible progress surface or exposed terminal UI state. Availability may depend on version, configuration, platform, and the outer terminal when multiplexing. This is a vendor extension rather than a universally implemented terminal standard; unsupported terminals generally ignore it, but older or incompatible parsers can expose escape text or mishandle it.

| Terminal                          | Support and representation                                                                                                                                                               | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
|-----------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **WezTerm**                       | Supported. The sequence is parsed into pane progress state that Lua can use to customize tab titles/status.                                                                              | The progress API currently documents `pane:get_progress()` as nightly-only. State 4 (paused) is explicitly not supported by that API; omitted error percentage is reported as zero. Treat this as partial state coverage and render a custom tab indicator if needed. [API](https://wezterm.org/config/lua/pane/get_progress.html)                                                                                                                                                                     |
| **Ghostty**                       | Supported since 1.2.0. Shows a native bar along the top of each terminal surface.                                                                                                        | About a 15-second inactivity timeout clears stale progress; refresh roughly once a second during long operations. OSC 9;4 is parsed as progress, so text notifications beginning with the conflicting `;4` form cannot be distinguished; use OSC 777 or another documented notification protocol for notifications. Progress styling can be disabled in newer releases. [Release notes](https://ghostty.org/docs/install/release-notes/1-2-0), [OSC reference](https://ghostty.org/docs/vt/osc/conemu) |
| **iTerm2**                        | Supported since 3.6.6. Presents a native progress bar; profile settings in 3.6.7+ can configure or disable progress bars.                                                                | iTerm2 also uses OSC 9 for notifications. Use the exact leading `9;4;` subcommand for progress and the documented notification form for notifications. Its docs allow a short clear form (`OSC 9;4 ST`) in addition to state 0. [Escape-code docs](https://iterm2.com/documentation-escape-codes.html), [release history](https://iterm2.com/downloads.html)                                                                                                                                           |
| **cmux**                          | Supported through its Ghostty-based terminal core; progress should be displayed wherever the embedded Ghostty surface exposes its progress bar.                                          | cmux is built on Ghostty. Its app-level pane/sidebar presentation may change independently, so verify against the cmux version in use. [cmux introduction](https://cmux.com/docs/getting-started)                                                                                                                                                                                                                                                                                                      |
| **kitty**                         | Supported in current releases; draws a progress bar at the top of the window, configurable with `progress_bar`.                                                                          | Older versions discarded OSC 9;4 to avoid confusing it with OSC 9 notifications. Do not infer support from an old kitty install; check the release/changelog and consider the setting. [kitty changelog](https://sw.kovidgoyal.net/kitty/changelog/)                                                                                                                                                                                                                                                   |
| **Konsole**                       | Current Konsole supports the sequence and reports progress through the tab/window/task-manager integration, depending on KDE version.                                                    | Support arrived after earlier Konsole releases (24.08.3 reported no UI support in December 2024). Distribution releases may lag; check the installed version rather than assuming all Konsole builds display it.                                                                                                                                                                                                                                                                                       |
| **Apple Terminal (Terminal.app)** | No reliable primary-source confirmation of OSC 9;4 rendering was found. Do not list as supported.                                                                                        | Some compatibility matrices report support, but without a corresponding Apple protocol reference or reproducible versioned evidence this is not enough to claim it. Provide ordinary in-terminal progress as a fallback.                                                                                                                                                                                                                                                                               |
| **Alacritty**                     | Not supported.                                                                                                                                                                           | The upstream feature request was closed as “wontfix.” Avoid emitting to Alacritty by default if unsupported bytes could leak into output; use an application-level opt-in or known-terminal allowlist. [Upstream issue](https://github.com/alacritty/alacritty/issues/5201)                                                                                                                                                                                                                            |
| **Contour**                       | Supported. Shows a thin progress stripe on the reporting tab, can show state through `{Progress}` in the status line, and aggregates session progress to the OS taskbar where available. | Its `progress_timeout` defaults to 0 (no expiry); a nonzero timeout clears a silent application's stale state. Values above 100 are clamped. [Contour protocol guide](https://contour-terminal.org/vt-extensions/osc-9-4-progress/)                                                                                                                                                                                                                                                                    |
| **Foot**                          | No documented OSC 9;4 UI support; the sequence is ignored.                                                                                                                               | Preserve an in-terminal progress display for foot users.                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| **GNOME Terminal (VTE)**          | Supported when built against VTE 0.79.0 or newer. VTE-based applications can surface progress in their tab/window UI.                                                                    | The capability comes from VTE, so older GNOME Terminal builds and other applications linked to older VTE do not gain it. Ptyxis added corresponding UI support; VTE parsing and application UI are separate layers. [GNOME implementation note](https://blogs.gnome.org/chergert/2024/12/03/ptyxis-progress-support/)                                                                                                                                                                                  |
| **Ptyxis**                        | Supported through VTE plus Ptyxis UI integration.                                                                                                                                        | Requires a sufficiently recent VTE/Ptyxis build.                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **Windows Terminal**              | Supported since version 1.6. Shows a progress ring in the tab header and uses the Windows taskbar progress indicator.                                                                    | Windows taskbar animation depends on the OS “Show animations” setting. [Microsoft guide](https://learn.microsoft.com/en-us/windows/terminal/tutorials/progress-bar-sequences)                                                                                                                                                                                                                                                                                                                          |
| **ConEmu**                        | Originator. Shows progress in the Windows 7 taskbar and ConEmu title.                                                                                                                    | ANSI processing must be enabled; ConEmu's documentation notes that `ConEmuHk` injection is needed for child processes in some configurations. [ConEmu reference](https://conemu.github.io/en/AnsiEscapeCodes.html#ConEmu_specific_OSC)                                                                                                                                                                                                                                                                 |
| **mintty**                        | Supported experimentally since 3.4.2; configurable progress bar.                                                                                                                         | mintty also accepts other progress-control forms, so applications should emit the OSC 9;4 grammar when targeting cross-terminal compatibility. [3.4.2 announcement](https://cygwin.com/pipermail/cygwin/2020-November/246674.html)                                                                                                                                                                                                                                                                     |
| **WebSSH**                        | Supported in version 32.10 and later, when enabled. Shows a state-bar indicator and a thin session-sidebar bar; mobile clients can provide haptic feedback on errors.                    | UI must be enabled/visible. Its clear state briefly displays completion before hiding. [WebSSH guide](https://webssh.net/documentation/terminal-progress-bar/)                                                                                                                                                                                                                                                                                                                                         |

## Origin, adoption, and interoperability

OSC 9;4 originated in ConEmu on Windows and was documented as setting progress on the Windows 7 taskbar and ConEmu title. Windows Terminal later adopted the same ConEmu sequence (documented from Windows Terminal 1.6). In 2020, mintty announced experimental support. Adoption broadened in late 2024 when systemd added OSC-based progress reporting and GNOME's VTE implemented parsing and Ptyxis added a UI surface; WezTerm followed in 2025, then Ghostty 1.2.0 and iTerm2 3.6.6. Current kitty releases and newer Konsole also support it. This represents continuing adoption across Windows, macOS, and Linux, though it remains an extension and support still varies among popular terminals.

There is an interoperability issue: OSC 9 has also been used for desktop notifications, especially by iTerm2/kitty conventions. OSC 9;4's subcommand disambiguates progress in implementations that recognize it, but older terminals and third-party emitters may disagree. Prefer OSC 9;4 with the exact `4` subcommand for progress, keep notifications on their own documented protocol, and retain a textual progress indicator for unsupported terminals. For software that automatically emits the sequence, terminal detection or a user setting matters: default-on emission has caused confusing output or notifications in terminals without support.

### References

- [ConEmu ANSI/OSC reference](https://conemu.github.io/en/AnsiEscapeCodes.html)
- [Microsoft Windows Terminal progress sequence guide](https://learn.microsoft.com/en-us/windows/terminal/tutorials/progress-bar-sequences)
- [Ghostty OSC 9 reference and release notes](https://ghostty.org/docs/vt/osc/conemu)
- [iTerm2 escape-code documentation](https://iterm2.com/documentation-escape-codes.html)
- [WezTerm progress API](https://wezterm.org/config/lua/pane/get_progress.html)
- [Contour OSC 9;4 guide](https://contour-terminal.org/vt-extensions/osc-9-4-progress/)
- [GNOME VTE/Ptyxis implementation note](https://blogs.gnome.org/chergert/2024/12/03/ptyxis-progress-support/)
