---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-09T13:19:52-08:00
spec: 2026-10-28-publish/spec.md
implemented: false
description: 'A **feature** review of `2026-10-28-publish/spec.md`'
feature: 2026-10-28-publish/review-2.md
previous: 2026-10-28-publish/review-1.md
---

# Publish review — iteration 2

The implementation is **production ready under this review's readiness rules**. The previous finding is verified fixed, and this review found no new reportable defect. This does not authorize publication or claim that the first live release has succeeded. Approval, release setup, the tap, credentials, and outstanding runtime/install evidence remain pending under the existing rollout gates.

## Previous Finding Verification

I read `review-1.md`, the repair log's `Finding Ledger`, and the original implementation log. The previous review contains no `Human rulings` section. Its informational choices remain accepted; this review does not reopen them.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Rollout records installation success without proving the required channels | R1-F1.1–R1-F1.9, including all clean siblings and the repair's additional tap-status instance below | verified fixed | Read the workflow, tap installer/report reader, candidate generator, ledger parser, policy gate, and preflight gate; `just distribution-check` passes the public-result tests and workflow contracts | No remaining defect; live execution is still pending and is not counted as passing evidence |
| R1-F1.1: Homebrew workflow and candidate generation | Four target install jobs and separate formula-publication/install results | verified fixed | `brew-install` uses the four-target build matrix, invokes `tap.py install-check`, and retains each target's report. `rollout-candidate` depends on this matrix and passes its result separately. `rollout_candidate` rejects successful publication without successful installation. `InstallCheckTest` verifies tap installation, installed version, command execution, host matching, and refusal paths; the distribution workflow contract checks the dependency and inputs | None |
| R1-F1.2: First-chain candidate and widening gate | Disabled, unchanged, failed, and successful channel combinations | verified fixed | `RolloutCandidateTests.test_only_a_run_that_installed_every_channel_proposes_the_first_chain` passes candidate records through the parser and widening gate. Only all four channels passing permits wider distribution | None |
| R1-F1.3: Ledger parser and widening consumers | `parse_rollout`, `check_rollout`, `check_policy_rollout`, and preflight's `gate_rollout` | verified fixed | Parsed entries retain channel results. `check_rollout` selects only a complete bh entry; policy and preflight use that decision. Tests reject disabled/unchanged records, reject incomplete proof before any run lookup, preserve an older complete record beneath a newer incomplete one, and require a successful canonical workflow run | None |
| R1-F1.4: Archive smoke route | All four target jobs and downstream upload dependency | verified fixed | Workflow smoke jobs execute the archived command via `build.sh`; upload requires smoke success, and shell installation follows upload. Candidate generation requires installation success | None; original clean sibling remains clean |
| R1-F1.5: Shell installation route | Published installer, isolated installation, installed command execution | verified fixed | `install_check.sh` compares the downloaded installer to the built copy, installs into scratch HOME/root with a system PATH, checks the installed file set and profile writes, and runs `--version` and `--help`; workflow candidate generation requires the install matrix to succeed | None; original clean sibling remains clean |
| R1-F1.6: Binstall route | Enabled and disabled channel jobs, URL/version verification | verified fixed | Candidate mapping requires success when enabled and skipped when disabled. `verify_binstall.py` uses the binary-only strategy, checks installed files and exact version tokens, and checks the actual release download URL; passing tests cover both channel states | None; original clean sibling remains clean |
| R1-F1.7: Local runtime verifier | Enabled channels without live proof; pending/disabled result classification | verified fixed | `test_verify_runtime.py` checks that enabled channels without live proof remain pending and that pending/disabled checks cannot report pass | None; original clean sibling remains clean |
| R1-F1.8: Prerelease formula decision | Formula publisher, installation job, and candidate classification | verified fixed | Prereleases do not update the stable formula; tap/install jobs are skipped and the candidate records unchanged. The install helper refuses a selection that does not update the formula; candidate tests reject impossible successful-job combinations | None; original clean sibling remains clean |
| R1-F1.9: Tap no-op status distinguishes identical from newer versions | Created, updated, identical no-op, newer-version no-op, and skipped reports | verified fixed | `publish` emits `formula_current` based on the selected version, the strict report reader validates it, and workflow installation runs only when true. `FormulaCurrentTest.test_publish_reports_round_trip` covers absent/older/equal/newer formulas through publication and report reading; candidate tests preserve unchanged for an older retry | None; additional instance found and repaired during the previous repair, not a new sibling found by this review |

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec coverage and verification

The review traced the repaired installation evidence to every decision that consumes it. It also checked the surrounding release selection, immutable upload, distribution configuration, archive/runtime verification, binstall verification, and documentation against the specification. The source/package selection remains separate from the complete-chain gate, so a record of an individual release with disabled channels remains valid without authorizing later packages.

| Requirement | Verification inspected | Assessment |
|---|---|---|
| Exact approved package/tag/revision; current and tagged approvals; excluded auxiliary binaries | L1 policy, selection, configuration, and plan fixtures; retained real dist-plan evidence for the first-set packages | Appropriate decision and plan coverage |
| Tagged-source builds, native-dependency checks, notices, archive layout, and supported platform floors | Build/inspection code, L1 archive/linkage/refusal tests, and retained real artifact/runtime reports | Appropriate automated checks; outstanding platform evidence remains explicitly pending |
| Immutable retries and partial releases | L1 fake-service tests and byte comparison through public publication APIs | Conflicts refuse writes; recovery requires the original verified bundle when assets already exist |
| Shell installation | L1 subprocess/fixture tests and four-target production installation jobs | Tests exercise installation and execution; pending live evidence is not promoted to pass |
| Binstall installation and package-tag URLs | L1 public verification tests, retained pinned-tool fixture acceptance, and four-target production jobs | Binary-only installation and version/URL checks remain enforced |
| Homebrew installation and no downgrade | L1 installer/report/publication tests plus workflow contract; four-target production install matrix | Formula publication alone cannot claim installation; same-version retry can install, older retry remains unchanged |
| First bh chain before wider rollout | L1 candidate → parsed ledger → policy/preflight decision tests | Every required channel must pass; a successful canonical distribution run is also required |
| Documentation and operational pending states | `docs/distribution.md`, `docs/releases.md`, rollout ledger, and rust-devops distribution contract | Installation results, retries, approvals, and rollout gates agree with the implementation |
| Terminal behavior | No interactive input or terminal-rendering requirement introduced | L2/L3 terminal or keyboard tests are not required for these noninteractive release checks |

**Executed:** `just distribution-check` succeeded: **447 Python tests**, no skips, in **18.876 seconds**; **10 distribution Rust workflow contracts passed** through Nextest. The 209 other contracts were excluded by the recipe's focused filter. Cargo emitted a registry-cache cleanup permission warning, but compilation and all selected tests succeeded.

The Python suite runner explicitly registers its suites, rejects unlisted distribution test files, checks pinned dependencies, and fails on skipped tests. The Rust contracts are compiled by the declared `ci_workflow_contracts` target and selected by the distribution recipe. No new test was added during review.

No release, approval, asset, or tap was changed. This review did not rerun cross-OS suites or manufacture live publication evidence; neither is required to decide implementation readiness here.

## Input robustness matrix

The default for invalid configuration/report shapes is a located error before the corresponding build, upload, or installation proceeds. Existing explicit exceptions remain: absent approval defaults false; empty policy/ledger lists grant no authority; disabled channels and incomplete release records are valid but do not establish the first complete chain.

The repair adds one load-bearing report field, `formula_current`, through the existing strict JSON loader and shared boolean validator. `FormulaCurrentTest` covers true/false controls, absence, explicit null, and wrong type, then tests status-dependent consistency. Existing shared-validator tests cover the remaining scalar shapes, including empty values; collection-element shapes do not apply to this boolean. Report status, version, and formula path have control and invalid/mismatching-value checks. The CLI wiring tests verify located malformed/missing-report errors and successful report reading.

The existing shared field-validator and loader suites passed for policy, metadata, selections, plans, provenance, bundle metadata, rollout records, and service responses. They exercise absence/null distinctions, invalid collection elements, emptiness, duplicate keys, trailing content, and format syntax at their owning validators/loaders. The independent build-image inspector's matrix also passed. Inspection of the new report reader found no permissive default, silent filtering, or swallowed parse failure: its decision fields are required and validated. Ledger channel values continue through the retained parsed result to all widening consumers.

## Recurrence

No finding recurs. R1-F1's class was swept again, including the repair's new report field and installation job, and no remaining or newly introduced instance was established. No human decision is required before repair because there is no remaining repair work.
