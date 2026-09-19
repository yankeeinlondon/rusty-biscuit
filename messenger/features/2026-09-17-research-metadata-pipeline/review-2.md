---
$schema: feature-review.yaml
ready: false
findings:
    - title: The accepted five-platform baseline and its generated publications still do not exist
      priority: high
    - title: Unbounded refresh intervals can produce invalid fixed-width dates
      priority: medium
human_review: true
human_review_items:
    - |-
        Complete and approve the first five-platform research publication after the remaining implementation finding in this review is fixed.

        The maintainer must:

        1. Choose an AI research agent and, if needed, its model.
        2. Set a maximum elapsed time and maximum number of agent launches for each platform. Either use one pair of limits for all five platforms or start with a small Discord run to measure realistic usage before setting the other limits.
        3. Run the reviewed workflow for Discord, Slack, Telegram, WhatsApp, and Signal.
        4. Inspect the evidence review and unresolved gaps, then provide the non-empty maintainer name that should be recorded as approving the initial publication.

        Approval should confirm that the catalog, cross-provider summary, skill projection, change history, and detailed platform documents agree with the reviewed evidence. This human decision is required by the specification; automated tests cannot replace it.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T03:04:33-07:00"
spec: 2026-09-17-research-metadata-pipeline/spec.md
implemented: true
implemented_by: claude/opus
log: messenger/features/2026-09-17-research-metadata-pipeline/implementation-log.md
description: "A **feature** review of `2026-09-17-research-metadata-pipeline/spec.md`"
feature: 2026-09-17-research-metadata-pipeline/review-2.md
previous: 2026-09-17-research-metadata-pipeline/review-1.md
next: 2026-09-17-research-metadata-pipeline/review-3.md
---

# Review 2: Research Metadata Pipeline

## Verdict

The feature is **not production ready**. The implementation closes the four
automatable high-priority defects from review 1, and the complete Messenger
Level 1 suite and lint pass. The feature's central deliverable is still absent,
however: no accepted five-platform research baseline has been produced, so the
generated catalog, cross-provider summary, skill projection, accepted review
history, and evidence-backed handoffs do not exist.

The review also found one bounded-input defect in the otherwise-correct date
fix. The roster schema accepts any positive `refresh_interval_days`, while the
`Date` representation and its ordering contract support only four-digit years.

## Review 1 closure

The previous review's four automatable findings were implemented:

- Empty/whitespace-only approvers and rejection inputs are rejected at the
  library boundary and by the real CLI; persisted records use the same
  validation.
- `Date::parse` now validates real proleptic-Gregorian dates, including leap
  years, and the CLI and renewal path have regressions.
- Unreadable run records now block their platform; preparation is serialized,
  and resuming an older failed run cannot create a second open run.
- Prepared inputs use repository-relative paths, reject a research root below
  a Git top level, and have real-CLI coverage for spaces, shell metacharacters,
  non-ASCII text, and platform-permitted newlines.

The baseline finding was not implemented because its prerequisites remain
blocked exactly as review 1 recorded: this non-interactive run has no selected
agent/model, per-platform budgets, or named human approver. None of those
blocked items was supplied or otherwise unblocked before this implementation.
The new passive publication guard is useful and correctly becomes strict once
a publication exists, but it is intentionally vacuous while the publication is
absent and therefore does not close the finding.

## Findings

### High — The accepted five-platform baseline and its generated publications still do not exist

The specification requires a reviewed initial baseline and generated
publications in deliverables 2–10 and acceptance criteria 2, 3, 9–12, and
15–22. The worktree still has no
`messenger/docs/research/publication.json`, generated `catalog.json`, generated
cross-provider summary, accepted review records, skill projection, or complete
evidence-backed handoffs. All five shipped platform documents remain legacy
prose without the research schema binding.

`just research-validate` reports one `SR-SCHEMA-BINDING` finding for each of
Discord, Signal, Slack, Telegram, and WhatsApp. `just research-check` exits 3
because no published snapshot exists. The Level 1
`shipped_publication_is_accepted_and_drift_free` guard returns no finding until
that same snapshot exists; its fixture tests correctly prove that it rejects
legacy or drifted content after publication, but it cannot verify an artifact
that has not been created.

Run the three research passes and independent evidence review for all five
platforms under explicit budgets, obtain the required named human approval,
promote the initial fleet together, and commit the complete generated snapshot.
This is reviewed research and approval evidence, not a Level 2 or Level 3
terminal test.

### Medium — Unbounded refresh intervals can produce invalid fixed-width dates

The roster schema allows `refresh_interval_days` to be any integer of at least
1 (`_types.yaml`), and the typed model accepts it as `u32`. Selection and
catalog projection pass that value directly to `Date::plus_days`. Adding a
sufficiently large but schema-valid interval to a four-digit date produces a
year of 10000 or later because the formatter's width is a minimum, not a
maximum.

That value violates `Date`'s documented `YYYY-MM-DD` fixed-width invariant.
Lexical ordering is no longer chronological, `Date::parse` cannot read the
projected value back, and methods that slice fixed byte offsets can interpret
the wrong fields. A schema-valid roster can therefore generate a catalog with
an invalid `refresh_due` or make refresh selection compare dates incorrectly.

Add an upper bound that guarantees `last_updated.plus_days(interval)` remains
within year 9999, and enforce it at the typed/semantic boundary as well as in
the schema. Prefer a checked date-addition result over constructing an invalid
`Date`. Add boundary tests for the largest accepted interval and the first
rejected overflow from a late year.

## Verification levels

| Requirement group | Strongest evidence present | Required level | Assessment |
| --- | --- | --- | --- |
| Roster mapping, schema and semantic rules, deterministic generation, recovery, deltas, reports, mappings, budgets, cleanup, decision validation, path handling, and CLI exit behavior | Level 1 unit, fixture, concurrency, and real-CLI subprocess tests | Level 1 | Passes, except for the refresh-interval bound above |
| Shipped five-platform metadata, generated publications, and evidence-backed handoffs | No accepted real-fleet artifact; the Level 1 guard is deliberately vacuous before first publication | Level 1 plus reviewed research evidence | **Gap** |
| Human approval of the initial substantive baseline | Not performed | Human decision | **Gap** |
| Terminal colors, glyph widths, scrolling, keyboard input, paste, IME, mouse, or hotkeys | Not asserted by this specification | Level 2/3 not applicable | No tier mismatch |

The CLI's human-readable output uses `TerminalRenderable` components, but the
specification does not promise exact terminal rendering or physical-input
behavior. No user-observable requirement is being claimed on Level 1 evidence
when Level 2 or Level 3 is required.

## Verification performed

- `messenger/ just test`: **682 passed**, 2 skipped.
- `messenger/ just lint`: passed for `messenger` and `messenger-cli`.
- `messenger/ just research-validate`: exit 1 with five schema-binding
  findings, one for each required platform document.
- `messenger/ just research-check`: exit 3 because
  `messenger/docs/research/publication.json` does not exist.
- Repository inspection confirmed that the generated skill projection and
  accepted publication manifest are absent.

Cross-OS execution evidence is intentionally not a readiness finding; CI/CD
owns that proof. The implementation and tests nevertheless use portable Rust
locking and path handling, with OS-specific filename cases gated where the
underlying filesystem forbids them.

## Ergonomics and performance

The maintenance feature remains isolated from ordinary send paths, uses
deterministic ordered collections appropriately, and introduces no blocking
performance concern. The revised relative-path design is simpler and safer
than shell-escaping an arbitrary checkout path. No additional ergonomic or
performance change is required for production readiness beyond the findings
above.
