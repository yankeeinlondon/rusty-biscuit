---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-merman/spec.md
plan: biscuit-visualized/features/2026-10-21-merman/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/Cargo.toml
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/Cargo.lock
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/src/main.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/FINDINGS.md
    - biscuit-visualized/features/2026-10-21-merman/spikes/s1-comparison/windows-leg.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/build-hosts.md
packages:
    - biscuit-visualized
    - biscuit-terminal
    - darkmatter
    - worktree
    - sniff
source_files_during_phase_2:
    - biscuit-visualized/src/Cargo.toml
    - Cargo.lock
    - biscuit-terminal/lib/Cargo.toml
    - biscuit-visualized/src/src/artifact.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/lib.rs
    - biscuit-visualized/src/src/fonts/mod.rs
    - biscuit-visualized/src/src/fonts/bundled.rs
    - biscuit-visualized/src/src/fonts/chain.rs
    - biscuit-visualized/src/src/fonts/query.rs
    - biscuit-visualized/src/src/fonts/metadata.rs
    - biscuit-visualized/src/src/fonts/matching.rs
    - biscuit-visualized/src/src/fonts/shaper.rs
    - biscuit-visualized/src/src/fonts/measure.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/newline.rs
    - biscuit-visualized/src/src/raster/mod.rs
    - biscuit-visualized/src/src/raster/png.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/bundled_font_tests.rs
    - biscuit-visualized/src/src/tests/font_chain_tests.rs
    - biscuit-visualized/src/src/tests/measure_tests.rs
    - biscuit-visualized/src/src/tests/newline_tests.rs
    - biscuit-visualized/src/src/tests/raster_tests.rs
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_class.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_er.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_flowchart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_mindmap.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_sequence.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_state.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_class.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_flowchart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_mindmap.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_sequence.mmd
    - biscuit-visualized/src/assets/fonts/inter/Inter-Regular.ttf
    - biscuit-visualized/src/assets/fonts/inter/Inter-Bold.ttf
    - biscuit-visualized/src/assets/fonts/noto-emoji/NotoEmoji-Variable.ttf
    - biscuit-terminal/lib/examples/discovery_probe.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/mod.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/parse.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/query.rs
    - biscuit-terminal/lib/src/discovery/startup_exchange.rs
    - biscuit-terminal/lib/tests/common/pty.rs
    - biscuit-terminal/lib/tests/l1/level1_graphics_probe.rs
    - biscuit-terminal/lib/tests/l1/level1_osc4_palette.rs
    - biscuit-terminal/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - docs/dependencies.md
    - biscuit-visualized/docs/dependencies.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-visualized/docs/components/graph_expression.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-terminal/docs/discovery/color.md
    - biscuit-terminal/docs/discovery/terminal-queries.md
docs_created_during_phase_2:
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/pr-01-gitgraph-commit-spacing.md
    - biscuit-visualized/src/assets/fonts/inter/README.md
    - biscuit-visualized/src/assets/fonts/inter/OFL.txt
    - biscuit-visualized/src/assets/fonts/noto-emoji/README.md
    - biscuit-visualized/src/assets/fonts/noto-emoji/OFL.txt
    - biscuit-visualized/src/src/tests/fixtures/merman/README.md
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
    - .claude/skills/biscuit-visualized/rasterization.md
    - .claude/skills/biscuit-terminal/discovery.md
source_files_during_phase_3:
    - Cargo.lock
    - biscuit-visualized/src/Cargo.toml
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/components/git_graph.rs
    - biscuit-visualized/src/src/components/git_graph/tests.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - biscuit-visualized/src/src/fonts/chain.rs
    - biscuit-visualized/src/src/fonts/measure.rs
    - biscuit-visualized/src/src/fonts/mod.rs
    - biscuit-visualized/src/src/fonts/query.rs
    - biscuit-visualized/src/src/mermaid/gitgraph.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-visualized/src/src/tests/font_chain_tests.rs
    - biscuit-visualized/src/src/tests/gitgraph_tests.rs
    - biscuit-visualized/src/src/tests/measure_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_tests.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/newline_tests.rs
    - biscuit-visualized/src/src/mermaid/gitgraph_layout.rs
    - biscuit-visualized/src/src/mermaid/postprocess.rs
    - biscuit-visualized/src/src/tests/cache_identity_tests.rs
    - biscuit-visualized/src/src/tests/fixtures/merman/families/LICENSES/mermaid/LICENSE
    - biscuit-visualized/src/src/tests/fixtures/merman/families/LICENSES/merman-LICENSE-APACHE
    - biscuit-visualized/src/src/tests/fixtures/merman/families/LICENSES/merman-LICENSE-MIT
    - biscuit-visualized/src/src/tests/fixtures/merman/families/agentflow.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/architecture.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/block.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/c4.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/class.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/cynefin.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/er.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/eventmodeling.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/flowchart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/gantt.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/gitgraph.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/info.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/ishikawa.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/journey.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/kanban.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/mindmap.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/packet.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/pie.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/quadrantchart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/radar.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/railroad.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/requirement.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/sankey.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/sequence.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/state.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/swimlane.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/timeline.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/treeView.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/treemap.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/usecase.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/venn.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/wardley.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/families/xychart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/empty_branch_merge.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/labeled_merge.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/layout.json
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/layout.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/long_labels.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/observation_1.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/tag_collision_adjacent.mmd
    - biscuit-visualized/src/src/tests/gitgraph_layout_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_cache_key_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_family_tests.rs
    - biscuit-visualized/src/src/tests/postprocess_tests.rs
docs_updated_during_phase_3:
    - biscuit-visualized/README.md
    - biscuit-visualized/docs/components/git_graph.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-visualized/docs/dependencies.md
    - biscuit-visualized/docs/mermaid-gitgraph.md
    - biscuit-visualized/docs/merman.md
    - docs/dependencies.md
    - biscuit-visualized/src/src/tests/fixtures/merman/README.md
    - biscuit-visualized/features/2026-10-21-merman/spec.md
docs_created_during_phase_3:
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/01-default-measurement-clips-labels.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/02-wrapped-labels-lose-line-breaks.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/03-neo-dark-dividers-invisible.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/04-normalize-backslash-n-like-br.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/05-backslash-n-pie-gitgraph-block.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/06-er-attribute-comment-br-clips.md
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/README.md
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
    - .claude/skills/biscuit-visualized/diagram-components.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
source_files_during_phase_4:
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/mermaid/postprocess.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/fonts/measure.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - biscuit-visualized/src/src/tests/html_static_tests.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/postprocess_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_cache_key_tests.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
    - darkmatter/lib/tests/l1/mermaid_static_svg.rs
    - darkmatter/lib/tests/l1/main.rs
    - sniff/cli/tests/l1/snapshots.rs
docs_updated_during_phase_4:
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - worktree/docs/git-graph.md
    - darkmatter/docs/rendering/mermaid.md
    - biscuit-terminal/cli/README.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-visualized/features/2026-10-21-merman/plan.md
    - biscuit-visualized/features/2026-10-21-merman/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
    - .claude/skills/worktree/git-graph.md
    - .claude/skills/biscuit-terminal/diagrams.md
human_review: true
human_review_items:
    - |-
      **Adding Merman changed how JSON objects are ordered in about 18 other packages, and Darkmatter's tests fail because of it (45 failures)**

      **What happened.** Merman, the new Mermaid renderer, switches on an option in the shared JSON library (`serde_json`'s `preserve_order`). Rust build features are shared across one build, so every program that includes biscuit-visualized now has it too: Darkmatter and `md`, sniff, worktree, claudine, messenger, research, dmls, and others. JSON objects used to be sorted alphabetically by key; now they keep the order keys were inserted.

      **Why it matters now.** Darkmatter promises sorted ("canonical") key order in places. With the option on, its frontmatter content hashes change (`md hash` results and anything cached by them), `md schema validate --json` output order changes, `as_json5` output changes, and 40 snapshots break. Phase 4's own gate (Darkmatter's `just test`) cannot pass until this is decided, and every later phase adds more dependents on top of it. Merman itself needs the option (its diagram models rely on insertion order), so it cannot simply be switched off.

      **Options**

      - **A. Make sorting explicit in the consumers, and keep Merman as is.** Darkmatter (and any other package found to depend on sorted keys) sorts keys itself where order is part of its contract: hashing, canonical JSON output, schema conversion. Snapshot tests that only compare structure use order-independent comparison (already done for sniff).
          - Pro: removes a hidden dependency on a library default that any future dependency could flip again, and leaves Merman's fork untouched.
          - Con: real work in Darkmatter (finding every place that relied on sorting), plus running the other dependents' suites to find their sensitive spots.
      - **B. Patch the Merman fork so it does not need the option** (replace its use of JSON maps with an order-keeping map of its own).
          - Pro: nothing else in the repo changes behavior.
          - Con: a deep change across Merman's internals that must be carried through every future upstream update; high risk of subtle diagram differences.
      - **C. Turn the option on deliberately for the whole repo and accept insertion order everywhere,** then update hashes, baselines, and snapshots.
          - Pro: one consistent behavior, with the least code.
          - Con: changes Darkmatter's documented canonical-order contract and its content hashes for everyone; the dependence on a library default stays hidden.

      **Recommendation: A.** The real defect is that Darkmatter's hashes and canonical output silently depended on a shared library default. Making the sort explicit fixes that whatever any dependency does, keeps the Merman fork small, and does not change outputs users already rely on. Concretely: a short Darkmatter task to canonicalize key order where it hashes or emits canonical JSON, then running the remaining dependents' suites (claudine, messenger, research, dmls, playa, biscuit-icon, biscuit-speaks).
message_to_agent: |-
  Phase 4's six tasks are implemented, but checkpoint 4 is NOT fully green.
  Read the log's "## Phase 4" section, especially "Blocking finding".

  - Blocking: Merman (merman-core) enables serde_json `preserve_order`, so
    every build that reaches biscuit-visualized gets insertion-ordered JSON
    maps. Darkmatter has 45 L1 failures from it (frontmatter hashes, `md
    schema validate --json` order, `as_json5`, 40 snapshots). A human ruling
    is pending (recommended: Darkmatter sorts keys explicitly where order is
    a contract). Do not "fix" it by accepting snapshots or changing hashes
    without that ruling. sniff's JSON snapshots now compare with insta
    `sort_maps`; the four sniff failures earlier phases called pre-existing
    were this regression.
  - New API: `mermaid::MermaidTarget { RasterSafe, HtmlStatic }`,
    `MermaidDiagram::{with_target, target}` (library and component). PNG
    ignores the target. The HTML-static cache key adds `"target":
    "html-static"`; default keys are unchanged (no `+bv` bump). If Phase 5
    changes the post-process, keep `postprocess::html_static` running after
    the palette edits and bump `+bv<n>` for output changes.
  - Darkmatter's static-SVG path selects `HtmlStatic`; the sanitizer admits
    `<filter>` + `<feDropShadow>` under D64 and drops an empty filter. If
    Phase 5's palettes add other filter primitives or named colors to
    shadows, the sanitizer will drop them (it accepts hex, rgb(), and hsl()
    only).
  - worktree: the D43 rewrite list is in the log. In a real 40-row terminal
    a three-lane view now loses a lane to the half-height cap.
  - biscuit-terminal `test-l2` has one failure unrelated to diagrams
    (`level2_tmux_draws_sixel_itself_at_the_bottom_of_the_viewport`, tmux 3.8
    row placement).
---

# Implementation Log for 2026-10-21-merman (7 phases)

## Phase 1

Phase 1 is rulings and spikes only. No package source changed; the only new
files are `spikes/s3-gitgraph-geometry/` and the R-S9 Windows record under
`spikes/s1-comparison/`.

### Rulings (Wave 2; ruled by Ken 2026-10-22)

Phase 1 ran non-interactively and first recorded the plan's defaults. Ken
ruled every item on 2026-10-22; the spec records each as a decision. The
"Ruling" column is Ken's; the last column shows what Phase 1 had recorded,
only where it differed.

| # | Topic | Ruling | Status | Spec | Phase 1 had assumed (where different) |
| - | ----- | ------ | ------ | ---- | ------------------------------------- |
| 1 | Q5 cache key / backend id | Backend id `merman@<12-char rev>+bv1`. Key: source as rewritten by step 3b, output format, hash of the compiled per-family design JSON, once-per-process content hash of every face file the chain can return (bundled faces included), and the identity (content hash + face index) of the face actually resolved for each requested family and fallback | ruled by Ken 2026-10-22 | D41, D63 | Same, without the resolved-face identity |
| 2 | Q8 CI tiers | SVG goldens, PNG goldens (AE ≤ 0.1%, RMSE ≤ 0.005), and OSC 4 parser tests at L1 on every CI environment; this work promotes `biscuit-visualized` to gating (`gates = true`, exclusion fields removed); fixtures in `source-inputs` only if read through a path CI cannot see | ruled by Ken 2026-10-22 | D42 | PNG comparisons on Linux/macOS only; `gates = false` promotion left to its own change |
| 3 | Q11 Unicode/ASCII text fallback | Out of scope; follow-up `2026-10-22-merman-text-fallback` | ruled by Ken 2026-10-22 | D48 | — |
| 4 | Q16 benchmark | Release profile; reference host is the Linux build host; baseline JSON beside the benchmark; fresh `BISCUIT_VISUALIZED_CACHE_DIR` per iteration outside the timed region | ruled by Ken 2026-10-22 | D44 | Ken's macOS host as the reference |
| 5 | Q18 contrast | Every text element meets 4.5:1; no large-text exception | ruled by Ken 2026-10-22 | D45 | Large text (≥ 24 px, or ≥ 18.66 px bold, in SVG user units) at 3:1 |
| 6 | Q19 OSC 4 on native Windows | Not carried: the non-Unix OSC query arm is a stub, so native Windows uses background-derived accents, documented | ruled by Ken 2026-10-22 | D46 | — (Phase 1 reached the same outcome from the evidence) |
| 7 | Q20 derived-palette goldens | Six SVG goldens: flowchart, sequence, gitGraph × {full ANSI set, background only} | ruled by Ken 2026-10-22 | D47 | — |
| 8 | Q21 fixtures location and cap | `biscuit-visualized/src/tests/fixtures/merman/`; ~20 MB cap separately for `spikes/` and fixtures; bundled fonts are library source and do not count | ruled by Ken 2026-10-22 | D49 | — |
| 9 | Q22 golden font | Bundled Inter (regular, bold, OFL 1.1) in `biscuit-visualized/src/assets/fonts/inter/`, compiled in and registered as `Inter`; goldens set `MERMAID_FONT=Inter` | ruled by Ken 2026-10-22 | D40 | A committed test-only font directory put ahead of D32 step 1 by an internal test hook |
| 10 | D32 step 4 rule (R-M8) | Among installed OS/2 sans-serif faces: regular and bold present, then widest Latin coverage, then proportional over monospace, then alphabetically first family name | ruled by Ken 2026-10-22 | D50 | Regular, non-italic, non-condensed; ties by case-folded family name, then file path |
| 11 | Fallback-face order | Resolved face, then installed faces by case-folded family name, then (Linux builds only) the bundled Noto Emoji, last | ruled by Ken 2026-10-22 | D59, D65 | Same, without the Linux-only Noto Emoji |
| 12 | Tag spacing vs cache check | Pre-parse step 3b, before the cache check, so the key covers the rewritten source | ruled by Ken 2026-10-22 | D55 | Spacing computed after the cache check |
| 13 | Darkmatter measured face per label | Additive "HTML static" target on `mermaid::MermaidDiagram`; `biscuit-visualized` rewrites `font-family` and allows overflow; Darkmatter selects and sanitizes. Sanitizer admits `<filter>` and `<feDropShadow>` only (number-or-color attributes besides `id`; `dx`, `dy`, `stdDeviation` ≤ 20; `flood-opacity` 0–1); other `fe*` dropped; local-only `url(#…)` unchanged | ruled by Ken 2026-10-22 | D60, D64 | Same target design; `filter`/`fe*` admission left to the sanitizer change |
| 14 | Merman feature set | `default-features = false`, `["svg", "all-diagrams", "layout-cytoscape"]`; `png`, `native-fonts`, ELK, `math` off | ruled by Ken 2026-10-22 | D58 | — (same set; `math` off now ruled) |
| 15 | Interim themes at cutover | The four themes map to Merman built-in themes until the palette phase replaces them | ruled by Ken 2026-10-22 | D61 | — |
| 16 | `tag_gap` derivation | Clearance between adjacent tags produced by the `commitSpacing` written in step 3b, from measured tag widths and layout `pos` | ruled by Ken 2026-10-22 | D56, AC-C12 | Clearance implied by "wider spacing via gitGraph config", blocked on ruling 17 |
| 17 | D22 decision point | Closed. Fixed rule: topology assertions pass unchanged; renderer-mechanics assertions may be rewritten, each logged with category and reason. `commitSpacing` on Ken's Merman fork keeps `tag_overlaps()` unchanged | ruled by Ken 2026-10-22 | D43, D56 | Open, left for Ken; AC-C4 ("tests unchanged") standing meanwhile |
| 18 | Circles `unsupported-paint` | Allow-listed per family with a comment naming it; general assertion not weakened | ruled by Ken 2026-10-22 | D62 | — |
| 19 | Prerequisite state | `2026-10-13-graphics-where-possible` has **not** delivered native Windows detection: its log shows Phase 1 of 9 done, and `biscuit-terminal/lib/src/discovery/osc_queries/query.rs` answers every non-Unix query with `OscQueryError::Unsupported` (the `cfg(not(unix))` arm at line 321). Implementation proceeds; release waits on it | recorded fact; ruled by Ken 2026-10-22 | D7, D46 | — |
| 20 | Upstream drafts | Six issue drafts (`upstream-issues/01-…md` to `06-…md`; 6 = ER `<br/>` comment clip) and the `commitSpacing` pull request draft; Ken files and opens them during the cutover; agents never post | ruled by Ken 2026-10-22 | D52, D56, D66 | Four issue drafts, filed on Ken's go-ahead |
| 21 | Emoji fallback font | Noto Emoji (monochrome, regular, TrueType, OFL 1.1), Linux builds only, beside Inter, last fallback; Linux glyph test; goldens exclude emoji | ruled by Ken 2026-10-22 | D65 | — (not covered in Phase 1) |
| 22 | Benchmark placement | Non-member crate at `biscuit-visualized/bench/` (empty `[workspace]`), no CI cell; `just bench` runs it directly instead of delegating to `_bench` (item 11's exception); baseline JSON there | ruled by Ken 2026-10-22 | D67, R-V6, AC-V2 | — (the plan left the placement and the recipe open) |
| 23 | Measurer crates | `rustybuzz` and `ttf-parser` as direct dependencies of `biscuit-visualized`, pinned to the versions usvg 0.47 uses (one copy each); pre-authorized in item 1 | ruled by Ken 2026-10-22 | D68, R-D5 | — (the plan added them without pre-authorization) |
| 24 | Clean-room check | At final verification a reviewer confirms no Zed copyright or GPL header and no Zed-specific identifiers (`zed-accent`, `mermaid_render`, `inject_css`, `accent_colors`, at least) in the changed code; logged | ruled by Ken 2026-10-22 | D69, R-T10, AC-T9 | — (R-T10 had no verifying AC) |

Ruling 14 (D58), `png` check: `cargo tree` of the S3 spike, built with exactly
the ruling-14 feature set, contains no `resvg`, `usvg`, `tiny-skia`, `fontdb`,
`png`, or ELK crate (the only name match is `palette_math`, a color crate), so
no umbrella feature in that set pulls `png` or a second usvg.

### R-S8 / AC-S6: gitGraph geometry spike

New standalone project `spikes/s3-gitgraph-geometry/` (empty `[workspace]`,
`Cargo.lock` seeded from the workspace lock, then `serde_json` bumped to
1.0.154 because Merman requires `^1.0.154`; Merman at
`482fcde7faffb05a466f198085d615e604075d97`). Full write-up:
`spikes/s3-gitgraph-geometry/FINDINGS.md`; raw rows:
`spikes/s3-gitgraph-geometry/out/results.json`.

**Which copy ran.** Neither route in the spec could run: Cargo `[patch]`
cannot replace `worktree-cli`'s **path** dependency on `biscuit-visualized`,
and `git_graph/tests.rs` is a private unit-test module of the `wt` binary
that cannot compile outside it. Instead the real tests were run once with
`geometry_of` temporarily instrumented to dump its Mermaid input and today's
geometry (`S3_CAPTURE_DIR`), 76/76 passed, and the instrumentation was
reverted with `git checkout` (verified: `git status` shows no change outside
the feature directory). The 31 captured calls from 14 tests are in
`spikes/s3-gitgraph-geometry/inputs/`, and the spike replays them through
Merman, checking every property a `worktree` geometry assertion reads.

**Per-assertion result** (S = ids/order/lanes/parents/tag texts; O =
`tag_overlaps()` empty; P = the line-2193 `commit_step` check):

| Assertion group | Inputs | Result on Merman geometry |
| --------------- | ------ | ------------------------- |
| S, all 14 tests | 31 | **pass** (both parents of every merge kept, no repair; `+N` square ids kept exactly) |
| O, 10 tests | 27 | **pass** except `gathered_graph_lays_out_long_labels` (both viewports): `fix/very-long-exact-branch-reference-beta` overlaps `main` and `origin/main` at Merman's fixed step |
| P, line 2193 | 1 | **pass**: derived step 50 = Merman default 50 (that graph has no colliding tags) |

**commit_step.** Derivable: consecutive `pos` differ by exactly 50 in every
input (`COMMIT_STEP` 40 + `LAYOUT_OFFSET` 10).

**Widened commit spacing (D36).** The required diagram-wide step is 125.9 for
`long_labels` and 64.2 for `observation_1`/`observation_2`; every other
input keeps 50. **But Merman cannot be given a step**: it is a compile-time
constant in `merman-render/src/gitgraph.rs`, and the only gitGraph config keys
Merman reads are `rotateCommitLabel`, `showCommitLabel`, `showBranches`,
`diagramPadding`, `parallelCommits`, `titleTopMargin` (Mermaid.js has no such
key either). D36's "wider commit spacing in the generated Mermaid (gitGraph
config)" is not implementable at this rev. If spacing were applied by some
other route, line 2193 still holds (no widening for that graph) and O passes
for `long_labels`.

**Natural size.** Merman's gitGraph width is a median 1.15× today's (range
0.52–1.17×). `worktree`'s planner trims to the viewport using natural size, so
its generated Mermaid will change at cutover; the replay used today's plans,
so that effect is not covered here and is first visible in Phase 3.

**Go/no-go for "worktree tests unchanged": no-go as specified.** One
assertion (`long_labels`' no-overlap) cannot hold without a spacing mechanism
Merman lacks. Raised for Ken (D22 / D36); Ken closed it on 2026-10-22 with
D43 (fixed rewrite rule) and D56 (`commitSpacing` on his Merman fork).

### R-S9 / AC-S8: Windows PNG leg

Run by a subagent; full record in `spikes/s1-comparison/windows-leg.md`. One
diagram (`upstream_cypress_sequencediagram_spec_example_001`, Merman column,
S1 settings: resvg 0.45, 2× natural width capped at 1000 px), built from the
same tree on all three hosts in this run (Linux re-rendered fresh, not taken
from the gallery). Metric: ImageMagick `compare` AE share (fuzz 0) and
normalized RMSE, same-size pairs only (`scripts/cross_os_diff.py`).

| Pair | Theme | AE share | RMSE |
| ---- | ----- | -------- | ---- |
| macOS–Windows | dark | 0.000% | 0.0000 |
| Linux–Windows | dark | 0.268% | 0.0365 |
| macOS–Linux | dark | 0.268% | 0.0365 |
| macOS–Windows | light | 0.000% | 0.0000 |
| Linux–Windows | light | 0.735% | 0.0661 |
| macOS–Linux | light | 0.735% | 0.0661 |

- All three hosts produce 1000×1298 and a byte-identical SVG; every difference
  is rasterization.
- macOS and Windows both draw the labels in Arial and match pixel for pixel.
  The whole Linux difference is font substitution (DejaVu Sans; no Arial).
- With real-font measurement (`merman-real`), macOS–Windows differ by at most
  11 px (RMSE 0.0008) from different Arial files.
- **Input to D13:** with the pinned golden font of ruling 9, a tolerance of
  AE share ≤ 0.1% and RMSE ≤ 0.005 leaves a wide margin over the worst
  same-font difference seen. Without a pinned font, the gap is a different
  typeface (0.74% here, up to 3.2% across S1's corpus), and a tolerance that
  loose would hide regressions. This supports ruling 9 (pin the font) and
  ruling 2 (PNG comparisons on Linux/macOS only). One diagram is a thin
  basis; Phase 6 should confirm on the golden corpus. *(Ken's rulings of
  2026-10-22: the bundled Inter, D40, and PNG goldens on every CI
  environment, D42.)*
- Two Windows traps found were added to the `os` skill
  (`.claude/skills/os/build-hosts.md`, `BUILD_WIN` row).

### Validation checkpoint 1

- Every ruling had a value; ruling 17 was left for Ken. All were ruled by
  Ken on 2026-10-22 (see Rulings); ruling 17 is closed by D43 and D56.
- AC-S6 recorded (R-S8 above) with a go/no-go: **no-go** for "worktree tests
  unchanged" as specified. AC-S8 recorded (R-S9 above).
- `git status` (below) shows changes only under `spikes/`, this log, the plan,
  and one `os` skill line; no package source changed. The `os` skill edit is
  a skill update required by the repository's rule to record new OS facts in
  the same change, not package code.

### Requirement-to-test mapping

Phase 1 adds no package behavior, so no package test was added or changed.
The spike evidence is the check: S3's replay of all 31 captured
`worktree` geometry inputs (`out/results.json`) and the R-S9 comparison.
`worktree`'s own `git_graph::tests` (76) passed on the unchanged tree during
capture.

### Gates

- Scope check: `git status --porcelain --untracked-files=all | awk '{print
  $NF}' | xargs python3 scripts/ci/affected_scope.py` gave `packages:
  ["darkmatter"]`, `change_class: package`, reason "changed test input read
  by package(s) darkmatter": darkmatter L1 tests walk `.claude/skills/`, which
  the `os` skill edit touched. Gates were therefore **not** skipped.
- `just test` (biscuit-visualized): 294 passed, 3 skipped.
- `just lint` (biscuit-visualized): clean.
- Narrowed darkmatter run for the skill-tree readers: `cargo nextest run -p
  darkmatter --test l1 -E
  'test(/shipped_schema_corpus::|current_root_migration_guard::/)'`: 9 passed.
  `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` took
  5.5 s and was flagged slow; it reads the whole shipped corpus and is not
  affected by a one-line skill edit (pre-existing, recorded only).
- `worktree-cli` `git_graph::tests`: 76 passed during capture, on the
  unchanged tree.
- No cross-OS check: no package code changed. The Windows host was exercised
  for R-S9 itself.

## Phase 2

Phase 2 built everything below the Merman render path: the `commitSpacing`
patch on Ken's Merman fork, the resvg/usvg 0.47 upgrade, the Merman git
dependency, the bundled fonts, one font-resolution chain shared by the
rasterizer and a rustybuzz measurer, the interim `\n` lexer, and the internal
OSC 4 query. Mermaid diagrams still render through `mermaid-rs-renderer`; the
rasterizer (graphs and today's Mermaid output alike) now draws through the
chain.

Work was split across five sub-agents (fork patch; usvg/fonts; OSC 4 in an
isolated worktree; font chain + `FontResolver`; measurer + oracle tests); the
`\n` lexer, the Merman dependency, the planned docs pages, and the gates were
done by the orchestrator.

### Stream C: Merman fork patch (D56, AC-C2a)

- Ken had **no GitHub fork** of `Latias94/merman` (`gh repo view
  yankeeinlondon/merman` failed; not among the upstream's forks). Item 8
  pre-authorizes pushing to "Ken's GitHub fork", so the fork was created as
  the minimal precondition: `gh repo fork Latias94/merman --clone=false
  --default-branch-only` → https://github.com/yankeeinlondon/merman.
- Local clone `/Users/ken/coding/forks/merman`: remote `ken` =
  `git@github.com:yankeeinlondon/merman.git` (the https push failed because
  the clone's credential helper points at a missing `/usr/bin/gh`; SSH with
  `BatchMode=yes` worked). Branch `gitgraph-commit-spacing` from
  `482fcde7faffb05a466f198085d615e604075d97`; commit
  **`6e6b1e6554e8e75d468c1f991652c1c53a85bace`**, signed (key
  `06385819D2659A03`, `git verify-commit` good), no attribution trailers.
  `git ls-remote ken gitgraph-commit-spacing` → that SHA. Nothing pushed to
  `origin`; no PR or issue opened.
- Patch (3 files, +195/−7):
    - `merman-render/src/gitgraph.rs`: `gitGraph.commitSpacing` (px, default
      50) replaces `COMMIT_STEP + LAYOUT_OFFSET` as the step; read like
      `diagramPadding` (numbers and numeric strings); zero, negative,
      non-numeric, `null`, and boolean values fall back to 50. Under
      `parallelCommits`, steps from a parent are `commitSpacing −
      LAYOUT_OFFSET`. Other `LAYOUT_OFFSET` uses (first-commit offset, BT axis)
      unchanged.
    - **Departure from the plan's description:** `merman-core/src/preprocess/mod.rs`
      gained a one-entry `MERMAN_EXTENSION_CONFIG_KEYS = ["commitSpacing"]`
      allow-list, because Merman (like Mermaid 11.16) drops `%%{init}%%` keys
      that Mermaid's default config does not know, which would have silently
      removed `commitSpacing` from directives.
    - Tests: 4 unit (default 50 and 120 across LR/TB/BT with and without
      `parallelCommits`; first-commit offset; numeric strings and invalid
      values), 1 SVG test (commit `cx` via default, site config, init
      directive, frontmatter), 1 directive-filter test.
- Fork test runs: `merman-render --lib gitgraph` 55 passed;
  `gitgraph_svg_test` 66 passed; `layout_snapshots_test` 3 passed (every
  layout golden, gitGraph included, unchanged at the default);
  `merman-core --lib` 499 passed; clippy clean on the touched targets.
- PR draft: `upstream-issues/pr-01-gitgraph-commit-spacing.md`.

### Stream A: dependencies and bundled fonts

- **resvg/usvg 0.47 (R-M3, AC-M5).** Direct users were `biscuit-terminal/lib`
  and `biscuit-visualized/src` only. Ken committed the first bump mid-phase as
  `81a1838bd` ("chore(deps): update SVG rendering dependencies": resvg 0.47 in
  both, png 0.18); everything else is uncommitted. resvg/usvg 0.47 needed no
  source change; png 0.18 renamed `Compression::Default` → `Balanced` (same
  zlib level 6) and `set_adaptive_filter` → `set_filter(Filter::Adaptive)`.
  `PNG_ENCODING` is now `png@0.18:compression=balanced,filter=adaptive` and
  `RASTERIZER` `resvg@0.47` (the PNG cache key changes, so no old PNG is
  served). tiny-skia 0.12's own encoder now equals ours byte for byte, so
  `raster_tests::the_encoding_is_lossless_and_no_larger_than_tiny_skias` now
  asserts `<=` (was `<`; failed at 11553 = 11553).
- **Versions:** resvg/usvg 0.47.0, fontdb 0.23.0, tiny-skia 0.12.0, rustybuzz
  0.20.1, ttf-parser 0.25.1, fontique 0.12.0, unicode-bidi 0.3.18,
  unicode-script 0.5.8, serde_json 1.0.154. `cargo tree -d --workspace
  --target all` shows one copy each of usvg, resvg, fontdb, tiny-skia,
  rustybuzz, ttf-parser, fontique (png 0.17 survives only under `ico` and
  `tauri-codegen`).
- **fontique** with `["system", "fontconfig-dlopen"]` (orchestrator's
  change): without `fontconfig-dlopen` fontique links libfontconfig at build
  time, so every Linux build would need the fontconfig dev package; with it
  the library is loaded at run time, and a host without it gets an empty
  fontconfig backend (`LIB_RESULT.is_err()` → default), so step 3 is skipped
  and step 4 still resolves.
- **Merman dependency (R-C2, D58, D10):** `merman = { git =
  "https://github.com/yankeeinlondon/merman.git", rev =
  "6e6b1e6554e8e75d468c1f991652c1c53a85bace", default-features = false,
  features = ["svg", "all-diagrams", "layout-cytoscape"], optional = true }`
  under `image`; `mermaid-rs-renderer` stays until Phase 3. Resolving it
  required raising locked minimums to Merman's requirements: thiserror
  2.0.18 → 2.0.21, serde 1.0.228 → 1.0.229, rustc-hash 2.1.2 → 2.1.3,
  unicode-segmentation 1.13.2 → 1.13.3, indexmap 2.14.0 → 2.14.2+,
  data-encoding 2.10.0 → 2.11.1, pulldown-cmark 0.13.3 → 0.13.5 (each by
  `cargo update -p <crate>@<locked>` with Merman temporarily removed, one
  crate per resolver conflict). `cargo tree -p merman` has no png, usvg,
  resvg, tiny-skia, fontdb, or ELK crate; `cargo tree -i usvg` shows only
  0.47 via our resvg.
- **Inter (D40, R-M12, AC-M9):** Inter v4.1
  (`https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip`,
  `extras/ttf/`): `Inter-Regular.ttf` 411,640 bytes, `Inter-Bold.ttf`
  420,428 bytes, `OFL.txt` 4,380 bytes; SHA-256s in
  `src/assets/fonts/inter/README.md`. **`OFL.txt` declares no Reserved Font
  Name**: its copyright line is only "Copyright (c) 2016 The Inter Project
  Authors (https://github.com/rsms/inter)" (input to
  `2026-10-21-mermaid-embedded-font` D8). Inter's OS/2 `sFamilyClass` is 0,
  not 8.
- **Noto Emoji (D65, R-M14):** no static regular TTF is published
  (googlefonts/noto-emoji has none; googlefonts/emoji-bw 404s), so the
  **variable** font (`wght` 300–700, default instance Regular 400) from
  google/fonts `ofl/notoemoji/NotoEmoji[wght].ttf` at commit
  `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5` is bundled as
  `NotoEmoji-Variable.ttf`, **exactly 1,982,596 bytes** (above the spec's
  1–1.5 MB estimate). Compiled in only under `#[cfg(target_os = "linux")]`
  (`fonts/bundled.rs`, constant defined inside that cfg).

### Stream B: OSC 4 (D19, D46, R-P1, R-P2, R-P7)

- Done in an isolated worktree; its base `f657c9148` is an ancestor of this
  branch and every touched file was identical between the two, so the files
  were copied back verbatim.
- Entry point `#[doc(hidden)] pub fn
  biscuit_terminal::discovery::osc_queries::ansi_palette() -> [Option<RgbValue>; 16]`
  (the one cross-crate hook for Phase 5's palette); everything behind it is
  `pub(crate)`. The sixteen `ESC]4;N;?BEL` queries ride the existing
  once-per-process OSC 10/11 startup exchange (after OSC 11, before the
  Kitty query and the DA1 sentinel), share its `DIRECT` cache, timeout
  (`DEFAULT_TIMEOUT`, 1 s) and silence budget, and use the color gate
  unchanged (no TTY, `CI`, `TMUX`/`ZELLIJ`/`STY`). Every outcome is cached,
  including all-unknown. Only `?` queries are sent.
- Parser `parse_osc4_palette`: `rgb:` with 1–4 hex digits per component
  (scaled from its own width), `#rrggbb`, BEL or ST; any order; mixed with
  other replies; truncated, malformed, or out-of-range replies leave the slot
  `None`; the first usable reply per index wins.
- **D46, restated (AC-P6), with a finding.** Native Windows sends no OSC 4:
  `ASKS_PALETTE = cfg!(unix)`, so its palette is all-unknown and Phase 5's
  diagrams there use background-derived accents; the docs say so. **But D46's
  recorded premise is stale on this branch:** the OSC query path is no longer
  a stub on non-Unix. `biscuit-terminal/lib/src/discovery/tty_query/windows.rs`
  exists and already answers OSC 10/11 in the startup exchange, so carrying
  OSC 4 there would need no Windows-specific code, which is D46's own
  condition for running it. The conservative outcome (not carried) was kept
  because it is unproven on Windows; turning it on is a one-line change plus a
  Windows proof. Ken ruled D71 (below): Phase 5 turns it on.
- Gate note: the color gate's `is_tty()` is stdout **or** stderr, not stdout
  alone as R-P7's wording says; unchanged.
- Unrelated pre-existing defect seen: `biscuit-terminal`'s lib test target
  does not compile with `--all-features` (28 errors in
  `components/horizontal_rule/mod.rs`, a `horizontal_rule_svg` argument-count
  change); `just test` uses `--features image` and is unaffected. Also the
  OSC 10/11 parser reads `rgb:ff/ff/ff` as 16-bit (≈1); a test asserts it;
  untouched.

### Wave 4–5: font chain, rasterizer, measurer

- **Chain** (`biscuit-visualized/src/src/fonts/{chain,query,metadata,matching}.rs`,
  `pub(crate)`, `image` feature): `FontChain::resolve`/`resolve_with_step`
  (D32 steps 1–4), `fallback(c, already_tried)` (usvg's contract:
  `already_tried[0]` is the resolved face), `fallback_order`, `database()`
  (the same `Arc<fontdb::Database>` usvg gets), `default_chain()` (system
  fonts loaded and `MERMAID_FONT` read once per process; fontique and the
  step-4 scan only when first needed). D63 hooks for Phase 3:
  `face_identity`, `query_identities`, `font_set_hash` (not yet in the key).
- Decisions inside the spec's room:
    - D50 Latin blocks: U+0000–007F, 0080–00FF, 0100–017F, 0180–024F,
      1E00–1EFF, 2C60–2C7F, A720–A7FF, AB30–AB6F, 10780–107BF, 1DF00–1DFFF;
      "regular" = weight 400 upright, "bold" = 700 upright; a variable font
      counts its default instance only.
    - Fallback order: installed faces by case-folded first family name;
      within a family, the face CSS matching picks for the resolved face's
      weight/style/stretch first, then PostScript name, face index, file
      path; then (Linux only) Noto Emoji. Bundled Inter is not in the
      fallback list. On macOS, hidden families (`.Apple Color Emoji UI`)
      sort first because of the leading `.`; documented.
    - Face matching within a family ports fontdb 0.23's private
      `find_best_match`, because fontdb's `query` matches names
      case-sensitively and cannot exclude the bundled faces.
    - S2's "mono/courier → monospace" name heuristic was dropped (a catalog of
      names, which D32 forbids); step 3 is the OS sans-serif answer, since a
      stack generic was already answered at step 2. A stack asking only for
      `courier` therefore gets sans-serif when Courier is not installed
      (`docs/merman.md` says so).
    - The `MERMAID_FONT` warning fires when a chain is built, behind a
      process-wide `AtomicBool`.
    - **Behavior change:** `MERMAID_FONT` now applies to every rasterized
      SVG, graphs and `GraphExpression::with_font_family()` included.
    - `available_font_families()` lists installed families plus `Inter`,
      not Noto Emoji.
- **Rasterizer (R-M4):** `raster::usvg_options(&Arc<FontChain>)` sets the
  chain's fontdb and a `usvg::FontResolver` calling `resolve`/`fallback`;
  `rasterize_svg_to_png_bytes_with` takes an explicit chain (tests).
- **Measurer (D31, R-M1, R-M2):** `fonts/shaper.rs` (usvg 0.47 text mirror)
  and `fonts/measure.rs` (`LabelMeasurer`; `text_measurer(chain) -> Arc<dyn
  merman::svg::TextMeasurer + Send + Sync>` through
  `DeterministicTextMeasurer::with_width_callback`; `width`, `faces`,
  `shape`). Strings are shaped exactly as given (unlike S2, `\n`/`\r` are
  not stripped). Three spike-vs-usvg mismatches fixed: usvg turns weight,
  stretch, italic, oblique into `wght`/`wdth`/`ital`/`slnt` variations;
  usvg empties a letter-spaced cluster when its advance is not > 0 (spike
  used ≥); usvg skips a face its `load_font` rejects. **No difference from
  usvg found: 0 px worst case on every compared label.**
- **Fixtures (D49):** `src/src/tests/fixtures/merman/{wrap,mixed}/` (10
  `.mmd`, 3,991 bytes total), written for the S2 spike; README says no
  third-party license applies.

### Facts the next phase needs

- **Merman's default font family.** Without a site config/theme, Merman draws
  in `"Recursive Variable"` while asking the measurer about `"trebuchet ms",
  …`; on macOS that clipped class labels by up to 14.6 px. With an explicit
  theme the two agree. **The Phase 3 render path must always pass a theme.**
- Merman sometimes passes `font_family` with a trailing CSS `;`; the measurer
  strips it. For some strings Merman also measures with plain `sans-serif`
  (Helvetica on macOS while drawn in Trebuchet); harmless (the wrap test shows
  no clip) but the face test accepts a match on any style Merman asked with.
- `font-kerning="none"` on `<text>` did not switch kerning off in usvg's
  output; `kerning="0"` does. Not investigated.
- **Font-set hash cost (D41/D63 vs D35).** On this Mac the once-per-process
  content hash of every file the chain can return covers 2,986 files, 3,419
  faces, 1,713,371,560 bytes: **571 ms and 950 ms** on two runs with the
  files already in the OS cache (cold disk will be slower). System font load
  ≈170–180 ms; first fontique answer ≈235–285 ms. Wired as specified, this
  alone exceeds D35's 500 ms cold-render rule on macOS. Raised for Ken;
  ruled by D70 (below).

### Departures from the plan/spec

- Fork creation on Ken's account (not in the plan; precondition of item 8).
- The Merman `%%{init}%%` allow-list for `commitSpacing` (above).
- Noto Emoji is the variable font, 1,982,596 bytes (no static regular face
  is published).
- fontique uses `fontconfig-dlopen`.
- D46's premise is stale (above); outcome kept as recorded, then reversed by
  D71 (below).

### Requirement-to-test mapping

| Requirement / AC | Tests (all L1, `biscuit-visualized` unless noted) |
| --- | --- |
| AC-C2a (fork) | Fork: 4 gitgraph unit tests, 1 SVG test, 1 directive-filter test; `git ls-remote ken`; PR draft file |
| R-M3/AC-M5 (one usvg 0.47) | `cargo tree -d` / `cargo tree -i usvg` (above); `raster_tests::*` |
| D40/AC-M9 (Inter) | `bundled_font_tests::inter_regular_is_upright_400_in_family_inter`, `inter_bold_is_upright_700_in_family_inter`; `font_chain_tests::mermaid_font_inter_on_an_empty_host_resolves_regular_and_bold` |
| D65/R-M14/AC-M11 (Noto Emoji) | `bundled_font_tests` Linux parse test; `font_chain_tests::bundled_noto_emoji_is_last_and_never_resolved` (Linux), `no_noto_emoji_face_is_registered_off_linux`; `measure_tests::linux_emoji_without_an_installed_emoji_face_draws_bundled_noto_emoji`, `linux_installed_emoji_face_wins_over_bundled_noto_emoji` (Linux) |
| AC-M3 (chain order, warning) | `font_chain_tests::mermaid_font_names_bundled_inter_in_any_casing`, `mermaid_font_names_an_installed_family_ignoring_case`, `unset_empty_or_unknown_mermaid_font_falls_through_to_the_stack`, `stack_names_match_ignoring_case_in_stack_order`, `a_stack_generic_takes_the_os_answer_for_that_generic`, `no_installed_stack_name_takes_the_os_sans_serif_answer`, `no_generic_answer_takes_the_installed_sans_serif_rule`, `bundled_inter_is_matched_by_name_only`, `an_uninstalled_mermaid_font_warns_exactly_once_per_process`, `unset_or_empty_mermaid_font_never_warns` |
| AC-M3 (measured face = drawn face) | `measure_tests::measured_faces_are_drawn_faces_for_the_wrap_corpus_with_inter`, `..._on_host_fonts` (76 / 75 drawn lines) |
| AC-M4 (non-stack host) | `font_chain_tests::a_host_of_only_non_stack_fonts_resolves_every_query`, `..._paints_text`, control `an_empty_host_without_mermaid_font_paints_no_text` |
| AC-M8 (D50) | four `font_chain_tests::sans_rule_*` preference tests (each tied above the preference under test), `sans_rule_considers_only_sans_serif_classed_faces`, `metadata_of_inter_regular_matches_its_tables` |
| AC-M10/D59 (fallback) | `font_chain_tests::a_missing_character_falls_back_by_case_folded_family_name` (resolver's `select_fallback` = `fallback()`), `fallback_order_lists_installed_faces_only_then_last_resort`, `fallback_within_a_family_prefers_the_matching_weight` |
| R-M4 (rasterizer uses chain) | `font_chain_tests::rasterizer_paints_a_lowercase_mermaid_stack_in_bundled_inter`, `available_font_families_include_bundled_inter`, `family_names_list_installed_families_and_bundled_inter` |
| D63 hooks | `font_chain_tests::face_identities_follow_content_and_resolution`, `font_set_hash_ignores_loading_order_and_follows_content` |
| R-V8/AC-M1 (oracle) | `measure_tests::oracle_widths_match_usvg_on_bundled_faces` (deterministic: Latin, weights, italic, kerning on/off, combining marks, letter/word spacing incl. negative, small caps, emoji fallback, RTL/CJK as `.notdef` on both sides), `oracle_widths_match_usvg_on_host_fonts`, `oracle_widths_match_usvg_for_the_mixed_script_corpus` |
| AC-M2 (Merman wraps) | `measure_tests::wrapped_labels_fit_their_boxes_with_inter`, `..._on_host_fonts` (allow-list: only `wrap_er` "when it was created" / "and stored", upstream issue 6, ≈6.0–6.2 px; the test fails if they stop clipping); `measurer_measures_candidates_without_collapsing_whitespace`, `merman_measurer_reports_the_shaped_width`, `measurer_queries_usvg_default_family_for_a_style_without_one` |
| R-M10/R-V9/AC-M6 (`\n` lexer, string level) | `newline_tests::*` (13): one positive test per family (class, ER, mindmap, requirement, `classDiagram-v2`); per-family inputs with `\n` in every excluded construct (frontmatter, single- and multi-line `%%{init}%%`, `%%` comments, `click`/`link`/`callback`, `classDef`/`style`/`cssClass`, `accTitle`, `accDescr` line and block) plus one label that must change; 15 non-listed families from the S2 corpus unchanged and borrowed; header detection past comments/frontmatter; escaped `\\n`; CRLF. AC-M6's "fitting box" half is end-to-end and lands with the Phase 3 render path that calls the lexer |
| AC-P1 (OSC 4 parser) | `biscuit-terminal` `osc4_tests::*` (14) |
| AC-P2 (timeout, no re-send) | `biscuit-terminal` `level1_osc4_palette::silent_terminal_palette_returns_within_its_timeout_and_is_never_resent`, `da1_only_terminal_palette_is_unknown_and_never_resent`, `answering_terminal_palette_rides_the_startup_exchange` |
| AC-P5/R-P7 (no query) | `level1_osc4_palette::ci_sends_no_palette_query`, `redirected_output_sends_no_palette_query`, `multiplexer_sends_no_palette_query` |
| AC-P6/D46 | `only_unix_asks_for_the_palette`; docs |

The input-robustness matrix does not apply: Phase 2 adds no file or
configuration reader (the OSC 4 reply is a terminal protocol, per the plan's
scope note; `MERMAID_FONT` is a single string whose shapes are tested above).

### Gates

- `just test` (biscuit-visualized): **375 passed, 3 skipped**.
- `just lint` (biscuit-visualized): clean.
- `just test` (biscuit-terminal): **3881 passed, 55 skipped**.
- `just lint` (biscuit-terminal): clean.
- Dependents (narrowed, serially):
    - `claudine-cli --test l1 -E 'test(/pty/)'` (its PTY harness fakes the
      startup exchange): 122 passed. One test over 5 s,
      `sequence_task_providers::empty_hint_nested_sequence_list_fails_its_body_step_before_any_task_launches`
      (selected only because "empty" contains "pty"), passed alone in 3.06 s
      at host load ≈22–28 from other sessions: load-sensitive, unrelated.
    - `darkmatter -E 'test(/(?i)mermaid|diagram|svg|png|image|graph|raster/)'`:
      299 passed.
    - `worktree-cli -E 'test(/git_graph/)'`: 76 passed.
    - `sniff-cli -E 'test(/(?i)graph|ui|render|dependenc/)'`: 129 passed.
- Cross-OS (font chain and the shared startup exchange carry OS risk):
    - `just cross-check biscuit-visualized --os windows --all-features`:
      377 passed, 1 skipped.
    - `just cross-check biscuit-terminal --os windows`: 3483 passed, 64
      skipped.
    - `just cross-check biscuit-visualized --os linux --all-features`: 380
      passed, 1 skipped (the Linux-only Noto Emoji tests included).
    - `cargo check --target x86_64-pc-windows-gnu` clean for both crates.
- Scope check: see below.

### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py`, run after every edit of the phase
(this paragraph excepted): `packages: ["biscuit-terminal",
"biscuit-visualized", "test-toolkit", "darkmatter", "sniff"]`,
`change_class: package`. `source_packages` are the two packages changed;
darkmatter and sniff are "changed test input read by package(s)", and
test-toolkit is "changed suite input owned by package(s) test-toolkit".
The gates were therefore **not** skipped. The commit `81a1838bd`, made
mid-phase, is not in the working tree, so the calculator did not see it; its
three files (`Cargo.lock`, both `Cargo.toml`s) are covered by the runs above.

Narrowed runs for the test-input readers:

- `cargo nextest run -p test-toolkit`: 478 passed, 2 skipped.
- `cargo nextest run -p darkmatter --test l1 -E
  'test(/shipped_schema_corpus::|current_root_migration_guard::|skill/)'`:
  13 passed.
- `cargo nextest run -p sniff -p sniff-cli -E
  'test(/(?i)repo|lock|package|skill|cargo|workspace/)'`: 1315 passed, **4
  failed, pre-existing**: `sniff-cli::l1
  snapshots::{cargo_monorepo_structure_json_snapshot,
  cargo_pnpm_monorepo_structure_json_snapshot,
  pnpm_nx_monorepo_structure_json_snapshot, repo_aggregate_json_snapshot}`.
  The new output adds a `"monorepo_standards"` field that sniff emits since
  `cf9361989` ("feat(sniff): expose package identity and ownership", already
  on this branch). The snapshots were last updated in `e4e1bafd0`, before
  that commit, and they read temp-dir fixtures, not repository files. Phase 2
  changes no sniff file, and no dependency bump can add a JSON field. Left
  for the sniff owners. Not caused by this phase.

### Re-invocation (2026-10-22)

The Phase 2 prompt ran a second time after Phase 2 had been committed
(`8adcc3670`, `7eead9fca`). Every Phase 2 task and checkpoint in the plan was
already ticked, the working tree was clean, and both files already carried
every `*_during_phase_2` key and `packages`. Nothing was re-implemented and no
source file changed; this note is the only edit. The frontmatter, including
`human_review: true` (the font-hash ruling Phase 3 needs) and
`message_to_agent`, was left as the Phase 2 run wrote it.

Gate check for this re-invocation: `git status --porcelain
--untracked-files=all | awk '{print $NF}' | xargs python3
scripts/ci/affected_scope.py` gave `packages: []`, `change_class:
documentation`, so `just test` and `just lint` were skipped for it. The
Phase 2 gate results above still stand for the committed code.

### Rulings after Phase 2 (Ken, 2026-10-22)

The Phase 2 review item and the D46 finding are ruled; `human_review` is
cleared and Phase 3 is unblocked.

| Ruling | Decision | Evidence |
| ------ | -------- | -------- |
| Font identity in the cache key: hash contents only of the faces the chain resolves for the design's stacks and the generic families, the bundled Inter, and (Linux) Noto Emoji; identify every other reachable font file by path, size, modification time, and face index (option A). Accepted residual: an in-place replacement with identical size and timestamp is missed | D70 (amends D41, D63) | 2,986 files, 3,419 faces, about 1.7 GB on macOS; full content hashing took 571 ms and 950 ms per process with a warm OS cache, over D35's 500 ms cold limit (above) |
| OSC 4 on native Windows runs, by D46's own rule; Phase 5 enables it through the existing Windows backend and verifies it on the Windows test host; no answer ⇒ background-derived accents | D71 (amends D46) | `biscuit-terminal/lib/src/discovery/tty_query/windows.rs` already answers OSC 10/11 in the startup exchange; today `ASKS_PALETTE = cfg!(unix)` (Stream B) |
| Bounded implementer autonomy: when evidence invalidates a decision's premise, proceed with an option that preserves its purpose, stays within every other decision and limit, and adds no dependency, outward-facing action, or scope; amend the spec, log under "Decided during implementation", flag for review; otherwise stop and set `human_review` | D72 (pre-authorized item 12); also `2026-10-22-merman-text-fallback` D10 and `2026-10-21-mermaid-embedded-font` D17 | Both Phase 2 findings stopped work for a ruling |

## Phase 3

Orchestration: Wave 6 ran as four parallel sub-agents on disjoint files
(cache identity in `fonts/`; the SVG post-process module; the gitGraph
layout-data loader and tag spacing; the six upstream issue drafts). The
cutover agent then put `mermaid::MermaidDiagram` on Merman and wired those
pieces in (Waves 7–8). Gates were run once, by the orchestrator, at the end.

### Wave 6: render path, cache identity, post-process, upstream drafts

- **Render path** (`mermaid/render.rs`): pipeline in the spec's order —
  `default_chain()` → site config (`{"theme": <built-in name>}` merged with
  `MermaidConfig::to_json()`) → step 3b → cache key → Merman SVG
  (`SvgPipeline::resvg_safe()`, our `LabelMeasurer`, diagram id `bv-<hash of
  the cache key>`) → `postprocess` → background rect → PNG through the chain →
  store. Every Merman request (SVG and `layout_json`) gets
  `OperationControl::with_deadline(time_budget)`; a cancelled request logs
  `tracing::warn!` and returns `MermaidError::RenderFailed`; the component
  shows the code block. `MermaidError` gained no variant (AC-C1).
- **New public API** (additions only): `mermaid::MermaidDiagram::{DEFAULT_TIME_BUDGET
  (2 s), with_time_budget(Duration), time_budget()}`;
  `components::mermaid::MermaidDiagram::{with_time_budget, time_budget}`,
  passing through `MermaidRenderer` to the library type. `MERMAID_BACKEND` is
  now `merman@6e6b1e6554e8+bv1`. `default_gitgraph_commit_step()` returns 50.
  API reviewed against `git diff main` (no `cargo public-api` installed);
  `cargo check --tests -p worktree-cli -p darkmatter -p biscuit-terminal-cli
  -p sniff-cli` passes.
- **Themes (D61):** `Dark`/`Default`/`Forest`/`Neutral` → Merman's
  `dark`/`default`/`forest`/`neutral`; a test proves each SVG is byte-identical
  to Merman's own render with that name. Merman paints no background, so
  `Background::Theme`/`Solid` insert a rect over the root viewBox
  (theme colors from Mermaid 12.2.0: dark `#333`, was `#020617` under mmdr;
  default/forest `white`; neutral `#ffffff`); `Transparent` inserts nothing.
- **Cache key:** backend id, rewritten source, output format, xx_hash of the
  site-config JSON (the interim stand-in for the compiled design JSON),
  `FontChain::cache_identity(stacks)` for the theme font stack at 400 and
  700, `request.cache_identity()`, title.
- **Cache identity (D70)** in `fonts/chain.rs`: `FontChain::cache_identity(&[FontQuery])
  -> u64`, memoized per chain and stack list; `FaceIdentity` is now
  `Contents { hash, index }` (bundled faces, faces resolved for the stacks,
  and for the five generics at regular and at every weight/style/stretch the
  stacks use) or `File { path, size, modified, index }` (every other file
  face). In-memory installed faces (tests only) are hashed by contents.
  Metadata is read once per process.
  **Timing on this Mac** (3,419 faces, warm OS cache, three processes; no
  cold-disk run possible without `sudo purge`): font identity alone
  **46–69 ms** (9 files hashed by contents, metadata for ~2,986), against
  571–950 ms for Phase 2's full hash and D35's 500 ms cold limit. The first
  `cache_identity` call totals 370–470 ms because it pays the first fontique
  answer (333–544 ms) when resolving the stacks; system font load is
  195–200 ms more. That fontique cost is not new, but a cold first render is
  close to D35's 500 ms on macOS; Phase 6's benchmark must measure it.
- **Post-process** (`mermaid/postprocess.rs`, `postprocess(&str) -> Cow<str>`):
  replaces each `stroke:url(#X)` whose gradient X is `objectBoundingBox` with
  the stylesheet's solid node stroke (first stop color if none), and sets
  `font-family="sans-serif"` on the root only when no font is set (Wardley
  output has none). **Finding:** the divider defect is in every non-default
  Merman theme (dark, forest, neutral, base), not dark only; the fix
  therefore also makes neo outlines solid in those themes. Merman emits
  `overflow:visible` itself (label `<svg>`s, `.label-icon`); we add none. The
  flowchart gradient id is `<id>-merman-flowchart-document-gradient-root`,
  which S1's `ends_with("-gradient")` check would have missed.
- **Removed (R-C4, D33):** `apply_svg_overrides`, legend baseline, pie
  contrast, `%%{init}%%` theme overrides, quadrant-fill and circle-radius
  string patches, the mmdr theme builders and color helpers. Merman needed
  none. mmdr-only behavior that goes with them: HIGHLIGHT (`+N`) gitGraph
  nodes no longer forced to the branch color; `point_label_font_size` now
  sizes only quadrant point labels.
- **Upstream drafts (AC-U1):** `upstream-issues/01-default-measurement-clips-labels.md`,
  `02-wrapped-labels-lose-line-breaks.md`, `03-neo-dark-dividers-invisible.md`,
  `04-normalize-backslash-n-like-br.md`, `05-backslash-n-pie-gitgraph-block.md`,
  `06-er-attribute-comment-br-clips.md`. None filed; no issue URL reported by
  Ken yet. Source caveats noted in the drafts: issues 1–3's counts come from
  S1 (resvg 0.45.1); only issue 6 is confirmed at fork rev `6e6b1e6`; issue
  6's spread is 4.6–7.2 px (spec says 5–7); issue 4 is not a Mermaid.js
  parity fix (12.2.0 shows `\n` literally in those four families).

### Wave 7: step 3b and gitGraph on layout data

- `mermaid/gitgraph_layout.rs`: strict loader of Merman's `layout_json`
  (`layout.GitGraphDiagram`: `direction`, `bounds`, `commits[]` with `id`,
  `branch`, `parents`, `tags`, `pos`, `x`, `y`; author `commitSpacing` from
  `meta.effective_config.gitGraph.commitSpacing`, the one optional read).
  Duplicate keys anywhere are rejected (tree parse that keeps duplicates),
  trailing content rejected (`Deserializer::from_str` + `.end()`); every
  error carries its JSON path (`layout.GitGraphDiagram.commits[4].parents[1]`).
  Control fixture `tests/fixtures/merman/gitgraph/layout.json` is unedited
  output at the pinned rev; same keys as S0's at every level; a drift test
  regenerates it.
- Tag bounds mirror Merman's tag box (widest/tallest tag per commit, stacked
  20 px apart, LR box `min(pos − w/2 − 2, x − w/2 − 4)…x + w/2 + 4`, TB/BT
  rotated 45° + `translate(12,12)`); they match Merman's drawn polygons
  within 0.01 px. `commit_step` = smallest positive `pos` difference (50 at
  default; also holds under `parallelCommits`).
- Step 3b: `rewrite_label_newlines`, then for a gitGraph with `tag:` one
  `layout_json` pass at 50 px and `commitSpacing` written as a trailing
  `%%{init: {"gitGraph": {"commitSpacing": N}}}%%` (composes with an
  author's directive and frontmatter); an author's own `commitSpacing` wins;
  value rounded up to 0.01 px; only LR graphs, tags on different commits
  with overlapping rows; same-column pairs (`parallelCommits`) skipped. 1 em =
  the theme font size (16 px default), as `TAG_GAP_EM` and `tag_gap` already
  documented; the tag *font* is `tagLabelFontSize` (10 px).
- Values (bundled Inter, 16 px gap) vs S3 (font-free measurer):
  `gathered_graph_lays_out_long_labels` **145.35** (S3 125.9),
  `observation_1` **69.88** (64.2), S1 `case_tag_collision_adjacent` 97.42.
  Wider because real faces are wider, as S3 predicted.
- Merge repair, its "empty branch" error, and the mmdr spacing loop are
  deleted (D36). The public `#[doc(hidden)]` geometry types stay in
  `mermaid/gitgraph.rs`, re-exported unchanged.

### Wave 8: mmdr removed, triage, docs

- `mermaid-rs-renderer` removed from `biscuit-visualized/src/Cargo.toml` and
  `Cargo.lock`; `cargo tree -i mermaid-rs-renderer --workspace` →
  "did not match any packages"; no `elk` crate in `cargo tree -p
  biscuit-visualized --all-features`. All Phase 2 `dead_code` allows for the
  render path are gone.
- **Test triage:** deleted 12 merge-repair tests, 12 synthetic step/loop
  tests (behavior removed; arithmetic covered by `gitgraph_layout_tests`),
  the right-to-left gitGraph test (RL is not a Mermaid gitGraph direction),
  `mermaid_highlight_node_uses_branch_color`, `mermaid_backend_smoke…`
  (replaced by the family smoke corpus), and the pie test's legend/contrast
  assertions. `gitgraph_tests.rs` now holds 12 spacing tests through the
  public API.
- **D53 / AC-C13 inputs:** `direction LR`/`TB` inside gitGraph became
  `gitGraph LR:`/`TB:`; two unquoted pie labels (`components/mermaid.rs`) and
  a `C:` in a `newline_tests` class label were fixed. Checked offline against
  Mermaid.js 12.2.0 (`npm pack mermaid@12.2.0 --offline` from the npm cache,
  the puppeteer/headless Chrome already installed with mermaid-cli; nothing
  installed): 97 inputs extracted from 15 test files, 49 fixtures, 16 dynamic
  sources, 5 re-checks. Only the two malformed flowcharts and two ZenUML
  inputs are rejected, and all four tests expect it. **Unverified:** the
  Mermaid text the `GitGraph` component emits at test time, and other
  packages' inputs.
- One test input change: `components/git_graph/tests.rs`
  `measured_plans…` uses a 120×48 viewport (was 120×40); Merman draws that
  three-lane graph 269 units (22 rows) tall, past the base view's 20-row cap.
  No topology assertion changed.
- **Snapshots:** no `tests/l1/snapshots` file changed and no `.snap.new` was
  produced, so none was accepted.
- **Orchestrator test fix (after Windows cross-check):** the time-budget
  control `the_large_diagrams_render_within_the_default_budget` failed on
  Windows: a 60-node flowchart exceeded the real 2 s budget in a debug build
  ("cancelled during postprocess"). On macOS the same test took 2.7 s.
  The budget diagrams were shrunk (flowchart 15 nodes: 172 ms in Merman on
  macOS debug, 86 ms on Linux; gitGraph 200 commits) and the control's floor
  set to 5 ms, several times the 2 ms test budget; the component's budget
  test uses the same 15-node flowchart. This is a test-margin fix; debug
  Merman is slow, and the release-profile cost is Phase 6's benchmark.
- **Docs:** `docs/merman.md` (pipeline diagram with steps and owners, time
  budget, fallbacks, interim themes, gitGraph tag spacing, post-process,
  cache identity; **planned**: terminal palette step, palettes/accents/
  contrast, HTML-static target and Darkmatter static SVG),
  `docs/components/mermaid_diagram.md`, `docs/components/git_graph.md`,
  `README.md`, both `docs/dependencies.md` (Merman fork + pin; also fixed
  their claim that fontique links fontconfig at build time — it uses
  `fontconfig-dlopen`). **`docs/mermaid-gitgraph.md` is kept** and rewritten
  as "Mermaid gitGraph tag spacing" (merges as Merman draws them, spacing
  rules with a flow diagram, when spacing is skipped, cost and cache key,
  the geometry test API); it no longer claims Mermaid.js refuses a merge of
  an empty branch (AC-D3). Skill `.claude/skills/biscuit-visualized/` updated
  (`SKILL.md` 161 lines).
- Docs left for Phase 4/7 (other packages): `biscuit-terminal/docs/dependencies.md:257`,
  `biscuit-terminal/cli/README.md:331,401,435`,
  `.claude/skills/biscuit-terminal/diagrams.md:43`,
  `worktree/docs/git-graph.md:23` (still credits a merge's second parent and
  links "Mermaid gitGraph corrections").

### Decided during implementation

Flagged for the review cycle (D72). The spec is amended at D55 and D58.

1. **Step 3b parses a tagged gitGraph before the cache check** (D41, D55).
   *Evidence:* tag collision depends on commit order, lanes, and tag rows,
   which only a parse yields; reimplementing gitGraph semantics textually
   would duplicate Merman. *Choice:* one Merman `layout_json` pass at the
   default spacing, only for a gitGraph source containing `tag:`; every other
   diagram reaches the key with no Merman call
   (`the_key_is_checked_before_any_merman_svg_call`). The key still covers
   the rewritten source (D55's purpose). Cost: a tagged gitGraph pays one
   layout even on a cache hit (13 ms debug for 100 commits on macOS).
2. **Math labels** (D58, AC-C2). *Evidence:* with `math` off, Merman returns
   `MissingCapability { Math }` for the whole diagram, and its math-renderer
   hook (`with_math_renderer`) is `pub(crate)`. *Choice:* the diagram falls
   back to the code block (D20), which shows the label's source text; `math`
   stays off (no dependency added). The alternative — a pre-parse `$$`
   rewrite — would add scope, so it is not taken without a ruling.
   Test: `a_math_label_shows_as_its_source_text` (library and component).

Other departures recorded without a spec change:

- **ZenUML:** D20's premise ("unsupported") is false: `all-diagrams` renders
  it. The fallback D20 and AC-C9 require is kept by a header check before any
  Merman call (`zenuml_is_a_render_failure_without_a_merman_call`); Mermaid.js
  core 12.2.0 rejects ZenUML too.
- **Each Merman request has its own budget,** so a tagged gitGraph render
  (layout pass + SVG) can spend up to twice the budget in Merman.
- **`natural_size()` renders through the cache** (writes an SVG artifact) so a
  later `render` is a hit.
- **Diagram id** = `bv-` + hash of the cache key: different diagrams always
  differ; identical diagrams share identical CSS, which is harmless; one
  diagram under two backgrounds gets two ids. A per-document ordinal is the
  HTML caller's concern (Phase 4).
- **1 em for tag spacing** is the theme font size, not the tag font size,
  matching existing docs and S3.

### Requirement-to-test mapping

All L1, `biscuit-visualized` lib target unless noted; module `tests::` omitted.

| Requirement / AC | Tests |
| --- | --- |
| AC-C1 (additions only) | `git diff main` review; `cargo check --tests` of worktree-cli, darkmatter, biscuit-terminal-cli, sniff-cli |
| AC-C2 (no mmdr, D58 set, math) | `cargo tree` results above; `mermaid_tests::a_math_label_shows_as_its_source_text`, `components::mermaid::tests::a_math_label_shows_as_its_source_text` |
| AC-C6 (old artifact not served) | `cache_tests::an_artifact_cached_before_the_cutover_is_not_served` (stores under `main`'s exact old key), `cache_key_different_mermaid_backend` |
| AC-C6a (key inputs) | `mermaid_cache_key_tests::{the_key_follows_the_rewritten_source, the_key_covers_the_written_commit_spacing, …_output_format, …_one_design_color, …_post_process_version, the_key_carries_the_font_cache_identity, font_contents_are_hashed_once_across_two_renders, the_key_is_checked_before_any_merman_svg_call}`; chain level `cache_identity_tests::{changing_a_resolved_faces_contents_changes_the_identity, changing_only_another_files_size_changes_the_identity, changing_only_another_files_modification_time_changes_the_identity, another_files_contents_are_never_read, changing_only_mermaid_font_changes_the_identity, changing_only_the_installed_font_set_changes_the_identity, changing_only_the_fallback_order_changes_the_identity, a_css_requested_courier_face_is_covered_without_naming_it, contents_are_hashed_once_across_two_calls}` |
| AC-C7 (no `foreignObject`) + smoke of every family | `mermaid_family_tests`: one SVG and one PNG test per family × 4 themes, 33 families (registry minus `error`, `zenuml`), guard `the_smoke_list_is_mermans_family_registry` |
| AC-C9 (fallbacks) | `mermaid_tests::{malformed_flowcharts_are_parse_errors, an_unknown_diagram_type_is_a_render_failure, zenuml_is_a_render_failure_without_a_merman_call}`; `components::mermaid::tests::{a_parse_error_shows_the_code_block, an_unknown_diagram_type…, zenuml_shows_the_code_block}`; l1 `render_tree_diagram_hook::invalid_mermaid_source_falls_back_to_code_under_warn` |
| AC-C10 (budget) | `mermaid_tests::{an_svg_render_over_its_time_budget_fails_with_a_warning, a_gitgraph_geometry_request_over_its_time_budget_fails_with_a_warning, the_large_diagrams_render_within_the_default_budget, mermaid_time_budget_defaults_to_two_seconds}`; `components::mermaid::tests::{a_render_over_its_time_budget_shows_the_code_block_with_a_warning, the_time_budget_defaults_to_two_seconds_and_reaches_the_library_type}`; Merman-call elapsed ≤ budget + 250 ms, warning captured by a hand-written subscriber |
| AC-C11 (merges) | `gitgraph_tests::{a_labeled_merge_keeps_both_parents, a_merge_of_a_branch_without_its_own_commit_renders}`; layout level `gitgraph_layout_tests` labeled-merge and empty-branch-merge cases |
| AC-C12 (tag spacing) | `gitgraph_layout_tests` (76: loader matrix 46 cells, document shapes, tag-box vs drawn-polygon, spacing cases incl. spacing − 0.5 px leaves < 1 em, `None` for no tags/stacked/far apart/TB/author value) and `gitgraph_tests` (12 public-API spacing tests: no overlaps, `commit_step` = written value, 50 without collisions) |
| AC-C13 (inputs) | Mermaid.js 12.2.0 offline check above; parse-error tests |
| AC-C15 (themes) | `mermaid_tests::each_theme_renders_through_its_merman_built_in_theme`; backgrounds `backgrounds_paint_the_theme_a_solid_color_or_nothing`; config `MermaidConfig` reaches the SVG test |
| AC-M6 (`\n` end to end) | `newline_tests::rendered::*` (class, ER, mindmap, requirement: two drawn lines, no overflow; control: the unrewritten class note overflows; sequence unchanged) plus Phase 2's string-level `newline_tests` |
| AC-M7 / R-V10 (dividers) | `postprocess_tests::{postprocess_paints_class_dividers_in_dark_theme, postprocess_paints_er_dividers_in_dark_theme}` (each also asserts the un-post-processed divider is unpainted, so a Merman fix makes it fail) |
| R-M9 (post-process scope) | `postprocess_tests::{postprocess_gives_unset_merman_text_sans_serif, postprocess_defaults_only_text_without_a_font, postprocess_keeps_merman_root_font_stack, postprocess_leaves_svg_without_defects_unchanged, postprocess_is_idempotent_and_adds_no_overflow_patch}` |
| `natural_size` = drawn size | `mermaid_tests` natural-size vs PNG-at-scale-1 test; `mermaid_natural_size_rejects_unparseable_input` |

**Input robustness matrix.** It applies to the `layout_json` loader. Default
outcome for every invalid shape: a located error (JSON path + field),
geometry not built, caller gets `MermaidError::RenderFailed`. Exceptions:
`parents: []` and `tags: []` are valid (root commit / no tags); trailing
whitespace is allowed; `commitSpacing` absent means "author did not set it".
The code-smell grep (`#[serde(default)]`, `.ok()`, `unwrap_or_default`,
`filter_map(.. as_str())`) finds none on the loader's parses. The site
config we build is written, not read, so no other reader changed.

### Gates

- `just test` (biscuit-visualized), after all Wave 6–8 work: **542 passed, 3
  skipped**; rerun after the orchestrator's budget-test fix: see below.
- `just lint` (biscuit-visualized): clean (exit 0, no warnings).
- `just check-tier-coverage biscuit-visualized`: stranded 0.
- Doctests (`cargo test --doc`, run by the cutover agent): 31 passed, 11
  ignored.
- Load note: one run of the six budget tests on macOS reported 3 slow
  (23.9 s total) at host load ≈31 from other sessions; rerun alone, each
  ≤ 2 s. Load-sensitive, not a failure.
- Cross-OS (cache identity reads file metadata; Linux-only bundled face):
    - `just cross-check biscuit-visualized --os windows --all-features`:
      attempt 1 failed (the budget control above); attempt 2 **544 passed,
      1 skipped**.
    - `just cross-check biscuit-visualized --os linux --all-features`: first
      run failed the same control's 10 ms floor (gitGraph 8.5 ms on that
      fast host); after the fix **547 passed, 1 skipped**.
- Dependents (narrowed, serially):
    - `worktree-cli -E 'test(/git_graph/)'`: 41 passed, **4 failed** —
      `gathered_graph_lays_out_continued_after_merge_diverged`,
      `gathered_graph_lays_out_observation_2`,
      `gathered_graph_lays_out_sparse_lanes` (lanes hidden at 120×40: Merman's
      lanes are taller), `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`
      (width trimming keeps only the tip after a `+N` square: Merman is
      wider). Expected at this phase; Phase 4 rewrites them under D43
      (worktree is a consumer; no worktree code was touched here).
    - `darkmatter -E 'test(/(?i)mermaid|diagram|svg|png|image|graph|raster/)'`:
      299 passed.
    - `biscuit-terminal-cli -E 'test(/(?i)mermaid|diagram|graph|svg|image/)'`:
      30 passed; `biscuit-terminal --features image -E
      'test(/(?i)mermaid|diagram|render_tree_code|graph_expression|git_graph/)'`:
      18 passed.
    - `sniff-cli -E 'test(/(?i)graph|ui|render|dependenc|mermaid/)'`: 129
      passed.

### Budget-test robustness fix and final gates

- The full-suite rerun after the size fix failed one test under host load
  ≈20: `components::mermaid::tests::a_render_over_its_time_budget_shows_the_code_block_with_a_warning`,
  "Merman ran 264.8 ms on a 2 ms budget". It passed alone three times. The
  cause: in a fresh test process the measurer's first face resolutions (the
  fallback order, fontique answers for styles the key's stacks did not
  cover) run **inside** the first Merman call, so they were timed as Merman
  overrunning its deadline. Each budget test now first renders an
  uncancelled twin (the source plus a `%%` comment, a different cache key),
  which warms the chain, then asserts the budgeted call is within budget +
  250 ms **and** shorter than the twin's full Merman time. Measured on
  macOS after the warm-up: cancelled **2.2–2.4 ms**, full 23–59 ms.
- **Production fact for Phase 6:** the same 15-node flowchart's Merman call
  took 172 ms in a cold process and 59 ms warm (debug), so about 110 ms of
  a process's first render is font resolution counted against the time
  budget and D35's cold limit.
- **Final gates** (after every edit of the phase):
    - `just test` (biscuit-visualized): **542 passed, 3 skipped** (52.9 s at
      host load ≈39 from other sessions; 39 tests reported slow and six
      over 5 s, the slowest `measure_tests::oracle_widths_match_usvg_for_the_mixed_script_corpus`
      at 9.0 s, a Phase 2 test. In the first full run at lower load the
      slowest test was 3.4 s (`components::git_graph::tests::measured_plans_place_merges_and_tags_without_overlap`);
      recorded as load-sensitive, not failures).
    - `just lint`: clean.
    - `just check-tier-coverage biscuit-visualized`: 0 stranded.
    - `just cross-check biscuit-visualized --os windows --all-features`:
      **544 passed, 1 skipped** (attempt 3; attempts 1 and 2 are above).
    - `just cross-check biscuit-visualized --os linux --all-features`: **547
      passed, 1 skipped**.
- Skipped or pre-existing: the four worktree failures above (Phase 4, D43);
  the Phase 2 note that `biscuit-terminal`'s lib tests do not compile with
  `--all-features` still stands (`--features image` used).

### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py`, run after every edit of the phase
(this section excepted): `packages: ["biscuit-visualized", "test-toolkit",
"darkmatter", "sniff"]`, `change_class: package`, `source_packages:
["biscuit-visualized"]`. darkmatter and sniff are "changed test input read
by package(s)", test-toolkit "changed suite input owned by package(s)
test-toolkit" (the same readers as Phase 2: docs and skill files). The
gates were therefore **not** skipped. Narrowed runs for the readers:

- `cargo nextest run -p test-toolkit`: first run 477 passed, **1 timed
  out** at host load 45–88 from other sessions; rerun of the whole package
  478 passed, 2 skipped. Load-sensitive.
- `cargo nextest run -p darkmatter --test l1 -E
  'test(/shipped_schema_corpus::|current_root_migration_guard::|skill/)'`:
  13 passed.
- `cargo nextest run -p sniff -p sniff-cli -E
  'test(/(?i)repo|lock|package|skill|cargo|workspace/)'`: 1315 passed, **4
  failed, pre-existing** (the same four `sniff-cli::l1 snapshots::*_json_snapshot`
  failures recorded in Phase 2: sniff's `monorepo_standards` field postdates
  the snapshots; this phase changes no sniff file).

## Phase 4

Wave 9 ran as three parallel subagents (worktree D43, the HTML-static target,
the Darkmatter sanitizer), each verifying only its own slice; the
orchestrator then wired Darkmatter's static-SVG path, fixed stale
mermaid-rs-renderer mentions in biscuit-terminal, and ran every gate serially.

### `worktree` under D43 (R-C13, R-C14, AC-C4, AC-S6)

All four failing tests were Merman geometry (taller lanes, wider graph);
none was topology. No production code changed in worktree or
biscuit-visualized. The rewrite list below covers **every** hunk of
`git diff main -- worktree/cli/src/commands/git_graph/tests.rs` (8 hunks,
checked against the diff by the orchestrator):

| # | Test / item (line ≈) | Category | Old | New | Reason |
| --- | --- | --- | --- | --- | --- |
| 1 | imports (5) | helper | `{GitGraphPlan, GraphViewport, NaturalSize}` | adds `DEFAULT_GIT_GRAPH_SCALE` | used by hunk 8 |
| 2 | `Evidence::hidden_lanes` doc (1809) | helper (doc) | "at 120×40 and at 56×60" | "at 120 and at 56 columns; zero plans rows that fit the whole graph, more plans one row short" | matches hunk 4; the `[usize; 2]` expectations are unchanged |
| 3 | `assert_laid_out` doc (1839) | helper (doc) | "at 120×40 and 56×60" | "at 120 and 56 columns" | as hunk 2 |
| 4 | `assert_laid_out` viewports (1850) | helper, mechanics | `[viewport(120, 40), viewport(56, 60)]` | rows derived from the measured graph: `2 * whole_rows` when the expected hidden count is 0, else `2 * (whole_rows - 1)`; columns stay 120 and 56; `assert_eq!(planned.hidden_lanes, hidden_lanes)` unchanged | the height cap is half the rows; 40 rows was a pixel budget that Merman's taller lanes overflow (whole graph 22 rows → cap 20 hid lanes in `observation-2` and `continued-Diverged`, and 2 instead of 1 in `sparse-lanes`). Deriving rows keeps every hidden-lane expectation and all topology evidence |
| 5 | `assert_laid_out` `--width 40` (1924) | helper, mechanics | comment "60 rows: tall enough…" + `viewport(120, 60)` | `viewport(120, rows_for(0))` | "60 rows hides no lane" was a pixel-budget claim; `hidden_lanes == 0`, `columns == 40`, `trimmed == 0` unchanged |
| 6 | `gathered_graph_lays_out_sparse_lanes` comment (2019) | mechanics (comment) | "At 40 rows the height cap leaves out…" | "One row short of the whole graph…" | matches hunk 4; `hidden_lanes: [1, 0]` unchanged |
| 7 | `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` doc (2178) | mechanics (doc) | "every long lane keeps more than its tip" | "…whenever another commit step fits the width" | matches hunk 8 |
| 8 | same test (2203) | mechanics | `assert!(kept.len() >= 2)` for `feat/schema-enhancement`, `fix/sniff` | `assert!(kept.len() >= 2 \|\| one_more_step > 200)` with `one_more_step = ImageWidth::scaled_columns(DEFAULT_GIT_GRAPH_SCALE, geometry.width + geometry.commit_step, CellSize::FALLBACK)` | a count of what fits 200 columns is a pixel budget (D43's own example). Note: with Merman's width the second branch holds today, so the test now proves trimming stops once the graph fits rather than that two commits are kept at 200 columns |

- Unchanged: the line-2193 `commit_step == default_gitgraph_commit_step()`
  check (already passing), both `tag_overlaps()` assertions (byte for byte),
  every topology assertion (tags on SHAs, merge parents, `post_merge`,
  `incomplete`, `hidden_lanes` values, tip identity, `W1` drawn, M103/M104
  lanes and parents, M104's exact tags `main`/`origin/main`),
  `a_direct_merge_into_the_default_branch_beats_the_parents_indirect_containment`.
- AC-S6 confirmed on the real implementation: S3's replay held; no topology
  assertion needed editing.
- Product-visible effect: in a real 40-row terminal a three-lane base view
  (such as `observation-2`) now loses a lane to the half-height cap.
- Fixture fragility noted by the subagent: at a hand-picked 120×40,
  `observation-2` hid 1 lane in one run and 2 in another (likely wall-clock
  commit times tying in the cap's ranking); derived rows avoid it.
- Docs: `worktree/docs/git-graph.md` (who-does-what table: Merman draws both
  merge parents, biscuit-visualized writes `commitSpacing` only when tags
  collide; spacing paragraph; tests section; two stale
  `biscuit-terminal/lib/src/components/git_graph*` paths fixed) and
  `.claude/skills/worktree/git-graph.md`.

### HTML static target (D60, R-C9a, AC-C8a)

- API (additions only): `mermaid::MermaidTarget { RasterSafe (default),
  HtmlStatic }` (`#[non_exhaustive]`, re-exported from
  `components::mermaid`), `mermaid::MermaidDiagram::{with_target, target}`,
  `components::mermaid::MermaidDiagram::with_target` (passes through to
  `render_to_svg`).
- Measured face: `LabelMeasurer::measured_family(&TextStyle)` reuses the
  measurer's own cached face lookup (`face_for` → `FontChain::resolve`);
  `render_svg` shares one `Arc<LabelMeasurer>` between Merman and
  `postprocess::html_static`, so no second resolution path exists.
- `postprocess::html_static` rewrites every `font-family` (in `<style>`
  rules, inline `style`, and attributes) to `'<Face>',<original stack>`, and
  sets `overflow` to `visible` on every non-root `<svg>` that holds `<text>`
  (Merman clips class, ER, mindmap, and state labels there).
- Decided during implementation:
    - **PNG ignores the target** (forced to `RasterSafe`) rather than
      rejecting it, so the rasterizer never sees either change; tested by a
      test-only record of the SVG fed to the rasterizer.
    - **Cache key:** `"target": "html-static"` is added to the options only
      for SVG + `HtmlStatic`; default keys are byte-identical, so **no
      `+bv<n>` bump**.
    - **The root `<svg>` keeps its overflow**: opening it would let content
      spill into the page; a label at the very edge can still be clipped by
      the browser's default.
    - The face is prepended even when the stack already starts with the same
      family in other casing (`'Trebuchet MS',"trebuchet ms",…`): only the
      exact lead the pass writes counts as done, keeping it idempotent.
    - A bug found on the way: splitting inline styles on `;` cut through
      `&quot;`; values are now entity-decoded before parsing.

### Darkmatter sanitizer (D37, D64, R-C9, AC-C8b)

- Survey: 22 families × 4 themes (plus front-matter themes and looks) through
  `render_to_svg`. **Only `filter` and `feDropShadow` were being dropped**;
  every other Merman element (svg, g, style, text, tspan, rect, path, circle,
  ellipse, line, polygon, defs, marker, use, symbol, title, clipPath,
  linearGradient, stop) and attribute already survived, so nothing else was
  widened.
- Shadows appear in flowchart, sequence, class, state, ER, mindmap,
  requirement, block (every theme) and timeline (redux themes). Every
  reachable shadow carries only `id`, `height`/`width` (130%/150%), `dx`/`dy`
  (4 or 2), `stdDeviation="0"`, `flood-opacity`, `flood-color` (hex) — it
  passes D64 as written.
- Rules as implemented: `<filter>` kept only when all attributes pass and at
  least one valid `<feDropShadow>` survives (an **empty filter is dropped**,
  because an empty `<filter>` renders its target transparent; a dangling
  `url(#…)` paints unfiltered); `dx`/`dy` capped by magnitude (−20 kept, −21
  dropped); `stdDeviation` one or two numbers in 0–20; `flood-opacity` a
  plain number in 0–1; any other attribute a number, percentage, or hex /
  `rgb()`/`hsl()` color (named colors rejected); a strict number grammar
  rejects `inf`/`NaN`; other `fe*` primitives, a self-closing filter, and a
  shadow outside a filter are dropped; local-only `url(#…)` unchanged.
- Latent conflict (not live): Merman's `ThemePreset` shadow path emits
  `in`, `filterUnits`, and a `feGaussianBlur`/`feOffset`/`feMerge` chain that
  D64 drops. biscuit-visualized never selects a `ThemePreset`.
- Not capped by D64: the filter region (`width="100000%"` passes as a
  number). Merman emits only 130% and 150%; a cap would be a spec decision.
- Module docs that named mermaid-rs-renderer and listed `filter` among the
  always-dropped families were corrected (drift, code assumed correct).

### Darkmatter static-SVG path (D14, D37, R-C8, R-V7, AC-C8, AC-C8a)

- `render_browser_mermaid` (`render_tree/code_renderer.rs`) selects
  `MermaidTarget::HtmlStatic` and sanitizes; `MermaidMode::Off` is untouched;
  `darkmatter/lib/src/mermaid/theme.rs` is not touched (D23).
- `darkmatter/docs/rendering/mermaid.md`: a "Static SVG" section (measured
  face first, overflow, sanitizer and drop-shadow limits, failure handling);
  the terminal `Image` row no longer says it "may launch a subprocess".

### biscuit-terminal and sniff (AC-C3)

- Stale mermaid-rs-renderer mentions fixed: `biscuit-terminal/cli/README.md`
  (three "Rendering backend" bullets), `biscuit-terminal/docs/dependencies.md`,
  `.claude/skills/biscuit-terminal/diagrams.md`.
- `lib/tests/l1/render_tree_code_context.rs:233` (`graph TD; A to B`) is a
  parse error under Merman, as intended; unchanged.

### Blocking finding: Merman makes `serde_json` keep insertion order in every dependent

`merman-core` (and `merman-bindings-core`) enable serde_json's
`preserve_order` and `float_roundtrip` features. Cargo unifies features per
build, so **every package whose build reaches biscuit-visualized** now gets
an insertion-ordered `serde_json::Map` instead of a sorted one. `cargo tree -e
features -i serde_json` names `merman-core` as the only source for
claudine-cli, claudine-gen, messenger-cli, research-cli, biscuit-icon-cli,
biscuit-speaks-cli, playa-cli, biscuit-terminal-cli, darkmatter-cli, dmls,
sniff-cli, and worktree-cli. Merman depends on it (its models iterate JSON
maps in insertion order, for example gitGraph arrows follow the commits
map), so dropping it in the fork is not a small patch.

This arrived with Phase 2's dependency, not with this phase's work. Earlier
phases' narrowed dependent runs missed it, and the four `sniff-cli`
`*_json_snapshot` failures that Phases 2 and 3 recorded as pre-existing
(`monorepo_standards`) were **this regression**: with maps sorted they pass.

- **sniff (fixed here, test-only):** the JSON snapshot tests parse the CLI's
  output into `serde_json::Value` and re-serialize it, so the snapshot order
  depended on the feature. `sniff/cli/tests/l1/snapshots.rs` now asserts
  through an `assert_sorted_json_snapshot!` macro (insta `sort_maps`), which
  is order-independent and matches the committed snapshots; all five
  (`os_json_snapshot` plus the four above) pass. sniff's own output is
  unchanged by this edit.
- **darkmatter (not fixed: needs a ruling): 45 L1 failures**, all key order:
    - `markdown::frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`
      asserts Darkmatter's documented contract that "ordinary serde_json maps
      keep their established canonical order";
    - `benchmark_fixtures::benchmark_manifest_matches_recorded_identities`:
      the **frontmatter hash** of `compose_schema_transclusion` and
      `replace_heavy` changed, so content identity depends on the flag;
    - `darkmatter-cli::l1 schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`:
      `md schema validate --json` key order changed;
    - `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`
      (`as_json5` output order);
    - 40 insta snapshots: 39 `schemas_convert_snapshots::*` and
      `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`
      (`$schema`/`type` order).
- **Not yet measured:** claudine, messenger, research, dmls, playa,
  biscuit-icon, and biscuit-speaks CLIs carry the same flag; their suites
  were not run in this phase.

Raised as `human_review_items` in this log's frontmatter.

### Requirement-to-test mapping

| Requirement / AC | Tests (all L1 unless noted) |
| --- | --- |
| AC-C4, AC-S6, R-C13, R-C14 (worktree D43) | `worktree-cli` `commands::git_graph::tests::*` (76 pass), incl. `gathered_graph_lays_out_continued_after_merge_diverged`, `gathered_graph_lays_out_observation_2`, `gathered_graph_lays_out_sparse_lanes`, `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60`; rewrite list above, reviewed against `git diff main` (8 hunks) |
| AC-C8a, R-C9a, D60 (HTML static target, biscuit-visualized) | `tests::html_static_tests::{html_static_leads_every_label_font_with_the_measured_face, html_static_names_the_face_the_measurer_resolves, html_static_lets_label_viewports_overflow, html_static_edits_only_font_values_and_text_viewports, html_static_is_idempotent, html_static_target_svg_leads_label_fonts_with_the_measured_face, default_target_svg_keeps_original_fonts_and_clipping, png_render_rasterizes_the_raster_safe_svg_under_the_html_static_target}`; `mermaid_cache_key_tests::the_html_static_target_has_its_own_svg_key_only`; existing `postprocess_is_idempotent_and_adds_no_overflow_patch` still passes |
| AC-C8b, R-C9, D64 (sanitizer) | `darkmatter` `markdown::render_tree::svg_sanitizer::tests::{keeps_merman_drop_shadow_verbatim, drop_shadow_offsets_are_capped_by_magnitude, drop_shadow_std_deviation_is_capped, drop_shadow_flood_opacity_is_limited_to_unit_range, drop_shadow_numbers_reject_empty_and_non_numeric_values, drop_shadow_colors_accept_hex_and_numeric_functions_only, drop_shadow_with_keyword_attribute_is_dropped, filter_with_non_numeric_attribute_is_dropped_with_its_children, filter_region_accepts_numbers_and_percentages_only, other_filter_primitives_are_dropped, filter_left_without_a_drop_shadow_is_dropped, drop_shadow_outside_a_filter_and_its_children_are_dropped, drops_external_url_on_filter_attribute_of_shadowed_element, unterminated_filter_returns_none, merman_flowchart_survives_with_labels_and_shadow}`; existing external-URL negatives unchanged (28 pass) |
| AC-C8, AC-C8a, R-C8, R-V7, D14 (Darkmatter per mode, public page path) | `darkmatter::l1 mermaid_static_svg::{mermaid_off_keeps_the_interactive_source_and_embeds_no_svg, mermaid_image_embeds_a_static_svg_whose_labels_and_shadows_survive, mermaid_image_leads_every_label_font_with_the_measured_face, mermaid_image_lets_clipped_labels_overflow}` (the font test compares the page's values with the raster-safe SVG: each is `'<face>',` + the original stack) |
| AC-C3 | gates below |

Input robustness: the sanitizer's attribute validators (number/percentage,
color, caps) are covered for empty, wrong-type, boundary, and out-of-range
values above. No file-format or configuration loader changed.

### Gates

Run serially after every subagent finished.

- `biscuit-visualized`: `just test` **551 passed, 3 skipped**; `just lint`
  clean; `just check-tier-coverage biscuit-visualized` 0 stranded;
  `just cross-check biscuit-visualized --os windows --all-features` **553
  passed, 1 skipped** (cross-OS risk: HTML-static tests resolve faces from
  host fonts).
- `worktree`: `just test` **1594 passed, 15 skipped**; `just lint` clean.
- `sniff`: first `just test` 3453 passed, **5 failed** (key order, above);
  after the snapshot fix **3463 passed, 33 skipped**; `just lint` clean.
- `biscuit-terminal`: `just test` **3881 passed, 55 skipped**; `just lint`
  clean; `just test-l2`: 252 + 7 passed, **1 failed**:
  `level2_graphics_tmux::level2_tmux_draws_sixel_itself_at_the_bottom_of_the_viewport`
  (the text after a tmux-drawn Sixel lands 5 rows below, expected 4; fails
  alone three times). It tests tmux's Sixel placement through
  biscuit-terminal's lib, reaches no diagram or biscuit-visualized code, and
  this phase changed no biscuit-terminal code (docs only): host tmux 3.8
  behavior, not caused here.
- `darkmatter`: `just lint` clean; `just test` (rerun with `--no-fail-fast`
  to size the problem) **10558 passed, 45 failed, 10 skipped**: every
  failure is the key-order finding above; the new and changed Mermaid
  tests all pass. **Checkpoint 4's first item is therefore not met.**

### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py` after every edit (the working tree
still carries Phase 3's uncommitted changes): `packages:
["biscuit-visualized", "darkmatter", "sniff-cli", "worktree-cli",
"test-toolkit", "biscuit-terminal-cli", "sniff"]`, `change_class: package`,
`source_packages: ["biscuit-visualized", "darkmatter", "sniff-cli",
"worktree-cli"]`. The gates were run, not skipped.
