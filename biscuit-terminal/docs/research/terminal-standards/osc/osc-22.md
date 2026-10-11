---
$schema:
    apps_supporting: string[](required)
    description: string(required)
prompt: |-
    The OSC 22 is an informal standard to set the mouse's pointer shape.

    Your task is to research this informal standard and:
        
    - provide a detailed overview of the standard. 
        - How it works. 
        - Whether it can be tested for. 
        - What visual representation does it provide? How does that vary by terminal app?
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

    - set `apps_supporting` as a list of those apps which are known to support OSC 22
    - set `description` as a one to two sentence description of this standard
apps_supporting:
  - Ghostty
  - kitty
  - iTerm2
  - Foot
  - xterm
  - Contour
  - Konsole
  - mintty
  - Rio
description: >-
  OSC 22 lets a terminal application request a mouse-pointer shape, such as a
  text I-beam, pointing hand, crosshair, or resize cursor. The original xterm
  sequence is informal and its shape names vary by terminal; kitty documents a
  richer, CSS-name-based extension with stack operations and capability queries.
hash: 0b1f05dc5c14c04d-d79bc312e5e83fa6
last_updated: 2026-09-28
---
# OSC 22: Set the mouse pointer shape

OSC 22 is an escape sequence that asks the terminal emulator to change the **mouse pointer** while it is over the terminal pane. It changes the GUI pointer (arrow, I-beam, hand, resize cursor, and so on), not the text insertion cursor drawn in the character grid. A TUI can use it to indicate that a region is clickable, editable, draggable, busy, or unavailable. The terminal remains responsible for drawing and tracking the pointer; OSC 22 does not report mouse coordinates and does not enable mouse input.

## Wire format and origins

The basic form is `ESC ] 22 ; shape BEL` or `ESC ] 22 ; shape ST`. Here `ST` is the string terminator, sent as `ESC` followed by a backslash. The shape is a string. The original xterm feature dates to xterm patch 367 (2021); xterm describes it in terms of the X cursor font and its `pointerShape` resource. The name repertoire is consequently implementation and cursor-theme dependent. xterm itself cautions that custom cursor themes may offer only some of the names and may add others. See the [xterm control-sequence documentation](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) and the [xterm `pointerShape` resource notes](https://invisible-island.net/xterm/manpage/xterm.html#VT100-Widget-Resources:pointerShape).

Kitty added support in 0.31.0 and published a more fully specified extension based on the xterm sequence. It defines 30 CSS `cursor` keyword names, a set/reset operation, a push/pop stack, and queries for current shape and individual shape support. The simple `OSC 22 ; pointer` operation remains compatible with xterm. Kitty's documentation is the most complete description of the extension: [Mouse pointer shapes](https://sw.kovidgoyal.net/kitty/pointer-shapes/). Other terminals have adopted portions of this extension, but there is no central standards body or universal version negotiation. Treat OSC 22 as an informal, evolving terminal convention rather than a universally implemented VT standard.

## Behavior, detection, and rendering

For broad compatibility, emit a simple shape-setting sequence, restore the default when the interaction ends, and tolerate terminals that ignore it. A program can use `default` to request the ordinary pointer. Kitty additionally specifies an empty payload as reset, `>shape` to push, `<` to pop, and `?names` to query. Its query response is another OSC 22: a list of `1`/`0` values for per-shape support, or a shape name for `?__current__`. Do not assume that every terminal implementing simple set also implements these queries, stack operations, or even the same name vocabulary.

There is no portable, reliable probe for the original one-way sequence: an unsupported terminal normally says nothing. Kitty's query is useful only after identifying that the terminal implements the kitty extension; otherwise an application must rely on a known terminal/version capability table or simply send a harmless request and accept a no-op. A visual probe is the practical way to verify actual rendering: move the physical pointer over the pane and observe it. Parser acceptance alone does not prove a visible native cursor change.

The requested shape is mapped by the terminal and host window system to a native cursor. Common examples are `text` → I-beam, `pointer` → pointing hand, `crosshair` → crosshair, `wait` → busy indicator, and `ew-resize` → horizontal resize arrows. Exact artwork, animation, size, and availability vary by OS, cursor theme, terminal toolkit, and name mapping. Some platforms lack a native cursor for certain CSS names; an implementation may map to a neighbor or leave the request ineffective. Hyperlink hover and text-selection/dragging behavior may override the application-requested shape. Kitty explicitly says OSC 22 is independent of mouse reporting and allows hyperlink and selection cursors to take precedence.

## Support by terminal application

The following distinguishes documented or source-confirmed support from absence of evidence. “Supports” here means at least the basic request is implemented; it does not imply that the full kitty extension, every shape, or the same native appearance is available. Version numbers are minimum versions where an authoritative source supplies one.

| Terminal                          | Status and notes                                                                                                                                                                                                                                                   |
|-----------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **WezTerm**                       | Unclear / conflicting public evidence. A compatibility table on Ghostty's OSC page lists WezTerm as unsupported, while a third-party support catalog lists it as supported without a clear version or source. Do not rely on it without checking the target build. |
| **Ghostty**                       | Supported since 1.0.0; uses CSS cursor names. Its docs warn the names have not been universally standardized. Native platform cursor availability can limit what visibly changes.                                                                                  |
| **iTerm2**                        | Supported since 3.5.6 (first release notes; 3.5.6 was not widely released, so use a later release). Its initial documented vocabulary is an X11 cursor-name subset, including `xterm`, `hand1`, `hand2`, and `left_ptr`, rather than the entire CSS list.          |
| **cmux**                          | cmux is built on Ghostty's terminal technology, so its OSC 22 behavior is expected to follow its bundled Ghostty core. The cmux project does not appear to publish a separate OSC 22 compatibility contract; confirm against the actual cmux release.              |
| **kitty**                         | Supported since 0.31.0, with the complete documented 30-name CSS vocabulary, set/reset, stack operations, and support/current-shape queries. See [kitty's protocol documentation](https://sw.kovidgoyal.net/kitty/pointer-shapes/).                                |
| **Konsole**                       | Support is reported in its current parser/source and in compatibility inventories, but a minimum release and public user-facing specification were not established in the sources reviewed. Test the deployed version.                                             |
| **Apple Terminal (Terminal.app)** | Current third-party compatibility listings report support, but Apple does not document OSC 22 in a public terminal protocol reference. Treat as observed/reported rather than a stable Apple contract and test the target macOS version.                           |
| **Alacritty**                     | No support found in its documented escape-code support list or current public compatibility references.                                                                                                                                                            |
| **Contour**                       | Supported in current project release notes as the kitty pointer-shape protocol, including set, push, and pop.                                                                                                                                                      |
| **Foot**                          | Supported from 1.12.0 as “set xcursor pointer.” The name mapping follows its cursor backend and may accept Xcursor/CSS-derived names rather than guaranteeing every CSS shape.                                                                                     |
| **GNOME Terminal**                | No support found. GNOME Terminal uses VTE; VTE's sequence documentation/source marks OSC 22 unsupported. The same caution applies to other VTE front ends unless they add their own handling.                                                                      |
| **xterm**                         | Original implementation, documented from patch 367. Uses X cursor-font names and theme-dependent availability.                                                                                                                                                     |
| **mintty**                        | Support is present in its terminal output implementation; names and visible mapping are implementation-specific.                                                                                                                                                   |
| **Rio**                           | Reported support from 0.0.17, mapping requests to its cursor-icon enum; expect platform-dependent substitutions.                                                                                                                                                   |

Useful cross-checks include [Ghostty's OSC 22 compatibility table](https://ghostty.org/docs/vt/osc/22), [Foot's OSC list and release history](https://codeberg.org/dnkl/foot), [Contour's release notes](https://github.com/contour-terminal/contour/blob/master/metainfo.xml), and [iTerm2's release notes](https://iterm2.com/downloads.html). Compatibility tables can lag current builds, and some report parser support while others mean visible native rendering. In particular, verify WezTerm, cmux, Konsole, and Terminal.app in the exact release you target instead of inferring the complete kitty extension from a basic-support claim.

## Adoption and implementation guidance

The feature began as an xterm extension and was made more portable by kitty's CSS-name vocabulary and richer state operations. Ghostty, Foot, and Contour document support, and iTerm2 added the basic operation; these implementations show continuing adoption across several terminal projects. Coverage remains uneven: Alacritty and VTE-based GNOME Terminal are notable gaps, names do not map uniformly, and advanced query/stack semantics are less widely guaranteed than simple set/reset. Do not make pointer shape essential to usability.

Keep the requested shape scoped to the interaction and restore `default` (or pop a shape only when the target is known to support the stack) on every exit path, including cancellation and errors. Prefer common names such as `text`, `pointer`, `crosshair`, and `default`; for iTerm2's older implementation, map to its known X11 aliases where needed. Do not depend on shape-only hover feedback to communicate an action, since the user may be on a terminal that ignores OSC 22 or the operating system may substitute a different cursor.
