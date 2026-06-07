# biscuit-icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `biscuit-icon` library and `icon` CLI — a curated, offline-embedded domain-icon catalog plus on-demand Iconify lookup with a local SQLite cache, rendered through `renderable` (browser/markdown) and `biscuit-terminal` (terminal glyph/image).

**Architecture:** A `lib` crate stores Iconify icon *bodies* (vendored offline for ~150 curated domain icons; cached in SQLite for network lookups) and assembles styled `<svg>` locally. `Icon` implements `renderable::tree::TreeRenderable` (inline SVG for browser/markdown) and `biscuit_terminal`'s `TerminalRenderable` (glyph → image → text ladder). A `cli` crate (`biscuit-icon-cli`, binary `icon`) exposes `icons`/`sets`/`completions`/`cache clear`.

**Tech Stack:** Rust edition 2024, `serde`/`serde_json`, `thiserror`, `strum`, `rusqlite` (bundled), `reqwest` (0.12), `tokio`, `dirs`, `tracing`, `renderable`, `biscuit-terminal`; CLI adds `clap` (derive/env/unstable-ext), `clap_complete` (unstable-dynamic), `color-eyre`, `tracing-subscriber`. Tests: `wiremock`, `tempfile`, `assert_cmd`, `predicates`.

---

## Spec refinements discovered during planning

These deviate from `docs/specs/2026-06-07-biscuit-icon-design.md` based on real-codebase findings; the spec will be synced to match once confirmed:

1. **No `image` cargo feature.** `biscuit-terminal` (a core dependency for terminal rendering via `Prose`) already depends on `resvg` and rasterizes SVG inside `TerminalImage`. Gating image rendering behind a feature would not remove `resvg` from the tree. Image-protocol rendering is therefore a runtime decision (`term.image_support != ImageSupport::None` and no glyph available), not a compile-time feature.
2. **The `iconify` crate is not used.** Our stored-body + local-assembly model needs icon bodies and `viewBox`; the `iconify` proc-macro returns full SVG strings and accepts only literals. The Iconify JSON API (`GET https://api.iconify.design/{prefix}.json?icons={name}`) returns `{ icons: { name: { body, width?, height? } }, width?, height? }`, which is exactly what we store. One `reqwest` client serves both the dev asset-population step and the Phase-2 runtime lookup.

---

## File Structure

```
biscuit-icon/
├── lib/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                  # crate root, re-exports
│   │   ├── error.rs                # IconError (thiserror)
│   │   ├── body.rs                 # IconBody (raw body + viewBox + dims)
│   │   ├── style.rs                # Style + builder + local SVG assembly
│   │   ├── glyph.rs                # Glyph (unicode + nerd font)
│   │   ├── icon.rs                 # Icon struct, constructors
│   │   ├── domain/
│   │   │   ├── mod.rs              # DomainIcon trait + set re-exports
│   │   │   ├── os.rs               # Os enum (worked example)
│   │   │   ├── emoji.rs … social.rs
│   │   │   └── generated.rs       # GENERATED: enum-variant → body map (committed)
│   │   ├── iconify/
│   │   │   ├── mod.rs
│   │   │   └── client.rs          # JSON API client (fetch body / collections)
│   │   ├── cache/
│   │   │   ├── mod.rs
│   │   │   └── store.rs           # rusqlite schema + queries
│   │   └── render.rs               # TreeRenderable + TerminalRenderable impls
│   ├── src/bin/
│   │   └── populate_assets.rs      # dev-only: fetch curated bodies → assets + generated.rs
│   └── tests/                      # integration tests
├── cli/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── args.rs
│       └── commands.rs
├── assets/icons/<set>/<name>.svg   # GENERATED + committed (icon bodies)
├── docs/
├── justfile
├── just.md
└── README.md                       # already exists
```

---

## Phase 0 — Scaffold

### Task 1: Create the library crate skeleton

**Files:**
- Create: `biscuit-icon/lib/Cargo.toml`
- Create: `biscuit-icon/lib/src/lib.rs`
- Modify: `Cargo.toml` (workspace root members array)

- [ ] **Step 1: Register the crates in the workspace**

In `/Users/ken/.claudine/worktrees/rusty-biscuit/icon/Cargo.toml`, insert into `members` immediately after `"biscuit-hash/lib",` (keeps alphabetical order):

```toml
    "biscuit-icon/cli",
    "biscuit-icon/lib",
```

- [ ] **Step 2: Write the library Cargo.toml**

Create `biscuit-icon/lib/Cargo.toml`:

```toml
[package]
name = "biscuit-icon"
version = "0.1.0"
edition = "2024"

[lib]
name = "biscuit_icon"
path = "src/lib.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
strum = "0.27"
strum_macros = "0.27"
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
dirs = "6"
tracing = "0.1"
renderable = { path = "../../renderable" }
biscuit-terminal = { path = "../../biscuit-terminal/lib" }

[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "test-util"] }
```

- [ ] **Step 3: Write the crate root**

Create `biscuit-icon/lib/src/lib.rs`:

```rust
//! Curated, offline-embedded domain icons plus on-demand Iconify lookup.
//!
//! Domain icons are accessed enum-first (e.g. [`domain::Os::Finder`]) with a
//! fallible string convenience layer ([`Icon::os`]). Any of the 200,000+
//! Iconify icons can be fetched at runtime via [`Icon::iconify`] and cached to
//! a local SQLite database.

pub mod body;
pub mod cache;
pub mod domain;
pub mod error;
pub mod glyph;
pub mod icon;
pub mod iconify;
pub mod render;
pub mod style;

pub use body::IconBody;
pub use error::IconError;
pub use glyph::Glyph;
pub use icon::{Icon, Source};
pub use style::Style;
```

- [ ] **Step 4: Verify it builds (will fail until modules exist — that's expected; create empty module stubs)**

Create empty stubs so the crate compiles: each of `body.rs`, `cache/mod.rs`, `domain/mod.rs`, `error.rs`, `glyph.rs`, `icon.rs`, `iconify/mod.rs`, `render.rs`, `style.rs`. For now put a single line in each that will be replaced by later tasks, e.g. in `body.rs`:

```rust
// Replaced in Task 4.
```

And in `lib.rs` temporarily comment out `pub use` lines whose types don't exist yet — or, simpler, implement Task 3 (error) and Task 4 (body) before first compile. Proceed to Task 2 and Task 3; do the first `cargo build -p biscuit-icon` at the end of Task 4.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml biscuit-icon/lib/Cargo.toml biscuit-icon/lib/src/lib.rs
git commit -m "chore(biscuit-icon): scaffold library crate"
```

### Task 2: Area justfile and just.md

**Files:**
- Create: `biscuit-icon/justfile`
- Create: `biscuit-icon/just.md`

- [ ] **Step 1: Write the justfile**

Create `biscuit-icon/justfile` (mirrors `biscuit-hash/justfile`, two packages):

```just
set dotenv-load
set positional-arguments
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

import "../just/lifecycle.just"
import "../just/plan.just"
import "../just/review.just"
import "../just/notify.just"
import "../just/ai.just"
import "../just/devops.just"

BOLD := '\033[1m'
DIM := '\033[2m'
ITALIC := '\033[3m'
RESET := '\033[0m'
RED := '\033[31m'

default:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v md &> /dev/null; then
        md just.md
    fi
    echo "biscuit-icon Library & CLI"
    echo "=========================="
    echo
    just --list | grep -v 'default'

build *args="":
    @just _build biscuit-icon "Biscuit Icon Library" {{args}}
    @just _build biscuit-icon-cli "Biscuit Icon CLI" {{args}}

sanity:
    @just _sanity biscuit-icon
    @just _sanity biscuit-icon-cli

test *args="":
    @just _test biscuit-icon {{args}}
    @just _test biscuit-icon-cli {{args}}

test-l2:
    @echo "test-l2: not applicable for biscuit-icon"

test-l3:
    @echo "test-l3: not applicable for biscuit-icon"

test-browser:
    @echo "test-browser: not applicable for biscuit-icon"

test-real:
    @echo "test-real: not applicable for biscuit-icon"

lint:
    @just _lint biscuit-icon
    @just _lint biscuit-icon-cli

coverage *args="":
    @just _coverage biscuit-icon {{args}}
    @just _coverage biscuit-icon-cli {{args}}

doctest *args="":
    @just _doctest biscuit-icon {{args}}
    @just _doctest biscuit-icon-cli {{args}}

fuzz:
    @echo "fuzz: not applicable for biscuit-icon"

all:
    @just sanity
    @just lint
    @just doctest
    @just test
    @just test-l2
    @just test-browser

lint-fix:
    @just _lint biscuit-icon || just _fix_lint biscuit-icon
    @just _lint biscuit-icon-cli || just _fix_lint biscuit-icon-cli

install *args="":
    @just build --release
    @just _install "./cli" "Biscuit Icon" {{args}}

docs:
    @cargo doc -p biscuit-icon --open

cli *args="":
    @cargo run -p biscuit-icon-cli -- {{args}}

# Dev-only: fetch curated icon bodies from the Iconify API into assets + generated.rs.
populate-assets *args="":
    @cargo run -p biscuit-icon --bin populate_assets -- {{args}}

_colors:
    @: {{BOLD}} {{ITALIC}} {{RESET}} {{RED}} {{DIM}} >/dev/null || true
```

- [ ] **Step 2: Write just.md**

Create `biscuit-icon/just.md`:

```markdown
# biscuit-icon

Curated offline domain icons + on-demand Iconify lookup. Library `biscuit-icon`, CLI `icon`.
```

- [ ] **Step 3: Verify just lists recipes**

Run: `cd biscuit-icon && just --list`
Expected: recipes `build`, `test`, `sanity`, `lint`, `populate-assets`, etc. listed (no error).

- [ ] **Step 4: Commit**

```bash
git add biscuit-icon/justfile biscuit-icon/just.md
git commit -m "chore(biscuit-icon): add area justfile"
```

### Task 3: Error type

**Files:**
- Create/replace: `biscuit-icon/lib/src/error.rs`

- [ ] **Step 1: Write the failing test**

Append to `biscuit-icon/lib/src/error.rs`:

```rust
//! Library error type.

use thiserror::Error;

/// Errors produced by `biscuit-icon`.
#[derive(Debug, Error)]
pub enum IconError {
    /// A string did not match any variant of the named domain set.
    #[error("unknown {set} icon: {name}")]
    UnknownDomainIcon {
        /// The domain set name, e.g. "os".
        set: &'static str,
        /// The unmatched icon name.
        name: String,
    },

    /// An Iconify identifier was not in `prefix:name` form.
    #[error("invalid iconify identifier: {0}")]
    InvalidIdentifier(String),

    /// The requested Iconify icon was not found upstream.
    #[error("iconify icon not found: {0}")]
    NotFound(String),

    /// A network request to the Iconify API failed.
    #[error("iconify fetch failed: {0}")]
    Fetch(String),

    /// A cache (SQLite) operation failed.
    #[error("cache error: {0}")]
    Cache(String),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, IconError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_domain_icon_renders_set_and_name() {
        let err = IconError::UnknownDomainIcon { set: "os", name: "frobnicate".into() };
        assert_eq!(err.to_string(), "unknown os icon: frobnicate");
    }

    #[test]
    fn invalid_identifier_renders_input() {
        let err = IconError::InvalidIdentifier("mdihome".into());
        assert_eq!(err.to_string(), "invalid iconify identifier: mdihome");
    }
}
```

- [ ] **Step 2: Run the test (expect fail to compile first, then pass)**

Run: `cargo test -p biscuit-icon error::tests -- --nocapture`
Expected: compiles and PASSES (the test is self-contained).

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/lib/src/error.rs
git commit -m "feat(biscuit-icon): add IconError"
```

---

## Phase 1 — Offline library

### Task 4: IconBody

**Files:**
- Create/replace: `biscuit-icon/lib/src/body.rs`

- [ ] **Step 1: Write the failing test + type**

Replace `biscuit-icon/lib/src/body.rs`:

```rust
//! The raw Iconify icon body plus geometry needed to assemble an `<svg>`.

use serde::{Deserialize, Serialize};

/// An Iconify icon body (the inner markup, e.g. `<path .../>`) and the
/// geometry required to wrap it in a complete `<svg>` element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconBody {
    /// Inner SVG markup (paths, groups), without the surrounding `<svg>`.
    pub body: String,
    /// Intrinsic width of the icon's coordinate system.
    pub width: u32,
    /// Intrinsic height of the icon's coordinate system.
    pub height: u32,
}

impl IconBody {
    /// Builds a body with an explicit coordinate system.
    #[must_use]
    pub fn new(body: impl Into<String>, width: u32, height: u32) -> Self {
        Self { body: body.into(), width, height }
    }

    /// The `viewBox` string, `"0 0 {width} {height}"`.
    #[must_use]
    pub fn view_box(&self) -> String {
        format!("0 0 {} {}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_box_uses_intrinsic_dimensions() {
        let body = IconBody::new("<path d=\"M0 0h24v24H0z\"/>", 24, 24);
        assert_eq!(body.view_box(), "0 0 24 24");
    }
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p biscuit-icon body::tests`
Expected: PASS.

- [ ] **Step 3: First full crate build (modules now exist enough)**

Temporarily ensure `lib.rs` only `pub use`s types defined so far. Set the remaining module files to empty (`// stub`) and comment the corresponding `pub use` lines in `lib.rs`. Then:

Run: `cargo build -p biscuit-icon`
Expected: builds (warnings about unused modules are fine).

- [ ] **Step 4: Commit**

```bash
git add biscuit-icon/lib/src/body.rs biscuit-icon/lib/src/lib.rs
git commit -m "feat(biscuit-icon): add IconBody"
```

### Task 5: Style and local SVG assembly

**Files:**
- Create/replace: `biscuit-icon/lib/src/style.rs`

- [ ] **Step 1: Write the failing tests + implementation**

Replace `biscuit-icon/lib/src/style.rs`:

```rust
//! Styling options applied to an icon, and the local `<svg>` assembler.

use crate::body::IconBody;

/// Accumulated presentation options. All default to "unset"; an unset option
/// means the corresponding SVG attribute is omitted (Iconify defaults apply).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    /// CSS color applied via the `color` style (drives `currentColor`).
    pub color: Option<String>,
    /// SVG width attribute (default `1em` when `None`).
    pub width: Option<String>,
    /// SVG height attribute (default `1em` when `None`).
    pub height: Option<String>,
    /// `horizontal`, `vertical`, or `both`.
    pub flip: Option<String>,
    /// `90`, `180`, or `270`.
    pub rotate: Option<String>,
    /// When true, emit a transparent bounding-box rect spanning the viewBox.
    pub view_box: bool,
}

impl Style {
    /// Builds the SVG `transform` value for the configured flip/rotate, if any.
    ///
    /// Returns `None` when neither flip nor rotate is set.
    fn transform(&self, body: &IconBody) -> Option<String> {
        let (w, h) = (f64::from(body.width), f64::from(body.height));
        let mut parts: Vec<String> = Vec::new();
        match self.flip.as_deref() {
            Some("horizontal") => parts.push(format!("translate({w} 0) scale(-1 1)")),
            Some("vertical") => parts.push(format!("translate(0 {h}) scale(1 -1)")),
            Some("both") => parts.push(format!("translate({w} {h}) scale(-1 -1)")),
            _ => {}
        }
        match self.rotate.as_deref() {
            Some("90") => parts.push(format!("rotate(90 {} {})", w / 2.0, h / 2.0)),
            Some("180") => parts.push(format!("rotate(180 {} {})", w / 2.0, h / 2.0)),
            Some("270") => parts.push(format!("rotate(270 {} {})", w / 2.0, h / 2.0)),
            _ => {}
        }
        if parts.is_empty() { None } else { Some(parts.join(" ")) }
    }

    /// Assembles a complete `<svg>` string from an icon body and this style.
    #[must_use]
    pub fn assemble(&self, body: &IconBody) -> String {
        let width = self.width.as_deref().unwrap_or("1em");
        let height = self.height.as_deref().unwrap_or("1em");
        let color_style = self
            .color
            .as_deref()
            .map(|c| format!(" style=\"color: {c}\""))
            .unwrap_or_default();

        let inner = match self.transform(body) {
            Some(t) => format!("<g transform=\"{t}\">{}</g>", body.body),
            None => body.body.clone(),
        };
        let view_rect = if self.view_box {
            format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"none\"/>",
                body.width, body.height
            )
        } else {
            String::new()
        };

        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
             viewBox=\"{vb}\"{color_style}>{view_rect}{inner}</svg>",
            vb = body.view_box(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> IconBody {
        IconBody::new("<path d=\"M0 0\"/>", 24, 24)
    }

    #[test]
    fn defaults_emit_1em_and_viewbox() {
        let svg = Style::default().assemble(&body());
        assert!(svg.contains("width=\"1em\""));
        assert!(svg.contains("height=\"1em\""));
        assert!(svg.contains("viewBox=\"0 0 24 24\""));
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn explicit_size_and_color_applied() {
        let style = Style { width: Some("32".into()), height: Some("32".into()), color: Some("#d97706".into()), ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("width=\"32\""));
        assert!(svg.contains("style=\"color: #d97706\""));
    }

    #[test]
    fn rotate_wraps_body_in_transform_group() {
        let style = Style { rotate: Some("90".into()), ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("<g transform=\"rotate(90 12 12)\">"));
    }

    #[test]
    fn flip_horizontal_emits_scale() {
        let style = Style { flip: Some("horizontal".into()), ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("translate(24 0) scale(-1 1)"));
    }

    #[test]
    fn view_box_flag_emits_transparent_rect() {
        let style = Style { view_box: true, ..Style::default() };
        let svg = style.assemble(&body());
        assert!(svg.contains("<rect width=\"24\" height=\"24\" fill=\"none\"/>"));
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p biscuit-icon style::tests`
Expected: all 5 PASS.

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/lib/src/style.rs
git commit -m "feat(biscuit-icon): add Style and local SVG assembly"
```

### Task 6: Glyph

**Files:**
- Create/replace: `biscuit-icon/lib/src/glyph.rs`

- [ ] **Step 1: Write the failing test + type**

Replace `biscuit-icon/lib/src/glyph.rs`:

```rust
//! Character representations for the curated subset of icons.

/// A character representation of an icon: an optional plain Unicode codepoint
/// and an optional Nerd Font private-use codepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// A standard Unicode character, if a faithful one exists.
    pub unicode: Option<char>,
    /// A Nerd Font (private use area) character, if mapped.
    pub nerd_font: Option<char>,
}

impl Glyph {
    /// A glyph with only a Unicode character.
    #[must_use]
    pub const fn unicode(c: char) -> Self {
        Self { unicode: Some(c), nerd_font: None }
    }

    /// A glyph with only a Nerd Font character.
    #[must_use]
    pub const fn nerd(c: char) -> Self {
        Self { unicode: None, nerd_font: Some(c) }
    }

    /// A glyph with both representations.
    #[must_use]
    pub const fn both(unicode: char, nerd_font: char) -> Self {
        Self { unicode: Some(unicode), nerd_font: Some(nerd_font) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sets_each_representation() {
        let g = Glyph::both('\u{1F600}', '\u{f118}');
        assert_eq!(g.unicode, Some('\u{1F600}'));
        assert_eq!(g.nerd_font, Some('\u{f118}'));
    }
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p biscuit-icon glyph::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/lib/src/glyph.rs
git commit -m "feat(biscuit-icon): add Glyph"
```

### Task 7: DomainIcon trait + the `Os` worked example + generated-map mechanism

This task establishes the exact pattern every domain enum follows. Task 10 replicates it for the remaining sets using the README variant lists.

**Files:**
- Replace: `biscuit-icon/lib/src/domain/mod.rs`
- Create: `biscuit-icon/lib/src/domain/os.rs`
- Create: `biscuit-icon/lib/src/domain/generated.rs` (hand-written placeholder map; regenerated in Task 9)

- [ ] **Step 1: Write the DomainIcon trait**

Replace `biscuit-icon/lib/src/domain/mod.rs`:

```rust
//! Curated domain icon sets, accessed enum-first.

mod os;
pub use os::Os;

pub(crate) mod generated;

use crate::body::IconBody;
use crate::glyph::Glyph;
use crate::icon::Icon;

/// Common behavior for every curated domain-icon enum.
pub trait DomainIcon: Copy {
    /// The upstream Iconify identifier, e.g. `"hugeicons:apple-finder"`.
    fn iconify_id(self) -> &'static str;

    /// The embedded icon body for this variant.
    fn body(self) -> IconBody;

    /// The character representation, if this icon defines one.
    fn glyph(self) -> Option<Glyph> {
        None
    }

    /// Builds an [`Icon`] for this domain variant.
    fn icon(self) -> Icon {
        Icon::from_domain(self.body(), self.glyph())
    }
}
```

- [ ] **Step 2: Write the `Os` enum (worked example)**

Create `biscuit-icon/lib/src/domain/os.rs`. The `body()` impl reads from the generated map keyed by the Iconify id:

```rust
use strum_macros::{Display, EnumIter, EnumString};

use crate::body::IconBody;
use crate::domain::DomainIcon;
use crate::domain::generated::body_for;

/// Operating-system and platform icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Os {
    Finder,
    AppStore,
    Windows,
    Linux,
    MacOs,
    Apple,
}

impl DomainIcon for Os {
    fn iconify_id(self) -> &'static str {
        match self {
            Os::Finder => "hugeicons:apple-finder",
            Os::AppStore => "ri:app-store-fill",
            Os::Windows => "whh:windowseight",
            Os::Linux => "ant-design:linux-outlined",
            Os::MacOs => "f7:logo-macos",
            Os::Apple => "ic:baseline-apple",
        }
    }

    fn body(self) -> IconBody {
        body_for(self.iconify_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn string_round_trip_uses_snake_case() {
        assert_eq!(Os::from_str("app_store").unwrap(), Os::AppStore);
        assert_eq!(Os::MacOs.to_string(), "mac_os");
    }

    #[test]
    fn every_variant_has_an_iconify_id() {
        for variant in Os::iter() {
            assert!(variant.iconify_id().contains(':'), "{variant:?} id must be prefix:name");
        }
    }
}
```

- [ ] **Step 3: Write a temporary `generated.rs` so tests compile before assets exist**

Create `biscuit-icon/lib/src/domain/generated.rs`. Task 9 overwrites this with real bodies; for now provide a panic-free placeholder so unit tests of *naming* compile, and a real lookup once assets land:

```rust
//! GENERATED by `cargo run -p biscuit-icon --bin populate_assets`.
//! Do not edit by hand. Maps Iconify ids to embedded bodies.

use crate::body::IconBody;

/// Returns the embedded body for an Iconify id.
///
/// # Panics
/// Panics if `id` is not a curated domain icon (a programmer error: every
/// `DomainIcon::iconify_id` must be present in this generated map).
#[must_use]
pub fn body_for(id: &str) -> IconBody {
    match id {
        // Placeholder until `populate_assets` runs (Task 9). A 1x1 empty body
        // keeps naming/round-trip tests compiling without network access.
        _ => IconBody::new(format!("<!-- {id} -->"), 24, 24),
    }
}
```

- [ ] **Step 4: Wire `lib.rs` to export `domain` and run naming tests**

Ensure `lib.rs` has `pub mod domain;`. Run: `cargo test -p biscuit-icon os::tests`
Expected: both `Os` tests PASS (they assert names, not bodies).

- [ ] **Step 5: Commit**

```bash
git add biscuit-icon/lib/src/domain/
git commit -m "feat(biscuit-icon): add DomainIcon trait and Os set"
```

### Task 8: Iconify JSON client

Shared by the dev asset pipeline (Task 9) and runtime lookup (Phase 2).

**Files:**
- Replace: `biscuit-icon/lib/src/iconify/mod.rs`
- Create: `biscuit-icon/lib/src/iconify/client.rs`

- [ ] **Step 1: Write the client with a configurable base URL (for wiremock)**

Replace `biscuit-icon/lib/src/iconify/mod.rs`:

```rust
//! Client for the public Iconify HTTP API.

mod client;
pub use client::{IconifyClient, parse_id};
```

Create `biscuit-icon/lib/src/iconify/client.rs`:

```rust
use serde::Deserialize;

use crate::body::IconBody;
use crate::error::{IconError, Result};

const DEFAULT_BASE: &str = "https://api.iconify.design";

/// A thin async client over the Iconify JSON API.
#[derive(Debug, Clone)]
pub struct IconifyClient {
    http: reqwest::Client,
    base: String,
}

/// Splits a `prefix:name` identifier into its parts.
///
/// # Errors
/// Returns [`IconError::InvalidIdentifier`] when there is not exactly one `:`
/// with non-empty parts on both sides.
pub fn parse_id(id: &str) -> Result<(String, String)> {
    match id.split_once(':') {
        Some((p, n)) if !p.is_empty() && !n.is_empty() => Ok((p.to_string(), n.to_string())),
        _ => Err(IconError::InvalidIdentifier(id.to_string())),
    }
}

#[derive(Deserialize)]
struct CollectionResponse {
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    icons: std::collections::HashMap<String, IconEntry>,
}

#[derive(Deserialize)]
struct IconEntry {
    body: String,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

impl IconifyClient {
    /// Builds a client targeting the public Iconify API.
    #[must_use]
    pub fn new() -> Self {
        Self { http: reqwest::Client::new(), base: DEFAULT_BASE.to_string() }
    }

    /// Builds a client targeting a custom base URL (used in tests).
    #[must_use]
    pub fn with_base(base: impl Into<String>) -> Self {
        Self { http: reqwest::Client::new(), base: base.into() }
    }

    /// Fetches a single icon body by `prefix:name`.
    ///
    /// # Errors
    /// - [`IconError::InvalidIdentifier`] for malformed ids.
    /// - [`IconError::Fetch`] on transport/HTTP failure.
    /// - [`IconError::NotFound`] when the icon is absent from the response.
    pub async fn fetch_body(&self, id: &str) -> Result<IconBody> {
        let (prefix, name) = parse_id(id)?;
        let url = format!("{}/{}.json?icons={}", self.base, prefix, name);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(IconError::Fetch(format!("HTTP {}", resp.status())));
        }
        let data: CollectionResponse = resp
            .json()
            .await
            .map_err(|e| IconError::Fetch(e.to_string()))?;
        let entry = data.icons.get(&name).ok_or_else(|| IconError::NotFound(id.to_string()))?;
        let width = entry.width.or(data.width).unwrap_or(16);
        let height = entry.height.or(data.height).unwrap_or(16);
        Ok(IconBody::new(entry.body.clone(), width, height))
    }
}

impl Default for IconifyClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn parse_id_rejects_missing_colon() {
        assert!(matches!(parse_id("mdihome"), Err(IconError::InvalidIdentifier(_))));
    }

    #[test]
    fn parse_id_accepts_prefix_name() {
        assert_eq!(parse_id("mdi:home").unwrap(), ("mdi".into(), "home".into()));
    }

    #[tokio::test]
    async fn fetch_body_parses_collection_response() {
        let server = MockServer::start().await;
        let json = serde_json::json!({
            "prefix": "mdi",
            "width": 24,
            "height": 24,
            "icons": { "home": { "body": "<path d=\"M0 0\"/>" } }
        });
        Mock::given(method("GET"))
            .and(path("/mdi.json"))
            .and(query_param("icons", "home"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;

        let client = IconifyClient::with_base(server.uri());
        let body = client.fetch_body("mdi:home").await.unwrap();
        assert_eq!(body.body, "<path d=\"M0 0\"/>");
        assert_eq!(body.width, 24);
    }

    #[tokio::test]
    async fn fetch_body_missing_icon_is_not_found() {
        let server = MockServer::start().await;
        let json = serde_json::json!({ "prefix": "mdi", "icons": {} });
        Mock::given(method("GET"))
            .and(path("/mdi.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;
        let client = IconifyClient::with_base(server.uri());
        assert!(matches!(client.fetch_body("mdi:ghost").await, Err(IconError::NotFound(_))));
    }
}
```

- [ ] **Step 2: Run the client tests**

Run: `cargo test -p biscuit-icon iconify::`
Expected: 4 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/lib/src/iconify/
git commit -m "feat(biscuit-icon): add Iconify JSON client"
```

### Task 9: Dev asset-population binary + vendor curated bodies

**Files:**
- Create: `biscuit-icon/lib/src/bin/populate_assets.rs`
- Generated/committed: `biscuit-icon/assets/icons/**` and `biscuit-icon/lib/src/domain/generated.rs`

- [ ] **Step 1: Write the populate binary**

Create `biscuit-icon/lib/src/bin/populate_assets.rs`. It reads the full curated id list (collected from all domain enums via `DomainIcon::iconify_id`), fetches each body, writes `assets/icons/<prefix>/<name>.svg` (body only), and regenerates `generated.rs` as a `match` over ids embedding each body via `include_str!`.

```rust
//! Dev-only: fetch curated icon bodies from the Iconify API and regenerate
//! the committed asset files + `domain/generated.rs`. Run via
//! `just populate-assets`. NOT compiled into the shipped library beyond being
//! a bin target; it performs network I/O and must be run manually.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use biscuit_icon::iconify::{IconifyClient, parse_id};

/// The complete curated id list. Source of truth: the README domain sets.
/// Extend this as domain enums are added (Task 10).
fn curated_ids() -> BTreeSet<&'static str> {
    // Start with the Os set; Task 10 appends the rest.
    [
        "hugeicons:apple-finder",
        "ri:app-store-fill",
        "whh:windowseight",
        "ant-design:linux-outlined",
        "f7:logo-macos",
        "ic:baseline-apple",
    ]
    .into_iter()
    .collect()
}

#[tokio::main]
async fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // biscuit-icon/lib
    let assets_root = manifest.join("../assets/icons");
    let client = IconifyClient::new();

    let mut arms: Vec<String> = Vec::new();
    for id in curated_ids() {
        let (prefix, name) = parse_id(id).expect("curated ids are well-formed");
        let body = client.fetch_body(id).await.unwrap_or_else(|e| panic!("fetch {id}: {e}"));
        let dir = assets_root.join(&prefix);
        fs::create_dir_all(&dir).expect("create asset dir");
        let file = dir.join(format!("{name}.svg"));
        fs::write(&file, &body.body).expect("write body");
        let rel = format!("../../../assets/icons/{prefix}/{name}.svg");
        arms.push(format!(
            "        {id:?} => IconBody::new(include_str!({rel:?}), {w}, {h}),",
            w = body.width,
            h = body.height,
        ));
        eprintln!("vendored {id} -> {}", file.display());
    }

    let generated = format!(
        "//! GENERATED by `cargo run -p biscuit-icon --bin populate_assets`.\n\
         //! Do not edit by hand. Maps Iconify ids to embedded bodies.\n\n\
         use crate::body::IconBody;\n\n\
         /// Returns the embedded body for an Iconify id.\n\
         ///\n/// # Panics\n/// Panics if `id` is not a curated domain icon.\n\
         #[must_use]\n\
         pub fn body_for(id: &str) -> IconBody {{\n\
         \x20   match id {{\n{arms}\n\
         \x20       other => panic!(\"missing curated body for {{other}}\"),\n\
         \x20   }}\n}}\n",
        arms = arms.join("\n"),
    );
    fs::write(manifest.join("src/domain/generated.rs"), generated).expect("write generated.rs");
    eprintln!("regenerated generated.rs with {} arms", curated_ids().len());
}
```

Note: `lib.rs` must `pub mod iconify;` so the bin can use `biscuit_icon::iconify`.

- [ ] **Step 2: Run the populate binary (requires network — a manual dev action)**

Run: `just -d biscuit-icon populate-assets` (or `cargo run -p biscuit-icon --bin populate_assets`)
Expected: prints `vendored hugeicons:apple-finder -> …` lines and `regenerated generated.rs with 6 arms`. Files appear under `biscuit-icon/assets/icons/`.

- [ ] **Step 3: Verify the real bodies compile and the Os icon assembles**

Add an integration test `biscuit-icon/lib/tests/embedded.rs`:

```rust
use biscuit_icon::domain::{DomainIcon, Os};

#[test]
fn finder_assembles_a_real_svg() {
    let svg = Os::Finder.icon().svg();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("viewBox="));
    // The placeholder body was a comment; a real body has markup.
    assert!(!svg.contains("<!-- hugeicons:apple-finder -->"));
}
```

Run: `cargo test -p biscuit-icon --test embedded`
Expected: PASS (depends on Task 11's `Icon::svg`; if running tasks in order, defer this test's authoring to after Task 11 and instead `cargo build -p biscuit-icon` here to confirm `generated.rs` compiles).

- [ ] **Step 4: Commit the vendored assets**

```bash
git add biscuit-icon/assets/icons biscuit-icon/lib/src/domain/generated.rs biscuit-icon/lib/src/bin/populate_assets.rs
git commit -m "feat(biscuit-icon): vendor Os icon bodies and asset pipeline"
```

### Task 10: Remaining domain enums

Apply the **exact pattern from Task 7's `Os`** to every remaining set. The variant list, PascalCase names, and Iconify ids are specified in `README.md` (the authoritative source list). Where the README gives an id in parentheses, use it; where it does not (e.g. several `Emoji`, `Arrow`, `Hardware` entries), pick the closest Iconify id from the Iconify browser and record it in the enum's `iconify_id` match — these choices are the implementer's to make and must be real `prefix:name` ids.

Sets to add (one file each under `domain/`, re-exported from `domain/mod.rs`): `Emoji`, `Arrow`, `Data`, `File`, `Hardware`, `Timing`, `Button`, `Control`, `Network`, `DevOps`, `Actors`, `Nav`, `Sport`, `Brand`, `Social`.

- [ ] **Step 1: For each set, create `domain/<set>.rs` mirroring `os.rs`**

For each set: define the enum with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]` + `#[strum(serialize_all = "snake_case")]`, implement `DomainIcon` (`iconify_id` match + `body` via `body_for`), and add the two tests from `os.rs` (snake_case round-trip on one variant; every-variant-has-colon-id loop). Add `mod <set>; pub use <set>::<Set>;` to `domain/mod.rs`.

For sets whose README entries define character representations (notably `Emoji`, and any others you choose to map), override `glyph()`. Example for `Emoji`:

```rust
fn glyph(self) -> Option<crate::glyph::Glyph> {
    use crate::glyph::Glyph;
    Some(match self {
        Emoji::Happy => Glyph::unicode('\u{1F600}'),
        Emoji::Sad => Glyph::unicode('\u{1F622}'),
        Emoji::Laughing => Glyph::unicode('\u{1F606}'),
        Emoji::Angry => Glyph::unicode('\u{1F620}'),
        Emoji::Surprised => Glyph::unicode('\u{1F632}'),
    })
}
```

Handle non-identifier names per strum rules: names starting with a digit (e.g. network `3G`, `4G`, `5G`) cannot be bare PascalCase variants — name them `ThreeG`, `FourG`, `FiveG` and set `#[strum(serialize = "3g")]` on each so the string lookup uses the natural token. `LTE` stays `Lte`.

- [ ] **Step 2: Extend `curated_ids()` in `populate_assets.rs`**

Add every new set's ids to the `curated_ids()` set (one literal per icon), matching the `iconify_id` matches exactly.

- [ ] **Step 3: Re-run the asset pipeline and rebuild**

Run: `just -d biscuit-icon populate-assets`
Then: `cargo build -p biscuit-icon`
Expected: all curated bodies fetched; `generated.rs` regenerated; crate compiles.

- [ ] **Step 4: Run all domain tests**

Run: `cargo test -p biscuit-icon domain::`
Expected: every set's naming tests PASS.

- [ ] **Step 5: Commit**

```bash
git add biscuit-icon/lib/src/domain biscuit-icon/assets/icons biscuit-icon/lib/src/bin/populate_assets.rs
git commit -m "feat(biscuit-icon): add remaining domain sets and vendor bodies"
```

### Task 11: Icon struct and constructors

**Files:**
- Replace: `biscuit-icon/lib/src/icon.rs`

- [ ] **Step 1: Write tests + implementation**

Replace `biscuit-icon/lib/src/icon.rs`:

```rust
//! The central `Icon` handle.

use std::str::FromStr;

use crate::body::IconBody;
use crate::error::{IconError, Result};
use crate::glyph::Glyph;
use crate::style::Style;

/// Where an icon's body came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A compiled-in curated domain icon.
    Embedded,
    /// A network-fetched / cache-resident Iconify icon.
    Network,
}

/// A renderable icon: a body plus accumulated style and optional glyph.
#[derive(Debug, Clone)]
pub struct Icon {
    pub(crate) body: IconBody,
    pub(crate) glyph: Option<Glyph>,
    pub(crate) source: Source,
    pub(crate) style: Style,
}

impl Icon {
    /// Builds an icon from an embedded domain body.
    #[must_use]
    pub(crate) fn from_domain(body: IconBody, glyph: Option<Glyph>) -> Self {
        Self { body, glyph, source: Source::Embedded, style: Style::default() }
    }

    /// Builds an icon from a network/cache body.
    #[must_use]
    pub(crate) fn from_network(body: IconBody) -> Self {
        Self { body, glyph: None, source: Source::Network, style: Style::default() }
    }

    /// The body's provenance.
    #[must_use]
    pub fn source(&self) -> Source {
        self.source
    }

    /// The raw body.
    #[must_use]
    pub fn body(&self) -> &IconBody {
        &self.body
    }

    /// The Unicode character, if this icon has one.
    #[must_use]
    pub fn unicode_char(&self) -> Option<char> {
        self.glyph.and_then(|g| g.unicode)
    }

    /// The Nerd Font character, if this icon has one.
    #[must_use]
    pub fn nerd_font_char(&self) -> Option<char> {
        self.glyph.and_then(|g| g.nerd_font)
    }

    /// Assembles the styled `<svg>` markup.
    #[must_use]
    pub fn svg(&self) -> String {
        self.style.assemble(&self.body)
    }

    // --- styling builders ---

    /// Sets the CSS color (drives `currentColor` for monochrome icons).
    #[must_use]
    pub fn color(mut self, c: impl Into<String>) -> Self {
        self.style.color = Some(c.into());
        self
    }

    /// Sets the SVG width.
    #[must_use]
    pub fn width(mut self, w: impl Into<String>) -> Self {
        self.style.width = Some(w.into());
        self
    }

    /// Sets the SVG height.
    #[must_use]
    pub fn height(mut self, h: impl Into<String>) -> Self {
        self.style.height = Some(h.into());
        self
    }

    /// Flips the icon: `horizontal`, `vertical`, or `both`.
    #[must_use]
    pub fn flip(mut self, f: impl Into<String>) -> Self {
        self.style.flip = Some(f.into());
        self
    }

    /// Rotates the icon: `90`, `180`, or `270`.
    #[must_use]
    pub fn rotate(mut self, r: impl Into<String>) -> Self {
        self.style.rotate = Some(r.into());
        self
    }

    /// Toggles the transparent bounding-box rect.
    #[must_use]
    pub fn view_box(mut self, on: bool) -> Self {
        self.style.view_box = on;
        self
    }
}

/// Generates a string-convenience constructor for a domain set.
macro_rules! domain_ctor {
    ($fn_name:ident, $enum:ty, $set:literal) => {
        impl Icon {
            #[doc = concat!("Looks up a `", $set, "` icon by its snake_case name.")]
            ///
            /// # Errors
            /// Returns [`IconError::UnknownDomainIcon`] when the name is unknown.
            pub fn $fn_name(name: &str) -> Result<Icon> {
                use crate::domain::DomainIcon;
                <$enum>::from_str(name)
                    .map(DomainIcon::icon)
                    .map_err(|_| IconError::UnknownDomainIcon { set: $set, name: name.to_string() })
            }
        }
    };
}

domain_ctor!(os, crate::domain::Os, "os");
// Task 10's sets get one line each here, e.g.:
// domain_ctor!(emoji, crate::domain::Emoji, "emoji");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainIcon, Os};

    #[test]
    fn builder_threads_style_into_svg() {
        let svg = Os::Apple.icon().color("red").width("48").svg();
        assert!(svg.contains("width=\"48\""));
        assert!(svg.contains("style=\"color: red\""));
    }

    #[test]
    fn string_ctor_unknown_name_errors() {
        let err = Icon::os("nope").unwrap_err();
        assert!(matches!(err, IconError::UnknownDomainIcon { set: "os", .. }));
    }

    #[test]
    fn string_ctor_known_name_succeeds() {
        assert!(Icon::os("finder").is_ok());
    }
}
```

- [ ] **Step 2: Add a `domain_ctor!` line for every Task-10 set**

Below the `domain_ctor!(os, …)` line add one per set: `emoji`, `arrow`, `data`, `file`, `hardware`, `timing`, `button`, `control`, `network`, `dev_ops`, `actors`, `nav`, `sport`, `brand`, `social` (matching the enum type and a lowercase set label string).

- [ ] **Step 3: Run the tests**

Run: `cargo test -p biscuit-icon icon::tests`
Expected: 3 PASS.

- [ ] **Step 4: Author the deferred embedded integration test from Task 9 Step 3 now**

Add `biscuit-icon/lib/tests/embedded.rs` (from Task 9 Step 3). Run: `cargo test -p biscuit-icon --test embedded`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add biscuit-icon/lib/src/icon.rs biscuit-icon/lib/tests/embedded.rs
git commit -m "feat(biscuit-icon): add Icon handle, builders, and string constructors"
```

### Task 12: TreeRenderable (browser / markdown via inline SVG)

**Files:**
- Replace: `biscuit-icon/lib/src/render.rs`

- [ ] **Step 1: Write tests + impl**

Replace `biscuit-icon/lib/src/render.rs`:

```rust
//! Multi-target rendering for [`Icon`].
//!
//! Browser/markdown targets emit the assembled SVG as a raw inline-HTML node.
//! Terminal rendering (glyph → image → text) lives in [`Icon::render_terminal`].

use renderable::tree::{RenderNode, TreeRenderable};

use crate::icon::Icon;

impl TreeRenderable for Icon {
    /// Projects the icon into a single inline raw-HTML node carrying the SVG.
    fn render_tree(&self) -> RenderNode {
        RenderNode::root(vec![RenderNode::html(self.svg(), false)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainIcon, Os};
    use renderable::tree::render::{BrowserRenderOptions, RawHtmlPolicy, render_browser_node};

    #[test]
    fn browser_target_emits_inline_svg_verbatim() {
        let icon = Os::Apple.icon();
        let node = icon.render_tree();
        let opts = BrowserRenderOptions { raw_html: RawHtmlPolicy::Allow, ..Default::default() };
        let rendered = render_browser_node(&node, &opts).unwrap();
        let html = rendered.output.to_string();
        assert!(html.contains("<svg"));
    }
}
```

Note: confirm the exact way to stringify `Rendered<BrowserFragment<Ready>>` (the explore report shows `render_browser_node -> Rendered<BrowserFragment<Ready>>`; `.output` is the fragment). If `BrowserFragment` does not expose `to_string()`, use `render_browser_document_html` with a single-node document instead — the implementer should pick whichever compiles, asserting the SVG substring is present.

- [ ] **Step 2: Run the test**

Run: `cargo test -p biscuit-icon render::tests::browser_target_emits_inline_svg_verbatim`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/lib/src/render.rs
git commit -m "feat(biscuit-icon): implement TreeRenderable for Icon"
```

### Task 13: Terminal rendering (glyph → image → text ladder)

**Files:**
- Modify: `biscuit-icon/lib/src/render.rs`

- [ ] **Step 1: Add the terminal render method + tests**

Append to `biscuit-icon/lib/src/render.rs`:

```rust
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;

/// Controls whether Nerd Font glyphs are eligible during terminal rendering.
///
/// Nerd Font presence is not reliably detectable, so the caller opts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NerdFontMode {
    /// Never emit Nerd Font glyphs.
    #[default]
    Off,
    /// Prefer a Nerd Font glyph when the icon has one.
    On,
}

impl Icon {
    /// Renders the icon to terminal escape output using the degradation ladder:
    /// Nerd Font glyph (when `nerd == On`) → Unicode glyph → image protocol
    /// (when the terminal supports one) → text fallback (the body's first id-ish
    /// token, here the SVG is unavailable so we emit a placeholder box).
    #[must_use]
    pub fn render_terminal(&self, term: &Terminal, nerd: NerdFontMode) -> String {
        if nerd == NerdFontMode::On {
            if let Some(c) = self.nerd_font_char() {
                return Prose::new(c.to_string()).render(term);
            }
        }
        if let Some(c) = self.unicode_char() {
            return Prose::new(c.to_string()).render(term);
        }
        if term.image_support != ImageSupport::None {
            if let Ok(s) = self.render_image(term) {
                return s;
            }
        }
        // Text fallback: a small bracketed placeholder.
        Prose::new("[icon]").render(term)
    }

    /// Rasterizes the assembled SVG to a temp file and renders it via the
    /// terminal's image protocol.
    fn render_image(&self, term: &Terminal) -> std::io::Result<String> {
        use std::io::Write;
        let mut file = tempfile::Builder::new().suffix(".svg").tempfile()?;
        file.write_all(self.svg().as_bytes())?;
        let img = TerminalImage::new(file.path())
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(img.render(term))
    }
}
```

Add `tempfile` to the library's **non-dev** dependencies (it is now needed at runtime for image rendering): in `biscuit-icon/lib/Cargo.toml` `[dependencies]` add `tempfile = "3"`.

- [ ] **Step 2: Write the glyph-priority tests**

Append to the `tests` module in `render.rs`:

```rust
#[test]
fn unicode_glyph_takes_priority_over_image() {
    use crate::domain::{DomainIcon, Emoji};
    let term = Terminal::new_optimistic(80);
    let out = Emoji::Happy.icon().render_terminal(&term, super::NerdFontMode::Off);
    assert!(out.contains('\u{1F600}'));
}

#[test]
fn glyphless_icon_without_image_support_uses_text_fallback() {
    use crate::domain::{DomainIcon, Os};
    // new_optimistic enables capabilities but image_support depends on detection;
    // construct a default (no image) terminal for a deterministic text fallback.
    let term = Terminal::default();
    let out = Os::Finder.icon().render_terminal(&term, super::NerdFontMode::Off);
    // Finder has no glyph; with no image support the fallback text is emitted.
    if term.image_support == ImageSupport::None {
        assert!(out.contains("[icon]"));
    }
}
```

Note: `Emoji` must exist (Task 10) with the `Happy` glyph override. If executing strictly in order and Task 10 is complete, this compiles. The `new_optimistic` image-support value may vary; the test asserts the glyph branch which does not depend on image support.

- [ ] **Step 3: Run the terminal tests**

Run: `cargo test -p biscuit-icon render::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add biscuit-icon/lib/src/render.rs biscuit-icon/lib/Cargo.toml
git commit -m "feat(biscuit-icon): add terminal glyph/image/text render ladder"
```

---

## Phase 2 — Network cache

### Task 14: SQLite cache store

**Files:**
- Replace: `biscuit-icon/lib/src/cache/mod.rs`
- Create: `biscuit-icon/lib/src/cache/store.rs`

- [ ] **Step 1: Write store + tests**

Replace `biscuit-icon/lib/src/cache/mod.rs`:

```rust
//! On-disk SQLite cache of network-fetched Iconify icons.

mod store;
pub use store::IconCache;
```

Create `biscuit-icon/lib/src/cache/store.rs`:

```rust
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::body::IconBody;
use crate::error::{IconError, Result};

/// A SQLite-backed cache of fetched Iconify icon bodies and set metadata.
pub struct IconCache {
    conn: Connection,
}

fn map_sql<E: std::fmt::Display>(e: E) -> IconError {
    IconError::Cache(e.to_string())
}

impl IconCache {
    /// Opens (creating if needed) the cache at the default user location
    /// `~/.cache/biscuit-icon/icons.db`.
    ///
    /// # Errors
    /// [`IconError::Cache`] if the directory or database cannot be created.
    pub fn open_default() -> Result<Self> {
        let dir = Self::default_dir()?;
        std::fs::create_dir_all(&dir).map_err(map_sql)?;
        Self::open_at(dir.join("icons.db"))
    }

    /// Opens (creating if needed) a cache at an explicit path (used in tests).
    ///
    /// # Errors
    /// [`IconError::Cache`] on connection or schema failure.
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).map_err(map_sql)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS icons (
                prefix     TEXT NOT NULL,
                name       TEXT NOT NULL,
                body       TEXT NOT NULL,
                width      INTEGER NOT NULL,
                height     INTEGER NOT NULL,
                fetched_at TEXT NOT NULL,
                PRIMARY KEY (prefix, name)
            );
            CREATE INDEX IF NOT EXISTS idx_icons_name ON icons(name);
            "#,
        )
        .map_err(map_sql)?;
        Ok(Self { conn })
    }

    fn default_dir() -> Result<PathBuf> {
        let base = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
        } else {
            dirs::home_dir().ok_or_else(|| IconError::Cache("no home dir".into()))?
        };
        Ok(base.join(".cache").join("biscuit-icon"))
    }

    /// Looks up a cached body by prefix and name.
    ///
    /// # Errors
    /// [`IconError::Cache`] on query failure.
    pub fn get(&self, prefix: &str, name: &str) -> Result<Option<IconBody>> {
        self.conn
            .query_row(
                "SELECT body, width, height FROM icons WHERE prefix = ?1 AND name = ?2",
                params![prefix, name],
                |row| Ok(IconBody::new(row.get::<_, String>(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(map_sql)
    }

    /// Inserts or replaces a cached body.
    ///
    /// # Errors
    /// [`IconError::Cache`] on write failure.
    pub fn put(&self, prefix: &str, name: &str, body: &IconBody) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO icons (prefix, name, body, width, height, fetched_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
                params![prefix, name, body.body, body.width, body.height],
            )
            .map(|_| ())
            .map_err(map_sql)
    }

    /// Returns cached `prefix:name` ids whose name contains `needle` (for completions).
    ///
    /// # Errors
    /// [`IconError::Cache`] on query failure.
    pub fn search_names(&self, needle: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT prefix, name FROM icons WHERE name LIKE ?1 ORDER BY prefix, name")
            .map_err(map_sql)?;
        let like = format!("%{needle}%");
        let rows = stmt
            .query_map(params![like], |row| {
                Ok(format!("{}:{}", row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(map_sql)
    }

    /// Deletes all cached rows.
    ///
    /// # Errors
    /// [`IconError::Cache`] on delete failure.
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM icons", []).map(|_| ()).map_err(map_sql)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache() -> (tempfile::TempDir, IconCache) {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::open_at(dir.path().join("icons.db")).unwrap();
        (dir, cache)
    }

    #[test]
    fn put_then_get_round_trips() {
        let (_d, cache) = temp_cache();
        let body = IconBody::new("<path/>", 24, 24);
        cache.put("mdi", "home", &body).unwrap();
        assert_eq!(cache.get("mdi", "home").unwrap(), Some(body));
    }

    #[test]
    fn get_miss_is_none() {
        let (_d, cache) = temp_cache();
        assert_eq!(cache.get("mdi", "ghost").unwrap(), None);
    }

    #[test]
    fn search_names_matches_substring() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
        cache.put("mdi", "home-outline", &IconBody::new("<b/>", 24, 24)).unwrap();
        cache.put("mdi", "alert", &IconBody::new("<c/>", 24, 24)).unwrap();
        let hits = cache.search_names("home").unwrap();
        assert_eq!(hits, vec!["mdi:home", "mdi:home-outline"]);
    }

    #[test]
    fn clear_empties_the_cache() {
        let (_d, cache) = temp_cache();
        cache.put("mdi", "home", &IconBody::new("<a/>", 24, 24)).unwrap();
        cache.clear().unwrap();
        assert_eq!(cache.get("mdi", "home").unwrap(), None);
    }
}
```

Add `tempfile` is already a dev-dependency (Task 1). Confirm `tempfile` is in `[dev-dependencies]` (used here) and also `[dependencies]` (added in Task 13).

- [ ] **Step 2: Run cache tests**

Run: `cargo test -p biscuit-icon cache::`
Expected: 4 PASS.

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/lib/src/cache/
git commit -m "feat(biscuit-icon): add SQLite icon cache"
```

### Task 15: `Icon::iconify` cache-first lookup

**Files:**
- Modify: `biscuit-icon/lib/src/icon.rs`

- [ ] **Step 1: Add the async lookup + a test**

Append to `impl Icon` in `icon.rs`:

```rust
    /// Fetches an Iconify icon by `prefix:name`, consulting the local cache
    /// first and falling back to the network (then caching the result).
    ///
    /// # Errors
    /// - [`IconError::InvalidIdentifier`] for malformed ids.
    /// - [`IconError::NotFound`] / [`IconError::Fetch`] on lookup failure.
    /// - [`IconError::Cache`] on cache failure.
    pub async fn iconify(id: &str) -> Result<Icon> {
        let cache = crate::cache::IconCache::open_default()?;
        let client = crate::iconify::IconifyClient::new();
        Self::iconify_with(id, &cache, &client).await
    }

    /// Cache-first lookup against an explicit cache and client (used in tests).
    ///
    /// # Errors
    /// See [`Icon::iconify`].
    pub async fn iconify_with(
        id: &str,
        cache: &crate::cache::IconCache,
        client: &crate::iconify::IconifyClient,
    ) -> Result<Icon> {
        let (prefix, name) = crate::iconify::parse_id(id)?;
        if let Some(body) = cache.get(&prefix, &name)? {
            return Ok(Icon::from_network(body));
        }
        let body = client.fetch_body(id).await?;
        cache.put(&prefix, &name, &body)?;
        Ok(Icon::from_network(body))
    }
```

- [ ] **Step 2: Add an integration test**

Create `biscuit-icon/lib/tests/iconify_lookup.rs`:

```rust
use biscuit_icon::cache::IconCache;
use biscuit_icon::iconify::IconifyClient;
use biscuit_icon::Icon;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn miss_fetches_then_hit_uses_cache() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "prefix": "mdi", "width": 24, "height": 24,
        "icons": { "home": { "body": "<path d=\"M1 1\"/>" } }
    });
    // Respond at most once: the second lookup must hit the cache, not the network.
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cache = IconCache::open_at(dir.path().join("c.db")).unwrap();
    let client = IconifyClient::with_base(server.uri());

    let first = Icon::iconify_with("mdi:home", &cache, &client).await.unwrap();
    assert!(first.svg().contains("M1 1"));
    let second = Icon::iconify_with("mdi:home", &cache, &client).await.unwrap();
    assert!(second.svg().contains("M1 1"));
    // `expect(1)` is verified on server drop: exactly one network call occurred.
}
```

- [ ] **Step 3: Run it**

Run: `cargo test -p biscuit-icon --test iconify_lookup`
Expected: PASS (one network call total).

- [ ] **Step 4: Commit**

```bash
git add biscuit-icon/lib/src/icon.rs biscuit-icon/lib/tests/iconify_lookup.rs
git commit -m "feat(biscuit-icon): add cache-first Icon::iconify lookup"
```

### Task 16: Collections (sets) listing

**Files:**
- Modify: `biscuit-icon/lib/src/iconify/client.rs`

- [ ] **Step 1: Add `fetch_collections` + test**

Add to `IconifyClient` in `client.rs`:

```rust
    /// Fetches the list of Iconify set prefixes, each with its human title.
    ///
    /// # Errors
    /// [`IconError::Fetch`] on transport/HTTP/parse failure.
    pub async fn fetch_collections(&self) -> Result<Vec<(String, String)>> {
        let url = format!("{}/collections", self.base);
        let resp = self.http.get(&url).send().await.map_err(|e| IconError::Fetch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(IconError::Fetch(format!("HTTP {}", resp.status())));
        }
        let map: std::collections::BTreeMap<String, CollectionMeta> =
            resp.json().await.map_err(|e| IconError::Fetch(e.to_string()))?;
        Ok(map.into_iter().map(|(prefix, meta)| (prefix, meta.name)).collect())
    }
```

And add near the other response structs:

```rust
#[derive(Deserialize)]
struct CollectionMeta {
    #[serde(default)]
    name: String,
}
```

- [ ] **Step 2: Add the test**

Append to the `tests` module in `client.rs`:

```rust
    #[tokio::test]
    async fn fetch_collections_lists_prefixes_and_titles() {
        let server = MockServer::start().await;
        let json = serde_json::json!({
            "mdi": { "name": "Material Design Icons" },
            "lucide": { "name": "Lucide" }
        });
        Mock::given(method("GET"))
            .and(path("/collections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;
        let client = IconifyClient::with_base(server.uri());
        let sets = client.fetch_collections().await.unwrap();
        assert!(sets.contains(&("mdi".into(), "Material Design Icons".into())));
        assert!(sets.contains(&("lucide".into(), "Lucide".into())));
    }
```

- [ ] **Step 3: Run it**

Run: `cargo test -p biscuit-icon iconify::client::tests::fetch_collections_lists_prefixes_and_titles`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add biscuit-icon/lib/src/iconify/client.rs
git commit -m "feat(biscuit-icon): add Iconify collections listing"
```

---

## Phase 3 — CLI

### Task 17: CLI skeleton + args

**Files:**
- Create: `biscuit-icon/cli/Cargo.toml`
- Create: `biscuit-icon/cli/src/main.rs`
- Create: `biscuit-icon/cli/src/args.rs`
- Create: `biscuit-icon/cli/src/commands.rs`

- [ ] **Step 1: Write the CLI Cargo.toml**

Create `biscuit-icon/cli/Cargo.toml`:

```toml
[package]
name = "biscuit-icon-cli"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "icon"
path = "src/main.rs"

[dependencies]
biscuit-icon = { path = "../lib" }
biscuit-terminal = { path = "../../biscuit-terminal/lib" }
clap = { version = "4.5", features = ["derive", "env", "unstable-ext", "wrap_help"] }
clap_complete = { version = "4.5", features = ["unstable-dynamic"] }
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 2: Write args.rs**

Create `biscuit-icon/cli/src/args.rs`:

```rust
use clap::{Parser, Subcommand};

/// Curated domain icons + on-demand Iconify lookup.
#[derive(Parser, Debug)]
#[command(name = "icon", version, about, long_about = None)]
pub struct Cli {
    /// Increase diagnostic verbosity on stderr (-v, -vv, -vvv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Default `icons` filter when no subcommand is given (e.g. `icon mdi:home`).
    #[arg(value_name = "FILTER")]
    pub filter: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List icons whose name matches FILTER (rendered visually).
    Icons {
        /// Substring or `prefix:name` to match.
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
        /// Limit to these sets (comma-separated prefixes), e.g. `fa,mdi`.
        #[arg(long, value_name = "CSV")]
        from: Option<String>,
    },
    /// List Iconify set names, optionally filtered.
    Sets {
        /// Substring to match against set prefixes/titles.
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },
    /// Cache maintenance.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Generate dynamic shell completions.
    Completions {
        /// Target shell.
        #[arg(value_name = "SHELL")]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Delete all cached icons.
    Clear,
}
```

- [ ] **Step 3: Write commands.rs and main.rs (dispatch + error rendering)**

Create `biscuit-icon/cli/src/commands.rs`:

```rust
use biscuit_icon::Icon;
use biscuit_icon::cache::IconCache;
use biscuit_icon::iconify::IconifyClient;
use biscuit_icon::render::NerdFontMode;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};

use crate::args::{CacheAction, Commands};

/// Runs the resolved command.
pub async fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Icons { filter, from } => icons(filter, from).await,
        Commands::Sets { filter } => sets(filter).await,
        Commands::Cache { action: CacheAction::Clear } => {
            IconCache::open_default()?.clear()?;
            println!("cache cleared");
            Ok(())
        }
        Commands::Completions { .. } => Ok(()), // handled in main before dispatch
    }
}

async fn icons(filter: Option<String>, _from: Option<String>) -> Result<()> {
    let term = Terminal::new();
    let needle = filter.unwrap_or_default();
    // A `prefix:name` filter is a direct lookup+render.
    if needle.contains(':') {
        let icon = Icon::iconify(&needle).await?;
        println!("{}  {needle}", icon.render_terminal(&term, NerdFontMode::Off));
        return Ok(());
    }
    // Otherwise list matching cached icons.
    let cache = IconCache::open_default()?;
    let hits = cache.search_names(&needle)?;
    if hits.is_empty() {
        return Err(eyre!("no cached icons match {needle:?}; try `icon <prefix:name>` to fetch"));
    }
    for id in hits {
        let icon = Icon::iconify(&id).await?;
        println!("{}  {id}", icon.render_terminal(&term, NerdFontMode::Off));
    }
    Ok(())
}

async fn sets(filter: Option<String>) -> Result<()> {
    let client = IconifyClient::new();
    let needle = filter.unwrap_or_default().to_lowercase();
    for (prefix, title) in client.fetch_collections().await? {
        if needle.is_empty()
            || prefix.to_lowercase().contains(&needle)
            || title.to_lowercase().contains(&needle)
        {
            println!("{prefix}\t{title}");
        }
    }
    Ok(())
}
```

Create `biscuit-icon/cli/src/main.rs`:

```rust
mod args;
mod commands;

use clap::{CommandFactory, Parser};
use clap_complete::aot::generate;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use args::{Cli, Commands};

#[tokio::main]
async fn main() {
    color_eyre::install().ok();
    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "icon", &mut std::io::stdout());
        return;
    }

    init_tracing(cli.verbose);

    // Resolve the default `icons` command when none is given.
    let command = cli.command.unwrap_or(Commands::Icons { filter: cli.filter, from: None });

    if let Err(err) = commands::run(command).await {
        eprintln!("\x1b[31m\x1b[1mError:\x1b[0m {err}");
        std::process::exit(1);
    }
}

fn init_tracing(verbose: u8) {
    let explicit = std::env::var("RUST_LOG").ok();
    if verbose == 0 && explicit.is_none() {
        return;
    }
    let base = explicit.unwrap_or_else(|| match verbose {
        1 => "warn,biscuit_icon=info,icon=info".into(),
        2 => "info,biscuit_icon=debug,icon=debug".into(),
        _ => "debug,biscuit_icon=trace,icon=trace".into(),
    });
    let filter = EnvFilter::try_new(&base).unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).compact())
        .init();
}
```

Note: `render::NerdFontMode` must be re-exported. Ensure `biscuit-icon/lib/src/lib.rs` has `pub mod render;` (already) and that `NerdFontMode` is `pub` in `render.rs` (it is). Add `pub use render::NerdFontMode;` to `lib.rs` if you prefer the shorter path; the CLI above uses `biscuit_icon::render::NerdFontMode`.

- [ ] **Step 4: Build the CLI**

Run: `cargo build -p biscuit-icon-cli`
Expected: builds. Confirm the `clap_complete::aot::generate` path compiles; if the `unstable-dynamic` feature relocates `generate`, use `clap_complete::generate` instead.

- [ ] **Step 5: Commit**

```bash
git add biscuit-icon/cli
git commit -m "feat(biscuit-icon-cli): scaffold icon CLI with args and dispatch"
```

### Task 18: `sets` command test

**Files:**
- Create: `biscuit-icon/cli/tests/cli.rs`

- [ ] **Step 1: Write a help/parse smoke test (no network)**

Create `biscuit-icon/cli/tests/cli.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("icon")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("icons"))
        .stdout(predicate::str::contains("sets"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn cache_clear_succeeds_with_isolated_home() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("icon")
        .unwrap()
        .env("HOME", home.path())
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache cleared"));
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p biscuit-icon-cli --test cli`
Expected: both PASS (`cache clear` uses an isolated `HOME`, so it creates a fresh empty DB and clears it).

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/cli/tests/cli.rs
git commit -m "test(biscuit-icon-cli): add help and cache-clear smoke tests"
```

### Task 19: Dynamic completions for icon names

**Files:**
- Modify: `biscuit-icon/cli/src/args.rs` (add a dynamic completer to the `Icons` filter)

- [ ] **Step 1: Add a dynamic value completer that queries built-ins + cache**

In `args.rs`, attach an `add = ArgValueCompleter` to the `Icons.filter` arg using `clap_complete::engine`. Implement a completer that yields built-in set prefixes and cached `prefix:name` ids:

```rust
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

fn icon_name_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let needle = current.to_string_lossy();
    let mut out = Vec::new();
    // Cached icon names (best-effort; ignore errors during completion).
    if let Ok(cache) = biscuit_icon::cache::IconCache::open_default() {
        if let Ok(hits) = cache.search_names(&needle) {
            out.extend(hits.into_iter().map(CompletionCandidate::new));
        }
    }
    out
}
```

And on the `Icons.filter` field:

```rust
        #[arg(value_name = "FILTER", add = ArgValueCompleter::new(icon_name_completer))]
        filter: Option<String>,
```

- [ ] **Step 2: Build and smoke-test completion generation**

Run: `cargo build -p biscuit-icon-cli`
Then add to `cli/tests/cli.rs`:

```rust
#[test]
fn completions_bash_emits_script() {
    Command::cargo_bin("icon")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("icon"));
}
```

Run: `cargo test -p biscuit-icon-cli --test cli`
Expected: PASS. If the dynamic-completion API surface differs (the `unstable-dynamic` feature evolves), fall back to static `clap_complete::generate` for the `completions` subcommand and keep the cache-querying completer behind whatever entrypoint the installed `clap_complete` exposes — the success criterion is: `icon completions <shell>` emits a script, and cached names are offered during dynamic completion.

- [ ] **Step 3: Commit**

```bash
git add biscuit-icon/cli/src/args.rs biscuit-icon/cli/tests/cli.rs
git commit -m "feat(biscuit-icon-cli): dynamic completions querying the cache"
```

### Task 20: Docs drift + dependencies + final gate

**Files:**
- Modify: `biscuit-icon/README.md` (sync the realized API surface)
- Create: `biscuit-icon/docs/dependencies.md`
- Modify: `docs/dependencies.md` (root) — add biscuit-icon's notable crates

- [ ] **Step 1: Update README to match the implemented API**

Reconcile `README.md` with reality: the `Icon::device("mobile_phone")`/`Icon::emoji("happy")` examples become the per-set string constructors (`Icon::os("finder")`, `Icon::emoji("happy")`), enum-first usage shown first (`Os::Finder.icon()`), and the styling table preserved. Note the offline-embedded domain catalog, the SQLite cache location, and the no-`iconify`-crate / JSON-API approach.

- [ ] **Step 2: Write biscuit-icon/docs/dependencies.md**

List the notable crates and why: `renderable` (multi-target tree), `biscuit-terminal` (terminal render + resvg rasterization), `rusqlite` bundled (cache), `reqwest` (Iconify JSON API), `strum` (enum↔string), `clap`/`clap_complete` (CLI). Note `resvg` arrives transitively via `biscuit-terminal`.

- [ ] **Step 3: Run the full PR gate**

Run: `cd biscuit-icon && just all`
Expected: `sanity`, `lint`, `doctest`, `test` all pass (the `test-l2`/`test-browser` recipes print "not applicable").

- [ ] **Step 4: Commit**

```bash
git add biscuit-icon/README.md biscuit-icon/docs/dependencies.md docs/dependencies.md
git commit -m "docs(biscuit-icon): sync README and dependencies"
```

---

## Self-Review

**Spec coverage check (against `docs/specs/2026-06-07-biscuit-icon-design.md`):**

- Goals: offline domain catalog (Tasks 7,9,10) ✓; on-demand lookup + cache (Tasks 14,15) ✓; uniform local styling (Task 5) ✓; renderable tree + browser (Task 12) ✓; terminal ladder (Task 13) ✓; CLI sets/icons/completions/cache (Tasks 17–20) ✓.
- Enum-first + string convenience API (Tasks 7,11) ✓.
- Local SVG assembly from stored body (Task 5) ✓.
- SQLite via rusqlite bundled, no expiry, `cache clear` (Tasks 14,17) ✓.
- Phasing (Phase 0–3 sections) ✓.
- `IconError` variants (Task 3) ✓.

**Deviations (flagged at top):** dropped `image` cargo feature; dropped `iconify` crate. Spec to be synced post-confirmation.

**Open implementer judgment calls (called out inline, not placeholders):**
- Exact stringification of the browser `Rendered` fragment (Task 12) — pick whichever of `.output.to_string()` / `render_browser_document_html` compiles; success = SVG substring present.
- `clap_complete` dynamic-completion entrypoint (Task 19) — API is `unstable`; success = `icon completions <shell>` emits a script and cached names complete.
- Real Iconify ids for README entries lacking parenthesized ids (Task 10) — implementer selects real `prefix:name` ids from the Iconify browser.

**Type consistency:** `IconBody::new(body, w, h)`, `Style::assemble(&body)`, `Icon::from_domain/from_network`, `IconCache::{open_at,get,put,search_names,clear}`, `IconifyClient::{with_base,fetch_body,fetch_collections}`, `parse_id` — names used consistently across tasks.
