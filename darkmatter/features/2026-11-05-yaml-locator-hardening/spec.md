---
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
kind: feature
created: 2026-11-07
status: draft-spec
area: darkmatter
reviewed: false
clarified: false
implemented: false
review_iterations: 0
packages:
    - simplified-schema
    - darkmatter
related:
    - 2026-11-04-loading-data
depends-on:
    - 2026-11-04-loading-data
---

# YAML Locator Hardening

Let a YAML data file that uses an alias, or writes a flow collection across
several lines, report exact lines and columns for `load_data` contract
violations, instead of falling back to a JSON Pointer alone for the whole
file. This removes the one pointer-only case 2026-11-04-loading-data left in
its diagnostics.

## Motivation

Every other `load_data` format reports a line and column for each contract
violation. YAML does too, except that one alias anywhere in a file, or one
multi-line `[…]`/`{…}`, removes every position from that file's report, even
for values far from the alias or the flow collection. Authors then have to
find `/servers/12/port` by hand.

## Current behavior

Verified against the code on 2026-11-07:

- **Positions come from `PositionMap`.** For YAML, Darkmatter's
  `compose/expression/data/locate.rs` asks simplified-schema's
  `PositionMap::byte_range` (built by `load_yaml_document`) where a pointer's
  value starts, and never uses its top-level-keys fallback, whose column-1
  positions are guesses.
- **An alias refuses the whole file.** `PositionMap::project`
  (`simplified-schema/lib/src/positions.rs`) returns `None` when any scalar
  in the document is an alias token (`matches_orders` checks
  `alias_token`), so no value of the file is located. A merge key
  (`<<: *b`) is refused the same way, because the merged mapping's keys do
  not match the authored ones. The positions tests
  `unsupported_yaml_keeps_the_top_level_fallback` and
  `an_alias_under_a_list_root_still_refuses_projection` pin this.
- **Multi-line flow collections are refused by Darkmatter, not by the
  locator.** The 2026-11-04-loading-data spike measured the locator giving
  up on them, but `PositionMap` now locates values inside them
  (`a_multiline_flow_collection_keeps_every_position_in_the_document`), and
  frontmatter already gets those positions. Darkmatter drops all positions
  for such a data file on purpose (`has_multi_line_flow_collection` in
  `data/locate.rs`), to keep the pointer-only rule its spec stated, and the
  test `only_a_flow_collection_across_lines_makes_yaml_pointer_only` pins it.
- **Ordering survives.** Without positions, records are still ordered by
  each value's authored index, so the report order is right; only line and
  column are missing.

## Scope

1. **Aliases (about half a day).** Let `matches_orders` skip the subtree
   below an alias token instead of refusing the document, so every value
   outside an alias keeps its exact position. A pointer that passes through
   an alias is located at the alias token itself (`*b`), which is where the
   author wrote that value; `locate_range_in` already stops a path at a
   scalar's start.
2. **Multi-line flow collections (less than the spike's one-day estimate,
   because the locator already handles them).** Remove
   `has_multi_line_flow_collection` and its pointer-only branch from
   `data/locate.rs`, and prove the locator with `load_data` fixtures: a
   multi-line flow sequence of objects, a nested multi-line flow mapping,
   and comments and quoted `]`/`}` inside the collection.
3. **Frontmatter.** Frontmatter validation problems use the same
   `PositionMap`. A frontmatter block with an alias gains exact positions
   for its other values; keep every existing frontmatter position test
   passing, and add one alias case.
4. **Docs.** The `darkmatter/docs/` loading-data page and
   `docs/cli/data-contracts.md` no longer describe a pointer-only YAML case
   for aliases or multi-line flow collections.

## Non-goals

- A second YAML parser. The spike tried `yaml-rust` (unmaintained,
  RUSTSEC-2024-0320, and a second grammar beside libyaml) and rejected it.
- Positions for a duplicate key (reported at its mapping's start), a second
  YAML document, or a non-string key, which neither the parser nor the
  position map locates today.
- Changing the top-level fallback for callers that already use it.

## Open questions

- **Merge keys.** Whether a value reached through `<<: *b` is located at the
  `<<` entry's alias token, like any other alias, or whether merge keys stay
  pointer-only for the mapping that holds them.
- **Anchored definitions.** A violation inside the anchored original
  (`base: &b {port: x}`) is reported once for each alias that reuses it,
  because each alias is its own pointer. Confirm that each of those records
  should point at its alias token rather than all at the anchor.

## Acceptance criteria

- In `base: &b {port: 1}\nservers:\n  - *b\n  - {port: x}\n` read with
  `"{ base: { port: number }, servers: { port: number }[] }"`, the violation
  at `/servers/1/port` reports line 4 and the column of `x`.
- A violation at a pointer that passes through an alias reports the alias
  token's line and column.
- A YAML data file with a multi-line flow sequence reports exact positions
  for violations inside it, and for violations after it.
- A frontmatter block with an alias reports exact positions for problems in
  its other values.
- The simplified-schema positions tests are updated only where they assert
  the old whole-file refusal; every other position test passes unchanged.
- No `load_data` diagnostic for YAML is pointer-only except for the
  non-goals listed above.
