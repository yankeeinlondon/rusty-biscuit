---
created: 2026-09-10
status: implemented
spec: ./spec.md
---

# Implementation Plan

The fix will resolve explicitly supplied partial file inputs before lifecycle
initialization while leaving missing-value collection and the full schema
verdict at their existing lifecycle boundaries. Success is a real terminal
run that confirms/selects a file, reads it in the initialize guard, and hands
the same identity to the proxy target without asking for unrelated inputs.

## 1. Establish boundaries and impact

- Refresh GitNexus and run upstream impact on existing symbols before editing.
- Inspect the schema classification, CLI chooser, caller provenance, and
  coordinator entry paths. Preserve the initialized-document schema deferral.
- Reuse `FileReference` and the caller's captured resolution context; reuse
  existing candidate filtering and terminal widgets.

## 2. Implement supplied-input completion

- Introduce a narrow supplied-file classification pass that can identify
  eligible unresolved inputs independently of other schema problems.
- Process every eligible supplied input before its first lifecycle consumer,
  preserving file-array shape and unrelated values.
- Pass an explicit launch-anchored candidate scope to the shared chooser.
- Record accepted values in both overrides and caller provenance, so proxy and
  fresh preparation consume the selected identity without another prompt.
- Preserve literal-first resolution, interactive gates, typed failure behavior,
  lazy-file semantics, and initialize bootstrap/skip behavior.
- Audit direct, inline, proxy, sequence, and retry/resume paths; modify only
  paths that need the supplied-input phase or retained selection.

## 3. Add regression evidence

- L1: classification independent of missing fields and unrelated invalid
  values; multiple supplied inputs; arrays; literal paths; interaction gates;
  launch origin and selected-value propagation.
- CLI L1: the reported router shape fails with a file diagnostic in non-TTY
  mode before its guard or provider executes.
- L2: a shipped-review-router fixture accepts one candidate under YOLO,
  chooses among multiple candidates, and cancels cleanly. Assert the selected
  identity reaches the guard/proxy and no missing plan/review prompt appears.
- Use hermetic fake providers and the shared headless terminal harness. Keep
  all windows unfocused. Demonstrate the regression against the old behavior
  where practical.

## 4. Review, document, and validate

- Review the combined diff for lifecycle ordering, caller ownership, array
  handling, diagnostics, and cross-platform paths.
- Correct drifted comments; update composition docs, the claudine skill
  snapshot, and README guidance where needed.
- Run focused tests first, then `just test`, `just test-l2`, and `just lint`
  for the affected area. Coordinate builds to avoid concurrent Cargo locks.
- Run affected tests on declared remote build hosts with `just cross-check`;
  record unsupported/unavailable evidence explicitly. This session declares
  `BUILD_LINUX`; native Windows and WSL hosts are not declared.
- Record commands, outcomes, and remaining limitations here. Mark completion
  and move the fix to `_completed` only when implementation and required
  verification are satisfied. Do not commit or run `cargo fmt`.

## Coordination

The implementation agent owns production Rust and helper-level L1 coverage.
The regression agent owns CLI integration and L2 test files. The orchestrator
owns this plan, architecture review, documentation, build/test execution, and
final integration. Agents report impact before edits and coordinate any shared
interfaces; they do not independently start broad builds.

## Validation Record

- Baseline: `just test-cli shipped_review_router` ran two new tests against
  unchanged production code. The partial-reference test failed with the
  reported lifecycle evaluation error; the literal-path routing control
  passed. This establishes that the regression assertion distinguishes the
  broken path. Log: `/tmp/claudine-completion-baseline.log` (session-local).
- Impact queries were invoked before production edits. The index returned
  UNKNOWN/lower-bound results with missing call edges; direct source review
  identified compose/inline preparation, the shared chooser, and sequence's
  contained proxy coordinator as affected. A full index rebuild was started.
- Supplied completion applies to eager file inputs. Lazy output-file references
  remain valid without an existing file. The old documentation example omitted
  `eager`; the implementation and documentation preserve the lazy contract.
- Production implementation, regression coverage, and documentation updates
  are complete. Native Windows and WSL2 verification remain pending.
- The first focused run (`just test supplied_ shipped_review_router`) passed
  22 tests but still failed the shipped router. Its YAML schema list projects
  to a root-level union, which the initial single-mapping classifier skipped.
  Classification was extended to a uniquely applicable alternative, with
  explicit coverage for ambiguous alternatives.
- After adding conservative root-union selection, the focused command passed
  all 25 selected tests, including the previously red shipped-router case.
  Selection uses Darkmatter validation of each alternative with caller-file
  existence deferred; zero or multiple applicable alternatives remain deferred
  without an early verdict.
- The refreshed impact graph reports `prepare_and_run_active_document` HIGH
  (two direct callers, eight affected symbols); the orchestrator warned before
  further edits. Other edited entry points report LOW. The MCP staleness
  envelope still claims two commits behind despite the successful rebuild.
- Linux connectivity: the declared build host did not complete a bounded
  25-second SSH probe. A local Docker run supplied focused Linux evidence
  instead; native Windows and WSL2 execution are pending declared hosts or CI.
- L2 passed: `just test-l2 provided_partial review_router
  proxy_target_schema initialize_precedes_schema` ran 14 CLI tests, all passing.
  This includes eight partial-file tests and six existing initialization-order
  tests. The generator phase selected no tests, as expected for this filter.
  All eight partial-file tests here are pseudo-terminal tests selected by the
  `level2_` prefix; the shared-harness coverage the plan's phase 3 named was
  added later under "Real-terminal capture" below.
- The initial full `just test` run passed 3,026 tests before an existing
  sequence-preflight matrix test exceeded its 30-second timeout. Its unchanged
  path does not invoke the new completion code. Running
  `just test-library bracket_target_identity_is_rejected_on_every_graph_shell_surface`
  alone passed in 4.352 seconds. The complete rerun with
  `just test -j 8 --no-fail-fast` passed 6,858 tests, with 11 skipped, in
  91.431 seconds. The previously timed-out test passed in 9.379 seconds.
  No test deadlines or source were altered to address this load-sensitive
  timeout.
- `just lint` passed all area checks, including the transport/lifecycle guards
  and Clippy for all five Claudine packages.
- `git diff --check` passed. Both edited composition documents were stamped
  with `md hash --save` and pass `md hash --diff`.
- Linux/arm64: `just test-library --lib supplied_` passed all 13 selected
  library tests in an isolated Docker container using
  `rust:1.97.1-bookworm`. This is focused library evidence, not a full Linux
  CLI or terminal suite. JUnit staging was skipped because the isolated copy
  lacked workspace report metadata; the nextest result was successful.
  Log: `/tmp/claudine-completion-linux-test.log` (session-local).
- Native Windows and WSL2 behavioral evidence remain pending hosts or CI.
  The new TTY tests reuse the Unix PTY target, so native Windows terminal
  interaction still needs separate proof. Keep the implemented fix active
  until the remaining required OS verification is available.

### Pre-push validation fixes

The first push exposed the sequence-preflight timeout again and a terminal
capture failure. Both required fixture fixes; reducing concurrency was only a
diagnostic step, not the resolution.

- The shared sequence-preflight loader called the ambient compatibility
  resolver, discovering the real monorepo on every fixture load. All four
  source-loading sites in that module now use a fixture-owned
  `FileResolutionContext` through the explicit resolver. The original
  16-case matrix and its assertions remain intact. With the same focused
  command and default concurrency, its execution fell from 4.241 seconds to
  0.030 seconds; the 44-test module fell from 4.801 to 0.655 seconds.
  `just test-library` then passed all 4,116 tests at default concurrency,
  including the matrix in 0.044 seconds. No timeout or thread limit changed.
- The dry-run terminal capture drivers accepted old prompts and retained old
  tables in shared panes. They now clear the viewport and wait for a unique
  command-completion marker. A same-pane no-agent → not-installed regression
  checks that only the current Agent row is captured, including its styling.
  All eight focused dry-run tests and all 244 CLI L2 tests passed using the
  package area's default parallel terminal mode.
  `BISCUIT_L2_THREADS=1 just test-l2 level2_dry_run_` also passed all eight
  tests through the shared-pane broker used by the pre-push hook.

### Real-terminal capture

Ruling of 2026-09-10 (see the specification's Rulings): the shared terminal
harness coverage is required alongside the PTY suite, not instead of it.

- Added `cli/tests/level2_provided_partial_file_capture.rs`, one accept-path
  test per backend (tmux, WezTerm), driving the shipped router with the reported
  partial inside a real emulator. It asserts the confirmation and styled
  candidate card are drawn before any provider is reached or lifecycle error
  printed, that the launch-area candidate alone is named, and that accepting
  through the emulator's key path lands `SELECTED=alpha` and the chosen path at
  the provider stub with exit status 0. The review-router fixture moved to
  `cli/tests/common/review_router.rs` so both suites seed identical topology.
- Two host facts the test had to absorb, both recorded in its comments: a `?`
  typed into the pane's interactive shell is Atuin AI's trigger key on a host
  that loads it, so the exit marker uses `&&`/`||` instead of `"$?"` (this is
  also why the two dry-run WezTerm captures noted under host-environment in
  `log.md` wedge on this host); and the candidate path wraps at whatever column
  the emulator has, so the path is matched against the frame's lines joined.
- macOS: `just test-l2 review_router_partial_confirms_before_initialize` —
  2 passed (tmux 3.6 s, WezTerm 2.3 s); `BISCUIT_L2_THREADS=1` broker mode —
  2 passed (tmux 1.7 s, WezTerm 2.7 s). The PTY suite still passes after the
  fixture move: `just test-l2 provided_partial review_router
  proxy_target_schema` — 9 passed. `just lint` clean across all five packages.
- Mutation check: with the candidate scope in
  `schema_interactive/supplied.rs` anchored at `repository_root()` instead of
  `base_dir()`, both capture tests fail — the decoy widens the match and the
  chooser is drawn instead of the confirmation. The mutation was reverted and
  the file is byte-identical to HEAD.
- Linux: CI's claudine L2 leg runs on tmux; the WezTerm test skips there by
  the backend gate. WSL2: met, by running the same suites inside the guest in
  archive mode — see the end of the next section.

### Native Windows interactive coverage — met

The `#![cfg(windows)]` twin
(`cli/tests/level2_windows_provided_partial_file_capture.rs`) — same claim,
same shipped router, `cmd.exe` pane through the WezTerm harness, a compiled
`goose.exe` provider, `%M%`-indirected exit marker — **passes on the Windows
build host**: `level2_wezterm_windows_review_router_partial_confirms_before_initialize`
PASS in 4.247 s against a headless `wezterm-mux-server`, with
`BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` set so a skip could not have printed
as a pass. Getting there took three findings, each measured rather than argued:

1. **Retracted — the `--cwd` harness change.** The first matrix that showed
   `spawn --new-window --workspace <ws>` hanging without `--cwd` ran its
   no-`--cwd` variants first and its `--cwd` variants later; the difference was
   cold-versus-warm, not the flag. `push_workspace_args` no longer adds
   `--cwd`; the helper stays as the shared workspace routing.
2. **Fixed in the harness — the client evaluates the developer's config.** On
   `build-win`, `~/.wezterm.lua` `dofile`s the shared config from a UNC path
   that no SSH, nextest, or mux-server process can reach, and each evaluation
   blocks for the SMB connect timeout: 21.2–21.3 s on every `wezterm cli spawn`
   from a test (deterministic; a second spawn seconds later is 0.1 s while
   Windows negative-caches the failure; `cli list` never evaluates the config,
   which is why the availability gate passed). The client's config cannot change
   anything the harness does — it only ever addresses `WEZTERM_UNIX_SOCKET` —
   so `WezTermHarness` now runs every client under an empty
   `WEZTERM_CONFIG_FILE` unless the caller set one (`wezterm_command`,
   `biscuit-test-harness/src/wezterm.rs`). Spawn under that: 0.1 s cold.
3. **Fixed in production — the partial-file predicate compared native text.**
   With the spawn working, the run reached the resolution and reported
   `/spec: no existing file matched reference 'fixes/2026-09-10-local'`.
   `file_candidate_paths` yields the walker's native `\` paths;
   `path_matches_query` did a raw lowercase `contains` of the `/`-spelled
   partial, so every candidate missed on Windows and the typed failure fired
   where macOS offered the confirmation. Pre-existing and shared with the
   operation-file autocomplete (`claudine compose plan`), so not specific to
   this fix. Both sides are now compared in portable spelling
   (`to_portable_string`, `\` normalized in the query);
   `completion::scopes::tests::path_matches_query_ignores_separator_spelling`
   pins it, and its `\` arm fails against the pre-fix predicate on macOS
   (mutation checked) while its `/` arm is the one that failed on Windows.
   The Windows-reachable non-interactive tests had passed throughout: the
   zero-candidate path and the interaction-denied path produce the same
   diagnostic, which is why only a real-terminal run on Windows could see it.

Evidence, final state:

- Windows: L2 twin PASS 4.247 s (executed; required-backend gate);
  `path_matches_query_ignores_separator_spelling`,
  `shipped_review_router_non_tty_partial_fails_before_initialize`, and
  `shipped_review_router_literal_does_not_collect_absent_route_inputs` pass
  via `just cross-check --os windows`.
- macOS after the predicate change: 115 L1 tests across the completion,
  operation-file, provided-partial, supplied, shipped-router, and autocomplete
  families; 17 L2 tests across the PTY, capture, and operation-file suites;
  four WezTerm L2 tests after the harness change (the `level2_dry_run_…osc8…`
  failure is the pre-existing Atuin `?` wedge recorded above, reconfirmed by
  its frame); harness and area `just lint` clean.
- Two corrections to earlier versions of this record. It once claimed
  Windows has no long-lived mux server: wrong — the servers observed dying were
  killed by this session's own scripts, which also twice killed the session
  Ken was working in. And `just cross-check --os windows` reports a WezTerm L2
  test **passing when it is skipping** (no `WEZTERM_UNIX_SOCKET` in that SSH
  session; ~0.02 s) — read the duration, or set the required-backend variable.
- WSL2: **met.** The Unix suites run unchanged inside the guest in archive
  mode — the shape of CI's `wsl2-ubuntu` leg, with `terminal-tests` enabled
  and `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` so a skip could not print as a
  pass: 17 of 17 pass. Recipe-driven on 2026-09-11 —
  `just cross-check claudine-cli --os wsl --features terminal-tests
  provided_partial review_router proxy_target_schema path_matches_query
  shipped_review_router` — the tmux capture executed in 3.96 s, all nine PTY
  tests executed (0.1–0.6 s), and the predicate unit test and both
  shipped-router tests pass. The WezTerm capture line is a 0.013 s skip — no
  WezTerm in the guest — recorded as such. The first run, on 2026-09-10, used
  cross-check's exact sequence by hand because the guest's `~/.config` (a
  CIFS mount of the Synology NAS's `config` share) was down: `git` died on its
  global config and cargo's package-file listing (gitoxide, honoring the
  global excludes at `$XDG_CONFIG_HOME/git/ignore`) died with "Host is down";
  `GIT_CONFIG_GLOBAL=/dev/null` plus an empty `XDG_CONFIG_HOME` bypassed both,
  and the same 17 passed (tmux capture 22.3 s then — the guest's login shell
  waiting on the dead share, as the 3.96 s recipe run confirms). Native
  Windows was re-run the same morning with the NAS reachable: the L2 twin
  passes in 4.42 s, matching 4.25 s with it down, so the harness's
  empty-config isolation is what keeps the spawn fast, independent of NAS
  state. Native Windows and WSL2 CI legs remain the tracked provisioning gap;
  the behavioral evidence for both exists.
