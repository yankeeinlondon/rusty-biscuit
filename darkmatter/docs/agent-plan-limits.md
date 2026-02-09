# Agentic Plan Limits

This plan will build out a struct called `AgentPlanStatus` which will check for the available amount left in a Agentic plans caps limits.

## Design

Many of the shapes and types have been started in the @ai-unchained/lib/src/primitives/services/AgentStatus.rs file. Review this first to get a context.

The mechanism we'll use for checking these caps will for now need to be exclusively through the CLI. Here's a rough draft example of how this might be done for Codex:

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::time::Duration;

pub fn codex_status_text() -> anyhow::Result<String> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new("codex");
    // You can add args here if you want (e.g. "--no-alt-screen"), depending on codex support.
    let mut child = pair.slave.spawn_command(cmd)?;

    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

    // Give TUI a moment to initialize
    std::thread::sleep(Duration::from_millis(600));

    writer.write_all(b"/status\n")?;
    writer.flush()?;

    std::thread::sleep(Duration::from_millis(600));

    // Read whatever is available (you'll likely want a loop + timeout)
    let mut buf = [0u8; 64 * 1024];
    let n = reader.read(&mut buf).unwrap_or(0);
    let text = String::from_utf8_lossy(&buf[..n]).to_string();

    // Stop codex (Ctrl-C)
    let _ = writer.write_all(&[0x03]);
    let _ = child.wait();

    Ok(text)
}
```

A similar approach should be possible with Claude Code.


## Starting up the CLI

We have a "unchained-ai/cli" directory but up until now we've not done anything with it. During this plan we should:

- setup the CLI using the `clap` and `clap_complete` crates
- make sure the `justfile` is updated to build, test, etc. the CLI as well as the library
- add a `limits` subcommand which leverages the `AgentPlanStatus` struct to report on limits
    - by default it should report to the screen in a terminal friendly manner.
    - it should consider using the `Prose`, `Table`, `BlockQuote` or other terminal components from `biscuit-terminal` to help render results.
    - Note: it would be nice to have a progress bar visual like both Codex and Claude Code provide in their `/status` commands. We should create this in a reusable manner by adding `Progress` struct to `biscuit-terminal` as a renderable component.
    - Note: the `AgenticStatusPlatform` should leverage sniff library to detect which of the agentic platforms are installed on the host computer (use sniff skill) for the `Default` implementation.
