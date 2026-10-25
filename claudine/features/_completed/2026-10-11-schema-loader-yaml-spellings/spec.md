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
review_iterations: 2
completed: true
human_review: false
message_to_agent: |-
  All four phases are implemented; read the "Phase 4" section of
  implementation-log.md first. Status is "implementation complete, ready for
  review". For a reviewer:

  1. Two recorded departures to weigh: grammar-error token ranging in DMLS
     also applies to single-line standalone definitions (not only folded
     ones), and the spec's description-only fold example originally loaded
     with a truncated span rather than failing as the spec stated.
  2. Behavior is documented in darkmatter/docs/topics/schemas/definition.md
     ("Source-Aware Presentation Boundary"), darkmatter/dmls/docs/diagnostics.md
     ("Ranging"), and docs/topics/schemas/dmls-schema-support.md. Every doc
     example was checked against the real `md` binary.
  3. Local evidence is macOS only (darkmatter 9150 passed / 11 skipped; lint
     clean). No OS-specific code changed; CI covers the other environments.
implemented: true
pipeline_step: 1
canonical_docs:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/diagnostics.md
related:
    - 2026-10-20-lifecycle-grammar
    - 2026-10-29-recursive-schema-types
---

# Schema Loader: Indentless Sequences and Folded Plain Scalars

Darkmatter's standalone SimplifiedSchema loader rejects two valid YAML
presentations: block sequences aligned with their parent mapping key and
plain scalar values continued across physical lines. A valid schema can fail
with `could not project SimplifiedSchema expression spans through YAML source`.
That prevents schema use in `md schema validate`, composition, schema-trigger
loading, and the Darkmatter language server (DMLS).

After this change, both presentations load with the same meaning as their
conventionally indented or single-line equivalents. Source ranges used for
schema diagnostics, hover, completion, and suggestion linting identify the
correct authored text.

The lifecycle-grammar investigation in `2026-10-20-lifecycle-grammar` recorded
these failures under its finding titled “Two valid-YAML spellings crash the
standalone loader.” Its generator currently works around them by indenting
sequences and disabling line wrapping. YAML serializers and editors that
insert physical line breaks can produce these presentations. Visual soft wrap
alone does not change a file and does not cause this failure.

## Decided

Decided 2026-10-10; clarified by inline review 2026-10-11: accept these two
presentations with precise source positions, while preserving the rest of the
closed source-map boundary.

### Indentless block sequences

A block sequence whose `-` sits at the same column as its parent mapping key
is accepted wherever the schema grammar already permits a sequence value.
Indentation changes presentation, never which values the schema grammar allows.

```yaml
kind: schema
types:
  t:
  - string
  - number
  next: boolean
```

The named definition `t` is the property-level union `[string, number]`.
`next` remains a sibling definition; it is not consumed as part of the list.
This also applies to union-valued properties in nested mapping arms and
compact mapping items
such as `- a: string`, where those structures are already semantically valid.

Both existing standalone envelopes retain their contracts: a pure document has
`$schema` as its sole root key; a kinded document has exactly `kind: schema`
and a `types` mapping. In particular, an indentless list under `$schema` may
be a root-level schema union, but a list under `types` is still invalid.

### Plain scalars continued across physical lines

A plain (unquoted) scalar continued onto more-indented lines is accepted,
including values in sequence items. The type-expression parser receives
exactly the decoded string that `serde_yaml_ng` produces. Ordinary adjacent
content lines fold to spaces; blank lines can preserve line breaks. Indentation,
comments, and trailing whitespace follow the existing YAML decoder's rules,
rather than a new rule that joins every line with a space.

```yaml
kind: schema
types:
  t: string(min(1)) -> a description long enough that the editor
    wraps it onto a second physical line
  next: boolean
```

The definition `t` retains the constraint `min(1)` and the description
`a description long enough that the editor wraps it onto a second physical
line`. `next` is a sibling, not continuation text. Continuations may occur in
the type expression, its constraints, or its description; this change is not
limited to wrapping descriptions. Folding never makes an otherwise invalid
type expression valid.

### Source ranges preserve authored coordinates

Structural spans and suggestion-candidate spans must both project through the
same decoded scalar coordinates. A token entirely on a continuation line has
a range on that line, excluding its leading indentation. A token or expression
that crosses a fold has one enclosing authored range, from its first authored
byte through its last; intervening line breaks and indentation lie inside that
range. This fits the existing contiguous-range API without introducing a new
public span representation.

Library source maps retain byte offsets into the original buffer. Add a
caller's document offset once, after scalar projection; do not normalize line
endings. DMLS converts those byte ranges using its existing negotiated position
encoding, including UTF-16, rather than treating byte offsets as editor columns.
Cursor lookup on continuation text must select the same decoded token and
schema node as its single-line counterpart. Positions in removed indentation
or folded whitespace follow the existing decoder's cursor-clamping behavior.

Malformed expressions need positions too, even though no successful parse
product exists. DMLS must project the grammar parser's expression-relative
error range through the offending decoded scalar. For example:

```yaml
kind: schema
types:
  t: string(min(1);
    bogus(2))
```

The unknown-constraint diagnostic identifies `bogus` on physical line 5,
starting at column 5 when displayed with one-based line and column numbers.
It must not underline the whole scalar starting on line 4.

Reader's note: the current standalone loader wraps failures in a
`SchemaError::SchemaDocument` without a token range, and DMLS's malformed-type
fallback currently selects the whole value. Accepting folded scalars alone
cannot fulfill this position requirement. Preserve the public error wrapper;
use the existing grammar error and decoded scalar mapping in DMLS to recover
the precise range. This requirement covers schema-authoring diagnostics, not
positions of invalid values in a document being validated.

A genuine projection failure retains the existing typed error and consumer
fallback behavior. Never fabricate a successful source range or substitute
`0..0` for a token in either newly supported presentation.

### The boundary stays closed otherwise

Block scalars (`|`, `>`), tags, multi-line quoted scalars, complex keys,
explicit flow-mapping keys, and directives remain outside the strict
schema-document source-map contract. Existing supported quoting, mappings,
flow sequences, anchors, and aliases retain their behavior.

The tolerant frontmatter locator continues to accept its existing broader set
of presentations. Reusing its decoder does not authorize those presentations
in strict schema authoring. This work changes neither standalone envelope
recognition nor raw JSON Schema interpretation.

## Requirements

- In the `darkmatter` library,
  [the schema source locator](../../../darkmatter/lib/src/markdown/schemas/simplified/source.rs)
  supplies authored ranges for schema values. Share its existing indentless
  sequence handling and the plain-scalar continuation decoder between strict
  schema authoring and tolerant frontmatter lookup. Separate these permissions
  from the current broad multiline/tolerant switch so strict parsing cannot
  accidentally admit tags, block scalars, quoted continuations, or empty values.
  Account for sibling keys, compact mapping items, and mapping-value anchors
  already supported by the strict locator. Do not add a third locator.
- In the `darkmatter` library,
  [the decoded scalar mapper](../../../darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs)
  preserves coordinates through indentation and folding. Reuse it for both
  structural spans and suggestion spans. The separate line-oriented suggestion
  scanner must also handle whole continued scalars; merely changing the block
  locator leaves schemas with `suggest(...)` vulnerable to projection failure.
  Use its range projection operation for nonempty ranges so starts exclude
  skipped indentation. Check decoded values against the semantic YAML value
  and fail on a mismatch. Repeated identical expressions must retain their
  own occurrence's spans.
- In `dmls`, [schema-authoring diagnostics](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs)
  report malformed definitions. Cover the error path described above as well
  as successful schema-node lookup, hover, and completion. Preserve diagnostic
  codes and severity, and preserve suggestion lint as advisory data that does
  not prevent a schema from loading.
- All parsing and projection remain passive and in-memory: no file reads,
  shell execution, network access, or document rewriting. File resolution in
  normal command invocations continues through the existing request context
  and `biscuit-file` reference handling.
- Update [Darkmatter's schema definition documentation](../../../darkmatter/docs/topics/schemas/definition.md),
  especially “Source-Aware Presentation Boundary,” with compact examples of
  both accepted presentations. Describe blank-line folding and the unchanged
  exclusions. Update the library's locator and suggestion-projection comments,
  and the darkmatter skill's schema notes if they describe the boundary.
  Update the area README only if its public claims conflict with this change.
- Reuse the existing scalar decoder and scans. No performance spike or new
  benchmark gate is required: this change concerns source decoding and range
  projection, not named-type expansion or resolved-schema size.

## Out of scope

- Other YAML presentations, new type-expression grammar, or new envelope forms.
- Named-type reference semantics, addressed separately by
  `2026-10-13-named-type-presence-and-local-names`.
- Positions of invalid frontmatter values in documents being validated,
  addressed separately by `2026-10-15-diagnostic-node-positions`.
- A public error-type redesign, a general YAML syntax tree, or a change to
  tolerant frontmatter lookup behavior.

## Acceptance criteria

1. Both valid examples load through the `darkmatter` library's
   [standalone schema parser](../../../darkmatter/lib/src/markdown/schemas/simplified/standalone.rs),
   which recognizes schema envelopes, and through `md schema validate` against
   a document referencing them. Compare the resolved JSON Schema values with
   conventional spellings using structural equality. Source spans are expected
   to differ; serialization byte equality is not the semantic contract. CLI
   tests include one valid instance and one invalid instance to prove the
   referenced schema was applied, not just that the process exited successfully.
2. Cover both envelopes, including an indentless pure `$schema` root union,
   a property union nested under a mapping arm, and a sequence of compact
   mappings. Include a sibling after each list and continuation to detect
   accidental consumption. A `types` sequence remains rejected.
3. Test folded plain values in mapping and sequence positions, multiple
   continuation lines, a blank line, a trailing comment, and a final line
   without a newline. Compare decoded text with `serde_yaml_ng`; compare
   resolved semantics with an equivalent supported scalar spelling.
4. Check exact structural and suggestion ranges on continuation text and
   across a fold. Include a lint-invalid suggestion on a continuation line
   and repeated identical definitions so occurrence lookup cannot silently
   point at a different node. The lint stays advisory. Include targeted CRLF,
   non-ASCII text, and nonzero document-offset cases; retain the original
   authored buffer when asserting byte ranges.
5. Through the existing DMLS L1 harness, the `bogus` example reports the
   continuation token's exact range. Hover and completion on continuation
   text and indentless items select the correct node. Check UTF-16 conversion
   with non-ASCII text before a target token. Cover both a complete expression
   and the existing partially typed completion path without requiring an
   invalid document to load successfully.
6. Regression cases for block scalars, tags, multi-line quoted scalars,
   complex keys, explicit flow-mapping keys, and directives remain rejected
   through strict source-aware parsing. Through the standalone loader,
   projection failures remain wrapped in `SchemaError::SchemaDocument`; do
   not require its caller to receive the inner projection error directly.
   Existing tolerant-frontmatter tests and supported anchor/alias tests pass.
7. The shipped schema corpus loads unchanged. Existing normal-invocation tests
   remain green. Run `just test` and `just lint` in `darkmatter/` through the
   repository's nextest recipes. New tests are L1, use the test toolkit's
   isolated fixtures for CLI calls, open no terminal or browser windows, and
   are portable to macOS, Linux, Windows, and WSL2. Construct LF and CRLF test
   strings explicitly; do not rely on checkout line endings. No new CI cells.

## Open Questions

None. The source-range behavior and error-wrapper boundary above are decided
within the existing APIs; no broader design change is required.
