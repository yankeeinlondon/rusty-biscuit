# Browser Utilities for Component Authors

Helpers exposed from `renderable::browser::utils` to remove the rote, error-prone
work component authors otherwise reinvent. Scope is deliberately narrow: every
helper here either prevents a correctness bug (escaping) or eliminates obvious
boilerplate that appears in more than one component.

Out of scope: ARIA shortcuts, `data-*` shortcuts, URL validation, microdata
wrappers. The typed `HtmlAttribute` enum already covers these, or they belong
on the `BrowserFragment` builder itself.

## Module layout

```text
renderable::browser::utils
├── escape          // text + attribute escaping
├── class           // class-name joining + CSS-identifier slugging
├── style           // inline style builder, CSS var references
└── color           // Color → CSS string (lives here until renderable::color lands)
```

Each submodule re-exports its public surface at `browser::utils` so consumers
write `use renderable::browser::utils::{escape_text, classes, Style};` rather
than walking the nested path.

## 1. HTML escaping

Two distinct functions because the rules differ. Text-content escaping that
leaks into an attribute value produces broken markup; attribute escaping that
leaks into text content over-escapes harmless characters.

### `escape_text`

```rust
pub fn escape_text(input: &str) -> Cow<'_, str>;
```

Escapes the three characters that change parser state inside element content:

| Character | Replacement |
|-----------|-------------|
| `&`       | `&amp;`     |
| `<`       | `&lt;`      |
| `>`       | `&gt;`      |

`Cow<'_, str>` so the common case (no special characters) returns the input
borrowed, with no allocation.

### `escape_attribute`

```rust
pub fn escape_attribute(input: &str) -> Cow<'_, str>;
```

Escapes the four characters relevant inside a double-quoted attribute value:

| Character | Replacement |
|-----------|-------------|
| `&`       | `&amp;`     |
| `<`       | `&lt;`      |
| `"`       | `&quot;`    |
| `'`       | `&#39;`     |

`>` is intentionally left alone — it is not special inside an attribute value
and skipping it keeps output more readable.

### Usage boundary

The rendering layer inside `BrowserFragment::render()` is the **only** caller
that must invoke these. Component authors who construct `HtmlAttribute` or
push `ComposableNode::TextFragment(String)` pass raw strings; rendering escapes
on emit. This keeps escaping centralized and idempotent.

The functions are public anyway so that components hand-assembling unusual
strings (e.g. composing a `style` value with user-supplied input) can opt in.

## 2. Class joining

### `classes`

```rust
pub fn classes<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = Option<S>>,
    S: AsRef<str>;
```

Joins class-name parts with a single space. `None` and empty strings are
filtered. Whitespace inside a part is preserved (a caller may legitimately
pass `"a b"`); leading/trailing whitespace on a part is trimmed before
filtering so `Some("")` and `Some("   ")` are both dropped.

```rust
let cls = classes([
    Some("simple-table"),
    striped.then_some("simple-table--striped"),
    size.map(|s| format!("simple-table--{s}")).as_deref(),
]);
// → "simple-table simple-table--striped simple-table--lg"
```

This is the only helper that touches `Option`; component authors almost
always have modifier flags they want to project into class names.

## 3. CSS identifier slugging

### `css_slug`

```rust
pub fn css_slug(input: &str) -> String;
```

Converts an arbitrary string into a CSS-safe identifier segment suitable for
use as a class-name suffix or `ComponentStylesheet::add` key.

Rules:

1. Unicode normalize (NFKD), drop combining marks.
2. Lowercase ASCII letters/digits pass through.
3. Any other character becomes `-`.
4. Consecutive `-` collapse to one; leading/trailing `-` are trimmed.
5. If the result starts with a digit, prepend `_`.
6. If the result is empty, return `_`.

```rust
css_slug("Order ID")       // "order-id"
css_slug("Café au lait")   // "cafe-au-lait"
css_slug("2024 totals")    // "_2024-totals"
css_slug("---")            // "_"
```

Component authors typically call this when projecting a column header,
enum variant, or i18n key into a class name. `ComponentStylesheet::add`
does **not** call this implicitly — the caller decides whether their input
is already slug-safe.

## 4. Inline style builder

### `Style`

```rust
pub struct Style { /* Vec<(String, String)> */ }

impl Style {
    pub fn new() -> Self;
    pub fn set(self, property: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn set_opt(self, property: impl Into<String>, value: Option<impl Into<String>>) -> Self;
    pub fn into_attribute(self) -> HtmlAttribute;
    pub fn to_css_string(&self) -> String;
}
```

Builds the value of a `style="…"` attribute. Stored as `Vec` (not `HashMap`)
so declaration order is preserved — relevant when emitting shorthand/longhand
pairs.

`set_opt` is the high-payoff method. Components frequently want to set
`background-color` only when a color was provided, and `set_opt` keeps the
fluent chain unbroken:

```rust
let style = Style::new()
    .set("color", fg.to_css())
    .set_opt("background-color", bg.map(|c| c.to_css()))
    .set("padding", "0.5rem 1rem");

block = block.add_attribute(style.into_attribute());
```

`into_attribute()` returns a `HtmlAttribute::Style(…)` variant (or whatever
shape the attribute model settles on); the helper centralizes that mapping
so component code never builds the attribute string by hand.

Values are stored raw — the rendering layer escapes when emitting the
attribute. The helper does **not** validate property names or values; that's
out of scope until the typed CSS model from `stylesheet-extraction.md`
lands.

## 5. CSS variable references

### `css_var`

```rust
pub fn css_var(name: &str) -> String;
pub fn css_var_with_fallback(name: &str, fallback: &str) -> String;
```

Returns `"var(--name)"` and `"var(--name, fallback)"` respectively. The
leading `--` is added by the helper; callers pass the bare name.

```rust
Style::new()
    .set("color", css_var("brand-fg"))
    .set("border-color", css_var_with_fallback("brand-border", "#ccc"));
```

The fallback is **not** escaped or quoted — it is emitted verbatim, matching
the raw-string contract of every other CSS value in this module.

## 6. Color helper

`Color` will live in `renderable::color` once moved over. Until then, this
module re-exports a `Color` placeholder and a `to_css` method so component
code does not need to be rewritten when the type relocates.

### Surface

```rust
pub enum Color {
    Rgb { r: u8, g: u8, b: u8 },
    Rgba { r: u8, g: u8, b: u8, a: f32 },
    Hsl { h: f32, s: f32, l: f32 },
    Named(&'static str),
    Var(String),   // CSS variable reference
}

impl Color {
    pub fn to_css(&self) -> String;
}
```

`to_css` chooses the shortest correct representation:

| Variant         | Output                          |
|-----------------|---------------------------------|
| `Rgb`           | `#rrggbb`                       |
| `Rgba` (a=1.0)  | `#rrggbb`                       |
| `Rgba` (a<1.0)  | `rgb(r g b / a)` (modern syntax)|
| `Hsl`           | `hsl(h s% l%)`                  |
| `Named`         | the literal name                |
| `Var`           | `var(--name)`                   |

Modern space-separated syntax over legacy comma syntax — all evergreen
browsers support it and it composes better with `color-mix()` and relative
color syntax when those land.

### Migration path

When `renderable::color` lands:

1. Move the enum, keep `to_css` on it.
2. `browser::utils` re-exports `Color` from the new location.
3. Component code is unchanged.

This is the single reason `Color` lives under `utils` to start — it's a
known-temporary home, not a permanent one. Mark the re-export with a
`// TODO: re-export from renderable::color once moved` comment so it's not
forgotten.

## Testing strategy

Each function has unit tests in-module. Escaping and slug functions are
property-tested with `proptest`:

- `escape_text` / `escape_attribute`: parsing the result back through a
  CommonMark-style HTML parser yields the original string.
- `css_slug`: result matches `^[_a-z][_a-z0-9-]*$` and is idempotent
  (`css_slug(css_slug(x)) == css_slug(x)`).

`Style` and `Color` have example-based tests only; their behavior is
narrow enough that property tests would not pay off.

## Non-goals

- **URL handling** — the `url` crate covers validation; escaping at emit
  time is handled by `escape_attribute`.
- **ARIA / `data-*` shortcuts** — `HtmlAttribute` is the right home.
- **Microdata wrappers** — `BrowserFragment::add_metadata_keypair` exists.
- **CSS unit constructors** (`px(8)`, `rem(1.5)`) — these belong on the
  typed CSS value model in `stylesheet-extraction.md`, not here.
- **Element-builder shortcuts** (e.g. a one-liner `div(class, children)`)
  — the typestate builder is the public surface; sugar that bypasses it
  fragments the API.
