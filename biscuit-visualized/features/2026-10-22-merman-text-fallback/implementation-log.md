---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-22-merman-text-fallback/spec.md
plan: biscuit-visualized/features/2026-10-22-merman-text-fallback/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - renderable
    - biscuit-terminal
    - biscuit-visualized
decisions_for_review:
    - |-
      Swimlane is eligible for text fallback. The spec listed six eligible families (flowchart, sequence, state, class, ER, XY chart), taken from Merman's support-matrix document. Merman's own metadata (`ascii_capabilities()`) at the delivered revision lists a seventh, swimlane: diagrammatic, partial, and admitting the auto layout. The document folds it into the flowchart row. Because D1 says eligibility comes from the metadata and never from a hand-kept list, swimlane is eligible. I amended D1 and AC-F3 under D10, so the eligibility snapshot will list seven families. Alternative: exclude swimlane explicitly. That needs a family-name exclusion list, which D1 forbids. If the reviewer wants swimlane excluded, Phase 3 gains one exclusion and its snapshot loses one row.
    - |-
      Rulings A–H, which the plan left for the author, were made by the implementer, adopting each recommendation (no human was in the loop). A: the diagram hook returns a three-way outcome (drawn output, render-tree nodes, declined). B: darkmatter's Mermaid mode gains an automatic variant, which `md --mermaid` uses; the library's image mode keeps meaning image-only. C: the three fallback feature names and the "most restrictive wins" fold live in biscuit-visualized behind its `clap` feature; `md` adds them, plus `no-mermaid-rendering`, to the `--feature` enum it already has. D: clap already rejects empty and unknown names with the valid names listed (proved by a throwaway program), so no custom parser is needed. E: automatic mode chooses text at full graphics when the terminal reports no image protocol. F: drawing uses Unicode when the terminal's existing "supports Unicode" flag is set, and ASCII otherwise. G: `bt git-graph` accepts the flags with no visible effect. H: no extra measurement. Reversing A changes Phase 2's trait signature and its test doubles. Reversing B or C changes only Phase 5's CLI wiring.
    - |-
      Until Phase 4, biscuit-visualized's diagram hook declines below full graphics. The trait change forced an edit to the one real hook. The plan put that hook's automatic behavior in Phase 4, but it now receives the graphics ceiling, and left unchanged it would draw an image when only text is allowed. So for now it returns "declined" (show the source as a code block, no warning) whenever the ceiling is below full graphics. Alternatives: keep drawing images regardless (breaks the ceiling rule), or build the text route now (that is Phase 3-4 work). Phase 4 replaces the decline with the text route. Nothing else needs redoing if the reviewer prefers another interim.
    - |-
      A diagram hook's fragment is rendered with Mermaid promotion turned off. The spec says the hook may return a source fallback as a `mermaid` code node. Rendered normally in automatic mode, that node would be handed back to the hook, which would return it again, forever. The renderer therefore lays the fragment out with promotion off, so its `mermaid` node prints as code. Alternative: let hooks return a different language tag for the source, which would lose Mermaid highlighting (D8/D9 require `mermaid`). If the reviewer wants another guard, only `render_diagram_fragment` in biscuit-terminal's renderer changes.
    - |-
      A new biscuit-terminal docs page, `biscuit-terminal/docs/render-tree.md`, documents the code-renderer and diagram hooks. The plan asked to update "the biscuit-terminal render-tree topic page", but none existed; only a README section did, and READMEs are overviews in this repo. Alternative: put the content in the README, which goes against that rule. If declined, the page's content would move into the renderable doc, which already has the conceptual version.
    - |-
      The text render is available only with the crate's `terminal` feature. The spec requires the fit check to use biscuit-terminal's display-width measurement, and biscuit-terminal is a dependency only under `terminal`. The eligibility lookup needs no terminal and stays under `image`. Alternatives: measure with the `unicode-width` crate directly (the same rule biscuit-terminal uses, but a second copy of it), or make `image` depend on biscuit-terminal (a much wider build for every image-only user). Every caller of the text render (the terminal component, the Markdown hook) already has `terminal`. If the reviewer wants the text render under `image`, only the `#[cfg]` on `render_text`, `judge` and `ascii_request` and the width function change.
    - |-
      The text render takes no display-width profile; it always asks Merman for its standard (non-CJK) widths. The plan listed a display-width profile among the options, but biscuit-terminal, which measures fit, has no CJK width mode, so a CJK request would be measured differently from how it was laid out. The time budget is also not a text option: the diagram's existing `with_time_budget` applies to text requests, as the spec's "the prerequisite's configurable deadline" says. Alternative: add both options; a CJK option would need a CJK measurement in biscuit-terminal first.
    - |-
      Over-wide output is requested with Merman's "allow overflow" policy, so Merman reports the overflow and the render shows the source. The spec's AC-F2 expects a dense class or ER diagram to come back as a `relations:` report (`StructuredText`). At the delivered rev Merman returns that report for a drawable family only under its other policy, "fall back on overflow", which this code does not use. So in practice an over-wide class diagram is reported as too wide, not as a report; both show the source. The projection guard (D1) is kept and tested against Merman's real report, produced by a test with the fallback policy. Alternative: request the fallback policy. That adds a Merman error ("fallback unavailable") that would have to be mapped to "show the source", and gains nothing visible.
    - |-
      Class and ER diagrams that Merman cannot route are drawn as stacked boxes followed by a `relations:` list, and Merman still reports the result as a drawing (`Diagrammatic`). Following D1 (Merman's metadata and reported projection decide, nothing else), these are shown as text. A simple ER chain (A to B to C) is drawn with real connectors. If the reviewer finds the boxes-plus-list output not a faithful-enough drawing, excluding it would mean inspecting the output text, which D1 forbids; the cleaner fix would be upstream in Merman's reported projection.
    - |-
      The `\n` to `<br/>` label rewrite that the SVG path applies to class, ER, mindmap and requirement labels also applies to text (the spec: same label normalization). In Merman's class text a rewritten label prints as `one<br>two`; without the rewrite it prints `one\ntwo`. Neither draws two rows, so the rewrite was kept for parity. Changing it means skipping the rewrite in `render_text` only.
message_to_agent: |-
    Phase 3 landed the text backend; read "## Phase 3" in this log for details.

    Facts for Phase 4:
    (1) API (all `biscuit_visualized::mermaid`): `MermaidDiagram::render_text(TextOptions) -> Result<TextRender, MermaidError>` and `MermaidDiagram::text_eligibility() -> Option<TextEligibility>`. `render_text` exists only with the `terminal` feature. `TextOptions::new(max_columns).with_charset(TextCharset::{Unicode, Ascii})`. `TextRender::Drawing(TextDrawing)` (`text()`, `into_text()`, `coverage() -> TextCoverage::{Full, Partial}`, `report()`) or `TextRender::Source(TextSource)` with `Ineligible`, `NoColumns`, `NotDiagrammatic(report)`, `TooWide(report)`, `Empty(report)` (`#[non_exhaustive]`). `Err` = parse error, Merman failure, time budget (already logged as a warning) or resource limit. Phase 4's infallible component maps every `Source` and every `Err` to the `mermaid` code node; the hook returns `Declined` for `Source` and `Err` for `Err`.
    (2) Policy belongs in Phase 4: "no partial" means `drawing.coverage() == Partial` -> source; "never" means skip `render_text` entirely (zero requests). The D3 policy is not in the backend.
    (3) The text never contains control characters other than `\n`. Rows may end in significant spaces; do not trim them. A trailing `\n` may be present; trim only trailing newlines.
    (4) Counters (crate-internal, `#[cfg(test)]`, thread-local, reset on read): `crate::mermaid::render::take_text_requests()`, `crate::fonts::chain::take_default_chain_requests()`, `crate::cache::file_cache::take_stored_files()`. They are visible only to the crate's own unit tests (`src/src/tests/`), not to the `tests/l1` binary, so put the AC-F13 component-level tests in `src/src/tests/` (the `components` module is compiled there under `--all-features`).
    (5) Image availability must be decided before calling `render_text`, and no image work may run on the text route (the counters prove it).
source_files_during_phase_2:
    - renderable/src/tree/mod.rs
    - biscuit-terminal/lib/src/render_tree/options.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/src/render_tree/mod.rs
    - biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs
    - biscuit-visualized/src/src/components/render_hook.rs
    - biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs
docs_updated_during_phase_2:
    - renderable/docs/tree-rendering.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-terminal/README.md
    - biscuit-terminal/lib/README.md
docs_created_during_phase_2:
    - biscuit-terminal/docs/render-tree.md
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-terminal/diagrams.md
    - .claude/skills/biscuit-visualized/diagram-components.md
    - .claude/skills/renderable/tree.md
source_files_during_phase_3:
    - biscuit-visualized/src/Cargo.toml
    - biscuit-visualized/src/src/mermaid/text.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/fonts/chain.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/text_tests.rs
docs_updated_during_phase_3:
    - biscuit-visualized/docs/dependencies.md
    - docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
---

# Implementation Log for 2026-10-22-merman-text-fallback (7 phases)

## Phase 1

Phase 1 changes no shipped code. It records the rulings, the prerequisite
state, and the exact API names Phases 2–5 use. Recon used a throwaway program
outside the repository (`/tmp/bv-recon-probe`, path-depending on the cargo git
checkout of the delivered Merman rev); nothing from it is committed.

### Rulings (2026-10-24, made by the implementer; see `decisions_for_review`)

- **Rule A: hook success shape.** Adopted. `DiagramRenderer::render_terminal_diagram`
  returns `Result<DiagramOutcome, DiagramRenderError>`, where `DiagramOutcome`
  is a generic `#[non_exhaustive]` enum: `Rendered(String)` (today's
  pre-formatted output), `Nodes(Vec<RenderNode>)` (a fragment laid out in
  place) and `Declined`. In-tree implementors to update:
  `biscuit-visualized/src/src/components/render_hook.rs:30`
  (`TerminalDiagramRenderer`), `biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs:52`
  (`RecordingDiagrams`) and `biscuit-terminal/lib/src/render_tree/render.rs:5567`
  (`FailingDiagrams`). Trait and request: `biscuit-terminal/lib/src/render_tree/options.rs:386-413`.
- **Rule B: darkmatter `MermaidMode`.** Adopted. Add `MermaidMode::Automatic`
  (`darkmatter/lib/src/markdown/output/terminal.rs:317`), map it to
  `TerminalMermaidMode::Automatic`, and have `md --mermaid`
  (`darkmatter/cli/src/render.rs:70-74`, today `Image`/`Off`) pass
  `Automatic`. `Image` keeps meaning image-only. The browser mappings
  (`entrypoints.rs:296`, `:604`) must also handle `Automatic`; the browser
  treats it as `Image` (static SVG at the HTML static path) or as `Off`
  (interactive). Phase 5 decides which, guided by "callers that set nothing keep
  today's behavior". `md --mermaid` today yields `Image`, which maps to
  `StaticSvg` at `:298` and `:607`, so mapping `Automatic` like `Image` keeps
  `md --mermaid --html` unchanged.
- **Rule C: where `--feature` parsing lives.** Adopted, with one adjustment
  from recon. `MermaidFallbackFeature` (clap `ValueEnum`) and the fold into the
  fallback policy live in biscuit-visualized behind its `clap` feature. `md`
  **already has** a `--feature` flag and a `Feature` `ValueEnum`
  (`darkmatter/cli/src/args/enums.rs`, `Feature::EmbedMermaidFont`), declared
  on both the implicit `md <INPUT>` form (`args/cli.rs`) and `md render`
  (`args/command.rs`). It is lowered by `apply_features`
  (`darkmatter/cli/src/render.rs:122`). This arrived in uncommitted
  working-tree changes from `2026-10-21-mermaid-embedded-font`'s review
  cycle, not from this plan. Phase 5 adds `NoMermaidFallbacks`,
  `NoMermaidPartialFallbacks`, `NoMermaidPartialWarning` and
  `NoMermaidRendering` to that enum, and maps the first three onto
  `MermaidFallbackFeature` for the shared fold. A clap `ValueEnum` cannot
  embed another enum's variants, so `md` repeats the three names. It does not
  wrap the type.
- **Rule D: empty and unknown names.** Confirmed with the throwaway program
  (clap 4, `value_enum`, `value_delimiter = ','`, `ignore_case = true`,
  `ArgAction::Append`):
    - `--feature no-mermaid-fallbacks,NO-MERMAID-PARTIAL-WARNING` → OK, both
      parsed (case-insensitive).
    - `--feature a,,b` and `--feature ""` → `ErrorKind::InvalidValue`: "a value
      is required for '--feature <FEATURE>' but none was supplied [possible
      values: …]". This is a usage error that lists the names.
    - `--feature bogus` → `InvalidValue`, "invalid value 'bogus' … [possible
      values: …]".
    - A repeated name yields a duplicate entry, which is harmless for a fold.
    - **No custom `value_parser` is needed.**
- **Rule E: image availability in Automatic mode.** Adopted. The hook trusts
  the request (no probing): `GraphicsMode::Rich` with
  `request.terminal.image_support() != ImageSupport::None`
  (`biscuit-terminal/lib/src/terminal.rs:505`, enum at
  `discovery/detection/image.rs:12`) → image. `Rich` with `ImageSupport::None`,
  or `Vector` → eligible text. `Off` → hook not called.
- **Rule F: drawing character set.** Adopted. Use the existing
  `Terminal::supports_unicode: bool` (`biscuit-terminal/lib/src/terminal.rs:309`;
  builder `TerminalBuilder::supports_unicode`). The render-tree context mirrors
  it as `TerminalRenderContext::supports_unicode` (`render_tree/options.rs:79`).
  `true` → `AsciiCharset::Unicode`, `false` → `AsciiCharset::Ascii`. Add no new
  capability. Display width:
  `biscuit_terminal::utils::block_constraint::visible_width(&str) -> u32`. It
  skips ANSI escapes and sums `unicode_width` per char (unknown → 1).
- **Rule G: `bt git-graph`.** Adopted. Recon refines its premise: `bt git-graph`
  does **not** use the typed `GitGraph` component. It builds a `gitGraph`
  Mermaid source and goes through `build_mermaid_diagram` + `display_mermaid`
  like the others (`biscuit-terminal/cli/src/commands/git_graph.rs:83-97`).
  gitGraph is `StructuredText`, so its text fallback always shows the source,
  and the flags have no visible effect. Accept and document them.
- **Rule H: wider measurement.** None.

### Task 1.1: prerequisite gate

- `2026-10-21-merman` has landed. `biscuit-visualized/src/Cargo.toml:76`:
  `merman = { git = "https://github.com/yankeeinlondon/merman.git", rev =
  "6e6b1e6554e8e75d468c1f991652c1c53a85bace", default-features = false,
  features = ["svg", "all-diagrams", "layout-cytoscape"], optional = true }`.
  The old renderer is gone (no `mermaid-rs-renderer` in the manifest).
  `src/src/mermaid/render.rs` owns the Merman requests: `renderer(&Prepared)`
  (`Renderer` with the site config), `control()` → `OperationControl::new().with_deadline(self.time_budget)`,
  `call_merman` (elapsed recording, `RenderError::Cancelled` → warning +
  `MermaidError::RenderFailed`), and `source_theme()`, which already runs Merman's
  `preprocess_diagram` + `registry().detect_type` and yields the family.
- **Delivered rev:** `6e6b1e6554e8` on the fork, branch `gitgraph-commit-spacing`.
  `git log` shows it is exactly `482fcde` plus one commit, "feat(gitgraph): add
  commitSpacing config", which touches three gitGraph render files only.
  `docs/rendering/ASCII_SUPPORT_MATRIX.md` is byte-identical between the two
  revs (`diff -q`). The spec's figures for AC-F6 and AC-F11 stand. AC-F3 was
  amended for swimlane (below).
- **Phase 3 is not blocked.**

### Task 1.2: Merman ascii API (rev `6e6b1e6`)

- **Cargo:** enable feature `"ascii"` on the existing `merman` dependency
  (`ascii = ["dep:merman-ascii"]`). The throwaway built offline with
  `["ascii", "all-diagrams", "layout-cytoscape"]`.
- **Module:** `merman::ascii` re-exports the `merman-ascii` types (listed
  below), plus `normalize_terminal_text(&str) -> Cow<str>` ("normalizes
  untrusted authored text for terminal display without changing printable
  text").
- **Capabilities:** `merman::ascii::ascii_capabilities() -> &'static [AsciiCapability]`,
  sorted by `diagram_type`. `AsciiCapability` (plain struct, public fields)
  has these fields:
    - `diagram_type: &'static str`
    - `display_name`
    - `semantic_coverage: Option<AsciiSemanticCoverage>` (`Full`/`Partial`,
      `#[non_exhaustive]`)
    - `primary_projection: AsciiPrimaryProjection` (`Diagrammatic`,
      `StructuredText`, `None`)
    - `structured_text_fallback: bool`
    - `layout_profiles: &'static [AsciiLayoutProfile]`
    - `width_profiles`
    - `encodings`
    - `support_level`
    - `supported_semantics`
    - `limits`
    - `evidence`

  It also has `supports_layout_profile(AsciiLayoutProfile) -> bool`.
  `ascii_diagrammatic_diagram_types()` is the `Diagrammatic` subset.
- **Matching the parsed family to a row:** take Merman's detected type (the
  `detect_type` result `source_theme()` already computes), then
  `merman::diagram_type_metadata_id(detected)` (public via `pub use
  merman_core::*`), then an exact match on `AsciiCapability::diagram_type`.
  Probed:

  | Source header | detected | metadata id | row |
  | --- | --- | --- | --- |
  | `flowchart LR`, `graph TD` | `flowchart-v2` | `flowchart` | Diagrammatic |
  | `sequenceDiagram` | `sequence` | `sequence` | Diagrammatic |
  | `stateDiagram`, `stateDiagram-v2` | `stateDiagram` | `state` | Diagrammatic |
  | `classDiagram` | `classDiagram` | `class` | Diagrammatic |
  | `erDiagram` | `er` | `er` | Diagrammatic |
  | `xychart-beta` | `xychart` | `xychart` | Diagrammatic |
  | `gitGraph` | `gitGraph` | `gitgraph` | StructuredText |
  | `mindmap` | `mindmap` | `mindmap` | StructuredText |
  | `pie` | `pie` | `pie` | None |
  | `quadrantChart` | `quadrantChart` | `quadrantchart` | None |

  Do **not** use `diagram_type_family_id`, which returns `gitGraph` and not
  the row's `gitgraph`. Merman's own `capability_for_diagram_type` is
  `pub(crate)`.
- **Derived table at `6e6b1e6`** (the R-F2 snapshot source):
    - **Diagrammatic, Partial:**
        - `class`, `er`, `state`, `xychart` with layouts `[Canonical]`.
        - `flowchart`, `sequence`, `swimlane` with layouts
          `[Canonical, Compact, Auto]`.
    - **StructuredText, Partial:** `gantt`, `gitgraph`, `journey`,
      `kanban`, `mindmap`, `packet`, `timeline`, `treeView`.
    - **None (no coverage):**
        - `agentflow`, `architecture`, `block`, `c4`, `cynefin`,
          `eventmodeling`, `info`, `ishikawa`, `pie`
        - `quadrantchart`, `radar`, `railroad`, `requirement`, `sankey`,
          `treemap`, `usecase`, `venn`, `wardley`, `zenuml`
- **Request:** `merman::RenderRequest::ascii(source, OperationControl, merman::AsciiRequest)`
  through the same `Renderer` (so the site config applies), giving
  `RenderOutput::Ascii(Option<AsciiOutput>)`. `AsciiRequest { options:
  AsciiRenderOptions, resources: AsciiResourcePolicy, viewport:
  AsciiViewportPolicy }` derives `Default`. `AsciiRenderOptions` is
  `#[non_exhaustive]`, so build it from `Default` and set these fields:
    - `charset: AsciiCharset` (`Unicode` default, `Ascii`)
    - `terminal_width_profile: TerminalWidthProfile` (`Unicode` default,
      `Cjk`)
    - `layout_profile: AsciiLayoutProfile` (`Canonical` default, `Compact`,
      `Auto`)
    - `color_mode: AsciiColorMode` (`Plain` default, so plain output needs
      nothing)

  `AsciiViewportPolicy { max_width: Option<usize>, overflow: OverflowPolicy
  (Allow default | Fallback | Error), trim: AsciiTrimPolicy }`.
- **Output:** `AsciiOutput` has these fields:
    - `projection: AsciiProjection` (`Diagrammatic`/`StructuredText`)
    - `text: String`
    - `primary_extent`/`emitted_extent: AsciiExtent { width, height }`
    - `layout_profile` (reported) and `requested_layout_profile`
    - `compact_attempted: bool`
    - `requested_max_width`
    - `overflowed: bool`
    - `outcome: AsciiOutputOutcome` (`Primary`, `WideAllowed`, `Fallback`,
      `Empty`)
    - `fallback: FallbackMetadata`
    - `lossiness`
- **Observed behavior** (spec flowchart example; output is byte-identical to
  the spec's figure):
    - Canonical, no max → `Diagrammatic`, layout `Canonical`,
      `overflowed=false`, `outcome=Primary`, extent 50×7.
    - Auto with `max_width=40` → requested `Auto`, `compact_attempted=true`,
      reported `Canonical`, `overflowed=true`, `outcome=WideAllowed`, still
      50 wide. With the default `OverflowPolicy::Allow`, over-width is
      **reported, not an error**. Phase 3 reads `overflowed` and also measures
      rows itself.
    - Canonical with `max_width=20` → `overflowed=true`, `WideAllowed`.
    - **Auto on a family that does not admit it** (state): `Err(RenderError::Ascii(InvalidOption
      { field: "layout_profile", … }))`. Only request `Auto` when
      `layout_profiles` contains it (D7), as planned.
    - **Unsupported family** (pie): `Err(RenderError::Ascii(UnsupportedDiagram {
      diagram_type: "pie" }))`. Decide eligibility before requesting, so an
      ineligible input makes zero requests.
- **Deadline/limit/cancellation:** the request's `OperationControl` deadline
  (as the SVG path) surfaces as `RenderError::Cancelled`. Resource limits
  surface as `RenderError::ResourceLimitExceeded`. Target errors surface as
  `RenderError::Ascii(AsciiError)`. The existing `call_merman` maps all of
  these.
- **biscuit-terminal accessors (Rule F):** `Terminal::supports_unicode`,
  `TerminalRenderContext::supports_unicode`, and
  `biscuit_terminal::utils::block_constraint::visible_width`.

### Task 1.3: CLI and workspace recon

- **`bt` Mermaid subcommands** (`biscuit-terminal/cli/src/commands/`):
    - The nine subcommands are `flowchart.rs`, `state_diagram.rs`,
      `erd.rs`, `pie.rs`, `quadrant.rs`, `timeline.rs`, `xy_chart.rs`,
      `git_graph.rs` and `mermaid.rs`.
    - Each builds its component with
      `commands::mermaid::build_mermaid_diagram` and shows it with
      `display_mermaid`.
    - `display_mermaid` calls the **fallible, image-only**
      `MermaidDiagram::try_render`. On `MermaidRenderError::NoImageSupport`,
      `handle_mermaid_error` prints a raw `` ```mermaid `` fence through
      `Prose` (`mermaid.rs:122-127`). That fence is the path the text fallback
      must replace in Phase 5.
    - An explicit image-export failure (`Visualization`, `DisplayError`)
      exits with an error and must keep doing so.
    - The one shared place to add `--feature` is the args each command
      passes to `build_mermaid_diagram`. Each command has its own args
      struct, so the flag is declared per command (or in a shared flattened
      args struct, like `LayoutArgs`).
- **`md` paths:**
    - The implicit `md <INPUT>` form reads its flags from `args/cli.rs`
      (`--mermaid` at `:52-55`, `--feature` at `:57-67`). `md render` reads
      them from `args/command.rs:38-48`.
    - Both reach `render_terminal_output` (`darkmatter/cli/src/render.rs:52`),
      which sets `with_mermaid_mode`, and `apply_features` (`:122`).
    - `--feature` works on both forms today.
- **Rule D:** proved (see above); the throwaway is not committed.
- **AC-F15 audit list** (exhaustive matches that must handle `TerminalMermaidMode::Automatic`
  or `MermaidMode::Automatic`):
    - `renderable/src/tree/mod.rs:101`: `TerminalMermaidMode` definition.
      There is **no** exhaustive `match` on it anywhere in the workspace. The
      only branch is an equality test at
      `biscuit-terminal/lib/src/render_tree/render.rs:1742`
      (`mermaid_mode == TerminalMermaidMode::Image`), which Phase 2 must
      widen for `Automatic`. That is a silent-behavior site, not a compile
      error, so it needs a test, not only `cargo check`.
    - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:296`: darkmatter
      `MermaidMode` → `BrowserMermaidMode` (page path). Compile-breaking.
    - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:604`: darkmatter
      `MermaidMode` → `BrowserMermaidMode` (HTML path). Compile-breaking.
    - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:723`: darkmatter
      `MermaidMode` → `TerminalMermaidMode`. Compile-breaking.
    - `darkmatter/cli/src/render.rs:70-74`: constructs `Image`/`Off` from
      `--mermaid`. This is a behavior site (Rule B).
    - Constructors only (no change needed):
        - `TerminalMermaidMode::{Image,Code}` assignments in
          `biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs`
          (108, 118, 166-169, 231) and `render_tree_code_context.rs`
        - `biscuit-terminal/lib/src/render_tree/render.rs:5587`
        - `biscuit-terminal/lib/src/render_tree/options.rs` tests
          (496, 510)
        - `biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs`
          (29, 58) and `display_size.rs:177`
        - darkmatter tests in `lib/tests/l1/{render_invariants,mermaid_static_svg,mermaid_embedded_font}.rs`
          and `lib/tests/browser/browser_render.rs`
        - darkmatter defaults in `markdown/{mod,output/*,render_tree/code_renderer,compose/tests}.rs`.
    - `BrowserMermaidMode` matches in `renderable/src/tree/render/browser.rs`
      (1214, 1220, 2144, 2150) match `BrowserMermaidMode`, which this work
      does not extend.
- **How `bt` renders code blocks today:**
    - No `bt` command installs a `code_renderer`. Only `quote.rs:116` and
      `block.rs:272` build `TerminalRenderOptions` and render a tree.
    - Mermaid sources are printed as `Prose`-wrapped fences
      (`mermaid.rs:108`, `mermaid_source_block`, and the
      `NoImageSupport` arm).
    - `graph.rs:199` prints `GraphDiagram::fallback_code_block()` the same
      way.
    - So Phase 5.4 installs darkmatter's code renderer in `quote` and
      `block`, and in every Mermaid subcommand once it renders fallbacks as
      code nodes through `TerminalRenderOptions`.
    - `graph` keeps its own fallback and is out of this spec's scope; it is
      not a Mermaid command.

### Decided during implementation

- **D1 amended (swimlane eligible).** Evidence: `ascii_capabilities()` at the
  delivered rev returns a separate `swimlane` row, `Diagrammatic`, `Partial`,
  layouts `[Canonical, Compact, Auto]`, and `ascii_diagrammatic_diagram_types()`
  returns `["class", "er", "flowchart", "sequence", "state", "swimlane",
  "xychart"]`. Merman's comment on `ascii_capabilities` says "Flowchart and
  Swimlane have separate rows even though they use the same terminal
  projection". The support-matrix document, which the spec quoted, has no
  swimlane row.
    - The choice meets D10's three conditions:
        - it keeps D1's purpose (metadata decides, no name list);
        - it is within every other decision;
        - it adds no dependency, outward action or scope.
    - Spec edits: D1 gains an "Amended during implementation" bullet, and
      AC-F3's family list adds swimlane. The Evidence section's table is left
      as the review recorded it.
    - Flagged in `decisions_for_review`.

### Requirement-to-test mapping

Phase 1 changes no behavior, so it adds no tests. The throwaway program is the
evidence for Rule D and the API facts. Phases 2–5 own the AC-F tests listed in
the plan's checkpoints. The facts above fix their exact inputs:
- the seven-family snapshot (AC-F3)
- `InvalidOption` for `Auto` on state (AC-F11's "exactly one request")
- `overflowed=true` with `WideAllowed` (over-width → source)
- `UnsupportedDiagram` for pie (pie → source, with no request)

### Gates

- The skip check (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`)
  reported `packages: ["darkmatter", "darkmatter-cli", "test-toolkit"]`,
  `change_class: package`. The cause is the pre-existing uncommitted
  embedded-font work, not this phase. The gates were therefore run.
- `just lint` in `biscuit-visualized`: clean.
- `just test` in `biscuit-visualized`: 1070 passed, 4 skipped, 0 failed (13.1 s).
- There are no dependent-package tests, because no package source changed.
  There is no cross-OS check, because Phase 1 has no OS-specific code.
- The biscuit-visualized skill is unchanged; Phase 1 changed no architecture.

## Phase 2

Generic contracts only: `renderable` gains the automatic mode and
`biscuit-terminal`'s diagram hook gains the ceiling, the declined outcome and
node output. No Mermaid-specific type was added to `biscuit-terminal`.

### Task 2.1: `TerminalMermaidMode::Automatic`

- Added `Automatic` to `renderable/src/tree/mod.rs` with the D5 meaning; `Code`
  stays the default.
- AC-F15: `cargo check --workspace --all-targets` is clean. As Phase 1 found,
  there is no exhaustive `match` on the enum; the only consumer is the
  promotion test in `render.rs`, now a `matches!` on `(mode, ceiling)`, pinned
  by tests (below). Darkmatter's own `MermaidMode` is untouched (Phase 5).

### Task 2.2: hook contract

- `options.rs`: `DiagramRenderer::render_terminal_diagram` returns
  `Result<DiagramOutcome, DiagramRenderError>`; `DiagramOutcome`
  (`#[non_exhaustive]`, `Clone`, `PartialEq`) is `Rendered(String)`,
  `Nodes(Vec<RenderNode>)`, `Declined`. `DiagramRenderRequest` gains
  `graphics_mode: GraphicsMode`. Re-exported from `render_tree`.
- `render.rs` `render_code_node`: the hook is consulted for `Image` at `Rich`
  and for `Automatic` at `Rich`/`Vector`, never at `Off`. `Rendered` prints
  verbatim; `Declined` falls through to the code renderer/plain code block with
  no diagnostic; `Err` keeps today's strictness handling.
- `Nodes` goes through a new `render_diagram_fragment`, which renders the
  fragment with `render_blocks` at the fence's context (so indent and width are
  the fence's) and **with `mermaid_mode = Code`**. Without that, a `mermaid`
  source-fallback node in the fragment would be offered to the hook again and
  recurse forever in Automatic mode.
- Test doubles updated: `FailingDiagrams` (`render.rs`) and `RecordingDiagrams`
  (`lib/tests/l1/render_tree_diagram_hook.rs`).
- The one real implementor, `biscuit-visualized`'s `TerminalDiagramRenderer`,
  had to change for the build. It wraps its image in `Rendered` and returns
  `Declined` when `graphics_mode != Rich`, so it never draws an image above
  the ceiling until Phase 4 adds the text route (see `decisions_for_review`).

### Docs

- `renderable/docs/tree-rendering.md`: new "Terminal Mermaid promotion and the
  diagram hook" section (modes table, ceiling in the request, the four answers
  with a Mermaid flow, code nodes through `code_renderer`, example).
- New `biscuit-terminal/docs/render-tree.md`: the two render-tree hooks, what
  each receives, what the renderer does with each answer; linked from
  `biscuit-terminal/README.md` and `biscuit-terminal/lib/README.md`.
  biscuit-terminal had no render-tree topic page, so the plan's "biscuit-terminal
  render-tree topic page" was created.
- `biscuit-visualized/docs/components/mermaid_diagram.md`: when the hook is
  offered a fence now includes Automatic; at `Vector` it declines.
- Skills: `biscuit-terminal/diagrams.md` (hook contract),
  `biscuit-visualized/diagram-components.md` (real hook behavior),
  `renderable/tree.md` (new "Terminal Mermaid modes" section). **Drift fixed:**
  `renderable/tree.md`'s `CodeRenderer` signature showed `width: u32` and no
  `meta`; the code takes `meta` and a `TerminalCodeContext`. The skill was
  corrected to the code, and a stale note about a planned widening removed.

### Requirement-to-test mapping

Boundary: the generic renderer ↔ a caller's hook, across the crate boundary
(F2, L1, explicit `Terminal` capabilities, no real terminal).

| Behavior (checkpoint row) | Test (`biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs` unless noted) |
| --- | --- |
| Image mode unchanged | existing `diagram_hook_is_consulted_before_the_code_hook` (now also asserts `graphics_mode: Rich` in the request), `diagram_hook_failure_follows_the_render_strictness`, `diagram_hook_receives_the_forced_terminal_snapshot`, `nested_mermaid_fence_reaches_the_diagram_hook_at_its_narrowed_width`; `render.rs` `mermaid_promotion_failure_rejects_under_strict` |
| Not consulted: Code at Rich/Vector, Image at Off/Vector, Automatic at Off, other/no language | `diagram_hook_is_not_consulted_unless_promotion_is_requested` (8 cases, 4 new) |
| Automatic at Rich and Vector, ceiling carried | `automatic_mode_offers_the_fence_at_rich_and_vector_with_the_ceiling` |
| Automatic at Off → code, no hook call | `automatic_mode_at_graphics_off_renders_the_code_block_without_the_hook` |
| No hook in Automatic → ordinary code, even Strict | `automatic_mode_without_a_diagram_hook_renders_the_code_block` |
| `Declined` silent under Strict/Warn/Lossy (Automatic Rich/Vector, Image Rich) | `declined_renders_the_code_block_silently_under_every_strictness` (9 cells), `declined_fence_still_reaches_the_code_hook` |
| `Err` rejected/recorded/silent in Automatic | `automatic_mode_hook_failure_follows_the_render_strictness` |
| `Nodes` through installed code renderer, no re-entry of the hook | `nodes_outcome_lays_out_its_code_nodes_through_the_code_hook` |
| `Nodes` without a code renderer → plain code blocks | `nodes_outcome_without_a_code_hook_renders_plain_code_blocks` |
| `Nodes` at the fence's indent and width | `nodes_outcome_in_a_list_item_is_laid_out_at_the_items_indent_and_width` |
| Real hook never emits an image at `Vector` | `biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs` `hook_never_draws_an_image_below_the_rich_ceiling` |

The Input Robustness Matrix does not apply (no parser or config reader).

### Gates

- Skip check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [biscuit-terminal, biscuit-visualized, renderable, test-toolkit,
  biscuit-terminal-cli, darkmatter, tree-hugger-cli]`, `change_class: package`.
  Gates run.
- `cargo check --workspace --all-targets`: clean.
- `renderable`: `just lint` clean; `just test` 684 passed.
- `biscuit-terminal`: `just lint` clean (after replacing a `match` with
  `matches!`); `just test` 3890 passed, 55 skipped. Targeted rerun after the
  lint fix: 18/18.
- `biscuit-visualized`: `just lint` clean; `just test` 1071 passed, 4 skipped.
- `just check-tier-coverage` for all three areas: 0 stranded.
- Dependents: darkmatter (installs the hook), filtered to
  `-E 'test(/mermaid|diagram/)'` with `--all-features`: 89 passed. sniff-cli
  and worktree-cli use the components, not the hook; they compile
  (workspace check) and were not run.
- No cross-OS check: no path, `cfg`, or OS-specific code changed.
- No failures, no load-sensitive reruns.

## Phase 3

The text backend in `biscuit-visualized`: Merman's `ascii` feature, eligibility
from Merman's metadata, one bounded text request, and the fit judgment. Policy,
presentation, the component and the hook are Phase 4.

Starting state: Phase 2's changes were still uncommitted in the working tree,
so the skip check below also covers them. Phase 3's own files are the ones
listed in `source_files_during_phase_3`.

### Task 3.1: enable `ascii`

- Added `"ascii"` to the existing `merman` dependency's features (same pinned
  rev, `default-features = false`). `Cargo.lock` did not change:
  `merman-ascii` at the same rev was already locked.
- `biscuit-visualized/docs/dependencies.md` and the root `docs/dependencies.md`
  name the feature and what it is for.

### Task 3.2: eligibility (`src/src/mermaid/text.rs`)

- `text_eligibility(diagram_type) -> Option<TextEligibility>` looks up the row
  in `ascii_capabilities()`; `eligibility_of(row)` decides: `Diagrammatic`
  primary projection only; coverage `Full`/`Partial`, missing → `Partial`,
  unknown value → not eligible; `admits_auto_layout()` from the row's layout
  profiles. No family names appear in the path.
- `MermaidDiagram::text_eligibility()` uses Merman's preprocess + `detect_type`
  (already run by `source_theme`, whose `SourceTheme` now also records
  the metadata id as `diagram_type`), so a `%% mindmap` comment is not read as the header.

### Tasks 3.3 and 3.4: text render and test seams

- `MermaidDiagram::render_text(TextOptions) -> Result<TextRender, MermaidError>`
  (`terminal` feature; see `decisions_for_review`). Ineligible or zero columns
  → `Source` with no request. Otherwise one `RenderRequest::ascii` through the
  same `Renderer` and site config as SVG, after the same `\n` rewrite; plain
  output, standard width profile, `Auto` when the family admits it else
  `Canonical`, `max_width` = the columns with overflow allowed; the diagram's
  time budget applies, and a cancellation logs the same warning as SVG
  (shared `call_merman`). No palette, design, font chain or cache is touched.
- `text::judge` neutralizes control characters
  (`merman::ascii::normalize_terminal_text`; Merman had already escaped them),
  measures the widest row with `biscuit_terminal::utils::block_constraint::visible_width`,
  and returns `Drawing` or `Source(NotDiagrammatic | Empty | TooWide)` with a
  `TextReport` (requested and reported layout, projection, width, overflow).
- Seams (`#[cfg(test)]`, thread-local, reset on read): `take_text_requests()`
  (render.rs), `take_default_chain_requests()` (fonts/chain.rs, counts every
  `default_chain()` call), `take_stored_files()` (cache/file_cache.rs, every
  artifact or sidecar write). Limit injection: crate-internal
  `render_text_with(options, AsciiResourcePolicy)`; cancellation: a zero time
  budget, past its deadline at Merman's first checkpoint. Nothing sleeps.

### Merman behavior observed at `6e6b1e6` (inputs for Phase 4)

- Spec flowchart: 50 cells canonical; `CHAIN_FLOWCHART` (5 boxes): 66
  canonical, 58 compact. State example: 12 cells, canonical only.
- A drawable family returns `StructuredText` only under
  `OverflowPolicy::Fallback`; under `Allow` it reports `overflowed`.
- Unroutable class/ER output: boxes plus a `relations:` list, reported
  `Diagrammatic` (see `decisions_for_review`).
- Mermaid collapses runs of spaces in a label (`"a   b"` draws `a b`); padding
  and trailing spaces in rows are kept.
- A class label with `\n` prints `one<br>two` after the shared rewrite.
- The XY chart example (`bar [1, 5, 9]`) is 260 cells wide: XY charts will
  usually show the source on an ordinary terminal.

### Requirement-to-test mapping

Boundary: biscuit-visualized ↔ Merman's `ascii` target, in process (F2, L1;
`src/src/tests/text_tests.rs`, compiled under `--all-features`). Nested
position and code-block padding are Phase 4 (the backend takes the final
column count).

| Requirement | Test |
| --- | --- |
| R-F2 snapshot of the derived table | `eligibility_table_at_the_pinned_rev` |
| AC-F3 exactly seven eligible, public lookup | `seven_families_are_eligible_at_the_pinned_rev` |
| D1 field rules (control + one edit per cell: no coverage, full, StructuredText, None, no Auto) | `eligibility_is_decided_by_projection_coverage_and_layouts` |
| Missing row / registry id is not a row id | `a_family_without_a_capability_row_is_not_eligible` |
| Parsed header, not a scan; mindmap, pie, gitGraph, garbage, empty | `diagram_eligibility_follows_merman_detection` |
| AC-F14 exact Unicode drawing (spec figure, byte-identical) | `spec_flowchart_draws_in_unicode` |
| AC-F14 ASCII drawing | `spec_flowchart_draws_in_ascii` |
| AC-F14 authored Unicode in both charsets | `authored_unicode_labels_survive_both_charsets` |
| AC-F14 significant spaces | `state_diagram_keeps_its_significant_spaces` |
| AC-F14 multiline labels | `multiline_label_draws_on_separate_rows` |
| AC-F14 label control characters | `label_control_characters_never_reach_the_terminal` |
| Fit boundary (50 fits, 49 does not) | `a_drawing_fits_exactly_its_own_width` |
| Fit in display cells, by this crate's check | `wide_characters_are_measured_in_cells` |
| AC-F11 compact when canonical too wide; one request | `flowchart_too_wide_canonically_draws_compactly` |
| AC-F11 too wide for both → source | `flowchart_too_wide_for_both_layouts_shows_source` |
| AC-F11 state too wide → source, canonical, one request | `state_diagram_too_wide_canonically_shows_source_after_one_request` |
| Zero columns → source, no request | `zero_columns_show_source_without_a_request` |
| AC-F2 dense class at narrow width → source | `dense_class_diagram_at_a_narrow_width_shows_source` |
| AC-F2 real `StructuredText` result → source, projection asserted | `structured_text_result_of_a_diagrammatic_family_shows_source` |
| Empty output → source (control + edit) | `empty_output_shows_source` |
| AC-F1/AC-F14 unsupported family → source, no request | `ineligible_families_show_source_without_a_request` |
| AC-F14 invalid source → error, one request | `invalid_source_is_an_error_after_one_request` |
| AC-F14 cancellation → error, warning, no retry | `exceeded_time_budget_is_an_error_with_a_warning_and_no_retry` |
| AC-F14 resource limit → error, no retry | `reached_resource_limit_is_an_error_and_no_retry` |
| AC-F13 / R-F10 backend half | `text_render_writes_no_artifact_and_asks_for_no_font_chain` |
| Counters are load-bearing (control) | `the_work_counters_count_image_work` |

Doctests on `TextEligibility`, `TextOptions`, `MermaidDiagram::text_eligibility`
and `MermaidDiagram::render_text` pass (`cargo test --doc -- text`).

The Input Robustness Matrix does not apply: no file format or configuration
reader was added (Merman's capability table is compiled data).

### Gates

- `just lint` (biscuit-visualized): clean. `cargo clippy --features image`
  (without `terminal`): clean.
- `just test` (biscuit-visualized): 1097 passed, 4 skipped, 0 failed (12.9 s).
  New: 26 tests, all under 0.2 s.
- `just check-tier-coverage biscuit-visualized`: 0 stranded.
- Skip check (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [biscuit-terminal, biscuit-visualized, renderable, test-toolkit,
  biscuit-terminal-cli, darkmatter, tree-hugger-cli]`, `change_class: package`
  (includes Phase 2's uncommitted files). Gates run.
- Dependents: `cargo check --workspace --all-targets` clean; darkmatter
  `-E 'test(/mermaid|diagram/)'` with `--all-features`: 89 passed.
  biscuit-terminal-cli, sniff-cli and worktree-cli use only the unchanged
  component API; they compile (workspace check).
- No cross-OS check: no path, `cfg(target_os)`, or OS-specific code was added.
- No failures and no load-sensitive reruns.
