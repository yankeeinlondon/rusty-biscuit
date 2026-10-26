---
implementation_1: "2026-10-21T18:00:22-07:00"
implementation_2: "2026-10-21T19:06:52-07:00"
---

# Review Implementation Log: Graphics Where Possible

## Implementation of Review Findings #1

> **started at:** 2026-10-21T18:00:22-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review has no `id`-tagged body findings; IDs come from the frontmatter `findings` list (HT-1, LV-1 … LV-5) and the `## Human rulings` section that fixes each one's intended behavior
- the three `observations` are not repaired here (they are carried to the backlog by the follow-up pass)
- review `implemented` was already `false` at start

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| HT-1 | Hidden-tmux detection sends the plain Kitty query, which retitles a real tmux pane | fixed | every child below verified; `route.rs` diff reviewed by orchestrator |
| HT-1.1 | Hidden-tmux hint (`TERM=tmux-*`/`screen-*`, no `TMUX`/`STY`): direct exchange carries DA1 only (no unwrapped APC) | fixed | `direct_da1_only: true` in `resolve_detected`; `plan_hidden_tmux_sends_da1_direct_and_kitty_wrapped`, `plan_hidden_screen_term_…`; PTY `hidden_tmux_passthrough_verifies_a_tmux_route` sees 0 plain and 1 wrapped Kitty query |
| HT-1.2 | "Plain query OK → no tmux after all" route removed | fixed | branch removed from `resolve_hidden_tmux`; `hidden_tmux_plain_ok_is_not_a_direct_route`, `…_with_forwarded_iterm2_…`; `direct_transport_override_with_misleading_term_uses_the_plain_ok` (Kitty Direct Verified) |
| HT-1.3 | Unanswered-probe hint names both `BISCUIT_GRAPHICS_TRANSPORT=tmux` and `=direct` | fixed | fix text in `resolve_hidden_tmux`; matrix rows and PTY `hidden_tmux_silence_suggests_the_transport_override` |
| HT-1.4 | graphics-routes docs, verification check 8, probe-matrix tests updated | fixed | `graphics-routes.md`, `terminal-queries.md`, skill `discovery.md`, `verification.md` check 8 (a `Gi=` title change now fails), `graphics_probe_matrix.rs`, `level1_graphics_probe.rs` |
| HT-1.5 | Sibling: GNU screen (`STY`) with unidentified terminal sent the plain Kitty query (screen takes APC as hardstatus) | fixed | `without_apc_under_multiplexer` in `decide`; `plan_gnu_screen_session_sends_da1_alone`, `…_with_screen_term_…`, PTY `gnu_screen_direct_exchange_carries_no_kitty_query` |
| HT-1.6 | Sibling: visible tmux + `BISCUIT_GRAPHICS_TRANSPORT=direct` sent the plain Kitty query into tmux | fixed | same guard; `plan_visible_tmux_with_direct_override_sends_da1_alone` |
| HT-1.7 | Sibling sites: visible tmux auto, declared tmux, forced/CI/probe-off, Zellij, `osc_queries` `STY` checks, `bt` hint rendering | clean | `plan_never_visible_tmux`, `graphics_tmux_matrix` rows, `plan_declared_tmux_*`, `plan_never_*`; Zellij answers Kitty itself (no title semantics); `osc_queries` sends OSC only; `cli/src/hints.rs` prints the fix verbatim (tests pass) |
| LV-1 | A guessed terminal background is treated as measured, so Sixel and half-block images show a box | fixed | every child below verified |
| LV-1.1 | Background not reported by the terminal is not labelled `Measured` | fixed | new `ColorSource { Measured, Estimated }`, `Terminal::background_source`; `only_a_live_answer_is_measured`, PTY `unanswered_background_query_is_an_estimate`, `an_unreported_background_is_an_estimate_not_a_measurement` |
| LV-1.2 | Transparent half-block halves use the terminal default background | fixed | exact-byte tests in `half_block/tests.rs` (`49` background, `▄` for a transparent top half, space for both transparent; partial alpha blends) |
| LV-1.3 | Transparent Sixel pixels left untouched (`P2=1`); guess only blends partial alpha | fixed | DCS confirmed `ESC P 9;1;0 q`; decode-based tests in `sixel/tests.rs` (alpha-0 pixel and padding unset, opaque exact, partial blended); golden bytes regenerated |
| LV-1.4 | Conditional: prefer Kitty passthrough if tmux's Sixel drops transparency | deferred | conditional on a live WezTerm-under-tmux check only a person can run (`human_review_items`); not triggered by any evidence here |
| LV-1.5 | Sibling: `bt` capability report and JSON label colors | fixed | each color line says `measured`/`estimated`, JSON `source`; `color_lines_say_whether_the_terminal_reported_the_color` |
| LV-1.6 | Sibling: biscuit-visualized `display_background` painted a theme box when unmeasured | fixed | estimate now keeps `Transparent`, measured keeps `Solid`; two unit tests + L1 `display_size.rs` |
| LV-1.7 | Sibling sites: Kitty/iTerm2 PNG pass-through and padding, render-tree `page_background_color`, `color_mode` | clean | no estimate composited (padding transparent or requested `Solid`); `page_background_color()` has no reader; `color_mode` is a theme choice, never reported as measured |
| LV-2 | tmux limits docs omit that tmux clears its own Sixel on pane resize | fixed | "Known limits of tmux's own Sixel" in `graphics-routes.md`; skill `discovery.md`; `verification.md` check 7 |
| LV-3 | WezTerm-under-tmux expectation wrong: tmux 3.8 draws Sixel itself; passthrough has no effect | fixed | `graphics-routes.md` precedence paragraph, `verification.md` check 7 split by `#{sixel_support}`, skill; code already correct: `graphics_tmux_matrix.rs` rows at lines 280, 284, 324 |
| LV-4 | Native Windows never measures cell size; Sixel drawn at 8x16 fallback | fixed | every child below verified; Windows cross-check 3461 passed |
| LV-4.1 | Native Windows startup exchange adds `CSI 16 t`; `Terminal::cell_size()` populated; `FALLBACK` only when unanswered | fixed | `find_cell_size_reply` parser tests (macOS); Windows L2 `the_cell_size_answer_reaches_a_windows_program` (10×20) and `an_unanswered_cell_size_query_leaves_the_size_unmeasured` (None) on `build-win-native` |
| LV-4.2 | Discovered: a `ProbeMode::Off` terminal on Windows started the exchange from `cell_size()` at render time | fixed | `received_cell_size()` cache-only read in `Terminal::cell_size`; Windows L2 `a_probe_off_terminal_never_asks_for_the_cell_size` reproduced the defect then passed (6/6) |
| LV-4.3 | Sibling sites: Windows SSH server, legacy console, Unix ioctl/CSI 14 t, tmux/Zellij, all `cell_size()` consumers | clean | SSH server and legacy console excluded by the color-query gate (documented, fallback applies); Unix measured per render as spec section 2 requires; consumers read `cell_size()` and gain the measurement |
| LV-5 | Windows SSH server route: line after iTerm2 image starts at the image's right edge | fixed | code repaired and tested; live recheck still for a person (`human_review_items`, `verification.md` check 2) |
| LV-5.1 | Write a space then CR after the image on the Windows SSH server route only | fixed | typed `GraphicsRoute::needs_cursor_resync`; `only_the_windows_ssh_server_iterm2_route_needs_a_cursor_resync`, `a_windows_ssh_iterm2_image_is_followed_by_a_space_and_a_carriage_return`, `a_direct_iterm2_image_gets_no_space_and_carriage_return`, `inline_and_standalone_windows_ssh_images_share_the_resync`; existing goldens unchanged |
| LV-5.2 | Sibling sites: standalone, inline (`TwoColumn`), encoded PNG, render-tree images, darkmatter/biscuit-icon, biscuit-visualized diagrams, `bt image` | fixed | all share `render_to_terminal`/`render_inline_pixels`; visualized `a_mermaid_diagram_on_the_windows_ssh_route_is_followed_by_a_space_and_a_carriage_return` |
| LV-5.3 | Sibling sites: Kitty/Sixel on that route; `render_as_iterm2`/`render_as_kitty` | clean | forced Kitty/Sixel are `BlockedTransport` there (existing test); the width-only helpers have no route and no production caller |

### Progress

- starting the work on 'HT-1' at 18:01:28
        - defect class: an unwrapped multiplexer-interpreted control (the Kitty APC) was sent on the direct exchange while a tmux or GNU screen was suspected or present
        - sites checked:
                - hidden-tmux plan (`resolve_detected`): fixed, `direct_da1_only: true`
                - "plain OK → no tmux" branch in `resolve_hidden_tmux`: removed; only a wrapped OK makes a route, otherwise fallback with a fix naming `=tmux` and `=direct`
                - GNU screen (`STY` set, unidentified terminal): fixed, was `ProbePlan::DIRECT` with the Kitty query; screen takes APC as a hardstatus change
                - visible tmux with `BISCUIT_GRAPHICS_TRANSPORT=direct`: fixed, was `ProbePlan::DIRECT` straight into a visible tmux
                - both fixed by `without_apc_under_multiplexer` in `decide` (forces `direct_da1_only` whenever `TMUX` or `STY` is set and the plan is direct)
                - visible tmux, auto transport: clean (`NATIVE_SIXEL_PLAN` is DA1-only; passthrough plan has `direct: false`; `plan_never_visible_tmux`)
                - declared tmux (`=tmux`, no `TMUX`): clean (`direct: false`; `plan_declared_tmux_unidentified_sends_wrapped_only`, new `plan_declared_tmux_inside_gnu_screen_sends_wrapped_only`)
                - forced format / CI / probe off: clean (`ProbePlan::NONE`)
                - Zellij: clean, intentionally: Zellij 0.45 implements Kitty graphics and answers the query itself; it does not read APC as a title
                - `osc_queries/query.rs` `STY` checks: clean, OSC color queries only, never a Kitty query; `startup_exchange` sends the Kitty query only per `probe_plan`
                - consumers: `bt` hint rendering (`cli/src/hints.rs`) prints the route's fix verbatim, no change needed
        - docs: `docs/discovery/graphics-routes.md` (hidden-tmux table, flowcharts, probe list, fix table, transport table, `direct_da1_only`), `docs/discovery/terminal-queries.md`, skill `discovery.md`, verification check 8 (title change is now a failure; hint names both overrides)
        - tests: probe-matrix rows rewritten (plain OK no longer direct; unanswered fix names both overrides; `=direct` with misleading TERM gives Kitty Direct Verified; plan rows for hidden tmux, screen TERM, STY, visible tmux + `=direct`, declared tmux in screen); PTY tests assert no direct Kitty query for hidden tmux and for `STY`
        - commands: `cargo nextest run -p biscuit-terminal --test l1 -E 'test(/graphics_probe_matrix::|level1_graphics_probe::|graphics_tmux_matrix::/)'` 193 passed; `--lib -E 'test(/route|probe|hidden|startup_exchange|osc_queries/)'` 90 passed; `-p biscuit-terminal-cli -E 'test(/graphics_route|hints/)'` 11 passed; `just lint` clean
- work completed for 'HT-1' at 18:07:41
- starting the work on 'LV-1' at 18:08:51
        - defect class: an estimated (unmeasured) terminal property was labelled or consumed as measured, and transparent pixels were composited onto that estimate
        - root cause: `osc_queries::query::resolve_color` folded the live reply, `COLORFGBG`, and the per-app default into one `Option<RgbValue>`; `Terminal::background_color` was `Some(#1e1e1e)` under tmux and `SixelBackground::for_terminal` read any `Some` as `Measured`
        - sites checked:
                - provenance (`osc_queries`): fixed; new `ColorSource { Measured, Estimated }`, `resolve_color` split into a pure `sourced_color`; only a live reply (Unix tty or the Windows console query, same path) is `Measured`; `COLORFGBG` and per-app defaults are `Estimated`; `bg_color_with_source`/`text_color_with_source`/`cursor_color_with_source` added
                - `Terminal`: fixed; new `background_source` field (all four full literals in `terminal.rs`; no other full literal in the workspace), `ProbeMode::Off` always `Estimated`
                - `SixelBackground::for_terminal`: fixed; `Measured` only when `background_source` is `Measured`; an estimate keeps the guessed color as the edge-blend color
                - Sixel encoder: fixed; alpha 0 and the row padding are sent as alpha 0, which icy_sixel 0.7 leaves with no sixel bit (alpha < 128 is unset); DCS confirmed `ESC P 9;1;0 q` (`P2=1`); partial alpha blended and drawn opaque; opaque unchanged
                - half-blocks: fixed; transparent halves use `SGR 49` (`▀` fg + `49`, `▄` when the upper half is transparent, a space when both are, `49` below an odd last row); partial alpha blends
                - `bt` capability report and JSON: fixed; each color line ends `measured`/`estimated`, `ColorInfo` gains `source` (`measured`/`estimated`) for background, foreground, and cursor
                - biscuit-visualized `display_background`: fixed; `Solid` only for a measured background, otherwise the `Transparent` request is kept (was `Theme`, an opaque theme box) so the Sixel encoder leaves alpha-0 pixels unset; consistent with the ruling, the measured case keeps the spec's opaque solid raster
                - PNG pass-through (Kitty/iTerm2) and diagram row padding: clean; PNG bytes sent unchanged, padding is transparent or the requested `Solid`, no estimate involved
                - `render_tree` `page_background_color`: clean; carried to `TerminalCodeContext` but no renderer reads `page_background_color()` (grep), nothing composited or labelled
                - `color_mode` from the background estimate: out of class (a theme choice, not reported as measured), unchanged
                - conditional Kitty-through-passthrough preference (LV-1.4): not triggered; needs the live WezTerm-under-tmux check, not implemented
        - tests: `sourced_color` provenance unit test; PTY `graphics_bg_source` asserted `Measured` with an OSC 11 reply, `Estimated` with none sent, new `unanswered_background_query_is_an_estimate` (Alacritty, DA1 only, keeps `#1e1e1e` as `Estimated`); `SixelBackground` estimate case; Sixel golden bytes regenerated (transparent pixel sets no bit), decode-based tests for unset transparent pixels, opaque pixels unchanged, partial-alpha blend, transparent padding; route-dispatch Sixel test for both sources; half-block golden, `▄`, space, partial blend, odd row exact bytes; CLI `color_lines_say_whether_the_terminal_reported_the_color`; visualized display unit + L1 tests for estimated → transparent
        - docs: `docs/components/terminal_image.md` (Sixel transparency table, padding, half-block transparency), `docs/discovery/terminal-queries.md` (provenance), biscuit-visualized `docs/components/mermaid_diagram.md` and `graph_expression.md`, skills `biscuit-terminal/{image-rendering,discovery,cli}.md`, `biscuit-visualized/diagram-components.md`
        - commands: `cargo nextest run -p biscuit-terminal --lib -E 'test(/sixel|half_block|osc_queries|background|terminal_image|terminal::/)'` 262 passed; `cargo build -p biscuit-terminal --example discovery_probe` then `--test l1 -E 'test(/level1_graphics_probe::|level1_osc_queries::|level1_terminal_osc_cache::|graphics_probe_matrix::/)'` 131 passed; `-p biscuit-terminal-cli -E 'test(/output::|color_line|render_pretty|metadata/)'` 14 passed; `-p biscuit-visualized --features terminal -E 'test(/display|sixel/)'` 15 passed; `cargo test -p biscuit-terminal --doc -- SixelBackground osc_queries` 5 passed; `just lint` clean in biscuit-terminal and biscuit-visualized
- work completed for 'LV-1' at 18:20:02
- starting the work on 'LV-2' and 'LV-3' (documentation only, done by the orchestrator) at 18:21:40
        - defect class: the documented expectations for tmux's own Sixel route (its precedence over passthrough and its limits) did not match tmux 3.8's live behavior
        - sites checked:
                - `docs/discovery/graphics-routes.md`: fixed; added that a Sixel-capable outer terminal (WezTerm under a Sixel-built tmux) gets tmux's Sixel even when it renders Kitty, that `allow-passthrough` then changes nothing, `BISCUIT_GRAPHICS=kitty` as the way to Kitty passthrough, and a "Known limits of tmux's own Sixel" paragraph (scrolls in copy mode, hides on window switch, cleared on pane resize or split, not shown to a later client)
                - `verification.md` check 7 (WezTerm under tmux): fixed; split into tmux-with-Sixel (expect Sixel `Direct` `Verified`, no box, cleared on resize, passthrough off has no effect, `BISCUIT_GRAPHICS=kitty` gives Kitty `Tmux`) and tmux-without-Sixel (old Kitty passthrough expectations)
                - skill `.claude/skills/biscuit-terminal/discovery.md`: fixed, same two facts
                - code: clean; `graphics_tmux_matrix.rs` rows at lines 280 (WezTerm Sixel client + Sixel DA1 → native Sixel), 284 (passthrough off still native Sixel), and 324 (forced kitty → passthrough) already assert the live behavior
                - `docs/components/terminal_image.md` tmux Sixel line: clean (links to graphics routes, makes no limit claim)
- work completed for 'LV-2' and 'LV-3' at 18:24:10
- starting the work on 'LV-4' at 18:22:19
        - defect class: a terminal geometry fact that a live query could measure was left unmeasured on one platform (native Windows `cell_size()` was hard-wired to `None`), so every image was sized from the 8×16 estimate
        - root cause: `discovery::fonts::cell_size()` had only the Unix `TIOCGWINSZ` ioctl and the Unix-only CSI 14 t round trip; the non-Unix branches returned `None`, and nothing ever asked a Windows terminal
        - change: the startup exchange on native Windows (`startup_exchange::ASKS_CELL_SIZE = cfg!(windows)`) adds `CSI 16 t` to the one request, after OSC 10/11 and before the Kitty query and the DA1 sentinel (`direct_request`); the answer is parsed with the new platform-independent `fonts::window_size::find_cell_size_reply` (`CSI 6;<h>;<w>t` anywhere in the reply, each dimension 1..=1024 px, otherwise `None`) and cached in `DirectState::cell`; `fonts::cell_size()` on Windows returns `startup_exchange::live_cell_size()`, which sends the exchange (colors riding along) only if no earlier caller did and the color-query gate passes; error and silence record `Some(None)`; a busy-input retry keeps the cell query
        - sites checked:
                - native Windows, local terminal: fixed (above); `CellSize::FALLBACK` only when nothing usable answers
                - Windows SSH server: clean by design; the color-query gate excludes `SSH_CONNECTION` on Windows, so `CSI 16 t` is never sent and the 8×16 estimate applies (matches the review observation that it gets no reply there); documented
                - legacy Windows console: clean; excluded by the same gate (not on the color list), no 500 ms silence wait added
                - Unix with pixel ioctl: clean; `TIOCGWINSZ` per lookup, unchanged
                - Unix without pixel ioctl: clean; CSI 14 t window pixels ÷ grid per lookup, unchanged
                - tmux / Zellij (Unix): clean for this class; the ioctl is per lookup, and Zellij keeps `needs_measured_cells` (fallback text instead of the estimate)
                - resize vs. caching: decided; the Windows answer is cached once per process. A cell's pixel size follows the font, not the window, so a resize does not change it; columns are still read per render. Re-asking per render would read console input mid-program, against the spec's "never probe from a running event loop". Limitation (a font change in a long-running Windows program is not seen) documented in `terminal-queries.md#cell-size`
                - consumers: all fixed through the one source: `Terminal::cell_size()` and `fonts::cell_size()` feed Sixel (`sixel.rs`), half-block (`half_block.rs`), Kitty and iTerm2 footprint (`kitty.rs`, `iterm.rs`, `protocol.rs`), `ImageWidth::Scale` (`mod.rs`), horizontal-rule image tier, biscuit-visualized `display_pixels`/mermaid/graph_expression/git_graph, and `bt image --debug`; `calculate_display_dimensions` (public helper with a fixed 8 px width, no `Terminal`, only in tests and the prelude) is out of class and unchanged; the `bt` capability report shows no cell size (clean)
                - typeahead: clean; `ESC[6;20;10t` is a CSI sequence, so `typeahead_ranges` never returns it and the Windows port never re-injects it (new unit test)
        - tests: `fonts::window_size` unit tests (Windows Terminal's reply, among OSC/Kitty/DA1 replies, split across 3-byte reads, zero/over-1024/overflow rejected, foreign CSI replies, a malformed reply does not hide a later one); `startup_exchange` unit tests (only Windows asks, `CSI 16 t` precedes the DA1 sentinel, left out unless asked, one interleaved reply parsed by every part, cell reply never typeahead); `discovery_probe` `PROBE_GRAPHICS_CELL_SIZE` prints `graphics_cell_size[i]=`; Windows ConPTY L2 `level2_windows_console_query::the_cell_size_answer_reaches_a_windows_program` (answers `ESC[6;20;10t`, asserts `Some(CellSize { width: 10, height: 20 })`) and `an_unanswered_cell_size_query_leaves_the_size_unmeasured` (asserts `None`)
        - docs: `docs/discovery/terminal-queries.md` (new "Cell size" section, exchange diagram, gate table, platforms, caching, Windows SSH server), `docs/components/terminal_image.md`, rustdoc on `fonts::cell_size`, `Terminal::cell_size`, `CellSize::FALLBACK`; skills `biscuit-terminal/{SKILL,discovery,image-rendering,os-environment}.md`; `os/windows-console.md` (ConPTY forwards `CSI 16 t` and does not answer it; a narrowed cross-check runs a stale `discovery_probe.exe`)
        - commands: `cargo nextest run -p biscuit-terminal --lib -E 'test(/startup_exchange::|fonts::window_size::|tty_query::|fonts::types::|terminal::tests/)'` 57 passed (first run 1 failed: my fixture mixed ST and BEL terminators, see below); `cargo build -p biscuit-terminal --example discovery_probe` then `--test l1 -E 'test(/level1_graphics_probe::|level1_query_budget::|level1_terminal_osc_cache::|level1_osc_queries::|graphics_probe_matrix::|level1_terminal_init::|terminal_image_parity::/)'` 174 passed; `just lint` clean; `cargo clippy -p biscuit-terminal --lib --target x86_64-pc-windows-gnu -- -D warnings` clean; `just cross-check biscuit-terminal --os windows` pass (3457 passed, 63 skipped); `./scripts/cross-check.sh --os windows biscuit-terminal --features terminal-tests --test level2 level2_windows_console_query` 5 passed after building the probe on the rig (first attempt: 2 failed, stale `discovery_probe.exe`)
        - noticed, not changed: `osc_queries::parse::parse_osc_color_response` ends a reply at the first BEL in the buffer even when an ST comes earlier, so an ST-terminated OSC 10 followed by a BEL-terminated OSC 11 loses OSC 10 (terminals use one terminator, so not seen live)
- work completed for 'LV-4' at 18:35:57
- orchestrator follow-up on 'LV-4' at 18:41:30
        - discovered: the subagent's lazy `live_cell_size()` let `Terminal::cell_size()` on native Windows start the startup exchange at render time for a terminal built with `ProbeMode::Off`, which breaks the spec's TUI escape hatch (probing off means the program owns console input)
        - fixed: new `startup_exchange::received_cell_size()` reads the cached `CSI 16 t` answer without sending; `Terminal::cell_size()` uses it on Windows (with `ProbeMode::Auto` construction already asked in the same exchange, so measured results are unchanged); the free `fonts::cell_size()` keeps the lazy, gate-checked behavior
        - test: new Windows L2 `a_probe_off_terminal_never_asks_for_the_cell_size` (console would answer `ESC[6;20;10t`; result must be `None`)
        - evidence: the first rig run (code fixed, test probe binary stale) failed with `Some(CellSize { width: 10, height: 20 })`, which reproduced the render-time query; after rebuilding `discovery_probe` for `x86_64-pc-windows-msvc` in the rig clone, `./scripts/cross-check.sh --os windows biscuit-terminal --features terminal-tests --test level2 level2_windows_console_query` gave 6 passed, 0 failed
        - Unix render-time cell size (ioctl, then CSI 14 t): clean against the contract; spec section 2 says cell geometry "is refreshed for each render and is not part of this capability cache", and this behavior predates the feature
        - macOS: `cargo nextest run -p biscuit-terminal --lib -E 'test(/startup_exchange::|fonts::window_size::|terminal::tests/)'` 48 passed
- starting the work on 'LV-5' at 18:41:07
        - defect class: post-image cursor repositioning relies on a control (CR) the intermediary console host may drop because its own cursor model (still at column 0, the iTerm2 image unmodeled) disagrees with the outer terminal's (at the image's right edge)
        - carrier: new `GraphicsRoute::needs_cursor_resync: bool`, same shape as `needs_measured_cells`; set by `resolve_windows_ssh` (iTerm2 outer) and by `resolve_forced` for a forced `iterm2` on the Windows SSH server; false everywhere else, including `From<ImageSupport>`. A `GraphicsTransport` variant was rejected: the bytes are not wrapped differently, and every `transport` reader would have to learn it
        - change: `protocol::cursor_resync` gives `" \r"` on such a route, `""` otherwise; `render_to_terminal` writes it right after the (transport-ready) image sequence, before the existing `ESC[<n>B\r` + scroll-compensation tail; `render_inline_pixels` appends it to the overlay (`ESC[s…ESC[u \r`), so the overlay leaves the cursor at column 0 of its first row on that route (`InlineImage::Overlay` doc updated)
        - sites:
                - standalone `TerminalImage` (`render_with_route`, `render_with_half_blocks`, `render()`): fixed (one shared tail in `render_to_terminal`)
                - inline `render_inline` (`TwoColumn`): fixed; every `TwoColumn` path follows the overlay with `\r`, a restore, or `ESC[nB\r`, so column 0 is safe
                - encoded-PNG input (`from_encoded_png`): fixed, same encoder path
                - render-tree image node (`render.rs` → `render_with_route`), darkmatter terminal images, biscuit-icon: fixed through the same path
                - biscuit-visualized `MermaidDiagram`/`GraphExpression`/`GitGraph` (`display_image` → `render_with_route`), `bt bar-chart`/`xy-chart`/mermaid commands: fixed through the same path (new L1 test)
                - `bt image` (`render_with_half_blocks`): fixed through the same path
                - Kitty/Sixel on this route: clean; `resolve_forced` blocks forced kitty/sixel there, `forced_by_caller` never lifts `BlockedTransport`
                - `render_as_iterm2`/`render_as_kitty` (Terminal-free public helpers; `render_optimistic` uses Kitty): out of reach, no route available, no production caller on this route
                - noticed, not changed: `HorizontalRule::render_image_tier` sends Kitty cells (`render_kitty_cells`) whenever `image_support()` is `ITerm`, so on the Windows SSH route the console host removes the APC and the rule shows blank rows instead of the Unicode tier; not this cursor class (nothing drawn, cursors agree)
        - tests: route unit `only_the_windows_ssh_server_iterm2_route_needs_a_cursor_resync`; `route_dispatch::a_windows_ssh_iterm2_image_is_followed_by_a_space_and_a_carriage_return` (exact: image sequence, ` \r`, `ESC[<rows>B\r`; equals the unflagged bytes with ` \r` inserted), `a_direct_iterm2_image_gets_no_space_and_carriage_return`, `inline_and_standalone_windows_ssh_images_share_the_resync`; biscuit-visualized `display_size::a_mermaid_diagram_on_the_windows_ssh_route_is_followed_by_a_space_and_a_carriage_return`; existing goldens unchanged
        - docs: `docs/discovery/graphics-routes.md` (field table, JSON example, Windows SSH server section with byte sequence and diagram, diagnosis marked unproven), `docs/components/terminal_image.md`; skills `biscuit-terminal/{image-rendering,discovery,cli}.md`, `os/windows-console.md`; `verification.md` check 2 adds `bt image --example` and the column-0 check
        - commands: `cargo nextest run -p biscuit-terminal --lib -E 'test(/terminal_image|iterm|cursor|route/)'` 226 passed; `--lib -E 'test(/two_column|horizontal_rule/)'` 105 passed; `cargo build -p biscuit-terminal --example discovery_probe` then `--test l1 -E 'test(/graphics_probe_matrix::|graphics_route|terminal_image_parity::|two_column|level1_graphics_probe::/)'` 322 passed; `cargo nextest run -p biscuit-terminal-cli -E 'test(/image|graphics/)'` 8 passed; `cargo nextest run -p biscuit-visualized --features terminal --test l1 -E 'test(/display/)'` 10 passed; `just lint` clean in biscuit-terminal and biscuit-visualized; no `#[cfg(windows)]` code added, so no cross-check
        - live check remains for a person (verification check 2)
- work completed for 'LV-5' at 18:47:33
- orchestrator: unrelated L2 repair found during the phase gates at 18:46:00
        - `level2_zellij_probe_is_verified_or_reasoned` (always skipped before; Zellij 0.45.1 is now installed here) failed before reaching the probe: Zellij rejected the session name ("must be less than 0 characters") because its socket directory sat under macOS's long `$TMPDIR`; once that was fixed, a first-run tips popup covered the output
        - fixed in the test only: a short `ZELLIJ_SOCKET_DIR` under `/tmp` (also passed to `kill-session`/`delete-session`) and a hermetic `--config` with `show_startup_tips false` and `show_release_notes false`; the stale "Not yet run" doc line removed; trap recorded in `.claude/skills/os/macos.md`
        - result: passes alone in 1.3 s; the full `just test-l2` tier is green

### Completion Report

> **completed at:** 2026-10-21T18:59:30-07:00 (elapsed about 59 minutes)

- **outcome: complete.** Every parent finding is repaired and verified, and every required check passes
- parent findings: total 6 = fixed 6 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - HT-1, LV-1, LV-2, LV-3, LV-4, LV-5: fixed
        - LV-1.4 (prefer Kitty through passthrough if tmux's own Sixel drops transparency) is a contingency in the ruling, not a required repair. It is deferred until the live WezTerm-under-tmux check shows dropped transparency. Next action: the person running `human_review_items` reports whether the box is gone
- verified repairs:
        - HT-1: a hidden tmux gets only the wrapped Kitty query, and the direct exchange carries DA1 alone. The plain-OK route is removed, and the fallback hint names both `BISCUIT_GRAPHICS_TRANSPORT=tmux` and `=direct`. The sweep found and fixed two more plain-APC leaks: GNU screen (`STY`), and a visible tmux with `=direct`
        - LV-1: background provenance is honest (`Measured` only for a live reply). Transparent half-block halves use the default background, and transparent Sixel pixels are left unset. The guess only blends partial alpha. The `bt` report labels each color, and diagram rasters stay transparent when nothing was measured
        - LV-2, LV-3: documented tmux's own-Sixel precedence and limits, and rewrote verification check 7
        - LV-4: native Windows asks `CSI 16 t` in the startup exchange, so `cell_size()` is measured there. The 8×16 fallback is used only when nothing answers. The orchestrator fixed a discovered render-time query for `ProbeMode::Off` terminals
        - LV-5: a typed `GraphicsRoute::needs_cursor_resync` adds a space and a CR after an iTerm2 image on the Windows SSH server route only. All other routes send the same bytes as before
- observations outside this review's findings (not repaired; for the follow-up backlog pass):
        - `HorizontalRule::render_image_tier` sends Kitty cells when `image_support()` is `ITerm`. On the Windows SSH server route the console host strips them, so a graphical rule shows as blank rows. The review states that existing graphical-rule behavior is outside this implementation
        - under GNU screen, an inherited `TERM_PROGRAM` naming a Kitty terminal still resolves to Kitty, direct, with no probe. That is a route-correctness question for screen, which the spec does not cover
        - LV-5 live check: with a full-width image, the outer terminal's cursor may sit in its pending-wrap column, so the added space could wrap. Try a `--width` 100% image during the live check
        - `parse_osc_color_response` stops at the first BEL even when an ST comes earlier. A mixed BEL/ST pair of replies could lose OSC 10; this has not been seen live
- still for a person (unchanged `human_review_items`): Windows Terminal (LV-4), WezTerm + tmux (LV-1), WezTerm → ssh build-win-native (LV-5), and WezTerm → tmux → ssh build-linux (HT-1)
- changed files:
        - library: `biscuit-terminal/lib/src/discovery/detection/route.rs` (HT-1, LV-5), `route/tests.rs` (LV-5), `discovery/osc_queries/{types,query,mod}.rs` (LV-1), `terminal.rs` (LV-1, LV-4), `components/terminal_image/{sixel,half_block,protocol}.rs` with `sixel/tests.rs`, `half_block/tests.rs`, `terminal_image/tests.rs` (LV-1, LV-5), `discovery/startup_exchange.rs`, `discovery/fonts/{window_size,types}.rs` (LV-4), `lib/examples/discovery_probe.rs` (LV-1, LV-4)
        - tests: `lib/tests/l1/{graphics_probe_matrix,level1_graphics_probe}.rs` (HT-1, LV-1), `lib/tests/level2/level2_windows_console_query.rs` (LV-4), `lib/tests/level2/level2_graphics_probe_wezterm.rs` (Zellij harness repair)
        - CLI: `biscuit-terminal/cli/src/output.rs` (LV-1, LV-5)
        - biscuit-visualized: `src/src/components/display.rs`, `src/tests/l1/display_size.rs` (LV-1, LV-5), `docs/components/{mermaid_diagram,graph_expression}.md` (LV-1)
        - docs: `biscuit-terminal/docs/discovery/{graphics-routes,terminal-queries}.md`, `docs/components/terminal_image.md` (all findings)
        - skills: `.claude/skills/biscuit-terminal/{SKILL,cli,discovery,image-rendering,os-environment}.md`, `.claude/skills/biscuit-visualized/diagram-components.md`, `.claude/skills/os/{windows-console,macos}.md`
        - feature folder: `verification.md` (checks 2, 7, 8), this log
- final verification (run by the orchestrator after the last change, with no subagent running):
        - `just test` in `biscuit-terminal/`: 3852 passed, 55 skipped (all findings)
        - `just test-l2` in `biscuit-terminal/`: 8 passed + 252 passed, 0 failed (HT-1 PTY tests, LV-1 probe, Zellij); the first run failed only the Zellij test, repaired above
        - `just lint` in `biscuit-terminal/`: clean
        - `just test` in `biscuit-visualized/`: 318 passed, 3 skipped; `just lint`: clean (LV-1, LV-5)
        - `just check-tier-coverage biscuit-terminal` and `biscuit-visualized`: 0 stranded
        - `cargo check --workspace --all-targets`: compiles (new public fields `Terminal::background_source`, `GraphicsRoute::needs_cursor_resync`)
        - `just cross-check biscuit-terminal --os windows`: 3461 passed, 64 skipped (LV-4, all)
        - `./scripts/cross-check.sh --os windows biscuit-terminal --features terminal-tests --test level2 level2_windows_console_query`: 6 passed (LV-4.1, LV-4.2); before that, a run with a stale rig probe binary failed one test and was rerun after rebuilding the probe
- no performance measurement was required; `deferred_perf_measurement` is not set

## Implementation of Review Findings #2

> **started at:** 2026-10-21T19:06:52-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review `implemented` was already `false` at start; it stays `false` until the cycle closes
- the review has two findings (R2-F1, R2-F2), no blocked findings and no observations; `human_review_items` are the existing real-terminal checks and are not repaired here

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | The real-tmux placeholder test depends on undeclared RGB support | fixed | every child verified; orchestrator reviewed the diff and ran `just test-l2 graphics` (6 passed) |
| R2-F1.1 | Kitty placeholder client declares RGB (client-scoped `-T RGB`) and reaches upload/grid assertions | fixed | `features: Some("RGB")` → client-scoped `tmux -u -T RGB`; `level2_tmux_kitty_client_gets_unicode_placeholders` passes through its upload, grid, color and forwarding assertions; module/test docs name the prerequisite |
| R2-F1.2 | Shared `TestClient::attach`: `-u` always, `-T` only when supplied | clean | `TestClient::attach` adds `-u` always and `-T` only for `Some(features)`; it is the client-scoped seam the repair uses |
| R2-F1.3 | Native Sixel client declares `sixel` explicitly | clean | declares `Some("sixel")`, matching the route's `has_feature("sixel")` requirement; `level2_tmux_draws_sixel_itself_at_the_bottom_of_the_viewport` passes |
| R2-F1.4 | Passthrough-on fixture: no client, no placeholder/RGB expectation | clean | no client; asserts only Kitty/Tmux/Unknown and wrapped chunks, all decided by the `allow-passthrough on` it sets; passes |
| R2-F1.5 | Passthrough-off fixture: no client, no placeholder/RGB expectation | clean | no client; fallback and hint follow from the `allow-passthrough off` it sets; passes |
| R2-F1.6 | WezTerm and Zellij probe fixtures assert actual replies, not placeholders | clean | WezTerm asserts `Verified` from its real reply; Zellij uses an explicit test config and accepts `Verified` or a reasoned `Unknown`; neither expects placeholders; both pass |
| R2-F2 | Direct Kitty transfers omit the required nonzero image IDs | fixed | every child verified; repaired in the shared builders so every ordinary upload inherits it; orchestrator reviewed `kitty.rs`/`protocol.rs` diff |
| R2-F2.1 | Standalone direct width-only Kitty upload carries `q=2` and nonzero `i=` | fixed | `a_standalone_direct_kitty_upload_is_numbered_and_sized_by_width`: `q=2,i=N,…,c=4,m=1`, N≠0, ID on first chunk only, distinct across renders |
| R2-F2.2 | Inline direct width-only Kitty upload carries nonzero `i=` | fixed | `an_inline_direct_kitty_upload_is_numbered_and_sized_by_width` through `TwoColumn::render` |
| R2-F2.3 | Standalone direct explicit-cell (WezTerm) Kitty upload carries nonzero `i=` | fixed | `a_standalone_direct_wezterm_upload_is_numbered_with_explicit_cells`: `…c=4,r=2,…` with nonzero first-chunk ID |
| R2-F2.4 | Inline direct explicit-cell Kitty upload carries nonzero `i=` | fixed | `an_inline_direct_wezterm_upload_is_numbered_with_explicit_cells` through `TwoColumn::render` |
| R2-F2.5 | Standalone tmux ordinary Kitty keeps a positive ID | clean | now uses `render_kitty_cells`; `without_rgb_the_same_client_gets_an_ordinary_overlay` gained `q=2` + nonzero `i=` assertions |
| R2-F2.6 | Inline tmux ordinary Kitty keeps a distinct positive ID | clean | `a_tmux_kitty_route_wraps_each_chunk_with_an_id_and_explicit_rows` passes unchanged (nonzero, fresh per render) |
| R2-F2.7 | Standalone tmux placeholders keep positive ID and `U=1` | clean | `render_kitty_placeholders` untouched; `the_standalone_path_uploads_a_virtual_placement_then_prints_the_grid` passes (24-bit nonzero ID, `U=1`) |
| R2-F2.8 | Inline tmux placeholders keep distinct positive ID and `U=1` | clean | `the_inline_path_returns_one_text_line_per_image_row` gained nonzero-ID and grid-color assertions; passes |
| R2-F2.9 | Graphics query keeps its supplied correlated ID | clean | `kitty_query(id)` untouched; probe ID tests pass in full `just test` |
| R2-F2.10 | Render-tree and diagram consumers inherit the repaired upload paths | fixed | inherited through `render_with_route`; biscuit-visualized `just test` 318 passed (`render_tree_diagram_hook` compares renders with the ID removed) |
| R2-F2.11 | Sibling: graphical `HorizontalRule` Kitty tier uploads with a nonzero ID | fixed | calls `render_kitty_cells`; `test_render_uses_kitty_image_tier_when_supported` asserts `q=2,i=<nonzero>` |
| R2-F2.12 | Sibling: legacy `render_as_kitty` / `render_optimistic` uploads with a nonzero ID | fixed | calls `render_kitty_width_only`; `test_render_as_kitty_produces_valid_output` asserts a `q=2,i=` prefix; `test_render_kitty_chunking` asserts one `i=` and `m=`-only continuations |

### Progress

- starting the work on 'R2-F1 real-tmux placeholder test depends on undeclared RGB support' at 19:07:27
        - defect class: a real-terminal fixture asserts a capability-dependent outcome without establishing the capability that selects it
        - delegated to a foreground `rust-developer` subagent (scoped to `level2_graphics_tmux.rs` and the other graphics L2 fixtures)
        - fixed: the Kitty placeholder client attaches with `features: Some("RGB")` (`tmux -u -T RGB`, client-scoped); module and test docs now name the UTF-8/RGB prerequisite; the `biscuit-terminal` skill's `image-rendering.md` L2 note records the same
        - sweep: shared `TestClient::attach`, native Sixel client, passthrough on/off fixtures, WezTerm/Zellij probe fixtures are clean (each declares the capability its assertion depends on, or asserts nothing capability-dependent)
        - this Mac's tmux infers RGB for `xterm-kitty` itself, so the old fixture also passes here; the repair removes the host dependence the reviewer hit rather than reproducing that failure locally
        - `just test-l2 graphics`: 6 passed; `just lint`: clean
- work completed for 'R2-F1 real-tmux placeholder test depends on undeclared RGB support' at 19:09:07
- starting the work on 'R2-F2 direct Kitty transfers omit the required nonzero image IDs' at 19:09:07
        - defect class: an ordinary Kitty upload builder emits a transfer without the image identity the protocol contract requires, so only callers that pass an ID explicitly get one
        - sibling sites found by orchestrator grep: `render_kitty_cells` / `render_kitty` (public; also used by graphical `HorizontalRule`), `render_kitty_width_only` (public), `render_as_kitty` (legacy optimistic path), and the standalone/inline direct branches in `kitty.rs` and `protocol.rs`
        - delegated to a foreground `rust-developer` subagent
        - fixed at the class: `render_kitty_cells` (and alias `render_kitty`) and `render_kitty_width_only` allocate a fresh nonzero ID from `next_kitty_id()` on the first chunk; `render_kitty_with_id` removed; tmux and WezTerm share the explicit-cell branch in both `kitty.rs` (standalone) and `protocol.rs` (inline)
        - new L1 tests in `terminal_image/tests.rs`: `a_standalone_direct_kitty_upload_is_numbered_and_sized_by_width`, `an_inline_direct_kitty_upload_is_numbered_and_sized_by_width`, `a_standalone_direct_wezterm_upload_is_numbered_with_explicit_cells`, `an_inline_direct_wezterm_upload_is_numbered_with_explicit_cells` (multi-chunk PNG; `q=2`, nonzero `i=` on the first chunk only, distinct IDs across renders; inline through `TwoColumn`)
        - assertions added for tmux ordinary standalone, tmux placeholder inline, `HorizontalRule` graphical tier, `render_kitty`/`render_as_kitty` prefixes and chunking
        - two byte-equality tests (`half_block/tests.rs::a_pixel_route_is_unchanged`, biscuit-visualized `render_tree_diagram_hook::hook_draws_the_fence_exactly_as_the_component_does`) now compare with the ID removed, because each render now has its own ID
        - sibling sites inherit the repair: `HorizontalRule` graphical rule, legacy `render_as_kitty`/`render_optimistic`, render-tree images and biscuit-visualized diagrams (all via `render_with_route`); `kitty_query(id)` untouched
        - docs: `docs/components/terminal_image.md`, `docs/discovery/graphics-routes.md`, skill `image-rendering.md`; builder rustdoc now shows `i=` on the first chunk
        - nonblocking observation (not a defect of this finding, not repaired): graphical `HorizontalRule` sends Kitty to terminals detected as iTerm2, which the spec's evidence says ignores `q=2` and replies `Gi=0;OK`; pre-existing and unchanged by the ID
        - subagent scoped runs: `just test terminal_image` 220, `kitty` 88, `horizontal_rule` 137, `two_column` 83, `image` 278, `render_tree` 282 passed; biscuit-visualized `render_tree_diagram_hook`/`display_size`/`mermaid_parity`/`graph_expression` passed; both lints clean
- work completed for 'R2-F2 direct Kitty transfers omit the required nonzero image IDs' at 19:14:38
- orchestrator verification after the last change, with no subagent running
        - found that `cd biscuit-terminal` from the repo root resolves through `CDPATH` to the main checkout (`/Users/ken/coding/personal/rusty-biscuit`); that first run was discarded and every gate below was rerun from the worktree's absolute path (`pwd -P` confirmed)
        - `just test` in `biscuit-terminal/`: 3856 passed, 55 skipped (R2-F2)
        - `just lint` in `biscuit-terminal/`: clean (R2-F1, R2-F2)
        - `just test-l2` in `biscuit-terminal/`: 8 passed (lib) + 252 passed (CLI, including the L2 tests that parse Kitty uploads), 0 failed (R2-F1, R2-F2)
        - `just test-l2 graphics`: 6 passed, including `level2_tmux_kitty_client_gets_unicode_placeholders` (R2-F1)
        - `just test` in `biscuit-visualized/`: 318 passed, 3 skipped; `just lint`: clean (R2-F2.10)
        - `just check-tier-coverage biscuit-terminal` and `biscuit-visualized`: 0 stranded (new tests are L1, in already-compiled modules)
        - no cross-OS check: the changes are an escape-sequence parameter and a test fixture, with no OS-specific code

### Completion Report

- **outcome: complete**; elapsed about 13 minutes (19:06:52 → 19:20 local)
- parent findings: 2 total = 2 fixed + 0 already fixed + 0 blocked + 0 deferred + 0 unresolved + 0 disputed
        - instance rows: 20 (R2-F1.1–F1.6, R2-F2.1–F2.12); 8 fixed, 12 clean, 0 open
- verified repairs
        - R2-F1: the Kitty placeholder L2 fixture declares the client's `RGB` feature through its existing `ClientSpec`; the production fallback without RGB is unchanged
        - R2-F2: every ordinary Kitty upload (direct width-only, direct explicit-cell, tmux) now carries `q=2` and a fresh nonzero `i=` on its first chunk, through the shared builders, so standalone, inline, render-tree, diagram, graphical-rule and legacy optimistic paths all inherit it
- open findings: none
- nonblocking observation for the follow-up pass: graphical `HorizontalRule` still sends Kitty to terminals detected as iTerm2, which may reply `Gi=…;OK` (pre-existing; outside both findings)
- the review's `human_review_items` (real-terminal checks in `verification.md`) remain for a person and are not part of this cycle
- changed files
        - `biscuit-terminal/lib/tests/level2/level2_graphics_tmux.rs` (R2-F1)
        - `biscuit-terminal/lib/src/components/terminal_image/kitty.rs`, `protocol.rs`, `tests.rs`, `half_block/tests.rs` (R2-F2)
        - `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` (test only, R2-F2.11)
        - `biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs` (R2-F2.10)
        - `biscuit-terminal/docs/components/terminal_image.md`, `biscuit-terminal/docs/discovery/graphics-routes.md` (R2-F2)
        - `.claude/skills/biscuit-terminal/image-rendering.md` (R2-F1, R2-F2)
        - this log
- no performance measurement was required; `deferred_perf_measurement` is not set
