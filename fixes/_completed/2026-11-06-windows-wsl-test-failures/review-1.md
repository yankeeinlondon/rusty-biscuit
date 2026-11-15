---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): ucrt64 Git installation discovery remains broken in gix'
  - 'R1-F2 (high): cold compiler tests still exceed the L1 time budget'
human_review: true
human_review_items:
  - 'Use discovered build-host aliases per invocation; no repository configuration change is needed.'
  - 'Treat the messenger failure as consistently reproducible on hosted runners, based on four failed nightlies.'
  - 'Place the compiler-toolchain requirement in the manifest table the existing planner supports.'
  - 'Do not accept the deferred ucrt64 Git repair; complete it under finding R1-F1.'
  - 'Reject Git installations with multiple prefix directories, including leftover directories, to avoid choosing the wrong configuration.'
  - 'Pass the messenger parent helper role through an argument and the grandchild role through its own environment.'
  - 'Keep the Windows ownership product repair and reject whole-target normalization because it erases paths that ownership checks must inspect.'
  - 'Compare worktree refusal paths using the spelling supplied by the relevant source.'
  - 'Keep the two new documentation pages for Windows Git execution and compiler-test prerequisites.'
  - 'Use rig-built relocated WSL2 archives as intermediate evidence, with hosted producer-to-consumer confirmation still pending.'
  - 'Accept the broader local WSL2 L1 runs used to search for relocation failures.'
  - 'Reuse passing macOS checks for unchanged code and rerun the tests affected by edited skill documents.'
  - 'Review the published rig receipts knowing that the Windows rig used mingw64 and does not clear the known hosted ucrt64 failure; repair R1-F1 before relying on those results.'
  - 'Continue review before post-merge nightlies; keep the three-run confirmation explicitly incomplete.'
  - 'Allow locked dependency fetching only in a compiler-toolchain-gated archive-consumer step.'
  - 'Do not accept the cold compiler test exceeding five seconds; address it under finding R1-F2.'
  - 'Keep disposable Windows installations and private clones for controlled reproductions rather than changing shared tools.'
  - 'Keep the WSL2 compiler-test capability gap explicit; retain the toolchain-free guest and leave hosted provisioning with its prerequisite.'
  - 'Accept the expected-test listing for the relocated L1 runs; filtered-out other tiers are not claimed as executed coverage.'
  - 'R1-F1: upgraded all six gix consumers to =0.90.0 (newest) rather than 0.89.x; 0.89 would also recognize ucrt64 with a smaller API change.'
  - 'R1-F1: sniff is_ancestor with no merge base now returns Ok(false) instead of Err, matching git merge-base --is-ancestor; the old Err came from gix typed NotFound.'
  - 'R1-F1: merge-conflict prediction expectation changed from node to node~HEAD for a file/directory conflict, matching real git merge output.'
  - 'R1-F1: identity and no-subprocess Git tests tightened (empty trace2 directory; reject --show-origin) because gix 0.90 made the old assertions vacuous.'
  - 'R1-F2: replaced model_id trybuild with compiled pass fixtures and in-process diagnostics compared to the .stderr files; rustc rendering and into_compile_error are no longer exercised.'
  - 'R1-F2: removed model_id requires-toolchain, its cargo fetch workflow step, and the contract exception, so the WSL2 model_id cell now executes instead of being an accepted gap.'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-10T17:42:18-08:00
spec: 2026-11-06-windows-wsl-test-failures/spec.md
implemented: true
description: 'A **fix** review of `2026-11-06-windows-wsl-test-failures/spec.md`'
fix: 2026-11-06-windows-wsl-test-failures/review-1.md
log: '/Volumes/coding/rusty-biscuit/fixes/2026-11-06-windows-wsl-test-failures/log.md'
implemented_by: claude/opus
next: 2026-11-06-windows-wsl-test-failures/review-2.md
---

# Review 1

The fix is not production ready. Most path and fixture repairs preserve their original guarantees, but the runner's Git layout still fails and the cold compiler test remains over budget. Neither repair requires an author decision: completing the Git repair and keeping ordinary tests within five seconds are already required. This is the first review, so there are no previous findings to reconcile.

## Findings

### R1-F1 (high): ucrt64 Git installation discovery remains broken in gix

**Authority: spec.** The outcome requires that “The Windows Git-layout failures are either corrected with regression coverage or disproved with recorded evidence explaining the actual cause.” The scope also states, “Any product defect found gets a product fix.” The implementation log instead confirms the original failures remain on the actual runner layout.

This is an incomplete installation-identity propagation repair: the worktree resolver recognizes a prefix that the pinned gix configuration reader cannot recognize.

In the worktree package, [`name_installation_for_gix`](../../../worktree/lib/src/git/program.rs) supplies the installation root through `EXEPATH`; gix-path 0.12.1's `system_prefix_from_exepath_var` only checks `clangarm64`, `mingw64`, and `mingw32`. With Git 2.56's `ucrt64` installation, it falls back to a Git subprocess and derives the wrong prefix. The worktree manifest still pins gix 0.84.0. The implementation log's “Native Windows results: rules 7–8” records the controlled PortableGit reproduction, including real `git init` and `git status`, for both launcher and direct entry.

The sweep follows the installation identity from recognition through its reader and the listing decisions that consume it. Results below are the implementer's recorded native-Windows reproductions, corroborated by inspection of the current resolver, dependency source, and assertions; this review did not rerun the Windows rig.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | worktree `launcher_layout` and Git command construction | mingw64/ucrt64, launcher/direct | Real init/status succeed for all four shapes; clean | Preserve executable and environment behavior |
| R1-F1.2 | worktree `entries::through_the_launcher_the_named_installation_is_the_prefix_git_reports` | ucrt64 launcher; mingw64 control | ucrt64 fails: installation root instead of `root/ucrt64`; mingw64 passes | gix prefix identifies the Git-reported prefix |
| R1-F1.3 | worktree `entries::through_the_direct_binary_the_named_installation_is_the_prefix_git_reports` | ucrt64 direct; mingw64 control | Same failure; mingw64 passes | Same identity for direct entry |
| R1-F1.4 | git-wt-cli `a_listing_never_asks_git_for_its_exec_path` | ucrt64 ambient entry; mingw64 control | Extra exec-path query; mingw64 passes | No discovery subprocess |
| R1-F1.5 | git-wt-cli `a_listing_through_the_launcher_never_asks_git_for_its_exec_path` | ucrt64 launcher; mingw64 control | Extra exec-path query; mingw64 passes | No discovery subprocess |
| R1-F1.6 | git-wt-cli `a_listing_through_the_direct_binary_never_asks_git_for_its_exec_path` | ucrt64 direct; mingw64 control | Extra exec-path query; mingw64 passes | No discovery subprocess |
| R1-F1.7 | worktree direct/launcher environment handling and child hint removal | Both entry types | No reported environment-parity failures in the same filtered runs; clean | Preserve inherited direct environment and launcher parity |

**Recommended repair:** complete the dependency compatibility work needed for gix to recognize ucrt64, preserving the existing identity and subprocess assertions. Prefer the compatible gix upgrade and required error-type migration over adding a second environment-override mechanism. This migration is larger than adding a prefix, but it repairs a normal installation used by the failing runner; it is not protection against a contrived input. Check every affected gix consumer for compilation and behavior where its error handling changes. Retain the four entry/layout controls above. The repair's changed inputs must invalidate the earlier mingw64 receipts; those receipts cannot establish that this defect is fixed.

### R1-F2 (high): cold compiler tests still exceed the L1 time budget

**Authority: spec and defect.** The spec requires, “Keep ordinary tests within the repository's five-second L1 budget.” The changed model_id test's normal archive-consumer input violates that budget: after fetching into a fresh Cargo home, the log records `ui` passing in **10.308 seconds**. It also records 9.1 seconds on macOS and 6.19 seconds in a fresh WSL2 extraction. This is an actual execution-time defect in a changed test, not missing cross-OS proof.

This is a test-work aggregation defect: one L1 test performs the complete nested compiler case set on a cold consumer.

In the model_id package, [`ui`](../../../unchained-ai/model_id/tests/ui.rs) groups four compile-pass fixtures and two compile-fail fixtures in one test. The prerequisite fetch downloads sources but does not remove the nested compiler work from the measured test. Calling that cost inherent does not meet the budget.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | model_id `ui`, pass and compile-fail branches | Fresh Windows archive consumer after locked fetch | All six compiler cases execute, but aggregate test takes 10.308 s | Preserve all six checks with every L1 test within 5 s |
| R1-F2.2 | Same public test | Fresh WSL2 extraction with Cargo; macOS implementation run | 6.19 s and 9.1 s respectively | Same budget where execution is supported |
| R1-F2.3 | Same public test, warm control | Review's `just test model_id` | Both tests pass; `ui` takes 3.685 s | Clean warm control; does not clear cold-consumer failure |
| R1-F2.4 | Same prerequisite branch | Cargo absent | Explicit failure recorded in the log; clean | Never silently pass without compiler checks |
| R1-F2.5 | Workflow prerequisite and offline branch | Fresh Cargo home without fetch, then with fetch | Missing dependency fails before fetch; six cases pass after fetch; clean correctness | Preserve offline failure and success semantics |

**Recommended repair:** first measure which nested work dominates, then divide compiler cases into independently selected tests where that makes the cold tests meet the budget. Keep a guard against adding an unselected fixture. Splitting is not sufficient if each test repeats the same expensive cold compilation; verify the resulting cold archive-consumer timings before claiming success. Preserve compile-pass and diagnostic assertions, offline execution, and the existing toolchain capability policy. Do not rename the test into a skipped tier, increase timeouts, or move compiler work into the archive-consumer workflow to disguise its cost.

## Verification and coverage

Reviewed the spec, plan, implementation log, the three implementation commits affecting production/test code (`f6270b859`, `348d5ee28`, `758c39850`), current callers, manifests, workflow prerequisite, and package documentation. Inspected the pinned gix-path source to confirm the missing ucrt64 reader branch. Ran `just test model_id`: 2 passed, no skips, compiler test 3.685 s. Other platform results quoted here are recorded implementation evidence, not new review runs. No full-suite or hosted workflow was triggered.

| Requirement | Strongest applicable verification | Assessment |
|---|---|---|
| Full path spelling and identity assertions | L1 filesystem and CLI tests, including recorded short-TEMP controls | Appropriate; no weakening found |
| Windows ownership traversal and outside-checkout rejection | L1 real-filesystem symlink tests through changed-file commands | Appropriate; new per-OS fixture checks the OS-opened identity |
| Git layout, configuration prefix, and subprocess behavior | L1 native Git operations, child environment parity, CLI traces | Appropriate level; ucrt64 still fails R1-F1 |
| Compiler prerequisites and expected diagnostics | L1 extracted-archive consumer with fresh Cargo home | Correctness demonstrated; cold cost fails R1-F2 |
| Descendant settlement, lock ownership, and no surviving writes | L1 native helper processes, process liveness and state assertions | Appropriate; hosted confirmation pending |
| Windows-only font test compilation | Affected binary built and embed tests executed on Windows, recorded in log | Appropriate |
| Archive relocation | L1 relocated rig archives with builder target hidden | Useful intermediate proof; hosted Ubuntu-to-WSL confirmation remains external |

No changed requirement handles physical keypresses or terminal-emulator rendering, so L2/L3 are not required for these repairs. The input robustness matrix does not apply: the changes do not introduce or change a configuration loader or field validator. The manifest adds a declaration to the existing reader, and the workflow fetches dependencies.

Missing three-nightly confirmation, hosted messenger confirmation, hosted archive provisioning, and the existing toolchain-free WSL2 capability gap are recorded limitations. Per this review's readiness rules, absence of cross-OS confirmation alone is not a finding. A positively reproduced unresolved Git defect is a finding.

## Implementation decisions (informational)

The two parallel decision extractions were reconciled with the log and the spec. No item needs a human before repair. The following choices and deferrals remain visible for the author's later review; no reaffirmation is required during this cycle.

1. Use discovered build-host aliases per invocation; no repository configuration change is needed.
2. Treat the messenger failure as consistently reproducible on hosted runners, based on four failed nightlies.
3. Place the compiler-toolchain requirement in the manifest table the existing planner supports.
4. Do not accept the deferred ucrt64 Git repair; complete it under finding R1-F1.
5. Reject Git installations with multiple prefix directories, including leftover directories, to avoid choosing the wrong configuration.
6. Pass the messenger parent helper role through an argument and the grandchild role through its own environment.
7. Keep the Windows ownership product repair and reject whole-target normalization because it erases paths that ownership checks must inspect.
8. Compare worktree refusal paths using the spelling supplied by the relevant source.
9. Keep the two new documentation pages for Windows Git execution and compiler-test prerequisites.
10. Use rig-built relocated WSL2 archives as intermediate evidence, with hosted producer-to-consumer confirmation still pending.
11. Accept the broader local WSL2 L1 runs used to search for relocation failures.
12. Reuse passing macOS checks for unchanged code and rerun the tests affected by edited skill documents.
13. Review the published rig receipts knowing that the Windows rig used mingw64 and does not clear the known hosted ucrt64 failure; repair R1-F1 before relying on those results.
14. Continue review before post-merge nightlies; keep the three-run confirmation explicitly incomplete.
15. Allow locked dependency fetching only in a compiler-toolchain-gated archive-consumer step.
16. Do not accept the cold compiler test exceeding five seconds; address it under finding R1-F2.
17. Keep disposable Windows installations and private clones for controlled reproductions rather than changing shared tools.
18. Keep the WSL2 compiler-test capability gap explicit; retain the toolchain-free guest and leave hosted provisioning with its prerequisite.
19. Accept the expected-test listing for the relocated L1 runs; filtered-out other tiers are not claimed as executed coverage.
