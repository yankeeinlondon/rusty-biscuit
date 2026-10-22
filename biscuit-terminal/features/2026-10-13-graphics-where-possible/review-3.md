---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: true
human_review_items:
    - |-
        Finish the remaining real-terminal checks in verification.md and record terminal versions and results. Confirm that transparent images have no surrounding box in WezTerm under tmux, Windows Terminal images have the intended size without blank rows, images over the Windows SSH server leave subsequent text at column zero, and probing through SSH from inside tmux preserves the pane title. These are the already-planned visual checks, not new design decisions or blockers for production readiness in this review.
has_blocked_findings: false
blocked: false
reviewed_by: "codex/gpt-6.1-sol"
recurrence: false
created: "2026-10-21T19:19:57-07:00"
spec: "2026-10-13-graphics-where-possible/spec.md"
implemented: false
description: "A **feature** review of `2026-10-13-graphics-where-possible/spec.md`"
feature: "2026-10-13-graphics-where-possible/review-3.md"
previous: "2026-10-13-graphics-where-possible/review-2.md"
---

# Graphics Where Possible — Review 3

**Production ready under this review's criteria.** Both findings from review 2 are verified fixed, including every instance in the repair ledger. No new findings were identified. The human visual checks remain a separate follow-up, as the specification explicitly requires.

This review covers the current working tree, including uncommitted repairs. I read the specification, implementation plan, both earlier reviews and the author's rulings, original implementation evidence, and both repair ledgers before assessing the implementation. Review covered route selection, query safety and background provenance, image dispatch and encoding, tmux placement and transport, diagram sizing/backgrounds, the dependency direction, render-tree integration, and test registration. Production code was not changed during review.

## Previous Finding Verification

The required instances come from the repair's `### Finding Ledger` in [log.md](log.md). The child-level evidence follows this table; completion flags and reported test counts were not used as substitutes for inspecting the code and running tests.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 — The real-tmux placeholder test depends on undeclared RGB support | R2-F1.1–R2-F1.6: Kitty fixture; shared attachment helper; Sixel fixture; passthrough on; passthrough off; WezTerm/Zellij fixtures | **verified fixed** | The Kitty client explicitly supplies `RGB`; the shared helper passes it with client-scoped `-T` and forces UTF-8 with `-u`. The graphics L2 selection passed all six tests. Every sibling fixture was inspected for the prerequisite its assertions depend on. | Nothing in this finding. |
| R2-F2 — Direct Kitty transfers omit the required nonzero image IDs | R2-F2.1–R2-F2.12: four direct dispatch cases; ordinary and placeholder tmux paths, standalone and inline; query identities; render-tree/diagram consumers; graphical rules; legacy helpers | **verified fixed** | Both shared ordinary upload builders allocate a fresh nonzero ID on the first chunk. Public standalone and `TwoColumn` tests cover both direct sizing branches, multiple chunks, quiet mode and distinct IDs across renders. All terminal and visualization L1 tests passed; the tmux upload/grid L2 test also passed. Consumer paths and the allocator were inspected. | Nothing in this finding. |

Review 2 closed the earlier findings: hidden-tmux title changes (HT-1), estimated backgrounds (LV-1), tmux resize documentation (LV-2), WezTerm/tmux expectations (LV-3), Windows cell measurement (LV-4), and Windows SSH cursor placement (LV-5). None was left open to carry into this review. I also checked that the repairs remain in place: hidden-tmux and visible multiplexer plans suppress plain Kitty queries; background provenance reaches Sixel, half-blocks and diagram rasterization; Windows geometry uses the startup answer without starting an exchange from `Terminal::cell_size()`; and the space/carriage-return workaround remains limited to Windows SSH iTerm2 routes. The passing L1 suites exercise the platform-independent parts of these repairs. Native Windows execution was not repeated in this review.

The earlier background finding's conditional change to prefer Kitty if tmux drops Sixel transparency remains untriggered. There is no new live evidence establishing that prerequisite. This review does not reopen the accepted native-Sixel preference or convert the outstanding visual check into an unresolved code finding.

### R2-F1 instance evidence

| Child ID | Site checked | Verification evidence |
| --- | --- | --- |
| R2-F1.1 | Kitty placeholder client | In biscuit-terminal's [tmux graphics test](../../lib/tests/level2/level2_graphics_tmux.rs), `features: Some("RGB")` is explicit. `level2_tmux_kitty_client_gets_unicode_placeholders` passed its route, upload, grid, foreground reset and client-forwarding assertions. |
| R2-F1.2 | Shared `TestClient::attach` | The same file adds `-T` only when the client specification supplies features, and always adds `-u`. The repair uses this existing client-scoped contract; it introduces no global RGB setting. |
| R2-F1.3 | Native Sixel client | Explicit `features: Some("sixel")`; `level2_tmux_draws_sixel_itself_at_the_bottom_of_the_viewport` passed. |
| R2-F1.4 | Passthrough-on fixture | Sets passthrough on and expects an ordinary, unverified route without attached clients. Its wrapped-chunk test passed and requires no placeholder color capability. |
| R2-F1.5 | Passthrough-off fixture | Sets passthrough off and checks fallback plus the setting-specific hint. Its L2 test passed and has no RGB prerequisite. |
| R2-F1.6 | WezTerm and Zellij fixtures | In biscuit-terminal's [real-terminal probe tests](../../lib/tests/level2/level2_graphics_probe_wezterm.rs), neither fixture expects placeholders. WezTerm checks a verified query answer; Zellij uses explicit popup-free configuration and accepts verified support or a reasoned unknown result. Both selected tests passed. The Zellij assertion does not promise that every host configuration will enable graphics. |

### R2-F2 instance evidence

The shared upload implementations are biscuit-terminal's [Kitty builders](../../lib/src/components/terminal_image/kitty.rs); inline dispatch is in [protocol.rs](../../lib/src/components/terminal_image/protocol.rs). The four direct tests below are in [terminal image tests](../../lib/src/components/terminal_image/tests.rs). They render a valid multi-chunk PNG through the public standalone or `TwoColumn` output, assert a nonzero first-chunk ID and quiet mode, check continuation headers, and render again to assert a distinct ID.

| Child ID | Site checked | Verification evidence |
| --- | --- | --- |
| R2-F2.1 | Standalone direct width-only Kitty | `a_standalone_direct_kitty_upload_is_numbered_and_sized_by_width` passed; header has `c=4`, no explicit rows, and the required ID. |
| R2-F2.2 | Inline direct width-only Kitty | `an_inline_direct_kitty_upload_is_numbered_and_sized_by_width` passed through `TwoColumn::render`. |
| R2-F2.3 | Standalone direct explicit-cell Kitty | `a_standalone_direct_wezterm_upload_is_numbered_with_explicit_cells` passed; header retains `c=4,r=2`. |
| R2-F2.4 | Inline direct explicit-cell Kitty | `an_inline_direct_wezterm_upload_is_numbered_with_explicit_cells` passed through `TwoColumn::render`. |
| R2-F2.5 | Standalone tmux ordinary Kitty | Dispatch uses the repaired explicit-cell builder. `without_rgb_the_same_client_gets_an_ordinary_overlay` passed its quiet/nonzero-ID assertions and retains ordinary placement. |
| R2-F2.6 | Inline tmux ordinary Kitty | `a_tmux_kitty_route_wraps_each_chunk_with_an_id_and_explicit_rows` passed, checking wrapping, rows, nonzero ID and a distinct ID on another render. |
| R2-F2.7 | Standalone tmux placeholders | `the_standalone_path_uploads_a_virtual_placement_then_prints_the_grid` passed. The virtual upload and grid continue to share one positive 24-bit ID. Real-tmux L2 also passed upload and color forwarding. |
| R2-F2.8 | Inline tmux placeholders | `the_inline_path_returns_one_text_line_per_image_row` passed, including the ID and matching grid-color assertions. Placeholder uploads still use their dedicated builder with `U=1`. |
| R2-F2.9 | Graphics queries | In biscuit-terminal's [probe wire implementation](../../lib/src/discovery/detection/probe.rs), `kitty_query(id)` preserves the supplied ID; the allocator skips zero and keeps IDs within the placeholder range. Query correlation and allocator tests passed in L1. |
| R2-F2.10 | Render-tree images and diagrams | Source inspection confirms that render-tree images use `render_with_route`; biscuit-visualized's [display adapter](../../../biscuit-visualized/src/src/components/display.rs) feeds that same path, with GitGraph delegating to graph-expression rendering. Public diagram PNG and render-hook tests passed. Equality tests discard only the intentionally variable image ID. |
| R2-F2.11 | Graphical horizontal rules | In biscuit-terminal's [horizontal rule component](../../lib/src/components/horizontal_rule/mod.rs), the graphical tier calls `render_kitty_cells`. `test_render_uses_kitty_image_tier_when_supported` passed its positive-ID assertion. |
| R2-F2.12 | Legacy Kitty helpers | `render_as_kitty` calls `render_kitty_width_only`, and optimistic rendering delegates to it. `test_render_as_kitty_produces_valid_output` and `test_render_kitty_chunking` passed; the latter checks that the identity occurs once and continuations carry only their continuation parameter. |

These are complete repairs of the two original findings. The graphical-rule and legacy-helper rows are additional consumers swept and repaired by the implementer, not newly discovered outstanding siblings.

## Unblocked Findings

None.

## Blocked Findings

None. Pending human visual checks do not independently block readiness.

## Verification Levels and Results

| User-facing requirement | Strongest verification inspected | Assessment |
| --- | --- | --- |
| Known terminal/SSH routes, overrides, invalid values and blocked transports | L1 public route matrices | Appropriate for deterministic route decisions; passed. |
| Supplied terminal snapshots, output policy, nonterminal descriptions and explicit graphics disablement | L1 image/render-tree tests | Appropriate for policy and returned output; passed. |
| Unknown-terminal graphics discovery and correlated replies without shell junk | L1 parser/PTY tests plus L2 real WezTerm query/capture test | Both levels passed. Manufactured replies are not treated as terminal-emulator evidence. |
| Hidden-tmux title preservation | L1 query-plan and PTY tests; existing human SSH checklist | Code obeys the author's ruling. The spec schedules the live SSH/title check separately. |
| tmux passthrough, disabled-passthrough fallback and hints | L2 real tmux raw-output and pane checks | Passed. |
| tmux placeholder upload, UTF-8 cells and 24-bit image-ID color forwarding | L1 encoding/placement tests plus L2 real tmux attached-client output | Passed with explicit RGB/UTF-8 prerequisites. Actual Kitty/Ghostty pixel appearance also has earlier author evidence; remaining visual checks stay on the checklist. |
| tmux-native Sixel placement at the bottom of the viewport | L1 encoding/cursor tests plus L2 real tmux pane checks | Passed; passthrough-off control retained. |
| Zellij query confirmation and fallback | L1 enabled/disabled/version/geometry matrices plus L2 Zellij-in-WezTerm test | Passed within the test's stated verified-or-reasoned-unknown contract. Earlier author checks also record a visible Zellij image and diagram. |
| Transparency and partially transparent edges | L1 decoded Sixel pixels, exact half-block output, provenance tests and public diagram artifacts | Passed. Repaired visual appearance remains an explicitly scheduled human check, rather than an unclaimed L2 pixel proof. |
| Native Windows query transport, cell measurement, typeahead and restored console modes | Registered windowless ConPTY tests with manufactured replies, plus L1 parsing/exchange tests | Test code inspected; prior repair log records Windows execution. Not rerun here and not presented as actual Windows Terminal decoding evidence. Live sizing remains on the human checklist. |
| Windows SSH iTerm2 text placement | L1 route-scoped bytes and public diagram output | Passed for the author's requested workaround; live recheck remains separate. |
| Plain-image fallback and diagram source fallback | L1 public image/policy/render-tree tests; earlier Terminal.app author check | Passed; no pixel promotion without the required capability/policy. |
| Final diagram dimensions, padding, PNG pass-through and background-separated artifacts | L1 public artifact/image tests in both packages | Passed, including standalone and inline transfer paths. |
| Diagram component move and render-tree hook behavior | L1 moved component/parity tests and hook error-policy tests; dependency inspection | Passed. Terminal runtime dependencies omit biscuit-visualized/Mermaid; artifact-only visualization dependencies omit biscuit-terminal. |
| PNG compression improvement | L1 lossless-encoding tests plus the original log's one-host icon measurement | Passed tests; the implementation log records the specification's one-time 400×400 icon comparison. No new performance matrix is required. |

No requirement in this feature handles OS keyboard events, so Level 3 injection is unnecessary. The specification explicitly schedules visual checks after implementation and excludes them from completion criteria. This review preserves that boundary and does not claim that protocol-byte tests prove every terminal's pixel appearance.

Commands run from the worktree's absolute package directories:

- `just test` in biscuit-terminal: **3,856 passed**, 55 skipped by selection.
- `just test` in biscuit-visualized: **318 passed**, 3 skipped by selection.
- `just test-l2 graphics` in biscuit-terminal: **6 passed** in the library; zero CLI tests match this narrowed selection. Windows ConPTY tests are not part of this macOS result.
- `just check-tier-coverage biscuit-terminal` and `just check-tier-coverage biscuit-visualized`: **zero stranded tests**. Inspected declared test targets, module registration, feature requirements and live tier recipes.
- `cargo tree -p biscuit-terminal --features image -e normal --prefix none` and `cargo tree -p biscuit-visualized --no-default-features -e normal --prefix none`: inspected dependency separation.

No file-format or configuration loader was added or changed by this feature, so the input robustness matrix does not apply. Graphics environment overrides use an exact-value validator with public matrices for absent, empty, invalid, padded/case-changed and non-UTF-8 values. Terminal protocol replies have their own parser and PTY coverage.

## Recurrence

No recurring finding. Both review-2 findings are fully repaired, and no unchanged instance, repair regression or missed sibling remains to carry. Accepted decisions from review 1 were not reopened.

## Observations

None newly raised. Earlier observations are not carried as findings.
