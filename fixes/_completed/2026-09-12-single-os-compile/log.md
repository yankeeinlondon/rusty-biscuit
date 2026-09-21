---
fix: 2026-09-12-single-os-compile
implementation_2: "2026-09-15T04:06:12-07:00"
deferred_perf_measurement: true
---

# Implementation Log — 2026-09-12-single-os-compile

## Implementation of Review Findings #2

> **started at:** 2026-09-15T04:06:12-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-single-os/fixes/2026-09-12-single-os-compile/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review findings under implementation:
        - **High** — Source identity rejects pull-request and pre-push archive orchestration
        - **High** — Real package archives still depend on the producer's absolute checkout path
        - **High** — The required performance acceptance gate has no usable baseline
        - **Medium** — Owner measurement publication is not schema-tested
- impacted package area is `repository-ci` (per `spec.md` frontmatter `area:`), so test and lint
  runs are scoped to the CI tooling suites: `scripts/` (Rust `ci-build`/`ci-rollup`/`ci-plan`
  binaries), `scripts/ci/` (Python), `scripts/ci/artifacts/` (Node), and
  `tools/test-toolkit/tests/ci_workflow_contracts.rs`
- starting the work on 'Source identity rejects pull-request and pre-push archive orchestration' at 04:07:12
- **revision identity chosen: the pull-request HEAD SHA, published through the plan**
        - the plan already labels itself `github.event.pull_request.head.sha`, and that commit is
          immutable; GitHub recreates the merge commit, so it is not a revision an archive can be
          bound to across jobs
        - `ci.yml` now resolves it ONCE, in a workflow-level `env.TESTED_REVISION`
          (`${{ github.event.pull_request.head.sha || github.sha }}`), used by the only two jobs
          that run before a plan exists (`validation`, `scope`)
        - the `scope` job gained a `head` output read back from the WRITTEN `resolved-plan.json`
          (`jq -r '.head'`). Every later job in `ci.yml` checks out `needs.scope.outputs.head`, so
          the document the producer is bound to is the document every consumer follows — not a
          second evaluation of the event expression
        - `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml` take a REQUIRED `tested-revision` input
          threaded down the chain; `biscuit-tui-windows-captured-stdout.yml` takes an optional one
          (it also has a standalone `workflow_dispatch` trigger, where empty means the default ref)
        - 14 `actions/checkout` steps pinned in total. `_area-ci.yml`'s accepted-gap `HEAD_SHA` now
          reads `inputs.tested-revision` too, so the check-run SHA and the checkout cannot diverge
        - discovery: `env` context is NOT available in job-level `with:` for a reusable-workflow
          call, which is part of why the plan output is the downstream source rather than the env
        - discovery (checked empirically): `git clone` of a detached-HEAD source with no branches
          still follows HEAD, so `_wsl-ci.yml`'s guest clone needed no change once the runner
          checkout became detached at the tested revision
- **cross-check now ships one immutable revision instead of a base + patch**
        - `scripts/cross-check.sh` built the plan with the local `HEAD` but checked the host out at
          `base_sha` and applied a patch, so `ci-build produce` had to refuse either the wrong HEAD,
          the dirty tree, or both
        - it now writes a throwaway commit LOCALLY (scratch `GIT_INDEX_FILE`, `git read-tree` the
          base then `git add --all`, `git write-tree`, `git commit-tree`) and ships that commit as a
          `git bundle` whose only prerequisite is the base every host already fetches
        - shipping the OBJECT rather than re-deriving it per host was chosen deliberately: a remote
          `git apply` + `commit-tree` would have to reproduce the same tree hash on Linux, macOS and
          native Windows, which CRLF translation and clean filters make a gamble. A bundle makes the
          four hosts run the same commit id by construction
        - non-interactive and non-invasive as required: `-c commit.gpgsign=false` plus explicit
          `GIT_AUTHOR_*`/`GIT_COMMITTER_*`, a scratch index (the real index, branch, and the SHARED
          stash stack are never touched), and the one temporary ref (`refs/cross-check/<run_id>`,
          needed because `git bundle` requires a ref) is deleted in the EXIT trap
        - when the working tree already matches the base, no commit and no bundle are made and the
          base IS the tested revision
        - the Unix prelude and the PowerShell body both `git fetch <bundle>` then
          `git checkout --detach <revision>`; neither applies a patch, and the second checkout
          (`git worktree add`) names the same revision explicitly instead of `HEAD`
        - the WSL receipt path is unaffected: qualification still compares the remote's tested TREE
          to the outgoing head's tree, and the remote is now clean by construction
- **tests added — all of them execute the boundary, none just read generated text**
        - `scripts/ci/test_cross_check.py`: new `CrossCheckSourceIdentityTests` builds a real git
          repo + bare origin, leaves it dirty / ahead-and-dirty / clean, runs the real script, then
          REPLAYS the shipped remote prelude against a fresh clone and asserts the host's `HEAD`
          equals `plan.head` and `git status --porcelain` is empty — i.e. exactly the two conditions
          `source_tree()` enforces. Also asserts the local content arrives as tracked content, that
          no host is ever asked to `git apply`, and that the developer's branch, index, stash stack,
          and refs are untouched afterwards
        - the fixture planner stub now honors `--head`, so `plan.head` is the revision the script
          actually shipped; a stub with a fixed head could not tell a correct orchestration from a
          broken one
        - `scripts/ci-build-archive-tests.rs`:
          `produce_accepts_the_planned_head_and_refuses_the_merge_revision` builds a real
          merge-commit-vs-head divergence in the Cargo fixture workspace and drives the shipped
          `ci-build produce`: refused with `build-source-mismatch` (naming the planned commit, with
          nothing compiled) at the merge revision, and producing a manifest whose `source_commit` is
          the head at the pinned one
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs`:
          `every_ci_checkout_pins_the_one_revision_the_plan_names` enumerates every
          `actions/checkout` in the four chain workflows and requires each to pin the allowed
          expression for its position, plus the single definition site and the plan-sourced scope
          output; `every_called_workflow_requires_the_tested_revision` requires the input and its
          pass-through at all three call sites. Mutation-checked: blanking one `ref:` fails it with
          the offending workflow and job named
- **docs updated where behavior changed**: `.github/ci/README.md` (new "One revision, end to end"
  paragraph beside the generation-3 source rules), `.claude/skills/rust-devops/ci-cd.md`, and
  `.claude/skills/os/build-hosts.md` (the cross-check sync model and its banner line)
- **drift fixed in the same change**: `the_gap_publisher_is_the_only_job_holding_checks_write`
  asserted the literal `github.event.pull_request.head.sha || github.sha` in `_area-ci.yml`'s
  accepted-gap job; that job now reads `inputs.tested-revision` (same value, one authority), so the
  contract was updated to require the input and keep its reason
- **verification** — every command below run from the worktree root:
        - `actionlint .github/workflows/*.yml` — pass
        - `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` — 596 pass (was 590)
        - `node --test scripts/ci/artifacts/publish.test.cjs` — 1 pass
        - `cargo nextest run … --features build-tools --bin ci-build` — 115 pass (was 114)
        - `cargo nextest run … --bin ci-rollup` — 195 pass; `--bin ci-plan` — 13 pass
        - `cargo nextest run -p test-toolkit --test ci_workflow_contracts` — 118 pass (was 116)
        - `just ci-local --plan` — pass, exit 0
        - `cargo clippy` over `scripts/` (`--features build-tools --bin ci-build --all-targets`) and
          `-p test-toolkit --tests`, both `-D warnings` — clean; `shellcheck -S warning
          scripts/cross-check.sh` — clean
        - the `--features rollup` / `--features plan` spellings in the verification list do not
          exist in `scripts/Cargo.toml`; used the workflows' own spellings
          (`--no-default-features --bin ci-rollup`, default features `--bin ci-plan`)
        - host-cache note: the shared target cache holds read-only `.rmeta` blobs, and the host's
          `kache` rustc wrapper served a CACHED fixture dylib whose baked absolute path pointed at
          another test's scratch directory, failing
          `a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status`. With
          `RUSTC_WRAPPER=""` (which every CI job sets explicitly) and an isolated
          `CARGO_TARGET_DIR`, all 115 pass. Host condition, not an implementation finding
- work completed for 'Source identity rejects pull-request and pre-push archive orchestration' at 04:36:08
- starting the work on 'Real package archives still depend on the producer's absolute checkout path' at 04:37:37
        - **Part A — corpus migration to `biscuit_test_harness::manifest_dir!()`.** 164 compile-time
          `env!("CARGO_MANIFEST_DIR")` sites at the start; every `tests/`, `benches/`, and
          `#[cfg(test)]` `src/` site now resolves at run time. Migration was fanned out across three
          parallel sub-agents by package area (claudine; darkmatter; the remaining ten areas), with
          `tools/test-toolkit` and `scripts/` done in the parent session.
        - discovery: the review brief's classification of two `src/` sites was wrong, and both were
          checked rather than assumed —
          `darkmatter/lib/src/markdown/compose/context/repository_scope.rs:196` IS inside
          `#[cfg(test)] mod tests` (migrated), and `scripts/ci/fixtures/archive-portability/crate/src/lib.rs`
          is the POSITIVE half of the portability fixture (it already prefers the runtime variable),
          not a negative control
        - discovery: there are TWO `concat!(env!("CARGO_MANIFEST_DIR"), …)` sites, not one. The second,
          `darkmatter/cli/src/commands/schema/about.rs:734`, is an `include_str!` — a compile-time
          byte embed that opens no path at run time, so it relocates correctly as written and is
          allow-listed rather than migrated
        - `tools/test-toolkit` needed `biscuit-test-harness` as a REGULAR dependency, not a dev one:
          `evidence::workspace_root()` is library code its dependents call. No cycle (the harness
          graph is libc/serde_json/tempfile/unicode-width), and its doc comment was rewritten — the
          old one justified the compile-time value it no longer uses
        - `scripts/` is deliberately NOT migrated and is excluded from the guard: it is a separate
          Cargo workspace (its own `[workspace]` table), so CI builds and runs its tests in place
          and never archives them
        - **Part B — `tools/test-toolkit/tests/archive_path_guard.rs`.** Scans the real repository
          for `env!("CARGO_MANIFEST_DIR")`, `env!("CARGO_BIN_EXE_…")`, and `"/home/runner/work/` path
          literals; names the offending `file:line` and the remedy. Skips `target`/`.git`/`node_modules`/
          `.gitnexus`/`scripts`/`examples`/`fuzz` and `build.rs`, each with its reason in the source.
          A second test fails on a stale allow-list entry, so the list burns down
        - the guard's absolute-path rule was deliberately narrowed to `/home/runner/work/` after a
          first draft flagged 20+ false positives: `"/Users/ken/repo/x.md"` and friends are synthetic
          INPUTS to pure path functions, not lookups, and no textual rule separates the two. A baked
          producer path that slips past the guard is caught instead by Part C
        - mutation-checked: a probe file with all three forms was reported by `file:line` and the
          run failed; a form inside a `//` comment was correctly ignored; probe reverted
        - **Part C — `scripts/ci-build-archive-tests.rs`, two `slow_` real-package fixtures.** Each
          copies the live WORKING tree (not `HEAD` — an uncommitted migration is exactly what has to
          be proven) to a throwaway producer checkout, `git init`s it because `source_tree()` refuses
          a dirty workspace, drives the shipped `ci-build produce`, copies the source to a second
          address, then DELETES the producer checkout outright and renames its target directory away
          before running the archived L1 suite through `--workspace-remap`
        - `assets/` (440 MB of the repo's 641 MB tracked bytes, read by no package under test) is
          excluded from the copy; without that the two copies dominated the fixture's wall clock
        - packages chosen: `test-toolkit` (56-crate graph, and its suite reads `.config/nextest.toml`,
          `just/devops.just`, and `.github/workflows/` out of the checkout, so nothing in it can pass
          from the wrong address) and `biscuit-file` (different area; its suite walks committed corpus
          DIRECTORIES rather than single files)
        - **Part D — `_wsl-ci.yml`.** New job-level `GUEST_ROOT: /home/biscuit/checkout` replaces all
          six `steps.manifest.outputs.producer_workspace` interpolations; the step output is deleted;
          the long comment justifying the workaround is replaced by one stating the new contract.
          `producer_workspace` STAYS in the build manifest — it is inside the realized digest and is
          asserted by `the_manifest_records_the_workspace_the_producer_compiled_at` — but nothing
          executes by it any more
        - `ci_workflow_contracts.rs` now asserts the guest clones to `env.GUEST_ROOT` AND that the
          workflow contains no `producer_workspace` at all, so reintroducing the derivation fails
        - docs updated for drift: `.claude/skills/os/wsl.md`, `.claude/skills/rust-devops/ci-cd.md`,
          `.claude/skills/rust-testing/SKILL.md`, `.github/ci/README.md`, and Task 6.4's note in
          `plan.md` (which had recorded the sweep as explicitly NOT done)
        - Part C findings while building the fixture, all real and all fixed:
            - the consumer checkout must be a `git clone` of the producer, not a file copy —
              `biscuit-file`'s `find_git_root_inside_repo` asks whether its ambient directory is
              inside a repository, and a `.git`-less copy failed it. A real consumer (the WSL2
              guest) clones, so the copy was the unfaithful half
            - `git gc --auto` fires after committing the repository's ~11k files and repacks the
              loose objects out from under a concurrent `git clone`, producing an intermittent
              `failed to copy file to .git/objects/…: No such file or directory`. Every fixture
              `git` invocation now carries `-c gc.auto=0`
            - two relocations running concurrently under nextest exhausted the temp filesystem and
              took `two_tier_configurations_compile_the_same_package_twice_and_are_reported_apart`
              down with them (that test passes in isolation — contention, not a regression). Merged
              into ONE test that drops each `Scratch` before the next, and the producer's target
              directory is now DELETED rather than renamed aside, which is both a stronger claim and
              what returns the dependency build to the disk before the archived suite starts
            - cross-OS: Git marks every object file read-only and Windows refuses to delete one, so
              `Scratch::drop` and the fixture's own teardown clear the bit first (`#[cfg(windows)]`;
              a no-op elsewhere rather than a shim that loosens Unix permissions). Verified with
              `cargo check --target x86_64-pc-windows-gnu --all-targets` over `scripts/` — clean.
              Scratch paths were also shortened (`reloc-{pkg}`, `producer`, `elsewhere/checkout`)
              to stay clear of the Windows 260-character limit
        - per-area migration counts (145 code sites, from 164 originally present):
            - `claudine` 57 — lib 20, cli 26, gen 11; one new `[dev-dependencies]` entry
              (`claudine/lib`); seven `claudine/gen` helpers returning `&'static Path` kept their
              signatures behind a function-local `OnceLock<PathBuf>` rather than churning callers
            - `darkmatter` 52 — lib 31 (incl. 6 in `benches/`), cli 11, dmls 9, zed-dmls-cli 1;
              `darkmatter/cli` gained the unconditional `[dev-dependencies]` entry alongside its
              optional feature-gated one (a pattern `schematic/gen` and `tree-hugger/cli` already use)
            - ten remaining areas 32 — biscuit-file 3, biscuit-icon 1, biscuit-terminal 5,
              tree-hugger 5, sniff 5, schematic 7, research 2, playa 1, unchained-ai 2, visualizer 1;
              12 manifests gained the dev-dependency
            - `tools/test-toolkit` 4 — three test binaries plus `evidence::workspace_root()`
        - two sites needed a REGULAR `[dependencies]` entry because non-test library code resolves
          repository source that a test then exercises, and each was proven rather than assumed:
          `darkmatter/dmls/zed-dmls-cli`'s `checked_in_extension_dir()` (reached only through the
          `stage` subcommand, which `tests/cli.rs` runs; setting `CARGO_MANIFEST_DIR` to a bogus path
          makes that test fail before the change and pass after) and `unchained-ai/gen`'s
          `catalog_generated_at()` (imported by `tests/catalog_drift.rs`, three of its five tests)
        - allow-list, five entries, each a target that is never archive-executed: the harness's own
          `bin_exe.rs`; `darkmatter/cli`'s `include_str!`; `biscuit-icon`'s `populate_assets` bin;
          and `unchained-ai/gen`'s two generator bins (no test spawns either — verified by searching
          every `*.rs`, `justfile`, and `*.just` for references)
        - verification, all real:
            - `actionlint .github/workflows/*.yml` — clean
            - `python3 -m unittest discover -s scripts/ci` — 596 pass
            - `cargo nextest run -p test-toolkit` — 196 pass (was 194; the guard adds 2)
            - `cargo nextest run … --features build-tools --bin ci-build` — 116 pass (was 115)
            - `just ci-local --plan` — exit 0
            - `cargo clippy -D warnings` — `-p test-toolkit --all-targets` and `scripts/`
              `--bin ci-build --all-targets`, both clean
            - `cargo check --target x86_64-pc-windows-gnu --all-targets` over `scripts/` — clean
            - per area `just test` / `just lint`, all exit 0: claudine 7017 pass; darkmatter 7803
              pass; biscuit-file 813; biscuit-icon 228; biscuit-terminal 3261; tree-hugger 585;
              sniff 2620; schematic 1699; research 606; playa 195; unchained-ai 367; visualizer 2
            - no pre-existing failure was encountered in any area, so no attribution question arose
        - the full `ci-build` suite reported `1 leaky` once under load; the relocation fixture is NOT
          leaky in isolation and the mark was not attributable to a named test, so it is recorded
          rather than claimed as clean or as mine. `scripts/` has no `.config/nextest.toml`, so the
          default 100 ms window and non-failing leak policy apply there
        - left undone, deliberately: `darkmatter/dmls/zed-dmls-cli/tests/cli.rs` locates its binary
          with `assert_cmd`'s `cargo_bin("zed-dmls")` rather than `bin_exe!()`. That is the
          `CARGO_BIN_EXE` hazard in a different spelling, it is out of this finding's scope, and the
          area has no `spawn_site_guard.rs` to catch it — flagging it rather than expanding scope
- work completed for 'Real package archives still depend on the producer's absolute checkout path' at 05:37:50
- starting the work on 'The required performance acceptance gate has no usable baseline' at 05:38:45
- Part A — chose the review's FIRST option, an instrumentation-only pre-cutover revision, not the
  replacement comparison. The evidence that a clean split was extractable, gathered before deciding:
        - no Phase-1-only state was ever committed — `git log 8aa105e7c..HEAD` is one planning
          commit and the whole implementation is an uncommitted working tree, so nothing could be
          checked out
        - but the split does not need history. Plan `source_files_during_phase_1` already names the
          Phase 1 surface, and the two `just` recipes that drive the instrument
          (`_ci_build_counter`, `_ci_build_report`) read only a label and a counter directory — no
          plan, no build record, no archive — so they graft onto the merge base unchanged
        - the merge base's five gate steps are uniform, uniquely anchored, and unchanged by the
          cutover's own instrumentation (`cargo check …`, `L1 tests`, `Lint`, `L2 tests`,
          `Browser tests`, plus `_wsl-ci.yml`'s `Build the nextest archive`)
        - so a replacement comparison would have been the easier path and was not needed
- built `scripts/ci/build_baseline_revision.py`: reads the merge base, overlays the counter tool
  byte for byte, appends the two recipes, threads `measure-compiler-work` through all four
  workflows, brackets each gate command, and writes an unreferenced commit via `git commit-tree`
  with fixed author/committer/dates. No branch moved, no index touched, nothing stashed, nothing
  signed (`-c commit.gpgsign=false`), no TTY
        - constructed revision: `224d1a6620b8f0e723929d56cd8c9e37364a58a8`, parented on
          `8aa105e7c`. Reproduced identically on repeated runs
        - the timing is taken by SIBLING steps rather than by rewriting each gate's `run:` body.
          Wrapping a body in `{ … } || status=$?` disables `set -e` inside the group, which would
          have silently changed what the baseline measures. The wrapper reaches the command through
          the step's `env:` map instead, so every measured command is byte-identical to the base
        - cutover-absence is proven structurally, not by a keyword list: `verify_additive` refuses
          any construction that removes a base line other than `workflow_dispatch: {}`, which has to
          grow an input. A tree that deletes nothing from the scheduling surface cannot schedule
          differently. `git diff 8aa105e7c <rev>` confirms exactly one removed line
        - `--update-documents` rewrites every documented hash in one step, so the pin below is a
          five-second fix rather than a treadmill when the counter tool changes
- `scripts/ci/test_build_baseline_revision.py`, 11 tests. Each builds a REAL temporary repository
  seeded from the actual merge base — so a moved anchor is caught, not mocked — and writes its
  objects only there. The published-id test uses `git clone --local --shared` so the real
  pre-cutover commit is the parent (the id is the documented one) without writing into this repo's
  object database. Covered: instrumentation present per measured job; cutover markers absent;
  additivity; measured commands byte-identical to the base; reproducible id; a moved anchor refuses
  rather than guesses; a non-additive construction refuses; HEAD/index/refs untouched; recipe
  extraction stops before the next recipe's comment block; document refresh leaves the base
  revision alone; and the documented hash equals the constructed one
- validated the constructed revision itself, not just the script: extracted it to a temp directory
  and ran `actionlint .github/workflows/*.yml` (clean) and `just --list` (parses). Three real defects
  were found and fixed that way — a `.format()` eating `${{ }}` braces, a shellcheck-hostile
  multi-line jq filter, and a recipe extractor that swallowed its neighbour's doc comment
- Part B — `rollout-2026-09-12.md` said "The first hosted run with `measure-compiler-work: true`
  after this branch merges is the action that closes both tasks", contradicting plan Task 7.5.
  Corrected in the ledger, with the plan's semantics winning: neither a first run nor merging closes
  either task. The "What a reader should take from this" summary carried a weaker form of the same
  error (six dispatches per environment, omitting the second revision and the ruling) and was
  corrected too. Swept both documents for other instances; `implementation-notes.md:334`'s "closes
  the Linux half on the first hosted run" is about `ci-build`'s own test execution, not AC8, and was
  left alone
- Part C — `baseline-2026-09-12.md` now names the constructed revision, carries the push and
  dispatch commands, defines cold by cache deletion and warm by cache restore, and defines
  "three consecutive green runs per environment" precisely (ordered by `run_number` on one revision,
  every cell of that environment `success`, a red run restarts the count). Its observation rows are
  still EMPTY and honestly so
- created `fixes/2026-09-12-single-os-compile/deferred-performance-measurements.md` (`kind: deferral`):
  what is deferred, why this session could not do it, what was built so it is now executable, the
  copy-pasteable procedure, the acceptance rule (15% band, three consecutive green per environment,
  cold/warm matched identities, total runner compute vs critical path reported apart, compiler
  invocations as the primary column), and what closes each of Tasks 1.6, 7.4, 7.5. Maps back to
  Finding 3 of `review-2.md`
- plan Tasks 1.6, 7.4, 7.5 carry Review 2 correction notes and all three remain UNCHECKED. Phase 7
  frontmatter lists the two new scripts, the new deferral document, and the updated baseline
- verification, all run from this worktree:
        - `actionlint .github/workflows/*.yml` — clean
        - `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` — 607 pass (596 + 11 new)
        - `python3 -m py_compile scripts/ci/*.py` — clean; `git diff --check` — clean
        - `cargo nextest run -p test-toolkit --no-fail-fast` — 196 pass, 2 skipped
        - `just ci-local --plan` — exit 0, macOS/Linux/Windows/WSL2 cells resolved
        - `cargo nextest run … --features build-tools --bin ci-build --no-fail-fast` — 116 pass ONLY
          with `RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/ac8-isolated-target`. Against the shared
          `scripts/target` it reported 99 pass / 17 fail, every failure in `archive::tests` and every
          one a missing artifact under a temp fixture target. That is the documented host cache
          condition, not an implementation finding: this finding changed no Rust at all
        - no Clippy run: nothing Rust was touched. Nothing shell was touched either, so no shellcheck
- what remains deferred, unchanged by any of the above: the twelve hosted dispatches and the ruling.
  Nothing here measures anything, and nothing here should be read as evidence that CI got faster
- work completed for 'The required performance acceptance gate has no usable baseline' at 06:06:37
- starting the work on 'Owner measurement publication is not schema-tested' at 06:07:51
- the real schema, read out of the Rust producer and not out of the old JS fixture:
        - `manifest.compiler_work` is `serde_json::to_value(ci_build::summarize(&events))` —
          `ci-build-archive.rs:1146-1151` — so it is `Report` (`ci-build.rs:334-345`) verbatim:
          `schema_version`, `event_schema_version`, `events_read`, `compiler_invocations`,
          `primary_invocations`, `probes`, `compiler_ms`, `window_ms`, `slices[]`. Each slice
          (`ci-build.rs:310-332`) is `package`, `configuration`, `compiler_invocations`,
          `primary_invocations`, `probes`, `distinct_crates`, `compiler_ms`, `window_ms`,
          `failed_invocations`. Every field is required; `compiler_work` itself is the only optional
          part and is skipped entirely on an unmeasured run
        - the old fixture's `compiler_work: {invocations: 1}` matched nothing in that struct — the
          publisher's `|| 0` fallbacks turned it into a silent zero
        - `owner-timing.json` is written by `scripts/ci/produce-owner.sh:18` and is exactly
          `{"produce_wall_seconds": <integer>}`; `publish.cjs:21` `Object.assign`s it onto the owner
          aggregate, so a lost owner leg simply leaves the key absent
- fixture approach chosen: generated through the Rust producer, as a committed golden.
  `scripts/fixtures/compiler-work/publisher-documents.json` is written by
  `ci-build-tests.rs::the_javascript_publisher_reads_a_fixture_generated_from_this_report`, which
  serializes `summarize()` over three synthetic event sets and compares byte for byte against the
  committed file (`BLESS_COMPILER_WORK_FIXTURE=1` regenerates). `summarize` is the same call the
  producer makes, so this is the producer's own serialization and not a re-description of it; a
  renamed or dropped `Report` field fails in Rust before the Node test can assert a shape the
  producer never writes. Driving the real `ci-build produce` path instead was rejected: it needs
  real Cargo and a real archive, and its counts are host- and timing-dependent, so it cannot be a
  golden
        - the three documents are `alpha` (3 invocations, 260 compiler_ms, 1 primary, 1 probe, 1
          slice), `beta` (2 invocations, 90 compiler_ms, 2 slices across L1/L2), and `probes_only`
          (2 probe events, 0 invocations, 0 compiler_ms — a real zero the producer can actually emit)
        - the file had to live outside `scripts/ci/`: `build_baseline_revision.py` copies
          `ci-build-tests.rs` verbatim onto the pre-cutover revision, and its `verify_no_cutover`
          refuses any construction that introduces a `scripts/ci/` path. Added
          `scripts/fixtures/compiler-work/publisher-documents.json` to `CARRIER_PATHS` so the carried
          instrument can still pass its own suite, and re-ran
          `python3 scripts/ci/build_baseline_revision.py --update-documents` — the documented AC8
          revision moved to `fb8681b86902bc3bfe67dfee76bb35727b611eee` in `baseline-2026-09-12.md`,
          `deferred-performance-measurements.md`, and `plan.md`. Touching a carrier file rotates that
          hash; that is the script's designed behavior, not drift
- assertions added to `scripts/ci/artifacts/publish.test.cjs` (3 tests, was 1):
        - the existing partial-owner/isolation/every-key test is unchanged except that its manifests
          now carry the real `compiler_work` document instead of `{invocations: 1}`
        - new `the owner aggregate is the sum of its records' producer-written compiler work`: parses
          the uploaded `<artifact>.compiler-work.json` and asserts `record` deep-equals the manifest's
          document verbatim; `owner` deep-equals `{producer, requested_records: 3,
          measured_records: 2, compiler_invocations: alpha+beta, compiler_ms: alpha+beta,
          produce_wall_seconds: 41}` with both sums computed from the fixture rather than hardcoded;
          the second measured record republishes an identical owner block; and the third record,
          whose manifest has no `compiler_work` at all, publishes no measurement — which is what pins
          `measured_records` below `requested_records`
        - new `a measurement that counts nothing cannot pass for compile-once evidence`: a shared
          `assertOwnerMeasuredSomething()` guard rejects `measured_records`, `compiler_invocations`,
          or `compiler_ms` of zero and a missing `produce_wall_seconds`. Four scenarios prove it
          fires — the pre-schema `{invocations: 1}` name, probe-only compilation (real schema, zero
          invocations), a document with `compiler_ms` deleted (missing duration data), and an absent
          `owner-timing.json`. Each also asserts `measured_records` still counts the record, so a
          present-but-empty document is visible rather than invisible. All four passed the previous
          contract, which only checked that a measurement filename appeared in the upload
        - proof the new assertions bite: temporarily repointing `publish.cjs:16-17` at
          `compiler_work.invocations` / `.ms` turned both new tests red (`# pass 1 # fail 2`) while
          the original test stayed green, then `publish.cjs` was restored byte-identical
          (`git diff --stat` clean)
- rename: `ci_workflow_contracts::the_build_owner_expands_one_leg_per_planned_record` →
  `the_build_owner_expands_one_leg_per_producer`. Its assertions are the producer matrix, the
  projection-sourced runner, and one `produce-owner.sh` invocation with no `--key` — a per-producer
  topology, not a per-record one. A repo-wide grep found exactly one call site (the definition);
  `review-2.md` quotes the old name in prose and was left as the review record it is. The test's doc
  comment claimed it also pinned "the prerequisites each leg installs", which it never asserted —
  drifted comment, corrected to what the body actually checks
- `.claude/skills/rust-devops/ci-cd.md` gained a paragraph on the Rust→JavaScript measurement
  boundary: which `Report` fields `publish.cjs` reads by name, where the golden lives, how to
  regenerate it, and why it is not under `scripts/ci/`
- verification, all from this worktree, Rust runs with `RUSTC_WRAPPER=""` and a private
  `CARGO_TARGET_DIR` under `/tmp` as the poisoned shared `scripts/target` requires:
        - `node --test scripts/ci/artifacts/publish.test.cjs` — 3 pass, 0 fail
        - `node --test scripts/ci/artifacts/` — fails to even start on this host's Node v22.20.0,
          which resolves the directory as a module (`Cannot find module …/scripts/ci/artifacts`).
          Reproduced identically against a throwaway `/tmp` directory, so it is the Node build and
          not this change. `node --test 'scripts/ci/artifacts/*.test.cjs'` — 3 pass
        - `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` — 607 pass. One test,
          `PublishedRevision::test_the_documented_revision_matches_the_construction`, was red until
          `--update-documents` was re-run; it is red by design whenever a carrier file changes
        - `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
          --features build-tools --bin ci-build --no-fail-fast` — 117 pass (was 116), 1 leaky
        - `cargo nextest run -p test-toolkit --no-fail-fast` — 196 pass, 2 skipped
        - `actionlint .github/workflows/*.yml` — clean
        - `just ci-local --plan` — exit 0
        - `cargo clippy --manifest-path scripts/Cargo.toml --no-default-features --features
          build-tools --bin ci-build --all-targets -- -D warnings` — clean;
          `cargo clippy -p test-toolkit --all-targets -- -D warnings` — clean
        - no JavaScript linter exists for `scripts/ci/artifacts/`: no eslint/biome/prettier/oxlint
          config anywhere in the repo and no `just` recipe referencing one. Verified, not assumed
- one thing left unfixed and deliberately out of scope: nothing in `.github/workflows/` or any
  `justfile` ever runs `node --test`, so `publish.test.cjs` is not executed by CI at all. The Rust
  golden half of this contract does run in CI (it is part of the `ci-build` bin suite), so a schema
  drift is still caught — but the publisher's own assertions are local-only. Worth a separate finding
- work completed for 'Owner measurement publication is not schema-tested' at 06:23:15
- follow-on to Finding 4, closing the gap that finding's own log entry left open: the publisher
  suite now runs as a named CI step instead of only where a reader would not look for it
- premise correction found while doing it: the suite was NOT entirely unrun in CI. The contract
  test `every_build_publishes_a_keyed_artifact_and_a_status_for_any_outcome` shelled out to
  `node --test .../publish.test.cjs` mid-assertion, and `ci-tooling` runs that binary, so the
  suite executed in CI under an unrelated test's name. A `node` failure was reported as a
  workflow-contract failure, and the whole `ci_workflow_contracts` binary hard-failed
  (`.expect("node is required …")`) on any host without Node — which `test-toolkit` promotion to
  the Windows or WSL legs would have hit
- what changed:
        - `.github/workflows/ci.yml`: new `ci-tooling` step `Test the owner measurement publisher`
          running `node --test 'scripts/ci/artifacts/*.test.cjs'`, placed after the existing
          `Set up Node` (`actions/setup-node@v4`, `node-version: 22`) so it runs on the pinned
          Node and not the image default. No new Node setup was needed — the leg already pins one
          for the `tools/test-audit` check — and no `npm ci`, since the suite loads `publish.cjs`
          and a fixture and never `@actions/artifact`. Scope needed no change either: the
          `scripts/` prefix in `CI_TOOLING_PREFIXES` already selects this leg for a publisher edit
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs`: new
          `ci_tooling_leg_runs_the_owner_measurement_publisher_suite` asserting the step exists,
          is invoked by quoted glob, and is ordered after `Set up Node`. The `node` shell-out was
          removed from `every_build_publishes_a_keyed_artifact_and_a_status_for_any_outcome`,
          which keeps its workflow-text assertions; the suite is now executed by the workflow and
          asserted by the contract, rather than executed by the contract
        - `.github/ci/README.md` "CI's own tooling": names the two suites the leg is the only
          runner for (test-audit, artifact publisher) and that both use the leg's pinned Node
        - `.claude/skills/rust-devops/ci-cd.md`: the compiler-work section now says where the
          JavaScript half runs and which contract test holds it there
        - no `just` recipe added. Checked `just/devops.just`, `just/ci-local.just` and the root
          `justfile`: no recipe runs the Python `scripts/ci/test_*.py` suites, the
          `ci_workflow_contracts` binary, or the test-audit check either. Inventing one only for
          the Node suite would break that pattern, so the omission is deliberate
- mutation-checked the new contract test, each mutation reverted after: deleting the step fails
  with "the ci-tooling leg must run the artifact publisher's Node suite"; moving the step above
  `Set up Node` fails with "the publisher suite must run on the pinned Node, not the image's
  default"; rewriting the command to `node --test scripts/ci/artifacts/` fails with "the publisher
  suite must be invoked by quoted glob, not by directory"
- re-verified on this host that `node --test scripts/ci/artifacts/` is not merely slower but
  wrong on Node v22.20.0: `# pass 0 / # fail 1`, the directory resolved as a module. The quoted
  glob form is the one shipped, and the workflow comment records why
- verification, Rust with `RUSTC_WRAPPER=""` and `CARGO_TARGET_DIR=/tmp/tt-nodesuite-20260915`:
        - `actionlint .github/workflows/*.yml` — clean, exit 0
        - `node --test 'scripts/ci/artifacts/*.test.cjs'` — 3 pass, 0 fail
        - `cargo nextest run -p test-toolkit --no-fail-fast` — 197 pass, 2 skipped (196 before;
          the new contract test is the one added)
        - `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` — 607 pass
        - `just ci-local --plan` — exit 0, plan renders
- work completed for 'Owner measurement publication runs in CI (Finding 4 follow-on)' at 06:36:11

### Successful Completion

The implementation of review cycle 2 has completed successfully in 2h 34m 28s (04:06:12 →
06:40:40, 2026-09-15). During this implementation all 4 review findings were evaluated to see
if they could be fixed as a part of this implementation cycle: 3 were fixed, 1 was deferred
(see reasons below):

- **High — The required performance acceptance gate has no usable baseline** — *deferred, in
  part*. Everything the finding asks for that does not require hosted runner capacity was
  built and verified in this cycle:
        - the review's structural objection — that instrumentation and the ownership cutover
          live in one working-tree change, so a pre-cutover baseline "cannot be retroactively
          produced" — was removed by making the instrumentation-only revision a reproducible,
          mechanically-constructed artifact
          (`scripts/ci/build_baseline_revision.py`, 11 executable tests, constructed revision
          `fb8681b86902bc3bfe67dfee76bb35727b611eee` parented on the pre-cutover merge base
          `8aa105e7c`, cutover-absence proven structurally rather than by keyword)
        - the rollout ledger's claim that the first measured hosted run after merge closes
          Tasks 7.4 and 7.5 — which contradicted the plan — was corrected in both places it
          appeared, with the plan's semantics winning
        - the deferral was recorded formally in
          `fixes/2026-09-12-single-os-compile/deferred-performance-measurements.md`
        - **what remains deferred, and why:** AC8 requires three consecutive green *hosted*
          runs per environment in matched cold and warm conditions, on both the pre-cutover
          baseline revision and `feat/single-os` — twelve hosted dispatches — followed by the
          Task 7.5 ruling against the 15% band. Collecting them requires pushing the branch
          and dispatching `ci.yml` with `measure-compiler-work: true`, which this session has
          no authorization to do; a local `cross-check` run measures a rig, not a hosted
          critical path, and is not a substitute. Nothing was faked, estimated, or
          extrapolated: the baseline observation rows remain empty, plan Tasks 1.6/7.4/7.5
          remain unchecked, AC8 remains `NOT MET`, and the single-owner design is **not**
          recorded as accepted
        - this is a capability limit of the session, not a CPU-load measurement problem, but
          it is a deferred performance measurement and is flagged as such in this log's
          frontmatter (`deferred_perf_measurement: true`)

The three fixed findings — source-identity orchestration, real-archive relocation, and owner
measurement schema testing — are closed with executable coverage, not source assertions. A
follow-on item completing Finding 4 (making the publisher's Node suite a first-class
`ci-tooling` step rather than a shell-out buried inside an unrelated workflow-contract test)
was also implemented.

### Verification at close

All suites re-run by the orchestrator after every finding was implemented, from the worktree
root, with `RUSTC_WRAPPER=""` and an isolated `CARGO_TARGET_DIR` (this host's shared
`scripts/target` cache holds read-only `.rmeta` files and a `kache` wrapper that serve stale
artifacts — a host condition, not an implementation result):

- `actionlint .github/workflows/*.yml` — clean
- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` — **607 passed** (590 at review)
- `node --test 'scripts/ci/artifacts/*.test.cjs'` — **3 passed** (1 at review)
- `cargo nextest run … --features build-tools --bin ci-build` — **117 passed** (114 at review)
- `cargo nextest run … --bin ci-rollup` — **195 passed**; `--bin ci-plan` — **13 passed**
- `cargo nextest run -p test-toolkit` — **197 passed**, 2 skipped (116 contract tests at review)
- `just ci-local --plan` — exit 0, all four environments resolved

### Files changed

- **workflows:** `.github/workflows/ci.yml`, `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml`,
  `biscuit-tui-windows-captured-stdout.yml`
- **CI tooling:** `scripts/cross-check.sh`, `scripts/ci-build-archive.rs`,
  `scripts/ci-build-archive-tests.rs`, `scripts/ci-build-tests.rs`,
  `scripts/ci/build_baseline_revision.py` (new), `scripts/ci/test_build_baseline_revision.py`
  (new), `scripts/ci/test_cross_check.py`, `scripts/ci/artifacts/publish.test.cjs`,
  `scripts/fixtures/compiler-work/publisher-documents.json` (new)
- **test contracts and guards:** `tools/test-toolkit/tests/ci_workflow_contracts.rs`,
  `tools/test-toolkit/tests/archive_path_guard.rs` (new)
- **corpus relocation:** 145 `env!("CARGO_MANIFEST_DIR")` sites migrated to
  `biscuit_test_harness::manifest_dir!()` across claudine (57), darkmatter (52),
  `tools/test-toolkit` (4) and ten further areas (32), plus 17 `Cargo.toml` manifests that
  gained or un-gated the harness dependency
- **fix-cycle documents:** `baseline-2026-09-12.md`, `rollout-2026-09-12.md`, `plan.md`,
  `deferred-performance-measurements.md` (new), this log
- **skills and docs:** `.claude/skills/os/wsl.md`, `.claude/skills/os/build-hosts.md`,
  `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/rust-testing/SKILL.md`,
  `.github/ci/README.md`
