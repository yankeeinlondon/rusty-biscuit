---
kind: plan
total_phases: 6
created: 2026-11-08
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Any-host evidence implementation plan

## Summary and Definition of Done

Extend the prerequisite Rust evidence client so equivalent, complete work qualifies regardless of its producer's name, checkout location, or entry command. Add explicit execution and activity proof, record native lint/check and companion completion, and report all four environments on pull requests without scheduling deferred work. Keep immutable attempts, authenticated store access, snapshot revalidation, failure ordering, constraints, nightly certification, and the Linux/macOS merge policy.

Completion means local, remote, and hosted producers pass the same qualification tests; native Windows earns eligible native credit; native WSL2 cannot replace archive proof; report bytes are durably available; incomplete, contaminated, incompatible, or filtered work cannot earn whole-cell credit; lint/check/companions have genuine completion proof; reverse-dependent checks and standalone guards retain execution; and deferred evidence is visibly advisory. Frozen fixtures, fake-store integration, scoped package checks, and portability evidence must pass. Current documentation, schema mirrors, command help, and skills must describe the resulting behavior.

This plan builds on the completed `2026-11-06-rust-evidence-client` implementation. At planning time that feature has a plan, but its `evidence-client/` package and store implementation do not exist in this checkout. Its planned Rust module names below are intentional dependency contracts, not claims that those files already exist. Execute this feature after that prerequisite; do not reimplement its transport, authentication, migration, bootstrap, or failure resolver. An absent prerequisite is a named dependency failure, never a reason to fall back to Python qualification or fabricate proof.

Run all phases without a human gate. Waves are globally numbered; tasks in the same wave have disjoint ownership and may use subagents. Every subagent brief must state that the entire session tree is non-interactive. Stay in the starting branch/worktree; do not commit, push, run `cargo fmt`, or move the feature to `_completed`. Write progress, test evidence, and spec departures to this directory's `implementation-log.md`. Final human review follows the last phase and does not interrupt execution.

## Phase 1 — Define execution and proof contracts

### Necessary Rules

1. **One qualification owner.** Extend `evidence-client/lib/src/{contracts,qualification,recording,git_inputs,inspection}.rs` and the prerequisite CLI/adapters. `EvidenceClient::evaluate(&VerificationRequest)` remains the acceptance seam. Keep the prerequisite store/snapshot/pending APIs and ancestry/retry resolver. Do not restore `local_evidence.py` after its prerequisite removal. Evidence: prerequisite plan rules 4, 7, 11, and 15; today's `local_evidence.py::{_resolve,_supersedes}` already resolve failures independently of wall-clock order.

2. **Producer provenance only.** Keep `hook`, `rig`, `hosted`, and `seed` as provenance values; remove qualification branches on those values. All new passing observations require identical proof validation. Remove rig-only environment eligibility and archive-only entry-command restrictions in the Rust ports of `record_cross_check`, `_proof_problem`, and schema validation. Existing `cross_check_rejection` clean-tree and filter semantics remain coverage checks. Evidence: `schema.py::observation_problems` currently forbids hook proof, `_proof_problem` checks rig/seed only, and `CROSS_CHECK_HOSTS` restricts recording to two environments.

3. **Versioned rollout.** Introduce observation collection version 4, containing new activity cells rather than rewriting version-2 receipt cells. Keep legacy receipt versions 1/2 and collection 3 readable and immutable. New observation proof has `schema_version: 1`; add required `execution`, `checkout`, `activity`, and `coverage` objects. Keep existing observation IDs canonical over their actual original version. Bump resolved plan 7 to 8 and environment table 3 to 4 for required execution contracts and reporting requirements. Keep expected-test manifest 2, completion record 1, scope receipt 1, certification 1, result document 5, build archive formats, and prerequisite decision protocol 1 unchanged; new advisory output is a separate document. The prerequisite establishes these starting versions. Update `schema.py`, generated `contract.json`, `ci-rollup.rs`, `test_toolkit::archive_guard`, hook plan fixtures/stub, and every environment-version mirror together. Evidence: `docs/cicd/schema-versions.md` requires bumps for new required fields, independent format versions, and synchronized mirrors.

4. **Execution requirement.** Add required `execution_contract` to each planned cell and reporting requirement: `{modes, runtime, build}`. Modes are a nonempty set drawn from `native` and `archive`; runtime contains OS family, WSL identity, architecture, ABI/libc requirements; build binds the planner's toolchain/profile/features/configuration and target compatibility. Native test environments permit both modes; WSL2 tests require `archive`. Lint/check require native execution and an actual build toolchain. Existing environment policy still schedules those gates only on Linux. An archive result proves consumer behavior, never consumer compilation; it cannot supply L1-to-check substitution. Evidence: `environments.json` explicitly marks WSL2 `archive_only`, `build_identity`/`compatibility_reason` already model producer/runtime compatibility, and the specification requires mode comparison rather than producer-name trust.

5. **One coverage model.** Extend `affected_scope.py::{coverage_contract,package_cells,derive_build_records}` to bind execution, full companion descriptors (commands, inputs and tools, not merely names), runtime/browser identity, target kinds, selected tests, slow-test selection, features, and backends. The verifier recomputes committed input closure and coverage, not just submitted hashes. Include manifests, lockfile, build scripts, Cargo configuration, toolchain, recipe closure, and gate-global inputs conservatively; do not add version-only or lockfile-only exemptions. Missing Git objects or unknown closure means unproved. Evidence: `coverage_contract` currently hashes companion names only; `local_evidence.py::{cell_input_paths,gate_input_identity,_observation_refusal}` show the existing input/features/backend boundaries.

6. **Before/after checkout.** Add `RunSession::begin` and `RunSession::finish` in `recording.rs`; callers pass a canonical planned cell and requested mode, not a trusted clean flag. Capture committed head/tree plus fingerprints of tracked inputs and relevant untracked/ignored inputs before execution and again after completion. Use `sniff` for requested OS/hardware/repository discovery and `gix` for committed-object comparison; hash through `biscuit-hash`. Scan the declared source/build/recipe/companion closure, including directory contents and ignored files within source roots; exclude designated Cargo output and the session's staging directory, and exclude ordinary `.git` internals. A build closure reading a generated source input remains checked even under an output directory. Unknown input discovery or inability to enumerate produces unproved. A mismatch retains a diagnostic attempt and cannot qualify. Evidence: specification qualification item 1 and prerequisite `git_inputs`/`recording` ownership; a caller's `remote_dirty` flag is insufficient.

7. **Real activity proof.** New activity cells contain package/gate, outcome, exit status, completion, duration, gate-input identity, backends, and report references; counts are present only for test activities. `activity` is a tagged union: `tests` binds expected listing/completion/report digests and nextest version; `command` binds exact argv, working directory relative to repository, resolved executable/tool versions, relevant environment/build configuration, stdout/stderr digests and completion. `coverage` binds targets/features/filter/backends/browser and every required companion's completion proof. Command failure records its nonzero status and logs without invented test identities. A test failure retains observed failures without requiring a passing completion record. Interruptions remain diagnostic incomplete attempts. Evidence: `receipt_cell` currently turns missing JUnit or unnamed command failures into partial results, and the spec explicitly prohibits synthetic lint/check test counts.

8. **Durable proof required.** Every qualifying pass, including old evidence, must have accessible digest-checked reports and sufficient execution/coverage/checkout proof. Remove the foreign `host-dir` availability exception from qualification. Keep old records visible for diagnostics, but reject proof omissions with `observation-proof-missing` and missing bytes with `observation-report-unavailable`; do not manufacture new proof from provenance. New publication uses the prerequisite's immutable object upload and pending bundle recovery. Credential authorization remains authenticated service context, never JSON environment/host fields. Evidence: `_report_problem` currently accepts nonexistent foreign directories, prerequisite rule 8 preserves that behavior only for its policy-preserving port, and this spec now expressly requires retrievable proof and verified checkout/environment.

9. **Legacy compatibility is refusal-safe.** Legacy v1/v2/v3 records retain their original parsers, IDs, and diagnostic display. Evaluate them with the new requirements; existing fields may establish a requirement only from retained original evidence bytes. Missing before/after or execution coverage proof earns no new reuse, including exact-tree v1 passes. Complete qualifying historical failures remain resolver inputs under the prerequisite failure rules; missing new pass proof must not hide a previously qualifying failure. Mixed readers reject version 4 rather than treating it as v3. This intentionally tightens the prerequisite's legacy acceptance as authorized by this specification's rollout section.

10. **Reuse boundaries.** Enable lint, direct check, browser, and companion-bearing test reuse after their proof is implemented. Keep `dependents` checks non-reusable, with no L1 substitution there. Direct check proof wins over the existing L1 substitution; preserve that substitution only for passing, whole-tier, own-environment native L1, without broadening its acknowledged target-kind exception. Do not credit lint as check. Preserve exact-tree/same-filter cross-host reuse solely for planner-narrowed test-input cells. Manually filtered ordinary runs remain smoke attempts even when they exit zero. Evidence: `affected_scope.py::{cell_evidence,check_evidence,package_cells}` currently special-case check and disable companion/lint reuse; spec activity table preserves the restricted substitution and dependent exclusion.

11. **Guard coverage stays executable.** Keep cells carrying the `archive-path-guard` companion non-reusable in this feature. Preserve its planned scope and companion execution, including companions-only lint. Move Schematic's existing `test-fresh-output` step into a scoped build-owner guard job driven by a new required plan field `standalone_guards`; select it whenever source scope requires a `schematic-gen` build before evidence removes build records. Run once per original producer, using the existing fetch, recipe, artifact retention, and failure propagation, even when every associated test cell reuses. No package-wide or workspace-wide extra run. Evidence: `archive_guard_only_selection` relies on lint being non-reusable, while `ci.yml`'s fresh-output step lives inside build-owner execution and tests filtered out of archive L1. Merely leaving that step inside a removed build job loses coverage; explicit guard rows preserve it.

12. **Reporting is a projection.** Add required plan `evidence_requirements: {packages, builds, cells}` for the selected change's canonical requirements across all four environments, plus `standalone_guards`. Refactor `calculate_scope` to derive package/change selection once, resolve requirements with all environment contracts, then project event scheduling into existing executable `cells`, `builds`, rows, anchors, and constraints. Reporting requirements carry pre-reuse coverage, not post-reuse fields stripped by `mark_reused`; they never generate dispatch rows or prohibitions. Keep lint/check environment policy and nightly anchor policy. A docs-only scope reports zero required cells, never four passing OSes. Evidence: `schedule_environments` owns event policy, `coverage_contract` requires pre-reuse backends, and the spec forbids an independent reporter scheduler.

13. **Advisory document.** Add `evidence-client inspect --plan <path> --all-environments --json`, returning advisory schema version 1: evaluated head/tree, plan identity, snapshot revision, scope totals and per-cell states `passed`, `failed`, `conflicting`, `unproved`, `accepted-gap`, `unavailable`, with evidence IDs/reasons. Per-environment summaries show covered/required totals and all nonpass counts; partial scope never says the OS is fully proven. Publish an Actions job summary and artifact from a separate `continue-on-error` advisory job, without `checks: write`, required status, or a new `ci-gate` dependency. Store outage yields unavailable with zero credited passes, while existing required scheduled cells execute. Known deferred failures precede missing evidence in display. Accepted gaps never become passes or release authorization. Evidence: spec's three explicit rulings and `ci.yml`'s policy-free `ci-gate` fold.

14. **Freshness and retries.** Evaluate reuse and advisory results against the prerequisite snapshot protocol, then revalidate immediately before applying/publishing. Retain bounded full reevaluation and visible unavailable exhaustion. Do not change explicit retry validation, cycle detection, strict ancestry supersession, same-commit conflicts, or divergent-history ordering. A retry of one independent failure clears only that predecessor. Delayed uploads and clock skew establish no order. Evidence: prerequisite rules 7 and 11 and spec's failures/retries section.

15. **Commands and scope.** Keep current cross-check default execution mode and argument semantics; add explicit `--execution-mode native|archive` to request planned native work without a fake feature override. Default `archive` remains for the current convenience path. Matching native Windows/Linux/macOS runs may now qualify; native WSL2 remains diagnostic only. Build flags qualify only when their resolved features/build settings match the canonical contract and selection remains complete. `ci-seed` stays pinned to clean `origin/main`, resumable, and L1-only, reports unsupported gates with nonzero exit, and uses the same evaluation API. Ordinary package `just test` remains a test command; reusable recording occurs through the shared `ci-local`/cross-check/hosted runner, not a new hidden upload in every test recipe. Evidence: `cross-check.sh` switches to native upon build flags, `seed.py::SUPPORTED_GATES` limits execution, and the spec keeps both command boundaries.

16. **No spike or schedule expansion.** No spike is needed: the spec settles archive portability and retry semantics explicitly. Add no benchmark, new OS matrix, merge-triggered run, failure-issue workflow, release-readiness rule, or retention verifier. Portability uses existing per-environment validation paths and qualifying evidence. Planning host discovery found only `BUILD_LINUX` declared; do not invent Windows/WSL SSH destinations. Missing required runtime evidence is reported as incomplete validation, not a claimed pass or permission to change scheduling.

### Wave 1 — Contract foundation

- [ ] **Freeze boundaries**
  - Create `implementation-log.md` and fixtures under `evidence-client/tests/fixtures/any_host/` from prerequisite real-shape plans, attempts, reports, and Git histories. Record prerequisite revision, starting schema versions, and policy departures in rules 8–11.
  - Freeze clean controls for native and archive tests, command activities, companions, narrowed cells, gaps, and guard-only selections. Preserve old-version bytes as negative/pass-compatibility controls; no live credentials.
- [ ] **Define contracts**
  - Own `contracts.rs`, schema definitions/mirrors, environment table, and `docs/cicd/schema-versions.md`. Implement rules 3–7, 12–13 as typed public contracts with strict loading and explicit versions, and add fake-store fixture support for version 4.
  - Provide shared validation types `ExecutionContract`, `ExecutionProof`, `CheckoutProof`, `ActivityProof`, `CoverageProof`, and `AdvisoryReport`. Complete the Input Robustness Matrix below before exposing acceptance.

Checkpoint: version-specific fixtures load through the public library, malformed inputs are located errors, unsupported readers refuse the new version, and generated schema mirrors agree. No producer emits new accepted evidence yet.

## Phase 2 — Implement qualification and canonical planning

### Wave 2 — Independent policy work

- [ ] **Verify proof**
  - Own `qualification.rs` and its tests. Apply uniform producer-independent proof checks, durable byte validation, environment/build/mode coverage, and legacy refusal rules. Keep the prerequisite resolver and snapshot machinery; add specific diagnostic details to existing rejection categories.
  - Test identical proof with every producer kind, remote/local/hosted provenance, foreign paths, missing/modified objects, dirty/changing trees, x86/arm mismatches, Windows/WSL separation, native/archive compatibility, archive-only WSL, browser/runtime/backends, features, companions, target kinds, and narrowed versus manual filtering.
  - Exercise documentation-only and merge histories using committed Git objects, manifest/version/lockfile changes, missing objects, surviving failures, same-commit unordered runs, divergent branches, multiple independent retry predecessors, delayed upload, and concurrent revision changes. Assert public `VerificationResult` decisions and retained failure diagnostics.
- [ ] **Resolve requirements**
  - Own `affected_scope.py`, planner adapter, planner fixtures/tests, and `schema.py` runtime validation after Wave 1's definitions. Resolve one canonical requirement graph and project scheduling; enable eligible lint/check/companion cells only as defined above.
  - Update `cell_evidence`, `check_evidence`, `apply_accepted_cells`, coverage hashing, build pruning, and anchor coverage to honor execution contracts and direct check proof. Add scoped `standalone_guards` selection before reuse pruning.
  - Prove PR scheduled rows stay Linux/macOS only, lint/check remain Linux only, producer-only builds stay producer-only, docs-only selection stays empty, nightly certification reopens failures, dependent checks execute, and archive-guard lint remains non-reusable.

Checkpoint: fake-store public evaluation and frozen plan tests agree on cell identity, coverage, failures, guard selection, and event projections. Advisory-only requirements create no executing rows. Direct-check and restricted L1 substitutions have distinct tests.

## Phase 3 — Capture complete producer attempts

### Wave 3 — Capture before wiring

- [ ] **Capture sessions**
  - Own `recording.rs`, `git_inputs.rs`, and session tests. Implement before/after discovery and contamination checks, preflight output data, actual runtime/build capture, durable immutable report staging, command logs, and companion proof aggregation.
  - Keep output free of credentials; collect resolved tool identities once per session. Dirty or interrupted sessions retain diagnostics. A successful test run with failed upload retains a pending bundle and does not claim publication. Reuse prerequisite locking, retry-upload, atomic persistence, authentication, and digest utilities.
  - Test tracked modification/reversion, untracked/ignored source contamination, output-only changes, unreadable inputs, source-generating build scripts, child command failure, interrupted execution, and partial uploads through the public recording API.

### Wave 4 — Independent producer adapters

- [ ] **Wire local runners**
  - Own CLI recording/run integration, `just/ci-local.just`, `.githooks/pre-push`, and their tests. Render planned cells, actual mode, and known nonqualification reasons before execution using `biscuit-terminal`.
  - Replace local lint/check package-gate guesses with per-cell execution decisions; run direct check even when lint also exists, and skip qualified lint/check only with complete proof. Capture canonical argv/tool versions, companions, native L1/L2/browser proof, and matching narrowed filters.
  - Preserve original activity exit status and existing constraint enforcement. Run-session finish and publication must not mask failure; pending-upload diagnostics must distinguish execution success from publication success.
- [ ] **Wire remote runners**
  - Own `scripts/cross-check.sh`, `scripts/ci/seed.py`, host-script tests, and os command documentation. Route all four environments and both explicit execution modes through shared recording; remove rig-name privilege and native-mode blanket recording refusal.
  - Retain archive verification, builder-directory hiding/relocation, actual verified build key/digest, remote clone locks, noninteractive SSH, report transfer, and unchanged default archive behavior. Keep seed supported-gate checks and resume behavior; a seed run never credits unsupported gates.
- [ ] **Wire hosted producers**
  - Own `_package-ci.yml`, `_wsl-ci.yml`, hosted CLI integration, and completion integration tests. Capture command/tool/log proof for lint/check, existing expected-test/completion proof for tests, and every declared companion outcome. Missing activity proof leaves the entire required cell unproved.
  - Keep `completion.py`'s expected/report identity comparison, skip policy, backend validation, and completion version; attach its output to new activity proof rather than inventing test counts for commands. Store immutable bytes and preserve trusted publication roles from the prerequisite.

Checkpoint: one shared fake-store scenario per entry path records, uploads, reads from a distinct simulated reader host, and reuses matching proof. Negative controls launch no unintended work, and interrupted/failed/publication-failed runs remain visibly distinct. Tasks own disjoint adapters; shared recording changes are completed in Wave 3.

## Phase 4 — Preserve guards and publish advisory evidence

### Wave 5 — Independent workflow surfaces

- [ ] **Retain build guards**
  - Own `ci.yml` build-owner/fresh-output changes and workflow guard tests. Consume `standalone_guards` in a scoped native producer job, preserving Schematic's exact existing fetch/recipe/report path and making its failure reach `ci-gate` through ordinary blocking job results.
  - Remove duplicate fresh-output execution from archive production. Test executing builds, all tests reused, mixed producer slices, no Schematic selection, guard failure, and docs-only selection. Each selected original producer runs the guard once; no selection runs none.
- [ ] **Report deferred evidence**
  - Own `inspection.rs`, CLI inspect rendering, advisory summary helper/job integration file, and advisory fixtures. Evaluate only canonical `evidence_requirements`; retain snapshot revision/head/scope, all six states, per-environment counts, and prominent known deferred failures.
  - Add job wiring to `ci.yml` after the guard task's edits are integrated to avoid concurrent ownership. The advisory job has no gate dependency and never emits a blocking readiness check. It uploads the versioned JSON plus readable summary; timeout/outage produces unavailable output.
  - Test partial package coverage, no required cells, expired/missing report bytes, failed/conflicting/unproved/unavailable deferred results, accepted gaps, stale snapshot reevaluation, and failure of the reporter itself. For every scenario assert unchanged merge scheduling and gate outcome.

Checkpoint: a frozen PR plan displays all four environments, schedules only existing event work, and retains both guard paths. Reporting cannot turn an unavailable result or gap into pass. A full-reuse Schematic case still runs its scoped guard.

## Phase 5 — Validate integration and portability

### Wave 6 — Integrated verification

- [ ] **Exercise end-to-end**
  - Run the combined fake-store suite once after adapter integration. Cover native local/remote/hosted equivalence; archive relocation and native WSL refusal; matching lint/check/companions; unsupported seed gates; delayed pass after failure; retry graph with two independent failures; immutable duplicate publication; changing checkout; report loss; interrupted upload/retry; and snapshot revision change before application.
  - CLI wiring uses one control and two malformed input cases per loader, asserting located errors and no child launch/upload. Other robustness cases run in-process through public APIs. Keep tests hermetic and under the L1 speed contract.
- [ ] **Prove portability**
  - Reuse qualifying evidence first, then run missing scoped checks through the existing OS pathways from the os skill. Use local macOS, declared `BUILD_LINUX`, and existing Windows/WSL validation facilities; SSH always uses `BatchMode=yes`. Native Windows verifies native recording, Windows path handling, and durable reload; WSL executes a relocated archive with builder paths unavailable and no Cargo dependency.
  - Run `just test evidence-client evidence-client-cli repo-deps` for the integrated Rust package scope. Run the surviving Python contract suites directly: `python3 scripts/ci/test_schema.py`, `python3 scripts/ci/test_affected_scope.py`, `python3 scripts/ci/test_resolved_plan.py`, `python3 scripts/ci/test_ci_local.py`, `python3 scripts/ci/test_completion.py`, `python3 scripts/ci/test_cross_check.py`, and `python3 scripts/ci/test_seed.py`. The prerequisite removes legacy qualification tests; port the relevant cases from today's `test_evidence_reuse.py` into the Rust suite in Wave 2, then run that suite rather than restoring removed modules. Run `scripts/cross-check-hosts-tests.sh` for shell wiring.
  - Run `cargo clippy -p evidence-client -p evidence-client-cli -p repo-deps --all-targets -- -D warnings`; this matches the shared `_lint` recipe's compiler arguments without formatting or workspace-wide orchestration. Real terminal/browser proof goes through `just test-l2`/`just test-browser` with focus-safe harnesses, only for changed behavior requiring those resources. Injected backend/browser evidence validation is L1 and does not need windows.
  - Record each environment's actual command, revision, coverage, and result. An unavailable host/credential is a named incomplete validation result; continue all independent checks and finish with truthful status. Do not add environments, dispatch full-workspace CI, bypass signing/authentication, or label fixture coverage as runtime proof.

Checkpoint: relevant L1 and contract tests pass; unchanged policy controls pass; actual portability evidence is identified by environment and activity. No additional CI scope is introduced to obtain evidence.

## Phase 6 — Update current records and close implementation

### Wave 7 — Documentation and final audit

- [ ] **Document behavior**
  - Update `docs/cicd/{evidence,schema-versions,test-inputs}.md`, `.github/ci/schemas/README.md`, relevant execution/CI topic pages, evidence-client docs/help, rust-devops `ci-cd.md`, and os `ci-evidence.md`, `build-hosts.md`, and `wsl.md`. Keep README changes to overview/examples and link to current topic pages.
  - Update root `AGENTS.md` and `CLAUDE.md` statements that still restrict host evidence or assume lint is never reusable; preserve event schedule, merge policy, seed's L1 limit, and archive-only WSL. Document compatibility refusal, pending-upload recovery, native versus archive proof, and advisory states. Add a compact end-to-end example and Mermaid lifecycle to evidence docs; current topic pages never refer readers to feature snapshots.
  - Review comments/docstrings beside changed symbols, especially `check_evidence`, `package_cells`, `archive_guard_only_selection`, and ci-local's header: replace stale non-reusable/receipt claims with the new contract, preserving the documented restricted L1 exception.
- [ ] **Audit completion**
  - Compare each acceptance item in the specification to a test or recorded environment result. Check generated schema mirrors and pinned reader capability declarations; update the prerequisite's client pin through its existing distribution process only after real supported assets exist, never placeholder checksums. Version-4 emission requires new-version readers; unsupported old clients execute rather than guess.
  - Search changed validators for the robustness smells listed below, and changed production paths for producer-kind eligibility, foreign-directory trust, synthetic counts, latest-pass resolution, and swallowed publication errors. Fix findings and rerun only affected checks.
  - Finish implementation-log with completed phases, validations, compatibility changes, and remaining environmental failures. Leave the feature in its active directory, ready for review; do not mark completion while required validation is missing.

### Input Robustness Matrix

This feature changes JSON plan, environment, observation/proof, pending-bundle, and advisory-request loading. Use the prerequisite strict JSON loader for every one; do not add YAML/TOML configuration loaders. Default outcome for an invalid shape is a located public error naming document, cell/index and field, with no execution, publication, or reusable decision. Corrupt stored attempts remain diagnosable and unproved; unrelated valid immutable attempts remain evaluable. Invalid canonical plans abort the request. Transport unavailability is a separate unavailable result, never an empty successful dataset.

Test field shapes once per shared validator, using one edit to a positive real-shape fixture per case:

| Shared validator | Load-bearing members | Explicit exceptions |
|---|---|---|
| Exact version integer | collection/proof/advisory/plan/environment versions | Supported historical versions enter only their original typed parser; unsupported versions are refused before fields. Boolean and integral float are not integers. |
| Nonempty identity/string | package, environment, gate, head/tree, OS/arch/ABI, tool/version, command cwd, digest and object reference | No empty/null/absent identity defaults. Digests additionally validate algorithm/length/encoding; paths reject traversal. |
| Enum/tag | mode, activity kind, outcome, completion, advisory state | Unknown tags are errors, not diagnostic passes. |
| String collection | modes, target kinds, backends, features/selection arguments, companion identities, argv | `[]` is valid for backends, companions and feature arguments only where the canonical coverage requires none; test target kinds may be empty for a planned companion-only activity. Modes and argv must be nonempty. Every member is validated; no filtering. |
| Required object/record collection | execution/runtime/build, checkout before/after, activity, coverage, reports, companion proofs, evidence requirements, standalone guards | Empty report/proof sets cannot qualify a passing activity. Empty requirements/guard collections mean explicitly no selected work; no OS pass is inferred. Command activities omit test counts/listing; test activities require them. |
| Present optional field | retry_of, test_filter, test counts, archive build reference, browser identity | Absence means no retry/filter, no counts for command activities, no archive reference for native work, no browser for nonbrowser work. Explicit null is invalid. Empty filter/ID is invalid; archive and browser activities require their references. |
| Numeric/status | exit code, duration, counts, byte size | Zero is valid; negative duration/count/size and nonfinite numbers are errors. Failed command status is data and never reclassified as a successful parse or test pass. |

For each scalar validator cover absent/null/wrong whole type/empty where representable; for collection validators also cover one invalid member, all invalid members, and explicit empty. Each additional field wired to a shared validator gets a positive control and one invalid edit. Nested objects validate required children, not just object shape. Assert errors and side effects through `EvidenceClient` public evaluation/recording/request APIs, including unknown tags and mutually inconsistent mode/activity fields.

At the shared strict JSON document loader, test duplicate keys at root and nested levels, trailing garbage, truncated/invalid JSON, NaN/Infinity, blank document, wrong root type, and a second document. JSON comments/trailing commas are rejected. Add valid whitespace/Unicode controls. The same loader handles every listed document, so do not multiply document-shape tests per format name. Version dispatch tests still exercise every document family.

Audit `#[serde(default)]`, `Option<T>`, `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()` on all changed load-bearing paths. Use presence-aware decoding for absence/null distinctions. A robustness review finding must report every other shape in that validator row and every validator using the implicated loader shape; fix the shared decision once and retain wiring controls.
