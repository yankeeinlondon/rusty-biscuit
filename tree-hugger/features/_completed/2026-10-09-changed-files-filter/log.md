---
implementation_3: "2026-10-07T21:35:53-07:00"
implementation_2: "2026-10-07T21:17:15-07:00"
implementation_1: "2026-10-07T20:48:37-07:00"
deferred_perf_measurement: false
---

## Implementation of Review Findings #1

> **started at:** 2026-10-07T20:48:37-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-09-changed-files-filter/review-1.md'
- this is iteration 1 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|---|---|---|---|
| R1-F1 | Unstaged type changes remain changed paths and consistent status evidence | fixed | All required readers repaired through shared worktree classification; 33 sniff changed_paths tests and 49 CLI changed tests passed; every child below verified. |
| R1-F1.S1 | Path collector / changed_paths: both unstaged directions select leaf.rs | fixed | file_type_replacements_agree_across_status_readers_and_commit_phases asserts exact [leaf.rs] for both unstaged directions; sniff nextest ed18286b, 33 passed. |
| R1-F1.S2 | file_changes / repo_status: modified unstaged file, dirty, count 1 | fixed | Same public matrix asserts exactly one leaf.rs Modified/Modified entry, dirty true and unstaged 1 for both directions. |
| R1-F1.S3 | is_repo_dirty / minimal-summary requests: dirty true | fixed | Same matrix asserts direct is_repo_dirty and detect_with_request(minimal/summary) dirty true, both directions; early-stop return retained. |
| R1-F1.S4 | Detailed and total count readers: dirty true, unstaged 1, total 1 | fixed | Same matrix asserts detailed counts (true,0,1,0), total (true,1), and count-only detect_with_request exact categories. |
| R1-F1.S5 | Staged classification: both directions staged count 1 and selected | clean | Same matrix stages both directions: exact [leaf.rs], one Staged/Modified file, staged 1, unstaged/untracked 0; CLI all four commands exact staged counts. |
| R1-F1.S6 | Committed tip comparison: both directions selected, status clean | clean | Same matrix commits both directions: base-tip exact [leaf.rs], omitted paths empty, file_changes empty, all status/count readers clean; CLI all four commands base counts and HEAD zero. |
| R1-F1.S7 | Conflicts anywhere fail before reporting | clean | index_conflicts_anywhere_fail_naming_every_path and conflicts_outside_the_package_still_fail passed in sniff ed18286b and CLI 92d1c5ba; shared helper retains conflict arm. |
| R1-F1.C1 | functions: correct records/files/indexes/count/stats across report modes | fixed | file_type_replacements_report_current_declarations_in_every_mode pins functions=1, current name, one file/index, JSON Lines, stats, text/plain/table/tree, uncached facts and actual warm hits in both directions; CLI 92d1c5ba, 49 passed. |
| R1-F1.C2 | types: correct records/files/indexes/count/stats across report modes | fixed | Same CLI matrix pins types=1 and expected current struct in every supported mode, both directions; complete indexes retain both declarations. |
| R1-F1.C3 | symbols: correct records/files/indexes/count/stats across report modes | fixed | Same CLI matrix pins symbols=2 with exact current names in every supported mode, both directions, staged/committed count controls. |
| R1-F1.C4 | classes: correct records/files/indexes/count/stats across report modes | fixed | Same CLI matrix pins classes=1 with expected current type in every supported mode (outline instead of tree), both directions and warm cache hits. |
| R1-F2 | Canonical changed identities retain active-checkout ownership | fixed | Ownership guard validates candidate spelling, every symlink hop, canonical ancestor boundaries and checkout root before insertion; all required controls pass in final local L1 (1,859 tests) and Linux changed target (61 tests), including the added external-directory control. |
| R1-F2.S1 | External repository aliases: alias and target selectors empty | fixed | external_{untracked,staged,committed}_aliases matrix: alias and target empty in all commands/modes; exact zero cache/parse work and uncached empty facts. |
| R1-F2.S2 | Nested repository aliases: alias and target selectors empty | fixed | nested_directory_* and nested_file_* matrices cover .git directory and linked-checkout .git file boundaries, both selectors, all buckets and reports. |
| R1-F2.S3 | Same-checkout aliases: alias and target preserve shared identity | clean | same_checkout_aliases_preserve_expected_declarations asserts exact one file, clean/Clean names and 1/1/2/1 counts for alias and target; uncached facts and actual warm hit verified. |
| R1-F2.S4 | Explicit foreign file without alias remains empty | clean | an_explicit_file_in_another_repository_is_not_selected passed in 85309bfa; independently expects empty files/errors without any changed alias. |
| R1-F2.S5 | Ordinary nested repositories and submodules remain excluded | clean | a_nested_repository_is_not_descended_into and submodule_contents_are_never_selected passed in 85309bfa; independently assert no nested/submodule declarations. |
| R1-F2.S6 | Outside-package same-repo file and linked checkout remain eligible | clean | an_explicit_file_outside_the_package_root_stays_eligible, a_package_invocation_keeps_the_scan_scope and a_linked_worktree_selects_its_own_checkout passed in 85309bfa. |
| R1-F2.C1 | functions: foreign alias empty across every report mode | fixed | Nine ownership matrices assert functions empty terminal/plain/table, JSON, JSON Lines, full indexes, count, stats and stats JSON (tree for non-classes); zero cache/parse work each invocation. |
| R1-F2.C2 | types: foreign alias empty across every report mode | fixed | Nine ownership matrices assert types empty terminal/plain/table, JSON, JSON Lines, full indexes, count, stats and stats JSON (tree for non-classes); zero cache/parse work each invocation. |
| R1-F2.C3 | symbols: foreign alias empty across every report mode | fixed | Nine ownership matrices assert symbols empty terminal/plain/table, JSON, JSON Lines, full indexes, count, stats and stats JSON (tree for non-classes); zero cache/parse work each invocation. |
| R1-F2.C4 | classes: foreign alias empty across every report mode | fixed | Nine ownership matrices assert classes empty terminal/plain/table, JSON, JSON Lines, full indexes, count, stats and stats JSON (tree for non-classes); zero cache/parse work each invocation. |
| R1-F2.S8 | Candidate ancestor boundaries: nested alias back to outer checkout rejected | fixed | nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout adds .git after tracking a nested alias and changes its target spelling; outer clean.rs must remain unselected. |
| R1-F2.S10 | Unexpected ownership inspection errors fail rather than empty-success | fixed | Canonicalization/try_exists/metadata/read_link failures propagate actionable error; direct ownership_inspection_failure_is_an_error_naming_the_path passed in 5f798f64. |
| R1-F2.S9 | Directory aliases crossing nested/external ancestors rejected | fixed | Nested candidate test routes bridge -> inner/subdir -> back.rs -> outer clean.rs; external_directory_alias_cannot_return_changed_identity_to_active_checkout routes bridge -> foreign/subdir -> back.rs -> active clean.rs. Both independently prove changed evidence and reject alias/target across all commands/modes with zero cache/parse work. |
| R1-F2.S7 | Untracked, staged and committed alias union buckets enforce ownership | fixed | Nine static boundary x bucket tests exercise untracked/staged/committed aliases with both selectors and exact empty public results. |

- Inventory reconciled the frontmatter and entire review body: two high-priority findings, no blockers or nonblocking follow-ups, and all required instances/consumers above.
- Input robustness matrix does not apply: repairs consume typed Git observations and filesystem identities, with no configuration or file-format parser changes.

- Verification scope: sniff (`sniff`, `sniff-cli`) and tree-hugger (`tree-hugger`, `tree-hugger-cli`), confirmed with focused sniff package inventory and status-reader caller searches. GitNexus impact cannot bind this checkout: available indexes belong to the main checkout and another worktree, so the caller sweep uses `rg` rather than trusting a different tree.
- OS planning: only `BUILD_LINUX=build-linux` is declared; Windows, WSL2, and remote macOS variables are absent. Symlink/ownership risks merit focused Linux checks after local gates. L1 exercises public API and hermetic CLI subprocesses; no terminal/browser behavior or performance threshold requires higher tiers or timing measurements.


- starting the work on 'R1-F1' at 20:50:02
        - Defect class: status classifiers discard a valid Git change category, causing path selection and dirty/count projections to lose file-type replacements.
        - The sibling sweep (`rg` over Git status decisions and request consumers) found three index-to-worktree classifiers: rich/path collection, fast dirty detection, and detailed counts; total counts and minimal/summary requests project those readers.
        - Both regular-to-symlink and symlink-to-regular replacements will be checked unstaged, staged, and committed; CLI regressions assert independent declaration counts and names across all report modes.

        - Shared `classify_worktree_status` now classifies file-type replacements as modifications for the rich/path collector, fast dirty reader, and detailed count reader; fast dirty detection still stops at its first classified change.
        - Public-result tests exercise both replacement directions across unstaged, staged, and committed phases, including minimal/summary and count-only request projections. Exact path, file and category counts rule out extra results as well as omissions.
        - CLI tests exercise all four commands and every supported report mode, exact current names/counts, complete indexes, staged/committed controls, independently expected uncached output, and actual warm cache hits.
        - Focused verification: `just test changed_paths` in sniff passed 33 tests (nextest `ed18286b-7305-406e-82a6-2ff013f5bc87`); `just test --test changed` in tree-hugger passed 49 tests (nextest `92d1c5ba-960f-4d6c-97da-1a66613bfe04`), including conflict and one-status/no-diff work controls. Initial compile/test mismatches in the new tests were corrected before these passing runs.
        - Tests are compiled by sniff's declared unit-test module and the CLI's auto-discovered `tests/changed/main.rs` with an explicit `mod type_changes`; ordinary test names select L1, as both nextest runs demonstrate. No L2/L3 resource or performance measurement is required.
        - Updated the sniff API topic, CLI README, symbol-report topic, and sniff/tree-hugger skill guidance with file-type replacement behavior.
- work completed for 'R1-F1' at 20:53:21

- starting the work on 'R1-F2' at 20:54:32
        - Defect class: canonical filesystem identities erase checkout ownership, allowing changed aliases to transfer selection across repository boundaries.
        - The sibling sweep found one candidate insertion and one identity membership decision before analysis/cache and every report consumer; all untracked, staged, and committed paths share that insertion.
        - Both the Git candidate spelling and canonical target ancestors need ownership checks: nested directory/file markers, directory aliases, and nested aliases returning to the outer checkout are required siblings. Metadata failures reject ownership rather than treating unreadable markers as absent.

        - Guarded every canonical candidate before insertion using simplified native paths; each alias hop and resolved ancestor chain must remain owned by the active checkout, including nested `.git` files and directories. No second Git discovery, status observation, or submodule scan was added.
        - Ownership filesystem errors now propagate as actionable failures; absent candidates are skipped with `try_exists`, and unexpected marker inspection errors cannot become successful empty reports.
        - New statically declared boundary/bucket matrices assert every report's empty representation, both selector spellings, zero cache/parse work and independently expected uncached emptiness. Positive same-checkout aliases assert exact current declarations and actual warm cache hits. An intervening directory alias into a nested subdirectory and nested alias back to the outer checkout are included.
        - Focused verification: `just test --test changed` in tree-hugger passed 60 tests (nextest `85309bfa-6f0d-4f21-8eb7-d57f5347695b`); `just test --bin hug changed::tests::ownership` passed the error regression (nextest `5f798f64-6829-4332-869b-9b6fa8b00e2b`). Initial new test assertions were corrected to the existing classes terminal/plain versus table empty-output contracts; subsequent complete matrices passed.
        - Tests are declared by `mod ownership` in the existing changed target and by the CLI unit-test module, selected as L1 and executed by nextest. Boundary/bucket splitting keeps each matrix below the five-second slow-test threshold on this host.
        - Updated CLI README, symbol-report topic, tree-hugger skill and resolver error docs to describe ownership and boundary rejection. Final area-wide test/lint gates remain with the orchestrator.
        - Strengthened the return-to-outer test with an independent sniff assertion that `inner/alias.rs` really supplies changed-path evidence; focused rerun passed (`b0ab0146-70ce-4f37-bb23-6dd6c297e82a`).
- work completed for 'R1-F2' at 20:59:41

- Combined inspection confirmed the shared classifier reaches every status consumer, and one ownership insertion guard precedes all declaration outputs and per-file cache work. Reviewed every child test against independently expected paths, declarations, empty representations and positive controls.
- Detected documentation drift: classes terminal/plain empty outlines emit no text, while tables and other declaration listings use `(no symbols)`. Corrected the README, topic page and skill to match the existing code and tested output; no rendering behavior changed.
- Final full local gates started for sniff and tree-hugger, plus both tier-coverage checks. The first sniff full run failed on four unchanged macOS audio/hardware discovery timeouts (1,018 passed, four timed out, 32 skipped; 2,132 not run after fail-fast). An isolated audio test is being run to distinguish host discovery from scheduler contention.
- Final tree-hugger `just test`: 1,858 passed, zero skipped (nextest duration 185.854 s). Both package-area `just lint` commands passed. Sniff tier coverage reports zero stranded tests.
- Isolated macOS audio regression passed in 9.875 s (`just test detect_audio_devices_on_macos --test-threads 1`), supporting contention diagnosis; full sniff suite rerun uses four workers without changing tests or timeout policy.
- Linux focused verification uses `just cross-check` with explicit feature flags to exercise runtime Git/symlink behavior in native mode; it is not an archive/receipt claim. The snapshot banner identifies revision `db6604d71`; caller index and branch remain unchanged.

- starting the reconciliation of 'R1-F2.S9 external directory alias sibling' at 21:06:00
        - Added the external directory back-link control: the alias canonicalizes to active-checkout clean.rs but traverses foreign/subdir first; both selector spellings must remain empty in every supported report with zero cache/parse work.
        - Focused `just test --test changed ownership::external_directory_alias` passed (nextest `d4d59fbc-1101-4472-b0cf-938415119181`): one test, 60 skipped by the focused filter, no remaining commands running.
- work completed for 'R1-F2.S9 external directory alias sibling' at 21:06:14
- Final sniff `just test --test-threads 4`: 3,154 passed, 32 skipped by existing tier selection; no failure (96.906 s). All four originally timed-out macOS tests passed in about 11 s. Recorded this observed host-probe contention and bounded-worker verification in the OS skill, as repository maintenance requires.
- Strict sniff all-target Clippy (`cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`) passed, and both package-area tier-coverage checks report zero stranded tests.
- Linux sniff focused check passed 33 tests; Linux CLI changed target passed 60 tests before the final external-directory control. The final CLI snapshot rerun includes that added test.
- Reconciliation added the missing external-directory return-to-checkout control for R1-F2.S9; focused local check passed. Full tree-hugger test/lint and the Linux changed target are being rerun after that last test edit.
- Final Linux CLI changed target passed all 61 tests (2.514 s), including the external-directory alias control. Final tree-hugger lint passed after the last test change. All 27 ledger rows have verified dispositions: two fixed parents, 18 fixed children, seven clean sibling/control children; no pending/open child remains. Final full tree-hugger L1 rerun is the remaining completion prerequisite.

### Completion Report

- Outcome: **complete** — implementation complete, ready for review.
- Finished at: `2026-10-07T21:10:48-07:00`; elapsed duration: **22 min 11 s**, measured from the requested start timestamp.
- Parent finding count: `total 2 = fixed 2 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0`.
- Ledger reconciliation: all 27 unique IDs verified; two fixed parents, 18 fixed instance/consumer rows and seven clean sibling/control rows. No pending rows remain.
- R1-F1: one shared worktree classifier preserves unstaged file-type evidence for path selection, rich status, fast dirty detection and count projections. Both replacement directions are verified unstaged, staged and committed; all declaration commands/modes preserve expected current source and actual warm cache hits.
- R1-F2: ownership checks precede canonical identity insertion and source/cache work, reject external/nested boundaries through every alias hop and resolved ancestor chain, and propagate unexpected filesystem errors. Every union bucket and selector spelling is verified across all commands/modes; same-checkout aliases, package scope, linked worktrees, normal nested repositories and submodules retain their expected behavior.
- Open finding/child IDs: **none**. No blocker or deferral remains; the next action is independent review. The reviewer's original `ready: false` judgment, findings and body are preserved.
- Performance: no wall-clock measurement is required by this repair; existing one-status/no-optional-diff work assertions passed. `deferred_perf_measurement: false`; no deferred-performance document is needed.
- Final verification:
        - `tree-hugger/`: `just test` — **1,859 passed**, zero skipped, 162.891 s; verifies R1-F1/R1-F2 and all declaration/cache consumers after the final test edit.
        - `tree-hugger/`: `just lint` — **passed** after the final test edit; library and CLI all-target Clippy.
        - `sniff/`: `just test --test-threads 4` — **3,154 passed**, 32 existing skips, 96.906 s; verifies R1-F1 and neighboring public/status/request readers.
        - `sniff/`: `just lint` — **passed** with the area's remote feature coverage.
        - Repository root: `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` — **passed**, covering test targets and strict lint requirements.
        - Repository root: `just check-tier-coverage sniff` and `just check-tier-coverage tree-hugger` — **passed**, zero stranded tests each. Both new CLI modules are declared by the changed target; all new ordinary test names execute in L1, as final local and Linux runs prove.
        - Repository root: `just cross-check sniff --os linux --features remote changed_paths` — **33 passed**; R1-F1/status/conflict/cost regressions on Linux.
        - Repository root: `just cross-check tree-hugger-cli --os linux --no-default-features --test changed` — **61 passed**, zero skipped, on final snapshot `49ca1e379`; R1-F1/R1-F2 across native Linux Git/symlink behavior. These are native runtime checks, not archive receipts.
        - `git diff --check` — **passed**.
- Earlier verification failure accounted for: the first default-concurrency sniff full run timed out four unchanged macOS CoreAudio/audio hardware tests (1,018 passed, four timed out, 32 skipped; 2,132 not run after fail-fast). The isolated audio test passed, and the complete four-worker rerun passed every selected test including those four. No test, timeout, assertion or tier was weakened.
- OS evidence: macOS full-area gates and Linux focused runtime checks passed. `BUILD_WIN`, `BUILD_WSL` and remote macOS hosts were not declared; this cycle makes no native Windows/WSL2 runtime claim. Existing CI provides those environments.
- Changed files in this cycle:
        - `sniff/lib/src/filesystem/git/status.rs` — R1-F1 shared classification repair.
        - `sniff/lib/src/filesystem/git/changed_paths_tests.rs` — R1-F1 public/status/request phase matrix.
        - `tree-hugger/cli/src/changed.rs` — R1-F2 ownership guard and filesystem error regression.
        - `tree-hugger/cli/tests/changed/main.rs`, `type_changes.rs`, `ownership.rs` — declared R1-F1/R1-F2 L1 regressions.
        - `sniff/docs/sniff-library-architecture.md`, `tree-hugger/cli/README.md`, `tree-hugger/docs/symbol-reports.md` — current behavior and corrected classes empty-output documentation.
        - `.claude/skills/sniff/SKILL.md`, `.claude/skills/tree-hugger/SKILL.md` — classification and checkout ownership guidance.
        - `.claude/skills/os/macos.md` — observed host-probe contention and successful bounded-worker verification.
        - This iteration's `log.md` and `review-1.md` — ledger, results and required completion metadata.
- Existing staged changes, the author's specification edits and lifecycle location were preserved. No formatting command or branch commit was performed. Every started command and subagent finished and its result was read.

## Implementation of Review Findings #2

> **started at:** 2026-10-07T21:17:15-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-09-changed-files-filter/review-2.md'
- this is iteration 2 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Recurring R1-F2 ownership normalization class: reject forbidden traversal before canonical matching | fixed | Orchestrator verified every child and combined traversal/report decision; final macOS L1 1,883 passed, all-target lint and tier coverage passed, final Linux changed target 85 passed. |
| R2-F1.S1 | Nested .git directory, parent cancellation: all buckets/selectors empty | fixed | nested_directory_parent_{untracked,staged,committed}: both selectors, all four commands/every report, zero source/cache work; changed target ef400c0b passed. |
| R2-F1.S2 | Nested .git file, parent cancellation: all buckets/selectors empty | fixed | nested_file_parent_{untracked,staged,committed}: linked-checkout .git file, every selector/report, zero work; ef400c0b passed. |
| R2-F1.S3 | External root traversal: all buckets/selectors empty | fixed | external_parent_{untracked,staged,committed}: absolute foreign/subdir/../../active traversal returns canonical clean.rs but every selector/report is empty; ef400c0b passed. |
| R2-F1.S4 | Directory alias with parents: all buckets/selectors empty | fixed | directory_alias_parent_{untracked,staged,committed}: bridge resolves inner/subdir before parents, all selectors/reports empty and zero work; ef400c0b passed. |
| R2-F1.S5 | Recursive file alias: all buckets/selectors empty | fixed | recursive_file_alias_parent_{untracked,staged,committed}: middle.rs committed before sole changed alias, recursive boundary traversal rejected everywhere; ef400c0b passed. |
| R2-F1.S6 | Dot-plus-parent traversal: all buckets/selectors empty | fixed | dot_parent_{untracked,staged,committed}: explicit dot/parent target, every selector/report empty and zero work; ef400c0b passed. |
| R2-F1.S7 | Ordinary parent traversal: exact positive declarations, every bucket/selector | clean | ordinary_parent_{untracked,staged,committed}: exact clean/Clean declarations and counts 1/1/2/1 for both selectors, cached/uncached facts and actual warm hits; ef400c0b passed. |
| R2-F1.S8 | Existing boundary controls retain empty results | clean | Nine existing boundary/bucket matrices and both return-to-checkout back-link tests retain independently expected emptiness/zero work; all passed in ef400c0b. |
| R2-F1.C1 | functions: every report empty with zero source/cache work | fixed | Every S1–S6 shape across three buckets and two selectors asserts functions empty in all supported report modes, zero cache/parse work and independently empty uncached JSON; ef400c0b passed. |
| R2-F1.C2 | types: every report empty with zero source/cache work | fixed | Every S1–S6 shape across three buckets and two selectors asserts types empty in all supported report modes, zero cache/parse work and independently empty uncached JSON; ef400c0b passed. |
| R2-F1.C3 | symbols: every report empty with zero source/cache work | fixed | Every S1–S6 shape across three buckets and two selectors asserts symbols empty in all supported report modes, zero cache/parse work and independently empty uncached JSON; ef400c0b passed. |
| R2-F1.C4 | classes: every report empty with zero source/cache work | fixed | Every S1–S6 shape across three buckets and two selectors asserts classes empty in all supported report modes, zero cache/parse work and independently empty uncached JSON; ef400c0b passed. |
| R2-F1.S9 | Inspection errors, one Git observation, package/linked checkout and cache positive controls preserved | clean | Ownership error 570196ee passed; package/outside-package/linked checkout/cache/paths/type-change regressions all passed in ef400c0b. resolve still calls changed_paths once; sniff code unchanged, no extra Git discovery or status call added. |
| R2-F1.S10 | Ownership comments, README, topic and skill guidance remain accurate | fixed | Ownership contract comments, CLI README, Symbol Reports and tree-hugger skill now explain parent cancellation and resolved directory-alias semantics; checked against passing negative/positive controls. |
| R2-F1.S11 | Discovered ordinary directory-alias parent semantics: required outer declarations preserved | clean | ordinary_directory_alias_parent_{untracked,staged,committed}: bridge → inner/subdir then ../../ reaches clean.rs; exact names/counts 1/1/2/1, both selectors, cached/uncached facts and actual warm hits; ef400c0b passed. |

- Inventory reconciles the full review: one unblocked recurring parent finding R2-F1 (historical class R1-F2), eight named sites, four consumers, and preservation/documentation requirements. Historical R1-F1 and original R1-F2 rows remain in the previous iteration and require regression verification, not duplicate repair counts. No nonblocking follow-up or performance measurement is required.
- Input robustness matrix does not apply: no file-format/configuration parser is changed.

- Orchestrator scope inspection: the ownership helper guards the sole changed identity insertion; main intersects its set before analysis/cache and all report consumers. This cycle changes only tree-hugger behavior; sniff remains unchanged and its existing cost/status controls are regression dependencies.
- Loaded the OS skill and Windows path guidance for traversal comparisons. BUILD_LINUX is declared; BUILD_WIN, BUILD_WSL and BUILD_MACOS are absent. Plan focused Linux runtime verification after local gates. L1 hermetic Git/filesystem/CLI checks cover this repair; no L2/L3 resources or timing threshold are required.
- Inspected the live lint recipe: just lint runs all-target Clippy without a formatting command, so it complies with the explicit no-formatting rule. Existing staged/unstaged author changes remain intact.

- starting the work on 'R2-F1' at 21:18:30
        - Defect class: normalizing traversal before checking ownership erases forbidden directory visits and transfers eligibility to unchanged identities.
        - Sibling sweep found the normalized-root stopping condition, ancestor canonicalization and recursive alias-target reader in the single ownership guard; all Git buckets feed its one insertion, and main intersects identities before every declaration/cache/report consumer.
        - Repair scope includes forward component traversal with actual directory-symlink parent semantics, every S1–S6 boundary shape across all buckets/selectors/modes, and independently expected ordinary-parent positive controls.

        - Forward traversal checks visited directories before parents cancel them; recursive targets return their resolved path so later parents follow directory-alias destinations. Root ancestry permits macOS filesystem aliases and Windows native-prefix/short-name spellings; a bare drive prefix is never probed as a drive-relative path. Resolved destination ancestor checks preserve reparse-point protection.
        - Added 24 ordinary L1 cases: six forbidden shapes × three Git buckets, ordinary-parent controls × three buckets, and discovered ordinary-directory-alias controls × three buckets. Each fixture independently proves alias-only Git evidence and equality to the unchanged outer canonical identity. Forbidden cases assert every command/report/selector empty with zero cache/parse work; positive cases assert required declarations rather than mere cache equivalence.
        - Initial verification setup failures were corrected: auxiliary nested roots/bridge entries are ignored in the fixture's committed .gitignore, and full JSON seeds its own cache coverage before a warm-hit assertion (count coverage is narrower). No assertion or behavior requirement was weakened.
        - Focused final checks: `just test --test changed` passed all **85** tests, zero skipped (8.300 s; nextest `ef400c0b-7425-433c-bf3b-ed11aaaa3ed1`); `just test --bin hug changed::tests::ownership_inspection_failure` passed **1**, 27 filtered out (nextest `570196ee-673e-4c10-910b-3a7c4808c54e`). `git diff --check` passed.
        - Every new case is compiled in the existing auto-discovered changed target through `mod ownership`; macro-generated test names have no tier marker, and the actual nextest listing/execution proves all 24 execute in L1. No window, device, performance threshold, or new parser requires higher tiers or an input robustness matrix.
        - Changed in this repair: `tree-hugger/cli/src/changed.rs`, `tree-hugger/cli/tests/changed/ownership.rs`, `tree-hugger/cli/README.md`, `tree-hugger/docs/symbol-reports.md`, `.claude/skills/tree-hugger/SKILL.md`, and this iteration's log. Parent reconciliation/full gates and completion metadata remain with the orchestrator.
- work completed for 'R2-F1' at 21:20:49

- Orchestrator reconciliation inspected the component traversal, marker checks, recursive targets, native-prefix handling, identity insertion and all added fixture assertions. All six failing shapes cover untracked/staged/committed evidence and both alias/target selectors; independent sniff and canonical-identity controls prevent vacuous empty passes. S11 proves real directory-alias parent semantics without sacrificing required positive results.
- Final local gates after the last code/test edit: tree-hugger just lint passed both all-target Clippy runs; just check-tier-coverage tree-hugger passed with zero stranded tests. Full just test is executing 1,883 selected L1 tests. The sole subagent finished and all its command results were read.

- Final full macOS tree-hugger `just test` passed **1,883 tests**, zero skipped (185.840 s; nextest `caa7c6c0-cb1a-41f8-b321-13d1b5acb4d4`). All 24 new cases and historical controls executed in the selected L1 suite. Linux focused runtime verification has started through the authorized cross-check recipe; the snapshot does not alter the local branch or index.

- Final Linux focused runtime check passed **85 tests**, zero skipped (3.771 s; nextest `114a8883-c23c-4088-a37a-de5c72e8b072`, snapshot `f50df2d17`). This is native Linux runtime evidence, not an archive receipt. The cross-check finished successfully and its result was read.

### Completion Report

- Outcome: **complete** — implementation complete, ready for review.
- Finished at: `2026-10-07T21:25:06-07:00`; elapsed duration: **7 min 51 s** from the requested start timestamp.
- Parent finding count: `total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0`. R2-F1 closes the recurring historical R1-F2 class; previously verified historical findings are not counted twice.
- Reconciled ledger: **16 unique IDs**, consisting of one fixed parent, 11 fixed required child/consumer rows, and four clean positive/preservation controls. No pending or open row remains.
- Verified repair: filesystem components and alias targets are checked before parents erase repository boundaries. Nested `.git` files/directories and external traversals cannot transfer eligibility to an unchanged outer identity. All six forbidden shapes are verified across three Git buckets, both selector spellings, all four commands and every supported report mode with zero source/cache work. Ordinary parents and directory-alias parents preserve exact declarations, cached/uncached facts and actual warm hits. Error propagation, linked checkout/package scope, existing boundaries, and one-observation wiring remain verified.
- Open finding/child IDs: **none**. No blockers or deferred work remain; next action is independent review.
- Final verification commands/results tied to R2-F1:
        - `tree-hugger/`: `just test` — **1,883 passed**, zero skipped, 185.840 s, nextest `caa7c6c0-cb1a-41f8-b321-13d1b5acb4d4`; full local regressions and all selection/report/cache consumers after the final code/test edit.
        - `tree-hugger/`: `just lint` — **passed**, all-target Clippy for library and CLI.
        - Repository root: `just check-tier-coverage tree-hugger` — **passed**, zero stranded tests; all 24 new tests compiled in the existing changed target and selected by L1.
        - Repository root: `GIT_TERMINAL_PROMPT=0 just cross-check tree-hugger-cli --os linux --no-default-features --test changed` — **85 passed**, zero skipped; verifies S1–S11/C1–C4 on native Linux.
        - Focused local checks: `just test --test changed` — **85 passed**; `just test --bin hug changed::tests::ownership_inspection_failure` — **1 passed**, 27 filtered out; protects explicit ownership failure behavior.
        - `git diff --check` — **passed**.
- Earlier verification setup failures were corrected before passing runs: an initial command used the root recipe with no matching package; fixture auxiliary paths supplied unintended untracked evidence; a count-seeded cache lacked JSON capability coverage. Corrected the command location, committed fixture ignores and explicit cached-JSON seed; required assertions remain intact. No unrelated final gate failed.
- Performance: no timing threshold or measurement is required by this repair. `deferred_perf_measurement: false`; no deferred-performance document is necessary.
- OS evidence: full macOS gates and focused Linux runtime checks passed. Windows/WSL2 build hosts were not declared; no runtime claim is made for them. Existing CI retains those environments. No additional CI cells or test tiers were introduced.
- Changed files for this cycle:
        - `tree-hugger/cli/src/changed.rs` — traversal ownership repair.
        - `tree-hugger/cli/tests/changed/ownership.rs` — 24 permanent phase regressions and independent controls.
        - `tree-hugger/cli/README.md`, `tree-hugger/docs/symbol-reports.md`, `.claude/skills/tree-hugger/SKILL.md` — current traversal/parent semantics and examples.
        - `tree-hugger/features/2026-10-09-changed-files-filter/log.md`, `tree-hugger/features/2026-10-09-changed-files-filter/review-2.md` — reconciled ledger, report and completion metadata.
- Review `implemented` is true; `implemented_by` and `log` metadata are recorded while preserving the reviewer's `ready: false`, findings and body. Log `implementation_2` carries the requested timestamp. Author specification/plan, lifecycle location, local branch/index and existing changes were preserved. No formatting command or branch commit was performed. Every started command and subagent finished and its result was read.

## Implementation of Review Findings #3

> **started at:** 2026-10-07T21:35:53-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-09-changed-files-filter/review-3.md'
- this is iteration 3 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Contain malformed-index decoder panics at all shared boundaries | fixed | Orchestrator inspected every required/discovered child and decision path; final macOS area L1 and lint, strict all-target Clippy, tier checks, and focused Linux API/CLI runtime checks passed. |
| R3-F1.S1 | Optional cache sizing must never panic; status still errors | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); direct configure_cache and every shared-cache consumer checked |
| R3-F1.S2 | Shared status collector returns typed error, no partial status | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); direct path/rich and public status errors |
| R3-F1.S3 | Fast dirty reader returns typed error and preserves early stop | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); shared initialization only, first-change returns retained |
| R3-F1.S4 | Detailed/total counts return typed error, no zero fallback | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); exact valid `(true,1)` / `(true,0,1,0)`, malformed errors |
| R3-F1.S5 | Rich StatusContext cannot reinterpret invalid index as empty | fixed | Fallible context reload; direct 12-prefix context test passed (`ca22d01f-eb1a-4aef-b5dd-0998e7498377`), valid context succeeds |
| R3-F1.S6 | Conflict reader preserves contained typed-error contract | already fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); direct/public conflict errors; compatibility suppression preserved through shared helper |
| R3-F1.S7 | Discovery and identity-only projection preserve identity | clean | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); main branch identity with no status at every prefix; discovery precedes any index read |
| R3-F1.S8 | Linked-checkout cache sizing/detail errors never panic | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); main reads separately corrupted linked administrative index and returns Git error |
| R3-F1.C1 | functions all report modes preserve specified error/metadata contract | fixed | CLI `index_corruption` main/linked × 12 prefixes × commands × bases × formats: exact exit 1, empty stdout, root/index cause, no cache stats; changed target 110 passed (`3cd76c58-146f-454d-a49e-4a04785b7d18`) |
| R3-F1.C2 | types all report modes preserve specified error/metadata contract | fixed | CLI `index_corruption` main/linked × 12 prefixes × commands × bases × formats: exact exit 1, empty stdout, root/index cause, no cache stats; changed target 110 passed (`3cd76c58-146f-454d-a49e-4a04785b7d18`) |
| R3-F1.C3 | symbols all report modes preserve specified error/metadata contract | fixed | CLI `index_corruption` main/linked × 12 prefixes × commands × bases × formats: exact exit 1, empty stdout, root/index cause, no cache stats; changed target 110 passed (`3cd76c58-146f-454d-a49e-4a04785b7d18`) |
| R3-F1.C4 | classes all report modes preserve specified error/metadata contract | fixed | CLI `index_corruption` main/linked × 12 prefixes × commands × bases × formats: exact exit 1, empty stdout, root/index cause, no cache stats; changed target 110 passed (`3cd76c58-146f-454d-a49e-4a04785b7d18`) |
| R3-F1.C5 | changed_paths omitted/explicit HEAD preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); both omitted/explicit HEAD errors and exact leaf.rs controls |
| R3-F1.C6 | file_changes/repo_status preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); one Modified leaf.rs, exact counts and typed corruption errors |
| R3-F1.C7 | minimal/summary requests preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); valid dirty flag, no category counts; malformed Git errors |
| R3-F1.C8 | count-only/full requests preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); count request proven nonminimal; exact valid category counts; malformed Git errors |
| R3-F1.C9 | branch readers and compatibility wrapper preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); one main branch, ahead/behind 0, current true across corruption |
| R3-F1.C10 | tracking readers and compatibility wrapper preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); both tracking APIs retain independently expected no-remotes empty result |
| R3-F1.C11 | detailed worktrees, including corrupt linked index preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); no registered linked worktrees remains empty despite main corruption; linked detail corruption errors |
| R3-F1.C12 | legacy/structured recent commits preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); legacy one expected commit preserved; structured reader returns typed attribute-index error |
| R3-F1.C13 | existing-handle worktree list/name preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); one main worktree/branch, process outside fixture (is_current false), name None |
| R3-F1.C14 | existing-handle preferred remote preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); independently expected no configured remote remains None |
| R3-F1.C15 | aggregate observations preserve specified error/metadata contract | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); valid aggregate contains one commit; corrupt attribute-index errors contained |
| R3-F1.S9 | Absent/valid empty/unborn/main and linked index positive controls | clean | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); active/main/linked absent index and unborn absent/tool-written empty select exact current paths; CLI names controls passed |
| R3-F1.S10 | Current error-contract comments/docs/skills | fixed | API/index comments, sniff architecture, Symbol Reports, CLI README, sniff/tree-hugger skills updated; diff whitespace check passed |
| R3-F1.S11 | One observation, early stopping, no discovery/network enrichment | clean | One-walk/no-enrichment counters passed in changed_paths suite; guard wraps initializer only, preserving fast dirty returns; no new discovery/network calls |
| R3-F1.S12 | Historical type-change/ownership/cache regressions remain valid | clean | Historical CLI ownership/type-change/cache/parser/error tests all passed in 110-test changed target; API type/union/native/cost regressions passed in 35-test suite |
| R3-F1.S13 | Committed-diff attribute cache reads live index indirectly; all three initializers contain decoder errors | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); three explicit committed-diff initializers use guarded tree_diff_cache |
| R3-F1.C16 | Commit-file API and compatibility reader preserve typed error/empty policy on malformed indexes | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); direct/focused commit-file errors, compatibility empty; original exact leaf.rs Added |
| R3-F1.C17 | Path-history API and compatibility reader preserve typed error/empty policy on malformed indexes | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); direct/focused path-history errors, compatibility empty; original one expected commit |
| R3-F1.S14 | Committed-only merge predictor never reads live index; remain valid with corrupt index | clean | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); committed merge with corrupt index remains empty as independently expected; index_from_tree plus own committed attributes/cache never read live index |
| R3-F1.S15 | Tip-diff convenience iterator indirectly loads index; guarded explicit cache preserves typed errors | fixed | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); convenience tree diff replaced with guarded explicit cache; direct malformed/empty-tip comparison errors and exact leaf.rs positive |
| R3-F1.C18 | Captured-cache committed-file/enrichment readers use captured attribute index and retain committed facts | clean | API `index_reads` full 12-prefix matrix + exact positive status/metadata controls; changed_paths suite 35 passed (`51dc8528-3f08-4cb2-ab5d-e984075219ef`); captured-cache direct/wrapper/enriched committed facts retain one leaf.rs Added under every mutation; no live-index reinitialization |

- Inventory: one unblocked parent finding, eight named sites, 15 consumers, and four preservation/documentation controls; no nonblocking recommendation or performance measurement. Historical findings are regression controls, not additional parent counts.
- Defect class: upstream binary index decoding can unwind across fallible public readers and optional optimizations, bypassing typed error contracts.
- Input field robustness matrix does not apply to this existing Git binary decoder; permanent tests must cover all 12 truncations plus restored tool-written, absent, empty, unborn and linked controls.

- Orchestrator independently swept live index loads: optional sizing, rich context, conflict reader and three status-iterator entry points form the repair boundary. Shared-cache metadata consumers and linked detail setup are in scope; downstream declaration reports intersect only after successful Git validation.
- Final verification scope is sniff and tree-hugger. Their lint recipes were inspected and invoke no formatter. Hermetic Git/API/CLI L1 is the required tier; no rendering or L2/L3 behavior changes. OS skill loaded: BUILD_LINUX is available; Windows, WSL and remote macOS variables are absent. Focused Linux runtime checks will supplement local macOS gates.


- starting the work on 'R3-F1 — Truncated Git index containment' at 21:37:15
        - Defect class: upstream binary decoders can panic on malformed external input and escape APIs that promise fallible results.
        - Swept every production live-index reader: optional cache sizing, three status iterator initializers, rich enrichment context, and conflict reader. Both cache callers feed the same helper; metadata consumers must preserve their existing outputs.
        - Implemented one index containment helper used by optional cache sizing and conflict/rich index reads; shared iterator initialization protects all three status-reader paths without a preflight or extra observation.
        - Made rich context construction fallible so a failed index reload cannot degrade to empty evidence.
        - CLI restored real-tool matrix passed: 25 tests, all main/linked prefixes × commands × bases × supported modes; exact exit 1, empty stdout, repository/index cause, no cache stats. Nextest `32ffa806-10a5-41de-9a10-874f79c76d7e`.
        - API control initially caught an incorrect fixture expectation: worktree `is_current` refers to process CWD, not the discovered handle. Corrected the independently expected value; added direct rich-context and exact status/count controls before rerunning.
        - Public API regression exposed another sibling: gix's committed-diff resource cache indirectly loads the live index for attribute mappings. Backtrace identified recent-commit collection; sweep found both commit-file and path-history initializers too. Added S13/C16/C17 and guarded all three through one shared initializer.
        - Added S14 for the merge predictor, whose committed-tree attribute/cache construction explicitly bypasses the live index.
        - Worktree details with no linked registrations correctly remain empty; registered linked details still require valid status and reject corrupted administrative indexes.
        - Upstream source sweep found the convenience tree-diff iterator also creates that resource cache internally, including the tip comparison. Replaced it with the same guarded explicit cache, preserving existing work and no line-diff enrichment; added S15 and direct malformed/positive tip-diff controls.
        - Added captured-cache controls for commit-file wrappers and enriched committed changes: cached attribute snapshots retain the independently expected committed path/kind even after the live index becomes malformed.
        - Test registration verified: API module is included under the existing `changed_paths_tests` L1 unit target; CLI module is declared in the `changed` binary. Both area tier checks passed with zero stranded tests.
        - Current docs and skill guidance now describe typed malformed-index failures, valid absent/empty state, metadata preservation, committed-diff attribute dependencies, and committed-only merge independence.

- Orchestrator inspection verified shared containment for all direct and indirect index loads, fallible context construction, four committed-diff initializers (including implicit tip comparison), metadata/compatibility behavior, captured-cache and committed-only merge invariants. Permanent CLI tests register 24 bounded prefix cases and one absence/empty case; API matrix asserts independently expected valid paths/status/counts/history before corruption.
- Final full-area L1 runs started after code/test/documentation freeze: tree-hugger `just test`; sniff `just test --test-threads 4` (the established macOS host-probe concurrency setting). No L2/L3 resource or performance threshold is required.
        - Final focused checks passed after the last source/test edit: sniff `just test changed_paths` 35 passed; sniff `just test status_context_rejects` 1 passed; tree-hugger `just test --test changed` 110 passed, zero skipped. Both tier-coverage checks and `git diff --check` passed.
        - Reconciled every required/discovered child row with independent output/error evidence. No child remains pending, blocked, deferred, or unresolved. Full area gates and parent closure remain orchestrator-owned.
- work completed for 'R3-F1 — Truncated Git index containment' at 21:47:04

- Final sniff macOS L1 passed: **3,157 passed**, 32 existing skips, 131.125 s; area `just lint` passed. Focused native Linux sniff cross-check passed **36** tests (2,250 filtered/skipped), snapshot `8304ad172`: 35 changed-path tests plus direct context regression. Tree-hugger full L1, remaining lint/tier gates and Linux CLI cross-check continue; no final failure has occurred.

- Initial full tree-hugger gate failed: **1,693 passed**, two unchanged CLI cache-robustness tests timed out at 30.015 s, 213 tests not run after fail-fast (199.085 s). Timed-out names: `every_kind_evidence_identities_name_retained_declarations_in_every_language` and `retained_geometry_written_paths_and_candidate_order_are_checked_in_every_language`. No malformed-index assertion failed. Diagnose with isolated unchanged cases, then rerun full L1 at four workers after concurrent compile work finishes; no retries, timeout changes, tier changes or weakened assertions.
- Both area lint commands and strict all-target sniff Clippy passed. Native Linux tree-hugger changed target passed **110** tests, zero skipped (4.955 s), snapshot `772600181`. Every subagent and both remote commands finished and results were read.

- Timeout diagnosis: both unchanged cases passed in isolation at two workers (6.623/7.030 s; nextest `3eed7579-ecf8-42ca-a66a-6e60f52fa953`). Full 1,908-test L1 rerun started with four workers after competing compiles finished. Recorded observed macOS contention in OS skill; source/tests and timeout policy unchanged. Both final tier checks passed with zero stranded tests.

### Completion Report

- Outcome: **complete** — implementation complete, ready for review.
- Finished at: `2026-10-07T21:58:18-07:00`; elapsed duration: **22 min 25 s**, measured from the requested start timestamp.
- Parent finding count: `total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0`.
- Reconciled ledger: **34 unique rows** — one fixed parent and 33 verified children (26 fixed, one already fixed, six clean). No pending or open row remains. Historical findings are verified regression controls rather than additional parent counts.
- Verified repair: shared containment handles every direct live-index load and status iterator initializer, both optional cache-sizing call sites, rich-context reloads, and explicit/implicit committed-diff cache initialization. Malformed indexes produce typed Git errors and CLI exit 1 before successful output. Metadata-only APIs and compatibility wrappers preserve their existing results; captured caches and committed-only merge prediction remain independent of subsequent live-index corruption.
- Permanent restored tool-written matrices cover all 12 short/long truncations, main/linked administrative indexes, every declaration command/base/report mode, independent positive paths/status/counts/history, absent indexes, tool-written empty indexes, and unborn repositories. Existing ownership/type-change/cache/cost controls pass.
- Open finding/child IDs: **none**. No blocker or deferral remains; next action is independent review.
- Final verification commands/results tied to R3-F1 and its children:
        - `tree-hugger/`: `just test --test-threads 4` — **1,908 passed**, zero skipped, 363.933 s; nextest `1ba1cd41-8cbf-4d48-a1ff-f6697c86a1dd`. Full final L1 verifies every declaration/cache/ownership consumer, including the two initially timed-out unchanged tests.
        - `sniff/`: `just test --test-threads 4` — **3,157 passed**, 32 existing skips, 131.125 s; nextest `b7e9ae7b-102a-4f41-948a-245f09174da3`. Full final L1 verifies shared Git readers and neighboring request/metadata consumers.
        - `tree-hugger/`: `just lint` — **passed**, library and CLI all-target Clippy.
        - `sniff/`: `just lint` — **passed**, including remote feature coverage.
        - Repository root: `cargo clippy -p sniff --features remote -p sniff-cli --all-targets -- -D warnings` — **passed**, including test targets.
        - Repository root: `just check-tier-coverage sniff` and `just check-tier-coverage tree-hugger` — **passed**, zero stranded tests each. New test modules are compiled by declared existing targets and their ordinary names execute in L1.
        - Repository root: `GIT_TERMINAL_PROMPT=0 just cross-check sniff --os linux --features remote changed_paths status_context_rejects` — **36 passed**, 2,250 filtered/skipped, 0.278 s; snapshot `8304ad172`, nextest `394c6767-961d-460e-a94b-bc443238dcb2`. Native Linux reader/context/cost/type-change runtime evidence.
        - Repository root: `GIT_TERMINAL_PROMPT=0 just cross-check tree-hugger-cli --os linux --no-default-features --test changed` — **110 passed**, zero skipped, 4.955 s; snapshot `772600181`, nextest `a293e611-1060-4e06-8b6b-2fd1c6a0c57c`. Native Linux CLI corruption and historical selection/report/cache evidence.
        - Focused macOS checks: sniff `just test changed_paths` — **35 passed**; sniff `just test status_context_rejects` — **one passed**; tree-hugger `just test --test changed` — **110 passed**.
        - `GIT_TERMINAL_PROMPT=0 git diff --check` — **passed**.
- Initial gate failure accounted for: default-worker tree-hugger full L1 ran alongside sniff tests and Clippy, passed 1,693 tests, timed out two unchanged cache-robustness tests, and left 213 tests unrun after fail-fast. Both cases passed in isolation (6.623/7.030 s) and in the full four-worker run (14.417/8.593 s). No timeout, assertion, tier, or automatic retry policy changed. Recorded observed contention in the OS skill.
- Performance: this review requires no wall-clock measurement or benchmark. One-observation/no-enrichment work assertions passed; `deferred_perf_measurement: false`. No deferred-performance document is required.
- OS evidence: full macOS gates and focused native Linux runtime checks passed. BUILD_WIN, BUILD_WSL and remote macOS were not declared; no Windows/WSL2 runtime claim is made. Existing CI retains those environments; this repair adds no CI cells.
- Changed files for this cycle:
        - `sniff/lib/src/filesystem/git/open.rs`, `status.rs` — shared containment, guarded status initialization, and fallible rich-context reload.
        - `sniff/lib/src/filesystem/git/discovery.rs`, `recent_commits/collect.rs`, `types.rs` — guarded committed-diff initialization and accurate error contracts.
        - `sniff/lib/src/filesystem/git/changed_paths_tests.rs`, `index_reads_tests.rs` — registered public API and sibling matrices.
        - `tree-hugger/cli/tests/changed/main.rs`, `index_corruption.rs` — registered bounded CLI prefix/report and valid-state controls.
        - `sniff/docs/sniff-library-architecture.md`, `tree-hugger/cli/README.md`, `tree-hugger/docs/symbol-reports.md` — current corruption/error and metadata contracts.
        - `.claude/skills/sniff/SKILL.md`, `.claude/skills/tree-hugger/SKILL.md`, `.claude/skills/os/macos.md` — shared boundaries, permanent coverage, and observed contention guidance.
        - This `log.md` and `review-3.md` — reconciled ledger, completion report and required metadata.
- Review `implemented` is true, `implemented_by: codex/gpt-6.1-sol` and `log` are recorded; original `ready: false`, findings and body remain intact. Log `implementation_3` carries the requested timestamp. Existing author changes, staged index and lifecycle location were preserved. No formatter, staging operation, commit or lifecycle move was performed. Every started command and subagent finished and its result was read.
