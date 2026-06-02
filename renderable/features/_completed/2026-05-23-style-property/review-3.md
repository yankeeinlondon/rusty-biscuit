---
ready: true
agent: codex
model: ""
---

# Review: Sub-Spec #3 Existing-Component Wiring

## Findings

### High: CLI fill overrides skip required width/max-width validation

`apply_common_style` only checks `style.{bucket}.width` and `style.{bucket}.max-width` conflicts inside `if !fill_claimed` ([darkmatter/lib/src/style/apply.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/style/apply.rs:359)). That means a document with both fields in the same bucket succeeds whenever the user also passes `--fill`, `--fill-tables`, `--fill-images`, or `--fill-block-quotes`.

The spec requires width and max-width to be mutually exclusive and to return `StyleApplyError` before rendering. The implementation notes also say to validate exclusivity before applying fill if not CLI-claimed. CLI precedence should preserve the CLI fill value, but it should not make an ambiguous frontmatter bucket valid.

Fix: move the `(Some(_), Some(_)) => ComponentWidthConflict` check before the `fill_claimed` branch. Add regression coverage for `style.table.{width,max-width}` plus `--fill max=60` and one component-specific flag, e.g. `--fill-images max=40`.

Verification level present: Level 1 tests cover conflicts only when no CLI fill claim is present. Required: Level 1 is sufficient for this validation contract.

### High: Frontmatter-driven terminal layout is not verified at Level 2

The user-visible requirements are terminal layout behaviors: table right alignment, table max-width, image alignment/max-width fallback layout, and block-quote max-width wrapping. Current sub-spec #3 tests mostly assert resolved `DarkmatterPage` state ([darkmatter/cli/tests/cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/tests/cli.rs:3544)) or render in-process and strip ANSI ([darkmatter/cli/tests/cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/tests/cli.rs:3819)). Existing Level 2 tests exercise CLI flags such as `--align-tables` and `--fill-images` ([darkmatter/cli/tests/level2_layout.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/tests/level2_layout.rs:496)), but they do not exercise the new `style:` frontmatter path.

Per the requested rigor model, real-terminal rendering requirements need Level 2 capture through WezTerm/Kitty/tmux. The current strongest tests for frontmatter-driven table/image/block-quote layout are Level 1, so the feature should not be marked production-ready.

Fix: add Level 2 cases that run `md` on synthetic documents or the `style-prop.md` fixture with `style.table.alignment/max-width`, `style.images.alignment/max-width`, and `style.block-quote.max-width`, then capture pane text and assert visible indentation/wrapping/width.

Verification level present: Level 1 for frontmatter; Level 2 only for equivalent CLI flags. Required: Level 2 for frontmatter user-observable terminal layout.

### Medium: HTML acceptance is only smoke-tested for the frontmatter path

The spec requires `md --output html darkmatter/example-docs/rendering/style-prop.md` to emit matching table/image/block-quote layout CSS. The current CLI test only checks that HTML rendering succeeds in dry-run mode ([darkmatter/cli/tests/cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/tests/cli.rs:3657)). CSS generation has builder-level tests, but not frontmatter-to-HTML assertions for the sub-spec #3 buckets.

Fix: add an integration test that renders HTML from frontmatter and asserts the expected selectors and declarations for `table`, `img`, and `blockquote`. For stronger CSS confidence, use a headless browser computed-style assertion when feasible.

Verification level present: Level 1 smoke only. Required: at least Level 1 output assertion for HTML serialization; computed-style browser coverage would be stronger.

## Requirement Coverage

- `style.table.width/max-width/alignment` lowering: Level 1 structural coverage exists; terminal rendering via frontmatter lacks Level 2.
- `style.images.width/max-width/alignment` lowering: Level 1 structural coverage exists; terminal fallback and HTML CSS through frontmatter lack direct assertions.
- `style.block-quote.width/max-width/alignment` lowering: Level 1 structural/in-process coverage exists; frontmatter Level 2 capture is missing.
- CLI-over-frontmatter precedence: Level 1 coverage exists for global and component-specific claims.
- Active wiring warnings: Level 1 parser coverage exists for sub-spec #3 keys and sub-spec #5 color keys.
- Invalid component fill CSS and width/max-width conflict: Level 1 coverage exists without CLI claims; conflict validation is incomplete when CLI fill claims the bucket.

## Verification

- Attempted `cargo test --color=never -p darkmatter style_frontmatter --test style_frontmatter`; compilation exceeded the non-interactive session bound, so I terminated the cargo process and did not get a completed test result.
- Attempted `cargo test --color=never -p darkmatter-cli component_overrides_global_fill_claims_every_bucket style_prop_fixture_resolves_to_expected_table_layout --test cli`; command syntax was invalid because `cargo test` accepts one positional filter.

## Production Readiness

Not ready. The core wiring is mostly in place, but invalid frontmatter can slip through when CLI fill overrides are present, and the frontmatter-driven terminal behaviors required by the spec are not yet verified at Level 2.
