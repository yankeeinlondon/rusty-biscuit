---
plan: fixes/2026-08-07-retire-specialized-workflows/plan.md
inspected_revision: dd2061b43bd3ee7c6acea3000f4806412eacdffb
---

# Specialized workflow retirement implementation notes

## Phase 1 prerequisite audit

Revision `dd2061b43bd3ee7c6acea3000f4806412eacdffb` contains the package-keyed
cutover required by `fixes/2026-08-06-cicd/spec.md`:

- `scripts/ci/affected_scope.py` emits selected package names, resolved package
  policy, and one matrix record per gating package.
- `.github/workflows/ci.yml` fans each matrix record into one invocation of
  `_package-ci.yml`. Its result-producing jobs receive one `package` input.
- `_package-ci.yml` delegates the package's WSL2 L1 cell to `_wsl-ci.yml` with
  the same package, check arguments, test arguments, native prerequisites, and
  slow-test policy used by the native flow.
- Native and WSL2 L1 jobs upload package/environment/tier-keyed JUnit staging
  trees and producer-status artifacts. `ci-verdict` downloads all artifacts
  and invokes `ci-rollup` with the selected package scope, resolved policy, and
  environment table. It does not infer cell identity from job display names.

This audit found no incomplete prerequisite, so Phase 1 may proceed.

## Clean-revision coverage inventory

The listings below were generated from a temporary `git archive` of the
inspected revision with Cargo nextest 0.9.136. The host's pinned Rust toolchain
was missing its `rustc` component, so the already-installed stable toolchain
was selected explicitly for these read-only listings.

| Retired command | Clean-revision targets | Replacement package cell |
|---|---|---|
| `cargo check -p messenger --all-features` | Messenger library plus all six feature-gated helper bins; Messenger has no example or bench target | `messenger` check on Windows uses `--all-targets --all-features`; native and WSL2 `messenger` L1 builds cover the all-feature library and test targets |
| `cargo build -p messenger --features desktop --bins` | `stub_dunstify`, `stub_notify_send`, `stub_snoretoast`, `stub_burnttoast`, `stub_terminal_notifier`, and `stub_alerter` | `messenger`'s `messenger-desktop-stubs` runner tool prebuilds the six bins for native L1 and ships their Linux sidecar to the WSL2 L1 cell |
| `cargo test -p messenger --features desktop --lib` | 396 library tests | Full all-feature `messenger` L1 on Ubuntu, Windows, macOS, and WSL2 |
| `cargo test -p messenger-cli` | 106 tests | Full `messenger-cli` L1 on Ubuntu, Windows, macOS, and WSL2 |
| `cd claudine/rendezvous && just test` | 273 tests: 82 `rendezvous-core`, 21 `rendezvous-client`, and 170 `rendezvous-daemon` | The three package-keyed L1 cells on Ubuntu, Windows, macOS, and WSL2 |
| `cargo nextest run --color=never -p claudine-cli -E 'test(/dashboard/) + test(/session_report/) + test(/requeue/) + test(/commands::handle/)'` | 50 tests: 28 dashboard, 8 session-report, 4 requeue, and 10 command-handler | Full `claudine-cli` L1 on Ubuntu, Windows, macOS, and WSL2 |

The normal grid intentionally does not preserve the retired workflow's
macOS/Linux `cargo check --all-targets` coverage for non-test examples. The
only presently affected extra target is
`claudine/rendezvous/daemon/examples/register_compaction_spike.rs`, a
development spike. Windows retains the all-target check; native and WSL2 L1
continue to compile and execute package test targets.

## Phase 1 behavior-to-test map

| Changed contract | Targeted regression test | Expected pre-retirement result |
|---|---|---|
| Messenger resolves as gating, all-feature policy with `libdbus-1-dev` and `messenger-desktop-stubs`, and produces identical feature arguments for check, native L1, and WSL2 archive construction | `RealWorkspaceRetirementScopeTests.test_messenger_policy_and_matrix_contract_are_promoted` | Fails on `gates = false`, desktop-only features, and the absent runner tool |
| A Messenger CLI source change reaches the ordinary package matrix without a prefix flag | `RealWorkspaceRetirementScopeTests.test_messenger_cli_change_selects_its_normal_package_cell` | Fails because `messenger-cli` is selected but excluded from the matrix |
| A Sniff library change produces the exact Cargo-derived downstream package set, including all Rendezvous packages and Claudine call sites | `RealWorkspaceRetirementScopeTests.test_sniff_change_selects_exact_reverse_dependency_closure` | Passes and pins the prerequisite closure before graph deletion |
| Retired workflow files, orchestration jobs, and specialized inventory entries are absent | `retired_specialized_workflows_and_jobs_are_absent` and `specialized_inventory_contains_only_surviving_workflows` | Fail on the two live files/jobs and four-entry inventory |
| Messenger's closed runner tool is implemented in both native and WSL2 execution paths | `messenger_stub_runner_tool_reaches_native_and_wsl2_execution` | Fails because neither reusable workflow implements the tool |
| Messenger and Rendezvous failures reach `ci-verdict` as package-keyed JUnit and producer-status evidence, not specialized `needs` entries | `retired_packages_reach_verdict_through_package_evidence` | Fails because Messenger remains non-gating and both specialized jobs remain in `ci-verdict.needs` |

These are passive shipped-artifact tests plus real-workspace end-to-end scope
calculations through the normal `calculate_scope` path. This phase changes no
parser, schema, template, prompt, or persisted value, so corpus and repeated
read/write/read persistence tests are not applicable.

## Validation checkpoint 1

`RUSTUP_TOOLCHAIN=stable python3 scripts/ci/test_affected_scope.py` ran 56
tests. Fifty-four passed. The two expected failures were:

- `test_messenger_policy_and_matrix_contract_are_promoted`: reported the
  pre-retirement gating, all-feature, runner-tool, and ordinary matrix-cell
  contracts as missing. The retained D-Bus prerequisite was not reported
  missing.
- `test_messenger_cli_change_selects_its_normal_package_cell`: proved the exact
  source input selects `messenger-cli` in package scope but not yet in the
  result-producing matrix.

`RUSTUP_TOOLCHAIN=stable cargo nextest run -p test-toolkit --test
ci_workflow_contracts` compiled the shipped-artifact contract binary and failed
on the first retirement contract under nextest's default fail-fast behavior. A
targeted `--no-fail-fast` run then executed all four new retirement contracts:

- `retired_specialized_workflows_and_jobs_are_absent` reported both workflow
  files and both `ci.yml` jobs.
- `specialized_inventory_contains_only_surviving_workflows` reported the two
  obsolete entries alongside the two intended survivors.
- `messenger_stub_runner_tool_reaches_native_and_wsl2_execution` reported the
  missing native implementation, WSL2 sidecar path, and reusable-workflow
  forwarding.
- `retired_packages_reach_verdict_through_package_evidence` reported both
  non-gating Messenger manifests and both specialized `ci-verdict.needs`
  entries.

An inverse-filter nextest run executed the remaining 54 workflow-contract
tests: all 54 passed. The expected red contracts therefore fail on the missing
retirement behavior, not on compilation, fixture setup, unrelated workflow
contracts, or Cargo metadata discovery.

The package-area gates produced the same result at broader scope:

- `cd tools && RUSTUP_TOOLCHAIN=stable just build`: passed.
- `cd tools && RUSTUP_TOOLCHAIN=stable just lint`: passed, including Clippy and
  the read-only formatting check.
- `cd tools && RUSTUP_TOOLCHAIN=stable just test`: reached the intentional
  Messenger runner-tool contract failure; 95 tests passed before default
  fail-fast canceled the remainder.
- An inverse-filter full-package nextest run executed all 120 unaffected L1
  tests across five binaries; all 120 passed.
- `git diff --check`: passed.

The canonical suite is intentionally red at the end of Phase 1 because the
phase freezes contracts that Phases 2 and 3 implement. Making these tests green
in Phase 1 would require performing later-phase policy, runner-tool, workflow
deletion, and verdict changes out of order.

## Phase 2 behavior-to-test map

| Changed behavior | Targeted test |
|---|---|
| Messenger is gating with all features, retains the Ubuntu D-Bus prerequisite, owns `messenger-desktop-stubs`, and forwards the same feature contract to check, native L1, and WSL2 | `RealWorkspaceRetirementScopeTests.test_messenger_policy_and_matrix_contract_are_promoted` |
| The new runner tool is accepted by the closed vocabulary but does not propagate from a dependency to its consumer | `PackagePolicyTests.test_messenger_desktop_stubs_runner_tool_is_accepted` and `NonPropagationTests.test_a_dependent_keeps_its_own_tiers_tools_and_companions` |
| An explicit stub directory wins over the target directory and resolves all six shipped fixtures | `explicit_directory_takes_precedence_and_resolves_all_six_stubs` |
| Windows fixture lookup appends `.exe` | `windows_stub_names_use_the_executable_suffix` |
| A missing fixture in an explicit directory reports the variable, binary, and path without falling back | `missing_explicit_fixture_is_an_authoritative_error` |
| An unset explicit directory and missing target fixture remains eligible for local build-on-demand without starting Cargo in the test | `missing_target_fixture_remains_eligible_for_local_build_fallback` |
| Native CI prebuilds and verifies six stubs once before L1, exports their directory, and WSL2 stages the Linux sidecar with executable permissions and unprivileged ownership | `messenger_stub_runner_tool_reaches_native_and_wsl2_execution` |
| Messenger WSL2 L1 verifies the unprivileged guest has neither Cargo nor rustc before executing the archive | `messenger_stub_runner_tool_reaches_native_and_wsl2_execution` together with `wsl_is_an_environment_and_never_a_runner_label` |

The scope tests exercise the checked-in manifests through the normal
`cargo metadata` and `calculate_scope` path. The workflow contracts passively
inspect the shipped reusable workflows. Together they provide the required
corpus and end-to-end coverage for configuration-driven behavior. Phase 2 adds
no persisted values, so a read/write/read round trip is not applicable.

## Validation checkpoint 2

- The pre-implementation policy tests failed on the unknown runner tool,
  non-gating manifests, desktop-only feature policy, and absent ordinary
  Messenger matrix cell. The workflow contract failed on both missing delivery
  paths. These were the intended regression failures.
- The four resolver tests passed under `cargo nextest`, including all six
  explicit fixtures and the original missing-explicit-directory case.
- A local reproduction built the six stubs once with Messenger's all-feature
  contract, verified each executable, exported `MESSENGER_STUB_BIN_DIR`, and
  ran `cd messenger && just test --all-features`: Messenger passed 453/453
  tests (2 skipped) and Messenger CLI passed 106/106.
- `cd messenger && just lint` passed for both packages. The stricter
  `cargo clippy -p messenger --all-features --all-targets -- -D warnings` also
  passed.
- `python3 scripts/ci/test_affected_scope.py` passed 57/57 tests. The focused
  workflow suite passed the Messenger sidecar contract plus the existing WSL
  environment and slow-test forwarding contracts, 3/3.
- `cargo clippy -p test-toolkit --test ci_workflow_contracts -- -D warnings`
  passed. `actionlint` remained nonzero on the same six pre-existing SC2086
  informational findings present at `HEAD`; it added no finding in a changed
  shell step.
- `git diff --check` passed.

This macOS validation proves local fixture resolution and the static
cross-platform workflow contracts. It is not Windows, Linux, or WSL2 runtime
evidence; those remain Phase 6 promotion-run obligations.

## Phase 4 command-to-cell evidence

The exact L1 environment set below is
`{ubuntu-latest, windows-latest, macos-latest, wsl2-ubuntu}`. Expanding that
set, rather than treating “cross-platform” as one result, gives the following
replacement ownership:

| Retired command | Exact replacement producers |
|---|---|
| `cargo check -p messenger --all-features` | `messenger/windows-latest/check` compiles all targets with all features. The all-feature library and test targets are also built by `messenger/ubuntu-latest/L1`, `messenger/windows-latest/L1`, `messenger/macos-latest/L1`, and `messenger/wsl2-ubuntu/L1`; those L1 cells do not claim all-target compilation. |
| `cargo build -p messenger --features desktop --bins` | Fixture setup for `messenger/ubuntu-latest/L1`, `messenger/windows-latest/L1`, and `messenger/macos-latest/L1` uses one native prebuild; `messenger/wsl2-ubuntu/L1` uses the Linux sidecar. This command contributes fixtures, not a separate result cell. |
| `cargo test -p messenger --features desktop --lib` | `messenger/ubuntu-latest/L1`, `messenger/windows-latest/L1`, `messenger/macos-latest/L1`, and `messenger/wsl2-ubuntu/L1`. The clean-revision listing contained 396 library tests; the replacement runs the strict all-feature superset. |
| `cargo test -p messenger-cli` | `messenger-cli/ubuntu-latest/L1`, `messenger-cli/windows-latest/L1`, `messenger-cli/macos-latest/L1`, and `messenger-cli/wsl2-ubuntu/L1`. The clean-revision listing contained 106 tests. |
| `cd claudine/rendezvous && just test` | Four L1 cells apiece for `rendezvous-core`, `rendezvous-client`, and `rendezvous-daemon`, one for every environment in the set above. The clean-revision listings contained 82, 21, and 170 tests respectively (273 total). |
| `cargo nextest run --color=never -p claudine-cli -E 'test(/dashboard/) + test(/session_report/) + test(/requeue/) + test(/commands::handle/)'` | `claudine-cli/ubuntu-latest/L1`, `claudine-cli/windows-latest/L1`, `claudine-cli/macos-latest/L1`, and `claudine-cli/wsl2-ubuntu/L1`. The clean-revision filter selected 50 tests: 28 dashboard, 8 session-report, 4 requeue, and 10 command-handler tests; full L1 is the replacement superset. |

The retired macOS/Linux compile step had one target not covered by those L1
producers: the non-test
`claudine/rendezvous/daemon/examples/register_compaction_spike.rs` development
spike. It still receives the ordinary Windows all-target check, but it no
longer receives macOS/Linux compile-only coverage. No equivalent all-target
claim is made for macOS, Linux, or WSL2.

Native fixture proof is the single `Build messenger desktop stubs` step in
`_package-ci.yml`: it builds all six named binaries in one Cargo invocation,
checks each platform-correct executable, exports `MESSENGER_STUB_BIN_DIR`
through `GITHUB_ENV`, and precedes the fixture-build-free L1 step. WSL2 fixture proof is
the `_wsl-ci.yml` archive-side Linux build and sidecar artifact followed by the
guest-side ext4 copy, executable mode, unprivileged ownership, explicit
directory export, six-file verification, and Cargo/rustc absence check. The
`messenger_stub_runner_tool_reaches_native_and_wsl2_execution` shipped-workflow
contract enforces these details.

## Phase 4 authority-to-contract review

| Active statement | Executable authority |
|---|---|
| `messenger-desktop-stubs` is closed vocabulary and Messenger selects all features, its D-Bus prerequisite, and the same feature arguments in each flow | `PackagePolicyTests.test_messenger_desktop_stubs_runner_tool_is_accepted` and `RealWorkspaceRetirementScopeTests.test_messenger_policy_and_matrix_contract_are_promoted` |
| Native Messenger fixtures are prebuilt once; WSL2 receives a six-binary sidecar in a toolchain-free guest | `messenger_stub_runner_tool_reaches_native_and_wsl2_execution` |
| Messenger and Rendezvous use package-keyed native and WSL2 L1 evidence consumed by `ci-verdict` | `retired_packages_reach_verdict_through_package_evidence` plus the generic JUnit and producer-status contracts in `ci_workflow_contracts` |
| Only the captured-stdout Biscuit TUI and Playa Windows workflows remain in the specialized inventory | `specialized_inventory_contains_only_surviving_workflows` and `specialized_contracts_are_reusable_and_orchestrated_by_primary_ci` |
| Active docs contain no retired workflow ownership and describe the shipped fixture/result path | `active_ci_authority_matches_the_retirement_contract` |
| Rendezvous runtime evidence is owned by its three gating package manifests on all declared environments | `retired_packages_reach_verdict_through_package_evidence`, environment capability validation, and the generic per-package L1 workflow contracts |
| `register_compaction_spike` retains only the Windows all-target compile guarantee | the Windows `cargo check --all-targets` workflow contract, ordinary L1 target selection, and absence of the retired workflows |

`active_ci_authority_matches_the_retirement_contract` is the Phase 4 passive
corpus test over every shipped authority document. Before the documentation
changes it failed on the absent explicit fixture path/toolchain-free sidecar
contract. It uses the real checked-in files and the same workflow-contract
binary used by normal package validation. Phase 4 changes no persisted values,
so repeated read/write/read testing is not applicable.

## Validation checkpoint 4

- The focused `active_ci_authority_matches_the_retirement_contract` regression
  passed after failing before the documentation changes for the intended
  missing fixture-delivery contract.
- `cd tools && just test` passed 125/125 nextest tests across five binaries;
  the pre-declared ignored tests `cargo_nextest_flags_slow_test_in_output` and
  `slow_fixture_for_nextest_verification` were skipped. This includes all 59
  shipped CI workflow contracts and the passive documentation corpus.
- `cd tools && just lint` passed for `test-toolkit`.
- `python3 scripts/ci/test_affected_scope.py` passed 57/57 tests, including the
  real-workspace Messenger feature/tool policy and exact reverse-dependency
  scenarios cited by the active docs.
- The active-file audit found the retired workflow filenames only in intentional
  negative contract assertions. It found no live deleted-workflow ownership,
  Messenger desktop-only policy, retired SID transformation, specialized cache
  key, or obsolete Messenger prefix-selection claim.
- Every active Phase 4 statement about environment coverage, feature selection,
  fixture delivery, result identity, specialized inventory, and the intentional
  example-only compile reduction is paired with an executable authority in the
  table above.

## Validation checkpoint 5

The deterministic macOS validation passed:

- `python3 scripts/ci/test_affected_scope.py`: 57/57 tests. The
  messenger-cli-only fixture asserts exactly `['messenger-cli']` for both scope
  and matrix output. The Sniff fixture asserts the complete ordered 30-package
  Cargo-derived reverse-dependency closure.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 59/59 tests,
  including the shipped workflow corpus and package-evidence retirement
  contracts.
- `cd messenger && just test`: 425/425 Messenger tests with 2 skipped, followed
  by 106/106 Messenger CLI tests. `cd messenger && just lint` passed for both
  packages.
- The promoted Messenger path used one Cargo invocation to prebuild all six
  helper executables, verified all six before nextest, exported the explicit
  `MESSENGER_STUB_BIN_DIR`, and passed 453/453 all-feature Messenger tests with
  2 skipped. `cargo clippy -p messenger --all-features --all-targets -- -D
  warnings` also passed. Because the explicit directory is authoritative and
  all helper tests passed, the local build-on-demand branch was not reached.
- `cd claudine/rendezvous && just test`: 82/82 core, 168/168 daemon with 2
  skipped, and 21/21 client tests. `just lint` passed for all three packages.
- `cd claudine && just _test claudine-cli`: 2370/2370 L1 tests with 245 skipped,
  covering dashboard, session-report, requeue, and command-handler call sites
  as a strict superset of the retired filter. The macOS linker emitted one
  non-fatal compact-unwind size warning while building the test binary.
- `actionlint` and `yq` parsed all three changed workflows. Actionlint passed
  after excluding only the six pre-existing SC2086 findings for intentional
  native-package argument splitting; no changed shell step introduced a
  finding. The active-file retirement scan, negative-contract presence check,
  workflow/manifest/historical/local-recipe scope checks, and `git diff
  --check` all passed.

The portability review found no static defect. Native fixture setup uses Bash
arrays on every runner; the Windows branch uses Git Bash `pwd -W` and the
`.exe` suffix; paths are quoted; the WSL sidecar is copied to ext4 with mode
`0755` and unprivileged ownership; and artifact names use stable package and
environment tokens. No PowerShell step changed.

The following are Phase 6 promotion-run observation items rather than assumed
passes: Windows Git Bash executable detection for all six `.exe` files, the
Linux-target sidecar build, and Windows artifact download followed by WSL2
ext4 copy/execution in the toolchain-free `biscuit` account.

## Phase 6 behavior-to-test map

Phase 6 changes no implementation behavior. It promotes the already-tested
retirement diff into GitHub Actions and requires runtime evidence from each
newly owned environment. The local preflight maps each promotion claim to the
test that must remain green before a remote run is meaningful:

| Promotion claim | Local regression evidence | Required remote evidence |
|---|---|---|
| A messenger-cli-only change selects its ordinary package cell without a prefix flag | `RealWorkspaceRetirementScopeTests.test_messenger_cli_change_selects_its_normal_package_cell` | The retirement PR's scope summary and `messenger-cli` package jobs |
| The original `sniff/lib/src/lib.rs` input selects the exact Cargo-derived Rendezvous/Claudine closure | `RealWorkspaceRetirementScopeTests.test_sniff_change_selects_exact_reverse_dependency_closure` | The retirement PR's scope summary and scheduled closure |
| Native Messenger builds all six fixtures once before nextest; WSL2 receives the six-file sidecar and runs without Cargo or rustc | `messenger_stub_runner_tool_reaches_native_and_wsl2_execution` | Native and WSL2 setup/test logs plus uploaded artifacts |
| Messenger and Rendezvous failures are package-keyed and block the verdict unless an exact baseline applies | `retired_packages_reach_verdict_through_package_evidence` and the generic producer-status/JUnit contracts | A controlled or observed failing promotion cell and `ci-verdict` result |
| Only the two out-of-scope specialized workflows remain | `specialized_inventory_contains_only_surviving_workflows` and `retired_specialized_workflows_and_jobs_are_absent` | The workflow graph for the retirement PR |

These tests use the exact shipped manifests, workflows, documentation, and
normal scope-calculation path. Phase 6 changes no parser, schema, template,
prompt, configuration format, or persisted value, so it needs no new corpus or
read/write/read round-trip test. No targeted test was added in Phase 6 because
the Phase 1 regression tests already cover every locally observable contract;
the new evidence required by this phase is runner and artifact evidence.

## Phase 6 promotion preflight

Local preflight on 2026-08-12 passed:

- `python3 scripts/ci/test_affected_scope.py`: 57/57 tests.
- `cargo nextest run --color=never -p test-toolkit --test
  ci_workflow_contracts`: 59/59 tests.
- `cd tools && just test`: 125/125 tests, with the two pre-declared slow tests
  skipped.
- `cd tools && just lint`: passed with no warnings.
- `cd messenger && just test`: Messenger passed 425/425 tests with two
  pre-declared skips; Messenger CLI passed 106/106 tests.
- `cd messenger && just lint`: passed for both packages.
- `cd claudine/rendezvous && just test`: core passed 82/82, daemon passed
  168/168 with two pre-declared skips, and client passed 21/21.
- `cd claudine/rendezvous && just lint`: passed for all three packages.
- `cd claudine && just test-cli`: 2370/2370 tests with 245 pre-declared skips.
  The macOS linker repeated the non-fatal compact-unwind size warning recorded
  in Validation checkpoint 5.
- `cd claudine && just lint`: passed for catalog types, library, contract
  adapter, CLI, and generator packages, including the 18/18 error-guard tests.
- `git diff --check`: passed.

No Phase 6 source, baseline, or skill change was required by these results.

The promotion run cannot be started from this checkout. The retirement changes
are uncommitted on local branch `main`, the task explicitly forbids staging,
committing, or pushing them, and GitHub CLI has no authenticated session. The
public repository has no open pull request. Its latest observed `main` revision,
`5232fbcd143f5f590ccae9fa6bec37c0627e8b37`, still contains both specialized
workflow files; CI run `31547306246` therefore cannot be evidence for this
diff. There are consequently no honest promotion run IDs, package/environment
artifacts, first-run failures, baseline candidates, or final verdict to record.

Tasks 6.1 through 6.6 and Validation checkpoint 6 remain unchecked. Completing
them requires an authorized process to commit and push the atomic retirement,
open its PR, and inspect that PR's native and WSL2 jobs and artifacts. Marking
any of those tasks complete from static macOS evidence would contradict their
explicit promotion-run requirements.
