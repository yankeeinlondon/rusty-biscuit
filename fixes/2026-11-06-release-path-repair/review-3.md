---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
  - |-
    Before approving the real release proposal, inspect its generated versions and changelogs. The verified calculation proposes git-wt-cli 0.1.1 and sniff-detection-cli 0.2.0. The latter follows release-plz's breaking-change calculation against the unchanged historical tag. This carries forward the previous review's release-approval item; no new decision is requested.
  - |-
    The shared setup builds release-plz 0.4.3 from a verified upstream archive with a small repository patch for renamed packages. This carries forward the previous review's informational implementation choice, without reopening it. The patch can be removed when an upstream version handles these historical identities.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: '2026-11-08T20:52:44-08:00'
spec: 2026-11-06-release-path-repair/spec.md
implemented: false
description: 'A **fix** review of `2026-11-06-release-path-repair/spec.md`'
fix: 2026-11-06-release-path-repair/review-3.md
previous: 2026-11-06-release-path-repair/review-2.md
---

# Release path repair review, iteration 3

The implementation is production ready within this repair's review boundary. Both previous findings are verified fixed, and there are no new findings. This does not authorize publication or claim that the App setup, hosted dry run, prerequisite verification on main, or first real release has happened. Those remain the explicitly recorded live-release prerequisites.

## Previous Finding Verification

Read review-1.md, review-2.md, both implementation logs, and the iteration-2 Finding Ledger before checking the implementation. Neither earlier review contains author decisions under a `Human rulings` section. The spec's author-approved two-package restriction remains controlling. Review-2 already established that repairing the pinned tool is within scope and does not require moving published tags; that decision is not reopened.

The required instances below include every iteration-2 ledger child, including the clean sites. Evidence paths refer to retained files inspected during this review, not merely the ledger's summaries.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — Release calculation cannot produce the approved proposal | All children below | verified fixed | Inspected the exact patch, successful Linux build/test records, unchanged-tag calculation, generated diff and report; reran release contracts. | Nothing in this finding. |
| R1-F1.1 — Dry run/shared setup | Shared working tool; actual Linux success | verified fixed | All three jobs use setup-release-plz. Action verifies archive, applies patch, tests, builds locked, checks version, then caches. Retained calculation exits 0. | Nothing. |
| R1-F1.2 — Proposal preparation | Shared calculation environment; clean-tree checks | verified fixed | `release-pr` and `update` share `calculation_environment`; actual update succeeds there. Proposal fixture and workflow clean-tree/credential contracts pass. | Nothing; live proposal remains separately authorized. |
| R1-F1.3 — Historical checkout regression | Original failure and repaired success | verified fixed | Both real-Git history-walk tests pass (1.030 s and 0.514 s), retaining the PowerShell rule and clean checkout checks. | Nothing. |
| R1-F1.4 — Publication/recovery | Exact head, complete rerun, tag-only recovery, partial failure under chosen build | verified fixed | Patch touches only calculation lookup/map/lock comparison. Upstream publication and Git tag code remain unchanged; exact-head, recovery, failure and pin-guard tests pass. | Nothing; no live publication claimed. |
| R1-F1.5 — Reports/candidate decisions | Actual approved edits, generated files, no dependency bump, unchanged tags | verified fixed | Retained report contains exactly git-wt-cli 0.1.0→0.1.1 and sniff-detection-cli 0.1.0→0.2.0; diff changes only their manifest/lock versions and changelog; new worktree changelog listed. Tag objects/targets and complete tag lists compare equal. | Nothing. |
| R1-F1.6 — Porcelain column | Preserve first status line's leading space | verified fixed | `_git_lines` preserves it; actual report begins ` M Cargo.lock`; report driver assertions pass. | Nothing. |
| R1-F1.7a — Historical git-wt-cli identity | worktree-cli at original tag | verified fixed | Patch's collected-package test uses this exact path/name pair; real Linux stderr resolves it; original tag unchanged. | Nothing. |
| R1-F1.7b — Historical sniff-detection-cli identity | sniff-cli at original tag | verified fixed | Same test covers sniff/cli; real calculation resolves it and produces 0.2.0 from the original baseline. | Nothing. |
| R1-F1.7c — Excluded darkmatter-lsp identity | dmls despite release exclusion | verified fixed | Test explicitly configures release=false yet asserts collection; real stderr resolves dmls, with no unexpected release candidate. | Nothing. |
| R1-F1.7d — Normal identity path | Current name first; neither name nor path fails | verified fixed | Read both control tests and retained 21-test passing output; both controls also pass without repair. | Nothing. |
| R1-F1.8 — Other consumers | Map key, lock comparison, credential/diagnostic/publication callers | verified fixed | Collected map keys use current names; lock comparison calls the same resolver. Lock regression fails without repair and passes with it. Caller/environment contracts pass; publication code is unpatched. | Nothing. |
| R1-F1.9 — Stale decision-only documentation | Docs, skill, log correction | verified fixed | Operational docs and skill describe the reproducible patched build and its removal; log explicitly corrects the earlier claim that an author ruling was necessary. | Nothing. |
| R2-F1 — HTTP fixtures time out on name resolution | All children below | verified fixed | Read numeric bind and resolver guard; reran every consumer through the normal recipe without diagnostic monkeypatches. | Nothing. |
| R2-F1.1 — FakeApi constructor | Bind without name lookup | verified fixed | LoopbackHTTPServer calls TCPServer.server_bind and supplies numeric server_name/server_port; every construction runs inside no_name_resolution. | Nothing. |
| R2-F1.2 — Positive HTTP consumer | Existing successful API fixture | verified fixed | Forge driver group passes in 0.184 s, including this case. | Nothing. |
| R2-F1.3 — Failed/malformed HTTP consumer | Errors remain errors, finish within budget | verified fixed | Same passing group covers malformed responses and HTTP failures. | Nothing. |
| R2-F1.4 — Proposal HTTP consumer | Credentials/environment/checkout assertions execute | verified fixed | Proposal driver group passes in 1.403 s. | Nothing. |
| R2-F1.5 — Other boundaries | Remaining release/integration fixture startup | verified fixed | One HTTP server constructor; remaining local probe uses a numeric socket bind, while in-memory and recording boundaries need no name lookup. | Nothing. |
| R2-F1.6 — Permanent startup guard | Reject attempted name resolution | verified fixed | Guard replaces all listed resolver functions with failures during construction and restores them afterward; both consumer groups execute with it enabled. | Nothing. |

The former unresolved historical-identity work is now repaired, rather than a newly discovered sibling or a changed tag policy. The two previous findings have no remaining partial instances.

## Unblocked Findings

None.

## Blocked Findings

None. No decision requires stopping the review loop.

## Recurrence

No finding in this review repeats an earlier defect class. The recurring R1-F1 from the first two reviews is now closed across its complete instance list.

## Spec review and verification

Reviewed preparation/publication event selection, App identity and credential ordering, branch freshness, exact merge topology, candidate selection, remote tag/Release inventory, isolated publication clones, recovery configuration, diagnostic/report behavior, integration routing, the shared setup action, patch, and current documentation. No additional actionable defect was established.

| Requirement | Strongest evidence examined | Assessment |
|---|---|---|
| Only successful canonical main-push CI prepares a proposal; stale main stands down | Level-1 executable event and Git fixtures; workflow contracts | Appropriate and passing |
| App-only writes; reject unrelated authors/forks; no dry-run credentials | Level-1 selection/command fixtures and workflow permission/token contracts | Appropriate before separately authorized App/live checks |
| Tag exactly the reviewed proposal head; refuse wrong topology or conflicting tag | Level-1 isolated Git/publication fixtures; unchanged pinned upstream publication source | Appropriate and passing |
| Recover missing Releases without moving tags; retain first failure | Level-1 recording-tool and remote-state fixtures; pinned-source recovery contract | Appropriate and passing |
| Diagnose and repair historical checkout/calculation | Level-1 real-Git regressions plus retained real-tool Linux calculation | Original failure and repaired success proven |
| Exactly approved package/version/changelog edits, including new files | Real-tool Linux report, diff, status and immutable tag comparisons | Approved set only; no dependency bump |
| Build only the matching integration at its tag commit | Level-1 executable selector/tag fixtures and workflow checkout/upload contract | Appropriate and passing; no artificial integration release needed |
| Clear current operational instructions | docs/releases.md and rust-devops contract inspected | Patched tool, App setup, review/merge/recovery and planned distribution described |

Commands run during this review:

- `cd tools && just test release`: 36 passed, none skipped within the selection, longest 1.904 s. This broad name filter includes one unrelated passing audio-spool test.
- `cd tools && just test integration`: one workflow contract passed. Executable integration selection cases also run within the release driver selection.
- `/tmp/actionlint.xrKp/actionlint .github/workflows/release-plz.yml .github/workflows/build-integrations.yml`: exit 0.
- Compared the checked-in-worktree patch byte-for-byte with the patch used for Linux evidence; equal. Compared before/after tag objects and targets, and clone/mirror complete tag lists; equal.

Cargo reported an unrelated registry cache-cleanup permission warning; both test commands exited successfully. Cargo automatically discovers ci_workflow_contracts.rs; these tests have ordinary Level-1 names, and the passing driver-group guard checks case assignment. No terminal rendering or keyboard behavior is changed, so Level 2 or Level 3 is unnecessary here.

Reused `/tmp/rpr/evidence-r2/` rather than repeating an expensive Linux calculation: inspected build script and output, tests-without-fix.log (19 pass, two relevant failures), build-with-fix.summary (21 pass, locked release build and version success), replay script, and `out-r2/r2-update-patched-0.4.3.*`. The actual calculation exits 0; its stderr resolves all three old identities, its status/report identify the new changelog, and its tracked diff contains no other package or dependency version changes. The replay uses repair configuration on a disposable main-based clone and the production calculation wrapper, not a hosted Actions execution; the retained hosted/live prerequisites remain explicit.

### Input robustness

Invalid consumed fields default to a located error before publication; supported unrelated events deliberately select no work. Required scalar and label validators distinguish absence/null, reject wrong whole-field and member types, and reject invalid empty scalars. An empty label list is a valid no-label case; absent optional carried release settings retain defaults. Shared JSON loading rejects duplicate keys, trailing content and nonstandard constants. TOML loading rejects duplicate/invalid syntax, while supported equivalent syntax remains accepted.

Read the shared validators and JSON/TOML loaders, configuration/manifest projections, and their driver matrices; the public-function and rendered-command groups pass. The coercion-pattern search found no `serde(default)`, `filter_map`, `unwrap_or_default`, or `.ok()` parse in production release_path.py. The patch reuses upstream Cargo metadata loading and changes package identity resolution, not a configuration loader; its controls, three renamed-member instances, excluded-member decision, current-name map projection and lockfile consumer are covered without adding a redundant format matrix.

## Release boundary

Implementation complete, ready for review. Production readiness here closes code defects; it does not certify the later live release. App installation/configuration, the prerequisite's successful main run, hosted dry-run execution, proposal approval and passing ci-gate, authorized merge/publication, final tag targets, and published-event delivery must still be verified as the spec requires. The approved first release contains no integration, so focused integration coverage remains sufficient until a genuine integration release occurs. No credentials, remote tags, branches, or releases were changed by this review.
