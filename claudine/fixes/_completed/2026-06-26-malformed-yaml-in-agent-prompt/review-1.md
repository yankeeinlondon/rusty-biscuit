---
ready: false
implemented: true
agent: codex/default
created: 2026-06-28T08:28:12
---

# Review 1: Malformed YAML in Agent Prompt

## Findings

### High: Source-load fence mismatches do not use Claudine's frontmatter-excerpt enrichment path

The spec requires `FrontmatterFenceMismatch` to wire into Claudine's existing `CompositionError::enrich_frontmatter` / `WithFrontmatter` path so the CLI appends the authored frontmatter excerpt at the render boundary. The implementation adds support for `CompositionError::FrontmatterParse(MarkdownError::FrontmatterFenceMismatch)` in `frontmatter_block_spec` and tests it synthetically, but the real `compose`, `inline-compose`, and `sequence` source-load path cannot reach that enrichment.

`resolve_composition_source` maps the Darkmatter error to `CompositionError::FrontmatterParse` before a `ResolvedCompositionSource` exists (`claudine/lib/src/composition/resolve.rs:18-22`). The command wrappers then return that error directly on load failure (`claudine/cli/src/commands/compose/prep.rs:365-378`, `claudine/cli/src/commands/sequence.rs:176-185`). Since `enrich_frontmatter` needs a `ResolvedCompositionSource`, the actual malformed top-level prompt file path never becomes `WithFrontmatter`.

Darkmatter's own `MarkdownError` block now includes a source excerpt, so the user still gets a useful line-1 diagnostic. That is good, but it is not the Claudine frontmatter appendix required by the spec and tested in `CompositionError` only through a manually constructed source. Either adjust the spec to make the Darkmatter block the intended load-error rendering, or change the load-error path to enrich from the `MarkdownError`'s `SourceContext` / original text without requiring a resolved source.

### High: No Level 2 verification for the user-visible highlighted CLI diagnostic

The acceptance criteria require the malformed-fence error to render a highlighted excerpt, with non-TTY behavior preserving the typed message and hint while stripping styling. The strongest tests I found are Level 1 unit/render tests: Darkmatter block string assertions, `FrontmatterExcerpt` unit tests, synthetic `error_walker` rendering, and resolver mapping tests.

There is no CLI-level PTY or real-terminal capture for `claudine compose`, `claudine inline-compose`, or `claudine sequence` against a malformed `----` prompt. Per the review rubric, user-observable terminal rendering such as highlighted excerpts, line gutters, ANSI stripping, and "no YAML in Agent Prompt" needs verification at the appropriate level. At minimum, add a Level 2 test that runs the CLI in a real terminal/multiplexer, captures stderr/stdout, and proves:

- `compose` fails before provider launch;
- the rendered diagnostic includes the fence mismatch and line-1 highlight;
- the Agent Prompt section is not emitted with raw YAML;
- `inline-compose` and `sequence` surface equivalent diagnostics.

Until that exists, the rendering behavior is not production-ready under the stated test rigor rules.

### Medium: Sequence acceptance is inferred, not directly tested

The spec calls out sequence symmetry: each malformed step should surface the error and sequence should aggregate per-step as it already does. The current tests prove the shared source resolver maps a malformed Markdown file to `FrontmatterParse`, but I did not find a sequence command or sequence execution test that feeds a malformed-fenced root document or malformed step document through the `sequence` path.

Because `sequence` has its own source-resolution branch and YAML-sequence support, this deserves a direct regression test. A focused Level 1 command/path test is enough for the control-flow contract; Level 2 can cover the terminal rendering once for the family of commands.

## What Looks Good

The Darkmatter parser heuristic is conservative and matches the spec: it only errors for a top-of-document dash-only run of four or more dashes, requires a matching closing run of the exact same length, and only treats the interior as a frontmatter near-miss when it parses as a non-empty YAML mapping (`darkmatter/lib/src/markdown/frontmatter.rs:224-271`). The unit tests cover four and five dashes, scalar/sequence/empty-map false positives, mismatched fence lengths, thematic-break prose, and valid `---` regression cases.

The typed error and Darkmatter block rendering are also wired cleanly through `MarkdownError::FrontmatterFenceMismatch`, including a path-aware `StatusBlock` with the actionable hint to use exactly three dashes.

## Verification

I attempted a focused `cargo nextest run --color=never -p darkmatter -p claudine -p claudine-cli ...` for the touched tests. It was still compiling dependencies after roughly one minute, so I aborted it per the non-interactive session rules. No test pass/fail result should be inferred from that run.

## Production Readiness

Not ready for production. The parser behavior is in good shape, but the actual command-path rendering does not prove the Claudine enrichment requirement, and the user-visible diagnostics are not verified at the required level.
