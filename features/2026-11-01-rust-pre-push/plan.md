---
kind: plan
total_phases: 5
created: 2026-11-08
phase: 2
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
- Cargo.lock
- scripts/Cargo.toml
- scripts/lib.rs
- scripts/ci-push.rs
- scripts/ci-push/mod.rs
- scripts/ci-push/host.rs
- scripts/ci-push/json.rs
- scripts/ci-push/mode.rs
- scripts/ci-push/updates.rs
- scripts/ci-push/documents.rs
- scripts/ci-push/tests/mod.rs
- scripts/ci-push/tests/documents.rs
- scripts/ci-push/tests/run.rs
- scripts/ci-push/tests/updates.rs
- scripts/ci/fixtures/pre-push/constraints/minimal.json
- scripts/ci/fixtures/pre-push/constraints/scoped.json
- scripts/ci/fixtures/pre-push/plans/plan-macos-executing.json
- scripts/ci/fixtures/pre-push/plans/plan-macos-two-packages.json
- scripts/ci/fixtures/pre-push/plans/plan-wsl-absent.json
- scripts/ci/fixtures/pre-push/plans/plan-wsl-executing.json
- scripts/ci/fixtures/pre-push/plans/plan-wsl-reused.json
- scripts/ci/fixtures/pre-push/pull-requests/none.json
- scripts/ci/fixtures/pre-push/pull-requests/two-open.json
docs_updated_during_phase_1:
- docs/dependencies.md
docs_created_during_phase_1:
- scripts/ci/fixtures/pre-push/README.md
skills_files_updated_during_phase_1: []
packages:
- repo-deps
---

# Rust Pre-push Implementation Plan

## Summary and Definition of Done

Move pre-push orchestration into `repo-deps`'s `ci-push` binary, leaving the versioned hook as a Cargo launcher and preserving the installed worktree-aware dispatcher. Rust owns update parsing, context resolution, committed-tree review, path classification, constraint enforcement, gate execution, cancellation, and publication eligibility. Python remains the planner and evidence implementation; every Python process reached through the hook uses the interpreter Rust proved works.

Completion means:

- Every Bash runner case has a named Rust L1 counterpart, with a checked mapping in `implementation-log.md` beside this plan. The current runner contains 72 `run_test` registrations; inventory the file at execution start to capture subsequent additions too.
- All updates and contexts pass review before publication or gates. Failures in review block every mode; ordinary gate failures alone can pass in `warn`. Interruption always blocks and withholds validation.
- Clean HEAD gates consume the first HEAD context's reviewed plan. Dirty, overridden, changed-HEAD, and non-HEAD execution cannot publish exact-tree validation evidence. Scope remains independent of checkout cleanliness.
- Native Windows Git launched from Git Bash and PowerShell proves automatic no-gate pushes, explicit strict gates, supported Python fallback, scope publication, and correct argument/path handling without `--no-verify`.
- The owning L1 suite passes on macOS, Linux, native Windows, and WSL2 using the existing scheduling and archive mechanisms. No focused terminal/browser windows, external user stores, or authenticated fixture remotes are used.
- The Bash orchestration suite, its report converter, and dedicated workflow are retired; Rust retains dispatcher coverage. Docs and skills describe the shipped flow. No planner, receipt, or resolved-plan version changes occur.

The five phases execute consecutively without human gates. Waves identify independent task ownership; every wave finishes before the next begins. Concurrent agents receive narrow file ownership and the explicit instruction that the entire session tree is non-interactive. All changes stay in the starting worktree and branch. Do not commit, push the development branch, run `just complete`, or move the feature directory.

## Phase 1 — Establish Contracts and Foundations

### Necessary Rules

1. **Choose `local-tools`.** Register `scripts/ci-push.rs` with `required-features = ["local-tools"]`. The launcher runs `exec cargo run --quiet -p repo-deps --no-default-features --features local-tools --bin ci-push -- "$@"` after entering its invoking checkout. `scripts/Cargo.toml` already places `sniff` and `ctrlc` here; the spec recommends this tier. Preserve the dispatcher's selection of the invoking worktree. No timestamp-selected binary, global installation, bootstrap download, narrower sniff feature project, or performance spike is included. The spec explicitly rejects a bootstrap benchmark and supplies the process-cost rationale already.

2. **Use a small library boundary.** Add feature-gated `scripts/lib.rs` exposing `pub mod ci_push` from `scripts/ci-push/mod.rs`. Its public entry is `run(request: PushRequest, services: &mut dyn PushServices) -> anyhow::Result<PushOutcome>`. `PushRequest` carries repository cwd, remote name/URL, complete input bytes, mode, and selection; `PushOutcome` carries exit code and structured diagnostics. Production services own environment reads, process execution, clock/date, filesystem, and Git access; tests inject these boundaries. Put domain types and validators in this module tree, not a second workspace crate. Existing binaries are flat files with separately included test modules (`scripts/ci-plan.rs`, `scripts/ci-plan-tests.rs`); the new library permits public-API robustness tests without spawning a CLI for every cell.

3. **Keep dependency tiers isolated.** Add optional `gix = "=0.84.0"` with the revision/status/blob-diff/dirwalk/excludes/parallel/sha1 features used by sniff, optional `git2 = "0.20"` with default features disabled, `tempfile = "3"`, `dirs = "6"`, and `chrono = "0.4"` under `local-tools`. Add target-specific optional `libc = "0.2"` on Unix and `windows-sys = "0.61"` with Foundation, Threading, JobObjects, Console, and Security APIs on Windows, enabled by `local-tools`. Add test-toolkit and biscuit-test-harness as dev dependencies for runtime paths and gating. Preserve no-feature `ci-rollup` and `build-tools` dependency isolation. Versions follow sniff/worktree's existing manifests. Update `docs/dependencies.md` for additions.

4. **Use sniff discovery and gix reads.** Use `sniff::filesystem::git::GitRepo` discovery and its dirty-state observation, `api::remote_url` for the fallback remote, and `sniff::os::detect_os` for environment identity. Use a gix handle for object peeling, HEAD identity, ancestry, tree comparison, refs, and notes reads. `sniff::GitRepo::changed_paths` includes checkout status and loses rename classification, so it is not the committed two-tree diff API needed here. Map direct tree changes into changed/deleted/renamed-from paths, with Git-compatible rename detection and copies contributing their destination only. Compare against `git diff --name-status -z BASE HEAD --` in parity tests. Preserve raw repository path bytes on Unix; reject a path that cannot cross the native Windows/Python boundary with a located review error rather than lossy conversion.

5. **Bound Git exceptions explicitly.** Use Git CLI only for remote `ls-remote`, fetch, and push, which honor the user's transport configuration. Create detached runtime worktrees and remove their administrative records through git2: gix 0.84's `repository/worktree.rs` lists/proxies worktrees but provides no linked-worktree creation API; sniff's manifest explicitly names linked-worktree creation and checkout as git2 fixture operations. Create a uniquely owned temporary ref for git2 worktree creation, detach the new worktree at the reviewed commit, then remove that ref. Use git2 for scope note writes, whose existing implementation is the hook's `attach_note`; retain observation note union/publication in Python. Never run global worktree pruning or remove another invocation's artifacts. Explain these capability exceptions at the adapter module boundary.

6. **Parse all input first.** Validate mode before discovery or children. Default remote name to `origin`, resolve an absent/empty URL through sniff, and preserve an unresolved URL as unknown identity. Read all stdin before children; accept LF, CRLF, and a final line without newline; ignore whitespace-only lines. Each nonempty line must contain exactly four whitespace-separated tokens, valid full refs, and hex OIDs of the repository hash length. A malformed line names its line number and blocks before any review, publication, or gate. Reject duplicate destination updates as malformed. Skip deletions and non-branch destinations by name, and count `pushes_head` only for reviewed branch updates. The spec's empty/skipped-only rule takes precedence over the shell setting `PUSHES_HEAD` before checking destination kind.

7. **Preserve context ordering.** `review_ref_update`, `resolve_trigger_contexts`, `resolve_target_states`, and `remote_branch_tip` in `.githooks/pre-push` define the port: Git update order, gh result order, current target before distinct incoming target. Main uses its supplied previous tip and event `push`; other branches use `pull_request`. Invoke `gh pr list --repo OWNER/REPO --head DESTINATION --state open --json number,baseRefName` with null stdin and `GH_NO_UPDATE_NOTIFIER=1`. Parse JSON in Rust, eliminating jq as a pre-push prerequisite. Keep the existing accepted github.com URL spellings and fail closed for an unrecognized spelling containing github.com. PR lookup errors block. Cache remote tip lookups per branch, fetch missing commits once, and verify the exact advertised commit exists after fetch. A concurrently created base has incoming-only context; deletion of a PR base blocks. Non-GitHub and empty PR results review provisional remote-main contexts.

8. **Review committed inputs.** Reuse the checkout only for the initial clean HEAD commit; otherwise create a detached temporary worktree for that update. Invoke that revision's `affected_scope.py`, `local_evidence.py verify --cells`, and `affected_scope.py --apply-to`, preserving the arguments in `plan_trigger_scope` and `review_trigger_context`. Pass a NUL-delimited `--arguments-file` written by Rust, including `--deleted`, `--renamed-from`, and `--`; retain `diff_scope.py` for hosted CI and ci-local. An all-zero main base uses `--all`. Check constraints only against the resulting evidence-applied plan and the pushed URL plus destination/local branch names. No constraint path is handed to the planner during trigger review.

9. **Keep first-context identity separate from publication.** Capture raw and evidence-applied plans from HEAD's first context before attempting scope recording, including an all-zero base. Nonzero bases remain usable for gates even when scope recording refuses non-ancestry or fails. An all-zero base cannot bind a scope receipt or validation comparison and therefore gates replan for feedback. Other contexts never replace the first context. Review all contexts before attaching or pushing notes. This follows the spec's exact-plan contract; the shell's `record_scope_evidence` currently returns before copying all-zero plans, so recording and gate selection must become separate state.

10. **Select Python once.** `PythonCommand { executable: PathBuf, prefix: Vec<OsString> }` tries `python3`, Windows-only `py -3`, then `python`. Execute each candidate with a version probe requiring Python >=3.10, a two-second timeout, drained stdout/stderr, null stdin, and reaped timeout children. Save the proved command for every direct Python child. Report all attempted candidates in one final error. Export `BISCUIT_CI_PYTHON` as the selected native executable path and `BISCUIT_CI_PYTHON_ARGS_FILE` as the native path to a staging file containing its NUL-delimited argv prefix to gates. In `just/ci-local.just`, construct a Bash argv array from these values; read the prefix file with Bash `IFS= read -r -d ''` into that array, requiring no interpreter bootstrap. Without these variables preserve ci-local's standalone `python3` default. Replace every Python invocation in this recipe's execution path, including constraints directory lookup, planner/apply, evidence verification, self-tests, and companion suites, with the array. Python-to-Python subprocesses use `sys.executable`. The inspected planner/evidence/observation modules currently import their helpers rather than respawning python3; `build_key.helper_command` invokes ci-build/Cargo, not Python. Shebang-only references need no edit.

11. **Separate Cargo from execution.** `cargo run` finishes compilation before starting ci-push. Do not launch helper builds from a build script or hold a Cargo file lock inside Rust. Leave `build_key.helper_command`'s existing helper fallback intact; it can build after Cargo has released its lock. Do not select a sibling ci-build by timestamps or reuse the host helper for an older committed tree. Prove the fallback ordering with a fixture process protocol rather than a cold nested Cargo build in every L1 test.

12. **Own children and cancellation.** Centralize `Command` construction, null stdin by default, concurrent pipe draining, and child ownership. Unix children use a new process group; Windows children are assigned to a kill-on-close Job Object before resuming execution. Use `libc::sigaction` on Unix for INT/TERM/HUP, preserving the signal number in an atomic, and `ctrlc::set_handler` on Windows for Ctrl-C; handlers only set cancellation state. Terminate the owned process tree, reap direct children, join pipe readers, close Git handles, then clean worktrees. Return 130/143/129 for the corresponding Unix signal and 130 for Windows Ctrl-C; gate exits >=128 and the staged `interrupted` marker are interruptions in all modes. Warn never converts these to success. Print retained paths and exact manual removal actions on cleanup failure. No claim covers forced kill or power loss.

13. **Sanitize narrowly.** Remove `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`, `GIT_COMMON_DIR`, and `GIT_PREFIX` from repository-independent children; retain unrelated Git config/credential variables. Set `GIT_TERMINAL_PROMPT=0`, `PYTHONDONTWRITEBYTECODE=1`, and `PYTHONUTF8=1`. Clear inherited `BISCUIT_CI_PLAN_IN` before choosing the gates' input. Pass separate native arguments throughout. The launcher passes only Git's args and stdin, so Git Bash never converts Python's native paths. Selection is ASCII-whitespace-separated package/area names, without glob expansion; `.githooks/pre-push` currently shell-splits this variable and the spec promises space-separated selectors, not shell expressions.

14. **Preserve evidence responsibilities.** Scope uses `local_evidence.py scope-record` and `refs/notes/ci-local/scope`; validation uses `record-cells` and `observation_notes.py publish-receipt`. Keep their existing schemas and observation-union behavior. Before and after gates, verify HEAD equals the captured outgoing HEAD and the checkout is clean. Initial dirtiness always withholds validation, even after gates clean the checkout. Selection overrides, missing outgoing HEAD, missing reviewed nonzero comparison, missing local output plan, empty manifest, incomplete/interrupted runs, unsupported OS, and failed report retention each get an explicit withholding reason. Retain completed failure reports and publish diagnostic observations before returning the strict gate failure. Retry reports overlay the durable `<evidence-root>/<head>/<environment>` directory. Publication errors remain advisory; preserve reports even after a failed note push. Relative evidence roots resolve against the invoking checkout. Native Windows home follows USERPROFILE; Unix follows HOME. OS mapping is MacOS→macos-latest, Windows→windows-latest, Linux with Microsoft kernel→wsl2-ubuntu, other Linux→ubuntu-latest.

15. **Tighten malformed constraint inputs consistently.** `constraints.py::Constraint` currently uses truthiness, `str(...)`, and optional `.get`, accepting numbers/collections for required text and silently exempting malformed branches. Replace these coercions in both Python and Rust with the same documented validator: required nonempty strings `environment`, `reason`, `owner`, `expiry`; optional strings `gate`, `repository`, `branch`; expiry is a real `YYYY-MM-DD` calendar date inclusive of today. Unknown fields and explicit null are errors. Empty optional strings retain the existing wildcard behavior. Preserve arbitrary nonempty environment/gate strings rather than inventing a new enum. This is a deliberate fail-closed correction to malformed-input behavior, recorded in the implementation log and `docs/cicd/schema-versions.md`; the constraint store is unversioned, so no unrelated plan/receipt schema bump is made. Canonical valid records and matching semantics are unchanged. Update the Python reader in the same wave, so neither reader silently accepts another format.

16. **Keep the constraint matching algorithm.** Port `repository_identity`, `store_segments`, `default_directory`, `resolve_directory`, `load`, `Constraint.applies_to`, `expired`, `unsatisfied`, and `read_plan` from `scripts/ci/constraints.py`. Normalize URL credentials/scheme/.git exactly as today, including filesystem remote spellings and portable directory sanitization. An unknown repository/branch cannot exempt a record. Recursively sort JSON paths; malformed records bind even for another repository. Missing/non-directory store remains empty, matching `load`; unreadable existing files and traversal failures block. `execute` cells and `state == prohibited` violate matching environment/gate constraints; reused, accepted-gap, and absent cells do not. An unreadable plan blocks even with an empty store. Inject today's local calendar date into both parity runners.

17. **Use shared rendering without subprocesses.** Extract ci-plan's existing parse/render entry into `scripts/ci-plan-render.rs` with `render_document(bytes: &[u8], term: &Terminal) -> Result<String>`; keep `ci-plan`'s CLI behavior and existing tests. Call it from ci-push using biscuit-terminal `Prose`/`UnorderedList` for actionable diagnostics. Do not retain the release-binary/jq fallback in `render_trigger_plan`. Rendering compatibility defaults remain display-only; security decisions use separate strict validators.

18. **Retire duplicate scheduling.** Port `.githooks/tests/test-pre-push-dispatcher.sh` into Rust alongside orchestration tests, then delete both shell suites, `report-pre-push.py`, and `.github/workflows/pre-push-hook-tests.yml`. Keep root `test-pre-push-hook`, `test-pre-push-dispatcher`, and `test-githooks` as aliases to narrowly filtered repo-deps canonical L1 runs. Add literal runtime reads of `.githooks/pre-push` and `.githooks/pre-push-dispatcher` in their owning Rust tests so the established test-input index selects them. No new CI matrix, suite owner, or environment schedule is added. `scripts/Cargo.toml` already declares L1 with default local-tools enabled.

### Wave 1

- [x] **Freeze parity**
  - Create `implementation-log.md` with one row per current Bash `run_test` registration, original function name, assigned Rust test name, and eventual result. Include the separate dispatcher contract. Preserve the inventory after retiring files.
  - Add shared fixtures under `scripts/ci/fixtures/pre-push/` for constraints, valid resolved plans, PR output, path inventories, and evidence scenarios. Derive controls from existing hook/Python fixtures; mutate one load-bearing field per negative case.
  - Record the malformed-constraint correction and the first-context/all-zero state separation as explicit spec departures, without editing the spec snapshot.
- [x] **Build foundation**
  - Add the binary/library/module layout and optional dependencies specified above. Add `PushMode`, `RefUpdate`, `ReviewContext`, `PythonCommand`, `PushRequest`, `PushOutcome`, `PushServices`, and structured review/withholding errors.
  - Implement complete update parsing and strict document validators. The binary reads environment/stdin and renders the library result; validators remain usable in-process through `run`.
  - This task owns Cargo manifests, library wiring, and `ci-push/mod.rs`; fixture inventory work owns only fixtures/logs, allowing concurrency.

### Input Robustness Matrix

Default: invalid shape yields an error naming file/document, field/index, and update/context; review errors run no gate and publish nothing. Scope/validation helper output failures after successful review use the advisory publication rules instead. Test through `ci_push::run`'s public result with injected services and assertions that no gate/publication boundary was called.

Distinct JSON loaders are the shared constraint/plan/PR loader in Rust and Python's constraint loader. Rust uses a duplicate-detecting JSON visitor, then field validators; Python uses `object_pairs_hook` with duplicate rejection. Both reject trailing garbage, malformed JSON, comments, and trailing commas. Constraint documents must be objects; PR output must be an array; plan documents must be objects. Empty/whitespace JSON files error. A missing store and an empty PR array are explicit valid no-record/no-PR states, not parse failures. Test each loader's syntax policy once with a real fixture control.

Field validators and their exceptions:

- Required nonempty text: constraint `environment/reason/owner`, PR `baseRefName`, and required cell identity/status strings. Absence, null, wrong scalar/container type, and empty string error. Share one shape matrix, plus a control and one invalid wiring case per field. Constraint expiry uses that validator plus calendar-date validation; test invalid day, basic/week-date spelling, expiry yesterday/today/tomorrow.
- Optional wildcard text: constraint `gate/repository/branch`. Absent and `""` mean wildcard; null and every non-string error. Each field has a control and invalid wiring case. Required `reason` and `owner` never inherit this default.
- Required arrays: resolved plan `cells`, gh's top-level array, and each validator-owned collection read by orchestration. Absent, null, whole-field wrong type, mixed-type elements, and all-wrong elements error; `[]` explicitly means no cells/PRs. Cell elements require `package/environment/gate/execution/state`; optional `state` is allowed absent only in the constraint reader's legacy minimal plan view, matching Python `read_plan`, and is never synthesized as `prohibited`. In that minimal view, absence means no prohibited-state override, while present null/wrong type errors. Validate the current resolved document fully enough for rendering and identity binding before gating; never turn an unreadable `cells` into an empty list.
- PR `number`: required positive integer, excluding booleans, null, floats, strings, and zero. Unknown gh fields are tolerated because gh owns this document; constraint unknown fields error. Unknown plan fields are retained/tolerated for additive schema compatibility.
- Mode: absence and empty environment string mean `auto`, following shell `${...:-auto}`. `off` is deprecated scope-only. All other unknown values error before work. Selector absence/whitespace means no override.

Python evidence formats are not ported: verification and recording retain their public subprocess error boundaries; do not duplicate their field matrices in Rust. Add CLI wiring only once per owned JSON loader: one positive control and malformed output with a located diagnostic and no launched gate. Constraint parity fixtures cover repository URL variants, optional field semantics, expiry, recursive stores, duplicate keys, malformed documents, and every executing/reused/prohibited outcome. Both public runners must agree.

Before completing the matrix, search changed parsers for `#[serde(default)]`, `Option<T>`, `filter_map`, `as_str`, `unwrap_or_default`, and `.ok()`. Remove silent load-bearing fallbacks; document display-only uses. A finding records the other shapes of its validator and other fields wired to it in the log, avoiding isolated-cell repair.

Validation checkpoint: `cargo check --color=never -p repo-deps --no-default-features --features local-tools --bin ci-push`; focused parser tests through `just _test repo-deps <filter>`. No runtime spike is needed: remaining questions are API and control-flow work already settled here.

## Phase 2 — Implement Review Adapters

### Wave 2

- [ ] **Manage processes**
  - Own `scripts/ci-push/process.rs` and `python.rs`: implement supervised native commands, Python probes, signals/Job Objects, bounded probe capture, null stdin, sanitized child environment, and cleanup ownership.
  - Add a compiled `ci-push-test-tool` fixture executable that dispatches by executable name or explicit scenario argument. It supplies gh/just/python/cargo stubs, argv/environment/stdin recording, child/grandchild readiness, failures, and staged reports. Copy the native executable with `.exe` on Windows rather than creating shell scripts or symlinks.
  - Every wait has a deadline and readiness condition. No fixed sleeps or inherited undrained pipes. Fixture-only launch failures must not leak to real tools on PATH.
- [ ] **Adapt repositories**
  - Own `scripts/ci-push/git.rs` and `worktree.rs`: sniff discovery, gix reads/diffs, explicit remote transport boundary, git2 runtime worktree and scope-note writes, cleanup guards.
  - Verify renames, copies, deletion/type changes, spaces, Unicode, newline filenames on Unix, detached/linked checkouts, current versus advertised remote tips, missing objects, and ancestor checks. Use CLI parity solely as a test oracle; no test contacts user remotes.
  - Release all repository handles before removal on Windows. Cleanup reports only invocation-owned paths and refs.
- [ ] **Validate constraints**
  - Own `scripts/ci-push/constraints.rs`, `scripts/ci/constraints.py`, `scripts/ci/test_constraints.py`, and shared constraint parity cases. Implement matching and both readers' malformed-input correction from the rules and matrix.
  - Keep the Python command surface used by ci-local/planner. Use an explicit date and fixture root in parity tests; errors name the offending JSON path/field.
- [ ] **Share presentation**
  - Own `scripts/ci-plan-render.rs`, the minimal `scripts/ci-plan.rs` extraction, and ci-plan regression tests. Expose the shared renderer and preserve current CLI results.
  - Correct stale feature-tier commentary in the touched module: the current ci-plan module says local-tools although its manifest selects build-tools. Do not change rendering behavior or unrelated comments.

Validation checkpoint: run each task's focused Rust tests once and the Python constraint companion through `scripts/ci/suite_runner.py test_constraints.py` using a supported interpreter. Check ci-plan under build-tools and ci-rollup with no default features; neither may acquire local-tools dependencies. Wave tasks use disjoint source files; add fixture-tool registration and dependency wiring in the Phase 1 owner before concurrency starts.

## Phase 3 — Assemble Orchestration

### Wave 3

- [ ] **Review contexts**
  - Own `scripts/ci-push/review.rs`: resolve every branch/context from the complete update list, materialize committed trees, generate argument files, run planner→verify→apply→render→constraints in order, and capture the first HEAD context independently from receipt success.
  - Keep a complete review ledger; expose it in tests to prove update/context ordering, skipped updates, multi-base reviews, renamed branch constraint identities, and no publication/gates after any review failure.
  - Review every context before proceeding to publication. Python and adapter failures include revision, base, command, and actionable diagnostics.
- [ ] **Bridge interpreter**
  - Own `just/ci-local.just` and its Python companion tests. Replace the identified Python calls with the selected interpreter argv array, retaining standalone defaults.
  - Add a regression with broken python3 and selected fallback that enters the real ci-local recipe using a reviewed plan, and a dirty/override case that enters replanning. Assert helper probes/logs contain the selected interpreter and no restarted python3.
  - Preserve all scope/evidence rules and command arguments. Use native path environment values and separate argv; the NUL-prefix reader must not evaluate shell text and must reject a truncated nonempty record.

### Wave 4

- [ ] **Run gates**
  - Wire the coordinator in `scripts/ci-push/mod.rs` to the completed review and process modules. Resolve auto from the full reviewed ledger; scope-only/off run no gates, strict/warn invoke `just pre-push` with split selectors.
  - Set `BISCUIT_CI_SCOPE_OUT`, `BISCUIT_CI_PLAN_OUT`, `BISCUIT_CI_REPORTS_OUT`, and clean/unmodified nonzero HEAD context `BISCUIT_CI_PLAN_IN` exactly once. Dirty/override/no-HEAD-context feedback replans locally.
  - Add before/after HEAD and cleanliness checks and all receipt-withholding reasons; ordinary gate failure retains its status, warn maps only ordinary failure to zero, and cancellation never reaches the success path.
- [ ] **Publish evidence**
  - Follow gate wiring sequentially, owning `scripts/ci-push/evidence.rs` and coordinator publication integration. Publish eligible scope only after successful full review; record validation only after retaining completed reports.
  - Preserve scope/validation note refs, retry unions, and Python publication status meanings (including status 4 for local evidence retained but unpublished). Inject copy/record/push failures and assert durable reports survive.
  - Keep staging directories after retention failure and print their paths. Never publish validation for changed HEAD, initial/final dirtiness, interrupted/incomplete execution, overrides, or gates unrelated to outgoing HEAD.

Wave 4 is deliberately sequential: publication uses the gate outcome and eligibility state and must not race gate execution.

Validation checkpoint: run focused coordinator tests through canonical L1. Prove a later-context review failure causes zero note writes and zero gates; a complete strict failure publishes diagnostic evidence then returns failure; an interrupted warn run blocks without validation. Check all robustness controls and negative rows at the public `run` boundary.

## Phase 4 — Cut Over and Prove Parity

### Wave 5

- [ ] **Port parity**
  - Complete every log row with a concrete Rust test and preserve assertions crossing the real boundary exercised by the original case. Group fast in-process decision matrices separately from hermetic end-to-end evidence tests; do not replace receipt round trips with command-log assertions.
  - Add malformed/CRLF/empty input, missing/broken/old/timed-out Python candidates, all interpreters missing, helper fallback propagation, complete multi-update base changes, nonancestor/all-zero bases, post-gate HEAD changes, cancellation, grandchild cleanup, report retention, and publication failure cases.
  - Test compiled stubs with PATH built via `join_paths`; isolate HOME/USERPROFILE, evidence and constraint roots, Git identity, default branch, config, gh settings, and remotes per fixture. Null stdin on review children must leave the full Git update list intact.
- [ ] **Exercise launcher**
  - Own hook/dispatcher integration tests and replace `.githooks/pre-push` with the thin launcher. Retain `.githooks/pre-push-dispatcher` unchanged unless its tests demonstrate a forwarding defect; forwarding fixes remain confined to cwd/argv/stdin/status.
  - Invoke actual Git push against bare fixture remotes with the dispatcher installed. Use the compiled cargo stub to assert exact package/feature/binary/`--` selectors, stdin and remote args, freshness delegation on successive pushes, Cargo failure blocking, and invoking checkout selection across linked worktrees. This avoids nested Cargo builds per L1 test.
  - Native Windows runs the same actual Git fixture through non-interactive Git Bash and PowerShell entry commands. Test a path with spaces and Unicode, fallback Python, auto and strict. Use OS signal injection and process readiness for cancellation, without opening/focusing windows. Assert worktree/admin cleanup and no surviving fixture child/grandchild.
  - Use `biscuit_test_harness::bin_exe!` and `manifest_dir!` at runtime. Fixture executables must be archive-addressable, not compile-time absolute paths.

### Wave 6

- [ ] **Retire shell**
  - After every parity row has a passing Rust counterpart, delete `.githooks/tests/test-pre-push.sh`, `.githooks/tests/test-pre-push-dispatcher.sh`, `.githooks/tests/report-pre-push.py`, and `.github/workflows/pre-push-hook-tests.yml`.
  - Update root aliases to canonical filtered L1 execution; ensure each alias runs its intended module and reports a nonzero test count. Retain all three alias names for developer use.
  - Test the planner's existing test-input indexing against both hook paths and dispatcher: each selects the relevant repo-deps tests. Use literal runtime repository reads; add no broad infrastructure trigger or duplicate workflow.

Validation checkpoint: complete all parity log rows, run repo-deps L1 once after integration, and run affected planner/ci-local/constraint companion suites. Inspect plan selection for both hook paths and ensure only their owning narrowed test cells are added. No full-workspace CI run is needed to answer this ownership question.

## Phase 5 — Validate Platforms and Document Behavior

### Wave 7

- [ ] **Update documentation**
  - Update `docs/topics/ci-cd.md`, `docs/testing-strategy.md`, and README overview links with the thin launcher, local-tools build, Python requirement/fallback, all-mode review, auto/strict behavior, temporary committed trees, retained reports, cancellation limits, and canonical Rust test commands.
  - Update `.claude/skills/rust-devops/ci-cd.md`, its index where needed, and `.claude/skills/os/windows.md` plus the os index where needed. Remove retired Bash suite/workflow instructions and describe both native Windows launch paths.
  - Document constraint optional-field and invalid-shape rules and the unversioned validation correction in `docs/cicd/schema-versions.md`. Docs must describe behavior without referencing this dated feature. Update `docs/dependencies.md`; README remains an overview.
- [ ] **Prove platforms**
  - Run `just test repo-deps` and `just _lint repo-deps` on the local host, plus only affected Python companion suites. Never run cargo fmt; make necessary formatting corrections manually and allow the lint recipe's check-only formatter.
  - Run `just cross-check repo-deps --os linux`, `just cross-check repo-deps --os windows`, and `just cross-check repo-deps --os wsl` using the configured BUILD_* destinations; local macOS supplies the fourth environment. Preserve canonical archive mode for WSL2 and do not add feature flags that switch it to native execution. Follow os/build-hosts.md for non-interactive transport.
  - Native Windows acceptance tests must execute both PowerShell and Git Bash cases, not merely compile cfg(windows). Missing supported shells are a failed prerequisite on that native acceptance host, not skipped proof. Record exact commands, environment, test counts, statuses, and retained report locations in the implementation log. Unavailable rigs are reported as unproved acceptance evidence, never marked passing or converted to a human gate; continue all independent checks and report incomplete acceptance in the final result.
  - Correct platform defects in the same implementation, add newly learned OS facts to the os skill, and rerun only failed/changed slices. Reuse qualifying existing evidence; never treat a requested lack of duplicate passing tests as an execution ban.

Validation checkpoint: all required platform evidence is passing, log parity mapping is complete, and docs/skills contain no active references to the retired suite/workflow. Verify no-feature ci-rollup and build-tools ci-plan/ci-build still compile independently of ci-push. Verify no new CI environment cells or changed evidence schemas. Review the final diff for unsolicited refactors, generated bytecode, compile-time fixture paths, leaked fixture processes, and changes outside the starting worktree.

- [ ] **Close implementation**
  - Update the implementation log with completed task results, malformed-constraint and first-context departures, cleanup/diagnostic examples, platform evidence, and any acceptance evidence that could not be produced. Keep durable reports outside temporary review trees.
  - Mark only verified tasks complete. Report implementation complete and ready for review only when all definition-of-done items are proved; otherwise report the exact remaining failures without claiming completion. Leave the feature active for the author's review cycle.