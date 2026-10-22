# Detection Functions

Low-level detection functions for terminal capabilities. These are used internally by `Terminal::new()` but can be called directly for specific checks.

## Terminal Application

```rust
use biscuit_terminal::discovery::detection::get_terminal_app;

let app = get_terminal_app();
match app {
    TerminalApp::Wezterm => println!("Running in WezTerm"),
    TerminalApp::Kitty => println!("Running in Kitty"),
    TerminalApp::ITerm2 => println!("Running in iTerm2"),
    TerminalApp::Ghostty => println!("Running in Ghostty"),
    TerminalApp::Other(name) => println!("Running in: {}", name),
    _ => {}
}
```

Detection order:
1. `TERM_PROGRAM` environment variable
2. Terminal-specific env vars (`KITTY_WINDOW_ID`, `WEZTERM_PANE`, etc.)
3. `TERM` variable pattern matching

## Color Detection

### Color Depth

```rust
use biscuit_terminal::discovery::detection::color_depth;

match color_depth() {
    ColorDepth::TrueColor => println!("24-bit color (16M)"),
    ColorDepth::Enhanced => println!("256 colors"),
    ColorDepth::Basic => println!("16 colors"),
    ColorDepth::Minimal => println!("8 colors"),
    ColorDepth::None => println!("No color"),
}
```

Detection: `COLORTERM` env var → terminfo `MaxColors` → None

### Color Mode (Light/Dark)

```rust
use biscuit_terminal::discovery::detection::color_mode;

match color_mode() {
    ColorMode::Light => println!("Light background"),
    ColorMode::Dark => println!("Dark background"),
    ColorMode::Unknown => println!("Unknown"),
}
```

Detection: OSC11 query → `DARK_MODE` env → macOS `AppleInterfaceStyle` → Dark default

The "OSC11 query" step is really `bg_color()`: a live query only for apps in
`osc_queries::support::answers_color_queries`, then `COLORFGBG`, then a
per-app default. **Trap:** a per-app default decides the mode outright, so a
wrong one beats the system appearance. Apple Terminal used to default to a
white background, which made every dark Terminal.app profile read as Light
(a near-white row highlight with white text, 2026-10-04). It now has no
default and is never queried, so it falls through to `AppleInterfaceStyle`.

Provenance: `bg_color_with_source()` / `text_color_with_source()` /
`cursor_color_with_source()` return `(RgbValue, ColorSource)`; only a live
reply is `Measured`, `COLORFGBG` and per-app defaults are `Estimated`.
`Terminal::background_source` carries it for `background_color`
(`ProbeMode::Off` is always `Estimated`). Under tmux nothing is queried, so
the background is WezTerm's per-app `#1e1e1e` guess, `Estimated`.

## Feature Support

### Image Support

```rust
use biscuit_terminal::discovery::detection::{graphics_route, GraphicsFormat};

let route = graphics_route(); // or Terminal::new().graphics_route
match route.format {
    GraphicsFormat::Kitty | GraphicsFormat::Iterm2 | GraphicsFormat::Sixel => {}
    GraphicsFormat::None => println!("no graphics: {}", route.reason),
}
```

Resolution is environment first (`BISCUIT_GRAPHICS`,
`BISCUIT_GRAPHICS_TRANSPORT`, `BISCUIT_TERMINAL_PROBE`, Windows SSH, Zellij,
tmux, known terminals); `probe_plan(&inputs)` says when the environment cannot
decide, and then the graphics probe does. Developer guide:
`biscuit-terminal/docs/discovery/graphics-routes.md`. `image_support()` is a
projection kept for existing callers. The Windows SSH server's iTerm2 route
(detected or forced) sets `needs_cursor_resync`; no other route does.

The graphics probe (`detection::probe`, live exchange in
`discovery::startup_exchange`):

- Rides on the once-per-process OSC 10/11 exchange: a Kitty query
  (`ESC _ G i=<fresh id>,s=1,v=1,a=q,t=d,f=24;AAAA ESC \`) in the same write,
  Sixel read from the DA1 sentinel's reply. Whichever of color lookup / route
  resolution needs it first sends both parts. Inside a multiplexer only the
  graphics part is sent (colors stay off).
- Outcomes: correlated Kitty `OK` → Kitty `Verified` (beats Sixel); DA1
  attribute `4` (never the class, never a digit of another number) → Sixel
  `Verified`; DA1 without either or a Kitty rejection → NoSupport `Verified`;
  silence, conflicting replies, busy input, unusable tty → `Unknown`, never a
  confident negative.
- Runs for: unidentified terminal, SSH without forwarded hints, Zellij
  (always; pixel routes set `needs_measured_cells`), Windows Terminal with
  version unreadable or < 1.22, likely hidden tmux (`TERM=tmux-*`/`screen-*`
  without `TMUX`/`STY`: DA1-only direct plus a tmux-wrapped Kitty exchange; a
  plain `OK` never makes a route, silence → fix naming both
  `BISCUIT_GRAPHICS_TRANSPORT=tmux` and `=direct`), GNU screen (`STY`,
  unidentified terminal: DA1 only), declared tmux with
  unidentified outer terminal (wrapped only), visible tmux whose
  `allow-passthrough` is unreadable (wrapped only). Never for: known terminals,
  non-tty, CI, forced/none/invalid overrides, probing off, Windows SSH server,
  legacy Windows console (no `WT_SESSION`/`TERM_PROGRAM`/`MSYSTEM`, not SSH →
  NoSupport), visible tmux whose passthrough was read and that cannot draw
  Sixel itself.
- Visible tmux (`TMUX` set): `RouteInputs::tmux` (`TmuxFacts { version,
  passthrough, clients }`, `detection/tmux.rs`) is read in `from_process` via
  `tmux -V`, `show-options -Apv -t $TMUX_PANE allow-passthrough`,
  `list-clients -t $TMUX_PANE -F` (termname, termtype, `client_utf8`,
  `client_termfeatures`; tab-separated)
  (500 ms deadline each, no shell; unreadable = unknown, never off). < 3.3 →
  BlockedTransport + upgrade fix; `off` → BlockedTransport + fix
  `set -g allow-passthrough on`; `on` → the clients' common format (Kitty
  preferred; unrecognized client = Kitty-only; none recognized → Kitty with
  confidence `Unknown`). Placement (`route.rs::with_placement`, also for a
  forced Kitty): `Placeholders` only when every client is kitty/Ghostty AND
  each has `utf8` and the `RGB` feature (tmux would round the 24-bit ID color
  otherwise); kitty/Ghostty missing either → ordinary + "placed ordinarily"
  in the reason; WezTerm/unidentified/hidden or declared tmux → ordinary. Rows:
  `lib/tests/l1/graphics_tmux_matrix.rs`; live: `tests/level2/level2_graphics_tmux.rs`.
- tmux-drawn Sixel (`route.rs::resolve_visible_tmux`) comes before every
  passthrough route, even when clients could take Kitty: tmux ≥ 3.4
  (`TMUX_NATIVE_SIXEL_MIN`), every client of the pane's session has `sixel`
  in `client_termfeatures` (empty/unreadable list → no), and tmux's own
  unwrapped DA1 reports `4` → Sixel, `Direct`, `Verified`. Independent of
  `allow-passthrough` (off blocks tunneled graphics only). Probing off/CI →
  DA1 not asked → passthrough rules. Forced `sixel` with version + clients
  qualifying → Sixel `Direct` `Forced`, no DA1; otherwise passthrough.
  Forced kitty/iterm2 unchanged. So WezTerm under a Sixel-built tmux (3.8)
  gets tmux's Sixel, not Kitty passthrough, and passthrough on/off is moot.
  tmux clears its own Sixel on pane resize (documented limit).
- `ProbePlan::direct_da1_only`: set for that candidate (`direct` +
  `direct_da1_only`, plus `tunneled` if passthrough unreadable); the direct
  exchange sends DA1 alone (colors only if wanted), result `kitty: NoReply`.
  **Trap:** never send an unwrapped APC/Kitty query to a visible tmux: tmux
  treats `ESC _ … ESC \` as a pane-title change and the pane is retitled
  `Gi=…` (seen live on tmux 3.8). `route.rs::without_apc_under_multiplexer`
  forces `direct_da1_only` whenever `TMUX` or `STY` is set (even with
  `BISCUIT_GRAPHICS_TRANSPORT=direct`); the hidden-tmux plan sets it too.
- Busy input: graphics query not sent, route `Unknown`, skip not cached.
- Tests set probe answers through `RouteInputs::probe` (`GraphicsProbe`,
  `ProbeResult`, `ProbeReply`, `KittyReply`, `Da1Reply`); see
  `lib/tests/l1/graphics_probe_matrix.rs`.

### OSC8 Hyperlinks

```rust
use biscuit_terminal::discovery::detection::osc8_link_support;

if osc8_link_support() {
    println!("\x1b]8;;https://example.com\x07Link\x1b]8;;\x07");
}
```

### Italics

```rust
use biscuit_terminal::discovery::detection::italics_support;

if italics_support() {
    println!("\x1b[3mItalic text\x1b[23m");
}
```

Detection: terminfo `EnterItalicsMode` → `TERM_PROGRAM` → `TERM` patterns

### Underline Styles

```rust
use biscuit_terminal::discovery::detection::underline_support;

let support = underline_support();

if support.curly {
    // Curly underline for LSP errors
    println!("\x1b[4:3m\x1b[58:2::255:0:0mError\x1b[0m");
} else if support.straight {
    println!("\x1b[4mUnderlined\x1b[0m");
}

// Available flags:
// support.straight, support.double, support.curly
// support.dotted, support.dashed, support.colored
```

## Terminal Dimensions

```rust
use biscuit_terminal::discovery::detection::{terminal_width, terminal_height, dimensions};

let width = terminal_width();   // Default: 80
let height = terminal_height(); // Default: 24
let (w, h) = dimensions();
```

## TTY Detection

```rust
use biscuit_terminal::discovery::detection::is_tty;

if is_tty() {
    println!("\x1b[32mColored output\x1b[0m");
} else {
    println!("Plain output (piped)");
}
```

## Horizontal-Tab Interval

```rust
use biscuit_terminal::discovery::detection::tab_width;

let width = tab_width();
```

Detection uses terminfo's `init_tabs` capability and falls back to the standard
eight-column interval. Normal rendering should read the cached
`Terminal::tab_width` field; call this low-level helper when constructing a
terminal capability context.

## Multiplexing

```rust
use biscuit_terminal::discovery::detection::multiplex_support;

match multiplex_support() {
    MultiplexSupport::Tmux { split_window, session_persistence, .. } => {
        println!("tmux with persistence: {}", session_persistence);
    }
    MultiplexSupport::Zellij { floating_panes, .. } => {
        println!("Zellij with floating panes: {}", floating_panes);
    }
    MultiplexSupport::Native { split_window, .. } => {
        println!("Native multiplexing");
    }
    MultiplexSupport::None => {}
}
```

### MultiplexSupport Variants

```rust
pub enum MultiplexSupport {
    None,
    Native {
        split_window: bool,
        resize_pane: bool,
        focus_pane: bool,
        multiple_tabs: bool,
    },
    Tmux {
        split_window: bool,
        resize_pane: bool,
        focus_pane: bool,
        multiple_windows: bool,
        session_persistence: bool,
        detach_session: bool,
    },
    Zellij {
        split_window: bool,
        resize_pane: bool,
        focus_pane: bool,
        multiple_tabs: bool,
        session_resurrection: bool,
        floating_panes: bool,
        detach_session: bool,
    },
}
```

## Connection Detection

```rust
use biscuit_terminal::discovery::detection::detect_connection;

match detect_connection() {
    Connection::Local => println!("Local session"),
    Connection::SshClient(ssh) => {
        println!("SSH from {} (port {})", ssh.host, ssh.source_port);
    }
    Connection::MoshClient(mosh) => {
        println!("Mosh: {}", mosh.connection);
    }
}
```

Detection: `MOSH_CONNECTION` → `SSH_CLIENT` → Local

## OSC Queries

For querying terminal colors:

```rust
use biscuit_terminal::discovery::osc_queries::*;

// Query terminal colors (returns Option<RgbValue>)
let bg = bg_color();
let fg = text_color();
let cursor = cursor_color();

// Check support
let has_fg_query = osc10_support();
let has_bg_query = osc11_support();
let has_cursor_query = osc12_support();

// RgbValue methods
if let Some(bg) = bg_color() {
    let lum = bg.luminance();  // 0.0-1.0
    println!("Background: #{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b);
}
```

### Live query cost contract

Every live query (OSC 10/11/12, the graphics probe, CSI 14 t, DSR `CSI 6 n`)
goes through one engine, `discovery::tty_query`: a shared loop (deadline,
silence budget, `MAX_RESPONSE_BYTES`, caller-chosen DA1 sentinel so a wrapped
request gets a wrapped sentinel) over a platform `QueryPort`: `/dev/tty` in
raw mode via `select(2)` on Unix (typed keys read are discarded), the console
handles (`CONIN$`/`CONOUT$`) on native Windows (modes restored). Two entry
points: `round_trip` for single-purpose queries (cursor, CSI 14 t, standalone
OSC), which memoizes a request answered only by DA1 so it is never re-sent;
and `exchange`, the structured path the startup exchange uses (returns the
DA1 reply too; can skip when input is pending; the tmux-wrapped exchange turns
off silence recording). A terminal silent for the 500 ms local budget is
marked silent for the whole process (nothing is written again; do not shrink
the budget, WezTerm answers in 170-200 ms). `bg_color()`, `text_color()`, and
the direct graphics probe share one cached request
(`discovery::startup_exchange`); the wrapped exchange is cached separately.
Remote sessions (`SSH_CLIENT`, Mosh) get the full timeout;
`BISCUIT_TERMINAL_QUERY_SILENCE_MS` overrides the budget. On native Windows
only colors (color-list terminals plus Windows Terminal), the cell size
(`CSI 16 t`, riding in the startup exchange and cached per process, because
Windows has no pixel ioctl), and the graphics probe are queried, never in a
Windows SSH server session (there the 8×16 `CellSize::FALLBACK` applies);
cursor and CSI 14 t stay Unix-only. Typed keys read during a Windows exchange are written back with
`WriteConsoleInputW`. Full contract:
`biscuit-terminal/docs/discovery/terminal-queries.md`.

Construction-time probing is off with `Terminal::builder().probe(ProbeMode::Off)`
or `BISCUIT_TERMINAL_PROBE=off` (exact lowercase; an invalid value is an
`InvalidInput` route and queries nothing). Only the first construction in a
process probes; a TUI builds before its input loop and never probes inside it
or inside a mode 2026 synchronized-output block.

Add any new terminal query through `tty_query`, never a hand-rolled read
loop: a private loop forgets the silent memo, and one unanswering pty then pays
every timeout in turn (`wt list` once spent ~3.3 s and ~2 s of startup on two
1 s OSC waits, three CSI 14 t waits, and a 1 s DSR wait).

## Clipboard (OSC52)

```rust
use biscuit_terminal::discovery::clipboard::*;

if osc52_support() {
    set_clipboard("text to copy")?;
    // get_clipboard() has limited support
}
```

Supported: Kitty, WezTerm, iTerm2, Ghostty, Alacritty, Foot, Contour

## Mode 2027 (Grapheme Clustering)

```rust
use biscuit_terminal::discovery::mode_2027::supports_mode_2027;

if supports_mode_2027() {
    // Terminal handles Unicode grapheme clusters correctly
}
```

## Related

- [Terminal Struct](./terminal-struct.md) - Uses these functions internally
- [OS & Environment](./os-environment.md) - OS detection, CI, fonts
