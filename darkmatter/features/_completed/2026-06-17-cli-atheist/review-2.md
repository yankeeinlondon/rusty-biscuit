---
ready: true
agent: codex
model: ""
resolved: 2026-06-17
---

# Review 2

## Findings

### High: Most Level 2 tests can pass against a stale installed `md`

The split Level 2 harness defines `MD_BIN = env!("CARGO_BIN_EXE_md")` and documents that using it prevents real-terminal tests from passing against an installed host binary. However, the default helpers still run bare `md` through the terminal pane's `PATH`:

- `darkmatter/cli/tests/common/level2.rs:30`
- `darkmatter/cli/tests/common/level2.rs:112`
- `darkmatter/cli/tests/common/level2.rs:127`
- `darkmatter/cli/tests/common/level2.rs:132`

Only `run_md_built` uses the shim, and only `level2_disclosure_blocks.rs` imports that helper. The rest of the Level 2 suites call `run_md` / `run_md_env`, including layout dimensions, code-block styling, tables, images, horizontal rules, and ordered lists. Those tests still verify real-terminal rendering behavior, but not necessarily the implementation under review.

Verification level: Level 2 is the correct level for these user-observable terminal rendering requirements, but the current Level 2 evidence is invalid unless the developer's pane `PATH` happens to resolve `md` to the just-built binary. This is a high-severity test-rigor gap because it can hide regressions in exactly the behavior the Level 2 suite is meant to prove.

Fix: make `run_md` and `run_md_env` use `md_shim()` by default, then delete or rename the built-binary special case. Keep any command-echo workarounds inside the shim path. Add a small assertion/helper test that the command invoked in the pane is the Cargo-built binary, not a host install.

**Resolution (2026-06-17):** Resolved. `darkmatter/cli/tests/common/level2.rs`
now routes every default Level 2 helper through `md_shim()`:

- `run_md`, `run_md_env`, and `run_md_after_shell_prefix` invoke
  `md_shim()` instead of the bare host `PATH` `md`. The bare-`md` code
  path is gone.
- `run_md_built` is deleted (it was the special case that used the
  shim); `level2_disclosure_blocks.rs` now calls `run_md` like every
  other Level 2 file.
- `md_shim()` calls `assert_shim_resolves_to_built` on first invocation
  inside the `OnceLock`. The assertion canonicalizes both the symlink
  target and `CARGO_BIN_EXE_md` and aborts the test binary if they
  disagree, so the suite cannot silently run a stale host `md`.
- A new `darkmatter/cli/tests/level2_harness_integrity.rs` binary
  pins the integrity contract with four tests: the shim resolves to the
  Cargo-built binary, `assert_shim_resolves_to_built` accepts a valid
  symlink and rejects a foreign symlink, and the shim path is an
  absolute temp-dir link (never a bare `md` that would re-resolve via
  `PATH`).

### High: `md validate refs` still hand-rolls report JSON and text instead of using the extracted library surfaces

The spec requires the CLI JSON paths to serialize the library values directly after adding serde support, and it requires validation-report terminal rendering to move to a `TerminalRenderable` library view. The library now has both pieces:

- `ReferenceValidationReport` implements `Serialize` in `darkmatter/lib/src/markdown/reference/validate.rs:75`
- `ValidationReportView` implements `TerminalRenderable` in `darkmatter/lib/src/markdown/reference/validate.rs:109`

But `md validate refs` still has CLI-local formatting logic:

- `print_validation_report_text` manually prints the text report in `darkmatter/cli/src/commands/validate.rs:72`
- `print_validation_report_json` manually builds a reduced JSON object in `darkmatter/cli/src/commands/validate.rs:117`

This leaves two public shapes for the same report. The library serde shape emits `valid`, `kind`, `reference`, and `source` fields; the CLI path emits `is_valid` and omits several issue fields. The baseline tests pin the current CLI shape, but their comments say they are exercising the "serde-backed JSON path", which is not true. The implementation has preserved behavior, but it has not completed the designed extraction and leaves the drift risk the spec was trying to remove.

Verification level: Level 1 is appropriate for JSON shape compatibility and the new baseline tests cover the legacy CLI shape. What is missing is Level 1 coverage that the CLI output is produced by, or at least stays equivalent to, the library serialization/view contract.

Fix: either route `md validate refs --format json` through the library report serialization with compatibility-preserving serde attributes/manual impls, or explicitly document that `md validate refs` intentionally keeps a legacy CLI JSON shape separate from `ReferenceValidationReport`'s library serde shape. For text output, use `ValidationReportView` or document why this command intentionally remains plain text while `md graph --validate` uses the renderable view.

**Resolution (2026-06-17):** Resolved.
`darkmatter/cli/src/commands/validate.rs` now uses the library surfaces:

- `print_validation_report_json` is deleted. `md validate refs --format
  json` now serializes the library `ReferenceValidationReport` directly
  via `serde_json::to_string_pretty(&report)`. `md validate refs --json`
  and `md graph --validate --json` therefore share a single serde
  contract; the prior `is_valid` / `reference_id` CLI shape is retired.
- The baseline fixtures under
  `darkmatter/features/2026-06-17-cli-atheist/baseline/json/validate_refs_*.json`
  are regenerated to the library serde shape (`valid`, per-issue
  `kind` / `reference` / `source`, no path-dependent `reference_id`).
  The `common::baseline` normalizer handles the remaining temp-path
  redaction so the fixtures stay stable.
- `print_validation_report_text` stays as the *primary* per-issue
  report and now carries a documentation comment explaining the
  rationale: it lists every issue (errors, warnings, info), prints the
  count header, and stays readable in CI logs that strip ANSI.
  `ValidationReportView` is a styled error-only *summary* used as a
  footer by `md graph --validate`; routing `--format text` through it
  would silently drop non-error issues and the success case.

## Requirement Coverage

- Args, commands, render/artifact, `tests/cli.rs`, and `tests/level2_layout.rs` are decomposed into the intended module/test layout.
- Review 1's JSON and delta compatibility gaps are materially improved with Level 1 baseline/golden tests.
- Level 2 coverage now reliably tests the built binary — every default helper routes through `md_shim()` and `md_shim()` self-checks that the symlink resolves to `CARGO_BIN_EXE_md`.
- `md validate refs --json` and `md graph --json` now share a single library serde contract.
- No Level 3 coverage is required by this feature; it does not define keyboard, paste, mouse, or terminal input-encoder behavior.

## Verification Run

- `cargo test -p darkmatter-cli --tests` passed (577 tests across all binaries, including the four new `level2_harness_integrity` tests).
- `cargo test -p darkmatter-cli --test validate_refs --test graph --test delta --test level2_harness_integrity` passed.
- `cargo clippy -p darkmatter-cli --tests --no-deps` clean.
