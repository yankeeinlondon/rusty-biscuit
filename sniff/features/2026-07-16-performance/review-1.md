---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T05:58:51-07:00
spec: 2026-07-16-performance/spec.md
implemented: false
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-1.md
previous: /
---

# Review 1

## Findings

### High: `RepoRequest::structure()` still performs the enrichment the specification forbids

R5 requires structure mode to stop after membership and minimum package identity parsing, leaving
dependencies, test runners, features, languages, frameworks, and file lists empty
([spec.md:183](spec.md#L183)). Instead, both modes merge seeds and call the same
`create_package_from_seed` path ([detection.rs:461](../../lib/src/filesystem/repo/detection.rs#L461)).
That path calls `create_package`, which probes package managers, detects test runners, parses features,
and parses Cargo, Node, Python, and Go dependencies
([detection.rs:1386](../../lib/src/filesystem/repo/detection.rs#L1386)). The structure/full work-count
test then asserts that both modes perform the same number of package enrichments
([detection.rs:2269](../../lib/src/filesystem/repo/detection.rs#L2269)); it protects the current
behavior rather than the specified shallow contract.

This is user-visible as both a performance contract violation and a request-semantics violation. The
phase record explicitly says the change was blocked because three CLI commands currently depend on
accidentally enriched structure results
([04-package-enrichment-and-ownership/spec.md:151](phases/04-package-enrichment-and-ownership/spec.md#L151)).
Resolve that API decision rather than keeping the accidental coupling: use a focused detail request
for package managers/dependencies/test runners, migrate those CLI commands, and add an L1 work-count
test proving structure mode performs zero enrichment-only probes/parses.

### High: the manifest cache is per package, not per detection

R5.5–R5.6 require one request-scoped `ManifestStore` and at most one parse of a shared root
`Cargo.toml`, lockfile, or root-scoped configuration input
([spec.md:184](spec.md#L184)). `ManifestCache` is instead created afresh by every
`PackageBuildContext` and is explicitly documented as being cleared for each package
([detection.rs:242](../../lib/src/filesystem/repo/detection.rs#L242)). Workspace-inherited versions
also bypass it and call `read_toml_at` directly
([cargo.rs:243](../../lib/src/filesystem/repo/cargo.rs#L243)), so a workspace whose members use
`version.workspace = true` reopens and reparses the root manifest for every member.

The existing cache tests exercise repeated access through one standalone cache; they do not measure a
multi-member detection. Add the request-scoped store, route inherited-version and root test-runner
configuration reads through it, and add a fixture with several inheriting Cargo members that asserts
one root-manifest open/parse for the whole detection. Current strongest verification is Level 1, but
it verifies only the weaker per-package behavior.

### High: Windows subprocess hardening still contains the pipe-deadlock pattern and unbounded probes

The new shared helper is intended to drain pipes concurrently and bound subprocess waits
([spec.md:259](spec.md#L259)). Windows audio detection still polls `try_wait()` while PowerShell's
stdout is piped but undrained ([audio.rs:809](../../lib/src/hardware/audio.rs#L809)); enough device
output can fill the pipe, block PowerShell before exit, and turn a valid result into a timeout and an
empty device list. Windows default-route detection still uses unbounded `Command::output()`
([network/mod.rs:969](../../lib/src/network/mod.rs#L969)), and timezone detection does the same for
`tzutil` ([os/time.rs:228](../../lib/src/os/time.rs#L228)). These paths also contradict the maintained
skill's claim that every subprocess goes through `process::run_with_timeout`.

Move these detection probes onto the shared helper with named policy deadlines. Add Level-1 Windows
tests using injected short deadlines and manufactured output larger than a pipe buffer; parser-only
tests and cross-compilation do not exercise the failure mode. Level 2 and Level 3 are not applicable
because this is host-process observation, not terminal rendering or OS keyboard encoding.

### Medium: the shared ownership index required by R6.4 was not implemented

The specification requires one normalized deepest-prefix index shared by inventory, documents, and
commit attribution ([spec.md:196](spec.md#L196)). `RepoInfo::package_for_dir` still canonicalizes the
query and every candidate package during each lookup
([types.rs:273](../../lib/src/filesystem/repo/types.rs#L273)), while other consumers retain separate
ownership logic. The final counter record confirms that canonicalizations stayed at 600 and attributes
the missed reduction to the unimplemented R6 work
([08-cross-platform-validation/spec.md:163](phases/08-cross-platform-validation/spec.md#L163)).

Current L1 tests establish correct deepest-prefix behavior, but there is no work-count test proving
shared lookup reuse. Build the request-scoped component-aware index, route all three consumers through
it, and retain native `PathBuf`/Windows-prefix fixtures.

### Medium: focused Git requests still repeat ref and worktree observation

R9.5 requires remote-tracking tips to be reused within a request, and R9.6 requires worktree listing to
avoid reopening every linked worktree when metadata is sufficient
([spec.md:227](spec.md#L227)). Branch detection builds one remote-tip map, but tracking status separately
iterates remote names and peels matching refs ([remote_refresh.rs:326](../../lib/src/filesystem/git/remote_refresh.rs#L326));
`GitRepo::detect_with_request` calls those paths independently
([types.rs:986](../../lib/src/filesystem/git/types.rs#L986)). Worktree enumeration still opens each
linked worktree as a full repository
([remote_refresh.rs:809](../../lib/src/filesystem/git/remote_refresh.rs#L809)). The Phase 5 record
explicitly marks both requirements and their counter tests as not done
([05-git-observation/spec.md:294](phases/05-git-observation/spec.md#L294)).

These are performance rather than result-correctness failures, so Level-1 counter tests are the right
verification level. Add a request-scoped ref snapshot and direct worktree-metadata projection, then
assert one ref observation and no unnecessary worktree opens for focused requests.

### Medium: the completion evidence is red and cross-platform results for this implementation are not attached

The specification requires the canonical checks to pass and macOS/Linux/Windows correctness to be
demonstrated ([spec.md:373](spec.md#L373), [spec.md:396](spec.md#L396)). In this review, `just test`
finished 1,603/1,604 with `detect_area_errors_when_not_in_repo` timing out on all four attempts. The
failure is recorded as pre-existing, but the acceptance gate remains red. `just lint` passed.

The implementation plan says the current host is macOS-only and relies on future scheduled artifacts
([plan.md:493](plan.md#L493)); Phase 8 expressly states that the umbrella completion boundary is not
met ([08-cross-platform-validation/spec.md:236](phases/08-cross-platform-validation/spec.md#L236)). Before
production readiness, make the L1 suite deterministic, run the changed implementation on all three OS
legs, and retain the work-count artifacts. The required Criterion fixture families and `just bench`
were also deferred; either add/run them or narrow the specification's verification commitments.

## Verification Levels

- Aggregate/default/plain/JSON CLI projection and valid-JSON-only stdout: Level 1 process and snapshot
  coverage is present and appropriate. No input-encoder behavior is involved.
- Inventory truncation fields, focused Git results, bounded history, remote snapshots, WAN fallback,
  default NTP policy, and service batching: Level 1 unit/integration/work-count coverage is the
  appropriate level.
- Existing terminal styling: Level 2 tests are present and the recorded `just test-l2` run passed 2/2.
  This feature does not change styling, glyph width, scrolling, or terminal control sequences.
- Level 3: not applicable; there are no keyboard, modifier, paste, IME, or mouse requirements.
- Windows audio/route/timezone subprocess behavior: Level 1 is the correct tier, but the necessary
  timeout and large-output tests are absent, as described above.

## Checks Run

```text
sniff repo packages --json
just test       # 1603 passed, 1 timed out, 4 skipped
just lint       # passed
git diff --check # unrelated pre-existing trailing whitespace in prompts/plan.md
```

No previous review exists for iteration 1, so there was no previous-review frontmatter to update.
Production readiness: **not ready**.
