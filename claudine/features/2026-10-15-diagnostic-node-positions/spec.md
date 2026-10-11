---
area: darkmatter
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 5
related:
    - 2026-10-17-content-policy-base-schema
---

# Validation Diagnostics Point at the Offending Node

A frontmatter validation problem already carries the JSON path of the node
that failed (for example `/start/0/then/0`), but its `line` and `column` are
those of the **top-level key** (`start:` on line 2), not of the offending list
item or nested field (line 5). In a long nested list the author has to count
entries to find the problem. After this change `md schema validate` and
`md compose` report the line and column of the offending node itself.

DMLS already ranges these diagnostics on the offending node (it walks its
frontmatter syntax tree by path); this spec brings the CLI and composition
error reports to the same position and keeps DMLS in agreement.

## Decided

- A `ValidationProblem`'s `line`/`column` are the position of the node its
  instance path names, wherever that node's source position is known:
  - type, constraint, file-reference, and other value failures: the start of
    the offending value (a list item, a nested field's value);
  - unknown key: the offending key;
  - missing required key: the start of the mapping that lacks it (the
    parent object, which may be a list item);
  - a problem whose path cannot be located in the source (for example a value
    produced by an override layer or interpolation, or a block scalar the
    locator does not project): the nearest located ancestor, falling back to
    the top-level key as today, and finally to no position.
- The JSON path, message, code, and every other field are unchanged. Only the
  position becomes more precise.
- The same positions appear in every surface that shows a validation problem
  with a location: `md schema validate` (human and `--output json`),
  composition's schema-validation error block, and DMLS. DMLS ranges stay as
  they are; the requirement is that the CLI line/column equals the start of
  the DMLS range for the same problem.

## Requirements

- Replace the top-level-key-only position lookup (`build_position_map` /
  `PositionMap` in `darkmatter/lib/src/markdown/schemas/validate.rs`, used by
  `build_problem`) with a lookup by instance path over located frontmatter
  nodes. Reuse the existing frontmatter source locator
  (`locate_frontmatter_value` in `schemas/simplified/source.rs`) rather than
  writing a new YAML scanner. `PositionMap` is public; if its shape changes,
  update its callers (`md schema validate`, composition's schema validation,
  `hash/write.rs`, `errors/blocks.rs`, DMLS) in the same change.
- Update the `md schema validate` JSON example in
  `darkmatter/docs/topics/schemas/definition.md` (~:1295) and any prose that
  says positions are those of the top-level key.

## Out of scope

- DMLS range rules (already node-precise).
- Positions inside schema documents: `2026-10-11-schema-loader-yaml-spellings`.

## Acceptance criteria

1. The spike case: a typo'd key at `/start/0/then/0` reports the line and
   column of the offending item (line 5 in the fixture), not `line 2,
   column 1`, in `md schema validate --output json` and in the human report.
2. A wrong-typed item in a list (`tags: [1, oops, 3]` in block form) reports
   the line of `oops`; a missing required field inside the second list item
   reports the line of that item; an unknown nested key reports the key's
   line and column.
3. `md compose` on a document with the same problem shows the same line and
   column in its schema-validation error.
4. A parity test asserts that for each problem in criteria 1-2 the CLI
   line/column equals the start of the DMLS diagnostic range (after
   converting the CLI's 1-based line/column to LSP's 0-based position).
5. A problem on a value that has no source node (an override-layer value)
   still reports the top-level key's position.
6. `just test` and `just lint` pass in `darkmatter/`. Tests are L1 (CLI tests
   use `CliProcessFixture`), use no terminal or browser windows, and are
   portable to macOS, Linux, Windows, and WSL2. No new CI cells.

## Open questions

None.
