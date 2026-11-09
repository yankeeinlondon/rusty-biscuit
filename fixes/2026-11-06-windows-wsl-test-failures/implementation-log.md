---
spec: "/Volumes/coding/rusty-biscuit/fixes/2026-11-06-windows-wsl-test-failures/spec.md"
plan: "fixes/2026-11-06-windows-wsl-test-failures/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
decisions_for_review:
  - |-
    Build-host roles. The plan recorded that only `BUILD_WSL=build-win` was
    declared at planning time and that native Windows had no destination. On
    2026-11-09 no `BUILD_*` variable was set in this session, and probing the
    aliases in `~/.ssh/config` with `BatchMode=yes` showed that `build-win` is
    native Windows (PowerShell, Git 2.55.0.windows.3, `CODING_DIR=B:\coding`),
    `build-wsl` is the WSL2 Ubuntu guest on the same machine (no cargo), and
    `build-linux` is native Linux. The alias `build-win-native` used by the S5
    spike no longer resolves. I decided later phases pass
    `BUILD_WIN=build-win BUILD_WSL=build-wsl BUILD_LINUX=build-linux` per
    invocation, never written into repository scripts. Alternative: treat
    native Windows as undeclared, as the plan's note implied, which would
    leave every Windows reproduction unverified. Nothing needs redoing if a
    reviewer chooses different aliases, other than rerunning the evidence on
    the hosts they name.
  - |-
    Messenger classification. The spec's table calls the messenger failure
    "probably flaky" and S5 called it a plausible cold-start race. The hosted
    nightly logs show the same assertion (`research_refresh.rs:1467`, "the
    grandchild started") failing in all four nightlies at `a816f1447`
    (Nov 6, 7, 8, 9; 4.4–5.5 s each), so I recorded it as deterministic on
    the hosted runner. That changes no rule: plan rule 12 already replaces
    the shell-startup race at the fixture, with no retries or larger
    timeouts. The single before/after rig capture planned for Phase 2 is
    still required for the child status and output. Alternative: keep
    "intermittent" until the rig capture; that would understate a failure
    that has never passed on the hosted runner.
message_to_agent: |-
  Phase 1 changed no code. The full 141-row ledger, excerpts, and contract
  clarifications are under "## Phase 1" in this log. Points for Phase 2:
  (1) No BUILD_* variables are set in the session environment. Pass them per
  command: BUILD_WIN=build-win (native Windows, PowerShell, Git 2.55 mingw64
  only, CODING_DIR=B:\coding), BUILD_WSL=build-wsl (toolchain-free WSL2
  guest), BUILD_LINUX=build-linux. The S5 alias build-win-native no longer
  resolves. Always use ssh -o BatchMode=yes.
  (2) No affected source file has changed since a816f1447, so no earlier
  repair exists. Only .github/workflows/_package-ci.yml has moved (the
  archive-disk prerequisite's edits on this branch); insert the rule 10
  fetch step into the current file and keep those edits.
  (3) The rule 11 fix must clear E0283 as well as E0284 (same line;
  encode_unicode adds a Vec<u16> FromIterator impl on Windows).
  parse::<u16>() fixes both.
  (4) The messenger runner is messenger/lib/src/research/refresh/runner.rs.
  The test failed deterministically in 4/4 hosted nightlies.
  (5) 8.3 short-TEMP reproduction works only on the rig's C: volume: use a
  long-named directory under C:\Users\ken\AppData\Local\Temp and its %~sI
  spelling (S5). B: cannot produce short names.
  (6) The ucrt64 parity evidence needs a disposable Git for Windows >=2.56
  install. Do not upgrade the rig's shared install.
  (7) WSL2 archives are still unavailable on main: the prerequisite is
  implemented on this branch but not merged. Every nightly fails all 66 WSL2
  cells on "Artifact not found".
---

# Implementation Log for 2026-11-06-windows-wsl-test-failures (4 phases)

## Phase 1

Phase 1 establishes evidence and contracts only. No source, documentation,
or skill file was changed, no Cargo test ran, and no standing clone or host
installation was touched.

### Nightly under repair

- Run: [42246114963](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963),
  scheduled, 2026-11-06T15:46Z, commit `a816f14473c1a63737c79aeaf414222bb0a83706`
  (`a816f1447`, merge of PR #145). Conclusion: failure.
- `main` has not moved since. The nightlies of Nov 4, 5, 7, 8 and 9
  (`41863490340`, `42062626209`, `42383053581`, `42491863852`,
  `42674352356`) all ran `a816f1447` and failed the same nine Windows test
  cells, both `build` jobs, and 66 WSL2 cells.
- Excerpts were retrieved read-only with `GIT_TERMINAL_PROMPT=0 gh run view
  42246114963 --job <id> --log-failed`.

Failed Windows jobs in run 42246114963:

| Job | Cell | Nextest summary |
|---|---|---|
| [127999511428](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/127999511428) | `build (windows-latest)` | compile failure, see below |
| [128082100036](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100036) | `biscuit-visualized` L1 windows-latest | no tests: `Artifact not found for name: build-biscuit-visualized-windows-latest-526eec3a39162ec4` (consequence of the compile failure) |
| [128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) | `tree-hugger-cli` | 2722 run: 2602 passed, 120 failed, 31 skipped |
| [128082096885](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096885) | `messenger` | 676 run: 675 passed, 1 failed, 2 skipped |
| [128082098594](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082098594) | `claudine-cli` | 3864 run: 3862 passed, 2 failed, 13 skipped |
| [128082099316](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082099316) | `model_id` | 2 run: 1 passed, 1 failed |
| [128082100322](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100322) | `git-wt-cli` | 567 run: 566 passed, 1 failed, 43 skipped |
| [128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) | `worktree` | 849 run: 841 passed, 8 failed |
| [128082100405](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100405) | `darkmatter` | 9760 run: 9756 passed, 4 failed, 66 skipped |
| [128082100564](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100564) | `darkmatter-cli` | 1483 run: 1479 passed, 4 failed, 69 skipped |

### Reconciliation with the spec's table

The nextest `FAIL` lines give 120 + 1 + 2 + 1 + 1 + 8 + 4 + 4 = **141**
distinct test identities, plus the separate compile failure. No
parameterized control is counted: each identity is a distinct nextest test
(the `ownership` and `index_corruption` cases are macro-generated tests, one
per row), and the 48 passing `ownership` alias-rejection tests and the
passing `index_corruption` case are excluded.

| Spec row | Spec count | Nightly count | Reconciled cause (S5 / S5b) |
|---|---|---|---|
| `biscuit-visualized` compile | 1 | 1 | E0284 **and** E0283 at `embed_render_tests.rs:450:66`; Windows only because `encode_unicode` (Windows dependency graph) adds `FromIterator<Utf16Char> for Vec<u16>`, making `parse()`'s target ambiguous |
| `tree-hugger-cli` `index_corruption` | 24 | 24 | 8.3 TEMP spelling; test bug |
| `tree-hugger-cli` `ownership` | 96 | 96 | `/` in relative symlink targets; not a short-name failure; test bug |
| `worktree` remove lifecycle | 6 | 6 | 8.3 TEMP spelling in `assert_changed`; test bug |
| `claudine-cli` | 2 | 2 | product emits launch (short) spelling, test expects canonical long; test bug |
| `darkmatter` `kind_semantic_checks` | 4 | 4 | string suffix on a native `\` label; test bug once rule 6 fixes the label as native (it is `path.display()`) |
| `model_id` `ui` | 1 | 1 | trybuild offline nested build lacks `proc-macro2` sources; test environment plus an early-return that hides missing Cargo |
| `worktree` + `git-wt-cli` Git layout | 3 | 2 + 1 | PATH names the prefix binary directly (S5b); a product gap in direct-entry recognition, separate from the ucrt64 prefix gap |
| `messenger` | 1 | 1 | grandchild readiness race; deterministic on hosted runner (4/4 nightlies), see decisions |
| `darkmatter-cli` | 4 | 4 | **reclassified from "unexplained"**: mixed separators + 8.3 TEMP, deterministic (S5); test bug |
| **Total** | **141** | **141** | |

### Compile failure excerpt

From `build (windows-latest)`, job 127999511428, command
`cargo test --no-run --message-format=json-render-diagnostics --package=biscuit-visualized --all-features --profile=test --target=x86_64-pc-windows-msvc`
(toolchain 1.98.1):

```text
error[E0284]: type annotations needed
   --> biscuit-visualized\src\src\tests\embed_render_tests.rs:450:66
450 |         .map(|rest| rest[..rest.find(';').expect("weight ends")].parse().expect("a weight"))
    |                                                                  ^^^^^ cannot infer type of the type parameter `F` declared on the method `parse`
    = note: cannot satisfy `<_ as FromStr>::Err == _`
error[E0283]: type annotations needed
   --> biscuit-visualized\src\src\tests\embed_render_tests.rs:450:66
451 |         .collect()
    |          ------- type must be known at this point
    = note: multiple `impl`s satisfying `Vec<u16>: FromIterator<_>` found in the following crates: `alloc`, `encode_unicode`:
            - impl FromIterator<encode_unicode::utf16_char::Utf16Char> for Vec<u16>;
            - impl<'a> FromIterator<&'a encode_unicode::utf16_char::Utf16Char> for Vec<u16>;
            - impl<T> FromIterator<T> for Vec<T>;
error: could not compile `biscuit-visualized` (lib test) due to 2 previous errors
```

Rule 11's `parse::<u16>()` resolves both errors: it fixes `F`, and the
collected item type then becomes `u16`.

### model_id excerpt

Job 128082099316, `model_id::ui ui`, 0.403 s:

```text
error: no matching package named `proc-macro2` found
location searched: crates.io index
required by package `model_id-tests v0.0.0 (D:\a\rusty-biscuit\rusty-biscuit\target\tests\trybuild\model_id)`
note: offline mode (via `--offline`) can sometimes cause surprising resolution failures
thread 'ui' (6368) panicked at C:\Users\runneradmin\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\trybuild-1.0.116\src\run.rs:62:13:
tests failed
```

### messenger excerpt

Job 128082096885, 4.377 s:

```text
thread 'a_timed_out_run_settles_its_descendants_before_releasing_the_fleet_lock' (4916) panicked at messenger\lib\tests\research_refresh.rs:1467:9:
the grandchild started
```

The same panic recurs in nightly jobs 128465115390 (Nov 7, 4.530 s),
128803985100 (Nov 8, 4.584 s) and 129449387320 (Nov 9, 5.522 s). The CI log
does not carry the child's exit status or output; Phase 2 captures them once
on the rig, per rule 13.

### Imported reproduction results

From `spikes/S5-windows-repro.md` (native Windows rig, private clone at
`a816f1447`, `just _test` with `NEXTEST_PROFILE=ci`, long TEMP
`C:\Users\ken\AppData\Local\Temp\a3-spike-long-temporary` versus its 8.3
spelling `…\A3-SPI~1`; B: has 8.3 creation disabled, C: enabled):

- `darkmatter-cli` `invalid_trigger_warnings`: long 3 pass / 1 fail
  (`compose_reference_refusal_keeps_a_child_only_warning`), short 4 fail;
  3/3 runs each. Panics at `:1015:5` and `:470:9`.
- `tree-hugger-cli` `index_corruption`: long 25/25 pass, short 24 fail at
  `index_corruption.rs:35:21`. The product names the long path.
- `tree-hugger-cli` `ownership`: 96 fail in both modes at `ownership.rs:260:5`.
  A `CreateSymbolicLinkW` probe shows a `/`-containing relative target is
  created but cannot be followed (error 123); a `\` target works.
- `worktree` `remove::lifecycle`: long 64/64 pass (with `git::program::`),
  short exactly the CI six fail at `tests.rs:1245`.
- `claudine-cli` `compose_schema_cli`: long 2/2 pass, short 2 fail at
  `:1276`; product emits the launch spelling.
- `darkmatter` `kind_semantic_checks`: 4 fail in both modes at `:163:9`.
- Incidental: gix-index 0.52 debug-build subtraction-overflow panic on a
  truncated index; the product already catches it and the test expects it.

From `spikes/S5b-ucrt64.md`:

- Git for Windows v2.56.0 (2026-09-28) replaced `mingw64` with `ucrt64` on
  x86_64. Hosted runner: 2.56.0.windows.4 (`ucrt64`). Rig: 2.55.0.windows.3
  (`mingw64` only; reconfirmed 2026-11-09).
- The three Git failures reproduce on the rig by putting `mingw64\bin` first
  on `PATH`, and pass with `cmd` first: `program.rs:285`, `program.rs:310`,
  `list_output.rs:94` (`left: 1, right: 0`). The cause is direct-binary
  entry, so adding `ucrt64` to `PREFIXES` alone would not clear them; rules
  7–8 cover both.
- `wt list` output is identical in every arrangement; only cost differs
  (+33% direct entry, +70% unrecognized launcher). No new timing study is
  planned (rule 13).

### Current source against `a816f1447`

`git diff --stat a816f1447 --` is empty and the working tree is clean for
every file the repairs touch: `index_corruption.rs`, `ownership.rs`,
`worktree/lib/src/remove/lifecycle/tests.rs`, `compose_schema_cli.rs`,
`invalid_trigger_warnings.rs`, `yaml_documents/registry.rs`,
`kind_semantic_checks.rs`, `worktree/lib/src/git/program.rs`,
`worktree/cli/tests/list_output.rs`, `unchained-ai/model_id/Cargo.toml`,
`unchained-ai/model_id/tests/ui.rs`, `embed_render_tests.rs`, and
`messenger/lib/tests/research_refresh.rs`. **No earlier repair exists for any
row**, so nothing needs re-verifying before Phase 2.

The one shared file that has moved is `.github/workflows/_package-ci.yml`
(+38/−4 since `a816f1447`, commits `2b86f5dc7` and `05af7a4ee`, the archive
prerequisite's work on this branch). Rule 10's fetch step must be inserted
into the current version, preserving those edits.

Anchors confirmed in current source:

- `ownership.rs:240`, `:244`, `:249-250` spell `Path::new("inner/subdir")`,
  `"inner/../clean.rs"`, `"inner/./../clean.rs"` (the `cfg(windows)`
  `symlink_dir` at `:240` included); `:189` already joins components.
- `registry.rs:91-95`: `DefinitionSource::label` returns
  `path.display().to_string()` for file definitions, so the label is native.
- `program.rs:103`: `PREFIXES` lists `mingw64`, `clangarm64`, `mingw32`;
  `launcher_layout` at `:110`.
- `model_id/tests/ui.rs:3-9` early-returns with "skipping trybuild UI tests
  because cargo is unavailable"; `model_id/Cargo.toml` has no
  `[package.metadata.ci]` table.
- `cargo_toolchain` is a governed capability in `.github/ci/environments.json`
  (`false`, with a gap record, for `wsl2-ubuntu` at `:194`), consumed by
  `scripts/ci/affected_scope.py` and `scripts/ci/cell_contract.py`;
  `requires-toolchain` is already used by `tools/test-toolkit` and `scripts`.
- `embed_render_tests.rs:446-451`: `rule_weights` returns `Vec<u16>` with an
  untyped `parse()`.
- `research_refresh.rs:1457` sets `max_seconds = 2` on Windows and waits 4 s
  at `:1467`. Rule 12's runner is
  `messenger/lib/src/research/refresh/runner.rs` (the plan's `runner.rs::run`).

### Environment

- Local host: macOS (this session). No `BUILD_*` variable is declared in the
  session environment.
- `~/.ssh/config` aliases, probed 2026-11-09 with
  `ssh -o BatchMode=yes -o ConnectTimeout=8`:

| Alias | Reachable | Role observed | Proposed variable |
|---|---|---|---|
| `build-win` | yes | native Windows, PowerShell, Git 2.55.0.windows.3 (`mingw64`, no `ucrt64`), `CODING_DIR=B:\coding` | `BUILD_WIN` |
| `build-wsl` | yes | WSL2 Ubuntu, kernel `6.18.33.2-microsoft-standard-WSL2`, no `cargo` on `PATH` (toolchain-free, as required) | `BUILD_WSL` |
| `build-linux` | yes | native Linux (Proxmox kernel `7.0.14-25-pve`) | `BUILD_LINUX` |
| `build-win-native` | no | does not resolve (S5's alias) | — |
| (none) | — | no second macOS host | `BUILD_MACOS` unset |

- Later phases pass these per invocation, e.g.
  `BUILD_WIN=build-win just cross-check <pkg> --os windows <filter>`; they
  are not written into any repository script.
- The rig cannot show the runner's `ucrt64` layout (rule 7's ucrt64 parity
  needs a disposable ≥2.56 install, per the plan; the shared install is not
  to be upgraded).
- 8.3 short-name reproduction is possible only on the rig's C: volume (S5).

### Archive prerequisite status

- `2026-11-06-ubuntu-archive-disk`: phases 1–4 checked off, `implemented:
  true`, `review-1.md` records no findings but carries human-review items;
  spec status still `planned`. Its commits are on this branch
  (`feat/fuzz-infra-release`, 50 commits ahead of `main`) and are **not on
  `main`**.
- Every nightly through Nov 9 therefore still fails all 66 WSL2 cells for
  lack of archives, e.g. job 128082104353:
  `Unable to download artifact(s): Artifact not found for name: build-schematic-gen-ubuntu-latest-6e10ebd31c763cef`
  then `wsl-bash: command not found`. These are provisioning failures owned
  by the prerequisite (rule 14), not test failures; no WSL2 test identity can
  be inventoried from them.
- The Ubuntu build log for that run (job 127999511507) is no longer
  retrievable (`log not found`); the prerequisite spec attributes the failure
  to `No space left on device`.

### Existing qualifying evidence

- None for any repaired behavior: no repair exists yet, no
  `windows-latest` or `wsl2-ubuntu` receipt exists under
  `~/.rusty-biscuit/ci-evidence/`, and no scheduled run on `main` has passed
  since the failures appeared. The S5/S5b rig runs are targeted
  **reproductions of the failures**, not passing evidence.
- WSL2: no archive-mode execution evidence exists for any package.
- `model_id` on WSL2: the `cargo_toolchain` gap is a governed, unmet
  capability; it is not a pass, and the old early-return PASS lines are not
  reused (rule 9).

### Contracts fixed for Phase 2

Every row in the ledger below maps to a plan rule, and that rule's contract
stands as written, with these clarifications from current source:

- Rule 6 is a test bug, not a product bug: the label is native by
  construction (`path.display()`), so no serialization changes.
- Rule 11 must clear E0283 as well as E0284; both come from the same line.
- Rule 12 targets `messenger/lib/src/research/refresh/runner.rs`.
- Rule 10's workflow edit goes into the current `_package-ci.yml`, which
  differs from `a816f1447`.

### Per-identity ledger (141 rows)

| # | Nextest identity | Cause (S5/S5b evidence) | Repair location | Filter | Repro environment | Status | Evidence |
|---|---|---|---|---|---|---|---|
| 1 | `claudine-cli::l1 compose_schema_cli::compose_eager_spec_setter_anchors_before_plan_expression_from_root_and_area::from_the_package_area` | 8.3 TEMP: product emits launch (short) spelling; test expects canonical long path (S5) | rule 5: `compose_schema_cli.rs` shipped-plan-route assertion | `compose_schema_cli::compose_eager_spec_setter_anchors_before_plan_expression_from_root_and_area::from_the_package_area` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082098594](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082098594) |
| 2 | `claudine-cli::l1 compose_schema_cli::compose_eager_spec_setter_anchors_before_plan_expression_from_root_and_area::from_the_repository_root` | 8.3 TEMP: product emits launch (short) spelling; test expects canonical long path (S5) | rule 5: `compose_schema_cli.rs` shipped-plan-route assertion | `compose_schema_cli::compose_eager_spec_setter_anchors_before_plan_expression_from_root_and_area::from_the_repository_root` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082098594](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082098594) |
| 3 | `darkmatter::l1 kind_semantic_checks::an_imported_file_is_still_admitted_against_the_compiled_definition` | native `\` definition label vs string `ends_with("schemas/schema.yaml")` (S5: fails long and short) | rule 6: `kind_semantic_checks.rs::OpenedCopy` | `kind_semantic_checks::an_imported_file_is_still_admitted_against_the_compiled_definition` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100405](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100405) |
| 4 | `darkmatter::l1 kind_semantic_checks::an_opened_schema_definition_admits_an_added_property_and_keeps_its_checks` | native `\` definition label vs string `ends_with("schemas/schema.yaml")` (S5: fails long and short) | rule 6: `kind_semantic_checks.rs::OpenedCopy` | `kind_semantic_checks::an_opened_schema_definition_admits_an_added_property_and_keeps_its_checks` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100405](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100405) |
| 5 | `darkmatter::l1 kind_semantic_checks::an_opened_trigger_definition_admits_an_added_property_and_keeps_its_checks` | native `\` definition label vs string `ends_with("schemas/schema.yaml")` (S5: fails long and short) | rule 6: `kind_semantic_checks.rs::OpenedCopy` | `kind_semantic_checks::an_opened_trigger_definition_admits_an_added_property_and_keeps_its_checks` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100405](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100405) |
| 6 | `darkmatter::l1 kind_semantic_checks::an_opened_yaml_schema_definition_admits_an_added_property_and_keeps_its_checks` | native `\` definition label vs string `ends_with("schemas/schema.yaml")` (S5: fails long and short) | rule 6: `kind_semantic_checks.rs::OpenedCopy` | `kind_semantic_checks::an_opened_yaml_schema_definition_admits_an_added_property_and_keeps_its_checks` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100405](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100405) |
| 7 | `darkmatter-cli::l1 invalid_trigger_warnings::compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` | mixed separators + 8.3 TEMP: raw-or-canonical alternative at `:1012-1015` matches only when TEMP is long (S5: pass long, fail short) | rule 5: `invalid_trigger_warnings.rs::write` + both warning checks | `invalid_trigger_warnings::compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100564](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100564) |
| 8 | `darkmatter-cli::l1 invalid_trigger_warnings::compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell` | mixed separators + 8.3 TEMP: raw-or-canonical alternative at `:1012-1015` matches only when TEMP is long (S5: pass long, fail short) | rule 5: `invalid_trigger_warnings.rs::write` + both warning checks | `invalid_trigger_warnings::compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100564](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100564) |
| 9 | `darkmatter-cli::l1 invalid_trigger_warnings::compose_lifecycle_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` | mixed separators + 8.3 TEMP: raw-or-canonical alternative at `:1012-1015` matches only when TEMP is long (S5: pass long, fail short) | rule 5: `invalid_trigger_warnings.rs::write` + both warning checks | `invalid_trigger_warnings::compose_lifecycle_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100564](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100564) |
| 10 | `darkmatter-cli::l1 invalid_trigger_warnings::compose_reference_refusal_keeps_a_child_only_warning` | mixed separators from `root.join("alpha/schemas/…")`; `:470` compares `trigger.display()` only, fails on every Windows host (S5: fails long and short) | rule 5: `invalid_trigger_warnings.rs::write` + both warning checks | `invalid_trigger_warnings::compose_reference_refusal_keeps_a_child_only_warning` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100564](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100564) |
| 11 | `git-wt-cli::list_output a_listing_never_asks_git_for_its_exec_path` | PATH names `<top>\ucrt64\bin\git.exe` directly; `launcher_layout` recognizes launchers only (S5b: reproduced with `mingw64\bin` first on PATH) | rules 7–8: `worktree/lib/src/git/program.rs` | `a_listing_never_asks_git_for_its_exec_path` | Windows, direct-binary PATH | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100322](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100322) |
| 12 | `messenger::research_refresh a_timed_out_run_settles_its_descendants_before_releasing_the_fleet_lock` | PowerShell grandchild must write readiness inside a 2 s elapsed limit; fails `:1467` "the grandchild started" in 4/4 nightlies (Nov 6–9) | rule 12: `research_refresh.rs` native helper-mode fixture | `a_timed_out_run_settles_its_descendants_before_releasing_the_fleet_lock` | Windows, native Windows | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096885](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096885) |
| 13 | `model_id::ui ui` | trybuild nested `cargo --offline` build: `no matching package named proc-macro2` (registry sources absent) | rules 9–10: `model_id/Cargo.toml`, `tests/ui.rs`, `_package-ci.yml` fetch step | `ui` | Windows, extracted archive, fresh CARGO_HOME | failed at `a816f1447`; repair pending (Phase 2) | [job 128082099316](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082099316) |
| 14 | `tree-hugger-cli::changed index_corruption::linked_0` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_0` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 15 | `tree-hugger-cli::changed index_corruption::linked_1` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_1` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 16 | `tree-hugger-cli::changed index_corruption::linked_103` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_103` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 17 | `tree-hugger-cli::changed index_corruption::linked_11` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_11` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 18 | `tree-hugger-cli::changed index_corruption::linked_12` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_12` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 19 | `tree-hugger-cli::changed index_corruption::linked_13` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_13` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 20 | `tree-hugger-cli::changed index_corruption::linked_16` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_16` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 21 | `tree-hugger-cli::changed index_corruption::linked_20` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_20` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 22 | `tree-hugger-cli::changed index_corruption::linked_31` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_31` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 23 | `tree-hugger-cli::changed index_corruption::linked_4` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_4` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 24 | `tree-hugger-cli::changed index_corruption::linked_8` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_8` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 25 | `tree-hugger-cli::changed index_corruption::linked_84` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::linked_84` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 26 | `tree-hugger-cli::changed index_corruption::main_0` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_0` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 27 | `tree-hugger-cli::changed index_corruption::main_1` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_1` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 28 | `tree-hugger-cli::changed index_corruption::main_103` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_103` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 29 | `tree-hugger-cli::changed index_corruption::main_11` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_11` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 30 | `tree-hugger-cli::changed index_corruption::main_12` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_12` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 31 | `tree-hugger-cli::changed index_corruption::main_13` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_13` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 32 | `tree-hugger-cli::changed index_corruption::main_16` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_16` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 33 | `tree-hugger-cli::changed index_corruption::main_20` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_20` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 34 | `tree-hugger-cli::changed index_corruption::main_31` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_31` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 35 | `tree-hugger-cli::changed index_corruption::main_4` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_4` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 36 | `tree-hugger-cli::changed index_corruption::main_8` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_8` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 37 | `tree-hugger-cli::changed index_corruption::main_84` | 8.3 TEMP: product reports canonical long repo path; test expects TEMP spelling (S5: 24 fail short, 25/25 pass long) | rule 2: `index_corruption.rs::assert_reports_reject` | `index_corruption::main_84` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 38 | `tree-hugger-cli::changed ownership::directory_alias_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 39 | `tree-hugger-cli::changed ownership::directory_alias_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 40 | `tree-hugger-cli::changed ownership::directory_alias_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 41 | `tree-hugger-cli::changed ownership::directory_alias_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 42 | `tree-hugger-cli::changed ownership::directory_alias_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 43 | `tree-hugger-cli::changed ownership::directory_alias_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 44 | `tree-hugger-cli::changed ownership::directory_alias_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 45 | `tree-hugger-cli::changed ownership::directory_alias_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 46 | `tree-hugger-cli::changed ownership::directory_alias_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 47 | `tree-hugger-cli::changed ownership::directory_alias_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 48 | `tree-hugger-cli::changed ownership::directory_alias_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 49 | `tree-hugger-cli::changed ownership::directory_alias_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::directory_alias_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 50 | `tree-hugger-cli::changed ownership::dot_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 51 | `tree-hugger-cli::changed ownership::dot_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 52 | `tree-hugger-cli::changed ownership::dot_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 53 | `tree-hugger-cli::changed ownership::dot_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 54 | `tree-hugger-cli::changed ownership::dot_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 55 | `tree-hugger-cli::changed ownership::dot_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 56 | `tree-hugger-cli::changed ownership::dot_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 57 | `tree-hugger-cli::changed ownership::dot_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 58 | `tree-hugger-cli::changed ownership::dot_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 59 | `tree-hugger-cli::changed ownership::dot_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 60 | `tree-hugger-cli::changed ownership::dot_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 61 | `tree-hugger-cli::changed ownership::dot_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::dot_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 62 | `tree-hugger-cli::changed ownership::external_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 63 | `tree-hugger-cli::changed ownership::external_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 64 | `tree-hugger-cli::changed ownership::external_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 65 | `tree-hugger-cli::changed ownership::external_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 66 | `tree-hugger-cli::changed ownership::external_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 67 | `tree-hugger-cli::changed ownership::external_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 68 | `tree-hugger-cli::changed ownership::external_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 69 | `tree-hugger-cli::changed ownership::external_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 70 | `tree-hugger-cli::changed ownership::external_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 71 | `tree-hugger-cli::changed ownership::external_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 72 | `tree-hugger-cli::changed ownership::external_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 73 | `tree-hugger-cli::changed ownership::external_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::external_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 74 | `tree-hugger-cli::changed ownership::nested_directory_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 75 | `tree-hugger-cli::changed ownership::nested_directory_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 76 | `tree-hugger-cli::changed ownership::nested_directory_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 77 | `tree-hugger-cli::changed ownership::nested_directory_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 78 | `tree-hugger-cli::changed ownership::nested_directory_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 79 | `tree-hugger-cli::changed ownership::nested_directory_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 80 | `tree-hugger-cli::changed ownership::nested_directory_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 81 | `tree-hugger-cli::changed ownership::nested_directory_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 82 | `tree-hugger-cli::changed ownership::nested_directory_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 83 | `tree-hugger-cli::changed ownership::nested_directory_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 84 | `tree-hugger-cli::changed ownership::nested_directory_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 85 | `tree-hugger-cli::changed ownership::nested_directory_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_directory_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 86 | `tree-hugger-cli::changed ownership::nested_file_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 87 | `tree-hugger-cli::changed ownership::nested_file_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 88 | `tree-hugger-cli::changed ownership::nested_file_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 89 | `tree-hugger-cli::changed ownership::nested_file_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 90 | `tree-hugger-cli::changed ownership::nested_file_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 91 | `tree-hugger-cli::changed ownership::nested_file_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 92 | `tree-hugger-cli::changed ownership::nested_file_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 93 | `tree-hugger-cli::changed ownership::nested_file_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 94 | `tree-hugger-cli::changed ownership::nested_file_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 95 | `tree-hugger-cli::changed ownership::nested_file_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 96 | `tree-hugger-cli::changed ownership::nested_file_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 97 | `tree-hugger-cli::changed ownership::nested_file_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::nested_file_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 98 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 99 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 100 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 101 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 102 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 103 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 104 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 105 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 106 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 107 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 108 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 109 | `tree-hugger-cli::changed ownership::ordinary_directory_alias_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_directory_alias_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 110 | `tree-hugger-cli::changed ownership::ordinary_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 111 | `tree-hugger-cli::changed ownership::ordinary_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 112 | `tree-hugger-cli::changed ownership::ordinary_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 113 | `tree-hugger-cli::changed ownership::ordinary_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 114 | `tree-hugger-cli::changed ownership::ordinary_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 115 | `tree-hugger-cli::changed ownership::ordinary_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 116 | `tree-hugger-cli::changed ownership::ordinary_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 117 | `tree-hugger-cli::changed ownership::ordinary_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 118 | `tree-hugger-cli::changed ownership::ordinary_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 119 | `tree-hugger-cli::changed ownership::ordinary_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 120 | `tree-hugger-cli::changed ownership::ordinary_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 121 | `tree-hugger-cli::changed ownership::ordinary_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::ordinary_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 122 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_committed::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_committed::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 123 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_committed::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_committed::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 124 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_committed::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_committed::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 125 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_committed::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_committed::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 126 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_staged::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_staged::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 127 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_staged::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_staged::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 128 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_staged::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_staged::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 129 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_staged::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_staged::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 130 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_untracked::classes` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_untracked::classes` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 131 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_untracked::functions` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_untracked::functions` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 132 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_untracked::symbols` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_untracked::symbols` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 133 | `tree-hugger-cli::changed ownership::recursive_file_alias_parent_untracked::types` | fixture relative symlink target contains `/`; Windows creates but cannot follow it (S5 CreateSymbolicLinkW probe; fails long and short) | rule 3: `ownership.rs::parent_traversal` | `ownership::recursive_file_alias_parent_untracked::types` | Windows, long + short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082096315](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082096315) |
| 134 | `worktree git::program::tests::the_direct_program_sees_the_launchers_environment` | PATH names `<top>\ucrt64\bin\git.exe` directly; `launcher_layout` recognizes launchers only (S5b: reproduced with `mingw64\bin` first on PATH) | rules 7–8: `worktree/lib/src/git/program.rs` | `git::program::tests::the_direct_program_sees_the_launchers_environment` | Windows, direct-binary PATH | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 135 | `worktree git::program::tests::the_named_installation_is_the_prefix_git_reports_and_stays_out_of_git_children` | PATH names `<top>\ucrt64\bin\git.exe` directly; `launcher_layout` recognizes launchers only (S5b: reproduced with `mingw64\bin` first on PATH) | rules 7–8: `worktree/lib/src/git/program.rs` | `git::program::tests::the_named_installation_is_the_prefix_git_reports_and_stays_out_of_git_children` | Windows, direct-binary PATH | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 136 | `worktree remove::lifecycle::tests::a_checkout_replaced_after_the_question_is_refused_and_kept` | 8.3 TEMP: refusal names Git's long spelling; `assert_changed` expects TEMP spelling (S5: exactly these 6 fail short) | rule 4: `lifecycle/tests.rs::assert_changed` | `remove::lifecycle::tests::a_checkout_replaced_after_the_question_is_refused_and_kept` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 137 | `worktree remove::lifecycle::tests::a_file_replaced_after_the_question_is_kept` | 8.3 TEMP: refusal names Git's long spelling; `assert_changed` expects TEMP spelling (S5: exactly these 6 fail short) | rule 4: `lifecycle/tests.rs::assert_changed` | `remove::lifecycle::tests::a_file_replaced_after_the_question_is_kept` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 138 | `worktree remove::lifecycle::tests::a_file_replaced_before_a_saved_removal_resumes_is_kept` | 8.3 TEMP: refusal names Git's long spelling; `assert_changed` expects TEMP spelling (S5: exactly these 6 fail short) | rule 4: `lifecycle/tests.rs::assert_changed` | `remove::lifecycle::tests::a_file_replaced_before_a_saved_removal_resumes_is_kept` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 139 | `worktree remove::lifecycle::tests::an_own_link_replaced_by_a_file_after_the_question_deletes_neither` | 8.3 TEMP: refusal names Git's long spelling; `assert_changed` expects TEMP spelling (S5: exactly these 6 fail short) | rule 4: `lifecycle/tests.rs::assert_changed` | `remove::lifecycle::tests::an_own_link_replaced_by_a_file_after_the_question_deletes_neither` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 140 | `worktree remove::lifecycle::tests::an_unreadable_checkout_replaced_after_the_question_is_refused_and_kept` | 8.3 TEMP: refusal names Git's long spelling; `assert_changed` expects TEMP spelling (S5: exactly these 6 fail short) | rule 4: `lifecycle/tests.rs::assert_changed` | `remove::lifecycle::tests::an_unreadable_checkout_replaced_after_the_question_is_refused_and_kept` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |
| 141 | `worktree remove::lifecycle::tests::an_unrelated_link_replaced_by_a_file_after_the_question_is_kept` | 8.3 TEMP: refusal names Git's long spelling; `assert_changed` expects TEMP spelling (S5: exactly these 6 fail short) | rule 4: `lifecycle/tests.rs::assert_changed` | `remove::lifecycle::tests::an_unrelated_link_replaced_by_a_file_after_the_question_is_kept` | Windows, short TEMP | failed at `a816f1447`; repair pending (Phase 2) | [job 128082100327](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/128082100327) |

The compile failure is tracked separately: `biscuit-visualized` lib test
target, rule 11, repro by building and executing the containing Windows test
binary; status failed at `a816f1447`, repair pending (Phase 2); evidence
[job 127999511428](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42246114963/job/127999511428).

WSL2 rows: none yet. They are inventoried in Phase 3 once usable archives
execute.

### Phase 1 gates

Run from the repository root after all Phase 1 edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `"packages": []`, `"change_class": "documentation"`. The input also
held the pre-existing uncommitted `fixes/2026-11-06-ubuntu-archive-disk/`
planning files (not this phase's work, left untouched). Per the phase rules,
`just test` and `just lint` were **skipped**: nothing any package reads
changed. The spec prompt named no spec frontmatter properties for this phase,
so `spec.md` is unchanged.

Checkpoint met: every original row (141 identities plus the compile failure)
has an identified repair rule, its existing reproduction evidence, a filter,
and a reproduction environment. No contract above needed to change before
edits begin.
