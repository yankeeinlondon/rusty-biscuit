# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Breaking

- `question choose-one` and `question choose-many` now exit with code
  `1` when the user presses `Esc`. The previous behaviour lumped `Esc`
  onto the same exit code as `Ctrl+C` (`130`). Scripts that branched
  on `130` to detect an aborted selection must now distinguish
  `1` (user aborted) from `130` (SIGINT / Ctrl+C). This change aligns
  the CLI's exit status with the tech-design §5.1 contract. All other
  subcommands (`text-input`, `text-area-input`, `boolean-switch`,
  `input-table`) adopt the same split for consistency.

### Changed

- `question choose-many --selected` no longer splits comma-separated
  values. Each occurrence of the flag is now passed through verbatim
  (empty strings are still filtered), so values that contain literal
  commas — e.g. `--selected "one,two"` — are preserved as a single
  pre-selection. To pre-select multiple values, repeat the flag
  (`--selected a --selected b`). Callers that relied on the old CSV
  expansion can still use the deprecated `--initial` flag, which
  continues to split on commas for backward compatibility.

### Added

- `choose-one` / `choose-many`: read option strings from STDIN,
  positional args, or a mix of the legacy `--options*` flags.
- `--delimiter <CHAR>` splits each option into `label:value` pairs.
- `--selected <VALUE>` pre-selects an option by value. Deprecates
  `--initial` (still accepted, hidden from help, warned on stderr).
- `--sort <natural|reverse|asc|desc>` orders the option list before
  rendering.
- Inline fuzzy search: typing alphanumerics opens the search prompt
  and filters the list live. Use `--no-filter` to disable filtering
  (alphanumeric keys are then ignored).
- `Ctrl+A` selects every enabled option in `choose-many`; `Ctrl+D`
  clears the selection.
- Fallback submit: pressing Enter with no explicit selection promotes
  the currently hovered option (skipping disabled items).
- Border chrome: `--border`, `--border-label`, `--border-style`.
- Margin chrome: `--margin` + per-side `--mt / --mb / --ml / --mr`.
- `--height <N | NN%>` accepts both cell counts and percentages, with
  a floor of 3 rows.
