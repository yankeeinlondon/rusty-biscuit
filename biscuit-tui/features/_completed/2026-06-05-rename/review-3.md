---
ready: false
agent: codex/default
created: 2026-06-30T18:26:22
---

# Review: Rename `tui-chrome` to `biscuit-tui`

## Findings

### High — The authoritative feature records now describe a no-op package rename

The code-level rename appears complete, but the spec and plan no longer clearly record the package rename they are validating. The spec title says `biscuit-tui` → `biscuit-tui` ([spec.md:7](spec.md:7)), the goal says the historical crates were named `biscuit-tui*` instead of `biscuit-tui*` ([spec.md:21](spec.md:21)), the package table lists identical current/new package names ([spec.md:27](spec.md:27), [spec.md:28](spec.md:28)), and the Cargo note says to change the dependency key from `biscuit-tui` to `biscuit-tui` ([spec.md:43](spec.md:43)). The plan repeats the same no-op operations in its title and Phase 1 checklist ([plan.md:93](plan.md:93), [plan.md:107](plan.md:107), [plan.md:108](plan.md:108), [plan.md:109](plan.md:109)).

That conflicts with the same spec's stale-reference definition, which still identifies `tui-chrome` as the old package literal and `tui_chrome` as the old Rust import path ([spec.md:287](spec.md:287), [spec.md:288](spec.md:288)). It also conflicts with the out-of-scope rule saying this feature's records necessarily name both old and new crates ([spec.md:243](spec.md:243), [spec.md:244](spec.md:244), [spec.md:245](spec.md:245)). As written, a reviewer or future maintainer cannot reconstruct the actual package rename boundary from the authoritative spec/plan without inferring it from the residual grep.

Recommended fix: restore the old package literals in this feature's own records where they are semantically required (`tui-chrome`, `tui-chrome-cli`, and `tui_chrome::`), while keeping the corrected residual search that excludes `biscuit-tui/features/2026-06-05-rename/**`. The feature records are explicitly excluded from stale-reference validation, so preserving the historical old name there will not break the live-code search.

### Medium — Suggested verification text still omits `biscuit-icon`

The success criteria now correctly require `just build`, `just test`, `just doctest`, and `just lint` in `biscuit-icon/` as a live external dependent ([spec.md:67](spec.md:67), [spec.md:68](spec.md:68), [spec.md:69](spec.md:69)), and `validation-1.md` records that matrix as passing ([validation-1.md:62](validation-1.md:62), [validation-1.md:66](validation-1.md:66)). However, the suggested execution order still says to verify only the `biscuit-tui` area and `claudine` ([spec.md:262](spec.md:262), [spec.md:263](spec.md:263), [spec.md:264](spec.md:264)).

This is documentation drift, not a code defect, because the binding success criteria and validation matrix include `biscuit-icon`. Still, the execution-order checklist is the path implementers will follow, so it should include `biscuit-icon` to avoid reintroducing the validation-scope gap fixed in review 2.

## Test-Level Assessment

This rename is a compile-time package/import rename. The spec keeps the `question` binary name, CLI flags, output formats, exit codes, public APIs, and TUI behavior unchanged. There are no new user-observable terminal behavior requirements for modifier presses, hotkey activation, paste, IME, mouse, scrolling, glyph width, or SGR styling.

Appropriate verification level:

- Package identity and dependency graph: Level 1 via `cargo metadata` and `sniff repo packages`.
- Import-path correctness: Level 1 via build/test/doctest/lint.
- Stale-reference absence: repository search, not a terminal-behavior test.

Level 2 and Level 3 tests are not required for this rename. I verified during this review that metadata reports `biscuit-tui` and `biscuit-tui-cli`, `sniff repo packages --package-area biscuit-tui --list` reports `biscuit-tui-cli` and `biscuit-tui`, the reverse-dependent scan reports `biscuit-icon-cli`, `claudine-cli`, and `biscuit-tui-cli`, and the spec's residual search for `tui_chrome|tui-chrome` returns no live matches. I did not re-run the full `just build|test|doctest|lint` matrix; `validation-1.md` records those results.

## Verdict

Not ready for production as recorded. The live code rename and Level 1 validation evidence look healthy, but the feature's source-of-truth records currently erase the old package name in the very places that define the rename. Fixing that documentation drift should be surgical and should not require code changes.
