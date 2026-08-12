---
ready: false
agent: codex/default
created: 2026-06-30T15:13:38
---

# Review: Rename `biscuit-tui` to `biscuit-tui`

## Findings

### High: residual-reference verification cannot pass as written

The spec makes the residual-reference command a success criterion: it must return no unexpected matches after excluding historical records and this feature's own records ([spec.md:68](spec.md:68)). The command currently searches for `biscuit-tui|tui_chrome` ([spec.md:254](spec.md:254)), but `biscuit-tui` is the new, intended package/area name and is expected to remain throughout live manifests, docs, workflows, and source comments.

Running the command as written reports hundreds of live, expected `biscuit-tui` matches, including root workspace members, package manifests, docs, tests, release-plz config, and the biscuit-tui skill. This makes the plan's completed Phase 5 claim unreproducible ([plan.md:197](plan.md:197)). The stale Rust import path itself appears clean: a targeted live search for `tui_chrome|tui-chrome`, excluding completed/historical records and this feature's own files, produced no matches. Fix the residual command so it only searches stale identifiers, or explicitly separate "old package name" checks from expected new-name references.

### High: validation scope misses a live external caller

The spec says `claudine` is the only external workspace member that depends on the library and asks implementation to re-run that scan rather than treating the statement as permanently exhaustive ([spec.md:136](spec.md:136), [spec.md:139](spec.md:139)). The current metadata scan shows three packages depending on `biscuit-tui`: `biscuit-tui-cli`, `claudine-cli`, and `biscuit-icon-cli`. `biscuit-icon-cli` has a live dependency on the renamed library at [biscuit-icon/cli/Cargo.toml:24](../../../biscuit-icon/cli/Cargo.toml:24).

The implementation plan's package list and final validation only include `biscuit-tui/lib`, `biscuit-tui/cli`, and `claudine/cli` ([plan.md:86](plan.md:86), [plan.md:198](plan.md:198), [plan.md:199](plan.md:199)). Even though `biscuit-icon-cli` already uses the new dependency key, this rename is a compile-time package identity change, so every live dependent package must be in the validation set. Add `biscuit-icon-cli` to the caller inventory and run at least its build/test path, or update the spec with an explicit reason it is out of scope.

## Test Rigor

This rename does not specify a user-observable TUI behavior change: the `question` binary name, flags, output formats, exit codes, and widget behavior are unchanged. The relevant requirements are compile-time package identity, Rust import paths, docs, and repo metadata. Level 1 compile/test/doctest/lint verification is the appropriate minimum. Level 2 and Level 3 tests are not required for this feature because no terminal rendering, input encoder, keybinding, paste, mouse, or modifier-press behavior is changing.

Verified during this review:

- `cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(biscuit-tui|biscuit-tui-cli)$'` prints `biscuit-tui` and `biscuit-tui-cli`.
- `sniff repo packages --package-area biscuit-tui --list` prints `biscuit-tui-cli` and `biscuit-tui`.
- Targeted stale-reference search for `tui_chrome|tui-chrome` outside historical records returned no live matches.
- Dependency scan for packages depending on `biscuit-tui` prints `biscuit-icon-cli`, `claudine-cli`, and `biscuit-tui-cli`.

I did not rerun the full `just build|test|doctest|lint` matrix for `biscuit-tui`, `claudine`, or `biscuit-icon` as part of this review.

## Production Readiness

Not ready for production as recorded. The code rename itself appears largely complete for stale `tui_chrome` references, but one explicit success criterion is impossible to reproduce as written, and the documented validation scope omits a current live workspace caller.
