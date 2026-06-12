# Curated Domain Icons

`biscuit-icon` ships a small, curated subset of icons that are
**compiled into the binary** — no network, no cache, always available.
Each set is exposed as a `Copy` enum implementing the `DomainIcon`
trait; the enums derive `strum::{EnumString, EnumIter, Display}` so
they support `FromStr`, iteration, and `to_string()`.

A subset of variants define an optional `Glyph` — a plain Unicode
codepoint and/or a Nerd Font private-use codepoint. The
`renderable`-based terminal ladder prefers the glyph (when
`Icon::nerd_font(true)` is set and a Nerd Font codepoint exists) over
the Unicode codepoint over the image-protocol render over the
identifier as text.

There are **16 enums** and **~150 variants** in total. The full
machine-readable list is `biscuit_icon::domain::all_iconify_ids()`.

## Sets

### `Os`

Operating-system and platform marks.

- `Finder` (hugeicons-apple-finder)
- `AppStore` (ri-app-store-fill)
- `Windows` (whh-windowseight)
- `Linux` (ant-design-linux-outlined)
- `Macos` (f7-logo-macos)
- `Apple` (ic-baseline-apple)

### `Emoji`

Common emoji, each with a Unicode codepoint.

- `Happy`, `Sad`, `Laughing`, `Angry`, `Surprised`

### `Arrow`

Directional arrow icons.

- `CircularLeft`, `CircularRight`, `CircularUp`, `CircularDown`

### `Data`

Storage and data-structure icons.

- `Cloud`, `Database`, `Floppy`, `SdCard`
- `UnorderedList`, `OrderedList`, `List`

### `File`

File-type icons (language and format).

- `Markdown` (material-symbols-markdown)
- `Pdf` (ant-design-file-pdf-filled)
- `Json` (lucide-file-json)
- `Toml` (file-icons-toml)
- `Yaml` (file-icons-yaml-alt1)
- `Xml` (mdi-file-xml-box)
- `WordDoc` (teenyicons-ms-word-outline)
- `Spreadsheet` (mdi-spreadsheet)
- `Image` (material-symbols-image-rounded)
- `Svg` (ci-file-svg)
- `Css` (tabler-brand-css3)
- `Html` (ci-file-html)
- `Rust` (mdi-language-rust)
- `Javascript` (proicons-javascript)
- `Typescript` (mdi-language-typescript)
- `Python` (mdi-language-python)
- `Folder` (material-symbols-light-folder-outline-rounded)
- `FolderFill` (material-symbols-light-folder-rounded)

### `Hardware`

Compute, I/O, and tool icons.

- `ServerNode`, `FileServer` (uil-file-network), `ServerNetwork`,
  `ServerTower`
- `Laptop`, `Monitor`, `Chip`
- `Camera`, `Microphone`, `Speaker`
- `Hammer`, `Wrench`, `Printer`

### `Timing`

Clock and timing icons.

- `StartFlag`, `StopSign`, `StopSquare`, `Timer`

### `Button`

Media-control icons.

- `Play`, `Pause`, `FastForward`, `Rewind`, `Stop`, `Mute`, `Power`

### `Control`

Form-control icons. Includes Nerd Font codepoints for several.

- `RadioUnselected`, `RadioSelected`, `RadioDisabled`,
  `RadioDisabledSelected` (fluent-radio-button-*)
- `CircularCheck`, `CircularCheckUnread`, `CircularCheckOutline`,
  `CircularCheckOutlineUnread` (material-symbols-check-circle-*)
- `SquareUnchecked`, `SquareChecked`, `SquareCheckedFill`
  (material-symbols-check-box-*)

### `Network`

Connectivity icons. Several define Nerd Font codepoints.

- `WifiStrong`, `WifiWeak`
- `Nodes` (carbon-network-1), `NodesStructured` (carbon-network-2)
- `Ethernet` (mdi-ethernet)
- `3G`, `4G`, `5G`, `LTE` (streamline-freehand-cellular-network-wifi-*)

### `DevOps`

Version control and CI/CD icons. Several define Nerd Font codepoints.

- `Git` (ion-git-network), `GitAlt` (fe-git)
- `Github` (uil-github), `GitMerge` (bx-git-merge)
- `GitLab` (lucide-gitlab), `Gitea` (pajamas-gitea)
- `CiCd` (clarity-ci-cd-line), `Deployment`
  (material-symbols-light-deployed-code-sharp)
- `Versions` (system-uicons-versions)

### `Actors`

User and group icons.

- `ProfileCircular` (material-symbols-account-circle)
- `ProfileSquare` (material-symbols-account-box)
- `ProfilePin` (material-symbols-person-pin)
- `Group` (material-symbols-group-rounded)

### `Nav`

Common navigation icons.

- `Home` (material-symbols-home)
- `Settings` (material-symbols-settings)
- `Profile` (material-symbols-account-circle)
- `Location` (material-symbols-light-my-location)
- `Cart` (material-symbols-light-shopping-cart-outline)
- `Bag` (material-symbols-light-shopping-bag-outline)

### `Sport`

Sport icons.

- `Baseball` (material-symbols-sports-baseball)
- `Basketball` (ic-sharp-sports-basketball)
- `Football` (ic-round-sports-football)
- `Soccer` (ic-baseline-sports-soccer)
- `Tennis` (material-symbols-light-sports-tennis-rounded)
- `Cricket` (ic-baseline-sports-soccer)
- `Cycling` (solar-bicycling-outline)
- `Running` (solar-running-2-bold)
- `Swimming` (maki-swimming)
- `Golf` (ic-baseline-sports-golf)
- `MartialArts` (ic-twotone-sports-gymnastics)
- `Volleyball` (material-symbols-light-sports-volleyball-outline)

### `Brand`

Product and brand marks.

- `Ubiquiti` (cbi-ubiquiti)
- `UbiquitiAccessPoint` (cbi-ubiquiti-ap)
- `Anthropic` (ri-anthropic-fill)
- `OpenAi` (ri-openai-fill)

### `Social`

Social-network icons.

- `WhatsApp` (tabler-brand-whatsapp-filled)
- `Twitter` (mdi-twitter)
- `FacebookCircular` (ic-baseline-facebook)
- `FacebookSquare` (ri-facebook-box-fill)
- `InstagramCircular` (typcn-social-instagram-circular)
- `Instagram` (typcn-social-instagram)
- `X` (mingcute-social-x-line)
- `PinterestCircular` (ion-social-pinterest-outline)
- `LinkedInCircular` (typcn-social-linkedin-circular)
- `BlueSky` (mingcute-bluesky-social-line)
- `YouTube` (famicons-logo-youtube)
- `YouTubeAlt` (zmdi-youtube)

## The `DomainIcon` Trait

Every curated enum implements:

```rust
pub trait DomainIcon: Copy {
    /// The upstream Iconify identifier, e.g. `"hugeicons:apple-finder"`.
    fn iconify_id(self) -> &'static str;
    /// The embedded icon body for this variant.
    fn body(self) -> IconBody;
    /// Character representation, if this icon defines one.
    fn glyph(self) -> Option<Glyph> { None }
    /// Builds an [`Icon`] for this domain variant.
    fn icon(self) -> Icon;
}
```

The enum-first idiom:

```rust
use biscuit_icon::Icon;
use biscuit_icon::domain::{DomainIcon, Os, File, Brand};

let finder = Os::Finder.icon();
let rust_logo = File::Rust.icon();
let claude = Brand::Anthropic.icon();
```

The string-convenience layer (one per set) uses `strum`'s `FromStr`:

```rust
use biscuit_icon::Icon;
use biscuit_icon::IconError;

let openai: Result<Icon, IconError> = Icon::brand("open_ai");
// or: "openai" / "OpenAi" — see the enum's exact snake_case name
```

`Icon::os(name)`, `Icon::emoji(name)`, ..., `Icon::brand(name)` all
return `Result<Icon, IconError>`. A miss yields
`IconError::UnknownDomainIcon { set, name }`.

## Asset Pipeline (Dev-only)

Curated bodies are vendored under `biscuit-icon/assets/icons/<set>/<name>.svg`
and committed. A `populate_assets` binary (dev-only) refreshes them
from Iconify:

```bash
just populate-assets
```

This binary is intentionally **not** part of the normal library or
CLI dependency graph. Refreshing the assets is treated like a
dependency update: review the diff, since upstream artwork and
licensing may change. Normal `cargo build` never touches the
network — the assets are pinned, reviewed bytes.

The string representations (Unicode + Nerd Font codepoints) for the
curated subset are hand-curated in source. They are never fetched.

## Which Variants Have Glyphs?

The hand-curated `glyph()` impl on each variant decides. The
canonical sources of truth are the per-variant `glyph(self) -> Option<Glyph>`
overrides in `biscuit-icon/lib/src/domain/{os,emoji,...,brand,social}.rs`.
Use `Icon::unicode_char()` and `Icon::nerd_font_char()` at runtime to
project the curated codepoint for a given variant.

Common glyph-bearing variants include the `Emoji` set (Unicode), the
`Button` set (Unicode), parts of `Control` (Nerd Font), parts of
`DevOps` (Nerd Font, including `Github` and `Git`), and parts of
`Network` (Nerd Font).
