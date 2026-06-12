---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: The mandatory performance gate still fails

The checked-in comparison still reports regressions in discovery, three revwalk
variants, commit-file diffs, and eight-worktree fan-out
(`sniff/lib/baselines/gix.md:57-74`). The largest is
`git_ops/diff_commit_files` at `+235.15%`. The specification requires every
benchmark ID to avoid a statistically significant regression before release
(`spec.md:496-511`).

The comparison also remains methodologically incomplete: the git2 baseline was
captured with reduced sampling and explicitly requires a default-sampling
recapture before a release decision (`sniff/lib/baselines/git2.md:29-34`), while
the gix baseline used default sampling.

Recapture both implementations on the same host with identical default
Criterion settings, then optimize every remaining regression or explicitly
revise the specification.

### High: Branch and tracking APIs still suppress repository failures

Iteration 4 made the shared revwalk helpers fallible, but their other production
callers immediately discard those errors:

- local-branch ahead/behind failures become `(0, 0)`
  (`remote_refresh.rs:190-193`);
- tracking behind/ahead failures become `0`
  (`remote_refresh.rs:247-250`);
- ref-store, ref-iteration, and peel failures are converted to empty or omitted
  branch/tracking data (`remote_refresh.rs:161-181`, `228-243`, `275-284`).

`GitRepo::branches()`, `GitRepo::tracking_status()`, and the full detection path
therefore report valid-looking metadata for corrupt or unreadable repositories
(`types.rs:648-657`, `759-768`). This violates the spec's rule that infallible
convenience accessors may suppress errors only by delegating to a documented
fallible query (`spec.md:358-365`).

Add fallible branch and tracking queries that propagate operation-tagged errors.
The existing convenience methods may delegate and document suppression if API
compatibility requires them. Add corrupt-object/ref fixtures through both the
fallible APIs and `detect_with_request`.

### High: Linux and Windows release verification remains absent

The migration requires macOS, Linux, and Windows builds and parity tests
(`spec.md:49-52`, `650-652`). The repository's general test and sanity workflows
run only on `ubuntu-latest`; no Windows sniff job exists. The review evidence is
macOS-only, and this session cannot run Rust tests because rustup has no
installed toolchain.

Add or record successful sniff library and CLI runs on Linux and Windows. The
Windows run is especially necessary for worktree paths, non-Unix config paths,
permission handling, and the new cross-platform corruption fixtures.

### Medium: Cache sizing adds index I/O to every repository open

`trusted_discover` and `trusted_open` now call `configure_cache`, which loads the
index solely to size an object cache (`open.rs:32-38`, `50-59`, `69-73`). This
affects root-only discovery, config reads, minimal status requests, and other
paths that may never decode repeated objects. Discovery is already a recorded
regression.

The specification recommends cache sizing for repeated object access in
revwalk, diff, and ref enumeration, not as unconditional open-time work
(`spec.md:519-525`). Move cache configuration to those object-intensive paths,
or initialize it lazily when such a request is selected, then remeasure
`git_ops/discover` and the request-level benchmarks.

### Medium: The 12-key fallback test is not isolated from parallel tests or system config

`extra_system_config_fallback_reads_all_12_keys` mutates `HOME` and
`GIT_CONFIG_GLOBAL` process-wide but uses a function-local mutex
(`remote_refresh.rs:1212-1217`, `1260-1283`). That mutex does not serialize it
with the config tests in `git_parity.rs`, which mutate the same environment
variables under a different lock. A panic before manual restoration also leaks
the modified environment.

The test additionally leaves system config enabled, so its claim that the
injected fallback is the only source is host-dependent. Use the shared
`EnvGuard` plus `#[serial_test::serial]`, and isolate both global and system
config with temporary files. This is L1 coverage, but it is not currently a
reliable cross-platform verification of the fallback contract.

## Verification Levels

All user-observable requirements in this migration concern repository and CLI
data behavior, so Level 1 is appropriate. No terminal rendering or keyboard
behavior requires Level 2 or Level 3.

| Requirement | Required | Strongest evidence | Result |
|---|---|---|---|
| Discovery, status, diff, history, refs | L1 on macOS/Linux/Windows | macOS parity artifacts; Linux-only generic CI | Windows gap |
| Worktree error propagation | L1 | Corrupt index/object helper tests | Present for worktrees |
| Branch/tracking error propagation | L1 | Success parity tests | Gap |
| Platform config layering | L1 on macOS/Windows | Injectable 12-key unit test | Test isolation gap; no Windows run |
| CLI output parity | L1 integration on all targets | No recorded Windows run | Gap |
| Performance | Criterion, outside L1-L3 | Same-host record with mismatched sampling | Failed |

## Verification

- Reviewed the specification, plan, prior review, iteration-4 changes,
  production Git paths, parity tests, workflows, and benchmark records.
- `git diff --check` passes.
- Production `git2` remains confined to dev dependencies; CLI production source
  has no `git2` or `gix` imports.
- `cargo fmt`, tests, Clippy, and doctests could not run because rustup reports
  no installed toolchains in this session.

## Decision

Not ready for production. Iteration 4 fixes worktree post-open error propagation,
documents the breaking API change, and updates dependency documentation, but the
hard performance and three-platform gates remain open. Branch/tracking error
suppression and the unconditional open-time cache work also require correction.
