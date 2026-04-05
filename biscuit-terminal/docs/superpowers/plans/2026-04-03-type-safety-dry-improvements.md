# Type Safety, DRY, and Error Handling Improvements

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve biscuit-terminal's type safety, eliminate DRY violations, fix error handling, and clean up dead code — all items from the 2026-04-03 code review.

**Architecture:** New types (`WidthSpec`, `HexColor`, `DetectionMethod`, `CliStyles`) are created in existing modules alongside their consumers. Shared utilities (escape code stripping, raw mode guard, unescape helper) are consolidated into canonical locations in the library. CLI error paths switch from `process::exit(1)` to proper `Result` returns.

**Tech Stack:** Rust, clap (derive), serde, regex, libc, color-eyre

---

## File Map

### New Files
- `biscuit-terminal/cli/src/types.rs` — `WidthSpec`, `HexColor`, `CliStyles`, `unescape_shell_escapes()`
- `biscuit-terminal/lib/src/discovery/raw_mode.rs` — shared `RawModeGuard` and `TERMINAL_QUERY_MUTEX`

### Modified Files (Library)
- `biscuit-terminal/lib/src/discovery/detection.rs` — `DetectionMethod` enum, `MultiplexSupport` simplification
- `biscuit-terminal/lib/src/discovery/mod.rs` — add `pub mod raw_mode`
- `biscuit-terminal/lib/src/discovery/osc_queries.rs` — use shared `RawModeGuard`
- `biscuit-terminal/lib/src/discovery/cursor_position.rs` — use shared `RawModeGuard`
- `biscuit-terminal/lib/src/discovery/fonts.rs` — use shared `RawModeGuard`
- `biscuit-terminal/lib/src/discovery/eval.rs` — make `ANSI_ESCAPE_RE` and `strip_ansi_codes` public
- `biscuit-terminal/lib/src/utils/escape_codes.rs` — use canonical regex from eval.rs
- `biscuit-terminal/lib/src/utils/text.rs` — use canonical regex from eval.rs
- `biscuit-terminal/lib/src/utils/mod.rs` — remove `truncate` (dead module)
- `biscuit-terminal/lib/src/utils/word_wrap.rs` — fix `word_wrap()` to return `Vec<String>`

### Modified Files (CLI)
- `biscuit-terminal/cli/src/main.rs` — use `CliStyles`, clap `ValueEnum` for completions
- `biscuit-terminal/cli/src/args.rs` — use `WidthSpec`, `HexColor` types
- `biscuit-terminal/cli/src/commands.rs` — use `CliStyles`, `unescape_shell_escapes`, `emit_vertical_margins` consistently, extract `build_render_meta`, `output_render_meta`
- `biscuit-terminal/cli/src/output.rs` — use `CliStyles`, `impl From<RgbColor> for ColorInfo`, derive `Serialize` on source enums

---

## Phase 1: Library Types & Consolidation

### Task 1: Create `DetectionMethod` enum (review item 1.3)

**Files:**
- Modify: `biscuit-terminal/lib/src/discovery/detection.rs`

- [ ] **Step 1: Add `DetectionMethod` enum after `ImageSupportResult`**

In `biscuit-terminal/lib/src/discovery/detection.rs`, after the `ImageSupportResult` struct (line 42), add:

```rust
/// The method used to detect image support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Direct TTY capability check
    TtyCheck,
    /// Detection via viuer library probing
    Viuer,
    /// Heuristic based on environment variables
    EnvHeuristic,
    /// Known terminal application lookup
    KnownTerminal,
}

impl std::fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TtyCheck => write!(f, "tty_check"),
            Self::Viuer => write!(f, "viuer"),
            Self::EnvHeuristic => write!(f, "env_heuristic"),
            Self::KnownTerminal => write!(f, "known_terminal"),
        }
    }
}
```

- [ ] **Step 2: Change `ImageSupportResult.method` from `String` to `DetectionMethod`**

Change the struct field:

```rust
pub struct ImageSupportResult {
    pub support: ImageSupport,
    pub reason: String,
    pub method: DetectionMethod,
}
```

- [ ] **Step 3: Replace all `method: "xxx".to_string()` with enum variants**

In the same file, replace every occurrence:
- `method: "tty_check".to_string()` → `method: DetectionMethod::TtyCheck`
- `method: "viuer".to_string()` → `method: DetectionMethod::Viuer`
- `method: "env_heuristic".to_string()` → `method: DetectionMethod::EnvHeuristic`
- `method: "known_terminal".to_string()` → `method: DetectionMethod::KnownTerminal`
- `method: "test_method".to_string()` → `method: DetectionMethod::TtyCheck` (in test code)

These are at lines: 567, 599, 620, 671, 687, 705, 719, 749, 767, 784, 800, 813, 1360, 1373, 1386.

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/discovery/detection.rs
git commit -m "refactor(biscuit-terminal): replace ImageSupportResult.method String with DetectionMethod enum"
```

---

### Task 2: Simplify `MultiplexSupport` to unit variants (review item 1.7)

**Files:**
- Modify: `biscuit-terminal/lib/src/discovery/detection.rs`
- Modify: `biscuit-terminal/cli/src/output.rs` (format_multiplex)

- [ ] **Step 1: Replace `MultiplexSupport` with unit variants**

In `detection.rs`, replace the entire `MultiplexSupport` enum (lines 157-228) with:

```rust
/// Type of terminal multiplexing support available.
///
/// All detected multiplexers support their full capability set (split, resize,
/// focus, tabs). Individual capability fields were removed because every variant
/// always returned `true` for all fields — no version-based detection exists.
///
/// ## Detection
///
/// Detection is based on environment variables:
/// - `TMUX` - Set when running inside tmux
/// - `ZELLIJ` - Set when running inside Zellij
/// - `TERM_PROGRAM` - Identifies terminals with native multiplexing (Kitty, WezTerm, Ghostty)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MultiplexSupport {
    /// No multiplexing support available
    None,
    /// Native multiplexing built into the terminal emulator (Kitty, WezTerm, Ghostty)
    Native,
    /// tmux multiplexer detected
    Tmux,
    /// Zellij multiplexer detected
    Zellij,
}
```

- [ ] **Step 2: Update `multiplex_support()` function to return unit variants**

Replace all the struct-style returns in `multiplex_support()` (lines 900-973):

```rust
pub fn multiplex_support() -> MultiplexSupport {
    if env::var("TMUX").is_ok() {
        return MultiplexSupport::Tmux;
    }

    if env::var("ZELLIJ").is_ok() {
        return MultiplexSupport::Zellij;
    }

    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "kitty" | "WezTerm" | "ghostty" => {
                return MultiplexSupport::Native;
            }
            _ => {}
        }
    }

    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") || term.contains("wezterm") || term.contains("ghostty") {
        return MultiplexSupport::Native;
    }

    MultiplexSupport::None
}
```

- [ ] **Step 3: Update `format_multiplex` in `output.rs`**

In `biscuit-terminal/cli/src/output.rs`, update `format_multiplex` (line 347):

```rust
pub fn format_multiplex(m: MultiplexSupport) -> String {
    match m {
        MultiplexSupport::None => "None".to_string(),
        MultiplexSupport::Native => "Native".to_string(),
        MultiplexSupport::Tmux => "tmux".to_string(),
        MultiplexSupport::Zellij => "Zellij".to_string(),
    }
}
```

- [ ] **Step 4: Update any pattern matches against `MultiplexSupport` with struct fields**

Search for any `MultiplexSupport::Tmux { .. }` or similar patterns in the codebase and update to unit variant matches.

- [ ] **Step 5: Run tests**

Run: `just test -p biscuit-terminal && just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add biscuit-terminal/lib/src/discovery/detection.rs biscuit-terminal/cli/src/output.rs
git commit -m "refactor(biscuit-terminal): simplify MultiplexSupport to unit variants

All capability booleans were always true — no version-based detection exists."
```

---

### Task 3: Make `ANSI_ESCAPE_RE` and `strip_ansi_codes` public in eval.rs (review item 2.1, part 1)

**Files:**
- Modify: `biscuit-terminal/lib/src/discovery/eval.rs`

- [ ] **Step 1: Make `strip_ansi_codes` public**

In `eval.rs` line 44, change:

```rust
fn strip_ansi_codes(text: &str) -> String {
```
to:
```rust
pub fn strip_ansi_codes(text: &str) -> String {
```

- [ ] **Step 2: Make `ANSI_ESCAPE_RE` public**

In `eval.rs` line 20, change:

```rust
static ANSI_ESCAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
```
to:
```rust
pub static ANSI_ESCAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
```

- [ ] **Step 3: Run tests**

Run: `just test -p biscuit-terminal`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-terminal/lib/src/discovery/eval.rs
git commit -m "refactor(biscuit-terminal): make ANSI_ESCAPE_RE and strip_ansi_codes public for reuse"
```

---

### Task 4: Consolidate escape code stripping (review item 2.1, part 2)

**Files:**
- Modify: `biscuit-terminal/lib/src/utils/escape_codes.rs`
- Modify: `biscuit-terminal/lib/src/utils/text.rs`

- [ ] **Step 1: Rewrite `escape_codes.rs` to use canonical regex from `eval.rs`**

Replace the entire contents of `biscuit-terminal/lib/src/utils/escape_codes.rs` with:

```rust
use crate::discovery::eval::ANSI_ESCAPE_RE;
use std::sync::LazyLock;

use regex::Regex;

/// Strips **all** escape codes from the passed-in string.
///
/// Uses the canonical `ANSI_ESCAPE_RE` from `discovery::eval` which handles
/// CSI, OSC (with both BEL and ST terminators), and Fe escape sequences.
pub fn strip_escape_codes<T: Into<String>>(content: T) -> String {
    let content = content.into();
    ANSI_ESCAPE_RE.replace_all(&content, "").into_owned()
}

/// Strips all OSC8 hyperlinks from the passed-in text while retaining
/// other escape codes.
///
/// OSC8 links have the format: `\x1b]8;;<uri>\x07<link text>\x1b]8;;\x07`
/// Also handles ST terminator variant: `\x1b]8;;<uri>\x1b\\`
pub fn strip_osc8_links<T: Into<String>>(content: T) -> String {
    static OSC8_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\x1b\]8;;[^\x07\x1b]*(?:\x07|\x1b\\)").expect("Invalid OSC8 link regex")
    });

    let content = content.into();
    OSC8_LINK_RE.replace_all(&content, "").into_owned()
}

/// Strip escape codes used for cursor movement while retaining other escape codes.
///
/// Cursor movement CSI sequences:
/// - `\x1b[<n>A` through `\x1b[<n>G` — directional movement
/// - `\x1b[<row>;<col>H` / `\x1b[<row>;<col>f` — absolute positioning
/// - `\x1b[s` / `\x1b[u` — save/restore cursor position
pub fn strip_cursor_movement_codes<T: Into<String>>(content: T) -> String {
    static CURSOR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\x1b\[[0-9;]*[ABCDEFGHfsu]").expect("Invalid cursor movement regex")
    });

    let content = content.into();
    CURSOR_RE.replace_all(&content, "").into_owned()
}

/// Strip terminal query codes from a string while retaining other escape codes.
///
/// Query codes include Device Attributes (`c`) and Device Status Report (`n`).
pub fn strip_query_codes<T: Into<String>>(content: T) -> String {
    static QUERY_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\x1b\[[0-9;>]*[cn]").expect("Invalid query code regex")
    });

    let content = content.into();
    QUERY_RE.replace_all(&content, "").into_owned()
}

/// Strip color/SGR codes from a string while retaining other escape codes.
///
/// SGR sequences end with `m` (e.g., `\x1b[31m` for red, `\x1b[0m` for reset).
pub fn strip_color_codes<T: Into<String>>(content: T) -> String {
    static SGR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\x1b\[[0-9;]*m").expect("Invalid SGR regex")
    });

    let content = content.into();
    SGR_RE.replace_all(&content, "").into_owned()
}
```

- [ ] **Step 2: Rewrite `text.rs` to use canonical regex**

Replace the entire contents of `biscuit-terminal/lib/src/utils/text.rs` with:

```rust
use crate::discovery::eval::strip_ansi_codes;

/// Produces a vector where each element represents a line's length
/// after all escape codes have been stripped.
pub fn content_length(content: &str) -> Vec<u32> {
    content
        .lines()
        .map(|line| strip_ansi_codes(line).len() as u32)
        .collect()
}
```

- [ ] **Step 3: Run tests**

Run: `just test -p biscuit-terminal`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-terminal/lib/src/utils/escape_codes.rs biscuit-terminal/lib/src/utils/text.rs
git commit -m "refactor(biscuit-terminal): consolidate escape code stripping to use canonical LazyLock regex

Eliminates per-call Regex::new() in escape_codes.rs and text.rs.
Fixes OSC ST-terminator bug in 3 locations."
```

---

### Task 5: Extract shared `RawModeGuard` (review item 2.2)

**Files:**
- Create: `biscuit-terminal/lib/src/discovery/raw_mode.rs`
- Modify: `biscuit-terminal/lib/src/discovery/mod.rs`
- Modify: `biscuit-terminal/lib/src/discovery/osc_queries.rs`
- Modify: `biscuit-terminal/lib/src/discovery/cursor_position.rs`
- Modify: `biscuit-terminal/lib/src/discovery/fonts.rs`

- [ ] **Step 1: Create `raw_mode.rs` with shared `RawModeGuard` and `TERMINAL_QUERY_MUTEX`**

Create `biscuit-terminal/lib/src/discovery/raw_mode.rs`:

```rust
//! Shared raw terminal mode infrastructure.
//!
//! Provides an RAII guard for entering/exiting raw terminal mode and a global
//! mutex to serialize all terminal queries (OSC, DSR, CSI 14 t, etc.).

#[cfg(unix)]
use std::sync::Mutex;

/// Global mutex to serialize terminal queries.
///
/// Multiple concurrent queries (OSC color probes, DSR cursor position,
/// CSI 14 t window size) would race on stdin/stdout and corrupt responses.
/// All query functions must hold this lock.
#[cfg(unix)]
pub static TERMINAL_QUERY_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard that enters raw terminal mode on creation and restores the
/// original terminal state on drop.
///
/// ## Errors
///
/// Returns `Err` if `tcgetattr` or `tcsetattr` fails (e.g., not a real TTY).
#[cfg(unix)]
pub struct RawModeGuard {
    original: libc::termios,
    fd: libc::c_int,
}

#[cfg(unix)]
impl RawModeGuard {
    /// Enter raw mode on the given file descriptor.
    ///
    /// Disables canonical mode and echo. Sets VMIN=0 and VTIME=1 (100ms read timeout).
    pub fn new(fd: libc::c_int) -> Result<Self, String> {
        let mut original: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
            return Err("failed to get terminal attributes".into());
        }
        let mut raw = original;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO);
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 1;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err("failed to set raw mode".into());
        }
        Ok(Self { original, fd })
    }

    /// Enter raw mode on stdin (`STDIN_FILENO`).
    pub fn stdin() -> Result<Self, String> {
        Self::new(libc::STDIN_FILENO)
    }
}

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}
```

- [ ] **Step 2: Add `pub mod raw_mode` to `discovery/mod.rs`**

In `biscuit-terminal/lib/src/discovery/mod.rs`, add:

```rust
pub mod raw_mode;
```

- [ ] **Step 3: Update `osc_queries.rs` to use shared guard and re-export mutex**

In `osc_queries.rs`:
1. Remove the local `TERMINAL_QUERY_MUTEX` static (line 98).
2. Remove the inline `RawModeGuard` struct and its impls (lines 626-664).
3. Import from the shared module:

```rust
use super::raw_mode::{RawModeGuard, TERMINAL_QUERY_MUTEX};
```

4. Replace `RawModeGuard::new()` calls with `RawModeGuard::stdin()`. The `?` operator will need to map the `String` error to `OscQueryError::IoError`:

```rust
let _guard = RawModeGuard::stdin().map_err(|e| OscQueryError::IoError(e))?;
```

- [ ] **Step 4: Update `cursor_position.rs` to use shared guard and mutex**

In `cursor_position.rs`:
1. Remove the inline `RawModeGuard` struct and its impls (lines 60-91).
2. Import:

```rust
use super::raw_mode::{RawModeGuard, TERMINAL_QUERY_MUTEX};
```

3. Add mutex lock before the guard:

```rust
let _lock = TERMINAL_QUERY_MUTEX.lock().map_err(|_| "terminal query mutex poisoned".to_string())?;
let _guard = RawModeGuard::stdin()?;
```

- [ ] **Step 5: Update `fonts.rs` `window_size_pixels()` to use shared guard**

In `fonts.rs`, in the `window_size_pixels()` function (around line 600):
1. Import:

```rust
use super::raw_mode::{RawModeGuard, TERMINAL_QUERY_MUTEX};
```

2. Replace the inline raw mode setup (lines 612-632) with:

```rust
let _lock = TERMINAL_QUERY_MUTEX.lock().ok()?;
let _guard = RawModeGuard::new(fd).ok()?;
```

3. Remove the `restore` closure (line 630-632) and all `restore(fd, &orig_termios)` calls — the guard handles restoration on drop.

- [ ] **Step 6: Run tests**

Run: `just test -p biscuit-terminal && just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add biscuit-terminal/lib/src/discovery/raw_mode.rs biscuit-terminal/lib/src/discovery/mod.rs biscuit-terminal/lib/src/discovery/osc_queries.rs biscuit-terminal/lib/src/discovery/cursor_position.rs biscuit-terminal/lib/src/discovery/fonts.rs
git commit -m "refactor(biscuit-terminal): extract shared RawModeGuard into discovery::raw_mode

Eliminates ~150 lines of duplicated raw mode setup across osc_queries,
cursor_position, and fonts. All terminal queries now serialize through
a shared TERMINAL_QUERY_MUTEX."
```

---

### Task 6: Fix `word_wrap()` return value and delete empty `truncate.rs` (review items 2.10, 2.11)

**Files:**
- Modify: `biscuit-terminal/lib/src/utils/word_wrap.rs`
- Delete: `biscuit-terminal/lib/src/utils/truncate.rs`

- [ ] **Step 1: Fix `word_wrap()` to return its result**

In `word_wrap.rs` line 18, change:

```rust
pub fn word_wrap<T: Into<String>>(content: T, strategy: WordWrap, width: u32) {
    let lines = split_lines(content);
    let _ = wrap_lines(lines, &strategy, width);
}
```

to:

```rust
pub fn word_wrap<T: Into<String>>(content: T, strategy: WordWrap, width: u32) -> Vec<String> {
    let lines = split_lines(content);
    wrap_lines(lines, &strategy, width)
}
```

- [ ] **Step 2: Verify `word_wrap` callers still compile**

Run: `cargo check -p biscuit-terminal`

If any callers used `word_wrap()` as a statement (ignoring the return), they'll still compile since `Vec<String>` is not `#[must_use]`.

- [ ] **Step 3: Delete `truncate.rs`**

```bash
rm biscuit-terminal/lib/src/utils/truncate.rs
```

Verify it is NOT declared in `utils/mod.rs` (it isn't — confirmed by reading the file).

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/utils/word_wrap.rs
git rm biscuit-terminal/lib/src/utils/truncate.rs
git commit -m "fix(biscuit-terminal): word_wrap() now returns Vec<String>, delete empty truncate.rs"
```

---

## Phase 2: CLI Types & Newtypes

### Task 7: Create `WidthSpec` newtype (review item 1.1)

**Files:**
- Create: `biscuit-terminal/cli/src/types.rs`
- Modify: `biscuit-terminal/cli/src/main.rs` (add `mod types`)
- Modify: `biscuit-terminal/cli/src/args.rs`

- [ ] **Step 1: Create `types.rs` with `WidthSpec`**

Create `biscuit-terminal/cli/src/types.rs`:

```rust
use std::fmt;
use std::str::FromStr;

/// Parsed display width specification for images and diagrams.
///
/// Accepted formats:
/// - `"50%"` — percentage of terminal width
/// - `"80ch"` or `"80"` — fixed character width
/// - `"fill"` — fill available terminal width
///
/// Implements `FromStr` for clap parse-time validation.
#[derive(Debug, Clone, PartialEq)]
pub enum WidthSpec {
    /// Percentage of terminal width (1-100)
    Percent(u8),
    /// Fixed width in characters
    Chars(u32),
    /// Fill available terminal width
    Fill,
}

impl FromStr for WidthSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.eq_ignore_ascii_case("fill") {
            return Ok(Self::Fill);
        }

        if let Some(pct) = s.strip_suffix('%') {
            let value: u8 = pct
                .parse()
                .map_err(|_| format!("invalid percentage: '{}'", pct))?;
            if value == 0 || value > 100 {
                return Err(format!("percentage must be 1-100, got {}", value));
            }
            return Ok(Self::Percent(value));
        }

        if let Some(chars) = s.strip_suffix("ch") {
            let value: u32 = chars
                .parse()
                .map_err(|_| format!("invalid character width: '{}'", chars))?;
            if value == 0 {
                return Err("character width must be > 0".to_string());
            }
            return Ok(Self::Chars(value));
        }

        // Plain number = characters
        let value: u32 = s
            .parse()
            .map_err(|_| format!("invalid width spec '{}': expected percentage (e.g., 50%), characters (e.g., 80 or 80ch), or 'fill'", s))?;
        if value == 0 {
            return Err("character width must be > 0".to_string());
        }
        Ok(Self::Chars(value))
    }
}

impl fmt::Display for WidthSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Percent(p) => write!(f, "{}%", p),
            Self::Chars(c) => write!(f, "{}", c),
            Self::Fill => write!(f, "fill"),
        }
    }
}
```

- [ ] **Step 2: Add `pub mod types;` to `main.rs`**

In `biscuit-terminal/cli/src/main.rs`, add after the existing module declarations:

```rust
pub mod types;
```

And add to the `use crate::*` section or explicit import in `main.rs` and `args.rs`.

- [ ] **Step 3: Replace `width: Option<String>` with `width: Option<WidthSpec>` in `args.rs`**

In `args.rs`, for all 11 occurrences of `width: Option<String>` (lines 105, 173, 300, 424, 509, 590, 679, 757, 823, 907, 974), replace with:

```rust
/// Display width: percentage (e.g., "50%"), characters (e.g., "80ch" or "80"), or "fill"
#[arg(long, short = 'w')]
width: Option<crate::types::WidthSpec>,
```

Also handle `left_width` in `TwoColumn` (line 1275) if it uses the same pattern.

- [ ] **Step 4: Update all callers that pass `width.as_deref()` or `width: Option<&str>`**

In `commands.rs` and `main.rs`, update function signatures and call sites:
- Functions like `build_mermaid_diagram` that take `width: Option<&str>` should now take `width: Option<&WidthSpec>` and call `width.map(|w| w.to_string())` at the point where they call `parse_width_spec()`, OR better — pass the `WidthSpec` string form to `parse_width_spec`:

```rust
// Before:
let image_width = parse_width_spec(w).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

// After:
let image_width = parse_width_spec(&w.to_string()).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
```

For the JSON output blocks that serialize `width`, use `width.as_ref().map(|w| w.to_string())`.

- [ ] **Step 5: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass. Invalid widths like `--width abc` now produce a clap error instead of reaching render time.

- [ ] **Step 6: Commit**

```bash
git add biscuit-terminal/cli/src/types.rs biscuit-terminal/cli/src/main.rs biscuit-terminal/cli/src/args.rs biscuit-terminal/cli/src/commands.rs
git commit -m "feat(biscuit-terminal): add WidthSpec newtype for parse-time width validation

Rejects invalid width strings at argument parse time instead of at render time.
Covers all 11 subcommand width fields."
```

---

### Task 8: Create `HexColor` newtype (review item 1.2)

**Files:**
- Modify: `biscuit-terminal/cli/src/types.rs`
- Modify: `biscuit-terminal/cli/src/args.rs`
- Modify: `biscuit-terminal/cli/src/commands.rs`

- [ ] **Step 1: Add `HexColor` to `types.rs`**

Append to `biscuit-terminal/cli/src/types.rs`:

```rust
/// A validated hexadecimal color value.
///
/// Accepted formats: `#rgb`, `#rrggbb`, `#rrggbbaa`
///
/// Implements `FromStr` for clap parse-time validation.
#[derive(Debug, Clone, PartialEq)]
pub struct HexColor(String);

impl HexColor {
    /// Returns the hex color string including the `#` prefix.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for HexColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if !s.starts_with('#') {
            return Err(format!("hex color must start with '#', got '{}'", s));
        }

        let hex_part = &s[1..];
        if !matches!(hex_part.len(), 3 | 6 | 8) {
            return Err(format!(
                "invalid hex color '{}': expected #rgb, #rrggbb, or #rrggbbaa",
                s
            ));
        }

        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("invalid hex characters in '{}'", s));
        }

        Ok(Self(s.to_string()))
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

- [ ] **Step 2: Update `args.rs` quadrant fill fields to use `HexColor`**

In `args.rs`, change the four fill fields (lines 323-337):

```rust
/// Top-right quadrant (q1) fill color (hex, e.g., "#e8f5e9")
#[arg(long = "q1-fill")]
q1_fill: Option<crate::types::HexColor>,

/// Top-left quadrant (q2) fill color (hex, e.g., "#ffffff")
#[arg(long = "q2-fill")]
q2_fill: Option<crate::types::HexColor>,

/// Bottom-left quadrant (q3) fill color (hex, e.g., "#ffebee")
#[arg(long = "q3-fill")]
q3_fill: Option<crate::types::HexColor>,

/// Bottom-right quadrant (q4) fill color (hex, e.g., "#ffffff")
#[arg(long = "q4-fill")]
q4_fill: Option<crate::types::HexColor>,
```

- [ ] **Step 3: Update callers to use `.as_str()`**

In `commands.rs` `render_quadrant`, update the function signature and color usage:
- Change `q1_fill: Option<&str>` → `q1_fill: Option<&HexColor>` (and same for q2-q4)
- Where `cfg.with_quadrant_fill(1, color)` is called, use `color.as_str()`

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass. Invalid colors like `--q1-fill red` now produce a clap error.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/cli/src/types.rs biscuit-terminal/cli/src/args.rs biscuit-terminal/cli/src/commands.rs
git commit -m "feat(biscuit-terminal): add HexColor newtype for parse-time color validation"
```

---

### Task 9: Make `--completions` a clap `ValueEnum` (review item 1.4)

**Files:**
- Modify: `biscuit-terminal/cli/src/args.rs`
- Modify: `biscuit-terminal/cli/src/main.rs`

- [ ] **Step 1: Add `ShellType` enum to `args.rs`**

At the top of `args.rs`, add:

```rust
/// Shell type for completion script generation.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ShellType {
    Bash,
    Elvish,
    Fish,
    #[value(alias = "pwsh")]
    Powershell,
    Zsh,
}
```

- [ ] **Step 2: Change `completions` field type**

In the `Args` struct (line 76), change:

```rust
#[arg(long, value_name = "SHELL", global = true, display_order = 102)]
pub completions: Option<ShellType>,
```

- [ ] **Step 3: Simplify `handle_completions` in `main.rs`**

Replace `handle_completions` (lines 500-525):

```rust
fn handle_completions(shell_type: &ShellType) -> color_eyre::Result<()> {
    let shell = match shell_type {
        ShellType::Bash => Shell::Bash,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Fish => Shell::Fish,
        ShellType::Powershell => Shell::PowerShell,
        ShellType::Zsh => Shell::Zsh,
    };

    print_completions(shell);
    Ok(())
}
```

- [ ] **Step 4: Update the call site in `main.rs`**

Change the call site (line 100-101):

```rust
if let Some(ref shell_type) = args.completions {
    return handle_completions(shell_type);
}
```

Note: The "help" subcommand of completions is no longer needed — clap's `ValueEnum` auto-generates valid value lists. If you want to keep the help option, add a `Help` variant to the enum and handle it separately.

- [ ] **Step 5: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass. Invalid shells now produce a clap error listing valid values.

- [ ] **Step 6: Commit**

```bash
git add biscuit-terminal/cli/src/args.rs biscuit-terminal/cli/src/main.rs
git commit -m "refactor(biscuit-terminal): use clap ValueEnum for --completions shell type

Invalid shells now rejected at parse time with auto-generated help."
```

---

### Task 10: Validate `aspect_ratio` with `PositiveF32` (review item 1.8)

**Files:**
- Modify: `biscuit-terminal/cli/src/types.rs`
- Modify: `biscuit-terminal/cli/src/args.rs`
- Modify: `biscuit-terminal/cli/src/commands.rs`

- [ ] **Step 1: Add `PositiveF32` to `types.rs`**

Append to `biscuit-terminal/cli/src/types.rs`:

```rust
/// A positive, finite f32 value (> 0.0, not NaN, not infinity).
///
/// Used for aspect ratios and other values that must be positive real numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositiveF32(f32);

impl PositiveF32 {
    /// Returns the inner f32 value.
    pub fn value(self) -> f32 {
        self.0
    }
}

impl FromStr for PositiveF32 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = s
            .parse()
            .map_err(|_| format!("invalid number: '{}'", s))?;
        if !value.is_finite() {
            return Err(format!("value must be finite, got {}", value));
        }
        if value <= 0.0 {
            return Err(format!("value must be positive, got {}", value));
        }
        Ok(Self(value))
    }
}

impl fmt::Display for PositiveF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

- [ ] **Step 2: Update `aspect_ratio` fields in `args.rs`**

Change lines 605 and 694:

```rust
/// Aspect ratio (width/height). Default: 1.5
#[arg(long)]
aspect_ratio: Option<crate::types::PositiveF32>,
```

- [ ] **Step 3: Update callers to use `.value()`**

In `commands.rs`, wherever `aspect_ratio` is used as `f32`, call `.value()`:

```rust
// Before:
if let Some(ar) = aspect_ratio { ... use ar ... }
// After:
if let Some(ar) = aspect_ratio { ... use ar.value() ... }
```

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass. `--aspect-ratio 0`, `--aspect-ratio -1`, `--aspect-ratio inf` now produce clap errors.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/cli/src/types.rs biscuit-terminal/cli/src/args.rs biscuit-terminal/cli/src/commands.rs
git commit -m "feat(biscuit-terminal): add PositiveF32 newtype for aspect_ratio validation"
```

---

## Phase 3: CLI DRY & Error Handling

### Task 11: Extract `CliStyles` helper (review item 2.3)

**Files:**
- Modify: `biscuit-terminal/cli/src/types.rs`
- Modify: `biscuit-terminal/cli/src/main.rs`
- Modify: `biscuit-terminal/cli/src/commands.rs`
- Modify: `biscuit-terminal/cli/src/output.rs`

- [ ] **Step 1: Add `CliStyles` to `types.rs`**

Append to `types.rs`:

```rust
/// Terminal text styles that respect the `NO_COLOR` environment variable.
///
/// When `NO_COLOR` is set, all style codes are empty strings.
pub struct CliStyles {
    pub bold: &'static str,
    pub dim: &'static str,
    pub reset: &'static str,
    pub green: &'static str,
    pub yellow: &'static str,
    pub blue: &'static str,
    pub red: &'static str,
}

impl CliStyles {
    /// Detect whether to use ANSI styles based on `NO_COLOR` env var.
    pub fn detect() -> Self {
        if std::env::var("NO_COLOR").is_ok() {
            Self::plain()
        } else {
            Self::ansi()
        }
    }

    /// No styling (for `NO_COLOR` or non-TTY output).
    pub fn plain() -> Self {
        Self {
            bold: "",
            dim: "",
            reset: "",
            green: "",
            yellow: "",
            blue: "",
            red: "",
        }
    }

    /// ANSI escape code styling.
    pub fn ansi() -> Self {
        Self {
            bold: "\x1b[1m",
            dim: "\x1b[2m",
            reset: "\x1b[0m",
            green: "\x1b[32m",
            yellow: "\x1b[33m",
            blue: "\x1b[34m",
            red: "\x1b[31m",
        }
    }
}
```

- [ ] **Step 2: Update `print_example_command` in `main.rs`**

Replace lines 63-81:

```rust
fn print_example_command(cmd: &str) {
    let s = crate::types::CliStyles::detect();

    println!();
    println!("{}Command:{}", s.bold, s.reset);
    println!("{}{}{}", s.dim, cmd, s.reset);
}
```

- [ ] **Step 3: Update `handle_mermaid_error` in `commands.rs`**

Replace lines 1641-1646:

```rust
let s = crate::types::CliStyles::detect();
```

Then use `s.red`, `s.bold`, `s.dim`, `s.reset` in place of the local variables.

- [ ] **Step 4: Update `handle_graph_error` in `commands.rs`**

Same pattern — replace lines 1683-1687 with `let s = CliStyles::detect();`.

- [ ] **Step 5: Update `print_content_analysis` in `output.rs`**

Replace lines 301-305:

```rust
let s = crate::types::CliStyles::detect();
```

Use `s.bold`, `s.dim`, `s.reset`, `s.green` throughout.

- [ ] **Step 6: Update `print_pretty` in `output.rs`**

Replace lines 358-365:

```rust
let s = crate::types::CliStyles::detect();
```

Use `s.bold`, `s.dim`, `s.reset`, `s.green`, `s.yellow`, `s.blue` throughout.

- [ ] **Step 7: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add biscuit-terminal/cli/src/types.rs biscuit-terminal/cli/src/main.rs biscuit-terminal/cli/src/commands.rs biscuit-terminal/cli/src/output.rs
git commit -m "refactor(biscuit-terminal): extract CliStyles to eliminate 5 duplicate NO_COLOR checks"
```

---

### Task 12: Use `emit_vertical_margins` consistently (review item 2.4)

**Files:**
- Modify: `biscuit-terminal/cli/src/commands.rs`

- [ ] **Step 1: Update `render_image` to use `emit_vertical_margins`**

In `commands.rs`, replace lines 136-142:

```rust
// Output the result with vertical margins
for _ in 0..layout.margin_top.unwrap_or(0) {
    println!();
}
emit_image_output(&output)?;
for _ in 0..layout.margin_bottom.unwrap_or(0) {
    println!();
}
```

with:

```rust
// Output the result with vertical margins
emit_vertical_margins(layout, || {
    emit_image_output(&output)
})?;
```

- [ ] **Step 2: Update `display_mermaid` to use `emit_vertical_margins`**

Replace lines 322-348 (the margin + emit + meta + margin block):

```rust
emit_vertical_margins(layout, || {
    emit_image_output(&result.output)
})?;

// Output metadata if requested
if meta {
    let file_size_bytes = std::fs::metadata(&result.png_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let render_meta = RenderMeta {
        filename: result.png_path.to_string_lossy().to_string(),
        cache_hit: result.cache_hit,
        file_size_bytes,
        render_time_ms,
    };

    eprintln!("{}", serde_json::to_string(&render_meta)?);
}
```

- [ ] **Step 3: Update `display_graph` to use `emit_vertical_margins`**

Replace the margin code blocks in `display_graph` (lines 373-379 and 387-410) similarly. For the `NoImageSupport` fallback case:

```rust
Err(GraphRenderError::NoImageSupport) => {
    emit_vertical_margins(layout, || {
        print!("{}", graph.render(&terminal));
        Ok(())
    })?;
    return Ok(());
}
```

And for the success case:

```rust
emit_vertical_margins(layout, || {
    emit_image_output(&result.output)
})?;
```

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/cli/src/commands.rs
git commit -m "refactor(biscuit-terminal): use emit_vertical_margins consistently in all render functions"
```

---

### Task 13: Extract `unescape_shell_escapes` helper (review item 2.7)

**Files:**
- Modify: `biscuit-terminal/cli/src/types.rs`
- Modify: `biscuit-terminal/cli/src/commands.rs`

- [ ] **Step 1: Add `unescape_shell_escapes` to `types.rs`**

Append to `types.rs`:

```rust
/// Unescape common shell escape sequences.
///
/// The shell passes literal `\n`, `\t`, `\r` as two-character strings.
/// This converts them back to actual control characters.
pub fn unescape_shell_escapes(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
}
```

- [ ] **Step 2: Replace all inline unescape patterns in `commands.rs`**

Replace each `.replace("\\n", "\n").replace("\\t", "\t").replace("\\r", "\r")` block with a call to `crate::types::unescape_shell_escapes()`.

Locations (5 occurrences):
- `render_prose` (line 1733-1736)
- `render_quote` (line 1841-1844)
- `render_list` (line 1899-1902) — inside the `.map()` closure
- `render_columns` left_text (line 1951-1954)
- `render_columns` right_text (line 1955-1958)

Example for `render_prose`:

```rust
// Before:
let text = text
    .replace("\\n", "\n")
    .replace("\\t", "\t")
    .replace("\\r", "\r");

// After:
let text = crate::types::unescape_shell_escapes(&text);
```

- [ ] **Step 3: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-terminal/cli/src/types.rs biscuit-terminal/cli/src/commands.rs
git commit -m "refactor(biscuit-terminal): extract unescape_shell_escapes helper (5 call sites)"
```

---

### Task 14: Extract `output_render_meta` helper (review items 2.5, 2.6)

**Files:**
- Modify: `biscuit-terminal/cli/src/commands.rs`

- [ ] **Step 1: Add `output_render_meta` helper function**

Add near the top of `commands.rs` (after `emit_vertical_margins`):

```rust
/// Output render metadata to stderr as JSON.
///
/// Shared by `render_image`, `display_mermaid`, and `display_graph`.
fn output_render_meta(render_meta: &RenderMeta) -> color_eyre::Result<()> {
    eprintln!("{}", serde_json::to_string(render_meta)?);
    Ok(())
}
```

- [ ] **Step 2: Replace inline `eprintln!("{}", serde_json::to_string(&render_meta)?)` calls**

In `render_image` (line 246), `display_mermaid` (line 342), and `display_graph` (line 405), replace:

```rust
eprintln!("{}", serde_json::to_string(&render_meta)?);
```

with:

```rust
output_render_meta(&render_meta)?;
```

- [ ] **Step 3: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-terminal/cli/src/commands.rs
git commit -m "refactor(biscuit-terminal): extract output_render_meta helper (3 call sites)"
```

---

### Task 15: Replace `std::process::exit(1)` with proper errors (review item 3.1)

**Files:**
- Modify: `biscuit-terminal/cli/src/commands.rs`

- [ ] **Step 1: Update `handle_mermaid_error` to return `Err` instead of `exit(1)`**

In `commands.rs`, replace line 1672:

```rust
std::process::exit(1);
```

with:

```rust
Err(color_eyre::eyre::eyre!("Failed to render {} diagram", diagram_type))
```

Also change the function to not return `Ok(())` on the `NoImageSupport` path — it should continue to `return Ok(())` there, but the other paths should return the error.

The updated function body for the error cases:

```rust
MermaidRenderError::Visualization(ref viz_err) => {
    eprintln!();
    eprintln!("{}{}Error:{} {}", s.red, s.bold, s.reset, viz_err);
    eprintln!("\n{}Mermaid {} was defined as:{}\n", s.dim, diagram_type, s.reset);
    eprintln!("```mermaid\n{}\n```", instructions);
    return Err(color_eyre::eyre::eyre!("{}", viz_err));
}
MermaidRenderError::DisplayError(ref msg) => {
    return Err(color_eyre::eyre::eyre!("Failed to display image: {}", msg));
}
```

- [ ] **Step 2: Update `handle_graph_error` to return `Err` instead of `exit(1)`**

Same pattern — replace line 1710:

```rust
std::process::exit(1);
```

with proper error returns from each match arm.

- [ ] **Step 3: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add biscuit-terminal/cli/src/commands.rs
git commit -m "fix(biscuit-terminal): replace process::exit(1) with proper error returns through color_eyre"
```

---

### Task 16: Add `impl From<RgbColor> for ColorInfo` (review item 2.8)

**Files:**
- Modify: `biscuit-terminal/cli/src/output.rs`

- [ ] **Step 1: Add the `From` impl**

After the `ColorInfo` struct definition (line 176), add:

```rust
impl From<biscuit_terminal::discovery::osc_queries::RgbColor> for ColorInfo {
    fn from(c: biscuit_terminal::discovery::osc_queries::RgbColor) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            hex: Some(format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
        }
    }
}
```

- [ ] **Step 2: Simplify `collect_metadata` color construction**

Replace lines 198-217:

```rust
let bg_color = osc_queries::bg_color().map(ColorInfo::from);
let text_color = osc_queries::text_color().map(ColorInfo::from);
let cursor_color = osc_queries::cursor_color().map(ColorInfo::from);
```

- [ ] **Step 3: Verify `RgbColor` is public**

Check that `osc_queries::RgbColor` is a public struct. If not, the `From` impl needs to use the full path.

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/cli/src/output.rs
git commit -m "refactor(biscuit-terminal): add From<RgbColor> for ColorInfo, simplify 3 construction sites"
```

---

### Task 17: Derive `Serialize` on source enums for stable JSON (review item 1.5)

**Files:**
- Modify: `biscuit-terminal/lib/src/discovery/detection.rs`
- Modify: `biscuit-terminal/cli/src/output.rs`

- [ ] **Step 1: Add `#[serde(rename_all = "kebab-case")]` to relevant enums**

In `detection.rs`, the enums `TerminalApp`, `ImageSupport`, `MultiplexSupport`, and `ColorDepth` / `ColorMode` already derive `Serialize`. Verify they have `#[serde(rename_all = "kebab-case")]` or appropriate `#[serde(rename = "...")]` attributes for stable output.

For `TerminalApp` (line 44), add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalApp {
```

But check: `ITerm2` should serialize as `"iterm2"`, `VsCode` as `"vs-code"`, etc. If the existing `Debug` output was used as JSON values by consumers, this is a breaking change. Use per-variant `#[serde(rename = "...")]` instead if needed for backwards compatibility.

- [ ] **Step 2: Replace `format!("{:?}", ...)` in `collect_metadata` with direct serialization**

In `output.rs` `collect_metadata()`, change the fields that use `format!("{:?}", ...)`:

```rust
// Before:
app: format!("{:?}", terminal.app),
color_depth: format!("{:?}", terminal.color_depth),
color_mode: format!("{:?}", Terminal::color_mode()),
image_support: format!("{:?}", terminal.image_support),
char_encoding: format!("{:?}", terminal.char_encoding),

// After (change field types in TerminalMetadata to use the enum types directly):
```

This requires changing the `TerminalMetadata` struct fields from `String` to the actual enum types. For example:

```rust
pub app: TerminalApp,
pub color_depth: ColorDepth,
pub color_mode: ColorMode,
pub image_support: ImageSupport,
```

The `multiplex` field was already simplified to a `String` via `format_multiplex`. With `MultiplexSupport` now being a simple enum that derives `Serialize`, it can be used directly:

```rust
pub multiplex: MultiplexSupport,
```

- [ ] **Step 3: Update `print_pretty` to use `Display` or serialized form**

The `print_pretty` function formats these fields for human display. With enum types instead of strings, use the enum's `Debug` or a custom `Display` impl.

- [ ] **Step 4: Run tests**

Run: `just test -p biscuit-terminal-cli`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/discovery/detection.rs biscuit-terminal/cli/src/output.rs
git commit -m "refactor(biscuit-terminal): use enum types in TerminalMetadata for stable JSON serialization

Replaces Debug-format strings with properly serialized enum values."
```

---

## Phase 4: Final Cleanup

### Task 18: Run full test suite and lint

- [ ] **Step 1: Run lints**

Run: `just lint` from the `biscuit-terminal/` directory.

- [ ] **Step 2: Run all tests**

Run: `just test` from the `biscuit-terminal/` directory.

- [ ] **Step 3: Fix any issues**

Address any compiler warnings, clippy lints, or test failures.

- [ ] **Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore(biscuit-terminal): fix lint and test issues from type safety refactor"
```

---

## Items Deferred (Lower Priority / Higher Risk)

These items from the review are intentionally deferred from this plan:

1. **Item 1.6: Mixed integer types** — Standardizing on `u16` for all width calculations touches deeply nested code across many files. Risk of subtle truncation bugs. Better done as a focused follow-up.

2. **Item 1.9: `extract_color` return type** — The function works correctly. A named struct adds verbosity for a 2-field return. Low ROI.

3. **Item 2.9: BarChart/LineChart shared struct** — Clap derive macros make struct composition awkward with `#[command(flatten)]` on enum variants. The duplication is in arg definitions, not logic. Low ROI.

4. **Item 3.2: Error chain preservation** — Changing `.map_err(|e| eyre!("{}", e))` to `.wrap_err()` requires the source errors to implement `std::error::Error`. Some library errors may only implement `Display`. Needs case-by-case verification.

5. **Items 4.1-4.4: Test coverage** — Adding tests is valuable but orthogonal to the type safety/DRY work. Should be a separate focused effort.
