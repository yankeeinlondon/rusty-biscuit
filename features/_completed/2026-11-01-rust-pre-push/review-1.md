---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Checkout reuse relies on stale cleanliness and HEAD snapshots'
  - 'R1-F2 (high): Real recipe receipt and retry coverage is excluded from Windows'
human_review: true
human_review_items:
  - "Use local-tools for the launcher, as the spec recommends; no author action is required."
  - "Match the current gix 0.90 pin rather than the plan’s stale 0.84 pin; retain this choice."
  - "Apply the existing libgit2-sys ceiling to avoid Windows link failures; retain this choice."
  - "Accept Git source expressions while validating destination refs and lowercase object IDs; reject non-UTF-8 update lines; retain this choice."
  - "Put strict constraint validators in the first implementation phase and share them with matching code; retain this organization."
  - "Refuse reviewable pushes in the intermediate unwired implementation until review exists; this temporary state has been replaced."
  - "Validate raw mode and selection values at the public library boundary and keep production services in the library; retain this choice."
  - "Add fixture categories with their consumers and generate invalid cases by editing controls in memory; the temporary fixture deferral is closed."
  - "Defer CLI wiring checks until the integration target exists; that temporary deferral is closed."
  - "Use a declared fixture binary and integration target for archive-safe executable discovery; retain this choice."
  - "Share the plan renderer through the library while keeping ci-rollup’s independent inclusion; retain this choice."
  - "Use Git-default rename detection to match CI rather than user-specific settings; accept the documented threshold and tie differences."
  - "Kill supervised descendants after parent exit and bound output draining to 500 ms; retain this choice."
  - "Declare Windows thread-resume APIs locally, accept only executable-file tool candidates, and fall back to direct-child ownership if Job assignment fails; author may revisit that documented cleanup limitation after repair."
  - "Align both constraint readers on alphabetically first unknown-field errors and rejection of year zero; retain this choice."
  - "Use a callback-scoped PushSession to hold repository state and remote caches; retain this internal structure."
  - "Preserve exit categories while making an interruption marker block warn mode too, as the spec requires; retain this choice."
  - "Bind receipts to the first context of the first update carrying HEAD; retain this spec-aligned correction."
  - "Order evidence-withholding reasons, reject incomplete output, replan when HEAD moves, and treat missing just as status 127; retain these choices, subject to R1-F1’s cleanliness repair."
  - "Validate reviewed plans through both the full reader and constraint projection, refusing unreadable plans with status 1; retain this choice."
  - "Select Python once per reviewable push and relay captured helper stderr after completion; retain this choice."
  - "Write diagnostics immediately to stderr, using UnorderedList for constraint refusals and plain text elsewhere; exact ANSI styling is outside the compatibility contract."
  - "Treat an unreadable remote configuration as unknown identity, conservatively matching constraints; retain this choice."
  - "Refuse an interpreter prefix without its executable and refuse truncated prefix files; use sys.executable in Python test helpers; retain these choices."
  - "Keep two real receipt round trips in the four near-budget tests rather than seed evidence; current review run stayed below five seconds, so retain them."
  - "Compile real-recipe retry tests and Unix signal cases only on Unix; restore portable recipe coverage under R1-F2 rather than ask the author to waive it."
  - "Map equivalent retired shell rows onto existing Rust tests, share related cases, and omit ANSI assertions; retain the mapping approach."
  - "Relabel evidence fixtures for the host’s CI environment instead of adding a production override; retain this choice."
  - "Test publication failure with an unwritable local notes ref and cover remote status 4 in process; retain this bounded alternative to randomized retry delays."
  - "Embed launcher and dispatcher inputs in owning tests for changed-file scheduling; retain the small local fixture helpers."
  - "Split root hook test aliases into library and integration invocations to preserve nextest selection; retain this choice."
  - "Replace obsolete shell-source assertions with assertions on the Rust launcher, deletion arguments, and constraint enforcement; retain this choice."
  - "Leave native Windows execution evidence open after the rig’s storage preflight refused it; CI can supply OS evidence, and no storage override is requested."
  - "Document the unversioned constraint record in the central schema changelog rather than create another page; retain this choice."
  - "Tighten malformed constraint handling in both readers while preserving absent and empty optional wildcards; the implementation plan explicitly authorizes this documented correction."
  - "Capture first-context plans independently of successful scope recording, including zero bases, while withholding receipts for zero bases; retain this correction."
  - "Leave remaining evidence and planner ports to the later features named by the spec; this is an explicit scope boundary, not unfinished work here."
  - "R1-F2.27: real-recipe parity tests now run on Windows but take 5.7–12.3 s alone (up to about 22 s under load) on the Defender-on rig against the 5 s L1 budget; choose between seeding first receipts with a stub gate, trimming recipe/helper process spawns, or a Windows-specific budget for these tests."
  - "R1-F1 repair keeps an initially dirty checkout on a temporary tree even if it is clean by selection time, and reports mid-push dirtiness through existing withhold reasons rather than a new variant."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T00:42:38-08:00
spec: 2026-11-01-rust-pre-push/spec.md
implemented: false
log: /Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/log.md
implemented_by: claude/opus
description: 'A **feature** review of `2026-11-01-rust-pre-push/spec.md`'
feature: 2026-11-01-rust-pre-push/review-1.md
next: 2026-11-01-rust-pre-push/review-2.md
---

# Review 1

The feature is not production ready. Two findings require repair; neither waits on an author decision. The Rust launcher, strict input readers, constraint matching, and evidence publication have useful executable coverage, but checkout reuse can use an outdated state, and Windows cannot run part of the promised recipe coverage.

## Findings

### R1-F1 (high): Checkout reuse relies on stale cleanliness and HEAD snapshots

**Authority — spec:** “materializing a temporary detached worktree when the reviewed revision is not the clean checkout”; “Feed the first reviewed HEAD context’s plan to clean, unmodified local gates”; and “A dirty checkout ... still replans only for local feedback.” The implementation reads cleanliness once before interpreter probing and remote review, then reuses that snapshot. An editor saving a file, or another process checking out a commit while Python probing or a remote lookup runs, is a realistic input.

**Defect class:** an earlier repository-state snapshot is reused to authorize later execution against a mutable checkout.

In repo-deps, [review_update](../../scripts/ci-push/review.rs) chooses the checkout from `trigger == review.head` and `review.initial_dirty`; it does not check the current HEAD or cleanliness after resolving remote contexts. [run_gates](../../scripts/ci-push/mod.rs) refreshes HEAD but still uses the original cleanliness flag. This can run a changed planner as though it came from the outgoing commit, or feed the committed plan to gates whose working tree now includes changes that require different local feedback. The final validation check protects against a still-dirty tree after gates, but does not repair the earlier planning or gate decision. Scope-only exits without that final validation check.

I reproduced both dirty-state sites and the HEAD-change sibling through the public `ci_push::run` boundary, using the existing `Fake` fixture with one state transition. A temporary probe set `dirty = true` during interpreter selection; a scope-only feature push still reported `TreeSource::Checkout` and `ScopePublication::Published`. A second probe set `dirty = true` during scope-note publication; a strict push still passed `BISCUIT_CI_PLAN_IN` to its gate and only afterward withheld validation as `FinalDirty`. A third probe moved HEAD to the existing `OTHER` fixture commit during interpreter selection; scope-only review still selected `TreeSource::Checkout` and published scope for the originally captured HEAD. All three probes ran through `just _test repo-deps --lib review_probe_…`; their temporary source edits were restored.

The bounded sweep covers every repository-state decision in the review/gate/publication path:

| Child ID | Site | Shape tested or invariant | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | `review_push` initial capture | Clean control | Captures current identity and cleanliness | Clean; an initial snapshot is appropriate here |
| R1-F1.2 | `review_update` checkout selection | Clean at capture, dirty during interpreter probe | Uses checkout and publishes scope | Use a detached committed tree |
| R1-F1.3 | `review_update` identity selection | HEAD changes after capture | Public API probe moved HEAD during selection; still used checkout and published scope | Confirm checkout still holds the reviewed commit before reuse |
| R1-F1.4 | `run_gates` plan selection | Clean at capture, dirty during scope publication | Supplies committed plan | Replan for local feedback and withhold validation |
| R1-F1.5 | `run_gates` identity selection | HEAD changes before gates | Current `head()` comparison rejects plan reuse | Clean by explicit current-identity check |
| R1-F1.6 | `withhold_reason` final checks | Tree remains dirty, or HEAD differs after gates | Withholds validation | Clean; checks current HEAD and cleanliness |
| R1-F1.7 | Initial dirty/non-HEAD review and gate paths | Existing dirty and non-HEAD fixture cases | Detached review; dirty gates replan; validation withheld | Clean controls in the passing suite |

The probes change state at deterministic public service boundaries rather than relying on the timing of an editor or concurrent checkout. Existing passing controls cover initially clean, initially dirty, and non-HEAD pushes.

**Smallest repair:** refresh HEAD and cleanliness when deciding whether the checkout can serve as the committed review tree, and refresh cleanliness alongside the existing pre-gate HEAD check. Conservatively use a temporary tree or replan when the state cannot be read. Keep initial dirtiness as an independent reason to withhold validation. Add deterministic public-boundary transition tests; no filesystem watcher or locking system is needed, and this finding does not ask for atomic protection against every possible concurrent edit.

### R1-F2 (high): Real recipe receipt and retry coverage is excluded from Windows

**Authority — spec, Tests:** “Every behavior `test-pre-push.sh` covers today becomes an L1 nextest test ... so the suite runs on macOS, Linux, native Windows, and WSL2,” plus acceptance criteria 1–2 requiring the selected fallback interpreter to reach invoked helpers and strict gates. The implementation log explicitly leaves real-recipe parity rows Unix-only because the recipe uses Bash. Bash is also part of the Windows production path; excluding that path from compilation prevents CI from proving it even when Windows execution evidence becomes available.

**Defect class:** platform compilation guards remove the end-to-end consumer of the interpreter bridge and receipt retry behavior on the platform this port is intended to fix.

In repo-deps, [the parity tests](../../scripts/ci-push/integration/parity.rs) and [their real-recipe fixture](../../scripts/ci-push/integration/evidence_fixture.rs) put the recipe harness behind `#[cfg(unix)]`. The Windows launcher tests use a compiled `just` stand-in, and the fallback test records the environment handed to that stand-in. Neither executes the actual Bash consumer of `BISCUIT_CI_PYTHON` and its NUL-delimited prefix file on Windows. This is missing coverage, not a demand for local cross-OS proof.

For this coverage finding, the reproduction is target selection: each listed test has a Unix compilation guard, so it cannot appear in a Windows nextest binary. The corresponding tests executed successfully in this review’s macOS process suite. The bounded sweep includes every Unix-only parity function and both Windows launcher paths:

| Child ID | Site | Shape checked | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | Real recipe, two hook runs | Unix-only | Compile/run on Windows too |
| R1-F2.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | Receipt reuse after docs change | Unix-only | Compile/run on Windows too |
| R1-F2.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell` | Changed input invalidates evidence | Unix-only | Compile/run on Windows too |
| R1-F2.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | Failed evidence retry | Unix-only | Compile/run on Windows too |
| R1-F2.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | Real recipe and stacked base | Unix-only | Compile/run on Windows too |
| R1-F2.6 | `a_real_recipe_interruption_withholds_earlier_reports` | Malformed manifest / status 130 / recorded 143 | All three cases Unix-only, despite using manufactured gate outcomes rather than Unix signals | Compile/run on Windows too |
| R1-F2.7 | Signal-injection tests: `an_interrupted_hook_never_publishes_staged_evidence`, `an_interrupt_during_the_gates_ends_their_tree_and_publishes_nothing`, `a_termination_during_the_review_removes_its_temporary_worktree` | Unix signal operations | Unix-only | Clean platform-specific implementation; retain separate portable cancellation coverage |
| R1-F2.8 | Git Bash and PowerShell launcher tests | Real Git hook, real ci-push, stub just | Windows tests exist, but do not consume the interpreter bridge in the real recipe | Clean launcher coverage; insufficient substitute for rows above |
| R1-F2.9 | Broken-Python fallback parity test | Real Python helpers, stub just records interpreter variable | Cross-platform test exists | Clean direct-helper coverage; add real recipe consumption of fallback executable and prefix |

**Smallest repair:** make the existing real-recipe fixture work with the Windows Bash installation and native argument paths, then remove the Unix guards from the six recipe test functions above. Exercise the real recipe with broken `python3` and a selected `py -3` prefix in a focused Windows test. Keep actual Unix signal tests platform-specific. Reuse the existing L1 target and environment schedule; no extra CI workflow or matrix is required.

## Verification and robustness

- `just test-pre-push-hook`: 314 library tests and 67 integration/process tests passed on macOS. The longest process test took 4.987 seconds; none reached five seconds. No L2 or L3 is needed for hook decisions, subprocess argument forwarding, Git invocation, or evidence files; these are L1 requirements. This feature does not promise terminal keyboard or visual rendering behavior needing a stronger level.
- Three temporary in-process probes reproduced stale cleanliness and HEAD identity through the public orchestration result and gate arguments. Temporary implementation/test edits were restored; this review leaves no production-code changes.
- The declared `ci-push` integration target requires `local-tools`; the existing library tests and process target are selected by the canonical root alias. Launcher tests embed hook/dispatcher inputs inside tests. The issue in R1-F2 is conditional compilation, rather than a missing target or tier marker.
- The strict JSON loader rejects duplicates, invalid/trailing content, comments, trailing commas, empty input, and wrong root shapes. Shared required-text, optional-wildcard, positive-integer, array/object, and calendar-date validation has public-parser matrix tests, plus field wiring checks. Absent optional wildcard fields and explicit empty strings intentionally mean all; explicit null is rejected. An absent constraint-plan `state` has its documented older-plan meaning. No field-shape finding was found. The requested smell search found only date-component `.ok()?` conversions, which propagate invalid dates to the located error rather than supply permissive defaults.
- Full plan and constraint plan projections validate before constraint decisions; malformed records remain binding; scope and validation eligibility remain separate. Display-only renderer defaults are not used as decision authority. Receipt cell counting in `evidence.rs` is diagnostic only.
- The implementation’s native-Windows execution evidence is still open because its build rig refused the storage preflight. That absence does not itself block readiness in this review. R1-F2 concerns tests that cannot run there at all until repaired.

## Implementation decisions (informational)

Two agents independently read the implementation log and spec/plan in parallel before this comparison. Every recorded decision and deferral is listed below and summarized individually in `human_review_items`. None requires the author to stop the repair cycle. The author may reverse an informational choice after repairs; no reaffirmation is requested now.

1. Use local-tools for the launcher, as the spec recommends; no author action is required.
2. Match the current gix 0.90 pin rather than the plan’s stale 0.84 pin; retain this choice.
3. Apply the existing libgit2-sys ceiling to avoid Windows link failures; retain this choice.
4. Accept Git source expressions while validating destination refs and lowercase object IDs; reject non-UTF-8 update lines; retain this choice.
5. Put strict constraint validators in the first implementation phase and share them with matching code; retain this organization.
6. Refuse reviewable pushes in the intermediate unwired implementation until review exists; this temporary state has been replaced.
7. Validate raw mode and selection values at the public library boundary and keep production services in the library; retain this choice.
8. Add fixture categories with their consumers and generate invalid cases by editing controls in memory; the temporary fixture deferral is closed.
9. Defer CLI wiring checks until the integration target exists; that temporary deferral is closed.
10. Use a declared fixture binary and integration target for archive-safe executable discovery; retain this choice.
11. Share the plan renderer through the library while keeping ci-rollup’s independent inclusion; retain this choice.
12. Use Git-default rename detection to match CI rather than user-specific settings; accept the documented threshold and tie differences.
13. Kill supervised descendants after parent exit and bound output draining to 500 ms; retain this choice.
14. Declare Windows thread-resume APIs locally, accept only executable-file tool candidates, and fall back to direct-child ownership if Job assignment fails; author may revisit that documented cleanup limitation after repair.
15. Align both constraint readers on alphabetically first unknown-field errors and rejection of year zero; retain this choice.
16. Use a callback-scoped PushSession to hold repository state and remote caches; retain this internal structure.
17. Preserve exit categories while making an interruption marker block warn mode too, as the spec requires; retain this choice.
18. Bind receipts to the first context of the first update carrying HEAD; retain this spec-aligned correction.
19. Order evidence-withholding reasons, reject incomplete output, replan when HEAD moves, and treat missing just as status 127; retain these choices, subject to R1-F1’s cleanliness repair.
20. Validate reviewed plans through both the full reader and constraint projection, refusing unreadable plans with status 1; retain this choice.
21. Select Python once per reviewable push and relay captured helper stderr after completion; retain this choice.
22. Write diagnostics immediately to stderr, using UnorderedList for constraint refusals and plain text elsewhere; exact ANSI styling is outside the compatibility contract.
23. Treat an unreadable remote configuration as unknown identity, conservatively matching constraints; retain this choice.
24. Refuse an interpreter prefix without its executable and refuse truncated prefix files; use sys.executable in Python test helpers; retain these choices.
25. Keep two real receipt round trips in the four near-budget tests rather than seed evidence; current review run stayed below five seconds, so retain them.
26. Compile real-recipe retry tests and Unix signal cases only on Unix; restore portable recipe coverage under R1-F2 rather than ask the author to waive it.
27. Map equivalent retired shell rows onto existing Rust tests, share related cases, and omit ANSI assertions; retain the mapping approach.
28. Relabel evidence fixtures for the host’s CI environment instead of adding a production override; retain this choice.
29. Test publication failure with an unwritable local notes ref and cover remote status 4 in process; retain this bounded alternative to randomized retry delays.
30. Embed launcher and dispatcher inputs in owning tests for changed-file scheduling; retain the small local fixture helpers.
31. Split root hook test aliases into library and integration invocations to preserve nextest selection; retain this choice.
32. Replace obsolete shell-source assertions with assertions on the Rust launcher, deletion arguments, and constraint enforcement; retain this choice.
33. Leave native Windows execution evidence open after the rig’s storage preflight refused it; CI can supply OS evidence, and no storage override is requested.
34. Document the unversioned constraint record in the central schema changelog rather than create another page; retain this choice.
35. Tighten malformed constraint handling in both readers while preserving absent and empty optional wildcards; the implementation plan explicitly authorizes this documented correction.
36. Capture first-context plans independently of successful scope recording, including zero bases, while withholding receipts for zero bases; retain this correction.
37. Leave remaining evidence and planner ports to the later features named by the spec; this is an explicit scope boundary, not unfinished work here.
