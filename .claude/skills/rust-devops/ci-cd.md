# CI/CD and Releases in Rusty Biscuit

This page records the non-obvious decisions an agent needs before changing the
repository's CI, pre-push hook, or release automation. The live authorities are
`docs/topics/ci-cd.md`, `.github/ci/README.md`, package
`[package.metadata.ci]`, and `.github/ci/environments.json`.

## The six jobs of `ci.yml`

`ci.yml` defines exactly six top-level jobs and a contract test pins the set:
`validation`, `scope`, `preflight`, `area-ci`, `ci-gate`, `ci-reporting`. The
first five block; `ci-reporting` is advisory and carries
`continue-on-error: true`.

**No job owns a test suite on CI's behalf.** `preflight` is bootstrap
prerequisites only — checkout, `rustup show`, `just`, nextest, the
toolchain/tooling verification, the no-wrapper `cargo metadata` probe, and
`just check-canonical`. It runs no Python, Rust, or TypeScript suite, because a
suite there would be a second scheduler for work the plan already placed. Two
further jobs and one specialized Biscuit TUI workflow were deleted for the same
reason — each existed to run work a package cell now owns — and a contract test
fails if any of them reappears.

`preflight` and `area-ci` are matrix jobs carrying a **scalar** `if:` guard read
before matrix expansion (`preflight_os != '[]'`, `has_packages == 'true'`), so a
run that schedules nothing resolves both to `skipped` right after `scope`
instead of depending on GitHub's empty-matrix handling. Neither may declare a
`name:` containing a matrix expression — GitHub never evaluates the matrix
context for a job it skips, so such a label reaches the Checks tab as raw
expression text.

## Affected scope

`scripts/ci/affected_scope.py` is the canonical deterministic calculator for
local and hosted runs. Its package policy is deliberately narrow:

- A package owning changed source receives lint, L1, and its declared higher
  tiers — and `check` when it declares `example` or `bench` targets or has
  unchanged direct reverse dependents. The L1 build already compiles the
  `lib`, `bin`, and `test` kinds, so a separate compile job exists for the
  kinds no test gate produces: one check cell per native environment, running
  `cargo check -p <pkg>` with explicit `--examples`/`--benches` selectors
  (`check_args`), never `--all-targets`. A check cell is satisfied by the
  package's passing per-cell L1 receipt on the same environment
  (`check_evidence`), so the host that just built and tested the package is
  not compile-checked again. The reused cell links that receipt with no test
  counts; a version-1 whole-environment note never satisfies it, and the
  `ubuntu-latest` cell that compiles unchanged dependents is never reusable.
  `_wsl-ci.yml` declares no archive selector at all — the guest is handed the
  plan's build records and downloads one — so a check selector has nowhere to
  leak into. Every cell records `target_kinds` and `compile_coverage_from`, and
  an archive-only environment names the runner that built its archive rather
  than claiming to have compiled anything.
- An unchanged direct reverse dependency is **reported by name** in the plan's
  `reverse_dependencies` and selected nowhere: no area, no job, no result cell.
  It used to receive a compile-check entry, which presented an untested area as
  a green top-level result (PR #76). Its seam is compiled inside the changed
  package's own `ubuntu-latest` check cell instead (Open Question 1, ruled
  Option B 2026-09-12): the package record's `dependent_seam` carries the
  sorted names and one `cargo check` string (`-p` per dependent plus the
  `--lib`/`--bins`/`--tests` selectors their declared kinds need), the cell's
  status records the names and which half failed, and the rollup renders
  "also compiled N dependent(s)" on the cell. A dependent that is itself
  selected, or gates nothing, is not compiled there; local `ci-local` runs
  execute no check cell. Neither ordinary dependencies nor transitive reverse
  dependencies are selected.
- `dependent_seam.native` carries the sorted Ubuntu prerequisites of the
  consumers' complete build closures. The projection's `dependents_native`
  reaches only the owning Ubuntu check, where setup combines it with the
  changed package's own requirements. Other jobs retain the original native
  map.
- Documentation, manifests, lockfiles, and Just recipes select no package jobs.
- CI's own inputs are the exception, because CI's own suites now have owners.
  `SUITE_REGISTRY` in `affected_scope.py` declares every suite's owner,
  canonical recipe, environment, and kind (`cargo` or `companion`);
  `SUITE_OWNER_PREFIXES` / `SUITE_OWNER_PATHS` map the inputs those suites read
  to that owner. `.github/ci/**` and `scripts/Cargo.toml` select `repo-deps`;
  `.github/workflows/**`, `tools/test-audit/**`, `pnpm-lock.yaml`,
  `pnpm-workspace.yaml`, and `tools/test-toolkit/Cargo.toml` select
  `test-toolkit`. `scripts/**` and `tools/test-toolkit/**` need no entry — they
  are those packages' own directories. The manifest entries are a deliberate
  two-path exception to the repository-wide "a manifest selects nothing" rule
  and are not generalized. A trigger selection is narrower than a source
  change: it reports no reverse dependencies and carries no dependent seam,
  because the changed path says nothing about the owner's public API.
- `workflow_dispatch` is the explicit full-workspace path. Do not turn an
  infrastructure edit or uncertainty into an implicit full run.

The scope job calculates once and emits **one** canonical resolved plan:
selected areas with a reason each, the packages contributing to each, and one
`{package, environment, gate}` cell per unit of work carrying its execution
(`execute`/`reuse`), origin, state, evidence, and governance.
`schema.validate_resolved_plan` is its contract and
`.github/ci/schemas/contract.json` is the field list Rust tooling asserts
against. Downstream jobs consume that document rather than rediscovering scope.
Package remains the stored identity everywhere; **area is a derived grouping**,
computed from the manifest directory with the same rule as
`sniff repo package-area` and kept honest by a drift contract rather than by a
committed mapping file.

`RESOLVED_PLAN_SCHEMA_VERSION` is **4**. Two changes each called themselves
version 3 on separate branches — build records, and the `change_inventory`
below — so "3" named two incompatible shapes and a document from either branch
was refused for a *missing field* rather than a version skew. 4 names the union.
The validator now checks the version before the field set for that reason: an
older document usually differs in both, and the field complaint sends the reader
after a corrupt document.

Version 3 added the required
`change_inventory`: the calculator's own input paths, normalized to one
repository-relative spelling, sorted, de-duplicated, and bucketed exhaustively
into `configuration`, `documentation`, `source`, `other` with per-bucket and
total counts. A rename is one logical path; `--all` records that no diff was
consulted rather than an empty list that would read as "nothing changed". It is
a **sibling** of `change_class`, not a replacement — `classify_preflight()`
still returns `change_class`, and that is still what sets preflight breadth.
Both readers consume the one field and neither re-derives it: `ci-plan` in the
terminal, `ci-reporting` in GitHub Markdown.

Only the plan version moved. `SCOPE_RECEIPT_SCHEMA_VERSION` stays 1 — its
embedded `plan_schema_version` check is what refuses a version-2 receipt once
with the existing `scope-schema` reason, forcing one fresh calculation rather
than an in-place upgrade. `RECEIPT_SCHEMA_VERSION` and
`LEGACY_RECEIPT_SCHEMA_VERSION` are unchanged, so validation receipts stay
reusable wherever their cell and gate-input checks still qualify.

## Local scope and validation evidence

Linked worktrees share one Git hooks directory. Never install `pre-push` as a
symlink to one checkout's tracked hook: whichever worktree last ran the
installer would make every sibling execute that checkout's version. `just
init` installs the stable `.githooks/pre-push-dispatcher` as a regular file;
at invocation it resolves `git rev-parse --show-toplevel` and executes the
active worktree's `.githooks/pre-push`.

Parallel test workers default to `max(1, logical_cores - 2)` locally. CI uses
all logical cores on runners with four or fewer, otherwise `logical_cores - 2`.
The policy preserves capacity on developer and larger shared hosts without
crippling small CI runners. It controls test concurrency, not CPU affinity or a
guaranteed reservation. Shared-resource L2 stays serial; isolated suites opt in
through `l2-parallel-self-spawn`, with explicit `BISCUIT_L2_THREADS` overriding
the default.

`_test_threads` in `just/devops.just` detects CI using `CI=true`,
`GITHUB_ACTIONS=true`, or nonempty `BISCUIT_CI_ENVIRONMENT`; the `ci` Nextest
profile alone leaves the local budget in effect. L1, sanity, and real-resource
recipes preserve explicit `NEXTEST_TEST_THREADS` and otherwise export this
default. Direct local Nextest runs use `.config/nextest.toml`'s
`test-threads = -2`. Cargo build-job limits are unchanged. Existing CI-profile
groups still cap Claudine L1 at four, Claudine CLI L1 at one, and Sniff L1 on
Windows at one, even when the overall budget is larger. See the
[central policy](../../../docs/topics/ci-cd.md#layer-1--local-pre-push-hook).

Keep two claims distinct:

- **Scope evidence** identifies what the deterministic calculator selected for
  an exact `{base, head, tree, schema}` tuple.
- **Validation evidence** records the per-package, per-environment, per-tier
  outcomes of a complete local run against that scope.

When changing the current evidence implementation, preserve these agreed
semantics:

- A matching local scope receipt is authoritative. CI reuses it; a missing,
  stale, malformed, or mismatched receipt makes CI calculate scope itself.
  Live as of 2026-09-11: the hook publishes it on `refs/notes/ci-local/scope`
  in every mode, before any gate, from the committed `base..head` path set;
  `ci.yml` runs `local_evidence.py scope-verify` before the planner and
  reports `scope source` in its summary with the miss code on a fallback;
  `rustup show` runs only on that miss, right before selection, so a hit
  sets up no toolchain, and the summary's `validation environments`,
  `reused passing cells`, and `cells retained (evidence incomplete or
  rejected)` rows come from the written plan.
  Validation evidence never reopens selection: on a hit with accepted cells,
  `affected_scope.py --apply-to` overlays them on the carried plan and
  re-projects `scope.json` from it, reading nothing from the checkout. The
  plan is schema version 3 so that it can — it carries its `environments`
  table, per-package `l1_include_slow`/`exclusion`, per-cell `reusable`, and
  (since version 3) its `builds[]` records; an older receipt misses as
  `scope-schema` rather than being partially upgraded. The overlay derives no
  build key: it only drops the demand a satisfied cell no longer makes, which
  is what keeps the valid-receipt path free of a Rust toolchain.
- Only a complete passing cell is reusable. CI omits exactly those host cells;
  a complete failure remains published for diagnosis but is rejected with
  `failed-cell` and scheduled again. This rule is enforced by both the
  receipt verifier and the planner boundary.
- An interrupted run, an unavailable required backend, dirty outgoing state,
  or an explicit package override is not complete exact-tree evidence. CI runs
  any cells that are not proven.
- Local evidence may suppress only the equivalent L1/L2/browser cells it
  actually measured. `lint` and `check` stage no JUnit report, so they can
  never come from a local receipt and are always CI-origin — do not expect a
  local-origin lint cell. A receipt is keyed by environment, so it never stands
  in for another OS, for Level 3, or for a companion suite it did not execute.

Browser receipts remain supported by the existing per-cell implementation:
only a measured `browser` outcome for the matching package and environment
can satisfy that gate, under the same completion and gate-input verification
as other recordable tiers. This creates no browser job or cross-environment
coverage. Audit W9 records an explicit B0 deferral of alignment with the
absorbed September 10 specification's browser-reuse exclusion; it is not a
new user ruling or authorization to expand browser scheduling.

These semantics are live as of 2026-09-12. Version 1 is the legacy
whole-environment receipt: exact-tree-only, pass-only, and unable to carry
per-cell measurements or gate-input equivalence. Version 2 is keyed per
`{package, environment, gate}`; `strict` and `warn` both publish a complete run,
passing or failing, but only passing cells qualify for reuse. `off` is a
deprecated alias of `scope-only`. A receipt's
`host.report_dir` is where the hook retained the run's JUnit reports —
`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/`, root default
`~/.rusty-biscuit/ci-evidence` — copied there before the receipt exists;
`record-cells` refuses an empty or non-retaining directory rather than
inventing one.

Same-head retries must update that evidence cumulatively. The planner skips
the already-passing cells, so replacing the note or clearing the retained
report directory with only the retry's staged files silently discards exactly
the evidence that caused those skips. Overlay the report directory, merge only
a receipt with identical execution bindings and host provenance, and let the
new run replace the cells it actually reran.

The rollup consumes that evidence. `ci-rollup rollup --plan` reads the resolved
execution plan, so a cell a receipt satisfied is reported as a completed
local-origin result with its counts, duration, and the notes ref behind it —
not as `MISSING`. Result documents are `schema_version: 4` — version 4 made a
cell's `counts` an optional measurement, so an unmeasured cell omits it instead
of reporting zero — and a document from another generation is refused by its
version before any cell is interpreted, naming the migration and telling the
reader to re-run the rollup. The skip-only baseline keeps its own version 3. `--area` on `rollup`
and `verdict` narrows a document to one area's slice (cells, scope, scheduled
set, and accepted evidence together), and `summarize` folds slices into a view
that applies no policy.

Two cases worth knowing before reading a result:

- **A cell the plan reused and CI also executed reports the execution.** The two
  documents can disagree; when they do, the result with a report behind it is
  the honest one and the disagreement is stated in the cell's reasons. A reused
  cell is therefore not a guarantee that no job ran for it.
- **`lint` and `check` cells have no JUnit walker behind them.** Their state
  comes from the producer status, which is why a lint job that never reported is
  `MISSING` rather than absent. Any scheduling change must keep uploading those
  statuses.

`ci.yml` fans out one caller identity per selected AREA (`area-ci`, over the
planner's `scheduled_areas`) into `_area-ci.yml`, which fans out the area's
packages into `_package-ci.yml`, which delegates the WSL2 cell to
`_wsl-ci.yml` — four levels including the caller, GitHub's maximum, with no
margin for another. Each area's `coverage-audit` job runs `if: always()` behind
its producers, renders the result slice, and narrows runner-loss attribution to
that area's packages. It runs `verdict --area` only when every producer
succeeded: an ordinary producer failure already reaches `ci-gate`, while the
audit remains fail-closed for missing or unscheduled evidence, invalid gaps,
and exact-skip violations. This avoids two red checks for one test failure.
Area remains a grouping, not an identity; only the per-area slice artifact name
contains it (`ci-results-<slug>`, with `/` spelled `--`).

**An all-reused area must still fan out.** If a receipt covers every cell an
area owns and the area then dropped out of `scheduled_areas`, no
`ci-results-<slug>` slice would be written and that area's local-origin results
would be reported nowhere. The planner builds the matrix from each package's
*declared* gates rather than its executing cells, which is what gets this
right; nothing turns red when it breaks, so it is pinned by a fixture.

Two rules the presentation depends on, both cheap to break:

- A job that can be skipped as a whole carries **no `name:`**. GitHub does not
  evaluate the matrix context for a skipped job, so a `name:` holding
  `${{ matrix.… }}` reaches the Checks tab as raw expression text. Omitting it
  makes the label the job id when skipped and `job-id (matrix values)` when it
  runs. The package half of the identity comes from the caller, because a
  called workflow's jobs render as `<caller job name> / <called job name>`.
  That composite label is a **parsed contract**, not just presentation:
  `runner_loss.py` reads `area-ci (<area>) / <package> / <gate> (<env>)` from
  the tail to attribute a dead runner's cell. Phase 6's renaming broke all six
  producer labels at once and nothing turned red, because every fixture spelled
  the names by hand. `test_runner_loss.py` now derives them from the shipped
  workflows instead, and runs in `just ci-local`'s self-test loop.
- Advisory jobs carry `continue-on-error: true`. `ci-gate` folds
  `needs.*.result`, and `continue-on-error` turns a failed job's result into
  `success` for that fold, so it is exactly what keeps a reporting job out of
  the gate — and exactly why no blocking job may carry it.
- A failed gate command fails both the cell and its producer job. Status and
  JUnit publication run under `always()` or `!cancelled()`, so the area
  coverage audit still provides the package/environment/gate diagnosis.
  Every matrix uses `fail-fast: false`, so one OS failure does not cancel
  sibling cells that still require execution. Only recovery, diagnostic, and
  advisory steps may carry `continue-on-error`; workflow contract tests pin
  these properties.

## Producing and consuming a Nextest archive

`fixes/2026-09-12-single-os-compile/` splits a test cell in two: a native
**producer** compiles one immutable archive per planned build key, and every
**consumer** verifies and executes those exact outputs without a compiler.
`ci-build produce` and `ci-build verify` (`scripts/ci-build-archive.rs`) are the
two halves; the plan's `builds[]` records are their only scheduling input.

Four facts about `cargo nextest` this cost real time to learn, on 0.9.136:

- **`archive.include` accepts `relative-to = "target"` only.** There is no
  `"workspace-root"`. A repository *file* therefore cannot enter an archive that
  way — which is fine, because every consumer already checks the source out and
  `--workspace-remap` points the run-time `CARGO_MANIFEST_DIR` at it. Archive
  includes are **build outputs**.
- **A tool config's `profile.default` loses to the repository's.** Passing
  `--tool-config-file` with a `[profile.default] archive.include` is silently
  ignored when `.config/nextest.toml` sets the same key. A **named** profile in
  the tool config wins, so `produce` writes `[profile.ci-build-archive]` and
  passes `--profile ci-build-archive` — and merges the repository's own
  `profile.default` entries in, because replacing them would quietly drop
  whatever the repository declared from every archive.
- **A workspace `dylib` is not archived.** Test binaries, non-test `bin`
  targets, build-script output directories, and linked paths are; a `dylib` and
  an example are not. That is what `archive-includes` exists for.
- **An include only COPIES; an example is never built.** `cargo nextest archive`
  builds lib, bin, and test targets, so a declared `examples/<name>` include
  would error on a file that was never going to exist. `ci-build produce`
  therefore builds each declared example first. This was live: the repository's
  own `profile.default.archive.include` carried `discovery_probe` for two
  hard-coded layouts with `on-missing = "ignore"`, which matched neither the
  macOS nor the Windows producer and silently shipped `biscuit-terminal`
  archives without the example its PTY tests panic on. It is now
  `biscuit-terminal`'s own `archive-includes` entry.
- **`nextest list --message-format json` is the inventory.** `rust-binaries`
  gives every binary id and `rust-build-meta` gives the non-test binaries,
  build-script output directories, and linked paths — so the manifest's
  inventory and runtime assets are *discovered*, not restated.

A verification failure is a **verdict, not an error**: `ci-build verify` answers
a versioned document, exits `3` (distinct from `2`, a tool failure), and never
compiles a replacement for what it refused. Codes come from
`schema.BUILD_REJECTIONS`, asserted against `contract.json` from the Rust side.

All three canonical tier recipes (`_test`, `_test_l2`, `_test_browser`) accept
archive mode: `-p <pkg>` moves into the filterset because `--archive-file`
forbids it, `_archive_drop_build_flags` removes the Cargo build flags, and a
missing `cargo-nextest` is a hard error — the `cargo test` fallback recompiles,
which is the one thing an archive consumer must never do.

`scripts/ci/fixtures/archive-portability/` is the proof: a three-member
workspace carrying one of every payload class, built in one checkout and run
from another with the producer's target directory renamed away and no Cargo,
rustc, or linker on `PATH`.

### The owner job, and what it is not

`ci.yml`'s `build` job uses one matrix leg per producer environment from
`build_owners`. `scripts/ci/produce-owner.sh` invokes the producer once without
`--key`, sharing its target tree across package invocations. The programmatic
publisher in `scripts/ci/artifacts/` uploads separate package-keyed archives and
statuses, including successful records when a sibling fails. Artifact transport
must not change build ownership. Its Node dependencies are pinned by `npm ci`.

Manifest generation 3 verifies clean tracked source at the planned commit on
both boundaries. That makes the checkout ref part of the contract: every
`actions/checkout` in `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, and
`_wsl-ci.yml` is pinned, because `pull_request` otherwise checks out GitHub's
merge branch and no job could produce or verify. `ci.yml` resolves the revision
once as `github.event.pull_request.head.sha || github.sha` for the two jobs that
precede the plan; the scope job then publishes the plan's own `head`, and the
reusable workflows are handed it as a required `tested-revision` input rather
than recomputing it. `scripts/cross-check.sh` meets the same contract by
committing the local tree as one throwaway commit, shipping it as a `git
bundle`, and naming it as the plan's head — no host applies a patch.

Native requirements are observed from payload binaries and
resolved again on the consumer; external library hashes/Mach-O UUIDs must match.
This conservative compatibility rule can reject ABI-compatible library upgrades.
`--source-tree` only asserts the observed Git tree; it cannot override it.
The producer-scoped workflow entry point is covered by a real-Cargo shared-dependency
fixture. Opt-in measurements include per-record counts and owner totals in each
package artifact; hosted comparisons still decide the architecture's acceptance.

Declaring `executes` in `environments.json` is what makes an environment a
producer, and all three own the archive their consumers execute: Linux (with the
WSL2 guest it hosts), macOS, and native Windows. There is no held-back state —
Task 6.5 removed the `archive_cutover` migration switch together with the
compile-in-place paths it guarded, and a contract that still carries the field
is refused. **No test tier installs a toolchain or restores a Cargo cache**, and
`just _ci_build_consumer` refuses a cell the plan names no build for rather than
answering "compile in place": on a toolchain-free consumer that fallback would
not even fail honestly.

**The runner label is not the compatibility authority.** `ci-build produce`
refuses a record at the `preflight` stage — before it compiles, per record, so a
sibling key still finishes — when `rustc -vV`'s host is not the record's `host`,
or when the toolchain has no standard library for its `target`. Compiler host
and target are keyed inputs, so an archive produced by the wrong toolchain
verifies fine (the consumer compares against the *planned* key) and hands its
consumers binaries of another machine.

**Consumer paths are spelled for the consumer's own OS.** Under Git Bash — what
`shell: bash` gets on a Windows runner — `$PWD` is `/d/a/repo`, which
`--workspace-remap`, insta, and the JUnit staging root cannot open, while
`$RUNNER_TEMP` is a Windows path in the same shell. `just _native_path` answers
the drive-qualified forward-slash spelling (`cygpath -m`, no `\\?\` prefix, the
one form both MSYS and Win32 accept) and `_ci_build_verify` computes the
workspace once, publishing it as the `workspace` step output that all three
tiers bind to `ARCHIVE_WORKSPACE`.

**`check` and `lint` compile on purpose and are counted as their own
configurations.** Clippy is another compiler driver with its own flags, and
check-only example/bench kinds may emit no executable, so neither consumes an
archive. Both jobs now carry the command-scoped compiler-work counter under
`check <environment>` / `lint ubuntu-latest`, so a post-cutover measurement
shows one archive plus two *named* configurations rather than an unexplained
second compile.

**A build is not a result cell.** It has no `{package, environment, tier}`
identity, publishes no JUnit or `status-…` artifact, and is never baselined. It
publishes `build-status-<package>-<producer>-<key>/build-status.json` under
`always()` — success, compile failure, upload failure, cancellation — and
`runner_loss.py attribute --plan` synthesizes one from the plan when the owner's
runner died. `ci-rollup` renders a dependent cell as
`MISSING — blocked by build <key> …`, which blocks and which the skip baseline
cannot excuse; an unrelated key proceeds, and a real test result outranks the
diagnostic.

The producer stages `ci-build` itself inside the artifact. A consumer with no
Cargo — the WSL2 guest above all — cannot build a verifier, and shipping it
separately would let the archive and its verifier drift apart. Verification runs
**in the guest**, not on its Windows host: `host_runtime()` reads `cfg!`, so a
host-side run would report `msvc` and prove nothing about the machine that runs
the tests.

The guest clones to a path of its own (`GUEST_ROOT`), unrelated to the
producer's. It previously recreated the manifest's `producer_workspace`, because
`--workspace-remap` rewrites only the run-time `CARGO_MANIFEST_DIR` and ~160
test sites read the compile-time `env!`. Those sites now use
`biscuit_test_harness::manifest_dir!()`;
`tools/test-toolkit/tests/archive_path_guard.rs` scans the repository and fails
on a new one, and the `slow_` relocation fixtures in
`scripts/ci-build-archive-tests.rs` produce a real package's archive, delete the
producer's checkout, and run it from somewhere else. `producer_workspace` stays
in the manifest as provenance inside the realized digest, and nothing executes
by it.

### What each stage cost, and why three numbers are observed from outside

Seven windows are reported, never folded into one another: producer **queue**,
**compile+archive**, and **upload**; consumer **download**, **verify**,
**extract**, and **execute**. The point is negative — a measurement must not be
able to hide transfer or setup inside test time.

Three of them no tool can see from inside itself, so the workflow observes them
between steps. Queueing closes before `ci-build` starts (`scope` publishes a
`plan_epoch`; the owner marks its own start) and the upload opens after it exits,
and both are merged into `build-status.json` as `stage_seconds`. An artifact
download is an action rather than a command, so a `transfer` marker step opens
the window and `_ci_build_verify` closes it.

**Extraction is the verifier's, deliberately.** `cargo nextest run
--archive-file` extracts inside the run, so a consumer cannot time that
extraction apart from its tests without replacing `--archive-file` with
`--binaries-metadata`/`--target-dir-remap` in every tier recipe. The verifier
already performs a full `--extract-to` of the same archive on the same host
(listing the inventory requires it), and `ci-build verify --verdict-out` reports
that window as `extract_ms`. Same for compile and archive on the producer side:
one `cargo nextest archive` command, one number, named for what it measures.

**Seconds outside, milliseconds inside.** macOS and Git Bash have no
`date +%s%3N`, so workflow-observed windows are whole seconds and tool-measured
ones are milliseconds; the unit is in every field name. An **absent**
measurement stays absent — `ci-rollup` renders `—`, never `0s`, and a status
step omits the object rather than publishing zeros. Both status scripts treat a
malformed measurement as absent too: they run under `always()` and are the
cell's only evidence, so instrumentation must not be able to lose it.

The WSL2 guest cannot write `$GITHUB_OUTPUT` (it is a Windows path), so it
leaves `wsl-timing/verify.seconds`, `wsl-timing/l1.seconds`, and a copy of the
verdict in the 9p workspace and the host's status step reads them — the same
reason the manifest is read on the host.

`ci-rollup` renders **Build provenance** (one row per executing cell: planned
key, realized digest, producer, and its four stages) and **Build records** (one
row per key: producer, result, queue, compile+archive, upload). Neither table is
an identity: artifacts, receipts, baselines, and JUnit stay keyed on
`{package, environment, tier}`, and no gate outcome is derived from either.

`ci-rollup` keeps writing plain GFM rather than the `renderable` Markdown tree.
That is not an oversight: it is the always-runs merge-gate binary, it links none
of the monorepo's crates, and its only output target is GitHub's own renderer.
`ci-plan` and `ci-build` do render through `TerminalRenderable`.

## Two traps in the workflow files themselves

`actionlint`'s `github` context model **does not include `run_started_at`**,
even though GitHub documents and populates it. A workflow that reads it lints
red on the required `actionlint` check with "property is not defined in object
type". Carry the run id and attempt plus a `date +%s` taken in the job's first
step instead, and join them against the jobs API wherever the reader lives.
That is also the portable choice: parsing an ISO timestamp in a workflow needs
GNU `date -d` on Linux and BSD `date -jf` on macOS.

**`RUSTC_WRAPPER` is per-command, never per-job.** All four reader-facing
workflows set `RUSTC_WRAPPER: ""` at workflow level to clear a stray host
value. The Phase 1 compiler-work counter of
`fixes/2026-09-12-single-os-compile/` is the only thing that ever sets a
non-empty one, and it does so in a single gate step's own `env:`, sourced from
the preceding step's output. A job-level value would also wrap the runner-tool
stub builds and the counter's own build, mixing wrapped and unwrapped units in
one `target/`. `ci_workflow_contracts::the_compiler_work_wrapper_is_never_global`
enforces the rule; the switch that enables any of it,
`measure-compiler-work`, is exposed on `ci.yml`'s `workflow_dispatch` alone and
defaults false at every level of the chain.

**The counter's report crosses into JavaScript.** `ci-build`'s `Report` is
serialized into each manifest's `compiler_work`, and
`scripts/ci/artifacts/publish.cjs` reads `compiler_invocations` and
`compiler_ms` out of it by name to build the owner aggregate that AC8's
compile-once claim is read from. Node cannot see the Rust struct, so its test
asserts against `scripts/fixtures/compiler-work/publisher-documents.json`,
which `ci-build-tests.rs::the_javascript_publisher_reads_a_fixture_generated_from_this_report`
generates and compares byte for byte. Renaming a `Report` field turns that test
red; regenerate with `BLESS_COMPILER_WORK_FIXTURE=1` and update the publisher in
the same change. Nothing selects that suite through a Cargo package or a pnpm
workspace entry, so its `SUITE_REGISTRY` entry `artifact-publisher` is the only
thing that schedules it. The fixture lives outside `scripts/ci/` because
`build_baseline_revision.py` carries it with the instrument and refuses a
constructed revision that adds a `scripts/ci/` path.

## Where CI's own suites run, and where area drift is enforced

**CI runs no separate job for its own tooling.** Those suites are scheduled by
`SUITE_REGISTRY` in `scripts/ci/affected_scope.py` and executed by
`scripts/ci/companion_suites.py` from the `Companion suites` step of
`_package-ci.yml`'s `test` job (and `Companion suites (lint)` in `lint`, for a
suite declaring a `lint_recipe`). A registered suite must also be named in its
owner's `companion-suites` manifest list, or `validate_suite_registry` refuses
the plan — registration alone schedules nothing.

`repo-deps` owns the thirteen `scripts/ci/test_*.py` planner contracts plus
`artifact-publisher`; its Rust binaries (`ci-rollup`, `ci-plan`, `ci-build`,
`drift`) run in its own `repo-deps-l1` cell. `test-toolkit` owns
`ci_workflow_contracts` (`test-toolkit-l1`), `test-audit-typecheck`, and
`test-audit-vitest`.

The sniff area contracts (`scripts/ci/test_resolved_plan.py`, class
`AreaGroupingTests`) are enforced **on the merge path**, by `ci.yml`'s own
`area-drift` job:
`ci-gate` folds `needs.area-drift.result` like every other blocking job. The
job builds `sniff-cli` once behind a `rust-cache` entry, puts it on `PATH`, and
sets `BISCUIT_REQUIRE_SNIFF=1` so an absent sniff **fails** rather than skips.

It is a job of its own rather than a companion suite because a release
`sniff-cli` measures 264.6s cold / 99.5s warm, and because a sniff compile
error would then redden a package's ordinary test cell rather than the check
that exists to report it. What makes the cost acceptable is scope. A
second planner flag, `area_drift`, schedules the job, and it is true for
`sniff/**`, `scripts/ci/affected_scope.py`,
`scripts/ci/test_resolved_plan.py`, **and every `Cargo.toml` at any depth** — a
manifest that appears or moves re-maps areas without touching either
implementation, and `git diff --name-only` cannot say which manifest edits did.
On every other pull request the job is `skipped`, which the fold accepts.

`.github/workflows/area-drift.yml` keeps only its schedule and
`workflow_dispatch`, as the backstop for the one defect the gate job cannot
catch: the gate job is scheduled *by* the planner, so a bug in `area_drift`
itself would skip the check silently. Its `pull_request` trigger is gone —
on a pull request the gate job already runs the identical class.

The contract asks sniff what area a *directory* resolves to; the cheaper
inverted query (areas, then packages per area) answers a different question and
loses `biscuit-test-harness`, which is the one divergence
`SNIFF_SELF_INCONSISTENT` exists to record.

Three rules when adding a Python suite to the registry:

- **Gate every host tool through `scripts/ci/tool_guard.py`.**
  `require_tools("just", "jq", enforced_by=…)` (or the `@requires_tools` class
  decorator) skips where the tool is genuinely absent and **fails** where a job
  declared it provisioned with `BISCUIT_REQUIRE_<TOOL>=1`. `enforced_by` is
  keyword-only with no default, so a guard cannot be written without naming the
  job that does run the contract — a skip saying only "requires just" claims
  nothing about coverage. The declaration is **per job, never a blanket fail
  under `CI`**: `preflight` runs on up to three operating systems and
  provisions neither `sniff` nor `jq`, so a global rule would turn macOS and
  Windows red on every push. Today `_package-ci.yml`'s `Companion suites` step
  declares `BISCUIT_REQUIRE_CARGO`/`JUST`/`JQ`/`BASH`, and `area-drift`
  declares `SNIFF`.
  `ci_workflow_contracts::every_tool_guard_declaration_is_set_by_the_job_that_enforces_it`
  holds both directions: a guard whose variable no job sets, and a variable no
  guard reads.
- **Check what the suite needs from Git history.** The registry vocabulary has
  no fetch-depth field and nothing consumes one, so history is a *job-side*
  guarantee: `_package-ci.yml`'s `test` job checks out with
  `fetch-depth: ${{ inputs.companion-suites != '[]' && '0' || '1' }}`, which
  gives full history to the packages declaring companions and depth 1 to the
  rest. The digits are quoted because GitHub reads a bare `0` as false, which
  would yield 1 on both branches. `test_build_baseline_revision.py` resolves
  `BASE_REVISION`; at depth 1 that revision is absent and the suite reported
  **11 tests green having run 3**.
- **Expect no ordering control.** The registry cannot say "run after the step
  that built what I shell out to". `test_build_key.py` therefore runs where
  `ci-build` is not on disk and `build_key.py` falls back to `cargo run`:
  correct, but ~14–26s rather than the 0.1s it costs beside a built binary.

The `just ci-local` self-test list is spelled out in four coupled files
(`just/ci-local.just`, `scripts/ci/test_ci_local.py` twice, and
`.githooks/tests/test-pre-push.sh`). Editing the recipe alone produces ten
failures — the fixtures stub each suite by name.

Measurements: `reviews/2026-09-15-python-test-code/spike-3-results.md` and
`spike-4-results.md`.

## The merge gate

`ci.yml`'s `ci-gate` job is the single required check: a policy-free fold
(Open Question 3, ruled 2026-09-12; the ruleset "require workflows" rule is
organization-only, so the fixed-name conjunction job was selected and proven in
`fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`). It `needs`
every blocking top-level job, runs `if: always()`, and passes only when each
`needs.*.result` is `success` or `skipped`. An unselected area's skipped job
does not block; `failure` and `cancelled` do; a `MISSING` cell is caught by
its area's coverage audit, never by the fold. It reads no plan, policy, baseline, or
artifact — `ci_gate_is_the_single_required_check` in
`tools/test-toolkit/tests/ci_workflow_contracts.rs` pins that, and
`WorkflowGateStepTests` in `scripts/ci/test_ci_local.py` runs the extracted
fold against every result shape.

Ruleset `protect-your-bacon` (19747338) has required `ci-gate` since
2026-09-13. It named `ci-verdict` until then, and because that job no longer
existed in `ci.yml`, every pull request sat on a `ci-verdict — Expected` check
that could never arrive; the swap was taken as the cicd-cleanup
specification's Validation and Rollout step 6. The admin bypass actor is
untouched. The consumers moved with the job: `runner_loss.py`
excludes `ci-gate`, `just ci-diff` downloads the per-area `ci-results-<slug>`
slices and folds them through `ci-rollup compare --base … --base … --head …`,
and `.claudine/scripts/ci-watchdog.ts` waits for `ci-gate`.

The run conclusion is a faithful conjunction: exactly one job
(`ci.yml:ci-reporting`) carries `continue-on-error: true`, asserted as an exact
set over all four reader-facing workflows. `reuse_validation.py` and
`release-plz.yml` key on a completed, successful `ci` run, which the scratch
fixture showed can read `cancelled` with a green gate (s6) — the safe
direction, so neither needs rewiring.

## `ci-reporting`

The run's one reader-facing report, and deliberately powerless. `if: always()`,
`continue-on-error: true`, and `needs: [validation, scope, area-ci, ci-gate]`
— it waits for the whole run so it is written after the decision it declines to
make. It may state that no package test was required; it must never claim
mergeability, and it applies no baseline, accepted-gap, missing-cell, or merge
policy.

Three modes, chosen from its `needs` results:

1. **Reused PR validation** — link the authoritative prior run, state that this
   run executed no package cell.
2. **Successful scope** — download the resolved plan and this run's
   `ci-results-<slug>` slices and render them through `ci-rollup summarize`.
   That is the same typed model the areas wrote; parsing the raw JUnit
   artifacts here would build a second result model free to disagree with the
   area that produced them.
3. **Failed or cancelled bootstrap** — name the first actionable infrastructure
   failure in dependency order (`validation`, then `scope`). `area-ci` is
   deliberately absent: every package gate is a cell in its own area's coverage
   audit.

Mode 2 renders the change inventory, the direct and reverse dependency sets,
per-environment test counts and durations including machine-recorded companion
counts, the Linux-only `ci` lint **command** duration labeled as such, and each
cell's literal `ci` / `local` / `prior-local` origin. An unavailable measurement
renders as `not recorded` with its reason — never `0`. There is no `cicd`
origin literal, and `check` and `lint` stay CI-origin.

Every area uploads its result slice under `always()`, including an area whose
cells were all reused, so a missing slice means that area never started rather
than that it had nothing to say.

## Governed policy gaps

A tier a package owns tests for that an environment cannot host is governed
**once**, in `.github/ci/environments.json`, as a capability object carrying
`available: false` plus `reason`, `owner`, `expiry`, and optionally `closes` —
the tracked work that ends the gap. A plain `false` is an *ungoverned* absence.

- A governed, unexpired gap is a distinct machine-readable **`ACCEPTED GAP`**
  state: neither a pass nor a test failure, and it does not block. The coverage audit
  renders its owner, expiry, policy entry, `closes` link, and revocation
  instructions where a reader sees the cell.
- It is also published **immediately** — before any producer runs — as one
  `neutral` check run per cell on the PR head by `_area-ci.yml`'s
  `accepted-gaps` job (`scripts/ci/publish_gaps.py`, OQ4 ruled 2026-09-12).
  The name identifies the cell; title, summary, and text carry the marker,
  owner, expiry, reason, revoke instructions, `closes`, and a `details_url`
  on the policy entry. `neutral` leaves the PR clean and never alters the run
  conclusion; the tool refuses an ungoverned or expired cell rather than
  present it as harmless. That job is the only one holding `checks: write`:
  `ci.yml`'s `area-ci` carries the grant as a cap (a called workflow's token
  cannot exceed its caller's) and `package-ci`/`coverage-audit` stay read-only.
- An absent, incomplete, or expired acceptance is a blocking `POLICY GAP`.
- The state is decided by the planner before the run and is **never inferred
  from a GitHub cancellation conclusion**. A real failure outranks it.
- Do not put a policy gap in `ci-baseline.toml`; gaps are planner-owned
  governance records, while the baseline contains only exact skip budgets.

A `gates = false` package owns no plan cells at all. Its governed
`NOT SCHEDULED` entries come from the resolved-package policy document, which is
the only place its owner, class, and expiry live — that document cannot be
deleted without moving the exclusion metadata into the plan first.

A receipt's `base` is the reviewed trigger context's base — the scope
receipt's, so a stacked pull request records its target tip rather than a
merge base with `origin/main` — and scope verification requires that exact
comparison base, even when the PR target has advanced beyond the head's branch point. Version-2 recording
accepts that advanced base. Validation verification uses the merge base only
to enumerate candidate notes; reuse still requires exact tested-tree identity
or independently recomputed gate-input equivalence.

The local gates consume evidence too (ruling D2, audit W14): on a clean
checkout the hook feeds HEAD's reviewed, evidence-overlaid plan to `just
pre-push` (`just ci-local --plan-in`, via `BISCUIT_CI_PLAN_IN`), so the
planner never selects twice, a cell prior passing evidence covers is skipped,
a cell whose newest evidence is a failure is rerun, lint always runs, and the
receipt — recorded against that same plan and base, so its `scope_identity`
is the scope receipt's plan identity — lists only the cells that ran. A dirty
checkout or `RUSTY_BISCUIT_PRE_PUSH_AREAS` keeps the working-tree replan and
publishes nothing, saying why.

An interrupted local validation publishes no validation receipt, even when an
earlier gate staged a passing report. The hook handles INT/TERM/HUP directly;
`ci-local` aborts on signal exit codes and writes an `interrupted` marker in
its report directory so the hook retains that decision across Just wrappers.
Ordinary complete failures still publish their measured outcomes.

A receipt from an **older head** is reusable only when the cell's gate-input
identity is unchanged — the `git ls-tree` entries of the tested package's build
closure (dev-dependencies included, and the lockfile) plus that gate's global
inputs. Verification recomputes that over both trees rather than trusting the
identity the receipt stored. `schema_version: 1` notes are exact-tree,
pass-only, whole-environment, never upgraded in place, and render their
measurements as `not recorded (v1 receipt)`.

A gate that staged no JUnit report is recorded `partial`: it has an exit code
and nothing to attribute it to, so it is published and refused for reuse rather
than credited as a tested cell. That is how a compile failure stays a CI job.
For local evidence, Nextest still resolves the configured relative JUnit path
under the workspace `target/nextest/<profile>/` tree even when
`CARGO_TARGET_DIR` isolates build artifacts elsewhere. `just ci-local` therefore
sets `BISCUIT_JUNIT_TARGET_DIR` to the workspace target explicitly and uses the
`local-evidence` profile: it emits the report without inheriting the CI-only
Claudine concurrency caps.

## Execution constraints before a push

Reuse qualifying passing evidence per required cell on every OS. If no
qualifying passing evidence exists, execute the required tests. A request to
avoid rerunning passed tests is not an environment ban. Only a separately
explicit instruction (such as an environment unavailable during maintenance)
creates an execution constraint; never infer a blanket WSL prohibition.

A separately explicit execution ban also applies to automatically triggered
jobs. Review the final matrix against those bans. The CI cleanup WSL example
was mistakenly promoted to a permanent ban; Ken corrected it on 2026-09-12.
Missing qualifying passing evidence schedules required coverage, including WSL.

The planner takes a verified **per-cell** result set (`--accepted-cells`),
either while selecting or, in `ci.yml` and `just ci-local --plan`, applied
afterwards to the plan already in hand (`--apply-to`). Both paths decide a
reused cell's shape in one helper and are pinned byte-identical. An accepted
cell has its execution omitted and stays a cell with local origin, so the
rollup expects a local-origin result for it instead of reporting `MISSING`;
suppressing the matrix entry while leaving the policy expecting a CI result was
the PR #76 seven-cell regression.

`local_evidence.py verify --cells` produces that set by reading **every**
note on every environment's notes ref between the merge base and the outgoing
head, so a macOS receipt from this push and a prior WSL receipt combine in one
run. Within one environment each cell is resolved by the **newest note that
qualifies for it**, so a later run that covered fewer packages does not hide an
older, still-equivalent passing receipt for the rest. Complete failures are
diagnostic records and never enter the accepted set. Each refusal describes
one candidate note, carries a code from `schema.REJECTIONS`, and is published
with the plan.

`scripts/cross-check.sh` publishes a `wsl2-ubuntu` receipt only when its WSL leg
ran the outgoing head's exact tree on a clean remote worktree with no test
filter; every other run prints why it published nothing. It ships the
developer's local tree, uncommitted work included, so most of its runs test a
tree no head names.

**Record the restriction, do not remember it.** The store at
`<home>/.rusty-biscuit/ci-constraints/<repository>/` (beside the evidence
directory; `BISCUIT_CI_CONSTRAINTS_DIR` overrides it) holds `{environment,
gate?, reason, owner, expiry, repository?, branch?}` records. `just ci-local
--plan` and the pre-push hook enforce them; CI never reads them, so a
constraint can only stop a push. The default is resolved only at those two
trigger boundaries — `constraints.py directory` hands it to the planner's
`--constraints`, which CI never passes — and an unknown repository reads the
store root recursively, so every record binds. `<home>` is `Path.home()`, so a
test that relocates it sets both `HOME` and `USERPROFILE`.

The hook decides them per **branch update**, in the order Git supplies them:
each pushed revision's committed `base..head` path set, planned by the planner,
manifests, and policy committed there — in a temporary detached worktree unless
the revision is the clean checkout — with published evidence applied through
`--apply-to`. Each update is checked under its own identity: the repository is
the remote URL Git hands the hook, the branch is the remote branch the update
writes, and a renamed refspec (`feature:other`) binds a record under either
name; the checked-out branch and `origin` are never substituted. Deletions,
tags, and other non-branch refs trigger no run and are skipped by name; the
first failing update blocks the push before any note is published. HEAD's
reviewed plan is also the scope receipt it publishes, so the two cannot
disagree. `just ci-local --plan` is the
working-tree preview; a committed change masked by an unstaged revert is
absent there and present in the hook's review, which is why the preview is
not the decision.

Each update is planned against the base of every run it triggers, because
`ci.yml` fires `push` for `main` only and `pull_request` for every target
branch: the remote's `main` for a push to `main`; otherwise the current remote
tip of each open pull request's target branch (`gh pr list` on a GitHub
remote — `gh` or `jq` missing, unauthenticated, or failing blocks the push
and names the command), each context planned and checked in turn with the
first failure blocking; or a provisional plan against the remote's `main` when
no pull request is open. When the same push also updates a target branch, the
server may apply the two updates in either order, so that run is reviewed in
both states — against the target's current tip and against the incoming
revision; a target the push deletes leaves its pull request no base, and the
hook blocks that update by name. The pull request opened next — from the web UI, where
no hook runs — is a trigger the hook cannot see, so the provisional plan is
constrained too. The scope receipt binds the first context's base and is
recorded even when that base has advanced beyond HEAD's branch point. An
all-zero branch-creation base still publishes no scope receipt. The reviewed
plan feeds clean local validation independently of receipt publication success.

If prior evidence cannot be reused or CI cannot express the requested
exclusions, resolve that limitation before pushing. Preserve the restriction
while explaining what is missing; do not silently substitute a new test run or
fabricate current-head evidence. These are execution constraints, distinct from
whether a package must support the environment.

## Intentional bypass modes

Prefer a repository-provided **scope-only** mode over `git push --no-verify`
when the goal is to skip local tests and let CI exercise every supported
environment. Scope-only resolves and prints the plan of every pushed branch's
committed tree — so a recorded execution constraint is still enforced and the
run is still reviewable — but runs no gate, publishes no validation outcomes,
and excludes no CI cells. It does publish the standalone *scope* receipt, so CI
takes the committed scope from it on an exact `{base, head, tree}` match and
recalculates only on a miss.

`git push --no-verify` prevents the pre-push hook from executing and produces
no new evidence. It does not invalidate already-published matching receipts:
CI still verifies them and can omit their covered environment. If strict
validation was run separately on the exact clean outgoing head and its receipt
was published and verified, the branch transfer can use `--no-verify` without
repeating that validation. Otherwise, the absence of qualifying evidence leaves
the corresponding CI cells scheduled. The flag itself excludes no environment.

A hook run long enough to select the whole workspace (a workflow-file or
`.config/nextest.toml` change; about 45 minutes on the development Mac,
2026-09-17) can outlive the SSH transport Git opened before the hook started.
The symptom is `Pre-push validation passed.` followed by `git push` exiting
141 (SIGPIPE) with the branch ref unchanged on the remote, while the hook's
own note push of the evidence went through on its own connection. The hook
did not fail and nothing needs bypassing: run `git push` again. The published
receipt already covers the head, so the hook reuses every cell and finishes in
minutes, and the transfer completes. Confirm with `git ls-remote --heads
origin <branch>` after every push rather than trusting the exit code.

Mode intent is:

| Mode | Push after local failure | Scope evidence | Complete host outcomes | CI host cells |
|---|---:|---:|---:|---|
| `strict` | No | Yes, before the gates | Pass or fail | Omit qualifying passes; rerun failures |
| `warn` | Yes | Yes, before the gates | Pass or fail | Omit qualifying passes; rerun failures |
| `scope-only` | No tests run | Yes; plan resolved and printed | No | Run all |
| `--no-verify` | Yes; hook does not run | No new evidence | No new evidence | Existing valid receipts still apply |

Do not implement a failing local-evidence job as an upstream dependency that
causes the remaining matrix to skip. Keep failures diagnostic-only and ensure
every remaining OS continues before the final verdict fails.

## Release contract

Release-plz is the sole version/tag/changelog authority:

1. A successful `ci` run on `main` triggers a non-canceling release-plz job for
   that exact validated commit. It opens or updates a draft release PR.
2. Merging a PR labeled `release` triggers publication. Ordinary pushes do not
   publish releases.
3. `publish = false` means no crate is published to crates.io. Git tags and
   GitHub releases are the current package release channel; the specialized
   integration workflow may attach its own cross-compiled release assets.

Do not introduce a second version authority, publish from a feature branch, or
silently turn on crates.io. A new registry, installer generator, signing path,
or binary matrix is a separate design decision whose credentials, target
coverage, checksums, and rollback behavior must be explicit.

## Verification boundaries

- Package tests use Nextest and canonical `just` recipes; load the
  `rust-testing` skill before changing their gates or tiers.
- OS evidence is environment-specific; load the `os` skill before changing
  platform matrices or claiming an environment cannot be exercised locally.
- CI and release workflow changes require their compact contract suites and
  `actionlint`; they do not justify running every package.
- Preserve the pinned toolchain. Required CI follows `rust-toolchain.toml`;
  floating stable and nightly belong only to their advisory workflows.
