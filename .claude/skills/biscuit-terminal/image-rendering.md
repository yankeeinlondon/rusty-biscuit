# Image Rendering

`TerminalImage` renders inline images as escape-sequence strings (Kitty, iTerm2, or Sixel) for the terminal's graphics route, with terminal-aware cursor management; without a pixel route a plain image becomes half-blocks or alt text.

## Basic Usage

```rust
use biscuit_terminal::components::terminal_image::{ImageOutput, TerminalImage};
use biscuit_terminal::terminal::Terminal;
use std::path::Path;

let term = Terminal::new();
let img = TerminalImage::new(Path::new("photo.jpg"))?;

match img.render_with_half_blocks(&term)? {
    ImageOutput::Pixels(sequence) => print!("{sequence}"),
    ImageOutput::HalfBlocks { text, .. } | ImageOutput::Fallback { text, .. } => println!("{text}"),
}
```

## Path and Width Validation

`TerminalImage::new()` validates that:
- The file exists
- The path can be canonicalized to a local absolute path

Width input is validated by `parse_width_spec()`:
- `50%` -> `ImageWidth::Percent(0.5)`
- `80` or `80ch` -> `ImageWidth::Characters(80)`
- `fill` -> `ImageWidth::Fill`

`ImageWidth::Scale(f32)` has no string form. It sizes content from its natural width relative to terminal text (pixels per unit = scale × cell height ÷ 16, columns rounded up, clamped to the available columns). Resolve it with `TerminalImage::resolve_scaled_dimensions_for(width, layout, term_width, natural_width, cell)`. The plain `resolve_dimensions_for` has no natural width, so there it behaves like `Fill`. Use `CellSize::FALLBACK` (8×16) for an unknown cell size rather than a new literal.

```rust
use biscuit_terminal::components::terminal_image::{parse_width_spec, TerminalImage};
use std::path::Path;

let width = parse_width_spec("75%")?;
let img = TerminalImage::new(Path::new("photo.jpg"))?.with_width(width);
```

## Already-encoded PNG (`EncodedPng`)

For PNG bytes the program produced itself (e.g. `biscuit-visualized`'s diagram
components, which rasterize at the display size), skip the file round-trip:

```rust
use biscuit_terminal::components::terminal_image::{EncodedPng, ImageWidth, TerminalImage};

let png = EncodedPng::new(png_bytes)?; // validates; no decode
let img = TerminalImage::from_encoded_png(png).with_width(ImageWidth::Characters(60));
```

- `EncodedPng::new(Vec<u8>)` checks, before any decode or row reservation:
  ≤ 10 MiB (`DEFAULT_MAX_FILE_SIZE`, else `FileTooLarge`), a readable PNG
  header (else `ImageLoadError`), and dimensions within the `image` crate's
  default 512 MiB decode allocation limit (else `ImageLoadError` with a limits
  error). Accessors: `bytes()`, `width()`, `height()` (from the header).
- Kitty and iTerm2 standalone paths, the inline (`TwoColumn`) path, and
  `render_as_kitty` send the bytes **unchanged**. They are decoded only by
  `load_image()` or an actual resize (`render_as_iterm2` when the target pixel
  width, columns × cell width, differs from the PNG width). Internally the
  protocol paths go through `ImagePixels` (`Encoded` borrows the bytes,
  `Decoded` encodes a loaded file).
- No file: iTerm2 `name=` is `image.png`, alt text `[Image: image]`, unless
  `with_alt_text` is set.
- Not for caller-supplied files; those keep using `new` / `from_spec`.

## PNG encoding

`encode_as_png` (and every internal encode: decoded files, resizes) uses
`PngEncoder::new_with_quality(CompressionType::Default, FilterType::Adaptive)`
(`encoded_png::encode_png`), not `write_to`, whose fast settings can exceed
the raw pixel size. Measured once on macOS: the 400x400 iTerm2 app icon went
from 53,114 to 43,044 bytes.

## Policy Controls with TerminalImageOptions

`TerminalImageOptions` is a policy helper (`base_path`, `max_file_size`, `allow_remote`, `width`) that your application can enforce before rendering.

Current runtime flow (`TerminalImage::new()`, `from_spec()`, `render_with_route()` / `render_with_half_blocks()`) does **not** automatically enforce remote/path/size policy checks.

### TerminalImageOptions Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_path` | `Option<PathBuf>` | `None` | Optional directory boundary for allowed files |
| `max_file_size` | `u64` | `10MB` | Maximum allowed file size |
| `allow_remote` | `bool` | `false` | Whether URL-like inputs are allowed |
| `width` | `ImageWidth` | `Percent(0.5)` | Default width policy |

### App-Enforced Policy Example

```rust
use biscuit_terminal::components::image_options::{ImageWidth, TerminalImageOptions};
use biscuit_terminal::components::terminal_image::TerminalImage;
use std::path::{Path, PathBuf};

let options = TerminalImageOptions::builder()
    .base_path(PathBuf::from("/safe/images"))
    .max_file_size(5 * 1024 * 1024)
    .allow_remote(false)
    .width(ImageWidth::Percent(0.6))
    .build();

let path = Path::new("photo.jpg");
let file_size = std::fs::metadata(path)?.len();

if !options.is_size_allowed(file_size) || !options.is_path_allowed(path) {
    panic!("Image blocked by policy");
}

let img = TerminalImage::new(path)?.with_width(options.width.clone());
```

## Width Syntax

```
image.png           -> Default 50% width
image.png|25%       -> 25% of terminal width
image.png|80        -> Fixed 80 columns
image.png|80ch      -> Fixed 80 columns
image.png|fill      -> Fill available width
```

## Protocol Selection and Cursor Behavior

The route's encoder selects protocol and terminal-specific cursor handling:

| Terminal | Protocol | Sizing Strategy | Cursor Behavior |
|----------|----------|-----------------|-----------------|
| Kitty/Ghostty/Konsole | Kitty (`c=` only) | Terminal derives rows | Auto-advance with overshoot correction |
| WezTerm | Kitty (`c=` + `r=`) | Explicit rows required | Explicit row advance + carriage return |
| iTerm2 | iTerm2 OSC 1337 | Native scaling (`preserveAspectRatio=1`) | Overshoot correction + row advance |
| Warp | Kitty (`c=` only) | Terminal derives rows | Uses floor-based row count to avoid blank-line overshoot |
| Others | None | N/A | Half-blocks or alt text (`render_with_half_blocks`); alt text (`render_with_route`) |

Pixel output normalizes back to column 0 with `\r`. It usually avoids trailing newlines, but may append one `\n` when bottom-of-screen scroll compensation is needed.

## Direct Protocol Methods

For lower-level composition paths:

```rust
// Includes its own cursor advancement
let kitty = img.render_as_kitty(80)?;
let iterm = img.render_as_iterm2(80)?;

// Raw protocol payload constructors
let png_data = img.encode_as_png(&loaded_image)?;
let kitty_escape = img.render_kitty_cells(&png_data, width_cells, height_cells);
let iterm_escape = img.render_iterm2(&png_data, "40", "filename.png");
```

## Cell Size and Aspect Ratio

`TerminalImage` uses measured cell size when available (`discovery::fonts::cell_size()`), with an `8x16` fallback. This avoids visibly distorted images on terminals with non-2:1 cell geometry. Unix measures it per lookup (`TIOCGWINSZ`, then CSI 14 t); native Windows reads the `CSI 16 t` answer from the startup exchange, once per process (`docs/discovery/terminal-queries.md#cell-size`).

## Error Handling Notes

`TerminalImageError` includes:
- File/path/encoding/runtime errors (`FileNotFound`, `InvalidPath`, `ImageLoadError`, etc.)
- Policy-style variants (`PathTraversalBlocked`, `FileTooLarge`, `RemoteUrlBlocked`)

The policy-style variants are primarily tied to helper checks; they are not all emitted by the default `new()`/`render_with_route()` path.

## Graphics Route and Fallback

- The encoder follows `Terminal::graphics_route` (a `GraphicsRoute`: format,
  transport, placement, confidence, structured `RouteReason`,
  `needs_measured_cells`, `needs_cursor_resync`), resolved once
  per `Terminal`; never re-resolve per image. Developer guide:
  `biscuit-terminal/docs/discovery/graphics-routes.md`.
- `render_with_route(&term)` returns `ImageOutput::Pixels(seq)` or
  `ImageOutput::Fallback { text, reason }` (alt text with controls stripped).
  Load/decode errors stay `Err` either way; `render()` maps errors to `""`.
  It **never** returns `HalfBlocks`; diagrams (`MermaidDiagram`,
  `GraphExpression`) must keep using it so their fallback stays the source.
- `render_with_half_blocks(&term)` is the plain-image variant (used by
  `render()`/`display()`, the `TwoColumn` inline path via `render_inline` →
  `InlineImage::Lines`, and `bt image`): no pixel format, or a pixel route
  refused late (tmux 1 MiB, unmeasured Zellij cells) →
  `ImageOutput::HalfBlocks { text, reason }` (`▀` cells, truecolor fg = upper
  pixel, bg = lower; rows joined with `\n`, no trailing newline; never the alt
  text; `into_text()` returns it). Permitted only when `term.is_tty`,
  `TrueColor`, `supports_unicode`, and the reason kind is not `UserDisabled`,
  `InvalidInput`, or `NotATerminal` (`half_blocks_permitted`); otherwise
  `Fallback`. So pipes get alt text even with `FORCE_COLOR=1`. Width uses the
  pixel paths' resolver; height keeps aspect (half-block pixel = 1 cell wide,
  ½ cell tall, measured or 8×16); alpha 0 is never painted (`SGR 49`
  default background: `▄` when the upper pixel is transparent, a space when
  both are, `49` below an odd last row), partial alpha blends onto
  `SixelBackground::for_terminal`; triangle filter. Not image support:
  `ImageSupport`/route `format` unchanged. Render-tree image nodes and
  Darkmatter's legacy `▉ IMAGE[...]` never draw half-blocks.
- A Kitty route to `TerminalApp::ITerm2` uses the iTerm2 format unless the
  route is `Forced` (iTerm2 ignores `q=2` and places nothing for width-only `c=`).
- A Sixel route draws via `sixel.rs` (`Encoder::Sixel`, `icy_sixel` 0.7, no
  cargo feature): Wu quantization, `SIXEL_PALETTE_COLORS` (256), diffusion 0,
  so output is deterministic. Resized to columns × cell width (measured or
  `CellSize::FALLBACK`), padded below with unset pixels to whole rows
  (never stretched). DCS `ESC P 9;1;0 q` (`P2=1`) + `"1;1;W;H`. Alpha 0
  sets no sixel bit (icy_sixel leaves alpha < 128 unset; the canvas only
  ever holds 0 or 255); partial alpha blends onto
  `SixelBackground::for_terminal` (`Measured` only when
  `Terminal::background_source` is `ColorSource::Measured`, an OSC 11 reply;
  else `Estimated` = the `COLORFGBG`/per-app guess in `background_color`, or
  white for `ColorMode::Light`, else black; logged at debug in "Sixel image
  render"). **Trap:** `background_color` is `Some(#1e1e1e)` under tmux with
  no reply; never treat `Some` as measured, read `background_source`.
- Sixel cursor: IND (`ESC D`) per row first (scroll happens before drawing,
  never under a saved cursor), CUU back, `ESC[s`, optional CUF, DCS,
  `ESC[u`; standalone adds CUD rows + CR, inline (`TwoColumn`) omits it. No
  scroll compensation, and never set/reset private modes 80 or 1070.
  Through tmux passthrough only the DCS is wrapped (1 MiB budget applies).
- A route with no pixels falls back with the route's reason.
- Transport `Tmux`: `protocol::for_transport` wraps every APC/OSC/DCS in its
  own envelope (`terminal_image/tmux.rs`, `wrap_graphics_for_tmux`; CSI such
  as `ESC[s`/`ESC[u`/mode 2026 stays unwrapped, an existing envelope is never
  re-wrapped). Kitty uploads through tmux use `render_kitty_cells`
  (`c=`+`r=`). Every ordinary upload (`render_kitty_cells`,
  `render_kitty_width_only`, so also direct routes, `render_as_kitty` and the
  graphical `HorizontalRule`) is `q=2` with a fresh `i=` from
  `probe::next_kitty_id` on the first chunk only. Any wrapped message
  over `TMUX_MESSAGE_BUDGET` (1 MiB) → `Fallback` (`BlockedTransport`, size in
  the message) before rows are reserved; `render_inline` returns
  `UnsupportedTerminal` instead.
- Placement `Placeholders` (Kitty through tmux): `render_kitty_placeholders`
  uploads with `a=T,U=1,i=<id>,c=,r=` (wrapped) and returns one text row per
  image row from `placeholder.rs::placeholder_rows` (`U+10EEEE` + row and
  column marks from Kitty's 297-entry diacritic table, ID as `38;2;R;G;B`,
  `SGR 39` per row; never wrapped). Standalone ends each row with `\r\n`;
  `render_inline` returns `InlineImage::Lines` (upload prefixed to row 0)
  versus `InlineImage::Overlay` for cursor overlays, and `TwoColumn` lays
  `Lines` out as ordinary text. A grid over 297 cells either way, or an ID
  over 24 bits, places ordinarily. `next_kitty_id` stays within
  `MAX_KITTY_ID` (24 bits) and wraps to 1. `visible_width` treats a DCS /
  tmux envelope (doubled ESCs inside) as zero columns.
- L2: `level2_tmux_kitty_client_gets_unicode_placeholders` attaches a
  `script`-hosted client with `TERM=xterm-kitty` (system terminfo, else
  kitty.app's bundled one; skips without either) and reads what tmux
  forwards to it. The client attaches with `tmux -u -T RGB`: without a
  declared `RGB` feature tmux infers color support from the host's terminfo,
  and where it does not infer `RGB` the route correctly places ordinarily.
- A `needs_cursor_resync` route (Windows SSH server, iTerm2 only) writes
  `" \r"` right after the image sequence, before the `ESC[<n>B\r` tail
  (standalone) or at the end of the `render_inline` overlay; every other
  route's bytes are unchanged. conhost drops the CR otherwise (see the `os`
  skill, `windows-console.md`).
- A `needs_measured_cells` route (Zellij) falls back when `term.cell_size()`
  is `None` (`unmeasured_cells_reason`); never size such a route from the
  8×16 estimate. The library never prints hints; `bt`
  does (stderr, terminal only, once per reason, only with a `fix`).
- Tests: assert pixel dispatch through `protocol::encoder_for` (rendering
  pixels queries the live cursor), and build routes with
  `resolve_graphics_route(&RouteInputs::new(true).with_var(..))`, never by
  mutating the process env.

## Related

- [Terminal Struct](./terminal-struct.md) - `graphics_route` and its `ImageSupport` projection
- [Discovery Functions](./discovery.md) - `graphics_route()` / `resolve_graphics_route()`
