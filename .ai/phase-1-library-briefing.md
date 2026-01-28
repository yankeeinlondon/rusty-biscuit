# Phase 1 Briefing: Library Development (Subagent 1 - Rust Library Specialist)

**Duration**: 2-3 days | **Owner**: Subagent 1 | **Blocks**: Phases 2 & 3

---

## Overview

Phase 1 completes the `biscuit-terminal` library with full terminal metadata detection. This phase requires implementing three stub functions, enhancing one detection method, and creating a public metadata aggregation API.

**Related Files**:
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/discovery/detection.rs` (679 lines, mostly complete)
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/discovery/eval.rs` (17 lines, mostly stubs)
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/lib.rs` (5 lines, minimal)
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/Cargo.toml`

---

## Task 1.1: Eval Module Implementation (1-1.5 days)

### What Needs to Be Done

Three functions in `eval.rs` need complete implementation:

```rust
pub fn line_widths<T: Into<String>>(content: T) -> Vec<u16>
pub fn has_escape_codes<T: Into<String>>(content: T) -> bool
pub fn has_osc8_link<T: Into<String>>(content: T) -> bool
```

### Function 1: `has_escape_codes(content) -> bool`

**Purpose**: Detect if content contains any ANSI escape sequences (colors, styles, etc.)

**ANSI Sequences to Detect**:
1. **CSI (Control Sequence Introducer)**: `\x1b[` or `\x9b`
   - SGR (Select Graphic Rendition): `\x1b[30;40m` (colors, bold, underline, etc.)
   - Pattern: `\x1b\[[0-9;]*m` (simplified) or use full CSI pattern
   - Examples: `\x1b[31m` (red), `\x1b[1;31;40m` (bold red on black)

2. **OSC (Operating System Command)**: `\x1b]` or `\x9d`
   - OSC 0-2: Set title
   - OSC 8: Hyperlinks (covered by has_osc8_link)
   - Pattern: `\x1b].*?[\x07\x1b\\]` (matches until BEL or ST)
   - Examples: `\x1b]0;Title\x07`, `\x1b]11;rgb:FF/00/00\x1b\\`

3. **Other sequences**:
   - Save/restore cursor: `\x1b[s`, `\x1b[u`
   - Clear screen: `\x1b[2J`
   - Home cursor: `\x1b[H`

**Implementation Strategy**:

Option A: Use regex (simpler, single pattern match)
```rust
pub fn has_escape_codes<T: Into<String>>(content: T) -> bool {
    let content = content.into();
    // Matches both CSI and OSC sequences
    let pattern = regex::Regex::new(r"\x1b[\[\]\]|[\[\]\].*?[\x07\x1b\\]")
        .expect("regex pattern");
    pattern.is_match(&content)
}
```

Option B: Simple byte scanning (no dependencies, more explicit)
```rust
pub fn has_escape_codes<T: Into<String>>(content: T) -> bool {
    let bytes = content.into().into_bytes();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte == 0x1b { // ESC character
            // Check if next char is '[' (CSI), ']' (OSC), or other control
            if i + 1 < bytes.len() {
                let next = bytes[i + 1];
                if matches!(next, b'[' | b']' | 0x9b) {
                    return true;
                }
            }
        }
    }
    false
}
```

**Recommendation**: Use Option B (no new dependencies) with a comment explaining the patterns detected.

**Test Cases**:
```rust
#[test]
fn empty_string_has_no_escape_codes() {
    assert!(!has_escape_codes(""));
}

#[test]
fn plain_text_has_no_escape_codes() {
    assert!(!has_escape_codes("Hello, World!"));
}

#[test]
fn sgr_codes_detected() {
    assert!(has_escape_codes("\x1b[31mRed\x1b[0m"));
}

#[test]
fn osc_title_detected() {
    assert!(has_escape_codes("\x1b]0;Title\x07Some text"));
}

#[test]
fn mixed_text_and_codes() {
    assert!(has_escape_codes("Normal \x1b[1mBold\x1b[0m text"));
}
```

---

### Function 2: `has_osc8_link(content) -> bool`

**Purpose**: Detect if content contains OSC 8 hyperlink sequences (used by terminals to support clickable links)

**OSC 8 Format**:
```
OSC 8 ; params ; URL ST <link text> OSC 8 ; ; ST
\x1b]8;;https://example.com\x1b\\Click me!\x1b]8;;\x1b\\
```

**Breakdown**:
- Opening: `\x1b]8;<params>;<url>\x1b\\` or `\x1b]8;<params>;<url>\x07`
- Closing: `\x1b]8;;\x1b\\` or `\x1b]8;;\x07`
- `<params>`: Key=Value pairs (e.g., `id=xyz123`)
- `<url>`: File path or HTTP URL
- `\x1b\\`: String Terminator (preferred, `ESC \`)
- `\x07`: BEL terminator (alternative, older terminals)

**Implementation Strategy**:

```rust
pub fn has_osc8_link<T: Into<String>>(content: T) -> bool {
    let content = content.into();
    // Pattern: ESC ] 8 ; [params] ; [url] (BEL or ESC \)
    // Simple version: look for the opening sequence
    let pattern = regex::Regex::new(r"\x1b\]8;[^;]*;[^\x07\x1b]*[\x07\x1b]")
        .expect("regex pattern");
    pattern.is_match(&content)
}
```

Or without regex:
```rust
pub fn has_osc8_link<T: Into<String>>(content: T) -> bool {
    let content = content.into();
    // Simple search: look for OSC 8 opening sequence
    if let Some(pos) = content.find("\x1b]8;") {
        // Verify format: should have at least one more `;` and then terminator
        let after = &content[pos..];
        if let Some(semicolon_pos) = after.find(';') {
            let rest = &after[semicolon_pos + 1..];
            // Check for terminator (BEL or ESC \)
            rest.contains('\x07') || rest.contains("\x1b\\")
        } else {
            false
        }
    } else {
        false
    }
}
```

**Test Cases**:
```rust
#[test]
fn empty_string_has_no_osc8() {
    assert!(!has_osc8_link(""));
}

#[test]
fn plain_text_has_no_osc8() {
    assert!(!has_osc8_link("Visit example.com"));
}

#[test]
fn osc8_with_bel_terminator() {
    assert!(has_osc8_link("\x1b]8;;https://example.com\x07Click me!\x1b]8;;\x07"));
}

#[test]
fn osc8_with_st_terminator() {
    assert!(has_osc8_link("\x1b]8;;https://example.com\x1b\\Click me!\x1b]8;;\x1b\\"));
}

#[test]
fn osc8_with_id_parameter() {
    assert!(has_osc8_link("\x1b]8;id=link1;file:///path/to/file\x1b\\"));
}
```

---

### Function 3: `line_widths(content) -> Vec<u16>`

**Purpose**: Calculate the visual width of each line in the content, accounting for ANSI escape codes and Unicode character widths.

**Key Challenges**:
1. Must strip ANSI escape codes before measuring (they're invisible)
2. Must handle Unicode properly:
   - ASCII characters: width 1
   - Most Unicode: width 1-2 (e.g., emoji: width 2)
   - Zero-width characters: width 0 (combining marks, format codes)
   - Fullwidth characters (CJK): width 2
3. Must handle multi-line content (split on `\n` and measure each)

**Implementation Strategy**:

Use the `unicode-width` crate (already a common dependency in terminal libraries):

```rust
pub fn line_widths<T: Into<String>>(content: T) -> Vec<u16> {
    use unicode_width::UnicodeWidthStr;

    let content = content.into();

    // Strip ANSI codes first
    let cleaned = strip_ansi_codes(&content);

    // Split on newlines and measure each line
    cleaned
        .lines()
        .map(|line| {
            UnicodeWidthStr::width(line) as u16
        })
        .collect()
}

/// Helper function to remove ANSI escape sequences from text
fn strip_ansi_codes(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() {
            let next = bytes[i + 1];

            if next == b'[' {
                // CSI sequence - skip until 'm' (SGR terminator)
                i += 2;
                while i < bytes.len() && bytes[i] != b'm' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1; // skip the 'm'
                }
            } else if next == b']' {
                // OSC sequence - skip until BEL or ST
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            } else {
                result.push(bytes[i] as char);
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}
```

**Dependency Addition**:
Add to `Cargo.toml` in dependencies:
```toml
unicode-width = "0.1"
```

**Test Cases**:
```rust
#[test]
fn empty_string_returns_empty_widths() {
    assert_eq!(line_widths(""), vec![]);
}

#[test]
fn single_line_ascii() {
    assert_eq!(line_widths("Hello"), vec![5]);
}

#[test]
fn single_line_with_escape_codes() {
    // "Hello" with red color: \x1b[31mHello\x1b[0m
    let content = "\x1b[31mHello\x1b[0m";
    assert_eq!(line_widths(content), vec![5]);
}

#[test]
fn multiple_lines() {
    assert_eq!(line_widths("Hello\nWorld\nRust"), vec![5, 5, 4]);
}

#[test]
fn unicode_widths() {
    // Most emoji are width 2
    let content = "Hi😀"; // 2 chars but width 4
    // Note: exact width depends on emoji rendering
    let widths = line_widths(content);
    assert_eq!(widths[0], 4); // 'H' (1) + 'i' (1) + '😀' (2)
}

#[test]
fn multiline_with_codes() {
    let content = "\x1b[31mRed\x1b[0m\n\x1b[32mGreen\x1b[0m";
    assert_eq!(line_widths(content), vec![3, 5]);
}
```

---

## Task 1.2: Color Mode Detection Enhancement (1 day)

### Current Implementation

```rust
pub fn color_mode() -> ColorMode {
    ColorMode::Dark // Default for now
}
```

### Enhanced Implementation

Use termbg-style detection or direct OSC 11 querying:

**Strategy**: Query terminal for background color via OSC 11, parse RGB, calculate luminance

```rust
pub fn color_mode() -> ColorMode {
    use std::io::IsTerminal;

    // Skip if not a TTY
    if !std::io::stdout().is_terminal() {
        return ColorMode::Dark;
    }

    // Strategy 1: Check COLORFGBG environment variable (set by some terminals)
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        // Format: "7;0" where first is foreground, second is background
        // If background is dark (0-7), return Dark; if light (8-15), return Light
        if let Some(bg_part) = colorfgbg.split(';').nth(1) {
            if let Ok(bg_num) = bg_part.parse::<u8>() {
                return if bg_num >= 8 { ColorMode::Light } else { ColorMode::Dark };
            }
        }
    }

    // Strategy 2: Try to query terminal directly via OSC 11 (background color)
    // This is more reliable but requires I/O
    if let Ok(color) = query_terminal_background_color() {
        if is_light_color(color) {
            return ColorMode::Light;
        } else {
            return ColorMode::Dark;
        }
    }

    // Fallback
    ColorMode::Dark
}

/// Query the terminal's background color via OSC 11
/// Returns (r, g, b) in 0-255 range
fn query_terminal_background_color() -> Result<(u8, u8, u8), Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    use std::time::Duration;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    // Send OSC 11 query for background color
    // Format: OSC 11 ? ST
    write!(handle, "\x1b]11;?\x07")?;
    handle.flush()?;

    // Read response with timeout (100ms)
    // Response format: OSC 11 ; rgb:RRRR/GGGG/BBBB ST
    // where RRRR is 16-bit hex (e.g., FF00 for red component 255)

    let mut response = vec![0u8; 256];

    // This is tricky without async - we need non-blocking I/O
    // For now, we'll use the termbg crate if available, or skip

    Err("OSC 11 query not implemented".into())
}

/// Determine if an RGB color is "light" based on luminance
fn is_light_color((r, g, b): (u8, u8, u8)) -> bool {
    // Relative luminance formula (from WCAG)
    // L = 0.2126 * R + 0.7152 * G + 0.0722 * B
    // where R, G, B are normalized to 0-1 and adjusted for gamma

    let normalize = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let r = normalize(r);
    let g = normalize(g);
    let b = normalize(b);

    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    // Threshold: 0.5 is middle gray
    luminance > 0.5
}
```

**Alternative: Use termbg crate**

Add to dependencies:
```toml
termbg = "0.5"
```

Then:
```rust
pub fn color_mode() -> ColorMode {
    use std::io::IsTerminal;

    if !std::io::stdout().is_terminal() {
        return ColorMode::Dark;
    }

    // Try termbg first
    match termbg::bg(std::time::Duration::from_millis(100)) {
        Ok(termbg::Color::Rgb(r, g, b)) => {
            if is_light_color((r, g, b)) {
                return ColorMode::Light;
            } else {
                return ColorMode::Dark;
            }
        }
        _ => {} // Fall through to other methods
    }

    // Fallback to COLORFGBG
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg_part) = colorfgbg.split(';').nth(1) {
            if let Ok(bg_num) = bg_part.parse::<u8>() {
                return if bg_num >= 8 { ColorMode::Light } else { ColorMode::Dark };
            }
        }
    }

    ColorMode::Dark
}
```

**Recommendation**: Use termbg for cleaner implementation, as it handles OSC 11 safely with timeout.

**Test Cases**:
```rust
#[test]
fn dark_background_detected() {
    // Mock dark background
    std::env::set_var("COLORFGBG", "7;0");
    assert_eq!(color_mode(), ColorMode::Dark);
    std::env::remove_var("COLORFGBG");
}

#[test]
fn light_background_detected() {
    // Mock light background
    std::env::set_var("COLORFGBG", "0;15");
    assert_eq!(color_mode(), ColorMode::Light);
    std::env::remove_var("COLORFGBG");
}

#[test]
fn luminance_calculation_dark() {
    // Near-black RGB
    assert!(!is_light_color((10, 10, 10)));
}

#[test]
fn luminance_calculation_light() {
    // Near-white RGB
    assert!(is_light_color((245, 245, 245)));
}
```

---

## Task 1.3: Library Exports & Metadata Aggregation (0.5-1 day)

### What Needs to Be Done

1. Create a `TerminalMetadata` struct that aggregates all detection results
2. Update `lib.rs` to properly export all public types
3. Create a prelude module for convenient imports
4. Add comprehensive rustdoc

### Implementation

**Create `discovery/metadata.rs`**:

```rust
//! Terminal metadata aggregation
//!
//! This module provides the [`TerminalMetadata`] struct which represents
//! all available terminal capabilities and settings.

use super::detection::*;
use serde::{Deserialize, Serialize};

/// Complete terminal metadata and capabilities
///
/// This struct aggregates all detection functions into a single,
/// serializable representation of the terminal's capabilities.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::TerminalMetadata;
///
/// let metadata = TerminalMetadata::detect();
/// println!("Terminal: {:?}", metadata.app);
/// println!("Colors: {:?}", metadata.color_depth);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalMetadata {
    /// The detected terminal application
    pub app: TerminalApp,

    /// Color depth support (8-bit, 24-bit, etc.)
    pub color_depth: ColorDepth,

    /// Terminal background color mode (light/dark)
    pub color_mode: ColorMode,

    /// Terminal dimensions in characters (width, height)
    pub dimensions: (u32, u32),

    /// Image protocol support (Kitty, iTerm, none)
    pub image_support: ImageSupport,

    /// Whether the terminal supports OSC 8 hyperlinks
    pub osc8_link_support: bool,

    /// Terminal multiplexing capabilities
    pub multiplex: MultiplexSupport,

    /// Underline style support
    pub underline_support: UnderlineSupport,

    /// Whether the terminal supports italic text
    pub italics_support: bool,

    /// Whether stdout is connected to a TTY
    pub is_tty: bool,
}

impl TerminalMetadata {
    /// Detect and aggregate all terminal metadata
    ///
    /// This method calls all detection functions and combines their results
    /// into a single structure.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use biscuit_terminal::discovery::TerminalMetadata;
    ///
    /// let metadata = TerminalMetadata::detect();
    /// // All fields are now populated with detected values
    /// ```
    #[must_use]
    pub fn detect() -> Self {
        Self {
            app: get_terminal_app(),
            color_depth: color_depth(),
            color_mode: color_mode(),
            dimensions: dimensions(),
            image_support: image_support(),
            osc8_link_support: osc8_link_support(),
            multiplex: multiplex_support(),
            underline_support: underline_support(),
            italics_support: italics_support(),
            is_tty: is_tty(),
        }
    }

    /// Get terminal width in characters
    #[must_use]
    pub fn width(&self) -> u32 {
        self.dimensions.0
    }

    /// Get terminal height in characters
    #[must_use]
    pub fn height(&self)) -> u32 {
        self.dimensions.1
    }
}

impl Default for TerminalMetadata {
    fn default() -> Self {
        Self::detect()
    }
}
```

**Update `discovery/mod.rs`**:

```rust
//! Terminal capability utilities
//!
//! This module provides functions for detecting terminal color support,
//! capabilities, and metadata. It includes:
//!
//! - **Detection functions**: Individual queries for specific capabilities
//! - **Metadata aggregation**: [`TerminalMetadata`] for complete information
//! - **Content evaluation**: Analyzing text for escape codes and widths

pub mod detection;
pub mod eval;
pub mod metadata;

pub use detection::{
    color_depth, color_mode, get_terminal_app, image_support,
    is_tty, multiplex_support, osc8_link_support, terminal_height,
    terminal_width, underline_support, italics_support, dimensions,
    ColorDepth, ColorMode, TerminalApp, ImageSupport, MultiplexSupport,
    UnderlineSupport,
};
pub use metadata::TerminalMetadata;
pub use eval::{has_escape_codes, has_osc8_link, line_widths};
```

**Update `lib.rs`**:

```rust
//! Biscuit Terminal - Terminal utilities and capabilities
//!
//! This library provides utilities for working with terminal emulators,
//! including capability detection, styling, and rendering.
//!
//! ## Discovery
//!
//! Detect terminal capabilities and metadata:
//!
//! ```no_run
//! use biscuit_terminal::discovery::TerminalMetadata;
//!
//! let metadata = TerminalMetadata::detect();
//! println!("Terminal: {:?}", metadata.app);
//! println!("Colors: {:?}", metadata.color_depth);
//! ```
//!
//! ## Evaluation
//!
//! Analyze text content:
//!
//! ```no_run
//! use biscuit_terminal::discovery::eval;
//!
//! let has_codes = eval::has_escape_codes("Hello\x1b[31mWorld");
//! let widths = eval::line_widths("Line 1\nLine 2");
//! ```

pub mod components;
pub mod discovery;
pub mod terminal;
pub mod utils;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::discovery::{
        color_depth, color_mode, get_terminal_app, image_support,
        is_tty, multiplex_support, osc8_link_support, terminal_height,
        terminal_width, underline_support, italics_support, dimensions,
        ColorDepth, ColorMode, TerminalApp, ImageSupport, MultiplexSupport,
        UnderlineSupport, TerminalMetadata,
    };
    pub use crate::discovery::eval::{has_escape_codes, has_osc8_link, line_widths};
}
```

### Documentation Requirements

Add rustdoc to all public items:
- Module documentation explaining purpose
- Function documentation with Examples section
- Enum variant documentation
- Example code blocks that actually compile

**Reference**: CLAUDE.md specifies:
- No explicit H1 headings in `///` docs
- Use H2 for primary sections
- Examples should have `no_run` or be complete and runnable

---

## File Changes Summary

### Files to Create
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/discovery/metadata.rs`

### Files to Modify
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/discovery/eval.rs` - Implement 3 functions
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/discovery/detection.rs` - Enhance color_mode()
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/discovery/mod.rs` - Update exports
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/src/lib.rs` - Add prelude and docs
- `/Volumes/coding/personal/dockhand/biscuit-terminal/lib/Cargo.toml` - Add unicode-width (and optionally termbg)

### Dependency Changes

**Add to Cargo.toml**:
```toml
unicode-width = "0.1"
# Optional, for better color_mode detection:
termbg = "0.5"
```

---

## Testing Requirements

Unit tests for:
1. `has_escape_codes()` - 5+ test cases covering CSI, OSC, mixed
2. `has_osc8_link()` - 4+ test cases covering BEL and ST terminators
3. `line_widths()` - 6+ test cases covering ASCII, Unicode, multi-line
4. `color_mode()` - 4+ test cases covering COLORFGBG, luminance
5. `TerminalMetadata::detect()` - 2+ test cases

**Test Pattern**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(key: &str, value: &str, f: F) {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe { std::env::set_var(key, value); }
        f();
        unsafe { std::env::remove_var(key); }
    }

    #[test]
    fn test_name() {
        // test code
    }
}
```

---

## Validation Checklist

- [ ] All three eval.rs functions implemented
- [ ] color_mode() enhanced with COLORFGBG or OSC 11 detection
- [ ] TerminalMetadata struct created with all fields
- [ ] Library exports updated in lib.rs
- [ ] Prelude module created for convenient imports
- [ ] All public items have rustdoc
- [ ] Rustdoc examples compile (checked with `cargo test --doc`)
- [ ] Unit tests pass: `cargo test -p biscuit-terminal`
- [ ] No clippy warnings: `cargo clippy -p biscuit-terminal`
- [ ] Dependencies added to Cargo.toml
- [ ] Lib builds clean: `cargo check -p biscuit-terminal`

---

## Known Challenges & Solutions

### Challenge 1: Unicode Width Calculation
**Problem**: Some characters (emoji, fullwidth CJK) have width > 1; combining marks have width 0

**Solution**: Use `unicode-width` crate which handles these correctly
- Dependency: `unicode-width = "0.1"`
- Function: `UnicodeWidthStr::width(line)` returns visual width

### Challenge 2: ANSI Code Detection Accuracy
**Problem**: Many variants of escape codes (CSI, OSC, others)

**Solution**: Simple byte scanning for ESC (0x1b) followed by validated markers
- Don't try to parse full sequences, just detect presence
- Keep implementation simple to maintain

### Challenge 3: Color Mode OSC 11 Query
**Problem**: OSC 11 queries require I/O to terminal with timeout

**Solution**: Use `termbg` crate which handles this safely
- Built-in timeout (default 100ms)
- Fallback to COLORFGBG environment variable
- If both fail, default to Dark mode

### Challenge 4: Test Isolation with Environment Variables
**Problem**: Tests modifying env vars can interfere with each other

**Solution**: Use Mutex wrapper for serial execution (from queue-lib pattern)
- Acquire lock before modifying env vars
- Release lock after test completes
- Prevents race conditions in parallel test runs

---

## Success Criteria

Phase 1 is complete when:

- [ ] `cargo test -p biscuit-terminal` passes all tests
- [ ] `cargo check -p biscuit-terminal` shows no errors or warnings
- [ ] `cargo clippy -p biscuit-terminal` shows no warnings
- [ ] `cargo test --doc -p biscuit-terminal` passes all doc examples
- [ ] Library can be imported: `use biscuit_terminal::discovery::*;`
- [ ] TerminalMetadata::detect() aggregates all detection results
- [ ] eval module correctly analyzes text content
- [ ] All public functions have comprehensive rustdoc

---

## Next Steps After Phase 1

Once this phase is complete:
1. **Subagent 2** can begin Phase 2.1 (CLI architecture)
2. **Subagent 3** can begin Phase 3.1 (library unit tests in isolation)
3. CLI will import the library and use TerminalMetadata::detect()
4. Integration tests will verify CLI output matches detected metadata

---

## Resources & References

### Example Implementations
- `/Volumes/coding/personal/dockhand/queue/lib/src/terminal.rs` - TerminalDetector pattern
- `.claude/skills/rust/` - Rust best practices
- `.claude/skills/terminal/` - Terminal detection patterns

### Dependencies Documentation
- `unicode-width`: https://docs.rs/unicode-width/
- `termbg`: https://docs.rs/termbg/
- `termini`: https://docs.rs/termini/

### ANSI Sequences Reference
- CSI SGR: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters
- OSC 8: https://gist.github.com/egmontkob/eb114294efbcd5adc25f#the-escape-sequence
- terminfo: https://man7.org/linux/man-pages/man5/terminfo.5.html

---

**Phase 1 Owner**: Subagent 1 (Rust Library Specialist)
**Estimated Duration**: 2-3 days
**Blocks**: Phases 2 & 3
**Next Briefing**: Phase 2.1 (CLI Architecture) - for Subagent 2
