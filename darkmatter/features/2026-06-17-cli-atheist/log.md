# CLI Atheist — Decision Log

## ADR-1: `CliStyleClaims` lives on `darkmatter::style`

**Status:** Ratified in Phase 4.

**Decision:** `CliStyleClaims` lives on `darkmatter::style` because that is where the `apply_*_style` functions already live. It is a neutral data model expressed in library/layout types (`PageComponent`, `Layout`, `PaintColor`, `PageBackground`, `Alignment`, `Length`, `ThemePair`) — no clap, no CLI-only wrapper types. The CLI builder (`darkmatter/cli/src/style_claims.rs`) is the only code that knows about `Cli`, `CliFill`, and clap aliases.

**Consequences:**
- Precedence and override logic is defined once in the library and reused by the CLI.
- The library can be unit-tested in isolation without pulling in clap or the CLI arg surface.
- Future frontmatter/CLI consumers can build `CliStyleClaims` directly from other input sources.

## ADR-7: Shared Level 2 harness lives in `tests/common/level2.rs`

**Status:** Ratified in Phase 7.

**Decision:** The shared Level 2 WezTerm harness, just-built `md` shim,
skip/enforce policy, sentinel polling, and fixture-running helpers live in
`darkmatter/cli/tests/common/level2.rs`. Each top-level `level2_*` test file
imports that harness through `mod common;` and keeps only concern-specific
assertions.

**Consequences:**
- No duplicated WezTerm setup across the split Level 2 layout suites.
- Skip/enforce behavior stays consistent for all layout-related real-terminal tests.
- Tests are less likely to accidentally exercise a host-installed `md` instead of the just-built binary.
- `tests/common` is more substantial, so helpers must stay clearly namespaced under `common::level2`.

## Phase 8: Deferred bool parser cleanup

`parse_bool_str` / `parse_bool_env` duplication remains intentionally
out-of-scope for CLI Atheist. Their eventual move to `biscuit-terminal::env`
is a low-priority cleanup and not required for this structural split.

## Review 1 follow-up: byte-for-byte compatibility coverage

Review 1 called out two High findings where the Phase 1 leak extractions
claimed byte-for-byte behavior preservation but the test suite only
asserted parseability or substring matches.

- **JSON compatibility baselines** (`md validate refs --json`, `md graph
  --json`): added Level 1 baseline comparison tests in
  `darkmatter/cli/tests/validate_refs.rs` and
  `darkmatter/cli/tests/graph.rs` plus a shared `common::baseline` helper
  (`darkmatter/cli/tests/common/mod.rs`) that loads the captured fixtures
  from `baseline/json/` and normalizes environment-specific values (temp
  directory paths and the FNV-1a reference-id hash prefix derived from
  them). Coverage now spans local paths, remote URLs, fragments, data
  URIs, inline CSS/script/meta records, validation errors, and graph
  insertions / transclusions.
- **`DeltaReport` text renderer**: added Level 1 golden tests in
  `darkmatter/cli/tests/delta.rs` covering no-change, frontmatter-only,
  preamble-only, section added/removed/modified/moved, whitespace-only,
  code-block content modification, code-block language change, and
  verbose-mode output (statistics block + visual diff for both
  frontmatter-with-content and content-only scenarios).

## Review 1 follow-up: accepted over-cap exceptions

Review 1's Medium finding observed that `just lint-files` still reports
five files over the ~500-line soft cap. Per the review's "Fix" guidance
the list is now explicit rather than implicit:

- ADR-5 stands (no CI gate), but `just lint-files` now carries an inline
  accepted-exception list with a one-line reason for each entry and
  prints `(accepted: <reason>)` next to those entries. Any *new*
  over-cap file is still flagged for review.
- The spec gains an **Accepted Over-Cap Exceptions** section that
  records the rationale per file. None of the five files matches the
  god-file pattern (hundreds of unrelated top-level symbols); each is a
  single-responsibility module whose line count is driven by coverage
  of that responsibility. Splitting them further would create
  artificial fragmentation rather than reduce coupling.

## Review 2 follow-up: Level 2 binary integrity

Review 2's first High finding observed that the Level 2 harness helpers
`run_md` / `run_md_env` / `run_md_after_shell_prefix` invoked bare `md`
through the pane's `PATH`, so the real-terminal suite could silently
pass against a stale host-installed `md` while the code under review
failed. Only `run_md_built` used the Cargo-built shim, and only the
disclosure-blocks test used `run_md_built`.

Resolution:

- `run_md`, `run_md_env`, and `run_md_after_shell_prefix` now route
  through `md_shim()` by default. The bare `md` form is gone.
- `run_md_built` is deleted; `level2_disclosure_blocks.rs` calls
  `run_md` like every other Level 2 file.
- `md_shim` now calls `assert_shim_resolves_to_built` on first
  invocation. The assertion fails the test binary fast if the symlink
  does not resolve to the `CARGO_BIN_EXE_md` path baked in at compile
  time.
- A new `tests/level2_harness_integrity.rs` binary exercises the
  integrity contract: it verifies the shim resolves to the built
  binary, that `assert_shim_resolves_to_built` accepts a valid link,
  rejects a foreign link, and that the shim path is an absolute temp
  dir link (never a bare `md`).

## Review 2 follow-up: `md validate refs` JSON consolidation

Review 2's second High finding observed that `md validate refs` still
hand-rolled its report JSON (`print_validation_report_json`) and text
output (`print_validation_report_text`) instead of using the extracted
library surfaces. The CLI shape used `is_valid` and `reference_id`
while the library serde shape used `valid`, `kind`, `reference`, and
`source`, so the two CLI surfaces (`md validate refs --json` and
`md graph --validate --json`) could drift silently.

Resolution:

- `md validate refs --format json` now serializes the library
  `ReferenceValidationReport` directly via `serde_json::to_string_pretty`.
  `print_validation_report_json` is deleted. The CLI and library now
  share a single serde contract pinned by the updated baseline
  fixtures under `baseline/json/validate_refs_*.json`.
- The baseline fixtures are regenerated to the library serde shape
  (`valid` instead of `is_valid`, full per-issue records with `kind`,
  `reference`, and `source` fields, no path-dependent `reference_id`).
  The existing `common::baseline` normalizer handles the temp-path
  redaction so the fixtures stay stable.
- `print_validation_report_text` stays as-is with a documentation
  comment explaining why: it is the *primary* per-issue report
  (warnings, info-severity issues, count header, success case), whereas
  `ValidationReportView` is a styled error-only summary used as a
  footer by `md graph --validate`. Routing `--format text` through the
  view would silently drop non-error issues and the success case.

## Review 3 follow-up: Level 2 gate ordering and Windows-safe shim creation

Review 3's High finding observed that `run_md_env` resolved `md_shim()`
before entering `run_md_env_bin`, where `wezterm_decision()` performs
the Level 2 skip/enforce decision. On a host without usable WezTerm
the suite still tried to create and canonicalize the shim before
skipping cleanly. On Windows this is not just wasteful —
`std::os::windows::fs::symlink_file` can require elevated privileges
or Developer Mode, so `just test-l2` could fail during harness setup
even when the real-terminal tier should skip. The new
`level2_harness_integrity` tests also executed `md_shim()` in the
Level 1/sanity filter (their test names do not start with `level2_`,
so the nextest `test(/level2_/)` filter does not match them).

Resolution:

- `run_md_env_bin` now takes `FnOnce() -> &'static str` for the binary
  path. The Level 2 gate runs before the closure is called, so
  `md_shim()`'s filesystem work happens only when the gate has passed.
  `run_md_env` passes `md_shim` (as a function reference) so the
  closure resolves lazily inside the gated helper. A host that skips
  the Level 2 tier never touches the filesystem for shim creation.
- `md_shim` now creates the shim via a new `link_or_copy` helper with
  a graceful fallback ladder: symlink → hard link → copy. The Windows
  shim path uses the `md.exe` extension so the pane can resolve it as
  an executable. Hard links work without extra privileges on the same
  volume; copies handle cross-volume temp directories. The fallback
  ladder is the same one already used implicitly by `tempfile` and
  `cargo` for other cross-platform file shimming.
- `assert_shim_resolves_to_built` is rewritten on top of a new
  `is_same_binary` helper that uses file identity (inode on Unix,
  volume serial + file index on Windows) as a fast path and falls back
  to byte-for-byte content comparison. The previous `canonicalize`
  check only worked for symlinks; `is_same_binary` also accepts hard
  links and copies, which the new fallback ladder can produce.
- The `level2_harness_integrity` binary now uses `link_or_copy` and
  `is_same_binary` instead of direct `symlink_file` calls. The
  structural tests no longer require symlink privileges to run, so
  they stay valid in the Level 1/sanity filter on Windows hosts that
  lack Developer Mode.
- `darkmatter/cli/tests/common/level2.rs` grew past the ~500-line
  soft cap after adding the new helpers. It is added to the
  `ACCEPTED_OVERCAP` list in `darkmatter/justfile` and to the spec's
  "Accepted Over-Cap Exceptions" table with a one-line rationale:
  the file is one cohesive Level 2 test-harness module, and splitting
  it would fragment that concern rather than reduce coupling.
