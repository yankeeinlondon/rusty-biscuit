---
prompt: |-
    The [`aurora_iconify`](https://docs.rs/aurora_iconify/latest/aurora_iconify) for Rust programs allows for the embedding of [Iconify](https://icon-sets.iconify.design/) icons into a Rust program.

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
    - How does aurora_iconify compare to the `iconify` crate?
last_updated: 2026-06-06
---
# `aurora_iconify`

`aurora_iconify` is a procedural-macro crate that downloads complete [Iconify](https://iconify.design/) icon sets during compilation, generates Rust accessors for every icon, and embeds the resulting SVG documents as `&'static str` values.

The latest release is **0.1.1**, published on **June 3, 2026**.

## Key URLs

- [Crate on crates.io](https://crates.io/crates/aurora_iconify)
- [API documentation on docs.rs](https://docs.rs/aurora_iconify/latest/aurora_iconify/)
- [Published source on docs.rs](https://docs.rs/crate/aurora_iconify/latest/source/)
- [GitHub repository](https://github.com/drew-chase/aurora-ui)
- [Iconify documentation](https://iconify.design/docs/)
- [Iconify icon browser](https://icon-sets.iconify.design/)
- [Iconify API](https://api.iconify.design/)
- [Iconify API documentation](https://iconify.design/docs/api/)

The crate lives under `crates/aurora_iconify` in the larger Aurora UI repository rather than in a dedicated repository.

## Installation

```toml
[dependencies]
aurora_iconify = "0.1.1"
```

The package manifest uses Rust edition 2024. In practice, consumers should use a Rust toolchain that supports edition 2024, despite the README currently stating “Rust edition 2021 or later.”

## Functional Footprint

The crate exposes one procedural macro:

```rust
aurora_iconify::icon_sets!("feather", "lucide");
```

For each requested set, the macro:

1. Queries the Iconify collection endpoint for every icon name in the set.
2. Downloads icon data in batches of 80 icons.
3. Constructs complete `<svg>` documents from the returned Iconify bodies.
4. Writes those SVGs under `target/.iconify-cache/<set>/`.
5. Generates an accessor method for every icon.
6. Generates dynamic lookup tables for icon and set names.
7. Uses `include_str!` to embed the generated files into the program.

For the declaration above, the generated API has approximately this shape:

```rust
pub struct Icon;

impl Icon {
    pub fn feather() -> FeatherIcons;
    pub fn lucide() -> LucideIcons;
    pub fn from_set(name: &str) -> Option<DynIconSet>;
}

pub struct FeatherIcons;

impl FeatherIcons {
    pub fn home(&self) -> &'static str;
    pub fn arrow_left(&self) -> &'static str;
    pub fn by_name(&self, name: &str) -> Option<&'static str>;
}
```

The crate does not render SVGs, rasterize them, create GUI widgets, or manipulate SVG attributes. It supplies SVG source strings for another component to render or insert into output.

## Generated Names

Iconify names are converted into Rust method names:

| Iconify name      | Generated method    |
|-------------------|---------------------|
| `arrow-left`      | `arrow_left()`      |
| `calendar-2-fill` | `calendar_2_fill()` |
| `3d-rotate`       | `_3d_rotate()`      |
| `box`             | `box_icon()`        |

Rust keywords receive an `_icon` suffix. Names beginning with an ASCII digit receive a leading underscore.

## Use Cases

### Type-safe access to known icons

This is the crate’s primary use case. Misspelled generated method names become compiler errors.

```rust
aurora_iconify::icon_sets!("feather");

fn main() {
    let svg: &'static str = Icon::feather().home();

    assert!(svg.starts_with("<svg"));
    println!("{svg}");
}
```

Use this pattern when both the icon set and icon are known while writing the program.

### Runtime selection within a known set

Use `by_name()` when the set is fixed but the icon name comes from configuration, user input, or application state.

```rust
aurora_iconify::icon_sets!("feather");

fn main() {
    let requested = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "home".to_owned());

    match Icon::feather().by_name(&requested) {
        Some(svg) => println!("{svg}"),
        None => {
            eprintln!("Unknown Feather icon: {requested}");
            std::process::exit(1);
        }
    }
}
```

The lookup returns `None` for unknown names. It expects the original Iconify name, such as `arrow-left`, rather than the generated Rust name `arrow_left`.

### Runtime selection of both set and icon

`Icon::from_set()` supports applications whose configuration selects both values dynamically.

```rust
aurora_iconify::icon_sets!("feather", "lucide");

fn find_icon(set: &str, name: &str) -> Option<&'static str> {
    Icon::from_set(set).and_then(|icons| icons.by_name(name))
}

fn main() {
    match find_icon("lucide", "settings") {
        Some(svg) => println!("{svg}"),
        None => eprintln!("Unknown icon"),
    }
}
```

Only sets listed in `icon_sets!` are available. This is dynamic lookup over a compile-time-generated catalog, not runtime access to arbitrary Iconify sets.

### Embedding icons in generated HTML

The returned value is raw SVG markup and can be inserted directly into trusted HTML output.

```rust
aurora_iconify::icon_sets!("feather");

fn main() -> std::io::Result<()> {
    let home = Icon::feather().home();

    let page = format!(
        r#"<!doctype html>
<html lang="en">
  <body>
    <a href="/" aria-label="Home">{home}</a>
  </body>
</html>
"#
    );

    std::fs::write("index.html", page)
}
```

Template engines generally require their “raw” or “safe HTML” mechanism; otherwise, the `<svg>` markup may be escaped and displayed as text.

The SVG is trusted at runtime because it was downloaded and embedded during compilation. However, it remains externally sourced build input.

### Exporting an embedded icon as an asset

An application can also write an embedded icon to disk or pass it to an SVG parser.

```rust
aurora_iconify::icon_sets!("feather");

fn main() -> std::io::Result<()> {
    let svg = Icon::feather().github();
    std::fs::write("github.svg", svg)
}
```

For desktop applications, the same string can be passed to an SVG renderer such as `resvg`, an image widget, or the Aurora UI SVG facilities.

## Features

### Cargo feature flags

Version 0.1.1 has **no Cargo feature flags**.

There is therefore no reason to use a dependency declaration such as:

```toml
aurora_iconify = { version = "0.1.1", features = ["..."] }
```

Capabilities such as caching, network access, type-safe generation, and dynamic lookup cannot currently be enabled or disabled independently.

### Functional capabilities

| Capability              | Description                                                                |
|-------------------------|----------------------------------------------------------------------------|
| Complete-set generation | Generates accessors for every icon in each requested set                   |
| Type-safe access        | Known icons can be selected through generated methods                      |
| Dynamic icon lookup     | `by_name()` selects an icon using its original name                        |
| Dynamic set lookup      | `Icon::from_set()` selects among embedded sets                             |
| Compile-time fetching   | Icon data is retrieved while the procedural macro runs                     |
| Local caching           | Set JSON and generated SVG files are stored under `target/.iconify-cache/` |
| Static embedding        | SVG documents are returned as `&'static str`                               |
| Rustdoc previews        | Generated icon methods include previews loaded from the Iconify API        |

## Important Design Decisions

### The unit of inclusion is an entire icon set

Calling:

```rust
aurora_iconify::icon_sets!("lucide");
```

does not request only the icons used by the program. It downloads the complete Lucide collection and generates a method and dynamic match arm for every icon.

This improves discoverability and provides a type-safe catalog, but increases:

- Initial compilation time
- Network traffic
- Cache size
- Generated-code volume
- Rust Analyzer and rustdoc workload
- Potential binary size, especially when dynamic lookup keeps all icons reachable

Use smaller sets when possible. If an application needs only a handful of icons from large collections, the `iconify` crate is usually a better fit.

### Builds are not fully reproducible

The icon set is retrieved from the live Iconify API and is not pinned to an Iconify dataset version. The same source and `Cargo.lock` can therefore produce different embedded SVGs after the seven-day cache expires.

For reproducible or audited builds, consider:

- Preserving and restoring `target/.iconify-cache/` in CI
- Vendoring generated SVGs into the repository
- Using a crate with a prepared offline mode
- Treating cache refreshes as dependency updates that require review

### Dynamic access has a larger retention cost

A direct call such as:

```rust
Icon::feather().home()
```

allows the compiler and linker more opportunity to discard unused generated functions and strings.

Calling:

```rust
Icon::feather().by_name(name)
```

creates runtime match paths to every icon in that set, making it more likely that the complete set remains in the final binary.

### SVG presentation is fixed during generation

Version 0.1.1 creates an outer SVG similar to:

```html
<svg xmlns="http://www.w3.org/2000/svg"
     width="24"
     height="24"
     viewBox="0 0 24 24">
  ...
</svg>
```

There is no macro syntax for requesting a color, custom dimensions, rotation, flipping, or API transformation. Consumers must style compatible icon bodies through CSS or modify/parse the SVG after retrieval.

## Gotchas and Workarounds

### First builds require network access

A missing or expired cache causes the procedural macro to contact `api.iconify.design`. Firewalls, offline CI workers, proxy requirements, API outages, or TLS problems can fail compilation.

**Workaround:** populate and preserve `target/.iconify-cache/`, or vendor the resulting assets. There is no supported offline feature in 0.1.1.

### Documented stale-cache fallback is not implemented

The README says that an expired cache is used with a warning when the API is unavailable. The published 0.1.1 source instead attempts a refresh and returns a compile error if that request fails.

**Workaround:** keep cache modification times fresh in offline environments, restore a recently generated cache before building, or vendor the assets.

### `cargo clean` removes the cache

The cache is deliberately stored inside `target/`. Running `cargo clean` therefore turns the next build into another network-dependent full download.

**Workaround:** cache `target/.iconify-cache/` separately in CI or copy it out before cleaning.

### Custom Cargo target directories are not honored

The implementation discovers a workspace and then uses its literal `target/` directory. It does not consult `CARGO_TARGET_DIR`.

**Workaround:** expect the icon cache under the repository’s `target/` even when compilation artifacts are redirected elsewhere.

### Use one invocation per module

Every invocation generates types named `Icon` and `DynIconSet`. Two calls in the same module will produce duplicate definitions.

Prefer one combined declaration:

```rust
aurora_iconify::icon_sets!("feather", "lucide", "tabler");
```

Separate invocations can be placed in separate modules when separate generated namespaces are required.

### Duplicate and colliding names are not checked explicitly

Repeated set names generate duplicate methods and enum variants. Sanitization can also theoretically map two upstream names to the same Rust identifier.

**Workaround:** do not repeat set names, and treat identifier-related compiler failures as possible upstream-name collisions.

### Icon-set licenses remain relevant

`aurora_iconify` is licensed under MIT OR Apache-2.0, but the embedded icons retain the licenses of their individual Iconify collections.

**Workaround:** review each selected collection’s license and attribution requirements in the [Iconify collection browser](https://icon-sets.iconify.design/).

### Accessibility is the consumer’s responsibility

The generated SVG documents do not add application-specific titles, labels, roles, or `aria-hidden` attributes.

**Workaround:** add an accessible label to the surrounding control, or parse and augment the SVG when it conveys information rather than decoration.

### Generated Rustdoc can make network requests

Each generated icon method has a documentation preview URL hosted by the Iconify API. Viewing generated documentation may therefore retrieve remote images.

### API data is simplified

The implementation consumes icon bodies and width/height values but does not expose the complete Iconify data model. Consumers needing aliases, transformations, metadata, or custom API parameters should use a lower-level Iconify integration.

## Adoption

No major company or established product is publicly documented as using `aurora_iconify`.

As of June 6, 2026:

- crates.io reports no reverse dependencies.
- The crate has roughly 32 total downloads.
- Its parent repository has two GitHub stars.
- Its principal known consumer is the still-in-development [Aurora UI](https://github.com/drew-chase/aurora-ui) project.
- The Aurora UI repository explicitly describes the framework as not ready for production use.

Absence from public dependency data does not rule out private use, but there is currently no evidence of broad production adoption.

## Release History

Only two releases exist, so a five-release history cannot be provided.

| Release | Date         | Changes                                                                                                                                                                                                                 |
|---------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `0.1.1` | June 3, 2026 | Changed generated icon accessors from large inline string literals to per-icon `include_str!` files under the cache. This was intended to reduce the SVG text Rust tooling had to parse and improve editor performance. |
| `0.1.0` | June 2, 2026 | Initial crates.io release with complete-set fetching, seven-day caching, batched API requests, generated type-safe methods, `by_name()`, and `Icon::from_set()`.                                                        |

The repository does not publish GitHub Releases or a dedicated changelog for these versions. The `0.1.1` change is documented by the corresponding [implementation commit](https://github.com/Drew-Chase/aurora-ui/commit/7c0a0b26b4df85fd8ed6281ce843511de8ed603e).

### Major versions

There has been no stable `1.x` release. The only development series is `0.1.x`, first published on June 2, 2026.

## Maturity and Maintenance

`aurora_iconify` should be considered **experimental and very immature**.

Positive indicators include:

- Complete rustdoc coverage reported by docs.rs
- A small and understandable implementation
- A follow-up performance fix immediately after the first release
- Continued integration with its parent UI framework

Risk indicators include:

- Less than one week of published history
- Only two releases
- Very low download and adoption numbers
- No reverse dependencies on crates.io
- No dedicated issue history for the crate
- No offline or reproducible-build mode
- Documentation drift from the implementation
- No declared minimum supported Rust version
- Development under a parent framework that is itself marked not production-ready

The project was updated actively around its launch, but there is not enough history to establish a sustained maintenance cadence.

## Comparison with `iconify`

The similarly named [`iconify`](https://crates.io/crates/iconify) crate takes a substantially different approach.

| Area                             | `aurora_iconify` 0.1.1             | `iconify` 0.3.1                              |
|----------------------------------|------------------------------------|----------------------------------------------|
| Selection unit                   | Complete icon set                  | Individual icon                              |
| Primary syntax                   | `Icon::feather().home()`           | `iconify::svg!("feather:home")`              |
| Type-safe catalog                | Yes, generated methods             | No                                           |
| Runtime lookup                   | Generated set/name matches         | Not its primary model                        |
| API transformations              | No                                 | Yes                                          |
| Width, height, and color options | No macro options                   | Supported through macro arguments            |
| Cache                            | Mandatory, seven-day project cache | Optional `cache` feature, enabled by default |
| Offline mode                     | No                                 | `offline` feature with prepared assets       |
| Custom API URL                   | No                                 | `ICONIFY_URL`                                |
| Cargo features                   | None                               | `cache`, `tls`, and `offline`                |
| Published history                | Since June 2026                    | Since July 2023                              |
| Latest release                   | June 2026                          | April 2024                                   |
| Approximate downloads            | 32                                 | More than 22,000                             |
| Rust edition                     | 2024                               | 2021                                         |

An equivalent `iconify` call is:

```rust
fn main() {
    let svg: &'static str = iconify::svg!(
        "feather:home",
        width = "32",
        height = "32",
        color = "rebeccapurple",
    );

    println!("{svg}");
}
```

Choose `aurora_iconify` when:

- IDE discovery of every icon as a Rust method is valuable.
- The application uses many icons from the same set.
- Runtime lookup among a predefined catalog is required.
- Complete-set embedding is acceptable.

Choose `iconify` when:

- Only a small number of icons are needed.
- Builds must support a prepared offline mode.
- Width, height, color, rotation, or other Iconify API options are required.
- Smaller generated APIs and more selective asset inclusion are priorities.
- A crate with a longer release history and broader adoption is preferred.

For most applications that use a limited, explicitly known icon list, `iconify` offers the more focused and operationally mature model. `aurora_iconify` is most compelling when the complete-set, generated-catalog API is itself the desired feature.
