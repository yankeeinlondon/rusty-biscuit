---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: >-
    An xterm-derived family of dynamic-color OSC sequences for mouse-pointer,
    Tektronix, and text-selection colors, with matching reset commands. Support
    is uneven: some terminals implement selection colors, some pointer colors,
    and many implement neither.
apps_supporting:
    - xterm
prompt: |-
    The OSC 13-19 _and_ 110-119 are grouped as an informal standard involving mouse pointers, and selection highlight colors.

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

    - set `apps_supporting` as a list of those apps which are known to support OSC 13-19 and 110-119
    - set `description` as a one to two sentence description of this standard
hash: 5e7a8bc0c38a90aa-8238ed2766f71134
last_updated: 2026-09-28
---
# OSC 13–19 and OSC 110–119: pointer, Tektronix, and selection colors

## Overview

This is an informal xterm control-sequence family, not an ANSI or ECMA-48 standard. The numbers extend xterm's dynamic-color interface (OSC 10–12) to additional color slots. OSC 13 and 14 name the mouse pointer's foreground and background; OSC 15 and 16 name the Tektronix 4014 emulation's foreground and background; OSC 18 is that emulation's cursor color; OSC 17 and 19 are the selection highlight background and foreground. In practice, modern terminals without Tektronix display emulation have little reason to render 15, 16, or 18. The xterm convention has reset counterparts OSC 113–119 for these slots; the larger reset range also includes OSC 110–112 for the OSC 10–12 foreground, background, and text-cursor colors. See [xterm's control-sequence reference](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) and [Ghostty's index table](https://ghostty.org/docs/vt/osc/1x).

The basic wire form is `ESC ] n ; color ST` to set and `ESC ] n ; ? ST` to query a color, where `n` is the OSC number and `ST` is normally `ESC \\` (BEL is also accepted by many xterm-compatible terminals). A terminal response to a query is another OSC carrying a color specification and terminator. For example, `ESC ] 17 ; #RRGGBB ST` sets selection background and OSC 117 resets it to the terminal's configured default. xterm also allows several successive color fields after one OSC number, advancing the slot for each field. Color specification details and accepted encodings differ; use the common `rgb:RR/GG/BB` or `#RRGGBB` forms only after checking the target terminal. The mapping is:

| Set/query | Reset   | Color slot           |
|-----------|---------|----------------------|
| OSC 13    | OSC 113 | Pointer foreground   |
| OSC 14    | OSC 114 | Pointer background   |
| OSC 15    | OSC 115 | Tektronix foreground |
| OSC 16    | OSC 116 | Tektronix background |
| OSC 17    | OSC 117 | Selection background |
| OSC 18    | OSC 118 | Tektronix cursor     |
| OSC 19    | OSC 119 | Selection foreground |

OSC 110, 111, and 112 reset the neighboring OSC 10, 11, and 12 colors (default text foreground, default text background, and text cursor). They are included in the requested reset range but do not correspond to OSC 13–19.

The sequences can be tested at the protocol level by setting a conspicuous color, querying it, checking the terminal's reply, then sending the matching reset. Query and set support are separate capabilities: a terminal can accept a set while not replying to a query, or return a configured color without visibly applying it. A visual test should make a text selection and move the pointer over both light and dark cell backgrounds. Send reset commands even after a failed test so an override is not left active. There is no universal capability bit for this family. Querying every slot with a short timeout is a useful probe but silence is ambiguous (unsupported, filtered by a multiplexer, or delayed), and tests can collide with keyboard input. Do not infer support from `$TERM` alone.

## Visual behavior and deployment

OSC 13/14 describe the terminal's mouse pointer, not the text cursor at the cell position (OSC 12). On GUI terminals their practical effect depends on whether the native cursor backend permits per-terminal recoloring. Some platform cursor themes are operating-system-owned and cannot be recolored; some implementations accept and remember the values without a visible pointer change. The Tektronix colors affect only a Tektronix graphics display, if one exists. OSC 17/19 color selected text in the terminal's selection highlight. The foreground may be fixed, dynamically chosen for contrast, or composited with transparency by the app, so an accepted foreground color need not be visually exact. Selection is also often controlled by profile/theme settings.

The numbering and semantics are documented as xterm extensions; the current xterm reference and xterm-derived implementations are the primary provenance. The accessible documentation establishes xterm as the source but does not establish a reliable first release date for each of OSC 13–19. Treat claims of a specific introduction year as unverified unless tied to a dated xterm changelog entry. Adoption remains fragmented: the selection pair has some uptake, the pointer pair less, and the Tektronix slots are legacy compatibility hooks. Newer terminals often prefer their own color-control protocol or offer only profile-level selection styling. There is some continuing adoption: for example, Windows Terminal added selection-color setting and querying in its 1.22 preview line; this is not evidence of broad support for the whole range.

## Terminal support and caveats

The list below distinguishes documented support from general color or cursor features. “Partial” means only the stated slots are confirmed, not that the whole OSC 13–19 plus 110–119 family works. Versions evolve, so verify against the version shipped to users. References: [Contour's sequence list](https://github.com/contour-terminal/contour), [Foot's supported OSC list](https://codeberg.org/dnkl/foot), [Windows Terminal 1.22 preview notes](https://github.com/microsoft/terminal/discussions/17809), [kitty color control](https://sw.kovidgoyal.net/kitty/color-stack/), [iTerm2 color preferences](https://iterm2.com/documentation-preferences-profiles-colors.html), and [Ghostty's explicit support note](https://ghostty.org/docs/vt/osc/1x).

| Terminal                  | Confirmed support relevant to this family                                                                                                              | Developer notes                                                                                                                                                       |
|---------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| xterm                     | Defines the full family, including color queries and reset counterparts.                                                                               | Canonical reference behavior; use ST and the xterm color grammar. X resources and user policy can affect whether clients may change colors.                           |
| Contour                   | Implements pointer foreground/background (13/14), selection background/foreground (17/19), and their reset codes.                                      | Its documented sequence inventory does not list Tektronix 15/16/18. Do not assume those slots or all query forms.                                                     |
| Foot                      | Implements selection background/foreground (17/19) and resets 117/119.                                                                                 | Its OSC documentation warns that multiple device-attributes replies may arrive together; parse replies rather than assuming one response. It does not document 13/14. |
| Windows Terminal          | OSC 17 selection background, including set and query, in the 1.22 preview notes.                                                                       | That release note does not claim OSC 19 or pointer slots. Use OSC 17 alone as an optional enhancement.                                                                |
| Ghostty                   | No support for 13–19 currently; its dynamic-color documentation says only indices 10–12 are supported and other values are ignored.                    | OSC 10–12 support does not imply this extension. cmux uses libghostty and currently shares this limitation.                                                           |
| cmux                      | No documented support for 13–19; it is based on libghostty.                                                                                            | Treat it like Ghostty for these slots until its backend documents otherwise.                                                                                          |
| WezTerm                   | Its documented OSC list includes only dynamic colors 10–12; no 13–19 support is documented.                                                            | Do not rely on its general palette or cursor-color support as evidence for this family.                                                                               |
| kitty                     | Selection colors are supported through kitty's OSC 21 color protocol (`selection_background` and `selection_foreground`), not documented as OSC 17/19. | Prefer OSC 21 when kitty-specific support is acceptable; a compatibility client should still probe OSC 17/19 and fall back.                                           |
| iTerm2                    | Profile settings and its own APIs expose selection colors; current public sequence documentation does not establish OSC 17/19 support.                 | Do not equate UI configurability or OSC 1337 color controls with xterm OSC 17/19.                                                                                     |
| Apple Terminal            | No primary documentation found confirming OSC 13–19 or 113–119.                                                                                        | Profile selection colors, where available, are not proof that runtime OSC overrides work.                                                                             |
| Alacritty                 | No primary documentation found confirming these slots.                                                                                                 | OSC 10–12 or OSC 4 support is not sufficient evidence.                                                                                                                |
| Konsole                   | No primary documentation found confirming these slots.                                                                                                 | Selection appearance may be derived from the active color scheme; that is distinct from OSC runtime control.                                                          |
| GNOME Terminal / VTE      | No primary documentation found confirming these slots.                                                                                                 | VTE-based terminals share much parser behavior, but do not infer support in GNOME Terminal from another VTE consumer.                                                 |
| mintty                    | No primary documentation found confirming the full family.                                                                                             | Probe the exact version and check separately for selection and pointer behavior.                                                                                      |
| Windows Terminal (stable) | Support varies by release; the cited 1.22 preview documents OSC 17.                                                                                    | Check the deployed version; do not assume the preview capability exists in older stable builds.                                                                       |

## Practical guidance

Treat selection color (17/19) and pointer color (13/14) as independent, optional capabilities. Query before setting when a reply is available, retain the original values if possible, and always restore with the corresponding reset code when the application exits. For kitty, use OSC 21 as a separately detected fallback. For terminals that ignore the OSCs, leave the user's profile/theme colors alone and preserve normal selection and pointer behavior. Multiplexers and remote shells may filter or fail to pass replies, so probe from the same path the application will use rather than from a direct local terminal alone.
