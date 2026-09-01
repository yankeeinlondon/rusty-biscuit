---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-01T21:43:08+01:00
spec: /Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
implemented: false
description: "A **fix** review of `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md`"
fix: 2026-09-01-inline-compose-frontmatter/review-1.md
---

# Review 1: Inline Compose Frontmatter

## Verdict

Not ready for production. The textual hash writer, authorized response-frontmatter harvest, guardrail migration, and source-drift restore behavior are implemented and exercise the intended L1 surfaces. One response-protocol boundary still violates the specification: leading whitespace is discarded before the parser decides whether the response starts with an exact frontmatter delimiter, which can reject a valid body and makes warning line numbers inaccurate.

## Findings

### High: Trimming before delimiter detection violates the exact-leading-delimiter contract and shifts response line diagnostics

`extract_replacement_parts` calls `provider_output.trim()` before passing the response to `split_frontmatter_parts` (`claudine/lib/src/composition/closure.rs:47-54`). As a result, a response such as `"  ---\nnot: metadata\n---\nBody"` is treated as a response-frontmatter attempt even though it does not begin with the exact `---` delimiter required by D2. Depending on the indented body content, Claudine can reject a response that the specification requires it to treat as ordinary body text.

The same preprocessing removes leading blank lines before `top_level_key_locations` computes response locations (`closure.rs:75-83`). An unauthorized property originally returned on line 4 can therefore be reported as line 2, contrary to AC7's source-accurate warning requirement.

Detect the optional response-frontmatter block against the original response bytes. Body normalization may remain an explicit later step, but line locations must retain an offset into the untrimmed response. Add L1 tests for a whitespace-prefixed delimiter remaining ordinary body, a genuinely leading delimiter after several blank lines following the opener, and an unauthorized key whose reported line includes all original leading lines.

## Requirement Verification Levels

These requirements concern parsing, file transformation, provider-stub execution, and captured CLI status text. Level 1 is appropriate throughout; none depends on a terminal emulator's renderer or input encoder, so Level 2 or Level 3 would add no relevant verification.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| AC1 authored-byte preservation | Level 1 unit | Appropriate: exact full-document comparison permits only the managed hash and date changes. |
| AC2 no escaped prompt regression | Level 1 unit | Appropriate. |
| AC3 hash consistency/idempotence | Level 1 unit and CLI process | Appropriate, including the AC1 multiline fixture. |
| AC4 Structured-to-Simple downgrade | Level 1 unit | Appropriate. |
| AC5 generic textual hash writer | Level 1 unit and CLI process | Appropriate: LF/CRLF, representations, custom/quoted keys, and no-write flow-root failures are exercised. |
| AC6 authorized response harvest | Level 1 unit and provider-stub CLI process | Appropriate. |
| AC7 authority, immutability, and source-accurate warnings | Level 1 unit and CLI process | Functional gap: warning lines are inaccurate when the response has leading whitespace. |
| AC8 invalid harvest is non-mutating | Level 1 unit | Functional gap: whitespace-prefixed delimiters are incorrectly promoted to harvest attempts. |
| AC9 guardrail migration | Level 1 unit | Appropriate, including failure warning and non-truncation behavior. |
| AC10 end-to-end inline compose | Level 1 provider-stub CLI process | Appropriate for the covered response shape. |
| AC11 mid-run drift restoration/reporting | Level 1 unit and CLI process | Appropriate: additions, removals, malformed shapes, canonical value drift, body drift, and non-attributing status text are covered. |
| AC12 unchanged-body rejection | Level 1 unit | Appropriate. |

## Validation

- Focused Claudine Nextest run: 32 passed, 0 failed.
- Focused Darkmatter Nextest run: 17 passed, 0 failed.
- `claudine/just lint`: passed for all five crates and the diagnostic guards.
- `darkmatter/just lint`: passed for all three crates.
- `claudine/just test`: 4,589 passed, 1 failed, 11 skipped, and 2,052 not run after fail-fast cancellation. The failure is the unrelated `shipped_prompt_contract` check against `prompts/fixes/2026-09-01-file-param-anchoring/plan.md`, whose body contains an invalid `{{ ... }}` expression; no inline-compose-frontmatter test failed.
- `git diff --check`: passed before writing this review.

## Positive Notes

The earlier review findings are resolved in the current tree. Hash stamping remains textual through both Claudine and `md hash --save`; generated properties are statically allowlisted and safely serialized; closure-owned properties remain protected; guardrail migration is atomic and fail-soft; and source drift is restored and reported without attributing an unknown writer. The CLI's new status output is built from `TerminalRenderable` components.
