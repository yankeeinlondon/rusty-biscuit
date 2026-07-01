---
ready: false
agent: codex/default
created: 2026-06-30T18:20:01
implemented: true
---

# Review: Rename `biscuit-tui` to `biscuit-tui`

## Findings

### High — Residual-reference verification no longer reproduces because `validation-1.md` matches the stale identifiers

The spec's success criteria require the residual-reference command in the Verification section to return no matches. The command excludes the spec, plan, plan-1, and review-1 records, but it does not exclude the newly added validation record:

- `spec.md:275` through `spec.md:286` define the residual-reference command and exclusions.
- `validation-1.md:3`, `validation-1.md:34`, `validation-1.md:55`, and `validation-1.md:93` contain `tui-chrome` / `tui_chrome`.
- `validation-1.md:48` says the same command produced no live matches.

Re-running the spec command now returns those `validation-1.md` matches, so the recorded evidence is not reproducible from the current tree. That blocks production readiness because the rename's explicit completion check fails.

Recommended fix: either add `biscuit-tui/features/2026-06-05-rename/validation-1.md` to the intentional feature-record exclusions in the spec, plan, and validation command, or rewrite the validation record so it does not contain the stale literals. Then re-run the residual-reference command after the final record exists and record the actual result.

## Test-Level Assessment

This feature is a compile-time package/import rename. The spec explicitly keeps the `question` binary name, CLI flags, output formats, exit codes, public APIs, and TUI behavior unchanged. There are no new user-observable terminal behavior requirements for modifier presses, keybindings, paste, mouse, scrolling, glyph layout, or style rendering.

Appropriate verification level:

- Package identity and dependency graph: Level 1 via `cargo metadata` and `sniff repo packages`.
- Import-path correctness: Level 1 via build/test/doctest/lint.
- Stale-reference absence: repository search, not a terminal-behavior test.

Level 2 and Level 3 tests are not required for this rename. The recorded matrix in `validation-1.md` says the `biscuit-tui`, `claudine`, and `biscuit-icon` `build`, `test`, `doctest`, and `lint` recipes passed, and I confirmed the package identity and reverse-dependent metadata scan. I did not re-run the full `just` matrix during this review.

## Notes

- `cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(biscuit-tui|biscuit-tui-cli)$'` prints `biscuit-tui` and `biscuit-tui-cli`.
- `sniff repo packages --package-area biscuit-tui --list` reports `biscuit-tui-cli` and `biscuit-tui`.
- The reverse-dependent scan reports `biscuit-tui-cli`, `claudine-cli`, and `biscuit-icon-cli`, matching the updated spec scope.
- `plan.md:204` still has the `biscuit-icon` validation item unchecked even though `validation-1.md:69` records the biscuit-icon matrix as passing. This is secondary to the blocking residual-search issue, but it should be reconciled when updating the validation record.

## Verdict

Not ready for production. The implementation appears to have the correct package identities and dependent set, and Level 1 is the right verification level for this rename, but the required residual-reference check currently fails against the repository's own new validation record.
