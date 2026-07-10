# Spec: Prettier code blocks

- **Date:** 2026-07-10
- **Trigger:** Claudine's streamed assistant output renders code blocks as full-bleed, inverse-theme panels — edge-to-edge slabs whose background inverts the page (dark page → bright light panel), visually dominating the surrounding prose.
- **Status:** DRAFT — awaiting Ken's review
- **Areas:** `darkmatter` (lib + cli), `claudine` (lib + cli)

## Problem

Three compounding issues:

1. **Layout.** Code blocks span the full terminal width with no horizontal
   margin and no width cap. On a wide terminal a short YAML snippet becomes a
   200-column background slab.
2. **Contrast.** The default code-block theme variant is `inverse`
   (dark page → light panel). Good for making a block "pop"; too aggressive as
   the *default* — the page in the screenshot is mostly panel, barely prose.
3. **No override surface.** There is no way for a user or a repo to change
   these defaults for Claudine's rendering. The knobs that do exist
   (`md --code-block`, `style:` frontmatter) only apply to documents rendered
   through the `md` CLI, not to Claudine's streamed provider output.

## Current State (verified 2026-07-10)

### Darkmatter

- **`CodeBlockMode`** — `darkmatter/lib/src/markdown/highlighting/themes.rs:30`
  (`Inverse` | `Dark` | `Light` | `Same`, with `FromStr`/`Display`). The
  `inverse` default is set at **two independent layers**:
  - library: `#[default]` on `Inverse` (`themes.rs:33`), inherited by
    `TerminalOptions::default()` (`markdown/output/terminal.rs:746`) and
    `DarkmatterPage::new` (`layout/page.rs:158`);
  - CLI: clap `default_value_t = CodeBlockArg::Inverse`
    (`darkmatter/cli/src/args/cli.rs:33`).
- **Style system** — the `style:` frontmatter `code-block` bucket
  (`CodeBlockStyle`, `lib/src/style/schema/components.rs:32`) flattens the
  shared `CommonStyle` (`lib/src/style/schema/common.rs:391`). `max-width` is
  **already supported** there. Margins are **not** expressible per-side:
  `margin` is an all-four-sides shorthand (`ComponentEdges`,
  `common.rs:118-155`). Only the `page` bucket (and `ul.left-margin`) has
  per-side keys.
- **Layout engine lives in `DarkmatterPage`** (`lib/src/layout/page.rs`).
  Component styles reach rendering via
  `apply_component_style(page, &StyleFrontmatter, overrides)`
  (`lib/src/style/apply.rs:388`) → `PageComponent::CodeBlocks`.
  `Markdown::as_terminal(TerminalOptions)` — the embeddable path — **cannot
  observe any of this**; `TerminalOptions` carries themes/modes but no
  component layout. A default `DarkmatterPage` renders byte-for-byte identical
  to `as_terminal(TerminalOptions::default())` (documented invariant,
  `page.rs:56-89`).
- **Schema** — `darkmatter/docs/schemas/darkmatter.yaml` (SimplifiedSchema,
  `include_str!`-embedded at `lib/src/markdown/schemas/mod.rs:118`).
  `page.code.theme` is a free `string` (line 42-43) — not enumerated. The
  `code-block` bucket is `*style-common` (line 126) — no theme, no variant
  mode. There is **no** `kind: schema` / `types:` construct in
  SimplifiedSchema; named types are the file's **top-level `$schema:`
  entries**, imported elsewhere via `Name@fileref` / `Name@this`
  (`lib/src/markdown/schemas/resolve.rs:747`, `simplified/types.rs:156`).
- **`ThemePair`** — nine variants (`themes.rs:111`, `#[non_exhaustive]`), kebab
  names: `base16-ocean`, `github`, `gruvbox`, `one-half` (the fallback
  default), `solarized`, `nord`, `dracula`, `monokai`, `vs-dark`.
  > Corrections vs. the request sketch: `Base160Ocean` → `Base16Ocean`,
  > `Grubbox` → `Gruvbox`; `OneHalf` was missing. `Nord`/`Dracula` are
  > dark-only (light resolves to `OneHalfLight`); `VisualStudioDark`'s light
  > variant resolves to `GithubLight`.

### Claudine

- The streamed render path is `AssistantStream`
  (`claudine/lib/src/render/assistant_stream.rs`) → `FinalMessage::render`
  (`claudine/lib/src/render/final_message.rs:45`) →
  `Markdown::new(text).as_terminal(opts)`. The only options construction site
  is `new_assistant_stream()`
  (`claudine/cli/src/commands/wrap/exec/mod.rs:123`):
  `TerminalOptions::default()` with `image_mode = Never`. **Claudine sets no
  code-block styling anywhere** — the inverse slab is inherited darkmatter
  default. `DarkmatterPage` is unused in claudine today.
- Config is **JSON5, not YAML**: user `~/.claudine/config.json`
  (`ClaudineConfig`, `claudine/lib/src/config/claudine_config.rs:185`,
  `deny_unknown_fields`), repo `<repo_root>/.claudine/config.json`
  (`RepoOverrideConfig`, `claudine_config.rs:337`), merged by
  `merge_repo_override` (`claudine/lib/src/config/merge.rs:20`). No `session`
  section, no style settings exist. Config loading performs **no**
  `FileReference` expansion today (that lives only in composition/sequence).

## Changes

Ordered so each lands independently; 1 and 2 are darkmatter-only, 3-5 build on
them.

### 1. Darkmatter: default `CodeBlockMode` flips `inverse` → `same`

The new default — for Darkmatter and everything downstream — is `same`: the
code panel uses the page's own color-mode variant. The panel still paints the
syntax theme's background (ratified 2026-06: code blocks always paint their
theme bg, even on transparent pages), so blocks remain visually distinct
panels — just no longer polarity-flipped.

- Move `#[default]` from `Inverse` to `Same`
  (`themes.rs:33`). `TerminalOptions` and `DarkmatterPage` inherit
  automatically.
- Flip the CLI default: `default_value_t = CodeBlockArg::Same`
  (`cli/src/args/cli.rs:33`) and reorder/annotate the `--code-block` help so
  `same` reads as the default and `inverse` as an explicit choice.
- **Doc/test drift pass (required by repo comment discipline):** the
  `TerminalOptions.code_block_mode` docblock ("`Inverse` (default)"),
  `CodeBlockMode` rustdoc, `code_block_mode_default_is_inverse` test
  (`themes.rs:1104`), any README/skill text quoting the inverse default, and
  snapshot/L2 assertions that encode an inverted panel background.

### 2. Darkmatter: per-side margins on component style buckets

`code-block.max-width` already works; the blocker for "left-margin 4ch /
right-margin 4ch" is that component buckets only accept the all-sides `margin`
shorthand (an all-sides `4ch` would also add 4 *rows* above/below — not
wanted).

- Add `left-margin` and `right-margin` (type `Length`, same `&style-length`
  grammar as the page bucket) to `CommonStyle`, following the existing
  `ul.left-margin` precedent. Top/bottom stay shorthand-only until a concrete
  need appears (Simplicity First).
- Precedence: a per-side key overrides the `margin` shorthand for that side.
- Wire through `apply_common_style` → `PageComponent` margins; document the
  `width`/`max-width` conflict rule unchanged.
- Schema: extend the `&style-common` anchor in `darkmatter.yaml` with the two
  keys (all buckets sharing the anchor gain them — table, images, block-quote,
  code-block — which is coherent, not scope creep, since the layout engine is
  shared).

### 3. Darkmatter: schema reorganization for theming

Two gaps: `page.code.theme` is an un-enumerated string tucked under `page`,
and the code-block *variant mode* (`inverse|dark|light|same`) is CLI-only —
not settable from `style:` frontmatter at all, which change 5 needs.

- **New named types** (SimplifiedSchema's actual mechanism — top-level
  `$schema:` entries imported via `Name@fileref`, not the `kind:`/`types:`
  sketch, which doesn't exist): add
  `docs/schemas/style-components.yaml` declaring:

  ```yaml
  $schema:
      theme: "enum(base16-ocean, github, gruvbox, one-half, solarized, nord, dracula, monokai, vs-dark) -> Syntax-highlighting theme pair (each has light + dark variants)."
      code-block-variant: "enum(inverse, dark, light, same) -> How the code panel's theme variant is chosen relative to the page color mode."
  ```

- **Move theming into the `code-block` bucket** where it belongs:
  - `code-block.theme: theme@./style-components.yaml`
  - `code-block.variant: code-block-variant@./style-components.yaml`
  - `page.code.theme` stays as a **deprecated alias** of `code-block.theme`
    (schema description marks it deprecated; loader maps it; removal is a
    later cleanup).
- Rust side: `CodeBlockStyle` gains `theme: Option<ThemePair>` and
  `variant: Option<CodeBlockMode>` (serde reuses the existing `FromStr`
  vocabularies); `apply_component_style` routes them to
  `with_code_theme` / `with_code_block_mode`. CLI flags keep authority via the
  existing `PageStyleOverrides`/claims pattern.

### 4. Claudine: adopt `DarkmatterPage` + built-in code-block defaults

`as_terminal` cannot express margins, so claudine's render path migrates to
the page abstraction (this is the strategic fix — it also gives claudine the
entire style system for free, which change 5 requires):

- `FinalMessage` (and by extension `AssistantStream` /
  `ThinkingStream`) renders through a configured `DarkmatterPage`
  (`page.render(&Markdown)`) instead of `Markdown::as_terminal`. The page
  carries the existing `TerminalOptions` (`with_terminal_options`,
  `page.rs:705`), preserving `image_mode = Never`. At defaults this is
  byte-identical (documented parity invariant), so the migration itself is
  behavior-neutral and separately committable.
- Claudine's **built-in default style**, expressed as a `StyleFrontmatter`
  constant in the claudine lib render module (so it is the same shape a repo
  override file uses):

  ```yaml
  code-block:
      left-margin: 4ch
      right-margin: 4ch
      max-width: 120ch
  ```

  Applied via the public `apply_*` functions with empty overrides. The
  variant mode is *not* pinned here — claudine simply inherits darkmatter's
  new `same` default (change 1), so a future darkmatter default change flows
  through.
- The prompt-preview path (`render/prompt/formatting.rs`) is out of scope; it
  renders user-authored prompts, not provider output.

### 5. Claudine config: `session.default_style`

Let a user or repo replace the built-in default style wholesale.

- New optional `session` section in **both** `ClaudineConfig` and
  `RepoOverrideConfig` (both are `deny_unknown_fields`, so both structs must
  declare it):

  ```json5
  {
      session: {
          // file reference to a style document
          default_style: "./.claudine/style.yaml",
      },
  }
  ```

  > The request sketch showed YAML config; claudine config is JSON5
  > (`config.json`). The referenced style **document** is YAML.

- **Value semantics:** a file reference resolved via
  `biscuit_file::FileReference` (first use of FileReference in the config
  layer). Resolution anchor: repo root for the repo config, `~` for the user
  config. Merge rule: repo `session` replaces user `session` when present
  (consistent with `guard_settings` replacement semantics in
  `merge_repo_override`).
- **Style document shape:** top-level buckets identical to `style:`
  frontmatter (`page`, `code-block`, `table`, …) — "override the defaults any
  way they please", not just code blocks. When the document declares
  `$schema:`, claudine validates it through the existing SimplifiedSchema
  machinery (same as composition schema validation) and surfaces typed errors;
  darkmatter ships the authoritative schema (change 3), so a style file can
  point at it.
- **Precedence (lowest → highest):**
  1. darkmatter built-ins (`same` variant, no margins)
  2. claudine built-in default style (change 4)
  3. user `session.default_style`
  4. repo `session.default_style`
  5. document `style:` frontmatter (compose paths only; provider stream
     fragments have no frontmatter)
- A `default_style` that fails to resolve or parse is a **launch-blocking
  config error** (consistent with the ratified file-reference fatality rule:
  a present reference that fails to resolve is fatal), reported with the
  standard styled config-error output.

## Testing

- **Darkmatter L1:** default-flip tests (`CodeBlockMode::default()`,
  `TerminalOptions::default()`, CLI clap default), per-side margin
  deserialize + precedence-over-shorthand, `code-block.theme`/`variant`
  apply-routing, schema named-type import round-trip, `page.code.theme`
  alias mapping.
- **Claudine L1:** `session` section parse (user + repo), repo-replaces-user
  merge, FileReference resolution anchors, invalid-reference fatality,
  built-in default `StyleFrontmatter` constant applies cleanly.
- **Claudine L2:** captured-frame assertions that a streamed fenced block
  renders with 4-column left/right insets and caps at 120 columns on a wide
  pane, and that the panel background matches the page variant (no polarity
  flip). Reuse the tmux/WezTerm + `FORCE_COLOR=1` capture pattern; mind SGR
  collapsing (semantic checks, not byte equality).
- **Regression watch:** darkmatter snapshot tests asserting inverse panels;
  `md` CLI help text tests; the DarkmatterPage/as_terminal byte-parity tests
  (parity holds only at *darkmatter* defaults — claudine's margins are applied
  on top, not baked into the parity fixture).

## Open Questions

1. **`style@…` syntax.** The request example
   `default_style: style@darkmatter/docs/schemas/darkmatter.yaml` points at
   the *schema* file using what reads as the SimplifiedSchema named-type
   import form (`Name@fileref`). This spec interprets `default_style` as a
   plain file reference to a *style document* (values, not schema), with
   `$schema:` inside that document handling validation. Confirm, or specify
   what `style@` should denote (e.g. a typed-reference syntax meaning "a file
   conforming to type `style` from that schema").
2. **`page.code.theme` deprecation.** This spec keeps it as an alias of
   `code-block.theme`. OK to mark deprecated now and remove in a later
   cleanup?
3. **Per-side scope.** Left/right margins only (this spec), or all four sides
   on component buckets while we're in there?
4. **Section name.** `session` was requested; noting these settings govern
   rendering rather than session lifecycle — `session` is fine if the intent
   is "settings for a claudine session," flagging only in case `render` or
   `style` reads better long-term.
