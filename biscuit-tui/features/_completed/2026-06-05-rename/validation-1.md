# Phase 3 Validation Matrix - Review Remediation Results

Reproducible evidence that the `tui-chrome` → `biscuit-tui` rename is healthy across
the package area and every live dependent. All commands run on macOS (Darwin 25.5.0)
from the `tui` worktree at `/Users/ken/.claudine/worktrees/rusty-biscuit/tui`.

## Package Identity

```bash
cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(biscuit-tui|biscuit-tui-cli)$'
```

Result: prints exactly

```
biscuit-tui
biscuit-tui-cli
```

```bash
sniff repo packages --package-area biscuit-tui --list
```

Result: reports

```
biscuit-tui-cli
biscuit-tui
```

## Stale-Reference Search (corrected Phase 1 command)

```bash
rg -n --hidden 'tui_chrome|tui-chrome' . \
  -g '!target/**' \
  -g '!.git/**' \
  -g '!**/features/_completed/**' \
  -g '!**/reviews/**' \
  -g '!features/2026-05-24-testing-best-practices/review-*.md' \
  -g '!.claude/skills/claudine/timeline.md' \
  -g '!claudine/claudine-output/**' \
  -g '!biscuit-tui/features/2026-06-05-rename/**'
```

Result: no live matches (exit code 1).

## Cargo.lock

Root `Cargo.lock` is present. Searched for stale package names:

```bash
rg -n 'tui-chrome|tui_chrome' Cargo.lock
```

Result: no matches (exit code 1).

## Build / Test / Doctest / Lint Matrix

All three areas expose the full shared recipe set (`build`, `test`, `doctest`,
`lint`), so no package-specific substitutions were required.

| Area | `just build` | `just test` | `just doctest` | `just lint` |
|------|:---:|:---:|:---:|:---:|
| `biscuit-tui` | pass | pass (411 tests, 25 skipped) | pass (18 passed, 1 ignored) | pass |
| `claudine` | pass | pass (1858 tests, 147 skipped) | pass | pass |
| `biscuit-icon` | pass | pass (97 tests, 10 skipped) | pass (0 doctests) | pass |

### Notes, flakes, and one environment-only failure

- **claudine `just test` — first run had one failure, root cause was an unbuilt
  test fixture, not the rename.** `claudine-cli::inline_compose_hash
  inline_compose_writes_hash_that_passes_md_diff` failed with `md binary not found
  at .../target/debug/md`. This test shells out to the darkmatter `md` CLI, which
  was not yet present in the shared `target/`. After
  `cargo build -p darkmatter-cli --bin md`, the test passed in isolation and the
  full suite then reported `1858 tests run: 1858 passed`. Unrelated to the rename.
- **Flaky retries (passed on retry, not failures):**
  - `biscuit-tui` / claudine harnesses report nextest retries that resolve green.
  - claudine: `sequence_perf_renders_single_aggregated_report` (FLAKY 3/4) and
    `commands::wrap::exec::exit::tests::first_response_preference_semantic_over_raw_over_stderr`
    (FLAKY 2/4) both passed on retry. Pre-existing timing-sensitive tests,
    unrelated to the rename.
- **biscuit-icon `just doctest`** ran 0 doctests (CLI-only area, no lib doctests);
  this is expected, not a skip caused by the rename.

### biscuit-icon dependent confirmation

`biscuit-icon/cli/Cargo.toml` depends on `biscuit-tui` via
`path = "../../biscuit-tui/lib"` (verified in Phase 2). A stale-reference scan of
`biscuit-icon/cli` returns no `tui_chrome` / `tui-chrome` output.

## Conclusion

Every Phase 3 validation command passes. The single observed `just test` failure
was an environment artifact (missing `md` fixture binary), resolved by building
`darkmatter-cli`; the remaining non-green lines were nextest flaky retries that
passed. No Level 2 / Level 3 terminal tests were required — this remediation
changes validation/documentation scope, not TUI rendering, input, keybindings,
paste, mouse, or modifier behavior.
