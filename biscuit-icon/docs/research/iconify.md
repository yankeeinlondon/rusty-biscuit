---
prompt: |-
    The [`iconify`](https://docs.rs/iconify/latest/iconify/) crate for Rust programs allows for the embedding of [Iconify](https://icon-sets.iconify.design/) icons into a Rust program.

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
last_updated: 2026-06-06
---
# `iconify` for Rust

## Overview

[`iconify`](https://crates.io/crates/iconify) is a small procedural-macro crate that downloads SVG icons from the [Iconify API](https://iconify.design/docs/api/) during compilation and embeds the resulting SVG markup into the program as a string literal.

```rust
let svg: &str = iconify::svg!("mdi:home");
```

The crate's public API consists of one macro, `svg!`. Its functional footprint is deliberately narrow:

- Select an Iconify icon using its `prefix:name` identifier.
- Request the generated SVG at compile time.
- Optionally customize its color, dimensions, rotation, flip, and bounding box.
- Cache downloaded SVGs between builds.
- Prepare and consume a checked-in offline icon directory.
- Use another Iconify-compatible API through environment configuration.
- Return the SVG as an embedded `&'static str`, requiring no client-side Iconify runtime.

It does **not** provide an icon picker, runtime icon lookup, an SVG parser, Rust UI components, CSS generation, accessibility attributes, or sanitization APIs.

The latest published release is **0.3.1**, released on **April 20, 2024**.

## Key URLs

- [Crate on crates.io](https://crates.io/crates/iconify)
- [Rust API documentation](https://docs.rs/iconify/latest/iconify/)
- [`svg!` macro documentation](https://docs.rs/iconify/latest/iconify/macro.svg.html)
- [Source repository](https://github.com/wrapperup/iconify-rs)
- [Repository releases](https://github.com/wrapperup/iconify-rs/releases)
- [Repository issues](https://github.com/wrapperup/iconify-rs/issues)
- [Iconify website](https://iconify.design/)
- [Icon search and icon-set browser](https://icon-sets.iconify.design/)
- [Iconify API documentation](https://iconify.design/docs/api/)
- [SVG API documentation](https://iconify.design/docs/api/svg.html)
- [Iconify icon licenses](https://iconify.design/docs/icons/licenses.html)

## Installation

The normal configuration enables HTTPS and persistent caching:

```toml
[dependencies]
iconify = "0.3.1"
```

The crate is a procedural macro, so its HTTP request occurs on the build host while Rust is expanding the macro. It does not add an Iconify HTTP client to the running application.

## Macro Syntax

The icon identifier and all option values must be literals:

```rust
let svg = iconify::svg!(
    "mdi:home",
    color = "rebeccapurple",
    width = "24",
    height = "24",
    flip = "horizontal",
    rotate = "90",
    view_box = true,
);
```

Supported options are:

| Option     | Type            | Meaning                                   |
|------------|-----------------|-------------------------------------------|
| `color`    | String literal  | Any CSS color accepted by the Iconify API |
| `width`    | String literal  | SVG width, defaulting to `1em`            |
| `height`   | String literal  | SVG height, defaulting to `1em`           |
| `flip`     | String literal  | `horizontal`, `vertical`, or `both`       |
| `rotate`   | String literal  | `90`, `180`, or `270`                     |
| `view_box` | Boolean literal | Adds Iconify's transparent bounding box   |

The macro expands to an SVG string literal and therefore produces an `&'static str`.

## Common Use Cases

### Embed a UI icon

This is the simplest use case: embed one icon without shipping an icon font, JavaScript loader, or complete icon pack.

```rust
fn main() {
    let home: &'static str = iconify::svg!("mdi:home");

    println!("{home}");
}
```

The first build fetches approximately:

```text
https://api.iconify.design/mdi/home.svg
```

The returned SVG becomes part of the compiled program.

### Create reusable icon constants

Because the result is a string literal, it can be assigned to a constant:

```rust
const HOME_ICON: &str = iconify::svg!("lucide:house");
const SETTINGS_ICON: &str = iconify::svg!("lucide:settings");

fn icon_for_settings_page() -> &'static str {
    SETTINGS_ICON
}

fn main() {
    println!("{}", icon_for_settings_page());
    println!("{HOME_ICON}");
}
```

This works well for a small, statically known application icon set.

### Request a fixed size and color

Iconify can customize the SVG before it is embedded:

```rust
fn main() {
    let warning = iconify::svg!(
        "mdi:alert",
        width = "32px",
        height = "32px",
        color = "#d97706",
    );

    println!("{warning}");
}
```

For monochrome icons, omitting `color` commonly leaves `currentColor` in the SVG, allowing the surrounding HTML element's CSS color to control it.

Color customization does not necessarily affect multicolor icons whose paths contain fixed colors.

### Rotate or flip an icon

Transformations are applied by the Iconify API inside the SVG rather than by runtime CSS:

```rust
fn main() {
    let back_arrow = iconify::svg!(
        "mdi:arrow-right",
        width = "24",
        height = "24",
        flip = "horizontal",
        rotate = "90",
    );

    println!("{back_arrow}");
}
```

Use this when the transformed variant is itself a fixed build-time asset. Use CSS transforms instead if the orientation needs to change dynamically.

### Render inline SVG with Maud

The returned value is raw SVG markup. Maud must therefore be told not to HTML-escape it.

```toml
[dependencies]
iconify = "0.3.1"
maud = "0.27"
```

```rust
use maud::{html, Markup, PreEscaped};

fn page() -> Markup {
    html! {
        button type="button" {
            (PreEscaped(iconify::svg!(
                "lucide:save",
                width = "20",
                height = "20",
            )))
            " Save"
        }
    }
}

fn main() {
    println!("{}", page().into_string());
}
```

Only bypass escaping for SVG content that is trusted. In this case, the content was obtained from the configured Iconify server at build time.

### Use the SVG in a component-oriented UI

Frameworks that accept raw HTML can consume the string, but their raw-markup mechanism must be used because normal text interpolation escapes `<svg>`.

A minimal Dioxus example is:

```toml
[dependencies]
dioxus = "0.6"
iconify = "0.3.1"
```

```rust
use dioxus::prelude::*;

fn app() -> Element {
    let icon = iconify::svg!(
        "lucide:circle-check",
        width = "20",
        height = "20",
    );

    rsx! {
        button {
            dangerous_inner_html: "{icon}",
        }
    }
}

fn main() {
    dioxus::launch(app);
}
```

The exact raw-HTML API varies by framework. The important distinction is that `iconify` returns markup, not a framework-specific component.

### Build without network access

Enable offline support while retaining the default features:

```toml
[dependencies]
iconify = { version = "0.3.1", features = ["offline"] }
```

Use the macro normally:

```rust
fn main() {
    println!(
        "{}",
        iconify::svg!(
            "lucide:wifi-off",
            width = "24",
            height = "24",
        )
    );
}
```

Prepare the files while network access is available:

```console
ICONIFY_PREPARE=true cargo check
```

This creates an `icons` directory under `CARGO_MANIFEST_DIR`. Commit that directory if CI or downstream builds must be network-independent.

Subsequent builds omit `ICONIFY_PREPARE`:

```console
cargo build
```

A custom directory can be selected in both phases:

```console
ICONIFY_OFFLINE_DIR=assets/iconify ICONIFY_PREPARE=true cargo check
ICONIFY_OFFLINE_DIR=assets/iconify cargo build
```

If Cargo considers the crate fresh after changing an environment variable, touch the source containing the macro or run an appropriately scoped clean before preparing again.

### Use a self-hosted or proxied API

`ICONIFY_URL` changes the API base URL used by macro expansion:

```rust
fn main() {
    println!("{}", iconify::svg!("mdi:server"));
}
```

Build against an internal Iconify-compatible endpoint:

```console
ICONIFY_URL=https://icons.example.com cargo build
```

This can provide availability control, private icon sets, auditing, or network-policy compliance. The configured server must support the `/{prefix}/{name}.svg` endpoint and the query parameters generated by the crate.

## Cargo Features

### `default`

```toml
default = ["cache", "tls"]
```

This is the recommended configuration for ordinary development. It permits HTTPS requests to the public Iconify API and caches results between builds.

Avoid disabling defaults unless the consequences for both TLS and caching are intentional.

### `cache`

```toml
cache = ["directories", "blake3"]
```

This feature hashes the complete request URL and stores the downloaded SVG on disk. Different dimensions, colors, transformations, APIs, or icon names produce different cache entries.

On Unix systems, version 0.3.x uses:

```text
/tmp/iconify-rs
```

On Windows it uses a directory beneath the user's cache directory. `ICONIFY_CACHE_DIR` overrides the location.

Use `cache` when:

- Developers compile the project repeatedly.
- The build may contain many macro invocations.
- Reducing repeated API requests matters.
- Occasional network access on a cache miss is acceptable.

Consider disabling it when:

- Builds run in disposable environments where the cache has no value.
- Policy forbids writing outside the build tree.
- Every build must retrieve the current upstream SVG.
- You supply your own controlled build cache.

Without this feature, every expanded macro may issue an HTTP request whenever the containing code is compiled.

### `tls`

```toml
tls = ["ureq/tls"]
```

This enables TLS support in the internal `ureq` client.

Keep it enabled when using the default `https://api.iconify.design` endpoint or any HTTPS endpoint.

Disable it only when an explicitly configured API uses plain HTTP in a trusted environment. Disabling `tls` while retaining the default HTTPS URL will prevent normal fetching.

### `offline`

```toml
offline = ["blake3"]
```

This enables the prepare/read workflow for local SVG files.

Use it when:

- CI must not access the internet.
- Builds must work in an air-gapped environment.
- Icon bytes need to be reviewed and committed.
- Reproducible builds matter.
- Upstream Iconify availability must not affect compilation.

Do not add it merely as a fallback cache. Once enabled, normal builds read the offline directory and fail if the required prepared file is absent.

There is also an implementation mismatch in 0.3.1: offline paths call a helper compiled under the `cache` feature, even though `offline` only declares a dependency on `blake3`. Consequently, this minimal configuration is unsafe:

```toml
# Avoid with 0.3.1: offline-only compilation can fail.
iconify = {
    version = "0.3.1",
    default-features = false,
    features = ["offline"],
}
```

Retain the default features when enabling offline mode:

```toml
iconify = { version = "0.3.1", features = ["offline"] }
```

### Internal dependency features

Docs.rs also displays `blake3` and `directories` as feature flags. These are implementation dependencies activated by `cache` or `offline`; applications normally should select the higher-level features instead.

## Environment Variables

| Variable               | Purpose                                                  |
|------------------------|----------------------------------------------------------|
| `ICONIFY_URL`          | Overrides `https://api.iconify.design`                   |
| `ICONIFY_CACHE_DIR`    | Overrides the persistent cache directory                 |
| `ICONIFY_OFFLINE_DIR`  | Overrides the prepared icon directory                    |
| `ICONIFY_PREPARE=true` | Fetches and writes offline icons instead of reading them |

Only the exact lowercase value `true` activates preparation.

## Design Considerations

### Network activity happens during compilation

A clean build with an empty cache depends on DNS, TLS, the configured endpoint, and Iconify availability. A failure is reported as a Rust compilation error.

This is convenient locally but can conflict with hermetic builders, sandboxed package systems, corporate proxies, and CI policies. Offline mode is the appropriate production workflow when build reliability matters.

### Inputs must be compile-time literals

The macro parser accepts a string literal followed by literal name-value options. Variables, constants, loops, and runtime icon names cannot be passed:

```rust
let name = "mdi:home";

// Does not compile:
let svg = iconify::svg!(name);
```

This limitation is tracked in [issue #76](https://github.com/wrapperup/iconify-rs/issues/76).

Use separate macro invocations, define constants containing macro results, or choose a runtime Iconify client when icon names come from users, configuration, or a database.

### Remote SVG content is not pinned

A lockfile pins `iconify` and its Rust dependencies, but it does not pin the SVG returned by Iconify. The same source code can embed different bytes if an icon set changes upstream and the build cache is empty.

For reproducible or auditable artifacts:

1. Enable offline mode.
2. Prepare all icons.
3. Review and commit the generated files.
4. Build CI and releases without `ICONIFY_PREPARE`.

### The cache can preserve old SVGs

Cache keys are derived from the request URL, not the response contents or Iconify's icon-set revision. If upstream changes an icon without changing its identifier, an existing cache entry remains unchanged.

Delete the relevant cache entry or the complete cache directory when deliberately refreshing upstream icons:

```console
rm -rf /tmp/iconify-rs
```

A project-specific `ICONIFY_CACHE_DIR` is preferable when builds need controlled cache invalidation.

### Offline filenames are hashes, not friendly asset names

Prepared files include a BLAKE3-derived suffix based on the complete request URL. Changing `color`, dimensions, transforms, `view_box`, or `ICONIFY_URL` creates a different required file.

Run the preparation phase again after changing any macro option.

### Feature unification affects offline behavior

Cargo features are additive. If any dependency path enables `offline`, the procedural macro is built with offline behavior for the entire resolved package instance.

Applications should generally declare `iconify` directly and keep its feature selection consistent across the workspace.

### Raw SVG must bypass template escaping

Most template engines correctly escape strings by default. If an icon appears as literal `<svg ...>` text, use the framework's trusted/raw HTML wrapper.

Bypassing escaping is security-sensitive. Do not apply the same treatment to arbitrary user-controlled SVG.

### Accessibility is the application's responsibility

The generated SVG does not know whether it is decorative or meaningful. Add accessible surrounding markup as appropriate:

```html
<button type="button" aria-label="Save">
  <!-- decorative inline SVG -->
</button>
```

For standalone meaningful graphics, the application may need to add `role`, labels, or title elements. The macro does not expose arbitrary root-SVG attributes.

### Icon licensing is separate from crate licensing

The Rust crate is licensed under **MIT OR Apache-2.0**. Icons are sourced from many independent icon sets, each with its own license.

Before distributing an icon, inspect its set on the [Iconify icon-set browser](https://icon-sets.iconify.design/) and comply with attribution, trademark, or redistribution requirements. Brand icons deserve particular review.

### Embedded SVG increases artifact and response size

Every macro result is embedded separately as a string. A handful of icons is inexpensive, but a large catalog can increase binary size and duplicate markup in generated pages.

For hundreds or thousands of icons, consider an SVG sprite, a framework-specific icon library, static asset generation, or runtime loading.

## Reported and Practical Gotchas

| Gotcha                                                                   | Workaround                                                             |
|--------------------------------------------------------------------------|------------------------------------------------------------------------|
| Clean builds fail without network access                                 | Prepare and commit icons with `offline`                                |
| `cross-rs` builds could not write the old cache location                 | Upgrade to 0.3.0 or later; override with `ICONIFY_CACHE_DIR` if needed |
| Offline mode did not compile in older releases                           | Use at least 0.2.4                                                     |
| `offline` with defaults disabled can still fail in 0.3.1                 | Keep default features enabled alongside `offline`                      |
| Variables cannot be passed to `svg!`                                     | Use literal invocations or a runtime icon solution                     |
| Changing preparation environment variables may not trigger recompilation | Touch the invoking source or clean the affected package                |
| Cached icons do not automatically follow upstream revisions              | Delete the cache or regenerate committed offline assets                |
| An invalid icon fails compilation                                        | Confirm the exact `prefix:name` in the Iconify browser                 |
| Raw SVG is escaped by templates                                          | Use the template engine's explicit trusted-markup API                  |
| `color` appears ineffective                                              | The selected icon may be multicolor with fixed path colors             |
| Disabling `tls` breaks the default endpoint                              | Keep `tls`, or configure an intentional HTTP endpoint                  |
| Crate licensing is mistaken for icon licensing                           | Review the selected icon set's separate license                        |

## Known Users

There is no public evidence that a large, recognizable company or major product directly depends on this Rust crate.

Crates.io currently reports very limited public reverse-dependency adoption. One published dependent is [`escape-artist`](https://crates.io/crates/escape-artist), a terminal escape-sequence visualizer. Private repositories and applications are not visible in registry statistics, so this is not proof that no other users exist.

This should not be confused with adoption of the wider Iconify ecosystem, which is substantially larger. Evidence that a company uses Iconify's JavaScript packages or API does not establish that it uses the Rust `iconify` crate.

## Release History

### Last five published releases

#### 0.3.1 — April 20, 2024

- Updated the locked `rustls` version to 0.22.4 in response to CVE-2024-32650.
- Updated `quote` from 1.0.35 to 1.0.36.
- Documented the configuration environment variables.
- Did not introduce a new public API.

#### 0.3.0 — April 4, 2024

- Fixed `cross-rs` builds by moving the Unix cache location to `/tmp/iconify-rs`.
- Updated the Askama documentation after support for fully qualified macro paths improved.
- Updated `proc-macro2`, `quote`, `syn`, `ureq`, `url`, and `blake3`.
- Adjusted tests for a changed SVG response from the Iconify API.

The cache-location change is the release's main behavioral difference.

#### 0.2.6 — November 10, 2023

- Updated `proc-macro2` from 1.0.67 to 1.0.69.
- Updated `syn` from 2.0.37 to 2.0.39.
- No documented functional API changes.
- This registry release did not receive a corresponding GitHub release entry.

#### 0.2.5 — October 4, 2023

- Updated `blake3` to 1.5.0.
- Updated `ureq` to 2.8.0.
- Reduced duplicate `rustls-webpki` dependency versions through the newer `ureq`, with a potential compile-time improvement.
- Updated `syn`.

#### 0.2.4 — September 20, 2023

- Fixed offline mode compilation.
- Improved the missing-offline-icon diagnostic.
- Simplified offline error propagation.
- Updated `proc-macro2` to 1.0.67.

### Version-line milestones

| Version line | First release          | Significance                                                                                                 |
|--------------|------------------------|--------------------------------------------------------------------------------------------------------------|
| 0.0          | 0.0.1 on July 5, 2023  | Reserved initial publication; subsequently yanked                                                            |
| 0.1          | 0.1.0 on July 6, 2023  | First substantive implementation with caching and offline support                                            |
| 0.2          | 0.2.0 on July 12, 2023 | Replaced asynchronous `reqwest` with blocking `ureq`, improved errors and transforms, and added tests and CI |
| 0.3          | 0.3.0 on April 4, 2024 | Changed Unix cache placement to support `cross-rs` and refreshed dependencies                                |

There has never been a 1.0 release.

## Maturity and Maintenance

The crate is best characterized as **small, usable, but pre-1.0 and lightly maintained**.

Positive indicators include:

- A deliberately narrow implementation.
- Complete rustdoc coverage for the public API.
- Dual MIT/Apache-2.0 licensing.
- Caching and an offline workflow.
- Tests for parsing, transformations, missing icons, and basic API output.
- Fixes for reported offline and cross-compilation problems.

Cautionary indicators include:

- Latest publication: **April 20, 2024**.
- Latest merged commit on the default branch: **July 18, 2024**.
- No 1.0 stability commitment.
- A single public macro and a small contributor/user base.
- Approximately 13 GitHub stars and one current crates.io reverse dependency.
- Open dependency-update pull requests and an unresolved request for nonliteral inputs.
- Doctests are disabled because they require network access.
- Tests depend on exact output from a mutable external API.
- The current offline feature declaration does not fully describe its implementation dependency on `cache`.

The crate is reasonable for applications that need a modest, fixed set of inline icons and accept procedural-macro-based asset acquisition. For long-lived, security-sensitive, reproducible, or heavily regulated builds, use its offline workflow and treat prepared SVGs as reviewed source assets. For dynamic icon catalogs or large icon collections, a runtime client or dedicated asset-generation pipeline is a better architectural fit.
