# biscuit-icon

Curated, offline-embedded domain icons plus on-demand [Iconify](https://iconify.design/) lookup for Rust applications.

## Overview

`biscuit-icon` provides two complementary icon sources:

- **Curated domain icons** — 16 enums covering ~150 icons (OS, File, DevOps, Brand, Social, etc.) compiled directly into the binary. Always available, no network required.
- **Iconify network lookup** — on-demand access to any of the 200,000+ open-source icons from the Iconify project, cached locally in SQLite at `~/.cache/biscuit-icon/icons.db`.

Every icon can be styled (color, size, flip, rotation) and rendered through the shared `renderable` multi-target tree — producing inline SVG for browsers, SVG in markdown, and a degradation ladder (Nerd Font glyph → Unicode glyph → image protocol → text) for terminals.

## Quick Start

```rust
use biscuit_icon::Icon;
use biscuit_icon::domain::{DomainIcon, Os, File, Brand};

// Enum-first: infallible, compiled in, zero-cost
let finder = Os::Finder.icon();

// String convenience: fallible lookup by name
let happy = Icon::emoji("happy")?;

// Network/cache: any Iconify icon via prefix:name identifier
let home = Icon::iconify("mdi:home").await?;

// Style and emit SVG
let svg = finder.color("#d97706").width("24").height("24").svg();
```

## Key Types

### `Icon`

The central handle. Wraps an icon body, style, optional glyph, id, and provenance. Construct via domain enums, string convenience methods, or `Icon::iconify`. Chain builder methods and finish with `.svg()` for the assembled markup.

```rust
use biscuit_icon::{Icon, Flip, Rotate};

let icon = Icon::os("finder")?
    .color("#3b82f6")
    .width("32")
    .height("32")
    .flip(Flip::Horizontal)
    .rotate(Rotate::R90)
    .svg();
```

### `Source`

```rust
pub enum Source {
    Embedded,  // compiled-in curated domain icon
    Network,   // fetched / cache-resident Iconify icon
}
```

Determined at construction time. Use it to distinguish icons that are always available offline from those that require at least one successful network fetch.

### `IconBody`

The raw SVG body plus geometry (width, height, view-box origin). The body string contains inner SVG markup (paths, groups) without the surrounding `<svg>` element. `IconBody::view_box()` returns the formatted `"left top width height"` string.

### `Style`

Accumulated presentation options. All default to "unset" so unset options mean "let the SVG default apply". Applied via builder methods on `Icon`:

| Method | Type | Effect |
|--------|------|--------|
| `color` | `String` | Inline `style="color: …"`, driving `currentColor` for monochrome icons |
| `width` | `String` | SVG `width` attribute (default `1em`) |
| `height` | `String` | SVG `height` attribute (default `1em`) |
| `flip` | `Flip` | Horizontal / vertical / both-axis flip |
| `rotate` | `Rotate` | 90° / 180° / 270° rotation |
| `view_box` | `bool` | Prepends a transparent `<rect>` spanning the viewBox |
| `nerd_font` | `bool` | Prefers the Nerd Font glyph during terminal rendering |

### `Flip` / `Rotate`

```rust
pub enum Flip { Horizontal, Vertical, Both }
pub enum Rotate { R90, R180, R270 }
```

Both implement `TryFrom<&str>` for string-based configuration.

### `Glyph`

Character representation for curated icons that define one:

```rust
pub struct Glyph {
    pub unicode: Option<char>,    // plain Unicode codepoint
    pub nerd_font: Option<char>,  // Nerd Font private-use codepoint
}
```

Access via `Icon::unicode_char()` and `Icon::nerd_font_char()`.

### `IconError`

```rust
pub enum IconError {
    UnknownDomainIcon { set: &'static str, name: String },
    InvalidIdentifier(String),
    NotFound(String),
    Fetch(String),
    Cache(String),
}
```

## Domain Enums

16 curated enums, each implementing `Copy`, `FromStr`, `Display`, and the `DomainIcon` trait:

| Enum | Examples |
|------|----------|
| `Os` | `Finder`, `Windows`, `Linux`, `Apple` |
| `Emoji` | `Happy`, `Sad`, `Laughing`, `Angry` |
| `File` | `Markdown`, `Pdf`, `Rust`, `Typescript`, `Folder` |
| `Hardware` | `ServerTower`, `Laptop`, `Chip`, `Camera` |
| `DevOps` | `Git`, `Github`, `GitLab`, `CiCd` |
| `Brand` | `Anthropic`, `OpenAi`, `Ubiquiti` |
| `Social` | `WhatsApp`, `Twitter`, `BlueSky`, `YouTube` |
| `Button` | `Play`, `Pause`, `Stop`, `Power` |
| `Control` | `RadioSelected`, `SquareChecked`, `CircularCheck` |
| `Network` | `WifiStrong`, `Ethernet`, `5G` |
| `Nav` | `Home`, `Settings`, `Profile`, `Cart` |
| `Arrow` | `CircularLeft`, `CircularRight` |
| `Data` | `Cloud`, `Database`, `Floppy` |
| `Timing` | `StartFlag`, `StopSign`, `Timer` |
| `Actors` | `ProfileCircular`, `Group` |
| `Sport` | `Baseball`, `Soccer`, `Tennis` |

Use `biscuit_icon::domain::all_iconify_ids()` for the full machine-readable list, or `biscuit_icon::domain::icon_for_id(id)` to resolve an Iconify identifier to its curated `Icon`.

## Rendering

`Icon` implements both `TerminalRenderable` and `TreeRenderable` from the shared `renderable` ecosystem:

- **Browser/HTML** — inline SVG via the render tree
- **Markdown** — inline SVG in `MarkdownPlus` dialect
- **Terminal** — degradation ladder: Nerd Font glyph → Unicode glyph → image protocol → text identifier

```rust
use biscuit_icon::Icon;
use biscuit_icon::domain::{DomainIcon, Emoji};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();
let out = Emoji::Happy.icon().render(&term);
```

## Cargo Features

| Feature | Default | Effect |
|---------|---------|--------|
| `image` | off | Enables the image-protocol rung of the terminal ladder (rasterize SVG via `biscuit-visualized`/`resvg`). When off, the ladder stops at the text-identifier fallback for glyph-less icons. |

```toml
[dependencies]
biscuit-icon = { path = ".../biscuit-icon/lib", features = ["image"] }
```

## License

AGPL-3.0
