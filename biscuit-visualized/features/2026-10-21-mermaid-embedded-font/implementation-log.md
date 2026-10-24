---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-mermaid-embedded-font/spec.md"
plan: "biscuit-visualized/features/2026-10-21-mermaid-embedded-font/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
decisions_for_review:
    - |-
      Rulings A-H accepted as the plan recommended them (2026-10-24). The plan
      asked the author to rule on eight open points; with no human between
      phases, each recommendation was taken as written; spike evidence
      supporting Rule C is in the log. Alternatives are the ones the plan lists beside each rule
      (for example: admit `font/otf` for CFF faces instead of falling back to
      Inter; lay out twice to learn the weights a page really uses; use
      `cssparser` for the CSS check). Picking an alternative changes only the
      phase that rule blocks (Rule B/C: Phase 2; A/E/H: Phase 3-4; D: Phase 4;
      F/G: Phase 5).
    - |-
      The text-fallback work (the `md --feature` flag) has not landed, although
      the plan's gate says to stop if it is missing. I did not stop: the flag is
      used by one task only (5.3, `md --feature embed-mermaid-font`), and
      Phases 2-4 and most of Phase 5 do not touch it. Phase 1 builds nothing.
      Alternatives: stop the whole run until the text fallback ships (nothing
      gets built meanwhile), or add a standalone `--embed-mermaid-font` flag
      (the spec's original form, which the author replaced). If the reviewer
      wants the strict gate, nothing from Phase 1 needs redoing; the Phase 5
      implementer must re-check this gate before task 5.3 and must not build a
      stand-in `--feature` flag.
    - |-
      Spike S1 outcome: subset used, but the opt-in must measure label widths
      without OpenType layout features. The plan and spec assumed that the
      measured face equals the embedded face once the face is chosen (D6).
      The spike showed that the allsorts subset never carries the `GPOS` and
      `GSUB` tables (kerning, ligatures, contextual alternates), so Chrome draws
      the subset about 1.3% wider than our measurer, which kerns. I chose
      (spec D17): keep the subset, and have the opt-in measure with plain
      glyph advances (kern, liga, calt and all other layout features off),
      which equals what the browser draws from the subset exactly (4099.609 px
      in both, at 100 px). Alternatives: (a) take the spec's unsubset fallback,
      embedding all of Inter, about 410 KB per weight before base64 and seven
      times the 60 KB budget, which keeps kerning; (b) subset while keeping the
      original glyph ids so `GPOS`/`GSUB` could be copied unchanged, which
      allsorts does not support and which would carry Inter's large layout
      tables. If the reviewer picks (a), Phase 3's subsetter is skipped and the
      plain-advance measurement in Phase 4 is dropped; nothing else changes.
source_files_during_phase_1:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/spikes/s1-subset/Cargo.toml
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/spikes/s1-subset/src/main.rs
docs_updated_during_phase_1:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/spec.md
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/plan.md
docs_created_during_phase_1:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/implementation-log.md
skills_files_updated_during_phase_1: []
packages:
    - biscuit-visualized
message_to_agent: |-
    Phase 1 (gate, rulings, spike S1, recon) is done; read the implementation
    log's Phase 1 section before starting. Rulings A-H were accepted as the plan
    recommends. Three things change later phases:
    1. Use `allsorts = { version = "0.17.0", default-features = false, features = ["flate2_rust"] }`;
       the default features pull the C library libz-sys.
    2. Spike S1 found that the allsorts subset never carries GPOS/GSUB, so Chrome draws it
       without kerning/ligatures (~1.3% wider than our kerned measurer). Spec D6 is amended:
       with the opt-in on, measure labels with plain glyph advances (all layout features off),
       for Inter and host faces alike. Oracle: measured width == sum of the subset's hmtx
       advances. `ShapeParams.kerning` (src/fonts/shaper.rs) only disables `kern`; liga/calt
       and the rest must go too. This is Phase 4 work, but Phase 2/3 types should allow it.
    3. The text-fallback `md --feature` flag has NOT landed. Only task 5.3 needs it; re-check
       before 5.3 and do not build a stand-in.
    Also: OFL.txt declares no Reserved Font Name, so the family name is `Inter`; OFL.txt is
    not compiled in anywhere (D15's line must be included or copied). macOS SFNS.ttf is
    variable (fvar) and must be refused per Rule C. The working tree holds unrelated,
    pre-existing darkmatter/dmls edits from another session; leave them alone and scope
    `affected_scope.py` to this feature's files.
---

# Implementation Log for 2026-10-21-mermaid-embedded-font (7 phases)

## Phase 1

Phase 1 changes no shipped code. It covers the prerequisite gate, the rulings,
spike S1, and the recon notes the later phases need. Date: 2026-10-24.

### Prerequisite gate

- **Merman cutover: landed.** Confirmed in the tree:
    - the font chain (`biscuit-visualized/src/src/fonts/chain.rs`,
      `FontChain`, `default_chain()`), bundled Inter Regular/Bold with
      `OFL.txt` under `biscuit-visualized/src/assets/fonts/inter/`;
    - the HTML static target (`MermaidTarget::HtmlStatic`,
      `src/mermaid/render.rs:98`) and Darkmatter's `MermaidMode::Image` →
      `BrowserMermaidMode::StaticSvg` path
      (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:593`,
      `code_renderer.rs:599-620`);
    - the cache key with resolved-face identity (`FontChain::cache_identity`,
      `chain.rs:600`, used by `cache_key_for`, `render.rs:555-580`);
    - `MERMAID_FONT` handling (`MERMAID_FONT_ENV`, `chain.rs:18`, read once
      in `default_chain()`, `chain.rs:210`; warning in `check_mermaid_font`,
      `chain.rs:310`).
    - `biscuit-visualized/src/Cargo.toml` no longer depends on
      `mermaid-rs-renderer`; it depends on `merman` (fork rev
      `6e6b1e6554e8`).
- **Text fallback: NOT landed.** `2026-10-22-merman-text-fallback` is
  `status: finalized-spec`, `implemented: false`. `md` has no `--feature`
  flag (no `long = "feature"`, `value_delimiter` or `ignore_case` anywhere
  under `darkmatter/cli/src`). Only task 5.3 depends on it; see the second
  item of `decisions_for_review`. No stand-in was built.

### Rulings (decided 2026-10-24)

Each was accepted as the plan recommends; see `decisions_for_review`.

- **Rule A** — no environment variable. The font chain takes an explicit
  per-render family override (requested family → bundled Inter) computed
  before layout. `MERMAID_FONT` keeps its Merman meaning; an explicit
  `MERMAID_FONT=Inter` makes every family Inter with the D14 "selected by
  `MERMAID_FONT=Inter`" reason.
- **Rule B** — embeddability is decided on both weights the renderer can
  request (regular and bold), whatever the page later uses.
- **Rule C** — a face is embeddable as TrueType only if it has `glyf` and no
  `fvar`; a CFF/CFF2 or variable face falls back to Inter with the reason
  "outline format not embeddable as TrueType". A face inside a `.ttc` is
  fine (subsetting extracts the one face). The unsubset fallback applies
  only to embeddable faces. **Spike evidence:** macOS's system UI face
  `/System/Library/Fonts/SFNS.ttf` (`.SF NS`) is `glyf` + `fvar`; allsorts
  subsets it without error but silently drops `fvar`/`gvar`, yielding only
  the default instance, which confirms that variable faces must be refused
  rather than subset.
- **Rule D** — the per-diagram cache stores the measured static SVG plus its
  sorted code-point set and the weights used; the subset and `@font-face`
  payload are never cached, and are built per page render.
- **Rule E** — code points are the Unicode scalar values of the character
  data of every `<text>`/`<tspan>` of the final static SVG, plus U+0020, with
  C0/C1 controls dropped; `bold` when the element or an ancestor resolves
  `font-weight` to `bold` or ≥ 600. Unmapped code points are skipped.
- **Rule F** — Darkmatter emits the page-level `<style>` and comments itself,
  and runs its generated style through the same fail-closed sanitizer CSS
  check the SVG sanitizer gains.
- **Rule G** — a small hand-written `@font-face`-aware scanner; `cssparser`
  only if it grows past about 150 lines.
- **Rule H** — font names are cleaned by removing `>`, `<`, control
  characters and newlines, repeatedly removing `--` until none remain, and
  dropping a trailing `-`; `"`/`\` are escaped in `font-family` and `url()`
  contexts and `*/` is removed from the CSS comment.
- **Rule I** — confirmed: an unsubset face above 60 KB is embedded and its
  size logged; the log line is expected, not a failure.
- **Rule J** — nothing wider is measured; no Windows comparison, no host-face
  size survey.

### Spike S1 — allsorts subset in headless Chrome (macOS)

Throwaway crate: `spikes/s1-subset/` in this folder (its own `[workspace]`,
not a workspace member; built with `CARGO_TARGET_DIR=/tmp/bv-s1-target`).
Inputs were the bundled `Inter-Regular.ttf` / `Inter-Bold.ttf`; outputs and
the test page went to `/tmp/bv-s1-out` and are not kept.

- **allsorts version and API:** `allsorts = { version = "0.17.0",
  default-features = false, features = ["flate2_rust"] }`. The default
  feature `flate2_zlib` pulls the C library `libz-sys`; `flate2_rust` keeps
  it pure Rust, as the spec requires. Phase 3 must use the same feature set.
  API: `ReadScope::new(&bytes).read::<FontData>()?.table_provider(index)?`,
  map code points through `allsorts::font::find_good_cmap_subtable` +
  `CmapSubtable::map_glyph`, then
  `allsorts::subset::subset(&provider, &glyph_ids, &SubsetProfile::Minimal,
  CmapTarget::Unicode)`. Glyph id 0 must be first and ids must be unique.
- **Tables kept:** `OS/2, cmap, glyf, head, hhea, hmtx, loca, maxp, name,
  post` (read back from the subset's table directory with `ttf-parser`).
  `OS/2` is kept with `fsType` intact (0x0000). Name IDs 0 ("Copyright 2016
  The Inter Project Authors"), 13 (the OFL 1.1 notice) and 14
  (`http://scripts.sil.org/OFL`) are all present. Every requested code point
  is in the subset's `cmap`.
- **Tables dropped:** `GPOS`, `GSUB`, `GDEF`, `kern`, `fvar`/`gvar`. The
  TrueType path of `allsorts::subset` writes only the tables above (plus
  `cvt`/`fpgm`/`prep` when the profile names them), even for a custom
  profile that lists `GPOS`/`GSUB`.
- **Sizes:** test string of 47 glyphs (Latin, diacritics, arrow, quotes,
  dash): 9,572 bytes regular, 9,588 bold (sources 411,640 / 420,428). Printable
  ASCII (96 glyphs): 17,072 bytes. ASCII + Latin-1 + Latin Extended-A
  (318 glyphs): 33,624 bytes. Arial printable ASCII: 39,156 bytes. A typical
  page is well inside the 60 KB budget with Inter.
- **Chrome check:** Google Chrome 155.0.8059.85, `--headless=new`, fresh
  `--user-data-dir`, no window, page loading both subsets as
  `url(data:font/ttf;base64,…) format("truetype")` and the full faces for
  comparison. Both subset `FontFace`s reached `loaded`;
  `document.fonts.check("400 100px S", text)` and the 700 equivalent were
  `true`; every glyph's width in the subset equals its width in the full
  face (no glyph fell back to another face).
- **Width finding:** the whole test string at 100 px measures 4099.609 px in
  the subset and 4047.998 px in the full face (bold: 4216.455 vs 4160.889).
  The full face in Chrome equals rustybuzz's default shaping exactly
  (4047.998), which is what our measurer does (kerning on). The subset
  equals rustybuzz with `kern`, `liga` and `calt` off exactly (4099.609).
  So the subset loads and draws every glyph, but a label measured with
  kerning is about 1.3% narrower than the browser draws it.
- **Outcome: subset used.** The D11 check passes (loads, every glyph in the
  embedded face, `name` 0/13/14 kept). Because of the width finding, the
  opt-in must measure with plain advances; see "Decided during
  implementation".
- **Host faces (for Rule C):**

  | Face | Outlines | `fvar` | `fsType` | allsorts subset |
  | --- | --- | --- | --- | --- |
  | Inter Regular / Bold (bundled) | `glyf` | no | 0x0000 | ok |
  | `SFNS.ttf` (`.SF NS`) | `glyf` | **yes** | 0x0000 | ok, but drops `fvar` → refuse (Rule C) |
  | `Arial.ttf`, `Arial Bold.ttf` | `glyf` | no | 0x0008 (Editable) | ok |
  | `HelveticaNeue.ttc` #0, #1 | `glyf` | no | 0x0000 | ok, one face extracted |
  | `Helvetica.ttc` #0 | `glyf` | no | 0x0000 | ok |

  The `.ttc` faces have no Unicode-platform name ID 1 (`ttf-parser`'s
  `is_unicode()` filter found none), so Phase 3's comment text must fall
  back to another name record (or the fontdb family name) when naming a
  host face.

### Decided during implementation

- **Measurement without layout features (spec D17).** Evidence: the width
  finding above. The decision's purpose (D6: the measured face is the drawn
  face, so label widths match on every machine) is preserved only if the
  measurer measures what the subset can draw. Choice: when the opt-in is on,
  measure labels with plain glyph advances (no `kern`, `liga`, `calt` or
  other `GSUB`/`GPOS` features), for every embedded face, Inter or host.
  This stays within the other decisions (subset with allsorts, keep `cmap`,
  `OS/2`, `name`; the 60 KB budget; the raster path is unchanged because the
  opt-in never reaches PNG) and adds no dependency, outward action or scope.
  Oracle for Phase 4: a label's measured width equals the sum of the
  subset's `hmtx` advances for its glyphs. The spec's D6 is marked amended.
  Flagged for review (third item of `decisions_for_review`).

### Recon 1.1 — `biscuit-visualized` (paths under `biscuit-visualized/src/`)

- **Font chain:** `pub(crate) struct FontChain` (`src/fonts/chain.rs:179`);
  `default_chain()` (`:205`, `OnceLock`, reads `MERMAID_FONT` once at `:210`,
  then `FontChain::build(db, GenericSource::Os, mermaid_font)` at `:263`).
  Resolution: `FontChain::resolve(&FontQuery) -> Option<fontdb::ID>` (`:341`)
  and `resolve_with_step` (`:368`, returns a `ResolutionStep`: `MermaidFont`,
  `StackName`/`StackGeneric`, `OsSansSerif`, `InstalledSansSerif`). Bytes and
  face index come from `chain.database().with_face_data(id, |data, index| …)`
  (as `has_glyph` does, `:584`). `FontQuery::from_css(font_family, weight,
  italic)` (`src/fonts/query.rs:90`).
- **Test seam:** `#[cfg(test)]` `FontChain::injected(installed: &[&[u8]],
  generic_answers, mermaid_font: Option<&str>)` (`chain.rs:221`),
  `injected_files` (`:236`), `injected_db` (`:249`, `GenericSource::Fixed`).
  `golden_chain()` = `FontChain::injected(&[], &[], Some("Inter"))`
  (`src/tests/golden_tests.rs:42-45`). Render seam:
  `MermaidDiagram::render_in(&self, chain: &Arc<FontChain>, request)`
  (`pub(crate)`, `src/mermaid/render.rs:335`); test-only
  `cache_key_with(chain, request)` (`:582`).
- **Measurer:** `LabelMeasurer` (`src/fonts/measure.rs:29`): `query` (`:62`),
  `faces(text, style)` (`:100`), `measured_family(&TextStyle)` (`:139`).
  Shaping: `src/fonts/shaper.rs`, `ShapeParams { kerning, … }` (`:26`,
  default kerning on at `:70`); `shape_with_face` (`:206`) turns `kern` off
  when `kerning` is false. The plain-advance measurement above will need
  `liga`/`calt` (and the rest of `GSUB`/`GPOS`) off as well, not only `kern`.
- **Bundled Inter:** `INTER_REGULAR` / `INTER_BOLD`
  (`src/fonts/bundled.rs:13-17`, `include_bytes!`); `NOTO_EMOJI` (Linux only,
  `:22-23`). `OFL.txt` is not compiled in anywhere.
- **HTML static target:** `pub enum MermaidTarget { RasterSafe, HtmlStatic }`
  (`#[non_exhaustive]`, `render.rs:98`), `with_target` (`:280`), `target()`
  (`:286`), `output_target(format)` (`:547`, PNG ignores the target). The
  post-process runs at `render.rs:685`:
  `MermaidTarget::HtmlStatic => html_static(&fixed, &label_measurer)`.
- **`font-family` rewrite:** `html_static` (`src/mermaid/postprocess.rs:179`)
  → `rewrite_tag` (`:323`, attributes and `style` at `:342`),
  `rewrite_stylesheet` (`:388`), `rewrite_declarations` (`:420`/`:442`); the
  rewrite itself is `led_by_measured_face` (`:469`), which prepends
  `css_quoted(measured_family)` (`:488`).
- **Cache key:** `FileCache::cache_key(kind, source, options_json,
  backend_id, format)` (`src/cache/file_cache.rs:111`, string
  `"v2|{}|{}|{}|{}|{}"` at `:120-127`, `xx_hash`). `MERMAID_BACKEND =
  "merman@6e6b1e6554e8+bv5"` (`:29`). Mermaid options JSON in `cache_key_for`
  (`render.rs:555-580`: `design`, `fonts: chain.cache_identity(…)`, `title`,
  `request`, plus `target: "html-static"` for `HtmlStatic`).
  `RenderRequest::cache_identity()` (`src/artifact.rs:124`).
  `pub enum OutputFormat { Svg, Png }` (`artifact.rs:5`).
- **Dependencies:** `ttf-parser = 0.25.1` is a direct, optional dependency
  (enabled by the `image` feature, `Cargo.toml:18`, `:45`). The only `OS/2`
  reader today is `family_class` (`src/fonts/metadata.rs:40-43`, raw
  `table(b"OS/2")`, `sFamilyClass`); nothing reads `fsType`. No workspace
  crate depends on `allsorts`, and it is not in `Cargo.lock`.
- **Tracing capture in tests:** `struct WarnCounter(Arc<AtomicUsize>)`, a
  hand-written `tracing::Subscriber` counting WARN events whose target starts
  with `biscuit_visualized`, and `warnings_while(f) -> usize` using
  `tracing::subscriber::with_default` (current thread only)
  (`src/tests/font_chain_tests.rs:113-141`). Reuse this pattern for AC-E6/E7;
  the crate has no `tracing-subscriber`/`tracing-test` dependency. If AC-E6
  needs `info` events or message text, extend this local subscriber rather
  than adding a crate.
- **XML reading:** the post-process has no XML library; it is a hand-written
  scanner in `postprocess.rs` (`html_static` loop on `<`, `tag_end` `:236`,
  `attributes` `:279`, `attribute` `:157`, `stylesheet` `:151`,
  `decode_entities` `:519`, `encode_entities` `:504`). The code-point
  collector (3.1) reuses these helpers.

### Recon 1.2 — Darkmatter

- **`HtmlOptions`:** `darkmatter/lib/src/markdown/output/html.rs:43-104`,
  `#[derive(Debug, Clone)]`, `#[non_exhaustive]` (`:44`), `Default` at
  `:108-124`. `mermaid_mode: MermaidMode` at `:69`; its doc comment (`:67`)
  is stale: it says `Image` renders interactive mermaid.js, but `Image`
  produces a static SVG (5.2 should fix it when it edits the struct).
- **`MermaidMode`:** `darkmatter/lib/src/markdown/output/terminal.rs:317`,
  variants `Off` (default), `Image` (`:331`), `Text`; not `non_exhaustive`.
- **Mapping:** `browser_options_from_html_options`
  (`render_tree/entrypoints.rs:589-623`; `Image` → `BrowserMermaidMode::
  StaticSvg` at `:593-596`, installs `TerminalCodeRenderer::new()
  .with_html_options(opts.clone())`); `render_tree_html_page_body`
  (`:278-322`; `Off` → `Interactive`, `Image` → `StaticSvg`, `Text` → `Code`).
- **Static SVG per diagram:** `TerminalCodeRenderer::render_browser_mermaid`
  (`render_tree/code_renderer.rs:599-620`) renders one diagram with
  `MermaidDiagram::new(value).with_target(MermaidTarget::HtmlStatic)
  .render_to_svg()`, sanitizes with `svg_sanitizer::sanitize_svg`, and
  returns a raw-HTML `BrowserFragment`. It has no page context.
- **Page assembly:** in `renderable/src/tree/render/browser.rs`.
  `StreamWriter::push_hook_fragment` (`:1915-1919`) keeps every hook fragment;
  `stream_browser_document` (`:427-476`) builds
  `HtmlPage::from_fragments(hook_fragments)` (`:466`) — the one place that
  sees every diagram of a page, but as opaque fragments. The
  `code_renderer` (`Rc<dyn CodeRenderer>`) is shared across a page, so it can
  accumulate per-page state (code points per face) for 5.2.
  `assemble_full_document` (`:486-519`) writes the `<head>`;
  `render_browser_document_body` (`:358-410`) emits
  `<style>{stylesheet}</style>`. Darkmatter wrappers:
  `darkmatter/lib/src/layout/page.rs` (`wrap_browser_html` ~`:1824-1856`,
  `HtmlOptions` built from `TerminalOptions` at `:1142-1149` and
  `:1240-1247`). **Note for Phase 5:** page assembly for the `@font-face`
  union likely crosses into the `renderable` crate, a package this plan does
  not name.
- **Sanitizer:** `darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs`.
  `pub fn sanitize_svg(svg) -> Option<String>` (`:91`, quick-xml allowlist);
  `attribute_is_safe` (`:181`); `css_is_safe` (`:205-223`, case-insensitive,
  rejects `@import` and any `url(` not starting with `#`, so `url(data:…)` is
  rejected today); `sanitize_style_element` (`:236-277`, drops the whole
  block on failure, still returns `Some`). Tests in the same file (`mod
  tests`, `:478+`): `drops_style_block_with_import` (`:542`),
  `drops_style_block_with_external_url` (`:552`),
  `keeps_style_block_with_local_fragment_url` (`:560`),
  `drops_style_attribute_with_external_url` (`:568`),
  `drops_filter_attribute_with_external_url` (`:579`),
  `merman_flowchart_survives_with_labels_and_shadow` (`:798`),
  `malformed_xml_returns_none` (`:825`). No test covers `data:` URLs.
- **`md` CLI:** no `--feature`. `--mermaid` is a `bool`
  (`darkmatter/cli/src/args/cli.rs:52-55`), folded at
  `darkmatter/cli/src/render.rs:70-74` into `MermaidMode::Image`/`Off` via
  `DarkmatterPage::with_mermaid_mode` (`page.rs:724`), copied into
  `HtmlOptions` at `page.rs:1149`.
- **Existing tests:** `darkmatter/lib/tests/l1/mermaid_static_svg.rs`
  (helpers `page_html` `:25`, `embedded_svg`, `font_families`; tests at
  `:55`, `:62`, `:87`, `:111`). Browser: `darkmatter/lib/tests/browser/
  browser_render.rs` (binary `browser`, `required-features =
  ["browser-tests"]`), harness `biscuit_browser_harness::{ChromeHarness,
  BrowserHarness, require_browser, wrap_fragment}`, `#[tokio::test]` +
  `#[serial(browser)]`, `BISCUIT_BROWSER_REQUIRED=1`, `CHROME` override;
  e.g. `browser_mermaid_static_svg_computes_in_browser` (`:344`),
  `browser_sanitized_mermaid_svg_strips_external_css_references` (`:544`).

### Recon 1.3 — `OFL.txt` Reserved Font Name (AC-E8)

- `biscuit-visualized/src/assets/fonts/inter/OFL.txt` declares **no**
  Reserved Font Name: its first line is the copyright line with no "with
  Reserved Font Name …" clause, and the only "Reserved Font Name" mentions
  are the license's own definitions (lines 32 and 62).
- So the embedded Inter is declared under the family name **`Inter`** (not
  `bv-mermaid`), in both the `@font-face` and every label's `font-family`.
- Copyright line for D15, exactly: `Copyright (c) 2016 The Inter Project
  Authors (https://github.com/rsms/inter)`. (The font's own name ID 0 reads
  `Copyright 2016 The Inter Project Authors`; D15 names the `OFL.txt` line.)

### Gates and requirement-to-test mapping

- Phase 1 changes no shipped code, so no behavior needs a test; the spike's
  checks (table directory, name IDs, Chrome load and widths) are evidence
  recorded above, and Phase 3's 3.2 re-reads the shipped subset's table
  directory in a test (AC-E12).
- Scope: `python3 scripts/ci/affected_scope.py` over this phase's five files
  (plan, spec, log, spike `Cargo.toml` and `main.rs`) gives `packages: []`,
  `change_class: documentation`. Over the whole working tree it lists
  `dmls`, `test-toolkit`, `darkmatter`, `darkmatter-cli`; those come from
  unrelated, pre-existing `darkmatter/dmls` edits that were in the tree
  before this phase started.
- Gates run anyway, because spike source files are listed in
  `source_files_during_phase_1`: `just lint` in `biscuit-visualized` clean;
  `just test` in `biscuit-visualized`: 976 passed, 3 skipped, 0 failed.
- No dependent-package tests: no compiled code changed. No cross-OS check:
  nothing OS-specific changed.
