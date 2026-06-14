---
ready: false
agent: codex
model: ""
---

# Icon Sets Formatting Review

Iteration 3 closes the live terminal sizing, numeric alignment, and long-title
wrapping gaps from review 2. The feature is still not ready for production
because prefix wrapping remains unverified in a real terminal.

## Findings

### 1. High: Prefix wrapping still lacks Level 2 verification

The specification requires both `Set` and `Prefix` to wrap at bounded widths so
neither can displace or hide the count columns. The new Level 2 test exercises
an overlong title, but every seeded prefix is short. There is also no Level 1
rendering test for an overlong prefix.

Evidence:

- `cli/tests/level2_terminal.rs:594-605` seeds only `zzbig`, `zzshort`, and
  `zzwrap` as prefixes.
- `cli/tests/level2_terminal.rs:653-682` asserts title continuation behavior
  only.
- `cli/src/sets_table.rs:44-46` configures prefix wrapping, but configuration
  alone does not verify real-terminal rendering.
- `spec.md:176-179` requires both `Set` and `Prefix` to use bounded wrapping
  while retaining count values.

Add an overlong prefix row to the Level 2 test and assert that its continuation
line remains inside the Prefix cell, the title is not repeated, and the full
`Total` and `Cached` values remain visible on the row. A focused Level 1
rendering assertion should cover the same boundary for faster diagnosis.

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
| Unicode borders, thousands separators, and alternating background | Level 2 tmux capture | Appropriate |
| Layout selection from actual terminal dimensions | Level 2 resized tmux pane | Appropriate |
| Balanced column-major split ordering | Level 2 tmux capture plus Level 1 logic | Appropriate |
| Right-aligned `Total` and `Cached` columns | Level 2 tmux capture | Appropriate |
| Long title wrapping with count columns retained | Level 2 tmux capture | Appropriate |
| Long prefix wrapping with count columns retained | Component configuration only | **Level mismatch: requires Level 2** |
| Keyboard, mouse, paste, or IME behavior | Not applicable | No Level 3 requirement |

## Validation

- `git diff --cached --check`: passed.
- The tmux resize sequence was exercised directly with tmux 3.6a; the requested
  window and pane dimensions were reported correctly.
- `cargo test -p biscuit-icon`: not run; `rustup` reports no active toolchain.
- `cargo test -p biscuit-icon-cli`: not run for the same reason.
- `just -f biscuit-icon/justfile test-l2`: not run for the same reason.

The requested `biscuit-icon` skill is not present in the repository's
authoritative skill catalog or configured skill roots. The review used the
package specification, implementation, prior reviews, and the `rust-testing`
skill.
