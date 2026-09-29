---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: >-
    ECMA-48 (ISO/IEC 6429) defines coded control functions for character-imaging
    devices, including the C0/C1 controls and escape-sequence families behind
    modern terminal cursor, screen, and text-formatting operations. Terminal
    emulators implement practical subsets and extensions, so application
    compatibility depends on the specific functions used.
apps_supporting:
    - WezTerm
    - Ghostty
    - iTerm2
    - cmux
    - kitty
    - Konsole
    - Apple Terminal
    - Alacritty
    - Contour
    - foot
    - GNOME Terminal
    - xterm
    - Windows Console / Windows Terminal
prompt: |-
    ECMA-48 is a terminal standard. Your task is to research this informal standard and:
        
    - provide a detailed overview of the standard. 
        - How it works. 
        - Whether it can be tested for. 
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

    - set `apps_supporting` as a list of those apps which are known to support ECMA-48
    - set `description` as a one to two sentence description of this standard
hash: af2b778ef993c423-10cbe9787c9f04fa
last_updated: 2026-09-28
---
# ECMA-48: terminal control functions

## Overview

ECMA-48, *Control functions for coded character sets*, specifies how control functions are represented in character-coded data for character-imaging input/output devices. Its first edition was published in March 1976; the fifth edition, published in June 1991, is the latest edition listed by [Ecma International](https://ecma-international.org/publications-and-standards/standards/ecma-48/). The standard is also published as [ISO/IEC 6429](https://www.iso.org/standard/12782.html). It was designed as a general device standard, not a promise that every terminal implements every function: ECMA explicitly says implementations should select the facilities appropriate to their application.

Programs and terminals exchange a byte stream over a terminal connection, usually a pseudo-terminal. Printable characters are displayed; control characters and structured sequences change terminal state. ECMA-48 groups functions into C0 controls (such as BEL, BS, HT, LF, CR, and ESC) and C1 controls. Common sequence families include:

- **ESC sequences**, for functions such as index, reset, and character-set designation.
- **CSI (Control Sequence Introducer)**, commonly `ESC [` in 7-bit form, followed by parameters, optional intermediate bytes, and a final byte. Cursor movement (`CSI 4;10H`), erasure, modes, and rendition (`CSI 31m`) are familiar examples.
- **String controls**, including OSC and DCS, which introduce strings terminated by ST. Modern terminal features often use these, but the individual OSC/DCS conventions widely used today are often extensions or later specifications, not portable ECMA-48 guarantees.

The standard defines both 7-bit escape encodings and 8-bit C1 encodings. In modern UTF-8 terminal sessions, prefer the 7-bit form (`ESC [` for CSI, `ESC ]` for OSC, `ESC P` for DCS, and `ESC \` for ST). Literal 8-bit C1 bytes can be invalid UTF-8 or be decoded as ordinary Unicode characters; emulator behavior varies. Also distinguish terminal output controls from keyboard input encodings: many programs call all of these “ANSI escape codes,” but key reporting is a separate, mode-dependent compatibility surface.

## Testing and portability

ECMA-48 is testable function by function, but there is no single “supports ECMA-48” switch or useful universal pass/fail test. A terminal can implement CSI cursor positioning and SGR color while omitting less common standard functions and adding private modes. Device Attributes queries such as `CSI c` may report a terminal family or parameters, but the response is not a reliable inventory of every supported function; terminal identity strings and `TERM` are hints, not proof.

For application compatibility, test the exact sequences and state transitions the program needs in each target emulator. Use a known-good test suite such as [`vttest`](https://invisible-island.net/vttest/) for VT behavior, emulator-specific sequence references, and terminfo capabilities for portable common operations. Probe cautiously: queries write replies back into the same input stream used for keyboard input, so an application must parse replies without consuming user keystrokes. Prefer terminfo or a library abstraction for common cursor/color operations, provide a plain-text fallback, and use documented terminal-specific protocols only after capability detection. The [xterm control-sequence reference](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) is a useful account of both ECMA-48 groundwork and xterm additions.

ECMA-48 remains foundational and is in continuing practical use: terminal emulators, shells, TUIs, and libraries still implement its common control vocabulary. Current development mostly adds interoperable conventions and private extensions around that base, rather than revising the 1991 standard. “ANSI” therefore commonly means a shared subset, not strict full conformance.

## Terminal application support

The entries below mean the application implements commonly used ECMA-48/VT controls; none should be read as full implementation of every ECMA-48 function. Support can change by release and configuration. The most dependable references are each project’s own sequence documentation.

| Terminal                               | Evidence and developer notes                                                                                                                                                                                                                                                                                                                                                                                                                      |
|----------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **WezTerm**                            | Its [escape-sequence reference](https://wezterm.org/escape-sequences.html) documents C0/C1, ESC, CSI, OSC, and DCS behavior. It processes UTF-8; use 7-bit C1 forms in ordinary UTF-8 output. Its page is explicitly a living document and may lag or lead stable releases.                                                                                                                                                                       |
| **Ghostty**                            | The [VT concepts](https://ghostty.org/docs/vt/concepts) and [sequence reference](https://ghostty.org/docs/vt/reference) describe its supported controls. Ghostty documents broader actual support than the work-in-progress reference lists; do not infer unsupported status from an absent entry. It accepts BEL as an OSC terminator for compatibility, although ST is the standard delimiter.                                                  |
| **iTerm2**                             | iTerm2 supports the common ANSI/VT control stream and publishes [escape-code documentation](https://iterm2.com/documentation-escape-codes.html). That page focuses on iTerm2-specific extensions, which may not work through tmux/screen or in other emulators. Keep extensions optional and use the portable 7-bit forms for standard controls.                                                                                                  |
| **cmux**                               | cmux’s [project page](https://cmux.com/) says its terminal rendering uses Ghostty’s `libghostty`; standard VT/ECMA-48 behavior therefore follows that engine. cmux-specific UI, automation, and notification features are separate extensions.                                                                                                                                                                                                    |
| **kitty**                              | kitty implements common VT/ECMA-48 controls and publishes related protocol specifications, including its [keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) and [graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/). Those are extensions, not ECMA-48. Kitty warns that dumping arbitrary binary data can leave the parser waiting for a string terminator; do not send unescaped binary to a terminal. |
| **Konsole**                            | KDE’s [Konsole documentation](https://docs.kde.org/stable_kf6/en/konsole/konsole/index.html) describes terminal key bindings and control sequences. Key tables can change what key combinations send, so applications should handle the sequence variants indicated by terminfo rather than assume one keyboard encoding.                                                                                                                         |
| **Apple Terminal**                     | Apple exposes a selectable `TERM`/terminfo emulation and VT100 application-keypad behavior in its [Terminal profile settings](https://support.apple.com/guide/terminal/change-profiles-advanced-settings-trmladvn/mac). Apple cautions that Terminal and other software do not support the complete ANSI sequence/capability set; use advertised terminfo capabilities and avoid assuming uncommon styles such as dim or blink.                   |
| **Alacritty**                          | The project’s [escape-sequence status table](https://alacritty.org/misc-alacritty-escapes.html) lists implemented and partial CSI features. In particular, private mode handling is selective; do not assume every DEC mode is present just because CSI parsing works.                                                                                                                                                                            |
| **Contour**                            | Contour publishes a detailed [VT sequence list](https://contour-terminal.org/vt-sequence/) covering controls, ESC, CSI, and other families. The list includes many compatibility and extension sequences; check individual entries for exact semantics.                                                                                                                                                                                           |
| **foot**                               | The [foot-ctlseqs manual](https://manpages.debian.org/bookworm/foot/foot-ctlseqs.7.en.html) documents its supported controls and extensions, including CSI, SGR, OSC, and DCS. Bell behavior depends on the `foot.ini` bell setting, so BEL is not guaranteed to make an audible sound.                                                                                                                                                           |
| **GNOME Terminal**                     | GNOME Terminal uses the [VTE terminal widget](https://wiki.gnome.org/Apps/Terminal/VTE), whose behavior is broadly xterm-like. VTE’s release and the application’s configuration determine extensions; identify the terminal through its actual `TERM` value and terminfo entry rather than guessing from the desktop.                                                                                                                            |
| **xterm**                              | The extensive [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) reference is a major practical guide to standard controls and xterm-specific behavior. Its private modes and extensions are not portable merely because many emulators imitate them.                                                                                                                                                             |
| **Windows Console / Windows Terminal** | Microsoft documents [Console Virtual Terminal Sequences](https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences), with behavior based on VT100 and xterm. Native console applications may need to enable `ENABLE_VIRTUAL_TERMINAL_PROCESSING` (and input mode where needed); the documented sequence subset and character sets are not the whole ECMA-48 standard.                                                  |

### Practical quirks and workarounds

1. **Subset differences:** standard syntax does not guarantee a particular function. Use terminfo for common operations; gate optional SGR attributes, private modes, and string protocols behind detection or a conservative fallback.
2. **7-bit versus 8-bit controls:** UTF-8 is the normal interchange encoding. Emit `ESC [` and related 7-bit forms rather than raw C1 bytes unless the transport and emulator explicitly agree on 8-bit handling.
3. **Extensions are not standard guarantees:** OSC hyperlinks, clipboard access, window manipulation, graphics, and enhanced keyboard protocols have separate specifications or vendor conventions. Detect each one and never make core usability depend on it.
4. **Input/output are coupled:** replies to terminal queries arrive as bytes on standard input alongside keypresses. Avoid unsolicited probes, handle timeouts and user input correctly, and restore any modes enabled by the application before exit.
5. **`TERM` describes a contract:** programs often select behavior from `TERM` and terminfo. Do not manually set a misleading value; when connecting through SSH, tmux, or screen, account for the intermediary’s capabilities as well as the visible emulator.
