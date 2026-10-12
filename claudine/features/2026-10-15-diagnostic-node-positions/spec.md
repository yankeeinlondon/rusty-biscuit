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
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 0
human_review: false
message_to_agent: |-
    Phases 2 and 3 are done; see "## Phase 2" and "## Phase 3" in the
    implementation log. Phase 4 is verification and handoff only. Notes:
    (1) Shared fixture `darkmatter/lib/tests/position_fixtures/mod.rs` is
    included by the lib L1, CLI L1, and DMLS unit tests; it is listed in the
    cli and dmls `source-inputs`.
    (2) DMLS hides `MissingRequired` diagnostics unless `schema.strict` is on
    (existing behavior, unchanged), so missing-field parity is tested in
    strict mode only; DMLS starts equal library byte offsets everywhere.
    (3) The only production change in Phase 3 is the CLI's human location
    text `(at line L, column C)`. The compose block already rendered `at L:C`.
    (4) Downstream consumers of `PositionMap`: none outside darkmatter (Phase 1
    inventory); `darkmatter/` and `claudine/` just test + just lint passed on
    macOS at the end of Phase 3.
    (5) `darkmatter/README.md`'s `hash:` was already stale before this feature.
pipeline_step: 5
related:
    - 2026-10-17-content-policy-base-schema
---

# Validation Diagnostics Point at the Offending Node

A frontmatter validation problem already carries a JSON Pointer identifying
the failing value, such as `/start/0/then/0`, but its source location usually
points at the **top-level key**. In a long nested list, the author has to count
entries to find the problem. Some missing-field errors have no position at all.
After this change, `md schema validate` and `md compose` report the line and
column of the offending node when its authored position is available.

This is a Darkmatter change, stored here as part of Claudine's rollout plan.
It changes diagnostic positions, not which documents pass validation.
Darkmatter's language server, DMLS, already highlights values, unknown keys,
and mappings with missing fields. The command-line reports will identify the
same source start for ordinary located nodes without changing editor ranges.

> **Reader's note:** An unknown-key error's pointer identifies the parent
> mapping; its separate `offending_property` field names the key. A missing-key
> error also identifies the parent because the missing field has no source
> node. Position lookup must use these existing fields rather than changing
> the pointer or interpreting the human-readable message.

## Decided

- In Darkmatter, [ValidationProblem](../../../darkmatter/lib/src/markdown/schemas/mod.rs)
  carries a validation error's identity and optional source location. Its
  `line`/`column` locate the source selected by its error category:
  - type, constraint, file-reference, and other value failures: the start of
    the offending value (a list item, a nested field's value);
  - unknown key: the offending key;
  - missing required key: the start of the mapping that lacks it (the
    parent object, which may be a list item);
  - document-root failures, including a missing root property: the start of
    the root mapping when located;
  - a problem whose path cannot be located: the nearest located non-root
    ancestor's value, then the original top-level key, then no position. A
    newly supplied top-level property with no authored key has no position.
- Source positions refer to the original frontmatter, even when composition
  or a CLI assignment replaces a value. If the same path was authored, use
  its original node; if only an ancestor was authored, use that fallback.
  This is an editing location, not a claim that the replacement was written
  there. Do not introduce new tracking of where generated values came from.
- A literal or folded block scalar is one value node. Point at its scalar
  start, not an inferred position inside its decoded contents. Apply the
  same rule to quoted scalar content coerced into JSON or YAML objects.
  Descendant paths inside decoded content fall back to the authored scalar.
- The JSON path, message, code, and every other field are unchanged. Only the
  position becomes more precise. Keep the existing number and order of
  problems, including union-arm selection and errors listing several unknown
  keys. Use the existing selected `offending_property` for those errors.
- CLI and library coordinates are 1-based; columns count UTF-8 bytes from
  the beginning of the line, not terminal display cells. Parsed Markdown
  locations count lines in the complete file, including the opening `---`.
  The low-level YAML map builder counts lines in its supplied YAML slice.
- The human `md schema validate` report must show both line and column when
  both exist. JSON output and composition error blocks use the same position.
  Preserve JSON field names, null handling, and exit codes.
- DMLS ranges remain unchanged. For supported source nodes, CLI/library and
  DMLS positions must identify the same byte offset. DMLS converts that offset
  into the editor's negotiated UTF-8 or UTF-16 encoding. Subtracting one from
  the CLI column is sufficient only for UTF-8 editor positions; UTF-16
  requires conversion using the source line.

## Requirements

- In Darkmatter, replace the top-level-key-only lookup in
  [build_problem](../../../darkmatter/lib/src/markdown/schemas/validate.rs),
  which constructs validation problems, with category-aware lookup over
  located frontmatter nodes. Keep key and value positions distinct; one
  coordinate per pointer cannot represent both unknown-key and value errors.
- Reuse Darkmatter's
  [locate_frontmatter_value](../../../darkmatter/lib/src/markdown/schemas/simplified/source.rs),
  which returns authored byte spans and decoded mapping keys. Walk decoded
  [JsonPointer](../../../darkmatter/lib/src/markdown/schemas/mod.rs) segments
  to locate the value. Mapping keys such as `"0"`, `"a/b"`, `"a~b"`, and `""`
  remain keys; interpret a segment as an array index only at a sequence node.
- Build the located tree and line-start index once per source snapshot and
  reuse them for all problems, including union arms. Do not parse the YAML
  again or rescan the document for every problem. Position lookup is passive:
  it does not evaluate expressions, resolve files, or mutate frontmatter.
- Source projection is best effort. The existing locator is not a complete
  YAML parser, and an unsupported presentation can prevent projection of the
  whole block. Retain the existing top-level key scan as a fallback in that
  case; valid YAML must not become a validation or composition failure merely
  because precise locations are unavailable. Never index source nodes that
  cannot be matched to the semantic structure. Aliases, merge keys, and other
  unsupported forms need no new nested-position guarantees in this change.
- Preserve offsets in the original bytes, including CRLF line endings. The
  original raw source is preferred over serialization. Programmatically built
  Markdown without raw source retains its current serialized-view positions;
  they do not claim to locate a physical source file.
- Darkmatter's public
  [PositionMap and build_position_map](../../../darkmatter/lib/src/markdown/schemas/validate.rs)
  currently expose a top-level-key map. Changing that representation is
  permitted, but update all repository construction and lookup sites and their
  docs together. Preserve an empty-map construction for source-free validation.
  Update [positions_for](../../../darkmatter/lib/src/markdown/schemas/mod.rs),
  which applies the Markdown delimiter offset, so key, value, root, and
  fallback positions each receive that offset exactly once. Search actual
  callers rather than assuming consumers of the source locator also consume
  the position map: the hash writer and interpolation code share the locator
  and must keep their existing behavior.
- Update Darkmatter CLI's
  [format_location](../../../darkmatter/cli/src/commands/schema/validate.rs),
  which currently omits columns because all recorded columns were 1. Review
  composition's [error blocks](../../../darkmatter/lib/src/markdown/errors/blocks.rs)
  for excerpt and caret behavior with nested coordinates. Render through the
  existing biscuit-terminal components.
- Update the examples and coordinate/assignment explanation in
  [Schema Definition](../../../darkmatter/docs/topics/schemas/definition.md),
  the [Darkmatter README](../../../darkmatter/README.md), and the
  [schema skill reference](../../../.claude/skills/darkmatter/schema.md)
  where they describe affected behavior. Review docs and inline comments on
  changed symbols, particularly the current top-level-only descriptions.

## Out of scope

- DMLS range rules (already node-precise).
- Positions inside schema documents: `2026-10-11-schema-loader-yaml-spellings`.
- New YAML syntax support, source maps inside decoded scalar strings, or
  tracking the source of overrides and interpolation results.
- Schema grammar, coercion, validation policy, and diagnostic identity changes.
- A benchmark suite or multi-host performance spike. This change needs one
  source projection per validation snapshot, not a new performance gate.

## Acceptance criteria

1. Validate this YAML against a fixture schema in which `start` is a list of
   objects with a `then` list whose items allow `say`, not `sya`:

   ```yaml
   ---
   start:
     - then:
         - say: hello
         - sya: goodbye
   ---
   ```

   The unknown-key problem retains the parent pointer `/start/0/then/1`
   and `offending_property: sya`, and reports line 5, column 9 in both JSON
   and human output. The fixture uses a test schema rather than depending on
   lifecycle grammar that a later rollout step will introduce.
2. With `tags: number[]`, a wrong-typed item in a block list reports the
   start of `oops`, excluding the list dash. A missing required field inside
   the second object item reports that mapping's start, not the absent field
   or the list dash. An unknown nested key reports its key token; a wrong
   value reports its value token. Include a flow mapping and flow sequence
   so same-line errors cannot pass with line-only positions.
3. `md compose` on a document with the same problem shows the same line and
   column in its schema-validation error.
4. Compare the library/CLI positions with DMLS diagnostic starts for the
   supported nodes above using the same complete Markdown bytes. Test ASCII
   and a non-ASCII prefix containing a character represented by two UTF-16
   units, with DMLS in each supported position encoding. Compare byte offsets
   or correctly converted positions, not raw column integers.
5. Test an existing path replaced by an assignment, a newly added nested path,
   and a newly added top-level path: they use the original node, nearest
   authored ancestor, and no position respectively. A root missing-field
   error uses the root mapping start. A block scalar and a decoded-content
   descendant use the scalar start. A valid unsupported YAML presentation
   retains top-level fallback reporting and does not fail source projection
   as a validation error. Empty position maps still produce no positions.
6. Cover quoted/escaped and empty mapping keys, numeric mapping keys, LF and
   CRLF, leading comments or blank lines, and Markdown's delimiter offset.
   Assert that diagnostic fields other than positions and validation outcomes
   remain unchanged. Existing locator consumers retain their behavior.
7. `just test` and `just lint` pass in `darkmatter/`, including DMLS. Tests are
   L1, use the repository Test Toolkit and `CliProcessFixture` for CLI spawns,
   use no terminal or browser windows, and are portable to macOS, Linux,
   Windows, and WSL2. Compile affected downstream consumers if the public map
   changes. No new CI cells or tier requirements.

## Open questions

None.
