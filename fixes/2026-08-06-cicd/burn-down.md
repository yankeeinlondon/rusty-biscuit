# Native-environment burn-down — the low-hanging fruit list

Source: full-scope run 31184682085 (attempt 2, commit `69d704c69`), the last
run with complete native-environment evidence. wsl2 evidence is arriving via
the seven targeted `_wsl-ci` dispatches and is NOT in this document.

Counts: **macOS 8 · ubuntu 14 · windows 58** failing tests — but they cluster
into far fewer causes. Fix causes, not tests.

## Not fruit — CI-side, already owned (do not chase as test bugs)

- **6 L2 cells FAIL with zero failing tests** (`biscuit-terminal`,
  `darkmatter`, `dmls` × ubuntu/macos): the new backend-proof guard fails the
  job — "no evidence at …/backend-executions.jsonl" while the suite starts ~2
  tests and skips ~2996. CI L2 wiring issue (evidence-file path / shared-pane
  provisioning), not test regressions. Owner: the CI branch work.
- **All wsl2 cells in attempt 2/3** — died pre-test (`$NATIVE` unbound);
  fixed on the branch; dispatches under way.

## One fixture, 52 failures (25 windows + 27 wsl2)

`biscuit-terminal/lib/tests/common/pty.rs:46-52` — `discovery_probe_path()`
joins `"discovery_probe"` **without `std::env::consts::EXE_SUFFIX`**:

- **Windows (25 tests)**: the example IS built (`discovery_probe.exe`), the
  extension-less `exists()` check fails, every PTY-fixture test panics at
  `pty.rs:75`. One-line fix. (After it, these tests meet PTY-on-Windows
  reality for the first time — treat whatever appears next as new
  information, the fixture bug was masking it.)
- **wsl2 (27 tests)**: `cargo nextest archive` does not carry examples; the
  guest has no cargo to build one. Fix: include the example in the archive
  (`[profile.ci.archive] include` + build it in the archive step), matching
  the guest's `target/<triple>/debug/examples/` resolution.

Affected suites (the "OSC-looking" cluster): `level1_osc_queries`,
`level1_clipboard`, `level1_cursor`, `level1_mode_2027`,
`level1_apple_terminal_prose`, `level1_terminal_init`,
`level1_terminal_osc_cache`.

## Cross-OS failures (same test, all three OSes)

| Test | Note |
|---|---|
| `claudine-gen::generate_ux::clean_check_report_summary_matches_phase_1_snapshot` | Known: the archived review moved files the gen baseline test reads (memory: claudine-gen baseline broken on HEAD) |
| `sniff::filesystem::repo::standard::tests::resolve_acting_binary_returns_none_when_binary_missing` | Fails identically on ubuntu/windows/macos — suspect the runner environment satisfies a lookup the test expects to miss |

## macOS (8) — beyond the cross-OS two

- `sniff::hardware::gpu::tests::test_detect_gpus_on_macos` — GPU detection on
  a virtualized runner; likely env-dependent assertion.
- `claudine-cli` L2 `level2_context_capture` ×3 (also ×4 on ubuntu) — the
  140-cap fill family; capture-under-tmux styling.
- `biscuit-terminal-cli` L2 `level2_apple_terminal_double_underline_plain_text_visible`
  — Apple Terminal harness on a headless runner.
- `darkmatter-cli` L2 `level2_code_block_clears_inherited_dim_before_theme_colors`
  — known baseline family.

## ubuntu (14) — beyond cross-OS and the L2 families

- `claudine-cli` L1 ×6: `sequence_perf` ×2, `wrap_perf` ×2,
  `prompt_reporting::compose_frontmatter_verbose_shows_full_system_prompt`,
  `shipped_prompts::shipped_implement_prompt_runs_real_router_target` — the
  known ubuntu L1 baseline family (timing/perf-sensitive under runner load).
- `biscuit-terminal-cli` L2 `level2_diagram_fallback_when_no_image_protocol`.
- `darkmatter-cli` L2 `level2_schema_about_light_terminal_uses_dark_code_theme`.

## windows (58) — beyond the 25 fixture + cross-OS

| Cluster | Tests | Suspected cause |
|---|---|---|
| `rendezvous-daemon` ×10, `rendezvous-client` ×1 | named-pipe DACL/acceptor/server/phase6 | First-ever CI exposure of the Windows transport half (these packages were never in the old grid); real functional work |
| `biscuit-terminal` filesystem ×3, discovery resolver ×2 | path-safety + XDG override tests | Windows path semantics |
| `biscuit-terminal-cli` ×4 | about/integration (PTY + config paths) | Windows paths / PTY |
| `research` ×2 | symlink creation verification | Windows symlink privilege on runners (needs privilege or junction fallback) |
| `unchained-ai` ×3 | `pty_runner` echo/ansi/interactive | PTY (ConPTY) behavior on runners |
| `sniff-cli` ×4 | 2 insta snapshots + 2 worktree-verbose | Windows path forms in output (`runneradmin` home, backslashes) |
| `sniff-cli`/`sniff` lint ×2 (cells) | lint FAIL with 0 test fails | clippy `-D warnings` on ubuntu lint leg — read the lint log, not tests |
| `biscuit-tui-cli` ×1 | `bash_script_passes_syntax_check` | bash discovery on Windows runner |
| `darkmatter-cli` ×1 | `schema_validate_legacy_pretty_output_is_byte_identical` | 8.3 short path (`RUNNER~1`) in emitted file:// link — Ken has the fix |
| `reaper`/`visualizer` check cells | compile-check FAIL, 0 tests | `cargo check --all-targets` on windows — read the check log |

## Sequencing note

The 4.5-hour full-scope run happens once, after this list is burned down —
per Ken 2026-08-07. Until then: fix locally per OS, verify with targeted
means (stripped-PATH binary runs, Docker, `_wsl-ci` dispatches, single-leg
`workflow_dispatch`), and let pushes cancel any incidental full runs.
