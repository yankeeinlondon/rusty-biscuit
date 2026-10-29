---
ready: false
agent: codex
model: ""
---

# Review 3

## Findings

### High - Root canonical validation hangs on the full curated area list

Spec D6 requires CI to validate the canonical recipe set across every curated package area. The new PR workflow now calls `just check-canonical` (`.github/workflows/test.yml`), and the root recipe iterates the `areas` list and shells into each area with `just _check_canonical` (`justfile:316`, `justfile:333`, `justfile:340`). In practice, the full root command does not complete: `timeout 65s just check-canonical` exited with code `124` after printing success through `queue` and then stopping at `Checking homelab...`. The same targeted check, `timeout 15s just check-canonical homelab`, passed, so the regression is in the full-root validation path rather than the homelab justfile being obviously missing recipes.

Requirement verification level: Level 1/static plus command execution is appropriate for this workflow requirement. Current strongest verification fails: the exact command added to the PR gate hangs until killed, so the `test.yml` job can time out before it ever reaches `just all`.

Recommended fix: make `check-canonical` bounded and non-recursive enough to be reliable. A simple approach is to avoid invoking each package's own `just _check_canonical` and instead parse each `area/justfile` directly from the root validator, or add a short per-area timeout with diagnostics so one stuck justfile cannot hang the entire CI gate. After the fix, `timeout 65s just check-canonical` should complete successfully on the full curated list.

### High - Bare-Ctrl badge UX still lacks active Level 3 verification

The review instructions require Level 3 verification for user-observable requirements of the form "when the user holds/presses key X, Y happens" because only OS keyboard injection exercises the terminal's actual input encoder. The `biscuit-tui` bare-Ctrl badge behavior is still actively verified only by `level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`, which injects manufactured kitty bytes with `wezterm cli send-text` (`biscuit-tui/cli/tests/real_terminal_render.rs:288`, `biscuit-tui/cli/tests/real_terminal_render.rs:309`). The actual Level 3 test for "bare Ctrl held in real WezTerm reveals badges" is present but explicitly ignored (`biscuit-tui/cli/tests/real_terminal_render.rs:486`). The comments even state that the canonical verification is the Level 2 raw-bytes test (`biscuit-tui/cli/tests/real_terminal_render.rs:472`), which is exactly the verification-level mismatch this review rubric forbids.

Requirement verification level: Level 3 is required for the bare-modifier UX. Current strongest active verification is Level 2. The Level 3 arrow-down and Ctrl+R chord tests are useful OS-injection coverage, but they do not prove that WezTerm emits the expected bare-modifier bytes when the user presses Ctrl by itself.

Recommended fix: either implement a flagsChanged-capable macOS injector and unignore the bare-Ctrl Level 3 test, or narrow the production claim/spec so bare-modifier press visibility is explicitly not considered production-ready. Do not classify the bare-Ctrl badge UX as ready based on the Level 2 raw-byte test alone.

## Test Rigor Notes

- The prior `biscuit-tui` shared enforcement gap is addressed: the L2/L3 tests now call `test_toolkit::require_level!`.
- Canonical recipe presence is a Level 1 workflow requirement, but it must be validated by a root command that actually terminates on the full curated list.
- Terminal rendering/styling assertions such as badge SGR checks have appropriate Level 2 coverage through tmux capture. Bare keypress encoder behavior remains a Level 3 gap.

## Verification Performed

- Read `features/2026-05-24-testing-best-practices/spec.md`, `plan.md`, `review-1.md`, and `review-2.md`.
- Inspected `tools/test-toolkit`, `biscuit-browser-harness`, `biscuit-tui` real-terminal tests, root/package justfiles, and the CI workflows.
- Ran `cargo test --color=never -p test-toolkit -p biscuit-browser-harness --lib --no-run` successfully.
- Ran `timeout 15s just check-canonical homelab` successfully.
- Ran `timeout 65s just check-canonical`; it timed out at `Checking homelab...` with exit code `124`.
- Confirmed no tracked generated fuzz corpus/artifact/target files remain under the `biscuit-file` or `darkmatter` fuzz directories.

## Production Readiness

Not ready for production. The iteration closes the previous shared-level-helper and fuzz-issue gaps, but the root CI validator currently hangs on the full curated area list, and a key user-observable keyboard behavior is still being treated as covered by Level 2 raw-byte injection instead of active Level 3 OS keyboard injection.
