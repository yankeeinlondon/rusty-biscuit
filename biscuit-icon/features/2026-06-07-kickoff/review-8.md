---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 8 closes the earlier L1/L2 test separation, deterministic glyph/text
terminal tests, partial-fetch exit status, and empty-filter contract findings.
The feature is not ready for production: multi-prefix online filtering sends the
wrong Iconify API parameter, and the image fallback still lacks a passing,
image-specific Level 2 verification.

## Findings

### 1. High: Multi-set `--from` filtering sends an invalid Iconify search contract

`--from` is documented as a CSV list, but `search_icons` always joins every
requested prefix and sends it as the singular `prefix` query parameter. Iconify's
search API defines `prefix` for one icon set and `prefixes` for a comma-separated
list. Therefore `--from mdi,lucide` is sent as `prefix=mdi,lucide`, which the
service treats as one nonexistent prefix rather than two allowed sets.

The new wiremock test encodes the same incorrect request, so it passes while the
real integration is broken.

Evidence:

- `lib/src/iconify/client.rs:190-192` always emits `prefix=<joined CSV>`.
- `lib/src/iconify/client.rs:377-395` expects `prefix=mdi,lucide`.
- `cli/src/commands.rs:93-97` passes the complete `--from` set to that client.
- The [Iconify search API](https://iconify.design/docs/api/search.html) specifies
  `prefix` for one set and `prefixes` for comma-separated sets.

Emit `prefix` for one value and `prefixes` for multiple values, then add an L1
HTTP-contract test for both cases and a CLI test using at least two prefixes.

### 2. High: Image fallback still has no valid Level 2 verification

The required `just test-l2` run fails because the WezTerm harness cannot capture
a window image. Nextest retries the test four times and then fail-fast cancels the
other five Level 2 tests. A second run with `--no-fail-fast` confirms that Unicode,
Nerd Font, text fallback, listing, and styled-error tests pass, but the image
requirement remains unverified.

Even where screenshot permission is available, the revised assertion is not
image-specific. Its baseline is captured before the command is typed, then the
test accepts any new red pixel anywhere in the whole terminal window. Newly
rendered command text, prompt changes, diagnostics, or other UI changes can still
satisfy it. The comment claiming command text is excluded is therefore drifted.

Evidence:

- `cli/tests/level2_terminal.rs:178-197` captures the baseline before entering
  the command and captures the entire window afterward.
- `cli/tests/level2_terminal.rs:202-227` accepts the first newly red pixel at any
  screen coordinate.
- Canonical `just test-l2` result: 0 passed, 1 failed, 5 canceled.
- Diagnostic `just test-l2 --no-fail-fast` result: 5 passed, image test failed.

Capture and compare a bounded icon cell or assert a stable image-protocol witness
through a harness API that does not depend on unrelated window pixels. The test
must pass through a real image-capable terminal before the image feature is ready.

### 3. Medium: Online listings silently truncate at 100 results

The new bound prevents unbounded fetching, but it changes `icons <filter>` into a
silent first-100 listing. Neither the CLI output, README, nor specification tells
the user that more matches exist or provides a cursor/page/limit option. This is
especially misleading because the specification says a filtered listing reaches
the full Iconify catalog.

The added test named `icons_limits_online_results_with_large_catalog` returns only
three icons with `total: 3`; it never exercises a response larger than the cap,
does not assert an exact request/body-fetch count, and does not verify a visible
truncation notice.

Evidence:

- `cli/src/commands.rs:43-44` hard-codes `MAX_RESULTS` to 100.
- `cli/src/commands.rs:96-130` discards result-count metadata and prints no notice.
- `cli/tests/cli.rs:149-193` supplies only three results.
- `features/2026-06-07-kickoff/spec.md:287-290` promises access to the full online
  catalog for filtered listings.

Expose an explicit limit/page contract or print a deterministic truncation notice.
Test with more than 100 advertised and returned rows, asserting bounded search and
body request counts plus the user-visible continuation behavior.

### 4. Low: The L1 CLI suite passes only after retrying a leaked-handle test

`completions_bash_emits_script` was reported by nextest as successful but leaking
handles on its first attempt, then passed on retry. A retry masks intermittent
subprocess cleanup and weakens the package's fast confidence signal.

Evidence:

- `cli/tests/cli.rs:30-38` is the affected subprocess test.
- `just test` reported `LKFAIL` on attempt 1 and `FLAKY` after attempt 2 passed.

Identify the inherited handle or completion subprocess lifetime and make the test
pass without nextest retry support.

## Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Domain enums, SVG assembly, styling, cache, HTTP contracts | Level 1 | Passing, except the multi-prefix mock asserts the wrong upstream contract |
| Default command, direct lookup, cache clear, completions, online merge | Level 1 CLI subprocess | Passing; completion script test is flaky |
| Unicode glyph rendering | Level 2 tmux | Passed |
| Nerd Font glyph rendering | Level 2 tmux | Passed |
| Text fallback and multi-row listing | Level 2 tmux | Passed |
| Styled CLI errors | Level 2 tmux | Passed |
| Image-protocol fallback | Level 2 WezTerm attempted | **Requirement unmet: test fails and assertion is not region-specific** |
| OS keyboard/mouse behavior | Not applicable | No such UX requirement in this feature |

## Commands Run

- `just test`: library 94/94 passed; CLI 21/21 passed with one flaky retry; five
  Level 2 tests correctly excluded.
- `just test-l2`: failed; image test failed four attempts and five tests were
  canceled by fail-fast.
- `just test-l2 --no-fail-fast`: five non-image Level 2 tests passed; image test
  failed four attempts.
- `git diff --check`: passed.

The required `biscuit-icon` skill was not present in the authoritative local skill
catalog or either configured skill root, so package-specific review guidance was
derived from the repository instructions, specification, and implementation. The
`rust-testing` skill was used for the verification-level audit.
