---
implementation_1: "2026-10-23T15:04:41-07:00"
implementation_2: "2026-10-23T15:17:35-07:00"
---

# Merman Review/Fix Log

## Implementation of Review Findings #1

> **started at:** 2026-10-23T15:04:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-merman/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- set the review's `implemented` flag to `false` before inventory (it was already `false`)
- inventory of `review-1.md` at 15:05:04:
        - frontmatter `findings: []`; the body states "No code-review findings were produced in this decision-only pass"
        - `readiness_assessed: false`, `human_review: true`, `human_resolved: false`: the review stopped at the author-decision stage required by the spec's first-review approval rule
        - the six `human_in_the_loop` entries are author decisions (gallery approval, four styling departures, sequence-box contrast, render-time limit, verification gates, graph-cache font identity); each offers options with different user-visible outcomes and is reserved for the author, so none is a finding this cycle may decide
        - the body's "Deferred work" table reports log-recorded items (stale Darkmatter Mermaid comments, root dependency-document links, biscuit-terminal all-features build, worktree Kitty test) that the reviewer explicitly did not reproduce or classify as findings; they carry forward to the implementation review that follows the human rulings

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| — | No numbered findings in `review-1.md` (`findings: []`) | clean | Frontmatter `findings: []`; body section "Findings and continuation" confirms none were issued |
| R1-H1 | Author approval of current diagram galleries | not a finding (author decision) | Awaiting `## Human rulings`; an agent may not substitute its visual judgment (review body) |
| R1-H2 | Retain or revert four styling-rule departures | not a finding (author decision) | Awaiting `## Human rulings` |
| R1-H3 | Fix now or accept exception for sequence-box message contrast | not a finding (author decision) | Awaiting `## Human rulings`; repair belongs to the cycle after the ruling |
| R1-H4 | Render-time limit overrun (sequence ~56 ms, timeline ~84 ms) | not a finding (author decision) | Awaiting `## Human rulings`; no performance measurement required this cycle |
| R1-H5 | Close of dependent-package and terminal verification gates | not a finding (author decision) | Awaiting `## Human rulings` |
| R1-H6 | Graph cache key lacks font identity | not a finding (author decision) | Review records it unreproduced and offers "repair next cycle" vs "accept as separate repair"; implementing it now would pre-empt that choice |

### Completion Report

- **outcome:** complete (no findings to repair); elapsed about 3 minutes
- parent findings: total 0 = fixed 0 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- no subagents were dispatched: there was no finding to assign
- no source, test, or documentation files changed in this cycle; the only changed files are this log and the review's completion metadata
- verification: no code changed, so the area's `just test` was not rerun; `just lint` in `biscuit-visualized` passed (exit 0, `--all-features`)
- open items for the next stage (not findings of this review): R1-H1 through R1-H6 await the author's `## Human rulings` and `human_resolved: true`, after which the implementation review assesses the code

## Implementation of Review Findings #2

> **started at:** 2026-10-23T15:17:35-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-merman/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- review `implemented` was already `false`; left `false` before repairs
- inventory: 4 parent findings (R2-F1…R2-F4), all unblocked; `recurrence: true` names R2-F1, R2-F2, R2-F3 as previously disclosed classes; human review items (gallery approval, four styling choices) are author decisions, not findings

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Library-selected text becomes unreadable on author-selected fills | fixed | all 39 children verified; class swept across every family fixture and both output branches |
| R2-F1.1 | `agentflow` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `agentflow` white-primaryColor test passes; min library-text contrast 12.635:1; review observed: 0 control / 2 edited failing texts; minimum 1.606:1 |
| R2-F1.2 | `architecture` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `architecture` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.3 | `block` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `block` white-primaryColor test passes; min library-text contrast 4.715:1; review observed: 0 control / 5 edited failing texts; minimum 1.606:1 |
| R2-F1.4 | `c4` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `c4` white-primaryColor test passes; min library-text contrast 5.617:1 (author red/blue text kept); review observed: 5 control / 14 edited failing texts; minimum 1.012:1 |
| R2-F1.5 | `class` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `class` white-primaryColor test passes; min library-text contrast 12.635:1; review observed: 0 control / 15 edited failing texts; minimum 1.606:1 |
| R2-F1.6 | `cynefin` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `cynefin` white-primaryColor test passes; min library-text contrast 5.134:1; review observed: 0 control / 7 edited failing texts; minimum 1.282:1 |
| R2-F1.7 | `er` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `er` white-primaryColor test passes; min library-text contrast 10.123:1; review observed: 0 control / 26 edited failing texts; minimum 1.287:1 |
| R2-F1.8 | `eventmodeling` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `eventmodeling` white-primaryColor test passes; min library-text contrast 7.398:1; review observed: 0 control / 12 edited failing texts; minimum 1.063:1 |
| R2-F1.9 | `flowchart` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `flowchart` white-primaryColor test passes; min library-text contrast 5.695:1; review observed: 0 control / 6 edited failing texts; minimum 1.606:1 |
| R2-F1.10 | `gantt` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `gantt` white-primaryColor test passes; min library-text contrast 5.630:1; review observed: 0 control / 16 edited failing texts; minimum 1.606:1 |
| R2-F1.11 | `gitgraph` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `gitgraph` white-primaryColor test passes; min library-text contrast 5.333:1; review observed: 0 control / 3 edited failing texts; minimum 1.606:1 |
| R2-F1.12 | `info` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `info` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.13 | `ishikawa` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `ishikawa` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 5 edited failing texts; minimum 1.606:1 |
| R2-F1.14 | `journey` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `journey` white-primaryColor test passes; min library-text contrast 5.695:1; review observed: 0 control / 4 edited failing texts; minimum 1.606:1 |
| R2-F1.15 | `kanban` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `kanban` white-primaryColor test passes; min library-text contrast 7.005:1; review observed: 0 control / 5 edited failing texts; minimum 4.362:1 |
| R2-F1.16 | `mindmap` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `mindmap` white-primaryColor test passes; min library-text contrast 4.674:1; review observed: 0 control / 1 edited failing texts; minimum 2.910:1 |
| R2-F1.17 | `packet` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `packet` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 31 edited failing texts; minimum 1.662:1 |
| R2-F1.18 | `pie` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `pie` white-primaryColor test passes; min library-text contrast 5.826:1; review observed: 0 control / 2 edited failing texts; minimum 1.109:1 |
| R2-F1.19 | `quadrantchart` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `quadrantchart` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 10 edited failing texts; minimum 1.606:1 |
| R2-F1.20 | `radar` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `radar` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.21 | `railroad` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `railroad` white-primaryColor test passes; min library-text contrast 4.560:1; review observed: 0 control / 6 edited failing texts; minimum 1.606:1 |
| R2-F1.22 | `requirement` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `requirement` white-primaryColor test passes; min library-text contrast 6.770:1; review observed: 0 control / 9 edited failing texts; minimum 1.162:1 |
| R2-F1.23 | `sankey` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `sankey` white-primaryColor test passes; min library-text contrast 8.558:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.24 | `sequence` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `sequence` white-primaryColor test passes; min library-text contrast 5.954:1 (box included); review observed: 13 control / 23 edited failing texts; minimum 1.073:1 |
| R2-F1.25 | `state` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `state` white-primaryColor test passes; min library-text contrast 12.635:1; review observed: 0 control / 9 edited failing texts; minimum 1.606:1 |
| R2-F1.26 | `swimlane` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `swimlane` white-primaryColor test passes; min library-text contrast 5.695:1; review observed: 0 control / 6 edited failing texts; minimum 1.606:1 |
| R2-F1.27 | `timeline` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `timeline` white-primaryColor test passes; min library-text contrast 5.347:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.28 | `treeView` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `treeView` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 11 edited failing texts; minimum 1.662:1 |
| R2-F1.29 | `treemap` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `treemap` white-primaryColor test passes; min library-text contrast 6.975:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.30 | `usecase` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `usecase` white-primaryColor test passes; min library-text contrast 5.695:1; review observed: 0 control / 9 edited failing texts; minimum 1.606:1 |
| R2-F1.31 | `venn` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | fixed | `venn` white-primaryColor test passes; min library-text contrast 6.811:1 (author #333/red kept); review observed: 1 control / 3 edited failing texts; minimum 1.051:1 |
| R2-F1.32 | `wardley` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `wardley` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.33 | `xychart` — Unedited / white `primaryColor`; expected: Library-selected text ≥4.5:1; preserve explicit author text choices | clean | `xychart` white-primaryColor test passes; min library-text contrast 7.868:1; review observed: 0 control / 0 edited failures; clean |
| R2-F1.34 | Sequence golden, Dark — Existing `box LightGrey`, no added directive; expected: Correct library-selected text on the light box | fixed | exemption removed; box texts/tspans now #333333; `contrast_corpus_tests` passes without it |
| R2-F1.35 | Sequence golden, terminal ANSI palette — Same existing box; expected: Correct text; remove the exemption | fixed | exemption removed; box text #1a1b26; oracle passes |
| R2-F1.36 | Sequence golden, background-only terminal palette — Same existing box; expected: Correct text; remove the exemption | fixed | exemption removed; box text #1a1b26; oracle passes |
| R2-F1.37 | Sequence golden, Default/Forest/Neutral — Same existing box; expected: Stay readable | clean | goldens unchanged; corpus oracle passes |
| R2-F1.38 | SVG export, PNG raster input, HTML static output — Shared `render_svg` result; expected: Repair before the paths diverge | fixed | `render_svg` runs the pass unconditionally before RasterSafe/HtmlStatic/PNG split (render.rs diff) |
| R2-F1.39 | Contrast-test classification — Library-selected text versus explicit author text; expected: Keep only explicit author text exemptions; assert this demonstrated defect fails until repaired | fixed | only explicit-author-text entries remain in `ALLOWED`/`AUTHOR_TEXT`; 25 new tests fail on pre-fix code |
| R2-F2 | Graph PNG cache ignores the fonts used to draw text | fixed | all 7 children verified; sibling sweep found no other cached raster path |
| R2-F2.1 | Expression → PNG — Inter → Arial, same cache; expected: A font change misses and returns the Arial render | fixed | `expression_png_cache_follows_the_font_chain`: key differs, miss, bytes equal fresh render with new chain |
| R2-F2.2 | DOT → PNG — Same font change; expected: Same font-sensitive invalidation | fixed | `dot_png_cache_follows_the_font_chain` (dark theme font_family too) |
| R2-F2.3 | Expression → SVG — Same font change; expected: Font-independent SVG may reuse its key | clean | `graph_svg_cache_ignores_the_font_chain`: same key, hit |
| R2-F2.4 | DOT → SVG — Same font change; expected: Same | clean | same test covers DOT SVG |
| R2-F2.5 | Mermaid → PNG/SVG — Resolved-font identity; expected: Retain font-sensitive identity | clean | untouched; `cache_key_for` hashes `chain.cache_identity` |
| R2-F2.6 | GraphExpression terminal component and graph render-tree hook — Same graph source/request; expected: Shared graph cache fix reaches both consumers | fixed | GraphExpression delegates to `GraphDiagram::render` → `render_in(default_chain())`; render hook renders Mermaid only (already keyed) |
| R2-F2.7 | Standalone SVG rasterization API — Same SVG, different resolved font; expected: No new cache layer needed | clean | public `rasterize_svg*` have no cache lookup |
| R2-F3 | Recorded sequence and timeline renders exceed the warm limit | fixed | sequence 33.81 ms, timeline 40.76 ms warm on build-linux; full 33-family enforced run 'no rule fired' |
| R2-F3.1 | `agentflow` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (6.192 / 15.546; clean) and in the new enforced build-linux baseline run |
| R2-F3.2 | `architecture` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (19.756 / 30.841; clean) and in the new enforced build-linux baseline run |
| R2-F3.3 | `block` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (11.998 / 22.818; clean) and in the new enforced build-linux baseline run |
| R2-F3.4 | `c4` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (39.888 / 52.161; clean) and in the new enforced build-linux baseline run |
| R2-F3.5 | `class` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (22.601 / 35.038; clean) and in the new enforced build-linux baseline run |
| R2-F3.6 | `cynefin` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (25.515 / 39.250; clean) and in the new enforced build-linux baseline run |
| R2-F3.7 | `er` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (29.677 / 41.843; clean) and in the new enforced build-linux baseline run |
| R2-F3.8 | `eventmodeling` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (45.301 / 59.216; clean) and in the new enforced build-linux baseline run |
| R2-F3.9 | `flowchart` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (27.252 / 41.678; clean) and in the new enforced build-linux baseline run |
| R2-F3.10 | `gantt` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (13.328 / 23.578; clean) and in the new enforced build-linux baseline run |
| R2-F3.11 | `gitgraph` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (13.369 / 23.275; clean) and in the new enforced build-linux baseline run |
| R2-F3.12 | `info` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (2.444 / 11.613; clean) and in the new enforced build-linux baseline run |
| R2-F3.13 | `ishikawa` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (19.068 / 30.318; clean) and in the new enforced build-linux baseline run |
| R2-F3.14 | `journey` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (25.375 / 39.705; clean) and in the new enforced build-linux baseline run |
| R2-F3.15 | `kanban` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (42.078 / 54.186; clean) and in the new enforced build-linux baseline run |
| R2-F3.16 | `mindmap` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (38.503 / 53.306; clean) and in the new enforced build-linux baseline run |
| R2-F3.17 | `packet` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (19.607 / 31.932; clean) and in the new enforced build-linux baseline run |
| R2-F3.18 | `pie` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (18.860 / 30.379; clean) and in the new enforced build-linux baseline run |
| R2-F3.19 | `quadrantchart` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (14.551 / 25.100; clean) and in the new enforced build-linux baseline run |
| R2-F3.20 | `radar` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (27.051 / 39.555; clean) and in the new enforced build-linux baseline run |
| R2-F3.21 | `railroad` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (14.008 / 24.892; clean) and in the new enforced build-linux baseline run |
| R2-F3.22 | `requirement` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (13.511 / 23.795; clean) and in the new enforced build-linux baseline run |
| R2-F3.23 | `sankey` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (20.807 / 33.109; clean) and in the new enforced build-linux baseline run |
| R2-F3.24 | `sequence` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | fixed | build-linux warm 33.81 / cold 49.22 ms (was 56.381 / 73.124) |
| R2-F3.25 | `state` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (26.657 / 38.681; clean) and in the new enforced build-linux baseline run |
| R2-F3.26 | `swimlane` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (28.770 / 43.657; clean) and in the new enforced build-linux baseline run |
| R2-F3.27 | `timeline` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | fixed | build-linux warm 40.76 / cold 57.53 ms (was 84.306 / 98.032) |
| R2-F3.28 | `treeView` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (8.444 / 18.014; clean) and in the new enforced build-linux baseline run |
| R2-F3.29 | `treemap` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (25.792 / 37.214; clean) and in the new enforced build-linux baseline run |
| R2-F3.30 | `usecase` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (22.926 / 33.955; clean) and in the new enforced build-linux baseline run |
| R2-F3.31 | `venn` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (18.855 / 31.310; clean) and in the new enforced build-linux baseline run |
| R2-F3.32 | `wardley` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (28.425 / 43.362; clean) and in the new enforced build-linux baseline run |
| R2-F3.33 | `xychart` — Recorded uncached full render; expected: Warm ≤50 ms; cold <500 ms | clean | under limits in old baseline (14.700 / 26.643; clean) and in the new enforced build-linux baseline run |
| R2-F4 | Level-1 rendering tests exceed the five-second budget | fixed | final `just test`: 976 passed in 13.1 s, slowest 1.202 s; no marker/skip/timeout used |
| R2-F4.1 | `biscuit-visualized components::git_graph::tests::every_emitted_graph_parses` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.859 s |
| R2-F4.2 | `biscuit-visualized components::git_graph::tests::measured_plans_place_merges_and_tags_without_overlap` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 1.097 s |
| R2-F4.3 | `biscuit-visualized components::git_graph::tests::measured_sizes_trim_to_the_width_cap` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.903 s |
| R2-F4.4 | `biscuit-visualized components::git_graph::tests::render_without_notes_leaves_the_notes_to_the_caller` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.925 s |
| R2-F4.5 | `biscuit-visualized components::mermaid::tests::a_render_over_its_time_budget_shows_the_code_block_with_a_warning` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.883 s |
| R2-F4.6 | `biscuit-visualized components::mermaid::tests::diagram_try_render_populates_width_cells` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.518 s |
| R2-F4.7 | `biscuit-visualized components::mermaid::tests::scaled_columns_come_from_the_measured_svg_width` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.517 s |
| R2-F4.8 | `biscuit-visualized tests::contrast_corpus_tests::sequence_text_is_readable` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.873 s |
| R2-F4.9 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_inherit_is_measured_as_drawn` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.807 s |
| R2-F4.10 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_not_a_size_is_measured_as_drawn` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.812 s |
| R2-F4.11 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_relative_to_an_overridden_base_is_measured_as_drawn` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.817 s |
| R2-F4.12 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_unitless_string_is_measured_as_drawn` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.837 s |
| R2-F4.13 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_unset_is_measured_as_drawn` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.829 s |
| R2-F4.14 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_zero_is_measured_as_drawn` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.834 s |
| R2-F4.15 | `biscuit-visualized tests::gitgraph_tests::the_spaced_layout_is_the_measured_one` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.533 s |
| R2-F4.16 | `biscuit-visualized tests::html_static_tests::default_target_svg_keeps_original_fonts_and_clipping` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.809 s |
| R2-F4.17 | `biscuit-visualized tests::html_static_tests::html_static_target_svg_leads_label_fonts_with_the_measured_face` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.855 s |
| R2-F4.18 | `biscuit-visualized tests::html_static_tests::png_render_rasterizes_the_raster_safe_svg_under_the_html_static_target` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.852 s |
| R2-F4.19 | `biscuit-visualized tests::measure_tests::measured_faces_are_drawn_faces_for_the_wrap_corpus_on_host_fonts` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.827 s |
| R2-F4.20 | `biscuit-visualized tests::measure_tests::oracle_widths_match_usvg_for_the_mixed_script_corpus` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 1.202 s |
| R2-F4.21 | `biscuit-visualized tests::mermaid_cache_key_tests::the_key_follows_the_rewritten_source` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.573 s |
| R2-F4.22 | `biscuit-visualized tests::mermaid_cache_key_tests::the_key_is_checked_before_any_merman_svg_call` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.587 s |
| R2-F4.23 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_agentflow::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.122 s |
| R2-F4.24 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_agentflow::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.091 s |
| R2-F4.25 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_c4::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.472 s |
| R2-F4.26 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_c4::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.376 s |
| R2-F4.27 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_class::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.285 s |
| R2-F4.28 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_class::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.153 s |
| R2-F4.29 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_cynefin::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.167 s |
| R2-F4.30 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_cynefin::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.141 s |
| R2-F4.31 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_er::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.425 s |
| R2-F4.32 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_er::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.175 s |
| R2-F4.33 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_event_modeling::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.244 s |
| R2-F4.34 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_event_modeling::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.197 s |
| R2-F4.35 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_flowchart::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.239 s |
| R2-F4.36 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_flowchart::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.142 s |
| R2-F4.37 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gantt::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.250 s |
| R2-F4.38 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gantt::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.199 s |
| R2-F4.39 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gitgraph::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.252 s |
| R2-F4.40 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gitgraph::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.209 s |
| R2-F4.41 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_info::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.056 s |
| R2-F4.42 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_info::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.048 s |
| R2-F4.43 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_ishikawa::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.206 s |
| R2-F4.44 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_ishikawa::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.128 s |
| R2-F4.45 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_journey::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.192 s |
| R2-F4.46 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_journey::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.135 s |
| R2-F4.47 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_kanban::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.912 s |
| R2-F4.48 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_kanban::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.584 s |
| R2-F4.49 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_mindmap::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.915 s |
| R2-F4.50 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_mindmap::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.591 s |
| R2-F4.51 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_quadrant_chart::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.135 s |
| R2-F4.52 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_railroad::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.181 s |
| R2-F4.53 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_requirement::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.275 s |
| R2-F4.54 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_sequence::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.435 s |
| R2-F4.55 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_state::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.394 s |
| R2-F4.56 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_swimlane::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.304 s |
| R2-F4.57 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_timeline::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.524 s |
| R2-F4.58 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_timeline::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.354 s |
| R2-F4.59 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_tree_view::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.103 s |
| R2-F4.60 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_tree_view::svg` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.080 s |
| R2-F4.61 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_usecase::png` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.335 s |
| R2-F4.62 | `biscuit-visualized tests::mermaid_tests::an_svg_render_over_its_time_budget_fails_with_a_warning` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.976 s |
| R2-F4.63 | `biscuit-visualized tests::mermaid_tests::backgrounds_paint_the_theme_a_solid_color_or_nothing` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.954 s |
| R2-F4.64 | `biscuit-visualized tests::mermaid_tests::mermaid_render_separates_backgrounds_and_geometry` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 1.001 s |
| R2-F4.65 | `biscuit-visualized tests::mermaid_tests::mermaid_transparent_background_changes_svg_output` — Committed L1 fixture, four-thread suite; expected: ≤5 s, with assertions retained | fixed | final `just test` 0.847 s |
| R2-F4.66 | `biscuit-visualized components::display::tests` — Remaining 6 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.67 | `biscuit-visualized components::git_graph::tests` — Remaining 63 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.68 | `biscuit-visualized components::graph_expression::tests` — Remaining 8 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.69 | `biscuit-visualized components::mermaid::tests` — Remaining 21 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.70 | `biscuit-visualized mermaid::render::tests` — Remaining 3 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.71 | `biscuit-visualized tests` — Remaining 11 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.72 | `biscuit-visualized tests::artifact_tests` — Remaining 5 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.73 | `biscuit-visualized tests::bundled_font_tests` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.74 | `biscuit-visualized tests::cache_identity_tests` — Remaining 9 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.75 | `biscuit-visualized tests::cache_tests` — Remaining 14 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.76 | `biscuit-visualized tests::contrast_corpus_tests` — Remaining 37 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.77 | `biscuit-visualized tests::corpus_measure_tests` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.78 | `biscuit-visualized tests::corpus_measure_tests::agentflow` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.79 | `biscuit-visualized tests::corpus_measure_tests::architecture` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.80 | `biscuit-visualized tests::corpus_measure_tests::block` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.81 | `biscuit-visualized tests::corpus_measure_tests::c4` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.82 | `biscuit-visualized tests::corpus_measure_tests::class` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.83 | `biscuit-visualized tests::corpus_measure_tests::cynefin` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.84 | `biscuit-visualized tests::corpus_measure_tests::er` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.85 | `biscuit-visualized tests::corpus_measure_tests::event_modeling` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.86 | `biscuit-visualized tests::corpus_measure_tests::flowchart` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.87 | `biscuit-visualized tests::corpus_measure_tests::gantt` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.88 | `biscuit-visualized tests::corpus_measure_tests::gitgraph` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.89 | `biscuit-visualized tests::corpus_measure_tests::info` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.90 | `biscuit-visualized tests::corpus_measure_tests::ishikawa` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.91 | `biscuit-visualized tests::corpus_measure_tests::journey` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.92 | `biscuit-visualized tests::corpus_measure_tests::kanban` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.93 | `biscuit-visualized tests::corpus_measure_tests::mindmap` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.94 | `biscuit-visualized tests::corpus_measure_tests::packet` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.95 | `biscuit-visualized tests::corpus_measure_tests::pie` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.96 | `biscuit-visualized tests::corpus_measure_tests::quadrant_chart` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.97 | `biscuit-visualized tests::corpus_measure_tests::radar` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.98 | `biscuit-visualized tests::corpus_measure_tests::railroad` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.99 | `biscuit-visualized tests::corpus_measure_tests::requirement` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.100 | `biscuit-visualized tests::corpus_measure_tests::sankey` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.101 | `biscuit-visualized tests::corpus_measure_tests::sequence` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.102 | `biscuit-visualized tests::corpus_measure_tests::state` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.103 | `biscuit-visualized tests::corpus_measure_tests::swimlane` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.104 | `biscuit-visualized tests::corpus_measure_tests::timeline` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.105 | `biscuit-visualized tests::corpus_measure_tests::tree_view` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.106 | `biscuit-visualized tests::corpus_measure_tests::treemap` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.107 | `biscuit-visualized tests::corpus_measure_tests::usecase` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.108 | `biscuit-visualized tests::corpus_measure_tests::venn` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.109 | `biscuit-visualized tests::corpus_measure_tests::wardley` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.110 | `biscuit-visualized tests::corpus_measure_tests::xychart` — Remaining 2 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.111 | `biscuit-visualized tests::font_chain_tests` — Remaining 31 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.112 | `biscuit-visualized tests::gitgraph_layout_tests` — Remaining 76 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.113 | `biscuit-visualized tests::gitgraph_tests` — Remaining 20 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.114 | `biscuit-visualized tests::golden_tests` — Remaining 198 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.115 | `biscuit-visualized tests::graph_tests` — Remaining 28 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.116 | `biscuit-visualized tests::html_static_tests` — Remaining 5 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.117 | `biscuit-visualized tests::measure_tests` — Remaining 8 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.118 | `biscuit-visualized tests::mermaid_cache_key_tests` — Remaining 8 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.119 | `biscuit-visualized tests::mermaid_family_tests` — Remaining 28 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.120 | `biscuit-visualized tests::mermaid_tests` — Remaining 34 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.121 | `biscuit-visualized tests::newline_tests` — Remaining 14 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.122 | `biscuit-visualized tests::newline_tests::rendered` — Remaining 6 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.123 | `biscuit-visualized tests::palette_tests` — Remaining 14 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.124 | `biscuit-visualized tests::postprocess_tests` — Remaining 7 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.125 | `biscuit-visualized tests::raster_tests` — Remaining 5 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.126 | `biscuit-visualized tests::raster_tests::requests` — Remaining 3 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.127 | `biscuit-visualized tests::theme_tests` — Remaining 41 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.128 | `biscuit-visualized::l1 display_size` — Remaining 10 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.129 | `biscuit-visualized::l1 graph_expression_parity` — Remaining 13 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.130 | `biscuit-visualized::l1 graph_expression_parity::parity_helpers::tests` — Remaining 24 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.131 | `biscuit-visualized::l1 layout_matrix` — Remaining 6 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.132 | `biscuit-visualized::l1 mermaid_parity` — Remaining 13 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.133 | `biscuit-visualized::l1 mermaid_parity::parity_helpers::tests` — Remaining 24 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |
| R2-F4.134 | `biscuit-visualized::l1 render_tree_diagram_hook` — Remaining 5 committed L1 cases; expected: Keep coverage and budget | clean | final `just test`: every test in the module ≤1.202 s (package max) |

### Progress

- starting the work on 'R2-F1 library-selected text unreadable on author fills' at 15:18:10
        - defect class: a readability correction is skipped, or cannot see the painted background, whenever author styling (rather than the compiled palette design) determines the surface behind text the library chose
        - dispatched to one foreground subagent (serial orchestration)
        - root cause 1: `render_svg` ran `readable_text` only when a compiled design existed; an author color removed the pass
        - root cause 2: `readable_text` read only inline paint; stylesheet- or attribute-painted shapes (the sequence `box` is `fill="LightGrey"`, its texts stylesheet-painted) left texts undecided
        - repair: the pass now always runs in `render_svg` before SVG/PNG/HtmlStatic diverge; accents still only with a design; when the inline walk leaves a text undecided, usvg resolves paint and the pass samples a 5×3 grid over the text box (indexed stroke-only marker rects survive `<use>` placement); rewritten texts recolor their `<tspan>`s too
        - decision: "library-selected" text = any text color the author did not write as a text color (`author_text_colors` reads CSS `color:`, C4 `$fontColor`/`$textColor`, and text/label/title/font theme keys); author text is never rewritten even on a library fill. Alternatives rejected: a list of Merman base-theme colors (incomplete), every source color (an author `stroke:#333` would exempt library `#333` text), and keeping the old "or fill is a palette color" clause (would recolor C4's author red/blue)
        - consequence of the decision: unedited `venn_dark` "AB" (author `style A,B color:#333`) is now kept at 1.29:1 with an author-text allow-list entry; added to review `human_review_items`
        - new dependency `svgtypes` 0.16.1 under `image` (already in the lockfile via usvg); `MERMAID_BACKEND` bumped to `+bv5`
        - tests: 33 `*_text_reads_on_a_white_primary_color` (one per family, macro), guard `every_family_fixture_has_a_white_primary_test`, directive-placement check, unit tests in `contrast.rs`; pre-fix the new tests failed for exactly the 25 families the review listed
        - goldens: real color changes in sequence (Dark, terminal ANSI, background-only) and venn Dark; style-order-only changes in block, kanban, requirement, state
        - verification (subagent): nextest slice over contrast/golden/theme/mermaid/palette/cache/html_static/raster → 432/432 passed; new tests ≤0.37 s each; `just lint` clean
        - noted: `mermaid_tests::the_large_diagrams_render_within_the_default_budget` failed alone 3/3 (layout under its 5 ms minimum) — traced to another session's root `Cargo.toml` test-profile `opt-level` change; re-checked in the phase gate
        - concurrency event: another session (feat-merman-15) briefly restored an older `render.rs` 15:40–15:44 and replayed the edits; verified `render.rs` diff is the intended one and the crate compiles
- work completed for 'R2-F1 library-selected text unreadable on author fills' at 15:57:00
- starting the work on 'R2-F2 graph PNG cache ignores resolved fonts' at 15:57:09
        - defect class: a cached raster artifact's identity omits the resolved fonts that drew its text, so a font-chain change reuses stale pixels
        - dispatched to one foreground subagent
        - root cause: `MERMAID_FONT` overrides every font query, graph labels included, but `GraphDiagram::cache_key` hashed only the requested CSS family, never the resolved face
        - repair: `render` → new `pub(crate) render_in(chain, request)`; PNG keys add `"fonts": chain.cache_identity(&[FontQuery::from_css(label_family, 400, false)])` (theme `font_family` or layout-rs `Times, serif`, now the `LAYOUT_FONT_FAMILY` constant); PNG rasterized with `rasterize_svg_for_request_with(…, chain)` so the drawing chain is the keyed chain; SVG keys unchanged; non-`_with` raster helper is now `#[cfg(test)]`
        - decision: no `GRAPH_BACKEND` bump (PNG keys change via the new field; a bump would also invalidate the unaffected SVG cache)
        - sibling sweep: only `graph/render.rs` and `mermaid/render.rs` call `FileCache::cache_key`/`cache.store`/the request rasterizer; GraphExpression component delegates to `GraphDiagram::render`; the render-tree hook and GitGraph render Mermaid (already keyed)
        - tests: `expression_png_cache_follows_the_font_chain`, `dot_png_cache_follows_the_font_chain` (miss + bytes equal a fresh render with the new chain), `graph_svg_cache_ignores_the_font_chain`; mutation checks (drop `fonts` from key; rasterize with default chain) each failed both PNG tests
        - verification (subagent): `-E 'test(/graph/) | test(/cache/)'` 306/306, `font_chain` 34/34, `raster_tests` 8/8, `just lint` clean
- work completed for 'R2-F2 graph PNG cache ignores resolved fonts' at 16:05:03
- starting the work on 'R2-F3 sequence and timeline exceed the 50 ms warm limit' at 16:05:19
        - defect class: a family's warm full-render path does more work than the absolute per-family limit allows, and a stored baseline value is not an exemption
        - dispatched to one foreground subagent; note R2-F1 added a usvg fallback (~1.7 ms) to sequence
        - profile (build-linux, warm): PNG encoding dominated (sequence 28.1 ms, timeline 47.6 ms: zlib default level + adaptive filter + a 15–26 MB premultiplied-pixel copy); installed font files were reopened/mapped on every glyph read (Merman measurement, contrast parse, raster parse); the remainder is resvg drawing (not our code)
        - repair 1 (byte-identical): `fonts/chain.rs` maps an installed face's file once on first resolve/fallback and keeps it for the process (`drawing_database()` for measure/draw; `database()` stays the metadata index, so cache identity is unchanged)
        - repair 2 (byte-identical): in-place alpha demultiplication in `raster/png.rs`, no buffer copy
        - repair 3 (output change): PNG encoder now zlib level 1 + `Up` filter; `PNG_ENCODING` cache identity → `png@0.18:deflate=level1,filter=up`; pixels unchanged, files ~2× larger (timeline 142 → 282 KB); without it timeline stays ~75 ms on the reference host. Alternative: a custom parallel deflate encoder (much more code). Added to review `human_review_items`
        - test renamed `the_encoding_is_lossless_and_no_larger_than_tiny_skias` → `the_encoding_is_lossless_and_smaller_than_the_raw_pixels`; new `font_chain_tests::file_faces_the_chain_answers_with_stay_mapped_and_draw_as_in_memory`
        - `mermaid_tests::the_large_diagrams_render_within_the_default_budget` (also seen in R2-F1): gitGraph call got faster than the test's 5 ms floor; fixture doubled 200 → 400 commits; passes 3/3 alone
        - reference-host measurement performed: `ssh -o BatchMode=yes build-linux` (idle, load 1–6), tree copied by tar to `/tmp/r2f3/bench-tree` (removed afterward), `cargo run --release` in `bench/`; full 33-family `--write-baseline` run "no rule fired"; sequence 56.381 → 33.81 ms warm, timeline 84.306 → 40.76 ms warm; `bench/baseline.json` rewritten from that run
        - macOS indicative (load 11–29): sequence 52.41 → 27.81, timeline 71.70 → 32.55
        - verification (subagent): golden/measure/contrast/font_chain/palette/raster/artifact/cache_identity slice 461/461, no golden rewritten; cache/mermaid/gitgraph group 181/181; `just lint` clean
        - risk carried to the cross-OS check: a font file held mapped for the process life cannot be replaced on Windows while the process runs
- work completed for 'R2-F3 sequence and timeline exceed the 50 ms warm limit' at 16:40:57
- starting the work on 'R2-F4 L1 rendering tests exceed the five-second budget' at 16:44:02
        - defect class: render-verification tests repeat installed-font initialization, multi-theme rendering, or corpus work in one test process, so they exceed the per-test budget under suite load
        - re-measurement before repair (after R2-F1/F3 and another session's root `[profile.test.package.merman*]` `opt-level = 3`): `just test --test-threads 4` → 975 passed, 80.4 s, slowest 3.06 s (0 over 5 s); default-concurrency `just test` (16 cores, load ~15) → 975 passed, 65.1 s, **32 slow**: 30 `mermaid_family_tests::smoke_renders_*`, `measure_tests::oracle_widths_match_usvg_for_the_mixed_script_corpus` 5.25 s, `html_static_tests::png_render_rasterizes_the_raster_safe_svg_under_the_html_static_target` 5.01 s (max 5.79 s)
        - dispatched to one foreground subagent with both lists
        - root cause (sampled with `sample`): 97% of a contended host-font test process sat in `fontique::Collection::new` → CoreText → `CFBundleGetMainBundle`, which lists the test executable's directory `target/debug/deps` (120,170 entries); concurrent listings serialize, so cost scaled with concurrency (0.86 s alone, 3.91 s at 12 copies); a binary copied to a small directory ran 0.49 s alone / 0.89 s at 12
        - repair 1: `.config/nextest.toml` reuses the `coretext-small-bundle` setup script (added by another session for `worktree-cli`) for `package(biscuit-visualized) & !test(/(^|::)(level2_|level3_|browser_|real_)/)` on macOS hosts only; production binaries list their own small directory
        - repair 2 (reviewer's smallest repair): family smoke tests render through `render_in(golden_chain(), …)` (bundled Inter, per-process `OnceLock`), all four themes and assertions kept; new `smoke_renders_on_host_fonts_through_the_public_render` keeps public-`render` host-font coverage (~0.7 s)
        - measure/html_static tests keep host fonts (that is what they test); fixed by repair 1
        - counts: `mermaid_family_tests` 67 → 68; all other review modules unchanged (measure 10, html_static 8, corpus_measure 68, mermaid_tests 38, gitgraph_tests 27, cache_key 10, components::mermaid 24, components::git_graph 67, contrast_corpus 73)
        - verification (subagent): 325-test filter over the review's modules at default concurrency 27.2 s / 29 slow / max 5.556 s → 6.95 s / max 1.281 s; `l1` binary + graph/newline/theme 187/187, max 1.043 s; `just lint` exit 0; `just check-tier-coverage biscuit-visualized` stranded 0
        - decision left with the author: skipping fontique's macOS CoreText scan in production would speed every CLI process but contradicts the spec's ruling that the OS generic answer comes through fontique; not changed
- work completed for 'R2-F4 L1 rendering tests exceed the five-second budget' at 16:54:49
- phase gate started at 16:54:49 (no subagent running)
- phase gate (no subagent running), all from this macOS host unless named:
        - `just test` in `biscuit-visualized` → 976 passed, 3 skipped, 13.1 s, slowest 1.202 s (R2-F4, plus every repair's regression coverage)
        - `just lint` in `biscuit-visualized` → exit 0
        - `just check-tier-coverage biscuit-visualized` → stranded 0
        - `just cross-check biscuit-visualized --os windows` → 976 passed (font-file mapping, svgtypes, encoder change); `--os linux` → 979 passed
        - dependents: `cargo check -p biscuit-terminal-cli -p darkmatter -p sniff-cli -p worktree-cli --all-targets` → ok; their diagram-related tests (`-E 'test(/mermaid|diagram|git_graph|graph_expression|visualiz/)'`, all features) → 164 passed; the 11 slow ones were darkmatter browser-tier and one biscuit-terminal-cli level2 test, outside the L1 budget

### Completion Report

- **outcome: complete**; elapsed about 1 h 47 min (15:17:35 → 17:05)
- parent findings: total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- child rows: 213 (R2-F1 39, R2-F2 7, R2-F3 33, R2-F4 134); none pending, unresolved, blocked, or disputed
- verified repairs:
        - R2-F1: the contrast pass always runs; undecided paint is resolved through usvg; author-written text colors are never rewritten; three sequence-box exemptions removed; one author-text entry added for `venn_dark` "AB"
        - R2-F2: graph PNG keys carry the resolved font identity, and PNGs are drawn with the chain they were keyed from
        - R2-F3: installed font files stay mapped, demultiplication happens in place, and PNG encoding uses zlib level 1 with the `Up` filter; on build-linux sequence is now 33.81 ms and timeline 40.76 ms warm, and the baseline was rewritten from an enforced run with no rule fired
        - R2-F4: a macOS nextest setup script points the test processes' CoreText main bundle at a small directory, and the smoke tests use the bundled-font chain
- decisions recorded in the review's `human_review_items` for the author: which text colors count as the author's (venn "AB" is now kept at 1.29:1), and the PNG encoder trade (about 2× file size for the 50 ms rule)
- open items: none; no performance measurement was deferred (`deferred_perf_measurement` not set)
- changed files (this cycle):
        - library: `biscuit-visualized/src/Cargo.toml`, `Cargo.lock`, `biscuit-visualized/bench/Cargo.lock`, `src/src/{artifact.rs, cache/file_cache.rs, fonts/chain.rs, fonts/shaper.rs, graph/render.rs, mermaid/contrast.rs, mermaid/render.rs, raster/mod.rs, raster/png.rs}`
        - tests: `src/src/tests/{cache_tests, contrast_corpus_tests, font_chain_tests, graph_tests, mermaid_cache_key_tests, mermaid_family_tests, mermaid_tests, palette_tests, raster_tests}.rs`; goldens `sequence_dark`, `sequence_terminal_ansi`, `sequence_terminal_background`, and `venn_dark` (SVG, and PNG for sequence_dark and venn_dark), plus style-order-only changes to `block_dark`, `block_default`, `kanban_dark`, `requirement_dark`, and `state_dark`
        - bench: `biscuit-visualized/bench/baseline.json`
        - config: `.config/nextest.toml` (the biscuit-visualized block only)
        - docs and skills: `biscuit-visualized/docs/{merman.md, dependencies.md, mermaid-gitgraph.md, components/graph_expression.md}`, `.claude/skills/biscuit-visualized/{SKILL.md, mermaid-rendering.md, artifacts-caching.md, rasterization.md}`
- not this cycle's changes (another session in this worktree): the root `Cargo.toml` test-profile `opt-level`, the worktree crates, the worktree setup-script block in `.config/nextest.toml`, and `.claude/skills/{os,worktree}/`
