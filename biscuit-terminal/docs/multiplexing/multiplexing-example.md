# Comparing How Various Terminal Apps Would Programmatically Multiplex

This document defines one concrete multiplexing operation and shows how a Rust program would perform it per terminal.

## Target Operation

Given the currently active pane/session, we want to:

1. Split it into top/bottom panes (new pane on bottom).
2. Keep focus on the original top pane.
3. Start `top` in the new bottom pane.

Desired end state:

- Top pane: original shell, still focused.
- Bottom pane: running `top`.

## Terminal Set

- WezTerm
- Ghostty
- iTerm2
- ~~Apple Terminal~~
- Warp
- Kitty
- ~~Alacritty~~
- Konsole

`Apple Terminal` and `Alacritty` are excluded from this implementation document because they do not provide the required native independent-pane multiplexing model.

## Rust Execution Model

For all terminals, we use Rust to call the terminal's supported control surface (CLI/API/script bridge).

```rust
use std::io;
use std::process::{Command, Stdio};

fn run(cmd: &str, args: &[&str]) -> io::Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("command failed: {} {:?}", cmd, args),
        ))
    }
}
```

## WezTerm (Exact Fit)

Control surface: `wezterm cli`

Why it fits:

- Can split active pane and spawn a command in the new pane.
- Can explicitly re-activate original pane by id.

```rust
use std::env;
use std::io;
use std::process::Command;

fn wezterm_split_top_keep_focus() -> io::Result<()> {
    // Original pane id is exposed when running inside WezTerm.
    let original = env::var("WEZTERM_PANE").ok();

    // Split active pane; new bottom pane runs top.
    let status = Command::new("wezterm")
        .args(["cli", "split-pane", "--bottom", "--", "top"])
        .status()?;
    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "wezterm split-pane failed"));
    }

    // Re-focus original top pane.
    if let Some(pane_id) = original {
        let status = Command::new("wezterm")
            .args(["cli", "activate-pane", "--pane-id", &pane_id])
            .status()?;
        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "wezterm activate-pane failed"));
        }
    }

    Ok(())
}
```

Notes:

- Best reliability is running this from inside WezTerm so `WEZTERM_PANE` is available.

## Kitty (Exact Fit)

Control surface: `kitten @` remote control

Why it fits:

- Can launch into split location.
- Can keep current focus (`--keep-focus`).
- Can start command directly in new pane.

```rust
use std::io;
use std::process::Command;

fn kitty_split_top_keep_focus() -> io::Result<()> {
    // hsplit => top/bottom split in kitty's splits layout terminology.
    let status = Command::new("kitten")
        .args([
            "@",
            "launch",
            "--location=hsplit",
            "--keep-focus",
            "top",
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "kitty launch failed"))
    }
}
```

Prereq:

- Kitty remote control must be enabled (`allow_remote_control` or equivalent configuration).

## iTerm2 (Exact Fit)

Control surface: AppleScript (invoked from Rust via `osascript`)

Why it fits:

- Can split current session and run `top` in new split.
- Can explicitly re-select original session to keep focus on top pane.

```rust
use std::io;
use std::process::Command;

fn iterm2_split_top_keep_focus() -> io::Result<()> {
    let script = r#"
        tell application "iTerm2"
            tell current window
                tell current session of current tab
                    set originalSession to it
                    set newSession to (split vertically with same profile command "top")
                    select originalSession
                end tell
            end tell
        end tell
    "#;

    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "iTerm2 AppleScript failed"))
    }
}
```

Notes:

- This is macOS-only.
- iTerm2 Python API is an alternative to AppleScript if you prefer async Python control.

## Warp (Exact Fit via Launch Configuration)

Control surface: Launch Config YAML + Warp URI scheme

Why it fits:

- Launch Config defines split structure, focused pane, and startup commands.
- URI scheme can open the launch config programmatically.

### 1) Generate launch config YAML from Rust

```yaml
---
name: split-top-focused-with-top-below
windows:
  - tabs:
      - title: Multiplex Example
        layout:
          split_direction: vertical
          panes:
            - cwd: /ABSOLUTE/PATH
              is_focused: true
            - cwd: /ABSOLUTE/PATH
              commands:
                - exec: top
```

Write to:

- `$HOME/.warp/launch_configurations/split-top-focused-with-top-below.yaml`

### 2) Open it from Rust using Warp URI

```rust
use std::io;
use std::process::Command;

fn warp_open_launch_config() -> io::Result<()> {
    // Path style depends on Warp URI support in your version.
    // Example uses a file path component accepted by Warp's URI scheme docs.
    let uri = "warp://launch/split-top-focused-with-top-below.yaml";

    // macOS; on Linux use xdg-open.
    let status = Command::new("open").arg(uri).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "warp URI open failed"))
    }
}
```

Notes:

- Warp is strongest for startup-time declarative orchestration; runtime pane IPC is limited compared to WezTerm/Kitty.

## Ghostty (Best Effort, Not Exact Yet)

Control surface: `ghostty +action ...`

What works reliably:

- Create split (`new_split:down`)
- Move focus (`goto_split:up` / `goto_split:down`)
- Resize/equalize splits

Limitation for this exact scenario:

- A robust, documented pane-targeted "run this command in that specific existing split" flow is less explicit than WezTerm/Kitty.

Best-effort Rust flow:

```rust
use std::io;

fn ghostty_best_effort() -> io::Result<()> {
    run("ghostty", &["+action=new_split:down"])?;
    run("ghostty", &["+action=goto_split:up"])?;
    Ok(())
}
```

Interpretation:

- Structural part is scriptable.
- Deterministic command placement in the lower split remains qualified.

## Konsole (Best Effort, Qualified Semantics)

Control surface: `qdbus` + `konsole --layout`

What works:

- DBus scripting for sessions/windows.
- Layout save/load (`--layout`).

Why qualified for this exact scenario:

- Konsole split-view behavior is documented as duplicated output views, which differs from strict independent-pane multiplex semantics used in this comparison.

Best-effort approach from Rust:

1. Launch saved layout JSON.
2. Use DBus session methods (discovered via introspection) to run commands.

```rust
use std::io;

fn konsole_best_effort(layout_path: &str) -> io::Result<()> {
    run("konsole", &["--layout", layout_path])?;

    // Session ids and callable methods vary; introspect first:
    // qdbus org.kde.konsole /Sessions/1
    run(
        "qdbus",
        &[
            "org.kde.konsole",
            "/Sessions/1",
            "org.kde.konsole.Session.runCommand",
            "top",
        ],
    )?;

    Ok(())
}
```

## Recommendation for the Rust Implementation

If the project wants one code path per terminal that is deterministic and production-grade for this exact operation:

1. **Primary backends**: WezTerm, Kitty, iTerm2, Warp.
2. **Qualified backends**: Ghostty, Konsole (implement only as best-effort with capability warnings).
3. **Excluded**: Apple Terminal, Alacritty.

## Suggested Backend Trait

```rust
use std::io;

pub trait MultiplexBackend {
    fn name(&self) -> &'static str;
    fn split_keep_focus_run_top(&self) -> io::Result<()>;
    fn is_exact(&self) -> bool;
}
```

This keeps calling code simple while preserving per-terminal behavior differences.

## Sources

- WezTerm CLI split-pane: <https://wezterm.org/cli/cli/split-pane.html>
- WezTerm CLI activate-pane: <https://wezterm.org/cli/cli/activate-pane.html>
- Kitty remote control: <https://sw.kovidgoyal.net/kitty/remote-control/>
- Kitty launch options: <https://sw.kovidgoyal.net/kitty/launch/>
- iTerm2 Python API docs: <https://iterm2.com/documentation-python-api.html>
- iTerm2 scripting reference (AppleScript split commands): <https://iterm2.com/3.0/documentation-one-page.html>
- Warp split panes: <https://docs.warp.dev/terminal/windows/split-panes>
- Warp launch configurations: <https://docs.warp.dev/terminal/sessions/launch-configurations>
- Warp URI scheme: <https://docs.warp.dev/terminal/more-features/uri-scheme>
- Ghostty keybind action reference: <https://ghostty.org/docs/config/keybind/reference>
- Ghostty config reference (`ghostty +list-actions`, keybind docs): <https://ghostty.org/docs/config/reference>
- Konsole scripting (DBus): <https://docs.kde.org/stable5/en/konsole/konsole/scripting.html>
- Konsole split view command reference: <https://docs.kde.org/stable5/en/konsole/konsole/command-reference.html>
- Konsole command-line options (`--layout`): <https://docs.kde.org/stable5/en/konsole/konsole/command-line-options.html>
