# Plan: Replace Terminal Crates with biscuit-terminal

**Date:** 2026-03-18
**Scope:** darkmatter/lib — remove 5 direct dependencies by delegating to biscuit-terminal

## Motivation

darkmatter already depends on biscuit-terminal for image rendering and mermaid diagrams, yet it also pulls in 5 separate crates for terminal capabilities that biscuit-terminal already provides. Consolidating reduces dependency count, ensures consistent detection logic across the monorepo, and positions darkmatter to leverage biscuit-terminal's Renderable components and layout system in subsequent work.

## Crates to Remove

| Crate | darkmatter usage | biscuit-terminal replacement |
|---|---|---|
| `terminal_size` | 1 call site | `Terminal::width()` / `terminal_width()` |
| `supports-hyperlinks` | 3 call sites | `Terminal::osc_link_support` / `osc8_link_support()` |
| `termini` | 1 call site | `Terminal::color_depth` (already wraps terminfo) |
| `unicode-width` | 4 files, ~20 call sites | `visible_width()` / direct `unicode_width` re-export |
| `textwrap` | 2 identical functions | `wrap_lines()` with `WordWrap` strategies |

## Prerequisite: biscuit-terminal API Surface

Before starting, verify/add these public APIs in biscuit-terminal:

1. **`visible_width()`** — already public, ANSI-aware, uses `UnicodeWidthChar` internally. Works for darkmatter's terminal rendering needs.
2. **Re-export `unicode_width::UnicodeWidthStr`** — needed for `cleanup.rs` which measures raw markdown text width (no ANSI codes). biscuit-terminal already depends on `unicode-width` internally; add a `pub use` so darkmatter can use it without a direct dependency.
3. **`wrap_lines()`** — already public. Confirm `WordWrap::None` with `break_words` behavior matches `textwrap::Options::new(w).break_words(true)`.

---

## Phase 1: `termini` removal (1 file)

**File:** `darkmatter/lib/src/terminal/supports.rs`

**Current code:**
```rust
use termini::{StringCapability, TermInfo};

pub fn supports_setting_foreground() -> bool {
    match TermInfo::from_env() {
        Ok(term_info) => term_info
            .utf8_string_cap(StringCapability::SetForeground)
            .is_some(),
        Err(_) => false,
    }
}
```

**Replacement approach:**
- biscuit-terminal's `Terminal::color_depth` already queries terminfo for color capabilities
- `ColorDepth::None` or `ColorDepth::Minimal` means no foreground color support; anything else means yes
- Replace with: `Terminal::default().color_depth` check, or call `biscuit_terminal::discovery::color_depth()`
- Alternatively, if `supports_setting_foreground()` is only used to gate ANSI output, check whether callers could use a `Terminal` instance instead

**Action:**
1. Find all callers of `supports_setting_foreground()` — determine if a `Terminal` is already in scope
2. If yes: replace with `terminal.color_depth != ColorDepth::None`
3. If no: use `biscuit_terminal::discovery::color_depth() != ColorDepth::None`
4. Remove `termini` from `Cargo.toml`

---

## Phase 2: `terminal_size` removal (1 file)

**File:** `darkmatter/lib/src/markdown/output/terminal.rs` (line ~788)

**Current code:**
```rust
use terminal_size::{Width, terminal_size};

let terminal_width = options.max_width.unwrap_or_else(|| {
    terminal_size()
        .map(|(Width(w), _)| w)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
});
```

**Replacement approach:**
- The rendering function likely has (or should have) access to a `Terminal` instance
- Use `terminal.width() as u16` or `biscuit_terminal::discovery::terminal_width() as u16`
- The `DEFAULT_TERMINAL_WIDTH` fallback of 80 matches biscuit-terminal's default

**Action:**
1. Check if a `Terminal` is available in the rendering context (likely yes — the same file references `self.terminal`)
2. Replace with `self.terminal.width() as u16` or `options.max_width.unwrap_or_else(|| self.terminal.width() as u16)`
3. Remove `terminal_size` from `Cargo.toml`

---

## Phase 3: `supports-hyperlinks` removal (3 files)

**Files:**
- `darkmatter/lib/src/markdown/output/terminal.rs` — `HyperlinkMode::should_emit_osc8()`
- `darkmatter/lib/src/render/link.rs` — `Link::to_terminal()`
- `darkmatter/lib/src/render/image_ref.rs` — `ImageRef::to_terminal()`

**Current pattern (all 3 sites):**
```rust
supports_hyperlinks::on(Stream::Stdout)
```

**Replacement approach:**
- biscuit-terminal provides `osc8_link_support() -> bool` and `Terminal::osc_link_support`
- For `terminal.rs`: A `Terminal` instance is in scope — use `self.terminal.osc_link_support`
- For `link.rs` and `image_ref.rs`: These are `to_terminal()` methods on data structs without a `Terminal` param

**Action:**
1. **`terminal.rs`**: Replace `supports_hyperlinks::on(Stream::Stdout)` with `self.terminal.osc_link_support` (or pass the flag through `HyperlinkMode`)
2. **`link.rs` and `image_ref.rs`**: Two options:
   - **(a) Add `&Terminal` param** to `to_terminal()` → cleanest, avoids re-detecting per call
   - **(b) Call `biscuit_terminal::discovery::osc8_link_support()`** → simplest change, no signature change
   - Recommend **(a)** since these render methods will likely need `Terminal` for future Renderable migration anyway
3. Update all callers of `Link::to_terminal()` and `ImageRef::to_terminal()` to pass the terminal reference
4. Remove `supports-hyperlinks` from `Cargo.toml`

---

## Phase 4: `unicode-width` removal (4 files, ~20 call sites)

This is the largest change. There are two distinct usage patterns:

### Pattern A: ANSI-aware width (terminal rendering)

**Files:** `markdown/output/terminal.rs`, `diff/visual/side_by_side.rs`, `diff/visual/unified.rs`

These calculate display width of text that may contain ANSI escape codes.

**Replacement:** Use `biscuit_terminal::utils::visible_width()` which already handles ANSI stripping + unicode width. Note the return type is `u32` vs `usize` — may need casts.

### Pattern B: Raw text width (markdown measurement)

**File:** `markdown/cleanup.rs` (3 call sites)

This measures raw markdown text width (no ANSI codes) for line-length heuristics:
```rust
self.width += UnicodeWidthStr::width(text);
self.width += UnicodeWidthStr::width(code) + 2;
url_width: UnicodeWidthStr::width(url),
```

**Replacement:** `visible_width()` would work here too (no ANSI codes in raw markdown), but calling it for plain text adds unnecessary escape-code scanning overhead. Better to re-export `UnicodeWidthStr` from biscuit-terminal.

### Prerequisite

Add to biscuit-terminal's public API:
```rust
// In biscuit-terminal/lib/src/utils/mod.rs or lib.rs
pub use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};
```

### Action:
1. Add `pub use unicode_width::{UnicodeWidthStr, UnicodeWidthChar}` to biscuit-terminal's public API
2. **cleanup.rs**: Change import to `use biscuit_terminal::utils::UnicodeWidthStr`
3. **terminal.rs**: Replace `UnicodeWidthStr::width()` calls with `biscuit_terminal::utils::visible_width()` where ANSI content is possible; use re-exported `UnicodeWidthStr` where it's plain text
4. **side_by_side.rs**: Replace `UnicodeWidthStr` and `UnicodeWidthChar` with biscuit-terminal re-exports; use `visible_width()` for ANSI-aware measurements
5. **unified.rs**: Same as side_by_side
6. Remove `unicode-width` from darkmatter's `Cargo.toml`

---

## Phase 5: `textwrap` removal (2 files)

**Files:** `diff/visual/unified.rs` and `diff/visual/side_by_side.rs`

Both contain an identical `wrap_to_width()` function:
```rust
fn wrap_to_width(s: &str, max_width: usize) -> Vec<String> {
    let options = WrapOptions::new(max_width).break_words(true);
    let wrapped: Vec<String> = wrap(s, options)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect();
    if wrapped.is_empty() { vec![String::new()] } else { wrapped }
}
```

**Replacement approach:**
- biscuit-terminal's `wrap_lines()` takes `Vec<String>`, a `WordWrap` strategy, and a `u32` width
- `WordWrap::None` does hard breaks (no word boundaries) — this is closest to `break_words(true)` but doesn't preserve word boundaries at all
- `WordWrap::WrapProse(search_offset, hanging_indent)` does word-aware wrapping — closer to what `textwrap` does (break at words, fall back to break_words for long words)

**Action:**
1. Verify `WordWrap::WrapProse` behavior matches: wraps at word boundaries, breaks long words that exceed width
2. Replace both `wrap_to_width()` functions with a call to `wrap_lines()`:
   ```rust
   fn wrap_to_width(s: &str, max_width: usize) -> Vec<String> {
       if s.is_empty() || max_width == 0 {
           return vec![String::new()];
       }
       let lines = vec![s.to_string()];
       let result = biscuit_terminal::utils::wrap_lines(
           lines,
           &WordWrap::WrapProse(max_width as u32 / 4, 0),
           max_width as u32,
       );
       if result.is_empty() { vec![String::new()] } else { result }
   }
   ```
3. Run diff tests to verify wrapping behavior is equivalent
4. Remove `textwrap` from `Cargo.toml`

---

## Phase 6: Cleanup and Cargo.toml

1. Remove from `darkmatter/lib/Cargo.toml`:
   ```toml
   termini = "1.0"
   terminal_size = "0.4"
   supports-hyperlinks = "3.2.0"
   unicode-width = "0.2.2"
   textwrap = "0.16"
   ```
2. Run `cargo check -p darkmatter` to verify no remaining references
3. Run `just test` in darkmatter/ to verify all tests pass
4. Run `just lint` to check for warnings

---

## Execution Order and Dependencies

```
Phase 1 (termini)              ─┐
Phase 2 (terminal_size)        ─┤── Independent, can be done in any order
Phase 3 (supports-hyperlinks)  ─┤
Phase 5 (textwrap)             ─┘
         │
Phase 4 (unicode-width)       ← Requires biscuit-terminal re-export first
         │
Phase 6 (cleanup)             ← After all phases complete
```

Phases 1, 2, 3, and 5 are independent of each other. Phase 4 has a prerequisite in biscuit-terminal. Phase 6 is final cleanup.

## Risk Areas

- **`unicode-width` in cleanup.rs**: Measures markdown source text, not terminal output. Using `visible_width()` would work but is slightly wasteful. Re-exporting the trait is cleaner.
- **`textwrap` word-wrapping behavior**: `WordWrap::WrapProse` search offset needs tuning. If wrapping behavior diverges, diff output may look different. Test with multi-byte and long-word inputs.
- **`to_terminal()` signature changes** (Phase 3): Adding `&Terminal` to `Link::to_terminal()` and `ImageRef::to_terminal()` touches callers. Worth doing now to prepare for Renderable migration.
- **Type differences**: biscuit-terminal uses `u32` for widths; darkmatter uses `u16`/`usize` in places. Watch for truncation.

## Test Strategy

- Run `just test` in darkmatter/ after each phase
- Pay special attention to:
  - `diff/visual/` tests — wrapping and width calculations
  - `markdown/output/terminal.rs` tests — rendering width, hyperlinks
  - `markdown/cleanup.rs` tests — line width measurement
  - `render/link.rs` and `render/image_ref.rs` tests — hyperlink output
