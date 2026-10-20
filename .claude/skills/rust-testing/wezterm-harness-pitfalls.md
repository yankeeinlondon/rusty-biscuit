# WezTerm Harness Pitfalls

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Assertion patterns for L2 tests that read `frame.raw` from a real WezTerm pane
via `biscuit_test_harness::wezterm::WezTermHarness`. Backend selection,
`SharedHarness`, client turn-taking, and `--skip-config` live in the
`biscuit-test-harness` skill.

## Capture re-emits SGR only on transitions

`capture()` uses `wezterm cli get-text --escapes`, which walks the cell grid and
emits SGR only where attributes change. So `frame.raw` is not the bytes the CLI
wrote:

- Same-attribute runs collapse; a leading SGR may sit on an earlier row and not
  repeat on the next.
- Truecolor comes back in either semicolon (`\x1b[38;2;R;G;Bm`) or ITU colon
  (`\x1b[38:2::R:G:Bm`) form, and the form can change between runs.
- `\x1b[0m` may return as separate `39m`/`49m` resets, or vanish when the next
  cell shares attributes.

**Consequence:** never assert byte equality of two captures, per line or per
frame. Two identical CLI outputs can capture differently.

## Prove the CLI output first

When two captures differ, compare the program's real bytes before changing code:
run each case under `script -q /dev/null …`, pipe through `xxd`, and `diff`. If
the bytes match (apart from echo/prompt), the test is wrong, not the renderer.

## Assert semantically

- **Presence:** accept the expected color in both forms
  (`raw.contains(semi) || raw.contains(colon)`).
- **Absence:** reject the disallowed color in both forms.
- **Ordering:** to prove a span wraps a region, compare byte offsets
  (`open_at < alpha_at < beta_at`) and assert no reset between them.
- **Isolate the region** with unique text sentinels around the content under
  test, as `level2_style_hr_color_emits_sgr_in_real_terminal`
  (`darkmatter/cli/tests/level2/level2_horizontal_rules.rs`) does.
- **Parse SGR rather than substring-match** when you need cell attributes;
  `biscuit-terminal/cli/tests/level2/prose_cells.rs` and
  `biscuit-test-harness/src/layout_invariants.rs` accept both truecolor forms.

## Pick sharp witnesses

To prove "theme X is in effect", panel background is weak because themes share or
nearly share backgrounds. Use a signature foreground that differs unambiguously
between the themes being distinguished (for example the keyword color in a code
snippet); see `level2_code_block_styling.rs`.

## Gate and shell traps

- **A fast PASS can be a skip.** `WezTermHarness::available()` requires
  `WEZTERM_UNIX_SOCKET`; without it the test returns early and nextest reports a
  near-instant PASS. Set `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` to make that a
  failure ([tiers-and-gating.md](tiers-and-gating.md)).
- **Do not shell out to `wezterm cli` yourself.** Go through the harness, which
  runs clients with `--skip-config` and rations them per mux; a test's own
  clients get neither.
- **Never type `?` into an interactive pane**: with Atuin loaded it opens the AI
  overlay and wedges the pane (`"$?"` in an exit trailer is enough). Report status
  with `&& … || …`.
