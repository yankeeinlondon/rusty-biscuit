---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: A null npm link target can falsely corroborate a workspace
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T12:13:57-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-8.md
previous: 2026-09-26-lockfile-corroboration/review-7.md
next: 2026-09-26-lockfile-corroboration/review-9.md
---

# Review 8 — Lockfile corroboration

## Verdict

**Not ready for production.** Review 7's sole unblocked finding is implemented, and it had no blocked findings to reassess. One related npm parser defect remains: an invalid link target can be ignored while the library reports an exact lockfile match.

## Previous-review disposition

The Sniff library's [npm lockfile parser](../../lib/src/filesystem/repo/lockfile/npm.rs) now distinguishes an omitted root `workspaces` field from explicit `null`. A present `null` fails parsing. The [Level 1 public-result test](../../lib/tests/l1/lockfile_fixtures.rs) checks both a workspace and a root-only lockfile, the full `unreadable`/`parse_failed` observation, the parse counter, and unchanged provenance. The focused `just test null` run passed. This resolves review 7's finding.

## Unblocked Findings

### High — A null npm link target can falsely corroborate a workspace

The Sniff library's [npm lockfile parser](../../lib/src/filesystem/repo/lockfile/npm.rs) treats `resolved` on a `link: true` package record as `Option<String>`. Serde maps both an omitted field and explicit JSON `null` to `None`; the parser then drops that link target. A link target is membership evidence, so a present target with an invalid type is an invalid required membership field. The spec requires `unreadable` with `parse_failed` for that case. Silently dropping it can also produce a false `mismatch` when the link is the only record for a member.

I copied the npm 11.6.4 workspace fixture to a disposable repository and changed only `packages["node_modules/@fixture/beta"].resolved` to `null`. The shipped `sniff repo structure --json` exited successfully with empty stderr and reported `match`, with empty differences and a null reason. The other `packages/beta` record still supplies that member path, so the invalid link is masked and the exact match incorrectly upgrades provenance. Validate a present `resolved` on `link: true` records, including explicit `null`, before comparing members. Add a Level 1 parser case and a public-result fixture test that checks `unreadable`/`parse_failed` and no provenance upgrade.

## Verification level by requirement

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Lockfile statuses, member differences, and provenance | Level 1 parser and complete-result fixture tests | The right level for repository data. The invalid npm link-target case is missing and can produce a false `match`. |
| Standalone Poetry, PDM, and Composer observations | Level 1 library and shipped CLI tests | Appropriate for repository data. |
| JSON stdout, plain output, stderr separation, and success exit status | Level 1 shipped CLI tests | Appropriate for noninteractive commands. |
| Styled lists, wrapping, and status colors | Level 2 capture of the shipped CLI in tmux | Appropriate for real-terminal rendering; the focused test passed. |
| Keyboard, mouse, paste, or input encoding | No such feature requirement | Level 3 is unnecessary. |

## Review checks

I read the spec, review 7, the lockfile observation and parser code, the complete-result and CLI tests, and the declared test targets. `just check-tier-coverage sniff` found no stranded tests. `just test null` passed 11 focused Level 1 tests, including the new parser and public-result cases. `just test-l2 level2_lockfile` passed the tmux rendering test. I reproduced the link-target result through the shipped CLI on a disposable copy of a real-tool fixture. Cross-OS CI evidence and human approval are excluded from readiness as requested.

## Human review

No human decision is needed. The spec already defines the classification for invalid required membership fields.
