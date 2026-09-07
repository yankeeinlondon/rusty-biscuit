---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T19:30:49+01:00
spec: suggestion-and-sidecar/spec.md
log: claudine/fixes/suggestion-and-sidecar/log.md
implemented: true
implemented_by: codex/default
next: suggestion-and-sidecar/review-2.md
description: A **fix** review of `suggestion-and-sidecar/spec.md`
fix: suggestion-and-sidecar/review-1.md
---

# Review 1: Suggestion and Sidecar

## Verdict

The fix is **not ready for production**. The suggestion recovery behavior and
the typed schema advisory are otherwise implemented cleanly and verified at the
appropriate level, but the production repository walker can consume 20,001
walker items despite the specification's exact 20,000-item work bound.

## Findings

### Medium — The real walker excludes its root before applying the visit budget

`repository_basename_suggestions` removes the walker's depth-zero root with
`filter_map` before passing the iterator to `collect_repository_suggestions`.
The collector then applies `.take(SUGGESTION_ENTRY_BUDGET)` to the remaining
items. Because `ignore::Walk` yields the root as its first item, the production
path can consume the root plus 20,000 subsequent items: 20,001 yielded walker
items in total.

This contradicts D1 and AC2, which say every walker item is a budgeted visit and
that item 20,001 must not be consumed. It also leaves a small hole in the stated
worst-case work bound. The new boundary test does not detect the mismatch
because it invokes the pure collector directly with an iterator that has no
depth-zero root.

Apply the budget to the raw walker before filtering or mapping the root, or
represent the root as a budgeted non-match. Add a real-seam test with an
instrumentable walker seam, or factor the depth-zero transformation so a test
can prove that the root, errors, and ordinary entries all consume the same
shared budget.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: an earlier dangling symlink does not suppress sorted, capped suggestions in the rendered diagnostic and structured detail | Level 1 real-filesystem Unix fixture plus in-process diagnostic rendering/detail assertions | Appropriate and present. No terminal-emulator behavior is claimed. |
| AC2: first/middle entry errors are skipped, errors consume budget, matches survive exhaustion, and unusable roots return no suggestions | Level 1 synthetic iterator tests and real-filesystem root tests | Functional cases are present, but the synthetic boundary test misses the production walker's depth-zero item; see the finding. |
| AC3: directory symlinks remain pruned | Level 1 platform-specific filesystem tests on Unix and Windows | Appropriate and present. |
| AC4: a simplified-looking bare sidecar remains raw JSON Schema and produces one advisory across one or two validation passes | Level 1 resolver, compose-report, and composed-document tests | Appropriate and present. |
| AC5: JSON Schema keywords, custom vocabularies, supported envelopes, scalar references, and invalid/mixed maps do not warn | Level 1 resolver and validation-entry-point matrices | Appropriate and present. |
| AC6: typed advisory identity reaches Darkmatter validation/compose, Claudine stderr, `md schema validate` pretty/JSON output, and the consuming DMLS `$schema` range without changing validity | Level 1 library, subprocess, and LSP protocol tests | Appropriate and present. The behavior is data/diagnostic projection and does not require a real terminal or OS input. |
| AC6: `--silent`/`--quiet` suppress successful warning output | Level 1 subprocess tests | Appropriate and present. |
| AC7: macOS, Windows, and Linux compatibility | Level 1 package suites on macOS and native `claudine-cli` suites on Windows and Linux | Appropriate and present for the changed cross-platform production path. |

Levels 2 and 3 are not applicable. This fix does not depend on terminal
emulator rendering, glyph width, styling fidelity, scrolling, or physical input
encoding. Its human-readable outputs are adequately exercised as plain captured
Level 1 output.

## Additional Review Notes

The sidecar classifier uses the existing passive SimplifiedSchema parser rather
than duplicating its grammar, reserves Draft 2020-12 keywords and `$`/`x-`
vocabularies, and leaves raw JSON Schema behavior unchanged. Advisory collection
is typed, sorted, and deduplicated before consumer projection. Compose report
merging deduplicates only schema advisories, preserving existing duplicate
semantics for unrelated warnings. DMLS assigns the required source, code,
severity, and consuming `$schema` value range and invalidates its cached result
when the referenced sidecar changes.

No additional correctness, ergonomics, performance, or test-level mismatch was
found.

## Verification Performed

- `claudine/ just test`: **6,716 passed; 11 skipped**.
- `darkmatter/ just test`: **7,633 passed; 50 skipped**.
- `claudine/ just lint`: **passed** for all five Claudine packages and the
  diagnostic/documentation guards.
- `darkmatter/ just lint`: **passed** for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- Native Windows `claudine-cli` Level 1 cross-check: **2,069 passed; 8
  skipped**.
- Native Linux `claudine-cli` Level 1 cross-check: **2,431 passed; 9 skipped**.
- `git diff --check`: **passed** before writing this review.

GitNexus reports a **high** upstream blast radius for
`collect_repository_suggestions` (one direct caller, six affected symbols), a
**low** radius for `parse_yaml_referenced_file` (one direct caller, three
affected symbols), and a **critical** radius for the central
`EffectiveSchema::validate_instance` API (two direct callers, 93 affected
symbols). No indexed execution flows were reported for these symbols. The full
Claudine and Darkmatter package gates above cover the broad validation surface.

The repository's `just cross-check` launcher initially failed locally because
macOS `/bin/bash` 3.2 does not support its associative-array declaration. The
same helper was run with Homebrew Bash for Windows. Linux then encountered a
non-writable shared Cargo-cache artifact, so the already-patched remote checkout
was rerun with an isolated writable `CARGO_TARGET_DIR`; that full suite passed.

## Production Readiness

AC1 and AC3–AC7 are satisfied with appropriately leveled verification. AC2 is
not satisfied at the production walker seam, so the fix should remain out of
production until the 20,000-item bound includes the depth-zero walker item and a
test covers that exact real-path accounting.
