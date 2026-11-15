---
implementation_1: "2026-11-08T19:43:46-08:00"
implementation_2: "2026-11-08T20:34:38-08:00"
---

# Release path repair: review/fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-08T19:43:46-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/fixes/2026-11-06-release-path-repair/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter: one finding (`R1-F1`, high), no `human_in_the_loop`, no blocked findings, `implemented: false` (already false; left false until the cycle closes)
- local OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` are all unset in this session
        - the review accepted the reachable `build-linux` SSH alias (`BatchMode=yes`) as the Linux evidence host; it is reused for the bounded Linux calculation

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Release calculation remains unable to produce the approved proposal | blocked | Pin, environment, regression, publication re-check and docs are fixed; the real Linux calculation now stops on a newly found cause (R1-F1.7) whose only repairs touch immutable tags or the tag format, which the spec forbids; needs an author ruling |
| R1-F1.1 | Release Dry Run + shared setup: successful Linux calculation showing exactly the approved two-package version/changelog changes | blocked | Pin 0.3.165 → 0.4.3 and the dry run now runs `release_path.py update`; real `build-linux` run `r1-update-0.4.3` exit 1: `cannot find package "git-wt-cli" in workspace` (R1-F1.7). Waits on the R1-F1.7 ruling |
| R1-F1.2 | Create Release PR process environment: calculation completes without suppressing clean-tree checks or changing user checkouts (same calculation environment as dry run) | fixed | `_command_release_pr` runs inside `calculation_environment` (temporary `core.attributesFile` outside the checkout, appended after inherited `GIT_CONFIG_*`, before the credential helper); clean-tree checks unchanged; contract `the_release_calculation_is_provable_from_a_branch_without_releasing` forbids bare `release-plz` in dry-run/preparation jobs; driver cases cover appending to a pre-existing count. A live success shares the R1-F1.1 blocker |
| R1-F1.3 | Historical-checkout regression: keep the failure reproduction and add a focused success case using the actual repair | fixed | kept `release_history_walk_back_across_the_crlf_rule_aborts_on_a_stale_stat_cache` (0.81 s); added `release_history_walk_back_succeeds_in_the_release_calculation_environment` (0.39 s), which uses the production `calculation_env` and checks the rule matches `.gitattributes` line 14 |
| R1-F1.4 | Publish Release: exact-head publication and tag-only recovery valid under the replacement version (source re-inspection, pin guard, docs) | fixed | 0.4.3 source (`/tmp/rpz-upstream-r1/0.4.3`): local-only `tag_exists` early return (release.rs l.660, git_cmd lib.rs l.529), independent tag/Release switches (release.rs l.974–1078, config.rs l.349/387), merge-commit tag target (`should_release` l.749–789), unchanged `release-pr --output json`; pin guards name 0.4.3; publication deliberately outside the attributes environment (it checks out only the final proposal commit, which has `.gitattributes`); docs and skill updated |
| R1-F1.5 | Calculation report and proposal candidate checks retained; actual successful initial calculation recorded as acceptance evidence | blocked | checks retained (focused selection 35 passed); the acceptance evidence waits on the R1-F1.1/R1-F1.7 ruling. Diagnostic-only re-tag run (disposable clone): exit 0, exactly `git-wt-cli` and `sniff-detection-cli` 0.1.0→0.1.1, `Cargo.lock`, two `Cargo.toml`s, `sniff/cli/CHANGELOG.md`, new `worktree/cli/CHANGELOG.md`, no dependency bump — not acceptance evidence |
| R1-F1.6 | Sibling (sweep): `calculation_report` stripped the first porcelain status line's index column | fixed | `_git_lines` helper; new driver case `calculation_report_lists_the_approved_bumps_and_every_change` fails when the old stripping is restored |
| R1-F1.7 | Discovered cause: `sniff-detection-cli-v0.1.0`, `git-wt-cli-v0.1.0`, `darkmatter-lsp-v0.1.0` point at `907cd6e`, where only the pre-rename names (`sniff-cli`, `worktree-cli`, `dmls`) exist; 0.4.3 looks up every `git_only` package at its newest tag, eligible or not | blocked | verified independently by the orchestrator: `git rev-parse <tag>^{commit}` → `907cd6e45` for all three; `git show 907cd6e:worktree/cli/Cargo.toml` → `worktree-cli`; `next_ver.rs` partitions all `workspace_packages()` by `should_use_git_only`, not by eligibility. Every repair conflicts with a spec or review prohibition (moving published tags, changing the `{package}-v{version}` format, release-plz tag ownership) or is unreachable (upstream change), so it needs an author ruling |
| R1-F1.8 | Sibling sweep: every release-plz call or environment builder | clean | dry-run step → `release_path.py update` (fixed under F1.1); `_command_release_pr` (F1.2); `_release_one` publish/recovery checked, unchanged by design (F1.4); `credential_env` moved onto shared `append_git_config` (driver case unchanged and passing); `diagnose` reads only, unaffected |

### R1-F1: Release calculation remains unable to produce the approved proposal

- starting the work on 'R1-F1: release calculation' at 19:50:10
        - defect class: a release-tool pin and its process environment are chosen without proving that the pinned tool can complete the calculation on this repository's real history, so every job sharing the pin inherits the same unrunnable calculation
        - delegated to one foreground subagent (fork); it reports log items back to the orchestrator
        - pin moved from 0.3.165 to **0.4.3** (newest stable, 2026-11-02) after reading its source; publication and recovery behavior is unchanged from 0.3.165
        - added `append_git_config`, `calculation_env`, `calculation_environment`, and the `update` subcommand in `scripts/ci/release_path.py`; the dry run and `release-pr` share one calculation environment
        - the attributes file holds only `*.ps1 text eol=crlf` (identical to `.gitattributes` line 14); it is a fallback below any in-tree `.gitattributes`, so PowerShell users' checkouts and current trees are unaffected; only release-plz's temporary copies at commits before 2026-07-30 receive the rule
        - Linux (`build-linux`, Git 2.47.3, release-plz 0.4.3, host Git config isolated; evidence `/tmp/rpr/evidence-r1/`):
                - `r1-update-0.4.3` (real tags, repair config, repair environment): exit 1, `cannot find package "git-wt-cli" in workspace …`; tree unchanged
                - `d-retag-repair-0.4.3` (diagnostic; three tags re-pointed to rename commit `82e34966b` in the disposable clone only): exit 0, exactly the two approved packages 0.1.0→0.1.1, no other package, no dependency bump
                - `d-retag-control-0.4.3` (same, without the attributes environment): exit 0 — the two-package walk never crosses the 2026-07-30 `.gitattributes` commit
                - `d-retag-wide-repair-0.4.3` (workspace-wide config, repair environment): exit 0 after 5,595 checkouts including `813ac4cd`, where Phase 1's 0.4.2 run aborted; this shows the attributes repair is load-bearing for the later workspace-wide restoration
        - discovered and fixed sibling defect R1-F1.6 (`calculation_report` first-line index-column strip)
        - **blocked** on R1-F1.7: the three rename tags. Options for the author: (a) amend the spec to allow re-pointing the three `-v0.1.0` tags to `82e34966b` (the GitHub Releases for them were not checked; the last Release was July 12); (b) get an upstream release-plz change that resolves a package by manifest path at the tag; (c) another approach. Changing the release set does not help
        - orchestrator verification: tag targets and package names at `907cd6e` checked directly; release-plz 0.4.3 `next_ver.rs` read to confirm ineligible `git_only` packages are looked up too; combined diff inspected
- work completed for 'R1-F1: release calculation' at 20:27:45 (parent remains **blocked**)

### Completion Report

- **outcome: blocked** — elapsed 2026-11-08T19:43:46-08:00 → 20:30 (about 46 minutes)
- parent findings: total 1 = fixed 0 + already fixed 0 + blocked 1 + unresolved 0 + disputed 0
- instance rows: 8 (fixed 4: F1.2, F1.3, F1.4, F1.6; clean 1: F1.8; blocked 3: F1.1, F1.5, F1.7)
- verified repairs: release-plz pin 0.4.3 across all three jobs; a shared, process-scoped calculation environment for the dry run and proposal preparation; a success regression next to the preserved failure regression; publication/recovery re-checked against 0.4.3 source with updated pin guards; docs and skill updated; `calculation_report` strip defect fixed
- open:
        - **R1-F1.7** — the three rename tags at `907cd6e` stop every `git_only` calculation; the spec forbids moving published tags and changing the tag format, so this cannot be decided in this cycle. Next action: author ruling (see options above), then one bounded Linux `release_path.py update` run
        - **R1-F1.1** and **R1-F1.5** — the successful Linux dry run and the acceptance evidence follow from the R1-F1.7 ruling; no code change is expected beyond whatever the ruling requires
        - unchanged external prerequisites (not findings): hosted dry run, App installation, prerequisite green on `main`
- changed files this cycle:
        - `.github/actions/setup-release-plz/action.yml` (R1-F1.1)
        - `.github/workflows/release-plz.yml` (R1-F1.1, R1-F1.2)
        - `scripts/ci/release_path.py` (R1-F1.2, R1-F1.6, R1-F1.8)
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` (R1-F1.2, R1-F1.3, R1-F1.4)
        - `tools/test-toolkit/tests/fixtures/release_path/driver.py` (R1-F1.2, R1-F1.6)
        - `docs/releases.md`, `.claude/skills/rust-devops/ci-cd.md` (R1-F1.4, rename-tag stop, attributes repair)
- final verification (orchestrator, after all subagent work):
        - `cd tools && just test` → 513 passed, 4 skipped, 3 slow (`no_archive_executed_target_bakes_in_a_producer_path` 6.40 s, `the_shipped_planner_emits_plans_the_guards_reader_accepts` 5.67 s, `report_resolves_the_worktree_base_by_the_wt_contract` 7.47 s — the same pre-existing slow tests recorded in earlier phases, not touched here); the cargo registry "Permission denied" cleanup warning is environmental
        - `cd tools && just lint` → exit 0
        - `just check-tier-coverage tools` → stranded 0
        - `actionlint` 1.7.12 on `release-plz.yml` and `build-integrations.yml` → exit 0
        - focused: `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(/release_history_walk|calculation_report|release_calculation/)'` → 5 passed (R1-F1.2, R1-F1.3); subagent focused selection `test(/release/) | test(/integration/)` → 35 passed
        - Linux real-tool calculation → exit 1 (R1-F1.7), recorded above
- local OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` are unset; set them to the SSH hosts for Linux, native Windows, WSL, and macOS testing. The review accepted `build-linux` as the Linux host

## Implementation of Review Findings #2

> **started at:** 2026-11-08T20:34:38-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/fixes/2026-11-06-release-path-repair/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review frontmatter: two findings (`R1-F1` high, recurring; `R2-F1` high, new), no `human_in_the_loop`, `has_blocked_findings: false`, `implemented: false` (already false; left false until the cycle closes)
- the review rules that R1-F1.7 is not blocked: a narrowly patched, reproducibly pinned release-plz build is a reversible implementation choice inside the spec's tool/configuration scope, so this cycle makes that choice rather than waiting for a ruling
- local OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` are all unset in this session; the `build-linux` SSH alias (`BatchMode=yes`) is reachable and reused as the Linux evidence host

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Release calculation remains unable to produce the approved proposal | fixed | every child fixed or clean; real unchanged-tag Linux calculation exit 0 with exactly the approved two packages |
| R1-F1.1 | Dry run/shared setup: successful unchanged-tag Linux calculation with exactly the approved edits | fixed | patched 0.4.3 via the shared setup action; `build-linux` `release_path.py update` with real tags exit 0 (`/tmp/rpr/evidence-r2/out-r2/`) |
| R1-F1.2 | Proposal preparation: shared environment, completed calculation, clean checkout | fixed | `release-pr` job uses the same setup action and `calculation_environment`; one-setup-per-job and clean-tree contracts pass; real calculation in that environment exit 0 |
| R1-F1.3 | Historical checkout regressions (failure + repaired success) preserved | clean | `-E 'test(/release_history_walk/)'` 2/2 passed (0.38 s, 0.81 s); patch does not touch checkout |
| R1-F1.4 | Publication/recovery re-checked against the chosen build | clean | patch confined to `next_ver.rs`/`registry_packages.rs`/`lock_compare.rs` (contract-enforced); `release.rs` uses only untouched `takes_part_in_release`; exact-head, local-tag early return, independent tag/Release switches unchanged; pin guards pass |
| R1-F1.5 | Acceptance report with unchanged tags: generated files, exact two-package set, no dependency bump | fixed | report: `git-wt-cli` 0.1.0→0.1.1, `sniff-detection-cli` 0.1.0→0.2.0, `unexpected_packages: []`, new `worktree/cli/CHANGELOG.md`, `Cargo.lock` two version lines, no dependency bump; tags unchanged |
| R1-F1.6 | Porcelain status column preserved | clean | `calculation_report_lists_the_approved_bumps_and_every_change` passes in the full suite |
| R1-F1.7a | Historical `git-wt-cli` lookup (`worktree-cli` at `907cd6e`) resolves without moving the tag | fixed | real run: `git-wt-cli was released as worktree-cli` at `907cd6e`; unit test `git_only_finds_renamed_packages_by_manifest_path` fails without the change, passes with it |
| R1-F1.7b | Historical `sniff-detection-cli` lookup (`sniff-cli` at `907cd6e`) resolves without moving the tag | fixed | real run: `sniff-detection-cli … as sniff-cli`; same unit test covers the shape |
| R1-F1.7c | Historical excluded `darkmatter-lsp` lookup (`dmls` at `907cd6e`) handled despite exclusion | fixed | real run: excluded `darkmatter-lsp … as dmls` resolves without entering the release set; ineligible shape in the unit test |
| R1-F1.7d | Normal current-name lookup path unchanged | fixed | `git_only_matches_the_current_name_before_the_manifest_path` passes with and without the patch; unrenamed member in the rename test; `git_only_rejects_a_released_package_found_by_neither_name_nor_path` keeps the hard error |
| R1-F1.8 | Other tool/environment consumers | clean | all three jobs reach release-plz only through the one setup action; credentials/diagnose unchanged; lockfile comparison consumer fixed in the patch; other name-keyed consumers traced clean (see R1-F1 notes) |
| R1-F1.9 | Docs: correct the "only a decision about the tags" statement and the log's "every repair needs a ruling" claim | fixed | `docs/releases.md` stop section and its "only by a decision about the tags" sentence removed; patched-build section added; skill matched; log correction recorded |
| R2-F1   | Local HTTP fixtures depend on host name resolution and time out | fixed | all children fixed or clean; guard proves no lookup regardless of DNS speed |
| R2-F1.1 | `FakeApi.__init__`/`serving`: bind without host name lookup | fixed | `LoopbackHTTPServer` binds via `socketserver.TCPServer.server_bind` and assigns numeric name/port |
| R2-F1.2 | `github_forge_reads_release_state_over_http` finishes within L1 budget | fixed | case 0.11 s; Rust test `release_path_forge_never_reads_an_api_failure_as_absence` 0.157 s |
| R2-F1.3 | `github_forge_never_reads_a_failure_or_malformed_response_as_absence` finishes within budget | fixed | case 0.11 s (same Rust test) |
| R2-F1.4 | `release_pr_refuses_incomplete_credentials_and_leaves_the_checkout_clean` finishes within budget | fixed | case 0.65 s; Rust test `release_path_release_pr_refuses_incomplete_credentials_and_stays_clean` 1.10 s |
| R2-F1.5 | Remaining release/integration boundaries: no other server constructor or startup name lookup | clean | no other server constructor; numeric URLs; closed-port probe is a raw numeric bind; `gethostname()` in `test_cross_check.py` is outside fixture startup |
| R2-F1.6 | Guard: name resolution during fixture construction is rejected | fixed | `no_name_resolution()` around every `FakeApi` construction; reverting the subclass fails both Rust tests in ~1 s |

### R2-F1: Local HTTP fixtures depend on host name resolution and time out

- starting the work on 'R2-F1: local HTTP fixture startup' at 20:35:18
        - defect class: a test fixture's local server startup performs ambient host name resolution that the test never needs, so host DNS latency decides whether the test finishes
        - delegated to one foreground subagent; it reports log items back to the orchestrator
        - subagent: added `LoopbackHTTPServer` (bind via `socketserver.TCPServer.server_bind`, numeric `server_name`/actual port assigned directly) and a permanent `no_name_resolution()` guard around `FakeApi` construction that makes `getfqdn`, `gethostbyaddr`, `gethostbyname[_ex]`, `getaddrinfo`, `getnameinfo` raise
        - consumer case times (driver, one at a time): `github_forge_reads_release_state_over_http` 0.11 s, `github_forge_never_reads_a_failure_or_malformed_response_as_absence` 0.11 s, `release_pr_refuses_incomplete_credentials_and_leaves_the_checkout_clean` 0.65 s
        - guard proof: with the constructor reverted to `ThreadingHTTPServer`, both Rust tests failed in about 1 s with `fixture startup resolved a host name: ('127.0.0.1',)`; restored afterwards
        - sweep: `FakeApi` is the only server constructor; `url`/`api_at` use numeric URLs; the closed-port probe is a raw numeric bind; `scripts/ci/test_cross_check.py` `gethostname()` is a local read outside fixture startup; no Rust test starts a server
        - host DNS was fast during this cycle (`getfqdn('127.0.0.1')` 0.03 s), so the 30 s timeout was not reproduced; the guard proves the lookup is gone regardless of resolver speed
        - orchestrator verification: diff inspected; `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(/release_path_forge|release_path_release_pr/)'` → 2 passed (0.157 s, 1.100 s)
- work completed for 'R2-F1: local HTTP fixture startup' at 20:37:12

### R1-F1: Release calculation remains unable to produce the approved proposal

- starting the work on 'R1-F1: release calculation' at 20:37:12
        - defect class: release-tool compatibility selected without a successful calculation against this repository's real history; here, historical package identity is looked up only by the current package name
        - correction of iteration 1: the claim that every repair of R1-F1.7 needed an author ruling was too strong; patching the pinned tool is a reversible choice inside the spec's tool/configuration scope, and the docs' "resolved only by a decision about the tags" sentence was removed with its section
        - upstream check: 0.4.3 (2026-11-02) is still the newest release; upstream `main` since then has only dependency/CI/security commits, nothing on historical lookup, so 0.4.3 is patched
        - patch `.github/actions/setup-release-plz/release-plz-0.4.3-historical-manifest-path.patch` (sha256 `801ca6be…6ad3d`, three upstream files: `next_ver.rs`, `registry_packages.rs`, `lock_compare.rs`):
                - new `ReleasedWorkspace::package_for`: current name first (unchanged path); otherwise the released workspace member with the same manifest path relative to the workspace root; otherwise the original `cannot find package` error (never an initial release)
                - the result map is keyed on the current name; the `Cargo.lock` workspace comparison uses the same lookup
                - downstream consumers traced: registry-package lookups, package equality/changelog diff, README check, version comparison, TOML dependency comparison, changelog/tag regex, registry-mode paths — clean through the key fix or unreachable for a renamed package
        - setup action: downloads the upstream `release-plz-v0.4.3` archive, verifies SHA-256 `aa04f023…16bc98`, `git apply`s the patch, runs the patch's upstream unit tests, builds `--locked` with the repository toolchain; `actions/cache/restore`/`save` pinned by SHA, keyed on version + archive SHA + patch hash + toolchain + OS + arch, saved only after tests, build and version guard pass; binstall and any unpatched fallback removed; all three jobs share it
        - upstream unit tests (Linux): without the code change 19 passed / 2 failed (`git_only_finds_renamed_packages_by_manifest_path` with the real `cannot find package "git-wt-cli"` message; `workspace_lock_comparison_finds_a_renamed_package_by_manifest_path`); with it 21/21; the current-name-first and neither-name-nor-path tests pass both ways
        - Linux build on `build-linux` (Rust 1.98.1): tests 48 s, release build 73 s
        - **real unchanged-tag calculation** (`build-linux`, Git 2.47.3, patched 0.4.3, repair config + repair environment, `release_path.py update`, evidence `/tmp/rpr/evidence-r2/out-r2/`): exit 0 in 81 s; log names `git-wt-cli was released as worktree-cli`, `sniff-detection-cli … as sniff-cli`, `darkmatter-lsp … as dmls`, each at `907cd6e`
                - report: `git-wt-cli` 0.1.0 → 0.1.1, `sniff-detection-cli` 0.1.0 → **0.2.0**, `unexpected_packages: []`, `proves_initial_release: true`
                - changes: `Cargo.lock` (two version lines only), both `Cargo.toml`, `sniff/cli/CHANGELOG.md`, new `worktree/cli/CHANGELOG.md`; no other package, no dependency bump
                - tags: all three tag objects and their `907cd6e` targets identical before and after; clone tag list equals the mirror's 79 tags
        - surprise: the 0.2.0 comes from walking back to the real tag commit, which takes in `feat(sniff)!: …` (breaking change bumps the minor version on 0.x); the iteration-1 re-tag diagnostic started later and showed 0.1.1. The set is still exactly the approved two; the version is release-plz's rule and is flagged for the author in the review's `human_review_items`
        - cache trust: a dry run on a branch writes into that branch's cache scope; the credentialed jobs read only their own and the default branch's scope; anyone who can write a cache entry readable there can already push to the repository, so no further guard is added
        - repo contracts: new `the_release_tool_is_built_from_verified_source_with_the_in_repo_patch` (SHA pin, URL, check, patch, `--locked`, toolchain, cache key parts, step order, no binstall/prebuilt/`actions/cache@` route, patch confined to three files); mutation (dropping `--locked`) failed it; `the_release_tool_setup_pins_one_version_on_the_repository_toolchain` updated
        - docs: `docs/releases.md` banner reduced to the allowlist, "calculation currently stops" section removed, new "Why the release tool is a patched build" section with how to drop the patch; `.claude/skills/rust-devops/ci-cd.md` matched; no spec/fix named
        - orchestrator verification: action, patch core, and evidence files read; report JSON, status, tag files and diff version lines checked directly; phase gates below
- work completed for 'R1-F1: release calculation' at 20:51:35

### Completion Report

- **outcome: complete** — elapsed 2026-11-08T20:34:38-08:00 → 20:53 (about 19 minutes)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: R1-F1 children 13 (fixed 8: F1.1, F1.2, F1.5, F1.7a–d, F1.9; clean 4: F1.3, F1.4, F1.6, F1.8; the parent counted separately); R2-F1 children 6 (fixed 5, clean 1: R2-F1.5)
- verified repairs:
        - R1-F1: release-plz 0.4.3 built from its verified upstream archive with one in-repo patch that resolves a renamed package by manifest path; the real, unchanged-tag Linux calculation now succeeds with exactly `git-wt-cli` 0.1.0→0.1.1 and `sniff-detection-cli` 0.1.0→0.2.0
        - R2-F1: the fixture HTTP server binds without a host name lookup, and a guard rejects any lookup during its construction
- open: none. Author items (not findings): confirm `sniff-detection-cli` 0.2.0 and the patched-build choice (review `human_review_items`); unchanged external prerequisites — hosted dry run, App installation, prerequisite green on `main`, the approved real release — remain pending as the spec allows
- changed files this cycle:
        - `tools/test-toolkit/tests/fixtures/release_path/driver.py` (R2-F1)
        - `.github/actions/setup-release-plz/action.yml` (R1-F1.1, F1.2, F1.8)
        - `.github/actions/setup-release-plz/release-plz-0.4.3-historical-manifest-path.patch` — **new, untracked: must be added with the rest of the change, or CI's `hashFiles` and the contract test will not see it** (R1-F1.7)
        - `.github/workflows/release-plz.yml` (header comment only; R1-F1.1)
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` (R1-F1.1, F1.4, F1.7)
        - `docs/releases.md`, `.claude/skills/rust-devops/ci-cd.md` (R1-F1.9)
- final verification (orchestrator, after all subagent work, no subagent running):
        - `cd tools && just test` → 514 passed, 4 skipped, 3 slow (`no_archive_executed_target_bakes_in_a_producer_path` 6.45 s, `the_shipped_planner_emits_plans_the_guards_reader_accepts` 5.76 s, `report_resolves_the_worktree_base_by_the_wt_contract` 7.42 s — the same pre-existing slow tests recorded in iteration 1, untouched here); cargo registry cache-cleanup "Permission denied" warning is environmental
        - `cd tools && just lint` → exit 0
        - `just check-tier-coverage tools` → stranded 0
        - `actionlint` 1.7.12 on `release-plz.yml` and `build-integrations.yml` → exit 0
        - focused: `-E 'test(/release_path_forge|release_path_release_pr/)'` → 2 passed (R2-F1); subagent `-E 'test(/release/) | test(/integration/)'` → 36 passed (R1-F1)
        - Linux: patched upstream unit tests 21/21 (2 fail without the change); real calculation exit 0 (R1-F1.1, F1.5, F1.7)
        - `scripts/ci` Python tests not run: `pytest` is not installed on this host and no `scripts/ci` file changed this cycle
- local OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` are unset; set them to the SSH hosts for Linux, native Windows, WSL, and macOS testing. The `build-linux` alias was used for Linux evidence
