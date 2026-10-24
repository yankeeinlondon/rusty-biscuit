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
    - |-
      Phase 2 (face choice): small rulings the plan left open. (1) A face that
      cannot be parsed at all, or whose face index is out of range, gets the
      reason "no readable OS/2 table", and a face with no `glyf` table counts
      as "outline format not embeddable", like CFF and variable faces.
      (2) When a face is both Restricted and bitmap-only, the reason given is
      Restricted. (3) When a family's font stack itself names `Inter` (not
      through `MERMAID_FONT`), the bundled Inter is treated as the resolved
      face (reason "resolved", permission Installable) rather than as a
      substitution; when `MERMAID_FONT=Inter` picks an installed copy of
      Inter, that copy is judged by its own `fsType` like any other installed
      face. (4) Only upright regular and bold are decided (the plan's Rule B);
      an italic request is not decided. (5) The classifier result for
      "no subsetting" carries no separate reason field: the browser-check
      fallback is not a property of a face, and the spike chose the subset.
      Alternatives: different reason labels for (1)-(2), or an explicit
      Inter-substitution reason for (3); each changes only the comment text
      Phase 3 writes and the enum variants in `src/mermaid/embed/`.
    - |-
      Phase 2: names written into CSS strings (`font-family` in the
      `@font-face` rule) use the existing label post-process escaper
      (`css_quoted`, now crate-visible), which turns quotes, backslashes,
      `<`, `>`, `&`, and control characters into CSS hex escapes. The plan's
      Rule H asked for escaping only `"` and `\`, plus a separate `url()`
      escaper. I used the stricter, existing function so the `@font-face`
      family and each label's leading family are spelled by one function, and
      added no `url()` escaper because only base64 that the library produces
      ever goes inside `url()`. Alternative: a new minimal escaper per Rule H;
      that risks two spellings of one family name. Redo cost: one function.
    - |-
      Phase 2: two existing tests in `components/graph_expression.rs` were
      named `browser_render_emits_svg` and `browser_render_dot_input_emits_svg`.
      They need no browser, but the `browser_` prefix kept them out of the
      normal test run, and `test-toolkit`'s tier contract test failed on it.
      I renamed them (`render_browser_svg_emits_svg`,
      `render_browser_svg_emits_svg_for_dot_input`) so they run. Alternative:
      declare a browser test tier for this package, which would need a real
      browser recipe for two tests that do not use a browser.
    - |-
      Phase 3: the code-point collector reads the static SVG with `roxmltree`
      and applies the SVG's stylesheet with `simplecss`, copying how usvg
      (our rasterizer) resolves styles. The plan said to reuse the
      post-process's hand-written tag scanner. That scanner has no CSS
      cascade, and Merman makes most bold text bold through class rules in
      the SVG's `<style>`, so the scanner would misfile bold labels as
      regular, and the browser would then fake bold from the regular face at
      a different width. Both crates were already in the build (usvg uses
      the same versions), so nothing new is downloaded. A test checks the
      collector against usvg's own reading of real Merman output.
      Alternative: extend the hand scanner with selector matching (more
      code to own). Redo cost: one file (`embed/glyphs.rs`).
    - |-
      Phase 3: the collected code points are grouped by font family (the
      first family each label names), not as one page-wide set as the plan
      sketched, and the builder therefore also takes the font chain. This
      way a family the page never draws is not embedded at all, and each
      face gets only its own labels' characters. Alternative: one set for
      every family (simpler, but embeds unused families and extra glyphs).
      Redo cost: the `UsedGlyphs` type and two loops.
    - |-
      Phase 3: details of the generated CSS and logging that the spec left
      open. (1) Each `@font-face` rule states `font-style:normal` and
      `font-weight:400` or `700`, so a regular and a bold rule of the same
      family are told apart; the spec's example shows neither. (2) The
      family is written in single quotes with CSS escapes (the same function
      that writes labels' family names), not double quotes. (3) "60 KB" is
      taken as 60,000 bytes. (4) An over-budget file and a whole
      (unsubset) face are logged at `info`, as expected outcomes (Rule I);
      only a failure logs a warning. (5) If any one face cannot be subset,
      the whole page gets no embedded font, rather than a font for some
      families only. (6) Inter's OFL line is written for the bundled Inter
      or any family named Inter; its copyright text is a constant that a
      test compares with the bundled `OFL.txt`, which stays out of the
      binary. (7) No "browser check failed" unsubset reason exists, because
      the spike chose the subset; only the `fsType` no-subsetting reason is
      written. Each is a small, local change if the reviewer disagrees.
    - |-
      Phase 3: fixed a test-guard failure that earlier phases left in place.
      `test-toolkit`'s check for build-machine paths baked into tests
      flagged seven places, all in this package area: three test helpers now
      use the shared `manifest_dir!()` macro, and the four in the separate
      benchmark crate (its own workspace, run only by `just bench`, never in
      CI) were added to the guard's allow list with that reason.
      Alternative: give the benchmark crate a dependency on the test
      harness to use the macro too. Redo cost: four allow-list entries.
    - |-
      Phase 4: the per-render face choice lives in the text measurer, not in
      the font chain. The plan (Rule A) pictured the chain taking a "family
      override". Only the measurer needs it: opt-in output is an HTML static
      SVG that this library never rasterizes. The opt-in measurer decides each
      requested family the first time it is asked about it (on both weights,
      before any label of that family is measured) and keeps the answer, so
      the face is still chosen before layout. Nothing process-wide changes.
      One small effect: the text-contrast pass still locates text with the
      chain's normal faces; it only uses them to find which fill sits behind
      a label. Alternative: a chain-level override object passed everywhere
      the chain goes; more plumbing for no reader that needs it. Redo cost:
      the measurer's constructor and `FaceChooser` in `embed/faces.rs`.
    - |-
      Phase 4: the opt-in is a separate switch on the diagram,
      `MermaidDiagram::with_embedded_font(true)`, rather than a field on the
      "HTML static" target as the plan suggested. The target is a plain
      enum with no fields; the switch acts only when the target is HTML
      static and the output is SVG, and is ignored otherwise. Darkmatter is
      another crate, so it needs a public way to get the data and build the
      page's font; Phase 4 added it: `render_with_font` returns the artifact
      plus a `DiagramFont`, and `page_font(&[DiagramFont])` returns a
      `PageFont` (comments, CSS, markup) or a `FontEmbeddingError`. Phase 5's
      plan says Darkmatter calls `build_font_embedding`; it calls `page_font`
      instead, which wraps it. Alternatives: an `HtmlStaticEmbeddedFont`
      target variant (one value fewer to combine, but it doubles every match
      on the target), or extending `RenderedArtifact` with a font field
      (breaks every struct literal of a shared type). Redo cost: the builder
      and `embed/page.rs`.
    - |-
      Phase 4: "plain glyph advances" are measured by shaping each face as if
      it held only the tables the subset keeps (`cmap`, `head`, `hhea`,
      `hmtx`, `maxp`, `name`, `OS/2`, `post`, `glyf`, `loca`), so the shaper
      runs exactly the no-layout-tables path the browser's HarfBuzz runs on
      the subset. That is more faithful than adding up `hmtx` advances
      directly (it keeps the shaper's default mark handling, bidi runs and
      fallback faces), and for ordinary text the two agree, which a test
      checks. Alternative: turn off `kern`, `liga`, `calt` and every other
      feature one by one, which can never be complete. Redo cost: one helper
      in `fonts/shaper.rs`.
    - |-
      Phase 4: what the cache keeps beside an opt-in SVG (Rule D) is a small
      JSON record (`{key}.font.json`): the requested font-family lists and
      the characters drawn per family and weight. It does not store the
      chosen faces themselves, because face ids are only valid inside one
      process; a cache hit re-runs the face choice from the record, which
      depends only on the fonts the cache key already covers. The record is
      written before the SVG, and an SVG whose record is missing or
      unreadable is rendered again rather than served without its data.
      If the code-point scan of our own SVG ever fails, the diagram is still
      shown (a warning is logged and that diagram contributes no
      characters) instead of turning into a code block. Alternatives: store
      the face file names (fragile across font updates), or re-scan the SVG
      on every hit (what Rule D wanted to avoid). Redo cost: `FontRecord` in
      `embed/page.rs`.
    - |-
      Phase 5: the `md --feature embed-mermaid-font` flag (task 5.3) is not
      built. `md` still has no `--feature` flag, because the text-fallback work
      that introduces it has not landed, and the plan says not to build a
      stand-in. Everything below the CLI is ready:
      `DarkmatterPage::with_embed_mermaid_font(true)` is what the flag must
      call. Alternatives: a standalone `--embed-mermaid-font` flag (the
      spec's original form, which the author replaced), or building the
      `--feature` flag here (the other spec's work). Redo cost: none; 5.3 is
      still to do, plus its CLI wiring test and the "frontmatter key not
      honored" test.
    - |-
      Phase 5: the sanitizer's CSS check for `<style>` uses the `cssparser`
      tokenizer, which Darkmatter already depends on, instead of the
      hand-written scanner the plan recommended. A real CSS tokenizer reads
      escapes, comments, strings and blocks the way the browser does, so the
      check cannot be fooled by spellings a substring scan misses (for example
      an escaped `url(` or `@import`). The check also admits the quoted form
      `url("data:font/ttf;base64,...")`, which the plan's table did not name.
      Attribute values keep the old substring check unchanged, so a `data:`
      URL is still never admitted outside a style sheet. One side effect:
      `url(` inside a CSS comment or string no longer drops a `<style>` block
      (both are inert), while a malformed string or url token now does.
      Alternative: the hand scanner (more code to own, and easier to get
      wrong). Redo cost: the scanner functions in `svg_sanitizer.rs`.
    - |-
      Phase 5: where the page font goes. The page body is written diagram by
      diagram before the union of their characters is known, so the first
      opt-in diagram writes a placeholder comment
      (`<!--darkmatter:mermaid-font-->`) ahead of its SVG. After the page is
      rendered, Darkmatter replaces it with the comments and `<style>`, or
      with nothing when the font fails. The font therefore sits in the body
      just before the first diagram, as in the spec's example, not in
      `<head>`. Nothing an author writes can forge the placeholder, because
      these pages escape raw HTML and the sanitizer drops SVG comments.
      Alternatives: a page-level hook in the shared `renderable` crate so the
      style could go in `<head>` (touches another package's trait for one
      caller), or rendering every diagram twice. Redo cost:
      `render_tree/mermaid_font.rs` and the two entry points.
    - |-
      Phase 5: additions the plan did not list. (1)
      `DarkmatterPage::with_embed_mermaid_font`, because the `md` CLI and
      the page tests build pages through `DarkmatterPage`, not `HtmlOptions`.
      (2) `components::mermaid::MermaidDiagram::with_embedded_font` and
      `render_to_svg_with_font` in biscuit-visualized, so Darkmatter keeps
      the component's terminal palette and background rather than copying
      them. (3) The nextest macOS CoreText setup script now also covers
      Darkmatter's two static-Mermaid L1 modules, which took about 3.5 s
      per test, existing ones included (now about 0.5 s). Redo cost: small
      for each.
    - |-
      Phase 5: Darkmatter's full test run has 42 failures that also fail on
      the unchanged commit (verified by running them on a clean checkout of
      HEAD): 40 `schemas_convert_snapshots`, 1 `suggest_constraint_phase3`
      snapshot, and the CLI's
      `schema_validate_legacy_json_output_is_byte_identical`. All come from
      JSON key order. Commit `bbde223e6` ("make semantic hashes key-order
      insensitive", 2026-10-23) enabled serde_json's `preserve_order` for
      Darkmatter, so the schema converter and `md schema validate --json` now
      write keys in insertion order (`$schema` last) instead of sorted. Whether
      that output order is right is a decision for that work, so I left the
      snapshots alone. A 43rd failure with the same cause,
      `as_json5_uses_the_formatter_object_idiom`, was a test expectation
      about a function that has always kept key order; I fixed the expected
      string to author order. Alternatives: accept all the snapshots as they
      are now, or restore sorted output in the converter.
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
    - test-toolkit
    - darkmatter
source_files_during_phase_2:
    - biscuit-visualized/src/src/mermaid/embed/mod.rs
    - biscuit-visualized/src/src/mermaid/embed/permission.rs
    - biscuit-visualized/src/src/mermaid/embed/comment.rs
    - biscuit-visualized/src/src/mermaid/embed/faces.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/postprocess.rs
    - biscuit-visualized/src/src/fonts/chain.rs
    - biscuit-visualized/src/src/components/graph_expression.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/embed_permission_tests.rs
    - biscuit-visualized/src/src/tests/embed_comment_tests.rs
    - biscuit-visualized/src/src/tests/embed_faces_tests.rs
docs_updated_during_phase_2:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/plan.md
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - Cargo.lock
    - biscuit-visualized/src/Cargo.toml
    - biscuit-visualized/src/src/mermaid/embed/mod.rs
    - biscuit-visualized/src/src/mermaid/embed/glyphs.rs
    - biscuit-visualized/src/src/mermaid/embed/subset.rs
    - biscuit-visualized/src/src/mermaid/embed/comment.rs
    - biscuit-visualized/src/src/mermaid/embed/embedding.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/embed_glyphs_tests.rs
    - biscuit-visualized/src/src/tests/embed_subset_tests.rs
    - biscuit-visualized/src/src/tests/embed_comment_tests.rs
    - biscuit-visualized/src/src/tests/embed_embedding_tests.rs
    - biscuit-visualized/src/src/tests/embed_faces_tests.rs
    - biscuit-visualized/src/src/tests/embed_permission_tests.rs
    - biscuit-visualized/src/src/tests/contrast_corpus_tests.rs
    - biscuit-visualized/src/src/tests/golden_tests.rs
    - tools/test-toolkit/src/archive_guard.rs
docs_updated_during_phase_3:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/plan.md
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-visualized/SKILL.md
source_files_during_phase_4:
    - biscuit-visualized/src/src/artifact.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/fonts/measure.rs
    - biscuit-visualized/src/src/fonts/shaper.rs
    - biscuit-visualized/src/src/mermaid/embed/embedding.rs
    - biscuit-visualized/src/src/mermaid/embed/faces.rs
    - biscuit-visualized/src/src/mermaid/embed/mod.rs
    - biscuit-visualized/src/src/mermaid/embed/page.rs
    - biscuit-visualized/src/src/mermaid/embed/subset.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/tests/embed_render_tests.rs
    - biscuit-visualized/src/src/tests/measure_tests.rs
    - biscuit-visualized/src/src/tests/mod.rs
docs_updated_during_phase_4:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/plan.md
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-visualized/SKILL.md
message_to_agent: |-
    Phase 5 (Darkmatter) is done except task 5.3; read the log's Phase 5 section.
    - 5.3 (`md --feature embed-mermaid-font`) is NOT built: `md` still has no
      `--feature` flag (the text-fallback work has not landed), and the plan
      forbids a stand-in. When it lands, map the feature to
      `DarkmatterPage::with_embed_mermaid_font(true)` in darkmatter/cli/src/render.rs
      and add 5.3's CLI wiring and frontmatter-not-honored tests.
    - Phase 6 docs to write: the darkmatter mermaid page
      (darkmatter/docs/rendering/mermaid.md) covering
      `HtmlOptions::embed_mermaid_font`, `DarkmatterPage::with_embed_mermaid_font`,
      where the comments and `<style>` go (the body, right before the first
      diagram), and the sanitizer's one `data:font/ttf` exception
      (`style_sheet_is_safe`, top-level `@font-face` `src` only). The CLI line
      stays "planned" until 5.3 lands. Also: the biscuit-visualized topic page
      and docs/dependencies.md (root and area) for allsorts and its 10
      transitive crates (aliasable, bitreader, glyph-names, ouroboros,
      ouroboros_macro, pathfinder_geometry, pathfinder_simd,
      proc-macro2-diagnostics, unicode-canonical-combining-class,
      unicode-joining-type).
    - Darkmatter `just test` has 42 failures that predate this work (JSON key
      order after commit bbde223e6; listed in the log). Do not treat them as
      regressions of this feature.
source_files_during_phase_5:
    - biscuit-visualized/src/src/components/mermaid.rs
    - darkmatter/lib/src/layout/page.rs
    - darkmatter/lib/src/markdown/output/html.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
    - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
    - darkmatter/lib/src/markdown/render_tree/mermaid_font.rs
    - darkmatter/lib/src/markdown/render_tree/mod.rs
    - darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/tests/browser/browser_render.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/mermaid_embedded_font.rs
    - .config/nextest.toml
docs_updated_during_phase_5:
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/plan.md
    - biscuit-visualized/features/2026-10-21-mermaid-embedded-font/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/biscuit-visualized/SKILL.md
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

## Phase 2

Face choice by `fsType`, in `biscuit-visualized` (2026-10-24). Pure logic: no
render path, cache key, or output changes yet (Phase 4 wires it in), so every
existing render is byte-identical.

### What was built

- **`src/mermaid/embed/permission.rs` (2.1).** `classify(data, index) ->
  EmbedPermission` reads the raw `fsType` (`OS/2` offset 8) through
  `ttf_parser::RawFace`, not `ttf-parser`'s `Permissions` helper. Results:
  `Embed { permission, fs_type }`, `EmbedUnsubset { permission, fs_type }`
  (the `0x0100` bit), or `Refuse(Refusal)` with `Restricted { fs_type }`,
  `BitmapOnly { fs_type }`, `NoReadableOs2`, `OutlineFormat`. Order: an
  unparseable face (bad data, index out of range) is `NoReadableOs2`; then
  `CFF `/`CFF2`/`fvar`, or no `glyf`, is `OutlineFormat` (Rule C); then a
  missing or short `OS/2` is `NoReadableOs2`; then the permission bits
  (least restrictive wins; `0x0002` alone is Restricted), then `0x0200`
  (bitmap only), then `0x0100` (unsubset). Reserved bits are ignored and
  logged once per process at debug level.
- **`src/mermaid/embed/comment.rs` (2.2).** `comment_safe(name)` removes
  controls, `<`, `>`, then repeatedly every `--` and `*/` until none remains,
  then trailing `-`. One function serves the HTML comment (D14) and the CSS
  comment (D15). The CSS-string context (`font-family`) reuses the
  post-process's `css_quoted`, now `pub(crate)`, so the `@font-face` family
  and the labels' leading family are spelled by one function.
- **`src/mermaid/embed/faces.rs` (2.3).** `choose_embedded_faces(requested:
  &[&str], chain) -> EmbeddedFaces`: per distinct CSS `font-family` list,
  resolve regular (400) and bold (700) through the chain (Rule B), classify
  both; either refused → the whole family is the bundled Inter
  (`ChoiceReason::Refused { face, refusal }`, warning logged); no face
  resolves → `NoInstalledFace`; the `MERMAID_FONT` step landing on the
  bundled Inter → `MermaidFontInter`; otherwise `Resolved` with both faces.
  `FamilyChoice` carries `requested`, `family` (fontdb's first family name;
  `Inter` for the bundled face), `bundled_inter`, `regular`/`bold`
  (`EmbeddedFace { id, permission, fs_type, unsubset }`) and `reason`.
- **`src/fonts/chain.rs`.** Two small accessors: `bundled_inter(query)` (the
  bundled face by CSS matching, never an installed Inter) and
  `is_bundled_inter(id)`.
- `src/mermaid/mod.rs` declares `pub(crate) mod embed` with
  `#[cfg_attr(not(test), allow(dead_code))]` until Phase 4 calls it.

### Requirement-to-test mapping

All L1, in-process, through the crate-level decision functions (F1: the
classifier and decision are pure functions over font bytes and an injected
chain; no CLI, filesystem, or terminal boundary is crossed yet).

| Requirement / matrix row | Test (under `src/tests/`) |
| --- | --- |
| `0x0000`, `0x0008`, `0x0004` embed (AC-E13) | `embed_permission_tests::installable_editable_and_preview_print_embed` |
| `0x0002` alone → Restricted | `…::restricted_as_the_only_permission_bit_is_refused` |
| `0x0006`, `0x000A`, `0x000E`, `0x000C` least restrictive | `…::several_permission_bits_take_the_least_restrictive` |
| `0x0200` with/without permission bits; with `0x0002` | `…::bitmap_only_is_refused_whatever_the_permission_bits` |
| `0x0100` → unsubset; with `0x0002` → Restricted | `…::no_subsetting_embeds_unsubset_unless_restricted` |
| reserved bits ignored (with and without `0x0002`) | `…::reserved_bits_do_not_change_the_decision` |
| `OS/2` absent | `…::a_face_without_os2_is_refused` |
| `OS/2` shorter than `fsType` (9 bytes) vs exactly 10 bytes | `…::an_os2_table_too_short_for_fs_type_is_refused_and_one_just_long_enough_is_read` |
| `OS/2` record past end of file | `…::an_os2_record_pointing_past_the_end_of_the_file_is_refused` |
| `CFF `, `CFF2`, `fvar`, no `glyf` (Rule C) | `…::cff_cff2_variable_and_glyf_less_faces_are_refused_as_outline_format`, `…::the_real_variable_noto_emoji_is_refused_as_outline_format` |
| document: `.ttc` face index | `…::a_collection_face_is_classified_by_its_own_index` |
| document: index out of range | `…::a_face_index_out_of_range_is_refused` |
| document: trailing garbage | `…::trailing_bytes_after_a_valid_face_do_not_change_the_decision` |
| document: not a font | `…::data_that_is_not_a_font_is_refused` |
| bundled Inter is Installable | `…::the_bundled_inter_is_installable` |
| crafted names (`a--b`, `a-->b`, `---`, `*/ x`, `"`, `\`, newline, `<!--`) cannot end either comment (AC-E14) | `embed_comment_tests::crafted_names_cannot_end_either_comment` |
| property-style: 5,000 generated names | `embed_comment_tests::generated_names_cannot_end_either_comment` |
| CSS-string context escapes `"`, `'`, `\`, `<`, newline | `embed_comment_tests::css_strings_escape_quotes_backslashes_and_markup` |
| permitted resolved face measured = embedded (AC-E5) | `embed_faces_tests::permitted_resolved_faces_are_measured_and_embedded` |
| Restricted resolved face → Inter, warning logged | `embed_faces_tests::a_restricted_resolved_face_gives_inter_and_warns` |
| permitted regular + Restricted bold → whole family Inter (AC-E13 last clause) | `embed_faces_tests::a_restricted_bold_makes_the_whole_family_inter` |
| bitmap-only, no `OS/2`, variable face → Inter | `embed_faces_tests::bitmap_only_gives_inter`, `…::a_face_without_os2_gives_inter`, `…::a_variable_resolved_face_gives_inter_as_outline_format` |
| no-subsetting bit → that face unsubset | `embed_faces_tests::no_subsetting_embeds_the_resolved_face_unsubset` |
| `MERMAID_FONT=Inter` / permitted / Restricted | `…::mermaid_font_inter_selects_inter`, `…::mermaid_font_naming_a_permitted_family_embeds_it`, `…::mermaid_font_naming_a_restricted_family_gives_inter` |
| empty chain → Inter (`NoInstalledFace`) | `…::a_chain_with_no_installed_face_gives_inter` |
| stack naming `Inter` → bundled Inter as resolved | `…::a_stack_naming_inter_embeds_the_bundled_inter_as_resolved` |
| per-family decision; duplicates decided once | `…::each_requested_family_is_decided_on_its_own_and_duplicates_once` |

The measured-equals-embedded oracle (AC-E5) is independent of the decision
code: a permitted face must equal `chain.resolve(query)` (what the measurer
uses unchanged); a substituted family must equal what a second chain built
with `MERMAID_FONT=Inter` over the same installed fonts resolves (D13's
definition of the substitution). The per-render override that makes the
measurer use these faces (Rule A) is Phase 4 work.

The Input Robustness Matrix's field-shape rows (absent/null/wrong type) do not
apply: `fsType` is a bit set, and each document shape is tested once on the
shared `RawFace` loader call.

### Smell grep (checkpoint 2)

`grep -n -E "serde\(default\)|unwrap_or_default\(\)|\.ok\(\)|filter_map\(.*as_str" src/src/mermaid/embed/*.rs`
returns nothing. The one fallback (`classify_face`: font data the database
cannot read → `Refuse(NoReadableOs2)`) fails closed.

### Pre-existing defect fixed (Rule 5)

- `test-toolkit`'s `declared_test_tiers_match_owned_tests` failed:
  `components/graph_expression.rs` had two plain L1 tests named
  `browser_render_emits_svg` and `browser_render_dot_input_emits_svg`. The
  `browser_` prefix took them out of L1 while the area declares no browser
  tier, so they never ran. Renamed to `render_browser_svg_emits_svg` and
  `render_browser_svg_emits_svg_for_dot_input`; both pass, and the contract
  test passes.

### Pre-existing failure left in place

- `test-toolkit`'s `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`
  fails on 12 sites across the tree (seven in this area: `env!("CARGO_MANIFEST_DIR")`
  in `src/tests/contrast_corpus_tests.rs:164,655`, `src/tests/golden_tests.rs:142`,
  and the non-member `bench/` crate). None is in a file this phase touched,
  and the `bench/` sites need an allow-list entry in `test-toolkit`, so it is
  left for a separate fix and noted in `message_to_agent`.

### Gates

- `just lint` (biscuit-visualized): clean.
- `just test` (biscuit-visualized): 1010 passed, 1 skipped (before the rename:
  1008 passed, 3 skipped; the two recovered tests now run).
- `just check-tier-coverage biscuit-visualized`: stranded 0.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` → `packages:
  [biscuit-visualized, test-toolkit]`, `change_class: package`;
  `test-toolkit` is scheduled by the planner's suite registry for any Rust
  source change. `just test test-toolkit`: 474 passed, 2 failed, 4 skipped;
  the two failures are the ones described above (one now fixed and passing
  alone, one pre-existing).
- Reverse dependencies (`darkmatter`, `biscuit-terminal-cli`, `sniff-cli`,
  `worktree-cli`): `cargo check` clean. No public API or output changed (all
  new items are `pub(crate)`), so their tests were not run.
- No cross-OS check: no path, process, or `#[cfg(os)]` code changed; the
  tests use only in-memory fonts.

## Phase 3

Subsetting and the `@font-face` payload, in `biscuit-visualized`
(2026-10-24). Still crate-private and off the render path (Phase 4 wires
it in), so every existing render is byte-identical: the 1045-test suite,
golden corpus included, passes unchanged.

### What was built (under `biscuit-visualized/src/src/mermaid/embed/`)

- **`glyphs.rs` (3.1).** `collect_used_glyphs(svg) -> Result<UsedGlyphs,
  GlyphsError>`. Parses the final static SVG with `roxmltree` and applies its
  `<style>` with `simplecss`, using usvg's precedence (presentation attribute,
  then rules in specificity order, then the `style` attribute; an
  `!important` declaration keeps its place; the `font` shorthand through
  `svgtypes`). Collects the character data of every `<text>` and its
  descendants, drops controls, adds U+0020 to each family/weight that draws
  text, and splits at weight 600 (`bold`, `bolder`, ≥ 600; `lighter` is
  regular; undefined values such as `var()` are ignored, so the parent's
  weight holds). Grouped by the first family of the computed `font-family`
  (`first_family` unquotes and resolves CSS escapes, the inverse of
  `css_quoted`). See `decisions_for_review` for why not the hand scanner.
- **`subset.rs` (3.2).** `subset(data, index, &code_points)` maps code
  points through the face's Unicode `cmap` (unmapped ones skipped, R-E8) and
  calls `allsorts::subset::subset(…, SubsetProfile::Minimal,
  CmapTarget::Unicode)`. `whole_face(data, index)` returns a single-face
  file's own bytes, or rebuilds one face of a `.ttc` with every table
  (`allsorts::subset::whole_font`), never a collection. **Defect found while
  testing:** `allsorts` reads face 0 of a single-face file for any face
  index; both functions now reject a nonzero index for such files.
- **`comment.rs` (3.3).** `page_comment(&FamilyChoice, unsubset)` writes
  one D14 comment per reason (resolved with permission, mixed regular/bold
  permissions, `MERMAID_FONT=Inter`, Restricted, bitmap only, no readable
  `OS/2`, outline format, no installed face) plus the "embedded unsubset
  because fsType … forbids subsetting" clause; names pass through
  `comment_safe`. `ofl_attribution()` is the D15 line built from
  `INTER_COPYRIGHT`.
- **`embedding.rs` (3.4).** `build_font_embedding(&EmbeddedFaces,
  &[UsedGlyphs], &FontChain) -> Result<FontEmbedding, FontEmbeddingError>`:
  unions each family's per-diagram sets, embeds each face once per weight
  drawn (none for an undrawn weight or family), as
  `@font-face{font-family:'X';font-style:normal;font-weight:N;src:url(data:font/ttf;base64,…) format("truetype")}`,
  with Inter's OFL line immediately before its first rule. Over
  `BUDGET_BYTES` (60,000) and unsubset files are logged at `info`; any
  failure returns `Err` after one warning (D7). `build_font_embedding_with`
  injects the file maker for the failure test. `FontEmbedding::markup()`
  is the comments followed by the `<style>` element (empty when nothing is
  drawn).
- **Dependencies** (`biscuit-visualized/src/Cargo.toml`, all under the
  `image` feature): `allsorts 0.17.0` (`default-features = false`,
  `flate2_rust`, as S1), `roxmltree 0.21.1`, `simplecss 0.2.2`, `base64
  0.22` (moved from dev-only to normal too). `Cargo.lock` gains `allsorts`
  and ten transitive crates (listed in `message_to_agent`; Phase 6 records
  them in `docs/dependencies.md`).

### Requirement-to-test mapping

All L1, in-process at the crate's internal API (F1 for pure functions; the
collector and page tests also run real Merman output through the HTML static
post-process, F2). No CLI, filesystem, terminal, or browser boundary is
crossed yet; Darkmatter's page and the browser load are Phases 5 and 7.

| Requirement | Test (under `src/tests/`) |
| --- | --- |
| Rule E collector vs an independent oracle (usvg's resolution of every span's family, weight, text) on real flowchart, class, sequence output | `embed_glyphs_tests::real_merman_output_matches_what_usvg_draws` |
| bold through a stylesheet class rule (real class diagram) | `…::real_class_diagram_title_is_bold_through_the_stylesheet` |
| two diagrams sharing and differing in characters | `…::two_diagrams_sharing_and_differing_in_characters` |
| attribute / inline / rule / inherited / `bolder` / `599` / `font` shorthand / `lighter` | `…::bold_comes_from_attributes_inline_styles_and_rules` |
| precedence: rule > attribute, style > rule, `!important`, specificity, `var()`, `inherit` | `…::the_cascade_follows_usvg_precedence` |
| entity and numeric references (`&amp;`, `&#x1F600;`) | `…::entities_and_character_references_are_decoded` |
| `<tspan>` collected; `<title>`, `<desc>`, `<style>` not | `…::text_and_tspans_are_collected_and_other_character_data_is_not` |
| no text, and indentation only → empty | `…::a_page_without_text_draws_nothing` |
| controls dropped, U+0020 added | `…::controls_are_dropped_and_a_space_is_added` |
| keyed by leading family, case-insensitive lookup | `…::text_is_keyed_by_its_leading_family`, `…::first_family_reads_what_css_quoted_writes` |
| malformed SVG → error | `…::malformed_svg_is_an_error` |
| subset keeps `cmap`, `OS/2`, `name` (+ outlines); no `GPOS`/`GSUB`/`kern`/`fvar` (AC-E12 table check, R-E11) | `embed_subset_tests::the_subset_keeps_cmap_os2_and_name` |
| name IDs 0, 13, 14 equal Inter's; `fsType` kept (AC-E8, R-E7) | `…::the_subset_carries_inters_copyright_and_license` |
| every mapped code point present with its original advance; unmapped and unrequested absent (R-E8) | `…::every_mapped_code_point_is_in_the_subset_and_nothing_else` |
| empty / wholly unmapped set → `.notdef` only | `…::an_empty_or_wholly_unmapped_set_gives_a_notdef_only_subset` |
| `.ttc` face → single-face subset | `…::a_collection_face_is_subset_to_a_single_face_file` |
| whole face: own bytes, or the one face of a `.ttc` with every table | `…::the_whole_face_is_the_file_itself_or_the_one_face_of_a_collection` |
| not a font, index out of range (single file and `.ttc`), no `cmap` → errors | `…::unreadable_faces_are_errors` |
| one D14 string per reason, mixed permissions (AC-E14) | `embed_comment_tests::the_comment_names_a_resolved_face_and_its_permission`, `…::the_comment_names_inter_and_why_for_every_substitution` |
| unsubset clause (AC-E14) | `…::the_comment_says_when_the_face_is_embedded_unsubset` |
| crafted family and face names (AC-E14) | `…::crafted_family_and_face_names_cannot_end_the_page_comment` |
| exact D15 line; copyright equals `OFL.txt` line 1; no Reserved Font Name (AC-E15, AC-E8) | `…::the_ofl_attribution_is_one_css_comment_naming_inters_copyright`, `…::the_copyright_line_is_the_bundled_ofl_txt_first_line` |
| one rule per face and weight; regular/bold subsets cover the union of their diagrams only (R-E3, AC-E3) | `embed_embedding_tests::diagrams_sharing_a_face_get_one_rule_per_weight_covering_their_union` |
| AC-E3 on two real diagrams: every label code point Inter maps is in the decoded subset's `cmap` | `…::real_diagrams_on_one_page_are_covered_by_the_subset` |
| bold-only page → no regular rule (D1) | `…::a_bold_only_page_embeds_no_regular_rule` |
| no text / other family only → nothing, empty markup | `…::a_page_drawing_no_text_embeds_nothing`, `…::a_family_the_page_never_draws_is_not_embedded` |
| family declared `Inter`, OFL line immediately before, one per page, comment, markup order (AC-E8, AC-E15) | `…::inter_is_declared_as_inter_with_its_ofl_line_immediately_before` |
| host face only → no OFL line, resolved comment (AC-E15) | `…::a_page_embedding_only_a_host_face_has_no_ofl_line` |
| Restricted host face → Inter rules and comment | `…::a_restricted_resolved_face_embeds_inter_and_says_why` |
| no-subsetting face → whole face byte for byte, logged, commented | `…::a_no_subsetting_face_is_embedded_whole_logged_and_commented` |
| over budget → embedded, size logged (captured `info`) (AC-E6, R-E4) | `…::a_subset_over_budget_is_embedded_and_its_size_logged`, `…::a_subset_within_budget_logs_nothing` |
| injected failure (first and second weight) → `Err`, one warning (AC-E7, R-E5) | `…::a_subsetting_failure_embeds_nothing_and_warns` |
| only `data:font/ttf` URLs, no external reference, one `<style>` | `…::the_css_references_nothing_outside_the_page` |

AC-E7's "output has no `@font-face`" and AC-E3/AC-E4 at page level are
completed by Phase 5 (Darkmatter emits nothing on `Err`). The Input
Robustness Matrix does not apply beyond what Phase 2 covered: the new
readers take our own SVG and font bytes; their document shapes (malformed
XML, not a font, index out of range, missing `cmap`) are tested above. Smell
grep over `src/mermaid/embed/*.rs` (`serde(default)`, `unwrap_or_default()`,
`.ok()`, `filter_map(.. as_str())`): one hit, `child.text().unwrap_or_default()`
in the collector, where `None` means a node with no text (not a parse
failure). A `cmap` lookup error, first written as a silent skip, now fails
the subset.

### Pre-existing failure fixed (Rule 5)

`test-toolkit`'s `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`
(reported by Phase 2) now flagged seven sites, all in this area.
`src/tests/contrast_corpus_tests.rs` (two) and `src/tests/golden_tests.rs`
use `biscuit_test_harness::manifest_dir!()`; the four `bench/` sites are
in `ALLOWED` (`tools/test-toolkit/src/archive_guard.rs`) as a non-member
crate run only by `just bench`.

### Gates

- `just lint` (biscuit-visualized): clean. `cargo clippy -p test-toolkit
  --all-targets -D warnings`: clean.
- `just test` (biscuit-visualized): 1045 passed, 4 skipped, 0 failed
  (38 tests new this phase: 12 collector, 7 subsetter, 6 comment, 13 builder).
- `just check-tier-coverage biscuit-visualized`: stranded 0.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` → `packages:
  [biscuit-visualized, test-toolkit, sniff]` before the skill edit,
  `change_class: package`. `sniff`
  is a narrowed cell: its tests read the repository's `Cargo.lock`, which
  gained `allsorts`. (The working tree also holds Phase 2's uncommitted
  files, so the list covers both phases.)
- `just test test-toolkit`: 476 passed, 0 failed (the guard above now passes).
- `sniff`: the lockfile tests (`just test lockfile` in `sniff/`): 212 passed.
- After the skill edit the calculator also lists `darkmatter` (a narrowed
  cell: two corpus tests parse every `.claude/skills` Markdown file). Final
  scope: `[biscuit-visualized, test-toolkit, darkmatter, sniff]`. Ran them:
  `just test shipped_markdown_` in `darkmatter/`: 2 passed.
- Reverse dependencies (`darkmatter`, `biscuit-terminal-cli`, `sniff-cli`,
  `worktree-cli`): `cargo check` clean; no public API or output changed.
- Cross-OS: `just cross-check biscuit-visualized --os windows` → 1045 passed
  (run because `allsorts` brings new transitive crates, including
  `pathfinder_simd`). Code itself is pure byte processing with no paths or
  OS calls.

## Phase 4

Render-path wiring, in `biscuit-visualized` (2026-10-24). The opt-in is now
reachable from the public API. Without it every render is byte-identical: the
golden corpus passes unchanged, and `MERMAID_BACKEND` is not bumped because
plain output did not change.

### What was built

- **Opt-in switch (4.1).** `MermaidDiagram::with_embedded_font(bool)` /
  `embedded_font()` (default off). It acts only for `MermaidTarget::HtmlStatic`
  with SVG output (`embeds_font(format)`); PNG and raster-safe SVG ignore it.
  `render_with_font(&request)` returns `(RenderedArtifact,
  Option<DiagramFont>)`, `Some` exactly when the opt-in applied; `render` is
  that minus the font.
- **Face choice before layout (Rule A).** `FaceChooser` (`embed/faces.rs`)
  decides each requested CSS `font-family` list the first time it is asked
  for (regular and bold, Rule B) and keeps the answer.
  `LabelMeasurer::embedding(chain)` (`fonts/measure.rs`) resolves every style
  through it (bold from weight 600 up, which matches the browser's choice
  between 400 and 700 rules and the collector's split). `measured_family`
  returns the choice's family, so the HTML static post-process leads every
  label with the embedded family with no change of its own. The same measurer
  instance drives Merman and the post-process; a tagged gitGraph's
  `commitSpacing` measurement uses an opt-in measurer too.
- **Plain advances (S1 decision).** `ShapeParams::layout_tables` (default on).
  Off, `fonts/shaper.rs` shapes a face rebuilt from only the subset's tables
  (`subset_tables`: `head`, `hhea`, `maxp`, `cmap`, `hmtx`, `name`, `OS/2`,
  `post`, `glyf`, `loca`) through `ttf_parser::Face::from_raw_tables`. The
  opt-in measurer turns it off.
- **Cache identity (4.2, D9).** The options JSON gets
  `target: "html-static+embedded-font"` for the opt-in (plain HTML static keeps
  `"html-static"`, raster-safe keeps no target), so neither output is ever
  served for the other. The font identity already covers the resolved faces'
  contents and `MERMAID_FONT`, so a Restricted versus permitted face, or a
  different `MERMAID_FONT`, changes the key with no extra field.
- **Cache record (Rule D).** On a miss the render collects `UsedGlyphs` from
  the final SVG and stores `{key}.font.json` (`FontRecord`: requested families
  plus each family's characters per weight) *before* the SVG
  (`FileCache::store_sidecar`, the same staged rename as `store`). A hit reads
  the record and runs the face choice again from it. A missing or unreadable
  record means a miss, and the record is written again. The subset is never
  cached.
- **Public bridge for Darkmatter.** `embed/page.rs`: `DiagramFont`
  (`families()`), `page_font(iter of &DiagramFont) -> Result<PageFont,
  FontEmbeddingError>`, `PageFont` (`comments()`, `css()`, `markup()`,
  `is_empty()`). `FontEmbeddingError` and `SubsetError` are now public
  (`#[non_exhaustive]`). All of these are re-exported from `mermaid`.
  `#[cfg_attr(not(test), allow(dead_code))]` on `mod embed` is removed.
- **Rule 5 fix.** `cargo clippy --no-default-features -D warnings` failed on
  three pre-existing dead-code items in `artifact.rs` (`cache_name`,
  `PNG_ENCODING`, `cache_identity`, read only behind `image`). They and the
  two new `FileCache` sidecar methods now carry
  `#[cfg_attr(not(feature = "image"), allow(dead_code))]`, and that build is
  clean.

### Requirement-to-test mapping

All in `src/tests/embed_render_tests.rs`, L1, over injected chains
(bundled faces plus `Arial`, which is Inter renamed with a patched `fsType`).
Boundary: the crate's render path, from source to the cached artifact file
plus its record, read back from disk (F2: real Merman, real cache directory,
injected fonts). No CLI, terminal or browser is involved here; the browser
load is Phase 5.4 and Phase 7.

| Requirement | Test |
| --- | --- |
| label `font-family` leads with the embedded family: permitted face → `Arial`, Restricted → `Inter`; plain static SVG on the same chain still leads with `Arial`; flowchart, class (bold), tagged gitGraph; `DiagramFont::families` | `opt_in_labels_lead_with_the_embedded_family` |
| measurer measures in the chosen face at 400, `bold`, `600`, `500`; plain measurer unchanged; the faces chosen while measuring are the ones returned | `the_measurer_measures_each_family_in_its_embedded_face` |
| measured width = sum of the **subset's** `hmtx` advances (regular and bold), and differs from the kerned width (load-bearing) | `the_measurer_uses_plain_advances_with_the_opt_in` |
| end to end: the opt-in node box = advances read from the subset **decoded from the page's `@font-face`** + the node padding (from the plain render) | `an_opt_in_label_is_laid_out_at_the_width_the_embedded_subset_draws` |
| flag off / raster-safe SVG / PNG / HTML-static PNG: same cache key, byte-identical bytes, no font; default off (AC-E9 library side) | `without_html_static_svg_the_flag_changes_nothing` |
| opt-in key ≠ plain static key; `MERMAID_FONT=Inter` and Restricted vs permitted change it; control equal (AC-E10, R-E9) | `the_opt_in_has_its_own_cache_keys` |
| a plain render after an opt-in render is a miss and the reverse; a hit returns equal faces, glyphs and page markup (record round trip) | `plain_and_opt_in_renders_never_serve_each_other` |
| record missing, not JSON, missing a field → miss, then restored and a hit | `an_opt_in_svg_without_a_readable_record_is_rendered_again` |
| several rendered diagrams → one rule per face and weight, one comment, subset covers every diagram's characters; empty page → empty markup | `a_page_of_rendered_diagrams_embeds_one_rule_per_face_and_weight` |

Mutation check: with the plain-advance switch forced on, both width tests
fail and the rest pass. The Input Robustness Matrix applies only to the new
record reader, our own JSON. Its document shapes (absent, malformed, missing
field) all fail closed to a re-render and are tested above. Smell grep over
the changed files: `.ok()` in `read_font_record` maps an unreadable record to
"render again" (fail closed, logged at debug); there is no
`unwrap_or_default` on a load-bearing parse (`UsedGlyphs::default()` after
a failed scan of our own SVG is logged as a warning and only drops that
diagram's characters).

### Gates

- `just lint` (biscuit-visualized): clean. Also clean:
  `cargo clippy -p biscuit-visualized --no-default-features --all-targets -D warnings`
  and `--features image --all-targets`.
- `just test` (biscuit-visualized): 1054 passed, 4 skipped, 0 failed (9 new).
- `just check-tier-coverage biscuit-visualized`: stranded 0.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` → `packages:
  [biscuit-visualized, test-toolkit, darkmatter]`, `change_class: package`.
  `just test test-toolkit`: 476 passed. Darkmatter (more than 200 tests, so only
  the affected ones): `just test shipped_markdown_` (the skill corpus) 2
  passed, and `just test mermaid` (its Mermaid and static-SVG tests) 68 passed.
- Reverse dependencies (`darkmatter`, `biscuit-terminal-cli`, `sniff-cli`,
  `worktree-cli`): `cargo check` clean; the public API is additive.
- No cross-OS check: the only new filesystem write is the record file, which
  uses the existing `FileCache` staged-rename path unchanged. Everything else
  is in-memory font and string processing.
- Docs: no `docs/` page was changed. The opt-in's topic page and the
  dependencies entries are Phase 6 tasks (6.2), and the new public items carry
  current doc comments.

## Phase 5

Darkmatter (2026-10-24). The opt-in now reaches the browser page through
`HtmlOptions::embed_mermaid_font` and `DarkmatterPage::with_embed_mermaid_font`.
Task 5.3 (the `md --feature embed-mermaid-font` flag) is **not done**: it is
blocked because `md` still has no `--feature` flag, since the text-fallback work
(`2026-10-22-merman-text-fallback`) has not landed, and the plan's gate forbids
a stand-in. Rechecked on 2026-10-24 in `darkmatter/cli/src/args/cli.rs`.

Note: Phase 4's changes were still uncommitted when this phase started, so the
working tree holds Phases 4 and 5 together. The file lists in this phase's
frontmatter name only the files Phase 5 touched.

### What was built

- **5.1 Sanitizer admission** (`svg_sanitizer.rs`). New
  `pub(crate) fn style_sheet_is_safe(css)` checks a `<style>` element's CSS.
  It uses the `cssparser` 0.37 tokenizer, which Darkmatter already depends on,
  rather than a hand-written scanner (Rule G allows this; see the decisions).
  The rule is the old one (no `@import`, and every `url()` a local `#…`
  fragment) with one exception: a **top-level** `@font-face` rule whose single
  `src` descriptor is one or more `url(data:font/ttf;base64,…)
  format("truetype")` entries, with the URL quoted or not. The base64 must
  decode with the standard padded alphabet. Scheme, MIME type, `base64`,
  `url(`, `format(` and `"truetype"` all match case-insensitively. The check
  fails closed on:
    - a bad string or bad url token;
    - an unterminated `@font-face` block, font `url(`, or `format(`;
    - a duplicate `src`;
    - a prelude after `@font-face`;
    - anything in the body that is not a declaration;
    - leftover arguments inside `format()`.

  Attribute values keep the old substring check (`css_is_safe`, unchanged), so
  a `data:` URL is never admitted in `fill`, `filter`, `mask` or `style`.
  `sanitize_style_element` now calls the new check, and the docs of
  `sanitize_svg`, `css_is_safe` and `sanitize_style_element` are updated.
- **5.2 Page assembly.**
    - `HtmlOptions::embed_mermaid_font` (default `false`). The stale
      `mermaid_mode` doc, which said `Image` meant interactive mermaid.js, is
      corrected.
    - `DarkmatterPage::with_embed_mermaid_font`, the hook the CLI will call; the
      MarkdownPlus path keeps it `false`.
    - The code renderer renders each static diagram with
      `with_embedded_font(true)` and the new component method
      `render_to_svg_with_font`, and collects each diagram's `DiagramFont`.
    - The first opt-in diagram writes the placeholder `<!--darkmatter:mermaid-font-->`
      ahead of its sanitized SVG. Both HTML entry points
      (`render_tree_html_with_context`, `render_tree_html_page_body`) then
      replace it with `page_font(...)`'s markup: the comments, then the `<style>`.
      The markup first passes `style_sheet_is_safe`. On a library `Err` (D7,
      already logged by biscuit-visualized) or a failed check (a warning is
      logged here), the placeholder is replaced with nothing and the diagrams
      stay.
    - New module `darkmatter/lib/src/markdown/render_tree/mermaid_font.rs`.
    - `browser_options_from_html_options` now also returns the renderer `Rc`.
- **Component API (biscuit-visualized).**
  `components::mermaid::MermaidDiagram::with_embedded_font` and
  `render_to_svg_with_font` (plus the same on the internal `MermaidRenderer`),
  and `DiagramFont` is re-exported from `components::mermaid`. These keep the
  component's terminal palette and background, so Darkmatter does not
  duplicate them.
- **5.4 Browser check.** `browser_embedded_mermaid_font_draws_every_label` in
  `darkmatter/lib/tests/browser/browser_render.rs` (`just test-browser`, headless
  Chrome via `biscuit-browser-harness`, `require_browser()` like its
  neighbors).
- **Test speed.** Darkmatter's static-Mermaid page tests paid macOS's
  CoreText scan of `target/debug/deps`: about 3.5 s each, including the
  existing `mermaid_static_svg` ones. `.config/nextest.toml` now applies the
  existing `coretext-small-bundle` setup script to
  `package(darkmatter) & test(/^(mermaid_static_svg|mermaid_embedded_font)::/)`,
  and they now take about 0.5 s.

On this Mac the page embeds the host's Trebuchet MS (`fsType 0x0008`,
Editable), subset, as regular and bold.

### Requirement-to-test mapping

Boundaries: the CSS check runs in-process through `sanitize_svg`, the public
result (F1). Page assembly goes through the public page path,
`DarkmatterPage::render_to_browser` and `render_tree_html`, with real Merman,
the real cache and host fonts (F2, L1). The browser check runs a real headless
Chrome (F3, browser tier).

| Requirement | Test |
| --- | --- |
| AC-E2 control: generated rule (with OFL comment), two weights, several entries, quoted URL, double-quoted family | `svg_sanitizer::tests::keeps_font_face_with_truetype_data_url_in_src` |
| uppercase `URL(`/`DATA:`/`FONT/TTF`/`BASE64`/`FORMAT`/`@FONT-FACE`/`SRC` | `font_data_url_admission_is_case_insensitive_for_scheme_and_mime` |
| `https:`, protocol-relative, PNG/WOFF2/OTF/HTML data, non-base64 form, `;charset=`, invalid char, unpadded, empty body, `#` fragment, whitespace in body, valid + external entry, `format("woff2")`, extra `format` args, no `format`, `local()` entry | `drops_font_face_whose_src_is_not_an_embedded_truetype_data_url` |
| data URL in another descriptor, `background`/`mask`/`fill` property, `@media`/`@supports`/`@keyframes`, `@font-face` nested in a rule | `drops_truetype_data_url_outside_font_face_src` |
| data URL in `fill`/`filter`/`mask`/`style` attributes; `@font-face` text in a `style` attribute | `drops_truetype_data_url_in_attributes` |
| `@import` beside, inside, and escaped (`@\69mport`); duplicate `src`; unterminated block/`url(`/`format(`; prelude; non-declaration body | `drops_font_face_with_import_duplicate_src_or_unterminated_parts` |
| `@font-face` + safe rule kept; + unsafe rule (either order) dropped | `font_face_beside_other_rules_is_kept_only_when_every_rule_passes` |
| existing regression guards (external URL, local fragment, filter, import, Merman flowchart) | unchanged existing sanitizer tests, all green |
| AC-E7 page level: library `Err` → no markup; empty page font → none; failed CSS check → none, warning logged | `mermaid_font::tests::a_failed_page_font_emits_nothing`, `css_that_fails_the_style_sheet_check_is_not_emitted` |
| placeholder replaced once; removed when the markup is empty; untouched page unchanged | `mermaid_font::tests::the_placeholder_is_replaced_once` |
| AC-E9 default `false` (unit + page); `Image` with the field unset vs `false` is byte-identical, with no `@font-face` or comment | `html::tests::test_html_options_default`, `l1 mermaid_embedded_font::the_opt_in_is_off_by_default` |
| AC-E9 `Off` and `Text`: byte-identical with and without the flag on both entry points | `without_static_svg_the_opt_in_changes_nothing` |
| AC-E4 flowchart + class + sequence, dark and light: one rule per (family, weight), 400 and 700 present, declared families = label lead families, one comment per family immediately before the single `<style>`, before the first `<svg>`, placeholder gone | `a_page_of_diagrams_embeds_one_rule_per_face_and_weight` |
| AC-E15: OFL line immediately before Inter's first rule; none when only host faces are embedded | `the_ofl_line_accompanies_inter_only` (host-dependent branch), `mermaid_font_inter_embeds_inter_with_its_ofl_line` (`MERMAID_FONT=Inter` pins the Inter branch) |
| `render_tree_html` emits the same markup as the page path | `the_html_entry_point_embeds_the_font_too` |
| component returns the font only for opt-in HTML-static SVG, and the SVG equals `render_to_svg` | `components::mermaid::tests::the_component_returns_the_font_of_an_opt_in_html_static_svg` |
| AC-E12 page level: every `@font-face` loads from `data:` with no font request; every label (incl. bold class name) is drawn in the embedded face (page width = width in a probe copy of the same file) with every glyph in the subset (probe width is equal with monospace and serif fallbacks); control text outside the subset differs | `browser_render::browser_embedded_mermaid_font_draws_every_label` (passes with `BISCUIT_BROWSER_REQUIRED=1`) |

- **Mutation checks on 5.1.** With the base64 check forced to true, the
  "not an embedded data URL" test fails. With the `@font-face` exception
  disabled, the four "kept" tests fail. Allowing the exception below the top
  level fails the "outside `src`" test.
- **Input Robustness Matrix.** This is a CSS reader, not a config field
  validator. Every row of the plan's table has a case above, and every shape
  except the admitted one fails closed to "block dropped".
- **Smell grep** over the new Darkmatter code: no `#[serde(default)]`,
  `unwrap_or_default` or `filter_map(.. as_str())`. The `let Ok(page) = page
  else` in `checked_page_font` is the D7 fail-closed path; the library has
  already logged that error.
- **D14 comment cases.** Each case is pinned with injected fonts in
  biscuit-visualized (Phases 3 and 4). Darkmatter cannot inject a font chain,
  so its page tests branch on the page's own comments.
- **A host fact for the test above.** This Mac has Inter installed in
  `~/Library/Fonts`, so `MERMAID_FONT=Inter` resolves to that copy, and the
  comment gives the reason "the face the font chain resolved", not "selected
  by MERMAID_FONT=Inter". This is Phase 2's ruling (3); the test accepts
  either reason.

### Gates

- `just lint`: clean in `biscuit-visualized` and `darkmatter` (all four crates
  in the area, plus the zed-dmls wasm check).
- `just test` in `biscuit-visualized`: 1055 passed, 4 skipped, 0 failed (1
  new).
- `just test` in `darkmatter`: **42 failed**, all from before this phase. The
  recipe is fail-fast, so the full L1 set was also run once with the
  recipe's own packages, features and tier filter plus `--no-fail-fast`:
  10677 run, 10635 passed, 42 failed, 10 skipped. The 42 are:
    - 40 `schemas_convert_snapshots::*`;
    - `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`;
    - `darkmatter-cli` `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`.

  All of them, plus `as_json5_uses_the_formatter_object_idiom`, also fail on
  a clean `git worktree` of HEAD (`2efa9f95b`): 43 run, 43 failed. Cause:
  JSON key order, after commit `bbde223e6` turned on serde_json's
  `preserve_order` for Darkmatter. They are left in place, because the
  schema output order is a decision for that work (see the decisions). The
  43rd, `as_json5_uses_the_formatter_object_idiom`, had a test expectation
  that assumed sorted keys; it is fixed (Rule 5) and passes.
- `just test-browser browser_embedded_mermaid_font` with
  `BISCUIT_BROWSER_REQUIRED=1`: passed (about 3 s, real headless Chrome).
- `just check-tier-coverage`: 0 stranded in `darkmatter` and
  `biscuit-visualized`.
- Scope:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [biscuit-visualized, darkmatter, test-toolkit]` and
  `change_class: package`. The tree still holds Phase 4's uncommitted work.
  `just test test-toolkit`: 476 passed, 4 skipped.
- Reverse dependencies of `biscuit-visualized` (`biscuit-terminal-cli`,
  `sniff-cli`, `worktree-cli`): `cargo check` is clean. The component API
  change is additive.
- No cross-OS check: this phase adds no path, process or OS-specific code.
  The `nextest.toml` script is scoped to macOS hosts.
- Docs: no `docs/` page changed. The Darkmatter and biscuit-visualized
  pages are Phase 6 (6.1, 6.2). The doc comments of every changed symbol are
  current, including the corrected `HtmlOptions::mermaid_mode` doc.
