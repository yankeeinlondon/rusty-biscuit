---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: >-
    Kitty's informal OSC 21 color protocol queries, sets, and resets named terminal colors and ANSI palette entries in one string-keyed control sequence. It provides machine-readable color replies and lets applications temporarily control the terminal's default colors and related UI colors.
apps_supporting:
    - kitty
    - Ghostty
    - foot
    - cmux
prompt: |-
    The OSC 21 is an informal standard for Kitty's color control protocol.

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

    - set `apps_supporting` as a list of those apps which are known to support OSC 21
    - set `description` as a one to two sentence description of this standard
hash: 931a465ab6c116d2-b496b0bdf32365bd
last_updated: 2026-09-28
---
# OSC 21: Kitty color protocol

## Overview

OSC 21 is Kitty's informal extension for querying, setting, and resetting terminal color state. It uses OSC (Operating System Command) framing: `ESC ] 21 ; ... ST`, where `ST` is either BEL (`0x07`) or `ESC \\`. A body contains semicolon-separated `key=value` fields. The keys are names for special colors or decimal indices `0` through `255` for the ANSI palette. This string-keyed design lets the protocol add color slots without allocating more OSC numbers, unlike the older xterm family of color commands.

For example, `printf '\033]21;foreground=?;background=?\033\\'` asks for the default text and background colors. A supporting terminal replies on the same terminal stream with an OSC 21 sequence containing the values in place of `?`. A client must read and parse terminal input to use query replies; emitting the query alone does not reveal whether it was understood. Replies may contain `rgb:` values, and implementations may use different permitted precisions. An unset or dynamic color is returned with an empty value. Unknown keys are returned as `unknown=` fields containing the unpadded Base64 encoding of the key, rather than echoing arbitrary input.

Set a color with a value such as `foreground=#d0d0d0`; query and set operations may be combined in one command. Send a key without `=` to reset it to the terminal's default. Send `key=` with an empty value to request a dynamic color, where that slot supports the behavior. The protocol covers foreground, background, selection foreground/background, cursor and cursor-text colors, visual-bell color, transparent background color slots, and indexed palette colors. Dynamic-color semantics are advisory where terminal UI behavior differs. Applications should save and restore state around temporary theme changes; Kitty also defines separate color push/pop sequences (`OSC 30001` and `OSC 30101`), which are distinct from OSC 21 itself.

## What users see and how to test it

OSC 21 is a control interface, not a drawing or image protocol. Setting foreground/background changes subsequent terminal text and the cell background; setting selection, cursor, visual-bell, or transparent-background slots can affect those UI elements if the terminal implements them. The visible result depends on what UI surfaces the application exposes and on its chosen dynamic-color behavior. A successful query produces no intended visual mark: the application receives a textual color report. Unsupported terminals may ignore the command, so a lack of visible change is not a reliable capability test.

It is testable by sending a query to the actual terminal device and capturing the reply, checking the OSC 21 framing, requested keys, and parsed values. For a robust probe, query a standard known key such as `foreground` and also an intentionally unknown key, and impose a short timeout because unsupported terminals commonly remain silent. Verify setters by setting a temporary known color, querying it, and restoring the original value. Query support and setter support should be tested separately. Run probes directly against the terminal: tmux, screen, SSH, or another intermediary may consume, filter, or fail to forward the sequence. Do not assume `$TERM` or `COLORTERM` advertises OSC 21.

## Origin, maturity, and adoption

Kitty introduced the protocol as part of its color-control work and documents it as a Kitty extension, rather than an ECMA or xterm standard. The current Kitty documentation gives no explicit first-release date; Kitty 0.36.0 (2024) is commonly cited as the first release with this protocol. Treat that date as a release-level attribution rather than a standards publication date. The protocol has gained adoption in Ghostty and foot, and applications built on libghostty can inherit the parser behavior. That is meaningful continued implementation, but support remains narrow compared with the legacy OSC 10/11/12 and OSC 4 color controls. Portable applications should retain those fallbacks and treat OSC 21 as an optional capability.

## Terminal support

The list below distinguishes affirmative implementation evidence from terminals for which no reliable OSC 21 implementation was found. “No known support” is not a proof that every version ignores the sequence. `cmux` uses libghostty, so its OSC 21 support follows the linked libghostty version; it is inherited implementation evidence, not an independent cmux protocol implementation.

| Terminal app                | Status                   | Notes and workarounds                                                                                                                                                                                                                                                                                                                           |
|-----------------------------|--------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| kitty                       | Supported                | Reference implementation. Supports named slots, indexed colors, queries, sets, and resets. Use `printf` rather than non-portable `echo -e` when sending escapes.                                                                                                                                                                                |
| Ghostty                     | Supported                | Listed by Ghostty among its external protocols. An earlier implementation replied even to non-query OSC 21 sequences, which made fire-and-forget startup color-setting scripts unsafe; that behavior was fixed in upstream development in February 2025. Test the installed version, and use OSC 10/11 as a compatibility fallback when needed. |
| foot                        | Supported                | Listed as an implementation by current support inventories. Different UI/color-slot coverage may apply; query each slot and do not assume all Kitty-specific auxiliary slots are meaningful.                                                                                                                                                    |
| cmux                        | Supported via libghostty | cmux is built on libghostty, so behavior depends on the embedded library version. Its own chrome and pane theme are separate from terminal cell colors; OSC color control should not be assumed to recolor cmux's application chrome.                                                                                                           |
| WezTerm                     | No known support         | WezTerm documents dynamic palette changes through its supported color escape sequences, but current protocol inventories do not identify Kitty OSC 21. Use its documented OSC 10–12/4-compatible commands or a WezTerm-specific interface.                                                                                                      |
| iTerm2                      | No known support         | No OSC 21 implementation is documented. Use its supported OSC 10–12/4 color controls or iTerm2 APIs.                                                                                                                                                                                                                                            |
| Konsole                     | No known support         | No Kitty OSC 21 support found; use conventional xterm color OSCs where supported.                                                                                                                                                                                                                                                               |
| Apple Terminal              | No known support         | No Kitty OSC 21 support found; use conventional OSC 10/11/12 where available and retain configured defaults.                                                                                                                                                                                                                                    |
| Alacritty                   | No known support         | No Kitty OSC 21 support found; prefer configured colors or legacy OSC controls if supported by the installed release.                                                                                                                                                                                                                           |
| Contour                     | No known support         | No Kitty OSC 21 implementation found in its available protocol documentation; use conventional color OSCs as a fallback.                                                                                                                                                                                                                        |
| GNOME Terminal (VTE)        | No known support         | VTE-based terminals provide some xterm-style OSC color behavior, but that does not imply Kitty OSC 21. Probe directly and fall back to OSC 10/11/12 and OSC 4.                                                                                                                                                                                  |
| xterm                       | No known support         | Implements its own older numbered color protocol; Kitty OSC 21 is a separate extension.                                                                                                                                                                                                                                                         |
| Windows Terminal            | No known support         | No Kitty OSC 21 support found; rely on configured palette or supported legacy color controls.                                                                                                                                                                                                                                                   |
| VS Code integrated terminal | No reliable confirmation | Do not infer support from generic xterm.js OSC color support; query the actual bundled version and retain legacy OSC fallbacks.                                                                                                                                                                                                                 |

### Implementation cautions

Use the exact Kitty spelling for keys (`cursor_text`, `visual_bell`, and `transparent_background_color1` through `transparent_background_color7`); alternative names found in unofficial summaries are not the protocol's documented names. Preserve empty values and distinguish them from bare keys: `key=` requests a dynamic value, while `key` resets to default. Parse semicolon-delimited fields and the terminal's color formats rather than assuming only `#RRGGBB`. Query replies arrive asynchronously on the same input stream as key presses, so terminal applications need a parser that can separate control replies from user input. Applications should avoid sending OSC 21 indiscriminately through multiplexers or remote sessions unless those layers are known to pass it through.

## Sources

- [Kitty color control protocol documentation](https://sw.kovidgoyal.net/kitty/color-stack/) — syntax, slots, querying, setting, encoding, and color stack.
- [Ghostty external protocol list](https://ghostty.org/docs/vt/external) — identifies OSC 21 as the Kitty Color Protocol.
- [Ghostty issue #5834](https://github.com/ghostty-org/ghostty/issues/5834) — fix to avoid replies to non-query OSC 21 sequences.
- [cmux project site](https://cmux.com/) — describes cmux as a native terminal built on Ghostty/libghostty.
- [Kitty changelog](https://sw.kovidgoyal.net/kitty/changelog/) — release history; Kitty 0.36.0 introduced the feature in the 2024 release line.
