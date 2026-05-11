# Multiplexing Partial/Qualified Support Deep Dive

Last updated: 2026-02-14

This is a companion to `/Volumes/coding/personal/rusty-biscuit/biscuit-terminal/docs/multiplexing.md` and focuses only on the `⚠️ partial/qualified` entries.

## What "Partial/Qualified" Means Here

A feature is marked `⚠️` when at least one of these is true:

- The terminal supports a related capability, but not with independent-pane semantics.
- The capability exists, but only through static config or startup-time restore.
- The capability is scriptable, but not with robust targeting/control of arbitrary panes at runtime.
- The feature is available only at window/tab level, not pane level.

## Partial Matrix (All `⚠️` Items)

| Terminal | Feature | Why Qualified |
| --- | --- | --- |
| WezTerm | Save Layouts / Save-Restore (programmatic) | Strong scripting, but no single canonical "snapshot and restore everything exactly" flow equivalent to a dedicated session file UX. |
| Ghostty | Execute in Pane (overall + programmatic) | Actions/config are good for splits/navigation, but arbitrary command injection into specific existing splits is less explicit than Kitty/WezTerm style remote control. |
| Ghostty | Save/Restore (programmatic) | `window-save-state` exists, but this is policy/config-driven restore rather than a rich runtime session API. |
| iTerm2 | Save/Restore (programmatic) | Window Arrangements are strong, but this is closer to arrangement management than a universal runtime session snapshot API for all process state. |
| Apple Terminal | Split/Resize/Focus (overall) | "Split panes" behavior is view-oriented and does not provide independent-shell multiplex semantics. |
| Apple Terminal | Save/Restore (overall + programmatic) | Window Groups restore windows/tabs, but pane-level multiplex restore and automation are limited. |
| Warp | Resize (programmatic) | Launch Configs define startup layouts, but runtime external control is limited. |
| Warp | Focus (programmatic) | Focus behavior is mostly interactive; static launch config does not provide broad runtime focus API semantics. |
| Konsole | Split/Resize/Focus (overall + programmatic) | Split View is documented as duplicated output views; semantics differ from independent multiplexer panes. |
| Konsole | Execute in Pane (programmatic) | DBus methods can execute commands in sessions, but targeting/flow is more indirect than purpose-built pane-control APIs. |

## DBus/QDBus Deep Dive (Konsole and KDE)

### What DBus Is

DBus (Desktop Bus) is an IPC/RPC message bus used heavily on Linux desktops. Apps expose objects and methods on a bus; other processes call those methods.

Core concepts:

- **Bus**: usually session bus (user desktop) or system bus (system services).
- **Service name**: e.g. `org.kde.konsole`.
- **Object path**: e.g. `/Windows/1`, `/Sessions/1`.
- **Interface + method**: e.g. `org.kde.konsole.Session.runCommand`.

### What QDBus Is

`qdbus` is the Qt CLI tool for introspection and method calls over DBus.

- Discover services: `qdbus`
- Inspect a service: `qdbus org.kde.konsole`
- Inspect object methods: `qdbus org.kde.konsole /Windows/1`

`qdbusviewer` is the GUI equivalent for discovery/introspection.

### Practical Workflow for Konsole Automation

1. Find the running Konsole service name.
2. Introspect window/session objects to see available methods.
3. Create/load the window layout shape.
4. Run commands in the target sessions.

Example flow:

```bash
# 1) Find service
qdbus | grep konsole

# 2) Inspect known objects
qdbus org.kde.konsole /Windows/1
qdbus org.kde.konsole /Sessions/1

# 3) Load a saved layout at startup (JSON produced by Konsole UI)
konsole --layout /path/to/layout.json

# 4) Run command in a known session (method names can vary by version; introspect first)
qdbus org.kde.konsole /Sessions/1 org.kde.konsole.Session.runCommand "htop"
```

### Why Konsole Still Gets `⚠️`

- Official docs describe split view as duplicated output between views.
- You can automate parts of session control via DBus.
- But pane semantics and direct runtime pane-manipulation UX are less explicit/clean than Kitty (`kitten @`) or WezTerm (`wezterm cli` + Lua).

### DBus/QDBus Tips

- Always introspect methods on your installed version before scripting.
- Prefer stable startup flows (`--layout`) plus session command dispatch over brittle "click simulation."
- Handle service-name variation (`org.kde.konsole` vs instance-scoped names) in scripts.

## WezTerm: Why Save/Restore Is Qualified

What works well:

- Deterministic startup with Lua.
- Runtime control with `wezterm cli` (split panes, send text, etc.).

Why still `⚠️`:

- The model is "compose your session behavior" rather than "capture and restore every transient runtime detail automatically."
- In practice, teams treat this as layout/session bootstrapping rather than exact runtime snapshot replay.

How to work effectively:

- Maintain a Lua-defined startup topology.
- Use `wezterm cli send-text` and split commands for reproducible environments.

## Ghostty: Why Execute/Save-Restore Are Qualified

What works well:

- Native splits, split navigation, and split resizing via actions.
- `window-save-state` for restoring tabs/splits/window state.
- `ghostty +action ...` offers command-driven behavior.

Why `⚠️`:

- The strongest control surface is action/config oriented.
- Compared to Kitty/WezTerm, pane-targeted command execution and rich external runtime orchestration are less explicit.

How to work effectively:

- Use `window-save-state = always` for persistent geometry/state.
- Use actions for structural manipulation (split, focus, resize).
- For heavy pane-targeted automation, pair with tmux/zellij if needed.

## iTerm2: Why Programmatic Save/Restore Is Qualified

What works well:

- Excellent Python API and AppleScript.
- Strong split/focus/execute automation.
- Window Arrangements provide practical restore points.

Why `⚠️`:

- Arrangement workflows are robust for layout/session setup, but still not a guaranteed universal capture of all runtime process state in every pane.

How to work effectively:

- Treat arrangements as reproducible workspace templates.
- Use Python/AppleScript to layer command bootstrapping on top.

## Apple Terminal: Why Several Items Are Qualified

What works:

- Multiple windows/tabs.
- Window Groups restore a window/tab set.
- AppleScript for tabs/windows automation.

Why `⚠️`:

- Split behavior is not equivalent to independent multiplexer panes.
- Pane-focused multiplex controls and pane-targeted command automation are limited.
- Save/restore is better understood as window/tab restoration.

How to work effectively:

- Use Window Groups for repeatable tab sets.
- Use tmux/zellij inside Terminal when true multiplexing is required.

## Warp: Why Programmatic Resize/Focus Are Qualified

What works:

- Native panes interactively.
- Launch Configs (YAML) for startup layout + command bootstrapping.

Why `⚠️`:

- Launch Configs are strong for startup definition, not for rich external runtime pane control.
- Programmatic resize/focus on arbitrary existing panes is limited compared to explicit IPC/remote-control models.

How to work effectively:

- Put stable workspace topology in Launch Configs.
- Use interactive controls at runtime, or external multiplexer tools for deeper automation.

## Konsole: Practical "Partial but Useful" Pattern

A pragmatic automation pattern that avoids fragile assumptions:

1. Build the split/tab layout once in Konsole UI.
2. Save layout to JSON (`View -> Save tab layout to file`).
3. Launch with `konsole --layout ...`.
4. Use DBus (`qdbus`) to run per-session commands after startup.

This gives repeatability, but still does not fully match terminals with first-class pane-level IPC APIs and independent-pane semantics.

## Sources

- Companion baseline report: `/Volumes/coding/personal/rusty-biscuit/biscuit-terminal/docs/multiplexing.md`
- DBus specification: <https://dbus.freedesktop.org/doc/dbus-specification.html>
- Qt DBus overview: <https://doc.qt.io/qt-6/qtdbus-overview.html>
- Qt DBus viewer: <https://doc.qt.io/qt-6/qdbusviewer.html>
- Konsole scripting chapter: <https://docs.kde.org/stable5/en/konsole/konsole/scripting.html>
- Konsole command reference (split view behavior): <https://docs.kde.org/stable5/en/konsole/konsole/command-reference.html>
- Konsole command-line options (`--layout`): <https://docs.kde.org/stable5/en/konsole/konsole/command-line-options.html>
- Ghostty features/docs: <https://ghostty.org/docs/features>
- Ghostty config reference (`window-save-state`): <https://ghostty.org/docs/config/reference#window-save-state>
- Ghostty man page (`+action` and action list): <https://man.archlinux.org/man/ghostty.1>
- WezTerm CLI/docs: <https://wezterm.org/cli/cli/split-pane.html>
- WezTerm send-text: <https://wezterm.org/cli/cli/send-text.html>
- iTerm2 Python API: <https://iterm2.com/documentation-python-api.html>
- Apple Terminal window groups: <https://support.apple.com/guide/terminal/use-window-groups-trml1003/mac>
- Warp launch configs: <https://docs.warp.dev/features/launch-configurations>
- Kitty remote control: <https://sw.kovidgoyal.net/kitty/remote-control/>
