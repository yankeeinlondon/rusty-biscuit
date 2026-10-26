---
$schema: feature-review.yaml
next: "2026-10-13-graphics-where-possible/review-3.md"
ready: false
findings:
    - "R2-F1 (high): The real-tmux placeholder test depends on undeclared RGB support"
    - "R2-F2 (high): Direct Kitty transfers omit the required nonzero image IDs"
observations: []
human_review: true
human_review_items:
    - |-
        After the code repairs, finish the real-terminal checks in verification.md and record terminal versions and results. Confirm that transparent images have no surrounding box in WezTerm under tmux, Windows Terminal images have the intended size without blank rows, images over the Windows SSH server leave subsequent text at column zero, and probing through SSH from inside tmux preserves the pane title. These are the already-planned visual checks, not new design decisions or blockers for this repair cycle.
has_blocked_findings: false
blocked: false
reviewed_by: "codex/gpt-6.1-sol"
recurrence: false
created: "2026-10-21T18:58:43-07:00"
spec: "2026-10-13-graphics-where-possible/spec.md"
implemented: true
implemented_by: "claude/opus"
log: "/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/log.md"
description: "A **feature** review of `2026-10-13-graphics-where-possible/spec.md`"
feature: "2026-10-13-graphics-where-possible/review-2.md"
previous: "2026-10-13-graphics-where-possible/review-1.md"
---

# Graphics Where Possible — Review 2

All six previous findings are repaired against their recorded requirements. Two new high findings prevent readiness: a required real-tmux test fails because its client fixture does not establish RGB support, and direct Kitty transfers omit the image IDs explicitly required by the specification. Neither is a partial repair or a newly found sibling of a previous finding.

This review covers the current working tree, including uncommitted repairs. I read the specification, plan, original implementation log, previous review and human rulings, and repair Finding Ledger before examining the implementation. Reviewed areas include routing, queries, encoders and both dispatch paths, diagram sizing/background adapters, render-tree integration, documentation, and test registration. No production code was changed during review.

## Previous Finding Verification

Required instances below are the child rows in the repair log's Finding Ledger. Completion flags were not treated as proof. Windows execution results are attributed to that log: I inspected the implementation and tests but did not repeat its remote runs.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| HT-1 — Hidden-tmux probing changes pane titles | HT-1.1–HT-1.7: hidden hints; plain-OK route removal; both transport hints; docs; visible screen; visible tmux/direct override; other plans and hint consumers | **verified fixed** | In biscuit-terminal, [route resolution](../../lib/src/discovery/detection/route.rs) sets `direct_da1_only` for hidden tmux and applies `without_apc_under_multiplexer` to visible tmux/screen. Hidden-tmux resolution accepts only wrapped confirmation and names both overrides on failure. L1 plan and PTY tests passed. Checked declared tmux, forced format, CI, probe-off, Zellij, OSC-only queries, and CLI hint forwarding. Docs and verification check 8 match the author's title-preservation ruling. | Existing live hidden-tmux checklist item; no outstanding code instance. |
| LV-1 — Estimated backgrounds paint a box | LV-1.1–LV-1.7: provenance; half-blocks; Sixel; conditional Kitty preference; diagnostics; diagrams; PNG/padding and remaining consumers | **verified fixed** | In biscuit-terminal, [color resolution](../../lib/src/discovery/osc_queries/query.rs) labels only live replies measured; `Terminal` preserves that source. Passing [half-block tests](../../lib/src/components/terminal_image/half_block/tests.rs) cover transparent top/bottom/both halves, odd rows and partial alpha. Passing [Sixel tests](../../lib/src/components/terminal_image/sixel/tests.rs) decode unset transparent pixels/padding, opaque pixels and blended edges. In biscuit-visualized, [display background selection](../../../biscuit-visualized/src/src/components/display.rs) preserves transparency when unmeasured; its public diagram tests passed. Checked diagnostics, PNG transfer, padding, render-tree background propagation and theme selection. | LV-1.4 is an untriggered contingency: prefer Kitty only if a live check shows tmux itself dropping Sixel transparency. That prerequisite is not established. Finish the existing visual check. |
| LV-2 — Missing tmux Sixel resize limitation | Route docs, skill and verification checklist | **verified fixed** | [Graphics-route documentation](../../docs/discovery/graphics-routes.md) explicitly says resize/split clears tmux's Sixel images and they do not return. Verification check 7 and the terminal skill agree. The accepted choice keeps this route. | Nothing. |
| LV-3 — Incorrect WezTerm-under-tmux expectation | Native Sixel precedence; passthrough on/off; forced Kitty alternative | **verified fixed** | Topic page and verification check 7 distinguish tmux builds with/without Sixel. L1 matrix passed for native Sixel with passthrough on/off and forced Kitty. L2 native Sixel passed with passthrough off. | Nothing. R2-F1 concerns a different test and placement decision. |
| LV-4 — Native Windows never measures cell size | LV-4.1–LV-4.3: startup measurement; probe-off; SSH/legacy exclusions; Unix/multiplexer paths and geometry consumers | **verified fixed** | In biscuit-terminal, [startup exchange](../../lib/src/discovery/startup_exchange.rs) sends `CSI 16 t` on Windows before DA1 and caches the answer; busy-input retry retains the cell request. [Terminal geometry access](../../lib/src/terminal.rs) reads only the received answer on Windows, preventing a render-time probe. Checked eligibility, parser bounds, errors/silence, Unix paths and image/diagram consumers. Parser/exchange L1 tests passed. Registered Windows ConPTY tests assert 10×20 cells, unanswered → `None`, and probe-off → no measurement. The repair log reports six console tests passing after rebuilding the probe. | Existing Windows Terminal visual check. Cross-OS execution evidence does not block readiness. |
| LV-5 — Text after Windows SSH images starts at their right edge | LV-5.1–LV-5.3: route-scoped space/CR; standalone/inline/encoded/render-tree/diagram/CLI consumers; other formats and legacy helpers | **verified fixed** | In biscuit-terminal, only the Windows SSH iTerm2 route sets `needs_cursor_resync`; [protocol dispatch](../../lib/src/components/terminal_image/protocol.rs) adds space/CR on standalone and inline paths. Passing tests include route selection, direct-iTerm2 negative control and inline parity. In biscuit-visualized, the Mermaid Windows-SSH output test passed. Other listed production consumers share these paths; forced Kitty/Sixel remain blocked. Legacy width-only helpers have no route or production caller in this flow. | The author requested a live recheck of the proposed workaround. This remains a human checklist item, not an unrepaired code instance. |

## Unblocked Findings

### R2-F1 (high): The real-tmux placeholder test depends on undeclared RGB support

**Authority: spec.** Section 3 requires placeholders when the outer terminal is identified and has the required Unicode/color fidelity. Verification requires “L2: tmux through the shared test harness” and placeholder-grid coverage. The placeholder test fails on an ordinary supported setup: tmux 3.8 with an available `xterm-kitty` terminfo entry.

**Defect class:** a real-terminal fixture asserts a capability-dependent outcome without establishing the capability that selects it.

In biscuit-terminal's [placeholder test](../../lib/tests/level2/level2_graphics_tmux.rs:342), the client declares `TERM=xterm-kitty` but passes `features: None`. Its shared attachment helper forces UTF-8 with `-u`, but supplies no RGB feature unless the host happens to infer it. The library correctly refuses placeholders without RGB. The test fails expecting `Placeholders` instead of `Ordinary`, before reaching its upload/grid assertions.

Reproduction: `just test-l2 graphics` in biscuit-terminal ran six tests; five passed and this one failed at line 358. The captured reason says “placed ordinarily, because Unicode placeholders need tmux to send UTF-8 and 24-bit color (the RGB terminal feature) to every client.” Changing only this fixture to `features: Some("RGB")`, then running `just test-l2 kitty_client_gets_unicode_placeholders`, passed its upload, grid, color and forwarding assertions. Original source was restored afterward. This was a changed-fixture control, not a retry of the failing setup.

The sweep covers every graphics L2 client fixture and the shared attachment helper:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Kitty placeholder client | `xterm-kitty`, forced UTF-8, no explicit features | **Defect:** ordinary placement; assertion fails | Explicit RGB/UTF-8 prerequisite, then placeholder forwarding |
| R2-F1.2 | Shared `TestClient::attach` | Optional feature input | **Clean helper contract:** sends `-u`; sends `-T` only when supplied; RGB control passes | Client-scoped feature declaration |
| R2-F1.3 | Native Sixel client | `features: Some("sixel")` | **Clean:** bottom-of-viewport test passes | Explicit capability prerequisite |
| R2-F1.4 | Passthrough on | No attached client | **Clean:** ordinary unverified Kitty and wrapped chunks; passes | No placeholder/RGB expectation |
| R2-F1.5 | Passthrough off | No attached client | **Clean:** fallback and setting-specific hint; passes | No placeholder/RGB expectation |
| R2-F1.6 | WezTerm and Zellij probe fixtures | Real WezTerm; Zellij with explicit test config | **Clean:** both pass; neither requests placeholders | Verify actual query replies |

Repair the Kitty fixture with client-scoped `-T RGB` through its existing `ClientSpec` field. Keep the production fallback, which protects image-ID colors when tmux would degrade them. No global tmux setting or new abstraction is needed.

### R2-F2 (high): Direct Kitty transfers omit the required nonzero image IDs

**Authority: spec.** Section 2 explicitly states: “Ordinary Kitty transfers use quiet mode and nonzero IDs.” The new dispatch honors this for tmux transfers but omits IDs on direct transfers. This is an unmet explicit contract; it is not evidence that today's direct images are invisible.

**Defect class:** ordinary uploads apply their required identity parameter only on one transport branch.

In biscuit-terminal, [standalone Kitty rendering](../../lib/src/components/terminal_image/kitty.rs:130) uses `render_kitty_cells` for direct WezTerm and `render_kitty_width_only` for other direct terminals. Both write `q=2` without `i=`. [Inline dispatch](../../lib/src/components/terminal_image/protocol.rs:334) repeats these choices.

I exercised the public `TerminalImage::render_with_route` and `TwoColumn::render` APIs with the same valid 4×4 PNG, four-column image width, fixed 8×16 cells and supplied terminal snapshots. Only app/transport/placement changed. The temporary review example was removed afterward. Direct headers were:

```text
direct Kitty, standalone: q=2,f=100,a=T,t=d,c=4,m=0
direct Kitty, inline: q=2,f=100,a=T,t=d,c=4,m=0
direct WezTerm, standalone: q=2,f=100,a=T,t=d,c=4,r=2,m=0
direct WezTerm, inline: q=2,f=100,a=T,t=d,c=4,r=2,m=0
```

The same reproduction on tmux routes included distinct positive IDs on all four uploads. The sweep follows identity through both dispatch decisions and their consumers:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Standalone direct width-only Kitty | Kitty, ordinary direct route | **Defect:** no `i=` | Quiet upload with nonzero ID |
| R2-F2.2 | Inline direct width-only Kitty | Same route in `TwoColumn` | **Defect:** no `i=` | Same identity contract |
| R2-F2.3 | Standalone direct explicit-cell Kitty | WezTerm, ordinary direct route | **Defect:** no `i=` | Same identity contract |
| R2-F2.4 | Inline direct explicit-cell Kitty | Same route in `TwoColumn` | **Defect:** no `i=` | Same identity contract |
| R2-F2.5 | Standalone tmux ordinary Kitty | Same PNG, tmux transport | **Clean:** positive ID, quiet, explicit rows | Retain nonzero ID |
| R2-F2.6 | Inline tmux ordinary Kitty | Same route in `TwoColumn` | **Clean:** distinct positive ID | Retain nonzero ID |
| R2-F2.7 | Standalone tmux placeholders | Same PNG, placeholder placement | **Clean:** positive ID and `U=1` | Retain upload/grid identity |
| R2-F2.8 | Inline tmux placeholders | Same route in `TwoColumn` | **Clean:** distinct positive ID and `U=1` | Retain upload/grid identity |
| R2-F2.9 | Graphics query | Existing API and L1 tests | **Clean:** `kitty_query(id)` includes its supplied ID; fresh/nonzero counter tests pass | Preserve correlated query identities |
| R2-F2.10 | Render-tree and diagram consumers | Source invariant: render-tree images use `render_with_route`; Mermaid/graph expressions use `display_image(...).render_with_route`; GitGraph delegates to graph expressions | **Affected:** direct output shares the deficient standalone dispatch; no consumer assigns the missing ID afterward | Repair shared upload paths so consumers inherit the contract |

Use the existing image-ID allocator for direct uploads, preserving chunk continuation semantics and placeholder linkage. Add L1 assertions through standalone and inline public output for both direct sizing branches. This needs no new discovery or identity mechanism.

## Blocked Findings

None. Pending human visual checks do not block either repair or independently make `ready` false.

## Verification Levels and Results

| User-facing requirement | Verification inspected | Assessment |
| --- | --- | --- |
| Route choice, overrides/errors, blocked Windows SSH formats, supplied snapshots and output policy | L1 public route matrices and render-tree/image tests | Passed; these decisions do not need terminal decoding |
| Unknown-terminal discovery and no reply junk | L1 fragmented/correlated-reply PTY tests; L2 real WezTerm replies and capture | Passed |
| Hidden-tmux title preservation and hints | L1 query-plan/PTY checks; updated human SSH checklist | Code repair verified; real SSH/title check remains scheduled |
| tmux passthrough on/off and fallback hints | L2 real tmux raw output and pane capture | Passed |
| tmux placeholders and image-ID color forwarding | L1 encoding/placement matrices; L2 real tmux/client forwarding | **Gap:** original L2 fixture fails, R2-F1; explicit-RGB control passes |
| Native Sixel and text placement at viewport bottom | L1 encoder/cursor tests; L2 real tmux pane capture | Passed |
| Zellij pane graphics confirmation | L1 route matrix; L2 real Zellij inside WezTerm | Passed; no longer relying solely on a skipped test |
| Transparency, partial alpha and background provenance | L1 decoded Sixel pixels, half-block bytes, color-source and public diagram tests | Passed; repaired visual appearance remains on the existing human checklist |
| Windows colors, cell measurement, mode restoration and typeahead | Windowless ConPTY tests with manufactured replies; L1 parsing | Tests check Windows transport/public results. Repair log reports six console tests passing; actual Windows Terminal sizing/theme remains a human check |
| Windows SSH iTerm2 cursor placement | L1 route-scoped bytes and public diagram output | Matches author's requested workaround; live recheck remains scheduled |
| Plain-image fallback, nonterminal descriptions, diagram source and no-hook Mermaid behavior | L1 public output/policy tests; prior author Terminal.app check | No new gap found; transparency also has exact-byte tests |
| Diagram final size, aspect ratio/padding, unchanged PNG transfer, background-separated artifacts and moved components | L1 public artifact/render tests and render-tree tests | Passed; direct upload identity still fails under R2-F2 |

No requirement consumes OS keyboard events, so Level 3 injection is unnecessary. A ConPTY test's tier does not turn manufactured replies into evidence from a terminal emulator. The spec expressly places human checks after implementation and excludes them from completion criteria; pending human checks are not additional blocking findings.

Commands run:

- `just test` in biscuit-terminal: **3,852 passed**, 55 skipped by selection.
- `just test` in biscuit-visualized: **318 passed**, 3 skipped by selection.
- `just test-l2 graphics` in biscuit-terminal: **5 passed, 1 failed** (R2-F1). CLI package: no tests matching this narrowed filter.
- Explicit-RGB fixture control, `just test-l2 kitty_client_gets_unicode_placeholders`: **1 passed**; source restored.
- `just check-tier-coverage biscuit-terminal` and `just check-tier-coverage biscuit-visualized`: **zero stranded tests**. Checked declared targets, module registration, live tier recipes and CI feature selection.
- Temporary public-API upload experiment: eight route/placement outputs inspected under R2-F2; example removed.

This work adds no configuration-file loader, so the input robustness matrix does not apply. Environment overrides use a shared exact-value validator; public matrices cover absent, empty, invalid/case/padded and non-UTF-8 inputs. Terminal replies are covered by protocol/PTY tests rather than a configuration document matrix.

## Recurrence

Neither new finding repeats a review-1 defect class. Previous findings concern unsafe probing, background compositing, documented tmux behavior, missing Windows geometry measurement and Windows SSH cursor placement. R2-F1 concerns a fixture prerequisite; R2-F2 concerns direct upload identity. No earlier open findings remain.

## Observations

None newly raised. Review 1's observations are not carried as findings; accepted choices and documented limitations were not reopened.
