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
source_files_during_phase_5:
    - biscuit-terminal/lib/src/discovery/osc_queries/mod.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/query.rs
    - biscuit-terminal/lib/src/discovery/startup_exchange.rs
    - biscuit-terminal/lib/tests/level2/level2_windows_console_query.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/mermaid/config.rs
    - biscuit-visualized/src/src/mermaid/color.rs
    - biscuit-visualized/src/src/mermaid/palette.rs
    - biscuit-visualized/src/src/mermaid/design.rs
    - biscuit-visualized/src/src/mermaid/accents.rs
    - biscuit-visualized/src/src/mermaid/contrast.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_cache_key_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_tests.rs
    - biscuit-visualized/src/src/tests/palette_tests.rs
    - biscuit-visualized/src/src/tests/theme_tests.rs
    - biscuit-visualized/features/2026-10-21-merman/gallery/Cargo.toml
    - biscuit-visualized/features/2026-10-21-merman/gallery/Cargo.lock
    - biscuit-visualized/features/2026-10-21-merman/gallery/src/main.rs
docs_updated_during_phase_5:
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-terminal/docs/discovery/terminal-queries.md
    - biscuit-terminal/docs/discovery/color.md
    - biscuit-visualized/features/2026-10-21-merman/plan.md
    - biscuit-visualized/features/2026-10-21-merman/implementation-log.md
docs_created_during_phase_5:
    - biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-macos.html
    - biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-linux.html
skills_files_updated_during_phase_5:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
    - .claude/skills/biscuit-visualized/diagram-components.md
    - .claude/skills/biscuit-terminal/discovery.md
source_files_during_phase_6:
    - biscuit-visualized/src/Cargo.toml
    - biscuit-visualized/justfile
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/raster/png.rs
    - biscuit-visualized/src/src/mermaid/contrast.rs
    - biscuit-visualized/src/src/mermaid/design.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/golden_tests.rs
    - biscuit-visualized/src/src/tests/contrast_corpus_tests.rs
    - biscuit-visualized/src/src/tests/corpus_measure_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_family_tests.rs
    - biscuit-visualized/src/src/tests/measure_tests.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_cache_key_tests.rs
    - biscuit-visualized/src/src/tests/goldens/svg/*.snap (138 files)
    - biscuit-visualized/src/src/tests/goldens/png/*.png (66 files)
    - biscuit-visualized/bench/Cargo.toml
    - biscuit-visualized/bench/Cargo.lock
    - biscuit-visualized/bench/baseline.json
    - biscuit-visualized/bench/src/main.rs
    - biscuit-visualized/bench/src/baseline.rs
    - biscuit-visualized/bench/src/corpus.rs
    - biscuit-visualized/bench/src/stats.rs
    - biscuit-visualized/bench/tests/wiring.rs
docs_updated_during_phase_6:
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-macos.html
    - biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-linux.html
docs_created_during_phase_6:
    - biscuit-visualized/bench/README.md
skills_files_updated_during_phase_6:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
decisions_for_review:
    - |-
      **Sign off on the new diagram look (AC-T5; moved to this review by D74).**
      Phase 6 generated its golden snapshot files from this look; if you ask
      for changes, the review cycle changes the styling and regenerates the
      goldens.

      Open `biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-macos.html`
      (and `themed-gallery-linux.html`, the same diagrams drawn on the Linux
      build host). Each row is one diagram type; the six columns are the four
      fixed themes (Dark, Default, Forest, Neutral), a terminal that reported
      its full color set (Tokyo Night), and a terminal that reported only its
      background. Compare them with Zed screenshots of the same diagrams.

      - **Option A: approve as is.** Phase 6 can generate the goldens. Pro:
        no further delay. Con: anything you dislike later means regenerating
        goldens.
      - **Option B: approve with specific changes** (name the diagram types
        and what to change). Pro: the goldens capture the look you want.
        Con: one more styling round before Phase 6.
      - **Option C: reject the overall direction.** Con: largest rework.

      Recommendation: **B** if any row looks wrong to you, otherwise **A**.
      The known weak spots are listed at the end of the log's "## Phase 5"
      section ("Open"): sequence-diagram symbol participants (database,
      boundary, …) all share one accent color, and three rarely used diagram
      types (agentflow, usecase, wardley) cannot be themed by Merman's
      compiled themes at all.
    - |-
      **Approve four styling choices that bend the letter of the spec.**
      The spec says designs "never set the node base fill", that Mermaid
      variables appear "only in gitGraph", that an author's `%%{init}%%`
      keys win "for those keys", and that accents key on "our palette colors
      only". Following those exactly left many diagrams unreadable on dark
      backgrounds, because Merman's underlying `base` theme is light. These
      choices are shipped and tested; each can be reverted independently.

      1. Diagram types that Merman's compiled themes only partly reach
         (treemap, quadrant, ER, gantt, C4, block, architecture,
         requirement, packet, and others) also receive the palette through
         Merman's ordinary site configuration (the channel `MermaidConfig`
         already uses). The designs themselves still carry no Mermaid
         variables outside gitGraph.
      2. Class, block, and architecture boxes get the palette's surface as
         their base fill. That is safe there because those types have no
         rotating per-node palette for a base fill to override, which is the
         problem the spec's rule exists to prevent.
      3. When a diagram's source sets a theme *color* in `%%{init}%%` or
         frontmatter, the compiled theme is dropped entirely and the
         author's colors win, with the palette underneath. Yielding key by
         key is not possible: compiled styles are not keyed by Mermaid
         variable names.
      4. Sequence-diagram participants get their accent by matching the
         participant box's `name` attribute, not a palette color, because
         Merman paints those boxes from a stylesheet.

      - **Option A: accept all four** and amend the spec to match (a short
        new decision entry). Pro: the spec describes what ships. Con: none
        beyond the edit.
      - **Option B: accept some and revert the others.** Con: reverted ones
        bring back unreadable dark-palette output for those diagram types
        (visible in the gallery before this phase).
      - **Option C: revert all and accept the limits.**

      Recommendation: **A**. Each choice is what makes the dark palettes
      readable, and each is pinned by a test (log "## Phase 5", "Decided
      during implementation").
    - |-
      **The look changed in eleven diagram types after the gallery you have
      not yet reviewed; review it in the regenerated gallery.** Phase 6's
      automated contrast check (every text at 4.5:1 against what is drawn
      behind it) found 160 unreadable labels in 12 diagram types. They were
      fixed in the styling code, not by loosening the check: C4 boundaries are
      no longer filled (the fill was drawn over the nodes and hid them in
      PNGs), kanban columns are darker on dark themes, timeline sections use
      stronger colors (their text is a fixed dark gray), event-modeling and
      cynefin boxes take palette tints, gitGraph commit-label backgrounds are
      darker, and sankey, treemap, gantt, venn, and tree-view colors were
      adjusted. Both galleries were regenerated, and so were the goldens.
      - **Option A: accept.** Pro: every label is readable, proven by test.
      - **Option B: ask for different colors per type.** Any replacement must
        still pass the contrast test; the goldens are regenerated.
      Recommendation: **A**, after a look at C4, kanban, and timeline in
      `gallery/themed-gallery-macos.html`.
    - |-
      **One known unreadable case is allow-listed: sequence-diagram message
      text on an author's light `box` in dark palettes (contrast 1.07–1.50).**
      The box color is the author's but the text color is ours, so by the
      spec it should be fixed. The contrast pass cannot reach it: the text is
      colored by a stylesheet, the box by an attribute, and the messages are
      not inside the box's element. Fixing it needs the pass to reason about
      regions (which drawn rectangle contains a text), a larger change.
      Other allow-listed failures are all the authors' own colors (C4
      `$fontColor`/`$textColor`, venn `style … color:red`), which we never
      change.
      - **Option A: keep the allow-list and schedule a follow-up** for
        region-aware contrast. Pro: no risky change now. Con: dark-theme
        sequence diagrams with a light `box` stay hard to read.
      - **Option B: fix now** with geometry in the contrast pass. Con: more
        work in this feature; goldens of affected diagrams change.
      Recommendation: **A**. Removing the allow-list entry makes the test fail
      until it is fixed.
    - |-
      **The benchmark flags two diagram types on the Linux reference host.**
      Warm render medians there: sequence 56 ms and timeline 84 ms, over the
      spec's 50 ms limit (all others 2–45 ms; every cold render under 100 ms,
      far below 500 ms). `just bench` on build-linux therefore exits 1 today.
      The limits were not changed and the baseline records these timings.
      - **Option A: accept** and record it (the baseline then catches any
        further 25% slowdown). Pro: no work. Con: two families stay over the
        stated rule.
      - **Option B: raise the warm limit** (for example to 100 ms). Con:
        weakens the rule for every family.
      - **Option C: profile and speed up** sequence and timeline (much of a
        timeline render is rasterizing a 2390 px wide image at 2×). Con:
        work in Merman or the rasterizer.
      Recommendation: **A** now, **C** as a follow-up.
      Smaller benchmark choices the spec did not settle, all in
      `bench/README.md`: the reference host is recognized by hostname
      matching the baseline's `reference_host` (or `--reference`); the cold
      limit is judged on the median; a family with no baseline entry flags on
      the reference host; an empty `families` object is valid. Each is a
      few-line change if you prefer another.
    - |-
      **Goldens pin their font with a bundled-only font chain, not the
      `MERMAID_FONT` environment variable.** The spec says goldens render with
      `MERMAID_FONT=Inter`. The process-wide font chain reads that variable
      once, so setting it inside one test is unreliable. Instead the tests
      render through a new crate-private entry point that takes the chain,
      using a chain of the bundled fonts with `MERMAID_FONT=Inter` and no
      installed fonts. The result is the same as the spec's intent and
      stronger: no installed font can reach a golden. The PNG goldens also
      render at 1× (the default request is 2×) to stay small and fast. If you
      prefer the literal reading, the goldens would need `MERMAID_FONT` set
      for the whole test process (a nextest setup step), with the same
      output. The fixture license files keep their Phase 3 names
      (`families/LICENSES/merman-LICENSE-MIT`, `merman-LICENSE-APACHE`,
      `mermaid/LICENSE`) rather than the plan's `LICENSE-MIT`,
      `LICENSE-APACHE`, `THIRD_PARTY_LICENSES/mermaid`; renaming is a
      file move plus one README line.
    - |-
      **gitGraph tag sizes now follow Merman's own reading (a fix made in
      the final phase).** The final checks found that the tag spacing code
      read an author's `tagLabelFontSize` as a plain pixel number, while
      Merman accepts `px`, `em`, `%`, `inherit`, and `unset` and draws a
      bare number at 10 px. With `"150%"`, tags were spaced for 10 px but
      drawn at 24 px, so they overlapped. The code now reads the value
      exactly as Merman draws it, 12 new tests compare our measured tag
      boxes with the boxes Merman actually draws, and the cache version was
      bumped (`+bv4`) so no old image is reused.
      - **Option A: keep the fix.** Pro: tags never overlap, whatever unit
        the author uses. Con: none known.
      - **Option B: revert it** and document that only `px` sizes are
        supported. Con: overlapping tags for the other units.
      Recommendation: **A**. Reverting means undoing two small functions in
      `mermaid/render.rs` and deleting the tests.
    - |-
      **`docs/mermaid-gitgraph.md` was kept, not merged into
      `docs/merman.md`.** The spec allowed either. That page is the only
      full description of the gitGraph tag-spacing rules, which apply to
      every caller, not just the git graph component. `merman.md` keeps a
      short summary that links to it.
      - **Option A: keep it.** Pro: `merman.md` (already long) stays
        readable. Con: one more page.
      - **Option B: fold it into `merman.md`.** Con: a longer page; links
        from about six other pages and skills must be repointed.
      Recommendation: **A**.
    - |-
      **The final check of the other packages was scoped, not a full run of
      every suite.** The plan's final sweep lists `just test` and
      `just test-l2` in five packages. This phase changed only comments and
      docs in `biscuit-terminal`, `darkmatter`, and `worktree`, plus a
      gitGraph tag-size path none of them uses. So it ran the full suite in
      `biscuit-visualized` (933 passed) and the affected tests in the others
      (worktree graph 87/87 L1; darkmatter Mermaid 125/125; biscuit-terminal
      color and OSC 448/448; biscuit-terminal-cli 4/4). Still open from
      before: darkmatter's 45 key-order failures (their own fix) and one
      worktree Kitty L2 test.
      - **Option A: accept,** and let CI run the full cells (about 60–100
        minutes for a pull request).
      - **Option B: run every suite locally before merging.** Con: several
        hours of local runs that prove nothing new.
      Recommendation: **A**.
    - |-
      **Found, not fixed: the graph (non-Mermaid) image cache ignores
      fonts.** Graph PNGs are drawn through the same font chain as Mermaid
      diagrams, but their cache key has no font identity. After
      `MERMAID_FONT` changes, or a font is installed, a graph drawn earlier
      is served in its old face until its cache entry is removed (the
      component doc says so). Mermaid's key does include fonts.
      - **Option A: a small follow-up** that adds the font chain's cache
        identity to the graph key (and bumps the graph backend id).
      - **Option B: accept it.** Graphs rarely change fonts.
      Recommendation: **A**, as its own fix. Also noted for follow-up:
      stale comments in `darkmatter/lib/src/mermaid/mod.rs` (`mmdc`, icon
      packs) and `markdown/output/terminal.rs` ("mermaid.ink"); 58
      pre-existing broken workspace links in the root
      `docs/dependencies.md`; and `biscuit-terminal`'s lib tests not
      compiling with `--all-features`.
source_files_during_phase_7:
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/mermaid/config.rs
    - biscuit-visualized/src/src/mermaid/postprocess.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-visualized/src/src/tests/gitgraph_layout_tests.rs
    - biscuit-visualized/src/src/tests/gitgraph_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_cache_key_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_family_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_tests.rs
    - biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/mod.rs
    - biscuit-terminal/lib/src/discovery/startup_exchange.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
docs_updated_during_phase_7:
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/docs/mermaid-gitgraph.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-visualized/docs/components/git_graph.md
    - biscuit-visualized/docs/components/graph_expression.md
    - biscuit-visualized/docs/dependencies.md
    - biscuit-visualized/README.md
    - biscuit-visualized/bench/README.md
    - docs/dependencies.md
    - biscuit-terminal/docs/discovery/terminal-queries.md
    - biscuit-terminal/docs/discovery/color.md
    - biscuit-terminal/cli/README.md
    - darkmatter/docs/rendering/mermaid.md
    - worktree/docs/git-graph.md
    - worktree/docs/performance-testing.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
    - .claude/skills/biscuit-visualized/rasterization.md
    - .claude/skills/biscuit-terminal/diagrams.md
    - .claude/skills/biscuit-terminal/discovery.md
source_code:
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/Cargo.toml
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/Cargo.lock
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/src/main.rs
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
    - biscuit-visualized/src/src/components/git_graph.rs
    - biscuit-visualized/src/src/components/git_graph/tests.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - biscuit-visualized/src/src/mermaid/gitgraph.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/tests/cache_tests.rs
    - biscuit-visualized/src/src/tests/gitgraph_tests.rs
    - biscuit-visualized/src/src/tests/mermaid_tests.rs
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
    - biscuit-visualized/src/src/tests/html_static_tests.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - darkmatter/lib/src/markdown/render_tree/svg_sanitizer.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
    - darkmatter/lib/tests/l1/mermaid_static_svg.rs
    - darkmatter/lib/tests/l1/main.rs
    - sniff/cli/tests/l1/snapshots.rs
    - biscuit-terminal/lib/tests/level2/level2_windows_console_query.rs
    - biscuit-visualized/src/src/mermaid/config.rs
    - biscuit-visualized/src/src/mermaid/color.rs
    - biscuit-visualized/src/src/mermaid/palette.rs
    - biscuit-visualized/src/src/mermaid/design.rs
    - biscuit-visualized/src/src/mermaid/accents.rs
    - biscuit-visualized/src/src/mermaid/contrast.rs
    - biscuit-visualized/src/src/tests/palette_tests.rs
    - biscuit-visualized/src/src/tests/theme_tests.rs
    - biscuit-visualized/features/2026-10-21-merman/gallery/Cargo.toml
    - biscuit-visualized/features/2026-10-21-merman/gallery/Cargo.lock
    - biscuit-visualized/features/2026-10-21-merman/gallery/src/main.rs
    - biscuit-visualized/justfile
    - biscuit-visualized/src/src/tests/golden_tests.rs
    - biscuit-visualized/src/src/tests/contrast_corpus_tests.rs
    - biscuit-visualized/src/src/tests/corpus_measure_tests.rs
    - biscuit-visualized/bench/Cargo.toml
    - biscuit-visualized/bench/Cargo.lock
    - biscuit-visualized/bench/baseline.json
    - biscuit-visualized/bench/src/main.rs
    - biscuit-visualized/bench/src/baseline.rs
    - biscuit-visualized/bench/src/corpus.rs
    - biscuit-visualized/bench/src/stats.rs
    - biscuit-visualized/bench/tests/wiring.rs
    - biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs
documentation:
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/FINDINGS.md
    - biscuit-visualized/features/2026-10-21-merman/spikes/s1-comparison/windows-leg.md
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/pr-01-gitgraph-commit-spacing.md
    - biscuit-visualized/src/assets/fonts/inter/README.md
    - biscuit-visualized/src/assets/fonts/inter/OFL.txt
    - biscuit-visualized/src/assets/fonts/noto-emoji/README.md
    - biscuit-visualized/src/assets/fonts/noto-emoji/OFL.txt
    - biscuit-visualized/src/src/tests/fixtures/merman/README.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/01-default-measurement-clips-labels.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/02-wrapped-labels-lose-line-breaks.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/03-neo-dark-dividers-invisible.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/04-normalize-backslash-n-like-br.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/05-backslash-n-pie-gitgraph-block.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/06-er-attribute-comment-br-clips.md
    - biscuit-visualized/src/src/tests/fixtures/merman/gitgraph/README.md
    - biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-macos.html
    - biscuit-visualized/features/2026-10-21-merman/gallery/themed-gallery-linux.html
    - biscuit-visualized/bench/README.md
    - docs/dependencies.md
    - biscuit-visualized/docs/dependencies.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-visualized/docs/components/graph_expression.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-terminal/docs/discovery/color.md
    - biscuit-terminal/docs/discovery/terminal-queries.md
    - biscuit-visualized/README.md
    - biscuit-visualized/docs/components/git_graph.md
    - biscuit-visualized/docs/mermaid-gitgraph.md
    - biscuit-visualized/features/2026-10-21-merman/spec.md
    - worktree/docs/git-graph.md
    - darkmatter/docs/rendering/mermaid.md
    - biscuit-terminal/cli/README.md
    - biscuit-visualized/features/2026-10-21-merman/plan.md
    - biscuit-visualized/features/2026-10-21-merman/implementation-log.md
    - .claude/skills/os/build-hosts.md
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
    - .claude/skills/biscuit-visualized/rasterization.md
    - .claude/skills/biscuit-terminal/discovery.md
    - .claude/skills/biscuit-visualized/diagram-components.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
    - .claude/skills/worktree/git-graph.md
    - .claude/skills/biscuit-terminal/diagrams.md
    - worktree/docs/performance-testing.md
completed_phase: 7
implemented: true
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

### Checkpoint 4 re-run (2026-10-23)

The Phase 4 run stopped at this checkpoint for the key-order ruling. Ken
ruled the same day that a finding the remaining work does not depend on is
flagged and the run continues; the spec records it as D73 (amending D72)
and the plan's working rules now say so ("Findings that need a ruling do not
stop the run"). The checkpoint's first item was
re-run on the tree after the `main` merge (`d545a6256`), serially, with
`--no-fail-fast`:

| Package | `just test` | `just lint` |
| --- | --- | --- |
| `biscuit-visualized` | 551 passed, 3 skipped | clean |
| `biscuit-terminal` | 3881 passed, 55 skipped | clean |
| `sniff` | 3463 passed, 33 skipped | clean |
| `worktree` | 1598 passed, 15 skipped | clean |
| `darkmatter` | 10560 passed, **45 failed**, 10 skipped | clean |

The 45 Darkmatter failures are the same set as in "Blocking finding" above
(`as_json5_uses_the_formatter_object_idiom`,
`parsed_frontmatter_retains_nested_mapping_order_as_metadata`,
`schema_validate_legacy_json_output_is_byte_identical`,
`benchmark_manifest_matches_recorded_identities`,
`conversion_snapshot_covers_valid_and_invalid_metadata`, and 39
`schemas_convert_snapshots::*`); nothing new failed. The item stays
**open**, with `human_review: true`. Phases 5 and 6 do not depend on it:
Phase 5 gates only biscuit-visualized and biscuit-terminal, and no Phase 6
task touches Darkmatter's JSON output.

### Key-order ruling (Ken, 2026-10-23)

The review item raised in "Blocking finding" (options A, B, C) is ruled, and
`human_review` is cleared:

- Author key order is respected at every depth; insertion order is the
  contract (closest to option C, but Darkmatter will enable
  `preserve_order` itself rather than inherit it from Merman).
- Merging keeps the base document's order and appends new keys in the
  merged document's order.
- The default frontmatter hash was always meant to be Markdown-aware and
  order-independent; its dependence on nested key order is a bug. It is
  defined in Darkmatter (`hash_frontmatter_map`, which sorts only top-level
  keys); the order-independent JSON hashing moves into `biscuit-hash`.

The work is specified as the separate fix
`2026-10-23-frontmatter-key-order` (darkmatter area). It restores `main`'s
frontmatter hash values exactly, because `main`'s nested maps were sorted by
the same byte order. Checkpoint 4's open item closes when that fix lands and
darkmatter `just test` is re-run green. The Merman fork is unchanged
(option B rejected).

## Phase 5

Implemented by one agent without subagents (the work was one tightly coupled
design loop: probe Merman, render the gallery, fix, repeat). Throwaway probe
tests (`src/tests/zz_probe_tests.rs`) answered the design questions below and
were deleted before the gates.

### What shipped

- **`biscuit-terminal`: OSC 4 on native Windows (D46, D71, R-P1, AC-P6).**
  `ASKS_PALETTE` is gone; `palette_query_wanted()` is `color_query_wanted()`
  on every platform, so the sixteen OSC 4 queries ride the existing console
  backend (`tty_query/windows.rs`) with no Windows-specific query code. The
  unit test `the_palette_query_follows_the_color_gate_on_every_platform`
  replaces `only_unix_asks_for_the_palette`.
- **`mermaid::Rgb`, `mermaid::Palette`, `mermaid::TerminalColors`** (new,
  public; `color.rs`, `palette.rs`). Fixed palettes per `MermaidTheme` from
  Mermaid.js 12.2.0's themes (`fixed`), `Palette::new(bg, fg, accents)`,
  `Palette::for_terminal(&TerminalColors, is_dark)` (ANSI 1–6 → accents;
  unanswered slot → background-derived accent; no foreground → black/white;
  no background → `fixed(for_color_mode)`), `TerminalColors::detect()`
  (feature `terminal`; all `None` when stdout is not a terminal or `CI` is
  set). Every constructor runs the contrast passes (text 4.5:1 on background,
  surfaces, and tints; accents/border/line 3:1; D45).
- **`MermaidDiagram::{with_palette, palette}`**; `with_theme` resets the
  palette to the theme's fixed one. The terminal component's `new` uses the
  terminal palette (`terminal_palette()`); `with_theme`/`with_palette`
  override it.
- **Per-family compiled designs** (`design.rs`): role table filtered through
  `describe_theme_support(family, StandaloneSvg, …)`; compiled with
  `DiagramThemeCompiler` and applied with `RenderRequest::with_theme`. The
  family comes from Merman's own `preprocess_diagram` + `detect_type`, read
  before the cache check.
- **Site config** is now `{"theme": "base", "themeVariables": {"fontFamily":
  …}}` merged with `MermaidConfig`: **no code maps a `MermaidTheme` to a
  Merman built-in theme** (AC-C15; `each_theme_renders_through_its_compiled_palette_not_a_built_in_theme`
  replaces the cutover test `each_theme_renders_through_its_merman_built_in_theme`).
- **Accent post-process** (`accents.rs`) and **contrast pass**
  (`contrast.rs`), after `postprocess` and before `html_static` (which still
  runs last, per Phase 4's note).
- **Cache key:** `design` now hashes site config + design JSON + the
  palette's `Debug` (every color, R-P6). `MERMAID_BACKEND` →
  `merman@6e6b1e6554e8+bv2`.
- **Themed gallery** (`gallery/`, a non-member crate like the spikes):
  `themed-gallery-macos.html` and `themed-gallery-linux.html` (built on
  `build-linux` at the tested tree), every S1 corpus family × Dark, Default,
  Forest, Neutral, Tokyo-Night ANSI terminal, background-only terminal.

### Merman behavior found (the design rests on these)

- `describe_theme_support` reports `Unconditional`/`Conditional`; probes
  that matched the string `"supported"` would miss `unconditional`.
- **Explicit site-config `themeVariables` win over compiled styles**: even
  `darkMode` + `background` alone erased the flowchart node stroke, label
  fill, and edge stroke; `primaryColor` erased the ordinal palette. Pie,
  journey, timeline, kanban, radar, and state lost their ordinal palettes
  the same way. Only `fontFamily` is harmless.
- **Site-config `themeVariables` also win over the source's own
  `%%{init}%%`/frontmatter `themeVariables`** (the reverse of Mermaid.js).
- An author's `%%{init}%%` `primaryColor` is overridden by our compiled
  palette (D54's check: the compiled theme does override), while an author
  `lineColor` beats our edge stroke.
- Merman's `base` theme underneath is light: on a dark palette, every
  surface a family's compiled styles do not reach (treemap cells, kanban
  columns, quadrant and XY chart backgrounds, ER label boxes, state
  transition labels) stays light or keeps `#333` text.
- Mindmap labels are drawn in a fixed `#333` (`.section-N text`) that no
  compiled style reaches (`node-label` is rejected for mindmap).
- Kanban `task-label` fill is reported `conditional` but rejected at render
  (`unsupported-paint`, evidence `Residual`).
- Sequence actor paint is a stylesheet rule (`.actor{…}`), not inline;
  Merman writes the bottom row of actor boxes before the top row.
- Raster-safe labels are drawn once in a definitions group and placed with
  `<use href>`, so a label's text is not inside the shape it sits on.
- gitGraph's design with Mermaid label variables is `Residual` with a single
  `compatibility-unverified` diagnostic; every other family's design is
  `Verified` with no diagnostics; the flowchart circle's `unsupported-paint`
  (D62) is the only `unsupported-*`.

### Decided during implementation (flagged for review)

Each of these is in the log's `human_review_items` because several touch
the letter of R-T2/R-T3/AC-T1/R-T6 (D72's condition 2), so they are flagged
rather than taken as D72 "proceed" amendments; the spec is unchanged.

1. **Site palette variables for some families.** Families whose design has
   no ordinal palette and are not sequence/class (treemap, quadrant, ER,
   gantt, C4, block, architecture, requirement, packet, cynefin, …) also get
   the palette as `base` **root** theme variables in the **site config**
   (`wants_site_palette`); pie gets its three text-color variables and XY
   chart its `xyChart.*` background/axis colors (`family_site_variables`),
   each measured not to erase the design; packet, which reads its colors
   from its own top-level `packet` config (Merman ignores
   `themeVariables.packet`), gets `packet.blockFillColor` and its label
   colors (`family_site_config`), skipping any key the author's source sets. No `design.mermaid.variables`
   outside gitGraph (AC-T1 holds for the designs); this is the site-config
   channel `MermaidConfig` already used. Without it those families are
   unreadable on dark palettes.
2. **Node base fill where no ordinal palette exists.** `styles.node.fill` is
   the palette surface for families with no node ordinal palette (class,
   block, architecture); it is never set beside an ordinal palette (the S0
   surprise R-T2 guards against). Without it class boxes keep `base`'s
   cream `#fff4dd` under a dark palette.
3. **D54's yield is whole-theme, by color.** A source whose
   `%%{init}%%`/frontmatter sets a theme **color** (`sets_a_color`) gets no
   compiled theme; the palette goes into the site config as `base` root
   variables and the author's variables are copied on top (Merman's
   precedence is reversed, above). A named `theme` gets neither. Non-color
   variables (e.g. the corpus pie's `pieOuterStrokeWidth`) keep the compiled
   theme, since the designs set colors only. Per-key yielding inside a
   compiled theme is not possible: compiled styles are not keyed by Mermaid
   variable.
4. **Mindmap fills are lightened accents**, not tints: Merman's fixed `#333`
   label must reach 4.5:1 on them.
5. **Sequence accents key on the actor box's `name` attribute** (ordered by
   `x`), not on an inline palette color, because actor paint is stylesheet
   only. It is neither a class name nor an id (R-T6's letter), and it only
   runs when the palette's first accent (the design's actor stroke) is in
   the SVG. The symbol participants (boundary, control, database, …) keep
   that first accent.
6. **The contrast pass also recolors palette text on author fills** (the
   corpus requirement's `style test_entity fill:#f9f`), but never an
   author's text color on an author's fill.
7. **No CSS is injected** (R-T7 holds vacuously); AC-T2's test asserts that
   every selector of every `<style>` in two embedded diagrams is prefixed by
   its own SVG id (Merman's single stylesheet).
8. **Neutral's accents** were changed from Mermaid's grays where `#999`
   collided with the border after the contrast pass.
9. `MermaidTheme::as_str()` is kept as the theme's name only (the
   `biscuit-terminal` CLI's quadrant command prints it).

### AC-P6 Windows run (build-win-native)

- `just cross-check biscuit-terminal --os windows --features terminal-tests
  --test level2 palette measured_colors cell_size` at tree `5417b19ff`
  (native mode), after building `discovery_probe` in the rig clone at the
  tested revision (`os` skill: the first narrowed run used the stale probe
  and failed at the `probe.exists()` assert, as documented):
  **6 passed** — `the_ansi_palette_reaches_a_windows_program` (OSC 4 queries
  forwarded verbatim, slots 1–6 answered and read, others `None`, stable),
  `an_unanswered_palette_query_leaves_every_slot_unknown` (all 16 `None`,
  first read bounded under 5 s), and the four existing console tests.
- `cross-check biscuit-visualized --os windows --all-features --lib
  palette_tests theme_tests` (tree `a81cb0901`): **53 passed**, including
  `a_background_without_ansi_colors_gives_background_derived_accents`
  (known background, no ANSI reply). The archive mode refused (no
  `windows-latest` build for this non-gating package), so native mode.
- Linux (`build-linux`, tree `08290bead`, the final code): the same filter,
  **55 passed** (two tests were added after the Windows run:
  `labels_on_an_author_fill_are_made_readable` and
  `packet_colors_follow_the_palette_and_the_author_keeps_theirs`). Both
  galleries were generated from this tree.

### Requirement-to-test mapping

| Requirement | Test(s) (file) |
| --- | --- |
| R-T1, R-T4, D17 | `fixed_palettes_start_from_the_mermaid_themes_of_the_same_name` (palette_tests) |
| R-T2, R-T3, R-T9, AC-T1 | `each_fixed_palette_design_holds_only_supported_facets`, `the_only_font_stack_set_ends_in_sans_serif`, `site_palette_variables_go_only_where_they_cannot_erase_the_design` (theme_tests) |
| AC-C15 | `each_theme_renders_through_its_compiled_palette_not_a_built_in_theme` (mermaid_tests) |
| AC-T6, R-V4, D62 | `evidence_<family>` × 22 + `every_family_has_an_evidence_test` (guard), `the_circles_known_false_unsupported_paint_is_the_only_allowed_diagnostic` (theme_tests) |
| AC-T7, R-V5 | `guard_a_static_node_fill_beats_the_ordinal_palette`, `guard_an_unsupported_rule_erases_a_supported_one`, `guard_mermaid_variables_override_the_palette_and_downgrade_sequence` |
| AC-T3, R-T5, R-T6 | `flowchart_nodes_get_rotating_accents_and_author_styles_are_kept` (all four fixed palettes), `classes_take_accents_in_document_order`, `sequence_participants_take_one_accent_each`, `mindmap_accents_come_from_the_compiled_theme`, `the_accent_post_process_touches_no_paint_the_palette_did_not_emit` |
| AC-T8, D54 | `an_author_init_color_wins_over_the_compiled_theme` (directive and frontmatter), `only_author_colors_or_a_named_theme_displace_the_compiled_theme` (width-only variable, named theme, control) |
| R-T8, D45 | `every_palette_meets_the_contrast_floors` (9 palettes incl. hostile ones), `text_on_a_known_label_background_is_made_readable`, `a_used_label_is_judged_against_its_placement`, `author_paint_and_unknown_fills_are_left_alone`, `labels_on_an_author_fill_are_made_readable` (the original failing input; it and `a_used_label…` fail with the closed-group propagation removed, verified) |
| R-T7, AC-T2 | `two_diagrams_in_one_document_keep_their_own_styles` |
| AC-P3, R-P3–R-P5 | `a_full_terminal_reply_gives_ansi_accents`, `a_background_without_ansi_colors_gives_background_derived_accents`, `a_partial_ansi_reply_fills_only_the_missing_slots_from_the_background`, `no_background_gives_the_fixed_palette_for_the_color_mode`, `an_unreadable_terminal_foreground_is_corrected` |
| AC-P5, R-P7 (this crate's side) | `detection_reports_nothing_when_stdout_is_not_a_terminal_or_in_ci` |
| AC-P4, R-P6 | `one_palette_color_changes_the_cache_key` (one accent, one background channel; it failed before the palette joined the key, because a 1-unit accent change rounds to the same tint) |
| R-P1, AC-P6 | `the_palette_query_follows_the_color_gate_on_every_platform` (biscuit-terminal L1), `the_ansi_palette_reaches_a_windows_program`, `an_unanswered_palette_query_leaves_every_slot_unknown` (biscuit-terminal L2, Windows) |
| Colors/CSS parsing | `hex_colors_parse_in_both_lengths_and_reject_everything_else`, `contrast_follows_wcag_2`, `css_colors_in_merman_spellings_parse` |

Updated assertions: `backgrounds_paint_the_theme_a_solid_color_or_nothing`,
`mermaid_theme_changes_svg_output`, `mermaid_transparent_background_changes_svg_output`
(palette backgrounds are written as 6-digit hex: `#333333`, `#ffffff`),
`the_key_follows_the_post_process_version` (`+bv3` as the "other" id),
`cache_tests` and `mermaid_cache_key_tests` backend literal (`+bv2`).

Input robustness: no new loader of a file format or configuration; the
author-config read uses Merman's own preprocessor (its outcomes are Merman's),
and `sets_a_color` is covered by the color/width/named-theme cases above.

### Gates

| Package | `just test` | `just lint` | `check-tier-coverage` |
| --- | --- | --- | --- |
| `biscuit-visualized` | 606 passed, 3 skipped | clean | 0 stranded |
| `biscuit-terminal` | 3881 passed, 55 skipped | clean | 0 stranded |

Doctests: `cargo test --doc -p biscuit-visualized --all-features` 36 passed.
Both packages' `just test` and `just lint` were re-run after the last code
change (the packet site config, with its test
`packet_colors_follow_the_palette_and_the_author_keeps_theirs`); the figures
above are that final run.

Dependents (narrowed, serial):

- `darkmatter`: `-E 'test(/mermaid|svg_sanitizer|diagram/)'` **106
  passed**. (The 45 key-order failures are the separate fix, not in scope.)
- `worktree-cli`: `-E 'test(/git_graph|graph/)'` 82 passed, **2 failed**:
  `level2_graph_height_cap_elides_lanes_in_a_short_kitty_window` and
  `level2_graph_restores_lane_density_in_kitty` (L2 Kitty tests pulled in by
  the name filter; `just test` does not run them). **Pre-existing:** both
  also fail on a clean `HEAD` checkout without this phase's changes
  (`git worktree add --detach /tmp/wt-head-check HEAD`): there at
  `level2_graph_in_kitty.rs:727` and `:894`, here at `:836` ("5 hidden lanes
  of 5") and `:894` ("31 rows exceed half of 60"). This matches Phase 4's
  note that a three-lane view loses a lane to the half-height cap.
- `sniff-cli`, `biscuit-terminal-cli`: covered by `biscuit-terminal`'s
  `just test` (CLI included) / no Mermaid styling asserted.

Load-sensitive: with the host's load average at about 21 (other sessions),
many Mermaid render tests reported 5–6.5 s in a concurrent run; alone they
take 0.25–1.1 s, and the serial `just test` run reported 2 slow tests. The
first run of the 22-family evidence test hit the 2 s budget once
(`sequence`, "deadline_exceeded" during layout); it was split into one test
per family with a 60 s budget (the budget is not under test).

### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py` → `packages`: `biscuit-terminal`,
`biscuit-visualized`, `test-toolkit`, `darkmatter`; `change_class`:
`package`. The gates were run. (`darkmatter` comes from the untracked
`darkmatter/fixes/2026-10-23-frontmatter-key-order/`, which is not this
phase's; `.claude/skills/worktree/*` and `prompts/plan.md` were already
modified by other work and are not this phase's either.)

### Open

- **AC-T5 (Ken's gallery sign-off): open, planned stop before Phase 6.**
- Known gaps for the gallery review: sequence symbol participants
  (boundary, control, database, …) keep the first accent; agentflow,
  usecase, and wardley have no compiled-theme support at all (empty designs,
  `NotApplicable`) and take the palette only through the site variables;
  the second sequence corpus case's light `rect` band is the author's.
  Phase 6's automated contrast test (AC-T4) is the full check of R-T8.

## Phase 6

Orchestrated by one agent with three parallel subagents (benchmark crate;
AC-M1/AC-M2 over the corpus; AC-T4 contrast oracle and its fixes). The
golden suite, render seam, justfile, gating edit, docs, and cross-OS runs
were done directly.

**Commit note:** partway through this phase the early golden work
(`render_in` seam, `golden_tests.rs`, first goldens, plus the then-pending
plan/spec/log edits) was committed as `0630606c4` ("fix(biscuit-visualized):
render Mermaid PNGs with measured fonts"), not by this agent. The phase's
file lists in the frontmatter therefore include files in that commit as well
as the uncommitted ones.

### Golden corpus (R-V1, R-V2, R-V11, AC-V1, AC-V3, AC-C5, AC-C7)

- **Fixtures:** the 33 family fixtures already in
  `src/src/tests/fixtures/merman/families/` (Phase 3, one per family of
  Merman's registry, licenses in `families/LICENSES/`) are the golden inputs.
  No new fixture was copied; no fonts; the fixtures folder is about 50 KB.
  Nothing references Mermaid.js 11.x renders (goldens snapshot our own
  output, D13).
- **Seam:** `MermaidDiagram::render` now delegates to the crate-private
  `render_in(chain, request)`, and the PNG path rasterizes with that same
  chain (`raster::png::rasterize_svg_for_request_with`); before, the
  rasterizer always took `default_chain()`. Behavior-neutral for `render`
  (same chain object), so no backend bump for it.
- `src/src/tests/golden_tests.rs`: per family, `svg_{dark,default,forest,neutral}`
  (`insta`, `src/src/tests/goldens/svg/<fixture>_<palette>.snap`) and
  `png_{dark,default}` (`src/src/tests/goldens/png/<fixture>_<palette>.png`,
  AE share ≤ 0.001, RMSE ≤ 0.005, size must match); six `derived_golden_*`
  tests (flowchart, sequence, gitgraph × `terminal_ansi` Tokyo Night /
  `terminal_background` `#1a1b26`). 33 × 6 + 6 = 204 golden tests, 138 SVG
  snapshots, 66 PNGs, about 5 MB.
- Every golden pins its palette and renders with
  `FontChain::injected(&[], &[], Some("Inter"))`: `MERMAID_FONT=Inter` with
  no installed fonts. The SVG's `bv-<16 hex>` diagram id comes from the cache
  key (font set hash; Linux also bundles Noto Emoji) and is normalized to
  `bv-ID`.
- Guards: `every_smoke_family_has_goldens` (golden list = smoke list, which
  `the_smoke_list_is_mermans_family_registry` ties to Merman's registry),
  `the_six_derived_goldens_cover_three_families_and_two_palettes`,
  `every_golden_input_code_point_is_mapped_by_inter` (every non-control code
  point of every golden input, in both Inter faces: passes, no fixture edit
  needed), `normalize_ids_replaces_only_sixteen_hex_digits`,
  `png_difference_counts_differing_pixels_and_channel_error`.
- AC-C7: each `png_*` golden asserts the SVG handed to the rasterizer
  (`take_rasterized_svgs`) has no `foreignObject`; each SVG golden asserts
  the same of its SVG.
- PNG goldens use `scale: 1` (the default request is 2×) to keep them small
  and fast; slowest golden test 1.4 s (timeline PNG, 2390 × 687).

### Merman-only families (D16, AC-C5)

Registry confirmed at the pinned rev by the existing guard
`the_smoke_list_is_mermans_family_registry` (33 rendered families + `error`
and `zenuml` excluded with reasons). Each has its smoke tests
(`mermaid_family_tests`) and four fixed-palette goldens. The docs list of
Merman-only families is Phase 7's R-D1 task.

### Corpus contrast (R-V3, D45, AC-T4)

`src/src/tests/contrast_corpus_tests.rs`: an oracle independent of the
production pass. It reads each stored `.snap` (path joined to
`CARGO_MANIFEST_DIR`, so a golden change schedules these tests), parses it
with usvg on the golden chain (stylesheets, `<use>`, transforms resolved),
takes each text's resolved fill times group opacity and its glyph-ink box,
rasterizes the SVG with every `<text>` removed, and judges each text against
the most frequent pixel in its box. One test per family (`<family>_text_is_readable`,
fixed palettes plus the derived ones), `every_svg_golden_has_a_contrast_check`,
and four unit tests of the oracle (a stylesheet-painted label placed through
`<use>` and a translucent label must fail; a readable one must pass).

It first found **160 failing texts in 12 families**. Fixed in production
(`mermaid/design.rs`, `mermaid/contrast.rs`; `MERMAID_BACKEND` → `+bv3`):

| Family | Cause | Fix |
| --- | --- | --- |
| c4 (all) | compiled `cluster` fill reached C4 boundaries, drawn after their contents: nodes were hidden in the PNGs | no `cluster` fill for c4 (Mermaid's boundary is unfilled) |
| c4 dark | Merman's fixed `#444` boundary/relationship text; white on `#999` external elements | `c4.boundary_font_color`, `relationship_text_color` = palette text; `external_*_font_color` = best of text/black/white |
| treeView | fixed `descriptionColor` `#6a9955` | green accent adjusted to 4.5:1 |
| eventmodeling dark | light default lane/box fills | `em*` site variables from surfaces, tints, accents |
| kanban dark | stylesheet-painted light `cScale` columns | `cScale0..11` = second surface, 10 points darker (Merman lightens by 10) |
| timeline dark | fixed `#333` text on dark sections | section accents adjusted until `#333` reads (the mindmap approach) |
| cynefin dark | default domain colors | `cynefin.*Bg` = tints |
| gantt dark | `base`'s white alternate section at 0.2 opacity | `altSectionBkgColor` = background |
| treemap default | `cScale` from the surface, stacked translucently | `cScale0..11` = tints |
| sankey default | full accents under half-transparent links | node colors moved toward the background until text reads |
| gitGraph dark, both terminals | half-transparent commit label background over branch lines | label background moved toward black/white until it reads over every branch color |
| venn dark | derived near-background set color | `venn1..8` = accents |

The contrast pass now blends translucent fills over what they cover, and a
closed node group's fill reaches only its next siblings (it had spread to the
rest of the diagram, recoloring C4 boundary and venn text). Goldens of c4,
cynefin, eventmodeling, gantt, gitgraph (incl. both terminals), kanban,
sankey, timeline, treemap, treeView, and venn were regenerated; every other
golden stayed byte-identical. `docs/merman.md` ("Readable text", "How a
palette reaches Merman") updated.

**17 allow-listed failures** (each must still fail, so a fix removes it):
the c4 person's author `$fontColor="red"` on author `$bgColor="grey"` (4
palettes); c4 relationships' author `$textColor` on our background; venn
"React" author `color:red` on author `fill:#ff6b6b` (4 palettes); and
**sequence × dark/terminal_ansi/terminal_background: our message text on the
author's `box LightGrey`** (1.07–1.50). The last is ours to fix but the pass
cannot reach it (stylesheet text, presentation-attribute box, messages
outside the box's group); see `decisions_for_review`.

### Corpus measurement (AC-M1, AC-M2)

`src/src/tests/corpus_measure_tests.rs`: per family,
`drawn_widths_match_usvg` (AC-M1, with a per-family minimum label count) and
`labels_fit_their_boxes` (AC-M2), over the family's Default golden SVG on
the golden chain; guards `corpus_measure_families_are_the_golden_corpus`
(macro list = `GOLDEN_SOURCES`, minimums sum ≥ 450) and
`corpus_measure_allow_lists_name_golden_fixtures`. **483 labels compared;
worst width difference 0.0000153 px** (ishikawa), no glyph or face mismatch.
Overflows, none measurer-attributable:

- swimlane edge label "Needs code change": Merman wraps it at 120 px and
  sizes a two-line box but draws one line (a flowchart edge label with the
  same text is not wrapped); also overflows with Merman's own measurer.
  `MERMAN_CORPUS_OVERFLOWS`.
- radar axis label "Intelligence": radar's fixed 700 × 700 canvas; the label
  starts at x ≈ −17.5. `MERMAN_CORPUS_OVERFLOWS`.
- radar legend "Naruto", "Sasuke", "Ninja": no box; the survey picks the
  graticule circle. `NO_BOX_LABELS` (axis labels stay surveyed).

Both Merman defects are candidates for upstream issues (not filed).

### Benchmark (D12, D35, D44, D67, R-V6, AC-V2)

`biscuit-visualized/bench/` (non-member crate, empty `[workspace]`, own
`Cargo.lock` with the same Merman rev): warm (`render` to PNG, default
request, `Default` palette, after one untimed render) and cold (the binary
spawns itself, `__one-shot <family>`, spawn → exit), median and nearest-rank
p95 per family, 20 iterations each; every iteration gets a fresh
`BISCUIT_VISUALIZED_CACHE_DIR` checked empty before the timer, and a cache
hit (or a render that wrote nothing there) is an error. Flags: warm median
> 50 ms, cold median ≥ 500 ms, warm or cold median > 25% over the baseline,
no baseline entry; exit 1 names family and rule. `just bench` runs it
directly (`cd bench && cargo run --release --quiet -- "$@"`; `*args` without
a default, because `*args=""` passes an empty argument under
`positional-arguments`).

- Loader matrix (`bench/src/baseline.rs`, 36 tests): control (committed
  baseline), `median_ms`/`p95_ms` × absent, null, string, `[]`, `{}`,
  negative, zero, bare `NaN` (parse error), `"NaN"`; `p95 < median`; family
  missing → `NoBaseline` state; `"families": {}` valid (all no-baseline);
  empty document; no `families`; family not an object; no `cold`; unknown
  family / field; trailing garbage; duplicate family / field / top-level key
  (custom visitor); missing file. Every invalid shape is an error naming the
  JSON path; nothing compared or overwritten.
- Wiring (`bench/tests/wiring.rs`, spawns the binary with
  `--families flowchart --iterations 3 --reference`): control passes;
  doctored baseline flags ("slower than the baseline"); doctored limit
  `--warm-limit-ms 0.001` flags; no-baseline flags on the reference host;
  off-host a doctored baseline only prints "would flag"; a duplicate-key
  baseline exits 2 and the file is unchanged. 46/46 pass on macOS and on
  build-linux (`cargo nextest run --locked`).
- **Linux reference baseline** (build-linux, tree shipped by tar,
  `--reference --write-baseline`, scratch dir removed): warm medians
  2.4 ms (info) – 84.3 ms (timeline); cold medians 11.6–98.0 ms. **Over the
  50 ms warm limit: sequence 56.4 ms, timeline 84.3 ms**, so `just bench`
  on build-linux exits 1 today (eventmodeling 45.3 and kanban 42.1 are
  close). A second run auto-enforced via the hostname and landed within
  −1.3% to +7.2% of the baseline. macOS (informational, busy host): warm
  6–71 ms, cold 265–709 ms.
- `just ci-local --plan` shows no cell for the bench crate and no
  `biscuit-visualized/*/check` cell. A change to `bench/src/*.rs` alone
  selects only the repo-wide `test-toolkit` archive-path-guard lint
  (`affected_scope.py`), the same as the gallery and spike crates.
- The baseline was recorded while the contrast agent was changing designs
  (both ran concurrently); the design changes are colors only.

### CI: canonical recipes and gating (D42, R-V12, AC-V4)

- Justfile: the canonical set already existed except `lint-fix` (added,
  `just lint || just _fix_lint`) and `bench` (now real); `docs` now passes
  `--all-features`. `just check-tier-coverage biscuit-visualized`: 0
  stranded, no stubbed tier listed.
- **Before the gating edit**, `just ci-local --plan` planned **no cell at
  all** for `biscuit-visualized` (the `promotion-pending` exclusion keeps it
  out of the plan), so "L1 cells planned on each environment" could only be
  observed after the edit.
- `src/Cargo.toml`: `gates = true`; `exclusion-class`, `owner`, `reason`,
  `expiry` removed; comment updated. `source-inputs` unchanged: the fixtures
  are read with `include_str!`, the PNG goldens and the stored SVG goldens
  through literals joined to `CARGO_MANIFEST_DIR` (both CI-visible), and
  `source-inputs` only accepts another package's source.
- **After**, `just ci-local --plan`:

  ```text
  biscuit-visualized/ubuntu-latest/lint  execute  ci  pending  — lint gate for biscuit-visualized, hosted on ubuntu-latest
  biscuit-visualized/ubuntu-latest/L1  execute  ci  pending  — biscuit-visualized declares the L1 tier; ubuntu-latest is a required environment
  biscuit-visualized/windows-latest/L1  execute  ci  pending  — biscuit-visualized declares the L1 tier; windows-latest is a required environment
  biscuit-visualized/macos-latest/L1  execute  ci  pending  — biscuit-visualized declares the L1 tier; macos-latest is a required environment
  biscuit-visualized/wsl2-ubuntu/L1  execute  ci  pending  — biscuit-visualized declares the L1 tier; wsl2-ubuntu is a required environment
  ```

### Galleries

Regenerated after the contrast fixes, so the sign-off item shows the shipped
look: `gallery/themed-gallery-macos.html` locally and
`gallery/themed-gallery-linux.html` on build-linux (tree shipped by tar,
scratch removed).

### Docs and skill

- `docs/merman.md`: new "Golden tests" (corpus table, pinned palette/font,
  tolerance, regeneration, guards, "CI") and "Benchmark" sections. The plan's
  "Docs (shipped)" item was already checked, but `merman.md` had no such
  content; it was written now.
- `.claude/skills/biscuit-visualized/SKILL.md`: golden corpus, gating, bench
  notes; `+bv3` (with `artifacts-caching.md`).

### Cross-OS goldens and re-measure with Inter (R-C14, AC-S8, R-V12)

`just cross-check biscuit-visualized --os all --all-features --lib
--no-capture golden_tests contrast_corpus_tests corpus_measure_tests` on
the final tree (native mode, because of `--all-features`; no receipt):

| Host | Result | PNG goldens vs the macOS-generated ones |
| --- | --- | --- |
| macOS (local) | 315 passed | generated here |
| Linux (build-linux) | 315 passed | 66/66 AE share 0, RMSE 0 |
| native Windows (build-win-native) | 315 passed | 66/66 AE share 0, RMSE 0 |
| WSL2 (build-win) | 315 passed | 66/66 AE share 0, RMSE 0 |

Per host pair with `MERMAID_FONT=Inter` (the goldens' chain): macOS–Linux,
macOS–Windows, Linux–Windows (both equal to macOS, so equal to each other),
and macOS–WSL2 are all **pixel-identical: AE share 0.000000, RMSE 0.000000**
on every one of the 66 PNGs. The Phase 1 host-font record (Linux AE share
0.27%/0.74% from DejaVu substitution) is the prior one; with the bundled
Inter, the D42 tolerance is not even approached. An earlier Linux run, before
the contrast fixes, was identical too.

A first `--os all` attempt with an `-E '…'` expression died locally on a
shell-quoting error while its Windows leg kept running remotely and held the
clone lock; the rerun waited for it, then ran. Use plain name filters with
`cross-check`, not `-E`.

### Requirement-to-test mapping

| Requirement | Test(s) |
| --- | --- |
| R-V1, AC-V1 (SVG, every family × 4 palettes) | `golden_tests::golden_<family>::svg_{dark,default,forest,neutral}` (132) |
| R-V1, AC-V1, D42 (PNG, Dark/Default, tolerance) | `golden_tests::golden_<family>::png_{dark,default}` (66), `png_difference_counts_differing_pixels_and_channel_error` |
| AC-V1 (Inter maps every code point) | `every_golden_input_code_point_is_mapped_by_inter` |
| R-V11, AC-V3, D47 | `derived_golden_{flowchart,sequence,gitgraph}_{ansi,background}`, `the_six_derived_goldens_cover_three_families_and_two_palettes` |
| R-V2 (pinned palette and font) | every golden: `with_palette(…)` + `golden_chain()`; `normalize_ids_replaces_only_sixteen_hex_digits` |
| AC-C5, D16, D27 | `mermaid_family_tests` smoke tests + `the_smoke_list_is_mermans_family_registry`; `every_smoke_family_has_goldens` |
| AC-C7 | every `png_*` golden (rasterized SVG) and `svg_*` golden; smoke tests |
| AC-T4, R-V3, D45 | `contrast_corpus_tests::<family>_text_is_readable` (33), `every_svg_golden_has_a_contrast_check`, oracle unit tests (`strip_text_removes_every_text_element_and_nothing_else`, `snapshot_body_skips_the_insta_header`, `dominant_color_is_the_most_frequent_pixel_in_the_box`, `the_oracle_fails_a_low_contrast_label_and_passes_a_readable_one`) |
| AC-M1 | `corpus_measure_tests::<family>::drawn_widths_match_usvg` (33) |
| AC-M2 | `corpus_measure_tests::<family>::labels_fit_their_boxes` (33), `corpus_measure_families_are_the_golden_corpus`, `corpus_measure_allow_lists_name_golden_fixtures` |
| AC-V2, R-V6 (local only, never CI) | `bench/src/baseline.rs` loader matrix (36), `bench/tests/wiring.rs` (control, doctored baseline, doctored limit, no-baseline, off-host, duplicate key) |
| AC-V4, R-V12, D42 | `just ci-local --plan` record above; `just check-tier-coverage` 0 stranded |
| AC-S8, R-C14 | cross-OS table above |

Input robustness: the only new configuration loader is the benchmark's
baseline JSON (matrix above, in-process; the binary spawned only for
wiring). Its tests live in the non-member crate and so never run in CI, by
D67's design.

### Gates

| Check | Result |
| --- | --- |
| `just lint` (biscuit-visualized) | clean |
| `just test` (biscuit-visualized) | **921 passed, 3 skipped** (74 s; host load average 15–18 from other sessions; 145 tests reported SLOW > 5 s, all passing, slowest 6.9 s, all pre-existing render tests; slowest new test 1.4 s) |
| `just check-tier-coverage biscuit-visualized` | 0 stranded |
| bench crate (`cargo nextest run` in `bench/`) | 46 passed (macOS and build-linux) |
| cross-OS (Linux, native Windows, WSL2) | 315/315 each |
| darkmatter `-E 'test(/mermaid\|svg_sanitizer\|diagram/)'` | 106 passed |
| worktree-cli `test(/git_graph\|graph/)` & L1 filter | 85 passed |

The first `just test` invocation reported only 318 tests (a run that started
while another session's cargo build was in flight); an immediate rerun ran
the full 921, and that is the result above. `sniff-cli` and
`biscuit-terminal-cli` assert no Mermaid styling (as in Phase 5).

### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py` → `packages: []`, `change_class:
documentation`, because this phase's code had already been committed
(`0630606c4`, `96e554b61`, `8f081510e`, `bf8e0665c`) and only this log was
uncommitted. The gates were run regardless (above); over the phase's own
files the selected package is `biscuit-visualized`.

## Phase 7

Orchestrated by one agent: R-D1 (`docs/merman.md`) and R-D3 done directly;
R-D2, R-D5, the R-D4 audit of the other touched documents, and the
comment-drift / clean-room / matrix-smell checks by parallel subagents; the
final gates run once, by the orchestrator, after all subagents finished.

### R-D1: `biscuit-visualized/docs/merman.md` (AC-D1)

- [x] Merman overview: new "What Merman is" section (pure-Rust Mermaid,
  models Mermaid.js 12.2.0, pre-1.0, pinned git dependency) with a
  "Diagram types" table.
- [x] Merman-only family list (D16, AC-C5): agentflow, cynefin,
  eventmodeling, info, ishikawa, railroad (+ `railroadAbnf`, `railroadEbnf`,
  `railroadPeg`), swimlane, treeView, usecase, venn, wardley; `error` and
  ZenUML listed as in the registry but not rendered. The split comes from
  spike S1's corpus script (`MERMAN_ONLY`), cross-checked against the 33
  fixtures in `src/src/tests/fixtures/merman/families/` (= the registry guard
  `the_smoke_list_is_mermans_family_registry`).
- [x] Pipeline as a Mermaid diagram with steps and owners (already present;
  owner table kept, symbols now linked).
- [x] Worked example from Mermaid text to a terminal image (new section,
  class diagram through every step, Tokyo Night terminal).
- [x] Design decisions with reasons (new "Design decisions" table): real-font
  measurement, font chain and `MERMAID_FONT`, bundled Inter, Linux-only Noto
  Emoji, gradient divider fix, `\n` rewrite, `commitSpacing` and the fork,
  cache key with resolved faces, Darkmatter static SVG, drop-shadow limits,
  speed limits, feature set with `math` off.
- [x] Rejected approaches with reasons (new table): `overflow:visible`,
  native measurement, greedy wrap, `parity` for HTML, text as paths.
- [x] Future considerations: upstream issues and pull request with when each
  workaround goes, return to stock Merman, font embedding in HTML, text-art
  fallback, region-aware contrast, speed, Merman churn and rev bumps.
- Added a "Golden tests" section (the Phase 6 handoff said one existed; it
  did not).
- Linked the important symbols to their source files (`MermaidDiagram`,
  `DEFAULT_TIME_BUDGET`, `MermaidError`, `MermaidTheme`, `Palette`,
  `TerminalColors::detect`, `fonts::measure`, `MERMAID_BACKEND`, `raster`,
  `RenderRequest`, `natural_size`, `TerminalImage` docs).

### R-D3: `docs/mermaid-gitgraph.md` (AC-D3)

**Decision: kept, not deleted.** It is not redundant: it is the only
description of the tag-spacing rules (which pairs count, the one-em gap, the
smallest spacing formula, author override, vertical graphs unspaced, cost and
cache identity, the geometry test API), which apply to every gitGraph caller
(`MermaidDiagram`, the `GitGraph` component, `bt git-graph`, Darkmatter).
`merman.md` keeps a one-paragraph summary that links to it, and
`components/git_graph.md` covers the component's own emission and fitting.
Folding it into `merman.md` would make that page longer without removing a
concept. Its merge section already says a merge of a branch with no commit of
its own renders (no "Mermaid.js refuses" claim); the only error it names is a
merge *into* a branch with no commit, which Mermaid.js also rejects. So no
link needed repointing.

### Upstream drafts check (AC-U1, AC-C2a)

- `upstream-issues/` holds the six issue drafts
  (`01-default-measurement-clips-labels.md` … `06-er-attribute-comment-br-clips.md`)
  and the pull request draft `pr-01-gitgraph-commit-spacing.md`.
- No issue or pull request URL has been reported by Ken in this run; none is
  recorded. No agent posted anything (this phase made no GitHub call other
  than the read-only `git ls-remote` below).
- `git ls-remote https://github.com/yankeeinlondon/merman.git
  refs/heads/gitgraph-commit-spacing` →
  `6e6b1e6554e8e75d468c1f991652c1c53a85bace`, equal to the pin in
  `biscuit-visualized/src/Cargo.toml` and every Merman crate's `source` in
  `Cargo.lock` (exact rev on the fork).

### Dependency graph (AC-C2, AC-M5)

- `cargo tree -i mermaid-rs-renderer --workspace --all-features` →
  `package ID specification mermaid-rs-renderer did not match any packages`.
- `cargo tree -d --workspace --all-features` lists no `usvg`/`resvg`
  duplicate; `Cargo.lock` has one `usvg` (0.47.0) and one `resvg` (0.47.0).

### R-D2: component docs (AC-D2)

Done by a subagent and checked against the code. `mermaid_diagram.md`,
`git_graph.md`, and `graph_expression.md` now cover the font chain,
`MERMAID_FONT`, the bundled Inter and Linux-only Noto Emoji, palettes (fixed
and terminal), OSC 4 with the native Windows case, the time budget, and the
Merman-only families (linked to `merman.md#diagram-types`). Symbols link to
their sources; there are new Mermaid flows for render-and-fallback and
palette choice. Drift found (code wins), each fixed in the doc:

- the family list lacked the Merman-only families;
- `with_title` was described as drawn, but it is only alt text and cache key;
- `with_scale` was described as higher resolution, but it is ignored when a
  target width is set (always, for the component);
- `QuadrantTheme::Default` was "standard Mermaid colors"; it takes the
  palette;
- `Background::Theme` is the palette's background, not the theme's;
- `with_palette`, `inverted`, `render_to_svg`, and `fallback_code_block` were
  missing from the API table;
- `git_graph.md`: `with_theme` was undocumented (terminal palette, lane
  `i` → accent `i mod 6`);
- `graph_expression.md`: a margin example that did not compile
  (`Margin::Chars`), the orientation and input-syntax lists, a scale claim,
  `with_font_family` on parsed graphs, and font sizing (layout-rs sizes by
  character count).

No **planned** markers were present.

### R-D5: dependencies and skills (AC-D5)

Done by a subagent.

- Both `docs/dependencies.md` files record Merman (fork URL, branch, exact
  rev, features, defaults off, license, why a fork, the return path, and the
  `serde_json` `preserve_order` side effect); `fontique` 0.12.0,
  `ttf-parser` 0.25.1, `rustybuzz` 0.20.1, `unicode-bidi`, `unicode-script`;
  resvg/usvg 0.47 (fontdb 0.23); Inter (OFL 1.1, every OS) and Noto Emoji
  (OFL 1.1, Linux only).
- `mermaid-rs-renderer` is no longer listed. One dated, clearly historical
  note in the root file's change list remains.
- Drift fixed: Merman's crate list in `biscuit-visualized/docs/dependencies.md`
  (it pulls `merman-core`, `merman-render`, `merman-theme-contract`, and
  `merman-unicode-linebreak`); a stale fontconfig link-time note; the skill
  lists of site-config families, diagram types, render-flow steps, and the
  fallback order; spec IDs removed from the skills.
- `biscuit-terminal` skill: a new "Diagram colors (the OSC 4 palette)"
  section in `diagrams.md`; the native-Windows summary in `discovery.md`
  includes OSC 4.
- Code comments fixed: `biscuit-terminal/lib/src/discovery/osc_queries/mod.rs`
  and `startup_exchange.rs` still said native Windows sends no OSC 4 query
  (comments only).
- `worktree/docs/performance-testing.md` pointed at
  `mermaid_rs_renderer::compute_layout`; it now points at Merman and
  `natural_size()`.
- mmdr search over docs and skills: the only remaining hits are
  `docs/research/` and `biscuit-visualized/docs/data-visualization/visualizing-graph-expressions.md`,
  a dated research survey (`prompt:` frontmatter, `last_updated: 2026-03-18`),
  left as historical research.

### README (R-C12)

`biscuit-visualized/README.md` was cut back to an overview: what the
package is, installation and features, examples, and a Documentation section
linking `merman.md`, the component pages, `mermaid-gitgraph.md`,
`dot-graph.md`, and `dependencies.md`. The error, time-budget, background,
input-format, and orientation detail was removed; all of it already lives in
those pages.

### R-D4: audience validation (AC-D4, AC-D7)

All of the following pass the six checks: new reader, examples per behavior
section, symbols linked and explained, Mermaid flows, no spec or plan
reference, links resolve. `merman.md` was done directly; the component pages
and dependencies by their subagents; the rest by an audit subagent.

| Document | Result |
| --- | --- |
| `biscuit-visualized/docs/merman.md` | pass after this phase's additions (overview, example, decisions, rejected, future; symbols linked); palette claim fixed (Darkmatter's static SVG uses the terminal component, so the terminal palette, not a fixed one) |
| `biscuit-visualized/docs/mermaid-gitgraph.md` | fixed: em, lane, and directive explained; examples added to "Cost and cache identity" and "Checking a layout"; symbols linked; merge errors stated exactly; tag-size rule added (see below) |
| `docs/components/{mermaid_diagram,git_graph,graph_expression}.md` | fixed (R-D2 above) |
| `biscuit-visualized/docs/dependencies.md`, root `docs/dependencies.md` | fixed (R-D5 above) |
| `biscuit-visualized/README.md` | fixed (R-C12 above) |
| `biscuit-visualized/bench/README.md` | fixed: p95, reference host, and goldens explained; run-flow diagram, a results example, and a `baseline.json` example added; sources linked |
| `biscuit-terminal/docs/discovery/terminal-queries.md` | fixed drift: three places said native Windows sends no OSC 4; the gate is stdout *or stderr*; symbols linked |
| `biscuit-terminal/docs/discovery/color.md` | fixed: OSC 4 and `RgbValue` explained; empty "Using the `bt` CLI" section filled from the code |
| `biscuit-terminal/cli/README.md` (Mermaid parts) | fixed drift: the PNG is rendered at the column width, not 2×; terminal palette; `--inverse`; the error and fallback behavior |
| `darkmatter/docs/rendering/mermaid.md` | fixed major drift: terminal section rewritten for the render-tree hook; `mermaid_theme_for_syntect` scope; static SVG palette and background; dead "Icon Packs" section removed; CLI table corrected; sanitizer limits spelled out with examples |
| `worktree/docs/git-graph.md` (Mermaid parts) | fixed: `to_git_graph` maps onto `biscuit-visualized`'s component types; `gitgraph_geometry` linked and explained |

- **Link check:** `python3 -I /tmp/merman-linkcheck.py <19 touched docs>`
  (a checker written for this audit: relative targets plus GitHub-style
  heading anchors) is clean for every touched document except the root
  `docs/dependencies.md`. That file has 58 broken workspace-package links
  (`./pkg` where `../pkg` is meant), the same 58 as at `b0d59f187` before this
  phase; none is in the Merman, fonts, or resvg entries. They are recorded
  here as pre-existing, not fixed.
- **AC-D7 search** (`features/`, `fixes/`, `YYYY-MM-DD-name`) over the touched
  docs: one hit, root `docs/dependencies.md:123`
  (`2026-09-26-lockfile-corroboration`, an unrelated `sniff` entry from
  2026-09-26), pre-existing. `worktree/docs/performance-testing.md` (one line
  changed this phase) carries several pre-existing spec references unrelated
  to this work; left as they are.

### R-D6: planned markers (AC-D6)

`grep -i planned` over all 19 touched docs finds one hit: root
`docs/dependencies.md:173`, "computes every planned build", ordinary
English about CI, not a marker. No **planned** marker remains, for shipped or
unshipped behavior.

### Matrix smell grep, and the defect it found

By the comment-drift subagent over `gitgraph_layout.rs`, `bench/`, the OSC 4
parser, and `fonts/metadata.rs`: no `#[serde(default)]`. Each
`filter_map`/`unwrap_or`/`.ok()` hit was judged; all are safe except one:

**Defect fixed:** `mermaid/render.rs` `tag_metrics` read
`themeVariables.tagLabelFontSize` as a plain px number (a JSON number or a
`"Npx"` string). Merman (`merman-render` `gitgraph/theme/typography.rs`,
`GitGraphRoleFontSize::resolve`, at the pinned rev) reads only a string:
`px`, `em`, `%` (of the base size), `inherit`/`unset` (the base), `0`; a JSON
number or anything else draws at 10 px. The base `fontSize` is clamped to at
least 1. So `'150%'` was measured at 10 px but drawn at 24 px: the written
`commitSpacing` was too small, the drawn tags overlapped, and
`gitgraph_geometry()` reported bounds that did not match the drawing. A JSON
`14` was measured at 14 px but drawn at 10.

- **Fix:** `tag_metrics` now uses `base_font_size` and `tag_font_size`,
  mirroring Merman's reading. One observed difference from Merman's resolver
  source: `"14px !important"` is *drawn* at 10 px (the value is discarded
  before the resolver), so it is treated as not a size, which matches the
  drawing.
- **Tests:** `gitgraph_tests::tag_font_size_*`, 12 L1 tests through the
  public `MermaidDiagram` (`gitgraph_geometry()` plus `render` to SVG). Each
  compares every measured tag box with the polygon Merman drew
  (`gitgraph_layout_tests::drawn_tags`, now `pub(super)`) and asserts the
  drawn neighbors are at least the gap apart. Cases: absent (control),
  `14px`, `150%`, `1.5em`, `120%` with `fontSize: 20`, `inherit`, `unset`,
  `14px !important`, JSON `14`, `"14"`, `"large"`, `"0"`.
- **Before the fix:** `150%`, `1.5em`, base-relative `120%`, `inherit`, JSON
  `14`, and `"14"` failed (`unset` and `"0"` were added with the fix); absent,
  `14px`, `!important`, and `"large"` passed. All 12 pass after it.
- Each runs about 1.3 s alone, about 5 s when twelve start at once on this Mac
  (each process builds the macOS font chain), the same profile as the
  existing `gitgraph_tests`.
- `MERMAID_BACKEND` → `merman@6e6b1e6554e8+bv4` (spacing output changes for
  such inputs). `cache_tests`, `mermaid_cache_key_tests` (including "the next
  version" now `+bv5`), `merman.md`, `mermaid-gitgraph.md`, and the skill
  were updated. Goldens are unchanged (the diagram id is normalized, and no
  golden sets a tag size).
- `mermaid-gitgraph.md` ("Tag widths") and the skill's tag-spacing bullet
  describe the rule.

Other smell hits (all judged safe, no change): `gitgraph_layout.rs:474`
(`filter_map`, parents already checked); `:487` (`unwrap_or` for a one-commit
graph, documented); the `as_str()` membership checks; `bench/main.rs:432`
(hostname failure treated as not the reference host, minor: banner wording);
`osc_queries/parse.rs` (a bad reply leaves its slot unknown, by design);
`startup_exchange.rs`; `fonts/metadata.rs:42, 67` (an unparsable face is
excluded like one with no OS/2 table).

### Clean-room check (R-T10, D69, AC-T9)

By the comment-drift subagent over 420 paths: the non-test source list of
every phase, plus `git diff --name-only main...HEAD` and the working-tree
files under the five areas, excluding features, fixes, and spikes. The search
was `xargs git grep --untracked -n -I -E '<pattern>' --` over that list.

| Pattern | Hits | Verdict |
| --- | --- | --- |
| `Zed Industries`, `Copyright .*Zed` | 0 | clean |
| `GNU General Public License`, `SPDX-License-Identifier: GPL` | 0 | clean |
| `GPL-3` / `AGPL` | 3 | this repository's own `AGPL-3.0-only` declarations (`biscuit-terminal/lib/Cargo.toml`, `worktree/lib/Cargo.toml`, `biscuit-terminal/cli/README.md`), not a third-party header |
| `zed-accent`, `inject_css`, `accent_colors` | 0 | clean |
| `mermaid_render` | 7 | test names (`test_mermaid_renderer_new`, `mermaid_render_separates_…`) |
| `zed_` | 17 | all are "-ized" words (`rasterized_svgs`, `normalized_os`, …); a word-boundary `zed_` and standalone `Zed` find 0 |

No file carries a Zed copyright or a GPL header, and no Zed-specific
identifier is present.

### Comment-drift pass

Comment-only changes; a non-comment diff check came back empty.

- `components/mermaid.rs` and `src/tests/l1/render_tree_diagram_hook.rs`:
  "host lacks Mermaid rendering dependencies" → the render failed (Merman is
  pure Rust).
- `mermaid/postprocess.rs`: the guard-test path now includes `src/`.
- `tests/mermaid_family_tests.rs` and `tests/mermaid_tests.rs`: "interim
  theme" → fixed-palette theme.
- `darkmatter/.../code_renderer.rs`: "Host lacks the Mermaid toolchain" → a
  failed render.
- `biscuit-terminal` `osc_queries/mod.rs` and `startup_exchange.rs`: the OSC 4
  Windows claims (by the R-D5 subagent).
- Directly: `cache/file_cache.rs` (`FileCache::new` said
  `$BISCUIT_VISUALIZED_CACHE_DIR/v1/`; the code uses
  `<dir>/biscuit-visualized/v1/`; the skill's two copies of the claim were
  fixed too); `mermaid/config.rs` (`QuadrantTheme::Default` "Default Mermaid
  colors" → the quadrants take the palette).
- Checked and accurate: the backend id, the 2 s budget, the commit-spacing
  constants, fontdb 0.23 / usvg 0.47 / Mermaid.js 12.2.0, docs paths; no
  `mmdr`, TODO, or spec path in comments.

Found but outside this feature's files (recorded as follow-ups, not edited):
`darkmatter/lib/src/mermaid/mod.rs` `render_for_terminal` docs (mention
`mmdc`, icon packs, and a 10 KB limit) and
`darkmatter/lib/src/markdown/output/terminal.rs` (`MermaidMode::Image` "via
mermaid.ink"); `worktree/lib/src/timing.rs:150`'s perf label "graph image
render (biscuit-terminal)".

### Final verification sweep

**Commit note:** at 14:39 an outside process committed this phase's
in-progress work (for example `1d3b0dc371` "fix(biscuit-visualized): measure
gitGraph tags as Merman renders them", `7db08f9b68`, `cbb293922a`
"planning(biscuit-visualized): record Phase 7 close for merman"). This agent
made no commit. The file lists in the frontmatter cover the committed and
uncommitted Phase 7 files alike.

- `biscuit-visualized`: `just test` → 933 run, 933 passed, 3 skipped (golden,
  contrast, and measurement corpora included). A first run failed only
  `mermaid_cache_key_tests::the_key_follows_the_post_process_version` (it
  pinned the "next" backend as `+bv4`; now `+bv5`). `just lint` clean;
  `just test-l2` "not applicable"; `just check-tier-coverage
  biscuit-visualized` → stranded (0). After the two final comment edits:
  `cargo clippy -p biscuit-visualized --all-features --all-targets -- -D
  warnings` clean.
- Dependents, serially and scoped to what changed (the tag-size fix,
  comment-only edits, docs read by tests):
    - `worktree-cli` graph tests (`test(/git_graph|graph/)`, L1): 87/87 pass.
      L2 graph: 4/5; `level2_graph_restores_lane_density_in_kitty` fails, one
      of the two pre-existing Kitty failures named by Phase 6. Neither
      `worktree` nor the `GitGraph` component sets `fontSize` or
      `tagLabelFontSize`, so the fix cannot reach it.
    - `darkmatter` (`test(/mermaid|svg_sanitizer|code_renderer/)`): 125/125;
      `just lint` clean.
    - `biscuit-terminal` (`--features image`,
      `test(/osc|palette|startup_exchange|ansi|color/)`, L1): 448/448;
      `just lint` clean.
    - `biscuit-terminal-cli` (`test(/mermaid|git_graph|atomic_tokens|flowchart/)`,
      L1): 4/4, including `public_docs_do_not_advertise_removed_atomic_tokens`,
      which reads `cli/README.md`.
    - `sniff`: nothing changed in it this phase; not rerun.
- `cargo nextest … -p biscuit-terminal --all-features` does not compile the
  lib tests (28 errors such as `text_terminal_with_width`, `ScopedLcAll`).
  Since `b0d59f187` the `biscuit-terminal` source diff is comments only, so
  this is a pre-existing feature-combination problem; the area's own feature
  set (`image`) compiles and passes. Not fixed here.
- The full `just test` / `just test-l2` in `biscuit-terminal`, `darkmatter`,
  `sniff`, and `worktree` were not rerun in this phase. Earlier phases ran
  them (Phase 4 and Phase 6 logs), and this phase changed only comments and
  docs there, plus a tag-size path none of them exercises.
- `cargo tree -i mermaid-rs-renderer` is empty; one usvg (0.47.0); the Merman
  rev is exact on the fork (see "Dependency graph" above).
- `just ci-local --plan` (branch vs `origin/main`, 1167 changed paths): 53
  executing cells (29 ubuntu, 10 macOS, 7 Windows, 7 WSL2). A pull request
  runs the 39 Linux and macOS cells, the ">30" band: expect 60–100 min.
- **Cross-OS:** not rerun. This phase's code change is a pure string and
  number parse with no OS surface, and the new tests compare measured against
  drawn on whichever host runs them. Phase 6 proved goldens on all four
  environments, and no golden changed.
- **Gate-skip calculator:** not used to skip; both gates were run. (The
  working tree also holds other sessions' uncommitted files, so it no longer
  equals this phase's changes.)

### Requirement-to-test mapping (this phase)

| Behavior | Test (L1, `biscuit-visualized` lib) |
| --- | --- |
| Tag size read as Merman reads it (each shape) | `tests::gitgraph_tests::tag_font_size_*` (12) |
| Tags never overlap for a relative tag size | same, the drawn-gap assertion |
| Cache identity after the spacing change | `cache_tests::…` (pins `+bv4`), `mermaid_cache_key_tests::the_key_follows_the_post_process_version` (`+bv5` differs) |
| No output change elsewhere | `golden_tests::*` (204), unchanged |
| Docs read by tests | `biscuit-terminal-cli` `public_docs_do_not_advertise_removed_atomic_tokens`; `test-toolkit`'s CI-docs test (run by the R-D5 subagent) |

### Close-out

**AC-S4 (D30):** Ken chose Merman on 2026-10-21. The reason: it won the
spike scorecard (S1/S2) on fidelity, family coverage (11 more families), and
exact measurement, with faster cold renders. This is recorded in the spec's
D30 and in `spikes/s1-comparison/FINDINGS.md`.

**Acceptance criteria:**

| AC | Status | Evidence |
| --- | --- | --- |
| AC-S1–S3, AC-S5, AC-S7 | met (spikes) | `spikes/s1-comparison/FINDINGS.md`, galleries, scorecard |
| AC-S4 | met | D30; above |
| AC-S6, AC-C4 | met | Phase 1 spike S3; Phase 4 log (worktree tests, D43) |
| AC-S8 | met | Phase 6 log (pixel-identical PNGs on macOS, Linux, native Windows, WSL2) |
| AC-R1 | not applicable | D30 |
| AC-C1, C2, C6, C6a, C7, C9–C13, C15 | met | Phase 3 log, validation table |
| AC-C2a | met | Phase 2 log (fork test, push); `ls-remote` above; PR draft present |
| AC-C3 | **open, with its gate:** every package passes except `darkmatter` `just test` (45 frontmatter key-order failures from Merman's serde_json `preserve_order`), left to the separate fix `2026-10-23-frontmatter-key-order` (plan checkpoint 4, ruled 2026-10-23) | Phase 4 log; this phase's scoped dependent runs |
| AC-C5 | met | Phase 6 (smoke and goldens per family); this phase (docs list in `merman.md`) |
| AC-C8, C8a, C8b | met | Phase 4 log (Darkmatter tests) |
| AC-M1, M2 | met | Phase 6 (`corpus_measure_tests`, 483 labels) |
| AC-M3–M11 | met | Phase 2 log, validation table |
| AC-T1–T4, T6–T8 | met | Phase 5 log; T4 Phase 6 (`contrast_corpus_tests`, with the allow-list in `decisions_for_review`) |
| AC-T5 | **open, with its gate:** Ken's sign-off in the first review (D74) | gallery; `decisions_for_review` |
| AC-T9 | met | clean-room check above |
| AC-P1–P6 | met | Phase 2 and Phase 5 logs. P6 (native Windows OSC 4) is carried; Windows detection is a **release** prerequisite (D7) |
| AC-V1–V4 | met | Phase 6 log (V2: `just bench` exits 1 on build-linux, in `decisions_for_review`) |
| AC-U1 | met | drafts present; none filed by an agent |
| AC-D1–D7 | met | this phase (R-D1–R-D6 above) |

**Departures from the spec in this phase:**

- `docs/mermaid-gitgraph.md` was kept (R-D3 allows either outcome).
- `tag_metrics` now follows Merman's tag-size reading (a defect fix within
  D36/AC-C12), with the `+bv4` bump.
- The final sweep was scoped for the dependents, not full suites in all five
  packages (see above).

**Follow-ups** (none started):

- `2026-10-21-mermaid-embedded-font` (font embedding in HTML).
- `2026-10-22-merman-text-fallback` (Merman text-art fallback).
- `2026-10-23-frontmatter-key-order` (darkmatter key order under
  `preserve_order`; closes AC-C3 and plan checkpoint 4).
- D57's return to stock Merman once upstream lands `commitSpacing`.
- Region-aware contrast for the sequence `box` case.
- Sequence and timeline speed on build-linux.
- The graph cache key has no font identity (below).
- Stale Darkmatter Mermaid comments (`mermaid/mod.rs`, `output/terminal.rs`).
- The root `docs/dependencies.md` broken workspace links.
- The `biscuit-terminal --all-features` test compile.

**Prerequisites:** the Windows color-detection prerequisite (D7) gates the
release, not this merge.

The spec is left in place; `just complete` was not run.

**Implementation complete, ready for review.**
