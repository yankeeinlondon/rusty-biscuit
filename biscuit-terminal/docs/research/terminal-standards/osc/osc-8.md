---
$schema:
    apps_supporting: string[](required)
prompt: |-
    The OSC 8 informal standard is used for hyperlinks.
    
    Your task is to research this informal standard and:
        
    - provide a detailed overview of the standard. 
        - How it works. 
        - Whether it can be tested for. 
        - What visual representation does it provide for a link? To follow the link do you need to combine a mouse click with a keyboard key or just clicking is enough?
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
    
    - set `apps_supporting` as a list of those apps which are known to support OSC 8
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
    - Foot
    - GNOME Terminal
    - GNOME Console
    - Ptyxis
    - Guake
    - MATE Terminal
    - ROXTerm
    - Terminator
    - Tilix
    - Xfce Terminal
    - mintty
    - Windows Terminal
    - VS Code Integrated Terminal
    - Hyper
    - DomTerm
    - hterm
    - Tabby
    - AbsoluteTelnet/SSH
    - Secure ShellFish
    - tmux
    - Zellij
hash: f89d31757444101e-73139ae52b097e86
last_updated: 2026-09-28
---
# OSC 8 terminal hyperlinks

OSC 8 is an informal terminal-emulator convention for attaching a URI to text written to a terminal. It is commonly called “explicit hyperlinks” because the program producing output supplies both the displayed text and its target, instead of asking the terminal to guess URLs from ordinary text. It is an extension, not part of a universally implemented terminal standard; unsupported terminals generally ignore the control sequences and leave the displayed text readable.

## How it works

An OSC sequence begins with `ESC ]` (shown below as `OSC`) and ends with the string terminator `ESC \` (`ST`). OSC 8 uses this form:

```text
OSC 8 ; parameters ; URI ST visible text OSC 8 ; ; ST
```

For example, a shell can print the label “Project docs” linked to a URL:

```sh
printf '\033]8;;https://example.com/docs\033\\Project docs\033]8;;\033\\\n'
```

The opener changes the hyperlink attribute for subsequently painted character cells. The closer resets that attribute. A URI may be an `https:`, `http:`, `mailto:`, or `file:` URI, among others. The optional parameter field is a colon-separated list of `key=value` pairs. `id` is the defined parameter; it lets applications identify the same logical link across line wraps or partial screen updates. Unknown parameters should be ignored. The target should be URI-encoded. For local file links, include a host only when appropriate and ensure it identifies the local machine; a remote host in a file URI must not be silently treated as a local path.

This is stream state rather than nested markup: switching directly from one link target to another is valid, and a defensive close can be sent even if the application is unsure whether a link is active. Emit a close around every linked span so later output does not accidentally inherit the target. Full screen programs and multiplexers should preserve and namespace link `id`s to avoid collisions between panes or windows.

OSC 8 has no standardized capability query. In particular, applications should not infer support from `TERM` alone, and a query intended for unrelated terminal features does not establish OSC 8 support. A manual smoke test is to run the example above and click the visible text. If the terminal supports OSC 8, it should activate the URL; a dotted/colored/underlined treatment on hover is common, but the protocol mandates no appearance. With no link-specific styling, the label may look like ordinary text until hovered. The test is a human visual and interaction check, not a reliable automatic probe: lack of an obvious underline does not prove lack of support, and unsupported terminals can still display the label.

Click behavior is terminal-specific. WezTerm opens links with a simple click by default (its mouse bindings can require Ctrl). iTerm2 documents Command-click. Ghostty's built-in URL matching uses Ctrl on Linux and Command on macOS; OSC 8 links participate in its link handling. kitty allows a mouse click and can also open a link through its hints kitten. Other terminals may use a modifier or a click/context-menu workflow. Programs should not assume a particular modifier; the emulator owns interaction and URI launching.

## Origin, adoption, and safety

The proposal was written by Egmont Koblinger and published in 2017, building on VTE and iTerm2 implementation work. It was designed as a small, backward-safe extension: terminal control bytes carry the target while the user sees the ordinary label. It has since been adopted by a broad set of terminal emulators, terminal libraries, multiplexers, and command-line tools. Adoption continues: recent terminal and utility releases have added OSC 8 output, including new terminal emulators and tools such as Cargo, ripgrep, `fd`, and `ls` from GNU Coreutils. Support is widespread among actively maintained modern terminals, but not universal; old versions and the stock Apple Terminal before macOS Tahoe 26 remain notable gaps. Consumers should retain a useful plain-text label and should not make a link the only way to discover its destination. Treat links as untrusted input: opening a URI can launch a browser, application, or local file handler, and file-link handling should validate local hostnames and paths.

## Terminal emulator support

The list below records known OSC 8 implementations, not a guarantee that every version, configuration, or remote/multiplexed session will pass links through. Version thresholds are included where available. “Click behavior” varies by application and user configuration.

| Terminal                    | Support and caveats                                                                                                                                                                                                                                                                                          |
|-----------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| WezTerm                     | Supported since early 2018. Simple click opens by default; configure mouse bindings to require Ctrl if desired. Its file-link recipe requires a local shell for some editor actions; file links inside an alternate-screen program are handled differently.                                                  |
| Ghostty                     | Supported. URL text matching uses Ctrl on Linux or Command on macOS; its link previews can be limited to OSC 8 links.                                                                                                                                                                                        |
| iTerm2                      | Supported since 3.1. Its documented interaction is Command-click. `id` distinguishes neighboring links sharing a URI; use IDs for robust grouping in applications that repaint or wrap links.                                                                                                                |
| cmux                        | Supported (cmux uses Ghostty's terminal engine). Ghostty's platform modifier conventions apply; check cmux release behavior if embedding or forwarding the terminal stream.                                                                                                                                  |
| kitty                       | Supported since 0.19. `allow_hyperlinks` can disable OSC 8, and `underline_hyperlinks` controls visual treatment. Click actions are configurable; the hints kitten is a keyboard alternative.                                                                                                                |
| Konsole                     | Supported since July 2020, but disabled by default in the reported implementation; enable hyperlink handling in Konsole's profile/settings before diagnosing application output.                                                                                                                             |
| Apple Terminal              | OSC 8 support is reported for macOS Tahoe 26 (observed in September 2026); older macOS releases do not support it. Treat this as version-specific and verify on the target macOS build, since Apple’s public release notes do not clearly document OSC 8. Plain-text URL recognition is separate from OSC 8. |
| Alacritty                   | Supported since 0.11 (October 2022). Older installations ignore it.                                                                                                                                                                                                                                          |
| Contour                     | Supported. It provides clickable links and has file-URI handling; recent releases fixed fully qualified local hostnames and added an OSC 8 target tooltip.                                                                                                                                                   |
| Foot                        | Supported since 1.7.0 (March 2021).                                                                                                                                                                                                                                                                          |
| GNOME Terminal              | Supported through VTE since GNOME Terminal 3.26; requires VTE 0.50.4, 0.52.2, or newer to avoid a rare crash. VTE-based terminals commonly expose links on hover for opening/copying.                                                                                                                        |
| GNOME Console               | Supported through VTE (listed since version 46.0).                                                                                                                                                                                                                                                           |
| Ptyxis                      | Supported through VTE (listed since version 47.6).                                                                                                                                                                                                                                                           |
| Guake                       | Supported through VTE; listed since 3.2.1.                                                                                                                                                                                                                                                                   |
| MATE Terminal               | Supported through VTE since 1.28.0.                                                                                                                                                                                                                                                                          |
| ROXTerm                     | Supported through VTE since 3.5.1.                                                                                                                                                                                                                                                                           |
| Terminator                  | Supported through VTE since 2.0. This refers to the Python/GTK application, not the unrelated Java terminal with the same name.                                                                                                                                                                              |
| Tilix                       | Supported through VTE since 1.5.8.                                                                                                                                                                                                                                                                           |
| Xfce Terminal               | Supported through VTE since 1.1.0.                                                                                                                                                                                                                                                                           |
| mintty                      | Supported since 2.9.7 (March 2019).                                                                                                                                                                                                                                                                          |
| Windows Terminal            | Supported since 1.4.3141.0 (November 2020).                                                                                                                                                                                                                                                                  |
| VS Code Integrated Terminal | Supported through xterm.js since VS Code 1.72. Remote-workspace `file:` URIs may not resolve safely to the expected local machine.                                                                                                                                                                           |
| Hyper                       | Supported since October 2019.                                                                                                                                                                                                                                                                                |
| DomTerm                     | Supported since 1.0.2 (May 2018).                                                                                                                                                                                                                                                                            |
| hterm                       | Supported since 1.76 (June 2018).                                                                                                                                                                                                                                                                            |
| Tabby                       | Supported.                                                                                                                                                                                                                                                                                                   |
| AbsoluteTelnet/SSH          | Support listed since 14.02 (August 2026).                                                                                                                                                                                                                                                                    |
| Secure ShellFish            | Support listed since 2025.23.                                                                                                                                                                                                                                                                                |
| tmux                        | Supported since 3.4, but must be enabled in tmux configuration with `set -ga terminal-features '*:hyperlinks'`. WezTerm also documents that a Shift-click may be needed depending on mouse-reporting settings.                                                                                               |
| Zellij                      | Supported since 0.21.0.                                                                                                                                                                                                                                                                                      |

Support references: [OSC 8 adoption tracker](https://github.com/Alhadis/OSC8-Adoption), [iTerm2 hyperlink behavior and design notes](https://iterm2.com/feature-reporting/Hyperlinks_in_Terminal_Emulators.html), [WezTerm hyperlink recipe](https://wezterm.org/recipes/hyperlinks.html), [Ghostty external protocols](https://ghostty.org/docs/vt/external), [kitty hyperlink configuration](https://sw.kovidgoyal.net/kitty/conf/), [Contour clickable links](https://contour-terminal.org/vt-extensions/clickable-links/), and [cmux feature support](https://terminfo.dev/terminals/cmux). For current Apple Terminal behavior, see the [September 2026 feature probe](https://terminfo.dev/terminals/terminal-app); this is an observed compatibility report, not an Apple protocol guarantee. The OSC 8 design and lack of a capability query are described in the [iTerm2 proposal](https://iterm2.com/feature-reporting/Hyperlinks_in_Terminal_Emulators.html), and the 2017 origin is documented in the [GNOME VTE implementation discussion](https://bugzilla.gnome.org/show_bug.cgi?id=779734).

## Multiplexers and producing links

Even a capable emulator may not receive OSC 8 unchanged through a multiplexer. tmux 3.4 and later needs `set -ga terminal-features '*:hyperlinks'`; older tmux releases may strip or fail to preserve links. Zellij supports OSC 8 from 0.21.0. Applications should emit hyperlinks when useful without requiring them: recent GNU Coreutils `ls` supports `--hyperlink=auto`, `ripgrep` supports `--hyperlink-format=kitty`, and GCC can add links to diagnostics. On macOS, Apple's bundled `ls` does not provide GNU's hyperlink option; install GNU Coreutils (typically `gls`) or use a tool such as `eza`.
