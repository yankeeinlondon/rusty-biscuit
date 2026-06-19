---
ready: false
agent: codex
model: ""
---

# Icon Sets Formatting Review

The implementation addresses all five findings from review 1. The feature is
still not ready for production because some terminal-visible acceptance
criteria remain verified only with manufactured dimensions or are not asserted
at all in a real terminal.

## Findings

### 1. High: Level 2 tests bypass live terminal-size detection

Both new Level 2 tests set `BISCUIT_TERM_WIDTH` and
`BISCUIT_TERM_HEIGHT`, so the binary does not use the tmux pane's real
dimensions. They prove that output generated for synthetic dimensions survives
tmux rendering, but they do not prove the user-facing adaptive behavior:
selecting one or two tables from the actual terminal width and height.

Evidence:

- `cli/tests/level2_terminal.rs:337-356` always injects fixed dimensions.
- `cli/src/commands.rs:218-229` uses those overrides instead of
  `Terminal::new()`.
- Acceptance criterion 3 requires layout selection based on terminal width and
  height.

Add a Level 2 test that resizes the tmux pane, runs `icon sets` without the
dimension overrides, and verifies the layout changes between narrow/tall and
wide/short pane sizes. The pure Level 1 layout tests should remain for boundary
coverage.

### 2. High: Right alignment and wrapping lack Level 2 assertions

The single-table Level 2 test verifies border glyphs, header order, thousands
separators, and a striped row, but it does not verify that `Total` and `Cached`
share right-aligned cell boundaries. Neither Level 2 test uses a long set title
or prefix, so the required wrapping behavior is also unverified in a real
terminal.

Evidence:

- `cli/tests/level2_terminal.rs:385-426` checks presence and ordering, not cell
  alignment or wrapping.
- `cli/tests/level2_terminal.rs:440-479` uses only short titles and prefixes.
- Acceptance criteria 1 and 2 require aligned output; the table-output design
  requires wrapping instead of pushing count columns off screen.

Seed rows with different count widths and an overlong title/prefix. From the
captured pane text, assert equal right edges for numeric cells, continued
wrapped text rows, and unchanged count-column visibility.

## Verification Matrix

| Requirement | Strongest verification | Assessment |
|---|---:|---|
| Present, zero, and missing upstream totals parse in prefix order | Level 1 wiremock | Appropriate |
| Schema v2, nullable total, v0/v1 migration, and rollback | Level 1 SQLite | Appropriate |
| Set totals round-trip through cache queries | Level 1 SQLite | Appropriate |
| Cached counts use a grouped query, omit zero rows, and exclude embedded icons | Level 1 SQLite | Appropriate |
| Online totals persist for later offline display | Level 1 CLI subprocess | Appropriate |
| Empty online and offline results return errors | Level 1 CLI subprocess | Appropriate |
| Cached zero and nonzero values appear on the correct rows | Level 1 CLI subprocess | Appropriate |
| Bare invocation preserves default `icons` dispatch | Level 1 CLI subprocess | Appropriate |
| Unicode borders, thousands separators, and alternating background | Level 2 tmux capture | Appropriate |
| Single/split choice from manufactured dimensions | Level 2 tmux rendering plus Level 1 logic | Necessary, but live terminal sizing is unverified |
| Single/split choice from actual terminal dimensions | No direct verification | **Level mismatch: requires Level 2** |
| Right-aligned `Total` and `Cached` columns | Level 1 component configuration | **Level mismatch: requires Level 2** |
| Long title/prefix wrapping with count columns retained | No direct verification | **Level mismatch: requires Level 2** |
| Keyboard, mouse, paste, or IME behavior | Not applicable | No Level 3 requirement |

## Validation

- `git diff --cached --check`: passed.
- `cargo test -p biscuit-icon`: not run; `rustup` has no installed/default Rust
  toolchain.
- `cargo test -p biscuit-icon-cli`: not run for the same reason.
- `just -f biscuit-icon/justfile test-l2`: not run for the same reason.

The requested `biscuit-icon` skill is not present in the repository skill
catalog or configured skill roots. The review used the package specification,
prior package review, implementation, and the `rust-testing`, `sniff`, and
`darkmatter` skills.
