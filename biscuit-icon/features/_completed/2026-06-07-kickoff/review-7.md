---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 7 closes the default dependency-graph, direct-lookup injection,
set-catalog persistence, and first-page truncation findings from iteration 6.
The feature is not ready for production: ordinary filtered listings now perform
an unbounded sequence of live icon fetches, the required terminal rendering
verification is failing, and the no-filter command still cannot reach the
online catalog promised by the specification.

## Findings

### 1. High: Filtered listings fetch every online match serially and can become effectively unusable

`search_icons` now paginates until `total` is reached, after which the CLI calls
`Icon::iconify_with` once per result, serially. A broad query therefore performs
one search request plus potentially hundreds or thousands of body requests,
each with a ten-second timeout. With the `image` feature, every glyph-less result
also creates and rasterizes a temporary SVG during rendering. There is no result
limit, paging UI, concurrency bound, or API-side `--from` restriction.

This is visible in the canonical test run: `icon icons arrow`, `apple`, `github`,
and `grinning` queried the public service and produced enough remote rows to
scroll the expected curated rows out of the tmux pane. One fetch also failed to
decode, but the command continued.

Evidence:

- `lib/src/iconify/client.rs:161-190` accumulates every search page in memory.
- `cli/src/commands.rs:83-101` then awaits every uncached icon body one at a time.
- `cli/src/commands.rs:86-90` applies `--from` only after downloading the complete search result list.
- `lib/src/icon.rs:94-123` opens the SQLite database in two blocking tasks and performs a network request for every cache miss.
- `lib/src/icon.rs:244-257` creates a temporary SVG and invokes terminal image rendering per icon when images are enabled.

Define a bounded listing contract: expose a limit/page cursor or stream pages
with explicit continuation, pass allowed prefixes to the Iconify search API,
and fetch bodies with bounded concurrency or collection-level batching. Add a
test with a realistically large `total` that asserts request count, bounded
runtime/work, and user-visible truncation or pagination behavior.

### 2. High: Terminal glyph/text requirements do not currently have passing Level 2 verification

The four tmux tests for Unicode glyphs, Nerd Font glyphs, text fallback, and
multi-row listing all failed under `just test`. They use broad filters and do not
disable the live service, so the rows under test are displaced by online results.
These tests therefore neither isolate the rendering requirement nor provide a
stable real-terminal assertion.

The same Level 2 tests are also included by the package's ordinary `just test`
recipe. That contradicts the repository taxonomy, which requires Level 2 tests
to run only through `just test-l2` so shared harness setup, serialization, and
resource gating are applied correctly.

Evidence:

- `cli/tests/level2_terminal.rs:72`, `:101`, `:130`, and `:222` invoke broad filters that trigger live online searches.
- `justfile:37-39` delegates to `_test` without excluding `level2_` tests.
- `just/devops.just:165` runs the package without a Level 2 exclusion filter.
- `just test` produced 20 passing CLI tests and four repeatedly failing Level 2 tests.

Use direct built-in identifiers, or point `ICONIFY_BASE_URL` at a deterministic
dead/mock endpoint, so each test renders a known row without network expansion.
Exclude `level2_` tests from the L1 recipe and keep them in `just test-l2`.

### 3. High: The image Level 2 assertion can still pass because of unrelated red terminal pixels

The revised image test uses a distinctive red SVG and now fails when screenshot
capture is unavailable, which fixes one iteration-6 problem. It still scans the
entire terminal window for any red pixel. A red prompt, command text, shell
decoration, diagnostic, or theme element satisfies the predicate even when the
graphics protocol renders nothing. No baseline image or bounded icon region is
used.

Evidence:

- `cli/tests/level2_terminal.rs:178-184` leaves the command and shell UI in the captured window.
- `cli/tests/level2_terminal.rs:191-204` searches every pixel and accepts the first red one.
- `just test-l2` could not execute the assertion because window capture returned `None`; the test failed four times before fail-fast canceled the other five tests.

Capture a pre-render baseline and compare a known icon cell, or clear the pane
and assert the geometry/color of a tightly bounded witness region. Until that
passes in a real image-capable terminal, image fallback has no valid Level 2
verification and remains a production blocker.

### 4. High: `icon icons` without a filter never reaches the online catalog

The filter is optional, and the listing boundary says the full Iconify catalog
is reached through the API when online. The implementation returns immediately
after printing offline rows when the filter is empty. The new test explicitly
locks in this incomplete behavior by using a dead endpoint and expecting success.
The README also says the online catalog is always queried and merged.

Evidence:

- `cli/src/commands.rs:74-81` skips online lookup for an empty filter.
- `cli/tests/cli.rs:365-380` treats skipping the online service as the intended behavior.
- `README.md:201-205` describes an optional filter and says online results are always merged.
- `features/2026-06-07-kickoff/spec.md` defines `icons [filter]` and states that the full catalog is reached via the API when online.

Implement a supported all-icons catalog path with explicit pagination, or revise
the specification and README to make an empty filter intentionally offline-only.
The current implementation and accepted contract disagree.

### 5. Medium: Partial online fetch failures are reported per row but the command exits successfully

After search succeeds, body-fetch failures are printed to stderr and discarded.
The command returns success even though it did not list all matches and did not
cache the failed rows. This is especially likely with the unbounded request
pattern above and gives scripts no reliable indication that output is partial.

Evidence:

- `cli/src/commands.rs:96-101` logs each failed icon and continues.
- The real `arrow` run printed `jam:arrow-up: iconify fetch failed: error decoding response body` while continuing through the catalog.
- No CLI test covers a mixed success/failure search response or asserts the final exit status.

Aggregate failures and return a non-zero result, or print an explicit partial
results summary and define that exit behavior in the CLI contract. Add a mocked
mixed-result test.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate |
| Local SVG styling and non-zero origins | Level 1 | Appropriate |
| Cache-first direct lookup | Level 1 wiremock | Appropriate |
| Online search pagination | Level 1 wiremock | Functional pagination covered; bounded UX/work is not |
| Online set listing and persistence | Level 1 wiremock | Appropriate |
| Browser and Markdown inline SVG | Level 1 | Appropriate |
| Terminal Unicode glyph | Level 2 failing | **Level requirement unmet** |
| Terminal Nerd Font glyph | Level 2 failing | **Level requirement unmet** |
| Terminal text fallback and listing | Level 2 failing | **Level requirement unmet** |
| Image-protocol fallback | Level 2 attempted | **Level mismatch: assertion is not image-region-specific and could not run here** |
| Styled CLI errors | Level 2 passed in the ordinary test run | Behavior passed, but tier execution is misconfigured |
| OS keyboard behavior | Not applicable | No Level 3 requirement |

## Validation

- `just sanity`: passed 85 library tests; the CLI sanity target ran no integration tests.
- `just test`: library passed 92 tests; CLI passed 20 tests and failed four Level 2 tests after retries.
- `just lint`: passed for both crates.
- `cargo test -p biscuit-icon-cli --features image --no-run`: passed.
- `just test-l2`: failed because the required WezTerm screenshot capture returned `None`; fail-fast canceled the remaining five tests.
- `cargo tree -p biscuit-icon --no-default-features` contained no `resvg`, `biscuit-visualized`, or `mermaid-rs-renderer`, confirming the default-off image dependency boundary is fixed.
