---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_review_items: []
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-10T19:56:36-08:00
spec: 2026-11-06-windows-wsl-test-failures/spec.md
implemented: false
description: 'A **fix** review of `2026-11-06-windows-wsl-test-failures/spec.md`'
fix: 2026-11-06-windows-wsl-test-failures/review-2.md
previous: 2026-11-06-windows-wsl-test-failures/review-1.md
---

# Review 2

The fix is production ready under this review's readiness rules. Both previous findings are verified fixed; there are no new findings. Scheduled hosted confirmation remains incomplete, as recorded below, and is not represented as passing evidence.

## Previous Finding Verification

The required instances below include every child row in the repair's `log.md` Finding Ledger. Source inspection establishes the implemented behavior; recorded rig results supplement it and are identified as implementation evidence rather than fresh review executions. The previous review has no Human rulings section and no blocked finding.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: ucrt64 Git installation discovery remains broken in gix | R1-F1.1: four launcher/direct × mingw64/ucrt64 operations | verified fixed | Resolver retains executable selection and per-entry environment handling. Repair log records real init/status success in all four shapes. | None |
| R1-F1 | R1-F1.2: launcher prefix identity | verified fixed | All six manifests pin gix 0.90.0; locked gix-path 0.14.1 explicitly recognizes ucrt64. `named_installation` checks both prefix identity and an empty trace directory during discovery. Rig launcher controls pass. | None |
| R1-F1 | R1-F1.3: direct prefix identity | verified fixed | The direct-entry parent reaches the same child assertion; resolver derives the installation root from the direct executable. Recorded direct-entry controls pass. | None |
| R1-F1 | R1-F1.4: ambient listing discovery | verified fixed | CLI assertion rejects both `--exec-path` and `--show-origin` discovery queries, covering the upgraded dependency's fallback. Recorded ucrt64 run passes. | None |
| R1-F1 | R1-F1.5: launcher listing discovery | verified fixed | Same trace assertion with the launcher first on PATH; recorded launcher matrix passes. | None |
| R1-F1 | R1-F1.6: direct listing discovery | verified fixed | Same assertion with the direct executable first on PATH; recorded direct matrix passes. Broken-hint negative control fails all five identity/listing checks. | None |
| R1-F1 | R1-F1.7: environment parity and hint removal | verified fixed | `forget_installation_hint` remains in child construction; launcher environment edits and inherited direct environment are preserved. Existing child-environment assertions remain active; recorded matrix passes. | None |
| R1-F1 | R1-F1.8: six consumers and migrated error handling | verified fixed | Inspected discovery, optional HEAD/reference lookup, merge-base, path conversion, worktree metadata, config access, diff callbacks, and shallow-file migrations. Missing repositories remain distinct from read failures. New malformed-config, invalid-prefix, and unrelated-history tests exercise the changed decisions. Fresh `just test sniff`: 3,565 passed. Repair log records all six areas' test/lint checks and workspace compilation. | None |
| R1-F1 | R1-F1.9: invalidate older mingw64 receipts | verified fixed | Dependency and Cargo.lock inputs change; the repair records different closure keys. Old receipts are not claimed as proof of this tree. | None |
| R1-F1 | R1-F1.10: sibling prefix interpreters | verified fixed | Worktree's four-prefix table matches gix-path's candidates; both reject multiple directories. Sniff's extra system-config lookup is installation-root based, independent of prefix selection. | None |
| R1-F2: cold compiler tests still exceed the L1 time budget | R1-F2.1: four pass and two failure cases | verified fixed | `tests/derive.rs` compiles all four fixtures through the macro and executes their assertions. Unit tests require expansion failure for both invalid fixtures and compare message/span diagnostics with existing stderr fixtures. Fresh model_id run passes all nine tests, maximum 0.024 s. | None |
| R1-F2 | R1-F2.2: cold WSL2/macOS consumer | verified fixed | No runtime compiler or registry access remains. Recorded cold macOS/Windows runs and relocated WSL2 run execute all nine tests, maximum 0.045 s across those runs. | None |
| R1-F2 | R1-F2.3: warm control | verified fixed | Fresh `just test model_id`: nine passed, no skips. | None |
| R1-F2 | R1-F2.4: Cargo absent | verified fixed | There is no Cargo availability branch or early success. Pass fixtures must compile during archive production; failure fixtures must return errors at execution. Cargo absence cannot bypass an assertion. | None |
| R1-F2 | R1-F2.5: offline dependency prerequisite | verified fixed | Nested builds and their fetch prerequisite are removed together. Workflow still provisions toolchains for packages that require them; the workflow contract again rejects consumer Cargo invocations. No test downloads dependencies. | None |
| R1-F2 | R1-F2.6: fixture selection guards | verified fixed | Pass-case macro declares compiled modules and their inventory together; both directories have inventory guards. All current failure fixtures have explicit test functions and expected diagnostics. Fresh execution includes both guards. | None |
| R1-F2 | R1-F2.7: schematic runtime sibling | verified fixed | Existing `slow_` test is outside L1; no tier change was used to conceal this repair's cost. | None |
| R1-F2 | R1-F2.8: schematic schema sibling | verified fixed | Existing slow-tier case is in a workspace-excluded crate; unchanged by this repair. | None |
| R1-F2 | R1-F2.9: model_id lint selection | verified fixed | Unchained AI's lint and lint-fix recipes now include model_id. Repair log records passing lint. | None |

The compiler-test repair changes the measurement method: compilation of pass fixtures happens when the archive is built, and invalid inputs are checked against the macro's expansion function when tests execute. This preserves the required rejection, message, and span assertions without nested compiler startup. The production entry point still maps expansion errors directly through `syn::Error::into_compile_error`. The tests do not independently verify rustc's diagnostic rendering; this is documented, and the repair records a temporary real-compiler check of both unchanged stderr fixtures. Requiring ongoing tests of the compiler's rendering would exceed the behavioral guarantee needed here.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec coverage and verification level

| Requirement | Verification level and assessment |
|---|---|
| Full path identity, native diagnostic spelling, and portable output | L1 filesystem/library/CLI assertions remain intact. Original recorded Windows short- and long-TEMP reproductions cover the path groups; no repair in this iteration weakens them. |
| Symlink ownership and rejection outside the owning repository | L1 real-filesystem symlink and changed-command tests; platform-specific traversal remains covered, including ordinary-directory controls. |
| Git launcher/direct behavior and prefix configuration identity | L1 real native Git operations and process traces; four-shape rig controls are recorded. Source inspection confirms the dependency now recognizes the same prefixes as the resolver. |
| Macro success, rejection, and offline archive execution | L1 real compilation of pass fixtures, in-process expansion-error assertions, and relocated consumer execution. Tests are compiled by default targets and have no tier marker excluding them from L1. |
| Messenger descendant settlement before releasing the lock | L1 real helper processes and lock/state/liveness assertions. The existing bounded readiness fixture remains; hosted confirmation is still pending. |
| Windows-only font-test compilation | Recorded native Windows affected-binary build and embed-test execution; unchanged in this iteration. |
| WSL2 archive relocation | Recorded relocated archives with builder target hidden; model_id now also executes without Cargo. Hosted producer-to-consumer confirmation remains external. |

No changed requirement concerns terminal-emulator rendering or physical keyboard encoding, so no L2 or L3 verification is required. The input robustness matrix does not apply: this repair changes no configuration grammar, loader, or field validator. The gix API migration retains existing configuration projection behavior; removing a toolchain declaration does not add a new validator.

Fresh review checks: `just test model_id` (9 passed, no skips), `just test sniff` (3,565 passed, 33 excluded by the selected tier/platform policy; no executed test at or above five seconds), and `git diff --check` (clean). Reviewed the spec, previous review, both implementation logs, original repair commit scope, current source changes, manifests, lockfile, tests, workflow removal, and updated documentation. Native Windows and WSL2 results above are recorded implementation evidence; this review did not rerun remote rigs or dispatch a workflow.

## Remaining external validation

Three consecutive green scheduled runs on main have not yet occurred. Hosted messenger confirmation and hosted Ubuntu-to-WSL archive execution also remain pending. Those limitations stay explicit in the implementation log. They are cross-OS evidence obligations, which the review instructions expressly exclude from readiness findings. The former ucrt64 defect is repaired in source and exercised by the recorded controlled native-Windows matrix; it is no longer deferred behind hosted confirmation.

## Recurrence

No recurring defect found. Neither previous finding has an unchanged failing instance, a repair-induced regression, or a newly discovered failing sibling.
