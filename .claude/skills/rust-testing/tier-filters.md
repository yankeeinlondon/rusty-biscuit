# Nextest Filtersets

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

`_tier_filter` in `just/devops.just` is the single source of truth for tier
expressions (`.config/nextest.toml` defines no filterset aliases); the shared
`_sanity`, `_test_l2`, and similar recipes read it:

- `sanity`: `-E '!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/) + test(/(^|::)slow_/))'`
- `test-l2` / `test-l3` / `test-browser` / `test-real`: `-E 'test(/(^|::)<marker>_/)'`

## A tier marker is a prefix on a name segment

`test(/…/)` is an unanchored regex over the test name (`module::path::test_name`,
not the binary id). The `(^|::)` anchor confines a marker to a segment start,
so `render_browser_fragment_carries_same_figures` is L1 and `browser_fragment_…`
is not. Put the marker first in a segment or leave it out entirely. A marker on a
**module** puts every test in it in that tier.

An area that sets `BISCUIT_TEST_FILTER` replaces `_tier_filter` with its own
expressions and must anchor them too.

## Stranded tests

A name that starts with a marker for a tier whose `test-<tier>` recipe is a
stub runs in no tier: L1 excludes it and the tier recipe does nothing.

- `just check-tier-coverage` fails on that combination. It builds test
  binaries, so it is not wired into `test`, `lint`, or a hook; run it when tier
  markers or tier recipes change.
- CI catches the same thing for free: `scripts/ci/completion.py` refuses an L1
  expected-test listing that excludes a test marked for a stubbed tier
  (`completion-test-stranded`).
- `prompts/_test-tiers.md` restates these rules for implementation and review
  prompts.

## Narrowing a recipe to one subject

Pass a positional test-name filter through the recipe's `*args`:
`just test-cli context_command::` in `claudine/`. Nextest runs only tests that
match **both** the tier's `-E` expression and the name, so an L2 recipe plus an
L1-only module selects nothing.

- Keep the trailing `::` so the filter names the module, not every path
  containing the word.
- In a consolidated package the former per-file target is now a module (see
  [consolidated-binaries.md](consolidated-binaries.md)); `--test <old-stem>`
  names a target that no longer exists.
- `-E 'binary(x)'` does not survive the recipes' two-layer argument
  interpolation, and `BISCUIT_TEST_FILTER` replaces the tier expression outright.
- Where a public `test-l2` fans out to a second package (claudine's also runs
  `claudine-gen`), call the shared recipe directly:
  `just _test_l2 <pkg> --features terminal-tests <module>::`.
- Outside a recipe, `cargo nextest run -p <pkg> --test l1 <module>::` selects
  the binary and module but carries no tier expression, so tests from every
  tier compiled into that binary run.
