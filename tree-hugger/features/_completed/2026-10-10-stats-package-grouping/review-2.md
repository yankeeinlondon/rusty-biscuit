---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-08T06:54:05-07:00"
spec: 2026-10-10-stats-package-grouping/spec.md
implemented: false
description: "A **feature** review of `2026-10-10-stats-package-grouping/spec.md`"
feature: 2026-10-10-stats-package-grouping/review-2.md
previous: 2026-10-10-stats-package-grouping/review-1.md
---

# Review 2: Stats package grouping

The feature is **ready for production**. The previous finding is verified fixed across all seven required instances and consumers. The affected real-terminal checks passed on tmux and WezTerm, including complete grouped reports, literal heading styles, zero counters, and narrow-width cases. No new blocking defect was established in the broader grouping sweep.

Reviewed the specification, plan, original implementation log, review-1 body/frontmatter, and the repair ledger in `log.md`. Review-1 is the only earlier review in this directory: one finding, seven child rows, no blocked findings. Its completion metadata was treated as a claim and checked against implementation and fresh runs.

Automatic grouping and manifest-backed identity were accepted before implementation: the original implementation log's Phase 2 records that the explicit implementation request authorized those choices. This review preserves that policy. R1-F1 had no prerequisite or human-review blocker, and its repair changed no prerequisites.

## Previous Finding Verification

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
| --- | --- | --- | --- | --- |
| R1-F1 — Real-terminal stats assertions still enforce the old report and omit package rendering coverage | R1-F1.S1–S7; all backends, both widths, named rendering, L1 text consumers, shipped snapshots | verified fixed | Every child below checked; shared assertions compare complete independently expected reports. Fresh affected tests and tier audit passed. No blocker required removal before or after repair. | None |
| R1-F1.S1 — tmux report | `level2_declarations_in_tmux` → shared `check_stats` | verified fixed | Fresh executed L2 pass, 8.154 s. Overall and Unassigned each retain 12 declarations, documentation 0/11, one unavailable declaration. | None |
| R1-F1.S2 — WezTerm report | `level2_declarations_in_wezterm` → shared checks and WezTerm capture | verified fixed | Fresh executed L2 pass, 7.255 s; backend record says `run`. Complete section/style assertions passed. | None |
| R1-F1.S3 — Kitty comparison | `level2_declarations_in_kitty` → same shared checks and existing capture adapter | verified fixed | [Kitty entry point](../../cli/tests/level2_declarations.rs:1079) invokes repaired `check_stats`, which invokes `check_package_stats`. No separate obsolete expectation exists; capture adapter unchanged. Runtime gate recorded `skip`, so no Kitty execution is claimed. | None; availability limits runtime evidence, not the shared repair |
| R1-F1.S4 — narrow branch | Both fixtures at ordinary width and `COLUMNS=20` | verified fixed | Both executed backends completed both loops. Exact full-report comparisons retain all sections and natural-width tables, without discarding headings or truncating capture. | None |
| R1-F1.S5 — named rendering | Duplicate names, Unicode/markup/controls, zero-match package, Unassigned, excluded package | verified fixed | [Shared package fixture](../../cli/tests/level2_declarations.rs:695) independently requires overall=4, a=2, b=1, c=0, Unassigned=1, documentation=1/4 overall. Exact rows reject an extra d section. Captured styles require bold and reject italic/red inside markup and after controls; OSC 8/BEL injection rejected. Both widths passed on tmux/WezTerm. | None |
| R1-F1.S6 — L1 text consumers | Terminal/plain/table order, literal text, counts, zero groups | verified fixed | Fresh passing `terminal_sections_are_ordered_and_literal` and `text_modes_match_json_sections_and_counts`, with explicit expected headings and counts. | None |
| R1-F1.S7 — shipped snapshots | Terminal and additive JSON stats goldens | verified fixed | Fresh passing `golden::stats_symbols_over_multi_match_the_golden` and `golden::stats_json_symbols_over_multi_match_the_golden` in declared `symbols` target. | None |

## Unblocked Findings

None. No unchanged failing reproduction, repair regression, or newly discovered sibling remains from R1-F1.

## Blocked Findings

None. No human review is needed to complete the specified behavior.

## Recurrence

No current finding repeats a prior defect class; `recurrence: false`. R1-F1 concerned output-contract changes missing from shared terminal assertions and their coverage. All real-terminal stats consumers were swept: tmux, WezTerm, and Kitty call the repaired helper. Both width decisions and L1 terminal/plain/table and JSON consumers were checked. No other stats capture assertion retains the old report.

## Requirement Verification

| User-facing requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Cargo/npm membership; standalone with/without Git; separate library/CLI names | L1 shipped CLI membership, standalone, and shipped-manifest tests | Passed; explicit identities/counts |
| Root/virtual workspaces, unnamed nested owners, prefix siblings, duplicate names, manifest-less files | L1 public ownership and grouping tests | Passed; deepest owner before name usability; no invented parent/root identity |
| Subpackage/linked-worktree invocation; separate root frames; external files | L1 root-frame, linked-worktree, and external broken-manifest tests | Passed; active worktree, one discovery root |
| File/declaration filters; classes count containers; zero/failed groups | L1 selector, classes, and partial-failure tests | Passed; filtered counters, retained zero groups, one reported failure |
| Empty successful changed selection versus ordinary empty-scan rules | L1 `root_unassigned_zero_and_empty_reports`; upstream selection unchanged | Passed; successful empty stats have no groups and retain discovery frame |
| Group sums, unequal-size ratios, unavailable documentation | L1 accumulator/report tests; mixed-language L2 fixture | Passed; overall ratio uses total counts, not averaged group ratios |
| Synthetic cross-package exports belong to prelude | L1 shipped CLI and accumulator tests | Passed; target display path does not assign ownership |
| Discovery absence/errors; stats-only bounded discovery | L1 injected discovery-count and shipped failure tests | Passed; errors fail before output; other modes avoid catalog capture |
| Sorted JSON groups, identical stats shapes, version 1, field order/errors/newline | L1 JSON report tests and stats golden | Passed |
| Terminal/plain/table composition and conflicts across four commands | L1 explicit reports and option matrix | Passed |
| Terminal sections, literal styles, empty sections, width | L2 captures on tmux and WezTerm | Passed; appropriate verification level now present |
| Native aliases, no duplicate counting, nonlossy ownership | L1 public lookup and shipped alias tests; target-gated Windows/Linux cases | macOS cases passed; other target execution left to CI |
| Warm manifest rename refreshes labels without source parsing | L1 `manifest_renames_refresh_groups_on_actual_cache_hits` | Passed; expected labels, explicit hit/zero-parse counters, unchanged cache bytes |

No keyboard, mouse, paste, or input-encoding requirement needs L3. L1 is appropriate for ownership and machine-readable reports; terminal presentation now has L2 evidence. Cross-OS proof is not a readiness gate here.

## Input Robustness Matrix

The repair changes tests, not readers. The broader review rechecked the name/provenance readers and reran their public-result matrix. Each manifest test starts from a tool-produced checked-in fixture, edits one field, and asserts identity through discovery and ownership. Persisted provenance is also checked through ownership after round trips.

`M` = manifest identity; `F` = unusable identity with the documented fallback label; `E` = rejected serialized catalog. Tree-hugger maps `F` owners to Unassigned. Manifest columns describe known workspace members, except standalone Composer.

| Shape | Cargo `package.name` | npm `name` | Python `project.name` | Go `module` | Composer `name` | JSON catalog `name_source` |
| --- | --- | --- | --- | --- | --- | --- |
| Unedited control | M | M | M | M | M | M retained through persistence |
| Absent | F | F | F | F | F | F, explicit legacy compatibility |
| Explicit null | F, invalid TOML | F | F, invalid TOML | Bare `null` is a path token: M | F | E |
| Wrong whole-field type | F | F | F | List/object syntax: F; bare `123` is a path token: M | F | E |
| One wrong element | F | F | F | F | F | E |
| Every element wrong | F | F | F | F | F | E |
| Empty | F | F | F | F | F | E |
| Duplicate key/directive | F | Existing JSON last-wins, M/F according to final value | F | F | Existing JSON last-wins | E |
| Trailing/invalid content | F | F | F | F | F | E |

All six matrix tests passed. `malformed_root_discovery_is_an_error` separately preserves required root-workspace failures. Absent/null manifest names deliberately yield unusable identity; null provenance is rejected while absence preserves legacy fallback. Existing documented JSON duplicate policy is retained. String readers reject nonstrings; malformed names never become declared empty names. The provenance default applies to legacy absence and never asserts manifest identity.

## Ownership and Consumer Sweep

In sniff, the shared name resolver handles Cargo/npm/Python/Go; root synthesis handles standalone Composer. The matrix covers each identity branch. The borrowing ownership lookup normalizes catalog boundaries once, uses native components, picks the deepest boundary, and preserves alias precedence. Public tests assert required ownership and reject incorrect parent/sibling/outside-root attribution.

In tree-hugger-cli, [Catalog](../../cli/src/grouping.rs:15) captures one structure catalog for stats before prelude/source analysis. [Grouping](../../cli/src/grouping.rs:61) checks the selected owner's name provenance and initializes a group from each selected analysis path, even on failure or zero matches. Catalog-only packages stay absent. [tally](../../cli/src/declarations.rs:479) adds each filtered record to overall and group counters, using containers for classes. Prelude ownership follows the analysis file; no later lookup falls through an unnamed owner to its parent.

Sorted results feed one JSON projection and the grouped terminal writer. Names/paths are display values, never lookup keys; literal rendering neutralizes controls before Prose markup. Terminal assertions check missing required sections and incorrect extras. Package identities remain outside persistent entries; warm rename tests verify expected labels independently of cached/uncached agreement. No repeated catalog/index construction, per-record ownership lookup, inventory/network enrichment, or extra parsing was found.

The CLI README, symbol-report topic, release notes, sniff API docs, and skills agree on automatic grouping, both root frames, zero groups, Unassigned, synthetic ownership, and failures. The repaired helper documentation describes the grouped contract. No production code was changed in this review.

## Verification Record

Fresh runs, all exit 0:

- `tree-hugger/`: `just test grouping --test-threads 4` — 30 passed; run `1884ef53-ab7b-4a5f-88df-9288098cde0a`.
- `tree-hugger/`: `just test stats --test symbols --test-threads 4` — 14 passed, including both stats goldens; run `9be24786-d45c-499e-869d-a75e68c22224`.
- `sniff/`: `just test package_identity --test-threads 4` — 14 passed; run `15c6bd21-7232-4cfc-a00f-0f4f714d7fa8`.
- `tree-hugger/`: `BISCUIT_TEST_REQUIRED_BACKENDS=tmux BISCUIT_L2_THREADS=2 just test-l2 --test level2_declarations` — run `90e850ee-944d-4922-a9e0-0bb3e47f47ba`, 8.155 s. Three nextest passes represent two executed backends and one availability skip. Backend records: tmux `run`, WezTerm `run`, Kitty `skip`; required tmux proof `run=1 skip=0 panic=0`. The area's harness recipe keeps windows from gaining focus.
- Repository root: `just check-tier-coverage tree-hugger` — zero stranded tests. Grouping modules are registered in `symbols`; declared `level2_declarations` requires `terminal-tests`, enabled by the live L2 recipe and CI metadata. Added helpers run inside selected backend tests.

Filtered-out tests are not execution evidence. The repair ledger's full L1/lint results were read but are not represented as fresh runs; targeted reruns cover the repaired consumers and feature contracts. No commits, formatting commands, or feature lifecycle moves were performed.

## Nonblocking Recommendations

None established. Kitty execution remains availability-gated; its shared repair is checked and tmux/WezTerm provide the required terminal evidence.
