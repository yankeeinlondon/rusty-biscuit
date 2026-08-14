---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-14T07:12:33+01:00
spec: 2026-08-14-ci-failure-triage/spec.md
implemented: false
description: A **fix** review of `2026-08-14-ci-failure-triage/spec.md`
fix: 2026-08-14-ci-failure-triage/review-2.md
previous: 2026-08-14-ci-failure-triage/review-1.md
---

# Review 2 — CI failure triage

## Verdict

This fix is **not ready for production**. The inventory reconciliation is detailed, the local Claudine fixture changes are sound, and the affected Claudine Level-1 and lint gates pass. However, the specification's native-Windows attribution, fresh WSL2 no-superset proof, Linux same-host Level-2 comparison, Apple Terminal assertion execution, baseline policy, authoritative full-CI run, and final identity-aware comparison are incomplete. The checked-in plan and evidence correctly report most of these gates as blocked; they are acceptance requirements, not optional follow-up work.

## Findings

### Critical — F5 and the authoritative acceptance gate are not implemented

Every Phase 5 task and Validation checkpoint 5 remains unchecked in `plan.md`. No qualifying post-fix branch artifact exists, `.github/ci/ci-baseline.toml` has not been updated from current evidence, `ci-rollup verdict` has not validated the intended policy, no authoritative full CI run contains the Phase 4 fixture corrections, and `just ci-diff` has not reviewed the resulting identities and zero-identity diagnostics. This leaves F5 and success criteria 1, 2, and 7 unproven.

Commit and push the separately reviewed Phase 4 changes through the authorized workflow, obtain a current comparable `main` run, reject every non-comparable or branch-superset cell, apply only the resulting evidence-backed baseline changes, and require both `ci-verdict` and the final manual identity/diagnostic review to pass.

### High — The known Claudine CLI WSL2 branch superset has not been cleared

Run `31753281913` contains four Claudine CLI WSL2 identities that are absent from the selected `main` run. The implementation plausibly addresses them by splitting three aggregate tests into independent `rstest` cases and restoring the inherited system `PATH` for the shipped-prompt process test. Those eleven resulting Level-1 identities pass locally on macOS, but no canonical archive run in the toolchain-free WSL2 guest contains the changes. Validation checkpoint 4 therefore remains open, and F4 explicitly forbids accepting or retaining the cell-wide baseline while the only authoritative evidence is a branch superset.

Run the full canonical Claudine CLI Level-1 archive in WSL2 after the changes are committed. Compare exact JUnit identities with a current `main` cell and proceed only if the branch set is equal to or a subset of `main`. A macOS process run is the correct local Level-1 regression check, but it cannot verify archive relocation, WSL2 process timing, or the guest's deliberately reduced toolchain.

### High — The Darkmatter CLI Windows contradiction remains unresolved

F2 requires the focused pretty-output identity to execute on the same native Windows host for both branch and `main`, with byte-level input and environment comparison. Tasks 2.1, 2.2, and Validation checkpoint 2 remain unchecked. The GitHub-hosted evidence narrows the difference to `%TEMP%` short-name spelling (`RUNNER~1` versus `runneradmin`) and shows the same failure on branch and `main`, but the recorded native Windows 11 aggregate still claims all 655 tests passed and does not prove that this identity executed.

Run the focused test and the Darkmatter CLI Level-1 gate on the specified native Windows host for both revisions. Record the actual bytes and the profile, filter, features, color/hyperlink state, and path spelling. Until then, the environmental explanation is credible but does not satisfy F2 or success criterion 3.

### High — F3's Level-2 evidence does not close the required Linux and Apple Terminal observations

The Linux comparison uses separate GitHub-hosted workflow runs. That provides matched operating-system and runner-class evidence, but not the same-host branch-versus-`main` execution required by F3. The four Claudine context failures, Biscuit Terminal Mermaid fallback failure, and Darkmatter light-terminal theme failure are therefore not controlled against host-local state as specified.

The Apple Terminal double-underline identity has an additional verification gap: the branch-side local run timed out while attaching or spawning the harness, before the command and assertion executed, and both hosted macOS cells likewise fail during harness setup. A passing `main` control does not establish the branch's visible output. Because the contract asserts specific terminal-visible text and absence of unsupported SGR sequences, the strongest appropriate verification is Level 2 and it must reach the capture assertions.

Run branch and `main` from clean checkouts on the same Linux host with identical tmux, dimensions, environment, and binaries. Run the Apple Terminal identity through the canonical Level-2 harness without focus acquisition and retain a branch capture that reaches its positive and negative assertions. If the harness remains unable to execute, keep the identity unresolved rather than treating setup failure as product verification.

### Medium — The review iteration chain is missing its previous artifact

`fixes/2026-08-14-ci-failure-triage/review-1.md` does not exist in the worktree, repository history, or other local worktrees. Consequently, the claimed prior findings cannot be independently compared with this implementation, and the requested `implemented: true` and `next` frontmatter updates cannot be applied to the actual previous review. `review-2.md` retains the required `previous` value, but that reference is currently dangling.

Restore the original review artifact from its authoritative source, then set `implemented: true` and `next: 2026-08-14-ci-failure-triage/review-2.md`. Do not reconstruct substantive review findings from the later implementation notes.

## Requirement verification matrix

| Requirement or observable contract | Strongest verification present | Assessment |
| --- | --- | --- |
| F1 — complete failed-producer inventory and identity/diagnostic reconciliation | Artifact/API comparison and checked-in evidence; not a terminal interaction | Appropriate evidence is present for 25 failed producers, 55 distinct identities, and two zero-identity lint cells. |
| F2 — Darkmatter CLI pretty bytes on native Windows | Level 1 process test on `windows-latest`; aggregate native-Windows result | Wrong host proof: focused same-host native branch/control execution is missing. |
| F3 — Claudine context columns, wrapping, and 140-column cap | Level 2 tmux capture | Appropriate level exists, but the Linux branch/control runs are not same-host and the hosted observations remain red. |
| F3 — Darkmatter inherited-dim and light-terminal theme colors | Level 2 real-terminal capture | Appropriate level exists; macOS targets pass, but the Linux light-terminal attribution lacks the specified same-host control. |
| F3 — Mermaid fallback without an image protocol | Level 2 tmux capture | Appropriate level exists; the Linux attribution lacks the specified same-host control. |
| F3 — Apple Terminal double-underline visible text and SGR degradation | Level 2 Apple Terminal capture test | Insufficient execution: branch runs stop during harness setup before capture assertions. |
| F4 — split launch-context matrices retain every direct, inline, and loop case | Level 1 real-CLI process tests | Appropriate level and passing locally on macOS; fresh WSL2 archive execution is missing. |
| F4 — shipped context prompt retains package-area rendering with system tools available | Level 1 real-CLI process test plus passive shipped-prompt corpus test | Appropriate local level and passing; WSL2 archive/path proof is missing. |
| F4 — canonical full-Level-1 WSL2 contract and no branch superset | Level 1 in a real WSL2 archive guest, but only before the corrections | Stale for the implementation under review; the known four-identity superset remains authoritative. |
| F5 — baseline policy, verdict, and final identity/diagnostic comparison | No completed post-fix CI-policy evidence | Not implemented. |

No requirement concerns a physical keyboard, mouse, paste, IME, or terminal input encoder, so Level 3 OS input injection is not applicable. The required terminal-rendering observations are Level 2; the CLI/process and WSL2 archive observations are Level 1.

## Verification performed

- `cd claudine && just test --no-fail-fast` — passed all five package suites: 21 catalog tests, 4,043 library tests, 48 contract tests, 2,396 CLI tests, and 154 generator tests. The CLI result includes the split launch-context cases and shipped context prompt test.
- `cd claudine && just lint` — passed the 18 diagnostic guards, lifecycle documentation guard, rustfmt check, and Clippy for all five packages.
- Static review of the specification, plan, evidence, failed-cell catalog, parent handoffs, implementation diff, test fixtures, and review schema.

The only test-run warning was the pre-existing macOS linker warning about compact-unwind offsets. It is unrelated to this fix.

## Code quality and performance

Splitting the aggregate matrix tests is an ergonomic improvement for nextest: every launch-source case now has its own identity, timeout budget, and failure report without weakening assertions. Prepending the fake provider directory to `PATH` also preserves the executable under test while allowing repository discovery to use guest-provided system tools. No production-code performance or API regression was found in the two changed Rust test files.

The remaining blockers are evidence and gate completion, not a reason to broaden the implementation. In particular, the branch should not absorb the recorded main-side rendering and fixture defects merely to make the old run green.
