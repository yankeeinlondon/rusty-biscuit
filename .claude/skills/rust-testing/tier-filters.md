# Nextest Filtersets

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

The `.config/nextest.toml` does not yet define named filterset aliases (nextest
feature limitation). `just/devops.just`'s `_tier_filter` is the single source of
truth; the shared `_sanity`, `_test_l2`, etc. recipes read it:

- `sanity`: `-E '!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/) + test(/(^|::)slow_/))'`
- `test-l2`: `-E 'test(/(^|::)level2_/)'`
- `test-l3`: `-E 'test(/(^|::)level3_/)'`
- `test-browser`: `-E 'test(/(^|::)browser_/)'`
- `test-real`: `-E 'test(/(^|::)real_/)'`

**A tier marker is a prefix, not a substring.** `test(/…/)` is an unanchored
regex search over the test *name* (`module::path::test_name`; nextest does not
match it against the binary id), so the older bare `test(/browser_/)` also
caught `unresolved_browser_feature_fails_the_render` and
`image_emits_typed_browser_attributes` — ordinary unit tests. The `(^|::)`
anchor confines each marker to a path-segment boundary. Name a test so the
marker is the first segment of its final name or not present at all;
`render_browser_fragment_carries_same_figures` is L1, `browser_fragment_…`
is not.

Anchoring alone cannot save a test whose name genuinely *starts* with a marker
it does not earn. `renderable` had 16 such tests: excluded from L1, and its
`test-browser` was a stub, so they ran in no tier for as long as they existed.
`just check-tier-coverage` now fails on exactly that combination — a stub
`test-<tier>` recipe in an area where tests still match that tier. It builds the
stubbing areas' test binaries, so it is deliberately not wired into `test`,
`lint`, or a hook; run it when tier markers or tier recipes change. CI makes
the same check at no build cost: every L1 producer's expected-test listing
already holds the tests its filter excluded, and `completion.py` refuses one
that carries a marker for a stubbed tier (`completion-test-stranded`). A marker
on a *module* counts too — darkmatter's `real_shells` module stranded seven
tests until 2026-09-23. `prompts/_test-tiers.md` states these rules for
implementation and review prompts.

An area that sets `BISCUIT_TEST_FILTER` carries its own copy of these
expressions and must anchor them too — `_tier_filter` cannot reach inside an
override.

**Narrowing a recipe to one subject** is a positional test-name filter through
the recipe's `*args`: `just test-cli context_command::` in `claudine/`. The
recipe keeps the tier's `-E` expression, and nextest runs only tests that match
**both** the expression and the name (checked on nextest 0.9.136: an L2
expression plus an L1-only module selects nothing). Keep the trailing `::` so
the filter names the module, not every test path that contains the word. In a
package with consolidated binaries (next section), the former per-file target
is now that module, so this is how you select it. `--test <old-file-stem>`
names a target that no longer exists. `-E 'binary(x)'` does not survive the
recipes' two-layer argument interpolation, and `BISCUIT_TEST_FILTER` replaces
the tier expression outright. Where a public `test-l2` fans out to a second
package (claudine's also runs `claudine-gen`), narrow by calling the shared
recipe directly: `just _test_l2 <pkg> --features terminal-tests <module>::`.
Outside a recipe, `cargo nextest run -p <pkg> --test l1 <module>::` selects the
consolidated binary and the module. It carries no tier expression, so the
module's tests from every tier compiled into that binary run.
