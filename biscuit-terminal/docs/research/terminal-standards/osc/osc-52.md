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
    - Alacritty
    - Contour
    - Foot
    - xterm
    - Windows Terminal
    - tmux
    - GNU screen
prompt: |-
    The OSC 52 informal standard is used for Clipboard read and write as base64. Many terminals disable reading by default for securiy.

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

    - set `apps_supporting` as a list of those apps which are known to support OSC 52
hash: 70cca6f72e737ffd-687a064f6dede596
last_updated: 2026-09-28
---
# OSC 52: clipboard access

OSC 52 is an informal terminal control sequence originating in xterm. It is not part of ECMA-48 and has no central standards body or conformance suite. Xterm's control-sequence reference documents it as an xterm operation; the historical versioned references put the feature in the early xterm/X11 era, but do not identify a single introduction date. It is best described as a long-standing de facto protocol that became especially useful for copying from remote shells over SSH. Adoption continues: newer terminal emulators and multiplexers have added it, and recent implementations are expanding from write-only behavior to permission-controlled reads. See [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html).

## How it works

The conventional form is `OSC 52 ; Pc ; Pd ST`, where `OSC` is `ESC ]`, `ST` is `ESC \`, `Pc` selects a clipboard or selection, and `Pd` is either Base64 data or `?`. The common clipboard selector is `c`; `p` usually means the X11 PRIMARY selection. An empty selector is treated differently by implementations, so portable senders should use `c`. For example, the following writes the UTF-8 bytes for `hello` to the system clipboard:

```text
ESC ] 52 ; c ; aGVsbG8= ESC \
```

The terminal emulator consumes the sequence and decodes its payload; the escape bytes are not meant to be displayed. To request a read, send `OSC 52 ; c ; ? ST`. If permitted and supported, the terminal replies on the terminal input stream with an OSC 52 sequence containing Base64 clipboard data. Replies can be large, arrive asynchronously, or not arrive at all. Applications therefore need timeouts and must avoid confusing a reply with user keystrokes. BEL (`0x07`) is accepted as a terminator by many implementations, but `ST` is the canonical form.

OSC 52 moves bytes, not a visible object. There is no standard icon, overlay, toast, or on-screen indication: the visible effect is normally only that another application can paste the changed clipboard. A terminal may show a permission prompt, but that UI is implementation-specific. For example, Ghostty can ask before a program reads the clipboard; iTerm2 requires user consent for both directions. The sequence represents text encoded as Base64, not images or rich clipboard formats. Kitty's separate OSC 5522 extension handles richer MIME-typed data and explicit permission outcomes; it is not ordinary OSC 52.

## Security and detection

Writing is an integrity concern: untrusted output can replace the user's clipboard, potentially influencing a later paste. Reading is a confidentiality concern: a remote or compromised process could exfiltrate clipboard contents. Terminals commonly permit writes more readily than reads, but policies change by app, version, configuration, focus, and whether the process is local or remote. A security-conscious application should make clipboard reads opt-in or prompt the user and should not assume a read is authorized merely because writes work.

There is no universally reliable, non-invasive OSC 52 capability query. Device-attributes reports and `$TERM` values are not consistently implemented as feature advertisements. A write probe changes the clipboard, so it is a destructive test; a read probe itself requests sensitive data. For manual testing, first copy a known disposable value, issue a read request, parse the terminal's reply with a timeout, and restore the clipboard if needed. For write testing, send a known short test string and verify it by pasting. Test through the actual SSH and multiplexer path too: a capable outer terminal cannot handle bytes a middle layer drops. Applications should prefer explicit user configuration, known terminal identities/version data where trustworthy, and graceful fallback over probing by secretly reading clipboard data.

## Application support

Support below means at least one documented OSC 52 operation; it does not imply that both read and write work, that access is enabled by default, or that every clipboard selector is honored. Version and configuration matter. Terminal support is also distinct from forwarding through tmux, screen, SSH clients, IDE terminals, and other layers.

| Terminal app     | Known behavior and developer notes                                                                                                                                                                                                                                                                              |
|------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| WezTerm          | Supports setting and clearing clipboard contents. Its documented implementation ignores clipboard queries, so OSC 52 paste/read cannot be used; use the terminal's native paste action for host-to-remote paste. The WezTerm escape-sequence table documents this limitation.                                   |
| Ghostty          | Supports read and write. Its documented selector handling recognizes `c`, `p`, and `s`, aliases other selectors to `c`, and currently handles only one clipboard per sequence. Clipboard reads default to an ask policy; configure `clipboard-read` to allow or deny explicitly.                                |
| iTerm2           | Supports writes; clipboard query is documented for version 3.5 and later. User consent is required for both directions. Its documentation says the OSC 52 `Pc` selector is ignored on macOS, so it targets the general pasteboard rather than choosing X11-style selections.                                    |
| cmux             | Listed as a supporting terminal because its terminal surface is built on Ghostty/libghostty and supports terminal clipboard handling. Treat exact read-policy behavior as dependent on the bundled Ghostty version and cmux release; use the app's permission behavior rather than assuming a prompt-free read. |
| kitty            | OSC 52 copying is supported, including large copies in recent releases. Clipboard reads are controlled by `clipboard_control`; reading is commonly disabled unless `read-clipboard` and/or `read-primary` is enabled. Kitty also offers OSC 5522 for richer clipboard data, a separate protocol.                |
| Konsole          | Supports OSC 52 clipboard writes in current versions. Do not assume query/read support: its implementation history and compatibility reports establish the write path, but read support is not consistently documented.                                                                                         |
| Alacritty        | Supports configurable copy and paste directions. The `terminal.osc52` setting accepts `Disabled`, `OnlyCopy`, `OnlyPaste`, or `CopyPaste`; the documented default is `OnlyCopy`. Enable `CopyPaste` only when clipboard reads are an acceptable trust boundary.                                                 |
| Contour          | Supports OSC 52 writes. Current project release notes also describe OSC 52 reads behind an opt-in policy, so check the installed release and configure that policy before relying on queries.                                                                                                                   |
| Foot             | Supports clipboard access. Its `security.osc52` setting distinguishes `disabled`, `copy-enabled`, `paste-enabled`, and `enabled`; the setting is especially relevant for SSH/container sessions because the host clipboard is exposed to the remote process.                                                    |
| xterm            | The original/reference implementation supports OSC 52, subject to its window-operation security policy. Administrators may need to permit the relevant clipboard operations through X resources; do not recommend globally relaxing that policy on untrusted sessions.                                          |
| Windows Terminal | Supports OSC 52 writes in current releases, with `compatibility.allowOSC52` controlling whether applications may write. Clipboard reads have historically not been supported; check release documentation before depending on them.                                                                             |
| tmux             | Can translate or forward OSC 52, but configuration and version affect behavior. `set-clipboard` controls tmux's clipboard integration; passthrough is a separate concern for nested applications. Configure the outer terminal and tmux forwarding, and test the exact nesting path.                            |
| GNU screen       | Has OSC 52 clipboard integration in supported configurations, but forwarding and clipboard permissions depend on screen version and settings. Validate through screen rather than inferring support from the outer terminal.                                                                                    |

Apps from the requested list that should not be marked as supported on the available evidence: Apple Terminal (Terminal.app) has no reliable native OSC 52 support; GNOME Terminal and other VTE-based terminals do not currently implement OSC 52 in the VTE layer. In those environments, a multiplexer such as tmux may provide a workaround if it can communicate with a capable outer terminal, but this does not make the terminal emulator itself an OSC 52 implementation. For Konsole and Contour, distinguish write support from read support, which is newer or less established.

Useful implementation references: [Ghostty OSC 52](https://ghostty.org/docs/vt/osc/52), [WezTerm escape sequences](https://wezterm.org/escape-sequences.html), [iTerm2 escape-code documentation](https://iterm2.com/documentation-escape-codes.html), [Alacritty configuration](https://alacritty.org/config-alacritty.html), [Foot configuration](https://man.archlinux.org/man/foot.ini.5), [Contour project](https://github.com/contour-terminal/contour), [kitty clipboard documentation](https://sw.kovidgoyal.net/kitty/clipboard/), and the [VTE OSC 52 support request](https://gitlab.gnome.org/GNOME/vte/-/issues/2495).
