---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: "An informal OSC extension reserved by xterm for Emacs shell integration and reused by Vim's embedded terminal API. OSC 51 carries application-specific commands in a terminal control string; it is not a general-purpose feature implemented by ordinary terminal emulators."
apps_supporting: []
prompt: |-
    The OSC 51 code is reserved in xterm for Emacs shell use, and Vim's `:terminal` uses it as a channel for jobs in the terminal window to call Vim functions.

    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - How do Emacs and Vim (and Neovim) each use it, and what is the payload format?
        - What happens when OSC 51 is sent to a terminal emulator that does not reserve it?
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

    - set `apps_supporting` as a list of those apps which are known to support OSC 51
    - set `description` as a one to two sentence description of this standard
hash: bed1dddd797911a8-b6900ce59b6d1335
last_updated: 2026-09-28
---
# OSC 51: Emacs shell and Vim terminal requests

OSC 51 is an informal use of the Operating System Command (OSC) control-string family. Its conventional 7-bit form is `ESC ] 51 ; payload ST`, where `ST` is usually `ESC \\`; BEL is also commonly accepted as a string terminator. Xterm's control-sequence reference assigns parameter 51 as “reserved for Emacs shell,” but does not define a universal payload grammar or a standard query/reply. The meaning after `51;` depends on the receiving application. It is therefore best understood as a private application channel that happens to use an OSC number, not a terminal-independent API.

## How it works and how to test it

The process running in a terminal writes the OSC bytes to its pseudo-terminal output. A terminal emulator normally parses control strings itself; for OSC 51 to trigger an editor action, the component parsing that stream must recognize OSC 51 and dispatch its payload to the editor. In Vim, that parser is Vim's embedded terminal implementation. In Emacs packages such as vterm and Eat, it is the terminal emulator inside Emacs. A shell running directly in WezTerm, Ghostty, or another GUI terminal does not thereby gain access to Emacs or Vim.

There is no reliable, generic capability query for OSC 51. Xterm's reservation is not a query response, and terminal identification variables such as `TERM` do not certify handling of this application-specific payload. Test the actual consumer by emitting a harmless, expected request in the target context: for Vim, use a controlled `Tapi_` function in a Vim `:terminal` buffer; for an Emacs terminal buffer, use that emulator's documented shell helper. A request sent to an unrelated outer terminal only tests its fallback behavior, not whether Vim's or Emacs's internal terminal parser supports it.

## Payloads and consumers

### Emacs

Emacs use is not one single wire format across every terminal mode or package. The commonly used emacs-libvterm convention is `OSC 51 ; E` followed by a shell-quoted Emacs Lisp command string and a terminator. For example, `printf '\\e]51;Emessage "Hello"\\e\\\\'` asks vterm to evaluate a message command. Vterm restricts executable commands to its `vterm-eval-cmds` allowlist, and its documentation requires shell quoting/escaping of quotes and backslashes. Other Emacs terminal implementations may define their own OSC 51 subcodes; Eat has used `51;e;...` forms for its own terminal metadata and prompt integration. Do not assume an OSC 51 payload written for vterm is portable to Eat or built-in `term`.

The shell helpers matter: vterm commonly arranges `vterm_printf` to emit the sequence and to wrap it for tmux when needed. When bypassing those helpers, tmux may consume or fail to forward the control string. Use the emulator's helper or configure the multiplexer to pass the OSC through rather than assuming the outer GUI terminal will dispatch it.

### Vim

Vim's `:terminal` recognizes OSC 51 and interprets the payload as JSON. The common form is a JSON array beginning with an operation: `["drop", path]` asks Vim to drop/open a file, while `["call", "Tapi_Function", argument]` calls a Vim function with the terminal buffer number and decoded JSON argument. The default function prefix is `Tapi_`; `term_setapi()` can change it or disable function calls for a buffer. Vim's documentation warns that the called function should validate its argument. The command can be emitted, for example, with `printf '\\033]51;["drop", "README.md"]\\007'` from a Vim terminal job. BEL termination is shown in Vim's examples; ST also works in many implementations.

This protocol belongs to Vim's own terminal buffer parser: it does not require the outer terminal emulator to implement OSC 51. The `VIM_TERMINAL` environment variable is set for jobs launched in that context and is commonly used by shell integration to decide when to emit OSC 51. Windows console/ConPTY paths have had reports where the native console layer quietly ignores or mangles the sequence; the available evidence does not establish uniform behavior across every Windows host/version. Test the exact Vim build and console path, or use Vim's client-server mechanism as an alternative.

### Neovim

Neovim's current `:terminal` documentation describes `TermRequest` handling for OSC 7, OSC 52, and OSC 133, but does not document OSC 51 or Vim's `Tapi_`/JSON terminal API. Do not assume Vim's OSC 51 behavior is available in Neovim merely because both editors have terminal buffers. For Neovim, use its documented `TermRequest` event for supported request sequences or an explicit RPC/client-server integration.

## Unsupported receivers and support status

An unrecognized OSC is normally consumed as a control string by a terminal parser and has no visible effect. It should not be treated as printable text or as a request that will automatically reach a parent editor. Actual handling of unknown controls and malformed/incomplete strings is implementation dependent; a broken terminator can leave subsequent output interpreted as part of the control string. Keep payloads terminated, and do not send sensitive or side-effecting commands unless the receiver is trusted.

The support list below is intentionally strict: it lists standalone terminal applications with direct evidence that they dispatch OSC 51 to a host-side editor integration. The xterm reference reserves the number, but does not document a general xterm action or capability probe. Emacs vterm, Eat, and Vim's embedded terminal are OSC 51 consumers, but they are terminal implementations inside editors rather than standalone terminal applications; they are described above and are not entered as outer terminal apps.

| Terminal application          | Status                                              | Notes                                                                                                                                                                   |
|-------------------------------|-----------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| WezTerm                       | No verified OSC 51 dispatch                         | Vim's embedded terminal can handle the sequence before it reaches WezTerm.                                                                                              |
| Ghostty                       | No verified OSC 51 dispatch                         | A May 2026 Ghostty discussion requested OSC 51 parsing for ghostel, an Emacs frontend using libghostty; the request indicates this was not an existing general feature. |
| iTerm2                        | No verified OSC 51 dispatch                         | OSC 51 is not part of the documented iTerm2 shell integration protocol.                                                                                                 |
| cmux                          | No verified OSC 51 dispatch                         | No primary documentation found for OSC 51 handling.                                                                                                                     |
| kitty                         | No verified OSC 51 dispatch                         | No OSC 51 handler or capability report found in its published protocol documentation.                                                                                   |
| Konsole                       | No verified OSC 51 dispatch                         | No primary documentation found for OSC 51 handling.                                                                                                                     |
| Apple Terminal (Terminal.app) | No verified OSC 51 dispatch                         | No primary documentation found for OSC 51 handling.                                                                                                                     |
| Alacritty                     | No verified OSC 51 dispatch                         | No primary documentation found for OSC 51 handling.                                                                                                                     |
| Contour                       | No verified OSC 51 dispatch                         | No primary documentation found for OSC 51 handling.                                                                                                                     |
| foot                          | No verified OSC 51 dispatch                         | No primary documentation found for OSC 51 handling.                                                                                                                     |
| GNOME Terminal (VTE)          | No primary documentation found                      | No documented OSC 51 dispatch contract located.                                                                                                                         |
| xterm                         | Reserved, but not a documented general dispatch API | The reference says “reserved for Emacs shell”; reservation alone does not prove a general editor integration or queryable support.                                      |
| mintty                        | No verified OSC 51 dispatch                         | Its xterm compatibility target does not establish an OSC 51 editor-dispatch contract.                                                                                   |
| Windows Terminal              | No verified OSC 51 dispatch                         | Windows VT/ConPTY handling has been reported to ignore Vim's request; test the exact console stack.                                                                     |

This is not a claim that OSC 51 can never pass through these terminals in any configuration. Multiplexers, nested terminal parsers, platform console layers, and editor buffers can intercept, transform, consume, or forward control strings differently. There is no centrally maintained compatibility matrix and no evidence of broad terminal-emulator adoption; current use is concentrated in editor-owned terminal integrations. Adoption continues within those integrations, including newer Emacs terminal packages, but recent requests to add host-dispatch support to a general-purpose terminal engine show that OSC 51 has not become a common terminal feature.

## Origins and references

OSC itself is the long-standing ECMA-48 control-string mechanism. The application-specific OSC 51 assignment appears in Xterm Control Sequences as “reserved for Emacs shell.” The current xterm reference does not state when that reservation was first added, so a precise introduction date cannot be established from this primary source. Vim's `:terminal` JSON API is a later, separate reuse of the reserved number. A 2021 workflow write-up documents it in practical use, and current Vim help documents the API and its JSON/function rules.

References:

- [Xterm Control Sequences, OSC 51](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [Vim terminal help: terminal API and shell integration](https://vimhelp.org/terminal.txt.html)
- [Neovim terminal help](https://neovim.io/doc/user/terminal/)
- [emacs-libvterm message passing documentation](https://github.com/akermu/emacs-libvterm#message-passing)
- [emacs-libvterm command allowlist](https://github.com/akermu/emacs-libvterm/blob/master/vterm.el)
- [Ghostty discussion requesting OSC 51 dispatch for ghostel](https://github.com/ghostty-org/ghostty/discussions/12759)
- [Vim terminal OSC 51 example and Windows caveat](https://stackoverflow.com/questions/76548466/using-vims-terminal-json-api-from-pwsh-windows-terminal)
- [Opening files from Vim's terminal with the JSON API](https://incenp.org/notes/2021/open-file-from-vim-terminal.html)
