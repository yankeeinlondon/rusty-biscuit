---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/default
created: 2026-09-16T20:53:20-07:00
spec: 2026-09-13-better-static-analysis/spec.md
implemented: false
description: A **fix** review of `2026-09-13-better-static-analysis/spec.md`
fix: 2026-09-13-better-static-analysis/review-2.md
previous: 2026-09-13-better-static-analysis/review-1.md
---

# Review 2: Make Expression Defects Visible Before Anything Runs

## Verdict

The fix is **ready for production**. Both high-severity findings from review
#1 are resolved, no blocked finding existed to reassess, and this iteration
found no new functionality, correctness, performance, ergonomics, or test-rigor
gap.

The implementation now classifies escaped expression openers after the
expression lexer decodes a quoted literal while retaining an authored-byte map
for diagnostics and rewrites. Lifecycle source association now uses structural
field, arbitrary-map-key, and array-index segments, so display punctuation in a
`proxy.with` key cannot alias another surface. Both changes preserve the shared
prepare-time refusal contract and editor/CLI agreement.

Human review is not required. The specification already determines the escape
semantics, structural source identity, diagnostic behavior, and verification
boundaries, and the implementation conforms to those decisions.

## Prior Review Closure

Review #1 contains no `## Blocked Findings` section, so no previously blocked
finding became actionable before the last implementation.

| Review #1 finding | Status in this iteration |
| --- | --- |
| Expression-literal decoding reverses escaped-opener parity after the hard lint | **Implemented.** `flag_literal` scans the lexer-decoded literal value and maps each decoded byte boundary back to the authored literal. Darkmatter, Claudine preparation, and DMLS regressions cover authored backslash runs 1–6, including the runs whose parity changes during decoding. |
| Ambiguous proxy overlay paths can bypass prepare-time validation | **Implemented.** `LifecycleSurfacePath` distinguishes fields, arbitrary map keys, and array indices structurally. Six library cases place the defect on both sides of dotted-key, bracket-key, and nested mapping/array collisions; the CLI process regression proves the formerly bypassed document is refused before its provider marker is written. |

## Findings

None.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, inline, dry-run, proxy, retry/resume, loop, and sequence documents reject nested spans before provider launch | Level 1 library and fake-provider process tests | Appropriate and present. These tests assert failure details and provider non-execution; no terminal-emulator behavior is involved. |
| Shipped review and commit prompts compose and complete lifecycle handling without surviving braces | Level 1 passive shipped-artifact corpus and fake-provider process tests | Appropriate and present. The fixtures are stable copies rather than mutable source inputs. |
| Synthesized action bodies, mixed strings, ordinary frontmatter, and rescanning body expressions retain their existing behavior | Level 1 unit and process tests | Appropriate and present, with positive and negative controls. |
| DMLS reports nested spans with scalar-style-correct ranges, severity, and typed quick fixes | Level 1 in-process diagnostic, code-action, and LSP-session tests | Appropriate and present. This is editor protocol and source-rewrite behavior, not rendered terminal output. |
| Backslash escapes preserve authored bytes, distinguish odd/even runs after owning-syntax decoding, and suppress expression diagnostics | Level 1 scanner, compose, Claudine preparation, DMLS, and LSP-session tests | Appropriate and present. The review-1 decode-boundary gap is covered at all three consumers. |
| List-item diagnostics, array schema ranges, hover, completion, and navigation work without acting on synthetic indices | Level 1 in-process overlay/provider/LSP tests | Appropriate and present, including positive value positions and negative synthetic-marker positions. |
| Malformed, unknown, and nested-span severities follow the new policy while lifecycle-only roots remain scoped | Level 1 diagnostic tests and the mapping-only corpus | Appropriate and present for both frontmatter/body and lifecycle/non-lifecycle directions. |
| Runtime surviving-span errors name the lifecycle property and use the typed reason-specific hint | Level 1 executor and fake-provider process tests | Appropriate and present. The contract is textual diagnostic content rather than glyph, SGR, width, or scrolling behavior. |

Level 2 is not required because no requirement depends on terminal-emulator
decoding, glyph width, SGR styling, hyperlinks, or scrolling. Level 3 is not
required because no requirement depends on OS keyboard/mouse injection, input
encoding, hotkeys, paste, or IME behavior.

## Verification Performed

- Read the complete specification, review #1, its implementation record, and
  the changed lint, source-association, validator, and regression-test paths.
- Refreshed GitNexus for this exact worktree. The index is current at
  `afd7626`; `flag_literal` has exact low upstream impact (one direct caller,
  five total impacted symbols, no indexed process). The lifecycle validator's
  caller result remains `UNKNOWN`, so its shared prepare-stage,
  sequence-preflight, shipped-prompt, and test call sites were confirmed by
  source search rather than treating the empty graph edge set as evidence.
- Inspected the collision matrix and CLI provider-marker assertion directly.
  Typed path equality is used for lookup; dotted/bracket rendering is confined
  to diagnostics and can no longer affect identity.
- The implementation record reports the broader canonical gates green:
  Darkmatter Level 1 passed 7,924 selected tests, Claudine Level 1 passed 7,204
  selected tests with 9 tier-filtered skips, and Claudine `just lint` passed.
- The review's first focused run did not execute tests because the shared
  Cargo target directory was read-only. Focused regressions were rerun through
  the canonical `just` recipes with a fresh temporary `CARGO_TARGET_DIR`:
  Claudine nested-span tests **17 passed**, the CLI collision/provider-marker
  test **1 passed**, the Darkmatter decoded-parity test **1 passed**, and the
  DMLS decoded-parity diagnostic test **1 passed**.
- `md schema validate` passed for review #2, review #1, and the updated
  specification frontmatter.

## Design Assessment

The decoded-to-authored boundary map is the right abstraction: runtime escape
semantics follow the lexer value while diagnostics and rewrites remain anchored
to the author's bytes. The typed lifecycle path removes the entire collision
class without introducing escaping rules into identity or changing the
operator-facing property notation. Both solutions are bounded, allocation
costs are negligible at document-preparation scale, and no further ergonomic or
performance change is warranted.
