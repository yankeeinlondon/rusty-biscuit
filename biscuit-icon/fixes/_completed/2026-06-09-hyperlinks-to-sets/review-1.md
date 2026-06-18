---
ready: true
agent: open_code
model: ""
---

# Review: Hyperlinks to Iconify Set Pages in `icon sets`

## Summary

The implementation adds OSC8 hyperlinks to the `Set` column in `icon sets` output. Each set name links to `https://icon-sets.iconify.design/{prefix}`. When the terminal does not support OSC8, the title renders as plain text (no markdown fallback URL, avoiding column-width inflation). The change is minimal, well-scoped, and correctly handled.

## What Changed

| File | Change |
|------|--------|
| `biscuit-icon/cli/src/sets_table.rs` | Added `ICONIFY_SET_BASE_URL` constant, `title_cell()` function, and wired it into `build_table()`. `build_table` now takes `&Terminal`. |
| `biscuit-icon/cli/src/commands.rs` | Passes `&Terminal` through to `build_table` via `render_sets`/`choose_layout`. No other changes. |
| `biscuit-icon/README.md` | Updated `sets` description to document OSC8 hyperlink behavior. |
| `biscuit-icon/cli/tests/cli.rs` | Added `sets_output_contains_no_raw_prose_markup` integration test. |

## Findings

### 1. [low] `title_cell` escape safety is implicit

`Prose::escape_text` is called on `title` and `Prose::quoted_attr` on the URL, which is correct. However there is no dedicated unit test proving that a title containing `<`, `>`, `"`, or `&` is safely escaped through the Prose pipeline and never leaks raw markup. The existing `set_title_never_leaks_raw_prose_markup` test uses `"Material Design Icons"` (no special characters).

**Recommendation:** Add a unit test with a title like `"Set <with> & \"special\" chars"` verifying no raw Prose tags appear and the URL is correctly formed. This is a defensive measure — the Prose library already handles this — but a test documents the contract.

### 2. [low] No unit test for blue color in OSC8 output

The spec says the title should be rendered as `<blue><a href={url}>{set-name}</a></blue>`. The test `set_title_links_to_iconify_page` verifies the URL is present and `set_title_never_leaks_raw_prose_markup` verifies no raw markup leaks, but neither confirms the blue ANSI styling (`\x1b[34m` or similar) is actually present in the rendered output.

**Recommendation:** Add an assertion on the raw output (e.g., contains `\x1b[34m` or `\x1b[38;2;`) to confirm the blue color is emitted. This is low severity because the blue styling is handled entirely by the Prose library and is already tested there, but a direct assertion in the icon tests would prevent a wiring regression.

### 3. [info] Graceful degradation path is sound

When `osc_link_support == false`, `title_cell` returns plain text immediately. The doc comment explains why (markdown-style `[label](url)` fallback inflates column width). This is a good tradeoff and the doc comment is informative without being a HOW-narration.

### 4. [info] `Terminal::new()` in CLI `sets` command path

The `sets` function in `commands.rs` builds a `Terminal` via `Terminal::new()` or `Terminal::builder().width(w).height(h).build()`. Neither sets `osc_link_support` explicitly. When the CLI runs in a real TTY that advertises OSC8 support, `Terminal::new()` detects it correctly. When run via `assert_cmd` (L1 integration tests), the process has no TTY, so `osc_link_support` defaults to `false` — which means the CLI integration tests exercise only the non-hyperlink path. The unit tests explicitly test both paths.

## Test Coverage Assessment

| Requirement | Test(s) | Level | Adequate? |
|---|---|---|---|
| Set name links to `https://icon-sets.iconify.design/{prefix}` when OSC8 supported | `set_title_links_to_iconify_page` | L1 | Yes |
| No raw Prose markup leaks (OSC8 path) | `set_title_links_to_iconify_page` + `no_raw_prose_markup_in_output` | L1 | Yes |
| No raw Prose markup leaks (non-OSC8 path) | `set_title_never_leaks_raw_prose_markup` | L1 | Yes |
| Blue color applied to linked title | Not directly tested | — | See finding #2 (low) |
| Special characters in title safely escaped | Not directly tested | — | See finding #1 (low) |
| `sets` CLI output has no raw markup | `sets_output_contains_no_raw_prose_markup` | L1 | Yes |

All user-observable requirements are verified at L1. This is appropriate for the feature: hyperlink rendering is an output-format concern (OSC8 escapes + SGR color), not a terminal-input or layout concern, so L2 is not required.

## Verdict

**Ready for production.** The implementation is minimal, correct, and well-tested. The two low-severity findings are defensive gaps in test coverage rather than functional defects. The spec is fully implemented.
