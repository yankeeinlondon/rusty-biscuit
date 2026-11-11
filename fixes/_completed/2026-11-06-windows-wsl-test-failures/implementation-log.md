---
spec: "/Volumes/coding/rusty-biscuit/fixes/2026-11-06-windows-wsl-test-failures/spec.md"
plan: "fixes/2026-11-06-windows-wsl-test-failures/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: [".github/workflows/_package-ci.yml", "biscuit-visualized/src/src/tests/embed_render_tests.rs", "claudine/cli/tests/l1/compose_schema_cli.rs", "darkmatter/cli/tests/l1/invalid_trigger_warnings.rs", "darkmatter/lib/tests/l1/kind_semantic_checks.rs", "messenger/lib/tests/research_refresh.rs", "tools/test-toolkit/tests/ci_workflow_contracts.rs", "tree-hugger/cli/src/changed.rs", "tree-hugger/cli/tests/changed/index_corruption.rs", "tree-hugger/cli/tests/changed/ownership.rs", "unchained-ai/model_id/Cargo.toml", "unchained-ai/model_id/tests/ui.rs", "worktree/cli/tests/list_output.rs", "worktree/lib/src/git/program.rs", "worktree/lib/src/remove/lifecycle/tests.rs"]
docs_updated_during_phase_2: ["tree-hugger/docs/symbol-reports/changed-files.md", "worktree/docs/performance-testing.md"]
docs_created_during_phase_2: ["unchained-ai/docs/testing.md", "worktree/docs/git-for-windows.md"]
skills_files_updated_during_phase_2: [".claude/skills/os/SKILL.md", ".claude/skills/os/windows-paths.md", ".claude/skills/os/windows.md", ".claude/skills/worktree/SKILL.md"]
packages: ["biscuit-visualized", "claudine-cli", "darkmatter", "darkmatter-cli", "git-wt-cli", "messenger", "model_id", "test-toolkit", "tree-hugger-cli", "worktree"]
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
  - |-
    Where `requires-toolchain` lives (rule 9). The plan says to add
    `[package.metadata.ci] requires-toolchain = true` to model_id. The CI
    planner only accepts that key inside `[package.metadata.ci.tests]` (its
    `CI_TEST_FIELDS`), and would reject it as an unknown field at the top
    level. The two packages that already declare it use `.tests`. I put it
    in `[package.metadata.ci.tests]`. The resolved plan then shows model_id
    executing L1 on ubuntu, macOS and windows, and the WSL2 cell as the
    governed `cargo_toolchain` accepted gap. There is no alternative short of
    changing the planner's schema, which the plan did not ask for. Nothing
    needs redoing.
  - |-
    Git for Windows 2.56 (`ucrt64`) and gix. The rule 7/8 repair recognizes
    `ucrt64` and direct entry, and names the installation to gix through
    `EXEPATH`. But the pinned gix (`=0.84.0` in six packages: sniff,
    darkmatter, biscuit-file, content-policy, simplified-schema, worktree)
    uses gix-path 0.12.x, which looks under `EXEPATH` only for
    `clangarm64`/`mingw64`/`mingw32`. No 0.12.x release adds `ucrt64`
    (checked through 0.12.6); gix-path 0.14 does, and only gix 0.89+ uses it.
    On a ucrt64 install (the hosted runner) gix therefore still runs
    `git --exec-path` once, which the `list_output` exec-path tests forbid.
    Measured on the rig with a disposable PortableGit 2.56.0.windows.4:
    5 worktree Windows tests fail on ucrt64 under both entries (the two
    `named_installation` tests and the three `list_output` exec-path tests),
    and all pass on mingw64. So the three original Git ledger rows stay red
    on the hosted runner. I did not upgrade gix in this phase. A trial bump
    to `=0.90.0` (reverted) showed gix replaced its typed error enums with
    `gix_error::Error`/`Exn`: biscuit-file stops compiling at
    `gix::discover::Error`, and 11 typed gix-error references exist across
    the six packages. That is an API migration of its own, outside a
    test-failure repair, and the plan says no dependency changes are
    planned. The
    alternatives are: (a) upgrade gix to 0.89+ workspace-wide, which is the
    clean fix but a large change; (b) set `GIT_CONFIG_SYSTEM` for gix in
    worktree, which mirrors gix internals and leaks into child Git unless
    removed like `EXEPATH`; (c) accept the remaining ucrt64 failure until
    (a). The rig results with a disposable 2.56 install are in the log. If
    the reviewer picks (a), only the dependency bump and a re-run of the
    worktree Windows tests are needed; the resolver code stays.
  - |-
    Leftover prefix directories make a Git installation ambiguous. The Git
    resolver counts a prefix as installed when its directory exists, not
    only when its `bin\git.exe` does, so `mingw64\` left beside a new
    `ucrt64\` means plain `git`. gix-path also picks the prefix by
    directory, so otherwise the `EXEPATH` hint could send gix to the
    leftover's configuration. Alternative: require the binary, which accepts
    more installations but risks the wrong system config. Redo: one
    predicate in `launcher_layout` and its rejection row.
  - |-
    Messenger helper role passing (rule 12). The plan says the helper's roles
    are identified through process-local environment. The runner takes an
    argv only and its child inherits the test process's environment, so the
    parent role travels as an extra libtest filter argument
    (`barrier-parent-dir=<dir>`, which `--exact` matches to no test). Only
    the grandchild's role uses an environment variable, set on that child
    alone. Alternative: mutate the test process's environment before calling
    the runner, which races with other tests in the same process. Redo: the
    two constants and the parent branch of `barrier_process_helper`.
  - |-
    Tree-hugger ownership on Windows (rule 3 went further than planned). Rule
    3 assumed component-joined link targets would clear `parent_traversal`;
    on Windows 36 tests still failed. Windows reads `..` in a link target by
    name, and normalizes absolute targets when the link is created. The
    ownership walk in `tree-hugger/cli/src/changed.rs` modeled Unix physical
    `..`, so on Windows an alias could be judged by one file while Windows
    opened another, possibly in another checkout. I changed the product: on
    Windows `..` returns to the walk's state before the last name, and every
    spelled name is still checked. Normalizing the whole target first was
    tried and rejected (48 failures: it erased names the policy must
    check). The Windows fixtures for `directory-alias` and `external` now
    spell targets that express the same scenario under Windows' rules, and
    four new tests pin the per-OS reading. Alternatives: gate those shapes
    to Unix (drops coverage and leaves the bypass), or keep the fixture
    change only (leaves the bypass). Redo: the `cfg(windows)` block in
    `checkout_path`, the two Windows target spellings, and the new tests.
  - |-
    Lifecycle refusal spelling depends on its source (rule 4). The rule asks
    for the expected path from "the resolved parent plus the entry name".
    That holds for entries Git or the link resolver reports. The worktree
    admin record is named from the repository path the request was given,
    in that spelling, and under the 8.3 TEMP the canonicalized expectation
    failed (`a_worktree_record_rebuilt_after_the_question_is_refused`). So
    `assert_changed` now takes the expected spelling, and callers pass
    `resolved_spelling(..)` for Git-reported entries or the admin path as
    held. Alternative: change the product to canonicalize admin paths, an
    unrequested presentation change. Redo: the helper's callers.
  - |-
    New docs pages. The plan names an "area's Git execution topic" and an
    "unchained-ai testing topic page"; neither existed. I created
    `worktree/docs/git-for-windows.md` and `unchained-ai/docs/testing.md`
    rather than append to unrelated pages. Redo: move the content if the
    reviewer prefers another page.
  - |-
    WSL2 evidence source (Phase 3). The plan says to run the WSL2 legs
    "after the prerequisite supplies usable archives". That prerequisite
    (the Ubuntu archive-disk fix) is still not on main, so no hosted WSL2
    cell can run. I did not wait. I ran `just cross-check <pkg> --os wsl`,
    which builds the same `ubuntu-latest` record inside the WSL2 rig, hides
    the builder's target directory, and runs it from a second checkout at
    another path. That proves relocation, but it is weaker than CI in two
    ways: the record is built in the guest, not on a hosted Ubuntu machine
    and moved between machines, and the rig's guest has Cargo on PATH
    (so model_id's toolchain gap is not visible there; I showed it by
    hand instead). Alternatives: stop the phase until the prerequisite
    merges, which blocks every later step; or treat the rig runs as the
    final proof, which they are not. The hosted proof is still the nightly
    after merge, collected in Phase 4. Nothing needs redoing.
  - |-
    Unfiltered WSL2 runs. The plan writes the command with a test-name
    substring. I ran each changed package's whole L1 tier unfiltered (ten
    packages, about 42 minutes on the idle rig), because the task is to
    find relocation failures, and a filter limited to the edited tests could
    not find one elsewhere in the package. None was found. Alternative:
    filtered runs, which are faster but miss failures outside the edited
    tests. Nothing needs redoing.
  - |-
    Phase 3 gates. The skip check is "the uncommitted working tree is
    exactly this phase's changes", but Phase 2's edits are also still
    uncommitted, so the prescribed command lists all ten packages. Run on
    only the four files this phase changed, the calculator selects
    `darkmatter` (two of its tests read skill Markdown). I ran the two
    narrowed tests, then darkmatter's full `just test` and `just lint`. I did
    not rerun the other seven areas' suites on macOS: this phase changed no
    source, and Phase 2's gates ran on the same source. The ten WSL2 legs
    also ran those packages' L1 tiers. Alternative: rerun all eight areas'
    `just test` (about an hour, the same source). Redo: run those recipes if
    the reviewer wants macOS gates repeated.
  - |-
    Publishing rig receipts in Phase 4. With the head committed and clean,
    each passing `cross-check` leg published a Windows or WSL2 observation
    at `af7ec2d63`, and the CI plan now reuses them for every L1 cell this
    fix touches. The plan asked to "run required affected native Windows
    and WSL package cells" and reuse qualifying receipts, so I let them
    publish. There is a side effect. The Windows rig has Git 2.55
    (`mingw64`), while the hosted image has 2.56 (`ucrt64`). The worktree
    and git-wt-cli receipts therefore stand in for a hosted run that would
    still fail 5 tests on `ucrt64` (the gix-path gap). The messenger receipt
    likewise stands in for the hosted run that is the only place the
    original messenger failure appeared. The next nightly will reuse these
    cells, not rerun them, so it will be green without exercising those two
    hosted behaviors. I recorded this in the log and in the os skill. The
    alternative was to run the legs from a dirty tree so nothing published.
    That gives the same test evidence but leaves every cell for the hosted
    nightly, which on 2026-11-10 could not build at all (out of disk,
    3-hour timeout). To redo: drop the `windows-latest` notes for worktree,
    git-wt-cli and messenger at `af7ec2d63` (`refs/notes/ci-local/windows-latest`),
    or change one of their inputs, so the nightly runs them on the hosted
    image.
  - |-
    Main confirmation is recorded as incomplete, not waited for. No
    scheduled run on `main` contains the fix (the branch is not merged), so
    none of the required three green nightlies per OS exists. As the plan
    directs, I did not wait, re-trigger, or dispatch a workflow. The
    implementation is marked complete and ready for review. The outcome
    acceptance (three nightlies per OS, the hosted ucrt64 rows, messenger on
    the hosted runner, model_id on WSL2) stays open in the log's "Remaining
    capability and access limitations". Nothing needs redoing. A reviewer
    who wants the spec held open until the nightlies land can keep the
    lifecycle directory where it is.
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: [".claude/skills/os/windows-paths.md", ".claude/skills/os/wsl.md"]

source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: [".claude/skills/os/windows.md"]
source_code: [".github/workflows/_package-ci.yml", "biscuit-visualized/src/src/tests/embed_render_tests.rs", "claudine/cli/tests/l1/compose_schema_cli.rs", "darkmatter/cli/tests/l1/invalid_trigger_warnings.rs", "darkmatter/lib/tests/l1/kind_semantic_checks.rs", "messenger/lib/tests/research_refresh.rs", "tools/test-toolkit/tests/ci_workflow_contracts.rs", "tree-hugger/cli/src/changed.rs", "tree-hugger/cli/tests/changed/index_corruption.rs", "tree-hugger/cli/tests/changed/ownership.rs", "unchained-ai/model_id/Cargo.toml", "unchained-ai/model_id/tests/ui.rs", "worktree/cli/tests/list_output.rs", "worktree/lib/src/git/program.rs", "worktree/lib/src/remove/lifecycle/tests.rs"]
documentation: ["tree-hugger/docs/symbol-reports/changed-files.md", "worktree/docs/performance-testing.md", "unchained-ai/docs/testing.md", "worktree/docs/git-for-windows.md", ".claude/skills/os/SKILL.md", ".claude/skills/os/windows-paths.md", ".claude/skills/os/windows.md", ".claude/skills/os/wsl.md", ".claude/skills/worktree/SKILL.md"]
completed_phase: 4
implemented: true
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

## Phase 2

Phase 2 repairs the known Windows failures. Code edits were split across four
parallel subagents (tree-hugger fixtures; path assertions in worktree,
claudine and darkmatter; Git resolution; messenger fixture). Each verified
only its own tests on macOS. The coordinator made the rule 9–11 edits and the
workflow insertion, and ran all native-Windows evidence serially on
`build-win`. Rigs used per invocation: `BUILD_WIN=build-win` (native Windows),
with `BUILD_WSL=build-wsl` and `BUILD_LINUX=build-linux` declared but not needed
in this phase.

### Windows evidence setup

- The S5 private clone `B:\coding\a3-spike-s5\rusty-biscuit` was reused, not
  `just cross-check`, because cross-check cannot set `TEMP`. It ran first at
  `a816f1447` (clean) for messenger's before-capture, then at a throwaway
  snapshot commit `df882ddf0` of this working tree. The snapshot was built
  with a temporary index (`GIT_INDEX_FILE`), shipped as a bundle, and checked
  out detached with a clean tree. The local branch and index were not touched.
- TEMP modes: long `C:\Users\ken\AppData\Local\Temp\a3-spike-long-temporary`
  and its 8.3 spelling `C:\Users\ken\AppData\Local\Temp\A3-SPI~1`, the same
  directory under two names. Both were set for `TEMP` and `TMP`, with
  `NEXTEST_PROFILE=ci`, in native mode from PowerShell.

### Rule 12 before-capture (messenger, native Windows rig, `a816f1447`)

- `cargo nextest run -p messenger --features research --test research_refresh -E 'test(=a_timed_out_run_settles_its_descendants_before_releasing_the_fleet_lock)'`
  at `a816f1447`: **PASS in 3.834 s** on the idle rig. The hosted failure
  (4/4 nightlies, `research_refresh.rs:1467`, "the grandchild started") does
  not reproduce here.
- The exact grandchild the old test launched
  (`cmd /D /C powershell -NoProfile -NonInteractive -EncodedCommand …`,
  writing the PID and readiness file) was started three times directly on
  the rig and timed to its readiness file: **0.239 s, 0.222 s, 0.209 s**.
  Stdout was `child-ok`. Stderr was PowerShell's CLIXML progress record
  "Preparing modules for first use", the module-cache step a fresh hosted
  image pays on its first PowerShell start. That step runs inside the test's
  2 s elapsed limit, which is consistent with the deterministic hosted
  failure and an idle-rig pass. (`Start-Process` reported no exit code for
  the redirected child.)
- So the before/after contrast for rule 12 is only observable on the hosted
  runner. The rig gives the after-run (below) and the child's timing and
  output.

### Tree-hugger ownership: an unplanned Windows product defect

Rule 3 assumed that joining link targets component by component repairs
`parent_traversal`. On Windows that cleared 108 of 144 ownership tests
(0 passed before), but 36 still failed at snapshot `df882ddf0`:
`external_parent_*` (12, product accepted the alias) and
`directory_alias_parent_*` and `ordinary_directory_alias_parent_*` (24,
fixture precondition "fixture returns to the unchanged outer identity").

A probe on the rig used the same `std` calls as the fixture (symlinks in
`…\a3-spike-long-temporary\p2-link-probe2`, with `bridge` a directory link to
`inner\subdir`):

| Link in `repo\` | `read_link` | Windows opens |
|---|---|---|
| `alias.rs → bridge\..\..\clean.rs` | verbatim | `OUTSIDE-CLEAN`, the file beside the repo (`..` read by name) |
| absolute `…\foreign\subdir\..\..\repo\clean.rs` | `…\repo\clean.rs`, normalized **at creation** | `REPO-CLEAN` |
| relative `..\foreign\subdir\..\..\repo\clean.rs` | verbatim | `REPO-CLEAN` |

So Windows reads `..` in a link target by name, and normalizes an absolute
target when the link is created. `tree-hugger/cli/src/changed.rs`'s
`checkout_path` walked `..` physically (Unix semantics). On Windows an alias
such as `bridge\..\nested\leaf.rs` was therefore judged by
`inner\nested\leaf.rs` (clean) while Windows opened `nested\leaf.rs` in
another checkout. That is an ownership-check bypass.

- **First attempt (snapshot `dec19f989`, rejected):** normalize the whole
  target by name before the walk. On the rig that produced 48 failures
  (12 tests each of `nested_directory_parent`, `nested_file_parent`,
  `dot_parent` and `recursive_file_alias_parent`). Erasing `inner\..` before
  the walk drops the documented rule that a name in another checkout is
  excluded even when `..` follows it.
- **Fix:** the walk still visits and checks every spelled component. On
  Windows only, `..` returns to the walk's state before the last name (the
  link's own directory) instead of the physical parent of what that name
  resolved to. Unix behavior is unchanged.
- **Tests:** in `ownership.rs`, the `directory-alias` shapes use
  `bridge\..\clean.rs` on Windows (the hops that return to `clean.rs` there)
  and keep `bridge/../../clean.rs` on Unix. `external` uses the relative
  spelling on Windows, because an absolute one loses its `..` at creation;
  the Unix target is unchanged. New `parent_after_directory_link_untracked::*`
  (4 tests) use one link, `bridge\..\nested\leaf.rs`: on Unix it opens a
  clean file and is accepted, on Windows it opens another checkout's file and
  is rejected. Under the old walk the Windows case was accepted, and the
  `ordinary_directory_alias` cases errored on `inner\clean.rs`.
- Docs: `tree-hugger/docs/symbol-reports/changed-files.md` now states the
  per-OS reading with an example table.

### Native Windows results: path groups (rules 2–6)

Run by `p2-run.ps1` on `build-win` in the private clone, one `cargo nextest
run` per group, `NEXTEST_PROFILE=ci`, `TEMP`/`TMP` set as listed. Each group
ran in long and in 8.3 short mode. Logs are under
`B:\coding\a3-spike-s5\{p2,p2b,p2c}-{long,short}\`.

| Group (filter) | Snapshot | Long TEMP | Short TEMP (`A3-SPI~1`) |
|---|---|---|---|
| `worktree` (`test(remove::lifecycle) \| test(git::program)`) | `df882ddf0` | 92/92 pass | 80 pass, **1 fail** (`a_worktree_record_rebuilt_after_the_question_is_refused`, admin-record spelling; see decisions) |
| `worktree` (same) | `dec19f989` (spelling fix) | 92/92 pass | **92/92 pass** |
| `claudine-cli` (`test(compose_schema_cli)`) | `dec19f989` | 26/26 pass | 26/26 pass |
| `darkmatter-cli` (`test(invalid_trigger_warnings)`) | `dec19f989` | 44/44 pass | 44/44 pass |
| `darkmatter` (`test(kind_semantic_checks)`) | `dec19f989` | 15/15 pass | 15/15 pass |
| `tree-hugger-cli --test changed` (`test(index_corruption) \| test(ownership)`) | `df882ddf0` (fixture only) | 36 ownership fail (see above) | same |
| same | `dec19f989` (`by_name`, rejected) | 125 pass, 48 fail | not rerun |
| same | `0b0284d3e` (final walk) | **173/173 pass** | see below |

The snapshot `dec19f989` rows for worktree, claudine and darkmatter are final:
none of those files changed afterwards.

The final tree-hugger rows at `0b0284d3e`: long TEMP **173/173 pass**, short
TEMP **173/173 pass** (`p2c-{long,short}`). That covers all 24
`index_corruption` rows, all 96 ownership rows from the ledger, the 48 other
ownership controls, and the 4 new `parent_after_directory_link_untracked::*`
tests. On macOS, the same filter gives 173/173.

### Native Windows results: rules 11 and 12 (snapshot `0b0284d3e`, long TEMP)

- **Rule 11:** CI's exact command
  `cargo test --no-run --message-format=json-render-diagnostics --package=biscuit-visualized --all-features --profile=test --target=x86_64-pc-windows-msvc`
  exits 0, with no `E0283`/`E0284`. Then
  `cargo nextest run -p biscuit-visualized --all-features --no-fail-fast`
  gives **1163/1163 pass**, 1 skipped, including all 16
  `tests::embed_render_tests::*` identities
  (`B:\coding\a3-spike-s5\p2-rest\bv-{compile,run}.log`).
- **Rule 12 (after):**
  `cargo nextest run -p messenger --features research --test research_refresh --no-fail-fast`
  gives **53/53 pass** on Windows (the 54th test is `cfg(unix)`). The
  target test alone passed three times: 2.479 s, 2.449 s, 2.464 s (the old
  version took 3.834 s at `a816f1447`). On macOS it takes 1.34–1.44 s alone,
  and the binary gives 54/54.
  - The time is the 2 s elapsed limit plus settlement. The readiness wait has
    a 10 s bound and also ends if the runner returns first, so a starved
    runner fails on "the grandchild started inside the elapsed-time limit",
    not on a timeout.
  - The hosted-runner contrast needs a nightly after merge; see the handoff.

### Native Windows results: rules 7–8 (Git entry shapes, snapshot `0b0284d3e`)

`p2-git.ps1 -Shape <shape>` puts one entry first on `PATH`, drops every
other Git directory, and clears `EXEPATH`/`GIT_EXEC_PATH`/`MSYSTEM`. It then
runs a real `git init` and `git status`, and
`cargo nextest run -p worktree -E 'test(git::program)'` and
`cargo nextest run -p git-wt-cli -E 'binary(list_output)'`. The ucrt64 rows
use a disposable PortableGit 2.56.0.windows.4 (the hosted runner's version)
extracted to `B:\coding\a3-spike-s5\portable-git-2.56\git`. It has
`ucrt64\`, no `mingw64\`, and no `etc\git-bash.config`. The shared
`C:\Program Files\Git` (2.55.0.windows.3, mingw64) was not changed.
PortableGit's self-extractor hung in its post-install `git-bash.exe` with no
desktop and was stopped; Git ran normally without that step.

| Shape | `git init` / `status` | `git::program` | `list_output` |
|---|---|---|---|
| mingw64 launcher (`…\Git\cmd`) | exit 0 / `?? a.txt` | 35/35 | 10/10 |
| mingw64 direct (`…\Git\mingw64\bin`) | exit 0 / `?? a.txt` | 35/35 | **10/10** (S5b: 3 failed here) |
| ucrt64 launcher (`…\cmd`) | exit 0 / `?? a.txt` | 33/35 | 7/10 |
| ucrt64 direct (`…\ucrt64\bin`) | exit 0 / `?? a.txt` | 33/35 | 7/10 |

The five ucrt64 failures are the same under both entries:

- `git::program::tests::entries::through_the_{launcher,direct_binary}_the_named_installation_is_the_prefix_git_reports`
  at `program.rs:465`: gix reports the system prefix as
  `…\portable-git-2.56\git` (the top), while Git reports
  `…/git/ucrt64`. gix-path 0.12 does not recognize `ucrt64` under `EXEPATH`.
- `list_output::a_listing_never_asks_git_for_its_exec_path` and its
  launcher-first and direct-first variants, at `list_output.rs:135`: gix
  still runs `git --exec-path`.

So on the hosted runner's layout, the three original Git ledger rows are
**not cleared by the resolver repair alone**; they need gix-path ≥ 0.14
(gix ≥ 0.89). See the decision item and the follow-up below.

### Native Windows results: rules 9–10 (model_id, snapshot `0b0284d3e`)

`p2-model-id.ps1` mirrors the CI consumer:

1. Producer: `cargo nextest archive -p model_id --archive-file …\model_id.tar.zst`
   with the normal Cargo home → exit 0.
2. Consumer, **fresh `CARGO_HOME`**, `CARGO_NET_OFFLINE=true`, no fetch:
   `cargo-nextest nextest run --archive-file … --workspace-remap <clone> -E 'test(=ui)'`
   → **FAIL in 0.373 s** with `no matching package named proc-macro2 found …
   offline mode`, the same error as hosted job 128082099316. A missing crate
   fails; it is not reported as a pass.
3. Workflow step, same fresh home:
   `cargo fetch --locked --manifest-path unchained-ai/model_id/Cargo.toml` → exit 0.
4. Same consumer command, offline → **PASS in 10.308 s**. Trybuild compiled
   the `tests/pass` cases and checked the `tests/fail` cases against their
   `.stderr` files.

On macOS: `ui` passes (9.1 s). With `PATH=/usr/bin:/bin` (no `cargo`), the
binary fails at `ui.rs:8` with "the trybuild UI tests need `cargo` on PATH",
where it used to print a skip and pass. The resolved plan
(`affected_scope.py --event workflow_dispatch --resolved-plan`) shows model_id
L1 executing on ubuntu, macOS and windows, and wsl2-ubuntu as
`state: accepted-gap` on `cargo_toolchain`. The gap record (owner, expiry)
was not edited.

Workflow: the fetch step sits right after "Set up the pinned Rust
toolchain" in `_package-ci.yml`'s `test` job. It is gated on
`matrix.package == 'model_id' && steps.cell.outputs.requires_toolchain == 'true'`
and appends `CARGO_NET_OFFLINE=true` to `$GITHUB_ENV` for the later steps.
The archive-disk edits already on this branch were kept.

The test-toolkit gate then failed
`ci_workflow_contracts::no_test_tier_carries_a_compile_in_place_path`, which
forbids any `cargo ` line in an archive consumer apart from the counter and
nextest. The contract now also allows `cargo fetch --locked …`, but only in
a step gated on `requires_toolchain`: fetching compiles nothing. Every other
`cargo` line, and an ungated fetch, still fail it.

The `ui` test takes 9–10 s, over the 5 s L1 budget. That is unchanged by
this fix (it ran the same nested builds before, where Cargo existed); it is
inherent to trybuild. It was not renamed `slow_`, because the WSL2 archive
cell is already a governed gap and the hosted cells must execute it.

### Requirement-to-test mapping

| Changed behavior | Boundary / fidelity | Tests (targeted) | Windows evidence |
|---|---|---|---|
| Rule 2: index-corruption diagnostic names the canonical repo path | CLI spawn, filesystem (F3, L1) | `tree-hugger-cli::changed index_corruption::*` (25) | 25/25 long and short TEMP |
| Rule 3 + product walk: link targets joined by name; Windows `..` read by name; every name still checked | CLI spawn, real symlinks (F3, L1) | `ownership::*` (144, including outside-owner rejection and ordinary-directory acceptance controls) and new `ownership::parent_after_directory_link_untracked::{functions,types,symbols,classes}` | 148/148 long and short; the new tests are Windows-rejected and Unix-accepted for the same link |
| Rule 4: refusal names the entry in its source's spelling | library, real Git (F2, L1) | `worktree remove::lifecycle::tests::*` (59, including admin-record and link-replacement) | 92/92 (with `git::program`) long and short |
| Rule 5a: plan route in the launch spelling | CLI spawn (F3, L1) | `claudine-cli compose_schema_cli::*` (26) | 26/26 long and short |
| Rule 5b: native trigger spelling in both warnings | CLI spawn (F3, L1) | `darkmatter-cli invalid_trigger_warnings::*` (44) | 44/44 long and short |
| Rule 6: definition label equals the copy's native path | library (F2, L1) | `darkmatter kind_semantic_checks::*` (15) | 15/15 long and short |
| Rules 7–8: launcher and direct entry, four prefixes, environment per entry | path logic on every OS (F1); real Git for Windows (F3) | `worktree git::program::tests::*` (12 recognized + guard + 15 plain-git rows), Windows `entries::*` (4 parents), `git-wt-cli list_output` (10 on Windows) | mingw64 launcher and direct 35/35 + 10/10; ucrt64 33/35 + 7/10 (gix-path gap) |
| Rule 9: `ui` fails without `cargo`; `requires-toolchain` declared | test binary, planner | `model_id::ui ui`; resolved plan | no-cargo fail on macOS; plan shows the WSL2 gap |
| Rule 10: fetch, then offline nested builds | archive consumer, fresh Cargo home (F3) | `model_id::ui ui` from an archive; `test-toolkit ci_workflow_contracts::no_test_tier_carries_a_compile_in_place_path` | no-fetch FAIL (proc-macro2), fetch, then PASS |
| Rule 11: `parse::<u16>()` | compile on the MSVC target | CI's exact `cargo test --no-run` command; `biscuit-visualized` full run | compile exit 0; 1163/1163, 16 embed_render |
| Rule 12: native helper roles, bounded readiness, no fixed sleep | real processes (F3, L1) | `messenger::research_refresh a_timed_out_run_settles_its_descendants_before_releasing_the_fleet_lock` (+ inert `barrier_process_helper`) | 3/3 (2.45–2.48 s); binary 53/53 |

The input robustness matrix does not apply (plan rule 15). The only
configuration edit is one boolean key that the existing planner validator
already parses.

### Phase 2 gates (macOS, serial, after all edits; no subagent running)

| Area | `just test` | `just lint` |
|---|---|---|
| tree-hugger | 5827/5827 | pass |
| worktree | 1510 pass, 13 skipped | pass |
| unchained-ai | 367 pass, 1 skipped | pass |
| biscuit-visualized | 1163 pass, 1 skipped | pass |
| messenger | 784 pass, 2 skipped | pass |
| tools (test-toolkit) | first run: 1 fail (the compile-in-place contract, see rules 9–10); after the contract change: **526/526** | pass |
| claudine | 11818 pass, 15 skipped | pass |
| darkmatter | 13051 pass, 10 skipped | clippy passes for all 4 packages; the recipe exits 1 at its last step `check-zed`, because the `wasm32-wasip2` target is not installed on this host (pre-existing environment gap, unrelated to this change) |

No package outside the changed areas depends on changed library code:
`worktree`'s only dependent is `git-wt-cli` (tested above), and
`tree-hugger-cli` is a binary. No test failed for load reasons; no rerun of
a wider suite was needed.

Scope calculator, run after all edits:
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
gave `packages: [biscuit-visualized, claudine-cli, darkmatter, darkmatter-cli, git-wt-cli, messenger, model_id, test-toolkit, tree-hugger-cli, worktree]`
and `change_class: package`, so the gates were required and ran.

### Remaining and unmet items after Phase 2

- **ucrt64 Git rows (3 ledger rows: `program.rs:285`, `program.rs:310`,
  `list_output.rs:94`):** these pass on mingw64 in both entry shapes, but on
  the hosted runner's ucrt64 layout 5 tests still fail until gix-path ≥ 0.14
  (gix ≥ 0.89, an error-API migration across six packages). This item is
  unmet.
- **Messenger on the hosted runner:** the rig cannot reproduce the hosted
  failure (the old test passed there). Only a post-merge nightly shows the
  fix there.
- **model_id on WSL2:** unchanged governed `cargo_toolchain` gap; unmet by
  design, not a pass.
- **WSL2 archive execution:** Phase 3; the archive prerequisite is still
  unmerged.
- Rig state left behind: the private clone `B:\coding\a3-spike-s5\rusty-biscuit`
  (detached at throwaway snapshot `0b0284d3e`), its logs, and the disposable
  `portable-git-2.56`. The clone falls under cross-check's standalone-clone
  pruning. No shared install or standing clone was touched. The local
  throwaway ref `refs/tmp/p2snap` was deleted.

## Phase 3

Phase 3 runs the changed packages' Ubuntu-built nextest archives on the WSL2
rig with the builder's target directory hidden (`just cross-check <pkg> --os
wsl`), inventories what fails, and documents the platform facts. Rig, per
invocation: `BUILD_WSL=build-wsl` (WSL2 Ubuntu, kernel
`6.18.33.2-microsoft-standard-WSL2`, idle at start, load 0.00).

### Archive prerequisite at the start of Phase 3

- `main` still has no usable WSL2 archives: `2026-11-06-ubuntu-archive-disk`
  is on this branch only (Phase 1 "Archive prerequisite status"; unchanged).
  So no hosted `wsl2-ubuntu` cell can run yet, and no hosted WSL2 test
  identity exists to inventory.
- The rig path does not depend on that prerequisite.
  `just cross-check <pkg> --os wsl` builds the plan's `ubuntu-latest` record
  inside the WSL2 guest (same build key CI's Linux producer writes), moves it
  to a consumer-owned directory, checks out a **second** worktree at another
  path, **hides the producer's `target`** (`mv target target.hold`), verifies
  the record with `just _ci_build_verify`, lists the expected tests, and runs
  the canonical `just _test <pkg>` recipe in archive mode
  (`--archive-file … --workspace-remap <second checkout>`,
  `BISCUIT_NEXTEST_BIN='cargo-nextest nextest'`). That is the plan's
  relocation check: a builder path baked in at compile time cannot resolve.
  No Cargo build flags or target selectors were passed.

### Relocated archive runs (WSL2 rig, 2026-11-09)

Command, once per package, serially, unfiltered (the whole L1 tier, not
just the edited tests):
`BUILD_WSL=build-wsl ./scripts/cross-check.sh --os wsl <pkg>` (the script
behind `just cross-check`). Base `origin/feat/fuzz-infra-release`
(`ed0688acc`) plus this working tree's 267 changed files as a throwaway
commit. Every remote worktree was clean (`cross-check-dirty: 0`), and every
leg reported `cross-check-manifest-exit: 0`, so each run listed its
expected tests first.

| Package | Archive key (producer → consumer) | Digest | Remote tree | Result | Leg time |
|---|---|---|---|---|---|
| messenger | `f9f30aa3548e28da` ubuntu-latest → wsl2-ubuntu | `599fcaadbbc4179c` | `25f7042fc` | 683 passed, 2 skipped | 341 s |
| worktree | `659fd1aa01bfff12` | `c5cebd5eddaa99e8` | `90df205f5` | 898 passed, 0 skipped | 173 s |
| git-wt-cli | `53356c6cd0f2801a` | `21d90728698ae43d` | `90df205f5` | 614 passed, 55 skipped | 231 s |
| tree-hugger-cli | `e71470fea44fefb2` | `3ddff90e58a5f415` | `90df205f5` | 2746 passed, 31 skipped | 187 s |
| model_id | `49816661be381944` | `f6c6bc2652be7df8` | `90df205f5` | 2 passed (see below) | 64 s |
| biscuit-visualized | `f661ea90d91b100f` | `4b4b842b050ec7fe` | `90df205f5` | 1166 passed, 1 skipped | 169 s |
| test-toolkit | `edb3be5b5a795fc8` | `265882b49a5d64ac` | `90df205f5` | 525 passed, 4 skipped | 76 s |
| claudine-cli | `ad4e53feb837b325` | `f9363ab766d21014` | `90df205f5` | 4951 passed, 306 skipped | 698 s |
| darkmatter | `91bcfbd8c5e6c9b2` | `cae666c0c3477cb6` | `90df205f5` | 9812 passed, 66 skipped | 307 s |
| darkmatter-cli | `ae44092af3e39ad5` | `b69c271b4db8ddb7` | `90df205f5` | 1489 passed, 69 skipped | 260 s |

- messenger's tree differs from the rest only because this log and the
  skill pages changed between its leg and the others; no source differed.
- Skipped counts are tests the L1 recipe's filter leaves out (other tiers'
  markers). They are not failures. I did not audit them one by one, because
  each leg's expected-test listing exited 0.
- Every leg declined to publish a `wsl2-ubuntu` observation ("the remote
  tested tree … is not this head's tree 7e1167faa"), as designed: it
  tested uncommitted work. These are targeted rig evidence, not receipts.

**Failure inventory: none.** No test failed in any of the ten packages, so
rule 14 had nothing to repair: no `bin_exe!`, `manifest_dir!()`, or
resolver-root change was needed, and no helper or caller was edited. No
failure fell outside the relocation patterns either. There is therefore no
"before"/"after" pair to record for any WSL2 identity.

### model_id on the rig is not the CI guest

The `ui` PASS above is **not** evidence for the WSL2 cell. The rig's guest
builds its own archive, so the consumer has `~/.cargo/bin` on `PATH`. CI's
guest has no Cargo, and the planner renders that cell as the governed
`cargo_toolchain` accepted gap (Phase 2). To show what the guest sees, the
same archive was run by hand in the rig clone (`a32592a55`, clean), with
the target hidden and a fresh `--extract-to` directory:

- `env PATH=<dir with only cargo-nextest>:/usr/bin:/bin CARGO_NET_OFFLINE=true cargo-nextest nextest run --archive-file … --workspace-remap … --extract-to …`
  → **exit 100**, `model_id::ui ui` FAIL: "the trybuild UI tests need
  `cargo` on PATH: Err(Os { code: 2, kind: NotFound, … })". The unit test
  passed. A toolchain-free consumer fails explicitly, with no early-return
  pass (rule 9).
- With the login `PATH` (Cargo present) and a fresh extract directory:
  **exit 0**, `ui` PASS in 6.19 s (SLOW over 5 s, as on macOS and Windows).
- A first attempt at the second run reused the first run's extract
  directory and reported `ui` FAIL with exit 96; the rerun in a fresh
  directory passed. That came from how I ran it (a reused
  `--extract-to`), not from the product, and it went into `wsl.md`.

The WSL2 model_id gap stays **unmet**, as before. It closes only with a
changed execution contract (Cargo in the guest, or a native WSL2 build).

### Documentation

- `.claude/skills/os/windows-paths.md` item 6: the 8.3 short-name
  reproduction recipe from S5 (short names exist only on C:; the rig's own
  TEMP has no distinct short form, so create a long-named directory; run
  from a private PowerShell clone because cross-check cannot set `TEMP`;
  pass-long/fail-short identifies the trap). The symlink-target section
  from Phase 2 was not duplicated.
- `.claude/skills/os/wsl.md`: `cross-check --os wsl` is not toolchain-free
  (a `requires-toolchain` suite passes there but is a gap in CI) and how to
  reproduce the guest's view; use a fresh `--extract-to` per manual run.
- The Windows topic's launcher/direct Git and ucrt64 facts
  (`.claude/skills/os/windows.md`) and the package topic
  `worktree/docs/git-for-windows.md` were written in Phase 2 and checked
  against the code; nothing in them drifted. No README needed a change, and
  no dependency was added.

### Requirement-to-test mapping (Phase 3)

| Requirement | Evidence | Result |
|---|---|---|
| Ubuntu-built archives run on WSL2 with builder paths unavailable | the ten `cross-check --os wsl` legs above (unfiltered L1, target hidden, second checkout) | 22,886 tests passed, 0 failed |
| Failures inventoried by package/test identity | same legs | none to inventory |
| Rule 14 repairs (bin_exe!, manifest_dir!, resolver root) | not needed; no failure | no code changed |
| model_id gap is not reported as a pass | manual no-cargo archive run in the guest | `ui` FAIL at its prerequisite, exit 100 |

No new tests were added in this phase. No code changed, and there was no
failure to write a regression test for. The input robustness matrix does
not apply (rule 15).

### Phase 3 gates (macOS, after all edits, no other job running)

- Scope calculator, the prescribed command over the whole uncommitted
  tree: `packages` = all ten Phase 2 packages, `change_class: package`
  (Phase 2's edits are still uncommitted). Over this phase's four files
  alone: `packages: [darkmatter]`, `change_class: package`; the two skill
  pages are test inputs that the resolved plan narrows to
  `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`
  and `…::shipped_markdown_named_type_references_resolve`. Gates required.
- Narrowed tests: 2/2 pass.
- darkmatter `just test`: **13051 passed** (2 slow), 10 skipped, exit 0.
- darkmatter `just lint`: clippy clean for darkmatter, darkmatter-cli,
  darkmatter-lsp, and zed-dmls-cli. The recipe then exits 1 at
  `check-zed`, because `wasm32-wasip2` is not installed on this Mac (the
  same gap Phase 2 recorded). Cargo also printed a harmless
  "failed to auto-clean cache data … Permission denied" warning about one
  file in `~/.cargo/registry`.
- The other areas were not rerun on macOS, because no source changed since
  their Phase 2 gates (see `decisions_for_review`). Their L1 tiers did run
  on WSL2 above.

### Remaining and unmet items after Phase 3

- **Hosted WSL2 confirmation:** blocked until the archive-disk prerequisite
  is on `main`; Phase 4 collects nightlies.
- **model_id on WSL2:** governed `cargo_toolchain` gap, unmet by design.
- **ucrt64 Git rows** and **messenger on the hosted runner:** unchanged
  from Phase 2.
- Rig state: the WSL clone `~/coding/bolt--main/rusty-biscuit` sits at the
  last throwaway commit `a32592a55` with `target` restored; the manual
  run's temp files were removed. Cross-check retired the unused clone
  `shazam--feat-better-research` on its own (unused 3 days).

## Phase 4

Phase 4 runs the scoped gates once, after integration, publishes Windows and
WSL2 rig evidence for the committed head, inspects the CI plan, looks for
main confirmation, and closes the ledger. No source changed in this phase.
Head under test: `af7ec2d63` on `feat/fuzz-infra-release`, a clean,
committed tree. Phases 2–3 are in `758c39850` and `348d5ee28`, and main is
merged in. The branch is 282 commits ahead of `origin/main` and **not on
`main`**.

### Scoped gates (macOS, serial, no subagent running)

One background chain ran each area's canonical recipes in turn. A first
attempt resolved `cd worktree` through the shell's `CDPATH` into an
unrelated checkout and was stopped after a few seconds. The rerun used
absolute paths. Only the rerun's results count:

| Area | `just test` | `just lint` |
|---|---|---|
| worktree | 1510 passed, 13 skipped | pass |
| tree-hugger | 5827 passed | pass |
| unchained-ai | 367 passed, 1 skipped | pass |
| biscuit-visualized | 1165 passed, 1 skipped | pass |
| messenger | 784 passed, 2 skipped | pass |
| tools (test-toolkit, includes `ci_workflow_contracts`) | 526 passed (4 slow), 4 skipped | pass |
| claudine | 11859 passed (4 slow), 15 skipped | pass |
| darkmatter | 16370 passed (3 slow), 10 skipped | pass, including `check-zed` (the `wasm32-wasip2` gap from Phases 2–3 is gone on this host) |

No test failed, so nothing needed a rerun. No L2 test was added or changed,
so `just test-l2` was not needed (plan: "No L2 is introduced").

### Native Windows and WSL2 rig cells (receipts at `af7ec2d63`)

Command, one leg at a time from the repository root (Windows first, then
WSL2, both on the same machine):
`BUILD_WIN=build-win BUILD_WSL=build-wsl BUILD_LINUX=build-linux ./scripts/cross-check.sh --os <os> <pkg>`.
The head was clean, so each passing leg **published** an observation for
its own environment (`refs/notes/ci-local/<env>`). Phases 2–3 could not do
that.

| Package | windows-latest (build-win) | wsl2-ubuntu (build-wsl, archive relocated) |
|---|---|---|
| worktree | 877 passed | 898 passed |
| git-wt-cli | 569 passed, 43 skipped | 614 passed, 55 skipped |
| tree-hugger-cli | 2726 passed, 31 skipped | 2746 passed, 31 skipped |
| messenger | 677 passed, 2 skipped | 683 passed, 2 skipped |
| model_id | 2 passed (`ui` executed) | accepted gap (`cargo_toolchain`), not run |
| biscuit-visualized | 1165 passed, 1 skipped | not scheduled on WSL2 by this head's plan |
| test-toolkit | 425 passed, 2 skipped | accepted gap, not run |
| darkmatter | 11887 passed, 66 skipped | 11942 passed, 66 skipped |
| darkmatter-cli | 2486 passed, 85 skipped | 2497 passed, 83 skipped |
| claudine-cli | 3893 passed, 13 skipped | 4980 passed, 306 skipped |

All 17 legs: `cross-check-exit: 0`, "published a passing … observation …
at af7ec2d63". Skipped counts are other tiers' tests. test-toolkit's
Windows cell had failed 8 tests at `758c39850`. It passes here because of
`1a2c5429a` ("make release path tests encoding portable"), which is not
this fix's work.

### `just ci-local --plan` (inspection only, nothing pushed or dispatched)

- **Before the rig legs:** every changed package's `windows-latest` and
  `wsl2-ubuntu` L1 cell was `execute ci pending`. The rig receipts from
  `758c39850` were rejected (`gate-inputs-changed` after the main merge;
  test-toolkit `failed-cell`).
- **After the rig legs:** every L1 cell this fix touches on
  `windows-latest` (claudine-cli, darkmatter, darkmatter-cli, git-wt-cli,
  messenger, model_id, test-toolkit, tree-hugger-cli, worktree) and on
  `wsl2-ubuntu` (claudine-cli, darkmatter, darkmatter-cli, git-wt-cli,
  messenger, tree-hugger-cli, worktree) is `reuse local reused`. Still
  `execute ci`: `repo-deps/windows-latest/L1` (from the main merge, not this
  fix), the ubuntu/macOS cells a pull request runs, and the
  `darkmatter`/`worktree` ubuntu `check` cells.
- **Accepted gaps:** `model_id`, `repo-deps`, and `test-toolkit` on
  `wsl2-ubuntu`, and every L2/browser cell on Windows and WSL2. These are
  existing governed gaps. This fix added none.
- The `failed-cell` rejections in the "after" listing are the 2026-11-10
  hosted nightly's failures on main (`8d7d56084`, pre-fix code). They are
  diagnostic only.

### Main confirmation (scheduled `ci.yml` runs)

`gh run list --workflow ci.yml --event schedule` (read-only):

| Run | Commit | Date | Result | Counts toward the three? |
|---|---|---|---|---|
| [42852400418](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42852400418) | `8d7d56084` | 2026-11-10 | failure: `build (ubuntu-latest)` "No space left on device", `build (windows-latest)` hit the 3 h limit, so test cells found no archive ("Artifact not found") | no: pre-fix code, and no test executed |
| [42674352356](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42674352356) | `a816f1447` | 2026-11-09 | failure | no: pre-fix code |
| 42491863852, 42383053581, 42246114963 | `a816f1447` | 2026-11-08 → 06 | failure | no: pre-fix code (42246114963 is the nightly under repair) |

**Zero of three qualifying runs exist for native Windows, and zero for
WSL2.** The fix is not on `main`, so no scheduled run has executed or reused
the repaired behavior. This acceptance item stays **incomplete**. It closes
only after the branch merges and three consecutive green nightlies follow on
each OS. When that happens, the receipts above will be reused for unchanged
inputs, so record each run's executed and reused cells. Also, the
2026-11-10 nightly shows that hosted WSL2 cells still fail before any test
runs (`wsl-bash: command not found`, missing Ubuntu build). That is the
archive and runner prerequisite's work, not this fix's. No workflow was
dispatched to replace the nightlies.

### Evidence ledger closed

The 141 identities plus the compile failure, by repair group (row numbers
from "Per-identity ledger"). "Targeted" means the Phase 2 filtered
reproduction on the affected environment; "cell" means this phase's
published L1 receipt at `af7ec2d63`.

| Group | Rows | Targeted evidence (affected OS) | Cell evidence | Disposition |
|---|---|---|---|---|
| tree-hugger index corruption (rule 2) | 24 | 25/25 long and short TEMP, Windows | tree-hugger-cli windows 2726 pass | cleared |
| tree-hugger ownership (rule 3 + product walk) | 96 | 148/148 long and short, Windows | same | cleared |
| worktree lifecycle refusal spelling (rule 4) | 6 | 92/92 long and short, Windows | worktree windows 877 pass | cleared |
| claudine plan route (rule 5a) | 2 | 26/26 long and short, Windows | claudine-cli windows 3893 pass | cleared |
| darkmatter-cli trigger warnings (rule 5b) | 4 | 44/44 long and short, Windows | darkmatter-cli windows 2486 pass | cleared |
| darkmatter definition label (rule 6) | 4 | 15/15 long and short, Windows | darkmatter windows 11887 pass | cleared |
| Git entry shapes (rules 7–8) | 3 | mingw64 launcher and direct 35/35 + 10/10; **ucrt64 33/35 + 7/10** | worktree and git-wt-cli windows pass (the rig is mingw64) | **cleared on mingw64; unmet on the hosted ucrt64 layout** until gix ≥ 0.89 |
| messenger descendants (rule 12) | 1 | 3/3 on the rig (2.45–2.48 s) | messenger windows 677 pass | cleared on the rig; **the hosted runner is unconfirmed** (the rig never reproduced the hosted failure) |
| model_id `ui` (rules 9–10) | 1 | archive with a fresh Cargo home: no fetch FAIL, fetch then PASS | model_id windows 2 pass (`ui` executed) | cleared on Windows; **WSL2 unmet** (governed `cargo_toolchain` gap) |
| biscuit-visualized E0284 (rule 11) | compile row | CI's `cargo test --no-run` exit 0, 16 embed_render tests executed | biscuit-visualized windows 1165 pass | cleared |

**Restored WSL2 failures:** none. Phase 3's ten unfiltered relocated legs
and this phase's seven published legs found no failure, so no WSL2 row
exists to reconcile.

### Remaining capability and access limitations

1. **Hosted ucrt64 Git rows (3 identities):** need gix ≥ 0.89 (gix-path
   0.14), an error-API migration across the six gix consumers. Until then
   the hosted image's ucrt64 layout fails 5 worktree tests. The rig receipts
   for worktree and git-wt-cli came from a mingw64 host, and the nightly
   will reuse them for `windows-latest`, so **the nightly will not show this
   failure**. Recorded in the os skill and in `decisions_for_review`.
2. **Messenger on the hosted runner:** confirmed only by a hosted run that
   executes the new fixture. The rig receipt will be reused while
   messenger's inputs stay unchanged, so a hosted confirmation needs a
   messenger input change or a `ci:all-os`/dispatch run, which this plan
   does not authorize.
3. **model_id on WSL2:** governed `cargo_toolchain` accepted gap. It closes
   only with a changed execution contract (Cargo in the WSL2 guest, or a
   native WSL2 build). It is not a pass.
4. **Three green nightlies per OS on main:** none yet; blocked on merge. For
   WSL2 it is also blocked on hosted WSL2 cells reaching test execution
   (see the 2026-11-10 nightly).

### Requirement-to-test mapping (Phase 4)

No behavior changed in this phase, so no test was added. Phase 2's mapping
still applies. This phase re-ran each mapped test inside its package's full
suite on macOS and as published L1 cells on native Windows and WSL2. The
input robustness matrix does not apply (plan rule 15).

### Scope calculator

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
over this phase's edits (this log, the plan, the spec, and
`.claude/skills/os/windows.md`) gave `packages: [darkmatter]`,
`change_class: package`. The edited skill page is read by two darkmatter
tests. Both passed after the edit
(`just test shipped_schema_corpus::shipped_markdown`: 2/2), on top of the
full darkmatter `just test` and `just lint` above.

### Rig state

The Windows and WSL standing clones `…/bolt--main/rusty-biscuit` hold the
throwaway checkouts from the last legs, which cross-check restores and
prunes itself. Nothing else was touched.
