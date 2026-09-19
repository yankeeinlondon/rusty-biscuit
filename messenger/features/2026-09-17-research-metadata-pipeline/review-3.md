---
$schema: feature-review.yaml
ready: false
findings:
    - title: The accepted five-platform baseline and its generated publications still do not exist
      priority: high
human_review: true
human_review_items:
    - |-
        Complete and approve the first publication for Discord, Slack, Telegram, WhatsApp, and Signal.

        Before an automated research run can start, choose:

        1. The AI research agent and, if needed, its model.
        2. A maximum elapsed time and maximum number of agent launches for each platform. Either use one pair of limits for all five platforms, choose limits per platform, or begin with a small Discord run and use its measured consumption to set the remaining limits.
        3. The maintainer name to record when the completed research is approved.

        After the runs finish, inspect the cited evidence, unresolved questions, and proposed source-list changes. Approve the initial publication only if the five detailed documents, generated catalog, cross-provider summary, skill summary, change history, and implementation handoffs all agree with that evidence. The specification requires this human approval; automated validation cannot replace it.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T09:16:05-07:00"
spec: 2026-09-17-research-metadata-pipeline/spec.md
implemented: false
description: "A **feature** review of `2026-09-17-research-metadata-pipeline/spec.md`"
feature: 2026-09-17-research-metadata-pipeline/review-3.md
previous: 2026-09-17-research-metadata-pipeline/review-2.md
---

# Review 3: Research Metadata Pipeline

## Verdict

The feature is **not production ready**. The implementation fully closes the
unblocked refresh-date finding from review 2, and Messenger's Level 1 suite,
lint, and all-features compile check pass. The feature's central reviewed
artifact remains absent, however: no accepted five-platform research baseline
or generated publication exists.

## Review 2 closure

Review 2 did not use literal `Unblocked Findings` and `Blocked Findings`
headings, so this review classified its medium implementation finding as
unblocked and its high baseline finding plus human-review prerequisites as
blocked.

The unblocked finding, **Unbounded refresh intervals can produce invalid
fixed-width dates**, is implemented:

- Both roster schemas cap the default and per-platform interval at 3,660 days.
- The typed SR-ROSTER validator independently enforces the same range.
- `Date::checked_plus_days` refuses dates after `9999-12-31`.
- Document validation reports an overflowing due date, selection classifies
  the platform as schema-invalid rather than current or expired, and catalog
  projection independently refuses to serialize the invalid date.
- Level 1 tests cover the maximum interval, the first interval above it, the
  last representable due date, the first overflowing date, selection, schema
  fixtures, typed validation, and projection.

The blocked finding, **The accepted five-platform baseline and its generated
publications still do not exist**, was not unblocked before the implementation.
No agent/model, per-platform time and invocation limits, or named approver was
provided. Those inputs are intentionally required before research starts, and
the initial substantive publication requires human review. The implementation
therefore could not close this finding in the non-interactive session.

## Findings

### High — The accepted five-platform baseline and its generated publications still do not exist

The specification requires a reviewed initial baseline and generated
publications in deliverables 2–10 and acceptance criteria 2, 3, 9–12, and
15–22. The worktree still has no
`messenger/docs/research/publication.json`, generated `catalog.json`, accepted
review records, generated cross-provider summary, or
`.claude/skills/messenger/platform-metadata.md`. All five shipped platform
documents remain legacy prose without the research schema binding.

`just research-validate` reports one `SR-SCHEMA-BINDING` finding for each of
Discord, Signal, Slack, Telegram, and WhatsApp. `just research-check` exits 3
because no published snapshot exists. The passive Level 1 publication guard
continues to be deliberately vacuous before the first publication; its fixture
tests prove the post-publication behavior but cannot verify artifacts that do
not exist.

Choose the research agent and explicit budgets, run the three research passes
and independent evidence review for all five platforms, obtain the required
named human approval, promote the initial fleet together, and commit the
complete generated snapshot. This is reviewed research evidence and generated
file verification, not a Level 2 or Level 3 terminal test.

## Verification levels

| Requirement group | Strongest evidence present | Required level | Assessment |
| --- | --- | --- | --- |
| Roster interval bounds, checked date arithmetic, validation, selection, catalog refusal, schemas, and CLI behavior | Level 1 unit, fixture, and real-CLI subprocess tests | Level 1 | Passes |
| Remaining deterministic research pipeline behavior | Level 1 unit, fixture, concurrency, fault-injection, and real-CLI subprocess tests | Level 1 | Passes for implemented behavior |
| Shipped five-platform metadata, generated publications, and evidence-backed handoffs | No accepted real-fleet artifact; the Level 1 guard is deliberately vacuous before first publication | Level 1 plus reviewed research evidence | **Gap** |
| Human approval of the initial substantive baseline | Not performed | Human decision | **Gap** |
| Terminal colors, glyph widths, scrolling, keyboard input, paste, IME, mouse, or hotkeys | Not asserted by this specification | Level 2/3 not applicable | No tier mismatch |

The CLI uses terminal-renderable components, but this specification does not
promise exact terminal rendering or physical-input behavior. Its observable
contracts are file contents, validation results, command output, and exit
statuses, all of which are correctly exercised at Level 1.

## Verification performed

- `messenger/ just test`: **687 passed**, 2 skipped.
- `messenger/ just lint`: passed for `messenger` and `messenger-cli`.
- `cargo check -p messenger --all-features --all-targets --color=never`:
  passed, covering the maintenance feature together with all provider feature
  combinations at compile time.
- `messenger/ just research-validate`: exit 1 with five schema-binding
  findings, one for each required platform document.
- `messenger/ just research-check`: exit 3 because
  `messenger/docs/research/publication.json` does not exist.
- Repository inspection confirmed that the publication manifest, catalog,
  accepted review records, and skill projection are absent.

Cross-OS execution evidence is intentionally not a readiness finding; CI/CD
owns that proof. Level 2 and Level 3 tests are not required because this feature
asserts no real-terminal rendering or OS-input behavior.

## Ergonomics and performance

The maintenance dependency graph remains isolated from ordinary message sends.
The date fix uses checked arithmetic once per platform and preserves stable,
deterministic diagnostics; it introduces no material performance cost. No
additional ergonomic or performance finding is warranted.
