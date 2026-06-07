# Biscuit Icon

A library and CLI which provide both SVG (_and in a subset of cases_) character based icons.

- the [Iconify](https://iconify.design/) project is the source for all SVG icons
    - has over 200,000 open source icons across 100+ icon sets

## Library Features

1. Domain icon sets (represented as enums) which have the SVG directly included in binary:
    - Os
        - Finder (hugeicons-apple-finder)
        - AppStore (ri-app-store-fill)
        - Windows (whh-windowseight)
        - Linux (ant-design-linux-outlined)
        - macOS (f7-logo-macos)
        - Apple (ic-baseline-apple)
    - Emoji
        - Happy
        - Sad
        - Laughing
        - Angry
        - Surprised
    - Arrow
        - CircularLeft
        - CircularRight
        - CircularUp
        - CircularDown
    - Data
        - Cloud
        - Database
        - Floppy
        - SdCard
        - UnorderedList
        - OrderedList
        - List
    - File
        - Markdown (material-symbols-markdown)
        - Pdf (ant-design-file-pdf-filled)
        - Json (lucide-file-json)
        - Toml (file-icons-toml)
        - Yaml (file-icons-yaml-alt1)
        - Xml (mdi-file-xml-box)
        - WordDoc (teenyicons-ms-word-outline)
        - Spreadsheet (mdi-spreadsheet)
        - Image (material-symbols-image-rounded)
        - Svg (ci-file-svg)
        - Css (tabler-brand-css3)
        - Html (ci-file-html)
        - Rust (mdi-language-rust)
        - Javascript (proicons-javascript)
        - Typescript (mdi-language-typescript)
        - Python (mdi-language-python)
        - Folder (material-symbols-light-folder-outline-rounded)
        - FolderFill (material-symbols-light-folder-rounded)
    - Hardware
        - ServerNode
        - FileServer (uil-file-network)
        - ServerNetwork
        - ServerTower
        - Laptop
        - Monitor
        - Chip
        - Camera
        - Microphone
        - Speaker
        - Hammer
        - Wrench
        - Printer
    - Timing
        - StartFlag
        - StopSign
        - StopSquare
        - Timer
    - Button
        - Play
        - Pause
        - FastForward
        - Rewind
        - Stop
        - Mute
        - Power
    - Control
        - RadioUnselected (fluent-radio-button-24-regular)
        - RadioSelected (fluent-radio-button-24-filled)
        - RadioDisabled (fluent-radio-button-off-16-regular)
        - RadioDisabledSelected (fluent-radio-button-off-16-filled)
        - CircularCheck (material-symbols-check-circle-rounded)
        - CircularCheckUnread (material-symbols-check-circle-unread)
        - CircularCheckOutline (material-symbols-check-circle-outline-rounded)
        - CircularCheckOutlineUnread (material-symbols-check-circle-unread-outline-rounded)
        - SquareUnchecked (material-symbols-check-box-outline-blank)
        - SquareChecked (material-symbols-check-box-outline-rounded)
        - SquareCheckedFill (material-symbols-check-box-rounded)
    - Network
        - WifiStrong
        - WifiWeak
        - Nodes (carbon-network-1)
        - NodesStructured (carbon-network-2)
        - Ethernet (mdi-ethernet)
        - 3G (streamline-freehand-cellular-network-wifi-3g)
        - 4G (streamline-freehand-cellular-network-wifi-4g)
        - 5G (streamline-freehand-cellular-network-wifi-5g)
        - LTE (streamline-freehand-cellular-network-wifi-lte)
    - DevOps
        - Git (ion-git-network)
        - GitAlt (fe-git)
        - Github (uil-github)
        - GitMerge (bx-git-merge)
        - GitLab (lucide-gitlab)
        - Gitea (pajamas-gitea)
        - CiCd (clarity-ci-cd-line)
        - Deployment (material-symbols-light-deployed-code-sharp)
        - Versions (system-uicons-versions)
    - Actors
        - ProfileCircular(material-symbols-account-circle)
        - ProfileSquare(material-symbols-account-box)
        - ProfilePin (material-symbols-person-pin)
        - Group (material-symbols-group-rounded)
    - Nav
        - Home (material-symbols-home)
        - Settings (material-symbols-settings)
        - Profile (material-symbols-account-circle)
        - Location (material-symbols-light-my-location)
        - Cart (material-symbols-light-shopping-cart-outline)
        - Bag (material-symbols-light-shopping-bag-outline)
    - Sport
        - Baseball (material-symbols-sports-baseball)
        - Basketball (ic-sharp-sports-basketball)
        - Football (ic-round-sports-football)
        - Soccer (ic-baseline-sports-soccer)
        - Tennis (material-symbols-light-sports-tennis-rounded)
        - Cricket (ic-baseline-sports-soccer)
        - Cycling (solar-bicycling-outline)
        - Running (solar-running-2-bold)
        - Swimming (maki-swimming)
        - Golf (ic-baseline-sports-golf)
        - MartialArts (ic-twotone-sports-gymnastics)
        - Volleyball (material-symbols-light-sports-volleyball-outline)
    - Brand
        - Ubiquiti (cbi-ubiquiti)
        - UbiquitiAccessPoint (cbi-ubiquiti-ap)
        - Anthropic (ri-anthropic-fill)
        - OpenAi (ri-openai-fill)
    - Social
        - WhatsApp (tabler-brand-whatsapp-filled)
        - Twitter (mdi-twitter)
        - FacebookCircular (ic-baseline-facebook)
        - FacebookSquare (ri-facebook-box-fill)
        - InstagramCircular (typcn-social-instagram-circular)
        - Instagram (typcn-social-instagram)
        - X (mingcute-social-x-line)
        - PinterestCircular (ion-social-pinterest-outline)
        - LinkedInCircular (typcn-social-linkedin-circular)
        - BlueSky (mingcute-bluesky-social-line)
        - YouTube (famicons-logo-youtube)
        - YouTubeAlt (zmdi-youtube)

    These domain sets provide a small footprint set of highly reusable icons which are always immediately available. Also a subset of them provide a "character" representation which takes two forms:

    1. Normal Unicode Character
    2. Nerd Font Character

2. A network lookup service that will get _and cache into a local DB_ any of the 200,000+ icons from Iconify on demand


For most callers, the way they will interact with this library is through the `Icon` struct.

```rust
// using built-in icons
let phone = Icon::device("mobile_phone");
let happy = Icon::emoji("happy");

// from Iconify via network or cache
let home = await Icon::iconify("mdi:home")?;
```

The `Icon` struct then provides builder methods for stylizing the icon:

| Option     | Type            | Meaning                                   |
|------------|-----------------|-------------------------------------------|
| `color`    | String literal  | Any CSS color accepted by the Iconify API |
| `width`    | String literal  | SVG width, defaulting to `1em`            |
| `height`   | String literal  | SVG height, defaulting to `1em`           |
| `flip`     | String literal  | `horizontal`, `vertical`, or `both`       |
| `rotate`   | String literal  | `90`, `180`, or `270`                     |
| `view_box` | Boolean literal | Adds Iconify's transparent bounding box   |

## CLI API Surface

The CLI binary `icon` has an API surface of:

- `sets <filter>` provides the list of icon sets (names only) which Iconify provides; you can optionally filter down the returned sets with the `<filter>` param
- `icons [filter]` 
    - provides a list of icons (and icon names) who's name includes the `filter` parameter
    - you can isolate to an enumerated set of icon sets using the `--from <csv>` filter (e.g., `icon icons happy --from fa,mdi`)
- `completions`
    - provides dynamic shell completions
    - it will always know the icon set names and the static icon sets built into the binary
    - but it will also be able to query the database for cached icon names too
- the `icons` subcommand is the _default_ command so a caller can call `icon icons mdi:home` or `icon mdi:home` and both are identical in behavior.

## Tech Stack

### Library

- `iconify` crate provide handy procedural macro used for building Icon's domain icon sets.

### CLI

- `clap` primary tool for CLI functionality (using "derive", "env", "unstable-ext" features)
- `clap_complete` for shell completions (using "unstable-dynamic" feature)

### Shared

- `tracing` and `tracing-subscriber`
    - spans, metrics, and debug reporting
