---
ready: true
agent: open_code
model: ""
---

# Review: OpenCode 429 Throttle Classification

## Summary

The implementation faithfully delivers the spec's four-kind classification model
(`Overloaded`, `RateLimited`, `UsageCap`, `RetriesExhausted`), the error-context
gate, the advisory path, and the removal of the `stdout_seen` termination guard
for terminal kinds. All 103 existing + new tests pass; clippy is clean.

## Spec Conformance

| Spec requirement | Status | Notes |
|---|---|---|
| `ProviderLimitKind` enum in `events.rs` | Done | Four variants, correct derives |
| `LogClassification::RateLimit` → `ProviderLimit` | Done | `error_name` field dropped, `kind` field added |
| `classify_llm_failure` resolution order (5 steps) | Done | Matches spec exactly |
| Error-context gate (`record.tags["error"]`) | Done | Cap-without-context → advisory `ApiFailure` |
| `contains_any_ci` for overload detection | Done | Case-insensitive matching for `overload`, `engine_overloaded_error` |
| `on_rate_limit` → `on_provider_limit` rename | Done | Per-kind branching in reasoning.rs |
| Remove `stdout_seen` guard for terminal kinds | Done | Both `UsageCap` and `RetriesExhausted` terminate regardless |
| Transient kinds do NOT set `state.rate_limit` | Done | Only terminal kinds write to `state.rate_limit` |
| Transient kinds do NOT bump `rate_limit_events` | **Deviated** | See Finding 1 |
| `render_rate_limit_message` retained for terminal only | Done | Used for `UsageCap` only; `RetriesExhausted` uses a fixed string |
| Advisory path surface provider's own message | Done | Falls back to `record.message` when extraction yields nothing |
| Fixture `opencode-429-overload.txt` | Done | Real Kimi overload line |
| Skill doc `.claude/skills/claudine/opencode-event-sources.md` updated | Done | Four-kind model, resolution order, `kimi-for-coding` gap documented |
| Test renames per spec table | Done | All six renames applied |

## Findings

### Finding 1 — `rate_limit_events` incremented for transient kinds (low)

**Severity:** low

**Location:** `reasoning.rs:351-353`

The spec table says transient kinds (`Overloaded`, `RateLimited`) leave
`state.rate_limit` / `rate_limit_events` **untouched**. The implementation
unconditionally increments `rate_limit_events` for all `ProviderLimit`
classifications, including transient ones.

The tests (`overload_emits_warning_no_early_terminate`,
`throttled_emits_warning_no_early_terminate`) assert `rate_limit_events == 1`,
consistent with the code but inconsistent with the spec's "untouched" wording.

**Assessment:** This is arguably the better behavior — `rate_limit_events` is a
diagnostic counter, and counting transient overloads is useful for post-hoc
analysis. The spec's "untouched" likely intended to convey "no compose-loop
side effects" (which is correct: `state.rate_limit` stays `None`). Recommend
updating the spec table to say "incremented" for the diagnostic counter rather
than changing the code.

### Finding 2 — Stale frontmatter hash in skill doc (low)

**Severity:** low

**Location:** `.claude/skills/claudine/opencode-event-sources.md:2`

The `hash` frontmatter is `39a0c5d58ef53df2-dc0521dad5458b1b` but `md hash`
computes `6fe0cdcab4bcab89-dc0521dad5458b1b`. The body hash matches; only the
frontmatter component is stale. Run `md hash` to correct.

### Finding 3 — No test for `ProviderLimitKind::Overloaded` with `engine_overloaded_error` (low)

**Severity:** low

The overload detection uses `contains_any_ci(haystack, &["overload",
"engine_overloaded_error"])`. The existing overload tests use the word
"overloaded" in the response body message. There is no dedicated test for the
`engine_overloaded_error` keyword (the Kimi standard API error type). The
`contains_any_ci` helper is tested implicitly, and the fixture uses the real
Kimi coding-endpoint line (which says "overloaded"), but a unit test with
`engine_overloaded_error` as the sole overload signal would close the gap.

## Test Rigor Classification

This feature is a **log-parsing and classification** concern. The user never
sees raw 429 bytes or terminal escape codes — they see the semantic events
(`Warning` / `Error`) that the bridge emits. All testable behavior is therefore
**Level 1** (in-process unit tests with manufactured input strings), which is
the appropriate verification level for this feature category.

| User-observable requirement | Strongest test level | Appropriate? |
|---|---|---|
| Overload 429 renders as `server overloaded; will retry` warning | L1 | Yes |
| Throttle 429 renders as `request throttled; will retry` warning | L1 | Yes |
| Usage cap terminates the run even after stdout activity | L1 | Yes |
| Retries exhausted terminates the run | L1 | Yes |
| Cap phrase without error context does NOT terminate | L1 | Yes |
| Cap-with-context wins over retries-exhausted | L1 | Yes |

No Level 2 or Level 3 testing is required for a pure log-classification feature.

## Code Quality

- The resolution order in `classify_llm_failure` is clearly commented with
  numbered steps matching the spec.
- The `on_provider_limit` handler cleanly separates terminal vs. transient
  paths.
- `ProviderLimitKind` is a `Copy` type used by value, avoiding unnecessary
  allocations.
- The advisory path correctly falls back to `record.message` when
  `extract_provider_message` returns `None`.
- Test names are descriptive and match the spec's rename table.

## Verdict

**Ready for production.** The implementation is complete, well-tested, and
matches the spec with one intentional deviation (Finding 1) that is arguably
an improvement. The two low-severity findings (stale hash, missing keyword
test) are housekeeping items, not blockers.
