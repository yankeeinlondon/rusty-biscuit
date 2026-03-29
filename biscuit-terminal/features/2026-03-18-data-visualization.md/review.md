# Data Visualization Implementation Review

This review compares the implementation against:

- `spec.md`
- `plan.md`
- `tech-design.md`

The overall direction is good: the visualization split into `biscuit-visualized` happened, the `bt graph-expression` command exists, and the example graph is reasonable for a first-run experience (`Start -> Validate -> Render; Validate -> Retry`). The main gaps are that several render options are currently dead code, some planned graph scope is missing, and the CLI/test surface is not yet proving the new behavior end to end.

## Highest-Priority Findings

1. `biscuit-visualized` does not actually honor Mermaid theme/config/background settings, so a large part of the new API surface is currently a no-op.
   `MermaidDiagram::with_theme()` and `with_config()` store values, but `render_svg()` calls `mermaid_rs_renderer::render(&self.instructions)` without using `self.theme` or `self.config`, and `RenderRequest.transparent_background` is also ignored. See `biscuit-visualized/src/src/mermaid/render.rs:81-96`, `biscuit-visualized/src/src/mermaid/render.rs:146-157`, and `biscuit-visualized/src/src/mermaid/render.rs:227-232`. This means terminal-side calls like `--inverse`, `with_theme(...)`, quadrant theme presets, and fill overrides do not reliably change the rendered artifact even though the wrappers expose them.

2. Both visualization backends have incorrect cache-key coverage for render-affecting options, so the cache can return the wrong artifact.
   Mermaid cache keys only include `instructions`, `config.to_json()`, backend id, and output format; they omit theme, title, scale, and transparent-background state. See `biscuit-visualized/src/src/mermaid/render.rs:146-157`. Graph cache keys only include DOT source, orientation/title JSON, backend id, and output format; they omit scale and transparent-background state. See `biscuit-visualized/src/src/graph/render.rs:351-371`. In practice, a PNG rendered at scale 1 and then requested at scale 3 can hit the same cache entry.

3. The graph feature set is smaller than the planned scope.
   The plan called for four orientations, but the implementation only supports `LeftToRight` and `TopToBottom` in both the library and CLI. See `biscuit-visualized/src/src/graph/render.rs:23-40` and `biscuit-terminal/cli/src/args.rs:1297-1310`. The tech design also called for builder-level orientation support, but `GraphBuilder` has no orientation field or setter. See `biscuit-visualized/src/src/graph/builder.rs:19-23` and `biscuit-visualized/src/src/graph/builder.rs:25-123`.

4. The DOT unsupported-feature policy from the design is only partially implemented.
   `validate_dot()` rejects HTML labels, but it does not detect nested subgraphs/clusters or other unsupported constructs the design explicitly called out. See `biscuit-visualized/src/src/graph/dot.rs:64-100`. Right now those cases are left to `layout-rs` behavior instead of being surfaced as structured `GraphError::UnsupportedDotFeature(...)`.

5. The CLI fallback path for graph rendering is not wired up effectively on terminals without image support.
   `GraphExpressionRenderer` has a `fallback_code_block()`, but the CLI path does not use it. `render_graph_expression()` always goes through `display_graph_diagram()`, which renders a PNG and then hands it to `TerminalImage`. `TerminalImage::render()` returns `unwrap_or_default()`, so unsupported terminals produce an empty string rather than a code-block fallback. See `biscuit-terminal/cli/src/commands.rs:1491-1565`, `biscuit-terminal/lib/src/components/graph_expression.rs:349-376`, and `biscuit-terminal/lib/src/components/terminal_image.rs:272-289`.

## `biscuit-visualized`

### Functional Scope Misses

- Mermaid rendering is missing the designed backend-neutral behavior for theme/config/background application.
  The crate owns `MermaidTheme`, `QuadrantTheme`, and `MermaidConfig`, but today those values do not flow into the rendered SVG. The most visible user impact is that quadrant theming and inverse mode are exposed but ineffective.

- Graph rendering does not fully implement the planned orientation surface.
  The plan defined `LeftRight`, `TopBottom`, `BottomTop`, and `RightLeft`; the implementation narrowed that to two values without reflecting the change back into the plan or documenting it as an intentional reduction.

- The DOT validation policy is incomplete.
  The design explicitly preferred clear structured errors for unsupported constructs rather than silent degradation. Only HTML labels are checked today.

- `GraphExpressionRenderer::fallback_code_block()` always emits a `dot` fenced block.
  The tech design called for using either `dot` or `graph-expression` depending on the input path. See `biscuit-terminal/lib/src/components/graph_expression.rs:366-368`.

### Ergonomics

- `GraphBuilder` is less ergonomic than the design intended.
  The design specified `&mut Self` setters; the implementation consumes `self` for every `add_node`/`add_edge` call. See `biscuit-visualized/src/src/graph/builder.rs:76-103`. Fluent chaining works, but incremental assembly in loops/branches is more awkward than necessary.

- `GraphBuilder::build()` uses `expect(...)` in production code.
  See `biscuit-visualized/src/src/graph/builder.rs:119-123`. That conflicts with the repo-wide guidance to avoid `unwrap()`/`expect()` in production paths. Returning `Result<GraphDiagram, GraphError>` would be cleaner.

- DOT alt text is inaccurate for DOT inputs.
  `generate_alt_text()` labels the value as a node count, but for DOT sources it actually counts edge-like lines. See `biscuit-visualized/src/src/graph/render.rs:422-435`.

- The README is badly out of sync with the implemented API.
  It still documents `MermaidDiagram::new(..., None)?`, `GraphDiagram::new(...)`, and `GraphInputSyntax::Arrow` / `GraphInputSyntax::Dash`, none of which exist. See `biscuit-visualized/README.md:38-83`.

### Performance

- The graph-expression parser does O(n^2) node deduplication.
  `GraphExpression::parse()` repeatedly calls `nodes.contains(&node)` on a `Vec<String>`. See `biscuit-visualized/src/src/graph/expression.rs:65-84`. For larger graphs, an `IndexSet` or `HashSet + Vec` pattern would keep stable ordering without quadratic growth.

- PNG generation does avoidable disk I/O in both backends.
  Mermaid writes SVG to a temp dir, rasterizes to a temp PNG, then reads it back before caching. See `biscuit-visualized/src/src/mermaid/render.rs:189-206`. Graph does the same with `NamedTempFile`. See `biscuit-visualized/src/src/graph/render.rs:389-401`. If `rasterize_svg` accepted SVG bytes or `&str`, both paths could rasterize in memory and only write the final cached artifact once.

### Test Coverage

- `cargo test -p biscuit-visualized` passed, but that success overstates coverage of the risky paths.
  The most important graph render/cache tests are ignored: see `biscuit-visualized/src/src/tests/graph_tests.rs:154-203`.

- There are no tests proving that Mermaid theme/config/background changes alter output.
  Given the current implementation, such tests would likely fail, which is exactly why they are needed.

- There are no tests covering cache-key separation by scale, theme, or transparency.
  That is the easiest way to prevent the stale-artifact bug from recurring.

## `biscuit-terminal`

### Functional Scope Misses

- The new `bt graph-expression` command exists and the `--example` flag is present with a good example string.
  See `biscuit-terminal/cli/src/args.rs:845-927` and `biscuit-terminal/cli/src/commands.rs:1487-1489`. The example is small, readable, and demonstrates a branch.

- The CLI still does not fully meet the design’s “effective terminal rendering” requirement because non-image terminals silently get no output instead of a fallback code block.
  This is the biggest usability gap on the `biscuit-terminal` side.

- Mermaid wrapper ergonomics over-promise relative to the actual backend.
  `MermaidRenderer::with_theme`, `with_config`, and `with_transparent_background` are all still exposed, and CLI commands actively use them, but the underlying crate does not apply them. That makes the adapter API misleading.

### Ergonomics

- `render_graph_expression(... --json ...)` emits orientation using `Debug` formatting (`TopToBottom`, `LeftToRight`) instead of the CLI spelling.
  See `biscuit-terminal/cli/src/commands.rs:1525-1534`. Hyphenated values would be friendlier and more consistent with clap help.

- The graph renderer also keeps `From<String>` / `From<&str>` impls that panic on invalid input.
  See `biscuit-terminal/lib/src/components/graph_expression.rs:379-389`. That is convenient, but it undermines the otherwise `Result`-based API.

### Test Coverage

- There is no dedicated CLI integration coverage for `graph-expression`.
  I ran `cargo test -p biscuit-terminal-cli graph_expression -- --nocapture`; it ran `0` tests. The current integration file includes lots of diagram coverage, but none for the new command.

- CLI rendering tests are still written around `mmdc`, which no longer matches the feature architecture.
  See the entire block at `biscuit-terminal/cli/tests/integration_test.rs:1225-1405`. Those tests are named and gated around Mermaid CLI availability instead of validating the native Rust rendering stack.

- The package-level `just test` signal is currently noisy for this feature review.
  `just test` in `biscuit-terminal` fails, but the failures are unrelated list-rendering tests, not the new visualization code. That is not a blocker for the feature itself, but it does mean the package is not green while this work is landing.

## Recommended Follow-Up Order

1. Fix the dead render options in `biscuit-visualized`.
   Either wire theme/config/background into the generated Mermaid source/render backend, or remove the options until they are real. Also include all render-affecting fields in cache keys.

2. Fix the graph CLI fallback behavior.
   The CLI should print `fallback_code_block()` when image rendering is unavailable, not silently emit nothing.

3. Decide whether the orientation reduction is intentional.
   If only LR/TB are truly supported, update the plan/tech design/docs. If not, finish the missing variants.

4. Add focused tests for the new behavior.
   Minimum set: `bt graph-expression --json --example`, explicit DOT mode, fallback behavior on unsupported terminals, cache-key separation by scale, and one end-to-end graph render test that is not ignored.

5. Clean up docs and examples.
   Start with `biscuit-visualized/README.md` and the stale `mmdc` references in `biscuit-terminal/cli/README.md`.

## Validation Notes

- `cargo test -p biscuit-visualized`: passed (`51 passed`, `2 ignored`)
- `cargo test -p biscuit-terminal-cli graph_expression -- --nocapture`: passed but ran `0` matching tests
- `just test` in `biscuit-terminal`: failed due to unrelated `components::list` assertions, not data-visualization code
