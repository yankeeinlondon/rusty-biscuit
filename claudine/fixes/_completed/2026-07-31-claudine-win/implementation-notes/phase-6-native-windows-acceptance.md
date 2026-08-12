# Phase 6 native Windows acceptance

Date: 2026-08-01
Host: native Windows, branch `fix/claudine-windows`
Status: **native Windows completion contract met; portability follow-ups deferred**

This note records the acceptance evidence available at the Phase 6 closeout
boundary. The final native Windows CI-profile and canonical L1 reruns are green
on the `x86_64-pc-windows-msvc` host, which is the revised completion authority.
Linux execution was attempted but became inconclusive after WSL
subsystem/filesystem failures; macOS, xwin, and GNU cross-check evidence remains
unavailable. Those checks are explicitly deferred and do not block lifecycle
completion.

## Implementation history

The implementation is split into the planned, sequentially landable commits:

| Commit | Scope |
|---|---|
| `bf50d4569` | Align ownership between the July 29 security tranche and July 31 umbrella fix. |
| `ac68ddb7e` | Centralize separator-neutral security path matching. |
| `cdbc33e9b` | Retry measured transient Windows atomic-replacement failures. |
| `4b664428d` | Render library paths portably and construct real file URLs. |
| `3878bb6dc` | Render CLI paths, completions, and reports portably. |
| `448019e09` | Harden native Windows process behavior and process fixtures. |
| `5f82a4f34` | Preserve native file identity while giving Darkmatter interpolation a portable presentation value. |
| `3613e1157` | Close the remaining native Windows path, discovery, link, configuration, and fixture failures. |
| `75d1fda62` | Reuse generator discovery state so drift checks remain inside the ordinary timeout. |
| `9616f403d` | Close the final Windows correctness, test-placement, process-reaping, lint, and scheduler findings. |
| `5d3c20c8e` | Clear the Messenger desktop-provider findings that blocked the area lint gate. |
| `15461dd71` | Clarify Darkmatter's native eager-file identity and portable presentation contract. |

This acceptance note is the only plan-owned closeout artifact still staged at
the evidence boundary described here.

## Native Windows gate record

Logs are retained under
`C:\Users\ken\AppData\Local\Temp\claudine-phase6-final-20260801-062253`.
The constrained package set is `claudine-catalog-types`, `claudine`,
`claudine-contract`, `claudine-cli`, and `claudine-gen`.

| Gate | Evidence | Status |
|---|---|---|
| Constrained test build | `09-final-build-tests.log`: `cargo build --tests -j 4` finished successfully in 1m 56s after the lint-closure edits. | Passed |
| Area sanity | `10b-final-claudine-just-sanity.log`: all five packages passed. Package summaries were 21/21 catalog-types tests, 3,870/3,870 library tests, 47/47 contract tests, 1,619/1,619 CLI tests, and 93/93 generator tests. | Passed |
| Area lint | `03i-lint-through-generator.log`: the 18 transport guards passed, the lifecycle-doc-facets guard passed, and all five packages completed Clippy with warnings denied. This preceded the last build-only verification, so it must be rerun if later worktree edits touch linted source. | Passed at lint-closure snapshot |
| Area doctest | `11-final-claudine-just-doctest.log`: catalog types and generator had zero doctests; Claudine passed 20 runnable doctests with 7 ignored plus 3 compile-fail doctests; the contract passed 2/2; the CLI has no library target. | Passed |
| Deterministic CI-profile L1 | `12-final-claudine-ci-test.log` ran 6,191 tests and passed 6,190 with 13 skipped and no timeout; the sole failure was committed dispatch-inventory line drift. After the inventory-only correction, `15-final-claudine-ci-test-rerun.log` passed 6,191/6,191 with 13 skipped, retries disabled, in 227.6 seconds. | Passed |
| Canonical local L1 | `16-final-claudine-canonical-test.log` passed 6,191/6,191 with 13 skipped, no failure, no timeout, and no flaky test in 244.3 seconds. | Passed |
| Rendezvous live-daemon coverage | The focused native Windows live-daemon set passed 5/5 with `rendezvous-daemon` and DuckDB enabled. | Passed |

The failed inventory guard did not expose a new dispatch site. Logs 13 and 14
regenerated and then independently verified the inventory, with both runs
passing 12/12 tests. The generated diff updates exactly 16 existing entries by
`+2` lines after two test-fixture lines were added; provider sets, forms,
dispatch classes, and the number of sites are unchanged.

Before the lint-closure edits, two complete area runs established a useful but
non-final reference point:

- `05-test-ci.log`, CI profile with retries disabled: all five packages passed;
  catalog types 21/21, Claudine 3,973/3,973, contract 47/47 with 4 skipped,
  CLI 1,996/1,996 with 8 skipped, and generator 154/154 with 1 skipped. Total:
  6,191 passed, 13 skipped, 0 failed, and 0 timed out.
- `06c-test-canonical-group-cap.log`, canonical recipe with the bounded Cargo
  group cap: the same 6,191 tests passed with 13 skipped, 0 failed, and
  0 timed out. Two earlier canonical attempts without that cap timed out under
  suite contention; their focused tests and the bounded rerun passed, so they
  are retained as resource-accounting evidence rather than accepted final
  results.

No L2, L3, browser, or real-provider tier is required: the changes exercise L1
filesystem, rendering, process, configuration, and generator behavior.

## Deferred portability follow-up record

| Required evidence | Status |
|---|---|
| Linux Claudine build, lint, doctest, and L1 tests | **Deferred after inconclusive environment**. The constrained five-package build passed from the WSL-mounted worktree after one source correction, but mounted-tree sanity timed out and the verified native-Linux snapshot then encountered WSL subsystem/filesystem failure during its cold build. No later Linux gate ran. |
| macOS Claudine build, lint, doctest, and L1 tests | **Deferred; runner unavailable**. No exact-candidate macOS runner was available. Local `HEAD` plus the dirty worktree has no remote CI result, and the available `gh` token is invalid. |
| `cargo xwin check --target x86_64-pc-windows-msvc -p claudine --all-targets` from a supported non-Windows host | **Deferred; tool unavailable**. The xwin tooling is not installed. |
| `just check-windows` GNU-target compile check | **Execution deferred; tool unavailable**. The MinGW compiler/target is not installed. `cargo tree --target x86_64-pc-windows-gnu` nevertheless verifies that `rendezvous-daemon -> duckdb -> libduckdb-sys` remains in the target graph. This is graph evidence, not a successful GNU compilation. |

The first WSL mounted-tree build found one real portability regression:
`provider_error_finalize.rs` imported a Windows-gated fixture helper without
the matching `cfg`. After the one-line import correction, the exact constrained
five-package build passed in 90.7 seconds.

Mounted-tree `just sanity` then ran for 829.7 seconds and failed solely on 24
repeated 30-second timeouts: 12 in the Claudine library and 12 in the CLI. The
source tree was mounted from NTFS, so that run is not accepted as native-Linux
timing evidence.

To remove the mounted-filesystem variable, a native-Linux snapshot was created
and verified byte-for-byte against all 67 worktree entries. Its cold constrained
build exited after 575.5 seconds without a Rust diagnostic. WSL then reported
subsystem/filesystem failures (`getpwnam/getpwuid failed 5` and `/bin/sh` I/O
error), so no reliable compiler verdict or later Linux lint, doctest, or L1
result exists.

These results are not inferred portability claims. The successful mounted-tree
build and host-independent tests reduce risk, while a complete native-Linux
run, a macOS run, and the cross-compiles remain explicit follow-up evidence.
DuckDB dependency selection is not conditional on those follow-ups:
`rendezvous-daemon` and its DuckDB pipeline are required on every platform.

## Focused evidence by implementation phase

### Security matching

The private `path_semantics` comparison grammar recognizes POSIX-rooted,
Windows drive-rooted, and UNC absolute spellings, rejects drive-relative
spellings, normalizes both separators, and owns exact-or-descendant boundary
semantics. Focused regressions include:

- `path_separator_spellings_are_interchangeable` and
  `portable_directory_matching_covers_every_pattern_form`;
- `directory_prefix_requires_a_segment_boundary` and
  `descendant_match_requires_a_segment_boundary`;
- `windows_directory_deny_rule_blocks_child_path` and
  `windows_directory_allow_rule_grants_child_path`;
- `windows_home_credential_paths_are_sensitive_on_every_host`;
- `windows_absolute_allow_is_portable_and_boundary_aware` and
  `windows_drive_relative_allow_is_not_absolute`; and
- `windows_sensitive_path_is_blocked_through_protect_service` and
  `windows_sensitive_path_allow_rule_reaches_protect_service`.

`path_matching_modules_keep_separator_semantics_centralized` is the source
inventory guard. The deleted warning name remains only as a forbidden needle
inside that guard; it has no definition or call site.

### Atomic replacement

`concurrent_writers_produce_intact_payload` starts writers behind a barrier and
asserts that the destination contains one complete payload. Windows-only unit
coverage includes
`transient_windows_persist_errors_are_narrowly_classified`,
`non_transient_windows_persist_error_is_not_retried`,
`transient_windows_persist_retries_are_bounded`, and
`transient_windows_retries_reuse_the_written_temp_file`.
`no_stray_tmp_sibling_after_write` covers cleanup. The implementation retries
the already-written temporary file only for named transient replacement/share
errors; there is no copy, truncate, or other non-atomic fallback.

### Portable text and file URLs

The refreshed Phase 3 inventory records 137 strict-production matching lines,
138 occurrences, and 67 files; tests and fixtures contribute 125 lines and
125 occurrences in 50 files. The union is 262 lines, 263 occurrences, and
109 files. It classifies every remaining `.display()`, `to_string_lossy()`, and
`file://` hit by consumer role.

There are zero production hand-built `file://` URLs. All 13 remaining literal
occurrences are fixtures or assertions around `Url::from_file_path`. Visible
paths use the biscuit-file portable-text boundary, while filesystem inputs,
provider arguments, environment state, persistence keys, hashes, session
identity, and operational working directories remain native.

### Residual Windows failures

The Phase 4 classification notes assign every original residual failure to a
product defect, a portable fixture defect, or an explicitly named Unix-only
fixture facility. Highlights:

- home/config/discovery cluster: 74/74 corrected native-Windows tests passed;
- native Windows file-link creation, collision, and already-linked cases:
  3/3 passed;
- process cluster: 4 library, 8 CLI spawn/wait, and 2 integration cases passed;
- Windows Job Object teardown stress: 20/20 passed after the test-owned native
  handshake; production Job logic was unchanged; and
- nine MCP subprocess fixtures remain Unix-only because their hermetic setup
  specifically depends on redirecting `$HOME`. Their full assertions remain on
  Unix, while explicit-path catalog/provider-state storage is covered
  cross-platform.

The Darkmatter overlay regression required a native-identity/presentation split:
effective frontmatter remains native, while final body interpolation receives a
portable presentation value. This avoids CommonMark consuming Windows
backslashes without turning rendered text into path identity.

### Generator drift

Each of the three formerly timing-out drift tests was measured alone three
times before and after the fix. Their pre-change bodies were approximately
44 seconds. After repository and package-area discovery was captured once per
request, all nine post-change invocations passed; steady-state invocation wall
times were 3.588-3.983 seconds, and focused test bodies were below one second.
No slow-test prefix or timeout override was added. The complete generator gate
passed 154/154 with 1 skipped.

## July 29 acceptance mapping (9 items)

| Acceptance item | Evidence | State |
|---|---|---|
| Directory-scoped deny rules apply on Windows. | `windows_directory_deny_rule_blocks_child_path`; protect-service sensitive-path deny regression; final Windows L1. | Met on Windows |
| Directory-scoped allow rules apply on Windows. | `windows_directory_allow_rule_grants_child_path`; protect-service allow regression; final Windows L1. | Met on Windows |
| Home-relative sensitive paths are classified on Windows. | `windows_home_credential_paths_are_sensitive_on_every_host` covers `.ssh`, `.aws`, `.gnupg`, and `.claude`; final Windows L1. | Met on Windows |
| Absolute allow entries are recognized on Windows. | `windows_absolute_allow_is_portable_and_boundary_aware`; drive-relative rejection companion; final Windows L1. | Met on Windows |
| Boundary logic has exactly one guarded definition. | Private `path_semantics` module plus `path_matching_modules_keep_separator_semantics_centralized`; final Windows L1. | Met locally |
| Prefix-but-not-child paths remain rejected. | `directory_prefix_requires_a_segment_boundary` and `descendant_match_requires_a_segment_boundary`; final Windows L1. | Met on Windows |
| Host-independent matcher tests are ungated and run in the native Windows L1 suite. | Windows-shaped comparison cases are not hidden behind a platform gate. | Met |
| Interim warning and both call sites are removed. | Only the forbidden source-guard needle remains; no definition or caller remains. | Met locally |
| Native Windows MSVC constrained build and full L1 gates are clean. | `rustc -vV` reports host `x86_64-pc-windows-msvc`; the constrained build, sanity, lint, doctest, CI-profile L1, and canonical L1 gates passed. | Met |

The July 29 acceptance boxes are complete under the revised native-Windows
authority. The unavailable xwin and GNU checks remain documented follow-ups.

## July 31 success mapping (6 items)

| Success criterion | Evidence | State |
|---|---|---|
| Claudine L1 is green on native Windows; only genuinely Unix-only fixtures may be gated. | Final CI-profile and canonical runs passed 6,191/6,191 with 13 justified skips. The only retained gates name their Unix `$HOME` or Unix process facility. | Met on Windows |
| No test exceeds the ordinary 30-second ceiling. | Generator drift tests are below one second in the focused body; both final area runs had zero timeouts. | Met on Windows |
| July 29 matching defects and warning are closed. | Security regressions, source-inventory guard, and final native Windows MSVC gates above. | Met |
| Atomic writes survive concurrent Windows writers. | Barrier contention, bounded retry, same-temp-file, permanent-error, and cleanup regressions passed in final Windows L1. | Met on Windows |
| Path-to-text boundaries use biscuit-file portable text. | Phase 3 inventory; zero unresolved presentation sites and zero production hand-built file URLs; final Windows L1. | Met on Windows |
| Host-independent contracts remain ungated and native Windows MSVC gates pass; external platform checks are follow-ups. | Matcher and renderer contracts are ungated; the corrected mounted-tree Linux build passed before WSL storage/subsystem failure; exact-candidate macOS, xwin, and GNU checks are deferred. | Met under revised contract |

## Integrated-plan completion mapping (7 items)

| Completion contract | Evidence | State |
|---|---|---|
| Separator-neutral allow/deny matching without prefix collision. | Shared comparison grammar and matcher/query tests passed in final Windows L1. | Met on Windows |
| Home-sensitive and Windows absolute-allow matching; warning removed. | Protect/query regressions, warning inventory, and final native Windows MSVC gates. | Met |
| Atomic intact last-successful-rename-wins behavior without non-atomic fallback. | Barrier contention and retry-state tests passed in final Windows L1; documented implementation contract. | Met on Windows |
| Portable user-facing paths/completions and real local file URLs. | Phase 3 inventory and URL/decline regressions passed in final Windows L1. | Met on Windows |
| Every original Windows L1 failure fixed or tied to a named Unix-only contract. | Four Phase 4 classification notes; no Windows product-path test was hidden; final Windows L1 is green. | Met on Windows |
| Three generator drift checks remain within the ordinary timeout. | Nine isolated post-change passes and the final 154/154 generator gates. | Met on Windows |
| Full native Windows MSVC constrained build/test/doctest/lint acceptance. | Windows build, sanity, lint, doctest, CI-profile L1, canonical L1, and 5/5 focused live-daemon tests passed. The all-platform graph retains `rendezvous-daemon -> duckdb -> libduckdb-sys`, including for `x86_64-pc-windows-gnu`; Linux/macOS/xwin/GNU execution remains deferred. | Met |

## Final audit record

- Warning and separator audit: the warning implementation/call sites are gone;
  the sole warning-name literal is the guard needle. Separator-boundary
  semantics are centralized and guarded.
- Path-to-text audit: every remaining production conversion is classified in
  `phase-3-path-text-inventory.md`; no production hand-built file URL remains.
- Identity audit: no path identity, process argument, environment value,
  filesystem input, hash input, or session key is routed through portable
  rendering.
- Docs/comments: the protect topic and Claudine skill mirror describe portable,
  boundary-aware rule separators. Atomic-write documentation records bounded
  retry and atomicity. `claudine/docs/dependencies.md` records the unexpected
  `dunce` dev dependency used for native canonical identity in tests.
- Comment drift was detected in sequence task tests and resolved by treating
  the code as authoritative: timestamps are available only for the Unix
  recording path, and the `cmd.exe` channel-contract twin is executed by the
  native Windows tests rather than merely compile-checked.
- Dependency drift was corrected by removing Windows-GNU exclusions for
  `rendezvous-daemon` and its tests. DuckDB remains an all-platform requirement;
  native Windows live-daemon tests pass 5/5, and the Windows-GNU Cargo graph
  includes `rendezvous-daemon -> duckdb -> libduckdb-sys`. No GNU compilation
  result is claimed.
- `git diff --check` and each scoped cached-diff check were clean before the
  implementation, Messenger, and Darkmatter documentation commits.
- GitNexus could not resolve the changed Rust symbols and returned `UNKNOWN`,
  never a trustworthy LOW result. Direct `rg` inventory was therefore used:
  the original path matcher had one production query caller; `atomic_write`
  had at least 24 production call sites; `system_shell_command` had one
  production caller; `debug_assert_child_env` had three spawn-path callers;
  and generator `inputs::load` had two production callers. The complete
  path-to-text consumer inventory is preserved in the Phase 3 note.
- Staged `detect_changes` was run for each committed scope and for this evidence
  note. It reported the changed files but zero resolvable symbols or processes;
  the upstream impact refresh likewise remained `UNKNOWN`. Scoped diff review
  and `git diff --cached --check` were clean. Rust-symbol unavailability is
  recorded here rather than converted into a false safety rating.

## Closeout decision

The revised native Windows MSVC completion contract is met, and the source-spec
acceptance state may reflect that result. Linux, macOS, xwin, and GNU checks
remain explicit deferred follow-ups and must not be described as passing.
After the final staged audits, both fix records and the integrated plan were
archived under `fixes/_completed`.
