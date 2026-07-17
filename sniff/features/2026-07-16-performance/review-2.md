---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T11:21:04-07:00
spec: 2026-07-16-performance/spec.md
implemented: false
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-2.md
---

# Review 2

## Findings

### High: the aggregate JSON builder still performs host and filesystem observation

R2.7 and the acceptance criteria require `build_aggregate_value` to be a pure projection that
performs no host or filesystem observation ([spec.md:131](spec.md#L131),
[spec.md:383](spec.md#L383)). The builder still resolves the current/base directory through seven
context helpers ([repo_json.rs:749](../../cli/src/output/repo_json.rs#L749),
[repo_json.rs:778](../../cli/src/output/repo_json.rs#L778)). Those helpers call public `RepoInfo`
lookups such as `package_for_dir` and `package_area_for_dir`; `package_for_dir` rebuilds an ownership
index and canonicalizes the root, every package path, and the query on every call
([types.rs:274](../../lib/src/filesystem/repo/types.rs#L274)), while `package_area_for_dir` adds more
canonicalization and calls `package_for_dir` again ([types.rs:367](../../lib/src/filesystem/repo/types.rs#L367)).
When no explicit base is supplied, `resolve_dir` also calls `std::env::current_dir`
([filesystem/mod.rs:1444](../../cli/src/output/filesystem/mod.rs#L1444)). This is both a semantic
violation and package-count-dependent work on the default JSON path.

The Level-1 work-count test documents the old residual and explicitly ignores every
`filesystem.io.canonicalizations` count instead of asserting an empty report
([repo_json.rs:3105](../../cli/src/output/repo_json.rs#L3105)). That allowance was supposed to be
removed when R6 landed, but the new request-scoped ownership index was not carried into aggregate
projection. Precompute the context during observation, or pass a reusable normalized lookup/context
object to the projection; then assert `report.counters.is_empty()` and avoid `current_dir` inside the
builder. Level 1 is the correct verification tier for this JSON/work-count requirement, but the
current test protects the violation.

### High: the per-detection manifest store is not used by non-Cargo workspace discovery

R5.5 requires one per-detection store for Cargo, Node, Python, Go, lockfile, and root configuration
inputs, and acceptance requires each unique manifest to be parsed at most once per detail phase
([spec.md:184](spec.md#L184), [spec.md:384](spec.md#L384)). A request-scoped `ManifestStore` now exists,
and Cargo discovery correctly receives it. Node and uv workspace detectors do not: the detection
orchestrator calls them without the store ([detection.rs:491](../../lib/src/filesystem/repo/detection.rs#L491),
[detection.rs:512](../../lib/src/filesystem/repo/detection.rs#L512)). Their parsers directly open and
parse `package.json` and `pyproject.toml`
([npm.rs:364](../../lib/src/filesystem/repo/npm.rs#L364),
[uv.rs:65](../../lib/src/filesystem/repo/uv.rs#L65)).

This produces a concrete duplicate for uv: discovery parses the root `pyproject.toml`, uv always
adds the workspace root as a package seed ([uv.rs:44](../../lib/src/filesystem/repo/uv.rs#L44)), and
package identity/enrichment then asks `ManifestStore::pyproject` to open and parse the same path
again. Similar bypasses remain for Node workspace manifests. The new Level-1 regression covers
shared Cargo inheritance, not discovery-plus-enrichment in these ecosystems. Route workspace
parsing through typed, error-preserving store accessors and add Node and uv fixtures asserting one
open/parse per unique root manifest in both structure and full detail.

### High: subprocess execution is still neither universal nor end-to-end bounded

The maintained subprocess contract says every child goes through `run_with_timeout` and that a
child cannot wedge detection ([process.rs:1](../../lib/src/process.rs#L1)). Automatic Windows program
detection still bypasses it: the BurntToast probe invokes `pwsh` with unbounded `Command::output`
([metadata.rs:3292](../../lib/src/programs/enums/metadata.rs#L3292)). `Get-Module -ListAvailable`
can scan module paths, so this leaves a default Windows detection latency cliff despite the Phase 6
hardening.

The shared helper also does not enforce its deadline across a process tree. On timeout it kills and
waits for only the direct child ([process.rs:168](../../lib/src/process.rs#L168)), then unconditionally
joins pipe-drain threads ([process.rs:195](../../lib/src/process.rs#L195)). If a descendant inherited
stdout or stderr, killing the parent does not close that descendant's write end; `read_to_end` does
not reach EOF and the joins can block past the deadline. Existing Level-1 tests cover a single
sleeping child and large direct-child output, but not inherited handles or descendants
([process.rs:261](../../lib/src/process.rs#L261), [process.rs:295](../../lib/src/process.rs#L295)).

Move BurntToast to the shared policy deadline. Make timeout cleanup process-tree-aware on Unix and
Windows (process groups/job objects or an equivalently bounded design), and add portable Level-1
fixtures where a child starts a descendant that retains both pipes. Assert wall-clock bounds only as
the functional deadline contract, not as performance evidence. Level 2 and Level 3 are not
applicable to host-process execution.

### High: the canonical Level-1 suite is red and two snapshots depend on ambient host configuration

The specification requires canonical checks and cross-platform correctness
([spec.md:341](spec.md#L341), [spec.md:373](spec.md#L373)). In this review, `just test` passed all
1,630 `sniff-lib` tests but failed two `sniff-cli` tests after retries:

- `os_json_snapshot` retained the live kernel and OS patch fields in its supposedly normalized
  value ([snapshots.rs:96](../../cli/tests/snapshots.rs#L96)); the checked snapshot expects macOS
  26.5.1 while this host reports 26.5.2.
- `repo_aggregate_json_snapshot` expected `main`, but its fixture uses
  `git2::Repository::init` without an explicit initial branch
  ([snapshots.rs:146](../../cli/tests/snapshots.rs#L146)). This host's Git configuration creates
  `master`, changing the aggregate's branches, status, and worktree values before the assertion
  ([snapshots.rs:719](../../cli/tests/snapshots.rs#L719)).

These are Level-1 output-parity tests, which is the appropriate tier, but red tests are not
verification. Normalize volatile OS version fields to the stable contract being tested and create
Git fixtures with an explicit initial branch. Then retain green macOS/Linux/Windows Level-1 results
and work-count artifacts. The deferred record only describes workflows and says confirmation must
happen after merge; it does not attach a run for this implementation
([deferred-perf-tests.md:13](deferred-perf-tests.md#L13)).

### Medium: the specified Criterion workload families are still absent

The specification explicitly requires fourteen workload families, including formatting-only
deep/wide trees, integrated-versus-standalone observation, 2,000 mixed packages, more than 10,000
files, dirty-file size scaling, synthetic service sets, and filesystem case variants
([spec.md:345](spec.md#L345)). The current benches do not implement that matrix. For example, package
boundary refresh covers 10/100/500 packages, not structure detection at 100/500/2,000 mixed packages
([repo.rs:22](../../lib/benches/cases/repo.rs#L22)); dirty Git coverage varies file count but uses a
deliberately fixed minimal file size ([git.rs:1](../../lib/benches/cases/git.rs#L1)); and service
inventory observes the host init system rather than a large synthetic set
([inventory.rs:25](../../lib/benches/cases/inventory.rs#L25)).

The deferral file's statement that “the fixture families exist” is therefore inaccurate
([deferred-perf-tests.md:32](deferred-perf-tests.md#L32)). Work counters remain the primary acceptance
evidence and wall-clock Criterion numbers should come from a stable runner, but that doctrine does
not remove the specification's requirement to define these workloads. Add the missing fixture
families and counter assertions first; archive Criterion results where the specification also asks
for timing, or explicitly narrow the specification before declaring completion.

### Medium: phase and completion records still contradict the implemented source

Several feature records remain materially stale after the review-1 fixes. Phase 4 still says the
`ManifestStore` is lockfile-only and the ownership index is not built
([04-package-enrichment-and-ownership/spec.md:279](phases/04-package-enrichment-and-ownership/spec.md#L279));
Phase 8 still lists R5, R6.4, and R9.5/R9.6 as open
([08-cross-platform-validation/spec.md:236](phases/08-cross-platform-validation/spec.md#L236)); and
Phase 2 still carries the aggregate canonicalization allowance that should have been closed by R6
([02-reuse-and-scope/spec.md:262](phases/02-reuse-and-scope/spec.md#L262)). Some of those source
features now exist, while the narrower gaps above remain. Update the records to distinguish what
landed from what is still incomplete so future reviews and performance baselines do not rely on
obsolete implementation claims.

## Verification Levels

- Default/plain/JSON shape, stdout/stderr behavior, aggregate projection, request scoping, manifest
  parse counts, Git work bounds, remote request counts, inventory saturation, and subprocess
  deadlines are correctly verified at Level 1. The aggregate purity assertion is too permissive,
  non-Cargo manifest count fixtures are missing, process-tree coverage is missing, and two canonical
  CLI snapshots are currently red.
- Cross-platform verification means running the Level-1 suite on macOS, Linux, and Windows; a CI
  workflow definition is not a retained passing artifact. No such passing three-OS run for this
  implementation was available in the feature records.
- Level 2 is not required by this feature: it does not change terminal glyphs, widths, SGR styling,
  scrolling, or emulator behavior.
- Level 3 is not applicable: there are no keyboard, modifier, hotkey, paste, IME, or mouse
  requirements.

## Checks Run

```text
bf reference @sniff/features/2026-07-16-performance/spec.md
sniff repo packages --json
just test       # sniff-lib: 1630 passed, 6 skipped; sniff-cli: 776 passed, 2 failed
just lint       # passed
```

The review also traced every review-1 finding against the current implementation. The shallow
structure contract, shared Cargo inheritance parse, originally named Windows audio/route/timezone
probes, request-scoped ownership for inventory/docs/commit attribution, and focused Git ref/worktree
reuse are present. The findings above are remaining or newly exposed gaps rather than repetitions of
already-fixed items.

Production readiness: **not ready**.
