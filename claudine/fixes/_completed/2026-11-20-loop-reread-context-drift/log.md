---
implementation_1: "2026-11-21T08:24:07+00:00"
---

# Review/Fix Log: loop reread context drift

## Implementation of Review Findings #1

> **started at:** 2026-11-21T08:24:07+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/fixes/2026-11-20-loop-reread-context-drift/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs
- no local OS testing rigs configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); cross-OS evidence is left to CI

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Group-framing tests fail under a supported non-UTF-8 locale | fixed | every child verified; class sweep found no further site (R1-F1.S) |
| R1-F1.1 | `every_member_task_opens_and_closes_exactly_one_stream`: one header and footer per task under C locale | fixed | `run` now uses `unicode_terminal()`; LC_ALL=C module run 12/12; UTF-8 module run 12/12; full C-locale and UTF-8 `just test` 12,676 passed |
| R1-F1.2 | `a_cap_of_one_preserves_the_group_bar_at_the_same_left_edge`: equal content columns under C locale | fixed | via `run`/`unicode_terminal()`; LC_ALL=C module run 12/12; UTF-8 module run 12/12; full C-locale and UTF-8 `just test` 12,676 passed |
| R1-F1.3 | `a_members_body_output_lands_on_the_data_channel_not_the_status_one`: headers on status, body on data | fixed | via `run`/`unicode_terminal()`; LC_ALL=C module run 12/12; UTF-8 module run 12/12; full C-locale and UTF-8 `just test` 12,676 passed |
| R1-F1.4 | `every_body_line_carries_its_own_tasks_bar`: body and header attribution match (forced color) | fixed | `forced_unicode_terminal()` keeps `new_forced` color and pins Unicode; LC_ALL=C module run 12/12; UTF-8 module run 12/12; full C-locale and UTF-8 `just test` 12,676 passed |
| R1-F1.5 | `a_parallel_group_gives_each_task_its_own_palette_entry`: three distinct prefixes | fixed | passed before, but its forced terminal had the same ambient glyph dependency; now `forced_unicode_terminal()`; LC_ALL=C module run 12/12; UTF-8 module run 12/12; full C-locale and UTF-8 `just test` 12,676 passed |
| R1-F1.6 | `a_no_color_terminal_still_attributes_every_task_by_name`: names preserved | fixed | passed before; builder now also pins `supports_unicode(true)` so the whole module is locale-independent; LC_ALL=C module run 12/12; UTF-8 module run 12/12; full C-locale and UTF-8 `just test` 12,676 passed |
| R1-F1.7 | `a_failed_task_reports_its_own_outcome_in_its_footer` | clean | asserts names/outcomes only; uses `run` (now pinned); passes under both locales |
| R1-F1.8 | `concurrent_siblings_never_split_one_frame_group` | clean | asserts frame-group integrity, not glyphs; passes under both locales |
| R1-F1.9 | `framing_never_reaches_the_captured_payload` | clean | asserts captured payload has no framing; passes under both locales |
| R1-F1.10 | `a_capped_members_body_keeps_its_group_bar_geometry` | clean | asserts bar gutter (`│` is drawn in both glyph modes); passes under both locales |
| R1-F1.11 | `partial_output_from_a_failing_task_is_still_framed` | clean | passes under both locales |
| R1-F1.12 | `a_silent_run_renders_no_frames_at_all` | clean | asserts absence of frames; passes under both locales |
| R1-F1.S | Class sweep: other Claudine tests asserting Unicode glyphs on terminals whose Unicode support follows the ambient locale | clean | empirical sweep: the full area L1 suite under `LANG=C LC_ALL=C LC_CTYPE=C just test` passed 12,676/12,676 (15 skipped, none >= 5 s). Renderer tests in `render::task_stream::tests` already pin `supports_unicode(true)` and cover the ASCII fallback (`a_limited_glyph_terminal_uses_the_ascii_header_marker`) |

### Work

- starting the work on 'R1-F1: group-framing locale dependence' at 08:24:26
        - defect class: terminal-output tests assert a fixed Unicode glyph while letting the host locale choose the glyph set
        - the repair is a fixture change of about 25 lines in one test module, so the orchestrator made it directly; a subagent brief would have cost more than the change
        - added `unicode_terminal()` (builder with `supports_unicode(true)`) and `forced_unicode_terminal()` (`new_forced` plus `supports_unicode: true`, keeping forced color) to `group_framing`; `run` and both forced sites use them, and the no-color builder pins Unicode too
        - production's ASCII fallback is unchanged and stays covered by `render::task_stream::tests::a_limited_glyph_terminal_uses_the_ascii_header_marker`
        - sibling sweep: a static grep for glyph assertions was noisy, so the whole L1 suite was run under an all-C locale instead; no other failure appeared
- work completed for 'R1-F1: group-framing locale dependence' at 08:30:40

### Completion Report

- **outcome: complete**; elapsed about 8 minutes (08:24:07 to about 08:32 UTC)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- child rows: 6 fixed, 6 clean, plus the clean class-sweep row R1-F1.S; no open IDs
- changed files:
        - `claudine/lib/src/composition/sequence/task/tests.rs` (R1-F1)
        - this log and the review frontmatter (metadata only)
- verification (from `claudine/`):
        - `LANG=C LC_ALL=C LC_CTYPE=C just test-library group_framing`: 12/12 passed (R1-F1.1 through R1-F1.12)
        - `LANG=C.UTF-8 LC_ALL=C.UTF-8 LC_CTYPE=C.UTF-8 just test-library group_framing`: 12/12 passed
        - `LANG=C LC_ALL=C LC_CTYPE=C just test`: 12,676 passed, 15 skipped, none at or above 5 s, exit 0 (R1-F1.S)
        - `LANG=C.UTF-8 LC_ALL=C.UTF-8 LC_CTYPE=C.UTF-8 just test`: 12,676 passed, 15 skipped, none at or above 5 s, exit 0
        - `just lint`: exit 0
        - `just check-tier-coverage claudine` (repo root): 0 stranded
        - `git diff --check`: clean
- no local OS rigs: set `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, and `BUILD_MACOS` to SSH hosts to enable cross-OS runs; CI covers the other operating systems
