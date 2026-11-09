---
kind: plan
total_phases: 4
created: 2026-11-08
phase: 5
agent: codex/gpt-6.1-sol
yolo: 'true'
source_files_during_phase_1:
- .github/actions/setup-release-plz/action.yml
- scripts/ci/release_path.py
- release-plz.toml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- tools/test-toolkit/Cargo.toml
docs_updated_during_phase_1:
- docs/releases.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- test-toolkit
- repo-deps
source_files_during_phase_2:
- .github/workflows/release-plz.yml
- .github/workflows/build-integrations.yml
- scripts/ci/release_path.py
- scripts/ci/integration_release.py
- release-plz.toml
- tools/test-toolkit/Cargo.toml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- tools/test-toolkit/tests/fixtures/release_path/driver.py
docs_updated_during_phase_2:
- docs/releases.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- scripts/ci/release_path.py
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- tools/test-toolkit/tests/fixtures/release_path/driver.py
docs_updated_during_phase_3:
- docs/releases.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_4:
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_4:
- docs/releases.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/rust-devops/ci-cd.md
source_code:
- .github/actions/setup-release-plz/action.yml
- .github/workflows/release-plz.yml
- .github/workflows/build-integrations.yml
- scripts/ci/release_path.py
- scripts/ci/integration_release.py
- release-plz.toml
- tools/test-toolkit/Cargo.toml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- tools/test-toolkit/tests/fixtures/release_path/driver.py
documentation:
- docs/releases.md
- .claude/skills/rust-devops/ci-cd.md
completed_phase: 4
implemented: true
---

# Release path repair implementation plan

## Summary and Definition of Done

Repair release preparation, publication, and integration release routing without changing the ordinary CI scheduler. Preparation must consume only a successful canonical `push` CI run on current `main`. Publication must consume only a merged release proposal authored by the configured App, and every package tag must resolve to the proposal's final reviewed commit. Initially, only `sniff-detection-cli` and `git-wt-cli` are eligible. Release-plz retains ownership of version calculation, changelogs, tags, and GitHub Releases.

Implementation is complete when the focused contracts and workflow lint pass, the Linux calculation succeeds with retained diagnostics and a demonstrated explanation of the historical checkout problem, the two-package restriction is proved, and current documentation describes setup and recovery. The implementation log must distinguish local evidence, hosted evidence, unavailable external prerequisites, and acceptance still pending. A failed diagnosis is a failed implementation, never permission to suppress clean-tree checks or describe the repair as complete.

The executable phases end at implementation complete, ready for review. App creation, installation, credentials provisioning, merging, publishing, and restoring workspace-wide eligibility are outside this execution's authorization. The spec's final live-release acceptance follows review and separate authorization; it is not a gate between these phases. Its evidence remains pending until actual run, proposal, tag, and Release URLs exist. No phase asks a human a question or waits for a response.

Work stays in the starting worktree and branch. Preserve the pre-existing staged and unstaged changes, especially the prerequisite's changes in `scripts/ci/affected_scope.py`, `scripts/ci/schema.py`, the producer workflows, the rust-devops skill, and `ci_workflow_contracts.rs`. Do not commit, push, create a branch, or move lifecycle directories during this implementation.

## Phase 1 — Establish the release boundary

### Necessary Rules

1. **Pin release-plz to `0.3.165` in all three jobs.** Keep the existing dry-run pin and make both production commands use the same installed binary. Install through the existing pinned cargo-binstall action and explicit `--no-confirm`; share these steps through `.github/actions/setup-release-plz/action.yml`. Install Rust using `dtolnay/rust-toolchain` with `toolchain` read from `rust-toolchain.toml`, currently `1.98.1`, rather than floating stable. Evidence: `release-plz.yml` pins only its dry run today; [the action definition](https://raw.githubusercontent.com/release-plz/action/v0.5/action.yml) exposes a version input but also configures Git identity. Direct invocation avoids that identity mutation and permits bounded partial-failure recovery.
2. **Preserve Git attributes and signing policy.** Do not edit `.gitattributes`, renormalize files, force checkouts, disable signing, or relax clean-tree assertions. `git ls-files --eol scripts/init.ps1` reports index LF and working-tree CRLF under the explicit PowerShell rule. Neither `.gitattributes` nor `scripts/init.ps1` exists at `9ba25e4636e5725c224017b870b07352684e3c30`; the failure crosses file introduction and attribute boundaries. That is evidence to capture, not proof of the cause. The chosen repair starts with tool-version alignment; runtime diagnosis below must prove it resolves the calculation rather than assuming the attribute rule is wrong.
3. **Use two small executable modules.** Put release event checks, freshness, revision checks, diagnostics, and publication orchestration in `scripts/ci/release_path.py`; put integration selection in `scripts/ci/integration_release.py`. Use Python standard-library JSON, subprocess, pathlib, and `tomllib`, available on the Ubuntu execution host. Expose importable functions returning explicit selection/skip/error results and thin command entry points. No new Rust package, dependency, permanent CI cell, or alternative test scheduler is introduced. Add these two source files to test-toolkit's existing `source-inputs` declaration, with explicit literal repository reads in its contract test. Evidence: existing executable Python fixtures are already driven from `tools/test-toolkit/tests/ci_workflow_contracts.rs`, including `python_interpreter()`.
4. **Preparation selection has one exact contract.** Require event `workflow_run`, run status `completed`, conclusion `success`, workflow name `ci`, triggering event `push`, `head_branch == main`, and both workflow execution repository and triggering head repository equal `yankeeinlondon/rusty-biscuit`. Checkout named `main`, full history, with upstream. Compare checkout HEAD to `workflow_run.head_sha`; mismatch yields a notice and successful no-op before token creation. Use the fixed concurrency group `release-plz-pr`, `cancel-in-progress: false`. Evidence: the current workflow already needs a branch for release-plz's `@{upstream}` lookup but omits push-event and source-repository checks. Preserve the `ci` conclusion contract; do not inspect its resolved plan.
5. **Publication identity is checked before minting credentials.** Require event `pull_request`, action `closed`, merged true, canonical execution/base/head repositories, base ref `main`, head ref beginning `release-plz-`, label `release`, author type `Bot`, and author login equal to repository variable `RELEASE_APP_BOT_LOGIN`. Define that additional variable as the exact App bot login, including `[bot]`; provisioning documentation must name it. Missing bot configuration is an actionable error on a plausible release proposal; unrelated events/proposals are successful no-ops. Require nonempty `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY` before token creation, never a default-token fallback. Evidence: the current publication predicate checks only merge, label, and execution repository; the spec requires configured App authorship without granting credential creation.
6. **Credential scope is repository-only.** Use `actions/create-github-app-token@v2` independently in preparation and publication, passing explicit owner, current repository name, and only contents/write and pull-requests/write. Keep automatic token revocation. Set checkout `persist-credentials: false`; supply the installation token as `GIT_TOKEN` to direct release-plz invocations and as the token for a process-scoped Git HTTPS credential helper. Do not interpolate tokens into logs, command arguments, persisted remote URLs, or tracked configuration. Use a temporary helper reading the token from its environment; remove it during cleanup. Default workflow permissions become `contents: read`; the dry run has no App credentials or private key. Evidence: [the token action](https://github.com/actions/create-github-app-token) supports installation scoping and explicit permissions; existing checkout would otherwise persist `GITHUB_TOKEN` authentication.
   - Set automated commit identity only for the release-plz subprocess in its disposable runner checkout: the configured bot login and its GitHub numeric user ID form `ID+LOGIN@users.noreply.github.com`. Resolve that public user record after identity validation. Do not edit global identity or signing settings, and do not disable signing. The direct command no longer inherits the upstream action's `git-config` step, so this explicit automation-only identity is required for proposal commits.
7. **The publication target is immutable.** Checkout the event's `pull_request.merge_commit_sha` with full history, assert exact HEAD, and verify it is a two-parent merge whose second parent equals `pull_request.head.sha`. Also verify that final head is an ancestor of the merge. Reject squash/rebase/foreign-parent topology before credentials. Then detach at the verified final proposal head before invoking release-plz; read manifests, release configuration, and changelogs from this commit. Never checkout current `main` or resolve a deleted PR branch. This is stricter than ancestry alone, which a fast-forward rebase can satisfy. Evidence: pinned upstream `command/release.rs::should_release` selects a proposal commit through associated-PR metadata and otherwise can fall back; explicitly starting at the validated proposal head removes dependence on a later main tip. Use fixed non-canceling publication concurrency `release-plz-release`.
8. **Temporary eligibility is an allowlist.** Set `[workspace].release = false`; explicitly set `release = true` for the existing `sniff-detection-cli` entry and a new `git-wt-cli` entry, with `worktree/cli/CHANGELOG.md`. Preserve all existing custom changelog paths, the four placeholder exclusions, `git_only = true`, `publish = false`, disabled semver checks, draft proposals, labels, and the tag template. Explicitly set `pr_branch_prefix = "release-plz-"` and `release_always = false`. Comment the temporary restriction and restoration after verified first-release evidence. Evidence: pinned upstream `config.rs::PackageConfig::merge` inherits workspace release settings; both CLI manifests use path-only workspace dependencies without version requirements. No declared dependency forces a companion version bump. A computed third package is an error, not an expansion of the allowlist.
9. **Bound recovery around a fixed candidate set.** Publication candidates are the two enabled packages whose manifest versions changed between merge first parent and verified proposal head; require exactly the approved two packages for this initial release. Derive complete tags from those head versions, never from current main or release titles. Before any mutation, inspect all candidate remote tags, peel annotated tags to commits, and query Releases by exact tag. Wrong-target tags, a Release without its tag, and unexpected draft Releases fail before mutation. Existing correct tag plus published Release is complete; neither exists is new work; correct tag plus missing Release is recovery work. API errors other than an explicit not-found are failures, never absence.
10. **Recovery still uses release-plz.** Pinned upstream [`release_package_if_needed`](https://raw.githubusercontent.com/release-plz/release-plz/release-plz-v0.3.165/crates/release_plz_core/src/command/release.rs) returns immediately for an existing local tag, before checking its Release. Therefore execute candidates one at a time in a fresh disposable clone at the verified proposal head. Generate an untracked temporary release-plz configuration outside that clone: workspace release disabled, `git_only`, `publish = false`, `release_always = false`, original tag format and release settings, and only the selected package enabled with its canonical changelog path. For recovery work, omit that one tag from the disposable clone's local refs and set `git_tag_enable = false`; release-plz then creates the missing Release through its own normal changelog and release implementation. The remote tag is never removed or moved. Recheck remote state immediately before each invocation and inspect all candidates after success or failure. A complete rerun performs no writes. Preserve the original failing exit status, append an inventory and exact original workflow run/merge revision to diagnostics, and require every completed tag to resolve to the verified head. No custom release-note generator or `gh release create` is allowed.
11. **Integration routing is exact and bounded.** Select one of `arcam-amp-integration`, `eversolo-integration`, or `sony-receiver-integration` only from a complete matching `{package}-v{semver}` release tag; validate the SemVer suffix, including prerelease/build identifiers. Emit a one-element matrix with the existing package directory; unrelated packages emit no build. Resolve and checkout the tag commit, assert peeled HEAD equality, retain `aarch64-unknown-linux-musl`, and upload only that package's existing archive name to the event's exact tag. Preserve optional `driver.json` behavior. A selection job uses read permissions; only the build/upload job needs contents/write. Evidence: `build-integrations.yml` currently builds every matrix entry and checks out default HEAD on every published release.
12. **Validation is focused and existing CI stays untouched.** Extend the release tests already in `ci_workflow_contracts.rs`, including replacing the stale assertion that requires the word `gitignored` with actual lockfile/clean-tree behavior. Keep the branch/upstream and dispatch-is-read-only contracts. Do not edit `ci.yml`, add `ready_for_review`, an OS matrix, or binary distribution. Evidence: its PR trigger already runs draft proposals; the spec explicitly rules out those additions.
13. **External acceptance is evidence, not assumed authorization.** The prerequisite implementation log records focused local passes but no hosted proof; check the canonical successful push run at the repaired revision and record the actual result. Planning-time public GitHub API reads identified failed release run `40848379464` at `37faaa9be815150df3243651524a98478953676b`; both run-log and job-log endpoints returned 403. Repository-variable listing required authentication, so App setup is unverified, not absent. Do not create credentials or change repository settings. All independent implementation tasks continue with missing external access recorded explicitly; live acceptance remains pending.

### Wave 1 — Baseline and calculation evidence

- [x] **Capture baseline**
  - Record initial worktree changes, prerequisite verification, release App configuration visibility, Git version, tool pin, and starting revision in `implementation-log.md` beside this plan. Never print a secret value.
  - Read the existing release contracts and shared test recipes; preserve unrelated in-progress edits. Use the sniff skill for host/repository discovery. Planning found `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL` declared; use BatchMode SSH with bounded connection timeouts for host reads.
  - Implement the shared installation action and the read-only diagnostic mode of `release_path.py` first. Its report includes source SHA, branch/upstream, release-plz and Git versions, effective attributes/config, tracked/untracked non-ignored status, index and working-tree byte characteristics for `scripts/init.ps1`, and historical tree presence at `9ba25e4636e5725c224017b870b07352684e3c30`.
  - Put scratch repositories and diagnostic artifacts outside the author's tree. Run production source commands only against those disposable copies; do not create a worktree attached to the session repository.

- [x] **Replay calculation**
  - **Single runtime spike:** replay the historical checkout sequence and the pinned `release-plz update` on Linux, using a disposable full-history clone and retained Git tracing. This is a runtime filesystem/tool interaction that the spec's unsuccessful macOS replay does not settle. Use the repair source/configuration for the final calculation, and reproduce the historical calculation from its failing source revision separately inside the same spike. Keep outputs outside the repository and copy concise evidence into the implementation log.
  - Outcomes are fixed: historical failure reproduced plus repaired calculation succeeds → retain the minimal failing sequence as a hermetic regression fixture and document the demonstrated tool/version interaction; historical failure not reproduced plus repaired calculation succeeds → retain the exact replay as a regression fixture, explicitly record the original cause as unproven, and leave the diagnosis acceptance criterion unmet; repaired calculation fails → preserve command, exit code, trace and tree states, keep the implementation unsuccessful, and do not introduce an unsupported attribute workaround. Infrastructure failure → retain its error, mark Linux evidence unavailable, and keep calculation acceptance unmet. None of these outcomes authorizes publication, guessed source fixes, or a claim that the historical cause was established.
  - Use this spike once. Later validation runs regression tests and reviews its evidence; it does not repeat live calculations to search for a preferred outcome. No performance measurements or additional OS matrices are included. No attribute change is planned, so no native-Windows attribute migration is required.
  - Capture `release-plz update` structured output and full diff/status, including new changelogs. Require exactly the approved package set and no other manifest version bump. A no-change calculation cannot prove the initial two-package proposal requirement; record it as unmet.

Checkpoint: installation and diagnostics are usable, all design rules are fixed, and the log honestly distinguishes demonstrated cause from unproven history. A non-success spike is a recorded verification failure; the remaining independent phases still execute and the final result remains unsuccessful.

## Phase 2 — Implement trusted release execution

### Wave 2 — Independent workflow surfaces

These tasks can use separate subagents, one owning each surface. Pass the full non-interactive constraint and the Phase 1 rules to both; neither waits on the other, edits the shared contract file, or runs the full suite.

- [x] **Repair release**
  - Own `release-plz.yml`, `release_path.py`, and `release-plz.toml`. Implement Rules 4–10 with job-level event guards plus executable defense-in-depth checks. Pass event data through `GITHUB_EVENT_PATH` and environment variables, never by interpolation into shell code.
  - Order preparation as checkout → event/freshness/config checks → tool setup → clean-tree assertion → App token → process-scoped Git authentication → `release-plz release-pr` → clean tracked-tree assertion. Mint the token near its use. Preserve branch upstream and full history.
  - Order publication as coarse identity guard → exact merge checkout → executable identity/topology/config checks → read-only candidate inventory → tool setup → App token → per-package immutable publication/recovery → final inventory and cleanup. Validate every candidate before publishing any candidate.
  - Run dry-run calculation with read-only permissions and no App token, reporting tracked edits and `git ls-files --others --exclude-standard` output. Retain trace, diff and report with failure-safe artifact/summary steps. Explicitly preserve the calculation's exit status; diagnostic success must not turn a failed calculation green.
  - Remove comments claiming `update` and `release-pr` leave the same diff, obsolete lockfile decisions, registry-token claims, and lifecycle-path references. `update` intentionally leaves edits; `release-pr` commits its proposal and must finish clean.
  - Handle token expiry, transport failures, release-plz failure and cleanup errors visibly; no automatic live publication retry loops. A rerun of the original merge event is the supported recovery boundary.

- [x] **Route integrations**
  - Own `build-integrations.yml` and `integration_release.py`. Implement Rule 11 with `select` and `build` jobs; skip the build before matrix expansion when selection is empty.
  - Populate the matrix through validated selector outputs, pin checkout to the resolved tag revision, verify HEAD, and keep existing cross-build/archive layout. Pass package, tag, and directory through validated environment values.
  - Keep published-release triggering and existing target only. No unrelated package build, manual dispatch, extra package release, or installer work is introduced.

Checkpoint: each mutation path has executable authorization and revision checks; the dry run is the sole dispatchable calculation path; integration selection produces zero or exactly one package. Review the combined diff before proceeding, preserving the author's unrelated changes.

## Phase 3 — Prove contracts and document operations

### Wave 3 — Independent coverage and documentation

- [x] **Prove boundaries**
  - Own `tools/test-toolkit/tests/ci_workflow_contracts.rs`, its source-input metadata, and fixtures under `tools/test-toolkit/tests/fixtures/release_path/`. Reuse `python_interpreter()` and fixture isolation rather than introducing platform-specific interpreter assumptions.
  - Invoke the production modules' importable selection/preflight functions from an in-process Python fixture driver launched once by the Rust contract test. Use one CLI control and invalid-input case per loader to prove command wiring, diagnostics and absence of launched mutations. Stub GitHub/Git/release-plz boundaries with argument-recording fakes; no external API or permanent tags in L1.
  - Cover canonical successful preparation and stale successful no-op; reject nightly/manual/PR CI runs, failed/incomplete runs, foreign head repository, wrong branch and workflow. Prove rejection happens before minting or release-plz invocation.
  - Cover valid App proposal and independent failures for author/login/type, branch, labels, base/head repository, merged/action/event fields, missing credentials and missing bot variable. Verify detached exact merge checkout, two-parent/second-parent/ancestry checks, moved main, deleted PR branch, rebase and squash rejection.
  - Exercise an actual hermetic Git merge graph: later main advancement cannot affect the captured candidate versions or tag target. Include annotated and lightweight tags, wrong-target preflight failure, complete no-op rerun, neither-object publication, tag-only recovery, unexpected Release-only state, draft Release rejection, API access failure, and failure after the first package. Assert retained exit status and inventories.
  - Run a fake release-plz against generated per-package configurations. Assert original tag/changelog/release settings, selected package only, no registry publication, local-only tag omission on recovery, `git_tag_enable = false` during recovery, remote tag unchanged, App token API/Git wiring and cleanup. Assert preparation leaves a clean proposal checkout.
  - Cover all three integration selectors, prerelease/build versions, misleading prefixes/suffixes, unrelated CLI releases, invalid versions, shell metacharacters, absent tag, and exact tag checkout/upload arguments. Require only one archive for one package.
  - Add structural contracts for permission scope, shared tool version/toolchain, token actions and cleanup, concurrency, draft labels, no new PR triggers, source-input declarations, failure-safe diagnostic uploads, and first-release allowlist. Explicitly read fixture/module paths in a form the repository input scanner recognizes.
  - Preserve the Phase 1 historical replay regression and the positive unchanged control fixture. Do not substitute substring assertions for the executable authorization and routing cases.

- [x] **Document operations**
  - Own `docs/releases.md` and the release-contract section of `.claude/skills/rust-devops/ci-cd.md`. Merge into the prerequisite's existing skill edits surgically.
  - Explain App creation/installation by the repository owner, least privileges, repository scope, `RELEASE_APP_ID`, `RELEASE_APP_PRIVATE_KEY`, and exact `RELEASE_APP_BOT_LOGIN`. Keep PR-creation settings and branch protections intact. Document actionable missing-configuration errors.
  - Explain automatic CI on draft proposals, the human draft-to-ready review action, required `ci-gate`, merge-commit-only release merges, and reviewed-head tags versus merge SHA and later main. Include a compact Mermaid flow from successful main push CI through draft proposal to immutable publication and package-specific integration selection.
  - Document the two-package temporary restriction and the separate later restoration, diagnostic artifacts and exit statuses, partial-failure inventory, wrong-tag refusal, and rerunning the original publication run tied to its merge SHA. A calculation dispatch never publishes.
  - State general binary distribution is planned. Current docs must contain no dated-spec references or links. Keep `docs/releases.md` as the canonical operational guide, with the skill summarizing the same contract.

### Input Robustness Matrix

The new modules read GitHub event JSON and publication configuration. Default: invalid document or load-bearing field shapes produce a located error, run no token/release/build mutation, and publish nothing. A supported but irrelevant event is a deliberate successful no-op. Load the whole relevant event/configuration before any mutation; do not silently filter malformed elements.

- Use a single strict JSON loader for event files and API responses, rejecting duplicate object keys, trailing garbage, invalid JSON, non-object event roots, and invalid constant tokens. API not-found is represented by HTTP status, never a malformed/empty response body. Test document shapes once through that loader and test each caller's wiring with a positive and one malformed input.
- Share validators for nonempty strings, repository full names, full 40-digit SHA strings, exact booleans, object fields, and lists of label objects. For each validator exercise absent, explicit null, wrong whole type, and empty values; for collections also exercise one bad element and every element bad. Errors name paths such as `pull_request.head.repo.full_name` or `workflow_run.event`. Test each field's wiring with a control and one invalid shape rather than repeating the full shared matrix.
- Exceptions: absent configuration credentials/bot login produce named setup errors only on an otherwise selected release path; unrelated events never require credentials. An empty label list is a valid unrelated proposal; a missing or null list on an otherwise relevant closed PR is invalid. Empty preparation update results are a valid no-op but do not prove the initial-release acceptance. Unknown extra GitHub fields are ignored; required selectors are strictly validated. Null repository on a deleted/foreign PR fails selection safely without dereferencing or creating credentials.
- Configuration is read with one `tomllib` loader, tested once for duplicate keys, malformed syntax/trailing content, and TOML's legitimate table/string syntax. Require correctly typed workspace/package release booleans, package names, tag template, changelog paths, Git-only/publish settings, branch prefix and release settings used by orchestration. Preserve absent optional settings through explicitly documented release-plz defaults; null has no TOML representation. `package = []` means no package overrides, which fails the required initial allowlist invariant; no missing-list default may silently enable the workspace.
- API ref objects, peeled annotated-tag objects, Release objects and release-plz update results use the same field validators. A 404 is the sole absence exception; an empty list is legitimate only for a list endpoint. Reject unexpected duplicate candidate packages/tags and invalid versions before mutation.
- Build mutation cases by making one edit to a positive real-shaped event/API/config fixture. Use public selection/preflight results and the command adapter's recorded calls, not private parsing output. Because these modules are scripts rather than crates, the importable module API is their public in-process boundary; Rust's existing contract suite owns execution.
- Before completing coverage, search changed modules for defaults on load-bearing fields, `.get(..., [])`, `or []`, swallowed exceptions, and filtered type coercions; inspect Rust additions for `#[serde(default)]`, `Option<T>` conflating null/absent, `filter_map`, `unwrap_or_default()` and `.ok()`. Every finding is fixed or justified against an explicit exception above. A review finding in one matrix cell requires recording all remaining shapes for that validator/loader, not just patching the example.

Checkpoint: executable positive/negative cases establish trust and revision behavior, malformed input fails closed, documentation matches the implemented workflow, and no test starts real publication or steals terminal/browser focus.

## Phase 4 — Validate and close implementation

### Wave 4 — Final verification

- [x] **Run focused checks**
  - Run `just test test-toolkit` from the root once after integrating Wave 3. Use the existing Nextest path; inspect actual test counts and failures. During development, use `cargo nextest run -p test-toolkit --test ci_workflow_contracts <test-name-filter>` for the changed slice only.
  - Run `cargo clippy -p test-toolkit --all-targets -- -D warnings`. Do not run `cargo fmt` or a recipe that invokes it. Run `actionlint .github/workflows/release-plz.yml .github/workflows/build-integrations.yml`, plus an explicit contract validating the referenced local composite action's inputs and installation commands. Use actionlint `1.7.12`; install its matching host archive into a temporary tools directory and verify it against the published `actionlint_1.7.12_checksums.txt`. Do not change repository dependencies for that tool.
  - Validate generated recovery TOML through the strict TOML loader and the fake-command contracts. Keep a focused pinned-upstream compatibility contract for `release_package_if_needed` and `release_package_git_only`, with the relevant source revision and behavior recorded in the log: no local tag reaches release handling, and disabled tag creation still permits Release creation. Do not mistake a fake-command test for live forge proof; the tag-only recovery path's actual GitHub operation remains part of separately authorized live acceptance. Keep all live credentials out of these tests.
  - Exercise the hosted **Release Dry Run** only from an already available repair-branch revision containing these changes; this plan does not authorize a push. Existing unavailable branch/dispatch permissions are an external verification failure recorded without waiting. The Phase 1 Linux replay remains the one calculation spike; a hosted execution here verifies workflow wiring, not another diagnosis search.
  - Check the prerequisite's actual canonical successful main push CI result and accessible App metadata/secret-name inventory read-only. Record unknown/unavailable states precisely. Do not claim a local prerequisite pass proves main is green or that a named secret proves a working installation.

- [x] **Record completion**
  - Populate `implementation-log.md` with changed files, rule departures, commands/results, Linux diagnosis evidence, first-release package/version proposal, regression mapping, prerequisite status, and App setup visibility. Include diagnostics paths/run URLs and remaining unmet acceptance criteria. Never rewrite the historical spec to match an implementation departure.
  - Record live-release acceptance as pending: proposal URL and final head, passing `ci-gate`, merge SHA, publication run, both complete tags and peeled targets, both Release URLs, and delivered published-release workflow events. The approved set contains no integration; focused integration tests are its required coverage, and no extra release is scheduled to exercise that consumer.
  - Inspect the final diff for accidental edits, secret exposure, stale comments, dated-spec links in current docs, new CI cells, identity/signing changes, and unintended eligibility. Keep the restriction in place; its restoration is a separate follow-up change after actual live evidence.
  - Mark tasks complete only with their stated evidence. End implementation complete, ready for review only with successful diagnosis and focused verification. Unproven historical cause, missing Linux proof, failed tests, unavailable workflow lint, or failed calculation yield an unsuccessful result naming the unmet criterion; external App/live-release evidence may remain explicitly pending as the spec permits. No interactive gate or request is inserted into execution.