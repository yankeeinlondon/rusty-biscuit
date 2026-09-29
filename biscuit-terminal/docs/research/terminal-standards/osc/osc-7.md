---
description: >-
    OSC 7 lets a shell report its current working directory to a terminal as a file URI. Terminals use that metadata for directory-aware tabs, panes, titles, and integrations.
apps_supporting:
    - WezTerm
    - Ghostty
    - iTerm2
    - cmux
    - kitty
    - Konsole
    - Apple Terminal
    - Contour
    - foot
    - GNOME Terminal (VTE)
$schema:
    description: string(required)
    apps_supporting: string[](required)
prompt: |-
    The OSC 7 informal standard lets a shell report its current working directory as a `file://host/path` URL so the terminal can open new tabs, panes, or windows in the same directory.

    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - What visible behavior does it enable? How does that vary by terminal app (new tab/pane inheritance, title display, semantic features)?
        - How are the hostname and percent-encoding of the path handled, and what happens for a remote (SSH) host?
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

    - set `apps_supporting` as a list of those apps which are known to support OSC 7
    - set `description` as a one to two sentence description of this standard
hash: cc7e8a15e7ca54c3-eb67090e7542ff8d
last_updated: 2026-09-28
---
## Overview

OSC 7 is an informal terminal escape convention, rather than an ECMA-48 or XTerm-standardized command. The shell (or another application that knows the effective working directory) sends a URI to the terminal, conventionally whenever the prompt is drawn or the directory changes:

```text
ESC ] 7 ; file://hostname/absolute/path ST
```

`ESC ]` starts an Operating System Command; `ST` is String Terminator (`ESC \\`). Many terminals also accept BEL (`\a`) as the terminator. For a local POSIX shell, a typical payload is `file://localhost/home/alice/project`. OSC 7 does not change the shell's directory; it updates the terminal's metadata for that session. Terminals can then use it to choose the starting directory of a new tab, pane, or window, show a directory in chrome, or expose a current-directory URI to an integration. See [Ghostty's OSC 7 reference](https://ghostty.org/docs/vt/osc/7), [WezTerm's shell integration guide](https://wezterm.org/shell-integration.html), and [foot's control-sequence reference](https://manpages.debian.org/bookworm/foot/foot-ctlseqs.7.en.html).

OSC 7 has no standard query or positive acknowledgement. A terminal generally does not reply when it accepts the report, so a shell cannot reliably ask “do you support OSC 7?” by sending a probe. A developer can test it by sending a known URI and observing a terminal API/property or a user-visible operation that consumes the value (for example, creating a new tab with “inherit directory” enabled). Automated tests can separately test sequence parsing and the terminal application's directory-inheritance policy; a headless VT parser accepting OSC 7 does not prove that its host application exposes or uses the directory.

The sequence itself has no required visible rendering. Its common visible effects are indirect: a new tab/split starts in the reported directory; a sidebar, tab tooltip, title, or menu may display it; and integrations may resolve relative file paths or offer file operations. Directory display is a terminal UI choice, not a title-setting function: OSC 0/2 set titles independently. Some apps combine title escape codes, shell integration, and OSC 7 to compose a title, while others show the directory in a separate chrome element or not at all. More semantic shell features—prompt/command boundaries, command status, history, notifications, and selection of command output—come from other sequences such as OSC 133 or vendor protocols, not from OSC 7 alone.

### URI, hostname, paths, and SSH

The payload is a `file:` URI, not a raw shell path. Use an absolute path and URI-percent-encode path bytes that are not legal as URI path characters (at minimum spaces, `%`, `#`, and `?`; encode UTF-8 bytes as needed). Do not percent-encode `/` separators. For example, `/home/alice/my project` becomes `/home/alice/my%20project`. URI encoding is different from shell quoting: quoting a string in `printf` does not URI-encode it. Use a URI-aware encoder in general-purpose integrations, and ensure the OSC terminator and any shell prompt machinery cannot be injected by path data.

For local sessions, `localhost` is the most portable hostname. Some implementations accept an empty authority or the local machine's hostname; others insist the authority identifies the local host. `foot` documents that its hostname must refer to the local host. A non-local hostname is meaningful in an SSH session: it identifies the machine that owns the path. It does not make the remote path exist on the terminal host, and OSC 7 does not define SSH transport, remote process launching, or filesystem access. If a terminal inherits a remote URI as though it were a local path, spawning a local shell can fail or point at an unrelated path. Integrations should preserve the remote identity and avoid treating a remote path as a local `cwd`; a terminal with explicit SSH integration can instead create a corresponding remote session. Terminal behavior varies: iTerm2 shell integration tracks host and directory over SSH, while cmux has documented cases where ordinary SSH leaves its displayed cwd at the local launch directory. See [iTerm2 shell integration](https://iterm2.com/documentation-shell-integration.html) and [cmux issue 4680](https://github.com/manaflow-ai/cmux/issues/4680).

### Origin, adoption, and support

WezTerm's documentation identifies the convention as originating in Apple's Terminal application; the sequence is commonly called OSC 7 because 7 is its operating-system command number. The historical date of the first implementation is not specified in the available terminal documentation, so it is safer to describe its origin without assigning a release year. The convention spread through shell integration scripts such as GNOME VTE's `vte.sh`, which has been installed since VTE 0.34.5 and is automatically sourced by some distributions. It is now implemented by a broad range of desktop terminal emulators and shell integrations, and current shell documentation (including Fish) continues to emit it. Adoption continues as newer terminals add working-directory reporting and semantic integrations, but there is no central normative specification or universal behavior contract. Support must be evaluated at the application level and may require shell integration to be enabled.

## Terminal application support

“Supports” here means the app consumes OSC 7 as current-directory metadata. Shell emission is a separate requirement. In particular, a VT parser can recognize the bytes without the application offering any cwd feature.

| Terminal                          | What OSC 7 enables / implementation notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
|-----------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| WezTerm                           | Shell integration reports the cwd; new tabs, panes, and windows generally inherit it. `pane:get_current_working_dir()` exposes the URI. If OSC 7 is absent for a local pane, WezTerm can fall back to OS process inspection; that fallback cannot discover a remote SSH cwd. [Docs](https://wezterm.org/shell-integration.html)                                                                                                                                                                                              |
| Ghostty                           | Built-in shell integration reports cwd; new terminals can inherit the focused terminal's working directory. Integration also enables separate prompt navigation and command-output features. macOS's bundled `/bin/bash` does not support Ghostty's automatic injection: source Ghostty's Bash integration manually or use a supported Bash. [Shell integration](https://ghostty.org/docs/features/shell-integration) · [OSC 7](https://ghostty.org/docs/vt/osc/7)                                                           |
| iTerm2                            | Uses OSC 7 as a synonym for its `RemoteHost` plus `CurrentDir` reports. Shell Integration adds cwd-aware new-tab behavior (configurable), directory history, path actions, and remote host awareness. Install/enable shell integration on remote hosts for tracking across SSH; regular tmux mode has limitations, while tmux control mode works better. [Escape codes](https://iterm2.com/documentation-escape-codes.html) · [Shell integration](https://iterm2.com/documentation-shell-integration.html)                   |
| cmux                              | Tracks current directory in its tab/sidebar and uses pane cwd when creating sessions. It uses Ghostty/libghostty terminal machinery. Plain SSH may not forward remote cwd to cmux, leaving the local launch directory displayed; first-class remote SSH needs cwd propagation across its relay. [Configuration](https://github.com/manaflow-ai/cmux/blob/main/cmux-tui/docs/configuration.md) · [SSH cwd issue](https://github.com/manaflow-ai/cmux/issues/4680)                                                             |
| kitty                             | Supports cwd-aware launches; its documented `launch --cwd=current` can start a new window or tab in the current cwd. Shell integration can also set a dynamic window title, but that title behavior is separate from OSC 7 itself. [FAQ](https://github.com/kovidgoyal/kitty/blob/master/docs/faq.rst)                                                                                                                                                                                                                       |
| Konsole                           | Reads OSC 7 as a local file URL and exposes a current working directory; tabs can use directory-aware naming and spawning. It uses process inspection as a fallback when no URL was reported. A stale OSC 7 report can take precedence, so shells or full-screen programs that change directories should report again on exit. [Source](https://sources.debian.org/src/konsole/4%3A16.12.0-4/src/Session.cpp) · [Handbook](https://docs.kde.org/stable_kf6/en/konsole/konsole/konsole.pdf)                                   |
| Apple Terminal (Terminal.app)     | The origin implementation; shell integration lets Terminal track the current directory and use it for new tabs/windows and directory-aware title/breadcrumb UI. Apple's default shell startup files provide integration for supported shells; replacing prompt hooks can interfere with it. [WezTerm's origin note](https://wezterm.org/shell-integration.html)                                                                                                                                                              |
| Contour                           | Parses OSC 7 as `SETCWD`; it has cwd-aware panes/tabs. Its release notes document a Windows bug involving remote OSC 7 directories being passed as local process paths, so validate locality before spawning. [Sequence list](https://contour-terminal.org/vt-sequence/) · [release notes](https://contour-terminal.org/release-notes/)                                                                                                                                                                                      |
| foot                              | Explicitly uses OSC 7 to launch newly spawned foot instances in the reported directory. The URI hostname must refer to the local host; do not send a remote host as if its path were local. [man page](https://manpages.debian.org/bookworm/foot/foot-ctlseqs.7.en.html)                                                                                                                                                                                                                                                     |
| GNOME Terminal / VTE applications | VTE stores the directory URI from OSC 7 and its `vte.sh` integration enables new terminals to start in the current directory. Since VTE 0.34.5 the script has been installed system-wide; whether it is sourced automatically depends on the distribution and login-shell settings. If customizing Bash `PROMPT_COMMAND`, preserve/call VTE's hook. [GNOME Terminal FAQ](https://wiki.gnome.org/Apps/Terminal/FAQ) · [VTE property](https://gnome.pages.gitlab.gnome.org/vte/gtk3/const.TERMPROP_CURRENT_DIRECTORY_URI.html) |
| Alacritty                         | The Alacritty terminal application does not currently expose OSC 7 cwd tracking/inheritance as a supported feature. Its `alacritty_terminal` parser has also been reported not to expose OSC 7 through its public API. Applications embedding that parser may need a separate OSC 7 scanner.                                                                                                                                                                                                                                 |

The support list reflects documented behavior as checked on 2026-09-28; support can change between releases and may differ between an emulator, an embedded terminal widget, and a shell integration.

### Practical shell integration and test guidance

Emit the report after every directory change or before each prompt, so a shell that changes directories through functions, hooks, or command wrappers updates the terminal. Prefer the terminal's maintained shell integration script where available: these scripts handle shell-specific prompt hooks, URI escaping, local hostname rules, and interactions with custom prompts. For a minimal local POSIX example, after correctly URI-encoding `$PWD`, emit `printf '\033]7;file://localhost%s\033\\' "$encoded_path"`. Do not blindly concatenate raw `$PWD` into a URI.

For a smoke test, move to a directory containing a space, verify the terminal's cwd display/API, then request a new tab or pane and confirm it starts there. Repeat inside SSH only when the terminal documents remote shell integration, and confirm that a remote URI is not accidentally launched as a local path. Because OSC 7 has no capability query or acknowledgement, this behavioral test is more meaningful than checking for an OSC response.
