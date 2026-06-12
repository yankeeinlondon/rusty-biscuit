---
ready: false
agent: codex
model: ""
---

# Review: Rename `tui-chrome` to `biscuit-tui`

## Findings

### High: Required residual-reference verification is not reproducible as written

The spec makes the residual-reference command a success criterion: it must return no unexpected matches, with historical records and the rename specification intentionally excluded ([spec.md:68](spec.md:68)). The command in the spec does not do that because its exclusion uses `2026-06-5-rename` instead of the actual `2026-06-05-rename` path ([spec.md:260](spec.md:260)). The implementation also added `biscuit-tui/features/2026-06-05-rename/plan.md`, which intentionally contains old names and is not excluded by the spec command.

Running the spec command exactly reports the rename spec and plan as matches. That means Phase 5 is marked complete even though the published verification command cannot pass without manual correction ([plan.md:196](plan.md:196)). Fix the date typo and exclude the plan, or update the spec to make the intended corrected command authoritative.

### Medium: Plan status contradicts the completed implementation

Phase 4 is still unchecked for the README, component docs, theming docs, skill, and active inventory updates ([plan.md:169](plan.md:169)), but Phase 5 is marked complete and claims the area and `claudine` verification passed ([plan.md:197](plan.md:197)). The files themselves appear updated, so this is plan/documentation drift rather than a code defect. It still matters because this feature uses the plan as its auditable implementation record.

## Test Rigor

This rename has no intended user-observable TUI behavior change: the `question` binary name, flags, output formats, and exit codes are specified as unchanged. The requirements are compile-time package identity, Rust import paths, docs, and repo metadata. Level 1 verification is the appropriate minimum; Level 2 and Level 3 are not required for the rename itself.

Observed verification:

- `cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(biscuit-tui|biscuit-tui-cli)$'` prints `biscuit-tui` and `biscuit-tui-cli`.
- `sniff repo packages --package-area biscuit-tui --list` prints `biscuit-tui-cli` and `biscuit-tui`.
- Corrected residual search excluding both the spec and plan returns no live old-name references.
- `just build`, `just test`, `just doctest`, and `just lint` pass from `biscuit-tui/`.
- `cargo check --manifest-path claudine/cli/Cargo.toml --color=never` passes for the external caller.

I did not rerun the full `claudine/` `just build|test|doctest|lint` suite during review.

## Readiness

Not ready for production as recorded because one of the spec's explicit success criteria is not reproducible from the committed instructions. The code rename itself looks complete once the residual search is corrected.
