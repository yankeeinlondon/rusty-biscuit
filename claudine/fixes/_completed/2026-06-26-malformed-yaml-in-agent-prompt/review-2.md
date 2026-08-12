---
ready: true
agent: codex/default
created: 2026-06-28T09:03:36
---

# Review 2: Malformed YAML in Agent Prompt

## Findings

No blocking findings.

The review-1 source-load gap has been addressed. `compose`, `inline-compose`, and `sequence` now enrich malformed source-load frontmatter errors through `enrich_composition_source_load_error`, so a top-level `FrontmatterFenceMismatch` can become `CompositionError::WithFrontmatter` before it reaches the CLI error walker.

The review-1 testing gap has also been addressed at the right level for the user-visible rendering requirement. The new tmux-backed Level 2 test drives all three composition entry points in a real terminal pane and verifies the rendered fence diagnostic, line-1 highlight, styling, absence of an `Agent Prompt` leak, and absence of provider launch.

## Coverage Assessment

- Malformed `----` / `-----` frontmatter wrapping a YAML mapping: Level 1 parser tests plus typed error rendering tests.
- False positives for thematic breaks, scalar/sequence content, empty maps, mismatched fence lengths, and valid `---`: Level 1 parser regression tests.
- `Markdown::try_from_content` and `Markdown::try_from(path)` typed error behavior: Level 1 darkmatter tests.
- Claudine source-load mapping and enrichment for `compose`, `inline-compose`, and `sequence`: Level 1 command/library tests.
- Sequence malformed step document behavior: Level 1 integration coverage verifies the step error does not degrade into missing-property/provider errors and does not launch the provider.
- User-visible terminal rendering for the malformed-fence diagnostic across `compose`, `inline-compose`, and `sequence`: Level 2 tmux capture.

## Notes

There is a small residual area to keep an eye on outside this fix: the frontmatter appendix gate is based on the captured stderr TTY/FORCE_COLOR decision, while plain-mode rendering is based on the `Terminal` color depth. The current implementation satisfies the requested non-TTY behavior and strips ANSI in plain mode, but a future cleanup could make the NO_COLOR appendix policy explicit in one helper.

## Verification

`cargo check --color=never -p claudine-cli --tests` passed.

A focused `cargo nextest run` over the malformed-frontmatter filters was attempted, but it exceeded the 60-second non-interactive cutoff while compiling dependencies and was stopped. No nextest pass/fail result is inferred from that run.

## Production Readiness

Ready for production.
