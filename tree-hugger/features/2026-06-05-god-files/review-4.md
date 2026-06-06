---
ready: false
agent: codex
model: ""
---

# Review 4: God Files

## Findings

### High: `--high-risk` is ignored for JSON output

The flag is applied only by the pretty/plain renderer (`cli/src/main.rs:886-897`). The JSON branch serializes the complete `analyses` vector without filtering, so `hug god-files --high-risk --json` still returns moderate-risk files. This violates the command contract that `--high-risk` filters output (`spec.md:228-235`).

The Level 1 integration suite verifies the flag only with `--plain` (`cli/tests/cli.rs:1608-1639`); there is no JSON filtering test. Filter the serialized slice/iterator as well and add a mixed-band JSON regression test asserting that only high-risk records are emitted.

Strongest verification present: **none for JSON filtering**. Required verification: **Level 1**.

### Medium: Dominance hints divide by overlapping symbol SLOC instead of file SLOC

`DominatedBySingleSymbol.share` is calculated as the largest block's SLOC divided by the sum of every eligible symbol block (`lib/src/god_files/analysis.rs:570-577`, `lib/src/god_files/analysis.rs:750-761`). Symbol spans overlap: a class or impl contains its methods, and nested functions can overlap again. The denominator therefore double-counts code and can suppress or understate the hint even when one container owns most of the file.

The rendered claim says the symbol “holds N% of the code,” and the specification's example compares the block to file SLOC. Use `signals.effective_sloc` as the denominator; it is already populated but currently marked dead code (`lib/src/god_files/analysis.rs:150-157`, `lib/src/god_files/analysis.rs:278-287`). Add a regression with a large container and nested blocks where summed block SLOC exceeds file SLOC.

### Medium: Container callouts count descendants and can bind duplicate containers incorrectly

`attach_many_members` treats every structural symbol whose byte range is contained by a container as a member (`lib/src/god_files/analysis.rs:630-683`). This includes methods of nested classes and other deeper descendants, so an outer container can cross the `MANY_MEMBERS_THRESHOLD` despite having few direct members. It also locates the container record by only `(name, kind)` (`lib/src/god_files/analysis.rs:641-645`); two same-named containers in different scopes can both receive the first container's member list.

The current test verifies only that variable/field/parameter kinds are excluded (`lib/src/god_files/analysis.rs:1287-1344`). Derive direct ownership from container identity/span rather than unrestricted containment, and add regressions for a nested container and duplicate container names.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, thresholds, effective SLOC, sorting, caching, blocks, signals, and hints | Level 1 unit | Level 1 | Covered, with semantic gaps above |
| Plain report grouping, empty scans, degraded parsing note, and `--high-risk` filtering | Level 1 CLI integration | Level 1 | Covered |
| JSON shape and degraded parsing note | Level 1 CLI integration | Level 1 | Covered |
| `--high-risk` JSON filtering | None | Level 1 | Gap and broken |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux/WezTerm harness | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never`: passed.
- `cargo test -p tree-hugger-cli --color=never`: passed.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets -- -D warnings`: passed.
- `just test-l2`: passed through the broker-backed, `-j 1` recipe; tmux and WezTerm panes were spawned and exercised. Kitty was unavailable and skip-clean.
