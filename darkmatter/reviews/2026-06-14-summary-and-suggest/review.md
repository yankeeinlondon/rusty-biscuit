---
agent: codex
created: "2026-06-14T15:21:41"
date_range: "2026-06-01 through 2026-06-14"
interactive: ""
model: default
summary: "Darkmatter delivered a large rendering and composition push over the last two weeks: Markdown-aware hashing matured, remote URL references and file-link directives landed, schema validation gained inline object schemas and better author guidance, the render tree became the main rendering path, CodeBlock replaced YamlBlock as the primary atomic code renderer, disclosure blocks shipped across terminal/browser/MarkdownPlus targets, and tests expanded substantially across snapshots, browser, and real-terminal tiers. The strongest follow-up opportunities are to trim checked-in artifacts/debug tests, split CLI/compose god files, reduce duplicated operation metadata and code-block serialization, and tighten documentation drift around compose phases."
suggestions: 9
features:
  - 2026-05-28-darkmatter-hashing
  - 2026-05-28-schema-coercion
  - 2026-06-01-more-context-variables
  - 2026-06-01-url-referencing
  - 2026-06-07-file-links
  - 2026-06-10-schema-improvement
  - 2026-06-11-simplified-rendering
  - 2026-06-12-disclosure
fixes:
  - 2026-04-26-indent-shell-expansion
  - 2026-05-22-color-depth
  - interpolation-error-handling
commits:
  - df6eca2
  - 8de7698
  - b50fe0d
  - 4b0a402
  - a741a8e
  - 38a27fa
  - 4fb0310
  - d4b48fb
  - db1f2aa
  - 5f20db0
  - 1100c5f
  - 6f8c3d5
  - 83a26b0
  - cdd1e18
  - cd6aa3d
  - d80aa8a
  - 367a850
  - 1a819aa
  - ecc2cbf
  - d544f36
  - 4961939
  - 4049764
  - 747657b
  - 3fea7bf
  - 973f57c
  - 0776415
  - 18ae1ef
  - 1f24623
  - eec2e02
  - 3808a2c
  - 3bef62f
  - df2cee3
  - 92a8ae1
  - 8dd8b80
  - 22db946
  - 9669183
  - aa3aceb
  - 9f2995d
  - cbb50e8
  - 9463ad8
  - a0f3485
  - bc213e3
  - 1bcc1db
  - e2739d3
  - 2631786
  - 0840cb5
  - b61a3df
  - 8f22e0d
  - 5625a19
  - c8b1435
  - b6e70d1
  - 6317ef7
  - 291a1bd
  - 3278af4
  - 99bb903
  - 1979da9
  - ae1dbc3
  - 2445a45
  - 3a54055
  - 34b7e64
  - 4120045
  - 16ad122
  - 4a67ac0
  - d5ccddd
  - ec45b80
  - d228f24
  - d3db7ea
  - 7ec7563
  - 9bb6d49
  - ec4e050
  - 177ad9e
  - 227bb7f
  - 7c80262
  - 135d7c8
  - 8c699b4
  - ceca103
  - 083a7aa
  - 94b9858
  - 6e391f3
  - 6a3d547
  - 6e09195
  - 40acf27
  - 1111236
  - 987b161
  - 497fee5
  - 01cc40b
  - 8ac20cd
  - 86fe86e
  - c867809
  - 1e98ee8
  - 3747efb
  - 8cebbfe
  - f9220d9
  - e69fc62
  - 8d5c628
  - c49bea1
  - 5f97680
  - 115ecee
  - 4aaa8e2
  - 47128f3
  - 5718d46
  - 742b7d4
  - d2dd23d
  - 3efd98c
  - 7c1ab87
  - 7b5a980
  - 506059f
  - b9592a4
  - 03c90a9
  - c7e58e1
  - 32d29d3
  - ead655d
  - c5e1294
  - 1993665
  - 1ff1e43
  - b35cea4
  - d9d9715
  - 293da6d
  - fe33321
  - 623c863
  - 982d6df
  - 342006f
  - 3e9d6dc
  - 3f3ff4d
  - 848c20d
  - ec8ecd6
  - d31c25c
  - 6cee4f4
  - 1680935
  - 0586738
  - c2dd589
  - 30f1ae9
  - 3e2ca7f
  - 1389882
  - 2d53df6
  - 8bf1e5d
  - 9d05a35
  - cb69f03
  - d1517bc
  - a9e17a7
  - ef4e7eb
  - 0056998
  - 531b183
  - 462db50
  - 52868e3
  - d6998bf
  - 907e348
  - 7c88df8
  - b1487d2
  - 1151612
  - 3e64552
  - a114b40
  - 7ceb43c
  - b2b957e
  - 0a447e6
  - 8d59446
  - 028868c
  - c63beb1
  - 3b607fe
  - b1c99f1
  - 4e81944
  - 22fc507
  - 90272ff
  - 08e5f15
  - 04b932e
  - a45a9d8
  - f8505cb
  - cfcc353
  - 29bd2c9
  - 49a3918
  - 2b6385c
  - 262ea2d
  - ba2c391
  - cad03ac
  - f12f9de
  - da6e248
  - d804040
  - fa8a618
  - b04ecce
  - 63a8f2b
  - c0ee99c
  - bd6b700
  - 00893bd
  - 9b43947
  - 356de1b
  - 3d4dddb
  - 2853d8b
  - 11f1b0c
  - f591791
  - e3c12cb
  - 07ffdd4
  - 1535fd8
  - 7fcad49
  - 72ab041
  - 0ed7018
  - 441c6dc
  - f70ca3a
  - b03989e
  - 51969a2
  - 67c952a
  - 06ea17f
  - 0d12da4
  - be1477b
  - a056688
  - eed34f5
  - 7801fbb
  - f936efd
  - 62c818d
  - 1b914b3
  - 35e6583
  - 84c4469
  - f57c0b7
  - deb630a
  - 6711910
  - b6f1137
  - 8118ae1
  - fbe87cd
  - 675b50e
  - 7654221
  - 80d24b7
  - 37dce83
  - ae9c658
  - 5c42c9b
---

# Achievements and Suggestions in the **darkmatter** Package Area (_2 weeks_)

## Achievements

Darkmatter shipped several substantial features between 2026-06-01 and 2026-06-14. The largest delivery was the simplified rendering push: `CodeBlock` became the primary atomic code renderer, `YamlBlock` was migrated to a deprecated compatibility wrapper, direct `md code-block` rendering was added, code panel contrast became configurable through `CodeBlockMode`, and the page/code theme source of truth was centralized around `Terminal` color mode. Key commits include `b61a3df`, `5625a19`, `9f2995d`, `cbb50e8`, `9669183`, `eec2e02`, `3fea7bf`, and `a0f3485`, tied to `darkmatter/features/_completed/2026-06-11-simplified-rendering/spec.md`.

Disclosure blocks moved from planned DSL to shipped rendering behavior. The package now supports `::disclosure` / `::details` / `::end-disclosure`, transclusion wrappers, terminal rendering as a dim italic block quote, browser rendering as native `<details>/<summary>`, MarkdownPlus output, and style/frontmatter precedence including width versus max-width rejection. Key commits include `1a819aa`, `d544f36`, `367a850`, `d80aa8a`, `cd6aa3d`, `1100c5f`, `db1f2aa`, `d4b48fb`, and `4fb0310`, tied to `darkmatter/features/_completed/2026-06-12-disclosure/spec.md`.

Composition grew more capable. Remote URL referencing added explicit host allowlists, remote read configuration, cache freshness behavior, expression URL reads, and revalidation of cached remote/local dependency chains (`e3c12cb`, `c0ee99c`, `00893bd`, `b04ecce`; `darkmatter/features/_completed/2026-06-01-url-referencing/spec.md`). The `::file-links` directive landed with repository-bound discovery, document filtering, embedded render-tree output, per-file OSC8 links, and CLI JSON output (`b9592a4`, `506059f`, `5f97680`, `c867809`, `987b161`; `darkmatter/features/_completed/2026-06-07-file-links/spec.md`). Context variables and expression/effect catalogs also expanded with area/dependency/UTC time values and typed registry metadata (`06ea17f`, `67c952a`, `8d5c628`; `darkmatter/features/_completed/2026-06-01-more-context-variables/spec.md`).

Schema work improved both author ergonomics and validation correctness. SimplifiedSchema gained inline nested object shapes and descriptor-backed `md schema about` reporting (`d3db7ea`, `d228f24`, `4a67ac0`, `6317ef7`; `darkmatter/features/_completed/2026-06-10-schema-improvement/spec.md`). Earlier schema coercion work continued to pay off in the compose pipeline, preserving real boolean/number semantics instead of leaking truthy strings (`80d24b7`, `7654221`; `darkmatter/features/_completed/2026-05-28-schema-coercion/spec.md`).

Hashing moved closer to a production maintenance tool. The Markdown-aware hasher was split into focused modules, directory hashing now propagates per-file parse/load failures instead of silently hashing empty documents, and detailed diff reporting was corrected for moved/reordered sections with simultaneous content edits (`8118ae1`, `ae9c658`, `5c42c9b`; `darkmatter/features/_completed/2026-05-28-darkmatter-hashing/spec.md`).

Several correctness bugs were fixed. Shell directive indentation now preserves CommonMark structure inside lists and block quotes, with shared indentation helpers and byte-preserving shell-block output (`b6f1137`, `6711910`, `deb630a`, `f57c0b7`, `1b914b3`; `darkmatter/fixes/_completed/2026-04-26-indent-shell-expansion/spec.md`). Interpolation behavior started moving toward the active fix spec: unknown functions are now fatal and transitively shell-blocked frontmatter keys are deferred correctly (`cdd1e18`, `38a27fa`; `darkmatter/fixes/interpolation-error-handling/spec.md`). Rendering correctness fixes included ANSI reset before nested code blocks (`aa3aceb`), browser max-width centering (`fe33321`), page foreground inheritance through the root (`c5e1294`), and geometry-based page-frame policies (`742b7d4`, `47128f3`).

Test coverage improved materially. New and refreshed coverage spans browser-observable rendering, L2 real-terminal captures, render-tree parity, layout snapshots, schema conversion fixtures, error snapshots, CLI output modes, directory hashing failures, and disclosure interaction/nesting tests. Notable commits include `29bd2c9`, `262ea2d`, `028868c`, `3f3ff4d`, `982d6df`, `1111236`, `987b161`, `df2cee3`, `22db946`, `18ae1ef`, `db1f2aa`, `4b0a402`, and `df6eca2`.

Documentation drift was actively corrected. READMEs and the darkmatter skill now describe the completed render-tree cutover, CodeBlock/YamlBlock migration, ThemePair as a `(light, dark)` couple, borrowed-light theme pairs, MarkdownPlus output, disclosure behavior, code highlighting architecture, block quote rendering, and current style-frontmatter semantics. Representative commits include `f8505cb`, `93c9950`, `d8897fa`, `b42b49a`, `4fb0310`, `b50fe0d`, and `8de7698`.

No new crate/package was added to the darkmatter package area in this window. The work was delivered inside the existing `darkmatter` library and `darkmatter-cli` split.

## Suggestions

1. Remove checked-in backup/orig files. `darkmatter/cli/src/commands.rs.orig` and `darkmatter/lib/src/markdown/compose/shell_expansion/store.rs.bak` appear to be editor artifacts. This is upside-only cleanup: fewer false search hits, less review noise, and no behavior change.

2. Replace the debug-only frontmatter repro test with assertions or remove it. `darkmatter/lib/src/markdown/compose/mod.rs:4523` prints `DM_DEBUG` output and does not assert behavior, while `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:201` has an env-controlled debug dump. Given `83a26b0` removed a similar assertion-free debug disclosure test, this should either become a real regression test for the interpolation fix or be deleted after the active fix lands.

3. Split CLI command handling into modules. `darkmatter/cli/src/commands.rs:194` is 2,819 lines and owns dispatch, compose, render, code-block, hash, graph, frontmatter get/set/rm, validation formatting, and perf reporting. The existing `commands/schema/*` layout is the right pattern; extract `commands/compose.rs`, `commands/hash.rs`, `commands/frontmatter.rs`, and `commands/code_block.rs`. This is a maintainability refactor with moderate churn but little conceptual risk.

4. Reduce `ComposeOperation` metadata drift. Operation count, index, phase, default order, display/perf mapping, and runner behavior are spread across `darkmatter/lib/src/markdown/compose/types.rs:176` and `darkmatter/lib/src/markdown/compose/mod.rs:754`. A single descriptor table would have some up-front complexity, but it would reduce the risk that the next compose stage updates one list and misses another.

5. Fix compose phase documentation drift. `darkmatter/lib/src/markdown/compose/mod.rs:4` says the pipeline has three phases, while `ComposePhase` includes `Finalization` at `types.rs:171` and the runner comment at `compose/mod.rs:547` also lists only three phases. Treat the code as ground truth and update the module docs, the runner comment, and related skill/topic docs.

6. DRY `md code-block --output markdown` serialization. The fence-info assembly is duplicated for stdout at `darkmatter/cli/src/commands.rs:664` and `--show` at `commands.rs:703`. A helper that returns the Markdown fence string is upside-only and also gives tests one place to pin escaping behavior.

7. Avoid parser drift for highlight syntax. `parse_highlight_cli` at `darkmatter/cli/src/commands.rs:746` intentionally mirrors `parse_code_info`. That keeps CLI errors nice, but it creates a second grammar path. Prefer exposing a library helper for `HighlightSpec` parsing that returns structured errors, then have both fence metadata and CLI flags call it.

8. Delay theme resolution for compose outputs that do not need it. `darkmatter/cli/src/commands.rs:1136` resolves themes before matching the compose output format, but `Auto`/`Markdown` output does not use the theme. Moving resolution into the `MarkdownPlus` and `Html` arms is a small performance win with only upside.

9. Keep watch on god-file pressure in the render fold and compose module. `darkmatter/lib/src/markdown/render_tree/fold.rs` is 2,674 lines and now owns CommonMark folding, inline extension dispatch, embedded render-tree regions, disclosure handling, diagnostics, and a large in-file test suite. `darkmatter/lib/src/markdown/compose/mod.rs` is 6,807 lines. Avoid speculative splits, but the next feature that adds another block extension or compose stage should come with an extraction boundary rather than adding another large section to these files.
