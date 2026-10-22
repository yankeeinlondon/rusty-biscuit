---
$schema: feature-review.yaml
next: "2026-10-13-graphics-where-possible/review-2.md"
reviewed_by: "codex/gpt-6.1-sol"
created: "2026-10-21T14:08:03-07:00"
spec: "2026-10-13-graphics-where-possible/spec.md"
feature: "2026-10-13-graphics-where-possible/review-1.md"
implemented: true
description: "A **feature** review of `2026-10-13-graphics-where-possible/spec.md`"
findings:
    - "HT-1 (medium): Hidden-tmux detection sends the plain Kitty query, which retitles a real tmux pane; implement the 2026-10-21 hidden-tmux ruling"
    - "LV-1 (high): A guessed terminal background is treated as measured, so Sixel and half-block images show a box under tmux"
    - "LV-2 (low): Documented tmux limits omit that tmux clears its own Sixel images when a pane is resized"
    - "LV-3 (low): The WezTerm-under-tmux expectation is wrong: tmux 3.8 draws Sixel itself, so allow-passthrough has no effect there"
    - "LV-4 (high): On native Windows the cell size is never measured, so Sixel images are drawn at 8x16 fallback cells: too small, with blank rows below"
    - "LV-5 (medium): On the Windows SSH server route, the line after an iTerm2 image starts at the image's right edge instead of column 0"
observations:
    - "Terminal identification has two paths: general app detection reports mintty as xterm (so its feature rows use xterm defaults), while the graphics route identifies mintty separately from TERM_PROGRAM=mintty; consolidate in the planned terminal research and detection work"
    - "Over the Windows SSH server from WezTerm, the capability report shows `Depth: None` although the session renders color; color-depth detection there is outside this feature but would disable the half-block fallback if that session ever needed it"
    - "Over the Windows SSH server, the cell-size query (`CSI 16 t`) gets no reply (Windows Terminal locally answers `ESC[6;20;10t`), so the LV-4 fix will not reach that route: diagrams there stay rasterized at the 8×16 fallback and scaled up by the terminal, which softens chart text. A cell-scaled format (iTerm2, Kitty) could rasterize above the fallback when the size is unknown"
recurrence: false
human_in_the_loop:
    - |-
        Decide whether automatic graphics detection may change a tmux pane's title when SSH hides the multiplexer from the remote program.

        The implementation log reports that detection sends both an ordinary Kitty graphics query and a query wrapped for tmux. tmux interprets the ordinary query as a pane-title command, replacing the title with text such as `Gi=…`. The implementer explicitly deferred deciding whether that ordinary query is still needed to handle sessions whose `TERM` value suggests tmux even though no tmux is actually present. This affects the promised automatic graphics support over SSH; the author previously approved confirming hidden tmux with a wrapped query, but did not approve this title-changing side effect.

        Choose the intended behavior:

        - Preserve pane titles when `TERM` suggests hidden tmux: suppress the ordinary Kitty query in that situation and rely on wrapped confirmation. Ambiguous sessions may fall back until the user selects `BISCUIT_GRAPHICS_TRANSPORT=direct`. Recommended: it avoids changing unrelated terminal state and retains an explicit escape hatch.
        - Retain both queries: preserve broader automatic discovery when the tmux hint is misleading, accept that a real tmux may change its pane title, and document that limitation in the current graphics-route documentation.

        Record the choice under `## Human rulings` in this review and set `human_resolved: true`. The subsequent implementation review should check the selected behavior and its tests; this triage has not compared code against the specification.
human_resolved: true
human_review: true
human_review_items:
    - "After the repairs, re-check live (no agent can see these): Windows Terminal (LV-4: the `bt image --example` square spans exactly 16 columns, no blank rows below); WezTerm + tmux (LV-1: no box around the image); WezTerm -> ssh build-win-native (LV-5: the line after each image starts at column 0); WezTerm -> tmux -> ssh build-linux (HT-1: image or a hint naming both transport settings, and the pane title unchanged)"
    - |-
        After the code review and repair cycle, perform the terminal checks in `verification.md`. Record terminal and multiplexer versions and the observed result for each setup available to you. Check that images and diagrams are visible, subsequent prompts contain no query text or unwanted blank rows, and tmux images behave as described when resizing or switching windows. Zellij's real-terminal test never ran during implementation because Zellij was unavailable; its live behavior particularly needs verification. These checks were already planned by the specification and are separate from the unresolved pane-title decision.
has_blocked_findings: false
blocked: false
log: "/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/log.md"
implemented_by: "claude/opus"
---

# Review 1: human decision required before implementation review

The review stops at the initial decision check. Two independent agents reviewed the specification/plan and implementation log. The specification's major choices are settled, but the implementation log leaves one user-visible tradeoff unresolved: graphics detection through SSH can overwrite a hidden tmux pane's title. No implementation-to-specification comparison or test execution was performed, and no code findings have been issued. Production readiness has not been assessed; `ready` is intentionally unset rather than treating a human decision as a code failure or marking unreviewed code ready. The specification is not marked completed.

## Decision needed: preserve pane titles or retain broader automatic detection

Consider a user running tmux locally, then connecting over SSH and running `bt image --example`. The remote program may receive `TERM=tmux-256color` without the `TMUX` variable that normally identifies a local tmux session. Detection therefore tries a query wrapped for tmux and an ordinary graphics query.

The implementation log reports that tmux treats the ordinary Kitty query as a title command, changing the pane title to query text such as `Gi=…`. It explicitly leaves the ordinary-query decision unresolved in its [phase 7 account](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1396), and carries the side effect into the [final implementation summary](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1709). Calling this pre-existing in phase 7 means it existed in an earlier implementation phase; it does not establish that the feature's author accepted it.

The author's [hidden-tmux decision](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/spec.md:788) approves inferring tmux from `TERM`, confirming it with a wrapped query, and retaining a transport override. It does not settle whether an additional ordinary query should be allowed to change the pane title.

The options and recommendation are recorded in `human_in_the_loop` above. I recommend preserving the title when tmux is suspected and using the existing explicit direct-transport setting for misleading tmux hints. The alternative is a deliberate, documented acceptance of the title change in exchange for broader automatic discovery. This needs an author decision before judging the implementation against the intended contract.

## Decisions recorded for information

These choices do not independently warrant stopping the review. They remain subject to correctness review after the pane-title decision is resolved:

- **Settled author decisions:** graphics probing joins the existing color-query exchange; hidden tmux uses wrapped confirmation and a transport override; native Windows gains live color/graphics queries, with Windows Terminal Sixel support established by either its reply or executable version. These are already recorded in the specification's Decisions section and need no reaffirmation.
- **Configuration and construction:** empty or non-UTF-8 graphics overrides are invalid. The existing builder supplies probe-disable and supplied-route options, with `BISCUIT_TERMINAL_PROBE=off` for command-line callers. Supplying the startup query outside an active terminal UI or synchronized-output block remains a caller responsibility. See the [adopted plan rules](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/plan.md:621) and [probe decisions](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1015).
- **Terminal identification:** mintty uses `TERM_PROGRAM=mintty`, with `MSYSTEM` considered only when `TERM_PROGRAM` is absent. The `cterm` identity is unverified, and Git Bash in a bare legacy Windows console can be mistaken for mintty. The Windows parent-process lookup stays in biscuit-terminal because sniff lacks the required public interface. See the [route decisions](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:840).
- **tmux routing and placement:** the implementation chooses a format common to relevant viewers, treating unidentified viewers as Kitty-only; it uses current client information rather than potentially stale session environment variables. Known enabled passthrough avoids another query, and placeholder placement requires every viewer to report UTF-8 and truecolor support through tmux. Forced Sixel can use tmux's own renderer without probing when its version/client facts qualify. See the [transport account](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1110), [probe choice](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1143), [placeholder account](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1230), and [forced-Sixel interpretation](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1385).
- **Pixels and fallback:** theme-background padding can remain transparent in a partial final row; PNG bytes pass through whenever no resize is needed. Plain images use half-blocks or alt text, while diagram and shared render-tree callers retain their specified source/placeholder behavior. Half-block output has no independent height cap, and ambiguous-width half-block glyphs in some CJK terminal configurations remain a limitation. See the [sizing decisions](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:666) and [fallback decisions](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1517).
- **Identifiers and input preservation:** placeholder IDs use a nonzero 24-bit process counter. Windows typeahead reinjection shipped, with a documented narrow window in which arriving keys can be reordered. These are recorded in the [placeholder account](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1224) and [Windows query decisions](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1023).

## Deferred work and verification

The ten terminal checks in [verification.md](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/verification.md) were explicitly scheduled for a person after implementation. The log also records that Zellij was unavailable and its live test always skipped; this is information about missing live evidence, not a conclusion here about the code's correctness. Wider performance measurements remain optional work for the author. See the [consolidated records](/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/implementation-log.md:1684).

The specification already excludes image restoration after reattachment, nested multiplexer combinations, multipart iTerm2 transfer, switching diagram engines, and a shared keyboard-input dispatcher. Existing graphical-rule behavior and older Darkmatter documentation inaccuracies were also left outside this implementation. None is being reopened as a new requirement in this triage.

## Human rulings

**2026-10-21 — Hidden tmux: preserve pane titles.** When `TERM` suggests a hidden tmux (`TERM=tmux-*`/`screen-*` with neither `TMUX` nor `STY`), detection sends only the wrapped Kitty query; the direct exchange carries DA1 without the Kitty query (the existing `ProbePlan::direct_da1_only` mechanism), so neither tmux nor GNU screen receives an unwrapped APC. The "OK to the plain query → no tmux after all" route is removed. A session with a misleading `TERM` and no tmux falls back until the user sets `BISCUIT_GRAPHICS_TRANSPORT=direct`; the unanswered-probe fix hint must name both overrides (`=tmux` if a tmux with allow-passthrough on runs between this program and the terminal, `=direct` if none does). The graphics-routes documentation, verification check 8, and the probe-matrix tests are updated to match. A DA2-first identification (tmux `>84`, screen `>83`) that would restore automatic discovery for misleading `TERM` values was considered and deferred until such sessions are reported.

The implementation review can now proceed and establish findings and production readiness.

**2026-10-21 — LV-1: real transparency.** Transparent pixels are left unpainted rather than composited onto a guessed color: a transparent half-block half uses the terminal's default background, and a transparent Sixel pixel is left untouched (the DCS already declares `P2=1`). A background that the terminal did not report is no longer treated as measured; a guess only blends partially transparent edges. The fix is verified live in WezTerm under tmux; if tmux's own Sixel renderer drops the transparency, prefer Kitty through passthrough there when passthrough is on and the terminal supports Kitty.

**2026-10-21 — LV-2: document, keep the route.** tmux's own Sixel stays preferred over passthrough (it scrolls in copy mode and hides on window switch); "cleared when the pane is resized" is added to the documented tmux limits.

**2026-10-21 — LV-4: query the cell size live on native Windows.** The startup exchange on native Windows adds the cell-size query (`CSI 16 t`), so `Terminal::cell_size()` is no longer always `None` there; the 8×16 `CellSize::FALLBACK` is used only when nothing answers. Live evidence: Windows Terminal answered `ESC[6;20;10t` (20 px tall, 10 px wide), matching the size measured from the screenshots.

**2026-10-21 — LV-5: space then carriage return after the image (Windows SSH server route only).** Diagnosis (fits both observations, not yet proven): ConPTY passes the iTerm2 sequence through without modeling it, WezTerm leaves its cursor at the image's right edge, and ConPTY drops the following carriage return because its own cursor is already at column 0. Writing one space and then a carriage return after the image moves ConPTY's cursor off column 0, so the carriage return must reach the terminal; the space lands to the right of the image. Verify live with the same test.

## Live verification (in progress)

Results from the author at real terminals on macOS, `bt` built from this branch.

| Setup | Route reported | Result |
| --- | --- | --- |
| iTerm2 3.7.6, no multiplexer | iTerm2, Direct | Pass: image and flowchart render, flowchart theme matches, clean prompt |
| Terminal.app, no multiplexer | none (half-blocks) | Pass: half-block image, diagram shown as its Mermaid source (as specified) |
| Ghostty, no multiplexer | Kitty, Direct | Pass |
| Ghostty + tmux 3.8 (`-f /dev/null`, passthrough on, `xterm-ghostty:RGB`) | `Kitty (Tmux, Placeholders, Known)` | Pass: transparent; image stays on its rows after a split and after a window switch and return |
| Ghostty + tmux 3.8, passthrough off | none (half-blocks) | Fallback and hint correct; half-blocks sit in a `#1e1e1e` box on a `#282c34` background (LV-1) |
| Kitty 0.49.2 + tmux 3.8 (`-f /dev/null`, passthrough on, `xterm-kitty:RGB`) | `Kitty (Tmux, Placeholders, Known)` | Pass: transparent; image stays on its rows after a split; hidden in a new window and restored on return |
| WezTerm 20260716, no multiplexer | Kitty | Pass: transparent |
| Windows Terminal (`build-win-native`, PowerShell 7.6.6), dark scheme | Sixel, Direct | Image renders, but at 80% of its size with blank rows below (LV-4): `bt image --example` at `--width 16` covers about 13 columns; real cells are 10×20 px against the 8×16 fallback |
| Windows Terminal, dark and light schemes, `bt bar-chart --example` | Sixel, Direct | Theme pass: the dark chart sits on the dark scheme and the light chart on the light scheme, with no mismatched band (Windows Terminal answers OSC 11, so the composite color is measured). Size and blank rows as in LV-4 |
| mintty 3.8.3 (Cygwin64 Terminal, `TERM_PROGRAM=mintty`, no `MSYSTEM`) | `Iterm2 (Direct, Ordinary, Known)`: identified from `TERM_PROGRAM=mintty` | Pass: full-size transparent image, no blank rows, clean prompt. The native `bt.exe` reached the terminal (graphics were emitted, not plain text) |
| WezTerm 20260716 → `ssh build-win-native` (Windows SSH server) | `Iterm2 (Direct, Ordinary, Known)`: "Windows SSH server: the console host passes only the iTerm2 format, which WezTerm (TERM_PROGRAM=WezTerm) renders" | Images render at the intended size (the 30-column image's art spans the same ~20.8 columns as on the Mac; the chart is ~60 columns, the diagram layout's width). The next line starts at the image's right edge: 30 columns after the image, ~68 after the chart (LV-5) |
| WezTerm 20260716 + Zellij 0.45.1 | `Kitty (Direct, Ordinary, Verified)`: Zellij answered the pane's Kitty query | Pass: transparent image and flowchart render. First live Zellij evidence (its L2 test had always skipped) |
| WezTerm + tmux 3.8 (`-f /dev/null`, passthrough on) | `Sixel (Direct, Ordinary, Verified)`: tmux draws Sixel itself | Image renders in a `#1e1e1e` box (LV-1); cleared by a pane split (LV-2); passthrough off has no effect (LV-3). Forcing `BISCUIT_GRAPHICS=kitty` in the same pane gives a transparent image, confirming the box is Sixel compositing |

Both reports under tmux show `Background: #1e1e1e` with `OSC11 BG: no`: the color is the per-app default in `discovery/osc_queries/query.rs`, and `SixelBackground::for_terminal` labels it `Measured`.

Also observed: the author's `~/.config/tmux/tmux.conf` line `set -g allow-passthrough detach-on-destroy` is rejected by tmux and leaves passthrough off; tests used `tmux -f /dev/null` to avoid it. This is host configuration, not a code finding.

Still to run: SSH from inside a local tmux (after the hidden-tmux change lands).
