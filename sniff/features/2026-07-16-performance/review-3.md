---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T14:10:33-07:00
spec: 2026-07-16-performance/spec.md
log: sniff/features/2026-07-16-performance/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-3.md
---

# Review 3

## Findings

### High: the aggregate builder still reads manifests, and its purity test cannot see that path

R2.7 requires `build_aggregate_value` to be a pure projection with no filesystem reads, while the
completion boundary also requires every unique manifest to be parsed at most once per detail phase
([spec.md:131](spec.md#L131), [spec.md:383](spec.md#L383)). The builder calls
`aggregate_repo_version` ([repo_json.rs:742](../../cli/src/output/repo_json.rs#L742)), which delegates
to `bare_aggregate_version`; that calls `aggregate_versions`, whose documented implementation probes
and re-reads each package's Cargo, Node, or Python manifest
([aggregate.rs:303](../../lib/src/filesystem/repo/aggregate.rs#L303),
[aggregate.rs:325](../../lib/src/filesystem/repo/aggregate.rs#L325)). The new counter-silence test
sets `RepoAggregate.repo` to `None` ([repo_json.rs:3056](../../cli/src/output/repo_json.rs#L3056)), so
the version helper returns before doing any work and the empty-counter assertion is a false proof.

The observation path also retains the independent single-package fallback detection that R2.6
explicitly says to remove ([aggregate_view.rs:142](../../lib/src/filesystem/repo/aggregate_view.rs#L142)).
Carry version attribution and the standalone-package projection out of the original library
detection, then make the purity test use a populated multi-package `RepoInfo` whose manifest files
would fail the test if touched. The correct verification tier is Level 1; the current Level-1 test
does not exercise the relevant branch.

### High: bare aggregate JSON performs more than one status walk in a linked worktree

R2.3 and the acceptance criteria require exactly one status walk in total
([spec.md:127](spec.md#L127), [spec.md:382](spec.md#L382)). Bare `repo --json` still selects
`GitRequest::full()` ([commands/mod.rs:2122](../../cli/src/commands/mod.rs#L2122)), whose worktree
metadata path recomputes status for the current linked worktree
([remote_refresh.rs:914](../../lib/src/filesystem/git/remote_refresh.rs#L914),
[remote_refresh.rs:970](../../lib/src/filesystem/git/remote_refresh.rs#L970)). On this worktree,
`sniff --base sniff/lib --perf repo --json` recorded `git.status_walks: 2` with one repository
discovery. The existing counter test creates only a main-worktree repository with
`git2::Repository::init`, so it observes one walk and misses the linked-worktree case
([aggregate_view.rs:483](../../lib/src/filesystem/repo/aggregate_view.rs#L483)).

Use focused Git metadata controls for the aggregate or inject the already-collected current status
into worktree projection. Add a Level-1 linked-worktree counter fixture that asserts one total walk;
this is host observation, so Levels 2 and 3 are not applicable.

### High: subprocess detection still bypasses the deadline and the helper is not process-tree bounded

R12 requires covered subprocesses to remain bounded without pipe deadlocks
([spec.md:393](spec.md#L393)). The Windows BurntToast availability probe still invokes `pwsh` with
unbounded `Command::output()` ([metadata.rs:3295](../../lib/src/programs/enums/metadata.rs#L3295)),
contradicting the maintained `process` module contract that every Sniff child uses
`run_with_timeout` ([process.rs:1](../../lib/src/process.rs#L1)).

The helper itself kills and waits for only the direct child on timeout, then unconditionally joins
the stdout/stderr drain threads ([process.rs:173](../../lib/src/process.rs#L173),
[process.rs:195](../../lib/src/process.rs#L195)). A descendant that inherited either pipe keeps the
write end open, so those joins can block indefinitely after the advertised deadline. The Level-1
tests cover a direct sleeping child and large direct-child output, but no descendant retaining the
pipes ([process.rs:261](../../lib/src/process.rs#L261),
[process.rs:295](../../lib/src/process.rs#L295)). Move BurntToast to a named timeout and make cleanup
process-tree-aware on Unix and Windows, with a portable Level-1 descendant fixture. Terminal and
keyboard tiers are not relevant to this contract.

### High: the canonical Level-1 suite is still red and cross-platform completion is unproven

The acceptance criteria require output-parity tests and macOS/Linux/Windows tests to pass
([spec.md:395](spec.md#L395)). This review's `just test` run passed all 1,637 `sniff-lib` tests but
failed two of 778 `sniff-cli` tests after all retries:

- `os_json_snapshot` includes live `kernel`, `version`, and `long_version` fields in its normalized
  value ([snapshots.rs:96](../../cli/tests/snapshots.rs#L96)); this host reports macOS 26.5.2 while
  the snapshot pins 26.5.1.
- `repo_aggregate_json_snapshot` initializes a repository without selecting an initial branch
  ([snapshots.rs:146](../../cli/tests/snapshots.rs#L146)); the host defaulted to `master` while the
  snapshot pins `main`.

These are Level-1 process/snapshot tests, which is the appropriate tier, but failed tests are not
verification. Normalize the OS values to the stable behavior under test and create the Git fixture
with an explicit initial branch. No retained passing Linux/Windows run or three-OS work-count
artifact for this implementation is present in the feature records; a workflow definition describes
future coverage but does not satisfy the completion boundary.

### Medium: the Node/uv manifest-store fix does not cache failures or verify detection-level reuse

R5.5 and the verification strategy require one per-detection store and one parse per unique manifest
([spec.md:184](spec.md#L184), [spec.md:328](spec.md#L328)). Valid Node, pnpm, and uv workspace
discovery now routes through `ManifestStore`, but each new `required_*` accessor inserts only after a
successful parse ([detection.rs:260](../../lib/src/filesystem/repo/detection.rs#L260),
[detection.rs:298](../../lib/src/filesystem/repo/detection.rs#L298),
[detection.rs:326](../../lib/src/filesystem/repo/detection.rs#L326)). For example, Nx first calls
`collect_default_workspace_patterns`, which swallows a malformed `package.json` error
([detection.rs:1026](../../lib/src/filesystem/repo/detection.rs#L1026)); the later npm detector then
opens and parses the same invalid file again because no failure was cached.

The available tests call the tolerant `npm()` and `pyproject()` accessors twice without a performance
collector ([detection.rs:2103](../../lib/src/filesystem/repo/detection.rs#L2103),
[detection.rs:2134](../../lib/src/filesystem/repo/detection.rs#L2134)). They do not exercise
discovery-plus-enrichment for npm/pnpm/uv or assert one open/parse for the root manifest. Cache an
error-preserving result per path and add Level-1 detection fixtures for valid and malformed roots.

### Medium: aggregate context uses string-prefix area matching

R6 requires native, component-aware path comparisons rather than lossy strings or string prefixes
([spec.md:198](spec.md#L198)). The new aggregate context converts Git paths with `to_str()` and tests
area membership using `str::starts_with`
([aggregate_view.rs:238](../../lib/src/filesystem/repo/aggregate_view.rs#L238)). An area named `app`
therefore claims a change under `app2/...`; the root-area exclusion has the same collision. This can
incorrectly set the user-visible `is_current_package_area_dirty` and
`package_area_has_source_code_changes` fields. The new Level-1 fixtures use disjoint `alpha`/`beta`
names and do not cover a prefix collision. Compare `Path` components and add the collision fixture.

### Medium: the specified Criterion workload families remain absent

The specification still requires the workload matrix at
[spec.md:345](spec.md#L345). Current repository benchmarks measure pure-Cargo boundary refresh at
10/100/500 packages rather than mixed-ecosystem structure detection at 100/500/2,000
([repo.rs:20](../../lib/benches/cases/repo.rs#L20)); dirty Git varies file count but not the required
1 KiB/100 KiB/multi-megabyte file sizes ([git.rs:21](../../lib/benches/cases/git.rs#L21)); service
inventory observes the host init system instead of a large synthetic service set
([inventory.rs:25](../../lib/benches/cases/inventory.rs#L25)). The formatting-only, integrated versus
standalone, >10,000-file, large document-attribution, and filesystem-case workloads are also absent.

Work counters remain the primary regression evidence, but that does not make required workload
definitions optional. Add the missing fixtures and counter bounds before collecting timing on a
stable runner, or narrow the specification through a reviewed spec change.

### Medium: phase and completion records still describe superseded source

The feature records were not updated after the review fixes. Phase 4 still says the manifest store is
lockfile-only and the ownership index is not built
([04-package-enrichment-and-ownership/spec.md:281](phases/04-package-enrichment-and-ownership/spec.md#L281),
[04-package-enrichment-and-ownership/spec.md:301](phases/04-package-enrichment-and-ownership/spec.md#L301));
Phase 8 still lists R5, R6.4, and R9.5/R9.6 as open
([08-cross-platform-validation/spec.md:236](phases/08-cross-platform-validation/spec.md#L236)); and
Phase 2 still documents the now-removed aggregate canonicalization allowance
([02-reuse-and-scope/spec.md:262](phases/02-reuse-and-scope/spec.md#L262)). Update these records to
separate implemented work from the narrower remaining gaps above. Performance baselines and future
reviews should not need to guess whether source or a stale completion claim is authoritative.

## Verification Levels

| Requirement | Strongest present verification | Review result |
|---|---|---|
| Aggregate JSON schema, stdout/stderr, context, status-work bounds | Level 1 process, snapshot, and counter tests | Appropriate tier, but two snapshots are red; the purity fixture skips populated repos; the status fixture skips linked worktrees. |
| Request scoping, inventory saturation, Git work bounds, remote reuse, NTP policy, service batching | Level 1 unit/integration/work-count tests | Appropriate tier for non-terminal host observation. No Level-2/3 requirement applies. |
| Per-detection Node/pnpm/uv manifest reuse | Level 1 accessor unit tests | Appropriate tier, but no detection-level one-open/one-parse assertion and no malformed-manifest reuse test. |
| Subprocess deadlines and pipe draining | Level 1 direct-child tests | Appropriate tier, but missing BurntToast coverage and descendant/process-tree behavior. |
| macOS/Linux/Windows output and path parity | Level 1 on this macOS host only | Insufficient: the macOS canonical suite is red and passing Linux/Windows artifacts are absent. |
| Terminal glyphs, widths, SGR styling, scrolling | No new behavior in this feature | Level 2 is not required because these presentation contracts were not changed. |
| Keyboard, modifier, hotkey, paste, IME, mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages --json     # package catalog resolved successfully
just test                      # sniff-lib: 1637 passed; sniff-cli: 776 passed, 2 failed
just lint                      # passed
just build                     # passed
sniff --base sniff/lib --perf repo --json
                               # git.status_walks: 2; git.repository_discoveries: 1
```

The library run also retried one leaked-handle failure in
`error::tests::test_shorthand_not_found_display`; it passed on the second attempt, so it is recorded
as flakiness rather than a separate feature blocker. Test-generated `.snap.new` files were removed.

Review-2's aggregate context precomputation and valid Node/pnpm/uv store wiring are present, but the
remaining review-2 findings and the newly exposed aggregate work gaps above keep the feature from
meeting its completion boundary.

Production readiness: **not ready**.
