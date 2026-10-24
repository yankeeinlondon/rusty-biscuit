---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-22-merman-text-fallback/spec.md
plan: biscuit-visualized/features/2026-10-22-merman-text-fallback/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
decisions_for_review:
    - |-
      Swimlane is eligible for text fallback. The spec listed six eligible families (flowchart, sequence, state, class, ER, XY chart), taken from Merman's support-matrix document. Merman's own metadata (`ascii_capabilities()`) at the delivered revision lists a seventh, swimlane: diagrammatic, partial, and admitting the auto layout. The document folds it into the flowchart row. Because D1 says eligibility comes from the metadata and never from a hand-kept list, swimlane is eligible. I amended D1 and AC-F3 under D10, so the eligibility snapshot will list seven families. Alternative: exclude swimlane explicitly. That needs a family-name exclusion list, which D1 forbids. If the reviewer wants swimlane excluded, Phase 3 gains one exclusion and its snapshot loses one row.
    - |-
      Rulings A–H, which the plan left for the author, were made by the implementer, adopting each recommendation (no human was in the loop). A: the diagram hook returns a three-way outcome (drawn output, render-tree nodes, declined). B: darkmatter's Mermaid mode gains an automatic variant, which `md --mermaid` uses; the library's image mode keeps meaning image-only. C: the three fallback feature names and the "most restrictive wins" fold live in biscuit-visualized behind its `clap` feature; `md` adds them, plus `no-mermaid-rendering`, to the `--feature` enum it already has. D: clap already rejects empty and unknown names with the valid names listed (proved by a throwaway program), so no custom parser is needed. E: automatic mode chooses text at full graphics when the terminal reports no image protocol. F: drawing uses Unicode when the terminal's existing "supports Unicode" flag is set, and ASCII otherwise. G: `bt git-graph` accepts the flags with no visible effect. H: no extra measurement. Reversing A changes Phase 2's trait signature and its test doubles. Reversing B or C changes only Phase 5's CLI wiring.
message_to_agent: |-
    Phase 1 changed no shipped code. Rulings A-H are all adopted as the plan recommended. The "## Phase 1" section of this log has the exact Merman ascii API names, file:line sites and the AC-F15 audit list.

    Facts that change later phases:
    (1) Seven families are eligible, not six. Swimlane is Diagrammatic, Partial, with Auto layout in Merman's metadata. D1 and AC-F3 are amended, so the R-F2 snapshot must list seven.
    (2) Map a source to its capability row like this: registry().detect_type (already done in mermaid/render.rs `source_theme`), then `merman::diagram_type_metadata_id`, then an exact match on `AsciiCapability::diagram_type`. Do not use `diagram_type_family_id`; it returns `gitGraph` where the row says `gitgraph`.
    (3) Requesting Auto layout for a family that does not list it returns `RenderError::Ascii(InvalidOption)`. Requesting an unsupported family returns `UnsupportedDiagram`. Over-width output with the default `OverflowPolicy::Allow` comes back Ok with `overflowed=true` and `outcome=WideAllowed`. So decide eligibility before any request, and read `overflowed` as well as measuring rows.
    (4) There is no exhaustive `match` on `TerminalMermaidMode` anywhere. The only branch is the `== TerminalMermaidMode::Image` test at biscuit-terminal/lib/src/render_tree/render.rs:1742. Adding `Automatic` will compile without touching it, so it needs a test. The three darkmatter matches in entrypoints.rs (296, 604, 723) do break the build, but they match darkmatter's own MermaidMode, which Phase 5 changes.
    (5) `md` already has `--feature` and a `Feature` ValueEnum (EmbedMermaidFont) on both `md <INPUT>` and `md render`. They come from uncommitted 2026-10-21-mermaid-embedded-font work that is still in this worktree. Extend that enum; do not add a second flag.
    (6) Every bt Mermaid subcommand uses the fallible `try_render`. On NoImageSupport it prints a raw fence through `handle_mermaid_error`, and that is the path the text fallback replaces.
    (7) The working tree holds unrelated uncommitted darkmatter, skill and embedded-font spec files. They are not this plan's work, and they make affected_scope report darkmatter/darkmatter-cli/test-toolkit.
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
