---
spike: S5
spec: 2026-11-06-windows-wsl-test-failures
question: Do the unexplained `darkmatter-cli` and `messenger` failures reproduce on `$BUILD_WIN`, and does an 8.3 short-name temp directory reproduce the `RUNNER~1` group?
answer: darkmatter-cli yes, deterministically (test bug, not intermittent); RUNNER~1 group yes, reproduced with an 8.3 TEMP on the rig's C: volume, but only 3 of its 5 rows are short-name failures; messenger pending (not yet run)
confidence: high (darkmatter-cli, RUNNER~1 group); messenger not determined
date: 2026-11-07
---

## Answer

Every row was rerun on `build-win-native` at `a816f1447`, the exact commit
the Nov 6 nightly (run 42246114963) tested. Each run used the long TEMP
spelling and the 8.3 short spelling of the same directory.

| Row (spec table) | Long TEMP | 8.3 TEMP | Cause, as reproduced | Verdict |
|---|---|---|---|---|
| `darkmatter-cli` `invalid_trigger_warnings`, 4 tests | 3 pass, 1 fails (3/3 runs) | all 4 fail (3/3 runs) | Mixed separators in the test's expected path, made unmatchable by the short name | **Test bug**, high. Deterministic, not intermittent |
| `tree-hugger-cli` `index_corruption`, 24 | 25/25 pass | 24 fail, same message as CI | Product reports the canonical long path; test expects the TEMP spelling | **Test bug**, high |
| `tree-hugger-cli` `ownership`, 96 | 96 fail | 96 fail | The fixture's relative symlink targets contain `/`, which Windows cannot resolve | **Test bug**, high. Not a short-name failure |
| `worktree` `remove::lifecycle`, 6 | 6/6 pass | exactly CI's 6 fail | Refusal names Git's long spelling; test expects the TEMP spelling | **Test bug**, high |
| `claudine-cli` `compose_schema_cli`, 2 | 2/2 pass | 2 fail | Product emits the launch spelling (short); test expects the canonical long path | **Test bug**, high |
| `darkmatter` `kind_semantic_checks`, 4 | 4 fail | 4 fail | String `ends_with("schemas/schema.yaml")` on a native `\` path | **Test bug**, medium (see below). Not a short-name failure |
| `messenger` `research_refresh` timed-out run, 1 | — | — | **Pending**: held until the shared Proxmox host is idle | Not determined |

The spec calls the `darkmatter-cli` failures "intermittent; cause unknown".
On the rig they are deterministic, and the cause is the same short-name
mechanism as the rest of the `RUNNER~1` group. On a development host with a
long TEMP, only `compose_reference_refusal_keeps_a_child_only_warning` fails,
and it fails there on every run. Why the spec saw intermittence is not
determined. The nightly log shows all four failing.

## Evidence

### Method

- The rig was `build-win-native` (`BUILD_WIN`); `BUILD_LINUX=build-linux` and
  `BUILD_WSL=build-win` were also declared, but not used here.
- `just cross-check` was **not** used. It ships the worktree's uncommitted
  files, and running it from a scratch clone prunes other worktrees' standing
  clones (`os` skill, build-hosts.md). Instead, a private clone was made at
  `B:\coding\a3-spike-s5\rusty-biscuit`, detached at `a816f1447`, with a
  clean tree (`dirty=0` on every run).
  - Tests ran through `just _test <pkg> --no-fail-fast <filters>` with
    `NEXTEST_PROFILE=ci`, from PowerShell over SSH, in native mode (not CI's
    archive mode).
  - No package declares CI features for these suites.
- Storage preflight: B: (ReFS, `CODING_DIR`) had 154 GiB free, and C: (NTFS)
  304 GiB. Another agent's cargo build (`shazam--feat-better-research`) was
  active on the rig at the start.
- **8.3 support:** `fsutil 8dot3name query` reports C: as **enabled**, and
  B: and W: as disabled. The rig's own TEMP (`C:\Users\ken\AppData\Local\Temp`)
  has no distinct short name, because `ken` is already short.
  - So one long-named directory was created on C:,
    `C:\Users\ken\AppData\Local\Temp\a3-spike-long-temporary`, whose
    generated short name is `C:\Users\ken\AppData\Local\Temp\A3-SPI~1`.
  - "Long" runs set `TEMP`/`TMP` to the first spelling and "short" runs to
    the second: the same directory, two spellings. That is the `RUNNER~1`
    situation, with the short component one level deeper.
- The failing test identities were taken from the nightly's failed-job logs
  (`gh run view --job <id> --log-failed`).

### darkmatter-cli `invalid_trigger_warnings`

```text
long  x3: FAIL compose_reference_refusal_keeps_a_child_only_warning
          PASS compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell
          PASS compose_lifecycle_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell
          PASS compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell
short x3: FAIL all four; panics at invalid_trigger_warnings.rs:1015:5 (three) and :470:9 (one), as in CI
          "message": "ignoring invalid schema trigger C:\\Users\\ken\\AppData\\Local\\Temp\\A3-SPI~1\\md-fixture-…\\cwd\\alpha\\schemas\\memory.yaml: …"
```

Mechanism (`darkmatter/cli/tests/l1/invalid_trigger_warnings.rs` at `a816f1447`):

- `write(root, "alpha/schemas/memory.yaml", …)` returns
  `root.join("alpha/schemas/memory.yaml")` (`:26-30`). On Windows that is
  `…\cwd\alpha/schemas/memory.yaml`, with mixed separators. `root` is
  `CliProcessFixture::cwd()`, which keeps TEMP's spelling
  (`darkmatter/cli/tests/common/fixture.rs:215-232`).
- The product reports the trigger with native separators, in TEMP's
  spelling.
- The check at `:1012-1015` accepts either `trigger` or
  `canonicalize_simplified(trigger)`. The raw form never matches, because of
  its mixed separators. The canonical form matches only when TEMP is already
  long, because canonicalizing re-spells `A3-SPI~1`/`RUNNER~1` to the long
  name. That explains pass-long and fail-short.
- The check at `:470` compares against `trigger.display()` only, so it fails
  on every Windows host.
- The product's message is consistent with the path it was given. The
  expectation is what is wrong.

### tree-hugger-cli

- `index_corruption`, under the short TEMP: 24 FAIL, matching CI's 24.
  - Panic at `index_corruption.rs:35:21`: "repository context". The product's
    error names `C:\Users\ken\AppData\Local\Temp\a3-spike-long-temporary\.tmp…\linked`
    (long). The test expects `root.display()` (short).
  - Under the long TEMP, all 25 pass.
  - Incidental: gix-index 0.52 panics with "attempt to subtract with
    overflow" (`gix-index-0.52.0/src/file/init.rs:73:67`) on a truncated
    index, in debug builds. The product catches it and reports "Git index
    decoder panicked on malformed input". This happens in both modes and is
    what the test expects.
- `ownership`: 96 FAIL in **both** modes. The 48 passing tests are the
  `*_aliases_are_rejected_*` and `*_cannot_return_*` families. Panic at
  `ownership.rs:260:5`:
  `left: "…\\alias.rs"  right: "…\\clean.rs"`.
  `file_identity(alias.rs)` returned the link's own path, because the link
  did not resolve.
  - The fixture creates links with `/` targets (`Path::new("inner/../clean.rs")`,
    `Path::new("inner/subdir")`, `ownership.rs:237-250`).
  - A direct `CreateSymbolicLinkW` probe on the rig (flag
    `ALLOW_UNPRIVILEGED_CREATE`) shows the cause:

  ```text
  file symlink target='inner/../clean.rs'   created=True readable=False  "The filename, directory name, or volume label syntax is incorrect."
  file symlink target='inner\..\clean.rs'   created=True readable=True
  file symlink target='inner/./../clean.rs' created=True readable=False
  dir  symlink target='inner/subdir'        created=True listable=False
  dir  symlink target='inner\subdir'        created=True listable=True
  ```

  Windows stores a relative target that contains `/` without complaint, but
  cannot follow it. The CI message shows a `RUNNER~1` versus `runneradmin`
  difference only because the unresolved link keeps the raw TEMP spelling.
  Short names are not the cause.

### worktree `remove::lifecycle`

- Long TEMP: 64/64 pass, including `remove::lifecycle::` and `git::program::`.
- Short TEMP: exactly CI's six fail, at `tests.rs:1245`:
  `C:/Users/ken/AppData/Local/Temp/a3-spike-long-temporary/.tmp…/wts/gone changed after the question…`.
  - `assert_changed` normalizes separators (`:1243-1244`), but not the
    short/long spelling of `path`.

### claudine-cli `compose_schema_cli`

- Long TEMP: 2/2 pass. Short TEMP: both fail at `compose_schema_cli.rs:1276`.
- The product emits
  `Save the plan as "C:/Users/ken/AppData/Local/Temp/A3-SPI~1/…/plan.md"`,
  which is the launch spelling. The test expects
  `canonicalize_simplified(...)`, the long spelling (`:1268-1275`).
- This is the "reversed" case the spec names. Per the `os` skill
  (windows-paths.md item 6), the test should model the spelling the child
  reports.

### darkmatter `kind_semantic_checks`

- 4 FAIL in both modes, at `kind_semantic_checks.rs:163:9`, with
  `control.definition` = `C:\Users\ken\AppData\Local\Temp\a3-spike-long-temporary\.tmp…\repo\simplified-schema\schemas\schema.yaml`.
- The assertion is a string `ends_with("schemas/schema.yaml")`.
- `YamlValidation::definition` is documented only as "the definition's
  label, or the `$schema` it named"
  (`darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs:60-61`). No
  separator spelling is promised.
- **Test bug** if the label is native, as the `os` skill says caller-supplied
  file values are. **Product bug** if the label is meant to be portable.
  Hence medium confidence: the field's contract decides it.

### messenger — pending

Not run, per instructions; the test is timing-sensitive and the Proxmox host
is shared with an active `build-linux` build. What the log and code show so far:

- CI failed after 4.377 s at `research_refresh.rs:1467`, "the grandchild
  started".
- The test gives the Windows run `max_seconds = 2` (`:1457`). It needs a
  PowerShell grandchild to write its readiness file before the runner's
  2-second limit stops it, and it waits up to 4 s for that file (`:1467`).
- That is a cold-start latency race: plausibly flaky. The repetition
  (about 10 runs on an idle host) is still to do. The scratch clone is kept
  on the rig for it; messenger is not yet built there.

## What this changes in the spec

- **Known failures table:**
  - `darkmatter-cli` changes from "Intermittent; cause unknown /
    Unexplained" to "mixed separators plus 8.3 TEMP; deterministic / Test
    bug". Three of its tests belong to the `RUNNER~1` group; one fails on
    every Windows host.
  - `tree-hugger-cli` `index_corruption` is confirmed as a test bug.
  - `tree-hugger-cli` `ownership` is confirmed as a test bug, but its
    apparent cause is the `/` symlink target, not short names. The `RUNNER~1`
    text in its message is a side effect.
  - `worktree` lifecycle and `claudine-cli` are confirmed as test bugs.
  - `darkmatter` `kind_semantic_checks` is a separator comparison, not short
    names. It stays "probably test bug" until the `definition` label's
    spelling is decided.
- **"Spike needed":** answered. A pass on `$BUILD_WIN` can now clear a
  `RUNNER~1` failure, as long as the run sets `TEMP`/`TMP` to an 8.3 spelling
  of a long-named directory on **C:**. B: (the `CODING_DIR` volume) cannot
  produce short names.
- **Outcome, "one shared helper compares paths in canonical form":** that
  covers only part of the work. Three distinct defects need three fixes:
  1. **Short versus long spelling** (index_corruption, worktree lifecycle,
     claudine-cli, darkmatter-cli's 3): a canonical compare fixes these.
     Note that claudine-cli's product emits the *launch* spelling and
     tree-hugger and worktree emit the *canonical* one, so the helper must
     canonicalize both sides, not pick one.
  2. **Separators** (darkmatter `kind_semantic_checks`, darkmatter-cli
     `:470`): a fixture that joins `"a/b/c"` onto a `Path` makes mixed
     separators. Component-wise comparison, or building fixtures from
     components, fixes these.
  3. **Fixture symlinks with `/` targets** (tree-hugger ownership, 96
     tests): this is not a comparison at all. The fixture must create links
     with native separators on Windows, or the links are broken.

## Surprises

- **The rig can reproduce the `RUNNER~1` group**, contrary to the spec's
  caution. This is an `os` skill fact worth recording in windows-paths.md
  item 6:
  - B: (ReFS) and W: have 8.3 creation disabled, and C: (NTFS) has it
    enabled.
  - A long-named directory under `C:\Users\ken\AppData\Local\Temp` gets a
    short name.
  - Point `TEMP`/`TMP` at its `%~sI` spelling.
- **A new OS fact** for the `os` skill, which the fix should record: a
  relative Windows symlink whose target contains `/` is created without
  error, but cannot be followed. File reads fail with error 123
  (`ERROR_INVALID_NAME`), and a directory link cannot be listed.
- The largest row (96 of the 141 failures) is not a path-comparison problem.
  That moves the "one shared helper" seam (see above).
- The gix-index 0.52 subtraction-overflow panic on a truncated index is a
  dependency defect that the product already contains. It is not a failure
  here, but it is a debug-only panic path worth knowing about.
