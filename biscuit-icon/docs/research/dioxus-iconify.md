---
prompt: |-
    The [`dioxus-iconify`](https://docs.rs/crate/dioxus-iconify/latest) for Rust programs allows for the embedding of [Iconify](https://icon-sets.iconify.design/) icons into a Rust program.

    Your task is to a deep dive into this package and provide a full overview of what this package offers, how it is used, and important design decisions to consider when using this crate.

    The document should answer the following questions:

    - key URLs (website, docs, etc.)
    - overview description of the functional footprint
    - list out the use-cases which this crate is typically used for
        - for every use case provide a simple but complete example of how the crate would be used; make sure that the code example is accurate and reflects the syntax of the latest release of the crate
    - what _features_ does the crate offer and what does each feature add? when should you add a feature? when should you not add a feature?
    - what "gotchas" report hitting when working with this crate? How can these gotchas be worked around?
    - are there any big and known companies or products which use this crate?
    - what are the last five releases of this product? what date were they released? what changes were introduced with each?
        - add all major version of the software along with dates
    - how mature is this crate? how actively updated?
    - How does dioxus-iconify compare to the `iconify` crate?
last_updated: 2026-06-06
---
# `dioxus-iconify`

## Overview

[`dioxus-iconify`](https://crates.io/crates/dioxus-iconify) is a CLI code generator for vendoring [Iconify](https://iconify.design/) icons and local SVG files into Dioxus applications.

Despite being distributed through crates.io, it is **not a Rust library dependency**. Running the CLI generates Rust source code containing:

- An `Icon` Dioxus component
- An `IconData` structure
- One module per icon collection
- One typed Rust constant per imported icon

The generated code depends only on Dioxus. The generator itself is not included in the application's compile-time or runtime dependency graph.

The latest release is **0.4.2**.

## Key URLs

- [crates.io](https://crates.io/crates/dioxus-iconify)
- [docs.rs](https://docs.rs/crate/dioxus-iconify/latest)
- [GitHub repository](https://github.com/davidB/dioxus-iconify)
- [Releases](https://github.com/davidB/dioxus-iconify/releases)
- [Changelog](https://github.com/davidB/dioxus-iconify/blob/main/CHANGELOG.md)
- [Issue tracker](https://github.com/davidB/dioxus-iconify/issues)
- [Iconify icon browser](https://icon-sets.iconify.design/)
- [Iconify API documentation](https://iconify.design/docs/api/)
- [Dioxus documentation](https://dioxus.dev/)

## Installation

Install the command globally rather than adding it to `[dependencies]`:

```bash
cargo install dioxus-iconify
```

Prebuilt binaries are also available:

```bash
cargo binstall dioxus-iconify
brew install davidB/tap/dioxus-iconify
mise install "ubi:davidB/dioxus-iconify"
```

The default output directory is `src/icons`. Override it globally with `--output`:

```bash
dioxus-iconify --output src/components/icons add lucide:settings
```

## Functional Footprint

The CLI exposes four commands:

| Command  | Purpose                                                                      |
|----------|------------------------------------------------------------------------------|
| `add`    | Downloads Iconify icons or imports local SVG files and generates Rust code   |
| `init`   | Creates the output directory and initial `mod.rs`                            |
| `list`   | Lists icons currently represented by generated collection files              |
| `update` | Downloads current Iconify data for existing identifiers and regenerates code |

`add` accepts Iconify identifiers, files, directories, or a mixture:

```bash
dioxus-iconify add mdi:home
dioxus-iconify add mdi:home heroicons:arrow-left
dioxus-iconify add ./assets/logo.svg
dioxus-iconify add ./assets/icons/
dioxus-iconify add mdi:home ./assets/logo.svg
```

A command such as:

```bash
dioxus-iconify add mdi:home heroicons:arrow-left
```

generates approximately this layout:

```text
src/icons/
├── mod.rs
├── heroicons.rs
└── mdi.rs
```

Each collection module contains constants such as `mdi::Home`. The generated `Icon` component renders the selected icon body through Dioxus's `dangerous_inner_html` SVG attribute.

## Use Cases

### Vendoring Iconify icons into a Dioxus application

This is the primary use case. Select icons in the Iconify browser and add them by their `collection:name` identifiers:

```bash
dioxus-iconify add mdi:home lucide:settings heroicons:arrow-left
```

Use the generated constants through the shared component:

```rust
use dioxus::prelude::*;

mod icons;

use icons::{heroicons, lucide, mdi, Icon};

fn App() -> Element {
    rsx! {
        nav {
            Icon { data: heroicons::ArrowLeft }
            Icon { data: mdi::Home }
            Icon { data: lucide::Settings }
        }
    }
}
```

The icons become ordinary project source files and no network access is required when compiling or running the application.

### Styling and sizing icons

The generated component accepts `size` and extends Dioxus's `SvgAttributes`. `size` sets both dimensions, while explicit SVG attributes provide independent control:

```rust
use dioxus::prelude::*;

mod icons;

use icons::{mdi, Icon};

fn App() -> Element {
    rsx! {
        div {
            Icon {
                data: mdi::Home,
                size: "1.25rem",
            }

            Icon {
                data: mdi::Home,
                width: "32",
                height: "24",
                fill: "currentColor",
                class: "text-blue-600",
                aria_label: "Home",
                role: "img",
            }
        }
    }
}
```

Whether `fill`, `stroke`, or CSS color changes the visible icon depends on the SVG's own attributes. Monochrome icons using `currentColor` are generally easiest to theme.

### Importing a custom SVG

Local SVGs can use the same generated component as Iconify icons:

```text
assets/
└── company-logo.svg
```

```bash
dioxus-iconify add ./assets/company-logo.svg
```

The parent directory becomes the collection name, producing `assets::CompanyLogo`:

```rust
use dioxus::prelude::*;

mod icons;

use icons::{assets, Icon};

fn App() -> Element {
    rsx! {
        header {
            Icon {
                data: assets::CompanyLogo,
                width: "160",
                height: "40",
                aria_label: "Company name",
                role: "img",
            }
        }
    }
}
```

The importer extracts the inner SVG body, `width`, `height`, and `viewBox`. Missing dimensions default to `24 × 24`.

### Importing an SVG directory

Directories are scanned recursively:

```text
assets/product-icons/
├── dashboard.svg
└── arrows/
    └── left.svg
```

```bash
dioxus-iconify add ./assets/product-icons/
```

Nested path segments are joined with hyphens. The generated constants are therefore `Dashboard` and `ArrowsLeft`:

```rust
use dioxus::prelude::*;

mod icons;

use icons::{product_icons, Icon};

fn App() -> Element {
    rsx! {
        Icon { data: product_icons::Dashboard }
        Icon { data: product_icons::ArrowsLeft }
    }
}
```

Use `--skip-existing` when repeatedly importing a directory whose generated entries may have been customized:

```bash
dioxus-iconify add --skip-existing ./assets/product-icons/
```

### Creating an application-level icon vocabulary

Applications can hide upstream collection choices behind semantic aliases. This makes replacing an icon set less disruptive.

After generating the icons, create `src/icons/app.rs`:

```rust
pub use super::heroicons::ArrowLeft as Back;
pub use super::mdi::Home;
pub use super::mdi::TrashCan as Delete;
```

Expose the application module while keeping collection modules private in `src/icons/mod.rs`:

```rust
pub mod app;

mod heroicons;
mod mdi;
```

Application components can then depend on semantic names:

```rust
use dioxus::prelude::*;

mod icons;

use icons::{app, Icon};

fn Toolbar() -> Element {
    rsx! {
        button {
            aria_label: "Go back",
            Icon { data: app::Back }
        }
        button {
            aria_label: "Delete",
            Icon { data: app::Delete }
        }
    }
}
```

Be aware that `update` regenerates `mod.rs`; custom module declarations may need to be restored.

### Refreshing vendored icons

Generated icons remain fixed until explicitly updated:

```bash
dioxus-iconify list
dioxus-iconify update
```

The application code does not change:

```rust
use dioxus::prelude::*;

mod icons;

use icons::{lucide, Icon};

fn SettingsLink() -> Element {
    rsx! {
        a {
            href: "/settings",
            Icon { data: lucide::Settings }
            "Settings"
        }
    }
}
```

Review and commit the generated diff after an update. Upstream artwork, dimensions, metadata, or licensing information may have changed.

## Features

### Cargo feature flags

Version 0.4.2 has **no Cargo feature flags**. There is no reason to use `default-features = false` or a `features = [...]` list.

The tool should normally be installed as a binary:

```bash
cargo install dioxus-iconify
```

It should generally **not** be added to an application's dependencies:

```toml
# Usually incorrect:
[dependencies]
dioxus-iconify = "0.4.2"
```

### Product capabilities

| Capability                    | What it adds                                                      | When to use it                                             |
|-------------------------------|-------------------------------------------------------------------|------------------------------------------------------------|
| Iconify importing             | Access to Iconify's large multi-collection catalog                | When icons already exist in a maintained public set        |
| Local SVG importing           | Converts project-specific artwork to the same representation      | For logos, proprietary artwork, or locally corrected icons |
| Recursive directory importing | Bulk-imports an SVG tree                                          | For an existing internal icon set                          |
| Typed constants               | Converts names to Rust constants such as `ArrowLeft`              | Almost always; this is central to the generated API        |
| SVG attribute forwarding      | Accepts Dioxus `SvgAttributes`                                    | For styling, accessibility, events, and dimensions         |
| `size` property               | Sets width and height together                                    | For square icons; use explicit dimensions otherwise        |
| Collection metadata           | Writes upstream author and license details into generated modules | Preserve it for attribution and license auditing           |
| `--skip-existing`             | Avoids replacing matching generated constants                     | When reimporting directories containing customized entries |
| `update`                      | Refreshes previously generated identifiers                        | When deliberately accepting upstream icon changes          |
| Custom output path            | Places generated modules outside `src/icons`                      | When matching an established component layout              |

## Design Considerations

### Generated code is owned by the application

This follows a shadcn-style model: the tool gives the application editable source code rather than an opaque library component.

Benefits include:

- No runtime dependency on `dioxus-iconify`
- No network access during builds
- Only selected icons are compiled
- Generated code can be reviewed and customized
- Icon updates are explicit

Costs include:

- Generated code must be committed and maintained
- Generator upgrades can change checked-in source
- Local changes may conflict with regeneration
- The project, rather than the crate author, owns generated-code defects

Keep application-specific wrappers and aliases outside generated collection files where possible.

### Dioxus version coupling exists in generated code

Release 0.4.2 generates a Dioxus component using:

```rust
#[props(extends = SvgAttributes)]
attributes: Vec<Attribute>,
```

This targets Dioxus 0.7-era APIs. Although the CLI itself does not depend on Dioxus, its output does. Projects using older or future incompatible Dioxus releases may need to modify the generated `Icon` component.

### Icon licensing is separate from the CLI license

The generator is licensed under CC0-1.0. That does not relicense imported artwork.

Each Iconify collection retains its upstream license, which may require attribution or impose brand-use restrictions. Review the collection's Iconify page and preserve generated metadata. Local SVGs likewise retain their existing copyright and license.

### Reproducibility depends on committing output

An `add` or `update` command queries the live Iconify API, so running it at different times may produce different SVG data. The CLI has no lockfile or source-version pinning mechanism.

For reproducible builds:

1. Commit `src/icons/`.
2. Do not run the generator during normal builds.
3. Treat updates as reviewed dependency changes.
4. Record the CLI version used by CI or developer tooling.

### Accessibility is the caller's responsibility

The generated component does not infer whether an icon is decorative or meaningful.

A meaningful icon should have an accessible name:

```rust
Icon {
    data: mdi::Alert,
    role: "img",
    aria_label: "Warning",
}
```

A decorative icon should be hidden from assistive technology:

```rust
Icon {
    data: mdi::ChevronRight,
    aria_hidden: "true",
    focusable: "false",
}
```

## Gotchas

### SVG bodies containing `"#` can break generated Rust

[Issue #12](https://github.com/davidB/dioxus-iconify/issues/12) reports that icons containing attributes such as `fill="#fff"` terminate the generated `r#"... "#` raw string early. `logos:chrome` is a reported example.

Until fixed, use one of these workarounds:

- Import a locally edited SVG without the conflicting sequence.
- Change the generated body literal to a higher delimiter such as `r##"..."##`.
- Patch the generator to choose a delimiter not present in the SVG body.
- Run `cargo check` after every generation operation.

### Fixed SVG fills and strokes can defeat caller styling

An outer `fill`, `stroke`, or `currentColor` class does not necessarily override values embedded directly in child paths.

Workarounds:

- Choose a monochrome Iconify variant designed around `currentColor`.
- Edit a local SVG before importing it.
- Modify the generated SVG body.
- Use CSS selectors with sufficient specificity when appropriate.

Multicolor logos should generally retain their fixed colors.

### `dangerous_inner_html` creates a trust boundary

Generated icon bodies are inserted as raw SVG markup. Iconify is a curated source, but arbitrary local SVG files could contain scripts, event attributes, external references, or other unwanted markup.

Only import trusted SVGs. The local parser validates XML structure but should not be treated as a security sanitizer.

### `update` is not a lockfile-aware operation

`update` fetches the current representation of every parsed identifier. It does not preserve an upstream revision, produce a lockfile, or explain visual differences.

Always inspect the generated Git diff and run visual or snapshot tests after updating.

### Local icons are also encountered by `update`

From the current implementation, `update` reads all generated `name` fields and attempts to resolve them through the Iconify API. A locally generated identifier such as `assets:company-logo` may therefore fail to update unless an identically named Iconify collection exists. The existing local constant remains in its collection file, but the command reports the failed fetch.

Reimport local files explicitly instead:

```bash
dioxus-iconify add ./assets/company-logo.svg
```

### Regeneration can replace `mod.rs` customization

The documentation specifically warns that application module declarations may need to be restored after `update`. Keep substantial custom components in separate files and minimize edits to generated `mod.rs`.

### Naming conversion can create collisions

Icon names are converted to PascalCase. Leading numeric names gain `_`, and Rust keywords gain an `Icon` suffix. Distinct source names can potentially normalize to the same Rust identifier.

Run `cargo check` and inspect generated collection modules when importing similarly named icons.

### Missing SVG dimensions default to 24

Local SVGs without usable dimensions or a `viewBox` receive `24 × 24` and `0 0 24 24`. This can crop or distort artwork whose actual coordinate system differs.

Add an accurate `viewBox` before importing the SVG.

### There is no `remove` command

The README lists removal as a future capability. Removing an icon currently requires editing or regenerating the relevant collection module manually.

## Known Users

No large company or widely known product publicly documents production use of `dioxus-iconify`.

The available evidence indicates a small, early-stage ecosystem project: the repository has roughly ten stars, one fork, a small issue tracker, and only a few hundred crates.io downloads as of June 2026. Absence of public references does not prove that no private products use it, but enterprise adoption should not currently be assumed.

Iconify itself is broadly used across the web ecosystem, but that does not imply use of this particular Dioxus generator.

## Release History

### Last five releases

Registry publication dates are used below. The 0.4.2 changelog labels the release `2025-12-07`, but crates.io records publication on **February 10, 2026**.

| Release | Published         | Changes                                                                                                                                       |
|---------|------------------:|-----------------------------------------------------------------------------------------------------------------------------------------------|
| 0.4.2   | February 10, 2026 | Documentation-only fix to enclose a URL correctly                                                                                             |
| 0.4.1   | December 3, 2025  | Changed generated component attributes from global HTML attributes to SVG attributes; updated tagline                                         |
| 0.4.0   | November 27, 2025 | Added local SVG file and recursive directory importing; fixed collection metadata extraction                                                  |
| 0.3.0   | November 27, 2025 | Preserved submodule visibility in `icons/mod.rs`; added collection author, license, and related metadata; documented application icon aliases |
| 0.2.3   | November 26, 2025 | Fixed generated collection formatting                                                                                                         |

### Version-line milestones

Because the project has not reached 1.0, these are minor-version milestones rather than SemVer major releases:

| Version line | First release     | Significance                                                                                                       |
|--------------|------------------:|--------------------------------------------------------------------------------------------------------------------|
| 0.1.x        | November 26, 2025 | Initial Iconify-to-Dioxus code-generation workflow; generated selected icons and the shared component              |
| 0.2.x        | November 26, 2025 | Added `size`, `list`, and `update`; moved HTTP to async Rust and rustls; several same-day generator fixes followed |
| 0.3.x        | November 27, 2025 | Improved module visibility preservation and collection metadata                                                    |
| 0.4.x        | November 27, 2025 | Added local SVG support and corrected generated SVG attribute handling                                             |

There has been no `1.x` release.

## Maturity and Maintenance

`dioxus-iconify` should be considered **experimental to early-stage**:

- It remains below version 1.0.
- Its first release was November 26, 2025.
- Most releases occurred over an eight-day initial development burst.
- Version 0.4.2 was published in February 2026.
- A code-generation bug affecting common color attributes was still open on June 5, 2026.
- The repository has a single primary maintainer and limited community adoption.
- No minimum supported Rust version is declared.
- Generated output is tied to current Dioxus component APIs.

The crate is usable for applications willing to own and review generated source. It is less suitable when a team expects a stable library API, automatic security maintenance, strict generator reproducibility, or established production support.

## Comparison with `iconify`

The [`iconify`](https://crates.io/crates/iconify) crate is a general-purpose procedural macro crate, not a Dioxus-specific component generator.

```rust
let svg = iconify::svg!(
    "mdi:home",
    width = "24",
    height = "24",
    color = "red",
);
```

Its macro downloads and embeds a complete SVG string at compile time. It supports API transformation options, caching, and an optional offline workflow.

| Concern                  | `dioxus-iconify`                             | `iconify`                                                           |
|--------------------------|----------------------------------------------|---------------------------------------------------------------------|
| Primary interface        | Installed CLI                                | Procedural macro dependency                                         |
| Framework scope          | Dioxus-specific                              | Framework-independent SVG strings                                   |
| Fetch time               | Explicit `add` or `update` command           | Macro expansion during compilation or preparation                   |
| Output                   | Checked-in Rust modules and Dioxus component | Embedded SVG string                                                 |
| Runtime dependency       | None beyond generated Dioxus code            | No network runtime, but crate participates in compilation           |
| Build-time network       | Not required after output is committed       | Required by default for uncached icons                              |
| Offline strategy         | Commit generated source                      | Enable `offline`, prepare icons, and commit/use its icon directory  |
| Type-level API           | Generated `IconData` constants               | String-returning `svg!` invocation                                  |
| Local SVG importing      | Supported                                    | Not its primary function                                            |
| Iconify API options      | Limited to fetching icon data                | Supports API options such as dimensions, color, and transformations |
| Cargo features           | None                                         | `cache`, `tls`, `offline`, and supporting dependency features       |
| Dioxus integration       | Generated `Icon` component                   | Caller must insert the SVG into Dioxus safely                       |
| Generated code ownership | Application owns and may edit it             | Macro implementation remains external                               |
| Update behavior          | Explicit regeneration with reviewable diffs  | Recompilation/cache preparation controls fetched content            |

Choose `dioxus-iconify` when:

- The application is built with Dioxus.
- Generated source should be committed.
- Builds must remain independent of the Iconify API.
- Local SVGs and Iconify icons should share one component.
- The team accepts ownership of generated code.

Choose `iconify` when:

- The SVG is needed outside Dioxus.
- A concise `svg!` macro is preferred.
- Iconify API transformations are important.
- Build-time fetching and caching are acceptable.
- The project's templating system already knows how to insert trusted SVG strings.

For most Dioxus applications prioritizing reproducible offline builds, `dioxus-iconify` has the cleaner deployment model. For framework-neutral code or applications requiring Iconify's transformation options, `iconify` is more flexible.
