---
area: claudine
status: implemented
created: 2026-10-20
origin: 2026-10-20-handle-indented-frontmatter
packages:
    - darkmatter
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-21
review_iterations: 0
implemented: true
implemented_by: claude/opus
---

# Handle-indented-frontmatter follow-ups

## Purpose and current state

Make Darkmatter's formatting guide accurately explain how cleanup handles
tab-indented YAML frontmatter, the metadata block at the start of a Markdown
document. This follow-up originated in an observation from the first
implementation review of `2026-10-20-handle-indented-frontmatter`, which
reported no findings blocking that fix.

**Defect, as drafted:** the guide's `frontmatter` section listed tab
indentation among the frontmatter that cleanup "keeps exactly as written" and
does not repair. Cleanup actually recovers tab indentation, as the
[`md clean` repair reference](../../../../darkmatter/docs/cli/clean.md#frontmatter-repair-and-schema-precedence)
and the tests named below show. The statement predated the origin fix, which
did not change it.

**Implemented, 2026-10-21:** commit `7d648be04` removed tab indentation from
the "kept as written" examples and added a tab-recovery paragraph linking the
repair reference. The 2026-10-21 spec review found that wording met the
requirements below; it was then scoped to repair being on, so it cannot be
read as overriding `frontmatter: false`.

## Scope and design decision

This is a documentation correction to Darkmatter's
[formatting guide](../../../../darkmatter/docs/topics/markdown-formatting.md#frontmatter).
It adopts the existing cleanup behavior; it introduces no change to YAML
acceptance, repair safety, command output, or editor formatting. No Claudine
code change is required despite this follow-up being filed in that area.

The guide must explain these cases:

- With frontmatter repair enabled, each indentation tab becomes two spaces,
  matching how Darkmatter reads that indentation. This does not mean replacing
  tabs everywhere in a document or inside scalar values.
- After tab recovery, cleanup can apply further safe repairs in the same run.
  In particular, `yaml.root-indentation` moves an eligible indented root
  mapping to column zero while preserving its parsed value. Do not imply that
  every YAML root form receives this repair. Link to the
  [`md clean` repair reference](../../../../darkmatter/docs/cli/clean.md#frontmatter-repair-and-schema-precedence)
  for eligibility and safety details rather than duplicating its catalog.
- Supported bare expression values such as
  `review: {{ ctx.area }}/review.md` remain the example of frontmatter that
  strict YAML rejects but Darkmatter reads. Cleanup preserves that block,
  reports the YAML finding as a suggestion, and formats the body. Do not put
  tab indentation back into this no-repair category.
- Preserve the adjacent `frontmatter: false` contract: the block stays
  byte-for-byte unchanged and findings remain report-only. If Darkmatter
  cannot read the block without a repair, cleanup stops with an error. The
  tab-recovery paragraph must not imply that it overrides this setting.

These requirements describe the same shared cleanup behavior used by the
`md clean` command and the Darkmatter language server's document formatting.
They do not promise successful cleanup when another part of the document
causes cleanup to refuse an edit.

## Acceptance and verification

1. Read the guide's `frontmatter` section and confirm the distinction between
   tab recovery and preserved bare expressions, the two-space conversion,
   the repair-reference link, and the repair-disabled contract above.
2. Check the wording against the existing cleanup tests in
   [Darkmatter's frontmatter repair suite](../../../../darkmatter/lib/tests/l1/clean_frontmatter_repair.rs)
   and the language server's
   [formatting transaction suite](../../../../darkmatter/dmls/tests/l1/formatting_transaction.rs).
   Both contain a tab-indented root example that is recovered and dedented
   in one run; both verify that a second cleanup offers no further change.
3. If the requirements are already met, record that result without making a
   cosmetic documentation edit. If wording has regressed, change only the
   affected explanation and retain the existing link to the repair reference.
   Current documentation must not link back to this fix or its predecessor.
4. For any resulting edit, check the diff for whitespace errors and confirm
   that the relative documentation link resolves to the intended section.

This review inspected the current guide, repair reference, cleanup source,
and existing test assertions; it did not rerun the tests. The guide already
meets the documentation requirements. No new automated tests, build, lint,
performance sample, or additional OS runs are required for this prose-only
scope. A code discrepancy discovered later would require a separately scoped
behavior fix rather than changing runtime behavior under this specification.

## Open Questions

None. The documentation contract follows existing behavior, and the requested
correction is already present. The author retains responsibility for closing
this follow-up after review.
