---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T20:55:25-07:00
spec: 2026-07-16-performance/spec.md
implemented: true
implemented_by: claude/default
log: sniff/features/2026-07-16-performance/log.md
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-11.md
---

# Review 11

## Findings

### High: native Linux and Windows execution and matched work-count artifacts remain absent

The production completion boundary still requires native macOS, Linux, and Windows correctness plus
comparable work-count artifacts from the scheduled matrix ([spec.md](spec.md#ci),
[spec.md](spec.md#acceptance-criteria)). The current implementation record explicitly defers the
Linux and Windows runs and the matched artifact set
([deferred-perf-tests.md](deferred-perf-tests.md#review-10-deferred-items)). The pre-review tree is
`c2a188379d1be770bfa3638f412552cb05310839`, and `git branch -r --contains` finds no fetched remote
branch containing it. A workflow definition is future coverage, not an execution record for this
implementation.

The macOS Level-1 suites are green. That verifies the platform-independent request, projection,
counter, and CLI contracts locally, but it does not execute Linux `/proc` descendant handling or
Windows Job Object, registry, path, and service behavior. Publish one immutable final implementation
identifier, retain green native Level-1 results on macOS, Linux, and Windows, and retain all three
`sniff-work-counts-{os}` artifacts for that same identifier.

### Medium: deleting the completed phase records broke live design and performance-evidence references

Commit `c2a188379` deleted all eight phase specifications and the Phase 1 log rather than moving
them under `_completed`. The deletion leaves live references to nonexistent files in the required
Sniff skill ([SKILL.md](../../../.claude/skills/sniff/SKILL.md)), the scheduled work-count workflow
([sniff-performance.yml](../../../.github/workflows/sniff-performance.yml)), production rustdoc
([snapshot.rs](../../lib/src/remote/snapshot.rs), [process.rs](../../lib/src/process.rs), and
[discovery.rs](../../lib/src/filesystem/git/discovery.rs)), and the still-active execution plan
([plan.md](plan.md)). Prior reviews also now link to missing evidence.

This is not only navigational drift. The deleted Phase 3, 4, 7, and 8 records held the corrected
counter baselines, the profile-guided keep/defer table, and the cross-platform completion record
that the skill tells future performance work to use. The umbrella specification does not reproduce
those details, and Git history is not an acceptable replacement for a live authoritative document.
Restore the phase records, preferably by archiving the completed phase tree under the feature
lifecycle's `_completed` area, and update every live reference to the archived paths. If deletion is
intentional, first migrate the full authoritative evidence into another tracked document and update
all references in the same change.

## Resolved Since Review 10

- Playa now has a composed Level-1 command-boundary regression. It drives a timed-out interview
  through the delegate, verifies the warning reaches captured output, and asserts a non-zero command
  verdict (`install_command_timeout_warns_and_returns_nonzero_verdict`).
- The Sniff skill and public CLI documentation now consistently describe ten detectable program
  categories, eight installable categories, and notification helpers/test runners as report-only.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| R1-R14 request scoping, work reduction, inventory completeness, Git bounds, remote reuse, NTP gating, service batching, ownership, and serialized result contracts | Level 1 unit, integration, fixture, work-counter, and spawned-CLI tests on macOS | Appropriate functional tier and green locally; the specification's native Linux/Windows execution requirement remains unverified. |
| Aggregate JSON schema, valid-JSON-only stdout, one status walk/discovery, ordering, and offline projection | Level 1 spawned-CLI tests, snapshots, and work-counter assertions | Appropriate tier and green locally. |
| Installation timeout warning and non-zero terminal verdict in Sniff and Playa consumers | Level 1 interview/event and composed command-boundary tests | Appropriate tier and green locally; exact SGR styling is not part of this contract. |
| Linux `/proc` descendant behavior and Windows Job Object, registry/path, and service behavior | Native macOS Level 1 plus previously recorded Windows target cross-compilation | Insufficient for the required native three-OS execution. |
| CLI glyph widths, SGR styling, and scrolling | No changed presentation-specific contract | No new Level-2 requirement. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages --json
  succeeded; confirmed sniff and sniff-cli workspace packages

just test  # sniff/
  sniff: 1,677 passed, 19 skipped
  sniff-cli: 782 passed, 3 skipped

just lint  # sniff/
  passed on macOS

just test  # playa/
  playa: 50 passed
  playa-cli: 19 passed

just lint  # playa/
  passed on macOS

git branch -r --contains c2a188379d1be770bfa3638f412552cb05310839
  no fetched remote branch contains the reviewed pre-edit tree

live-reference audit
  found deleted phase links in the Sniff skill, CI workflow, production rustdoc,
  active plan, and prior reviews
```

The implementation's local Level-1 behavior is green, and review 10's Playa and category-documentation
findings are resolved. The required native Linux/Windows execution and matched work-count artifacts
are still absent, and the completed phase evidence was deleted while live consumers still depend on
it. Production readiness: **not ready**.
