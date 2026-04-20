# CLI Rationalization Plan

**Spec**: [spec.md](./spec.md)
**Target**: `claudine/cli/src/`

---

## Phase 1: Remove `dry-run` and `about` subcommands

### 1a. Remove `dry-run`

- **`claudine/cli/src/commands/dry_run.rs`** — Delete the entire file
- **`claudine/cli/src/commands/mod.rs`** — Remove `pub mod dry_run;`
- **`claudine/cli/src/args.rs`** — Remove the `DryRun` variant from `Commands`
- **`claudine/cli/src/main.rs`** — Remove the `Some(Commands::DryRun(args))` match arm

### 1b. Remove `about`

- **`claudine/cli/src/commands/about.rs`** — Delete the entire file
- **`claudine/cli/src/commands/mod.rs`** — Remove `pub mod about;`
- **`claudine/cli/src/args.rs`** — Remove the `About` variant from `Commands`
- **`claudine/cli/src/main.rs`** — Remove the `Some(Commands::About)` match arm

### 1c. Clean up stale references

- Grep for `dry.run` and `about` across `claudine/cli/` to catch any remaining references (imports, comments, documentation strings)
- Update the `ABOUT_TEXT` content in the skill docs or any user-facing docs that reference `claudine dry-run` or `claudine about`

---

## Phase 2: Hide `help` and `handle` from the help menu

Both commands must continue to work but should not appear in the help system's command listing.

- **`claudine/cli/src/args.rs`** — Add `#[command(hide = true)]` attribute to:
  - `Handle` variant (the `handle` subcommand)
- For `help`: clap auto-generates a `help` subcommand. Use `#[command(disable_help_subcommand = true)]` on the top-level `Cli` struct. We will render help ourselves (Phase 4), so the auto-generated subcommand is no longer needed. The `--help` flag will still work via clap.

---

## Phase 3: Promote `compose-inline` to a top-level command

Currently `compose inline <FILE>` is a subcommand of `compose`. It needs to become `compose-inline <FILE>`.

### 3a. Add `ComposeInline` variant to `Commands`

- **`claudine/cli/src/args.rs`** — Add a new variant:
  ```rust
  /// Inline composition: use frontmatter prompt, replace body with output.
  #[command(name = "compose-inline")]
  ComposeInline(commands::compose::ComposeInlineArgs),
  ```

### 3b. Create `ComposeInlineArgs` struct

- **`claudine/cli/src/commands/compose.rs`** — Add a new top-level args struct:
  ```rust
  #[derive(Debug, Clone, Args)]
  pub struct ComposeInlineArgs {
      #[arg(short = 'i', long)]
      pub interactive: bool,
      #[arg(long = "exclude", value_name = "PROVIDER")]
      pub exclude: Vec<String>,
      #[arg(short = 't', long = "timeout", value_name = "SECONDS")]
      pub timeout: Option<u64>,
      #[arg(long)]
      pub silent: bool,
      /// File reference to compose.
      #[arg(value_name = "FILE")]
      pub file: String,
  }
  ```

### 3c. Add entry point and route

- **`claudine/cli/src/commands/compose.rs`** — Add `pub fn run_compose_inline(args: ComposeInlineArgs, verbose: u8) -> Result<()>` that constructs a `ComposeArgs` with `subcommand: Some(ComposeSubcommand::Inline(...))` and delegates to `run_compose`
- **`claudine/cli/src/main.rs`** — Add match arm:
  ```rust
  Some(Commands::ComposeInline(args)) => commands::compose::run_compose_inline(args, cli.verbose),
  ```

### 3d. Keep backward compatibility

- The existing `compose inline` subcommand should remain functional for now (don't remove `ComposeSubcommand`). It can be deprecated or hidden in a future pass.

---

## Phase 4: Custom grouped help system using Renderable components

This is the main visual change. Replace clap's default `print_help()` in the `None` arm with a custom renderer using `biscuit-terminal` components.

### 4a. Create `claudine/cli/src/commands/help.rs`

New module responsible for rendering the grouped help display. Will use:

- **`Prose`** — For the header, description, and styled text
- **`Section`** — For group headings (h2 level)
- **`Compose`** — To combine all parts into a single renderable
- **`TwoColumn`** or manual alignment — For command name + description pairs

### 4b. Define command groups

```rust
struct CommandGroup {
    name: &'static str,
    commands: Vec<CommandEntry>,
}

struct CommandEntry {
    name: &'static str,
    description: &'static str,
    future: bool, // renders as dimmed with "(future)" tag
}
```

Groups per spec:

1. **Shared Resources**
   - `skills` — List available skills and their scopes
   - `commands` — List available slash commands and their scopes
   - `agents` — List available agent definitions and their scopes
   - `mcp` — Manage MCP (Model Context Protocol) servers
   - `hooks` — Show registered hooks for all detected agents

2. **Wrapped Execution**
   - `claude` — Wrap Claude Code with Claudine preflight/env handling
   - `codex` — Wrap Codex CLI with Claudine preflight/env handling
   - `gemini` — Wrap Gemini CLI with Claudine preflight/env handling
   - `goose` — Wrap Goose with Claudine preflight/env handling
   - `kimi` — Wrap Kimi Code with Claudine preflight/env handling
   - `opencode` — Wrap OpenCode with Claudine preflight/env handling
   - `qwen` — Wrap Qwen Code with Claudine preflight/env handling

3. **Composition**
   - `compose` — Compose a Markdown document through an agentic CLI
   - `compose-inline` — Inline composition: use frontmatter prompt, replace body with output
   - `sequence` — Sequence multiple compositions *(future)*

4. **Administration**
   - `init` — Interactive setup wizard
   - `sync` — Re-sync hook registrations with detected agents
   - `uninstall` — Remove Claudine hooks from all agents

### 4c. Additional commands (not in spec groups but still needed)

These commands exist but are not listed in the spec's groups. They should appear in a catch-all group or be hidden:

- `actions` — Show which actions are configured and for which events
- `providers` — Show provider capability matrix
- `logs` — Query and sync Claudine JSONL logs
- `completions` — Generate shell completions
- `link` — Link skills and commands across providers

**Recommendation**: Add two more groups to accommodate these:
- Add `link` to **Shared Resources** (it manages cross-provider resource links)
- Add `actions` and `providers` to **Administration** (or a new **Inspection** group)
- Add `logs` to **Administration**
- Add `completions` to **Administration**

> **Decision needed from spec author**: Where do `actions`, `providers`, `logs`, `link`, and `completions` go? The plan proceeds with the grouping above pending confirmation.

### 4d. Rendering approach

```
Claudine — cross-agent hook/event system for agentic CLIs

Usage: claudine [OPTIONS] <COMMAND>

Shared Resources:
  skills           List available skills and their scopes
  commands         List available slash commands and their scopes
  agents           List available agent definitions and their scopes
  mcp              Manage MCP servers
  hooks            Show registered hooks for all detected agents
  link             Link skills and commands across providers

Wrapped Execution:
  claude           Wrap Claude Code with Claudine preflight/env handling
  codex            Wrap Codex CLI with Claudine preflight/env handling
  ...

Composition:
  compose          Compose a Markdown document through an agentic CLI
  compose-inline   Inline composition: frontmatter prompt, replace body
  sequence         Sequence multiple compositions (future)

Administration:
  init             Interactive setup wizard
  sync             Re-sync hook registrations with detected agents
  uninstall        Remove Claudine hooks from all agents
  actions          Show configured actions and events
  providers        Show provider capability matrix
  logs             Query and sync Claudine JSONL logs
  completions      Generate shell completions

Options:
  -v, --verbose    Increase verbosity (-v for verbose, -vv for debug)
      --plain      Strip ANSI escape codes from all output
  -h, --help       Print help
  -V, --version    Print version
```

Implementation:
- Use `Prose` for the title/description line with styling
- Use `Section::new(HeadingLevel::h3, "Group Name")` for each group heading
- Use `TwoColumn` or fixed-width `Prose` with padding for the command + description columns
- Use `Compose` to assemble all parts
- Dim styling for `(future)` entries
- Render with `crate::log::terminal()` to respect `--plain`

### 4e. Wire up the custom help

- **`claudine/cli/src/main.rs`** — In the `None` arm, replace `Cli::command().print_help()?` with `commands::help::run()`
- **`claudine/cli/src/commands/mod.rs`** — Add `pub mod help;`
- **`claudine/cli/src/args.rs`** — Add `#[command(disable_help_subcommand = true)]` to `Cli`

### 4f. The `sequence` future command

`sequence` appears in the Composition group display but is **not** an actual `Commands` variant. It is rendered as a dimmed entry with "(future)" suffix by the custom help renderer only. No args struct, no handler, no match arm needed.

---

## Phase 5: Verification and cleanup

### 5a. Build and test

- `just build -p claudine-cli` — Verify clean compilation
- `just test -p claudine-cli` — Run existing tests; remove/update any tests that reference `dry-run` or `about`
- `just lint -p claudine-cli` — Clean lint pass

### 5b. Manual verification

- `claudine` (no args) — Verify grouped help renders correctly
- `claudine --help` — Verify clap's `--help` flag still works (may want to override this too to use the same custom renderer)
- `claudine compose-inline --help` — Verify the new top-level command works
- `claudine compose inline <file>` — Verify backward compat still works
- `claudine handle <event>` — Verify hidden command still functions
- `claudine help` — Verify it's no longer listed but consider whether it should still work as a hidden alias

### 5c. Documentation updates

- Update the claudine skill file's CLI command table to reflect removals and additions
- Remove references to `dry-run` and `about` from any user-facing docs
- Add `compose-inline` and `sequence (future)` to the command table

---

## Execution Order

| Step | Phase | Depends On | Description |
|------|-------|------------|-------------|
| 1 | 1a | — | Remove `dry-run` command |
| 2 | 1b | — | Remove `about` command |
| 3 | 1c | 1, 2 | Clean stale references |
| 4 | 2 | — | Hide `help` and `handle` from menu |
| 5 | 3 | — | Promote `compose-inline` to top-level |
| 6 | 4 | 1–5 | Build custom grouped help renderer |
| 7 | 5 | 6 | Build, test, lint, verify |

Steps 1, 2, 4, and 5 are independent and can execute in parallel. Step 6 depends on all prior steps being complete since it defines the final command set. Step 7 is final verification.
