# Native Multiplexing in Terminal Apps

Last updated: 2026-02-14

This document consolidates and resolves the findings in `/Volumes/coding/personal/rusty-biscuit/biscuit-terminal/docs/multiplexing/a.md` and `/Volumes/coding/personal/rusty-biscuit/biscuit-terminal/docs/multiplexing/b.md`, then validates conclusions against primary documentation.

## Scope and Definitions

To keep comparisons consistent, this report uses a strict definition:

- **Split panes** means independent shells in one window (not just mirrored/split views of the same session).
- **Programmatic** means controllable by API/CLI/IPC/config-as-code, not only manual keybindings.
- **Save layouts** means restoring pane/tab/window structure (and ideally commands) without rebuilding manually.

## Summary Table 1: Feature Support (Any Native Interface)

| Terminal | Split Panes | Resize Panes | Focus Panes | Execute in Pane | Save Layouts |
| --- | --- | --- | --- | --- | --- |
| **WezTerm** | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| **Ghostty** | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| **iTerm2** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Apple Terminal** | ⚠️ (view split) | ⚠️ (view split only) | ⚠️ (view focus only) | ❌ | ⚠️ |
| **Warp** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Kitty** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Alacritty** | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Konsole** | ⚠️ (view split) | ⚠️ | ⚠️ | ❌ | ✅ |

Legend: ✅ supported, ⚠️ partial/qualified, ❌ not supported.

## Summary Table 2: Programmatic Access Only

| Terminal | Programmatic Mechanism | Split | Resize | Focus | Execute | Save/Restore |
| --- | --- | --- | --- | --- | --- | --- |
| **WezTerm** | Lua config/events + `wezterm cli` | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| **Ghostty** | `ghostty +action ...` + config | ✅ | ✅ | ✅ | ⚠️ | ⚠️ |
| **iTerm2** | Python API + AppleScript | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| **Apple Terminal** | AppleScript (tabs/windows) | ❌ | ❌ | ❌ | ❌ | ⚠️ |
| **Warp** | Launch Configs (YAML) | ✅ | ⚠️ | ⚠️ | ✅ | ✅ |
| **Kitty** | Remote control (`kitten @`) + sessions | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Alacritty** | None | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Konsole** | DBus/QDBus + layout files | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ |

## Terminal-by-Terminal Conclusions

### WezTerm

- Strong native multiplexer with panes/tabs and CLI control.
- Clear programmatic support for split/focus/resize/execute.
- Layout persistence is possible via scripted startup/workspace patterns, but first-class “snapshot/restore layout” UX is less explicit than Kitty/iTerm2.

### Ghostty

- Native split/focus/resize is solid.
- `+action` provides command-driven control (so it is more scriptable than one report suggested).
- `window-save-state` provides persistence, but command execution targeting an arbitrary existing split is less explicit than in Kitty/WezTerm.

### iTerm2

- Mature pane model plus robust scripting (Python/AppleScript).
- Strong for automation and pane command execution.
- Window arrangements provide practical save/restore.

### Apple Terminal

- Has split UI, but this is not a full independent-pane multiplexer model.
- Not a strong native multiplexer choice compared to WezTerm/Kitty/iTerm2.
- Window groups help restore windows/tabs, but pane-level multiplex control is limited.

### Warp

- Strong native pane UX.
- Launch Configs make static layout+command bootstrapping good.
- Dynamic runtime automation is weaker than WezTerm/Kitty (more config-driven than IPC/API-driven).

### Kitty

- One of the strongest native multiplexing implementations.
- `kitten @` remote control exposes deep pane automation.
- Session files provide explicit restore semantics.

### Alacritty

- Explicitly out of scope for native tabs/splits; expects external multiplexer (`tmux`, `zellij`).

### Konsole

- Supports split **views** and can save/load tab layouts.
- Docs describe duplicated output behavior in split view, so it does not map cleanly to strict independent-pane multiplexing.
- DBus/QDBus helps automation, but semantics differ from Kitty/WezTerm style pane multiplexing.

## Final Verdict

If the goal is a **native terminal multiplexer with strong automation**, the top tier is:

1. **Kitty** (best explicit remote-control + sessions story)
2. **WezTerm** (excellent programmable model with Lua + CLI)
3. **iTerm2** (excellent on macOS via Python/AppleScript)

For **interactive native panes with lighter automation**:

1. **Ghostty**
2. **Warp**

For **non-multiplexer or view-split-only workflows**:

1. **Apple Terminal**
2. **Konsole** (qualified: split-view/layout tooling exists, but pane semantics differ)
3. **Alacritty** (no native multiplexing by design)

## Primary Sources

- WezTerm panes: <https://wezterm.org/recipes/panes.html>
- WezTerm CLI split pane: <https://wezterm.org/cli/cli/split-pane.html>
- WezTerm CLI send text: <https://wezterm.org/cli/cli/send-text.html>
- Ghostty features: <https://ghostty.org/docs/features>
- Ghostty keybind actions: <https://ghostty.org/docs/config/keybind/reference>
- Ghostty `window-save-state`: <https://ghostty.org/docs/config/reference#window-save-state>
- Ghostty `+action` command reference: <https://man.archlinux.org/man/ghostty.1>
- iTerm2 docs (Python API + panes): <https://iterm2.com/documentation-python-api.html>
- iTerm2 docs (general): <https://iterm2.com/documentation-one-page.html>
- Apple Terminal keyboard shortcuts: <https://support.apple.com/guide/terminal/keyboard-shortcuts-trmlshtcts/mac>
- Apple Terminal window groups: <https://support.apple.com/guide/terminal/use-window-groups-trml1003/mac>
- Warp panes: <https://docs.warp.dev/terminal/panes>
- Warp launch configs: <https://docs.warp.dev/features/launch-configurations>
- Kitty remote control: <https://sw.kovidgoyal.net/kitty/remote-control/>
- Kitty sessions: <https://sw.kovidgoyal.net/kitty/sessions/>
- Alacritty FAQ (tabs/splits out of scope): <https://github.com/alacritty/alacritty#faq>
- Konsole command reference (split/load/save layout): <https://docs.kde.org/stable5/en/konsole/konsole/command-reference.html>
- Konsole scripting (DBus): <https://docs.kde.org/stable5/en/konsole/konsole/scripting.html>
