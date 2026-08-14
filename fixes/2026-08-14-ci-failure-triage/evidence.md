---
created: 2026-08-14
phase: 5
branch_run: 31753281913
main_run: 31588186544
status: blocked
---

# Phase 1 evidence integrity

## Frozen inputs

| Role | Workflow | Run | Branch | Head SHA | Created | Completed | Artifacts |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Branch | `ci` | [`31753281913`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913) | `fix/ctx-launch-anchor` | `a00ea7c08b6e31b1b6cd58a4588602211ed0bd17` | 2026-08-13T23:17:36Z | 2026-08-14T02:07:45Z | 762, all unexpired |
| Control | `ci` | [`31588186544`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31588186544) | `main` | `e03bafee9241fb0614c616cb246dbe49394be841` | 2026-08-12T10:36:44Z | 2026-08-12T15:04:58Z | `ci-results` available |

Runs `31706631121` and `31703954864` are newer `main` runs, but both were
canceled. Run `31588186544` is therefore the newest completed, non-canceled
`main` run before the branch run and is the same control used by the parent
fix's identity evidence.

The branch `ci-results` artifact is ID `9205201095`, 17,535 bytes, SHA-256
`bd32cc00f3b7da885ad445eb2924557572bb5d5a5b3b8f7330150e7500e2fdf1`.
The main artifact is SHA-256
`c5519cd2c6f8c12e21d3133fbf7c771431221b8f8bbe877465cf121be152a5ba`.

The temporary evidence workspace was
`/tmp/rusty-biscuit-ci-triage-phase1.xjSpP0`. It is not a committed source of
truth. This document retains the stable run, job, artifact, and digest
identifiers needed to reproduce the download.

### Artifact access note

The non-interactive host has no `GH_TOKEN` or authenticated `gh` session.
GitHub's direct artifact and complete-job-log download endpoints returned 401
or 403. The public artifact proxy indexes only the first 100 artifacts in a
run; this run has 762. It supplied `ci-results` and the four newest failed WSL2
JUnit bundles, while older per-cell archives returned 404. The complete
artifact API inventory still supplied every artifact's immutable ID, size,
expiry, and SHA-256 digest. Missing raw archives are not interpreted as empty:
the complete aggregate record is used, and the four retrievable raw bundles
provide an independent manifest/XML cross-check.

The downloaded WSL2 JUnit bundle digests are:

| Artifact | ID | SHA-256 |
| --- | ---: | --- |
| `junit-biscuit-speaks-cli-L1-wsl2-ubuntu` | `9204151628` | `ce91f76e9f18ce582fe133d13c11cf90ff6e16505a0a1ec3dae8fe193f09e0db` |
| `junit-claudine-L1-wsl2-ubuntu` | `9204364088` | `e298cc0771f8b6aba9b8178d81210c5055c5cbf6c13f523ab936b7fab00230c6` |
| `junit-claudine-cli-L1-wsl2-ubuntu` | `9205103915` | `45dde8eddbb5d07350f22fec1b5a045cb6386dcdfe0130e2c83f0fdfd1411b7c` |
| `junit-darkmatter-L1-wsl2-ubuntu` | `9204300399` | `b4f5128acf2739f6cffbf4a069ebc7466c6efc0240191b1c2b752d6f60a7a0f2` |

## Completed job inventory

The completed jobs API reports 603 jobs: 459 successful, 118 skipped, and 26
failed. Twenty-five failures are producers. The remaining failure is the
expected downstream [`ci-verdict` job
94651595195](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94651595195).

| Cell | Producer job | Failed/total | Baselined at `a00ea7c08` | Main relation |
| --- | ---: | ---: | --- | --- |
| `biscuit-speaks-cli/wsl2-ubuntu/L1` | [`94638865003`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94638865003) | 1/99 | no | subset (1/2) |
| `biscuit-terminal-cli/macos-latest/L2` | [`94635748562`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94635748562) | 1/1 | yes | equal (1/1) |
| `biscuit-terminal-cli/ubuntu-latest/L2` | [`94635748552`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94635748552) | 1/14 | yes | equal (1/1) |
| `biscuit-tui-cli/windows-latest/L1` | [`94624612538`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624612538) | 1/391 | yes | equal (1/1) |
| `claudine/wsl2-ubuntu/L1` | [`94643818956`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94643818956) | 3/4043 | no | equal (3/3) |
| `claudine-cli/macos-latest/L2` | [`94630530915`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94630530915) | 3/28 | yes | equal (3/3) |
| `claudine-cli/ubuntu-latest/L2` | [`94630530916`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94630530916) | 4/29 | yes | equal (4/4) |
| `claudine-cli/wsl2-ubuntu/L1` | [`94639713054`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94639713054) | 21/2389 | yes | mixed: 17 shared, 4 added, 9 removed |
| `darkmatter/wsl2-ubuntu/L1` | [`94640870498`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94640870498) | 7/6258 | yes | equal (7/7) |
| `darkmatter-cli/macos-latest/L2` | [`94634206790`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94634206790) | 1/3 | yes | equal (1/1) |
| `darkmatter-cli/ubuntu-latest/L2` | [`94634206852`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94634206852) | 1/68 | yes | equal (1/1) |
| `darkmatter-cli/windows-latest/L1` | [`94624611717`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624611717) | 1/655 | yes | equal (1/1) |
| `dmls/ubuntu-latest/L2` | [`94627543991`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94627543991) | 1/1 | no | equal (1/1) |
| `messenger/wsl2-ubuntu/L1` | [`94637189813`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94637189813) | 1/457 | no | equal (1/1) |
| `model_id/wsl2-ubuntu/L1` | [`94638098240`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94638098240) | 1/2 | yes | equal (1/1) |
| `rendezvous-daemon/windows-latest/L1` | [`94624612212`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624612212) | 2/155 | no | equal (2/2) |
| `sniff/macos-latest/L1` | [`94624610764`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624610764) | 2/1817 | no | equal (2/2) |
| `sniff/ubuntu-latest/L1` | [`94624610810`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624610810) | 1/1796 | no | equal (1/1) |
| `sniff/ubuntu-latest/lint` | [`94624610721`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624610721) | 0/0 | no | equal diagnostics |
| `sniff/windows-latest/L1` | [`94624610798`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624610798) | 1/1791 | no | equal (1/1) |
| `sniff/wsl2-ubuntu/L1` | [`94637944679`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94637944679) | 1/1796 | no | equal (1/1) |
| `sniff-cli/ubuntu-latest/lint` | [`94624610329`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624610329) | 0/0 | no | equal diagnostics |
| `sniff-cli/windows-latest/L1` | [`94624610505`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624610505) | 3/785 | yes | equal (3/3) |
| `sniff-cli/wsl2-ubuntu/L1` | [`94637640639`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94637640639) | 1/789 | yes | equal (1/1) |
| `unchained-ai/windows-latest/L1` | [`94624613091`](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/31753281913/job/94624613091) | 2/227 | no | equal (2/2) |

The `claudine-cli/wsl2-ubuntu/L1` relation is deliberately not reduced to a
subset or superset: it has 17 identities shared with `main`, four added on the
branch, and nine that stopped failing. The four additions reported by the real
`ci-rollup compare` path are:

- `claudine-cli::ctx_launch_anchor_baseline::cli_loop_reuses_launch_context_for_root_and_package_prompt_copies`
- `claudine-cli::ctx_launch_anchor_baseline::cli_uses_launch_context_across_launch_source_matrix`
- `claudine-cli::ctx_launch_anchor_baseline::inline_cli_uses_launch_context_across_launch_source_matrix`
- `claudine-cli::shipped_prompt_contract::shipped_context_prompt_renders_its_package_area_list_through_the_cli`

This mixed cell remains unresolved and blocks baseline acceptance until Phase 4
establishes the WSL2 fixture-level cause.

Disposition records are grouped by their owning evidence path:

- the Darkmatter CLI Windows row is governed by
  [`spec.md` F2](spec.md#f2--resolve-the-darkmatter-cli-windows-contradiction);
- the six macOS/Linux terminal rows are governed by
  [`spec.md` F3](spec.md#f3--attribute-the-level-2-rendering-failures);
- the Claudine, Claudine CLI, and Darkmatter WSL2 rows are governed by
  [`spec.md` F4](spec.md#f4--preserve-the-wsl2-contract-and-record-the-fixture-handoff);
- existing main-side identity evidence and handoffs for Biscuit Speaks CLI,
  Biscuit TUI CLI, DMLS, Messenger, `model_id`, Rendezvous, Sniff, Sniff CLI,
  and Unchained AI remain in
  [`ci-baseline-evidence.md`](../2026-08-13-finalize/ci-baseline-evidence.md#per-cell-identity-table)
  and [`problems.md`](../2026-08-13-finalize/problems.md).

## Artifact identifiers

Each JUnit-backed row has one JUnit artifact and one producer-status artifact.
The lint rows have status artifacts only. The table records the immutable IDs;
the complete API capture also records their full SHA-256 digests.

| Cell | JUnit artifact | Status artifact |
| --- | ---: | ---: |
| `biscuit-speaks-cli/wsl2-ubuntu/L1` | `9204151628` | `9204151922` |
| `biscuit-terminal-cli/macos-latest/L2` | `9204046099` | `9204046502` |
| `biscuit-terminal-cli/ubuntu-latest/L2` | `9204111502` | `9204111896` |
| `biscuit-tui-cli/windows-latest/L1` | `9203400145` | `9203400597` |
| `claudine/wsl2-ubuntu/L1` | `9204364088` | `9204364996` |
| `claudine-cli/macos-latest/L2` | `9204123405` | `9204123963` |
| `claudine-cli/ubuntu-latest/L2` | `9203777800` | `9203778082` |
| `claudine-cli/wsl2-ubuntu/L1` | `9205103915` | `9205105021` |
| `darkmatter/wsl2-ubuntu/L1` | `9204300399` | `9204300744` |
| `darkmatter-cli/macos-latest/L2` | `9204014685` | `9204015112` |
| `darkmatter-cli/ubuntu-latest/L2` | `9204084128` | `9204084406` |
| `darkmatter-cli/windows-latest/L1` | `9203005788` | `9203006259` |
| `dmls/ubuntu-latest/L2` | `9204049855` | `9204050293` |
| `messenger/wsl2-ubuntu/L1` | `9203884853` | `9203885322` |
| `model_id/wsl2-ubuntu/L1` | `9204039335` | `9204040048` |
| `rendezvous-daemon/windows-latest/L1` | `9203307862` | `9203308147` |
| `sniff/macos-latest/L1` | `9202555261` | `9202555510` |
| `sniff/ubuntu-latest/L1` | `9202488116` | `9202488559` |
| `sniff/ubuntu-latest/lint` | — | `9202556865` |
| `sniff/windows-latest/L1` | `9202627824` | `9202628088` |
| `sniff/wsl2-ubuntu/L1` | `9204111832` | `9204112217` |
| `sniff-cli/ubuntu-latest/lint` | — | `9202203597` |
| `sniff-cli/windows-latest/L1` | `9202564022` | `9202564300` |
| `sniff-cli/wsl2-ubuntu/L1` | `9204046321` | `9204046719` |
| `unchained-ai/windows-latest/L1` | `9203981619` | `9203981928` |

## Zero-identity diagnostics

The two lint cells have no JUnit records. Their current job-log download is
authorization-blocked, so the diagnostics are retained from the checked-in
capture of these exact jobs and cross-checked against the byte-identical main
run diagnostics already recorded in
[`ci-baseline-evidence.md`](../2026-08-13-finalize/ci-baseline-evidence.md).
Neither offending source changed between `e03bafee9` and `a00ea7c08`.

`sniff-cli/ubuntu-latest/lint`:

- `clippy::collapsible_if` at `sniff/cli/tests/snapshots.rs:672:17`.

`sniff/ubuntu-latest/lint`:

- `unused_imports` (`use super::*`) at `sniff/lib/src/services/launchd.rs:40:9`.
- `unused_variables` (`helpers`) at `sniff/lib/src/programs/notification_helpers.rs:169:13`.
- `permissions_set_readonly_false` at `sniff/lib/tests/merge_conflict_prediction.rs:480:5`.
- `items_after_test_module` at `sniff/lib/src/hardware/storage.rs:218:1`.
- `items_after_test_module` at `sniff/lib/src/programs/enums/metadata.rs:3324:1`.
- `zombie_processes` at `sniff/lib/src/process.rs:845:9`, `:863:26`,
  `:907:9`, `:937:26`, and `:989:26`.

## Independent closure

The independent sources reconcile as follows:

- completed jobs API: 603 total = 459 success + 118 skipped + 25 failed
  producers + 1 failed verdict;
- `ci-results.cells`: 25 `FAIL` producer cells, one row per failed producer;
- `ci-results.records`: 23 failed JUnit records;
- aggregate JUnit totals: 61 failing cell-occurrences and 55 distinct test
  identities;
- zero-identity diagnostics: 11 normalized diagnostics across two lint cells;
- downloaded raw WSL2 XML: 32 failures across four cells, exactly matching the
  corresponding aggregate rows;
- baseline at `a00ea7c08`: 13 of the 25 failed producer keys already present;
- `ci-rollup compare`: exit 2, four added identities, 13 removed identities,
  18 pre-existing failures carried by changed cells, and 439 identical cells.

The corrected catalog contains all 25 producer rows exactly once. Equality or
subset evidence supports the existing main-side candidates; it does not close
the Phase 2 same-host attribution requirements. The mixed Claudine CLI WSL2
cell is explicitly unresolved. No Phase 1 classification rests on package
ownership or subsystem scope.

## Phase 2 attribution

Phase 2 made no implementation, fixture, snapshot, or baseline change. The
tests below therefore map the existing public terminal and CLI contracts; no
new behavior needed a new regression test.

### Requirement-to-test map

| Observable contract | Exact regression input and variants | Verification |
| --- | --- | --- |
| Darkmatter pretty validation preserves legacy bytes and exit status | All eight shipped `schema_validate_baseline` cases, including the original failing `property_union_invalid`; pretty output with `NO_COLOR=1` and the sibling JSON representation | `schema_validate_legacy_pretty_output_is_byte_identical` and `schema_validate_legacy_json_output_is_byte_identical` exercise the real `md` binary, copied shipped fixtures, output bytes, document link, and success/error status. The pretty test is the focused Windows identity. |
| Claudine context reports retain their columns, wrapping, and 140-column cap | Real `claudine context` in tmux at 78, 140, and 160 columns, including the exact four CI identities | `level2_context_default_narrow_preserves_type_and_wraps_in_tmux`, `level2_context_default_at_140_fills_cap_in_tmux`, `level2_context_default_caps_at_140_in_wide_tmux`, and `level2_context_default_preserves_columns_at_min_width_in_tmux` assert rendered columns, rows, bounds, catalog content, and failure text. |
| A dim parent does not prevent the inverted code-panel background | `printf '\033[2m'`, `COLORFGBG=15;0`, `--max-width 60`, and the shipped Rust code document | `level2_code_block_clears_inherited_dim_before_theme_colors` captures the real `md` screen up to three times and asserts a truecolor background with luma above 175. |
| A light terminal selects the dark OneHalf code face | `COLORFGBG=0;15`, `THEME=one-half`, `CODE_THEME=one-half`, and real `md schema about` output | `level2_schema_about_light_terminal_uses_dark_code_theme` asserts the exact background, key, and scalar RGB values on the shipped schema-about YAML example. Its dark-terminal sibling is the representation control. |
| Apple Terminal degrades double underline to visible text | Exact input `<double-underline>important text</double-underline>` through the real `bt prose` command | `level2_apple_terminal_double_underline_plain_text_visible` asserts bounded visible text and negatively asserts both literal and raw unsupported SGR fragments. |
| tmux without an image protocol renders Mermaid source | Exact input `bt flowchart "A --> B"` | `level2_diagram_fallback_when_no_image_protocol` asserts the fenced fallback and negatively asserts Kitty and iTerm2 image protocol bytes. |

The passive-artifact and read/write/read requirements are not applicable:
Phase 2 changes no parser, schema, template, prompt, configuration, or
persistence behavior. The Darkmatter baseline test already iterates the full
shipped legacy fixture corpus and invokes the real CLI.

### macOS matched controls

Both revisions ran on the same Apple M4 Max host: macOS 27.0 (26A5406e),
arm64, tmux 3.7b, Apple Terminal 2.15, and WezTerm
20260716-195552-76b606ec. The shell environment carried
`TERM=xterm-256color`, `COLORTERM=truecolor`, and `NO_COLOR=1`. Test-specific
variables and pane dimensions came from each regression input above. Fixture
commands resolved to the binaries built from the corresponding checkout.

The branch checkout was `ffd8f2e546425fa8bf8a665eb48caa0ca03da560`.
The clean detached control checkout was
`e03bafee9241fb0614c616cb246dbe49394be841`. Each package area ran the exact
requested command:

```console
BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 --no-fail-fast
```

| Package area | Branch full-suite result | Main full-suite result | Exact Phase 2 identities |
| --- | --- | --- | --- |
| Claudine | `claudine-cli`: 228 passed, 2 failed (230 total) | `claudine-cli`: 218 passed, 11 failed, 1 timed out (230 total) | All three macOS context identities, plus the Ubuntu-only minimum-width identity used as a control, passed on both revisions. |
| Darkmatter | library 18/18 passed; CLI 31 passed, 37 failed, 1 timed out; DMLS 3/3 passed | library 1 passed, 16 failed, 1 timed out; CLI 7 passed, 62 failed; DMLS 3/3 passed | Both the dim/code-panel and schema-about identities passed on both revisions. |
| Biscuit Terminal | library 0/2 passed; CLI 6 passed, 65 failed, 5 timed out | library 1/2 passed; CLI 17 passed, 58 failed, 1 timed out | The tmux diagram identity passed on both revisions. The Apple identity timed out during branch harness setup and passed on main; the assertion itself did not fail. |

The unrelated local failures are harness availability/state failures caused by
hard-requiring every L2 backend on a developer host: OSC-query timeouts,
unavailable WezTerm/Kitty panes, and later tests losing the shared pane. Two
Claudine WezTerm tests also failed for that reason. They do not change the
target dispositions.

The Apple Terminal target is a harness/environment defect, not a product-byte
regression: the branch failure occurred in `attach/spawn Apple Terminal`
before the command or assertion ran, while the same identity passed on the
main control. The independent hosted macOS comparison is equal—both branch
job 94635748562 and the selected main run time out in Apple Terminal harness
setup—and the relevant CI frame is retained in
[`macos.md`](../2026-08-13-finalize/macos.md#biscuit-terminal-cli-l2-level2_apple_terminal_proselevel2_apple_terminal_double_underline_plain_text_visible).
No identity is classified as flaky, so Task 2.7's repeat protocol is not
invoked.

### Linux matched controls

The selected completed branch and main workflows are the matched Ubuntu host
controls. Both provision tmux and execute the canonical `_test_l2` nextest
recipe with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`; no result from this macOS
host substitutes for Linux. Run `31753281913` and main run `31588186544` have
equal identity sets in all three affected cells:

| Cell | Branch job | Identity-level result | Screen/byte cause and disposition |
| --- | ---: | --- | --- |
| `claudine-cli/ubuntu-latest/L2` | 94630530916 | Equal, 4/4 | All four frames show the first-run “Welcome to Claudine!” wizard because the tmux shell has an unseeded HOME; `claudine context` never reaches the table renderer. Main-side fixture isolation defect. |
| `biscuit-terminal-cli/ubuntu-latest/L2` | 94635748552 | Equal, 1/1 | The pane contains `bt: command not found` after the exact `bt flowchart "A --> B"` input. The harness types a bare binary name that is absent from the login shell PATH. Main-side fixture executable-path defect. |
| `darkmatter-cli/ubuntu-latest/L2` | 94634206852 | Equal, 1/1 | The simulated light-terminal row contains OneHalf Light background `48;2;250;250;250`, not the required OneHalf Dark `48;2;40;44;52`. Main-side terminal color-mode/harness defect. |

The exact captured commands, frames, RGB bytes, and downstream assertions are
retained in [`linux.md`](../2026-08-13-finalize/linux.md#claudine-cli-level2_context_capture-width-cap-tests-4),
[`linux.md`](../2026-08-13-finalize/linux.md#biscuit-terminal-cli-level2_diagramslevel2_diagram_fallback_when_no_image_protocol),
and [`linux.md`](../2026-08-13-finalize/linux.md#darkmatter-cli-level2_schema_aboutlevel2_schema_about_light_terminal_uses_dark_code_theme).
The current-run aggregate proves those identities remain equal; the preceding
same-branch capture supplies the raw frame because the non-interactive host
could not authenticate to download the expired per-cell archives.

### Windows track: unresolved host prerequisite

This macOS host has no native Windows runtime, Windows VM, or reachable Windows
test host. Docker is not running, and the local Podman VM is unavailable. The
required focused branch and main commands therefore cannot be executed on the
same native Windows 11 host that produced the recorded 655-test aggregate.
Tasks 2.1, 2.2, and Validation checkpoint 2 remain open; an aggregate green
count is deliberately not treated as proof that the identity executed.

The available `windows-latest` evidence does preserve the original failing
input and byte delta. For `property_union_invalid`, both branch job
94624611717 and the selected main cell fail the same identity. The CLI emits:

```text
file://C:\Users\RUNNER~1\AppData\Local\Temp\<tempdir>\doc.md
```

while the expected string rebuilt after `fs::canonicalize` contains:

```text
file://C:\Users\runneradmin\AppData\Local\Temp\<tempdir>\doc.md
```

The JSON representation passes, and the pretty test also checks the invalid
case's nonzero exit status. This isolates the contradiction to `%TEMP%` 8.3
short-name expansion in the pretty document URL. It is a strong environmental
explanation, but it is not promoted to closed attribution until the focused
identity runs on the recorded native host. The test comment claiming that the
CLI emits a canonical path has drifted from the observed code behavior; Phase
2 leaves that comment untouched because this phase permits no source edit.

### Phase 2 scope verification

No source code, fixture, expected-output snapshot, agent skill, or
`.github/ci/ci-baseline.toml` file changed during this phase. The only Phase 2
changes are this evidence section and plan progress/frontmatter. Phase 2 is
blocked solely on the unavailable native Windows matched-host execution.

## Phase 3 branch-owned repair checkpoint

### Frozen implementation scope

Phase 2 produced no branch-regression row to repair:

- the four Claudine context identities have equal branch and `main` failure
  sets on Ubuntu and pass on both revisions on the matched macOS host;
- the Darkmatter code-panel and schema-about identities have equal branch and
  `main` failure sets in their hosted cells and pass on both revisions on the
  matched macOS host;
- the Biscuit Terminal diagram identity has an equal branch and `main` failure
  set on Ubuntu and passes on both revisions on the matched macOS host;
- the Apple Terminal identity fails during harness setup in both hosted macOS
  cells, before the product command or terminal assertion executes; and
- the Darkmatter CLI Windows identity has the same failing identity and byte
  delta on the branch and `main` `windows-latest` cells. Native Windows
  reproduction remains a Phase 2 attribution prerequisite, but there is no
  evidence-backed branch-owned row from which Phase 3 could derive a repair.

The Phase 3 source/test worklist is therefore empty. In accordance with Task
3.1, no production code, test, fixture, snapshot, expected output, terminal
assertion, timeout, environment contract, or CI baseline is changed. The
Windows comment drift noted above is intentionally not used to manufacture a
comment-only source change in this branch-repair phase; it remains attached to
the open native-Windows attribution record.

### Requirement-to-test map

Phase 3 changes no public behavior. The existing targeted tests in the Phase 2
map remain the acceptance tests for the original failing inputs and their
dependent outputs:

| Disposition preserved by Phase 3 | Exact existing verification |
| --- | --- |
| Windows pretty output is not re-baselined without byte-level attribution | `schema_validate_legacy_pretty_output_is_byte_identical`, with all eight shipped cases including `property_union_invalid`, plus the JSON representation control and exit-status assertions |
| Claudine context layout assertions remain intact | The four exact `level2_context_capture` identities at 78, 140, and 160 columns |
| Darkmatter terminal color assertions remain intact | `level2_code_block_clears_inherited_dim_before_theme_colors` and `level2_schema_about_light_terminal_uses_dark_code_theme`, including the dark-terminal representation control |
| Biscuit Terminal fallback assertions remain intact | `level2_diagram_fallback_when_no_image_protocol` and `level2_apple_terminal_double_underline_plain_text_visible`, including their negative protocol/SGR assertions |

No regression test is added because there is no changed behavior and no
branch-owned cause. Passive shipped-artifact coverage, an end-to-end real
artifact path, and representation variants are already present in the
Darkmatter baseline suite. No parser, schema, template, prompt,
configuration-driven behavior, or persisted value changes in Phase 3, so no
new passive corpus or read/write/read round-trip test applies.

### Verification and skipped gates

The Phase 2 branch suites ran at
`ffd8f2e546425fa8bf8a665eb48caa0ca03da560`, which remains the current Phase 3
code revision. No source file changed after those runs. Their focused results
therefore remain the verification evidence for this no-op phase:

- the four Claudine context identities passed on the branch and `main` macOS
  controls;
- both Darkmatter terminal identities passed on the branch and `main` macOS
  controls;
- the Biscuit Terminal tmux diagram identity passed on the branch and `main`
  macOS controls; and
- the Apple Terminal command/assertion did not execute in the hosted failure,
  so Phase 3 does not claim a product-test pass from that setup timeout.

Targeted tests added: none. Corrected identities to rerun under Task 3.4: none.
Source packages changed under Tasks 3.2-3.3: none. The conditional package
gate was empty, but the broader user-level completion gate was run across all
three historically affected package areas:

- `just test claudine darkmatter biscuit-terminal` passed all 13 selected
  packages: `biscuit-terminal`, `biscuit-terminal-cli`, `claudine`,
  `claudine-catalog-types`, `claudine-cli`, `claudine-contract`,
  `claudine-gen`, `darkmatter`, `darkmatter-cli`, `dmls`,
  `rendezvous-client`, `rendezvous-core`, and `rendezvous-daemon`;
- `cd biscuit-terminal && just lint` passed;
- `cd claudine && just lint` passed, including its 18-test diagnostic guard;
  and
- `cd darkmatter && just lint` passed.

The focused Darkmatter baseline tests passed as part of the Level-1 gate:
`schema_validate_legacy_pretty_output_is_byte_identical` and
`schema_validate_legacy_json_output_is_byte_identical`. No fresh Level-2 run
was needed because the canonical Level-2 branch suites recorded above ran at
the unchanged current code revision.

The pre-existing/skipped results remain explicit:

- native Windows focused branch/control execution is unavailable on this host,
  leaving Phase 2 Tasks 2.1, 2.2, and checkpoint 2 open;
- the Phase 2 macOS full suites include unrelated hard-required backend,
  OSC-query, shared-pane, and Apple Terminal setup failures recorded above;
- the hosted Ubuntu failures are equal to `main` and are assigned to
  main-side fixture or harness handoffs; and
- no Windows, Linux, or WSL2 runtime is available locally for an additional
  cross-platform execution in Phase 3.

`git diff --check` passes for the tracked documentation diff. The Phase 3 diff
does not change a terminal assertion, timeout, environment contract, source
file, fixture, snapshot, or baseline. Validation checkpoint 3 is therefore
complete with an empty branch-owned set. Phase 5 remains blocked by the open
native-Windows attribution checkpoint and the unfinished Phase 4 work, not by
a branch-owned regression.

## Phase 4 WSL2 comparison and fixture handoffs

### Requirement-to-test map

| Observable contract | Original failing input and variants | Verification |
| --- | --- | --- |
| Direct composition anchors every launch-source variant without sharing one WSL2 timeout budget | The exact `launch-area`, `opposing-area`, `external-repo`, and `repo-root` documents formerly aggregated by `cli_uses_launch_context_across_launch_source_matrix` | Four independently executed `rstest` cases under that test name; each asserts body, frontmatter, preflight, lifecycle, conditional, repository, and area outputs. |
| Inline composition anchors the same launch-source variants | The exact four documents formerly aggregated by `inline_cli_uses_launch_context_across_launch_source_matrix` | Four independently executed cases under the inline matrix test with the same dependent prompt and stderr assertions. |
| Loop composition reuses one launch context for both prompt-copy locations | The exact `repo-root` and `launch-area` cases formerly aggregated by `cli_loop_reuses_launch_context_for_root_and_package_prompt_copies` | Two independently executed cases under the loop test; both also assert the provider ran exactly twice. |
| The shipped context prompt renders the discovered package-area list through the real CLI | The real `prompts/context.md` artifact, repository-root working directory, fake Codex provider, and `-y --codex` invocation from job 94639713054 | `shipped_context_prompt_renders_its_package_area_list_through_the_cli` retains the original positive and unevaluated-expression negative assertions while prepending, rather than replacing, the provider directory on `PATH`. `shipped_prompts_have_parseable_schemas_and_expressions` remains the passive full-corpus check. |
| WSL2 remains the full canonical Level-1 package suite from a Linux-built archive | All three package cells and every ordinary L1 identity, with no Cargo or rustc in the guest | The immutable JUnit manifests/XML from branch run 31753281913 and main run 31588186544 provide the cell comparison; `.github/workflows/_wsl-ci.yml` remains the end-to-end contract. |

No parser, schema, template, prompt, configuration, or persisted value changes
in Phase 4. The passive corpus and real-shipped-artifact requirements are
already exercised by the two shipped-prompt tests above. A read/write/read
round trip is not applicable.

### Canonical branch-versus-main cells

The comparison uses only `.github/workflows/_wsl-ci.yml` artifacts. Both runs
execute the Linux-built nextest archive as the unprivileged `biscuit` user in a
real `wsl2-ubuntu` guest. The workflow explicitly rejects Cargo or rustc in the
guest; a development WSL installation is not comparison evidence.

| Cell | Branch artifact/job | Main relation | Phase 4 disposition |
| --- | --- | --- | --- |
| `claudine/wsl2-ubuntu/L1` | JUnit artifact 9204364088; job 94643818956; 3/4043 failed | Equal, 3/3 | Main-side hermetic-fixture handoff. |
| `darkmatter/wsl2-ubuntu/L1` | JUnit artifact 9204300399; job 94640870498; 7/6258 failed | Equal, 7/7 | Six rustc fixture assumptions plus one pre-existing 30-second ambient-catalog timeout. |
| `claudine-cli/wsl2-ubuntu/L1` | JUnit artifact 9205103915; job 94639713054; 21/2389 failed | Mixed: 17 shared, 4 added, 9 removed | The four additions are branch-added end-to-end tests. Their test-fixture correction is implemented, but a fresh WSL2 artifact is still required. |

The four branch-only identities in run 31753281913 were:

- `claudine-cli::ctx_launch_anchor_baseline::cli_loop_reuses_launch_context_for_root_and_package_prompt_copies`;
- `claudine-cli::ctx_launch_anchor_baseline::cli_uses_launch_context_across_launch_source_matrix`;
- `claudine-cli::ctx_launch_anchor_baseline::inline_cli_uses_launch_context_across_launch_source_matrix`; and
- `claudine-cli::shipped_prompt_contract::shipped_context_prompt_renders_its_package_area_list_through_the_cli`.

The first three each put two or four independent real-CLI cases under one
90-second nextest budget. Other single-invocation tests in the same fixture
completed in 22.1–64.8 seconds in the archive-only guest. Phase 4 preserves
every input and assertion but gives each case its own test identity and timeout
budget. The shipped-prompt test replaced `PATH` with only its fake provider
directory while invoking repository discovery; it now prepends that directory
to the inherited system path, matching the other real-CLI fixtures. The fresh
WSL2 run remains the proof for both corrections.

The pre-change focused canonical run passed the four aggregate identities on
macOS in 3.55 seconds. The post-change focused run passed all 11 split and
shipped-artifact identities in 3.46 seconds. The exact targeted identities are:

- `cli_uses_launch_context_across_launch_source_matrix::case_1_launch_area`;
- `cli_uses_launch_context_across_launch_source_matrix::case_2_opposing_area`;
- `cli_uses_launch_context_across_launch_source_matrix::case_3_external_repository`;
- `cli_uses_launch_context_across_launch_source_matrix::case_4_repository_root`;
- `inline_cli_uses_launch_context_across_launch_source_matrix::case_1_launch_area`;
- `inline_cli_uses_launch_context_across_launch_source_matrix::case_2_opposing_area`;
- `inline_cli_uses_launch_context_across_launch_source_matrix::case_3_external_repository`;
- `inline_cli_uses_launch_context_across_launch_source_matrix::case_4_repository_root`;
- `cli_loop_reuses_launch_context_for_root_and_package_prompt_copies::case_1_repository_root`;
- `cli_loop_reuses_launch_context_for_root_and_package_prompt_copies::case_2_package_area`; and
- `shipped_context_prompt_renders_its_package_area_list_through_the_cli`.

These are deterministic local regression results, not substitutes for the
pending WSL2 comparison.

The broader affected-area gates also pass on macOS:

- `cd claudine && just build --color never` built all five Claudine packages;
- `cd claudine && just test --no-fail-fast` passed 21 catalog, 4,043 library,
  48 contract, 2,396 CLI, and 154 generator tests; and
- `cd claudine && just lint` passed the 18-test diagnostic guard, lifecycle
  documentation guard, rustfmt check, and Clippy for all five packages.

No Level-2 test is added or changed: the correction affects ordinary L1 tests
that spawn the CLI without a terminal harness. Native Windows, Linux, and WSL2
execution are unavailable on this macOS host. The tests remain cross-platform:
the matrix cases use the existing native provider fixture path on Windows, and
the shipped-prompt `PATH` import remains Unix-gated with its test.

### Darkmatter fixture classes

The corrected seven-identity Darkmatter ledger is in
[`../2026-08-13-finalize/failing.md`](../2026-08-13-finalize/failing.md). Six
identities depend on the unavailable compiler:

- `detects_no_cache_suffix`, `no_cache_defaults_false_without_suffix`, and
  `no_cache_combines_with_timeout_either_order` do not execute a process. The
  parser's command/property ladder consults `PATH` because the sole bare token
  `rustc` is ambiguous. With no rustc, classification changes before the
  no-cache or timeout suffix is tested.
- `frontmatter_literal_survives_shell_bracketed_interpolation_passes`,
  `shell_block_execution_failed_renders_inner_diagnostic`, and
  `shell_block_origin_counts_lines_not_bytes_with_crlf` execute `rustc` for
  output or controlled nonzero stderr.

All six identities predate this branch and are present on main run 31588186544.
The seventh identity, `every_catalog_variable_survives_ambient_options`, is a
main-identical 30-second WSL2 timing failure and is retained separately rather
than misreported as a compiler fixture.

### Durable main-side fixture handoffs

- Darkmatter's three parser-only cases should use a path-bearing dummy token
  such as `./fixture-command`. The parser can then classify it without `PATH`;
  the fixture need not exist because these tests do not execute it.
- Darkmatter's interpolation and coordinate/diagnostic cases should resolve a
  repository-owned cross-platform fixture executable from the nextest archive.
  Stable modes should emit predicted stdout or controlled stderr/nonzero status
  so the tests keep their subprocess-result and source-coordinate assertions.
- Claudine's diagnostic test needs the same controlled-failure fixture. Its two
  cwd tests should execute an archived cross-platform cwd probe directly rather
  than compile one with rustc at test runtime.
- The recovered Claudine CLI branch cell contains one rustc diagnostic fixture,
  one missing sibling-`md` fixture, and 19 timing-sensitive identities. After
  removing the four branch-only cases above, the 17 shared failures are the one
  rustc fixture, the `md` fixture, and 15 timing identities; main contributes
  nine additional timing identities. Main-side cleanup should deliver the
  required fixture binaries in the archive and replace aggregate wall-clock
  assumptions with isolated observable-result tests, without early returns or
  nominal passes.

### Baseline reason drafts

Phase 4 does not edit `.github/ci/ci-baseline.toml`. Phase 5 may apply these
reasons only after a fresh branch artifact satisfies the no-superset gate:

- Existing `darkmatter/wsl2-ubuntu/L1` entry, retaining expiry `2026-11-30`:
  “Main-identical archive-fixture assumptions on branch run 31753281913 versus
  main run 31588186544: three parser-only no-cache fixtures use an ambiguous
  bare rustc token and consult PATH, three subprocess fixtures require rustc,
  and the ambient catalog sweep exceeds its 30-second WSL2 budget (7 exact
  identities).”
- Parent-ratified `claudine/wsl2-ubuntu/L1` entry, retaining expiry
  `2026-09-30`: “The toolchain-free archive guest has no rustc; three
  main-identical tests use it as a diagnostic or cwd-probe fixture and take the
  command-not-found path. Exact 3-identity equality on branch run 31753281913
  and main run 31588186544.”

### WSL2 contract audit and checkpoint status

The Phase 4 diff does not modify `.github/workflows/_wsl-ci.yml`,
`.github/ci/environments.json`, or `.github/ci/ci-baseline.toml`; does not add a
toolchain to the guest; does not narrow a package or filter; and does not add an
early return or skip. The full canonical Level-1 contract is intact.

Tasks 4.1–4.5 are complete. Validation checkpoint 4 remains blocked on a fresh
canonical WSL2 run containing the split tests and restored `PATH`: run
31753281913 is still a four-identity Claudine CLI superset, while an uncommitted
worktree cannot produce an authoritative Actions artifact. Phase 5 must not
apply or retain a cell-wide Claudine CLI baseline until the fresh JUnit set is
equal to or a subset of main.

## Phase 5 entry-gate audit

Phase 5 cannot pass Task 5.1 in the current repository state. Validation
checkpoint 3 is complete, but validation checkpoint 4 remains open for the
fresh canonical WSL2 result described above. The public GitHub Actions API was
queried on 2026-08-14 and confirms that the newest `ci` run for
`fix/ctx-launch-anchor` is still run 31753281913 at
`a00ea7c08b6e31b1b6cd58a4588602211ed0bd17`. That run predates both the Phase 4
test corrections and local documentation commits.

The local branch is two commits ahead of `origin/fix/ctx-launch-anchor` at
`ffd8f2e546425fa8bf8a665eb48caa0ca03da560`, and the two Phase 4 test files are
still uncommitted. This phase's execution contract forbids committing, staging,
or applying policy against uncommitted or undispatched fixes. The host's GitHub
CLI is also unauthenticated; the non-interactive session cannot authenticate or
dispatch a workflow. Consequently:

- no qualifying branch `ci-results` artifact exists for Task 5.2;
- `.github/ci/ci-baseline.toml` remains unchanged under Tasks 5.3–5.4;
- no authoritative full CI run can be dispatched under Task 5.5;
- Tasks 5.6–5.7 and validation checkpoint 5 cannot claim acceptance without
  that run and its identity-aware comparison; and
- no Phase 5 todo is checked off.

This is a gate failure, not a test failure and not permission to accept the
known Claudine CLI WSL2 superset. Recovery requires the Phase 4 source and
documentation changes to be committed and pushed by the separate authorized
process, followed by a fresh full CI run. Phase 5 can then resume at Task 5.1
using that run and a current completed `main` control.

### Phase 5 behavior-to-test map and local verification

Phase 5 changes no source, parser, schema, template, prompt, configuration, or
persisted behavior. No new behavior test, passive corpus test, end-to-end test,
or read/write/read round trip applies. The existing Phase 4 regression map
remains the targeted coverage for the pending WSL2 corrections.

The affected working tree was nevertheless reverified locally on macOS:

- `cd claudine && just test --no-fail-fast` passed the complete Level-1 suite
  for all five Claudine packages, including all 11 split launch-context and
  shipped-prompt regression identities; and
- `cd claudine && just lint` passed the 18-test diagnostic guard, lifecycle
  documentation guard, rustfmt check, and Clippy for all five packages.

The only emitted build warning was the pre-existing macOS linker message that
the `__eh_frame` section is too large for compact unwind offsets. Native
Windows, Linux, WSL2, full CI, `ci-rollup verdict`, and `just ci-diff` remain
skipped because no dispatched revision contains the Phase 4 changes. No source
or skill file was changed during Phase 5, and no comment contract required an
update.
