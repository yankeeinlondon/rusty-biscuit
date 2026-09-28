---
$schema:
    apps_supporting: string[](required)
    description: string(required)
apps_supporting:
    - xterm
    - Contour
    - mintty
    - Linux console
description: >-
    OSC 50 is an informal xterm-originated sequence for selecting or querying
    the terminal's font. Support is limited and its meaning conflicts with
    other uses of OSC 50, so applications should treat it as an optional,
    terminal-specific enhancement.
prompt: |-
    The OSC 50 is an informal standard used to set or query the font.

    Your task is to research this informal standard and:
        
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

    - set `apps_supporting` as a list of those apps which are known to support OSC 50
    - set `description` as a one to two sentence description of this standard
hash: 9413b16b70590361-53a40ccaef430c74
last_updated: 2026-09-28
---
## Overview

OSC 50 is an informal terminal-emulator extension for changing or querying the current font. It is not part of ECMA-48, has no central registry, and should not be confused with standardized SGR text styles such as bold or italic. Implementations do not agree on its meaning: xterm and compatible terminals use it for font selection, while other software has assigned OSC 50 to unrelated functions. OSC numbers are therefore not globally safe to assume.

An OSC is introduced by `ESC ]` and conventionally ended by the string terminator `ESC \`. For xterm, the basic forms are:

```text
ESC ] 50 ; font-specifier ST    select a font
ESC ] 50 ; ? ST                 query the selected font
```

Xterm also accepts BEL (`0x07`) as a terminator for compatibility and returns the reply using the terminator used by the query. A query response is another OSC 50 sequence containing the setting that would select the current font; responses may therefore contain font-system-specific syntax rather than a portable family name.

In xterm, a font specifier can be an X font name/pattern. A parameter beginning with `#` selects an entry in the font menu: `#N` selects an absolute entry and `#+N` or `#-N` moves relative to the current one. The number after a relative sign is optional. A space followed by a font specification can accompany a menu index. When xterm is configured to render with TrueType fonts, the sequence selects or queries the face name. This is xterm behavior, not a cross-terminal grammar; Contour and other implementations may interpret font names according to their own font backends.

### Origin and adoption

The sequence is documented by XTerm as one of its Operating System Commands; historical XTerm references indicate OSC 50 font selection was documented by 1991. XTerm's broader OSC command-family history reaches back to X.V10R4 in December 1986, but that date should not be mistaken for a verified introduction date for OSC 50 itself. The font operation is an xterm extension rather than an ANSI/ECMA standard. Its history is consequently tied to the X Window System and X11 font model. Modern terminals generally expose font selection through profile/configuration interfaces instead. Current documented support remains sparse, and recent implementation evidence is concentrated in xterm-compatible emulators such as Contour and mintty. There is no clear evidence of broad continuing adoption: modern terminals have declined requests for it, and incompatible uses have made the code unattractive as a general-purpose capability.

### Detecting support

There is no reliable capability bit or universal terminal-identification value for OSC 50. The best non-destructive probe is to send `OSC 50 ; ? ST` and read the terminal's response with a short timeout. A syntactically valid OSC 50 reply is positive evidence. No reply is inconclusive: the terminal may not implement the operation, may filter it through a multiplexer or remote connection, or may have font operations disabled. A reply also identifies only the terminal's own interpretation; it does not prove that a requested font exists or that a set operation will be accepted.

Avoid probing by changing the font in an interactive user's session. If an application elects to offer font changes, gate them on an explicit terminal-specific capability decision or user opt-in, and provide an ordinary configuration fallback. In xterm, `allowFontOps` can disable font operations; querying OSC 60 for allowed feature categories can help determine whether `allowFontOps` is enabled, but this is itself an xterm extension.

## Terminal support

“Supported” below means there is direct evidence for the xterm-style operation of setting or querying the font. Merely recognizing OSC 50 for a different purpose does not count.

| Terminal                      | OSC 50 font support | Notes                                                                                                                                                                                                                                                  |
|-------------------------------|---------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| xterm                         | Yes                 | Reference implementation. Font operations can be disabled with `allowFontOps`; font names and `#` menu-index syntax depend on xterm resources and its font backend.                                                                                    |
| Contour                       | Yes                 | Documents get/set font and has a configurable permission for `change_font`; configure permission to allow the request in automation, or handle the user approval prompt in interactive use. Contour also defines OSC 60 for font size and style faces. |
| mintty                        | Yes                 | Listed as supporting font query/set; use mintty's accepted font naming syntax rather than assuming xterm's X font syntax works.                                                                                                                        |
| Linux console                 | Yes, limited        | The Linux console manual documents `ESC ] 50 ; fn ST` to set a console font and notes that the sequence is normally disabled. This is not the xterm query/set contract, so treat it as a related implementation rather than interchangeable support.   |
| WezTerm                       | No                  | Its OSC 50 support request remains an unimplemented feature request. Use WezTerm configuration or its Lua APIs instead.                                                                                                                                |
| Ghostty                       | No                  | OSC 50 is not in Ghostty's implemented OSC parser. Configure the font in Ghostty instead.                                                                                                                                                              |
| cmux                          | No evidence         | cmux uses libghostty, so Ghostty's lack of OSC 50 applies unless cmux adds a separate handler. Configure the host terminal's font.                                                                                                                     |
| iTerm2                        | No                  | iTerm2 documentation says its former OSC 50 use conflicted with xterm and was replaced by OSC 1337. Use profile selection/configuration or iTerm2's documented OSC 1337 features where applicable.                                                     |
| kitty                         | No                  | Its current OSC parser does not implement OSC 50 font selection. A request to add it is not evidence of support; use kitty configuration and remote-control features.                                                                                  |
| konsole                       | No                  | OSC 50 does not change the font in the available implementation/testing evidence. Use profile settings or Konsole's normal font controls.                                                                                                              |
| Apple Terminal (Terminal.app) | No evidence         | No OSC 50 font operation is documented. Use the profile's font setting.                                                                                                                                                                                |
| Alacritty                     | No                  | Its OSC 50 handling is for cursor shape, not font selection. Do not send a font request: it can affect cursor shape instead.                                                                                                                           |
| Foot                          | No                  | Font setting is not implemented through OSC 50; use Foot configuration.                                                                                                                                                                                |
| GNOME Terminal / VTE          | No                  | No font-changing OSC 50 operation is documented or implemented; set fonts through the profile.                                                                                                                                                         |
| Rio                           | No                  | Its documented OSC 50 implementation handles `CursorShape`, not font changes.                                                                                                                                                                          |

The most important compatibility trap is that an OSC 50 sequence may be recognized but mean something else. In particular, Alacritty and Rio use the code for cursor shape, and iTerm2 historically used it for file/clipboard operations before moving its extensions to OSC 1337. An application should never infer font support from a successful write or from the terminal name alone. Multiplexers may also consume or pass through OSC sequences differently; test through the actual connection path when support matters.

## References

- [XTerm Control Sequences: OSC 50 and OSC feature controls](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [XTerm change log](https://invisible-island.net/xterm/xterm.log.html)
- [Contour VT sequence reference](https://contour-terminal.org/vt-sequence/)
- [Contour profiles and `change_font` permission](https://contour-terminal.org/configuration/profiles/)
- [Linux console control codes](https://man7.org/linux/man-pages/man4/console_codes.4.html)
- [WezTerm OSC 50 feature request](https://github.com/wez/wezterm/issues/4181)
- [Ghostty OSC parser](https://github.com/ghostty-org/ghostty/blob/main/src/terminal/osc.zig)
- [iTerm2 proprietary escape codes](https://iterm2.com/documentation-escape-codes.html)
- [kitty OSC 50 feature request](https://github.com/kovidgoyal/kitty/issues/9508)
- [Alacritty escape sequence reference](https://github.com/alacritty/alacritty/blob/master/docs/escape.md)
- [Foot OSC 50 feature request](https://codeberg.org/dnkl/foot/issues/1472)
- [Rio escape sequence support](https://rioterm.com/es/docs/escape-sequence-support)
