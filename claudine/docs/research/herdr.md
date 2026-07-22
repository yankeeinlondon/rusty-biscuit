---
prompt: "The `herdr` multiplexer (https://herdr.dev/docs/) is becoming very popular to run AI workloads. \n\nYour task is to do research on it and then write your findings in this document. You should make sure your research can answer the following questions:\n\n- what is herdr? \n- what native features of a terminal app are lost when using herdr as a terminal (e.g., Kitty graphics, etc.)\n- what are the top features that herdr provides that are outside those you'd typically find in a multiplexor?\n- what is a herdr plugin (https://herdr.dev/docs/plugins/)?\n- what is the socket API that herdr provides? what would be common use cases that would benefit from that API?\n- what is a herdr integration? how does it compare to a plugin?\n- what are the ways in which herdr can be configured? we don't need every setting listed out but what are the various categories? \n- what ENV variables does herdr inject into the \n    - HERDR_SOCKET_PATH appears to be at least one ENV varible, is this the best way to \"detect\" herdr's activation? Is there a better way?"
last_updated: 2026-07-16
hash: 416a8d8c7d8fb036-a9894a02f2eecf32
---
# Herdr

> Research snapshot: July 16, 2026. Herdr is developing quickly, so experimental terminal features and integration coverage should be rechecked before implementation.

## What is Herdr?

[Herdr](https://herdr.dev/docs/) is an open-source, agent-aware terminal workspace manager and multiplexer. Like tmux or Zellij, it runs terminal processes in a background server so clients can detach and reconnect without stopping shells, servers, tests, or agents.

Its hierarchy is:

- **Session:** an isolated Herdr server, runtime, and socket namespace.
- **Workspace:** usually a repository, task, or investigation.
- **Tab:** a layout within a workspace.
- **Pane:** a real PTY-backed terminal.
- **Agent:** a recognized agent process within a pane.

Herdr is deliberately mouse-first: panes, tabs, agents, split borders, selections, and context menus are clickable. It also provides tmux-style prefix bindings, with `ctrl+b` as the default prefix. Its responsive UI remains usable in narrow phone and tablet terminals. [Herdr’s concepts documentation](https://herdr.dev/docs/concepts/) describes the client/server and workspace model.

The important distinction from a normal multiplexer is that Herdr understands AI coding agents. It recognizes agents, assigns semantic states such as `working`, `blocked`, `done`, and `idle`, and rolls those states up through panes, tabs, and workspaces.

## Features beyond a typical terminal multiplexer

Herdr’s most distinctive features are:

1. **Agent detection and attention management**
   
   Herdr recognizes many coding agents from their foreground process and terminal output. Optional integrations provide stronger lifecycle or session signals. Agent state is aggregated into a sidebar so a user can see which project needs input, is still working, or is ready for review. State can also drive notifications and API waits. [Agent detection and authority](https://herdr.dev/docs/agents/) are explicit parts of Herdr’s architecture.

2. **Native agent-session restoration**
   
   A traditional multiplexer preserves a live process while its server remains alive. Herdr additionally records native agent session identifiers through official integrations. After a full Herdr server restart—when the original PTYs are gone—it can relaunch supported agents using commands such as `claude --resume`, `codex resume`, or `opencode --session`.
   
   This does not restore arbitrary processes. Herdr separately restores workspace, tab, pane, layout, CWD, and focus metadata; arbitrary panes return as new shells. Optional pane-history replay restores display content but not the original process. [The session-state documentation](https://herdr.dev/docs/session-state/) explains these different persistence paths.

3. **Git worktree workflows**
   
   Worktrees are first-class objects in the UI, CLI, socket API, and event model. Herdr can create, open, remove, and group worktree-backed workspaces, making parallel branches natural places to run separate agents.

4. **Agent-oriented automation**
   
   Scripts can start an agent, read its pane, send input, wait for semantic state, subscribe to state changes, and attach directly to its terminal. This makes Herdr useful as a local orchestration substrate rather than only an interactive terminal UI.

5. **Remote thin-client mode**
   
   `herdr --remote <host>` connects over SSH, starts or attaches to a remote Herdr server, and renders it through a local client. It can preserve local keybindings and bridge local desktop behavior such as clipboard-image paste into the remote session. Normal “SSH first, then run Herdr” operation is also supported. [Remote access](https://herdr.dev/docs/persistence-remote/) works without a separate web or desktop application.

6. **Executable plugins and a broad local API**
   
   Herdr offers manifest-defined workflow plugins, event hooks, terminal plugin panes, link handlers, a CLI automation surface, and a raw socket protocol. This is substantially broader than the scripting interfaces exposed by many multiplexers.

7. **Continuously updated agent detection**
   
   Screen-detection manifests can receive compatible rule updates without upgrading the Herdr binary. Local TOML overrides take precedence, and `herdr agent explain` exposes the evidence behind a state decision.

## Terminal capabilities and trade-offs

Herdr is not a transparent byte-forwarding layer. It owns each pane’s PTY, interprets its terminal state, and renders that state into the outer terminal. Managed panes normally advertise `TERM=xterm-256color` and `COLORTERM=truecolor`, rather than inheriting the outer terminal’s identity. This avoids leaking terminal-specific terminfo across Herdr and SSH, but it also means an application is targeting Herdr’s emulation—not Kitty, Ghostty, iTerm2, or WezTerm directly. This behavior is visible in [Herdr’s pane implementation](https://github.com/ogulcancelik/herdr/blob/master/src/pane.rs).

| Capability                                                                            | Behavior inside Herdr                                                                                                                                                                                                                                                                                              |
|---------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| ANSI/VT text, styles, 256 colors, and true color                                      | Supported. These are the normal compatibility path.                                                                                                                                                                                                                                                                |
| OSC 8 hyperlinks                                                                      | Preserved by Herdr. With mouse capture enabled, Herdr handles pane links using Ctrl-click when the outer terminal reports the modified click.                                                                                                                                                                      |
| OSC 52 clipboard writes                                                               | Forwarded or bridged to the host clipboard. Herdr also supplies its own mouse selection and copy mode.                                                                                                                                                                                                             |
| Cursor shapes, focus reporting, bracketed paste, undercurl, and modern keyboard input | Supported or normalized by Herdr. Compatibility has been expanded over time; see the official [changelog](https://github.com/ogulcancelik/herdr/blob/master/docs/next/CHANGELOG.md).                                                                                                                               |
| Kitty graphics                                                                        | Experimental and disabled by default. Enable with `[experimental] kitty_graphics = true`. It should not yet be treated as equivalent to using a Kitty-capable application directly in the outer terminal.                                                                                                          |
| Sixel and iTerm2 inline-image protocols                                               | Not part of Herdr’s documented rendering surface. Source inspection found a Herdr renderer for Kitty graphics but no equivalent public Herdr renderer for these protocols. Treat them as unavailable unless a later release explicitly adds support.                                                               |
| Terminal-specific shell integration                                                   | Not automatically inherited. For example, Kitty’s automatic shell integration applies to shells Kitty launches directly, not shells created by a multiplexer. Manual shell integration may still be possible. [Herdr’s troubleshooting guide](https://herdr.dev/docs/troubleshooting/) documents this distinction. |
| Native terminal tabs, splits, command marks, and per-pane window controls             | Replaced by Herdr’s workspace, tab, pane, title, navigation, and scrollback model. An application inside a pane cannot directly create an outer-terminal tab or split through Herdr.                                                                                                                               |
| Native terminal selection and mouse behavior                                          | Herdr captures the mouse by default for selection, resizing, menus, and pane links. Set `ui.mouse_capture = false` when the outer terminal should handle ordinary clicks instead, at the cost of Herdr’s mouse UI.                                                                                                 |
| Host font rendering, glyphs, emoji, and potential ligatures                           | Still largely provided by the outer terminal because it ultimately renders Herdr’s text cells. They are not categorically lost.                                                                                                                                                                                    |

The practical rule is that portable xterm-compatible terminal behavior works well, while proprietary escape protocols require explicit Herdr support. Kitty graphics is currently the clearest example: it is implemented, but remains experimental and off by default. [The configuration guide](https://herdr.dev/docs/configuration/#kitty-graphics) documents that status.

## Plugins

A Herdr plugin is a shareable executable workflow package. It is a directory containing a `herdr-plugin.toml` manifest plus any scripts, binaries, dependencies, and supporting files the workflow requires.

A manifest can declare:

- actions invoked from the CLI, UI, or keybindings;
- event hooks such as `worktree.created`;
- terminal panes or popups;
- keybinding targets;
- modified-click link handlers;
- supported platforms and minimum Herdr version;
- optional installation-time build commands.

Plugins may be implemented in Bash, PowerShell, JavaScript, Python, Lua, Rust, Go, or any other executable form. There is no separate language SDK or restricted plugin command set: the Herdr CLI and socket API are the plugin API.

Herdr validates the manifest, supplies invocation context, launches declared commands, and records command logs. It does **not** sandbox or audit plugin code. Plugins run as the user, inherit the user’s environment, and can access the full Herdr control surface. They must therefore be treated like editor or shell extensions and installed only from trusted sources. [The plugin documentation](https://herdr.dev/docs/plugins/) covers the complete manifest and security model.

Plugins can be linked from a local directory during development or installed from GitHub shorthand such as `owner/repo/subdir`. Community discovery uses public repositories tagged with the `herdr-plugin` GitHub topic.

## Socket API

Herdr exposes the same underlying control surface through CLI wrappers and a local socket API. The raw protocol is newline-delimited JSON:

- Unix and macOS use a Unix domain socket.
- Windows uses a named pipe.
- Requests carry an `id`, method, and parameters.
- Responses repeat the request `id`.
- Subscription connections remain open and receive event messages.
- `herdr api schema --json` prints the JSON Schema bundled with the installed binary.

The API can manage sessions, workspaces, worktrees, tabs, pane layouts, panes, agents, plugins, integrations, notifications, server configuration, and terminal graphics. It can also read pane contents, send keys or text, report agent state, wait for conditions, and subscribe to lifecycle events. [The socket API guide](https://herdr.dev/docs/socket-api/) recommends using CLI wrappers for ordinary scripts and reserving the raw protocol for direct clients and long-lived subscriptions.

Common use cases include:

- spawning several agents and waiting until each becomes `done` or `blocked`;
- building a dashboard that subscribes to agent, pane, layout, or worktree events;
- reporting lifecycle state or native session identity from an agent hook;
- reading visible output or unwrapped scrollback for automation and diagnostics;
- applying reproducible pane layouts for a repository;
- driving terminal applications in integration tests;
- reacting to a new Git worktree by opening panes and starting project commands;
- sending desktop or in-terminal notifications from external tools;
- implementing a Herdr plugin without shelling out for every operation;
- integrating an unsupported agent by reporting semantic state over the socket.

Socket resolution follows this order:

1. explicit `--session <name>`;
2. `HERDR_SOCKET_PATH`;
3. `HERDR_SESSION=<name>`;
4. the default session socket.

`HERDR_SOCKET_PATH` is documented as a low-level override. Portable plugins should generally call the executable named by `HERDR_BIN_PATH`, because the CLI hides the Unix-socket versus Windows-named-pipe difference.

## Integrations compared with plugins

A Herdr integration is a built-in, provider-specific adapter that connects Herdr to an external coding agent’s native extension, plugin, or hook system. Its purpose is narrow: report authoritative lifecycle state, native session identity, or both.

Running `herdr integration install codex`, for example, installs Herdr-managed hook assets into Codex’s configuration. Other integrations install shell hooks, JavaScript plugins, TypeScript extensions, or provider-specific configuration entries. The reported data is sent back through Herdr’s local socket API.

|                 | Herdr plugin                                                    | Herdr integration                                                    |
|-----------------|-----------------------------------------------------------------|----------------------------------------------------------------------|
| Primary purpose | Add a reusable workflow or user-facing extension to Herdr       | Adapt a particular external agent to Herdr                           |
| Ownership       | Third-party or local package                                    | Bundled and managed by Herdr                                         |
| Installation    | `herdr plugin install` or `herdr plugin link`                   | `herdr integration install <agent>`                                  |
| Contract        | `herdr-plugin.toml`, executable commands, CLI/socket API        | Provider-specific hooks or extension APIs plus Herdr’s reporting API |
| Typical outputs | Actions, event reactions, panes, popups, layouts, link handlers | Agent state and native session identity                              |
| Distribution    | Local directory or GitHub repository                            | Shipped with the Herdr binary                                        |
| Trust model     | Arbitrary unsandboxed third-party code                          | Official Herdr-managed adapter                                       |

The terminology can be confusing when an external agent itself calls extensions “plugins.” For example, the OpenCode integration installs an **OpenCode plugin**, but that asset is a **Herdr integration**, not a Herdr plugin.

Integrations also differ in authority. Pi, OMP, Kimi, OpenCode, Kilo, Hermes, and MastraCode integrations can author lifecycle state. Claude Code, Codex, Copilot, Devin, Droid, Qoder, and Cursor integrations primarily provide native session identity, while Herdr continues using screen detection for state. [The integrations guide](https://herdr.dev/docs/integrations/) documents the per-agent behavior.

## Configuration

Herdr works without a configuration file. Configuration is available through several surfaces:

- **TOML configuration:** `~/.config/herdr/config.toml` on Unix and macOS, or `%APPDATA%\herdr\config.toml` on Windows.
- **In-app Settings:** exposes common preferences and integration installation.
- **CLI options:** select sessions, remote operation, startup behavior, and one-off command parameters.
- **Environment variables:** override paths, session selection, logging, and selected runtime behavior.
- **Local agent-detection manifests:** replace bundled or remotely updated detection rules for a known agent.
- **Plugin manifests:** configure plugin entrypoints and capabilities separately from core Herdr settings.

`herdr --default-config` prints a complete default file. Most changes can be applied with `herdr server reload-config` or the in-app reload action; startup-only changes require a restart.

The main configuration categories are:

- general and onboarding behavior;
- terminal shell, shell mode, and new-pane CWD policy;
- update channel and version or detection-manifest checks;
- keybindings, prefix behavior, indexed navigation, and custom commands;
- themes and custom colors;
- UI, mouse handling, sidebar layout, responsive/mobile behavior, and pane borders;
- toast, terminal, system, and clipboard notifications;
- sounds and per-agent sound preferences;
- session persistence, history, and native agent restoration;
- Git worktree location and behavior;
- remote attach and SSH management;
- scrollback and other advanced behavior;
- experimental features, including Kitty graphics, pane history, nested launches, and macOS input behavior.

The canonical setting list is available in the [config reference](https://herdr.dev/docs/config-reference/).

## Injected environment variables

Ordinary Herdr-managed pane processes receive:

| Variable              | Meaning                                                                |
|-----------------------|------------------------------------------------------------------------|
| `HERDR_ENV=1`         | Explicit marker that the process is inside a Herdr-managed environment |
| `HERDR_SOCKET_PATH`   | Socket or named-pipe endpoint for the active Herdr session             |
| `HERDR_WORKSPACE_ID`  | Public identifier of the containing workspace                          |
| `HERDR_TAB_ID`        | Public identifier of the containing tab                                |
| `HERDR_PANE_ID`       | Public identifier of the containing pane                               |
| `TERM=xterm-256color` | Stable terminal capability identity                                    |
| `COLORTERM=truecolor` | Advertises true-color support                                          |

Context-specific commands receive additional variables:

- **Custom keybinding commands:** `HERDR_BIN_PATH`, `HERDR_ACTIVE_WORKSPACE_ID`, `HERDR_ACTIVE_TAB_ID`, `HERDR_ACTIVE_PANE_ID`, and `HERDR_ACTIVE_PANE_CWD`.
- **Plugin runtime commands:** `HERDR_BIN_PATH`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`, and `HERDR_PLUGIN_CONTEXT_JSON`, plus available workspace, tab, and pane IDs.
- **Plugin actions:** `HERDR_PLUGIN_ACTION_ID`.
- **Plugin event hooks:** `HERDR_PLUGIN_EVENT` and `HERDR_PLUGIN_EVENT_JSON`.
- **Plugin pane entrypoints:** `HERDR_PLUGIN_ENTRYPOINT_ID`.
- **Plugin link handlers:** link-specific values are included in `HERDR_PLUGIN_CONTEXT_JSON`, with convenience variables for the clicked URL and handler ID.

Popup commands are not normal tiled panes and therefore do not receive `HERDR_PANE_ID`.

Variables such as `HERDR_CONFIG_PATH`, `HERDR_SESSION`, `HERDR_LOG`, and `HERDR_DISABLE_SOUND` are primarily inputs that Herdr reads; they should not be confused with pane identity variables that Herdr injects. The [CLI environment-variable reference](https://herdr.dev/docs/cli-reference/#environment-variables) lists the public variables.

### Detecting Herdr

`HERDR_ENV=1` is the best documented test for “this process is running inside a Herdr-managed pane”:

```sh
if [ "${HERDR_ENV:-}" = "1" ]; then
    echo "running inside Herdr"
fi
```

`HERDR_SOCKET_PATH` alone is a weaker detector because:

- it is explicitly a low-level endpoint override;
- a caller can set it outside Herdr;
- it identifies where a client should connect, not why the current process exists;
- an inherited value does not prove that the server is still reachable.

Use progressively stronger checks according to the operation:

1. Check `HERDR_ENV=1` to detect Herdr ancestry.
2. Require `HERDR_PANE_ID` when the operation must target the current pane.
3. Require `HERDR_SOCKET_PATH` when the operation needs the API.
4. Ping the server, or run a lightweight CLI status command, when live connectivity matters.

For example:

```sh
if [ "${HERDR_ENV:-}" = "1" ] &&
   [ -n "${HERDR_PANE_ID:-}" ] &&
   [ -n "${HERDR_SOCKET_PATH:-}" ]; then
    # Safe to attempt a pane-scoped Herdr API operation.
    :
fi
```

These variables are inherited by descendants, so they prove that a process originated within a Herdr-managed environment, not that it is currently the pane’s foreground process. `TERM` should not be used for detection because `xterm-256color` is intentionally generic.
