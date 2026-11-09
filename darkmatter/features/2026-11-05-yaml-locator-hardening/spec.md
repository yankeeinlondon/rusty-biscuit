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
status: finalized-spec
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-07
clarified: false
implemented: true
review_iterations: 0
packages:
    - simplified-schema
    - darkmatter
related:
    - 2026-11-04-loading-data
depends-on:
    - 2026-11-04-loading-data
implemented_by: "claude/opus"
---

# YAML Locator Hardening

Report authored locations for YAML contract violations when a data file uses
aliases (`*name`) or flow collections (`[…]` and `{…}`) across several lines.
A problem outside an alias points to its own value; a problem inside an
alias-expanded value points to the alias use. Neither construct should remove
positions from unrelated values.

This intentionally replaces the whole-file, pointer-only exception in
2026-11-04-loading-data. It changes diagnostic locations, not accepted YAML,
parsed values, contract validation, or diagnostic record counts.

## Motivation

Today, one alias anywhere in a YAML file removes positions from every
`load_data` contract violation in that file. Darkmatter also deliberately
removes them for a multi-line flow collection, although the shared locator
now supports those collections. Authors must then find a path such as
`/servers/12/port` by hand, even when that value has an unambiguous source
location.

An alias is a reference to an earlier anchored value (`&name`). Its expanded
children have no separate text at the use site. Reporting the alias token
makes the failing use actionable while retaining the full JSON Pointer,
which identifies the particular failing child.

## Current behavior and ownership

Verified against the code on 2026-11-07:

- In simplified-schema, [PositionMap](../../../simplified-schema/lib/src/positions.rs)
  projects the authored YAML tree onto the parsed mapping-order index. It
  checks both mapping keys and the total number of indexed mappings. Any
  alias currently refuses the whole projection, including aliases under a
  sequence root. Its top-level fallback remains available to other callers.
- In Darkmatter, [SourceLocator](../../lib/src/markdown/compose/expression/data/locate.rs)
  locates `load_data` failures through that map's byte ranges, then converts
  offsets to character columns. It never uses the top-level fallback's
  column-1 guesses. A separate multi-line-flow check deliberately suppresses
  the map even when projection succeeded.
- In Darkmatter, [load_yaml_document](../../lib/src/markdown/schemas/yaml_documents/load.rs)
  uses simplified-schema's
  [load_admitted_yaml](../../../simplified-schema/lib/src/numeric/ingress.rs)
  to parse one document and check numbers. Aliases are expanded by the YAML
  parser. Neither loader applies merge-key expansion: `<<: *b` retains a
  literal `<<` member whose value is the expanded alias.
- Mapping order remains available even without positions. Restoring locations
  must not reorder failures by their new coordinates.

**Reader's note:** the earlier draft described merge keys as an already
expanded mapping and proposed choosing a position for an inherited property.
That would introduce new YAML semantics. This revision preserves the loader's
literal `<<` behavior; adding inheritance would require a separate design.
Also, simply allowing an alias in the projection walk is insufficient:
expanded mappings below that alias still appear in the mapping-order index
and must be accounted for in its final completeness check.

## Design decisions

### Alias uses are authored boundaries

Keep the existing source tree; treat an alias as an opaque leaf for location
lookup, whether the parser expands it to a scalar, sequence, or mapping.

- A failure on the alias or any expanded descendant points to the `*` of
  that use, with the byte range covering only `*name`, excluding comments,
  whitespace, and neighboring delimiters. Do not redirect to the anchor.
- A failure in the anchored original follows the existing location rule for
  that original node. This change does not redefine whether an anchored
  node's existing span includes its anchor prefix.
- Keep one record per failing instance path. If `/base/port`,
  `/one/port`, and `/two/port` all fail, report all three; the latter two
  point to their respective alias uses. Do not merge records because their
  values or source coordinates coincide. Existing report limits still apply.
- A missing child keeps its missing path and uses its nearest existing
  parent. If that parent is inside an alias expansion, use the alias token.
  Multiple failing children can therefore share one alias location.
- Preserve target selection outside aliases: simplified-schema frontmatter
  problems identify an unexpected key's key token, while Darkmatter's
  `load_data` records identify its value. Missing-key problems identify the
  parent container. These existing differences are intentional.

For example, under a contract requiring numeric `port` values:

```yaml
base: &b {port: x}
one: *b
two: *b
```

The failing paths are `/base/port`, `/one/port`, and `/two/port`.
The first identifies `x`; the others identify `*b` on lines 2 and 3.

### Preserve projection verification

In simplified-schema, adapt
[matches_orders](../../../simplified-schema/lib/src/positions.rs), the
check that the source tree agrees with parsed mapping order, to account for
alias-expanded subtrees without inventing source children for them.

All authored non-alias mappings must still match their indexed keys in
order. An indexed mapping is accounted for either by a matching authored
mapping or because its pointer is at or below an authored alias boundary.
Recognize boundaries by decoded pointer segments (or correctly escaped
segment boundaries), so `/a` never exempts `/ab`. Mappings outside these
boundaries must still cause projection refusal when unmatched. Do not just
delete the final mapping-count check, and do not repeatedly scan the entire
index for each alias.

Alias recognition must use the shared token reader, not a search for `*` in
raw text; quoted strings, comments, and block scalar contents are not aliases.
The semantic parser remains responsible for undefined aliases and recursive
or otherwise rejected input. The locator must not resolve anchors again or
expand their values a second time.

If an unrelated source/index mismatch still prevents projection, retain the
existing whole-map fallback. Recovering partial positions for every possible
YAML construct is outside this change. A genuine inability to prove a range
still produces a pointer-only `load_data` record.

### Keep merge keys literal

For `base: &b {port: x}` followed by `server: {<<: *b, port: 2}`,
`/server/port` is the explicit `2`, and `/server/<</port` is a descendant of
the literal `<<` value. The latter locates at `*b`; there is no inherited
`/server/port` from `base`. The same rule applies to quoted `"<<"` keys and
sequences of aliases used as literal values. Preserve explicit key order and
validation of the literal member, including an unexpected-key failure if the
contract does not permit it.

**Merge-key hint (author decision, 2026-11-08).** When that unexpected key is
exactly `<<` (plain or quoted), the `load_data` contract-mismatch message gains
one sentence saying that YAML merge keys are not applied, so `<<` is read as an
ordinary key. The code, pointer, location, and record count are unchanged; only
the message text adds the hint. Merge-key inheritance itself stays out of scope
and no follow-up spec is filed.

### Enable multi-line flow positions

In Darkmatter, remove the multi-line-flow suppression and its now-unused
helper/import from
[SourceLocator](../../lib/src/markdown/compose/expression/data/locate.rs).
Use the shared position map as for any other YAML document. Cover flow
sequences of objects, nested flow mappings, comments, and quoted closing
brackets or braces. An alias inside such a collection obeys the same alias
boundary rule and does not suppress neighboring positions.

### Preserve coordinate and diagnostic contracts

The shared simplified-schema map continues to expose one-based UTF-8 byte
columns and byte ranges into its supplied text. Darkmatter's `load_data`
continues to convert byte offsets into one-based character columns using
`biscuit_file::source_position`; it must not copy the map's byte column into
`data_location.column`. Keep the established LF/CRLF rules, leading
byte-order-mark handling, and frontmatter delimiter offsets for each surface.

Only locations become more precise, apart from the merge-key hint above. Keep
JSON Pointers, messages, codes,
file/URL identity, authored ordering, report limits, and omission of unknown
coordinates unchanged. In CLI records, `source` remains the Markdown call
site; the improved YAML position belongs in `data_location`. No output schema
change is needed.

The shared map also serves frontmatter and whole-file YAML validation in
Darkmatter, plus editor diagnostics in DMLS. Their existing coordinate
conversion and range projection must continue to work; no consumer should
reinterpret an alias boundary as a range covering its expanded definition.

## Scope and implementation phases

### Phase 1: Shared alias projection

Update simplified-schema's projection verification and alias token ranges.
Update the tests that explicitly require whole-file alias refusal; keep
unquoted-template refusal and unrelated mismatch tests. Add cases for aliases
to scalars, mappings, and sequences, a sequence root, repeated uses, escaped
pointer keys, and literal `<<` entries. Prove both starts and byte ranges.

Review the affected public and inline comments in `positions.rs`; descriptions
that say every alias prevents projection must change with the behavior.

### Phase 2: Darkmatter integration

Remove the flow suppression. Update the locator's alias-refusal test and
flow-refusal test. Extend the library's
[load_data diagnostic tests](../../lib/tests/l1/load_data_diagnostics.rs)
and the CLI's [load_data tests](../../cli/tests/l1/load_data.rs) through
normal composition. Keep malformed-input behavior separate from contract
mismatches. Add shared-map integration coverage for frontmatter and whole-file
YAML validation, and a DMLS alias range regression through its existing
headless test harness.

### Phase 3: Documentation and verification

Update Darkmatter's [loading-data guide](../../docs/topics/loading-data.md)
and [CLI data contracts](../../docs/cli/data-contracts.md) with an alias-use
example, multi-line flow support, literal merge-key behavior, and the remaining
unproven-position fallback. Update the relevant simplified-schema diagnostic
documentation and skill guidance if they describe alias refusal. Preserve the
historical 2026-11-04-loading-data specification; record its intentional
behavioral replacement in this implementation's log.

Run `just test` in `simplified-schema/` and `darkmatter/`, and the appropriate
package-area lint checks. Use nextest through those recipes. These are parser
and diagnostic changes: tests must not open or focus terminal/browser windows.
No new dependency, performance spike, benchmark matrix, or OS-specific
implementation is required. Keep the existing parser and shared source tree;
avoid per-alias whole-document searches or repeated expansion.

## Non-goals

- A second YAML parser or changes to which YAML documents are accepted.
- Merge-key inheritance, anchor-definition navigation, or alias deduplication.
- Changing the top-level fallback for callers that already use it.
- Changing malformed-input locations. Duplicate keys, extra YAML documents,
  non-string keys, rejected tags/numbers, and undefined aliases fail loading
  before contract validation; they are not pointer-only contract mismatches.
  Preserve whatever location the existing loader can prove.
- Guaranteeing a position for every possible YAML source/index mismatch.

## Open Questions

None remain for this scope. Alias use-site locations and literal merge-key
behavior are explicit decisions above, and the author approved the merge-key
hint on 2026-11-08. Supporting merge inheritance would
change parsed data and needs a separate specification, rather than being
introduced as a diagnostic fix.

## Acceptance criteria

- With the data below and contract
  `"{ base: { port: number }, servers: { port: number }[] }"`, the sole
  violation `/servers/1/port` reports line 4, column 12 (`x`), despite the
  unrelated alias:

  ```yaml
  base: &b {port: 1}
  servers:
    - *b
    - {port: x}
  ```

- With the three-use example above, the original failure points to `x` at
  line 1, column 17, and `/one/port` and `/two/port` point to line 2,
  column 6 and line 3, column 6. A byte-range lookup returns only each
  alias token. Multiple failing children and a missing child retain distinct
  pointers even when they share that range.
- Aliases to scalar, sequence, and mapping values retain positions for
  neighboring values. Comments and quoted or block-scalar text containing
  `*name` are not treated as alias boundaries.
- Literal `<<` entries retain their parsed shape and key order. An explicit
  sibling value locates at itself, a descendant of the aliased literal value
  locates at its alias token, and no inherited property is manufactured. A
  closed contract that rejects `<<` reports the unexpected key with the
  merge-key hint; any other unexpected key's message is unchanged.
- Multi-line flow sequences and nested flow mappings report positions inside
  and after the collection, including when a flow item is an alias and when
  comments or quoted strings contain `]` or `}`.
- A projection with an unrelated mapping-key or mapping-count mismatch still
  refuses projection. Alias boundary `/a` does not hide a mismatch at `/ab`;
  keys containing `/` and `~` are compared as decoded segments.
- Frontmatter tests include both an alias-derived failure and an unrelated
  failure with document-relative locations. Whole-file YAML validation and
  DMLS ranges preserve their existing coordinate conventions.
- Unicode before a value proves character columns in `load_data` and byte
  columns in the shared map; LF/CRLF and a leading byte-order mark preserve
  correct offsets. CLI composition asserts the unchanged Markdown `source`
  alongside improved `data_location`, using the existing output schema.
- Diagnostic count, pointers, ordering, limits, and malformed-YAML behavior
  remain unchanged. Tests and docs no longer claim that any alias or
  multi-line flow collection alone forces the entire file to pointer-only.
