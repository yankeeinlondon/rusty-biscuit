# Review: `style:` Frontmatter Sub-Spec #1

This is a useful decomposition, and the "schema first, wiring later" split is
reasonable. The main improvements I would ask for before implementation are
about anchoring the schema to the existing renderable contracts and making the
intermediate state less noisy for users.

## Scope Gaps

### Gap: no canonical mapping to `renderable::layout::Layout` and `renderable::style::Style`

The spec defines a new `darkmatter::style` schema with `HorizontalLength`,
`Color`, `Alignment`, and `CommonStyle`, but it does not describe how these map
onto the canonical primitives already established in `renderable`:

- `renderable::layout::Length`
- `renderable::layout::TargetValue`
- `renderable::layout::Alignment`
- `renderable::style::Style`
- `renderable::color::Color`

That is the largest missing piece. Without an explicit adapter contract, the
subsequent wiring specs can easily drift into a fourth style/layout model
beside the render tree model, darkmatter's deprecated page-layout types, and
component-specific style structs.

Suggested ways to address this:

1. **Parse into a frontmatter schema, then provide explicit conversion helpers.**
   Keep `darkmatter::style::StyleFrontmatter` as an input-only type, but define
   conversion methods such as `PageStyle::to_layout_patch()`,
   `CommonStyle::to_renderable_style()`, and
   `CommonStyle::to_component_layout_patch()`. The schema stays close to the
   user-facing YAML while the conversion boundary makes the canonical runtime
   types clear.

2. **Parse directly into wrapper types around renderable primitives.**
   For example, `HorizontalLength` could be a deserialization wrapper that
   immediately produces `renderable::layout::Length::Ch` or
   `Length::Percent`, and `Alignment` could simply reuse
   `renderable::layout::Alignment` with a custom deserializer for the
   `centered` alias. This reduces type duplication, but can make frontmatter
   error messages slightly harder to tailor.

3. **Define a two-layer model: raw schema plus resolved style plan.**
   The parser returns the raw sparse schema, and a second `StylePlan` or
   `ResolvedStyle` translates it into target-agnostic layout/style operations.
   This is more ceremony, but it gives sub-specs #2-#7 a shared output shape
   and gives strict-mode diagnostics a natural place to distinguish parse
   errors from unsupported render effects.

### Gap: no source-location strategy for diagnostics

The spec asks for warning paths like `style.page.lft-margin`, but it does not
say whether diagnostics should eventually include source line/column. Existing
frontmatter retains raw YAML specifically so downstream systems can report
positions against the original source. A style parser that only reports string
paths will be useful initially, but it should not paint the implementation into
a corner.

Suggested approach: add a small "diagnostic position" note. Sub-spec #1 can
ship path-only warnings, but the parser should avoid APIs that make later
source spans impossible. For example, keep `StyleWarning.path` as required and
add an optional `source_span` or defer a `StyleDiagnostic` shape that can be
extended without breaking callers.

### Gap: strict mode is specified before the CLI/reporting contract exists

The spec says `--strict-style` upgrades warnings to errors, but CLI precedence
rules are explicitly out of scope. That creates a partial user-facing contract
inside a parser-only spec.

Two viable alternatives:

1. Keep `into_strict` in scope as a pure library helper, but remove the
   `--strict-style` flag from sub-spec #1 acceptance and out-of-spec text.
   Sub-spec #2 can introduce the CLI flag when it wires rendering.

2. Keep the flag mentioned as future behavior only, and define the parser API
   around a `StyleParseMode` or `StyleDiagnosticsMode` enum so the CLI can
   attach policy later without changing the parser.

## Repo Contract Mismatches

### Accidental: `serde_yaml::Value` does not match darkmatter frontmatter storage

The public API proposes:

```rust
pub fn from_yaml_value(value: &serde_yaml::Value)
```

Darkmatter's `Frontmatter` stores an `IndexMap<String, serde_json::Value>`, and
typed accessors deserialize through `serde_json::from_value`. The parser keeps
raw YAML text separately for source-aware diagnostics, but the normal structured
frontmatter contract is JSON values, not `serde_yaml::Value`.

This looks accidental rather than an intended standards change. If the spec
intends to introduce a YAML-value parsing path, call that out explicitly and
explain why style parsing differs from the rest of `Frontmatter`. Otherwise,
prefer one of these:

- `from_frontmatter(&Frontmatter)` plus an internal read from
  `frontmatter.as_map().get("style")`
- `from_json_value(&serde_json::Value)`
- `from_raw_yaml(style_yaml: &str)` only for the future source-location path

Also use the repo's existing YAML crate naming in implementation notes
(`biscuit_file::serde_yaml_ng`) rather than `serde_yaml`, unless the spec is
intentionally adding a new YAML dependency.

### Accidental: new `Color` enum duplicates `renderable::color::Color`

The spec defines a new `Color` with only Tailwind and hex. The repo already has
`renderable::color::Color`, `Tailwind`, `RgbColor`, `WebColor`, default colors,
and renderer-side lowering. Creating a darkmatter-local color enum would split
the color contract and force sub-spec #5 to translate between two similar but
not equivalent models.

This appears accidental. A better direction is to specify a frontmatter color
deserializer that returns `renderable::color::Color`, probably with an
additional wrapper if opacity must be preserved for HTML. If opacity is part of
the public style contract, name that as the real delta from the existing
`Color` type: for example `StyleColor { color: renderable::color::Color,
opacity: Option<u8> }`.

### Accidental: `Alignment` and length types duplicate canonical layout types

`renderable::layout::Alignment` already exists and uses `snake_case` serde.
`renderable::layout::Length` already models `Ch(u32)` and `Percent(f32)`.
The draft's `Alignment` and `HorizontalLength` should be described as parsing
adapters, not new semantic types, unless the intent is to keep darkmatter's
legacy page-layout contract alive longer.

For `WidthUnit`, the spec says "reuse `darkmatter::layout::WidthUnit` is
preferred" and then suggests widening it from `u16` to `u32`. That type is
already documented as deprecated in favor of `renderable::layout::Layout`.
Widening it would be an accidental change to a deprecated compatibility type.
Prefer parsing `2ch` / `50%` into `renderable::layout::Length`, then convert to
legacy `WidthUnit` only at the sub-spec #2/#3 boundary if the old
`DarkmatterPage` path still needs it.

### Intended, but should be made explicit: `block_quote` key shape

The parent doc lists `block_quote`, and the draft preserves that key while
using a Rust field named `block_quote` under `#[serde(rename_all =
"kebab-case")]`. That combination will serialize as `block-quote`, not
`block_quote`, unless an explicit rename is added.

If the intended user-facing key is `block_quote`, add
`#[serde(rename = "block_quote", alias = "block-quote")]` or equivalent. If
the intended standard is changing to `block-quote`, say that this is an
intentional cleanup and add a compatibility alias for the documented spelling.

### Accidental: `bg_color` will not be handled by `rename_all = "kebab-case"`

The parent doc names `bg_color`; `#[serde(rename_all = "kebab-case")]` expects
`bg-color`. The same issue exists for `local_style`, which would become
`local-style`. Some parts of the draft use underscores because the parent doc
does, while other parts use hyphens because the style-prop fixture does.

The spec should define one canonical frontmatter spelling policy and aliases:

- Canonical kebab-case: `max-width`, `bg-color`, `local-style`,
  `block-quote`
- Compatibility aliases for existing documented snake_case names:
  `max_width`, `bg_color`, `local_style`, `block_quote`

or the reverse. The important part is to make intended spelling changes clear
instead of relying on serde defaults that silently change the documented
contract.

### Accidental: `serde_ignored` cannot collect all typos if unknown fields are accepted

The sketch says "`UnknownKey` is the default for any path serde doesn't
recognize," but all schema structs are shown with `#[serde(default)]` and no
`deny_unknown_fields`. Serde normally ignores unknown fields unless the target
struct denies them; `serde_ignored` only helps if the deserializer surfaces
ignored fields in the way the crate expects. This needs a proof point in the
spec, especially through flattened structs.

Suggested improvement: add a short spike task or acceptance criterion proving
that `serde_ignored` catches:

- unknown root buckets
- unknown keys inside flattened `CommonStyle`
- unknown keys inside nested `local_style`
- both kebab-case and alias spellings

If it cannot do this cleanly with `serde_json::Value`, the spec should switch
to a manual map-walk parser for key collection and then deserialize known
subtrees.

### Accidental: `NotYetImplemented` warnings make clean parsing impossible

The spec says every known-but-unwired key emits `NotYetImplemented`, and then
the strict helper turns warnings into errors. Since all style keys are unwired
in sub-spec #1, `md --strict-style style-prop.md` would fail on a document that
is syntactically valid and future-compatible. That may be useful during a short
internal migration, but it is hostile as a public default for "strict schema"
validation.

Consider splitting diagnostics into categories:

- `UnknownKey` / `Deprecated` are strict-mode errors.
- `KnownButInactive` is an informational rendering diagnostic, not a schema
  warning.
- `UnsupportedForTarget` belongs to the renderer or style-plan resolver, not
  the parser.

That preserves the "silent ignore becomes audible" goal without making strict
schema validation fail on valid keys.

## Wording Improvements

- Replace "`Style` AST" with "`style frontmatter schema`" or "`style
  configuration schema`". In this repo, the canonical intermediate
  representation is the render tree, and the renderable skill explicitly avoids
  calling it an AST. Calling this parser output an AST adds unnecessary
  ambiguity.

- Replace "Length typing" with "frontmatter length parsing" or "style length
  deserialization". The runtime type should be the existing layout `Length`;
  the spec is really defining accepted user syntax.

- In "No cascading / inheritance semantics", distinguish frontmatter
  propagation from render-tree style inheritance. The repo already has a
  `renderable::style::Style::inherited_from` contract where only `color` and
  `emphasis` inherit. Suggested wording:

  > This spec does not resolve graph-level style propagation from composed
  > parent documents to child documents. It also does not change
  > render-tree style inheritance; runtime inheritance remains limited to the
  > fields documented by `renderable::style::Style`.

- In "Color parsing in scope", avoid saying this prevents a schema migration
  later. It only prevents that if the parser reuses or cleanly adapts to
  `renderable::color::Color`. Suggested wording:

  > Parse color syntax now, but lower it through the existing
  > `renderable::color` model so sub-spec #5 does not need a second color
  > migration.

- "`Box` keeps the `Option<CommonStyle>` size flat" is an implementation detail
  that distracts from the contract. I would remove it from the spec unless a
  measured size concern exists.

- "None block writing the spec" should be "None block implementation" in Open
  Questions, because this document is already the spec draft.

## Suggested Acceptance-Criteria Additions

- Parsing does not introduce a second runtime layout/style/color model; any
  local schema wrappers have documented conversions to renderable primitives.
- Existing documented snake_case keys and fixture kebab-case keys are either
  both accepted or the spec explicitly names the intentional spelling change.
- Unknown-key collection is proven for flattened structs and nested
  `local_style`.
- The parser operates on darkmatter's actual `Frontmatter` representation
  (`serde_json::Value` map plus optional raw YAML), or the spec explicitly
  justifies a new YAML-value path.
- Strict mode can validate the schema without failing solely because a known
  style key has not been wired to rendering yet.
